//! Il progetto d'esempio creato come commessa, salvato e riaperto: dal file si deve leggere
//! quello che si è disegnato, e Aggiorna Liste e Tabelle sul disegno riaperto non deve cambiare
//! nulla.

use cadcraft_engine::Session;
use serde_json::{Value, json};

fn esegui(s: &mut Session, id: &str, p: Value) -> Value {
    match s.execute(id, &p) {
        Ok(v) => v,
        Err(e) => panic!("{id}: {e}"),
    }
}

/// Quello che conta del progetto: fogli, componenti con i riferimenti, rimandi, materiali,
/// circuiti dell'unifilare e le liste principali.
fn istantanea(s: &mut Session) -> Vec<String> {
    let mut v = Vec::new();
    for f in elettrico::fogli::elenco(s).unwrap() {
        v.push(format!("foglio {} {} / {}", f.numero, f.titolo, f.titolo1));
    }
    let mut c: Vec<String> = elettrico::siglatura::componenti(s)
        .unwrap()
        .iter()
        .map(|c| format!("componente {} {} f{} {} {} {}", c.nome, c.pres, c.foglio, c.blocco.attributo("QUADRO"), c.blocco.attributo("REG"), c.blocco.attributo("FUNZIONE2")))
        .collect();
    c.sort();
    v.extend(c);
    let mut r: Vec<String> = elettrico::fili::rimandi(s).unwrap().iter().map(|r| format!("rimando {} {} f{} {}", r.segnale, r.partenza, r.foglio, r.riferimento)).collect();
    r.sort();
    v.extend(r);
    v.extend(elettrico::distinta::materiali(s).unwrap().iter().map(|m| format!("materiale {} {} {}", m.sigla, m.costruttore, m.codice)));
    let mut u = elettrico::unifilare::circuiti(s).unwrap();
    let calc = elettrico::unifilare::calcola_tutti(&mut u);
    for (c, r) in u.iter().zip(&calc) {
        v.push(format!("circuito {} {} {} {} {} {}", c.circuito, c.sigla, c.nominale, c.formazione(), elettrico::unifilare::numero(r.cdt_totale, 2), r.verifica()));
    }
    for tipo in ["morsettiere", "cablaggio", "cavi", "io"] {
        let (t, _, _) = elettrico::uscite::tabella(s, tipo, &json!({})).unwrap();
        v.extend(t.righe.iter().map(|riga| format!("{tipo} {riga:?}")));
    }
    v
}

#[test]
fn progetto_d_esempio_salvato_e_riaperto() {
    let _uno_alla_volta = crate::uno_alla_volta();
    crate::registra();
    let radice = std::env::temp_dir().join(format!("cadeo_esempio_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&radice);
    std::fs::create_dir_all(&radice).unwrap();

    let mut s = Session::empty();
    let r = esegui(&mut s, "commessa.esempio", json!({"radice": radice.to_string_lossy()}));
    let file = r["file"].as_str().unwrap().to_string();
    assert!(file.ends_with("SCHEMA.dxf") && std::path::Path::new(&file).exists(), "{r}");
    let c = elettrico::commesse::leggi(&radice.join("Esempio WireCraft")).unwrap();
    assert_eq!((c.cliente.as_str(), c.multifogli.len()), ("Officina di prova", 1));
    assert_eq!(elettrico::fogli::elenco(&mut s).unwrap().len(), 22);
    let prima = istantanea(&mut s);
    assert!(prima.iter().filter(|r| r.starts_with("circuito")).all(|r| r.ends_with(" OK") || r.ends_with(" -")), "{prima:#?}");

    // riaperto dal file: gli stessi dati
    let mut s2 = Session::empty();
    esegui(&mut s2, "open", json!({"path": file}));
    let dopo = istantanea(&mut s2);
    let diverse: Vec<(&String, &String)> = prima.iter().zip(&dopo).filter(|(a, b)| a != b).take(10).collect();
    assert!(diverse.is_empty() && prima.len() == dopo.len(), "{} righe prima, {} dopo; prime differenze: {diverse:#?}", prima.len(), dopo.len());

    // e Aggiorna Liste e Tabelle non cambia nulla
    esegui(&mut s2, "aggiornaliste", json!({}));
    let ancora = istantanea(&mut s2);
    let diverse: Vec<(&String, &String)> = prima.iter().zip(&ancora).filter(|(a, b)| a != b).take(10).collect();
    assert!(diverse.is_empty() && prima.len() == ancora.len(), "{} righe prima, {} dopo l'aggiornamento; prime differenze: {diverse:#?}", prima.len(), ancora.len());
    assert_eq!(elettrico::fogli::elenco(&mut s2).unwrap().len(), 22);

    elettrico::commesse::attiva_disegno(None);
    elettrico::impostazioni::usa(None, elettrico::impostazioni::Impostazioni::default());
    let _ = std::fs::remove_dir_all(&radice);
}
