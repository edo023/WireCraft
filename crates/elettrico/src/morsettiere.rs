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

// ---------- disegno della morsettiera ----------

use crate::collegamenti::{Capo, TipoCapo};
use crate::fili::Segmento;
use crate::numerazione::su;
use crate::uscite;

type P = (f64, f64);

fn stesso_punto(a: P, b: P) -> bool {
    (a.0 - b.0).abs() < 0.01 && (a.1 - b.1).abs() < 0.01
}

/// Il punto dove due segmenti si toccano (un estremo dell'uno sull'altro).
fn contatto(a: &Segmento, b: &Segmento) -> Option<P> {
    [b.a, b.b].into_iter().find(|p| su(a, *p)).or_else(|| [a.a, a.b].into_iter().find(|p| su(b, *p)))
}

/// I capi raggiunti dal morsetto in `p` lungo i fili, da una parte (verso l'alto, o verso
/// sinistra su un filo orizzontale: il lato interno) e dall'altra (il lato esterno), nell'ordine
/// in cui si incontrano. Il morsetto non interrompe il filo: lo si divide in `p`.
pub fn lati(segmenti: &[Segmento], capi: &[Capo], p: P) -> (Vec<Capo>, Vec<Capo>) {
    let Some(i) = segmenti.iter().position(|f| su(f, p)) else { return (Vec::new(), Vec::new()) };
    let f = &segmenti[i];
    let verticale = (f.a.0 - f.b.0).abs() < 0.01;
    let (primo, secondo) = if verticale {
        if f.a.1 >= f.b.1 { (f.a, f.b) } else { (f.b, f.a) }
    } else if f.a.0 <= f.b.0 {
        (f.a, f.b)
    } else {
        (f.b, f.a)
    };
    let lato = |fine: P| -> Vec<Capo> {
        if stesso_punto(p, fine) {
            return Vec::new();
        }
        let meta = Segmento { a: p, b: fine, handle: String::new() };
        // in ampiezza dai segmenti toccati, senza ripassare dal morsetto
        let mut visti = vec![false; segmenti.len()];
        visti[i] = true;
        let mut ordine: Vec<Segmento> = vec![meta];
        let mut k = 0;
        while k < ordine.len() {
            let c = ordine[k].clone();
            for (j, g) in segmenti.iter().enumerate() {
                if !visti[j] && contatto(&c, g).is_some_and(|q| !stesso_punto(q, p)) {
                    visti[j] = true;
                    ordine.push(g.clone());
                }
            }
            k += 1;
        }
        let mut v: Vec<Capo> = Vec::new();
        for g in &ordine {
            let mut qui: Vec<&Capo> = capi.iter().filter(|c| su(g, c.pos) && !stesso_punto(c.pos, p) && !v.iter().any(|x| x == *c)).collect();
            qui.sort_by(|a, b| (a.pos.0 - p.0).hypot(a.pos.1 - p.1).total_cmp(&(b.pos.0 - p.0).hypot(b.pos.1 - p.1)));
            v.extend(qui.into_iter().cloned());
        }
        v
    };
    (lato(primo), lato(secondo))
}

fn testo_capi(v: &[Capo]) -> String {
    let t: Vec<String> = v.iter().take(2).map(|c| if c.tipo == TipoCapo::Rimando { format!("→ {}", c.sigla) } else { c.testo() }).collect();
    let mut t = t.join(", ");
    if v.len() > 2 {
        t.push_str(&format!(" +{}", v.len() - 2));
    }
    t
}

/// Un morsetto con quello che ha collegato da una parte e dall'altra.
#[derive(Clone, Debug, PartialEq)]
pub struct Collegato {
    pub numero: String,
    pub livello: u32,
    pub ponte: String,
    pub filo: String,
    pub interno: String,
    pub esterno: String,
    pub posizione: String,
    pub pe: bool,
}

/// I morsetti della morsettiera in ordine, con quello che hanno collegato da una parte e
/// dall'altra.
pub fn collegati(s: &mut Session, morsettiera: &str) -> Result<Vec<Collegato>> {
    let v = in_ordine(s, morsettiera)?;
    let mut fogli_m: Vec<u32> = v.iter().map(|m| m.foglio).filter(|f| *f > 0).collect();
    fogli_m.sort_unstable();
    fogli_m.dedup();
    let reti = if fogli_m.is_empty() { Vec::new() } else { crate::collegamenti::reti(s, &fogli_m)? };
    let mut segmenti: Vec<(u32, Vec<Segmento>)> = Vec::new();
    for f in &fogli_m {
        segmenti.push((*f, crate::fili::fili(s, &crate::fogli::nome_layer(*f))?));
    }
    Ok(v
        .iter()
        .map(|m| {
            let capi: Vec<Capo> = reti.iter().filter(|r| r.foglio == m.foglio).flat_map(|r| r.capi.iter().filter(|c| c.blocco != m.blocco.handle).cloned()).collect();
            let seg = segmenti.iter().find(|(f, _)| *f == m.foglio).map_or(&[][..], |(_, s)| s.as_slice());
            let (interno, esterno) = lati(seg, &capi, m.blocco.pos);
            let f = filo(m);
            Collegato {
                numero: m.numero.clone(),
                livello: livello(m),
                ponte: m.blocco.attributo("PONTE").to_string(),
                pe: m.numero.trim().eq_ignore_ascii_case("PE") || f.eq_ignore_ascii_case("PE"),
                filo: f,
                interno: testo_capi(&interno),
                esterno: testo_capi(&esterno),
                posizione: squadratura::riferimento(m.foglio, m.blocco.pos.0, m.blocco.pos.1),
            }
        })
        .collect())
}

