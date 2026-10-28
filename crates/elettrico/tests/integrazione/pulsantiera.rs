//! Pulsantiera sulla porta: dispositivi del quadro, disposizione, foratura, uscita sul foglio.

use cadcraft_engine::Session;
use serde_json::{Value, json};

fn esegui(s: &mut Session, id: &str, p: Value) -> Value {
    match s.execute(id, &p) {
        Ok(v) => v,
        Err(e) => panic!("{id}: {e}"),
    }
}

#[test]
fn dispositivi_sulla_porta_e_foratura() {
    let _uno_alla_volta = crate::uno_alla_volta();
    crate::registra();
    let mut s = Session::empty();
    s.new_drawing(true);
    esegui(&mut s, "nuovomultifoglio", json!({"fogli": 1}));
    esegui(&mut s, "cambiofoglio", json!({"foglio": 1}));
    let mut sigle = Vec::new();
    for (k, id) in ["sbno", "sbnc", "sa", "hl", "sbem", "sbno"].iter().enumerate() {
        let r = esegui(&mut s, &format!("simbolo.{id}"), json!({"at": [40.0 + 50.0 * k as f64, 200.0], "quadro": "+QG", "funzione1": format!("Funzione {k}")}));
        sigle.push(r["nome"].as_str().unwrap().to_string());
    }
    // un sensore e un pulsante di un'altra pulsantiera non vanno su questa porta
    esegui(&mut s, "simbolo.sqno", json!({"at": [40.0, 100.0], "quadro": "+QG"}));
    esegui(&mut s, "simbolo.sbno", json!({"at": [100.0, 100.0], "quadro": "+PL"}));

    let r = esegui(&mut s, "pulsantiera", json!({"quadro": "+QG", "larghezza": 300, "altezza": 600}));
    let v = r["dispositivi"].as_array().unwrap();
    assert_eq!(v.len(), 6, "{r}");
    assert_eq!(v[0]["tipo"], "sbem", "l'emergenza per prima");
    // tutti dentro la porta, a passo costante, la prima fila 150 mm sotto il bordo
    assert!(v.iter().all(|d| (0.0..=300.0).contains(&d["x"].as_f64().unwrap()) && (0.0..=600.0).contains(&d["y"].as_f64().unwrap())), "{r}");
    assert_eq!(v[0]["y"], 450.0);
    let file_ = v.iter().map(|d| d["y"].as_f64().unwrap() as i64).collect::<std::collections::BTreeSet<_>>().len();
    assert_eq!(file_, 2, "6 dispositivi su una porta da 300: quattro e due");
    // l'ultima fila è centrata sulla porta
    let ultima: Vec<f64> = v.iter().filter(|d| d["y"].as_f64().unwrap() < 450.0).map(|d| d["x"].as_f64().unwrap()).collect();
    assert!((ultima.iter().sum::<f64>() / ultima.len() as f64 - 150.0).abs() < 1e-6, "{ultima:?}");

    // la foratura in CSV
    let dir = std::env::temp_dir().join(format!("cadeo_pulsantiera_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let f = dir.join("foratura.csv");
    esegui(&mut s, "pulsantiera", json!({"quadro": "+QG", "larghezza": 400, "altezza": 600, "file": f.to_string_lossy()}));
    let csv = std::fs::read_to_string(&f).unwrap();
    assert!(csv.starts_with("Sigla;Funzione;X;Y;Diametro\n"), "{csv}");
    assert_eq!(csv.lines().count(), 7);
    assert!(csv.contains(";22.5\n"));

    // troppi dispositivi per una porta piccola
    assert!(s.execute("pulsantiera", &json!({"quadro": "+QG", "larghezza": 150, "altezza": 200})).is_err());

    // sul foglio, e in tabella per Excel
    let u = esegui(&mut s, "uscita", json!({"tipo": "pulsantiera", "quadro": "+QG", "larghezza": 400, "altezza": 600}));
    let n = u["fogli"][0].as_u64().unwrap() as u32;
    assert_eq!(elettrico::fogli::elenco(&mut s).unwrap().iter().find(|x| x.numero == n).unwrap().titolo, "Pulsantiera");
    let x = dir.join("pulsantiera.xlsx");
    let r = esegui(&mut s, "excel", json!({"tipo": "pulsantiera", "quadro": "+QG", "file": x.to_string_lossy()}));
    assert_eq!(r["righe"], 6);
    esegui(&mut s, "aggiornaliste", json!({}));
    assert!(s.execute("uscita", &json!({"tipo": "pulsantiera", "quadro": "+XX"})).is_err(), "quadro senza dispositivi");
    let _ = std::fs::remove_dir_all(&dir);
}
