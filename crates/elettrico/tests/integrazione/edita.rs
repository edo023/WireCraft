//! Edita Fili e Morsetti (XS): morsetti, numeri dei fili e rimandi modificati dopo la posa.

use cadcraft_engine::Session;
use serde_json::{Value, json};

fn prepara(fogli: u32) -> Session {
    crate::registra();
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

/// I blocchi `nome` del layer: (handle, x, attributi).
fn blocchi(s: &mut Session, nome: &str, layer: &str) -> Vec<(String, f64, serde_json::Map<String, Value>)> {
    let r = esegui(s, "dataextraction", json!({"blocks": nome, "combine": false}));
    let col: Vec<String> = r["columns"].as_array().unwrap().iter().map(|c| c.as_str().unwrap().to_string()).collect();
    let mut v: Vec<_> = r["rows"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|x| x[col.iter().position(|c| c == "Layer").unwrap()] == layer)
        .map(|x| {
            let m: serde_json::Map<String, Value> = col.iter().enumerate().map(|(k, c)| (c.clone(), x[k].clone())).collect();
            (m["Handle"].as_str().unwrap().to_string(), m["Position X"].as_f64().unwrap(), m)
        })
        .collect();
    v.sort_by(|a, b| a.1.total_cmp(&b.1));
    v
}

#[test]
fn edita_morsetto_controlla_i_doppi() {
    let _uno_alla_volta = crate::uno_alla_volta();
    let mut s = prepara(1);
    for x in [100.0, 150.0] {
        esegui(&mut s, "filo", json!({"points": [[x, 250.0], [x, 100.0]]}));
    }
    esegui(&mut s, "morsetti", json!({"morsettiera": "-X1", "da": [90.0, 200.0], "a": [160.0, 200.0]}));
    let m = blocchi(&mut s, "CE_MORSETTO", "1");
    assert_eq!(m.len(), 2);
    let (h2, _, _) = &m[1];
    assert_eq!(esegui(&mut s, "xs.riconosci", json!({"handle": h2}))["tipo"], "morsetto");

    // il numero 1 c'è già nella morsettiera
    let e = s.execute("editamorsetto", &json!({"handle": h2, "numero": "1"})).unwrap_err();
    assert!(e.to_string().contains("esiste già"), "{e}");
    // in un'altra morsettiera va bene
    esegui(&mut s, "editamorsetto", json!({"handle": h2, "morsettiera": "-X2", "numero": "1", "sezione": "2.5", "utenza": "Pompa"}));
    let m = blocchi(&mut s, "CE_MORSETTO", "1");
    assert_eq!(m[1].2["MORSETTIERA"], "-X2");
    assert_eq!(m[1].2["NUMM"], "1");
    assert_eq!(m[1].2["SEZIONE"], "2.5");
    assert_eq!(m[1].2["UTENZA"], "Pompa");
    // gli altri dati restano
    assert_eq!(m[1].2["LIVELLO"], "1");
}

#[test]
fn edita_filo_rinomina_la_rete_e_la_blocca() {
    let _uno_alla_volta = crate::uno_alla_volta();
    let mut s = prepara(1);
    esegui(&mut s, "filo", json!({"points": [[100.0, 250.0], [100.0, 100.0]]}));
    esegui(&mut s, "filo", json!({"points": [[150.0, 250.0], [150.0, 100.0]]}));
    esegui(&mut s, "morsetti", json!({"morsettiera": "-X1", "at": [100.0, 200.0]}));
    esegui(&mut s, "numerafili", json!({}));
    let n = blocchi(&mut s, "CE_NUMFILO", "1");
    assert_eq!(n.len(), 2);
    let h = n[0].0.clone();
    assert_eq!(esegui(&mut s, "xs.riconosci", json!({"handle": h}))["tipo"], "filo");

    esegui(&mut s, "editafilo", json!({"handle": h, "numero": "'MOT1"}));
    let n = blocchi(&mut s, "CE_NUMFILO", "1");
    assert_eq!(n[0].2["N"], "MOT1");
    assert_eq!(n[0].2["BLOCCATO"], "1", "cambiato a mano resta bloccato");
    // il morsetto sul filo prende il nuovo numero
    let m = blocchi(&mut s, "CE_MORSETTO", "1");
    assert_eq!(m[0].2["NUMI"], "MOT1");
    // la numerazione automatica non lo cambia
    esegui(&mut s, "numerafili", json!({}));
    assert_eq!(blocchi(&mut s, "CE_NUMFILO", "1")[0].2["N"], "MOT1");
    // sbloccato sì
    esegui(&mut s, "editafilo", json!({"handle": h, "bloccato": false}));
    esegui(&mut s, "numerafili", json!({}));
    assert_ne!(blocchi(&mut s, "CE_NUMFILO", "1")[0].2["N"], "MOT1");
}

#[test]
fn edita_rimando_ricollega_i_riferimenti() {
    let _uno_alla_volta = crate::uno_alla_volta();
    let mut s = prepara(2);
    esegui(&mut s, "cambiofoglio", json!({"foglio": 1}));
    esegui(&mut s, "filo", json!({"points": [[100.0, 250.0], [300.0, 250.0]]}));
    esegui(&mut s, "rimandicollegamenti", json!({"at": [300.0, 250.0], "segnale": "A1"}));
    esegui(&mut s, "cambiofoglio", json!({"foglio": 2}));
    esegui(&mut s, "filo", json!({"points": [[50.0, 250.0], [200.0, 250.0]]}));
    esegui(&mut s, "rimandicollegamenti", json!({"at": [50.0, 250.0], "segnale": "B1", "tipo": "arrivo"}));
    let r = blocchi(&mut s, "_SIGN_*", "2");
    assert_eq!(r.len(), 1, "{r:?}");
    let h = r[0].0.clone();
    assert_eq!(esegui(&mut s, "xs.riconosci", json!({"handle": h}))["tipo"], "rimando");
    let v = esegui(&mut s, "editarimando", json!({"handle": h, "segnale": "A1"}));
    assert_eq!(v["collegati"], 2, "{v}");
    let r = blocchi(&mut s, "_SIGN_*", "2");
    assert_eq!(r[0].2["SEGNALE"], "A1");
    assert_eq!(r[0].2["REG"], "01/7A");
    // con una descrizione diversa dalla partenza non si collegano più
    let v = esegui(&mut s, "editarimando", json!({"handle": h, "descrizione": "Consenso"}));
    assert_eq!(v["senzaCoppia"], 2, "{v}");
    assert_eq!(blocchi(&mut s, "_SIGN_*", "2")[0].2["#LINEA"], "Consenso");
}

#[test]
fn connettori_come_morsetti() {
    let _uno_alla_volta = crate::uno_alla_volta();
    let mut s = prepara(1);
    for x in [100.0, 150.0, 200.0] {
        esegui(&mut s, "filo", json!({"points": [[x, 250.0], [x, 100.0]]}));
    }
    let r = esegui(&mut s, "connettori", json!({"connettore": "-XC1", "sesso": "M", "da": [90.0, 180.0], "a": [210.0, 180.0]}));
    assert_eq!(r["morsetti"].as_array().unwrap().len(), 3, "{r}");
    let c = blocchi(&mut s, "CE_CONNETTORE", "1");
    assert_eq!(c.iter().map(|x| x.2["NUMM"].as_str().unwrap()).collect::<Vec<_>>(), ["1", "2", "3"]);
    assert!(c.iter().all(|x| x.2["SESSO"] == "M" && x.2["MORSETTIERA"] == "-XC1"));
    // il pin si edita come un morsetto, e i fili numerati arrivano sui pin
    assert_eq!(esegui(&mut s, "xs.riconosci", json!({"handle": c[0].0}))["tipo"], "morsetto");
    esegui(&mut s, "numerafili", json!({}));
    let righe = elettrico::morsetti::righe(&mut s, "-XC1").unwrap();
    assert_eq!(righe.len(), 3);
    assert_eq!(righe[0][1], "01.01");
}
