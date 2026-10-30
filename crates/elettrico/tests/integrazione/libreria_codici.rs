//! Dal codice al disegno: la libreria di base nell'archivio, il simbolo posato dal codice con
//! costruttore, codice e descrizione, la larghezza che arriva al componente e al lay-out.

use cadcraft_engine::geom::Vec2;
use cadcraft_engine::{Input, Session};
use serde_json::{Value, json};

fn esegui(s: &mut Session, id: &str, p: Value) -> Value {
    match s.execute(id, &p) {
        Ok(v) => v,
        Err(e) => panic!("{id}: {e}"),
    }
}

fn componente(s: &mut Session, nome: &str) -> elettrico::siglatura::Componente {
    elettrico::siglatura::componenti(s).unwrap().into_iter().find(|c| c.madre() && c.nome == nome).unwrap()
}

#[test]
fn componenti_posati_dal_codice() {
    let _uno_alla_volta = crate::uno_alla_volta();
    crate::registra();
    let dir = std::env::temp_dir().join(format!("cadeo_libreria_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(elettrico::archivio::imposta(&dir.join("materiali.csv")).unwrap(), 0);
    let mut s = Session::empty();
    s.new_drawing(true);
    esegui(&mut s, "nuovomultifoglio", json!({"fogli": 1}));
    esegui(&mut s, "cambiofoglio", json!({"foglio": 1}));

    // la libreria di base: articoli con simbolo e larghezza; una seconda volta niente di nuovo
    let r = esegui(&mut s, "materiali.base", json!({}));
    assert!(r["nuovi"].as_u64().unwrap() > 60, "{r}");
    assert_eq!(esegui(&mut s, "materiali.base", json!({}))["nuovi"], 0);
    let a = elettrico::archivio::trova("", "ct-9-24dc").unwrap();
    assert_eq!((a.simbolo.as_str(), a.larghezza), ("km", 45.0));

    // dal codice: il contattore con i suoi dati e la sua larghezza
    let km = esegui(&mut s, "materiale.inserisci", json!({"codice": "CT-9-24DC", "at": [100.0, 200.0], "quadro": "+QG"}));
    let km = componente(&mut s, km["nome"].as_str().unwrap());
    assert_eq!((km.blocco.nome.as_str(), km.blocco.attributo("TIPO"), km.blocco.attributo("COSTRUTTORE"), km.blocco.attributo("LARGHEZZA")), ("CE_BOBINA_KM", "CT-9-24DC", "Generico", "45"));
    assert!(km.blocco.attributo("DESCRIZIONE").starts_with("Contattore 9 A"));
    assert_eq!(km.blocco.attributo("CODICE"), "CE_RIF_KM", "il codice dell'articolo non tocca lo specchietto");

    // interattivo: il codice, poi un punto per componente
    s.start("materiale.inserisci").unwrap();
    s.input(Input::Text("Generico|PL-VERDE-NO".into())).unwrap();
    s.input(Input::Point(Vec2::new(200.0, 200.0))).unwrap();
    s.input(Input::Point(Vec2::new(220.0, 200.0))).unwrap();
    s.input(Input::Enter).unwrap();
    let pulsanti = elettrico::siglatura::componenti(&mut s).unwrap().into_iter().filter(|c| c.blocco.attributo("TIPO") == "PL-VERDE-NO").count();
    assert_eq!(pulsanti, 2);

    // un articolo senza simbolo non si posa dal codice
    esegui(&mut s, "materiali.aggiungi", json!({"costruttore": "Altro", "codice": "X-1", "descrizione": "Senza simbolo"}));
    assert!(s.execute("materiale.inserisci", &json!({"codice": "X-1", "at": [10.0, 10.0]})).is_err());
    assert!(s.execute("materiali.aggiungi", &json!({"codice": "X-2", "simbolo": "boh"})).is_err(), "simbolo sconosciuto");

    // un materiale associato o cambiato porta la sua larghezza
    let qf = esegui(&mut s, "simbolo.qf3", json!({"at": [100.0, 120.0], "quadro": "+QG"}))["nome"].as_str().unwrap().to_string();
    esegui(&mut s, "materiale", json!({"sigla": qf, "codice": "MT-3P-C16", "costruttore": "Generico"}));
    assert_eq!(componente(&mut s, &qf).blocco.attributo("LARGHEZZA"), "54");
    let h = componente(&mut s, &qf).blocco.handle;
    esegui(&mut s, "editasimbolo", json!({"handle": h, "costruttore": "Generico", "tipo": "SEZ-3P-100"}));
    assert_eq!(componente(&mut s, &qf).blocco.attributo("LARGHEZZA"), "72");

    // il lay-out usa le larghezze vere
    let ing = elettrico::layout::ingombri(&mut s, "+QG").unwrap();
    let i = ing.iter().find(|i| i.sigla == km.nome).unwrap();
    assert_eq!((i.larghezza, i.stimata), (45.0, false));
    assert_eq!(ing.iter().find(|i| i.sigla == qf).map(|i| i.larghezza), Some(72.0));
    let _ = std::fs::remove_dir_all(&dir);
}
