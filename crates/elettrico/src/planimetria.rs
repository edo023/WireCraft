//! Planimetria dell'impianto: i quadri posati sulla pianta (un foglio con il disegno
//! dell'edificio o della linea, in scala) e la lunghezza dei cavi calcolata dai percorsi.
//!
//! Ogni quadro è un blocco `CE_PLAN_QUADRO` con il nome e la scala della pianta (1:100 = un
//! millimetro sul foglio è un decimetro). La lunghezza di un cavo tra due quadri posati sullo
//! stesso foglio è il percorso ortogonale tra i due (in canalina si va per linee dritte), più le
//! risalite alle due estremità, più la scorta; si scrive nella definizione del cavo e i percorsi
//! si disegnano sulla pianta con le sigle dei cavi.

use cadcraft_engine::cmd::has_doc;
use cadcraft_engine::{Accept, CommandSpec, Input, Interactive, Prompt, Result, Session, Step};
use serde_json::{Value, json};

use crate::util::{Inserito, blocchi_definiti, blocchi_inseriti, blocco, colore, errore, esegui, handles, in_gruppo, inserisci_su_layer, layer_corrente, linea, punto_param, su_layer_zero, testo, testo_param};
use crate::{cavi, fogli, impianto};

type P = (f64, f64);

pub const BLOCCO_QUADRO: &str = "CE_PLAN_QUADRO";
const PREFISSO_PERCORSI: &str = "CE_PLAN_PERCORSI_";
/// Scala proposta della pianta (1:k) e risalita a ogni estremità di un cavo (m).
pub const SCALA: f64 = 100.0;
pub const RISALITA: f64 = 2.0;

/// Un quadro posato sulla pianta.
#[derive(Clone, Debug)]
pub struct Posato {
    pub quadro: String,
    pub foglio: u32,
    pub pos: P,
    pub scala: f64,
    pub handle: String,
}

fn nome_quadro(q: &str) -> String {
    let q = q.trim().trim_start_matches('+').trim();
    if q.is_empty() { String::new() } else { format!("+{q}") }
}

fn posato(b: &Inserito) -> Posato {
    Posato {
        quadro: nome_quadro(b.attributo("QUADRO")),
        foglio: fogli::numero_da_layer(&b.layer).unwrap_or(0),
        pos: b.pos,
        scala: b.attributo("SCALA").trim().replace(',', ".").parse::<f64>().ok().filter(|k| *k > 0.0).unwrap_or(SCALA),
        handle: b.handle.clone(),
    }
}

/// I quadri posati sulle piante.
pub fn posati(s: &mut Session) -> Result<Vec<Posato>> {
    Ok(blocchi_inseriti(s, BLOCCO_QUADRO)?.iter().map(posato).collect())
}

fn crea_blocco(s: &mut Session) -> Result<()> {
    if blocchi_definiti(s)?.iter().any(|b| b.eq_ignore_ascii_case(BLOCCO_QUADRO)) {
        return Ok(());
    }
    su_layer_zero(s, |s| {
        let mut h = Vec::new();
        colore(s, "2")?;
        linea(s, &mut h, &[(-7.0, -4.0), (7.0, -4.0), (7.0, 4.0), (-7.0, 4.0)], true)?;
        let r = esegui(s, "attdef", json!({"tag": "QUADRO", "prompt": "Quadro", "default": "", "at": [0.0, -0.9], "height": 2.5}))?;
        let t = handles(&r);
        if let Some(x) = t.first() {
            crate::util::allinea(s, x, cadcraft_engine::doc::HAlign::Center)?;
        }
        h.extend(t);
        for tag in ["DESCRIZIONE", "SCALA"] {
            h.extend(handles(&esegui(s, "attdef", json!({"tag": tag, "prompt": tag, "default": "", "at": [0.0, -6.0], "height": 1.0, "invisible": true}))?));
        }
        blocco(s, BLOCCO_QUADRO, h, "Quadro sulla planimetria")
    })
}

