//! Commesse (progetti): una cartella con l'elenco dei suoi multifogli, i settaggi in `CFG` e i
//! file di uscita (`PDF`…).
//!
//! L'elenco sta in `Job-Description._sp`, nello stesso formato dei CAD elettrici diffusi,
//! così le commesse si aprono con entrambi:
//!
//! ```text
//! ("RELEASE" . "2026")
//! ("LAST_OPEN" . "SCHEMA")
//! ("CUSTOMER" . "Cliente")
//! ("DESCRIPTION" . "Descrizione della commessa")
//! (1 "SCHEMA" "percorso" "descrizione" "disegnatore" "prefisso" "AAAAMMGG.hhmm" "1")
//! ```
//!
//! Il multifoglio è `<nome>.dxf` (il nostro formato di lavoro) oppure `<nome>.dwg`.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use cadcraft_engine::cmd::always;
use cadcraft_engine::{CommandSpec, Result, Session};
use serde_json::{Value, json};

use crate::util::{errore, numero_param, testo_param};

pub const FILE_COMMESSA: &str = "Job-Description._sp";
/// Release scritta nei file nuovi.
const RELEASE: &str = "2026";

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Multifoglio {
    pub nr: u32,
    pub nome: String,
    /// Cartella registrata quando è stato creato (informativa: vale la cartella della commessa).
    pub percorso: String,
    pub descrizione: String,
    pub disegnatore: String,
    /// Prefisso dei numeri di foglio del multifoglio (es. `A` → A1, A2…).
    pub prefisso: String,
    /// Data di creazione `AAAAMMGG.hhmm`.
    pub data: String,
    /// Da gestire nelle elaborazioni (distinte, cross reference di progetto…).
    pub elaborare: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Commessa {
    pub cartella: PathBuf,
    pub release: String,
    pub ultimo: String,
    pub cliente: String,
    pub descrizione: String,
    pub multifogli: Vec<Multifoglio>,
}

impl Commessa {
    /// Il nome della commessa: quello della sua cartella.
    pub fn nome(&self) -> String {
        self.cartella.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()
    }
    /// Il file del multifoglio: `.dxf` se c'è, altrimenti `.dwg` se c'è, altrimenti il `.dxf` da creare.
    pub fn file(&self, nome: &str) -> PathBuf {
        let dxf = self.cartella.join(format!("{nome}.dxf"));
        if dxf.exists() {
            return dxf;
        }
        for ext in ["dwg", "DWG", "DXF"] {
            let f = self.cartella.join(format!("{nome}.{ext}"));
            if f.exists() {
                return f;
            }
        }
        dxf
    }
}

// ---------- formato del file ----------

/// Le parti tra virgolette di una riga (`\\` e `\"` dentro le virgolette); i numeri fuori dalle
/// virgolette tornano come testo.
fn parti(riga: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut dentro = false;
    let mut nudo = String::new();
    let mut it = riga.chars().peekable();
    while let Some(c) = it.next() {
        if dentro {
            match c {
                '\\' if matches!(it.peek(), Some('\\') | Some('"')) => {
                    if let Some(n) = it.next() {
                        cur.push(n);
                    }
                }
                '"' => {
                    dentro = false;
                    out.push(std::mem::take(&mut cur));
                }
                c => cur.push(c),
            }
        } else if c == '"' {
            dentro = true;
        } else if c.is_ascii_digit() {
            nudo.push(c);
        } else if !nudo.is_empty() {
            out.push(std::mem::take(&mut nudo));
        }
    }
    if !nudo.is_empty() {
        out.push(nudo);
    }
    out
}

fn virgolette(t: &str) -> String {
    format!("\"{}\"", t.replace('\\', "\\\\").replace('"', "\\\""))
}

/// Legge il testo di un `Job-Description._sp`.
pub fn leggi_testo(testo: &str, cartella: &Path) -> Commessa {
    let mut c = Commessa { cartella: cartella.to_path_buf(), ..Commessa::default() };
    for riga in testo.lines().map(str::trim).filter(|r| r.starts_with('(')) {
        let p = parti(riga);
        let coppia = riga.contains(" . ");
        match (coppia, p.as_slice()) {
            (true, [k, v, ..]) => match k.to_ascii_uppercase().as_str() {
                "RELEASE" => c.release = v.clone(),
                "LAST_OPEN" => c.ultimo = v.clone(),
                "CUSTOMER" => c.cliente = v.clone(),
                "DESCRIPTION" => c.descrizione = v.clone(),
                _ => {}
            },
            (false, [nr, nome, resto @ ..]) => {
                let g = |k: usize| resto.get(k).cloned().unwrap_or_default();
                c.multifogli.push(Multifoglio {
                    nr: nr.parse().unwrap_or(0),
                    nome: nome.clone(),
                    percorso: g(0),
                    descrizione: g(1),
                    disegnatore: g(2),
                    prefisso: g(3),
                    data: g(4),
                    elaborare: resto.get(5).is_none_or(|e| e.trim() != "0"),
                });
            }
            _ => {}
        }
    }
    c.multifogli.sort_by_key(|m| m.nr);
    c
}

/// Il testo del `Job-Description._sp` di una commessa (righe con CR LF, come l'originale).
pub fn scrivi_testo(c: &Commessa) -> String {
    let mut righe = vec![
        format!("(\"RELEASE\" . {})", virgolette(if c.release.is_empty() { RELEASE } else { &c.release })),
        format!("(\"LAST_OPEN\" . {})", virgolette(&c.ultimo)),
        format!("(\"CUSTOMER\" . {})", virgolette(&c.cliente)),
        format!("(\"DESCRIPTION\" . {})", virgolette(&c.descrizione)),
    ];
    for m in &c.multifogli {
        righe.push(format!(
            "({} {} {} {} {} {} {} {})",
            m.nr,
            virgolette(&m.nome),
            virgolette(&m.percorso),
            virgolette(&m.descrizione),
            virgolette(&m.disegnatore),
            virgolette(&m.prefisso),
            virgolette(&m.data),
            virgolette(if m.elaborare { "1" } else { "0" })
        ));
    }
    let mut t = righe.join("\r\n");
    t.push_str("\r\n");
    t
}

pub fn leggi(cartella: &Path) -> Result<Commessa> {
    let f = cartella.join(FILE_COMMESSA);
    let b = std::fs::read(&f).map_err(|e| errore(format!("{}: {e}", f.display())))?;
    // il file è ASCII/Latin-1 nei progetti esistenti
    let testo = String::from_utf8(b.clone()).unwrap_or_else(|_| b.iter().map(|&c| char::from(c)).collect());
    Ok(leggi_testo(&testo, cartella))
}

/// Scrive l'elenco della commessa (la versione precedente resta in `Job-Description.bak`).
pub fn scrivi(c: &Commessa) -> Result<()> {
    let f = c.cartella.join(FILE_COMMESSA);
    if f.exists() {
        let _ = std::fs::copy(&f, c.cartella.join("Job-Description.bak"));
    }
    cadcraft_engine::atomic_write::write_atomic(&f, scrivi_testo(c).as_bytes()).map_err(|e| errore(format!("{}: {e}", f.display())))
}

/// La commessa a cui appartiene un disegno (la cartella del disegno ha il file della commessa).
pub fn di_disegno(disegno: &Path) -> Option<Commessa> {
    let cartella = disegno.parent()?;
    cartella.join(FILE_COMMESSA).exists().then(|| leggi(cartella).ok()).flatten()
}

/// Le commesse dentro `radice` (una per sottocartella col file della commessa).
pub fn elenco(radice: &Path) -> Vec<Commessa> {
    let Ok(rd) = std::fs::read_dir(radice) else { return Vec::new() };
    let mut v: Vec<Commessa> = rd.flatten().map(|e| e.path()).filter(|p| p.join(FILE_COMMESSA).exists()).filter_map(|p| leggi(&p).ok()).collect();
    v.sort_by_key(|c| c.nome().to_lowercase());
    v
}

/// Un nome valido per una commessa o un multifoglio: niente caratteri vietati nei nomi di file,
/// al più 32 caratteri.
pub fn nome_valido(t: &str) -> Result<String> {
    let t = t.trim();
    if t.is_empty() || t.chars().count() > 32 || t.chars().any(|c| matches!(c, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' | ';' | ',')) || t.ends_with('.') {
        return Err(errore(format!("Nome non valido: {t:?} (al più 32 caratteri, senza punteggiatura speciale)")));
    }
    Ok(t.to_string())
}

/// Crea la commessa `nome` in `radice` (cartella, file della commessa, `CFG` con le impostazioni
/// in uso).
pub fn nuova(radice: &Path, nome: &str, descrizione: &str, cliente: &str) -> Result<Commessa> {
    let nome = nome_valido(nome)?;
    let cartella = radice.join(&nome);
    if cartella.join(FILE_COMMESSA).exists() {
        return Err(errore(format!("La commessa {nome} esiste già")));
    }
    std::fs::create_dir_all(cartella.join("CFG")).map_err(|e| errore(format!("{}: {e}", cartella.display())))?;
    let c = Commessa { cartella: cartella.clone(), release: RELEASE.into(), cliente: cliente.trim().into(), descrizione: descrizione.trim().into(), ..Commessa::default() };
    scrivi(&c)?;
    // la configurazione di partenza: le impostazioni in uso, copiate nella commessa
    crate::impostazioni::scrivi(&crate::impostazioni::file_commessa(&cartella), &crate::impostazioni::correnti())?;
    Ok(c)
}

/// Data e ora attuali come `AAAAMMGG.hhmm` (UTC: il sistema non dà il fuso senza librerie).
pub fn adesso() -> String {
    let secondi = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let giorni = i64::try_from(secondi / 86_400).unwrap_or(0);
    let resto = secondi % 86_400;
    let (a, m, g) = data_da_giorni(giorni);
    format!("{a:04}{m:02}{g:02}.{:02}{:02}", resto / 3600, (resto % 3600) / 60)
}

/// Anno, mese, giorno dai giorni dal 1/1/1970 (calendario gregoriano).
fn data_da_giorni(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = u32::try_from(doy - (153 * mp + 2) / 5 + 1).unwrap_or(1);
    let m = u32::try_from(if mp < 10 { mp + 3 } else { mp - 9 }).unwrap_or(1);
    let y = yoe + era * 400 + i64::from(m <= 2);
    (y, m, d)
}

/// `AAAAMMGG.hhmm` → `GG/MM/AAAA hh:mm`.
pub fn data_leggibile(d: &str) -> String {
    let (data, ora) = d.split_once('.').unwrap_or((d, ""));
    if data.len() != 8 || !data.bytes().all(|b| b.is_ascii_digit()) {
        return d.to_string();
    }
    let mut s = format!("{}/{}/{}", &data[6..8], &data[4..6], &data[..4]);
    if ora.len() == 4 {
        s.push_str(&format!(" {}:{}", &ora[..2], &ora[2..]));
    }
    s
}

/// Aggiunge il multifoglio `nome` alla commessa e restituisce il file del disegno da creare.
pub fn aggiungi_multifoglio(c: &mut Commessa, nome: &str, descrizione: &str, disegnatore: &str, prefisso: &str) -> Result<PathBuf> {
    let nome = nome_valido(nome)?;
    if c.multifogli.iter().any(|m| m.nome.eq_ignore_ascii_case(&nome)) {
        return Err(errore(format!("Il multifoglio {nome} esiste già nella commessa")));
    }
    let nr = c.multifogli.iter().map(|m| m.nr).max().unwrap_or(0) + 1;
    c.multifogli.push(Multifoglio {
        nr,
        nome: nome.clone(),
        percorso: c.cartella.to_string_lossy().to_string(),
        descrizione: descrizione.trim().into(),
        disegnatore: disegnatore.trim().into(),
        prefisso: prefisso.trim().chars().take(6).collect(),
        data: adesso(),
        elaborare: true,
    });
    c.ultimo = nome.clone();
    scrivi(c)?;
    Ok(c.cartella.join(format!("{nome}.dxf")))
}

// ---------- percorsi delle commesse ----------

static PERCORSI: Mutex<Option<PathBuf>> = Mutex::new(None);

/// Il file che elenca le cartelle delle commesse (una per riga).
pub fn imposta_file_percorsi(file: &Path) {
    if let Ok(mut g) = PERCORSI.lock() {
        *g = Some(file.to_path_buf());
    }
}

/// Le cartelle in cui stanno le commesse.
pub fn percorsi() -> Vec<PathBuf> {
    let file = PERCORSI.lock().ok().and_then(|g| g.clone());
    let Some(file) = file else { return Vec::new() };
    std::fs::read_to_string(file).unwrap_or_default().lines().map(str::trim).filter(|l| !l.is_empty()).map(PathBuf::from).collect()
}

/// Aggiunge (o toglie) una cartella di commesse.
pub fn cambia_percorsi(aggiungi: Option<&Path>, togli: Option<&Path>) -> Result<Vec<PathBuf>> {
    let file = PERCORSI.lock().ok().and_then(|g| g.clone()).ok_or_else(|| errore("Cartelle delle commesse non configurate"))?;
    let mut v = percorsi();
    if let Some(a) = aggiungi
        && !v.iter().any(|p| p == a)
    {
        v.push(a.to_path_buf());
    }
    if let Some(t) = togli {
        v.retain(|p| p != t);
    }
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir).map_err(|e| errore(format!("{}: {e}", dir.display())))?;
    }
    let testo: String = v.iter().map(|p| format!("{}\n", p.display())).collect();
    std::fs::write(&file, testo).map_err(|e| errore(format!("{}: {e}", file.display())))?;
    Ok(v)
}

