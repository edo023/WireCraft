//! Gestione del multifoglio: aggiungere, inserire, eliminare, svuotare, rinominare e spostare
//! (shift) i fogli, compattare la numerazione; copiare e spostare oggetti tra fogli; fogli
//! memorizzati (pagine standard salvate in una libreria e reinserite).
//!
//! Dopo ogni cambio di numerazione si riscrivono i numeri dei fogli e i riferimenti di
//! contatti, specchietti e rimandi; numeri dei fili e sigle che contengono il foglio restano da
//! rifare (`NUMERAFILI`, `SIGLECOMPONENTI`), come avvisa il messaggio.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use cadcraft_engine::cmd::{always, has_doc};
use cadcraft_engine::{CommandSpec, Result, Session};
use serde_json::{Value, json};

use crate::fogli::{self, nome_layer};
use crate::util::{errore, esegui, handles, in_gruppo, layer_corrente, numero_param, punto_param, testo_param};
use crate::{fili, siglatura};

/// Gli oggetti di un foglio (handle).
pub fn contenuto(s: &Session, layer: &str) -> Result<Vec<String>> {
    Ok(s.doc()?.model.iter().filter(|e| e.common.layer.eq_ignore_ascii_case(layer)).map(|e| e.handle.hex()).collect())
}

/// I numeri di foglio da un parametro: lista di numeri, numero, o testo come "3-5,8".
fn fogli_param(p: &Value, chiave: &str) -> Result<Vec<u32>> {
    let numero = |v: &Value| v.as_u64().and_then(|n| u32::try_from(n).ok()).or_else(|| v.as_str()?.trim().parse().ok());
    let mut v: Vec<u32> = match p.get(chiave) {
        Some(Value::Array(a)) => a.iter().filter_map(numero).collect(),
        Some(Value::Number(_)) => p.get(chiave).and_then(numero).into_iter().collect(),
        Some(Value::String(t)) => {
            let mut out = Vec::new();
            for parte in t.split([',', ';', ' ']).filter(|x| !x.trim().is_empty()) {
                let n = |x: &str| x.trim().parse::<u32>().map_err(|_| errore(format!("Elenco di fogli non valido: {t}")));
                match parte.split_once('-') {
                    Some((a, b)) => {
                        let (a, b) = (n(a)?, n(b)?);
                        out.extend(a.min(b)..=a.max(b));
                    }
                    None => out.push(n(parte)?),
                }
            }
            out
        }
        _ => Vec::new(),
    };
    v.sort_unstable();
    v.dedup();
    Ok(v)
}

/// Cancella il contenuto dei fogli dati (numeri, titolo e schema). Restituisce gli oggetti tolti.
pub fn svuota(s: &mut Session, fogli: &[u32]) -> Result<usize> {
    let mut n = 0;
    for f in fogli {
        let h = contenuto(s, &nome_layer(*f))?;
        n += h.len();
        if !h.is_empty() {
            esegui(s, "erase", json!({"handles": h}))?;
        }
    }
    Ok(n)
}

/// Elimina i fogli dati (layer e contenuto). Se tra questi c'è il foglio corrente si va al
/// foglio rimasto più vicino.
pub fn elimina(s: &mut Session, fogli: &[u32]) -> Result<()> {
    let tutti = fogli::elenco(s)?;
    for n in fogli {
        if !tutti.iter().any(|f| f.numero == *n) {
            return Err(errore(format!("Il foglio {n} non esiste")));
        }
    }
    let corrente = layer_corrente(s)?;
    if let Some(c) = fogli.iter().find(|n| nome_layer(**n) == corrente) {
        let restano: Vec<u32> = tutti.iter().map(|f| f.numero).filter(|n| !fogli.contains(n)).collect();
        let vicino = restano.iter().copied().filter(|n| n < c).max().or_else(|| restano.first().copied());
        match vicino {
            Some(n) => {
                fogli::vai(s, n)?;
            }
            None => {
                esegui(s, "layer.current", json!({"name": "0"}))?;
            }
        }
    }
    svuota(s, fogli)?;
    for n in fogli {
        esegui(s, "layer.delete", json!({"name": nome_layer(*n)}))?;
    }
    Ok(())
}

