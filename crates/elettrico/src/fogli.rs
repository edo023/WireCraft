//! Multifoglio: i fogli sono layer col nome uguale al numero (`1`, `2`, …), sovrapposti; se ne
//! vede uno alla volta, sopra il foglio 0 (layer `0`) che porta squadratura e cartiglio.

use cadcraft_engine::cmd::has_doc;
use cadcraft_engine::{Accept, CommandSpec, Interactive, Result, Session};
use serde_json::{Value, json};

use crate::squadratura::{self, BLOCCO_MASTER, BLOCCO_TITOLO, NUMERAZIONE, POS_TITOLO};
use crate::util::{Richiesta, blocchi_inseriti, errore, esegui, in_gruppo, inserisci_su_layer, numero_param, punto_param, su_layer_zero, testo_param};

const FOGLI_INIZIALI: u32 = 10;
/// Limite di sicurezza per "aggiungi N fogli".
const MAX_FOGLI: u32 = 2000;
/// Colore dei layer dei fogli: è il colore dei fili disegnati DaLayer.
const COLORE_FOGLI: u32 = 9;
/// I blocchi titolo dei fogli (il nome può cambiare tra profili: `TITOLO1`, `TITOLO_…`).
const FILTRO_TITOLI: &str = "TITOLO*";

#[derive(Clone, Debug, PartialEq)]
pub struct Foglio {
    pub numero: u32,
    pub layer: String,
    pub titolo: String,
    /// Titolo nella seconda lingua.
    pub titolo1: String,
    pub visibile: bool,
    pub corrente: bool,
}

pub fn nome_layer(n: u32) -> String {
    n.to_string()
}

/// Il numero di un foglio dal nome del suo layer (`"12"` → 12); il layer `0` non è un foglio.
pub fn numero_da_layer(nome: &str) -> Option<u32> {
    if nome.is_empty() || !nome.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    nome.parse().ok().filter(|n| *n > 0)
}

/// I fogli del disegno, in ordine di numero, con i titoli presi dai blocchi titolo.
pub fn elenco(s: &mut Session) -> Result<Vec<Foglio>> {
    let v = esegui(s, "layer", json!({}))?;
    let titoli = blocchi_inseriti(s, FILTRO_TITOLI)?;
    let mut fogli: Vec<Foglio> = v
        .get("layers")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|l| {
            let nome = l.get("name")?.as_str()?;
            let numero = numero_da_layer(nome)?;
            let flag = |k: &str| l.get(k).and_then(Value::as_bool).unwrap_or(false);
            let blocco = titoli.iter().find(|t| t.layer == nome);
            let titolo = blocco.map(|t| t.attributo("_TITOLO").to_string()).unwrap_or_default();
            let titolo1 = blocco.map(|t| t.attributo("_TITOLO1").to_string()).unwrap_or_default();
            Some(Foglio { numero, layer: nome.to_string(), titolo, titolo1, visibile: flag("on") && !flag("frozen"), corrente: flag("current") })
        })
        .collect();
    fogli.sort_by_key(|f| f.numero);
    Ok(fogli)
}

/// Il foglio su cui si lavora: il layer corrente se è un foglio, altrimenti il primo visibile.
pub fn corrente(s: &mut Session) -> Result<Option<Foglio>> {
    let fogli = elenco(s)?;
    Ok(fogli.iter().find(|f| f.corrente).or_else(|| fogli.iter().find(|f| f.visibile)).cloned())
}

/// Mostra il foglio `n`, lo rende corrente e nasconde (congela) gli altri.
pub fn vai(s: &mut Session, n: u32) -> Result<Foglio> {
    let fogli = elenco(s)?;
    let Some(dest) = fogli.iter().find(|f| f.numero == n).cloned() else {
        return Err(errore(format!("Il foglio {n} non esiste")));
    };
    esegui(s, "layer.set", json!({"name": dest.layer, "on": true, "frozen": false}))?;
    esegui(s, "layer.current", json!({"name": dest.layer}))?;
    for f in fogli.iter().filter(|f| f.numero != n && f.visibile) {
        esegui(s, "layer.set", json!({"name": f.layer, "frozen": true}))?;
    }
    Ok(dest)
}