/// Posa il quadro sulla pianta del foglio corrente nel punto `at`.
pub fn posa(s: &mut Session, quadro: &str, at: P, scala: Option<f64>) -> Result<Value> {
    let quadro = nome_quadro(quadro);
    if quadro.is_empty() {
        return Err(errore("Indicare il quadro (es. +QG)"));
    }
    let gia = posati(s)?;
    if let Some(p) = gia.iter().find(|p| p.quadro.eq_ignore_ascii_case(&quadro)) {
        return Err(errore(format!("Il quadro {quadro} è già sulla planimetria (foglio {}): spostalo", p.foglio)));
    }
    let layer = layer_corrente(s)?;
    if fogli::numero_da_layer(&layer).is_none() {
        return Err(errore("Vai sul foglio della planimetria"));
    }
    // la scala della pianta: quella data, o quella dei quadri già posati su questo foglio
    let scala = scala.filter(|k| *k > 0.0).or_else(|| gia.iter().find(|p| fogli::nome_layer(p.foglio).eq_ignore_ascii_case(&layer)).map(|p| p.scala)).unwrap_or(SCALA);
    crea_blocco(s)?;
    let descrizione = impianto::dati(s)?.0.into_iter().find(|n| n.nome.eq_ignore_ascii_case(&quadro)).map(|n| n.descrizione).unwrap_or_default();
    let h = inserisci_su_layer(s, BLOCCO_QUADRO, at, &json!({"QUADRO": quadro, "DESCRIZIONE": descrizione, "SCALA": crate::layout::misura_testo(scala)}), &layer)?;
    Ok(json!({"handle": h, "quadro": quadro, "scala": scala, "message": format!("{quadro} sulla planimetria (scala 1:{})", crate::layout::misura_testo(scala))}))
}

/// La lunghezza (m) di un cavo tra due quadri posati: percorso ortogonale in scala, più le
/// risalite, più la scorta (%), arrotondata al metro sopra.
pub fn lunghezza(a: &Posato, b: &Posato, risalita: f64, scorta: f64) -> f64 {
    let mm = (a.pos.0 - b.pos.0).abs() + (a.pos.1 - b.pos.1).abs();
    let m = mm * a.scala / 1000.0 + 2.0 * risalita;
    (m * (1.0 + scorta / 100.0)).ceil()
}

/// Un cavo misurato sulla pianta.
#[derive(Clone, Debug, PartialEq)]
pub struct Misura {
    pub cavo: String,
    pub da: String,
    pub a: String,
    pub metri: f64,
    pub prima: String,
}

/// Misura i cavi tra quadri posati sullo stesso foglio e scrive le lunghezze (tutte, o con
/// `solo_mancanti` quelle dei cavi che non ne hanno); ridisegna i percorsi sulle piante.
pub fn lunghezze(s: &mut Session, risalita: f64, scorta: f64, solo_mancanti: bool) -> Result<Vec<Misura>> {
    let p = posati(s)?;
    let trova = |q: &str| p.iter().find(|x| x.quadro.eq_ignore_ascii_case(&nome_quadro(q)));
    let (_, tratte) = impianto::dati(s)?;
    let mut misure = Vec::new();
    // (foglio, da, a, sigle e metri) per disegnare i percorsi
    let mut percorsi: Vec<(u32, P, P, Vec<String>)> = Vec::new();
    for t in &tratte {
        let (Some(a), Some(b)) = (trova(&t.da), trova(&t.a)) else { continue };
        if a.foglio != b.foglio {
            continue;
        }
        let metri = lunghezza(a, b, risalita, scorta);
        let mut sigle = Vec::new();
        for c in &t.cavi {
            sigle.push(format!("{} {} m", c.sigla, metri));
            if solo_mancanti && !c.lunghezza.trim().is_empty() {
                continue;
            }
            misure.push(Misura { cavo: c.sigla.clone(), da: t.da.clone(), a: t.a.clone(), metri, prima: c.lunghezza.clone() });
        }
        percorsi.push((a.foglio, a.pos, b.pos, sigle));
    }
    for m in &misure {
        cavi::definisci(s, &json!({"sigla": m.cavo, "lunghezza": crate::layout::misura_testo(m.metri)}))?;
    }
    disegna_percorsi(s, &percorsi)?;
    Ok(misure)
}