/// Cambia i numeri dei fogli secondo `mappa` (da, a) in un colpo solo: i numeri d'arrivo non
/// devono essere di altri fogli che restano.
pub fn rinumera(s: &mut Session, mappa: &[(u32, u32)]) -> Result<()> {
    let mappa: Vec<(u32, u32)> = mappa.iter().copied().filter(|(a, b)| a != b).collect();
    if mappa.is_empty() {
        return Ok(());
    }
    let tutti: Vec<u32> = fogli::elenco(s)?.iter().map(|f| f.numero).collect();
    for (da, a) in &mappa {
        if !tutti.contains(da) {
            return Err(errore(format!("Il foglio {da} non esiste")));
        }
        if *a == 0 {
            return Err(errore("Il foglio 0 è quello del cartiglio"));
        }
        if tutti.contains(a) && !mappa.iter().any(|(d, _)| d == a) {
            return Err(errore(format!("Il foglio {a} esiste già")));
        }
    }
    let arrivi: std::collections::HashSet<u32> = mappa.iter().map(|(_, a)| *a).collect();
    if arrivi.len() != mappa.len() {
        return Err(errore("Due fogli finirebbero sullo stesso numero"));
    }
    // prima su nomi provvisori, così gli scambi di numero non si scontrano
    for (da, _) in &mappa {
        esegui(s, "layer.set", json!({"name": nome_layer(*da), "newName": format!("CE_TMP_{da}")}))?;
    }
    for (da, a) in &mappa {
        esegui(s, "layer.set", json!({"name": format!("CE_TMP_{da}"), "newName": nome_layer(*a)}))?;
    }
    Ok(())
}

/// Dopo un cambio di numerazione: numeri dei fogli nei cartigli, riferimenti di contatti,
/// specchietti e rimandi.
fn riallinea(s: &mut Session) -> Result<()> {
    fogli::numera(s)?;
    siglatura::aggiorna_tutto(s)?;
    fili::aggiorna_rimandi(s)?;
    Ok(())
}

const AVVISO: &str = "Controllare numeri dei fili e sigle che contengono il foglio (Numera Fili, Siglatura Componenti)";

/// Sposta il gruppo di fogli `gruppo` di `posizioni` (avanti se positivo): gli altri fogli non
/// cambiano e i numeri d'arrivo devono essere liberi.
pub fn shift_gruppo(s: &mut Session, gruppo: &[u32], posizioni: i64) -> Result<Vec<(u32, u32)>> {
    let mappa: Vec<(u32, u32)> = gruppo
        .iter()
        .map(|&n| {
            let a = i64::from(n) + posizioni;
            u32::try_from(a).ok().filter(|a| *a > 0).map(|a| (n, a)).ok_or_else(|| errore(format!("Shift non possibile: il foglio {n} finirebbe sul numero {a}")))
        })
        .collect::<Result<_>>()?;
    rinumera(s, &mappa)?;
    riallinea(s)?;
    Ok(mappa)
}

/// Sposta di `posizioni` tutti i fogli a partire dal numero `da`.
pub fn shift_da(s: &mut Session, da: u32, posizioni: i64) -> Result<Vec<(u32, u32)>> {
    let gruppo: Vec<u32> = fogli::elenco(s)?.iter().map(|f| f.numero).filter(|n| *n >= da).collect();
    shift_gruppo(s, &gruppo, posizioni)
}

/// Numera i fogli in modo consecutivo a partire dal più basso.
pub fn compatta(s: &mut Session) -> Result<Vec<(u32, u32)>> {
    let tutti: Vec<u32> = fogli::elenco(s)?.iter().map(|f| f.numero).collect();
    let Some(&primo) = tutti.first() else { return Ok(Vec::new()) };
    let mappa: Vec<(u32, u32)> = tutti.iter().zip(primo..).map(|(&n, k)| (n, k)).filter(|(a, b)| a != b).collect();
    rinumera(s, &mappa)?;
    riallinea(s)?;
    Ok(mappa)
}

