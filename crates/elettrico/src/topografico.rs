//! Gestore topografico: i componenti del quadro da montare nel lay-out, con lo stato di ognuno
//! (posato, da posare, rimasto nel lay-out senza componente nello schema), la posa del singolo
//! ingombro in un punto (agganciato alla guida DIN più vicina) e l'allineamento allo schema.
//!
//! Ogni ingombro ricorda il suo componente (l'handle nell'attributo `RIF`): se nello schema il
//! componente cambia sigla o codice, il lay-out si aggiorna; se sparisce, l'ingombro resta e
//! viene segnalato. Gli ingombri dei lay-out più vecchi, senza `RIF`, si riconoscono dalla sigla.

use cadcraft_engine::cmd::has_doc;
use cadcraft_engine::{Accept, CommandSpec, Input, Interactive, Prompt, Result, Session, Step};
use serde_json::{Value, json};

use crate::layout::{self, BLOCCO_GUIDA, BLOCCO_PIASTRA, PREFISSO_INGOMBRO};
use crate::util::{Inserito, blocchi_inseriti, errore, esegui, imposta_attributi, in_gruppo, inserisci_su_layer, layer_corrente, punto_param, testo_param};
use crate::{fogli, impostazioni, siglatura};

type P = (f64, f64);

/// Lo stato di un elemento del topografico.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stato {
    Posato,
    DaPosare,
    /// Un ingombro nel lay-out senza più il suo componente nello schema.
    Orfano,
}

impl Stato {
    pub fn testo(self) -> &'static str {
        match self {
            Stato::Posato => "posato",
            Stato::DaPosare => "da posare",
            Stato::Orfano => "non nello schema",
        }
    }
}

/// Un elemento del topografico.
#[derive(Clone, Debug)]
pub struct Voce {
    pub sigla: String,
    pub descrizione: String,
    pub codice: String,
    pub larghezza: f64,
    pub altezza: f64,
    pub stimata: bool,
    pub stato: Stato,
    /// L'ingombro nel lay-out, se c'è: handle, foglio, posizione.
    pub ingombro: Option<(String, u32, P)>,
}

/// La piastra del lay-out di un quadro: layer, origine (angolo in basso a sinistra) e scala.
#[derive(Clone, Debug)]
pub struct Piastra {
    pub layer: String,
    pub origine: P,
    pub scala: f64,
}

fn piastra_di(piastre: &[Inserito], quadro: &str) -> Option<Piastra> {
    piastre.iter().find(|b| b.attributo("QUADRO").trim().eq_ignore_ascii_case(quadro.trim())).map(|b| Piastra {
        layer: b.layer.clone(),
        origine: b.pos,
        scala: b.attributo("SCALA").trim().replace(',', ".").parse::<f64>().ok().filter(|k| *k > 0.0).unwrap_or(1.0),
    })
}

/// La piastra del lay-out del quadro.
pub fn piastra(s: &mut Session, quadro: &str) -> Result<Option<Piastra>> {
    Ok(piastra_di(&blocchi_inseriti(s, BLOCCO_PIASTRA)?, quadro))
}

/// Il quadro del lay-out disegnato sul foglio corrente, se c'è.
pub fn quadro_del_foglio(s: &mut Session) -> Result<Option<String>> {
    let layer = layer_corrente(s)?;
    Ok(blocchi_inseriti(s, BLOCCO_PIASTRA)?.into_iter().find(|b| b.layer.eq_ignore_ascii_case(&layer)).map(|b| b.attributo("QUADRO").to_string()))
}

fn ingombri_posati(s: &mut Session, quadro: &str) -> Result<Vec<Inserito>> {
    Ok(blocchi_inseriti(s, &format!("{PREFISSO_INGOMBRO}*"))?.into_iter().filter(|b| b.attributo("QUADRO").trim().eq_ignore_ascii_case(quadro.trim())).collect())
}

