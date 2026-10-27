//! Salto tra contatto e madre e tra rimandi corrispondenti.

use cadcraft_engine::Session;
use serde_json::{Value, json};

fn esegui(s: &mut Session, id: &str, p: Value) -> Value {
    match s.execute(id, &p) {
        Ok(v) => v,
        Err(e) => panic!("{id}: {e}"),
    }
}

fn prepara(fogli: u32) -> Session {
    crate::registra();
    let mut s = Session::empty();
    s.new_drawing(true);
    esegui(&mut s, "nuovomultifoglio", json!({"fogli": fogli}));
    s
}

fn corrente(s: &mut Session) -> u32 {
    elettrico::fogli::corrente(s).unwrap().unwrap().numero
}

#[test]
fn dal_contatto_alla_bobina_e_ritorno() {
    let _uno_alla_volta = crate::uno_alla_volta();
    let mut s = prepara(3);
    esegui(&mut s, "cambiofoglio", json!({"foglio": 1}));
    let km = esegui(&mut s, "simbolo.km", json!({"at": [100.0, 200.0]}));
    let km = km["handle"].as_str().unwrap().to_string();
    esegui(&mut s, "cambiofoglio", json!({"foglio": 3}));
    let nc = esegui(&mut s, "simbolo.nc", json!({"at": [200.0, 150.0], "madre": "-KM01.1"}))["handle"].as_str().unwrap().to_string();
    esegui(&mut s, "cambiofoglio", json!({"foglio": 2}));
    let na = esegui(&mut s, "simbolo.na", json!({"at": [150.0, 200.0], "madre": "-KM01.1"}))["handle"].as_str().unwrap().to_string();

    // dal contatto: un solo arrivo, la bobina sul foglio 1, selezionata
    let r = esegui(&mut s, "salta", json!({"handle": na}));
    assert_eq!(r["arrivi"].as_array().unwrap().len(), 1, "{r}");
    assert_eq!(r["scelto"]["handle"], km.as_str());
    assert_eq!(r["scelto"]["riferimento"], "01/2B", "{r}");
    assert_eq!(corrente(&mut s), 1);
    assert_eq!(s.selection().iter().map(|h| h.hex()).collect::<Vec<_>>(), [km.as_str()]);

    // dalla bobina (la selezione): i contatti in ordine di foglio, il primo per primo
    let r = esegui(&mut s, "salta", json!({}));
    let arrivi: Vec<&str> = r["arrivi"].as_array().unwrap().iter().map(|a| a["handle"].as_str().unwrap()).collect();
    assert_eq!(arrivi, [na.as_str(), nc.as_str()], "{r}");
    assert_eq!(corrente(&mut s), 2);
    assert!(r["scelto"]["descrizione"].as_str().unwrap().contains("13-14"), "{r}");
    // il secondo contatto, senza muovere la vista
    let r = esegui(&mut s, "salta", json!({"handle": km, "indice": 1, "vai": false}));
    assert_eq!(r["scelto"]["handle"], nc.as_str());
    assert_eq!(corrente(&mut s), 2);
}

#[test]
fn da_un_rimando_al_suo_corrispondente() {
    let _uno_alla_volta = crate::uno_alla_volta();
    let mut s = prepara(3);
    esegui(&mut s, "cambiofoglio", json!({"foglio": 1}));
    let p = esegui(&mut s, "rimandicollegamenti", json!({"at": [400.0, 250.0], "segnale": "X1"}));
    let p = p["handle"].as_str().map(str::to_string).unwrap_or_else(|| elettrico::fili::rimandi(&mut s).unwrap()[0].handle.clone());
    esegui(&mut s, "cambiofoglio", json!({"foglio": 3}));
    esegui(&mut s, "rimandicollegamenti", json!({"at": [30.0, 250.0], "segnale": "X1", "tipo": "arrivo"}));
    // un rimando senza coppia
    esegui(&mut s, "rimandicollegamenti", json!({"at": [30.0, 200.0], "segnale": "Y9", "tipo": "arrivo"}));
    let tutti = elettrico::fili::rimandi(&mut s).unwrap();
    let arrivo = tutti.iter().find(|r| r.segnale == "X1" && !r.partenza).unwrap().handle.clone();
    let solo = tutti.iter().find(|r| r.segnale == "Y9").unwrap().handle.clone();

    esegui(&mut s, "cambiofoglio", json!({"foglio": 1}));
    let r = esegui(&mut s, "salta", json!({"handle": p}));
    assert_eq!(r["scelto"]["handle"], arrivo.as_str(), "{r}");
    assert_eq!(corrente(&mut s), 3);
    let r = esegui(&mut s, "salta", json!({"handle": arrivo}));
    assert_eq!(r["scelto"]["handle"], p.as_str());
    assert_eq!(corrente(&mut s), 1);

    let r = esegui(&mut s, "salta", json!({"handle": solo}));
    assert_eq!(r["arrivi"], json!([]));
    assert!(r["scelto"].is_null());
    // un oggetto qualunque non è un punto di partenza
    let linea = esegui(&mut s, "line", json!({"points": [[10.0, 10.0], [20.0, 10.0]]}));
    let h = linea["handles"][0].as_str().or_else(|| linea["handle"].as_str()).unwrap().to_string();
    assert!(s.execute("salta", &json!({"handle": h})).is_err());
}