/// Crea il foglio `n` (layer col colore dei fili, congelato).
pub fn crea(s: &mut Session, n: u32) -> Result<()> {
    let nome = nome_layer(n);
    esegui(s, "layer.new", json!({"name": nome, "color": COLORE_FOGLI}))?;
    esegui(s, "layer.set", json!({"name": nome, "frozen": true}))?;
    Ok(())
}

/// Aggiunge `quanti` fogli dopo l'ultimo; restituisce i loro numeri.
pub fn aggiungi(s: &mut Session, quanti: u32) -> Result<Vec<u32>> {
    let ultimo = elenco(s)?.last().map_or(0, |f| f.numero);
    aggiungi_da(s, ultimo + 1, quanti, 1)
}

/// Aggiunge `quanti` fogli a partire dal numero `primo`, ogni `incremento`; i numeri già usati
/// sono un errore. Restituisce i numeri dei fogli nuovi.
pub fn aggiungi_da(s: &mut Session, primo: u32, quanti: u32, incremento: u32) -> Result<Vec<u32>> {
    if quanti == 0 || quanti > MAX_FOGLI {
        return Err(errore(format!("Numero di fogli non valido: {quanti} (da 1 a {MAX_FOGLI})")));
    }
    if primo == 0 || incremento == 0 {
        return Err(errore("Il primo foglio e l'incremento partono da 1"));
    }
    let esistenti: Vec<u32> = elenco(s)?.iter().map(|f| f.numero).collect();
    let nuovi: Vec<u32> = (0..quanti).map(|k| primo.saturating_add(k.saturating_mul(incremento))).collect();
    if let Some(n) = nuovi.iter().find(|n| esistenti.contains(n)) {
        return Err(errore(format!("Il foglio {n} esiste già")));
    }
    for &n in &nuovi {
        crea(s, n)?;
    }
    Ok(nuovi)
}

/// Scrive numero, precedente, seguente e totale su ogni foglio e mette il blocco titolo dove
/// manca. Restituisce i fogli numerati.
pub fn numera(s: &mut Session) -> Result<usize> {
    su_layer_zero(s, squadratura::crea_blocchi)?;
    let fogli = elenco(s)?;
    let nomi: Vec<&str> = NUMERAZIONE.iter().map(|(n, ..)| *n).collect();
    let esistenti = blocchi_inseriti(s, &format!("{},{FILTRO_TITOLI}", nomi.join(",")))?;
    let due = |n: u32| format!("{n:02}");
    let totale = fogli.len() as u32;
    for (i, f) in fogli.iter().enumerate() {
        let prec = i.checked_sub(1).and_then(|k| fogli.get(k)).map_or_else(|| "/".to_string(), |g| due(g.numero));
        let segue = fogli.get(i + 1).map_or_else(|| "/".to_string(), |g| due(g.numero));
        for (nome, tag, pos, _) in NUMERAZIONE {
            let valore = match nome {
                "NUMFG" => due(f.numero),
                "NUMFGP" => prec.clone(),
                "NUMFGS" => segue.clone(),
                _ => due(totale),
            };
            let suoi: Vec<&str> = esistenti.iter().filter(|b| b.layer == f.layer && b.nome.eq_ignore_ascii_case(nome)).map(|b| b.handle.as_str()).collect();
            if suoi.is_empty() {
                inserisci_su_layer(s, nome, pos, &json!({ tag: valore }), &f.layer)?;
            }
            for h in suoi {
                esegui(s, "attedit", json!({"handle": h, "values": { tag: valore }}))?;
            }
        }
        let ha_titolo = esistenti.iter().any(|b| b.layer == f.layer && b.nome.to_ascii_uppercase().starts_with("TITOLO"));
        if !ha_titolo {
            inserisci_su_layer(s, BLOCCO_TITOLO, POS_TITOLO, &json!({"_TITOLO": f.titolo}), &f.layer)?;
        }
    }
    Ok(fogli.len())
}

