//! Archivio materiali: un file CSV aperto (separatore `;`) con costruttore, codice, descrizione,
//! categoria, preferito, simbolo della libreria e larghezza sulla guida DIN (mm). Si riempie
//! importando i listini dei costruttori (CSV con intestazione; le colonne si riconoscono dal
//! nome, in italiano o in inglese), con la libreria di base di articoli generici o a mano.
//!
//! Un articolo con il simbolo si posa direttamente (`MATERIALE.INSERISCI`): il simbolo giusto con
//! costruttore, codice e descrizione già scritti; la larghezza va al componente e il lay-out del
//! quadro la usa al posto della misura stimata.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use cadcraft_engine::cmd::{always, has_doc};
use cadcraft_engine::{Accept, CommandSpec, Input, Interactive, Prompt, Result, Session, Step};
use serde_json::{Value, json};

use crate::util::{errore, testo_param};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Articolo {
    pub costruttore: String,
    pub codice: String,
    pub descrizione: String,
    pub categoria: String,
    pub preferito: bool,
    /// Il simbolo della libreria con cui si disegna (`qf1`, `km`…); vuoto se non si sa.
    pub simbolo: String,
    /// Larghezza sulla guida DIN (mm); 0 se non si sa.
    pub larghezza: f64,
}

const INTESTAZIONE: &str = "Costruttore;Codice;Descrizione;Categoria;Preferito;Simbolo;Larghezza";
/// La libreria di base: articoli generici con simbolo e larghezza.
const BASE: &str = include_str!("libreria_base.csv");
/// Righe al massimo in un file importato (difesa da file enormi o sbagliati).
const MAX_RIGHE: usize = 2_000_000;

struct Stato {
    file: Option<PathBuf>,
    articoli: Vec<Articolo>,
}

static ARCHIVIO: Mutex<Stato> = Mutex::new(Stato { file: None, articoli: Vec::new() });

fn con<T>(f: impl FnOnce(&mut Stato) -> Result<T>) -> Result<T> {
    let mut g = ARCHIVIO.lock().map_err(|_| errore("archivio materiali non disponibile"))?;
    f(&mut g)
}

/// Divide una riga CSV (separatore `sep`, campi tra virgolette con `""` per le virgolette).
pub fn campi(riga: &str, sep: char) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut tra_virgolette = false;
    let mut it = riga.chars().peekable();
    while let Some(c) = it.next() {
        match c {
            '"' if tra_virgolette && it.peek() == Some(&'"') => {
                cur.push('"');
                it.next();
            }
            '"' => tra_virgolette = !tra_virgolette,
            c if c == sep && !tra_virgolette => out.push(std::mem::take(&mut cur).trim().to_string()),
            c => cur.push(c),
        }
    }
    out.push(cur.trim().to_string());
    out
}

fn campo_csv(t: &str) -> String {
    if t.contains([';', '"', '\n']) { format!("\"{}\"", t.replace('"', "\"\"")) } else { t.to_string() }
}

/// Legge un CSV di articoli riconoscendo le colonne dall'intestazione.
pub fn leggi_csv(testo: &str) -> Result<Vec<Articolo>> {
    let testo = testo.trim_start_matches('\u{feff}');
    let mut righe = testo.lines().filter(|l| !l.trim().is_empty());
    let intest = righe.next().ok_or_else(|| errore("file vuoto"))?;
    let sep = [';', ',', '\t'].into_iter().max_by_key(|c| intest.matches(*c).count()).unwrap_or(';');
    let nomi: Vec<String> = campi(intest, sep).into_iter().map(|n| n.to_lowercase()).collect();
    let colonna = |alternative: &[&str]| nomi.iter().position(|n| alternative.iter().any(|a| n.contains(a)));
    let c_cod = colonna(&["codice", "code", "part", "articolo", "tipo"]).ok_or_else(|| errore("manca la colonna del codice (Codice/Code)"))?;
    let c_cost = colonna(&["costruttore", "produttore", "marca", "manufacturer", "brand"]);
    let c_desc = colonna(&["descrizione", "description", "desc"]);
    let c_cat = colonna(&["categoria", "famiglia", "category", "family"]);
    let c_pref = colonna(&["preferit", "favorite", "favourite"]);
    let c_sim = colonna(&["simbolo", "symbol"]);
    let c_larg = colonna(&["larghezza", "width"]);
    let mut v = Vec::new();
    for riga in righe.take(MAX_RIGHE) {
        let f = campi(riga, sep);
        let get = |c: Option<usize>| c.and_then(|c| f.get(c)).cloned().unwrap_or_default();
        let codice = get(Some(c_cod));
        if codice.is_empty() {
            continue;
        }
        let pref = get(c_pref).to_lowercase();
        v.push(Articolo {
            costruttore: get(c_cost),
            codice,
            descrizione: get(c_desc),
            categoria: get(c_cat),
            preferito: matches!(pref.as_str(), "1" | "si" | "sì" | "x" | "true" | "yes"),
            simbolo: get(c_sim).to_ascii_lowercase(),
            larghezza: get(c_larg).replace(',', ".").parse::<f64>().ok().filter(|x| x.is_finite() && *x > 0.0).unwrap_or(0.0),
        });
    }
    Ok(v)
}

