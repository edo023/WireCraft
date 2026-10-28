//! Editor delle morsettiere: rinumerazione secondo lo schema, a mano, dai fili e con primo
//! numero e passo; morsetti a due livelli che restano insieme; nomi non numerici che restano;
//! rinomina e unione di morsettiere.

use cadcraft_engine::Session;
use serde_json::{Value, json};

fn esegui(s: &mut Session, id: &str, p: Value) -> Value {
    match s.execute(id, &p) {
        Ok(v) => v,
        Err(e) => panic!("{id}: {e}"),
    }
}

/// (ascissa, numero, livello) dei morsetti di una morsettiera, da sinistra a destra.
fn per_posizione(s: &mut Session, nome: &str) -> Vec<(i64, String, u32)> {
    let mut v: Vec<(i64, String, u32)> = elettrico::morsettiere::in_ordine(s, nome)
        .unwrap()
        .iter()
        .map(|m| (m.blocco.pos.0.round() as i64, m.numero.clone(), m.blocco.attributo("LIVELLO").parse().unwrap_or(1)))
        .collect();
    v.sort();
    v
}

fn coppie(v: &[(i64, &str)]) -> Vec<(i64, String)> {
    v.iter().map(|(x, n)| (*x, (*n).to_string())).collect()
}

fn senza_livello(v: Vec<(i64, String, u32)>) -> Vec<(i64, String)> {
    v.into_iter().map(|(x, n, _)| (x, n)).collect()
}

#[test]
fn rinumera_e_rinomina_le_morsettiere() {
    let _uno_alla_volta = crate::uno_alla_volta();
    crate::registra();
    let mut s = Session::empty();
    s.new_drawing(true);
    esegui(&mut s, "nuovomultifoglio", json!({"fogli": 1}));
    esegui(&mut s, "cambiofoglio", json!({"foglio": 1}));
    for x in [60.0, 80.0, 100.0, 120.0, 140.0, 160.0, 200.0] {
        esegui(&mut s, "filo", json!({"points": [[x, 250.0], [x, 100.0]]}));
    }
    // posati da destra verso sinistra: 1 a x=120 … 4 a x=60
    esegui(&mut s, "morsetti", json!({"morsettiera": "-X1", "da": [130.0, 150.0], "a": [50.0, 150.0]}));
    // il PE, e il secondo livello del morsetto 3
    esegui(&mut s, "morsetti", json!({"morsettiera": "-X1", "at": [140.0, 150.0], "numero": "PE"}));
    let m = esegui(&mut s, "morsetti", json!({"morsettiera": "-X1", "at": [160.0, 150.0], "numero": "3"}));
    esegui(&mut s, "editamorsetto", json!({"handle": m["morsetti"][0]["handle"], "livello": "2"}));
    esegui(&mut s, "numerafili", json!({"tutti": true}));
    assert_eq!(senza_livello(per_posizione(&mut s, "-X1")), coppie(&[(60, "4"), (80, "3"), (100, "2"), (120, "1"), (140, "PE"), (160, "3")]));

    // secondo lo schema: da sinistra a destra; i due livelli del 3 restano insieme, il PE resta
    let r = esegui(&mut s, "morsettiera.ordine", json!({"morsettiera": "-X1", "ordine": "schema"}));
    assert_eq!(r["cambiati"].as_array().unwrap().len(), 4, "{r}");
    let v = per_posizione(&mut s, "-X1");
    assert_eq!(senza_livello(v.clone()), coppie(&[(60, "1"), (80, "2"), (100, "3"), (120, "4"), (140, "PE"), (160, "2")]));
    assert_eq!(v[5].2, 2, "il secondo livello resta tale");
    // una seconda volta non cambia niente
    assert_eq!(esegui(&mut s, "morsettiera.ordine", json!({"morsettiera": "-X1", "ordine": "schema"}))["cambiati"].as_array().unwrap().len(), 0);

    // a mano: il 4 per primo, poi il 3; gli altri dopo, nell'ordine di adesso
    esegui(&mut s, "morsettiera.ordine", json!({"morsettiera": "-X1", "numeri": ["4", "3"]}));
    assert_eq!(senza_livello(per_posizione(&mut s, "-X1")), coppie(&[(60, "3"), (80, "4"), (100, "2"), (120, "1"), (140, "PE"), (160, "4")]));
    assert!(s.execute("morsettiera.ordine", &json!({"morsettiera": "-X1", "numeri": ["9"]})).is_err(), "morsetto che non c'è");

    // primo numero e passo
    esegui(&mut s, "morsettiera.ordine", json!({"morsettiera": "-X1", "da": 101, "passo": 2}));
    let numeri: Vec<String> = elettrico::morsettiere::in_ordine(&mut s, "-X1").unwrap().iter().map(|m| m.numero.clone()).collect();
    // 1, 2, 3, 4 (su due livelli), PE
    assert_eq!(numeri, ["101", "103", "105", "107", "107", "PE"]);
    // per fili: ogni morsetto ha il suo filo, i numeri restano 1..4
    esegui(&mut s, "morsettiera.ordine", json!({"morsettiera": "-X1", "ordine": "fili"}));
    let mut n: Vec<String> = elettrico::morsettiere::in_ordine(&mut s, "-X1").unwrap().iter().map(|m| m.numero.clone()).collect();
    n.dedup();
    assert_eq!(n, ["1", "2", "3", "4", "PE"]);

    // rinomina; in una morsettiera che c'è serve unisci, e i numeri continuano
    let r = esegui(&mut s, "morsettiera.rinomina", json!({"morsettiera": "-X1", "nome": "-X2"}));
    assert_eq!(r["morsetti"], 6);
    assert!(elettrico::morsettiere::in_ordine(&mut s, "-X1").unwrap().is_empty());
    esegui(&mut s, "morsetti", json!({"morsettiera": "-X3", "at": [200.0, 150.0]}));
    assert!(s.execute("morsettiera.rinomina", &json!({"morsettiera": "-X3", "nome": "-X2"})).is_err());
    esegui(&mut s, "morsettiera.rinomina", json!({"morsettiera": "-X3", "nome": "-X2", "unisci": true}));
    let v = per_posizione(&mut s, "-X2");
    assert_eq!(v.last().map(|m| (m.0, m.1.as_str())), Some((200, "5")));

    // l'elenco per l'editor
    let e = esegui(&mut s, "morsettiera.elenco", json!({}));
    let m = e["morsettiere"].as_array().unwrap();
    assert_eq!(m.len(), 1);
    assert_eq!((m[0]["morsettiera"].as_str(), m[0]["morsetti"].as_array().map(Vec::len)), (Some("-X2"), Some(7)));
    let primo = &m[0]["morsetti"][0];
    assert_eq!(primo["posizione"].as_str().unwrap(), elettrico::squadratura::riferimento(1, primo["x"].as_f64().unwrap(), primo["y"].as_f64().unwrap()));
    assert!(s.execute("morsettiera.ordine", &json!({"morsettiera": "-X9"})).is_err());
}