/// Inserisce un foglio vuoto con il numero `n`, spostando avanti di uno i fogli da `n` in poi.
pub fn inserisci_foglio(s: &mut Session, n: u32) -> Result<()> {
    if n == 0 {
        return Err(errore("Il foglio 0 è quello del cartiglio"));
    }
    if fogli::elenco(s)?.iter().any(|f| f.numero == n) {
        let gruppo: Vec<u32> = fogli::elenco(s)?.iter().map(|f| f.numero).filter(|k| *k >= n).collect();
        let mappa: Vec<(u32, u32)> = gruppo.iter().map(|&k| (k, k + 1)).collect();
        rinumera(s, &mappa)?;
    }
    fogli::crea(s, n)?;
    riallinea(s)?;
    Ok(())
}

/// Cambia il numero di un foglio.
pub fn rinomina(s: &mut Session, da: u32, a: u32) -> Result<()> {
    rinumera(s, &[(da, a)])?;
    riallinea(s)
}

/// Copia gli oggetti `h` (dello stesso punto) nei fogli `fogli`; le madri copiate prendono una
/// sigla nuova sul foglio d'arrivo e i loro contatti copiati con loro le seguono. Restituisce
/// i nuovi oggetti.
pub fn copia_tra_fogli(s: &mut Session, h: &[String], fogli: &[u32]) -> Result<Vec<String>> {
    let esistenti: Vec<u32> = fogli::elenco(s)?.iter().map(|f| f.numero).collect();
    // con una madre si copia anche il suo specchietto
    let componenti = siglatura::componenti(s)?;
    let scelto = |x: &str| h.iter().any(|y| y.eq_ignore_ascii_case(x));
    let specchietti: Vec<String> = componenti
        .iter()
        .filter(|r| r.pres.eq_ignore_ascii_case("R") && !scelto(&r.blocco.handle))
        .filter(|r| componenti.iter().any(|m| m.madre() && scelto(&m.blocco.handle) && r.blocco.attributo("MADRE").eq_ignore_ascii_case(&m.nome)))
        .map(|r| r.blocco.handle.clone())
        .collect();
    let h = &[h, &specchietti[..]].concat();
    let mut tutti = Vec::new();
    for &n in fogli {
        if !esistenti.contains(&n) {
            return Err(errore(format!("Il foglio {n} non esiste")));
        }
        let r = esegui(s, "copy", json!({"handles": h, "delta": [0.0, 0.0]}))?;
        let nuovi = handles(&r);
        if nuovi.is_empty() {
            continue;
        }
        esegui(s, "laymch", json!({"handles": nuovi, "layer": nome_layer(n)}))?;
        risigla_copie(s, &nuovi, n)?;
        tutti.extend(nuovi);
    }
    siglatura::aggiorna_tutto(s)?;
    fili::aggiorna_rimandi(s)?;
    Ok(tutti)
}

/// Dà sigle nuove alle madri tra gli oggetti `nuovi` (copiati sul foglio `n`) e le porta sui
/// contatti e specchietti copiati con loro.
pub fn risigla_copie(s: &mut Session, nuovi: &[String], n: u32) -> Result<()> {
    let tutti = siglatura::componenti(s)?;
    let copiati: Vec<&siglatura::Componente> = tutti.iter().filter(|c| nuovi.iter().any(|h| h.eq_ignore_ascii_case(&c.blocco.handle))).collect();
    let mut altri: Vec<siglatura::Componente> = tutti.iter().filter(|c| !nuovi.iter().any(|h| h.eq_ignore_ascii_case(&c.blocco.handle))).cloned().collect();
    let mut madri: Vec<&&siglatura::Componente> = copiati.iter().filter(|c| c.madre()).collect();
    madri.sort_by_key(|c| ((c.blocco.pos.0 * 1000.0).round() as i64, -(c.blocco.pos.1 * 1000.0).round() as i64));
    for m in madri {
        let Some(p) = siglatura::prefisso_di(&m.nome) else { continue };
        let nuova = siglatura::nuova_sigla(&altri, &p, n, m.blocco.pos);
        esegui(s, "attedit", json!({"handle": m.blocco.handle, "values": {"NOME": nuova}}))?;
        for c in copiati.iter().filter(|c| !c.madre()) {
            if c.figlio() && c.nome.eq_ignore_ascii_case(&m.nome) {
                esegui(s, "attedit", json!({"handle": c.blocco.handle, "values": {"NOME": nuova}}))?;
            }
            if c.blocco.attributo("MADRE").eq_ignore_ascii_case(&m.nome) {
                esegui(s, "attedit", json!({"handle": c.blocco.handle, "values": {"MADRE": nuova}}))?;
            }
        }
        let mut copia = (*m).clone();
        copia.nome = nuova;
        copia.foglio = n;
        altri.push(copia);
    }
    Ok(())
}