/// Larghezza di una colonna (un morsetto) e ascissa della prima, nel disegno.
const COLONNA: f64 = 7.0;
const X_PRIMA: f64 = 45.0;
/// Le fasce del disegno, dall'alto: (inizio, fine) in mm sotto il bordo dell'area.
const INTERNO: (f64, f64) = (-12.0, -72.0);
const FILO: (f64, f64) = (-72.0, -88.0);
const MORSETTO: (f64, f64) = (-88.0, -100.0);
const PONTE: (f64, f64) = (-100.0, -106.0);
const ESTERNO: (f64, f64) = (-106.0, -166.0);

fn colonne_per_pagina() -> usize {
    ((uscite::AREA.0 - X_PRIMA) / COLONNA).floor() as usize
}

/// Una pagina del disegno: la morsettiera, i suoi morsetti, la pagina e quante sono.
struct Pagina {
    nome: String,
    righe: Vec<Collegato>,
    k: usize,
    n: usize,
}

fn pagine(s: &mut Session, morsettiera: Option<&str>) -> Result<Vec<Pagina>> {
    let mut out = Vec::new();
    for nome in nomi(s)?.into_iter().filter(|n| morsettiera.is_none_or(|m| m.trim().eq_ignore_ascii_case(n))) {
        let righe = collegati(s, &nome)?;
        let per = colonne_per_pagina().max(1);
        let n = righe.len().div_ceil(per).max(1);
        for (k, pezzo) in righe.chunks(per).enumerate() {
            out.push(Pagina { nome: nome.clone(), righe: pezzo.to_vec(), k, n });
        }
    }
    Ok(out)
}

fn ruotato(s: &mut Session, h: &mut Vec<String>, at: P, t: &str, alt: f64, come: cadcraft_engine::doc::HAlign) -> Result<()> {
    if t.trim().is_empty() {
        return Ok(());
    }
    crate::util::testo(s, h, at, t, alt, "L")?;
    if let Some(ultimo) = h.last().cloned() {
        crate::util::ruota_testo(s, &ultimo, 90.0, come)?;
    }
    Ok(())
}

