//! Tensioni e colori dei conduttori: colore del potenziale nelle reti, fili colorati, stampa.

use cadcraft_engine::Session;
use cadcraft_engine::doc::color::Color;
use cadcraft_engine::doc::Handle;
use serde_json::{Value, json};

fn esegui(s: &mut Session, id: &str, p: Value) -> Value {
    match s.execute(id, &p) {
        Ok(v) => v,
        Err(e) => panic!("{id}: {e}"),
    }
}

/// Il colore dei fili della rete `nome`.
fn colori_rete(s: &mut Session, nome: &str) -> Vec<Color> {
    let reti = elettrico::collegamenti::reti(s, &[]).unwrap();
    let r = reti.iter().find(|r| r.nome == nome).unwrap();
    let d = s.doc().unwrap();
    r.fili.iter().map(|f| d.entity(Handle::parse_hex(&f.handle).unwrap()).unwrap().common.color).collect()
}

#[test]
fn colori_dei_potenziali_e_dei_fili() {
    let _uno_alla_volta = crate::uno_alla_volta();
    crate::registra();
    let prima = elettrico::tensioni::voci();
    let mut s = Session::empty();
    s.new_drawing(true);
    esegui(&mut s, "nuovomultifoglio", json!({"fogli": 1}));
    esegui(&mut s, "cambiofoglio", json!({"foglio": 1}));
    esegui(&mut s, "alimentazioni", json!({"nomi": "L1 PE", "da": [20.0, 250.0], "a": [300.0, 250.0]}));
    esegui(&mut s, "filo", json!({"points": [[100.0, 250.0], [100.0, 150.0]]}));

    // il conduttore delle reti senza indicazione viene dal potenziale
    let reti = elettrico::collegamenti::reti(&mut s, &[]).unwrap();
    let colore = |n: &str| reti.iter().find(|r| r.nome == n).map(|r| r.conduttore.colore.clone()).unwrap_or_default();
    assert_eq!((colore("L1").as_str(), colore("PE").as_str()), ("BN", "GNYE"));

    // i fili colorati come il conduttore, e di nuovo col colore del layer
    let r = esegui(&mut s, "colorafili", json!({}));
    assert!(r["fili"].as_u64().unwrap() >= 2, "{r}");
    assert!(colori_rete(&mut s, "L1").iter().all(|c| *c == Color::Index(34)));
    assert!(colori_rete(&mut s, "PE").iter().all(|c| *c == Color::Index(3)));
    // restano fili: le reti sono le stesse
    assert_eq!(elettrico::collegamenti::reti(&mut s, &[]).unwrap().len(), reti.len());
    esegui(&mut s, "colorafili", json!({"togli": true}));
    assert!(colori_rete(&mut s, "L1").iter().all(|c| *c == Color::ByLayer));

    // la tabella si cambia; un colore sconosciuto è un errore
    esegui(&mut s, "tensioni", json!({"voci": [{"nome": "L1", "tensione": "690 VAC", "colore": "bk"}, {"nome": "PE", "colore": "gn/ye"}]}));
    let t = esegui(&mut s, "tensioni", json!({}));
    assert_eq!(t["voci"], json!([{"nome": "L1", "tensione": "690 VAC", "colore": "BK"}, {"nome": "PE", "tensione": "", "colore": "GNYE"}]));
    assert!(s.execute("tensioni", &json!({"voci": [{"nome": "L1", "colore": "ZZ"}]})).is_err());
    // nero: il filo resta col colore del layer
    esegui(&mut s, "colorafili", json!({}));
    assert!(colori_rete(&mut s, "L1").iter().all(|c| *c == Color::ByLayer));

    // la stampa a fili colorati
    let dir = std::env::temp_dir().join(format!("cadeo_tensioni_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let f = dir.join("fili.pdf");
    let r = esegui(&mut s, "stampafogli", json!({"file": f.to_string_lossy(), "filiColorati": true}));
    assert_eq!(r["pagine"], 1, "{r}");
    assert!(std::fs::read(&f).unwrap().starts_with(b"%PDF"));
    let _ = std::fs::remove_dir_all(&dir);
    elettrico::tensioni::cambia(prima).unwrap();
}