// ---------- commessa del disegno aperto ----------

static ATTIVA: Mutex<Option<Commessa>> = Mutex::new(None);

/// Il disegno `percorso` è diventato quello attivo: se è in una commessa se ne caricano le
/// impostazioni e la commessa ricorda il multifoglio aperto. Restituisce la commessa.
pub fn attiva_disegno(percorso: Option<&Path>) -> Option<Commessa> {
    let c = percorso.and_then(di_disegno);
    let prima = ATTIVA.lock().ok().and_then(|g| g.as_ref().map(|c| c.cartella.clone()));
    match &c {
        Some(c) => {
            if prima.as_ref() != Some(&c.cartella) {
                crate::impostazioni::carica_commessa(&c.cartella);
            }
            if let Some(nome) = percorso.and_then(|p| p.file_stem()).map(|s| s.to_string_lossy().to_string())
                && !c.ultimo.eq_ignore_ascii_case(&nome)
                && c.multifogli.iter().any(|m| m.nome.eq_ignore_ascii_case(&nome))
            {
                let mut c2 = c.clone();
                c2.ultimo = nome;
                let _ = scrivi(&c2);
            }
        }
        None => {
            if prima.is_some() {
                // fuori da una commessa: le impostazioni generali
                if let Some(f) = crate::impostazioni::file_generali() {
                    crate::impostazioni::carica_generali(&f);
                }
            }
        }
    }
    if let Ok(mut g) = ATTIVA.lock() {
        g.clone_from(&c);
    }
    c
}

