//! Cavi: definizione dei cavi del progetto (sigla, formazione, codice, vene), posa delle vene sui
//! fili con i segnalini, elenco dei cavi e tabella dei conduttori.
//!
//! Come nei CAD elettrici diffusi il cavo si vede sullo schema come un segnalino obliquo su ogni
//! filo che ne usa una vena, con il nome della vena, e la sigla con la formazione accanto
//! all'ultimo del gruppo (`-W03 24G1`). La definizione del cavo sta nel disegno in un blocco
//! senza grafica (`CE_CAVO_DEF`) sul foglio 0, così il progetto resta un solo file.

use cadcraft_engine::cmd::has_doc;
use cadcraft_engine::{CommandSpec, Result, Session};
use serde_json::{Value, json};

use crate::collegamenti::{self, TipoCapo};
use crate::numerazione::{self, su};
use crate::util::{Inserito, blocchi_definiti, blocchi_inseriti, blocco, colore, errore, esegui, handles, in_gruppo, inserisci_su_layer, linea, punto_param, su_layer_zero, testo_param};
use crate::{fili, fogli, morsetti, squadratura};

pub const BLOCCO_DEF: &str = "CE_CAVO_DEF";
pub const BLOCCO_SEGNALINO: &str = "CE_SEGN_CAVO";
const TOLL: f64 = 0.01;

type P = (f64, f64);

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Cavo {
    pub sigla: String,
    pub funzione: String,
    pub costruttore: String,
    pub codice: String,
    /// Formazione: `4G1,5`, `3x2.5`, `24G1`…
    pub formazione: String,
    pub vene: Vec<String>,
    pub da_quadro: String,
    pub a_quadro: String,
    /// Lunghezza in metri (testo).
    pub lunghezza: String,
    pub handle: String,
}

/// Le vene proposte per una formazione: colori per i cavi fino a 5 conduttori, numeri oltre;
/// con la `G` l'ultimo è il giallo-verde.
pub fn vene_di(formazione: &str) -> Vec<String> {
    let f = formazione.trim().to_ascii_uppercase();
    let (n, terra) = match f.find(['G', 'X']) {
        Some(i) => (f.get(..i).and_then(|n| n.trim().parse::<usize>().ok()).unwrap_or(0), f.as_bytes().get(i) == Some(&b'G')),
        None => (f.parse::<usize>().unwrap_or(0), false),
    };
    let colori: Vec<&str> = match (n, terra) {
        (2, false) => vec!["BN", "BU"],
        (3, false) => vec!["BN", "BK", "GY"],
        (4, false) => vec!["BN", "BK", "GY", "BU"],
        (5, false) => vec!["BN", "BK", "GY", "BU", "BK"],
        (3, true) => vec!["BN", "BU", "GNYE"],
        (4, true) => vec!["BN", "BK", "GY", "GNYE"],
        (5, true) => vec!["BN", "BK", "GY", "BU", "GNYE"],
        _ => Vec::new(),
    };
    if !colori.is_empty() {
        return colori.into_iter().map(str::to_string).collect();
    }
    let mut v: Vec<String> = (1..=n.saturating_sub(usize::from(terra))).map(|k| k.to_string()).collect();
    if terra && n > 0 {
        v.push("GNYE".into());
    }
    v
}

fn crea_blocchi(s: &mut Session) -> Result<()> {
    let definiti = blocchi_definiti(s)?;
    let manca = |n: &str| !definiti.iter().any(|b| b.eq_ignore_ascii_case(n));
    su_layer_zero(s, |s| {
        let attr = |s: &mut Session, h: &mut Vec<String>, tag: &str, at: P, alt: f64, nascosto: bool| -> Result<()> {
            h.extend(handles(&esegui(s, "attdef", json!({"tag": tag, "prompt": tag, "default": "", "at": [at.0, at.1], "height": alt, "invisible": nascosto}))?));
            Ok(())
        };
        if manca(BLOCCO_DEF) {
            let mut h = Vec::new();
            for tag in ["SIGLA", "FUNZIONE", "COSTRUTTORE", "CODICE", "FORMAZIONE", "VENE", "DA_QUADRO", "A_QUADRO", "LUNGHEZZA"] {
                attr(s, &mut h, tag, (0.0, 0.0), 1.0, true)?;
            }
            blocco(s, BLOCCO_DEF, h, "Definizione di un cavo")?;
        }
        if manca(BLOCCO_SEGNALINO) {
            let mut h = Vec::new();
            colore(s, "7")?;
            // segnalino obliquo che attraversa il filo verticale
            linea(s, &mut h, &[(-1.5, -1.5), (1.5, 1.5)], false)?;
            colore(s, "BYLAYER")?;
            attr(s, &mut h, "VENA", (1.6, -2.4), 1.8, false)?;
            attr(s, &mut h, "ETICHETTA", (3.0, -6.0), 2.5, false)?;
            attr(s, &mut h, "CAVO", (0.0, 0.0), 1.0, true)?;
            attr(s, &mut h, "FORMAZIONE", (0.0, 0.0), 1.0, true)?;
            blocco(s, BLOCCO_SEGNALINO, h, "Segnalino di una vena di cavo")?;
        }
        Ok(())
    })
}