fn salva(st: &Stato) -> Result<()> {
    let Some(file) = &st.file else { return Ok(()) };
    let mut out = String::from(INTESTAZIONE);
    out.push('\n');
    for a in &st.articoli {
        let larghezza = if a.larghezza > 0.0 { crate::layout::misura_testo(a.larghezza) } else { String::new() };
        let campi = [a.costruttore.as_str(), a.codice.as_str(), a.descrizione.as_str(), a.categoria.as_str(), if a.preferito { "1" } else { "" }, a.simbolo.as_str(), larghezza.as_str()];
        out.push_str(&campi.iter().map(|c| campo_csv(c)).collect::<Vec<_>>().join(";"));
        out.push('\n');
    }
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir).map_err(|e| errore(format!("{}: {e}", dir.display())))?;
    }
    std::fs::write(file, out).map_err(|e| errore(format!("{}: {e}", file.display())))
}

/// Usa `file` come archivio (lo legge se esiste).
pub fn imposta(file: &Path) -> Result<usize> {
    let articoli = match std::fs::read_to_string(file) {
        Ok(t) => leggi_csv(&t)?,
        Err(_) => Vec::new(),
    };
    con(|st| {
        st.file = Some(file.to_path_buf());
        st.articoli = articoli;
        Ok(st.articoli.len())
    })
}

/// Gli articoli della libreria di base.
pub fn base() -> Vec<Articolo> {
    leggi_csv(BASE).unwrap_or_default()
}

/// L'articolo con il codice dato (e il costruttore, se non è vuoto).
pub fn trova(costruttore: &str, codice: &str) -> Option<Articolo> {
    let (costruttore, codice) = (costruttore.trim(), codice.trim());
    if codice.is_empty() {
        return None;
    }
    con(|st| Ok(st.articoli.iter().find(|a| a.codice.eq_ignore_ascii_case(codice) && (costruttore.is_empty() || a.costruttore.eq_ignore_ascii_case(costruttore))).cloned())).ok().flatten()
}

/// La larghezza (mm) del primo materiale di un componente, se l'archivio la conosce.
pub fn larghezza_di(costruttore: &str, tipo: &str) -> Option<f64> {
    let primo = |t: &str| t.split(';').next().unwrap_or_default().trim().to_string();
    trova(&primo(costruttore), &primo(tipo)).map(|a| a.larghezza).filter(|l| *l > 0.0)
}

fn stesso(a: &Articolo, b: &Articolo) -> bool {
    a.costruttore.eq_ignore_ascii_case(&b.costruttore) && a.codice.eq_ignore_ascii_case(&b.codice)
}

/// Aggiunge (o aggiorna) articoli e salva. Restituisce quanti erano nuovi.
pub fn aggiungi(nuovi: Vec<Articolo>) -> Result<usize> {
    con(|st| {
        let mut n = 0;
        for a in nuovi {
            match st.articoli.iter_mut().find(|x| stesso(x, &a)) {
                Some(x) => {
                    let pref = x.preferito || a.preferito;
                    *x = Articolo { preferito: pref, ..a };
                }
                None => {
                    st.articoli.push(a);
                    n += 1;
                }
            }
        }
        salva(st)?;
        Ok(n)
    })
}

/// Articoli che contengono tutte le parole di `testo` (in costruttore, codice, descrizione o
/// categoria); prima i preferiti.
pub fn cerca(testo: &str, solo_preferiti: bool, limite: usize) -> Result<Vec<Articolo>> {
    let parole: Vec<String> = testo.split_whitespace().map(str::to_lowercase).collect();
    con(|st| {
        let mut v: Vec<Articolo> = st
            .articoli
            .iter()
            .filter(|a| !solo_preferiti || a.preferito)
            .filter(|a| {
                let tutto = format!("{} {} {} {}", a.costruttore, a.codice, a.descrizione, a.categoria).to_lowercase();
                parole.iter().all(|p| tutto.contains(p))
            })
            .cloned()
            .collect();
        v.sort_by(|a, b| b.preferito.cmp(&a.preferito).then_with(|| a.costruttore.cmp(&b.costruttore)).then_with(|| a.codice.cmp(&b.codice)));
        v.truncate(limite);
        Ok(v)
    })
}