/// La commessa del disegno attivo.
pub fn attiva() -> Option<Commessa> {
    ATTIVA.lock().ok().and_then(|g| g.clone())
}

// ---------- comandi ----------

fn json_commessa(c: &Commessa) -> Value {
    json!({
        "nome": c.nome(),
        "cartella": c.cartella.to_string_lossy(),
        "cliente": c.cliente,
        "descrizione": c.descrizione,
        "ultimo": c.ultimo,
        "multifogli": c.multifogli.iter().map(|m| json!({
            "nr": m.nr, "nome": m.nome, "descrizione": m.descrizione, "disegnatore": m.disegnatore, "prefisso": m.prefisso,
            "data": m.data, "elaborare": m.elaborare, "file": c.file(&m.nome).to_string_lossy(),
        })).collect::<Vec<_>>(),
    })
}

fn run_elenco(_s: &mut Session, p: &Value) -> Result<Value> {
    let radici: Vec<PathBuf> = match testo_param(p, "radice") {
        Some(r) => vec![PathBuf::from(r)],
        None => percorsi(),
    };
    let v: Vec<Value> = radici.iter().flat_map(|r| elenco(r)).map(|c| json_commessa(&c)).collect();
    let righe: Vec<String> = v.iter().map(|c| format!("{}  {}  {}", c["nome"].as_str().unwrap_or(""), c["cliente"].as_str().unwrap_or(""), c["descrizione"].as_str().unwrap_or(""))).collect();
    Ok(json!({"commesse": v, "message": if righe.is_empty() { "Nessuna commessa".to_string() } else { righe.join("\n") }}))
}