fn da_blocco(b: &Inserito) -> Cavo {
    let v = |t: &str| b.attributo(t).to_string();
    Cavo {
        sigla: v("SIGLA"),
        funzione: v("FUNZIONE"),
        costruttore: v("COSTRUTTORE"),
        codice: v("CODICE"),
        formazione: v("FORMAZIONE"),
        vene: b.attributo("VENE").split(';').map(str::trim).filter(|x| !x.is_empty()).map(str::to_string).collect(),
        da_quadro: v("DA_QUADRO"),
        a_quadro: v("A_QUADRO"),
        lunghezza: v("LUNGHEZZA"),
        handle: b.handle.clone(),
    }
}

/// I cavi definiti nel disegno, per sigla.
pub fn cavi(s: &mut Session) -> Result<Vec<Cavo>> {
    let mut v: Vec<Cavo> = blocchi_inseriti(s, BLOCCO_DEF)?.iter().map(da_blocco).collect();
    v.sort_by(|a, b| a.sigla.cmp(&b.sigla));
    Ok(v)
}

/// Una vena posata: cavo, vena, segnalino, foglio e posizione.
#[derive(Clone, Debug, PartialEq)]
pub struct Posa {
    pub cavo: String,
    pub vena: String,
    pub handle: String,
    pub foglio: u32,
    pub pos: P,
}

pub fn pose(s: &mut Session) -> Result<Vec<Posa>> {
    Ok(blocchi_inseriti(s, BLOCCO_SEGNALINO)?
        .into_iter()
        .map(|b| Posa { cavo: b.attributo("CAVO").into(), vena: b.attributo("VENA").into(), foglio: fogli::numero_da_layer(&b.layer).unwrap_or(0), pos: b.pos, handle: b.handle })
        .collect())
}

fn json_cavo(c: &Cavo, usate: &[String]) -> Value {
    json!({
        "sigla": c.sigla, "funzione": c.funzione, "costruttore": c.costruttore, "codice": c.codice, "formazione": c.formazione,
        "vene": c.vene, "usate": usate, "daQuadro": c.da_quadro, "aQuadro": c.a_quadro, "lunghezza": c.lunghezza,
    })
}

/// La prossima sigla libera `-W01`, `-W02`…
fn prossima_sigla(esistenti: &[Cavo]) -> String {
    let n = esistenti.iter().filter_map(|c| c.sigla.trim_start_matches('-').strip_prefix('W').and_then(|n| n.parse::<u32>().ok())).max().unwrap_or(0) + 1;
    format!("-W{n:02}")
}

