//! Simboli, siglatura e cross reference figlio → madre, senza interfaccia.

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

/// (nome blocco, NOME, REG) dei componenti, per handle.
fn attributi(s: &mut Session, handle: &str) -> (String, String, String) {
    let r = esegui(s, "dataextraction", json!({"blocks": "CE_*", "combine": false}));
    let col: Vec<&str> = r["columns"].as_array().unwrap().iter().map(|c| c.as_str().unwrap()).collect();
    let i = |n: &str| col.iter().position(|c| *c == n);
    let riga = r["rows"].as_array().unwrap().iter().find(|x| x[i("Handle").unwrap()] == handle).unwrap();
    let v = |n: &str| i(n).and_then(|k| riga[k].as_str()).unwrap_or("").to_string();
    (v("Name"), v("NOME"), v("REG"))
}

fn fili(s: &Session, layer: &str) -> Vec<((f64, f64), (f64, f64))> {
    let mut v: Vec<_> = elettrico::fili::fili(s, layer).unwrap().into_iter().map(|f| (f.a, f.b)).collect();
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v
}

#[test]
fn sigle_automatiche_per_foglio_e_cross_del_contatto() {
    let _uno_alla_volta = crate::uno_alla_volta();
    let mut s = prepara(2);
    let k1 = esegui(&mut s, "simbolo.km", json!({"at": [100.0, 200.0]}));
    assert_eq!(k1["nome"], "-KM01.1");
    assert_eq!(esegui(&mut s, "simbolo.km", json!({"at": [200.0, 200.0]}))["nome"], "-KM01.2");
    assert_eq!(esegui(&mut s, "simbolo.qf3", json!({"at": [300.0, 200.0]}))["nome"], "-QF01.1");
    esegui(&mut s, "cambiofoglio", json!({"foglio": 2}));
    assert_eq!(esegui(&mut s, "simbolo.km", json!({"at": [100.0, 200.0]}))["nome"], "-KM02.1");

    // il contatto porta la sigla della madre e dove si trova
    let na = esegui(&mut s, "simbolo.na", json!({"at": [50.0, 200.0], "madre": "-KM01.1"}));
    let na = na["handle"].as_str().unwrap().to_string();
    assert_eq!(attributi(&mut s, &na), ("CE_NA".into(), "-KM01.1".into(), "01/2B".into()));
    // senza madre il figlio non si inserisce
    assert!(s.execute("simbolo.nc", &json!({"at": [60.0, 200.0]})).is_err());
}

#[test]
fn il_simbolo_interrompe_i_fili_sotto_ai_poli() {
    let _uno_alla_volta = crate::uno_alla_volta();
    let mut s = prepara(1);
    esegui(&mut s, "filo", json!({"points": [[300.0, 250.0], [300.0, 100.0]]}));
    esegui(&mut s, "simbolo.ka", json!({"at": [300.0, 200.0]}));
    assert_eq!(fili(&s, "1"), vec![((300.0, 190.0), (300.0, 100.0)), ((300.0, 250.0), (300.0, 200.0))]);

    // interruttore a 3 poli su tre calate
    for x in [50.0, 60.0, 70.0] {
        esegui(&mut s, "filo", json!({"points": [[x, 250.0], [x, 100.0]]}));
    }
    esegui(&mut s, "simbolo.qf3", json!({"at": [50.0, 220.0]}));
    let f = fili(&s, "1");
    for x in [50.0, 60.0, 70.0] {
        assert!(f.contains(&((x, 250.0), (x, 220.0))), "polo {x}: sopra");
        assert!(f.contains(&((x, 210.0), (x, 100.0))), "polo {x}: sotto");
    }
    // un solo annulla rimette il filo intero
    esegui(&mut s, "undo", json!({"count": 1}));
    assert!(fili(&s, "1").contains(&((50.0, 250.0), (50.0, 100.0))));
}

#[test]
fn sigle_componenti_rinumera_e_aggiorna_i_contatti() {
    let _uno_alla_volta = crate::uno_alla_volta();
    let mut s = prepara(2);
    esegui(&mut s, "simbolo.km", json!({"at": [100.0, 200.0]})); // -KM01.1
    esegui(&mut s, "simbolo.km", json!({"at": [200.0, 200.0]})); // -KM01.2
    let terzo = esegui(&mut s, "simbolo.km", json!({"at": [50.0, 200.0]})); // -KM01.3, ma è il primo da sinistra
    assert_eq!(terzo["nome"], "-KM01.3");
    esegui(&mut s, "cambiofoglio", json!({"foglio": 2}));
    let na = esegui(&mut s, "simbolo.na", json!({"at": [50.0, 200.0], "madre": "-KM01.1"}));
    let na = na["handle"].as_str().unwrap().to_string();

    let r = esegui(&mut s, "siglecomponenti", json!({}));
    assert_eq!(r["cambi"].as_array().unwrap().len(), 3);
    let terzo = terzo["handle"].as_str().unwrap().to_string();
    assert_eq!(attributi(&mut s, &terzo).1, "-KM01.1", "il più a sinistra diventa .1");
    // il contatto segue la sua bobina (era -KM01.1 a x=100, ora -KM01.2) e dove si trova
    assert_eq!(attributi(&mut s, &na).1, "-KM01.2");
    assert_eq!(attributi(&mut s, &na).2, "01/2B");
    // una seconda volta non cambia niente
    assert_eq!(esegui(&mut s, "siglecomponenti", json!({}))["cambi"].as_array().unwrap().len(), 0);
}

