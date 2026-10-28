//! Editor delle morsettiere: l'elenco dei morsetti di ogni morsettiera (e dei pin dei
//! connettori) nell'ordine della morsettiera, la rinumerazione secondo lo schema, i fili o un
//! ordine dato a mano, e la rinomina.
//!
//! I morsetti a più livelli hanno lo stesso numero e livelli diversi (`LIVELLO`): restano
//! insieme e prendono lo stesso numero nuovo. I numeri che non sono numeri (`PE`, `N`, `1A`)
//! tengono il loro nome e il loro posto. I ponticelli (`PONTE`) restano sugli stessi morsetti.

use cadcraft_engine::cmd::has_doc;
use cadcraft_engine::{CommandSpec, Result, Session};
use serde_json::{Value, json};

use crate::morsetti::{self, Morsetto};
use crate::squadratura;
use crate::targhette::naturale;
use crate::util::{errore, imposta_attributi, in_gruppo, testo_param};

/// Il numero del filo di un morsetto (in ingresso, o in uscita se manca).
fn filo(m: &Morsetto) -> String {
    let i = m.blocco.attributo("NUMI").trim();
    if i.is_empty() { m.blocco.attributo("NUMO").trim().to_string() } else { i.to_string() }
}

fn livello(m: &Morsetto) -> u32 {
    m.blocco.attributo("LIVELLO").trim().parse().unwrap_or(1)
}

/// I morsetti di una morsettiera nell'ordine attuale: prima i numeri, poi gli altri nomi (`PE`,
/// `N`) in ordine naturale; a parità di numero, per livello.
pub fn in_ordine(s: &mut Session, morsettiera: &str) -> Result<Vec<Morsetto>> {
    let mut v: Vec<Morsetto> = morsetti::morsetti(s)?.into_iter().filter(|m| m.morsettiera.trim().eq_ignore_ascii_case(morsettiera.trim())).collect();
    let chiave = |m: &Morsetto| (m.numero.trim().parse::<u64>().is_err(), naturale(m.numero.trim()), livello(m));
    v.sort_by_key(chiave);
    Ok(v)
}

/// Le morsettiere (e i connettori) del disegno, in ordine naturale.
pub fn nomi(s: &mut Session) -> Result<Vec<String>> {
    let mut v: Vec<String> = Vec::new();
    for m in morsetti::morsetti(s)? {
        if !v.iter().any(|x| x.trim().eq_ignore_ascii_case(m.morsettiera.trim())) {
            v.push(m.morsettiera.trim().to_string());
        }
    }
    v.sort_by_key(|n| naturale(n));
    Ok(v)
}

/// Come rinumerare una morsettiera.
#[derive(Clone, Debug, PartialEq)]
pub enum Criterio {
    /// L'ordine in cui i morsetti stanno nello schema: foglio, da sinistra a destra, dall'alto.
    Schema,
    /// L'ordine dei numeri dei fili.
    Fili,
    /// L'ordine di adesso, con i numeri compattati.
    Attuale,
    /// L'ordine dato con i numeri di adesso; quelli non elencati vengono dopo, nell'ordine di adesso.
    Manuale(Vec<String>),
}

impl Criterio {
    fn da_param(p: &Value) -> Result<Self> {
        if let Some(a) = p.get("numeri").and_then(Value::as_array) {
            return Ok(Criterio::Manuale(a.iter().filter_map(|v| v.as_str().map(str::to_string).or_else(|| v.as_u64().map(|n| n.to_string()))).collect()));
        }
        match testo_param(p, "ordine").unwrap_or_else(|| "attuale".into()).trim().to_ascii_lowercase().as_str() {
            "schema" => Ok(Criterio::Schema),
            "fili" | "filo" => Ok(Criterio::Fili),
            "attuale" | "compatta" => Ok(Criterio::Attuale),
            o => Err(errore(format!("Ordine sconosciuto: {o} (schema, fili, attuale, o l'elenco dei numeri)"))),
        }
    }
}

/// Un gruppo di morsetti con lo stesso numero (i livelli di un morsetto multiplo).
struct Gruppo {
    numero: String,
    membri: Vec<Morsetto>,
}

/// La posizione nello schema: foglio, ascissa, ordinata dall'alto.
fn chiave_schema(m: &Morsetto) -> (u32, i64, i64) {
    (m.foglio, (m.blocco.pos.0 * 100.0).round() as i64, (-m.blocco.pos.1 * 100.0).round() as i64)
}