/// Definisce (o ridefinisce) un cavo. Restituisce la sigla.
pub fn definisci(s: &mut Session, p: &Value) -> Result<String> {
    crea_blocchi(s)?;
    let esistenti = cavi(s)?;
    let sigla = testo_param(p, "sigla").map(|t| t.trim().to_string()).filter(|t| !t.is_empty()).unwrap_or_else(|| prossima_sigla(&esistenti));
    let vecchio = esistenti.iter().find(|c| c.sigla.eq_ignore_ascii_case(&sigla));
    let g = |k: &str, d: &str| testo_param(p, k).unwrap_or_else(|| d.to_string());
    let base = vecchio.cloned().unwrap_or_default();
    let formazione = g("formazione", &base.formazione);
    let vene: Vec<String> = match p.get("vene") {
        Some(Value::Array(a)) => a.iter().filter_map(Value::as_str).map(str::to_string).collect(),
        Some(Value::String(t)) => t.split([';', ',', ' ']).filter(|x| !x.is_empty()).map(str::to_string).collect(),
        _ if vecchio.is_some_and(|v| v.formazione == formazione) => base.vene.clone(),
        _ => vene_di(&formazione),
    };
    if vene.is_empty() {
        return Err(errore(format!("Formazione non riconosciuta: {formazione:?} (es. 4G1,5 o 3x2.5); indicare le vene")));
    }
    let attr = json!({
        "SIGLA": sigla, "FUNZIONE": g("funzione", &base.funzione), "COSTRUTTORE": g("costruttore", &base.costruttore), "CODICE": g("codice", &base.codice),
        "FORMAZIONE": formazione, "VENE": vene.join(";"), "DA_QUADRO": g("daQuadro", &base.da_quadro), "A_QUADRO": g("aQuadro", &base.a_quadro), "LUNGHEZZA": g("lunghezza", &base.lunghezza),
    });
    match vecchio {
        Some(v) => {
            esegui(s, "attedit", json!({"handle": v.handle, "values": attr}))?;
        }
        None => {
            inserisci_su_layer(s, BLOCCO_DEF, (0.0, 0.0), &attr, "0")?;
        }
    }
    Ok(sigla)
}

/// I punti dei fili toccati (punto o fence) sul foglio corrente, con la direzione del filo.
fn punti_toccati(s: &mut Session, at: Option<P>, fence: Option<(P, P)>) -> Result<(String, Vec<(P, bool)>)> {
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
    Ok((layer, punti.into_iter().map(|q| (q, segmenti.iter().find(|f| su(f, q)).is_some_and(|f| (f.a.0 - f.b.0).abs() < TOLL))).collect()))
}

/// Posa le vene del cavo `sigla` sui fili toccati: la vena data, o la prima libera (il
/// giallo-verde sul filo PE). Restituisce le vene posate.
pub fn posa(s: &mut Session, sigla: &str, at: Option<P>, fence: Option<(P, P)>, vena: Option<&str>) -> Result<Vec<String>> {
    crea_blocchi(s)?;
    let cavo = cavi(s)?.into_iter().find(|c| c.sigla.eq_ignore_ascii_case(sigla)).ok_or_else(|| errore(format!("Il cavo {sigla} non è definito")))?;
    let (layer, punti) = punti_toccati(s, at, fence)?;
    if punti.is_empty() {
        return Err(errore("Nessun filo nel punto indicato"));
    }
    let mut usate: Vec<String> = pose(s)?.into_iter().filter(|p| p.cavo.eq_ignore_ascii_case(&cavo.sigla)).map(|p| p.vena).collect();
    // il nome della rete di ogni punto (per il giallo-verde sul PE)
    let n = fogli::numero_da_layer(&layer).unwrap_or(0);
    let reti = collegamenti::reti(s, &[n])?;
    let mut posate = Vec::new();
    let mut handles_posati = Vec::new();
    for (k, (q, verticale)) in punti.iter().enumerate() {
        let rete = reti.iter().find(|r| r.fili.iter().any(|f| su(f, *q)));
        let pe = rete.is_some_and(|r| r.nome.eq_ignore_ascii_case("PE"));
        let scelta = match (vena, k) {
            (Some(v), 0) => Some(v.to_string()),
            _ => {
                let libera = |x: &&String| !usate.contains(x);
                if pe { cavo.vene.iter().find(|v| v.as_str() == "GNYE" && libera(v)) } else { cavo.vene.iter().filter(|v| v.as_str() != "GNYE").find(libera) }.cloned().or_else(|| cavo.vene.iter().find(|v| !usate.contains(v)).cloned())
            }
        };
        let Some(v) = scelta else { return Err(errore(format!("Il cavo {} non ha più vene libere", cavo.sigla))) };
        let h = inserisci_su_layer(s, BLOCCO_SEGNALINO, *q, &json!({"CAVO": cavo.sigla, "VENA": v, "FORMAZIONE": cavo.formazione, "ETICHETTA": ""}), &layer)?;
        if !verticale {
            esegui(s, "rotate", json!({"handles": [h], "base": [q.0, q.1], "angle": 90.0}))?;
        }
        usate.push(v.clone());
        posate.push(v);
        handles_posati.push(h);
    }
    // sigla e formazione accanto all'ultimo segnalino del gruppo
    if let Some(h) = handles_posati.last() {
        esegui(s, "attedit", json!({"handle": h, "values": {"ETICHETTA": format!("{} {}", cavo.sigla, cavo.formazione)}}))?;
    }
    Ok(posate)
}

