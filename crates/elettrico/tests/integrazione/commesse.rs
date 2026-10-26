//! Commesse: nuova commessa, nuovo multifoglio con il cartiglio compilato, riapertura; lettura
//! delle commesse dei CAD elettrici d'origine se installati.

use cadcraft_engine::Session;
use serde_json::{Value, json};

fn prepara() {
    static UNA_VOLTA: std::sync::Once = std::sync::Once::new();
    UNA_VOLTA.call_once(|| {
        crate::registra();
        cadcraft_engine::cmd::file::set_io(cadcraft_engine::cmd::file::IoHooks {
            read: |b, name| cadcraft_io::read(b, name).map_err(|e| e.to_string()),
            write: |d, name| cadcraft_io::write(d, name).map_err(|e| e.to_string()),
            plot: None,
        });
    });
}

fn esegui(s: &mut Session, id: &str, p: Value) -> Value {
    match s.execute(id, &p) {
        Ok(v) => v,
        Err(e) => panic!("{id}: {e}"),
    }
}

#[test]
fn nuova_commessa_nuovo_multifoglio_e_riapertura() {
    let _uno_alla_volta = crate::uno_alla_volta();
    prepara();
    let radice = std::env::temp_dir().join(format!("cadeo_commesse_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&radice);
    std::fs::create_dir_all(&radice).unwrap();
    let mut s = Session::empty();
    let c = esegui(&mut s, "commessa.nuova", json!({"radice": radice.to_string_lossy(), "nome": "Linea Imballo", "descrizione": "Quadro linea", "cliente": "Officina Rossi"}));
    let cartella = c["cartella"].as_str().unwrap().to_string();
    assert!(std::path::Path::new(&cartella).join("Job-Description._sp").exists());
    assert!(std::path::Path::new(&cartella).join("CFG").join("WireCraft.ini").exists(), "settaggi della commessa");
    assert!(s.execute("commessa.nuova", &json!({"radice": radice.to_string_lossy(), "nome": "Linea Imballo"})).is_err(), "doppia");

    let r = esegui(&mut s, "commessa.nuovomultifoglio", json!({"commessa": cartella, "nome": "SCHEMA", "descrizione": "Schema elettrico", "disegnatore": "AP", "fogli": 3}));
    let file = r["file"].as_str().unwrap().to_string();
    assert!(file.ends_with("SCHEMA.dxf") && std::path::Path::new(&file).exists());
    assert_eq!(elettrico::fogli::elenco(&mut s).unwrap().len(), 3);
    // il cartiglio porta i dati della commessa
    let m = esegui(&mut s, "dataextraction", json!({"blocks": "MASTER*", "combine": false}));
    let col: Vec<&str> = m["columns"].as_array().unwrap().iter().map(|c| c.as_str().unwrap()).collect();
    let v = |tag: &str| m["rows"][0][col.iter().position(|c| *c == tag).unwrap()].as_str().unwrap_or("").to_string();
    assert_eq!((v("JOB-ORDER"), v("CUSTOMER"), v("FILE-NAME"), v("DRAFTSMAN")), ("Linea Imballo".into(), "Officina Rossi".into(), "SCHEMA".into(), "AP".into()));
    assert_eq!(elettrico::commesse::attiva().unwrap().nome(), "Linea Imballo");

    // l'elenco della commessa nel formato d'origine
    let elenco = esegui(&mut s, "commesse.elenco", json!({"radice": radice.to_string_lossy()}));
    let mf = &elenco["commesse"][0]["multifogli"][0];
    assert_eq!((mf["nr"].as_u64(), mf["nome"].as_str(), mf["disegnatore"].as_str()), (Some(1), Some("SCHEMA"), Some("AP")));
    let testo = std::fs::read_to_string(std::path::Path::new(&cartella).join("Job-Description._sp")).unwrap();
    assert!(testo.contains("(\"CUSTOMER\" . \"Officina Rossi\")") && testo.contains("(1 \"SCHEMA\""), "{testo}");

    // riaperto dalla commessa: stessi fogli
    let mut s2 = Session::empty();
    esegui(&mut s2, "commessa.apri", json!({"commessa": cartella}));
    assert_eq!(elettrico::fogli::elenco(&mut s2).unwrap().len(), 3);
    esegui(&mut s2, "commessa.modifica", json!({"commessa": cartella, "multifoglio": {"nome": "SCHEMA", "elaborare": false, "prefisso": "A"}}));
    let c = elettrico::commesse::leggi(std::path::Path::new(&cartella)).unwrap();
    assert_eq!((c.multifogli[0].elaborare, c.multifogli[0].prefisso.as_str()), (false, "A"));
    // fuori dalla commessa: impostazioni predefinite, senza file
    elettrico::commesse::attiva_disegno(None);
    elettrico::impostazioni::usa(None, elettrico::impostazioni::Impostazioni::default());
    let _ = std::fs::remove_dir_all(&radice);
}

#[test]
fn legge_le_commesse_d_origine() {
    let _uno_alla_volta = crate::uno_alla_volta();
    let radice = std::path::Path::new(r"C:\SPAC Automazione CAD 2026\Commesse");
    if !radice.exists() {
        eprintln!("commesse d'origine non installate: test saltato");
        return;
    }
    let v = elettrico::commesse::elenco(radice);
    let es = v.iter().find(|c| c.nome() == "SPAC Esempio 2026 - IEC").unwrap();
    assert_eq!(es.ultimo, "ELECTRICAL DIAGRAM");
    let m = es.multifogli.iter().find(|m| m.nome == "ELECTRICAL DIAGRAM").unwrap();
    assert!(es.file(&m.nome).exists(), "il DWG del multifoglio");
    assert_eq!(m.disegnatore, "M.C.");
}