#[test]
fn finestra_di_inserimento_e_modifica() {
    let _uno_alla_volta = crate::uno_alla_volta();
    let mut s = prepara(1);
    // la finestra prepara sigla, quadro e materiali; la posa interattiva li usa
    elettrico::simboli::prepara("simbolo.km", json!({"nome": "-KM1", "quadro": "+QG", "costruttore": "Eaton;Eaton", "tipo": "199220;281200"}));
    s.start("simbolo.km").unwrap();
    s.input(cadcraft_engine::Input::Point(cadcraft_engine::geom::Vec2::new(100.0, 200.0))).unwrap();
    s.input(cadcraft_engine::Input::Point(cadcraft_engine::geom::Vec2::new(150.0, 200.0))).unwrap();
    s.input(cadcraft_engine::Input::Enter).unwrap();
    let m: Vec<_> = elettrico::siglatura::componenti(&mut s).unwrap().into_iter().filter(|c| c.madre()).collect();
    let nomi: Vec<&str> = m.iter().map(|c| c.nome.as_str()).collect();
    assert!(nomi.contains(&"-KM1") && nomi.contains(&"-KM01.1"), "{nomi:?}: la sigla data vale solo per la prima posa");
    let km1 = m.iter().find(|c| c.nome == "-KM1").unwrap();
    assert_eq!(km1.blocco.attributo("QUADRO"), "+QG");
    assert_eq!(km1.blocco.attributo("TIPO"), "199220;281200");
    // sigle doppie rifiutate
    assert!(s.execute("simbolo.km", &json!({"at": [200.0, 200.0], "nome": "-KM1"})).is_err());

    // un contatto della -KM1, poi la -KM1 cambia sigla: contatto e specchietto la seguono
    let na = esegui(&mut s, "simbolo.na", json!({"at": [250.0, 200.0], "madre": "-KM1"}));
    let h = km1.blocco.handle.clone();
    esegui(&mut s, "editasimbolo", json!({"handle": h, "nome": "-KM10", "funzione1": "Pompa"}));
    let na = na["handle"].as_str().unwrap().to_string();
    assert_eq!(attributi(&mut s, &na).1, "-KM10");
    let rif = elettrico::siglatura::componenti(&mut s).unwrap().into_iter().find(|c| c.pres == "R" && c.blocco.attributo("MADRE") == "-KM10");
    assert!(rif.is_some_and(|r| r.blocco.attributo("REG3") == "01/5B"), "lo specchietto della -KM10 segna il contatto");
    assert!(s.execute("editasimbolo", &json!({"handle": h, "nome": "-KM01.1"})).is_err(), "sigla già usata");
}

#[test]
fn archivio_materiali() {
    let _uno_alla_volta = crate::uno_alla_volta();
    let dir = std::env::temp_dir().join(format!("cadeo_archivio_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let file = dir.join("materiali.csv");
    assert_eq!(elettrico::archivio::imposta(&file).unwrap(), 0);
    let listino = dir.join("listino.csv");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(&listino, "Marca;Codice articolo;Descrizione\nEaton;199220;DILM7-10 contattore 3kW\nEaton;281200;Spegniarco RC\nSchneider Electric;A9F94210;Magnetotermico 2P C10\n").unwrap();
    let mut s = prepara(1);
    let r = esegui(&mut s, "materiali.importa", json!({"file": listino.to_string_lossy()}));
    assert_eq!((r["letti"].as_u64(), r["nuovi"].as_u64()), (Some(3), Some(3)));
    let trovati = esegui(&mut s, "materiali.cerca", json!({"testo": "eaton contattore"}));
    assert_eq!(trovati["articoli"].as_array().unwrap().len(), 1);
    esegui(&mut s, "materiali.aggiungi", json!({"costruttore": "Finder", "codice": "40.52", "descrizione": "Relè 2 scambi", "preferito": true}));
    // salvato su disco e riletto
    assert_eq!(elettrico::archivio::imposta(&file).unwrap(), 4);
    let primo = elettrico::archivio::cerca("", false, 10).unwrap();
    assert_eq!(primo[0].codice, "40.52", "i preferiti per primi");
    let _ = std::fs::remove_dir_all(&dir);
}
