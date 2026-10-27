//! Salto tra elementi collegati: dal contatto alla sua madre (bobina, interruttore, scheda PLC),
//! dalla madre ai suoi contatti, da un rimando a quello che gli corrisponde sull'altro foglio.
//!
//! Il salto porta al foglio dell'arrivo, lo inquadra e lo seleziona: ripetuto sull'arrivo torna
//! indietro. Una madre ha più contatti: `indice` sceglie quale (il successivo a ogni ripetizione
//! dalla finestra).

use cadcraft_engine::cmd::has_doc;
use cadcraft_engine::{CommandSpec, Result, Session};
use serde_json::{Value, json};

use crate::siglatura::{self, Componente};
use crate::util::{errore, esegui, numero_param, testo_param};
use crate::{fili, fogli, impostazioni, squadratura};

type P = (f64, f64);

/// Un elemento a cui si può saltare.
#[derive(Clone, Debug, PartialEq)]
pub struct Arrivo {
    pub handle: String,
    pub foglio: u32,
    pub pos: P,
    /// `foglio/colonnaRiga`, come nei riferimenti incrociati.
    pub riferimento: String,
    /// Che cosa è: la sigla con il tipo (`-KM1 bobina`, `-KM1 contatto 13-14`, `rimando L1`).
    pub descrizione: String,
}

impl Arrivo {
    fn da_componente(c: &Componente) -> Self {
        let b = &c.blocco;
        let cosa = if c.madre() {
            "madre".to_string()
        } else {
            let pin = [b.attributo("PINA1"), b.attributo("PINB1")].iter().filter(|p| !p.is_empty()).copied().collect::<Vec<_>>().join("-");
            if pin.is_empty() { "contatto".to_string() } else { format!("contatto {pin}") }
        };
        Arrivo { handle: b.handle.clone(), foglio: c.foglio, pos: b.pos, riferimento: squadratura::riferimento(c.foglio, b.pos.0, b.pos.1), descrizione: format!("{} {cosa}", c.nome) }
    }

    fn da_rimando(r: &fili::Rimando) -> Self {
        let verso = if r.partenza { "partenza" } else { "arrivo" };
        Arrivo { handle: r.handle.clone(), foglio: r.foglio, pos: r.pos, riferimento: squadratura::riferimento(r.foglio, r.pos.0, r.pos.1), descrizione: format!("rimando {} ({verso})", r.segnale) }
    }

    fn json(&self) -> Value {
        json!({"handle": self.handle, "foglio": self.foglio, "pos": [self.pos.0, self.pos.1], "riferimento": self.riferimento, "descrizione": self.descrizione})
    }
}

/// Due componenti hanno la stessa sigla (e lo stesso quadro, se le sigle si ripetono tra quadri).
fn stessa_sigla(a: &Componente, b: &Componente, per_quadro: bool) -> bool {
    let quadro = |c: &Componente| c.blocco.attributo("QUADRO").trim().to_ascii_uppercase();
    !a.nome.trim().is_empty() && a.nome.trim().eq_ignore_ascii_case(b.nome.trim()) && (!per_quadro || quadro(a) == quadro(b))
}

/// Dove si può saltare dall'elemento `handle`: la madre di un contatto, i contatti di una madre
/// (in ordine di foglio e di posizione), il rimando corrispondente. Vuoto se non c'è niente di
/// collegato; errore se l'handle non è un componente né un rimando.
pub fn arrivi(s: &mut Session, handle: &str) -> Result<Vec<Arrivo>> {
    let rimandi = fili::rimandi(s)?;
    if let Some(i) = rimandi.iter().position(|r| r.handle.eq_ignore_ascii_case(handle)) {
        let altri = fili::abbina(&rimandi).into_iter().filter_map(|(p, a)| if p == i { Some(a) } else if a == i { Some(p) } else { None });
        return Ok(altri.map(|k| Arrivo::da_rimando(&rimandi[k])).collect());
    }
    let tutti = siglatura::componenti(s)?;
    let c = tutti.iter().find(|c| c.blocco.handle.eq_ignore_ascii_case(handle)).ok_or_else(|| errore("Selezionare un componente (madre o contatto) o un rimando"))?;
    let per_quadro = impostazioni::correnti().sigle_uguali_quadri;
    let mut v: Vec<&Componente> = if c.madre() {
        tutti.iter().filter(|f| f.figlio() && stessa_sigla(f, c, per_quadro)).collect()
    } else {
        // la madre dello stesso quadro, se ce n'è più d'una con la stessa sigla
        let madri: Vec<&Componente> = tutti.iter().filter(|m| m.madre() && stessa_sigla(m, c, false)).collect();
        let stesso_quadro: Vec<&Componente> = madri.iter().copied().filter(|m| stessa_sigla(m, c, true)).collect();
        if stesso_quadro.is_empty() { madri } else { stesso_quadro }
    };
    v.sort_by(|a, b| (a.foglio, -a.blocco.pos.1, a.blocco.pos.0).partial_cmp(&(b.foglio, -b.blocco.pos.1, b.blocco.pos.0)).unwrap_or(std::cmp::Ordering::Equal));
    Ok(v.into_iter().map(Arrivo::da_componente).collect())
}

/// Porta la vista all'arrivo: foglio, inquadratura intorno al simbolo e selezione.
pub fn vai(s: &mut Session, a: &Arrivo) -> Result<()> {
    if a.foglio > 0 && fogli::corrente(s)?.is_none_or(|f| f.numero != a.foglio) {
        fogli::vai(s, a.foglio)?;
    }
    let (x, y) = a.pos;
    esegui(s, "zoom.window", json!({"p1": [x - 45.0, y - 30.0], "p2": [x + 45.0, y + 30.0]}))?;
    esegui(s, "select", json!({"handles": [a.handle]}))?;
    Ok(())
}

fn run_salta(s: &mut Session, p: &Value) -> Result<Value> {
    let handle = testo_param(p, "handle").or_else(|| s.selection().first().map(|h| h.hex())).ok_or_else(|| errore("Selezionare un componente o un rimando"))?;
    let arrivi = arrivi(s, &handle)?;
    let vai_davvero = p.get("vai").and_then(Value::as_bool).unwrap_or(true);
    let scelta = if arrivi.is_empty() { None } else { Some(numero_param(p, "indice").unwrap_or(0) as usize % arrivi.len()) };
    if let (Some(k), true) = (scelta, vai_davvero) {
        vai(s, &arrivi[k])?;
    }
    Ok(json!({
        "da": handle,
        "arrivi": arrivi.iter().map(Arrivo::json).collect::<Vec<_>>(),
        "scelto": scelta.map(|k| arrivi[k].json()),
    }))
}

pub fn comandi() -> Vec<CommandSpec> {
    vec![CommandSpec {
        id: "salta",
        label: "Vai al Collegato",
        menu: &["Elettrico", "Vai al Collegato"],
        shortcut: None,
        aliases: &[],
        params: "{handle? (default la selezione), indice?: n (quale contatto di una madre), vai?: bool (default true)} → {arrivi: [{handle, foglio, riferimento, descrizione}], scelto}",
        enabled: has_doc,
        run: run_salta,
        undoable: false,
        interactive: None,
        transparent: false,
    }]
}
