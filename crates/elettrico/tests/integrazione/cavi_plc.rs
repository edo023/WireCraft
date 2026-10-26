//! Cavi (definizione, posa delle vene, conduttori) e schede PLC (bit, indirizzi, elenco I/O).

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

#[test]
fn cavi_definiti_posati_e_in_tabella() {
    let _uno_alla_volta = crate::uno_alla_volta();
    let mut s = prepara(1);
    // tre fili verso un motore, il terzo è il PE
    esegui(&mut s, "alimentazioni", json!({"nomi": "U V PE", "da": [30.0, 250.0], "a": [60.0, 250.0], "fine": "nessuno"}));
    for x in [100.0, 110.0, 120.0] {
        esegui(&mut s, "filo", json!({"points": [[x, 230.0], [x, 100.0]]}));
    }
    esegui(&mut s, "morsetti", json!({"morsettiera": "-XM", "da": [90.0, 200.0], "a": [130.0, 200.0]}));
    esegui(&mut s, "simbolo.m3", json!({"at": [100.0, 100.0]}));
    esegui(&mut s, "numerafili", json!({}));

    let c = esegui(&mut s, "cavo.definisci", json!({"formazione": "4G1,5", "codice": "FROR 4G1,5", "funzione": "Motore pompa", "daQuadro": "+QG", "aQuadro": "+BM", "lunghezza": "12"}));
    assert_eq!(c["sigla"], "-W01");
    assert_eq!(c["vene"], json!(["BN", "BK", "GY", "GNYE"]));
    let r = esegui(&mut s, "cavo.posa", json!({"cavo": "-W01", "da": [90.0, 150.0], "a": [130.0, 150.0]}));
    assert_eq!(r["vene"], json!(["BN", "BK", "GY"]));
    let elenco = esegui(&mut s, "cavi", json!({}));
    assert_eq!(elenco["cavi"][0]["usate"].as_array().unwrap().len(), 3);
    // le vene con i capi: dal morsetto al motore
    let t = esegui(&mut s, "cavo.conduttori", json!({}));
    let righe = t["righe"].as_array().unwrap();
    assert_eq!(righe.len(), 3);
    assert_eq!(righe[0][3], "-XM:1", "{righe:?}");
    assert_eq!(righe[0][4], "-M01.1:U");
    // senza vene libere: errore; tolto un segnalino si libera la vena
    esegui(&mut s, "cavo.posa", json!({"cavo": "-W01", "at": [100.0, 120.0]}));
    assert!(s.execute("cavo.posa", &json!({"cavo": "-W01", "at": [110.0, 120.0]})).is_err());
    esegui(&mut s, "cavo.togli", json!({"at": [100.0, 120.0]}));
    let elenco = esegui(&mut s, "cavi", json!({}));
    assert_eq!(elenco["cavi"][0]["usate"].as_array().unwrap().len(), 2, "tolti i due segnalini sulla rete del primo filo");
    // in distinta e nelle liste
    let r = esegui(&mut s, "uscita", json!({"tipo": "cavi"}));
    assert_eq!(r["fogli"], json!([2]));
    let l = elettrico::uscite::tabella(&mut s, "cavi", &json!({})).unwrap().0;
    assert_eq!(l.righe[0][0], elettrico::uscite::Cella::T("-W01".into()));
    esegui(&mut s, "cavo.elimina", json!({"sigla": "-W01"}));
    assert!(esegui(&mut s, "cavi", json!({}))["cavi"].as_array().unwrap().is_empty());
}

#[test]
fn schede_plc_bit_e_indirizzi() {
    let _uno_alla_volta = crate::uno_alla_volta();
    let mut s = prepara(2);
    esegui(&mut s, "cambiofoglio", json!({"foglio": 2}));
    let r = esegui(
        &mut s,
        "plc.scheda",
        json!({"at": [60.0, 200.0], "bit": 8, "indirizzo": "E0.0", "costruttore": "Siemens", "codice": "6ES7521-1BH10-0AA0", "descrizione": "DI 16x24VDC", "commenti": [["Pulsante marcia", "Start pushbutton"]]}),
    );
    assert_eq!(r["sigla"], "-A02.1");
    assert_eq!(r["bit"].as_array().unwrap().len(), 8);
    let io = esegui(&mut s, "plc.elenco", json!({}));
    let righe = io["righe"].as_array().unwrap();
    assert_eq!(righe.len(), 8);
    assert_eq!((righe[0][0].as_str(), righe[0][2].as_str(), righe[0][5].as_str()), (Some("E0.0"), Some("Pulsante marcia"), Some("1")));
    assert_eq!(righe[7][0], "E0.7");
    // la scheda è una madre: va in distinta; i bit sanno dove sta
    let d = elettrico::distinta::materiali(&mut s).unwrap();
    assert!(d.iter().any(|m| m.sigla == "-A02.1" && m.codice == "6ES7521-1BH10-0AA0"));
    let bit = elettrico::siglatura::componenti(&mut s).unwrap().into_iter().find(|c| c.pres == "F_PLC").unwrap();
    assert_eq!(bit.blocco.attributo("REG"), "02/1C");
    // uscite da A1.0, poi rinumerate da A4.0
    esegui(&mut s, "plc.scheda", json!({"at": [60.0, 120.0], "bit": 4, "tipo": "uscite", "indirizzo": "A1.0"}));
    let r = esegui(&mut s, "plc.numerabit", json!({"sigla": "-A02.2", "indirizzo": "A4.0"}));
    assert_eq!(r["indirizzi"], json!(["A4.0", "A4.1", "A4.2", "A4.3"]));
    esegui(&mut s, "plc.commento", json!({"indirizzo": "A4.1", "commento": "Pompa", "commento2": "Pump"}));
    // l'elenco esportato e reimportato dopo un cambio di commento
    let dir = std::env::temp_dir().join(format!("cadeo_plc_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let f = dir.join("io.csv");
    std::fs::write(&f, "Indirizzo;Commento;Comment\nE0.3;Finecorsa avanti;Forward limit\n").unwrap();
    let r = esegui(&mut s, "plc.importa", json!({"file": f.to_string_lossy()}));
    assert_eq!(r["bit"], 1);
    let io = esegui(&mut s, "plc.elenco", json!({}));
    let e03 = io["righe"].as_array().unwrap().iter().find(|r| r[0] == "E0.3").unwrap().clone();
    assert_eq!((e03[2].as_str(), e03[3].as_str()), (Some("Finecorsa avanti"), Some("Forward limit")));
    let r = esegui(&mut s, "uscita", json!({"tipo": "io"}));
    assert_eq!(r["fogli"], json!([3]));
    let _ = std::fs::remove_dir_all(&dir);
}
