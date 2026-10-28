//! Nessun comando elettrico va in panic: con parametri vuoti, sbagliati o assurdi può solo dare
//! errore, senza disegno, su un disegno vuoto e sul progetto d'esempio. Un panic nell'app chiude
//! il programma e fa perdere il lavoro.

use cadcraft_engine::Session;
use serde_json::{Value, json};

/// Comandi che scrivono file o impostazioni fuori dal disegno: restano fuori dalla prova.
const ESCLUSI: &[&str] = &["commessa.", "commesse.", "configurazione.", "stampafogli", "excel", "targhette", "configuratore.genera", "traduttore.", "materiali.", "plc.importa", "memorizzafoglio", "tensioni"];

fn parametri() -> Vec<Value> {
    vec![
        json!({}),
        json!({"at": "x", "foglio": -1, "fogli": 0, "nome": 123, "handle": "ZZZ", "handles": ["ZZZ", 7], "da": [1], "a": null, "tipo": "???", "quadro": "", "colonna": -3, "testo": [], "valore": {}, "numero": "abc"}),
        json!({"at": [1e300, -1e300], "da": [0.0, 0.0], "a": [0.0, 0.0], "foglio": 99999, "fogli": -1, "handle": "", "nome": "", "simbolo": "", "potenza": -5, "lunghezza": "1,5,5", "in": 0, "sezione": 3, "cosfi": 0, "larghezza": 0, "altezza": -1}),
    ]
}

/// Esegue tutti i comandi con tutti i parametri e restituisce quelli andati in panic.
fn prova(s: &mut Session, stato: &str, solo_vuoti: bool) -> Vec<String> {
    let mut panic = Vec::new();
    for c in elettrico::comandi() {
        if ESCLUSI.iter().any(|e| c.id.starts_with(e)) {
            continue;
        }
        for p in parametri().into_iter().take(if solo_vuoti { 2 } else { 3 }) {
            let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| s.execute(c.id, &p)));
            if let Err(e) = r {
                let msg = e.downcast_ref::<String>().cloned().or_else(|| e.downcast_ref::<&str>().map(|t| (*t).to_string())).unwrap_or_default();
                panic.push(format!("{stato}: {} {p} → {msg}", c.id));
            }
        }
    }
    panic
}

#[test]
fn nessun_comando_va_in_panic() {
    let _uno_alla_volta = crate::uno_alla_volta();
    crate::registra();
    let mut trovati = Vec::new();
    let mut s = Session::empty();
    trovati.extend(prova(&mut s, "senza disegno", false));
    s.new_drawing(true);
    trovati.extend(prova(&mut s, "disegno vuoto", false));
    let mut s = Session::empty();
    s.new_drawing(true);
    elettrico::esempio::disegna(&mut s, &json!({"commessa": "PROVA", "multifoglio": "SCHEMA"})).unwrap();
    trovati.extend(prova(&mut s, "progetto d'esempio", true));
    elettrico::impostazioni::usa(None, elettrico::impostazioni::Impostazioni::default());
    assert!(trovati.is_empty(), "{} comandi in panic:\n{}", trovati.len(), trovati.join("\n"));
}
