//! Multifoglio senza interfaccia: creazione, navigazione, numerazione, annulla, DXF.

use cadcraft_engine::Session;
use serde_json::{Value, json};

fn prepara() -> Session {
    static UNA_VOLTA: std::sync::Once = std::sync::Once::new();
    UNA_VOLTA.call_once(|| {
        crate::registra();
    });
    let mut s = Session::empty();
    s.new_drawing(true);
    s
}

fn esegui(s: &mut Session, id: &str, p: Value) -> Value {
    match s.execute(id, &p) {
        Ok(v) => v,
        Err(e) => panic!("{id}: {e}"),
    }
}

fn numeri_fogli(s: &mut Session) -> Vec<(u64, bool)> {
    let v = esegui(s, "elencofogli", json!({}));
    v["fogli"].as_array().unwrap().iter().map(|f| (f["foglio"].as_u64().unwrap(), f["corrente"].as_bool().unwrap())).collect()
}

/// (numero, seguente, titolo) di ogni foglio, letti dai blocchi NUMFG, NUMFGS e TITOLO1.
fn cartigli(s: &mut Session) -> Vec<(String, String, String)> {
    let r = esegui(s, "dataextraction", json!({"blocks": "NUMFG,NUMFGS,TITOLO1", "combine": false}));
    let col: Vec<&str> = r["columns"].as_array().unwrap().iter().map(|c| c.as_str().unwrap()).collect();
    let i = |n: &str| col.iter().position(|c| *c == n).unwrap();
    let mut per_layer: std::collections::BTreeMap<u32, (String, String, String)> = Default::default();
    for riga in r["rows"].as_array().unwrap() {
        let layer: u32 = riga[i("Layer")].as_str().unwrap().parse().unwrap();
        let v = per_layer.entry(layer).or_default();
        let val = |t: &str| riga[i(t)].as_str().unwrap_or("").to_string();
        match riga[i("Name")].as_str().unwrap() {
            "NUMFG" => v.0 = val("NUMERO_DI_FOGLIO"),
            "NUMFGS" => v.1 = val("FOGLIO_SEGUENTE"),
            _ => v.2 = val("_TITOLO"),
        }
    }
    per_layer.into_values().collect()
}

#[test]
fn nuovo_multifoglio_navigazione_e_numerazione() {
    let _uno_alla_volta = crate::uno_alla_volta();
    let mut s = prepara();
    esegui(&mut s, "nuovomultifoglio", json!({"fogli": 3, "commessa": "PROVA", "cliente": "Officina Rossi"}));
    assert_eq!(numeri_fogli(&mut s), vec![(1, true), (2, false), (3, false)]);
    assert_eq!(
        cartigli(&mut s),
        vec![("01".into(), "02".into(), String::new()), ("02".into(), "03".into(), String::new()), ("03".into(), "/".into(), String::new())]
    );

    let v = esegui(&mut s, "fogliosuccessivo", json!({}));
    assert_eq!(v["foglio"], 2);
    esegui(&mut s, "fogliosuccessivo", json!({}));
    let v = esegui(&mut s, "fogliosuccessivo", json!({}));
    assert_eq!(v["foglio"], 3, "oltre l'ultimo foglio si resta sull'ultimo");
    esegui(&mut s, "cambiofoglio", json!({"foglio": 1}));
    assert_eq!(numeri_fogli(&mut s)[0], (1, true));
    assert!(s.execute("cambiofoglio", &json!({"foglio": 9})).is_err());

    // solo il foglio corrente è visibile (gli altri congelati), il foglio 0 sempre
    let layers = esegui(&mut s, "layer", json!({}));
    for l in layers["layers"].as_array().unwrap() {
        let nome = l["name"].as_str().unwrap();
        let congelato = l["frozen"].as_bool().unwrap();
        match nome {
            "0" | "1" => assert!(!congelato, "{nome} deve essere visibile"),
            "2" | "3" => assert!(congelato, "{nome} deve essere nascosto"),
            _ => {}
        }
    }

    esegui(&mut s, "titolofoglio", json!({"titolo": "Potenza"}));
    esegui(&mut s, "aggiungifogli", json!({"quanti": 2}));
    let c = cartigli(&mut s);
    assert_eq!(c.len(), 5);
    assert_eq!(c[0], ("01".into(), "02".into(), "Potenza".into()));
    assert_eq!(c[2], ("03".into(), "04".into(), String::new()), "il foglio 3 ora ha un seguente");

    let r = esegui(&mut s, "coordinata", json!({"at": [130.0, 200.0]}));
    assert_eq!(r["riferimento"], "01/2B");
}

#[test]
fn un_solo_annulla_toglie_il_multifoglio() {
    let _uno_alla_volta = crate::uno_alla_volta();
    let mut s = prepara();
    esegui(&mut s, "nuovomultifoglio", json!({"fogli": 2}));
    esegui(&mut s, "undo", json!({"count": 1}));
    assert!(numeri_fogli(&mut s).is_empty());
    let r = esegui(&mut s, "dataextraction", json!({"blocks": "MASTER*,NUMFG*,TITOLO*", "combine": false}));
    assert!(r["rows"].as_array().unwrap().is_empty());
}

#[test]
fn il_multifoglio_sopravvive_al_dxf() {
    let _uno_alla_volta = crate::uno_alla_volta();
    let mut s = prepara();
    esegui(&mut s, "nuovomultifoglio", json!({"fogli": 2, "commessa": "DXF"}));
    esegui(&mut s, "titolofoglio", json!({"foglio": 2, "titolo": "Ausiliari"}));
    let doc = s.doc().unwrap().clone();
    let byte = cadcraft_io::write(&doc, "prova.dxf").unwrap();
    let riletto = cadcraft_io::read(&byte, "prova.dxf").unwrap();
    let mut s2 = prepara();
    s2.open_drawing(riletto, "prova.dxf", None);
    let v = esegui(&mut s2, "elencofogli", json!({}));
    assert_eq!(v["fogli"][1]["titolo"], "Ausiliari");
    assert_eq!(cartigli(&mut s2)[1], ("02".into(), "/".into(), "Ausiliari".into()));
}