/// Rinumera la morsettiera secondo `criterio`, dal numero `da` con passo `passo`. Restituisce
/// (numero vecchio, numero nuovo) dei morsetti cambiati.
pub fn rinumera(s: &mut Session, morsettiera: &str, criterio: &Criterio, da: u64, passo: u64) -> Result<Vec<(String, String)>> {
    let tutti = in_ordine(s, morsettiera)?;
    if tutti.is_empty() {
        return Err(errore(format!("La morsettiera {morsettiera} non ha morsetti")));
    }
    let mut gruppi: Vec<Gruppo> = Vec::new();
    for m in tutti {
        match gruppi.iter_mut().find(|g| g.numero.eq_ignore_ascii_case(m.numero.trim())) {
            Some(g) => g.membri.push(m),
            None => gruppi.push(Gruppo { numero: m.numero.trim().to_string(), membri: vec![m] }),
        }
    }
    match criterio {
        Criterio::Attuale => {}
        Criterio::Schema => gruppi.sort_by_key(|g| g.membri.iter().map(chiave_schema).min()),
        Criterio::Fili => gruppi.sort_by(|a, b| {
            let chiave = |g: &Gruppo| {
                let f = g.membri.iter().map(filo).find(|f| !f.is_empty());
                (f.is_none(), f.map(|f| naturale(&f)).unwrap_or_default(), g.membri.iter().map(chiave_schema).min())
            };
            chiave(a).cmp(&chiave(b))
        }),
        Criterio::Manuale(numeri) => {
            if let Some(n) = numeri.iter().find(|n| !gruppi.iter().any(|g| g.numero.eq_ignore_ascii_case(n.trim()))) {
                return Err(errore(format!("La morsettiera {morsettiera} non ha il morsetto {n}")));
            }
            let posto = |g: &Gruppo| numeri.iter().position(|n| n.trim().eq_ignore_ascii_case(&g.numero)).unwrap_or(usize::MAX);
            // stabile: quelli non elencati restano nell'ordine di adesso, dopo
            gruppi.sort_by_key(posto);
        }
    }
    let passo = passo.max(1);
    let mut prossimo = da;
    let mut cambiati = Vec::new();
    for g in &gruppi {
        // i nomi che non sono numeri restano come sono
        if g.numero.parse::<u64>().is_err() {
            continue;
        }
        let nuovo = prossimo.to_string();
        prossimo += passo;
        if nuovo != g.numero {
            for m in &g.membri {
                imposta_attributi(s, &m.blocco.handle, &[("NUMM", nuovo.as_str())])?;
            }
            cambiati.push((g.numero.clone(), nuovo));
        }
    }
    Ok(cambiati)
}

/// Rinomina la morsettiera `da` in `a`. Se `a` esiste già serve `unisci`: i morsetti numerati
/// di `da` continuano dopo l'ultimo di `a`.
pub fn rinomina(s: &mut Session, da: &str, a: &str, unisci: bool) -> Result<usize> {
    let a = a.trim();
    if a.is_empty() {
        return Err(errore("Indicare il nuovo nome della morsettiera"));
    }
    let vecchi = in_ordine(s, da)?;
    if vecchi.is_empty() {
        return Err(errore(format!("La morsettiera {da} non ha morsetti")));
    }
    if da.trim().eq_ignore_ascii_case(a) {
        // solo maiuscole e minuscole
        for m in &vecchi {
            imposta_attributi(s, &m.blocco.handle, &[("MORSETTIERA", a)])?;
        }
        return Ok(vecchi.len());
    }
    let esistenti = in_ordine(s, a)?;
    if !esistenti.is_empty() && !unisci {
        return Err(errore(format!("La morsettiera {a} esiste già: per unirle usa unisci")));
    }
    let mut prossimo = esistenti.iter().filter_map(|m| m.numero.trim().parse::<u64>().ok()).max().unwrap_or(0) + 1;
    let mut nuovi: Vec<(String, String)> = Vec::new();
    for m in &vecchi {
        let mut valori = vec![("MORSETTIERA", a.to_string())];
        if !esistenti.is_empty() && m.numero.trim().parse::<u64>().is_ok() {
            // i livelli dello stesso morsetto restano insieme
            let n = match nuovi.iter().find(|(v, _)| v == m.numero.trim()) {
                Some((_, n)) => n.clone(),
                None => {
                    let n = prossimo.to_string();
                    prossimo += 1;
                    nuovi.push((m.numero.trim().to_string(), n.clone()));
                    n
                }
            };
            valori.push(("NUMM", n));
        }
        let coppie: Vec<(&str, &str)> = valori.iter().map(|(t, v)| (*t, v.as_str())).collect();
        imposta_attributi(s, &m.blocco.handle, &coppie)?;
    }
    Ok(vecchi.len())
}

