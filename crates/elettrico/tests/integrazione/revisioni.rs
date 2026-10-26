//! Revisioni dei fogli e descrizioni delle colonne.

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

fn madri(s: &mut Session) -> Vec<String> {
    let mut v: Vec<String> = elettrico::siglatura::componenti(s).unwrap().into_iter().filter(|c| c.madre()).map(|c| c.nome).collect();
    v.sort();
    v
}

#[test]
fn revisioni_archiviate_e_annullate() {
    let _uno_alla_volta = crate::uno_alla_volta();
    let mut s = prepara(2);
    esegui(&mut s, "simbolo.km", json!({"at": [100.0, 200.0]}));
    // prima emissione su tutti i fogli
    let r = esegui(&mut s, "revisione.crea", json!({"modifica": "Prima emissione", "data": "01/09/2026", "firma": "AP"}));
    assert_eq!(r["numero"], "00");
    // l'archivio non conta: una sola bobina
    assert_eq!(madri(&mut s), ["-KM01.1"]);
    // modifica sul foglio 1, revisione 01 solo lì
    esegui(&mut s, "simbolo.km", json!({"at": [200.0, 200.0]}));
    let r = esegui(&mut s, "revisione.crea", json!({"modifica": "Aggiunto contattore", "fogli": [1]}));
    assert_eq!(r["numero"], "01");
    let e = esegui(&mut s, "revisione.elenco", json!({}));
    let v = e["revisioni"].as_array().unwrap();
    assert_eq!(v.len(), 3, "00 su due fogli, 01 sul primo: {e}");
    assert!(v.iter().all(|r| r["archiviata"] == true));
    assert_eq!(e["prossima"], "02");
    // la legenda delle revisioni
    let t = elettrico::uscite::tabella(&mut s, "revisioni", &json!({})).unwrap().0;
    assert_eq!(t.righe.len(), 3);
    // si vede il foglio com'era, poi si torna
    esegui(&mut s, "revisione.vedi", json!({"foglio": 1, "numero": "00"}));
    esegui(&mut s, "revisione.vedi", json!({"foglio": 1}));
    // dopo la 01 si aggiunge un terzo contattore, poi si annulla la 01: il foglio torna com'era
    // quando si è creata la 01 (con due contattori)
    esegui(&mut s, "simbolo.km", json!({"at": [300.0, 200.0]}));
    assert_eq!(madri(&mut s).len(), 3);
    esegui(&mut s, "revisione.annulla", json!({"numero": "01"}));
    assert_eq!(madri(&mut s), ["-KM01.1", "-KM01.2"]);
    let e = esegui(&mut s, "revisione.elenco", json!({}));
    assert_eq!(e["revisioni"].as_array().unwrap().len(), 2);
    esegui(&mut s, "revisione.reset", json!({}));
    assert!(esegui(&mut s, "revisione.elenco", json!({}))["revisioni"].as_array().unwrap().is_empty());
}

#[test]
fn descrizioni_delle_colonne() {
    let _uno_alla_volta = crate::uno_alla_volta();
    let mut s = prepara(1);
    esegui(&mut s, "descrizionecolonna", json!({"colonna": 0, "testo": "ALIMENTAZIONE|400 VAC 50 HZ", "testo2": "POWER SUPPLY"}));
    esegui(&mut s, "descrizionecolonna", json!({"at": [150.0, 200.0], "colonne": 2, "testo": "COMANDO POMPA DI RICIRCOLO ACQUA"}));
    let d = elettrico::fogli::descrizioni_colonne(&mut s, 1).unwrap();
    assert_eq!(d, vec![(0, "ALIMENTAZIONE 400 VAC 50 HZ".to_string(), "POWER SUPPLY".to_string()), (3, "COMANDO POMPA DI RICIRCOLO ACQUA".to_string(), String::new())]);
    // riscritta e poi tolta
    esegui(&mut s, "descrizionecolonna", json!({"colonna": 0, "testo": "QUADRO"}));
    assert_eq!(elettrico::fogli::descrizioni_colonne(&mut s, 1).unwrap()[0].1, "QUADRO");
    esegui(&mut s, "descrizionecolonna", json!({"colonna": 0, "testo": ""}));
    assert_eq!(elettrico::fogli::descrizioni_colonne(&mut s, 1).unwrap().len(), 1);
    assert_eq!(elettrico::fogli::due_righe("UNA DESCRIZIONE MOLTO LUNGA DA DIVIDERE", 20), ("UNA DESCRIZIONE MOLTO".into(), "LUNGA DA DIVIDERE".into()));
}
