//! Collegamenti: le reti di fili con i morsetti dei componenti che toccano, il numero e il
//! conduttore di ogni rete; tabella di cablaggio (da componente/morsetto a componente/morsetto).
//!
//! Il conduttore di una rete (sezione, colore, codice) è quello di un blocco `CE_DATIFILO`
//! posato su un suo filo ("Specializza / Modifica materiale dei conduttori"); senza, quello in
//! uso nelle impostazioni (delle alimentazioni se la rete tocca un rimando di alimentazione).
//! Come nei CAD elettrici diffusi l'indicazione si scrive accanto al filo (`1x1.5 BK`).

use cadcraft_engine::cmd::has_doc;
use cadcraft_engine::{CommandSpec, Result, Session};
use serde_json::{Value, json};

use crate::fili::{self, Segmento};
use crate::impostazioni::{self, Conduttore};
use crate::numerazione::{self, BLOCCO_NUMERO, su};
use crate::util::{Inserito, blocchi_definiti, blocchi_inseriti, blocco, colore, errore, esegui, handles, in_gruppo, inserisci_su_layer, punto_param, su_layer_zero, testo_param};
use crate::{fogli, morsetti, simboli, squadratura};

pub const BLOCCO_DATI: &str = "CE_DATIFILO";
const TOLL: f64 = 0.01;

type P = (f64, f64);

fn vicini(a: P, b: P) -> bool {
    (a.0 - b.0).abs() < TOLL && (a.1 - b.1).abs() < TOLL
}