/// Gli elementi del topografico del quadro: i componenti da montare con il loro stato, poi gli
/// ingombri rimasti senza componente.
pub fn elenco(s: &mut Session, quadro: &str) -> Result<Vec<Voce>> {
    let da_montare = layout::ingombri(s, quadro)?;
    let posati = ingombri_posati(s, quadro)?;
    let mut usati = vec![false; posati.len()];
    let mut v = Vec::new();
    for i in &da_montare {
        // prima per il componente ricordato, poi per sigla
        let k = posati
            .iter()
            .enumerate()
            .position(|(k, b)| !usati[k] && !i.rif.is_empty() && b.attributo("RIF").eq_ignore_ascii_case(&i.rif))
            .or_else(|| posati.iter().enumerate().position(|(k, b)| !usati[k] && b.attributo("RIF").is_empty() && b.attributo("SIGLA").trim().eq_ignore_ascii_case(i.sigla.trim())));
        let ingombro = k.map(|k| {
            usati[k] = true;
            let b = &posati[k];
            (b.handle.clone(), fogli::numero_da_layer(&b.layer).unwrap_or(0), b.pos)
        });
        v.push(Voce {
            sigla: i.sigla.clone(),
            descrizione: i.descrizione.clone(),
            codice: i.codice.clone(),
            larghezza: i.larghezza,
            altezza: i.altezza,
            stimata: i.stimata,
            stato: if ingombro.is_some() { Stato::Posato } else { Stato::DaPosare },
            ingombro,
        });
    }
    for (b, _) in posati.iter().zip(&usati).filter(|(_, u)| !**u) {
        let n = |t: &str| b.attributo(t).trim().parse::<f64>().unwrap_or(0.0);
        v.push(Voce {
            sigla: b.attributo("SIGLA").to_string(),
            descrizione: String::new(),
            codice: b.attributo("CODICE").to_string(),
            larghezza: n("LARGHEZZA"),
            altezza: n("ALTEZZA"),
            stimata: false,
            stato: Stato::Orfano,
            ingombro: Some((b.handle.clone(), fogli::numero_da_layer(&b.layer).unwrap_or(0), b.pos)),
        });
    }
    Ok(v)
}

/// Il punto `at` agganciato alla guida DIN del layer più vicina (ordinata al centro della guida),
/// se cade sulla guida o a meno di una guida di distanza.
fn aggancia(s: &mut Session, layer: &str, k: f64, at: P) -> Result<P> {
    let mut migliore: Option<(f64, f64)> = None;
    for g in blocchi_inseriti(s, BLOCCO_GUIDA)?.into_iter().filter(|g| g.layer.eq_ignore_ascii_case(layer)) {
        let lunghezza = g.attributo("LUNGHEZZA").trim().parse::<f64>().unwrap_or(0.0) / k;
        let (x0, y) = g.pos;
        let d = (at.1 - y).abs();
        if at.0 >= x0 - 1.0 && at.0 <= x0 + lunghezza + 1.0 && d <= 35.0 / k && migliore.is_none_or(|(dm, _)| d < dm) {
            migliore = Some((d, y));
        }
    }
    Ok(migliore.map_or(at, |(_, y)| (at.0, y)))
}

/// Posa l'ingombro del componente `sigla` del quadro nel punto `at` del suo lay-out.
pub fn posa(s: &mut Session, quadro: &str, sigla: &str, at: P) -> Result<Value> {
    let p = piastra(s, quadro)?.ok_or_else(|| errore(format!("Il quadro {quadro} non ha ancora il lay-out: disegnalo con Layout Quadro")))?;
    let i = layout::ingombri(s, quadro)?
        .into_iter()
        .find(|i| i.sigla.trim().eq_ignore_ascii_case(sigla.trim()))
        .ok_or_else(|| errore(format!("{sigla} non è un componente da montare nel quadro {quadro}")))?;
    if elenco(s, quadro)?.iter().any(|v| v.sigla.eq_ignore_ascii_case(&i.sigla) && v.stato == Stato::Posato) {
        return Err(errore(format!("{} è già nel lay-out", i.sigla)));
    }
    let at = aggancia(s, &p.layer, p.scala, at)?;
    let nome = layout::blocco_ingombro(s, i.larghezza, i.altezza, p.scala)?;
    let m = layout::misura_testo;
    let h = inserisci_su_layer(s, &nome, at, &json!({"SIGLA": i.sigla, "QUADRO": quadro, "CODICE": i.codice, "LARGHEZZA": m(i.larghezza), "ALTEZZA": m(i.altezza)}), &p.layer)?;
    if !i.rif.is_empty() {
        imposta_attributi(s, &h, &[("RIF", i.rif.as_str())])?;
    }
    Ok(json!({"handle": h, "sigla": i.sigla, "at": [at.0, at.1], "message": format!("{} nel lay-out del quadro {quadro}", i.sigla)}))
}

