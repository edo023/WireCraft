//! Specchietto dei contatti e numerazione fili, senza interfaccia.

use cadcraft_engine::Session;
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

/// Attributi di tutti i blocchi `filtro`: (nome blocco, handle, layer, x, y, attributi).
/// (nome, handle, layer, x, y, attributi) dei blocchi inseriti.
type Blocco = (String, String, String, f64, f64, std::collections::HashMap<String, String>);

fn blocchi(s: &mut Session, filtro: &str) -> Vec<Blocco> {
    let r = esegui(s, "dataextraction", json!({"blocks": filtro, "combine": false}));
    let col: Vec<String> = r["columns"].as_array().unwrap().iter().map(|c| c.as_str().unwrap().to_string()).collect();
    r["rows"]
        .as_array()
        .unwrap()
        .iter()
        .map(|riga| {
            let mut a = std::collections::HashMap::new();
            for (k, c) in col.iter().enumerate() {
                if let Some(v) = riga[k].as_str() {
                    a.insert(c.clone(), v.to_string());
                }
            }
            (a["Name"].clone(), a["Handle"].clone(), a["Layer"].clone(), riga[col.iter().position(|c| c == "Position X").unwrap()].as_f64().unwrap(), riga[col.iter().position(|c| c == "Position Y").unwrap()].as_f64().unwrap(), a)
        })
        .collect()
}

#[test]
fn lo_specchietto_assegna_i_contatti_alle_righe() {
    let _uno_alla_volta = crate::uno_alla_volta();
    let mut s = prepara(2);
    let km = esegui(&mut s, "simbolo.km", json!({"at": [100.0, 200.0]}));
    let rif = blocchi(&mut s, "CE_RIF_*");
    assert_eq!(rif.len(), 1, "specchietto sotto la bobina");
    assert_eq!((rif[0].3, rif[0].4), elettrico::specchietto::posizione((100.0, 200.0)));
    assert_eq!(rif[0].5["MADRE"], "-KM01.1");

    esegui(&mut s, "cambiofoglio", json!({"foglio": 2}));
    let pot = esegui(&mut s, "simbolo.pot3", json!({"at": [50.0, 200.0], "madre": "-KM01.1"}));
    let na = esegui(&mut s, "simbolo.na", json!({"at": [150.0, 200.0], "madre": "-KM01.1"}));
    let nc = esegui(&mut s, "simbolo.nc", json!({"at": [200.0, 200.0], "madre": "-KM01.1"}));
    let r = blocchi(&mut s, "CE_RIF_*").remove(0).5;
    for k in 0..3 {
        assert_eq!(r[&format!("REG{k}")], "02/0B", "polo {k}");
    }
    assert_eq!(r["REG3"], "02/3B", "NA 13-14");
    assert_eq!(r["REG4"], "02/4B", "NC 21-22");
    // i contatti prendono i morsetti dalla riga occupata
    let cont = blocchi(&mut s, "CE_NA,CE_NC,CE_POT3");
    let di = |h: &Value| cont.iter().find(|b| b.1 == h.as_str().unwrap()).unwrap().5.clone();
    assert_eq!((di(&na["handle"])["PINA1"].as_str(), di(&na["handle"])["PINB1"].as_str()), ("13", "14"));
    assert_eq!((di(&nc["handle"])["PINA1"].as_str(), di(&nc["handle"])["PINB1"].as_str()), ("21", "22"));
    assert_eq!(di(&pot["handle"])["PINB3"], "6");

    // un secondo NA non trova posto: avviso
    let troppo = esegui(&mut s, "simbolo.na", json!({"at": [250.0, 200.0], "madre": "-KM01.1"}));
    assert!(troppo["message"].as_str().unwrap().contains("superata"), "{troppo}");
    let x = esegui(&mut s, "crossreference", json!({}));
    assert_eq!(x["contattiSenzaPosto"].as_array().unwrap().len(), 1);

    // spostando la bobina lo specchietto la segue
    esegui(&mut s, "move", json!({"handles": [km["handle"]], "delta": [40.0, 0.0]}));
    esegui(&mut s, "crossreference", json!({}));
    let rif = blocchi(&mut s, "CE_RIF_*");
    assert_eq!((rif[0].3, rif[0].4), elettrico::specchietto::posizione((140.0, 200.0)));
    // e i contatti indicano la nuova posizione
    let cont = blocchi(&mut s, "CE_NA");
    assert!(cont.iter().all(|b| b.5["REG"] == "01/3B"), "{:?}", cont.iter().map(|b| b.5["REG"].clone()).collect::<Vec<_>>());
}

