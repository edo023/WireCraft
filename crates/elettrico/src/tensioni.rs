//! Tensioni e colori dei conduttori: per ogni potenziale (`L1`, `N`, `PE`, `+24V`…) la tensione
//! e il colore del filo secondo la sigla IEC 60757 (`BK`, `BN`, `BU`, `GNYE`…). I predefiniti
//! seguono CEI EN 60445 per le fasi (marrone, nero, grigio), il blu per il neutro, il
//! giallo-verde per la protezione, il rosso per i comandi in alternata e il blu per la
//! continua (CEI EN 60204-1).
//!
//! Una rete senza conduttore specializzato prende il colore del suo potenziale (il nome della
//! linea o del rimando). `COLORAFILI` colora i fili del disegno come il conduttore vero; la
//! stampa a "fili colorati" tiene i fili a colori e mette in nero tutto il resto.
//!
//! La tabella è un CSV aperto (`Potenziale;Tensione;Colore`) accanto alle altre impostazioni.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use cadcraft_engine::cmd::{always, has_doc};
use cadcraft_engine::doc::color::Color;
use cadcraft_engine::doc::{EntityKind, Handle};
use cadcraft_engine::{CommandSpec, Result, Session};
use serde_json::{Value, json};

use crate::util::{errore, in_gruppo};
use crate::{collegamenti, fogli};

/// Un potenziale con la sua tensione e il colore del conduttore.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Potenziale {
    pub nome: String,
    pub tensione: String,
    pub colore: String,
}

const INTESTAZIONE: &str = "Potenziale;Tensione;Colore";

const PREDEFINITI: &[(&str, &str, &str)] = &[
    ("L1", "400 VAC", "BN"),
    ("L2", "400 VAC", "BK"),
    ("L3", "400 VAC", "GY"),
    ("N", "230 VAC", "BU"),
    ("PE", "", "GNYE"),
    ("L", "230 VAC", "RD"),
    ("+24V", "24 VDC", "BU"),
    ("0V", "24 VDC", "BU"),
];

struct Stato {
    file: Option<PathBuf>,
    voci: Vec<Potenziale>,
}

static STATO: Mutex<Option<Stato>> = Mutex::new(None);

fn predefiniti() -> Vec<Potenziale> {
    PREDEFINITI.iter().map(|(n, t, c)| Potenziale { nome: (*n).into(), tensione: (*t).into(), colore: (*c).into() }).collect()
}

fn con<T>(f: impl FnOnce(&mut Stato) -> T) -> T {
    let mut g = STATO.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    f(g.get_or_insert_with(|| Stato { file: None, voci: predefiniti() }))
}

/// La sigla del colore scritta in un modo solo: `gn/ye` → `GNYE`.
pub fn sigla_colore(t: &str) -> String {
    t.chars().filter(char::is_ascii_alphabetic).collect::<String>().to_ascii_uppercase()
}

/// Il colore del disegno (indice ACI) per la sigla IEC 60757 di un conduttore. Mai il 5: le linee
/// di quel colore sono grafica, non fili.
pub fn indice_colore(sigla: &str) -> Option<u8> {
    Some(match sigla_colore(sigla).as_str() {
        "BK" => 7,
        "BN" => 34,
        "RD" => 1,
        "OG" => 30,
        "YE" => 2,
        "GN" => 94,
        "BU" => 150,
        "VT" => 200,
        "GY" => 8,
        "WH" => 9,
        "PK" => 221,
        "TQ" => 4,
        "GNYE" | "YEGN" => 3,
        _ => return None,
    })
}

pub fn leggi_csv(testo: &str) -> Vec<Potenziale> {
    testo
        .lines()
        .map(str::trim)
        .filter(|r| !r.is_empty() && !r.eq_ignore_ascii_case(INTESTAZIONE))
        .filter_map(|r| {
            let c: Vec<&str> = r.split(';').map(str::trim).collect();
            let nome = (*c.first()?).to_string();
            (!nome.is_empty()).then(|| Potenziale { nome, tensione: c.get(1).copied().unwrap_or("").into(), colore: sigla_colore(c.get(2).copied().unwrap_or("")) })
        })
        .collect()
}

fn a_csv(v: &[Potenziale]) -> String {
    let mut t = format!("{INTESTAZIONE}\n");
    for p in v {
        t.push_str(&format!("{};{};{}\n", p.nome, p.tensione, p.colore));
    }
    t
}

/// Usa la tabella del file (creata con i predefiniti se manca).
pub fn imposta(file: &Path) -> Result<usize> {
    let voci = match std::fs::read_to_string(file) {
        Ok(t) => leggi_csv(&t),
        Err(_) => {
            let v = predefiniti();
            if let Some(d) = file.parent() {
                let _ = std::fs::create_dir_all(d);
            }
            std::fs::write(file, a_csv(&v)).map_err(|e| errore(format!("{}: {e}", file.display())))?;
            v
        }
    };
    let n = voci.len();
    con(|st| {
        st.file = Some(file.to_path_buf());
        st.voci = voci;
    });
    Ok(n)
}

pub fn voci() -> Vec<Potenziale> {
    con(|st| st.voci.clone())
}

/// Sostituisce la tabella e la salva nel suo file (se c'è).
pub fn cambia(voci: Vec<Potenziale>) -> Result<()> {
    let file = con(|st| {
        st.voci = voci.clone();
        st.file.clone()
    });
    if let Some(f) = file {
        std::fs::write(&f, a_csv(&voci)).map_err(|e| errore(format!("{}: {e}", f.display())))?;
    }
    Ok(())
}

/// Il potenziale di una rete dal suo nome.
pub fn potenziale(nome: &str) -> Option<Potenziale> {
    let n = nome.trim();
    if n.is_empty() {
        return None;
    }
    con(|st| st.voci.iter().find(|p| p.nome.eq_ignore_ascii_case(n)).cloned())
}