/// Scrive il titolo del foglio `n` nel suo blocco titolo (lo crea se manca).
pub fn imposta_titolo(s: &mut Session, n: u32, titolo: &str, seconda_lingua: Option<&str>) -> Result<()> {
    let layer = nome_layer(n);
    if !elenco(s)?.iter().any(|f| f.numero == n) {
        return Err(errore(format!("Il foglio {n} non esiste")));
    }
    let mut valori = json!({"_TITOLO": titolo});
    if let Some(t) = seconda_lingua {
        valori["_TITOLO1"] = json!(t);
    }
    match blocchi_inseriti(s, FILTRO_TITOLI)?.into_iter().find(|b| b.layer == layer) {
        Some(b) => {
            esegui(s, "attedit", json!({"handle": b.handle, "values": valori}))?;
        }
        None => {
            su_layer_zero(s, squadratura::crea_blocchi)?;
            inserisci_su_layer(s, BLOCCO_TITOLO, POS_TITOLO, &valori, &layer)?;
        }
    }
    Ok(())
}

fn ha_cartiglio(s: &mut Session) -> Result<bool> {
    Ok(blocchi_inseriti(s, "MASTER*")?.iter().any(|b| b.layer == "0"))
}

/// Prepara il disegno come multifoglio: squadratura e cartiglio sul foglio 0, `fogli` fogli vuoti.
pub fn nuovo_multifoglio(s: &mut Session, p: &Value) -> Result<Value> {
    in_gruppo(s, "Nuovo multifoglio", |s| {
        if ha_cartiglio(s)? {
            return Err(errore("Il disegno ha già un cartiglio: è già un multifoglio"));
        }
        let quanti = numero_param(p, "fogli").unwrap_or(FOGLI_INIZIALI);
        esegui(s, "units", json!({"insunits": 4}))?; // millimetri
        esegui(s, "layer.set", json!({"name": "0", "on": true, "frozen": false}))?;
        esegui(s, "layer.current", json!({"name": "0"}))?;
        squadratura::crea_blocchi(s)?;
        let mut attr = serde_json::Map::new();
        for (param, tag) in [
            ("commessa", "JOB-ORDER"),
            ("descrizione", "JOB-DESCRIPTION"),
            ("multifoglio", "FILE-NAME"),
            ("descrizioneMultifoglio", "DRAWING-DESCRIPTION"),
            ("cliente", "CUSTOMER"),
            ("cliente1", "CUSTOMER-LINE1"),
            ("cliente2", "CUSTOMER-LINE2"),
            ("disegnatore", "DRAFTSMAN"),
            ("data", "DATE"),
        ] {
            if let Some(v) = testo_param(p, param) {
                attr.insert(tag.into(), json!(v));
            }
        }
        esegui(s, "insert", json!({"name": BLOCCO_MASTER, "at": [0.0, 0.0], "attribs": attr}))?;
        aggiungi(s, quanti)?;
        numera(s)?;
        vai(s, 1)?;
        let (w, h) = squadratura::FOGLIO;
        esegui(s, "zoom.window", json!({"p1": [0.0, 0.0], "p2": [w, h]}))?;
        Ok(json!({"fogli": quanti, "message": format!("Multifoglio pronto: {quanti} fogli. Foglio corrente: 1")}))
    })
}

fn messaggio_foglio(f: &Foglio) -> String {
    if f.titolo.is_empty() { format!("Foglio: {}", f.numero) } else { format!("Foglio: {} – {}", f.numero, f.titolo) }
}

fn esito_foglio(f: &Foglio) -> Value {
    json!({"foglio": f.numero, "titolo": f.titolo, "message": messaggio_foglio(f)})
}

fn run_nuovo(s: &mut Session, p: &Value) -> Result<Value> {
    nuovo_multifoglio(s, p)
}