#[test]
fn morsetti_su_linea_e_morsettiera() {
    let _uno_alla_volta = crate::uno_alla_volta();
    let mut s = prepara(1);
    for x in [50.0, 60.0, 70.0] {
        esegui(&mut s, "filo", json!({"points": [[x, 250.0], [x, 100.0]]}));
    }
    let r = esegui(&mut s, "morsetti", json!({"morsettiera": "-XG", "da": [40.0, 150.0], "a": [80.0, 150.0]}));
    let numeri: Vec<&str> = r["morsetti"].as_array().unwrap().iter().map(|m| m["numero"].as_str().unwrap()).collect();
    assert_eq!(numeri, ["1", "2", "3"]);
    // i successivi continuano la numerazione
    let r = esegui(&mut s, "morsetti", json!({"morsettiera": "-XG", "at": [70.0, 120.0]}));
    assert_eq!(r["morsetti"][0]["numero"], "4");
    // la numerazione dei fili porta il numero del filo sui morsetti
    esegui(&mut s, "numerafili", json!({}));
    let m = esegui(&mut s, "morsettiera", json!({"morsettiera": "-XG", "at": [250.0, 200.0]}));
    let righe = m["righe"].as_array().unwrap();
    assert_eq!(righe.len(), 4);
    assert_eq!(righe[0], json!(["1", "01.01", "01/0C", ""]));
    assert_eq!(righe[3][1], "01.03", "il morsetto 4 sta sul terzo filo");
    assert!(s.execute("morsettiera", &json!({"morsettiera": "-XZ"})).is_err());
}

#[test]
fn distinta_materiali() {
    let _uno_alla_volta = crate::uno_alla_volta();
    let mut s = prepara(1);
    esegui(&mut s, "simbolo.km", json!({"at": [100.0, 200.0], "costruttore": "Eaton", "tipo": "199220", "descrizione": "Contattore 3kW"}));
    esegui(&mut s, "simbolo.km", json!({"at": [150.0, 200.0]}));
    esegui(&mut s, "materiale", json!({"sigla": "-KM01.2", "costruttore": "Eaton", "codice": "199220", "descrizione": "Contattore 3kW"}));
    esegui(&mut s, "materiale", json!({"sigla": "-KM01.2", "costruttore": "Eaton", "codice": "281200", "descrizione": "Spegniarco", "aggiungi": true}));
    let per_sigla = esegui(&mut s, "distintamateriali", json!({}));
    assert_eq!(per_sigla["righe"].as_array().unwrap().len(), 3);
    let per_tipo = esegui(&mut s, "distintamateriali", json!({"ordine": "tipo", "at": [250.0, 250.0]}));
    assert_eq!(per_tipo["righe"][0], json!(["Eaton", "199220", "Contattore 3kW", "2", "-KM01.1, -KM01.2"]));
    let file = std::env::temp_dir().join("distinta_prova.csv");
    esegui(&mut s, "distintamateriali", json!({"ordine": "tipo", "file": file.to_string_lossy()}));
    let testo = std::fs::read_to_string(&file).unwrap();
    assert!(testo.starts_with("Costruttore;Codice;Descrizione;Q.tà;Sigle\n"), "{testo}");
}

#[test]
fn numerazione_fili_per_reti() {
    let _uno_alla_volta = crate::uno_alla_volta();
    let mut s = prepara(1);
    esegui(&mut s, "alimentazioni", json!({"nomi": "L1 L2 L3", "da": [35.0, 257.5], "a": [397.5, 257.5]}));
    esegui(&mut s, "vv", json!({"at": [100.0, 257.5]}));
    esegui(&mut s, "simbolo.ka", json!({"at": [100.0, 200.0]}));
    esegui(&mut s, "filo", json!({"points": [[300.0, 150.0], [350.0, 150.0]]}));
    let r = esegui(&mut s, "numerafili", json!({}));
    let nomi: Vec<&str> = r["fili"]["1"].as_array().unwrap().iter().map(|v| v.as_str().unwrap()).collect();
    assert_eq!(nomi, ["L1", "L2", "L3", "01.01", "01.02"]);
    // L2 e L3 sono solo linee orizzontali con le frecce dei rimandi: il nome è già lì
    let etichette = blocchi(&mut s, "CE_NUMFILO");
    assert_eq!(etichette.len(), 3);
    // il numero della calata sotto la bobina: a 1 mm a sinistra, a metà del tratto
    let calata = etichette.iter().find(|b| b.5["N"] == "01.01").unwrap();
    assert_eq!((calata.3, calata.4), (99.0, 117.5));
    // rinumerare non duplica le etichette
    esegui(&mut s, "numerafili", json!({}));
    assert_eq!(blocchi(&mut s, "CE_NUMFILO").len(), 3);
}