/// Toglie i segnalini dai fili toccati.
pub fn togli(s: &mut Session, at: Option<P>, fence: Option<(P, P)>) -> Result<usize> {
    let (layer, punti) = punti_toccati(s, at, fence)?;
    let segmenti = fili::fili(s, &layer)?;
    let gruppi = numerazione::reti(&segmenti);
    let reti_toccate: Vec<&Vec<usize>> = gruppi.iter().filter(|g| punti.iter().any(|(q, _)| g.iter().any(|&i| segmenti.get(i).is_some_and(|f| su(f, *q))))).collect();
    let via: Vec<String> = pose(s)?
        .into_iter()
        .filter(|p| fogli::nome_layer(p.foglio) == layer && reti_toccate.iter().any(|g| g.iter().any(|&i| segmenti.get(i).is_some_and(|f| su(f, p.pos)))))
        .map(|p| p.handle)
        .collect();
    if !via.is_empty() {
        esegui(s, "erase", json!({"handles": via}))?;
    }
    Ok(via.len())
}

/// Una vena con il filo e i capi collegati.
#[derive(Clone, Debug, PartialEq)]
pub struct Conduttore {
    pub cavo: String,
    pub vena: String,
    pub filo: String,
    pub da: String,
    pub a: String,
    pub posizione: String,
}

/// Le vene posate di tutti i cavi, con il filo e i capi della rete (quelli sopra e sotto il
/// segnalino).
pub fn conduttori(s: &mut Session) -> Result<Vec<Conduttore>> {
    let reti = collegamenti::reti(s, &[])?;
    let mut v = Vec::new();
    for p in pose(s)? {
        let rete = reti.iter().find(|r| r.foglio == p.foglio && r.fili.iter().any(|f| su(f, p.pos)));
        let lato = |sopra: bool| {
            rete.map(|r| {
                r.capi
                    .iter()
                    .filter(|c| c.tipo != TipoCapo::Rimando && if sopra { c.pos.1 > p.pos.1 + TOLL || (c.pos.1 - p.pos.1).abs() < TOLL && c.pos.0 < p.pos.0 } else { c.pos.1 < p.pos.1 - TOLL || (c.pos.1 - p.pos.1).abs() < TOLL && c.pos.0 > p.pos.0 })
                    .min_by(|a, b| (a.pos.0 - p.pos.0).hypot(a.pos.1 - p.pos.1).total_cmp(&(b.pos.0 - p.pos.0).hypot(b.pos.1 - p.pos.1)))
                    .map(collegamenti::Capo::testo)
                    .unwrap_or_default()
            })
            .unwrap_or_default()
        };
        v.push(Conduttore {
            cavo: p.cavo.clone(),
            vena: p.vena.clone(),
            filo: rete.map(|r| r.nome.clone()).unwrap_or_default(),
            da: lato(true),
            a: lato(false),
            posizione: squadratura::riferimento(p.foglio, p.pos.0, p.pos.1),
        });
    }
    // per cavo, nell'ordine delle vene della sua definizione
    let definiti = cavi(s)?;
    let ordine = |c: &Conduttore| definiti.iter().find(|d| d.sigla.eq_ignore_ascii_case(&c.cavo)).and_then(|d| d.vene.iter().position(|x| *x == c.vena)).unwrap_or(usize::MAX);
    v.sort_by(|a, b| a.cavo.cmp(&b.cavo).then_with(|| ordine(a).cmp(&ordine(b))).then_with(|| a.vena.cmp(&b.vena)));
    Ok(v)
}

// ---------- comandi ----------

fn run_definisci(s: &mut Session, p: &Value) -> Result<Value> {
    in_gruppo(s, "Definisci cavo", |s| {
        let sigla = definisci(s, p)?;
        let c = cavi(s)?.into_iter().find(|c| c.sigla == sigla).unwrap_or_default();
        let mut r = json_cavo(&c, &[]);
        r["message"] = json!(format!("Cavo {sigla} {} ({} vene)", c.formazione, c.vene.len()));
        Ok(r)
    })
}

