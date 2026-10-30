//! Collegamenti, conduttori e uscite sui fogli (lista fogli, distinta, morsettiere, cablaggio,
//! legende) con "Aggiorna liste e tabelle".

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

/// Un circuito di comando: L1 → pulsante → morsetto → bobina → N.
fn circuito(s: &mut Session) {
    esegui(s, "titolofoglio", json!({"foglio": 1, "titolo": "Comandi", "titolo1": "Controls"}));
    esegui(s, "alimentazioni", json!({"nomi": "L1 N", "da": [30.0, 260.0], "a": [400.0, 260.0]}));
    esegui(s, "filo", json!({"points": [[100.0, 260.0], [100.0, 60.0], [200.0, 60.0], [200.0, 250.0]]}));
    esegui(s, "simbolo.sbno", json!({"at": [100.0, 220.0], "funzione1": "Marcia", "funzione2": "Start"}));
    esegui(s, "simbolo.km", json!({"at": [100.0, 140.0], "funzione1": "Contattore pompa", "costruttore": "Eaton", "tipo": "199220", "descrizione": "Contattore 3kW", "quadro": "+QG"}));
    esegui(s, "morsetti", json!({"morsettiera": "-XG", "at": [100.0, 180.0]}));
    esegui(s, "numerafili", json!({}));
}

#[test]
fn reti_conduttori_e_cablaggio() {
    let _uno_alla_volta = crate::uno_alla_volta();
    let mut s = prepara(2);
    circuito(&mut s);
    // la rete tra pulsante e bobina passa dal morsetto: capi dall'alto in basso
    let r = esegui(&mut s, "reti", json!({}));
    let rete = r["reti"].as_array().unwrap().iter().find(|r| r["capi"].as_array().unwrap().iter().any(|c| c == "-XG:1")).unwrap().clone();
    assert_eq!(rete["capi"], json!(["-SB01.1:14", "-XG:1", "-KM01.1:A1"]));
    assert_eq!(rete["conduttore"]["sezione"], "1x1.5", "conduttore in uso");
    // specializzato: indicazione sul filo e nella rete
    esegui(&mut s, "conduttore", json!({"at": [100.0, 170.0], "sezione": "1x0.75", "colore": "BU", "codice": "H05V-K 1x0.75 BU"}));
    let r = esegui(&mut s, "reti", json!({}));
    let rete = r["reti"].as_array().unwrap().iter().find(|r| r["capi"].as_array().unwrap().iter().any(|c| c == "-XG:1")).unwrap().clone();
    assert_eq!((rete["conduttore"]["colore"].as_str(), rete["specializzato"].as_bool()), (Some("BU"), Some(true)));
    // tabella di cablaggio: pulsante → morsetto → bobina
    let c = esegui(&mut s, "tabellacablaggio", json!({}));
    let righe = c["righe"].as_array().unwrap();
    let da_a: Vec<(String, String)> = righe.iter().map(|r| (format!("{}:{}", r[1].as_str().unwrap(), r[2].as_str().unwrap()), format!("{}:{}", r[6].as_str().unwrap(), r[7].as_str().unwrap()))).collect();
    assert!(da_a.contains(&("-SB01.1:14".into(), "-XG:1".into())), "{da_a:?}");
    assert!(da_a.contains(&("-XG:1".into(), "-KM01.1:A1".into())), "{da_a:?}");
    let riga = righe.iter().find(|r| r[1] == "-XG").unwrap();
    assert_eq!((riga[4].as_str(), riga[5].as_str()), (Some("1x0.75"), Some("BU")));
    esegui(&mut s, "conduttore", json!({"at": [100.0, 170.0], "togli": true}));
    let r = esegui(&mut s, "reti", json!({}));
    assert!(r["reti"].as_array().unwrap().iter().all(|r| r["specializzato"] == false));
}

#[test]
fn uscite_sui_fogli_e_aggiornamento() {
    let _uno_alla_volta = crate::uno_alla_volta();
    let mut s = prepara(2);
    circuito(&mut s);
    let r = esegui(&mut s, "uscita", json!({"tipo": "distinta"}));
    assert_eq!(r["fogli"], json!([3]), "dopo l'ultimo foglio");
    let fogli = elettrico::fogli::elenco(&mut s).unwrap();
    assert_eq!(fogli.iter().find(|f| f.numero == 3).unwrap().titolo, "Distinta materiali");
    for tipo in ["indice", "morsettiere", "cablaggio", "funzioni", "simboli"] {
        let r = esegui(&mut s, "uscita", json!({"tipo": tipo}));
        assert_eq!(r["fogli"].as_array().unwrap().len(), 1, "{tipo}: {r}");
    }
    assert_eq!(elettrico::fogli::elenco(&mut s).unwrap().len(), 8);
    // la lista fogli conosce i titoli
    let t = elettrico::uscite::tabella(&mut s, "indice", &json!({})).unwrap().0;
    assert!(t.righe.iter().any(|r| r[1] == elettrico::uscite::Cella::T("Comandi / Controls".into())));
    // un nuovo componente: aggiorna liste e tabelle lo porta nella distinta, sugli stessi fogli
    esegui(&mut s, "cambiofoglio", json!({"foglio": 2}));
    esegui(&mut s, "simbolo.qf3", json!({"at": [100.0, 200.0], "costruttore": "ABB", "tipo": "S203-C16", "descrizione": "Magnetotermico 3P C16"}));
    let a = esegui(&mut s, "aggiornaliste", json!({}));
    assert_eq!(a["uscite"].as_array().unwrap().len(), 6, "{a}");
    assert_eq!(elettrico::fogli::elenco(&mut s).unwrap().len(), 8, "stessi fogli");
    let d = elettrico::uscite::tabella(&mut s, "distinta", &json!({})).unwrap().0;
    assert!(d.righe.iter().any(|r| r[1] == elettrico::uscite::Cella::T("S203-C16".into())));
    // ogni uscita resta una sola pagina sul suo foglio
    let pagine = esegui(&mut s, "dataextraction", json!({"blocks": "CE_USCITA_*", "combine": false}));
    assert_eq!(pagine["rows"].as_array().unwrap().len(), 6);
}

#[test]
fn la_stessa_lista_due_volte_si_ridisegna_al_suo_posto() {
    let _uno_alla_volta = crate::uno_alla_volta();
    let mut s = prepara(1);
    circuito(&mut s);
    let a = esegui(&mut s, "uscita", json!({"tipo": "distinta"}));
    let b = esegui(&mut s, "uscita", json!({"tipo": "distinta"}));
    assert_eq!(a["fogli"], b["fogli"], "stessi fogli, nessuna copia");
    let pagine = esegui(&mut s, "dataextraction", json!({"blocks": "CE_USCITA_*", "combine": false}));
    assert_eq!(pagine["rows"].as_array().map(Vec::len), Some(a["fogli"].as_array().unwrap().len()));
    // un'altra morsettiera è un'altra uscita: si aggiunge
    let m1 = esegui(&mut s, "uscita", json!({"tipo": "morsettiere", "morsettiera": "-XG"}));
    let m2 = esegui(&mut s, "uscita", json!({"tipo": "morsettiere"}));
    assert_ne!(m1["fogli"], m2["fogli"]);
    // e nel foglio indicato si disegna dove si chiede
    let n = elettrico::fogli::elenco(&mut s).unwrap().last().unwrap().numero + 1;
    let c = esegui(&mut s, "uscita", json!({"tipo": "distinta", "foglio": n}));
    assert_eq!(c["fogli"][0].as_u64(), Some(u64::from(n)));
}