/// Posa nel lay-out già disegnato i componenti che mancano, in coda sulle guide dove c'è posto
/// (dall'alto); quelli già posati non si muovono. Restituisce (posati, quelli che non entrano).
pub fn completa(s: &mut Session, quadro: &str) -> Result<(usize, Vec<String>)> {
    let p = piastra(s, quadro)?.ok_or_else(|| errore(format!("Il quadro {quadro} non ha ancora il lay-out")))?;
    let k = p.scala;
    // le guide del foglio: (x iniziale, y, x finale), dall'alto
    let mut guide: Vec<(f64, f64, f64)> = blocchi_inseriti(s, BLOCCO_GUIDA)?
        .into_iter()
        .filter(|g| g.layer.eq_ignore_ascii_case(&p.layer))
        .map(|g| (g.pos.0, g.pos.1, g.pos.0 + g.attributo("LUNGHEZZA").trim().parse::<f64>().unwrap_or(0.0) / k))
        .collect();
    guide.sort_by(|a, b| b.1.total_cmp(&a.1));
    // dove finiscono gli ingombri già posati su ogni guida
    let mut fine: Vec<f64> = guide.iter().map(|g| g.0).collect();
    for b in blocchi_inseriti(s, &format!("{PREFISSO_INGOMBRO}*"))?.into_iter().filter(|b| b.layer.eq_ignore_ascii_case(&p.layer)) {
        let w = b.attributo("LARGHEZZA").trim().parse::<f64>().unwrap_or(0.0) / k;
        if let Some(i) = guide.iter().position(|g| (g.1 - b.pos.1).abs() < 0.5) {
            fine[i] = fine[i].max(b.pos.0 + w);
        }
    }
    let distacco = 2.0 / k;
    let (mut posati, mut fuori) = (0, Vec::new());
    for v in elenco(s, quadro)?.into_iter().filter(|v| v.stato == Stato::DaPosare) {
        let w = v.larghezza / k;
        match guide.iter().enumerate().find(|(i, g)| fine[*i] + distacco + w <= g.2 + 1e-6) {
            Some((i, g)) => {
                let x = fine[i] + if fine[i] > g.0 { distacco } else { 0.0 };
                let y = g.1;
                posa(s, quadro, &v.sigla, (x, y))?;
                fine[i] = x + w;
                posati += 1;
            }
            None => fuori.push(v.sigla),
        }
    }
    Ok((posati, fuori))
}

