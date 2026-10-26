//! Fili e collegamenti senza interfaccia.

use cadcraft_engine::Session;
use elettrico::fili::{self, BLOCCO_NODO};
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

fn connessioni(s: &Session, layer: &str) -> usize {
    s.doc()
        .unwrap()
        .model
        .iter()
        .filter(|e| e.common.layer == layer && matches!(&e.kind, cadcraft_engine::doc::EntityKind::Insert(i) if i.block == BLOCCO_NODO))
        .count()
}

/// RIFERIMENTO dei rimandi per (foglio, potenziale, partenza?).
fn riferimenti(s: &mut Session) -> Vec<(String, String, String, String)> {
    let r = esegui(s, "dataextraction", json!({"blocks": "_ALIM_*,_SIGN_*", "combine": false}));
    let col: Vec<&str> = r["columns"].as_array().unwrap().iter().map(|c| c.as_str().unwrap()).collect();
    let i = |n: &str| col.iter().position(|c| *c == n).unwrap();
    let mut v: Vec<(String, String, String, String)> = r["rows"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| {
            (x[i("Layer")].as_str().unwrap().into(), x[i("SEGNALE")].as_str().unwrap().into(), x[i("PRES")].as_str().unwrap().into(), x[i("REG")].as_str().unwrap().into())
        })
        .collect();
    v.sort();
    v
}

#[test]
fn alimentazioni_e_rimandi_tra_fogli() {
    let _uno_alla_volta = crate::uno_alla_volta();
    let mut s = prepara(3);
    esegui(&mut s, "alimentazioni", json!({"fili": 3, "da": [20.0, 270.0], "a": [400.0, 270.0]}));
    assert_eq!(fili::fili(&s, "1").unwrap().len(), 3);
    // senza arrivo, le partenze restano senza coppia
    assert!(riferimenti(&mut s).iter().all(|r| r.3 == "///"));

    esegui(&mut s, "cambiofoglio", json!({"foglio": 2}));
    esegui(&mut s, "alimentazioni", json!({"nomi": "L1 L2 L3", "da": [20.0, 270.0], "a": [200.0, 270.0], "inizio": "arrivo", "fine": "nessuno"}));
    let r = riferimenti(&mut s);
    assert_eq!(r.len(), 6);
    let trova = |layer: &str, pot: &str| r.iter().find(|x| x.0 == layer && x.1 == pot).unwrap().3.clone();
    assert_eq!(trova("1", "L1"), "02/0A", "la partenza indica dove arriva");
    assert_eq!(trova("2", "L1"), "01/9A", "l'arrivo indica da dove parte");
    assert_eq!(trova("2", "L3"), "01/9A");

    // un solo annulla toglie l'ultima alimentazione con le sue frecce
    esegui(&mut s, "undo", json!({"count": 1}));
    assert_eq!(fili::fili(&s, "2").unwrap().len(), 0);
    esegui(&mut s, "aggiornarimandi", json!({}));
    assert!(riferimenti(&mut s).iter().all(|r| r.3 == "///"));
}

#[test]
fn filo_rapido_e_collegamenti_mettono_i_pallini() {
    let _uno_alla_volta = crate::uno_alla_volta();
    let mut s = prepara(1);
    esegui(&mut s, "alimentazioni", json!({"fili": 3, "da": [20.0, 270.0], "a": [400.0, 270.0], "fine": "nessuno"}));
    // VV da L1: attraversa L2 e L3 (stesso fascio) e scende fino al cartiglio, pallino solo su L1
    esegui(&mut s, "vv", json!({"at": [100.0, 270.0]}));
    let f = fili::fili(&s, "1").unwrap();
    assert!(f.iter().any(|x| x.a == (100.0, 270.0) && x.b == (100.0, 45.0)), "fino al cartiglio");
    assert_eq!(connessioni(&s, "1"), 1);
    // HH da metà del filo verticale: va a destra fino al bordo, pallino sul T
    esegui(&mut s, "hh", json!({"at": [100.0, 150.0]}));
    assert_eq!(connessioni(&s, "1"), 2);
    // VV da L2: attraversa L3, si ferma sul filo orizzontale appena fatto
    esegui(&mut s, "vv", json!({"at": [300.0, 260.0]}));
    let f = fili::fili(&s, "1").unwrap();
    assert!(f.iter().any(|x| x.a == (300.0, 260.0) && x.b == (300.0, 150.0)), "{f:?}");
    assert_eq!(connessioni(&s, "1"), 4);

    // 3 calate da L2 che girano a destra: un pallino per filo sulla barra
    esegui(&mut s, "collegamenti", json!({"fili": 3, "points": [[150.0, 260.0], [150.0, 200.0], [250.0, 200.0]]}));
    assert_eq!(connessioni(&s, "1"), 7);
    let f = fili::fili(&s, "1").unwrap();
    assert!(f.iter().any(|x| x.a == (170.0, 260.0) && x.b == (170.0, 180.0)), "terzo filo: 20 mm a destra");
    assert!(f.iter().any(|x| x.a == (170.0, 180.0) && x.b == (250.0, 180.0)), "terzo filo: 20 mm sotto");
}

#[test]
fn le_linee_del_foglio_sono_fili_anche_dopo_il_dxf() {
    let _uno_alla_volta = crate::uno_alla_volta();
    let mut s = prepara(1);
    esegui(&mut s, "filo", json!({"points": [[50.0, 200.0], [150.0, 203.0], [150.0, 120.0]]}));
    let prima = fili::fili(&s, "1").unwrap();
    assert_eq!(prima.len(), 2);
    assert_eq!(prima[0].b, (150.0, 200.0), "tratto reso ortogonale");
    let byte = cadcraft_io::write(s.doc().unwrap(), "f.dxf").unwrap();
    let mut s2 = Session::empty();
    s2.open_drawing(cadcraft_io::read(&byte, "f.dxf").unwrap(), "f.dxf", None);
    assert_eq!(fili::fili(&s2, "1").unwrap().len(), 2);
}
