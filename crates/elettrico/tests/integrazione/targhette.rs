//! Estrazione targhette: componenti, fili, morsetti e cavi in CSV.

use cadcraft_engine::Session;
use serde_json::{Value, json};

fn esegui(s: &mut Session, id: &str, p: Value) -> Value {
    match s.execute(id, &p) {
        Ok(v) => v,
        Err(e) => panic!("{id}: {e}"),
    }
}

fn testi(r: &Value) -> Vec<String> {
    r["righe"].as_array().unwrap().iter().map(|x| x[0].as_str().unwrap().to_string()).collect()
}

#[test]
fn targhette_per_tipo_e_quadro() {
    let _uno_alla_volta = crate::uno_alla_volta();
    crate::registra();
    let mut s = Session::empty();
    s.new_drawing(true);
    esegui(&mut s, "nuovomultifoglio", json!({"fogli": 1}));
    esegui(&mut s, "cambiofoglio", json!({"foglio": 1}));
    for x in [60.0, 80.0, 100.0] {
        esegui(&mut s, "filo", json!({"points": [[x, 200.0], [x, 60.0]]}));
    }
    let qf = esegui(&mut s, "simbolo.qf3", json!({"at": [60.0, 180.0], "quadro": "+QG", "funzione1": "Interruttore generale"}))["nome"].as_str().unwrap().to_string();
    esegui(&mut s, "simbolo.km", json!({"at": [250.0, 180.0], "quadro": "+QG"}));
    esegui(&mut s, "morsetti", json!({"morsettiera": "-X1", "da": [50.0, 90.0], "a": [110.0, 90.0], "quadro": "+QG"}));
    esegui(&mut s, "cavo.definisci", json!({"formazione": "4G1,5", "funzione": "Motore", "daQuadro": "+QG", "aQuadro": "+BM"}));
    esegui(&mut s, "numerafili", json!({"tutti": true}));

    let r = esegui(&mut s, "targhette", json!({"tipo": "componenti"}));
    assert_eq!(testi(&r).len(), 2, "{r}");
    let riga_qf = r["righe"].as_array().unwrap().iter().find(|x| x[0] == qf.as_str()).unwrap().clone();
    assert_eq!(riga_qf, json!([qf, "Interruttore generale", "+QG", "1"]));
    assert_eq!(testi(&esegui(&mut s, "targhette", json!({"tipo": "componenti", "quadro": "+BM"}))).len(), 0);

    // morsetti: la testata e un numero per morsetto, in ordine naturale
    let r = esegui(&mut s, "targhette", json!({"tipo": "morsetti"}));
    assert_eq!(testi(&r), ["-X1", "-X1:1", "-X1:2", "-X1:3"]);

    // fili: una targhetta per capo collegato, tutte col numero del filo
    let r = esegui(&mut s, "targhette", json!({"tipo": "fili"}));
    assert!(r["targhette"].as_u64().unwrap() >= 6, "tre fili, due capi ciascuno almeno: {r}");
    assert!(testi(&r).iter().all(|t| !t.is_empty()));

    // cavi: un'estremità nel quadro e una a bordo macchina
    let r = esegui(&mut s, "targhette", json!({"tipo": "cavi"}));
    let quadri: Vec<&str> = r["righe"].as_array().unwrap().iter().map(|x| x[2].as_str().unwrap()).collect();
    assert_eq!(quadri, ["+BM", "+QG"], "{r}");
    assert!(s.execute("targhette", &json!({"tipo": "boh"})).is_err());

    // il CSV per la stampante
    let dir = std::env::temp_dir().join(format!("cadeo_targhette_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let f = dir.join("morsetti.csv");
    esegui(&mut s, "targhette", json!({"tipo": "morsetti", "file": f.to_string_lossy()}));
    let csv = std::fs::read_to_string(&f).unwrap();
    assert!(csv.starts_with("Testo;Descrizione;Quadro;Copie\n-X1;Morsettiera;+QG;1\n-X1:1;;+QG;1\n"), "{csv}");
    let _ = std::fs::remove_dir_all(&dir);
}