pub fn segna_preferito(costruttore: &str, codice: &str, preferito: bool) -> Result<()> {
    con(|st| {
        let chiave = Articolo { costruttore: costruttore.into(), codice: codice.into(), ..Default::default() };
        let a = st.articoli.iter_mut().find(|x| stesso(x, &chiave)).ok_or_else(|| errore(format!("{costruttore} {codice} non è in archivio")))?;
        a.preferito = preferito;
        salva(st)
    })
}

pub fn quanti() -> usize {
    con(|st| Ok(st.articoli.len())).unwrap_or(0)
}

fn json_articolo(a: &Articolo) -> Value {
    json!({"costruttore": a.costruttore, "codice": a.codice, "descrizione": a.descrizione, "categoria": a.categoria, "preferito": a.preferito, "simbolo": a.simbolo, "larghezza": a.larghezza})
}

fn run_cerca(_s: &mut Session, p: &Value) -> Result<Value> {
    let limite = p.get("limite").and_then(Value::as_u64).map_or(200, |l| l.min(10_000) as usize);
    let v = cerca(&testo_param(p, "testo").unwrap_or_default(), p.get("preferiti").and_then(Value::as_bool).unwrap_or(false), limite)?;
    Ok(json!({"articoli": v.iter().map(json_articolo).collect::<Vec<_>>(), "message": format!("{} articoli", v.len())}))
}

fn run_aggiungi(_s: &mut Session, p: &Value) -> Result<Value> {
    let codice = testo_param(p, "codice").filter(|c| !c.trim().is_empty()).ok_or_else(|| errore("Indicare il codice"))?;
    let a = Articolo {
        costruttore: testo_param(p, "costruttore").unwrap_or_default(),
        codice,
        descrizione: testo_param(p, "descrizione").unwrap_or_default(),
        categoria: testo_param(p, "categoria").unwrap_or_default(),
        preferito: p.get("preferito").and_then(Value::as_bool).unwrap_or(false),
        simbolo: testo_param(p, "simbolo").unwrap_or_default().trim().to_ascii_lowercase(),
        larghezza: p.get("larghezza").and_then(|v| v.as_f64().or_else(|| v.as_str().and_then(|t| t.trim().replace(',', ".").parse().ok()))).filter(|x: &f64| x.is_finite() && *x > 0.0).unwrap_or(0.0),
    };
    if !a.simbolo.is_empty() && crate::simboli::trova(&a.simbolo).is_none() {
        return Err(errore(format!("Simbolo sconosciuto: {}", a.simbolo)));
    }
    let msg = format!("Archivio: {} {}", a.costruttore, a.codice);
    aggiungi(vec![a])?;
    Ok(json!({"message": msg}))
}

fn run_importa(_s: &mut Session, p: &Value) -> Result<Value> {
    let file = testo_param(p, "file").ok_or_else(|| errore("Indicare il file CSV"))?;
    let testo = std::fs::read(&file).map_err(|e| errore(format!("{file}: {e}")))?;
    // i listini arrivano spesso in Windows-1252: senza UTF-8 valido si leggono come Latin-1
    let testo = String::from_utf8(testo).unwrap_or_else(|e| e.into_bytes().iter().map(|&b| char::from(b)).collect());
    let articoli = leggi_csv(&testo)?;
    let letti = articoli.len();
    let nuovi = aggiungi(articoli)?;
    Ok(json!({"letti": letti, "nuovi": nuovi, "message": format!("Importati {letti} articoli ({nuovi} nuovi). In archivio: {}", quanti())}))
}

fn run_base(_s: &mut Session, _p: &Value) -> Result<Value> {
    let v = base();
    let n = v.len();
    let nuovi = aggiungi(v)?;
    Ok(json!({"articoli": n, "nuovi": nuovi, "message": format!("Libreria di base: {n} articoli ({nuovi} nuovi). In archivio: {}", quanti())}))
}

/// Posa il simbolo dell'articolo (punto o fence, come `SIMBOLO.*`) con costruttore, codice e
/// descrizione dell'articolo.
pub fn posa(s: &mut Session, p: &Value) -> Result<Value> {
    let codice = testo_param(p, "codice").filter(|c| !c.trim().is_empty()).ok_or_else(|| errore("Indicare il codice del materiale"))?;
    let costruttore = testo_param(p, "costruttore").unwrap_or_default();
    let a = trova(&costruttore, &codice).ok_or_else(|| errore(format!("{} {codice} non è in archivio", costruttore.trim())))?;
    let sim = crate::simboli::trova(&a.simbolo).ok_or_else(|| errore(format!("L'articolo {} non dice con che simbolo si disegna: scrivilo nella colonna Simbolo dell'archivio", a.codice)))?;
    let mut q = p.clone();
    if let Some(m) = q.as_object_mut() {
        // CODICE nei simboli è lo specchietto dei contatti: il codice dell'articolo va in TIPO
        m.remove("codice");
        m.insert("costruttore".into(), json!(a.costruttore));
        m.insert("tipo".into(), json!(a.codice));
        m.insert("descrizione".into(), json!(a.descrizione));
    }
    s.execute(sim.comando, &q)
}