/// Allinea gli ingombri di tutti i lay-out allo schema: sigla e codice del componente
/// ricordato. Con `togli_orfani` cancella quelli rimasti senza componente. Restituisce
/// (aggiornati, orfani).
pub fn aggiorna(s: &mut Session, togli_orfani: bool) -> Result<(usize, usize)> {
    let ingombri = blocchi_inseriti(s, &format!("{PREFISSO_INGOMBRO}*"))?;
    if ingombri.is_empty() {
        return Ok((0, 0));
    }
    let madri: Vec<siglatura::Componente> = siglatura::componenti(s)?.into_iter().filter(siglatura::Componente::madre).collect();
    let morsettiere = crate::morsettiere::nomi(s)?;
    let (mut aggiornati, mut orfani) = (0, Vec::new());
    for b in &ingombri {
        let rif = b.attributo("RIF").trim();
        let sigla = b.attributo("SIGLA").trim();
        if rif.is_empty() {
            let presente = madri.iter().any(|c| c.nome.eq_ignore_ascii_case(sigla)) || morsettiere.iter().any(|m| m.eq_ignore_ascii_case(sigla));
            if !presente {
                orfani.push(b.handle.clone());
            }
            continue;
        }
        match madri.iter().find(|c| c.blocco.handle.eq_ignore_ascii_case(rif)) {
            Some(c) => {
                let codice = c.blocco.attributo("TIPO");
                if c.nome != sigla || codice != b.attributo("CODICE") {
                    imposta_attributi(s, &b.handle, &[("SIGLA", c.nome.as_str()), ("CODICE", codice)])?;
                    aggiornati += 1;
                }
            }
            None => orfani.push(b.handle.clone()),
        }
    }
    let n = orfani.len();
    if togli_orfani && !orfani.is_empty() {
        esegui(s, "erase", json!({"handles": orfani}))?;
    }
    Ok((aggiornati, n))
}

fn quadro_param(s: &mut Session, p: &Value) -> Result<String> {
    if let Some(q) = testo_param(p, "quadro").filter(|q| !q.trim().is_empty()) {
        return Ok(q);
    }
    Ok(quadro_del_foglio(s)?.or_else(impostazioni::quadro_corrente).unwrap_or_default())
}

fn voce_json(v: &Voce) -> Value {
    json!({
        "sigla": v.sigla, "descrizione": v.descrizione, "codice": v.codice, "larghezza": v.larghezza, "altezza": v.altezza,
        "stimata": v.stimata, "stato": v.stato.testo(),
        "handle": v.ingombro.as_ref().map(|i| i.0.clone()), "foglio": v.ingombro.as_ref().map(|i| i.1), "x": v.ingombro.as_ref().map(|i| i.2.0), "y": v.ingombro.as_ref().map(|i| i.2.1),
    })
}

fn run_elenco(s: &mut Session, p: &Value) -> Result<Value> {
    let quadro = quadro_param(s, p)?;
    let v = elenco(s, &quadro)?;
    let conta = |st: Stato| v.iter().filter(|x| x.stato == st).count();
    let lay = piastra(s, &quadro)?;
    Ok(json!({
        "quadro": quadro,
        "foglio": lay.as_ref().and_then(|p| fogli::numero_da_layer(&p.layer)),
        "elementi": v.iter().map(voce_json).collect::<Vec<_>>(),
        "message": format!("Quadro {quadro}: {} posati, {} da posare, {} non nello schema", conta(Stato::Posato), conta(Stato::DaPosare), conta(Stato::Orfano)),
    }))
}

fn run_posa(s: &mut Session, p: &Value) -> Result<Value> {
    let quadro = quadro_param(s, p)?;
    let sigla = testo_param(p, "sigla").filter(|t| !t.trim().is_empty()).ok_or_else(|| errore("Indicare il componente (sigla)"))?;
    let at = punto_param(p, "at").ok_or_else(|| errore("Indicare il punto (at)"))?;
    in_gruppo(s, "Posa nel lay-out", |s| posa(s, &quadro, &sigla, at))
}

fn run_aggiorna(s: &mut Session, p: &Value) -> Result<Value> {
    let togli = p.get("togliOrfani").and_then(Value::as_bool).unwrap_or(false);
    let (a, o) = in_gruppo(s, "Aggiorna topografico", |s| aggiorna(s, togli))?;
    let msg = match (a, o, togli) {
        (0, 0, _) => "Lay-out allineato allo schema".to_string(),
        (a, o, true) => format!("{a} ingombri aggiornati, {o} senza componente tolti"),
        (a, o, false) => format!("{a} ingombri aggiornati, {o} senza componente nello schema"),
    };
    Ok(json!({"aggiornati": a, "orfani": o, "message": msg}))
}