fn run_aggiungi(s: &mut Session, p: &Value) -> Result<Value> {
    let quanti = numero_param(p, "quanti").unwrap_or(FOGLI_INIZIALI);
    let titolo = testo_param(p, "titolo");
    in_gruppo(s, "Aggiungi fogli", |s| {
        let nuovi = match numero_param(p, "da") {
            Some(primo) => aggiungi_da(s, primo, quanti, numero_param(p, "incremento").unwrap_or(1))?,
            None => aggiungi(s, quanti)?,
        };
        numera(s)?;
        if let Some(t) = &titolo {
            for &n in &nuovi {
                imposta_titolo(s, n, t, None)?;
            }
        }
        let (a, b) = (nuovi.first().copied().unwrap_or(0), nuovi.last().copied().unwrap_or(0));
        Ok(json!({"fogli": nuovi, "message": format!("Aggiunti {quanti} fogli ({a}–{b})")}))
    })
}

fn passo(s: &mut Session, avanti: bool) -> Result<Value> {
    let fogli = elenco(s)?;
    let Some(att) = corrente(s)? else { return Err(errore("Il disegno non ha fogli: usa Nuovo multifoglio")) };
    let pos = fogli.iter().position(|f| f.numero == att.numero).unwrap_or(0);
    let dest = if avanti { fogli.get(pos + 1) } else { pos.checked_sub(1).and_then(|i| fogli.get(i)) };
    let Some(dest) = dest.map(|f| f.numero) else {
        return Ok(json!({"foglio": att.numero, "message": if avanti { "È l'ultimo foglio" } else { "È il primo foglio" }}));
    };
    in_gruppo(s, "Cambio foglio", |s| vai(s, dest).map(|f| esito_foglio(&f)))
}

fn run_seguente(s: &mut Session, _p: &Value) -> Result<Value> {
    passo(s, true)
}

fn run_precedente(s: &mut Session, _p: &Value) -> Result<Value> {
    passo(s, false)
}

fn run_cambio(s: &mut Session, p: &Value) -> Result<Value> {
    let Some(n) = numero_param(p, "foglio") else { return Err(errore("Indicare il numero del foglio")) };
    in_gruppo(s, "Cambio foglio", |s| vai(s, n).map(|f| esito_foglio(&f)))
}

fn run_titolo(s: &mut Session, p: &Value) -> Result<Value> {
    let titolo = testo_param(p, "titolo").unwrap_or_default();
    let seconda = testo_param(p, "titolo1");
    let n = match numero_param(p, "foglio") {
        Some(n) => n,
        None => corrente(s)?.map(|f| f.numero).ok_or_else(|| errore("Nessun foglio corrente"))?,
    };
    in_gruppo(s, "Titolo foglio", |s| {
        imposta_titolo(s, n, &titolo, seconda.as_deref())?;
        Ok(json!({"foglio": n, "titolo": titolo, "message": format!("Foglio {n}: {titolo}")}))
    })
}

fn run_numera(s: &mut Session, _p: &Value) -> Result<Value> {
    in_gruppo(s, "Numera fogli", |s| numera(s).map(|n| json!({"fogli": n, "message": format!("Numerati {n} fogli")})))
}

fn run_elenco(s: &mut Session, _p: &Value) -> Result<Value> {
    let fogli = elenco(s)?;
    let righe: Vec<String> = fogli.iter().map(|f| format!("{}{:>4}  {}", if f.corrente { "▶" } else { " " }, f.numero, f.titolo)).collect();
    let lista: Vec<Value> = fogli.iter().map(|f| json!({"foglio": f.numero, "titolo": f.titolo, "corrente": f.corrente, "layer": f.layer})).collect();
    Ok(json!({"fogli": lista, "message": if righe.is_empty() { "Nessun foglio".to_string() } else { righe.join("\n") }}))
}