/// Sposta gli oggetti `h` sul foglio `n` (stessa posizione).
pub fn sposta_tra_fogli(s: &mut Session, h: &[String], n: u32) -> Result<()> {
    if !fogli::elenco(s)?.iter().any(|f| f.numero == n) {
        return Err(errore(format!("Il foglio {n} non esiste")));
    }
    esegui(s, "laymch", json!({"handles": h, "layer": nome_layer(n)}))?;
    siglatura::aggiorna_tutto(s)?;
    fili::aggiorna_rimandi(s)?;
    Ok(())
}

// ---------- fogli memorizzati ----------

static LIBRERIA_FOGLI: Mutex<Option<PathBuf>> = Mutex::new(None);

/// La cartella dei fogli memorizzati (sottocartelle = categorie).
pub fn imposta_libreria(dir: &Path) {
    if let Ok(mut g) = LIBRERIA_FOGLI.lock() {
        *g = Some(dir.to_path_buf());
    }
}

pub fn libreria() -> Option<PathBuf> {
    LIBRERIA_FOGLI.lock().ok().and_then(|g| g.clone())
}

#[derive(Clone, Debug, PartialEq)]
pub struct FoglioMemorizzato {
    pub nome: String,
    pub categoria: String,
    pub descrizione: String,
    pub file: PathBuf,
}

/// Un nome valido per un file.
fn nome_file(t: &str) -> Result<String> {
    let t = t.trim();
    if t.is_empty() || t.chars().any(|c| matches!(c, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*')) || t.starts_with('.') {
        return Err(errore(format!("Nome non valido: {t:?}")));
    }
    Ok(t.to_string())
}

/// I fogli memorizzati nella libreria, per categoria e nome.
pub fn memorizzati() -> Vec<FoglioMemorizzato> {
    let Some(dir) = libreria() else { return Vec::new() };
    let mut v = Vec::new();
    let mut leggi = |cartella: &Path, categoria: &str| {
        let Ok(rd) = std::fs::read_dir(cartella) else { return };
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().is_some_and(|x| x.eq_ignore_ascii_case("dxf")) {
                let nome = p.file_stem().map(|x| x.to_string_lossy().to_string()).unwrap_or_default();
                let descrizione = std::fs::read_to_string(p.with_extension("txt")).unwrap_or_default().trim().to_string();
                v.push(FoglioMemorizzato { nome, categoria: categoria.to_string(), descrizione, file: p });
            }
        }
    };
    leggi(&dir, "");
    if let Ok(rd) = std::fs::read_dir(&dir) {
        for e in rd.flatten() {
            if e.path().is_dir() {
                leggi(&e.path(), &e.file_name().to_string_lossy());
            }
        }
    }
    v.sort_by(|a, b| a.categoria.cmp(&b.categoria).then_with(|| a.nome.to_lowercase().cmp(&b.nome.to_lowercase())));
    v
}