fn run_elimina(s: &mut Session, p: &Value) -> Result<Value> {
    let sigla = testo_param(p, "sigla").ok_or_else(|| errore("Indicare la sigla del cavo"))?;
    in_gruppo(s, "Elimina cavo", |s| {
        let c = cavi(s)?.into_iter().find(|c| c.sigla.eq_ignore_ascii_case(&sigla)).ok_or_else(|| errore(format!("Il cavo {sigla} non è definito")))?;
        let mut via = vec![c.handle.clone()];
        via.extend(pose(s)?.into_iter().filter(|p| p.cavo.eq_ignore_ascii_case(&sigla)).map(|p| p.handle));
        esegui(s, "erase", json!({"handles": via}))?;
        Ok(json!({"message": format!("Cavo {sigla} eliminato con i suoi segnalini")}))
    })
}

fn run_elenco(s: &mut Session, _p: &Value) -> Result<Value> {
    let tutte = pose(s)?;
    let v: Vec<Value> = cavi(s)?
        .iter()
        .map(|c| {
            let usate: Vec<String> = tutte.iter().filter(|p| p.cavo.eq_ignore_ascii_case(&c.sigla)).map(|p| p.vena.clone()).collect();
            json_cavo(c, &usate)
        })
        .collect();
    let righe: Vec<String> = v.iter().map(|c| format!("{} {} {} ({}/{})", c["sigla"].as_str().unwrap_or(""), c["formazione"].as_str().unwrap_or(""), c["codice"].as_str().unwrap_or(""), c["usate"].as_array().map_or(0, Vec::len), c["vene"].as_array().map_or(0, Vec::len))).collect();
    Ok(json!({"cavi": v, "message": if righe.is_empty() { "Nessun cavo definito".to_string() } else { righe.join("\n") }}))
}

fn run_posa(s: &mut Session, p: &Value) -> Result<Value> {
    let sigla = testo_param(p, "cavo").ok_or_else(|| errore("Indicare il cavo (es. -W01)"))?;
    let at = punto_param(p, "at");
    let fence = punto_param(p, "da").zip(punto_param(p, "a"));
    if at.is_none() && fence.is_none() {
        return Err(errore("Indicare il punto sul filo (at) o la fence (da, a)"));
    }
    let vena = testo_param(p, "vena");
    in_gruppo(s, "Posa cavo", |s| {
        let v = posa(s, &sigla, at, fence, vena.as_deref())?;
        Ok(json!({"vene": v, "message": format!("Cavo {sigla}: vene {}", v.join(", "))}))
    })
}

fn run_togli(s: &mut Session, p: &Value) -> Result<Value> {
    let at = punto_param(p, "at");
    let fence = punto_param(p, "da").zip(punto_param(p, "a"));
    in_gruppo(s, "Togli cavo", |s| {
        let n = togli(s, at, fence)?;
        Ok(json!({"segnalini": n, "message": format!("Tolti {n} segnalini di cavo")}))
    })
}

fn run_conduttori(s: &mut Session, p: &Value) -> Result<Value> {
    let v = conduttori(s)?;
    let righe: Vec<Vec<String>> = v.iter().map(|c| vec![c.cavo.clone(), c.vena.clone(), c.filo.clone(), c.da.clone(), c.a.clone(), c.posizione.clone()]).collect();
    if let Some(f) = testo_param(p, "file") {
        morsetti::scrivi_csv(&f, &["Cavo", "Vena", "Filo", "Da", "A", "Posizione"], &righe)?;
    }
    Ok(json!({"righe": righe, "message": format!("{} vene posate", righe.len())}))
}

/// Il cavo in uso per la prossima posa interattiva (scelto nella finestra dei cavi).
static IN_USO: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);

/// Il cavo che userà la prossima posa interattiva, senza chiederlo.
pub fn usa(sigla: &str) {
    if let Ok(mut g) = IN_USO.lock() {
        *g = Some(sigla.to_string());
    }
}

fn fabbrica_posa(_s: &Session) -> Result<Box<dyn cadcraft_engine::Interactive>> {
    let cavo = IN_USO.lock().ok().and_then(|mut g| g.take());
    Ok(Box::new(SceltaCavo { cavo, fence: None }))
}

/// POSA CAVO interattivo: sigla del cavo, poi fili (punto o fence).
struct SceltaCavo {
    cavo: Option<String>,
    fence: Option<Option<P>>,
}

