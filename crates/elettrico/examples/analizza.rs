//! Studio di un progetto esistente (per l'interoperabilità): elenca i blocchi inseriti con i loro
//! attributi, raggruppati per nome di blocco e valore di `PRES`.
//!
//! `cargo run --example analizza -- progetto.dwg [filtro blocchi] [--tutti]`

use std::collections::BTreeMap;

use cadcraft_engine::Session;
use serde_json::{Value, json};

fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let file = args.first().ok_or("indicare il file")?;
    let filtro = args.get(1).filter(|a| !a.starts_with("--")).cloned().unwrap_or_else(|| "*".into());
    let tutti = args.iter().any(|a| a == "--tutti");
    let byte = std::fs::read(file).map_err(|e| e.to_string())?;
    let doc = cadcraft_io::read(&byte, file).map_err(|e| e.to_string())?;
    let mut s = Session::empty();
    s.open_drawing(doc, file, None);
    let r = s.execute("dataextraction", &json!({"blocks": filtro, "combine": false})).map_err(|e| e.to_string())?;
    let col: Vec<String> = r["columns"].as_array().into_iter().flatten().filter_map(Value::as_str).map(str::to_string).collect();
    let idx = |n: &str| col.iter().position(|c| c == n);
    let (Some(cn), Some(cl)) = (idx("Name"), idx("Layer")) else { return Err("colonne mancanti".into()) };
    let cp = idx("PRES");
    // (blocco, PRES) -> (quanti, tag con valore, esempio)
    let mut gruppi: BTreeMap<(String, String), (usize, Vec<String>, String)> = BTreeMap::new();
    for riga in r["rows"].as_array().into_iter().flatten() {
        let testo = |k: usize| match riga.get(k) {
            Some(Value::String(t)) => t.clone(),
            Some(Value::Number(n)) => n.to_string(),
            _ => String::new(),
        };
        let nome = testo(cn);
        let pres = cp.map(testo).unwrap_or_default();
        let g = gruppi.entry((nome, pres)).or_default();
        g.0 += 1;
        let fisse = ["Name", "Handle", "Layer", "Position X", "Position Y", "Rotation"];
        let pieni: Vec<(String, String)> =
            col.iter().enumerate().filter(|(_, c)| !fisse.contains(&c.as_str())).map(|(k, c)| (c.clone(), testo(k))).filter(|(_, v)| !v.is_empty()).collect();
        for (t, _) in &pieni {
            if !g.1.contains(t) {
                g.1.push(t.clone());
            }
        }
        if g.2.is_empty() || tutti {
            let es = format!("fg {} {}", testo(cl), pieni.iter().map(|(t, v)| format!("{t}={v}")).collect::<Vec<_>>().join(" "));
            if tutti {
                println!("{} [{}] {es}", g.2.len(), testo(cn));
            }
            g.2 = es;
        }
    }
    if !tutti {
        for ((nome, pres), (n, tag, es)) in &gruppi {
            println!("{nome} PRES={pres} ×{n}\n    tag: {}\n    es.: {es}", tag.join(" "));
        }
    }
    Ok(())
}