fn morsettiera_param(p: &Value) -> Result<String> {
    testo_param(p, "morsettiera").filter(|m| !m.trim().is_empty()).ok_or_else(|| errore("Indicare la morsettiera (es. -XG)"))
}

fn numero_param(p: &Value, k: &str, default: u64) -> Result<u64> {
    match p.get(k) {
        None | Some(Value::Null) => Ok(default),
        Some(v) => v.as_u64().or_else(|| v.as_str().and_then(|t| t.trim().parse().ok())).ok_or_else(|| errore(format!("{k}: numero intero positivo"))),
    }
}

fn run_elenco(s: &mut Session, p: &Value) -> Result<Value> {
    let solo = testo_param(p, "morsettiera").filter(|m| !m.trim().is_empty());
    let mut out = Vec::new();
    for nome in nomi(s)?.into_iter().filter(|n| solo.as_ref().is_none_or(|x| x.trim().eq_ignore_ascii_case(n))) {
        let v = in_ordine(s, &nome)?;
        let connettore = v.iter().all(|m| m.blocco.nome.eq_ignore_ascii_case(morsetti::BLOCCO_CONNETTORE));
        let righe: Vec<Value> = v
            .iter()
            .map(|m| {
                let a = |t: &str| m.blocco.attributo(t).to_string();
                json!({
                    "handle": m.blocco.handle, "numero": m.numero, "livello": livello(m), "filo": filo(m), "ponte": a("PONTE"),
                    "foglio": m.foglio, "x": m.blocco.pos.0, "y": m.blocco.pos.1, "posizione": squadratura::riferimento(m.foglio, m.blocco.pos.0, m.blocco.pos.1),
                    "quadro": a("QUADRO"), "sezione": a("SEZIONE"), "tipo": a("TIPO"), "costruttore": a("COSTRUTTORE"), "utenza": a("UTENZA"), "sesso": a("SESSO"),
                })
            })
            .collect();
        out.push(json!({"morsettiera": nome, "connettore": connettore, "morsetti": righe}));
    }
    let msg = format!("{} morsettiere e connettori", out.len());
    Ok(json!({"morsettiere": out, "message": msg}))
}

fn run_ordine(s: &mut Session, p: &Value) -> Result<Value> {
    let nome = morsettiera_param(p)?;
    let criterio = Criterio::da_param(p)?;
    let (da, passo) = (numero_param(p, "da", 1)?, numero_param(p, "passo", 1)?);
    let cambiati = in_gruppo(s, "Rinumera morsettiera", |s| rinumera(s, &nome, &criterio, da, passo))?;
    let msg = if cambiati.is_empty() { format!("{nome}: numeri già in ordine") } else { format!("{nome}: {} morsetti rinumerati", cambiati.len()) };
    Ok(json!({"cambiati": cambiati.iter().map(|(v, n)| json!([v, n])).collect::<Vec<_>>(), "message": msg}))
}

fn run_rinomina(s: &mut Session, p: &Value) -> Result<Value> {
    let da = morsettiera_param(p)?;
    let a = testo_param(p, "nome").unwrap_or_default();
    let unisci = p.get("unisci").and_then(Value::as_bool).unwrap_or(false);
    let n = in_gruppo(s, "Rinomina morsettiera", |s| rinomina(s, &da, &a, unisci))?;
    Ok(json!({"morsetti": n, "message": format!("{da} → {}: {n} morsetti", a.trim())}))
}

pub fn comandi() -> Vec<CommandSpec> {
    let spec = |id, label, menu, params, run| CommandSpec { id, label, menu, shortcut: None, aliases: &[], params, enabled: has_doc, run, undoable: false, interactive: None, transparent: false };
    vec![
        spec(
            "morsettiera.elenco",
            "Elenco morsettiere",
            &[],
            "{morsettiera?} → {morsettiere: [{morsettiera, connettore, morsetti: [{handle, numero, livello, filo, ponte, foglio, posizione, quadro, sezione, tipo…}]}]}",
            run_elenco as cadcraft_engine::cmd::Run,
        ),
        spec(
            "morsettiera.ordine",
            "Rinumera morsettiera",
            &["Elettrico", "Morsettiere", "Rinumera…"],
            "{morsettiera, ordine?: schema|fili|attuale (default), numeri?: [numeri di adesso nell'ordine voluto], da?: 1, passo?: 1} → {cambiati: [[vecchio, nuovo]]}",
            run_ordine,
        ),
        spec("morsettiera.rinomina", "Rinomina morsettiera", &["Elettrico", "Morsettiere", "Rinomina…"], "{morsettiera, nome: nuovo nome, unisci?: bool (in una morsettiera che esiste)}", run_rinomina),
    ]
}
