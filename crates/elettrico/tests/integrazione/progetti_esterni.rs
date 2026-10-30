//! Schemi disegnati con altri programmi: un simbolo che non è della libreria, con i pin solo
//! negli attributi (`PINA1`, `PINB1` accanto agli estremi dei fili), e un morsetto che
//! interrompe il filo (i fili gli arrivano 7,5 mm sopra e sotto). Devono entrare nella tabella
//! di cablaggio e nel disegno della morsettiera come quelli della libreria.

use cadcraft_engine::Session;
use serde_json::{Value, json};

fn esegui(s: &mut Session, id: &str, p: Value) -> Value {
    match s.execute(id, &p) {
        Ok(v) => v,
        Err(e) => panic!("{id}: {e}"),
    }
}

fn handle(v: &Value) -> String {
    v["handle"].as_str().or_else(|| v["handles"][0].as_str()).unwrap().to_string()
}

/// Un blocco con gli attributi dati (tag, punto, valore predefinito, nascosto) e un segno.
fn blocco(s: &mut Session, nome: &str, attributi: &[(&str, (f64, f64), &str, bool)]) {
    let mut h = vec![handle(&esegui(s, "circle", json!({"center": [0.0, -15.0], "radius": 3.0})))];
    for (tag, at, def, nascosto) in attributi {
        h.push(handle(&esegui(s, "attdef", json!({"tag": tag, "prompt": tag, "default": def, "at": [at.0, at.1], "height": 1.8, "invisible": nascosto}))));
    }
    esegui(s, "-block", json!({"name": nome, "base": [0.0, 0.0], "handles": h, "keep": "delete", "description": ""}));
}

#[test]
fn simboli_e_morsetti_di_altri_programmi() {
    let _uno_alla_volta = crate::uno_alla_volta();
    crate::registra();
    let mut s = Session::empty();
    s.new_drawing(true);
    esegui(&mut s, "nuovomultifoglio", json!({"fogli": 1}));
    esegui(&mut s, "cambiofoglio", json!({"foglio": 1}));
    // un interruttore alto 30 mm: il pin 1 nel punto d'inserimento, il 2 trenta sotto; i numeri
    // dei pin un po' a sinistra, come li scrivono quei programmi
    blocco(&mut s, "Q_ALTRO", &[("PRES", (0.0, 0.0), "M", true), ("NOME", (-8.0, -15.0), "", false), ("PINA1", (-1.5, -2.25), "1", false), ("PINB1", (-1.5, -27.5), "2", false)]);
    blocco(&mut s, "_MORS-ALTRO", &[("MORSETTIERA", (3.0, -1.0), "", false), ("NUMM", (-3.0, 1.0), "", false)]);
    esegui(&mut s, "insert", json!({"name": "Q_ALTRO", "at": [100.0, 200.0], "attribs": {"NOME": "-Q1", "PRES": "M"}}));
    esegui(&mut s, "insert", json!({"name": "_MORS-ALTRO", "at": [100.0, 132.5], "attribs": {"MORSETTIERA": "-X9", "NUMM": "1"}}));
    esegui(&mut s, "filo", json!({"points": [[100.0, 250.0], [100.0, 200.0]]}));
    esegui(&mut s, "filo", json!({"points": [[100.0, 170.0], [100.0, 140.0]]}));
    esegui(&mut s, "filo", json!({"points": [[100.0, 125.0], [100.0, 60.0]]}));
    let hl = esegui(&mut s, "simbolo.hl", json!({"at": [100.0, 100.0]}))["nome"].as_str().unwrap().to_string();

    let c = elettrico::morsettiere::collegati(&mut s, "-X9").unwrap();
    assert_eq!(c.len(), 1);
    assert_eq!((c[0].interno.as_str(), c[0].esterno.as_str()), ("-Q1:2", format!("{hl}:X1").as_str()), "{c:?}");

    // nella tabella di cablaggio: dal pin 2 dell'interruttore al morsetto, dal morsetto alla lampada
    let reti = elettrico::collegamenti::reti(&mut s, &[]).unwrap();
    let fili: Vec<(String, String)> = elettrico::collegamenti::cablaggio(&reti).iter().map(|c| (c.da.testo(), c.a.testo())).collect();
    assert!(fili.contains(&("-Q1:2".to_string(), "-X9:1".to_string())), "{fili:?}");
    assert!(fili.contains(&("-X9:1".to_string(), format!("{hl}:X1"))), "{fili:?}");
}