fn run_posa(s: &mut Session, p: &Value) -> Result<Value> {
    posa(s, p)
}

/// Posa interattiva: il codice (`costruttore|codice`, o solo il codice) e poi un punto per ogni
/// componente da posare, fino a Invio.
struct PosaInterattiva {
    articolo: Option<Articolo>,
}

impl Interactive for PosaInterattiva {
    fn name(&self) -> &'static str {
        "MATERIALE.INSERISCI"
    }
    fn prompt(&self, _s: &Session) -> Prompt {
        match &self.articolo {
            Some(a) => Prompt::new(format!("Punto di {} {} (Invio per finire)", a.costruttore, a.codice), Accept::POINT),
            None => Prompt::new("Codice del materiale", Accept::TEXT),
        }
    }
    fn input(&mut self, s: &mut Session, i: Input) -> Result<Step> {
        match (i, &self.articolo) {
            (Input::Cancel | Input::Enter, _) => Ok(Step::Done),
            (Input::Text(t) | Input::Keyword(t), None) => {
                let (costruttore, codice) = t.split_once('|').unwrap_or(("", t.as_str()));
                match trova(costruttore, codice) {
                    Some(a) => self.articolo = Some(a),
                    None => s.echo(format!("{} non è in archivio", t.trim())),
                }
                Ok(Step::Continue)
            }
            (Input::Point(q), Some(a)) => {
                match posa(s, &json!({"costruttore": a.costruttore, "codice": a.codice, "at": [q.x, q.y]})) {
                    Ok(r) => s.echo(r["message"].as_str().unwrap_or_default().to_string()),
                    Err(e) => s.echo(e.to_string()),
                }
                Ok(Step::Continue)
            }
            _ => Ok(Step::Continue),
        }
    }
}

fn fabbrica(_s: &Session) -> Result<Box<dyn Interactive>> {
    Ok(Box::new(PosaInterattiva { articolo: None }))
}

pub fn comandi() -> Vec<CommandSpec> {
    let spec = |id, label, voce, params, run| CommandSpec {
        id,
        label,
        menu: voce,
        shortcut: None,
        aliases: &[],
        params,
        enabled: always,
        run,
        undoable: false,
        interactive: None,
        transparent: false,
    };
    vec![
        spec("materiali.cerca", "Cerca in Archivio Materiali", &[], "{testo?, preferiti?: bool, limite?} → {articoli: [{costruttore, codice, descrizione, categoria, preferito, simbolo, larghezza}]}", run_cerca as cadcraft_engine::cmd::Run),
        spec("materiali.aggiungi", "Aggiungi ad Archivio Materiali", &[], "{costruttore?, codice, descrizione?, categoria?, preferito?, simbolo?: id della libreria, larghezza?: mm sulla guida DIN}", run_aggiungi),
        spec("materiali.base", "Libreria di base", &["Progetto", "Libreria di base dei materiali"], "{} — aggiunge all'archivio gli articoli generici con simbolo e larghezza", run_base),
        CommandSpec {
            enabled: has_doc,
            interactive: Some(fabbrica),
            ..spec(
                "materiale.inserisci",
                "Inserisci da codice",
                &["Simboli", "Inserisci da codice…"],
                "{codice, costruttore?, at: [x,y] | da + a (fence), …i parametri del simbolo} — il simbolo dell'articolo con costruttore, codice e descrizione; interattivo: il codice, poi un punto per componente",
                run_posa,
            )
        },
        spec("materiali.importa", "Importa listino materiali", &["Progetto", "Importa listino materiali (CSV)"], "{file: percorso .csv} → {letti, nuovi}", run_importa),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csv_con_virgolette_e_colonne_in_inglese() {
        assert_eq!(campi("a;\"b;c\";\"d \"\"e\"\"\"", ';'), vec!["a", "b;c", "d \"e\""]);
        let v = leggi_csv("Manufacturer,Part number,Description\nEaton,199220,\"Contactor, 3kW\"\n,,\n").unwrap_or_default();
        assert_eq!(v, vec![Articolo { costruttore: "Eaton".into(), codice: "199220".into(), descrizione: "Contactor, 3kW".into(), ..Default::default() }]);
    }
}