fn run_coordinata(s: &mut Session, p: &Value) -> Result<Value> {
    let Some((x, y)) = punto_param(p, "at") else { return Err(errore("Indicare un punto")) };
    let n = corrente(s)?.map_or(0, |f| f.numero);
    let rif = squadratura::riferimento(n, x, y);
    Ok(json!({"riferimento": rif, "message": format!("Posizione: {rif}")}))
}

/// Divide una descrizione in al più due righe: a `|` o a capo, altrimenti allo spazio più
/// vicino alla metà se è più lunga di `max` caratteri.
pub fn due_righe(t: &str, max: usize) -> (String, String) {
    let t = t.trim();
    if let Some((a, b)) = t.split_once(['|', '\n']) {
        return (a.trim().to_string(), b.trim().replace(['|', '\n'], " "));
    }
    if t.chars().count() <= max || !t.contains(' ') {
        return (t.to_string(), String::new());
    }
    let meta = t.len() / 2;
    let taglio = t.match_indices(' ').map(|(i, _)| i).min_by_key(|i| i.abs_diff(meta)).unwrap_or(meta);
    (t[..taglio].trim().to_string(), t[taglio..].trim().to_string())
}

/// Scrive la descrizione della colonna `col` (larga `campate` colonne) nella fascia in testa
/// al foglio `n`, in due lingue; toglie quelle che c'erano su quelle colonne.
pub fn descrizione_colonna(s: &mut Session, n: u32, col: u32, campate: u32, testo1: &str, testo2: &str) -> Result<String> {
    use squadratura::{BLOCCO_DESCRIZIONE, COLONNE, CORNICE_MAX, CORNICE_MIN, FASCIA_DESCRIZIONI};
    if !elenco(s)?.iter().any(|f| f.numero == n) {
        return Err(errore(format!("Il foglio {n} non esiste")));
    }
    let campate = campate.clamp(1, COLONNE);
    let col = col.min(COLONNE - campate);
    let larg = (CORNICE_MAX.0 - CORNICE_MIN.0) / f64::from(COLONNE);
    let (x0, x1) = (CORNICE_MIN.0 + larg * f64::from(col), CORNICE_MIN.0 + larg * f64::from(col + campate));
    let layer = nome_layer(n);
    let via: Vec<String> = blocchi_inseriti(s, BLOCCO_DESCRIZIONE)?.into_iter().filter(|b| b.layer == layer && b.pos.0 > x0 && b.pos.0 < x1).map(|b| b.handle).collect();
    if !via.is_empty() {
        esegui(s, "erase", json!({"handles": via}))?;
    }
    if testo1.trim().is_empty() && testo2.trim().is_empty() {
        return Ok(String::new());
    }
    su_layer_zero(s, squadratura::crea_blocchi)?;
    let max = (17 * campate) as usize;
    let (r1, r2) = due_righe(testo1, max);
    let (r3, r4) = due_righe(testo2, max);
    let at = ((x0 + x1) / 2.0, CORNICE_MAX.1 - FASCIA_DESCRIZIONI / 2.0);
    inserisci_su_layer(s, BLOCCO_DESCRIZIONE, at, &json!({"RIGA1": r1, "RIGA2": r2, "RIGA3": r3, "RIGA4": r4}), &layer)
}

/// Le descrizioni delle colonne di un foglio: (colonna, prima lingua, seconda lingua).
pub fn descrizioni_colonne(s: &mut Session, n: u32) -> Result<Vec<(u32, String, String)>> {
    let layer = nome_layer(n);
    let mut v: Vec<(u32, String, String)> = blocchi_inseriti(s, squadratura::BLOCCO_DESCRIZIONE)?
        .into_iter()
        .filter(|b| b.layer == layer)
        .map(|b| {
            let unisci = |a: &str, c: &str| [a.trim(), c.trim()].iter().filter(|x| !x.is_empty()).copied().collect::<Vec<_>>().join(" ");
            (squadratura::colonna_riga(b.pos.0, b.pos.1).0, unisci(b.attributo("RIGA1"), b.attributo("RIGA2")), unisci(b.attributo("RIGA3"), b.attributo("RIGA4")))
        })
        .collect();
    v.sort_by_key(|d| d.0);
    Ok(v)
}