/// I percorsi dei cavi sulle piante: un blocco per foglio con le linee e le sigle, rifatto ogni volta.
fn disegna_percorsi(s: &mut Session, percorsi: &[(u32, P, P, Vec<String>)]) -> Result<()> {
    let vecchi: Vec<String> = blocchi_inseriti(s, &format!("{PREFISSO_PERCORSI}*"))?.into_iter().map(|b| b.handle).collect();
    if !vecchi.is_empty() {
        esegui(s, "erase", json!({"handles": vecchi}))?;
    }
    let mut fogli_p: Vec<u32> = percorsi.iter().map(|p| p.0).collect();
    fogli_p.sort_unstable();
    fogli_p.dedup();
    let definiti = blocchi_definiti(s)?;
    for f in fogli_p {
        // un nome nuovo a ogni ricalcolo: il blocco vecchio resta senza inserimenti
        let nome = (1..).map(|k| format!("{PREFISSO_PERCORSI}{f}_{k}")).find(|n| !definiti.iter().any(|d| d.eq_ignore_ascii_case(n))).unwrap_or_default();
        su_layer_zero(s, |s| {
            let mut h = Vec::new();
            for (_, a, b, sigle) in percorsi.iter().filter(|p| p.0 == f) {
                colore(s, "4")?;
                // orizzontale poi verticale, dai bordi dei riquadri
                let gomito = (b.0, a.1);
                linea(s, &mut h, &[*a, gomito, *b], false)?;
                colore(s, "7")?;
                let (xm, ym) = ((a.0 + gomito.0) / 2.0, a.1);
                let (x, y, centro) = if (a.0 - b.0).abs() > 30.0 { (xm, ym + 1.5, "BC") } else { (b.0 + 1.5, (a.1 + b.1) / 2.0, "L") };
                for (j, t) in sigle.iter().enumerate() {
                    testo(s, &mut h, (x, y + 3.0 * j as f64), t, 2.0, centro)?;
                }
            }
            blocco(s, &nome, h, "Percorsi dei cavi sulla planimetria")
        })?;
        inserisci_su_layer(s, &nome, (0.0, 0.0), &json!({}), &fogli::nome_layer(f))?;
    }
    Ok(())
}

fn numero(p: &Value, k: &str, default: f64) -> Result<f64> {
    match p.get(k) {
        None | Some(Value::Null) => Ok(default),
        Some(v) => v.as_f64().or_else(|| v.as_str().and_then(|t| t.trim().replace(',', ".").parse().ok())).filter(|x: &f64| x.is_finite() && *x >= 0.0).ok_or_else(|| errore(format!("{k}: numero positivo"))),
    }
}

fn run_elenco(s: &mut Session, _p: &Value) -> Result<Value> {
    let p = posati(s)?;
    let (nodi, _) = impianto::dati(s)?;
    let quadri: Vec<Value> = nodi
        .iter()
        .map(|n| {
            let q = p.iter().find(|x| x.quadro.eq_ignore_ascii_case(&n.nome));
            json!({"quadro": n.nome, "descrizione": n.descrizione, "posato": q.is_some(), "foglio": q.map(|x| x.foglio), "x": q.map(|x| x.pos.0), "y": q.map(|x| x.pos.1), "handle": q.map(|x| x.handle.clone()), "scala": q.map(|x| x.scala)})
        })
        .collect();
    let n = quadri.iter().filter(|q| q["posato"] == true).count();
    Ok(json!({"quadri": quadri, "message": format!("{n} quadri su {} sulla planimetria", nodi.len())}))
}

fn run_posa(s: &mut Session, p: &Value) -> Result<Value> {
    let quadro = testo_param(p, "quadro").ok_or_else(|| errore("Indicare il quadro (es. +QG)"))?;
    let at = punto_param(p, "at").ok_or_else(|| errore("Indicare il punto (at)"))?;
    let scala = p.get("scala").map(|_| numero(p, "scala", SCALA)).transpose()?;
    in_gruppo(s, "Quadro sulla planimetria", |s| posa(s, &quadro, at, scala))
}

fn run_lunghezze(s: &mut Session, p: &Value) -> Result<Value> {
    let risalita = numero(p, "risalita", RISALITA)?;
    let scorta = numero(p, "scorta", 10.0)?;
    let solo = p.get("soloMancanti").and_then(Value::as_bool).unwrap_or(false);
    let m = in_gruppo(s, "Lunghezze dei cavi", |s| lunghezze(s, risalita, scorta, solo))?;
    let msg = if m.is_empty() { "Nessun cavo tra quadri posati sulla stessa planimetria".to_string() } else { m.iter().map(|x| format!("{} {}→{}: {} m", x.cavo, x.da, x.a, x.metri)).collect::<Vec<_>>().join(", ") };
    Ok(json!({"cavi": m.iter().map(|x| json!({"cavo": x.cavo, "da": x.da, "a": x.a, "metri": x.metri, "prima": x.prima})).collect::<Vec<_>>(), "message": msg}))
}

fn run_nuovo(s: &mut Session, _p: &Value) -> Result<Value> {
    let n = fogli::elenco(s)?.last().map_or(1, |f| f.numero + 1);
    fogli::crea(s, n)?;
    fogli::imposta_titolo(s, n, "Planimetria impianto", Some("Plant layout"))?;
    fogli::numera(s)?;
    fogli::vai(s, n)?;
    Ok(json!({"foglio": n, "message": format!("Foglio {n}: disegna o importa la pianta, poi posa i quadri")}))
}

/// Posa interattiva: il quadro (Invio = il prossimo non ancora posato) e il punto.
struct PosaInterattiva {
    quadro: Option<String>,
    prossimo: Option<String>,
}

