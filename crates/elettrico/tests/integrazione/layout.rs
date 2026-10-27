//! Lay-out del quadro: piastra, canaline, guide e ingombri dei componenti; foratura.

use cadcraft_engine::Session;
use serde_json::{Value, json};

fn esegui(s: &mut Session, id: &str, p: Value) -> Value {
    match s.execute(id, &p) {
        Ok(v) => v,
        Err(e) => panic!("{id}: {e}"),
    }
}

/// I blocchi inseriti `filtro`: attributi e colonne di DATAEXTRACTION per ognuno.
fn inseriti(s: &mut Session, filtro: &str) -> Vec<serde_json::Map<String, Value>> {
    let r = esegui(s, "dataextraction", json!({"blocks": filtro, "combine": false}));
    let col: Vec<String> = r["columns"].as_array().unwrap().iter().map(|c| c.as_str().unwrap().to_string()).collect();
    r["rows"].as_array().unwrap().iter().map(|x| col.iter().enumerate().map(|(k, c)| (c.clone(), x[k].clone())).collect()).collect()
}

fn schema() -> Session {
    crate::registra();
    let mut s = Session::empty();
    s.new_drawing(true);
    esegui(&mut s, "nuovomultifoglio", json!({"fogli": 1}));
    esegui(&mut s, "cambiofoglio", json!({"foglio": 1}));
    for (id, x) in [("qf3", 60.0), ("km", 120.0), ("ka", 180.0), ("kt", 240.0), ("m3", 300.0)] {
        esegui(&mut s, &format!("simbolo.{id}"), json!({"at": [x, 200.0], "quadro": "+QG"}));
    }
    // un componente di un altro quadro non entra
    esegui(&mut s, "simbolo.km", json!({"at": [360.0, 200.0], "quadro": "+BM"}));
    for x in [60.0, 80.0, 100.0] {
        esegui(&mut s, "filo", json!({"points": [[x, 120.0], [x, 60.0]]}));
    }
    esegui(&mut s, "morsetti", json!({"morsettiera": "-X1", "da": [50.0, 90.0], "a": [110.0, 90.0], "quadro": "+QG"}));
    s
}

#[test]
fn layout_e_foratura() {
    let _uno_alla_volta = crate::uno_alla_volta();
    let mut s = schema();
    let r = esegui(&mut s, "layoutquadro", json!({"quadro": "+QG"}));
    assert_eq!(r["foglio"], 2, "{r}");
    assert_eq!(r["posati"], 5, "4 componenti e la morsettiera, non il motore: {r}");
    assert_eq!(r["fuori"], json!([]));
    let blocchi = inseriti(&mut s, "CE_LAY_ING_*");
    let mut sigle: Vec<String> = blocchi.iter().map(|b| b["SIGLA"].as_str().unwrap().to_string()).collect();
    sigle.sort();
    assert_eq!(sigle.len(), 5, "{sigle:?}");
    assert!(sigle.iter().any(|x| x == "-X1"));
    assert!(sigle.iter().all(|x| !x.starts_with("-M")), "{sigle:?}");
    let fogli = elettrico::fogli::elenco(&mut s).unwrap();
    assert_eq!(fogli[1].titolo, "LAYOUT INTERNO QUADRO +QG");
    // la protezione viene prima del contattore sulla prima guida
    let x = |sigla: &str| {
        let b = blocchi.iter().find(|b| b["SIGLA"].as_str().unwrap().starts_with(sigla)).unwrap();
        (b["Position X"].as_f64().unwrap(), b["Position Y"].as_f64().unwrap())
    };
    assert!(x("-QF").0 < x("-KM").0);
    assert!((x("-QF").1 - x("-KM").1).abs() < 1e-6, "stessa guida");
    assert!(x("-X1").1 < x("-QF").1, "la morsettiera su una guida più in basso");

    // rifatto sullo stesso foglio: non si duplica
    esegui(&mut s, "layoutquadro", json!({"quadro": "+QG", "foglio": 2, "larghezza": 800.0, "altezza": 1000.0}));
    assert_eq!(inseriti(&mut s, "CE_LAY_ING_*").len(), 5);
    assert_eq!(inseriti(&mut s, "CE_LAY_PIASTRA").len(), 1);

    let file = std::env::temp_dir().join(format!("wirecraft_fori_{}.csv", std::process::id()));
    let r = esegui(&mut s, "foraturapiastra", json!({"foglio": 2, "file": file.to_string_lossy()}));
    let fori = r["fori"].as_array().unwrap();
    assert!(fori.len() > 20, "{r}");
    for f in fori {
        let (x, y): (f64, f64) = (f[1].as_str().unwrap().parse().unwrap(), f[2].as_str().unwrap().parse().unwrap());
        assert!(x > 0.0 && x < 800.0 && y > 0.0 && y < 1000.0, "foro fuori dalla piastra: {f}");
    }
    let csv = std::fs::read_to_string(&file).unwrap();
    assert!(csv.starts_with("N;X;Y;Diametro;Elemento"));
    assert!(csv.contains("Guida DIN") && csv.contains("Canalina"));
    // i fori disegnati si rifanno, non si sommano
    esegui(&mut s, "foraturapiastra", json!({"foglio": 2}));
    assert_eq!(inseriti(&mut s, "CE_LAY_FORI_*").len(), 1);
    let _ = std::fs::remove_file(&file);
}

#[test]
fn gli_ingombri_non_sono_componenti() {
    let _uno_alla_volta = crate::uno_alla_volta();
    let mut s = schema();
    let prima = elettrico::siglatura::componenti(&mut s).unwrap().len();
    esegui(&mut s, "layoutquadro", json!({"quadro": "+QG"}));
    assert_eq!(elettrico::siglatura::componenti(&mut s).unwrap().len(), prima, "la distinta e il cross reference non li contano");
}