impl cadcraft_engine::Interactive for SceltaCavo {
    fn name(&self) -> &'static str {
        "cavo.posa"
    }
    fn prompt(&self, _s: &Session) -> cadcraft_engine::Prompt {
        use cadcraft_engine::{Accept, Prompt};
        match (&self.cavo, self.fence) {
            (None, _) => Prompt::new("Sigla del cavo (es. -W01)", Accept::TEXT),
            (Some(c), None) => Prompt::new(format!("Filo per una vena di {c} (Invio per finire)"), Accept::POINT).kw(&["Fence"]),
            (Some(_), Some(None)) => Prompt::new("Primo punto della fence", Accept::POINT),
            (Some(_), Some(Some(p))) => Prompt::new("Secondo punto della fence", Accept::POINT).base(cadcraft_engine::geom::Vec2::new(p.0, p.1)),
        }
    }
    fn input(&mut self, s: &mut Session, i: cadcraft_engine::Input) -> Result<cadcraft_engine::Step> {
        use cadcraft_engine::{Input, Step};
        match i {
            Input::Cancel => Ok(Step::Cancel),
            Input::Enter => Ok(Step::Done),
            Input::Text(t) if self.cavo.is_none() => {
                let t = t.trim().to_ascii_uppercase();
                if !t.is_empty() {
                    self.cavo = Some(if t.starts_with('-') { t } else { format!("-{t}") });
                }
                Ok(Step::Continue)
            }
            Input::Keyword(_) | Input::Text(_) => {
                self.fence = Some(None);
                Ok(Step::Continue)
            }
            Input::Point(q) => {
                let Some(c) = self.cavo.clone() else { return Ok(Step::Continue) };
                let p = match self.fence {
                    Some(None) => {
                        self.fence = Some(Some((q.x, q.y)));
                        return Ok(Step::Continue);
                    }
                    Some(Some(d)) => {
                        self.fence = None;
                        json!({"cavo": c, "da": [d.0, d.1], "a": [q.x, q.y]})
                    }
                    None => json!({"cavo": c, "at": [q.x, q.y]}),
                };
                let r = s.execute("cavo.posa", &p)?;
                if let Some(m) = r.get("message").and_then(Value::as_str) {
                    s.echo(m.to_string());
                }
                Ok(Step::Continue)
            }
            _ => Ok(Step::Continue),
        }
    }
}

pub fn comandi() -> Vec<CommandSpec> {
    let spec = |id, label, menu, params, run| CommandSpec { id, label, menu, shortcut: None, aliases: &[], params, enabled: has_doc, run, undoable: false, interactive: None, transparent: false };
    vec![
        spec(
            "cavo.definisci",
            "Definisci cavo",
            &["Progetto", "Cavi", "Definisci cavo…"],
            "{sigla?: (default -W01, -W02…), formazione: \"4G1,5\" | \"3x2.5\", codice?, costruttore?, funzione?, daQuadro?, aQuadro?, lunghezza?, vene?: [..]} → cavo",
            run_definisci as cadcraft_engine::cmd::Run,
        ),
        spec("cavo.elimina", "Elimina cavo", &[], "{sigla}", run_elimina),
        spec("cavi", "Elenco cavi", &["Progetto", "Cavi", "Elenco cavi"], "{} → {cavi: [{sigla, formazione, vene, usate…}]}", run_elenco),
        CommandSpec {
            interactive: Some(fabbrica_posa),
            ..spec("cavo.posa", "Posa cavo", &["Progetto", "Cavi", "Posa cavo"], "{cavo: \"-W01\", at: [x,y] | da + a (fence), vena?: (default la prima libera)} → {vene}", run_posa)
        },
        spec("cavo.togli", "Togli cavo dai fili", &["Progetto", "Cavi", "Togli cavo dai fili"], "{at | da + a}", run_togli),
        spec("cavo.conduttori", "Tabella conduttori dei cavi", &["Progetto", "Cavi", "Tabella conduttori (CSV)"], "{file?} → {righe: [[cavo, vena, filo, da, a, posizione]]}", run_conduttori),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vene_dalle_formazioni() {
        assert_eq!(vene_di("4G1,5"), ["BN", "BK", "GY", "GNYE"]);
        assert_eq!(vene_di("3x2.5"), ["BN", "BK", "GY"]);
        assert_eq!(vene_di("12G1"), ["1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "GNYE"]);
        assert_eq!(vene_di("7x1"), ["1", "2", "3", "4", "5", "6", "7"]);
        assert!(vene_di("?").is_empty());
        assert_eq!(prossima_sigla(&[Cavo { sigla: "-W07".into(), ..Cavo::default() }]), "-W08");
    }
}