/// Posa interattiva: il componente (Invio = il prossimo da posare) e il punto, poi il successivo.
struct PosaInterattiva {
    /// Il componente scelto, in attesa del punto.
    sigla: Option<String>,
    prossimo: Option<String>,
}

impl PosaInterattiva {
    fn prossimo(s: &mut Session) -> Option<String> {
        let quadro = quadro_del_foglio(s).ok().flatten()?;
        elenco(s, &quadro).ok()?.into_iter().find(|v| v.stato == Stato::DaPosare).map(|v| v.sigla)
    }
}

impl Interactive for PosaInterattiva {
    fn name(&self) -> &'static str {
        "TOPOGRAFICO.POSA"
    }
    fn prompt(&self, _s: &Session) -> Prompt {
        match &self.sigla {
            Some(sigla) => Prompt::new(format!("Punto di {sigla} nel lay-out"), Accept::POINT),
            None => Prompt { default: self.prossimo.clone(), ..Prompt::new("Componente da posare (Invio: il prossimo da posare)", Accept::TEXT) },
        }
    }
    fn input(&mut self, s: &mut Session, i: Input) -> Result<Step> {
        match (i, &self.sigla) {
            (Input::Cancel, _) => Ok(Step::Cancel),
            (Input::Point(q), Some(sigla)) => {
                let quadro = quadro_del_foglio(s)?.ok_or_else(|| errore("Vai sul foglio del lay-out del quadro"))?;
                match in_gruppo(s, "Posa nel lay-out", |s| posa(s, &quadro, sigla, (q.x, q.y))) {
                    Ok(r) => s.echo(r["message"].as_str().unwrap_or_default().to_string()),
                    Err(e) => s.echo(e.to_string()),
                }
                self.sigla = None;
                self.prossimo = Self::prossimo(s);
                Ok(Step::Continue)
            }
            (Input::Text(t) | Input::Keyword(t), None) if !t.trim().is_empty() => {
                self.sigla = Some(t.trim().to_string());
                Ok(Step::Continue)
            }
            (Input::Enter | Input::Text(_), None) => match self.prossimo.clone().or_else(|| Self::prossimo(s)) {
                Some(p) => {
                    self.sigla = Some(p);
                    Ok(Step::Continue)
                }
                None => {
                    s.echo("Tutti i componenti del quadro sono nel lay-out".to_string());
                    Ok(Step::Done)
                }
            },
            (Input::Enter, Some(_)) => {
                self.sigla = None;
                Ok(Step::Continue)
            }
            _ => Ok(Step::Continue),
        }
    }
}

fn fabbrica(_s: &Session) -> Result<Box<dyn Interactive>> {
    Ok(Box::new(PosaInterattiva { sigla: None, prossimo: None }))
}

pub fn comandi() -> Vec<CommandSpec> {
    let spec = |id, label, menu, params, run| CommandSpec { id, label, menu, shortcut: None, aliases: &[], params, enabled: has_doc, run, undoable: false, interactive: None, transparent: false };
    vec![
        spec(
            "topografico",
            "Gestore Topografico",
            &["Quadro", "Gestore Topografico"],
            "{quadro? (default: quello del lay-out sul foglio corrente)} → {elementi: [{sigla, descrizione, larghezza, altezza, stimata, stato: posato|da posare|non nello schema, handle?, foglio?}]}",
            run_elenco as cadcraft_engine::cmd::Run,
        ),
        CommandSpec {
            interactive: Some(fabbrica),
            ..spec("topografico.posa", "Posa nel Lay-out", &["Quadro", "Posa nel Lay-out"], "{sigla, at: [x,y], quadro?} — l'ingombro del componente nel lay-out, agganciato alla guida DIN più vicina", run_posa)
        },
        spec("topografico.aggiorna", "Aggiorna Topografico", &["Quadro", "Aggiorna Topografico"], "{togliOrfani?: bool} — sigle e codici degli ingombri dallo schema → {aggiornati, orfani}", run_aggiorna),
    ]
}
