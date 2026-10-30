//! Gestore topografico: stato dei componenti nel lay-out, posa di un ingombro (anche
//! interattiva, il prossimo da posare), aggancio alla guida DIN, componente rinominato o
//! cancellato nello schema, riapertura del file.

use cadcraft_engine::geom::Vec2;
use cadcraft_engine::{Input, Session};
use serde_json::{Value, json};

fn esegui(s: &mut Session, id: &str, p: Value) -> Value {
    match s.execute(id, &p) {
        Ok(v) => v,
        Err(e) => panic!("{id}: {e}"),
    }
}

/// (sigla, stato) degli elementi del topografico del quadro.
fn stati(s: &mut Session) -> Vec<(String, String)> {
    let r = esegui(s, "topografico", json!({"quadro": "+QG"}));
    r["elementi"].as_array().unwrap().iter().map(|e| (e["sigla"].as_str().unwrap().to_string(), e["stato"].as_str().unwrap().to_string())).collect()
}

fn stato_di(s: &mut Session, sigla: &str) -> String {
    stati(s).into_iter().find(|(x, _)| x == sigla).map(|(_, st)| st).unwrap_or_else(|| "assente".into())
}

#[test]
fn componenti_nel_lay_out() {
    let _uno_alla_volta = crate::uno_alla_volta();
    crate::registra();
    let mut s = Session::empty();
    s.new_drawing(true);
    esegui(&mut s, "nuovomultifoglio", json!({"fogli": 1}));
    esegui(&mut s, "cambiofoglio", json!({"foglio": 1}));
    let qf = esegui(&mut s, "simbolo.qf3", json!({"at": [60.0, 200.0], "quadro": "+QG"}))["nome"].as_str().unwrap().to_string();
    let km = esegui(&mut s, "simbolo.km", json!({"at": [150.0, 200.0], "quadro": "+QG", "tipo": "LC1D09"}));
    let km_nome = km["nome"].as_str().unwrap().to_string();
    let ka = esegui(&mut s, "simbolo.ka", json!({"at": [250.0, 200.0], "quadro": "+QG"}));
    let ka_nome = ka["nome"].as_str().unwrap().to_string();
    // il motore non va nel quadro
    esegui(&mut s, "simbolo.m3", json!({"at": [60.0, 100.0], "quadro": "+QG"}));

    let l = esegui(&mut s, "layoutquadro", json!({"quadro": "+QG"}));
    let foglio = l["foglio"].as_u64().unwrap();
    let st = stati(&mut s);
    assert_eq!(st.len(), 3, "{st:?}");
    assert!(st.iter().all(|(_, x)| x == "posato"), "{st:?}");

    // tolto un ingombro, il componente è da posare
    let r = esegui(&mut s, "topografico", json!({"quadro": "+QG"}));
    let e = r["elementi"].as_array().unwrap().iter().find(|e| e["sigla"] == ka_nome.as_str()).unwrap().clone();
    let (x, y) = (e["x"].as_f64().unwrap(), e["y"].as_f64().unwrap());
    esegui(&mut s, "erase", json!({"handles": [e["handle"]]}));
    assert_eq!(stato_di(&mut s, &ka_nome), "da posare");
    // posato di nuovo poco sopra la guida: l'aggancio lo rimette sulla guida
    let p = esegui(&mut s, "topografico.posa", json!({"quadro": "+QG", "sigla": ka_nome, "at": [x, y + 2.0]}));
    assert!((p["at"][1].as_f64().unwrap() - y).abs() < 1e-6, "{p}");
    assert_eq!(stato_di(&mut s, &ka_nome), "posato");
    assert!(s.execute("topografico.posa", &json!({"quadro": "+QG", "sigla": ka_nome, "at": [x, y]})).is_err(), "già posato");
    assert!(s.execute("topografico.posa", &json!({"quadro": "+QG", "sigla": "-M99", "at": [x, y]})).is_err(), "non è del quadro");

    // la posa interattiva dal foglio del lay-out: Invio prende il prossimo da posare
    let e = esegui(&mut s, "topografico", json!({"quadro": "+QG"}))["elementi"].as_array().unwrap().iter().find(|e| e["sigla"] == qf.as_str()).unwrap().clone();
    esegui(&mut s, "erase", json!({"handles": [e["handle"]]}));
    esegui(&mut s, "cambiofoglio", json!({"foglio": foglio}));
    s.start("topografico.posa").unwrap();
    s.input(Input::Enter).unwrap();
    s.input(Input::Point(Vec2::new(e["x"].as_f64().unwrap(), e["y"].as_f64().unwrap()))).unwrap();
    s.input(Input::Cancel).unwrap();
    assert_eq!(stato_di(&mut s, &qf), "posato");

    // Layout Quadro di nuovo: niente secondo foglio, solo il componente nuovo in coda sulla guida
    let fogli_prima = elettrico::fogli::elenco(&mut s).unwrap().len();
    esegui(&mut s, "cambiofoglio", json!({"foglio": 1}));
    let nuovo = esegui(&mut s, "simbolo.ka", json!({"at": [300.0, 200.0], "quadro": "+QG"}))["nome"].as_str().unwrap().to_string();
    let r = esegui(&mut s, "layoutquadro", json!({"quadro": "+QG"}));
    assert_eq!((r["foglio"].as_u64(), r["posati"].as_u64(), r["esistente"].as_bool()), (Some(foglio), Some(1), Some(true)), "{r}");
    assert_eq!(elettrico::fogli::elenco(&mut s).unwrap().len(), fogli_prima);
    assert_eq!(stato_di(&mut s, &nuovo), "posato");
    assert_eq!(stati(&mut s).len(), 4);
    // rifatto da capo: stesso foglio, tutti posati, nessun doppione
    let r = esegui(&mut s, "layoutquadro", json!({"quadro": "+QG", "rifai": true}));
    assert_eq!(r["foglio"].as_u64(), Some(foglio));
    assert_eq!(elettrico::fogli::elenco(&mut s).unwrap().len(), fogli_prima);
    let st = stati(&mut s);
    assert!(st.len() == 4 && st.iter().all(|(_, x)| x == "posato"), "{st:?}");
    let h_nuovo = elettrico::siglatura::componenti(&mut s).unwrap().into_iter().find(|c| c.nome == nuovo).unwrap().blocco.handle;
    esegui(&mut s, "cancellasimbolo", json!({"handles": [h_nuovo]}));
    esegui(&mut s, "topografico.aggiorna", json!({"togliOrfani": true}));

    // rinominato nello schema: l'ingombro lo segue, con il codice
    esegui(&mut s, "editasimbolo", json!({"handle": km["handle"], "nome": "-KM99", "tipo": "LC1D12"}));
    let r = esegui(&mut s, "topografico.aggiorna", json!({}));
    assert_eq!((r["aggiornati"].as_u64(), r["orfani"].as_u64()), (Some(1), Some(0)), "{r}");
    assert_eq!(stato_di(&mut s, "-KM99"), "posato");
    assert_eq!(stato_di(&mut s, &km_nome), "assente");

    // salvato e riaperto: gli ingombri ritrovano i loro componenti
    let dir = std::env::temp_dir().join(format!("cadeo_topografico_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let f = dir.join("lay.dxf");
    esegui(&mut s, "saveas", json!({"path": f.to_string_lossy()}));
    let mut s2 = Session::empty();
    esegui(&mut s2, "open", json!({"path": f.to_string_lossy()}));
    let st = stati(&mut s2);
    assert!(st.iter().all(|(_, x)| x == "posato") && st.len() == 3, "{st:?}");
    esegui(&mut s2, "editasimbolo", json!({"handle": ka["handle"], "nome": "-KA77"}));
    assert_eq!(esegui(&mut s2, "topografico.aggiorna", json!({}))["aggiornati"], 1, "anche dopo la riapertura");
    let _ = std::fs::remove_dir_all(&dir);

    // cancellato nello schema: l'ingombro resta, segnalato; poi si toglie
    esegui(&mut s, "cancellasimbolo", json!({"handles": [ka["handle"]]}));
    assert_eq!(esegui(&mut s, "topografico.aggiorna", json!({}))["orfani"], 1);
    assert_eq!(stato_di(&mut s, &ka_nome), "non nello schema");
    esegui(&mut s, "topografico.aggiorna", json!({"togliOrfani": true}));
    assert_eq!(stato_di(&mut s, &ka_nome), "assente");
    // e Aggiorna Liste e Tabelle tiene il lay-out allineato
    esegui(&mut s, "aggiornaliste", json!({}));
}
