//! Loop diagram: dal bit del PLC al dispositivo in campo, per morsetti, cavi e rimandi.

use cadcraft_engine::Session;
use serde_json::{Value, json};

fn esegui(s: &mut Session, id: &str, p: Value) -> Value {
    match s.execute(id, &p) {
        Ok(v) => v,
        Err(e) => panic!("{id}: {e}"),
    }
}

#[test]
fn dal_bit_al_dispositivo() {
    let _uno_alla_volta = crate::uno_alla_volta();
    crate::registra();
    let mut s = Session::empty();
    s.new_drawing(true);
    esegui(&mut s, "nuovomultifoglio", json!({"fogli": 2}));
    esegui(&mut s, "cambiofoglio", json!({"foglio": 1}));
    esegui(&mut s, "alimentazioni", json!({"nomi": "+24V 0V", "da": [20.0, 255.0], "a": [400.0, 255.0]}));
    // primo ingresso: sensore in campo, morsetto e cavo sullo stesso foglio
    esegui(&mut s, "filo", json!({"points": [[60.0, 255.0], [60.0, 140.0]]}));
    let sq1 = esegui(&mut s, "simbolo.sqno", json!({"at": [60.0, 230.0], "quadro": "+BM", "funzione1": "Finecorsa avanti"}))["nome"].as_str().unwrap().to_string();
    esegui(&mut s, "morsetti", json!({"morsettiera": "-X2", "da": [50.0, 170.0], "a": [70.0, 170.0]}));
    let cavo = esegui(&mut s, "cavo.definisci", json!({"formazione": "3G1", "funzione": "Finecorsa"}));
    let cavo = cavo["sigla"].as_str().unwrap_or("-W01").to_string();
    esegui(&mut s, "cavo.posa", json!({"cavo": cavo, "da": [50.0, 190.0], "a": [70.0, 190.0]}));
    // secondo ingresso: il filo va su un altro foglio con un rimando
    esegui(&mut s, "filo", json!({"points": [[80.0, 140.0], [80.0, 220.0]]}));
    esegui(&mut s, "rimandicollegamenti", json!({"at": [80.0, 220.0], "segnale": "S2"}));
    // terzo ingresso: sempre a +24V, senza dispositivo
    esegui(&mut s, "filo", json!({"points": [[100.0, 255.0], [100.0, 140.0]]}));
    esegui(&mut s, "plc.scheda", json!({"at": [60.0, 140.0], "bit": 4, "indirizzo": "E0.0", "commenti": [["Finecorsa avanti", "Forward limit"], ["Pressostato", "Pressure switch"]]}));
    esegui(&mut s, "cambiofoglio", json!({"foglio": 2}));
    esegui(&mut s, "filo", json!({"points": [[50.0, 250.0], [50.0, 150.0]]}));
    esegui(&mut s, "rimandicollegamenti", json!({"at": [50.0, 250.0], "segnale": "S2", "tipo": "arrivo"}));
    let sq2 = esegui(&mut s, "simbolo.sqno", json!({"at": [50.0, 210.0], "funzione1": "Pressostato"}))["nome"].as_str().unwrap().to_string();
    esegui(&mut s, "numerafili", json!({"tutti": true}));

    let r = esegui(&mut s, "loopdiagram", json!({}));
    let v = r["loop"].as_array().unwrap();
    assert_eq!(v.len(), 4, "{r}");
    let primo = &v[0];
    assert_eq!(primo["indirizzo"], "E0.0");
    assert_eq!(primo["dispositivo"]["sigla"], sq1.as_str(), "{primo}");
    assert_eq!(primo["dispositivo"]["descrizione"], "Finecorsa avanti");
    assert_eq!(primo["morsetti"], json!(["-X2:1"]), "{primo}");
    assert!(primo["cavi"].as_array().unwrap().iter().any(|c| c.as_str().unwrap().starts_with(&cavo)), "{primo}");
    assert_eq!(primo["altri"], json!([{"pin": "13", "rete": "+24V"}]), "l'altro capo del sensore va al +24V: {primo}");
    assert!(!primo["fili"].as_array().unwrap().is_empty());
    // il secondo arriva al pressostato sull'altro foglio
    let secondo = &v[1];
    assert_eq!(secondo["dispositivo"]["sigla"], sq2.as_str(), "{secondo}");
    assert_eq!(secondo["morsetti"], json!([]));
    // il terzo bit va dritto al +24V: non prende i sensori appesi alla linea
    assert!(v[2]["dispositivo"].is_null(), "{}", v[2]);
    assert_eq!(v[2]["potenziale"], "+24V");
    // il quarto non ha niente collegato
    assert!(v[3]["dispositivo"].is_null() && v[3]["potenziale"].is_null());
    assert!(r["message"].as_str().unwrap().contains("1 senza dispositivo"), "{r}");

    // sui fogli: disegno, e anche in tabella; poi si aggiornano
    let f = esegui(&mut s, "uscita", json!({"tipo": "loop"}));
    assert_eq!(f["fogli"], json!([3]), "{f}");
    let fogli = elettrico::fogli::elenco(&mut s).unwrap();
    assert_eq!(fogli[2].titolo, "Loop diagram");
    let t = esegui(&mut s, "uscita", json!({"tipo": "loop", "tabella": true}));
    assert_eq!(t["fogli"], json!([4]), "{t}");
    let righe = elettrico::loopdiagram::righe(&elettrico::loopdiagram::loops(&mut s).unwrap());
    assert_eq!(righe[0][4], "-X2:1");
    let a = esegui(&mut s, "aggiornaliste", json!({}));
    assert!(a["uscite"].as_array().unwrap().iter().filter(|u| u["tipo"] == "loop").count() == 2, "{a}");
}
