//! Numerazione dei fili: automatica, manuale (numeri e stringhe), fili bloccati, numeri
//! mantenuti e numeri ripetuti.

use cadcraft_engine::Session;
use serde_json::{Value, json};

fn prepara(fogli: u32) -> Session {
    static UNA_VOLTA: std::sync::Once = std::sync::Once::new();
    UNA_VOLTA.call_once(|| {
        crate::registra();
    });
    let mut s = Session::empty();
    s.new_drawing(true);
    esegui(&mut s, "nuovomultifoglio", json!({"fogli": fogli}));
    s
}

fn esegui(s: &mut Session, id: &str, p: Value) -> Value {
    match s.execute(id, &p) {
        Ok(v) => v,
        Err(e) => panic!("{id}: {e}"),
    }
}

/// I numeri scritti sul foglio, da sinistra a destra: (x del filo, numero, bloccato).
fn numeri(s: &mut Session, layer: &str) -> Vec<(f64, String, bool)> {
    let r = esegui(s, "dataextraction", json!({"blocks": "CE_NUMFILO", "combine": false}));
    let col: Vec<&str> = r["columns"].as_array().unwrap().iter().map(|c| c.as_str().unwrap()).collect();
    let i = |n: &str| col.iter().position(|c| *c == n).unwrap();
    let mut v: Vec<(f64, String, bool)> = r["rows"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|x| x[i("Layer")] == layer)
        .map(|x| (x[i("Position X")].as_f64().unwrap() + 1.0, x[i("N")].as_str().unwrap_or("").to_string(), col.contains(&"BLOCCATO") && x[i("BLOCCATO")] == "1"))
        .collect();
    v.sort_by(|a, b| a.0.total_cmp(&b.0));
    v
}

fn soli_numeri(v: &[(f64, String, bool)]) -> Vec<String> {
    v.iter().map(|x| x.1.clone()).collect()
}

#[test]
fn manuale_bloccati_e_mantenuti() {
    let _uno_alla_volta = crate::uno_alla_volta();
    let mut s = prepara(1);
    for x in [100.0, 150.0, 200.0, 250.0] {
        esegui(&mut s, "filo", json!({"points": [[x, 250.0], [x, 100.0]]}));
    }
    esegui(&mut s, "numerafili", json!({}));
    assert_eq!(soli_numeri(&numeri(&mut s, "1")), ["01.01", "01.02", "01.03", "01.04"]);

    // manuale con fence sui due fili centrali: 50 e 51, il secondo bloccato dopo
    let r = esegui(&mut s, "numerafilo", json!({"numero": "50", "da": [140.0, 180.0], "a": [210.0, 180.0]}));
    assert_eq!(r["numeri"], json!(["01.50", "01.51"]));
    assert_eq!(soli_numeri(&numeri(&mut s, "1")), ["01.01", "01.50", "01.51", "01.04"]);
    // stringa libera con l'apice, e bloccata
    esegui(&mut s, "numerafilo", json!({"numero": "'24V", "at": [250.0, 160.0], "blocca": true}));
    assert_eq!(numeri(&mut s, "1")[3].1, "24V");
    assert!(numeri(&mut s, "1")[3].2, "bloccato");

    // automatica mantenendo i numeri: nessun cambio
    esegui(&mut s, "numerafili", json!({"mantieni": true}));
    assert_eq!(soli_numeri(&numeri(&mut s, "1")), ["01.01", "01.50", "01.51", "24V"]);
    // automatica da capo: il bloccato resta
    esegui(&mut s, "numerafili", json!({}));
    assert_eq!(soli_numeri(&numeri(&mut s, "1")), ["01.01", "01.02", "01.03", "24V"]);
    // la manuale non cambia un filo bloccato; sbloccato sì
    let r = esegui(&mut s, "numerafilo", json!({"numero": "9", "at": [250.0, 160.0]}));
    assert_eq!(r["numeri"].as_array().unwrap().len(), 0);
    esegui(&mut s, "bloccafili", json!({"at": [250.0, 160.0], "bloccato": false}));
    esegui(&mut s, "numerafilo", json!({"numero": "9", "at": [250.0, 160.0]}));
    assert_eq!(numeri(&mut s, "1")[3].1, "01.09");

    // un numero ripetuto per errore
    esegui(&mut s, "numerafilo", json!({"numero": "'01.01", "at": [200.0, 160.0]}));
    let l = esegui(&mut s, "listanumerifili", json!({}));
    assert_eq!(l["ripetuti"], json!(["01.01"]));
}

#[test]
fn i_fili_dei_rimandi_prendono_il_segnale() {
    let _uno_alla_volta = crate::uno_alla_volta();
    let mut s = prepara(2);
    esegui(&mut s, "alimentazioni", json!({"nomi": "L1 N", "da": [30.0, 250.0], "a": [400.0, 250.0]}));
    esegui(&mut s, "vv", json!({"at": [100.0, 250.0]}));
    esegui(&mut s, "numerafili", json!({}));
    let n = soli_numeri(&numeri(&mut s, "1"));
    assert!(n.contains(&"L1".to_string()), "{n:?}");
    // il filo di un'alimentazione non si rinumera a mano
    let r = esegui(&mut s, "numerafilo", json!({"numero": "7", "at": [100.0, 200.0]}));
    assert_eq!(r["numeri"].as_array().unwrap().len(), 0, "{r}");
}