fn disegna_pagina(s: &mut Session, p: &Pagina, h: &mut Vec<String>) -> Result<()> {
    use crate::util::{colore, linea, testo};
    use cadcraft_engine::doc::HAlign;
    let fine = X_PRIMA + COLONNA * p.righe.len() as f64;
    colore(s, "7")?;
    let pagina = if p.n > 1 { format!(" ({}/{})", p.k + 1, p.n) } else { String::new() };
    testo(s, h, (uscite::AREA.0 / 2.0, -4.5), &format!("MORSETTIERA {} \\ TERMINAL STRIP {}{pagina}", p.nome, p.nome), 4.0, "MC")?;
    // le fasce con le intestazioni a sinistra
    for (fascia, it, en) in [(INTERNO, "Collegamenti interni", "Internal connections"), (FILO, "Filo", "Wire"), (MORSETTO, "Morsetto", "Terminal"), (PONTE, "Ponte", "Bridge"), (ESTERNO, "Collegamenti esterni", "External connections")] {
        let y = (fascia.0 + fascia.1) / 2.0;
        testo(s, h, (2.0, y + 0.6), it, 2.0, "L")?;
        if fascia.0 - fascia.1 > 8.0 {
            colore(s, "8")?;
            testo(s, h, (2.0, y - 2.6), en, 1.6, "L")?;
            colore(s, "7")?;
        }
        linea(s, h, &[(0.0, fascia.0), (fine, fascia.0)], false)?;
    }
    linea(s, h, &[(0.0, ESTERNO.1), (fine, ESTERNO.1)], false)?;
    linea(s, h, &[(0.0, INTERNO.0), (0.0, ESTERNO.1)], false)?;
    linea(s, h, &[(X_PRIMA, INTERNO.0), (X_PRIMA, ESTERNO.1)], false)?;
    for (i, r) in p.righe.iter().enumerate() {
        let x0 = X_PRIMA + COLONNA * i as f64;
        let x = x0 + COLONNA / 2.0;
        colore(s, "7")?;
        ruotato(s, h, (x + 0.9, INTERNO.1 + 2.0), &r.interno, 1.8, HAlign::Left)?;
        ruotato(s, h, (x + 0.9, FILO.1 + 2.0), &r.filo, 1.8, HAlign::Left)?;
        ruotato(s, h, (x + 0.9, ESTERNO.0 - 2.0), &r.esterno, 1.8, HAlign::Right)?;
        // il morsetto: il PE in verde
        colore(s, if r.pe { "3" } else { "7" })?;
        linea(s, h, &[(x0, MORSETTO.0), (x0 + COLONNA, MORSETTO.0), (x0 + COLONNA, MORSETTO.1), (x0, MORSETTO.1)], true)?;
        let alt = if r.numero.chars().count() > 3 { 1.5 } else { 2.2 };
        testo(s, h, (x, (MORSETTO.0 + MORSETTO.1) / 2.0 + if r.livello > 1 { 1.2 } else { 0.0 }), &r.numero, alt, "MC")?;
        if r.livello > 1 {
            testo(s, h, (x, MORSETTO.1 + 2.0), &format!("L{}", r.livello), 1.4, "MC")?;
        }
    }
    // i ponticelli: una linea con un punto su ogni morsetto del gruppo
    colore(s, "1")?;
    let y = (PONTE.0 + PONTE.1) / 2.0;
    let mut gruppi: Vec<&str> = p.righe.iter().map(|r| r.ponte.as_str()).filter(|g| !g.trim().is_empty()).collect();
    gruppi.sort_unstable();
    gruppi.dedup();
    for g in gruppi {
        let xs: Vec<f64> = p.righe.iter().enumerate().filter(|(_, r)| r.ponte == g).map(|(i, _)| X_PRIMA + COLONNA * (i as f64 + 0.5)).collect();
        if let (Some(a), Some(b)) = (xs.first(), xs.last()) {
            linea(s, h, &[(*a, y), (*b, y)], false)?;
        }
        for x in xs {
            h.extend(crate::util::handles(&crate::util::esegui(s, "circle", json!({"center": [x, y], "radius": 0.6}))?));
        }
    }
    colore(s, "7")
}

/// Disegna le morsettiere (tutte, o `morsettiera`) dal foglio `primo` (default dopo l'ultimo).
pub fn uscita(s: &mut Session, p: &Value, primo: Option<u32>) -> Result<Vec<u32>> {
    let solo = testo_param(p, "morsettiera").filter(|m| !m.trim().is_empty());
    let v = pagine(s, solo.as_deref())?;
    if v.is_empty() {
        return Err(errore("Nessuna morsettiera da disegnare"));
    }
    let mut par = serde_json::Map::new();
    if let Some(m) = &solo {
        par.insert("morsettiera".into(), json!(m));
    }
    uscite::disegna_con(s, v.len(), "disegnomorsettiere", &Value::Object(par), primo, "Disegno morsettiere", "Terminal strips", |s, k, h| match v.get(k) {
        Some(pg) => disegna_pagina(s, pg, h),
        None => Ok(()),
    })
}

/// La stessa cosa in tabella (Excel).
pub fn tabella(s: &mut Session, p: &Value) -> Result<uscite::Tabella> {
    let solo = testo_param(p, "morsettiera").filter(|m| !m.trim().is_empty());
    let mut righe = Vec::new();
    for nome in nomi(s)?.into_iter().filter(|n| solo.as_ref().is_none_or(|m| m.trim().eq_ignore_ascii_case(n))) {
        for r in collegati(s, &nome)? {
            righe.push([nome.clone(), r.numero, r.livello.to_string(), r.ponte, r.filo, r.interno, r.esterno, r.posizione].into_iter().map(uscite::Cella::T).collect());
        }
    }
    let col = |v: &[(&str, f64)]| v.iter().map(|(t, w)| ((*t).to_string(), *w)).collect::<Vec<_>>();
    Ok(uscite::Tabella {
        titolo: "DISEGNO MORSETTIERE \\ TERMINAL STRIPS".into(),
        colonne: col(&[
            ("Morsettiera\nTerminal strip", 26.0),
            ("Morsetto\nTerminal", 18.0),
            ("Liv.\nLevel", 12.0),
            ("Ponte\nBridge", 14.0),
            ("Filo\nWire", 20.0),
            ("Collegamento interno\nInternal connection", 90.0),
            ("Collegamento esterno\nExternal connection", 90.0),
            ("Posizione\nPosition", 22.0),
        ]),
        righe,
        altezza_riga: 6.0,
    })
}
