//! Traduttore: dizionario su disco e schema reso bilingue (funzioni, titoli, colonne).

use cadcraft_engine::Session;
use serde_json::{Value, json};

fn esegui(s: &mut Session, id: &str, p: Value) -> Value {
    match s.execute(id, &p) {
        Ok(v) => v,
        Err(e) => panic!("{id}: {e}"),
    }
}

#[test]
fn traduce_funzioni_titoli_e_colonne() {
    let _uno_alla_volta = crate::uno_alla_volta();
    crate::registra();
    let dir = std::env::temp_dir().join(format!("wirecraft_dizionario_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let file = dir.join("dizionario.csv");
    // senza file si parte dal dizionario iniziale, scritto su disco
    assert!(elettrico::traduttore::imposta(&file).unwrap() > 50);
    assert!(file.exists());

    let mut s = Session::empty();
    s.new_drawing(true);
    esegui(&mut s, "nuovomultifoglio", json!({"fogli": 2}));
    esegui(&mut s, "titolofoglio", json!({"foglio": 1, "titolo": "SCHEMA DI POTENZA"}));
    esegui(&mut s, "descrizionecolonna", json!({"foglio": 1, "colonna": 2, "testo": "VENTILATORE ARMADIO"}));
    esegui(&mut s, "cambiofoglio", json!({"foglio": 1}));
    esegui(&mut s, "simbolo.km", json!({"at": [100.0, 200.0], "funzione1": "Pompa idraulica"}));
    esegui(&mut s, "simbolo.km", json!({"at": [200.0, 200.0], "funzione1": "SPINTORE PEZZI"}));

    let r = esegui(&mut s, "traduci", json!({"a": "EN"}));
    assert_eq!(r["tradotti"], 3, "{r}");
    assert_eq!(r["mancanti"], json!(["SPINTORE PEZZI"]));
    let c = elettrico::siglatura::componenti(&mut s).unwrap();
    let f2 = |nome: &str| c.iter().find(|x| x.blocco.attributo("FUNZIONE1") == nome).unwrap().blocco.attributo("FUNZIONE2").to_string();
    assert_eq!(f2("Pompa idraulica"), "HYDRAULIC PUMP");
    let fogli = elettrico::fogli::elenco(&mut s).unwrap();
    assert_eq!(fogli[0].titolo1, "POWER DIAGRAM");
    assert_eq!(elettrico::fogli::descrizioni_colonne(&mut s, 1).unwrap(), vec![(2, "VENTILATORE ARMADIO".to_string(), "PANEL FAN".to_string())]);

    // la frase mancante si aggiunge al dizionario (salvato) e si ritraduce solo quella
    esegui(&mut s, "traduttore.aggiungi", json!({"frase": "Spintore pezzi", "traduzione": "Part pusher", "a": "EN"}));
    let r = esegui(&mut s, "traduci", json!({"a": "EN"}));
    assert_eq!((r["tradotti"].as_u64(), r["mancanti"].as_array().map(Vec::len)), (Some(1), Some(0)), "{r}");
    let c = elettrico::siglatura::componenti(&mut s).unwrap();
    assert!(c.iter().any(|x| x.blocco.attributo("FUNZIONE2") == "PART PUSHER"), "in maiuscolo come l'originale");
    assert!(std::fs::read_to_string(&file).unwrap().contains("Spintore pezzi;Part pusher"));

    // in tedesco, sovrascrivendo
    let r = esegui(&mut s, "traduci", json!({"a": "de", "sovrascrivi": true}));
    assert_eq!(r["mancanti"], json!(["SPINTORE PEZZI"]));
    assert_eq!(elettrico::fogli::elenco(&mut s).unwrap()[0].titolo1, "LEISTUNGSSCHALTPLAN");

    // un dizionario importato si unisce a quello in uso
    let altro = dir.join("altro.csv");
    std::fs::write(&altro, "IT;DE\nSPINTORE PEZZI;TEILESCHIEBER\n").unwrap();
    esegui(&mut s, "traduttore.importa", json!({"file": altro.to_string_lossy()}));
    let r = esegui(&mut s, "traduci", json!({"a": "DE"}));
    assert_eq!(r["mancanti"], json!([]), "{r}");
    let _ = std::fs::remove_dir_all(&dir);
}
