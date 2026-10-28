//! Schema d'impianto: quadri dai componenti e dai cavi, percorso dei cavi dalla definizione o
//! dalle vene posate, sul foglio e in tabella.

use cadcraft_engine::Session;
use serde_json::{Value, json};

fn esegui(s: &mut Session, id: &str, p: Value) -> Value {
    match s.execute(id, &p) {
        Ok(v) => v,
        Err(e) => panic!("{id}: {e}"),
    }
}

#[test]
fn quadri_e_cavi_dell_impianto() {
    let _uno_alla_volta = crate::uno_alla_volta();
    crate::registra();
    let mut s = Session::empty();
    s.new_drawing(true);
    esegui(&mut s, "nuovomultifoglio", json!({"fogli": 1}));
    esegui(&mut s, "cambiofoglio", json!({"foglio": 1}));
    esegui(&mut s, "filo", json!({"points": [[60.0, 250.0], [60.0, 80.0]]}));
    esegui(&mut s, "simbolo.qf1", json!({"at": [60.0, 220.0], "quadro": "+QG"}));
    esegui(&mut s, "simbolo.km", json!({"at": [150.0, 220.0], "quadro": "+QG"}));
    esegui(&mut s, "simbolo.hl", json!({"at": [60.0, 120.0], "quadro": "+PL"}));
    esegui(&mut s, "box.quadro", json!({"valore": "PL", "da": [40.0, 100.0], "a": [90.0, 140.0], "descrizione": "Pulpito"}));
    // un cavo con il percorso scritto, uno che lo prende dalle vene posate
    esegui(&mut s, "cavo.definisci", json!({"formazione": "4G1,5", "daQuadro": "+QG", "aQuadro": "+BM", "lunghezza": "15", "funzione": "Motore"}));
    let w2 = esegui(&mut s, "cavo.definisci", json!({"formazione": "3G1"}));
    let w2 = w2["sigla"].as_str().or_else(|| w2["cavo"].as_str()).unwrap_or("-W02").to_string();
    esegui(&mut s, "cavo.posa", json!({"cavo": w2, "da": [50.0, 160.0], "a": [70.0, 160.0]}));

    let (nodi, tratte) = elettrico::impianto::dati(&mut s).unwrap();
    let nomi: Vec<&str> = nodi.iter().map(|n| n.nome.as_str()).collect();
    assert_eq!(nomi, ["+QG", "+PL", "+BM"], "{nodi:?}");
    assert_eq!((nodi[0].componenti, nodi[2].componenti), (2, 0));
    assert_eq!(nodi[1].descrizione, "Pulpito");
    assert_eq!(tratte.len(), 2, "{tratte:?}");
    assert!(tratte.iter().any(|t| t.da == "+QG" && t.a == "+BM" && t.cavi[0].lunghezza == "15"));
    assert!(tratte.iter().any(|t| [t.da.as_str(), t.a.as_str()].contains(&"+PL") && t.cavi[0].sigla == w2), "il percorso dalle vene: {tratte:?}");

    let u = esegui(&mut s, "uscita", json!({"tipo": "impianto"}));
    assert_eq!(u["fogli"].as_array().unwrap().len(), 1);
    let (t, _, _) = elettrico::uscite::tabella(&mut s, "impianto", &json!({})).unwrap();
    assert_eq!(t.righe.len(), 2);
    esegui(&mut s, "aggiornaliste", json!({}));
}
