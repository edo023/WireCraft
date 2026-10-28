//! Configuratore macchina: opzioni, parti abbinate, progetto generato con le opzioni scelte.

use cadcraft_engine::Session;
use serde_json::{Value, json};

fn esegui(s: &mut Session, id: &str, p: Value) -> Value {
    match s.execute(id, &p) {
        Ok(v) => v,
        Err(e) => panic!("{id}: {e}"),
    }
}

/// Le sigle delle madri di un disegno letto da file.
fn madri_del_file(file: &std::path::Path) -> Vec<String> {
    let d = cadcraft_io::read(&std::fs::read(file).unwrap(), "p.dxf").unwrap();
    let mut s = Session::empty();
    s.open_drawing(d, "p.dxf", None);
    let mut v: Vec<String> = elettrico::siglatura::componenti(&mut s).unwrap().into_iter().filter(|c| c.madre()).map(|c| c.nome).collect();
    v.sort();
    v
}

#[test]
fn opzioni_abbinate_e_progetto_generato() {
    let _uno_alla_volta = crate::uno_alla_volta();
    crate::registra();
    let mut s = Session::empty();
    s.new_drawing(true);
    esegui(&mut s, "nuovomultifoglio", json!({"fogli": 1}));
    esegui(&mut s, "cambiofoglio", json!({"foglio": 1}));
    let mut h = std::collections::HashMap::new();
    for (id, x) in [("km", 100.0), ("ka", 200.0), ("kt", 300.0), ("qf3", 380.0)] {
        let r = esegui(&mut s, &format!("simbolo.{id}"), json!({"at": [x, 200.0]}));
        h.insert(id, (r["handle"].as_str().unwrap().to_string(), r["nome"].as_str().unwrap().to_string()));
    }
    // due PLC che si escludono e un accessorio libero
    let r = esegui(&mut s, "configuratore", json!({"definisci": [
        {"codice": "p10", "descrizione": "PLC tipo A", "gruppo": "PLC"},
        {"codice": "P11", "descrizione": "PLC tipo B", "gruppo": "plc"},
        {"codice": "P20", "descrizione": "Riscaldatore"},
    ]}));
    assert_eq!(r["opzioni"].as_array().unwrap().len(), 3, "{r}");
    assert_eq!(r["opzioni"][0], json!({"codice": "P10", "descrizione": "PLC tipo A", "gruppo": "PLC", "parti": 0}));
    esegui(&mut s, "configuratore.abbina", json!({"codice": "P10", "handles": [h["ka"].0]}));
    esegui(&mut s, "configuratore.abbina", json!({"codice": "P11", "handles": [h["kt"].0]}));
    // il magnetotermico serve con l'accessorio o con il PLC A; abbinato dalla selezione
    esegui(&mut s, "select", json!({"handles": [h["qf3"].0]}));
    esegui(&mut s, "configuratore.abbina", json!({"codice": "P20"}));
    let r = esegui(&mut s, "configuratore.abbina", json!({"codice": "P10", "handles": [h["qf3"].0]}));
    assert_eq!(r["parti"], 2);
    assert!(s.execute("configuratore.abbina", &json!({"codice": "P99", "handles": [h["ka"].0]})).is_err());

    let dir = std::env::temp_dir().join(format!("cadeo_configuratore_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let sigle = |ids: &[&str]| {
        let mut v: Vec<String> = ids.iter().map(|i| h[i].1.clone()).collect();
        v.sort();
        v
    };
    // con il PLC A: il contattore comune, il relè del PLC A e il magnetotermico
    let f = dir.join("plc-a.dxf");
    let r = esegui(&mut s, "configuratore.genera", json!({"scelte": "p10", "file": f.to_string_lossy()}));
    assert_eq!(madri_del_file(&f), sigle(&["km", "ka", "qf3"]), "{r}");
    // con il PLC B e senza accessori: niente magnetotermico
    let f = dir.join("plc-b.dxf");
    esegui(&mut s, "configuratore.genera", json!({"scelte": ["P11"], "file": f.to_string_lossy()}));
    assert_eq!(madri_del_file(&f), sigle(&["km", "kt"]));
    // due opzioni dello stesso gruppo non vanno insieme
    assert!(s.execute("configuratore.genera", &json!({"scelte": "P10, P11", "file": dir.join("x.dxf").to_string_lossy()})).is_err());
    // il disegno di partenza non cambia
    assert_eq!(elettrico::siglatura::componenti(&mut s).unwrap().iter().filter(|c| c.madre()).count(), 4);

    // le opzioni restano nel file del progetto
    let f = dir.join("progetto.dxf");
    std::fs::write(&f, cadcraft_io::write(s.doc().unwrap(), "progetto.dxf").unwrap()).unwrap();
    let d = cadcraft_io::read(&std::fs::read(&f).unwrap(), "progetto.dxf").unwrap();
    let v = elettrico::configuratore::opzioni(&d);
    assert_eq!(v.iter().map(|o| (o.codice.as_str(), o.gruppo.as_str(), o.parti)).collect::<Vec<_>>(), [("P10", "PLC", 2), ("P11", "PLC", 1), ("P20", "", 1)]);

    // staccare ed eliminare
    esegui(&mut s, "configuratore.abbina", json!({"codice": "P10", "handles": [h["qf3"].0], "togli": true}));
    esegui(&mut s, "configuratore.elimina", json!({"codice": "P20"}));
    let r = esegui(&mut s, "configuratore", json!({}));
    assert_eq!(r["opzioni"].as_array().unwrap().iter().map(|o| o["parti"].as_u64().unwrap()).collect::<Vec<_>>(), [1, 1]);
    let _ = std::fs::remove_dir_all(&dir);
}