fn run_memorizza(s: &mut Session, p: &Value) -> Result<Value> {
    let nome = nome_file(&testo_param(p, "nome").unwrap_or_default())?;
    let categoria = testo_param(p, "categoria").filter(|c| !c.trim().is_empty()).map(|c| nome_file(&c)).transpose()?;
    let dir = match testo_param(p, "cartella") {
        Some(c) => PathBuf::from(c),
        None => libreria().ok_or_else(|| errore("Libreria dei fogli memorizzati non impostata"))?,
    };
    let dir = match &categoria {
        Some(c) => dir.join(c),
        None => dir,
    };
    let h: Vec<String> = match p.get("handles").and_then(Value::as_array) {
        Some(a) => a.iter().filter_map(Value::as_str).map(str::to_string).collect(),
        None => {
            let sel: Vec<String> = s.selection().iter().map(|h| h.hex()).collect();
            if sel.is_empty() {
                // il foglio intero, senza i suoi numeri e il titolo: sono del foglio d'arrivo
                let layer = layer_corrente(s)?;
                let d = s.doc()?;
                let suo = |h: &String| {
                    let e = cadcraft_engine::doc::Handle::parse_hex(h).and_then(|h| d.entity(h));
                    !e.is_some_and(|e| matches!(&e.kind, cadcraft_engine::doc::EntityKind::Insert(i) if { let n = i.block.to_ascii_uppercase(); n.starts_with("NUMFG") || n.starts_with("TITOLO") }))
                };
                contenuto(s, &layer)?.into_iter().filter(suo).collect()
            } else {
                sel
            }
        }
    };
    if h.is_empty() {
        return Err(errore("Niente da memorizzare: il foglio è vuoto"));
    }
    std::fs::create_dir_all(&dir).map_err(|e| errore(format!("{}: {e}", dir.display())))?;
    let file = dir.join(format!("{nome}.dxf"));
    esegui(s, "wblock", json!({"path": file.to_string_lossy(), "handles": h}))?;
    if let Some(d) = testo_param(p, "descrizione") {
        std::fs::write(file.with_extension("txt"), d).map_err(|e| errore(format!("{}: {e}", file.display())))?;
    }
    Ok(json!({"file": file.to_string_lossy(), "oggetti": h.len(), "message": format!("Foglio memorizzato: {nome} ({} oggetti)", h.len())}))
}

fn run_inserisci_memorizzato(s: &mut Session, p: &Value) -> Result<Value> {
    let file = match testo_param(p, "file") {
        Some(f) => PathBuf::from(f),
        None => {
            let nome = testo_param(p, "nome").ok_or_else(|| errore("Indicare il foglio memorizzato (nome) o il file"))?;
            let cat = testo_param(p, "categoria").unwrap_or_default();
            memorizzati()
                .into_iter()
                .find(|m| m.nome.eq_ignore_ascii_case(&nome) && (cat.is_empty() || m.categoria.eq_ignore_ascii_case(&cat)))
                .map(|m| m.file)
                .ok_or_else(|| errore(format!("Foglio memorizzato non trovato: {nome}")))?
        }
    };
    let at = punto_param(p, "at").unwrap_or((0.0, 0.0));
    let n = fogli::corrente(s)?.map(|f| f.numero).ok_or_else(|| errore("Nessun foglio corrente"))?;
    in_gruppo(s, "Inserisci foglio memorizzato", |s| {
        let nome = format!("CE_FM_{}", file.file_stem().map(|x| x.to_string_lossy().to_string()).unwrap_or_default().replace(' ', "_"));
        let r = esegui(s, "insert", json!({"name": format!("*{nome}"), "file": file.to_string_lossy(), "at": [at.0, at.1]}))?;
        let nuovi = handles(&r);
        if !nuovi.is_empty() {
            esegui(s, "laymch", json!({"handles": nuovi, "layer": nome_layer(n)}))?;
            risigla_copie(s, &nuovi, n)?;
        }
        siglatura::aggiorna_tutto(s)?;
        fili::aggiorna_rimandi(s)?;
        Ok(json!({"handles": nuovi, "message": format!("Inseriti {} oggetti da {}", nuovi.len(), file.display())}))
    })
}

fn run_elenco_memorizzati(_s: &mut Session, _p: &Value) -> Result<Value> {
    let v = memorizzati();
    let righe: Vec<String> = v.iter().map(|m| format!("{}{}  {}", if m.categoria.is_empty() { String::new() } else { format!("{}/", m.categoria) }, m.nome, m.descrizione)).collect();
    Ok(json!({
        "fogli": v.iter().map(|m| json!({"nome": m.nome, "categoria": m.categoria, "descrizione": m.descrizione, "file": m.file.to_string_lossy()})).collect::<Vec<_>>(),
        "message": if righe.is_empty() { "Nessun foglio memorizzato".to_string() } else { righe.join("\n") },
    }))
}

// ---------- comandi ----------

fn esito(mappa: &[(u32, u32)], cosa: &str) -> Value {
    let elenco: Vec<String> = mappa.iter().map(|(a, b)| format!("{a}→{b}")).collect();
    json!({"cambi": mappa, "message": if mappa.is_empty() { format!("{cosa}: niente da cambiare") } else { format!("{cosa}: {}. {AVVISO}", elenco.join(", ")) }})
}

