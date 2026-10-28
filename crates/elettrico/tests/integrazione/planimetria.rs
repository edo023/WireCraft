//! Planimetria: quadri sulla pianta (anche uno dopo l'altro), lunghezze dei cavi dai percorsi
//! con risalite e scorta, solo dove mancano o per tutti, percorsi ridisegnati.

use cadcraft_engine::geom::Vec2;
use cadcraft_engine::{Input, Session};
use serde_json::{Value, json};

fn esegui(s: &mut Session, id: &str, p: Value) -> Value {
    match s.execute(id, &p) {
        Ok(v) => v,
        Err(e) => panic!("{id}: {e}"),
    }
}

fn lunghezza(s: &mut Session, sigla: &str) -> String {
    elettrico::cavi::cavi(s).unwrap().into_iter().find(|c| c.sigla == sigla).map(|c| c.lunghezza).unwrap_or_default()
}

#[test]
fn quadri_sulla_pianta_e_lunghezze_dei_cavi() {
    let _uno_alla_volta = crate::uno_alla_volta();
    crate::registra();
    let mut s = Session::empty();
    s.new_drawing(true);
    esegui(&mut s, "nuovomultifoglio", json!({"fogli": 1}));
    esegui(&mut s, "cambiofoglio", json!({"foglio": 1}));
    esegui(&mut s, "simbolo.qf1", json!({"at": [60.0, 220.0], "quadro": "+QG"}));
    esegui(&mut s, "simbolo.qf1", json!({"at": [160.0, 220.0], "quadro": "+QD"}));
    let w1 = esegui(&mut s, "cavo.definisci", json!({"formazione": "4G1,5", "daQuadro": "+QG", "aQuadro": "+BM"}))["sigla"].as_str().unwrap_or("-W01").to_string();
    let w2 = esegui(&mut s, "cavo.definisci", json!({"formazione": "5G4", "daQuadro": "+QD", "aQuadro": "+QG", "lunghezza": "50"}))["sigla"].as_str().unwrap_or("-W02").to_string();

    let n = esegui(&mut s, "planimetria.nuovo", json!({}));
    assert_eq!(n["foglio"], 2);
    esegui(&mut s, "planimetria.posa", json!({"quadro": "+QG", "at": [50.0, 100.0]}));
    esegui(&mut s, "planimetria.posa", json!({"quadro": "BM", "at": [150.0, 150.0]}));
    assert!(s.execute("planimetria.posa", &json!({"quadro": "+QG", "at": [10.0, 10.0]})).is_err(), "già posato");
    // il terzo uno dopo l'altro: Invio prende il prossimo non posato
    s.start("planimetria.posa").unwrap();
    s.input(Input::Enter).unwrap();
    s.input(Input::Point(Vec2::new(50.0, 200.0))).unwrap();
    s.input(Input::Cancel).unwrap();
    let e = esegui(&mut s, "planimetria", json!({}));
    assert!(e["quadri"].as_array().unwrap().iter().all(|q| q["posato"] == true), "{e}");

    // solo dove manca: 150 mm a 1:100 = 15 m + 2 + 2 = 19, +10 % → 21
    esegui(&mut s, "planimetria.lunghezze", json!({"soloMancanti": true}));
    assert_eq!((lunghezza(&mut s, &w1), lunghezza(&mut s, &w2)), ("21".to_string(), "50".to_string()));
    // per tutti: 100 mm = 10 m + 4 = 14, +10 % → 16
    let r = esegui(&mut s, "planimetria.lunghezze", json!({}));
    assert_eq!(lunghezza(&mut s, &w2), "16", "{r}");
    let percorsi = esegui(&mut s, "dataextraction", json!({"blocks": "CE_PLAN_PERCORSI_*", "combine": false}));
    assert_eq!(percorsi["rows"].as_array().map(Vec::len), Some(1), "un solo blocco dei percorsi sulla pianta");
    // senza scorta né risalite
    esegui(&mut s, "planimetria.lunghezze", json!({"scorta": 0, "risalita": 0}));
    assert_eq!(lunghezza(&mut s, &w2), "10");
}