fn run_nuova(_s: &mut Session, p: &Value) -> Result<Value> {
    let radice = testo_param(p, "radice").map(PathBuf::from).or_else(|| percorsi().into_iter().next()).ok_or_else(|| errore("Indicare la cartella delle commesse (radice)"))?;
    let c = nuova(&radice, &testo_param(p, "nome").unwrap_or_default(), &testo_param(p, "descrizione").unwrap_or_default(), &testo_param(p, "cliente").unwrap_or_default())?;
    let mut r = json_commessa(&c);
    r["message"] = json!(format!("Commessa {} creata in {}", c.nome(), radice.display()));
    Ok(r)
}

/// Crea un multifoglio nuovo nella commessa: aggiunge la voce, crea il disegno con squadratura,
/// cartiglio compilato coi dati della commessa e i primi fogli, lo salva e lo lascia aperto.
fn run_nuovo_multifoglio(s: &mut Session, p: &Value) -> Result<Value> {
    let cartella = testo_param(p, "commessa").map(PathBuf::from).ok_or_else(|| errore("Indicare la cartella della commessa"))?;
    let mut c = leggi(&cartella)?;
    let nome = testo_param(p, "nome").unwrap_or_default();
    let descrizione = testo_param(p, "descrizione").unwrap_or_default();
    let disegnatore = testo_param(p, "disegnatore").unwrap_or_default();
    let file = aggiungi_multifoglio(&mut c, &nome, &descrizione, &disegnatore, &testo_param(p, "prefisso").unwrap_or_default())?;
    crate::impostazioni::carica_commessa(&cartella);
    s.execute("new", &json!({"metric": true, "template": "."}))?;
    let oggi = data_leggibile(&adesso());
    let data = oggi.split(' ').next().unwrap_or_default().to_string();
    s.execute(
        "nuovomultifoglio",
        &json!({
            "fogli": numero_param(p, "fogli").unwrap_or(10),
            "commessa": c.nome(),
            "descrizione": c.descrizione,
            "multifoglio": nome.trim(),
            "descrizioneMultifoglio": descrizione,
            "cliente": c.cliente,
            "disegnatore": disegnatore,
            "data": data,
        }),
    )?;
    s.execute("saveas", &json!({"path": file.to_string_lossy()}))?;
    attiva_disegno(Some(&file));
    Ok(json!({"file": file.to_string_lossy(), "message": format!("Multifoglio {} creato: {}", nome.trim(), file.display())}))
}