fn run_elimina(s: &mut Session, p: &Value) -> Result<Value> {
    let f = fogli_param(p, "fogli")?;
    if f.is_empty() {
        return Err(errore("Indicare i fogli da eliminare"));
    }
    in_gruppo(s, "Elimina fogli", |s| {
        elimina(s, &f)?;
        fogli::numera(s)?;
        siglatura::aggiorna_tutto(s)?;
        fili::aggiorna_rimandi(s)?;
        Ok(json!({"fogli": f, "message": format!("Eliminati {} fogli", f.len())}))
    })
}

fn run_svuota(s: &mut Session, p: &Value) -> Result<Value> {
    let f = fogli_param(p, "fogli")?;
    if f.is_empty() {
        return Err(errore("Indicare i fogli da svuotare"));
    }
    in_gruppo(s, "Svuota fogli", |s| {
        let n = svuota(s, &f)?;
        fogli::numera(s)?;
        siglatura::aggiorna_tutto(s)?;
        fili::aggiorna_rimandi(s)?;
        Ok(json!({"oggetti": n, "message": format!("Svuotati {} fogli ({n} oggetti)", f.len())}))
    })
}

fn run_rinomina(s: &mut Session, p: &Value) -> Result<Value> {
    let da = numero_param(p, "da").ok_or_else(|| errore("Indicare il foglio (da)"))?;
    let a = numero_param(p, "a").ok_or_else(|| errore("Indicare il nuovo numero (a)"))?;
    in_gruppo(s, "Rinomina foglio", |s| {
        rinomina(s, da, a)?;
        Ok(esito(&[(da, a)], "Rinomina"))
    })
}

fn run_shift(s: &mut Session, p: &Value) -> Result<Value> {
    let posizioni = p.get("posizioni").and_then(Value::as_i64).ok_or_else(|| errore("Indicare le posizioni (positive avanti, negative indietro)"))?;
    let gruppo = fogli_param(p, "fogli")?;
    in_gruppo(s, "Shift fogli", |s| {
        let mappa = match (gruppo.is_empty(), numero_param(p, "da")) {
            (false, _) => shift_gruppo(s, &gruppo, posizioni)?,
            (true, Some(da)) => shift_da(s, da, posizioni)?,
            (true, None) => return Err(errore("Indicare i fogli (fogli) o il primo foglio da spostare (da)")),
        };
        Ok(esito(&mappa, "Shift"))
    })
}

fn run_compatta(s: &mut Session, _p: &Value) -> Result<Value> {
    in_gruppo(s, "Compatta fogli", |s| compatta(s).map(|m| esito(&m, "Compatta")))
}

fn run_inserisci(s: &mut Session, p: &Value) -> Result<Value> {
    let n = numero_param(p, "foglio").ok_or_else(|| errore("Indicare il numero del nuovo foglio"))?;
    let titolo = testo_param(p, "titolo");
    in_gruppo(s, "Inserisci foglio", |s| {
        inserisci_foglio(s, n)?;
        if let Some(t) = &titolo {
            fogli::imposta_titolo(s, n, t, None)?;
        }
        fogli::vai(s, n)?;
        Ok(json!({"foglio": n, "message": format!("Inserito il foglio {n}. {AVVISO}")}))
    })
}

fn handles_o_selezione(s: &Session, p: &Value) -> Vec<String> {
    match p.get("handles").and_then(Value::as_array) {
        Some(a) => a.iter().filter_map(Value::as_str).map(str::to_string).collect(),
        None => s.selection().iter().map(|h| h.hex()).collect(),
    }
}

fn run_copia_tra_fogli(s: &mut Session, p: &Value) -> Result<Value> {
    let h = handles_o_selezione(s, p);
    let f = fogli_param(p, "fogli")?;
    if h.is_empty() || f.is_empty() {
        return Err(errore("Indicare gli oggetti (handles o selezione) e i fogli d'arrivo"));
    }
    in_gruppo(s, "Copia tra fogli", |s| {
        let nuovi = copia_tra_fogli(s, &h, &f)?;
        Ok(json!({"handles": nuovi, "message": format!("Copiati {} oggetti in {} fogli", h.len(), f.len())}))
    })
}