/// Colora i fili di ogni rete come il suo conduttore (o li rimette col colore del layer con
/// `togli`). Restituisce i fili cambiati.
pub fn colora_fili(s: &mut Session, togli: bool) -> Result<usize> {
    let reti = collegamenti::reti(s, &[])?;
    let mut n = 0;
    let d = s.doc_mut()?;
    for r in &reti {
        let colore = if togli { Color::ByLayer } else { indice_colore(&r.conduttore.colore).filter(|&i| i != 7).map_or(Color::ByLayer, Color::Index) };
        for f in &r.fili {
            let Some(h) = Handle::parse_hex(&f.handle) else { continue };
            let cambia = d.entity(h).is_some_and(|e| matches!(e.kind, EntityKind::Line(_)) && e.common.color != colore);
            if cambia {
                d.modify_entity(h, |e| e.common.color = colore)?;
                n += 1;
            }
        }
    }
    Ok(n)
}

/// Prepara la copia di stampa a "fili colorati": i fili dei fogli tengono il loro colore, tutto il
/// resto (simboli, testi, cartiglio, tabelle) va in nero.
pub fn nero_tranne_i_fili(c: &mut cadcraft_engine::doc::Drawing) {
    let nero = Color::Index(7);
    for l in &mut c.layers {
        l.color = nero;
    }
    let fili_layer = |layer: &str| fogli::numero_da_layer(layer).is_some_and(|n| n > 0);
    for h in c.model.handles() {
        c.model.modify(h, |e| {
            let filo = matches!(e.kind, EntityKind::Line(_)) && fili_layer(&e.common.layer) && e.common.color != Color::Index(5);
            if !filo {
                e.common.color = nero;
            }
            if let EntityKind::Insert(i) = &mut e.kind {
                for a in &mut i.attribs {
                    a.props.color = None;
                }
            }
        });
    }
    for b in c.blocks.values_mut() {
        let b = std::sync::Arc::make_mut(b);
        for h in b.entities.handles() {
            b.entities.modify(h, |e| e.common.color = Color::ByBlock);
        }
    }
}

fn run_tensioni(_s: &mut Session, p: &Value) -> Result<Value> {
    if let Some(a) = p.get("voci").and_then(Value::as_array) {
        let mut v = Vec::new();
        for x in a {
            let nome = x.get("nome").and_then(Value::as_str).unwrap_or("").trim().to_string();
            if nome.is_empty() {
                continue;
            }
            let colore = sigla_colore(x.get("colore").and_then(Value::as_str).unwrap_or(""));
            if !colore.is_empty() && indice_colore(&colore).is_none() {
                return Err(errore(format!("Colore sconosciuto per {nome}: {colore} (BK, BN, RD, OG, YE, GN, BU, VT, GY, WH, PK, TQ, GNYE)")));
            }
            v.push(Potenziale { nome, tensione: x.get("tensione").and_then(Value::as_str).unwrap_or("").trim().to_string(), colore });
        }
        cambia(v)?;
    }
    let voci = voci();
    Ok(json!({"voci": voci.iter().map(|p| json!({"nome": p.nome, "tensione": p.tensione, "colore": p.colore})).collect::<Vec<_>>()}))
}

fn run_colora(s: &mut Session, p: &Value) -> Result<Value> {
    let togli = p.get("togli").and_then(Value::as_bool).unwrap_or(false);
    let n = in_gruppo(s, if togli { "Fili col colore del layer" } else { "Colora i fili" }, |s| colora_fili(s, togli))?;
    Ok(json!({"fili": n, "message": if togli { format!("{n} fili tornati al colore del layer") } else { format!("{n} fili colorati come il loro conduttore") }}))
}

pub fn comandi() -> Vec<CommandSpec> {
    let spec = |id, label, menu, params, enabled, run| CommandSpec { id, label, menu, shortcut: None, aliases: &[], params, enabled, run, undoable: false, interactive: None, transparent: false };
    vec![
        spec(
            "tensioni",
            "Tensioni",
            &["Elettrico", "Tensioni e Colori…"],
            "{voci?: [{nome, tensione, colore}]} — la tabella dei potenziali (senza voci la legge)",
            always as cadcraft_engine::cmd::Enabled,
            run_tensioni as cadcraft_engine::cmd::Run,
        ),
        spec("colorafili", "Colora i Fili", &["Elettrico", "Colora i Fili"], "{togli?: bool} — i fili col colore del conduttore (o di nuovo col colore del layer)", has_doc, run_colora),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sigle_e_colori() {
        assert_eq!(sigla_colore("gn/ye"), "GNYE");
        assert_eq!(indice_colore("BU"), Some(150));
        assert_eq!(indice_colore("GN-YE"), Some(3));
        assert_eq!(indice_colore("XX"), None);
        // nessun colore di filo è quello della grafica
        for c in ["BK", "BN", "RD", "OG", "YE", "GN", "BU", "VT", "GY", "WH", "PK", "TQ", "GNYE"] {
            assert_ne!(indice_colore(c), Some(5), "{c}");
        }
    }

    #[test]
    fn csv_della_tabella() {
        let v = leggi_csv("Potenziale;Tensione;Colore\nL1;400 VAC;bn\n\n+24V;24 VDC;BU\n;x;y\n");
        assert_eq!(v, vec![Potenziale { nome: "L1".into(), tensione: "400 VAC".into(), colore: "BN".into() }, Potenziale { nome: "+24V".into(), tensione: "24 VDC".into(), colore: "BU".into() }]);
        assert_eq!(leggi_csv(&a_csv(&v)), v);
    }
}