fn run_apri(s: &mut Session, p: &Value) -> Result<Value> {
    let cartella = testo_param(p, "commessa").map(PathBuf::from).ok_or_else(|| errore("Indicare la cartella della commessa"))?;
    let c = leggi(&cartella)?;
    let nome = testo_param(p, "nome").filter(|n| !n.is_empty()).unwrap_or_else(|| c.ultimo.clone());
    let file = c.file(&nome);
    if !file.exists() {
        return Err(errore(format!("Il disegno {} non esiste", file.display())));
    }
    s.execute("open", &json!({"path": file.to_string_lossy()}))?;
    attiva_disegno(Some(&file));
    Ok(json!({"file": file.to_string_lossy(), "message": format!("Aperto {}", file.display())}))
}

fn run_modifica(_s: &mut Session, p: &Value) -> Result<Value> {
    let cartella = testo_param(p, "commessa").map(PathBuf::from).ok_or_else(|| errore("Indicare la cartella della commessa"))?;
    let mut c = leggi(&cartella)?;
    if let Some(t) = testo_param(p, "descrizione") {
        c.descrizione = t;
    }
    if let Some(t) = testo_param(p, "cliente") {
        c.cliente = t;
    }
    if let Some(m) = p.get("multifoglio") {
        let nome = m.get("nome").and_then(Value::as_str).unwrap_or_default();
        let mf = c.multifogli.iter_mut().find(|x| x.nome.eq_ignore_ascii_case(nome)).ok_or_else(|| errore(format!("Multifoglio {nome} non trovato")))?;
        if let Some(t) = m.get("descrizione").and_then(Value::as_str) {
            mf.descrizione = t.into();
        }
        if let Some(t) = m.get("disegnatore").and_then(Value::as_str) {
            mf.disegnatore = t.into();
        }
        if let Some(t) = m.get("prefisso").and_then(Value::as_str) {
            mf.prefisso = t.chars().take(6).collect();
        }
        if let Some(b) = m.get("elaborare").and_then(Value::as_bool) {
            mf.elaborare = b;
        }
        if let Some(d) = m.get("sposta").and_then(Value::as_i64) {
            let i = c.multifogli.iter().position(|x| x.nome.eq_ignore_ascii_case(nome)).unwrap_or(0);
            let j = usize::try_from(i64::try_from(i).unwrap_or(0) + d).unwrap_or(0).min(c.multifogli.len().saturating_sub(1));
            let mf = c.multifogli.remove(i);
            c.multifogli.insert(j, mf);
            for (k, m) in c.multifogli.iter_mut().enumerate() {
                m.nr = u32::try_from(k + 1).unwrap_or(1);
            }
        }
    }
    scrivi(&c)?;
    let mut r = json_commessa(&c);
    r["message"] = json!(format!("Commessa {} aggiornata", c.nome()));
    Ok(r)
}