fn run_descrizione(s: &mut Session, p: &Value) -> Result<Value> {
    let n = match numero_param(p, "foglio") {
        Some(n) => n,
        None => corrente(s)?.map(|f| f.numero).ok_or_else(|| errore("Nessun foglio corrente"))?,
    };
    let col = match (numero_param(p, "colonna"), punto_param(p, "at")) {
        (Some(c), _) => c,
        (None, Some((x, y))) => squadratura::colonna_riga(x, y).0,
        _ => return Err(errore("Indicare la colonna (0-9) o un punto (at)")),
    };
    let campate = numero_param(p, "colonne").unwrap_or(1);
    let t1 = testo_param(p, "testo").unwrap_or_default();
    let t2 = testo_param(p, "testo2").unwrap_or_default();
    in_gruppo(s, "Descrizione colonna", |s| {
        descrizione_colonna(s, n, col, campate, &t1, &t2)?;
        Ok(json!({"foglio": n, "colonna": col, "message": format!("Foglio {n}, colonna {col}: {t1}")}))
    })
}

/// DESCRIZIONE COLONNA interattiva: punto nella colonna, testo, seconda lingua.
struct ChiediDescrizione {
    at: Option<(f64, f64)>,
    testo: Option<String>,
}

impl Interactive for ChiediDescrizione {
    fn name(&self) -> &'static str {
        "descrizionecolonna"
    }
    fn prompt(&self, _s: &Session) -> cadcraft_engine::Prompt {
        use cadcraft_engine::Prompt;
        match (&self.at, &self.testo) {
            (None, _) => Prompt::new("Punto nella colonna da descrivere", Accept::POINT),
            (Some(_), None) => Prompt::new("Descrizione (| per andare a capo)", Accept::TEXT),
            (Some(_), Some(_)) => Prompt::new("Seconda lingua (Invio per nessuna)", Accept::TEXT),
        }
    }
    fn input(&mut self, s: &mut Session, i: cadcraft_engine::Input) -> Result<cadcraft_engine::Step> {
        use cadcraft_engine::{Input, Step};
        match i {
            Input::Cancel => Ok(Step::Cancel),
            Input::Point(q) if self.at.is_none() => {
                self.at = Some((q.x, q.y));
                Ok(Step::Continue)
            }
            Input::Text(t) if self.at.is_some() && self.testo.is_none() => {
                self.testo = Some(t);
                Ok(Step::Continue)
            }
            Input::Text(_) | Input::Enter if self.testo.is_some() => {
                let t2 = if let Input::Text(t) = i { t } else { String::new() };
                let at = self.at.unwrap_or_default();
                let r = s.execute("descrizionecolonna", &json!({"at": [at.0, at.1], "testo": self.testo.clone().unwrap_or_default(), "testo2": t2}))?;
                if let Some(m) = r.get("message").and_then(Value::as_str) {
                    s.echo(m.to_string());
                }
                Ok(Step::Done)
            }
            Input::Enter => Ok(Step::Done),
            _ => Ok(Step::Continue),
        }
    }
}

fn chiedi_descrizione(_s: &Session) -> Result<Box<dyn Interactive>> {
    Ok(Box::new(ChiediDescrizione { at: None, testo: None }))
}

fn chiedi_quanti(_s: &Session) -> Result<Box<dyn Interactive>> {
    Ok(Box::new(Richiesta {
        nome: "aggiungifogli",
        messaggio: "Quanti fogli aggiungere".into(),
        default: Some(FOGLI_INIZIALI.to_string()),
        accetta: Accept::NUMBER,
        id: "aggiungifogli",
        chiave: "quanti",
    }))
}

fn chiedi_foglio(_s: &Session) -> Result<Box<dyn Interactive>> {
    Ok(Box::new(Richiesta { nome: "cambiofoglio", messaggio: "Numero del foglio".into(), default: None, accetta: Accept::NUMBER, id: "cambiofoglio", chiave: "foglio" }))
}