fn run_sposta_tra_fogli(s: &mut Session, p: &Value) -> Result<Value> {
    let h = handles_o_selezione(s, p);
    let n = numero_param(p, "foglio").ok_or_else(|| errore("Indicare il foglio d'arrivo"))?;
    if h.is_empty() {
        return Err(errore("Indicare gli oggetti da spostare (handles o selezione)"));
    }
    in_gruppo(s, "Sposta tra fogli", |s| {
        sposta_tra_fogli(s, &h, n)?;
        Ok(json!({"message": format!("Spostati {} oggetti sul foglio {n}", h.len())}))
    })
}

const MENU: &str = "Fogli";

pub fn comandi() -> Vec<CommandSpec> {
    let spec = |id, label, menu, params, run| CommandSpec { id, label, menu, shortcut: None, aliases: &[], params, enabled: has_doc, run, undoable: false, interactive: None, transparent: false };
    vec![
        spec("inseriscifoglio", "Inserisci Foglio", &[MENU, "Inserisci Foglio…"], "{foglio: numero, titolo?} — i fogli da quel numero in poi vanno avanti di uno", run_inserisci as cadcraft_engine::cmd::Run),
        spec("eliminafogli", "Elimina Fogli", &[MENU, "Elimina Fogli…"], "{fogli: [numeri] | \"3-5,8\"}", run_elimina),
        spec("svuotafogli", "Svuota Fogli", &[MENU, "Svuota Fogli…"], "{fogli: [numeri] | \"3-5,8\"}", run_svuota),
        spec("rinominafoglio", "Rinomina Foglio", &[MENU, "Rinomina Foglio…"], "{da: numero, a: numero}", run_rinomina),
        spec("shiftfogli", "Shift Fogli", &[MENU, "Scorri Fogli…"], "{posizioni: +n avanti | -n indietro, fogli?: [numeri] (gruppo) | da?: numero (dal foglio in poi)}", run_shift),
        spec("compattafogli", "Compatta Fogli", &[MENU, "Compatta Fogli"], "{} — numeri consecutivi dal più basso", run_compatta),
        spec("copiatrafogli", "COPIA tra Fogli", &[MENU, "Copia su Altri Fogli…"], "{handles?: [hex] (default la selezione), fogli: [numeri]} — le madri copiate prendono sigle nuove", run_copia_tra_fogli),
        spec("spostatrafogli", "SPOSTA tra Fogli", &[MENU, "Sposta su Altri Fogli…"], "{handles?: [hex] (default la selezione), foglio: numero}", run_sposta_tra_fogli),
        spec(
            "memorizzafoglio",
            "MEMORIZZA Foglio",
            &[MENU, "Salva Foglio in Libreria…"],
            "{nome, categoria?, descrizione?, handles? (default la selezione o tutto il foglio corrente), cartella?: libreria}",
            run_memorizza,
        ),
        spec("inseriscimemorizzato", "INS Fogli Memorizzati", &[MENU, "Inserisci Foglio da Libreria…"], "{nome, categoria? | file, at?: [x,y] (default 0,0)}", run_inserisci_memorizzato),
        CommandSpec {
            id: "fogli.memorizzati",
            label: "Elenco fogli memorizzati",
            menu: &[],
            shortcut: None,
            aliases: &[],
            params: "{} → {fogli: [{nome, categoria, descrizione, file}]}",
            enabled: always,
            run: run_elenco_memorizzati,
            undoable: false,
            interactive: None,
            transparent: false,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn elenchi_di_fogli() {
        assert_eq!(fogli_param(&json!({"f": "3-5, 8;1"}), "f").unwrap_or_default(), vec![1, 3, 4, 5, 8]);
        assert_eq!(fogli_param(&json!({"f": [4, "2", 4]}), "f").unwrap_or_default(), vec![2, 4]);
        assert_eq!(fogli_param(&json!({"f": 7}), "f").unwrap_or_default(), vec![7]);
        assert!(fogli_param(&json!({"f": "a-3"}), "f").is_err());
        assert!(nome_file("Avviamento stella/triangolo").is_err());
        assert_eq!(nome_file(" Ausiliari ").unwrap_or_default(), "Ausiliari");
    }
}