fn copia_cartella(da: &Path, a: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(a)?;
    for e in std::fs::read_dir(da)? {
        let e = e?;
        let dest = a.join(e.file_name());
        if e.file_type()?.is_dir() {
            copia_cartella(&e.path(), &dest)?;
        } else {
            std::fs::copy(e.path(), dest)?;
        }
    }
    Ok(())
}

fn run_copia(_s: &mut Session, p: &Value) -> Result<Value> {
    let da = testo_param(p, "commessa").map(PathBuf::from).ok_or_else(|| errore("Indicare la commessa da copiare"))?;
    let nome = nome_valido(&testo_param(p, "nome").unwrap_or_default())?;
    let radice = testo_param(p, "radice").map(PathBuf::from).or_else(|| da.parent().map(Path::to_path_buf)).ok_or_else(|| errore("Indicare la cartella di destinazione"))?;
    let a = radice.join(&nome);
    if a.exists() {
        return Err(errore(format!("{} esiste già", a.display())));
    }
    copia_cartella(&da, &a).map_err(|e| errore(format!("{}: {e}", a.display())))?;
    let mut c = leggi(&a)?;
    if let Some(d) = testo_param(p, "descrizione") {
        c.descrizione = d;
    }
    scrivi(&c)?;
    Ok(json!({"cartella": a.to_string_lossy(), "message": format!("Commessa copiata in {}", a.display())}))
}