#[test]
fn disegno_della_morsettiera_con_i_collegamenti() {
    let _uno_alla_volta = crate::uno_alla_volta();
    crate::registra();
    let mut s = Session::empty();
    s.new_drawing(true);
    esegui(&mut s, "nuovomultifoglio", json!({"fogli": 1}));
    esegui(&mut s, "cambiofoglio", json!({"foglio": 1}));
    // sopra il morsetto un magnetotermico, sotto una lampada; accanto il PE
    esegui(&mut s, "filo", json!({"points": [[60.0, 250.0], [60.0, 80.0]]}));
    esegui(&mut s, "filo", json!({"points": [[100.0, 250.0], [100.0, 80.0]]}));
    let qf = esegui(&mut s, "simbolo.qf1", json!({"at": [60.0, 220.0]}))["nome"].as_str().unwrap().to_string();
    let hl = esegui(&mut s, "simbolo.hl", json!({"at": [60.0, 120.0]}))["nome"].as_str().unwrap().to_string();
    esegui(&mut s, "morsetti", json!({"morsettiera": "-X1", "at": [60.0, 160.0]}));
    esegui(&mut s, "morsetti", json!({"morsettiera": "-X1", "at": [100.0, 160.0], "numero": "PE"}));
    esegui(&mut s, "ponticello", json!({"morsettiera": "-X1", "morsetti": ["1", "PE"]}));
    esegui(&mut s, "numerafili", json!({"tutti": true}));

    let c = elettrico::morsettiere::collegati(&mut s, "-X1").unwrap();
    assert_eq!(c.len(), 2);
    assert_eq!((c[0].interno.as_str(), c[0].esterno.as_str()), (format!("{qf}:2").as_str(), format!("{hl}:X1").as_str()), "{c:?}");
    assert!(!c[0].pe && c[1].pe);
    assert_eq!(c[0].ponte, c[1].ponte);

    // sul foglio e in Excel
    let u = esegui(&mut s, "uscita", json!({"tipo": "disegnomorsettiere", "morsettiera": "-X1"}));
    assert_eq!(u["fogli"].as_array().unwrap().len(), 1);
    let dir = std::env::temp_dir().join(format!("cadeo_dismors_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let r = esegui(&mut s, "excel", json!({"tipo": "disegnomorsettiere", "file": dir.join("m.xlsx").to_string_lossy()}));
    assert_eq!(r["righe"], 2);
    let _ = std::fs::remove_dir_all(&dir);
    esegui(&mut s, "aggiornaliste", json!({}));
}
