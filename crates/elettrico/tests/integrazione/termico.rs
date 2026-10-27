//! Calcolo termico del quadro: potenze dissipate, armadio dalla piastra, uscita sul foglio.

use cadcraft_engine::Session;
use serde_json::{Value, json};

fn esegui(s: &mut Session, id: &str, p: Value) -> Value {
    match s.execute(id, &p) {
        Ok(v) => v,
        Err(e) => panic!("{id}: {e}"),
    }
}

fn vicino(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6
}

#[test]
fn potenze_armadio_ed_esito() {
    let _uno_alla_volta = crate::uno_alla_volta();
    crate::registra();
    let mut s = Session::empty();
    s.new_drawing(true);
    esegui(&mut s, "nuovomultifoglio", json!({"fogli": 1}));
    esegui(&mut s, "cambiofoglio", json!({"foglio": 1}));
    let mut km = String::new();
    for (id, x) in [("qf3", 60.0), ("km", 120.0), ("ka", 180.0), ("kt", 240.0), ("m3", 300.0)] {
        let r = esegui(&mut s, &format!("simbolo.{id}"), json!({"at": [x, 200.0], "quadro": "+QG"}));
        if id == "km" {
            km = r["handle"].as_str().unwrap().to_string();
        }
    }
    // un componente di un altro quadro non conta
    esegui(&mut s, "simbolo.km", json!({"at": [360.0, 200.0], "quadro": "+BM"}));
    for x in [60.0, 80.0, 100.0] {
        esegui(&mut s, "filo", json!({"points": [[x, 120.0], [x, 60.0]]}));
    }
    esegui(&mut s, "morsetti", json!({"morsettiera": "-X1", "da": [50.0, 90.0], "a": [110.0, 90.0], "quadro": "+QG"}));

    // stime: magnetotermico 4,5 + contattore 2,5 + relè 0,8 + temporizzatore 1,5 + 3 morsetti 0,3;
    // il motore sta fuori dal quadro. Senza lay-out: armadio 600 × 800 × 250 a parete
    let r = esegui(&mut s, "calcolotermico", json!({"quadro": "+QG"}));
    assert!(vicino(r["potenza"].as_f64().unwrap(), 9.6), "{r}");
    assert_eq!(r["voci"].as_array().unwrap().len(), 5, "{r}");
    assert!(r["voci"].as_array().unwrap().iter().all(|v| v["stimata"] == true));
    assert_eq!(r["armadio"], json!({"larghezza": 600.0, "altezza": 800.0, "profondita": 250.0, "installazione": 2, "materiale": "acciaio"}));
    let a = 1.4 * 0.6 * (0.8 + 0.25) + 1.8 * 0.25 * 0.8;
    assert!(vicino(r["superficie"].as_f64().unwrap(), a));
    assert!(vicino(r["tInterna"].as_f64().unwrap(), 35.0 + 9.6 / (5.5 * a)));
    assert!(r["ventilatore"].is_null() && r["condizionatore"] == false, "poca potenza: niente raffreddamento");

    // la potenza vera del contattore (dalla finestra del simbolo) vale più della stima
    esegui(&mut s, "editasimbolo", json!({"handle": km, "dissipazione": "10,5"}));
    let r = esegui(&mut s, "calcolotermico", json!({"quadro": "+QG"}));
    assert!(vicino(r["potenza"].as_f64().unwrap(), 9.6 - 2.5 + 10.5), "{r}");
    let voce = r["voci"].as_array().unwrap().iter().find(|v| v["watt"] == 10.5).cloned().unwrap();
    assert_eq!(voce["stimata"], false);
    assert!(s.execute("editasimbolo", &json!({"handle": km, "dissipazione": "tanto"})).is_err());

    // con il lay-out l'armadio prende le misure della piastra
    esegui(&mut s, "layoutquadro", json!({"quadro": "+QG"}));
    let piastra = esegui(&mut s, "dataextraction", json!({"blocks": elettrico::layout::BLOCCO_PIASTRA, "combine": false}));
    let col: Vec<&str> = piastra["columns"].as_array().unwrap().iter().map(|c| c.as_str().unwrap()).collect();
    let riga = &piastra["rows"][0];
    let misura = |t: &str| riga[col.iter().position(|c| *c == t).unwrap()].as_str().unwrap().parse::<f64>().unwrap();
    let r = esegui(&mut s, "calcolotermico", json!({"quadro": "+QG"}));
    assert_eq!((r["armadio"]["larghezza"].as_f64().unwrap(), r["armadio"]["altezza"].as_f64().unwrap()), (misura("LARGHEZZA"), misura("ALTEZZA")));

    // un armadietto piccolo e caldo: ventilatore; con l'esterno a 40 °C: condizionatore
    esegui(&mut s, "editasimbolo", json!({"handle": km, "dissipazione": 200}));
    let piccolo = json!({"quadro": "+QG", "larghezza": 300, "altezza": 300, "profondita": 150, "tEsterna": 30, "tMassima": 35});
    let r = esegui(&mut s, "calcolotermico", piccolo.clone());
    assert!(r["ventilatore"].as_f64().unwrap() > 0.0 && r["condizionatore"] == false, "{r}");
    let mut caldo = piccolo.clone();
    caldo["tEsterna"] = json!(40);
    let r = esegui(&mut s, "calcolotermico", caldo);
    assert!(r["condizionatore"] == true && r["ventilatore"].is_null(), "{r}");
    assert!(s.execute("calcolotermico", &json!({"installazione": 9})).is_err());
    assert!(s.execute("calcolotermico", &json!({"materiale": "legno"})).is_err());

    // sul foglio, come uscita aggiornabile che ricorda i dati dell'armadio
    let f = esegui(&mut s, "uscita", json!({"tipo": "termico", "quadro": "+QG", "larghezza": 300, "altezza": 300, "profondita": 150, "installazione": 1}));
    let n = f["fogli"][0].as_u64().unwrap() as u32;
    let fogli = elettrico::fogli::elenco(&mut s).unwrap();
    assert_eq!(fogli.iter().find(|x| x.numero == n).unwrap().titolo, "Calcolo termico");
    let pagina = esegui(&mut s, "dataextraction", json!({"blocks": format!("CE_USCITA_{n}"), "combine": false}));
    let col: Vec<&str> = pagina["columns"].as_array().unwrap().iter().map(|c| c.as_str().unwrap()).collect();
    let par: Value = serde_json::from_str(pagina["rows"][0][col.iter().position(|c| *c == "PARAMETRI").unwrap()].as_str().unwrap()).unwrap();
    assert_eq!((par["installazione"].as_u64(), par["larghezza"].as_u64(), par["quadro"].as_str()), (Some(1), Some(300), Some("+QG")));
    esegui(&mut s, "aggiornaliste", json!({}));
}