pub fn comandi() -> Vec<CommandSpec> {
    let spec = |id, label, menu, params, run| CommandSpec { id, label, menu, shortcut: None, aliases: &[], params, enabled: always, run, undoable: false, interactive: None, transparent: false };
    vec![
        spec("commesse.elenco", "Elenco commesse", &[], "{radice?: cartella (default le cartelle configurate)} → {commesse: [{nome, cartella, cliente, descrizione, multifogli}]}", run_elenco as cadcraft_engine::cmd::Run),
        spec("commessa.nuova", "Nuovo Progetto", &[], "{nome, descrizione?, cliente?, radice?: cartella delle commesse}", run_nuova),
        spec(
            "commessa.nuovomultifoglio",
            "Nuovo Multifoglio",
            &[],
            "{commessa: cartella, nome, descrizione?, disegnatore?, prefisso?, fogli?: (default 10)} — crea, compila il cartiglio, salva in DXF e apre",
            run_nuovo_multifoglio,
        ),
        spec("commessa.apri", "Apri Multifoglio", &[], "{commessa: cartella, nome?: multifoglio (default l'ultimo aperto)}", run_apri),
        spec(
            "commessa.modifica",
            "Modifica Progetto",
            &[],
            "{commessa: cartella, descrizione?, cliente?, multifoglio?: {nome, descrizione?, disegnatore?, prefisso?, elaborare?, sposta?: +1|-1}}",
            run_modifica,
        ),
        spec("commessa.copia", "Copia Progetto", &[], "{commessa: cartella, nome: nuovo nome, radice?, descrizione?}", run_copia),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    const ESEMPIO: &str = "(\"RELEASE\" . \"2020\")\r\n(\"LAST_OPEN\" . \"ELECTRICAL DIAGRAM\")\r\n(\"CUSTOMER\" . \"Cliente\")\r\n(\"DESCRIPTION\" . \"Esempio\")\r\n(1 \"ELECTRICAL DIAGRAM\" \"C:\\\\Commesse\\\\Esempio\" \"Schema Elettrico\" \"M.C.\" \"\" \"20090216.1532\" \"1\")\r\n(2 \"LAYOUT\" \"C:\\Vecchio\\Percorso\" \"Layout\" \"M.C.\" \"LY\" \"\" \"0\")\r\n";

    #[test]
    fn file_della_commessa_avanti_e_indietro() {
        let c = leggi_testo(ESEMPIO, Path::new("C:/Commesse/Esempio"));
        assert_eq!((c.release.as_str(), c.ultimo.as_str(), c.cliente.as_str(), c.descrizione.as_str()), ("2020", "ELECTRICAL DIAGRAM", "Cliente", "Esempio"));
        assert_eq!(c.multifogli.len(), 2);
        let m = &c.multifogli[0];
        assert_eq!((m.nr, m.nome.as_str(), m.percorso.as_str(), m.disegnatore.as_str(), m.data.as_str(), m.elaborare), (1, "ELECTRICAL DIAGRAM", "C:\\Commesse\\Esempio", "M.C.", "20090216.1532", true));
        // le barre non raddoppiate dei file esistenti si leggono come sono
        assert_eq!(c.multifogli[1].percorso, "C:\\Vecchio\\Percorso");
        assert_eq!((c.multifogli[1].prefisso.as_str(), c.multifogli[1].elaborare), ("LY", false));
        assert_eq!(leggi_testo(&scrivi_testo(&c), Path::new("C:/Commesse/Esempio")), c);
        assert_eq!(c.nome(), "Esempio");
    }

    #[test]
    fn date() {
        assert_eq!(data_da_giorni(0), (1970, 1, 1));
        assert_eq!(data_da_giorni(20_752), (2026, 10, 26));
        assert_eq!(data_leggibile("20090216.1532"), "16/02/2009 15:32");
        assert_eq!(adesso().len(), 13);
        assert!(nome_valido("SCHEMA ELETTRICO").is_ok());
        assert!(nome_valido("A/B").is_err());
    }
}
