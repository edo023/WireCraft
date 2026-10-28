//! Ponticelli tra i morsetti di una morsettiera: automatici sullo stesso filo e a mano.

use cadcraft_engine::Session;
use serde_json::{Value, json};

fn esegui(s: &mut Session, id: &str, p: Value) -> Value {
    match s.execute(id, &p) {
        Ok(v) => v,
        Err(e) => panic!("{id}: {e}"),
    }
}

/// Il gruppo di ponte di ogni morsetto di `-X1`, in ordine.
fn ponti(s: &mut Session) -> Vec<String> {
    elettrico::morsetti::righe(s, "-X1").unwrap().into_iter().map(|r| r[4].clone()).collect()
}

#[test]
fn ponticelli_automatici_e_a_mano() {
    let _uno_alla_volta = crate::uno_alla_volta();
    crate::registra();
    let mut s = Session::empty();
    s.new_drawing(true);
    esegui(&mut s, "nuovomultifoglio", json!({"fogli": 1}));
    esegui(&mut s, "cambiofoglio", json!({"foglio": 1}));
    esegui(&mut s, "alimentazioni", json!({"nomi": "L1 N PE", "da": [20.0, 250.0], "a": [300.0, 250.0]}));
    // un morsetto su L1, due su N, due su PE
    let reti = elettrico::collegamenti::reti(&mut s, &[]).unwrap();
    let quota = |nome: &str| reti.iter().find(|r| r.nome == nome).unwrap().fili[0].a.1;
    for (x, linea) in [(60.0, "L1"), (80.0, "N"), (100.0, "N"), (120.0, "PE"), (140.0, "PE")] {
        esegui(&mut s, "filo", json!({"points": [[x, quota(linea)], [x, 100.0]]}));
    }
    esegui(&mut s, "morsetti", json!({"morsettiera": "-X1", "da": [50.0, 150.0], "a": [150.0, 150.0]}));
    esegui(&mut s, "numerafili", json!({"tutti": true}));
    assert_eq!(ponti(&mut s), ["", "", "", "", ""]);

    // automatici: N con N, PE con PE
    let r = esegui(&mut s, "ponticelli.auto", json!({}));
    assert_eq!(r["ponticelli"], 2, "{r}");
    assert_eq!(ponti(&mut s), ["", "P1", "P1", "P2", "P2"]);
    // la seconda volta non cambia niente
    assert_eq!(esegui(&mut s, "ponticelli.auto", json!({}))["ponticelli"], 0);

    // a mano: via il primo, poi uno nuovo tra 1 e 3
    esegui(&mut s, "ponticello", json!({"morsettiera": "-X1", "morsetti": "2-3", "togli": true}));
    assert_eq!(ponti(&mut s), ["", "", "", "P2", "P2"]);
    let r = esegui(&mut s, "ponticello", json!({"morsettiera": "-X1", "morsetti": [1, 3]}));
    assert_eq!(r["ponte"], "P1");
    assert_eq!(ponti(&mut s), ["P1", "", "P1", "P2", "P2"]);
    assert!(s.execute("ponticello", &json!({"morsettiera": "-X1", "morsetti": "4"})).is_err(), "almeno due morsetti");
    assert!(s.execute("ponticello", &json!({"morsettiera": "-X1", "morsetti": "4, 9"})).is_err(), "morsetto che non c'è");
    assert!(s.execute("ponticello", &json!({"morsettiera": "-X9", "morsetti": "1-2"})).is_err());

    // nella lista delle morsettiere
    esegui(&mut s, "uscita", json!({"tipo": "morsettiere"}));
    assert_eq!(elettrico::morsetti::numeri_da("1-3, 7"), ["1", "2", "3", "7"]);
}