impl PosaInterattiva {
    fn prossimo(s: &mut Session) -> Option<String> {
        let p = posati(s).ok()?;
        impianto::dati(s).ok()?.0.into_iter().map(|n| n.nome).find(|q| !p.iter().any(|x| x.quadro.eq_ignore_ascii_case(q)))
    }
}

impl Interactive for PosaInterattiva {
    fn name(&self) -> &'static str {
        "PLANIMETRIA.POSA"
    }
    fn prompt(&self, _s: &Session) -> Prompt {
        match &self.quadro {
            Some(q) => Prompt::new(format!("Punto del quadro {q} sulla pianta"), Accept::POINT),
            None => Prompt { default: self.prossimo.clone(), ..Prompt::new("Quadro da posare (Invio: il prossimo)", Accept::TEXT) },
        }
    }
    fn input(&mut self, s: &mut Session, i: Input) -> Result<Step> {
        match (i, &self.quadro) {
            (Input::Cancel, _) => Ok(Step::Cancel),
            (Input::Point(q), Some(quadro)) => {
                match in_gruppo(s, "Quadro sulla planimetria", |s| posa(s, quadro, (q.x, q.y), None)) {
                    Ok(r) => s.echo(r["message"].as_str().unwrap_or_default().to_string()),
                    Err(e) => s.echo(e.to_string()),
                }
                self.quadro = None;
                self.prossimo = Self::prossimo(s);
                Ok(Step::Continue)
            }
            (Input::Text(t) | Input::Keyword(t), None) if !t.trim().is_empty() => {
                self.quadro = Some(t.trim().to_string());
                Ok(Step::Continue)
            }
            (Input::Enter | Input::Text(_), None) => match self.prossimo.clone().or_else(|| Self::prossimo(s)) {
                Some(q) => {
                    self.quadro = Some(q);
                    Ok(Step::Continue)
                }
                None => {
                    s.echo("Tutti i quadri sono sulla planimetria".to_string());
                    Ok(Step::Done)
                }
            },
            (Input::Enter, Some(_)) => {
                self.quadro = None;
                Ok(Step::Continue)
            }
            _ => Ok(Step::Continue),
        }
    }
}

fn fabbrica(_s: &Session) -> Result<Box<dyn Interactive>> {
    Ok(Box::new(PosaInterattiva { quadro: None, prossimo: None }))
}

pub fn comandi() -> Vec<CommandSpec> {
    let spec = |id, label, menu, params, run| CommandSpec { id, label, menu, shortcut: None, aliases: &[], params, enabled: has_doc, run, undoable: false, interactive: None, transparent: false };
    vec![
        spec("planimetria", "Planimetria", &["Quadro", "Planimetria", "Quadri sulla planimetria"], "{} → {quadri: [{quadro, descrizione, posato, foglio?, x?, y?, scala?}]}", run_elenco as cadcraft_engine::cmd::Run),
        spec("planimetria.nuovo", "Nuovo foglio di planimetria", &["Quadro", "Planimetria", "Nuovo foglio"], "{} — un foglio nuovo per la pianta", run_nuovo),
        CommandSpec {
            interactive: Some(fabbrica),
            ..spec("planimetria.posa", "Quadro sulla planimetria", &["Quadro", "Planimetria", "Posa un quadro"], "{quadro, at: [x,y], scala?: 100 (1:k; default quella del foglio)} — il quadro sulla pianta del foglio corrente", run_posa)
        },
        spec(
            "planimetria.lunghezze",
            "Lunghezze dei cavi",
            &["Quadro", "Planimetria", "Lunghezze dei cavi"],
            "{scorta?: 10 (%), risalita?: 2 (m per estremità), soloMancanti?: bool} — lunghezza dei cavi tra quadri posati (percorso ortogonale in scala), scritta nei cavi; percorsi disegnati",
            run_lunghezze,
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f64, y: f64, scala: f64) -> Posato {
        Posato { quadro: String::new(), foglio: 1, pos: (x, y), scala, handle: String::new() }
    }

    #[test]
    fn lunghezza_dal_percorso() {
        // 100 mm + 50 mm a 1:100 = 15 m, più 2 m per parte = 19 m, più il 10 % = 20,9 → 21
        assert_eq!(lunghezza(&p(0.0, 0.0, 100.0), &p(100.0, 50.0, 100.0), 2.0, 10.0), 21.0);
        assert_eq!(lunghezza(&p(0.0, 0.0, 50.0), &p(0.0, 100.0, 50.0), 0.0, 0.0), 5.0);
    }
}