/// Un capo di una rete: il morsetto di un componente, un morsetto di morsettiera o un rimando.
#[derive(Clone, Debug, PartialEq)]
pub struct Capo {
    /// Sigla del componente, nome della morsettiera o segnale del rimando.
    pub sigla: String,
    /// Numero del morsetto (pin) o del morsetto della morsettiera.
    pub pin: String,
    pub quadro: String,
    pub pos: P,
    pub tipo: TipoCapo,
    /// Il blocco da cui viene (simbolo, morsetto o rimando).
    pub blocco: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TipoCapo {
    Componente,
    Morsetto,
    Rimando,
}

impl Capo {
    /// `-KM01.1:A1`, `-XG:12`.
    pub fn testo(&self) -> String {
        if self.pin.is_empty() { self.sigla.clone() } else { format!("{}:{}", self.sigla, self.pin) }
    }
}

/// Una rete di un foglio con i suoi capi, il numero e il conduttore.
#[derive(Clone, Debug)]
pub struct Rete {
    pub foglio: u32,
    pub nome: String,
    pub fili: Vec<Segmento>,
    pub capi: Vec<Capo>,
    pub conduttore: Conduttore,
    /// Il conduttore viene da un'indicazione sul filo (non dal predefinito).
    pub specializzato: bool,
}

fn crea_blocco(s: &mut Session) -> Result<()> {
    if blocchi_definiti(s)?.iter().any(|b| b.eq_ignore_ascii_case(BLOCCO_DATI)) {
        return Ok(());
    }
    su_layer_zero(s, |s| {
        colore(s, "BYLAYER")?;
        let mut h = handles(&esegui(s, "attdef", json!({"tag": "TESTO", "prompt": "Indicazione", "default": "", "at": [0.0, 0.0], "height": 1.8}))?);
        if let Some(t) = h.first() {
            crate::util::allinea(s, t, cadcraft_engine::doc::HAlign::Center)?;
        }
        for tag in ["SEZIONE", "COLORE", "CODICE", "COSTRUTTORE"] {
            h.extend(handles(&esegui(s, "attdef", json!({"tag": tag, "prompt": tag, "default": "", "at": [0.0, -1.0], "height": 1.0, "invisible": true}))?));
        }
        blocco(s, BLOCCO_DATI, h, "Dati del conduttore")
    })
}

/// L'indicazione sta su un filo della rete: 3 mm a destra di un tratto verticale o 3 mm sotto
/// uno orizzontale (dall'altra parte rispetto al numero).
fn dati_su(e: &Inserito, g: &[&Segmento]) -> bool {
    g.iter().any(|f| ((f.a.0 - f.b.0).abs() < TOLL && su(f, (e.pos.0 - 3.0, e.pos.1))) || ((f.a.1 - f.b.1).abs() < TOLL && su(f, (e.pos.0, e.pos.1 + 3.0))))
}

fn conduttore_da(e: &Inserito) -> Conduttore {
    Conduttore { sezione: e.attributo("SEZIONE").into(), colore: e.attributo("COLORE").into(), codice: e.attributo("CODICE").into(), costruttore: e.attributo("COSTRUTTORE").into() }
}

/// Distanza massima (mm) tra l'attributo di un pin e l'estremo del filo che ci arriva, nei
/// simboli di altri programmi.
const PIN_ATTRIBUTO: f64 = 4.0;

/// I morsetti dei componenti posati su un foglio: (capo, punto). Per i simboli della libreria
/// il punto viene dal simbolo; per quelli di altri programmi (blocchi con `PRES` e i pin
/// `PINA1`, `PINB1`…) è l'estremo di filo più vicino all'attributo di ogni pin.
fn capi_componenti(s: &Session, layer: &str, quadri: &[(String, String)], segmenti: &[Segmento]) -> Result<Vec<Capo>> {
    use cadcraft_engine::doc::EntityKind;
    let mut v = Vec::new();
    let estremi: Vec<P> = segmenti.iter().flat_map(|f| [f.a, f.b]).collect();
    for e in s.doc()?.model.iter().filter(|e| e.common.layer == layer) {
        let EntityKind::Insert(i) = &e.kind else { continue };
        let attributo = |tag: &str| i.attribs.iter().find(|a| a.tag.eq_ignore_ascii_case(tag)).map_or("", |a| a.text.value.as_str());
        let pres = attributo("PRES").trim().to_ascii_uppercase();
        // il componente è la madre o, per un contatto, la sigla della sua madre
        let sigla = attributo("NOME").to_string();
        let madre = pres == "M";
        let quadro = |sigla: &str| if madre { attributo("QUADRO").to_string() } else { quadri.iter().find(|(n, _)| n.eq_ignore_ascii_case(sigla)).map(|(_, q)| q.clone()).unwrap_or_default() };
        let capo = |pin: &str, pos: P| Capo { sigla: sigla.clone(), pin: pin.to_string(), quadro: quadro(&sigla), pos, tipo: TipoCapo::Componente, blocco: e.handle.hex() };
        if let Some(sim) = simboli::trova(&i.block) {
            if sim.grafico() {
                continue;
            }
            for (tag, p) in sim.pin((i.insert.x, i.insert.y)) {
                v.push(capo(attributo(&tag), p));
            }
            continue;
        }
        // un simbolo di un altro programma: le madri e i contatti, non i morsetti né gli specchietti
        if !(madre || pres.starts_with('F')) || !attributo("MORSETTIERA").is_empty() {
            continue;
        }
        let mut usati: Vec<P> = Vec::new();
        for a in i.attribs.iter().filter(|a| a.tag.to_ascii_uppercase().starts_with("PIN") && !a.text.value.trim().is_empty()) {
            let q = (a.text.insert.x, a.text.insert.y);
            let vicino = estremi
                .iter()
                .filter(|p| !usati.iter().any(|u| vicini(*u, **p)))
                .map(|p| (((p.0 - q.0).hypot(p.1 - q.1)), *p))
                .filter(|(d, _)| *d <= PIN_ATTRIBUTO)
                .min_by(|a, b| a.0.total_cmp(&b.0));
            if let Some((_, p)) = vicino {
                usati.push(p);
                v.push(capo(&a.text.value, p));
            }
        }
    }
    Ok(v)
}

/// Distanza massima (mm) tra un morsetto che interrompe il filo (come nei progetti di altri
/// programmi) e gli estremi dei fili che gli arrivano.
pub const MORSETTO_INTERROTTO: f64 = 10.0;

/// L'estremo `q` arriva al morsetto in `p` che interrompe il filo: allineato in verticale o in
/// orizzontale, entro [`MORSETTO_INTERROTTO`].
pub fn arriva_al_morsetto(p: P, q: P) -> bool {
    let (dx, dy) = ((q.0 - p.0).abs(), (q.1 - p.1).abs());
    (dx < TOLL && dy > TOLL && dy <= MORSETTO_INTERROTTO) || (dy < TOLL && dx > TOLL && dx <= MORSETTO_INTERROTTO)
}

/// Le reti dei fogli `fogli_da` (tutti se vuoto).
pub fn reti(s: &mut Session, fogli_da: &[u32]) -> Result<Vec<Rete>> {
    let imp = impostazioni::correnti();
    let rimandi = fili::rimandi(s)?;
    let numeri = blocchi_inseriti(s, BLOCCO_NUMERO)?;
    let dati = blocchi_inseriti(s, BLOCCO_DATI)?;
    let tutti_morsetti = morsetti::morsetti(s)?;
    // il quadro di un contatto è quello della sua madre (una volta per tutti i fogli)
    let quadri: Vec<(String, String)> =
        crate::siglatura::componenti(s)?.into_iter().filter(|c| c.madre()).map(|c| (c.nome.clone(), c.blocco.attributo("QUADRO").to_string())).collect();
    let mut out = Vec::new();
    for f in fogli::elenco(s)? {
        if !fogli_da.is_empty() && !fogli_da.contains(&f.numero) {
            continue;
        }
        let segmenti = fili::fili(s, &f.layer)?;
        let componenti = capi_componenti(s, &f.layer, &quadri, &segmenti)?;
        for g in numerazione::reti(&segmenti) {
            let g: Vec<&Segmento> = g.into_iter().filter_map(|i| segmenti.get(i)).collect();
            let tocca = |p: P| g.iter().any(|x| su(x, p));
            let estremo = |p: P| g.iter().any(|x| vicini(x.a, p) || vicini(x.b, p));
            let mut capi: Vec<Capo> = componenti.iter().filter(|c| tocca(c.pos)).cloned().collect();
            // il morsetto sta sul filo, o lo interrompe e i fili gli arrivano sopra e sotto
            let collegato = |p: P| tocca(p) || (!segmenti.iter().any(|x| su(x, p)) && g.iter().any(|x| arriva_al_morsetto(p, x.a) || arriva_al_morsetto(p, x.b)));
            for m in tutti_morsetti.iter().filter(|m| m.foglio == f.numero && collegato(m.blocco.pos)) {
                capi.push(Capo { sigla: m.morsettiera.clone(), pin: m.numero.clone(), quadro: m.blocco.attributo("QUADRO").into(), pos: m.blocco.pos, tipo: TipoCapo::Morsetto, blocco: m.blocco.handle.clone() });
            }
            let rim: Vec<&fili::Rimando> = rimandi.iter().filter(|r| r.foglio == f.numero && estremo(r.pos)).collect();
            for r in &rim {
                capi.push(Capo { sigla: r.segnale.clone(), pin: String::new(), quadro: String::new(), pos: r.pos, tipo: TipoCapo::Rimando, blocco: r.handle.clone() });
            }
            // dall'alto in basso, da sinistra a destra
            capi.sort_by(|a, b| b.pos.1.total_cmp(&a.pos.1).then(a.pos.0.total_cmp(&b.pos.0)));
            let nome = rim
                .iter()
                .map(|r| r.segnale.trim().to_string())
                .find(|t| !t.is_empty())
                .or_else(|| numeri.iter().filter(|e| e.layer == f.layer).find(|e| g.iter().any(|x| su(x, (e.pos.0 + 1.0, e.pos.1)) || su(x, (e.pos.0, e.pos.1 - 1.0)))).map(|e| e.attributo("N").to_string()))
                .unwrap_or_default();
            let indicazione = dati.iter().filter(|e| e.layer == f.layer).find(|e| dati_su(e, &g));
            let alimentazione = rim.iter().any(|r| r.famiglia == fili::Famiglia::Alimentazione);
            let (mut conduttore, specializzato) = match indicazione {
                Some(e) => (conduttore_da(e), true),
                None if alimentazione => (imp.conduttore_alimentazione.clone(), false),
                None => (imp.conduttore.clone(), false),
            };
            // senza indicazione sul filo, il colore del potenziale (L1 marrone, PE giallo-verde…)
            if !specializzato
                && let Some(p) = crate::tensioni::potenziale(&nome)
                && !p.colore.is_empty()
            {
                conduttore.colore = p.colore;
            }
            out.push(Rete { foglio: f.numero, nome, fili: g.into_iter().cloned().collect(), capi, conduttore, specializzato });
        }
    }
    Ok(out)
}

/// Una riga della tabella di cablaggio: un filo da un capo all'altro.
#[derive(Clone, Debug, PartialEq)]
pub struct Cablaggio {
    pub filo: String,
    pub da: Capo,
    pub a: Capo,
    pub conduttore: Conduttore,
    pub foglio: u32,
}

/// I fili da cablare: in ogni rete i capi (componenti e morsetti) collegati uno dopo l'altro,
/// dall'alto in basso. I rimandi non sono capi da cablare.
pub fn cablaggio(reti: &[Rete]) -> Vec<Cablaggio> {
    let mut v = Vec::new();
    for r in reti {
        let capi: Vec<&Capo> = r.capi.iter().filter(|c| c.tipo != TipoCapo::Rimando).collect();
        for w in capi.windows(2) {
            if let [a, b] = w {
                v.push(Cablaggio { filo: r.nome.clone(), da: (*a).clone(), a: (*b).clone(), conduttore: r.conduttore.clone(), foglio: r.foglio });
            }
        }
    }
    v
}

pub const INTESTAZIONE_CABLAGGIO: [&str; 10] = ["Da quadro", "Da componente", "Da pin", "Filo", "Sezione", "Colore", "A componente", "A pin", "A quadro", "Foglio"];

pub fn righe_cablaggio(c: &[Cablaggio]) -> Vec<Vec<String>> {
    c.iter()
        .map(|r| {
            vec![
                r.da.quadro.clone(),
                r.da.sigla.clone(),
                r.da.pin.clone(),
                r.filo.clone(),
                r.conduttore.sezione.clone(),
                r.conduttore.colore.clone(),
                r.a.sigla.clone(),
                r.a.pin.clone(),
                r.a.quadro.clone(),
                format!("{:02}", r.foglio),
            ]
        })
        .collect()
}

// ---------- conduttori ----------

/// Il layer del foglio corrente e le reti toccate, ognuna con il punto toccato.
type Toccate = (String, Vec<(Vec<Segmento>, P)>);

/// Le reti del foglio corrente toccate dal punto o dalla fence, con il punto toccato.
fn toccate(s: &mut Session, at: Option<P>, fence: Option<(P, P)>) -> Result<Toccate> {
    let n = fogli::corrente(s)?.map(|f| f.numero).ok_or_else(|| errore("Nessun foglio corrente"))?;
    let layer = fogli::nome_layer(n);
    let segmenti = fili::fili(s, &layer)?;
    let punti: Vec<P> = match (at, fence) {
        (Some(p), _) => segmenti
            .iter()
            .map(|f| {
                let q = (p.0.clamp(f.a.0.min(f.b.0), f.a.0.max(f.b.0)), p.1.clamp(f.a.1.min(f.b.1), f.a.1.max(f.b.1)));
                ((q.0 - p.0).hypot(q.1 - p.1), q)
            })
            .filter(|(d, _)| *d <= 2.0)
            .min_by(|a, b| a.0.total_cmp(&b.0))
            .map(|(_, q)| vec![q])
            .unwrap_or_default(),
        (None, Some((da, a))) => morsetti::attraversamenti(&segmenti, da, a),
        _ => Vec::new(),
    };
    let gruppi = numerazione::reti(&segmenti);
    let mut out: Vec<(Vec<usize>, P)> = Vec::new();
    for q in punti {
        if let Some(g) = gruppi.iter().find(|g| g.iter().any(|&i| segmenti.get(i).is_some_and(|f| su(f, q))))
            && !out.iter().any(|(x, _)| x == g)
        {
            out.push((g.clone(), q));
        }
    }
    Ok((layer, out.into_iter().map(|(g, q)| (g.iter().filter_map(|&i| segmenti.get(i)).cloned().collect(), q)).collect()))
}

/// Specializza il conduttore delle reti toccate (punto o fence): l'indicazione accanto al filo
/// toccato, al posto di quella che c'era. Restituisce le reti cambiate.
pub fn specializza(s: &mut Session, at: Option<P>, fence: Option<(P, P)>, c: &Conduttore) -> Result<usize> {
    crea_blocco(s)?;
    let (layer, reti) = toccate(s, at, fence)?;
    if reti.is_empty() {
        return Err(errore("Nessun filo nel punto indicato"));
    }
    let dati = blocchi_inseriti(s, BLOCCO_DATI)?;
    for (g, q) in &reti {
        let refs: Vec<&Segmento> = g.iter().collect();
        let via: Vec<String> = dati.iter().filter(|e| e.layer == layer && dati_su(e, &refs)).map(|e| e.handle.clone()).collect();
        if !via.is_empty() {
            esegui(s, "erase", json!({"handles": via}))?;
        }
        let verticale = g.iter().find(|f| su(f, *q)).is_some_and(|f| (f.a.0 - f.b.0).abs() < TOLL);
        let (p, rot) = if verticale { ((q.0 + 3.0, q.1), 90.0) } else { ((q.0, q.1 - 3.0), 0.0) };
        let h = inserisci_su_layer(
            s,
            BLOCCO_DATI,
            p,
            &json!({"TESTO": c.etichetta(), "SEZIONE": c.sezione, "COLORE": c.colore, "CODICE": c.codice, "COSTRUTTORE": c.costruttore}),
            &layer,
        )?;
        if rot != 0.0 {
            esegui(s, "rotate", json!({"handles": [h], "base": [p.0, p.1], "angle": rot}))?;
        }
    }
    Ok(reti.len())
}

/// Toglie l'indicazione del conduttore dalle reti toccate: tornano al conduttore predefinito.
pub fn togli(s: &mut Session, at: Option<P>, fence: Option<(P, P)>) -> Result<usize> {
    let (layer, reti) = toccate(s, at, fence)?;
    let dati = blocchi_inseriti(s, BLOCCO_DATI)?;
    let mut via = Vec::new();
    for (g, _) in &reti {
        let refs: Vec<&Segmento> = g.iter().collect();
        via.extend(dati.iter().filter(|e| e.layer == layer && dati_su(e, &refs)).map(|e| e.handle.clone()));
    }
    if !via.is_empty() {
        esegui(s, "erase", json!({"handles": via}))?;
    }
    Ok(via.len())
}

fn conduttore_param(p: &Value) -> Conduttore {
    let base = impostazioni::correnti().conduttore;
    let g = |k: &str, d: &str| testo_param(p, k).unwrap_or_else(|| d.to_string());
    Conduttore { sezione: g("sezione", &base.sezione), colore: g("colore", &base.colore), codice: g("codice", &base.codice), costruttore: g("costruttore", &base.costruttore) }
}

fn run_conduttore(s: &mut Session, p: &Value) -> Result<Value> {
    let at = punto_param(p, "at");
    let fence = punto_param(p, "da").zip(punto_param(p, "a"));
    if at.is_none() && fence.is_none() {
        return Err(errore("Indicare il punto sul filo (at) o la fence (da, a)"));
    }
    let togli_ = p.get("togli").and_then(Value::as_bool).unwrap_or(false);
    let c = conduttore_param(p);
    in_gruppo(s, "Conduttore", |s| {
        if togli_ {
            let n = togli(s, at, fence)?;
            return Ok(json!({"reti": n, "message": format!("Tolte {n} indicazioni di conduttore")}));
        }
        let n = specializza(s, at, fence, &c)?;
        Ok(json!({"reti": n, "message": format!("Conduttore {} {} su {n} fili", c.etichetta(), c.codice)}))
    })
}

fn run_cablaggio(s: &mut Session, p: &Value) -> Result<Value> {
    let r = reti(s, &[])?;
    let righe = righe_cablaggio(&cablaggio(&r));
    if let Some(f) = testo_param(p, "file") {
        morsetti::scrivi_csv(&f, &INTESTAZIONE_CABLAGGIO, &righe)?;
    }
    Ok(json!({"intestazione": INTESTAZIONE_CABLAGGIO, "righe": righe, "message": format!("Tabella di cablaggio: {} collegamenti", righe.len())}))
}

fn run_reti(s: &mut Session, p: &Value) -> Result<Value> {
    let fogli_da: Vec<u32> = p.get("fogli").and_then(Value::as_array).map(|a| a.iter().filter_map(|v| v.as_u64().and_then(|n| u32::try_from(n).ok())).collect()).unwrap_or_default();
    let r = reti(s, &fogli_da)?;
    let v: Vec<Value> = r
        .iter()
        .map(|r| {
            json!({
                "foglio": r.foglio,
                "nome": r.nome,
                "posizione": r.fili.first().map(|f| squadratura::riferimento(r.foglio, f.a.0, f.a.1)),
                "capi": r.capi.iter().map(Capo::testo).collect::<Vec<_>>(),
                "conduttore": {"sezione": r.conduttore.sezione, "colore": r.conduttore.colore, "codice": r.conduttore.codice},
                "specializzato": r.specializzato,
            })
        })
        .collect();
    Ok(json!({"reti": v, "message": format!("{} reti", v.len())}))
}

fn fabbrica_conduttore(_s: &Session) -> Result<Box<dyn cadcraft_engine::Interactive>> {
    let c = impostazioni::correnti().conduttore;
    Ok(Box::new(crate::util::PuntoOFence::nuovo(
        "conduttore",
        "Filo a cui dare il conduttore in uso",
        "conduttore",
        json!({"sezione": c.sezione, "colore": c.colore, "codice": c.codice, "costruttore": c.costruttore}),
    )))
}

pub fn comandi() -> Vec<CommandSpec> {
    let spec = |id, label, menu, params, run| CommandSpec { id, label, menu, shortcut: None, aliases: &[], params, enabled: has_doc, run, undoable: false, interactive: None, transparent: false };
    vec![
        CommandSpec {
            interactive: Some(fabbrica_conduttore),
            ..spec(
                "conduttore",
                "Materiale dei Conduttori",
                &["Elettrico", "Materiale dei Conduttori"],
                "{at: [x,y] | da + a (fence), sezione?, colore?, codice?, costruttore? (default il conduttore in uso), togli?: bool} → {reti}",
                run_conduttore as cadcraft_engine::cmd::Run,
            )
        },
        spec("tabellacablaggio", "Tabella di cablaggio", &["Progetto", "Tabella di cablaggio (CSV)"], "{file?: percorso .csv} → {intestazione, righe}", run_cablaggio),
        spec("reti", "Reti dei fili", &[], "{fogli?: [numeri]} → {reti: [{foglio, nome, posizione, capi, conduttore, specializzato}]}", run_reti),
    ]
}