fn chiedi_titolo(_s: &Session) -> Result<Box<dyn Interactive>> {
    Ok(Box::new(Richiesta { nome: "titolofoglio", messaggio: "Titolo del foglio corrente".into(), default: None, accetta: Accept::TEXT, id: "titolofoglio", chiave: "titolo" }))
}

fn chiedi_punto(_s: &Session) -> Result<Box<dyn Interactive>> {
    Ok(Box::new(Richiesta { nome: "coordinata", messaggio: "Indicare un punto".into(), default: None, accetta: Accept::POINT, id: "coordinata", chiave: "at" }))
}

const MENU: &str = "Fogli";

fn spec(
    id: &'static str,
    label: &'static str,
    voce: &'static [&'static str],
    aliases: &'static [&'static str],
    params: &'static str,
    run: cadcraft_engine::cmd::Run,
    interactive: Option<cadcraft_engine::cmd::Factory>,
) -> CommandSpec {
    CommandSpec { id, label, menu: voce, shortcut: None, aliases, params, enabled: has_doc, run, undoable: false, interactive, transparent: false }
}

pub fn comandi() -> Vec<CommandSpec> {
    vec![
        spec(
            "nuovomultifoglio",
            "Nuovo Multifoglio",
            &[MENU, "Nuovo Multifoglio"],
            &[],
            "{fogli?: numero (default 10), commessa?, descrizione?, multifoglio?, descrizioneMultifoglio?, cliente?, cliente1?, cliente2?, disegnatore?, data?} → {fogli}",
            run_nuovo,
            None,
        ),
        spec("foglioprecedente", "Foglio Precedente", &[MENU, "Foglio Precedente"], &["fp"], "{} → {foglio, titolo}", run_precedente, None),
        spec("cambiofoglio", "Cambio FOGLIO", &[MENU, "Cambio Foglio…"], &["cf"], "{foglio} → {foglio, titolo}", run_cambio, Some(chiedi_foglio)),
        spec("fogliosuccessivo", "Foglio Seguente", &[MENU, "Foglio Seguente"], &["fs"], "{} → {foglio, titolo}", run_seguente, None),
        spec(
            "aggiungifogli",
            "Aggiungi FOGLI",
            &[MENU, "Aggiungi Fogli…"],
            &[],
            "{quanti?: numero (default 10), da?: primo numero (default dopo l'ultimo), incremento?: (default 1), titolo?} → {fogli}",
            run_aggiungi,
            Some(chiedi_quanti),
        ),
        spec(
            "titolofoglio",
            "Titolo Foglio",
            &[MENU, "Titolo Foglio…"],
            &[],
            "{titolo, titolo1?: seconda lingua, foglio?: numero (default il corrente)}",
            run_titolo,
            Some(chiedi_titolo),
        ),
        spec("numerafogli", "Numera Fogli", &[MENU, "Numera Fogli"], &[], "{} → {fogli}", run_numera, None),
        spec(
            "descrizionecolonna",
            "Descrizione Colonna",
            &[MENU, "Descrizione Colonna…"],
            &[],
            "{colonna: 0-9 | at: [x,y], colonne?: quante colonne (1), testo: \"ALIMENTAZIONE|400 VAC\" (| va a capo), testo2?: seconda lingua, foglio?: (default il corrente)} — testo vuoto: toglie",
            run_descrizione,
            Some(chiedi_descrizione),
        ),
        spec("elencofogli", "Elenco Fogli", &[MENU, "Elenco Fogli"], &[], "{} → {fogli: [{foglio, titolo, corrente, layer}]}", run_elenco, None),
        spec("coordinata", "Coordinata Foglio", &[MENU, "Coordinata foglio/colonna riga"], &[], "{at: [x,y]} → {riferimento: \"05/3B\"}", run_coordinata, Some(chiedi_punto)),
    ]
}
