//! Schema unifilare: arrivo e partenze in colonna, dimensionamento, ricalcolo dalle partenze,
//! modifica, sbarre e utenze ridisegnate, tabella dei circuiti.

use cadcraft_engine::Session;
use cadcraft_engine::doc::EntityKind;
use serde_json::{Value, json};

fn esegui(s: &mut Session, id: &str, p: Value) -> Value {
    match s.execute(id, &p) {
        Ok(v) => v,
        Err(e) => panic!("{id}: {e}"),
    }
}

/// I blocchi inseriti sul layer il cui nome comincia con `prefisso`.
fn inserti(s: &Session, layer: &str, prefisso: &str) -> Vec<(String, f64, f64)> {
    s.doc()
        .unwrap()
        .model
        .iter()
        .filter(|e| e.common.layer == layer)
        .filter_map(|e| match &e.kind {
            EntityKind::Insert(i) if i.block.starts_with(prefisso) => Some((i.block.clone(), i.insert.x, i.insert.y)),
            _ => None,
        })
        .collect()
}

/// Le sbarre del layer: (x iniziale, x finale).
fn sbarre(s: &Session, layer: &str) -> Vec<(f64, f64)> {
    s.doc()
        .unwrap()
        .model
        .iter()
        .filter(|e| e.common.layer == layer)
        .filter_map(|e| match &e.kind {
            EntityKind::LwPolyline(p) if p.const_width > 0.5 => Some((p.vertices[0].p.x, p.vertices[1].p.x)),
            _ => None,
        })
        .collect()
}

#[test]
fn quadro_di_distribuzione() {
    let _uno_alla_volta = crate::uno_alla_volta();
    crate::registra();
    let mut s = Session::empty();
    s.new_drawing(true);
    esegui(&mut s, "nuovomultifoglio", json!({"fogli": 2}));
    esegui(&mut s, "cambiofoglio", json!({"foglio": 1}));

    let a = esegui(&mut s, "unifilare.arrivo", json!({"quadro": "+QD", "icc": 6, "lunghezza": 15, "codice": "S204-C25", "costruttore": "ABB"}));
    assert_eq!(a["colonna"], 1);
    assert_eq!(a["circuito"], "GEN");
    assert_eq!(a["sistema"], "3F+N+PE");
    // un secondo arrivo per lo stesso quadro no
    assert!(s.execute("unifilare.arrivo", &json!({"quadro": "+QD"})).is_err());

    let prese = esegui(&mut s, "unifilare.circuito", json!({"quadro": "+QD", "utenza": "prese", "potenza": 2, "denominazione": "Prese ufficio", "lunghezza": 25}));
    assert_eq!(prese["colonna"], 2);
    assert_eq!(prese["circuito"], "C1");
    assert_eq!(prese["sistema"], "F+N+PE");
    assert_eq!(prese["protezione"], "mtd");
    assert_eq!(prese["idn"], 30.0);
    // 2 kW a 230 V con cos φ 0,9: 9,7 A → 10 A, e le prese almeno 2,5 mm²
    assert_eq!(prese["in"], 10.0);
    assert_eq!(prese["formazione"], "3G2,5");
    assert_eq!(prese["ok"], true, "{prese}");

    let motore = esegui(&mut s, "unifilare.circuito", json!({"quadro": "+QD", "utenza": "motore", "potenza": 11, "cosfi": 0.85, "protezione": "mt", "denominazione": "Pompa", "lunghezza": 60}));
    assert_eq!(motore["sistema"], "3F+PE");
    assert_eq!(motore["curva"], "D");
    assert_eq!(motore["in"], 20.0);
    assert_eq!(motore["formazione"], "4G4", "{motore}");
    assert_eq!(motore["ok"], true, "{motore}");
    let riserva = esegui(&mut s, "unifilare.circuito", json!({"quadro": "+QD", "utenza": "riserva", "protezione": "mt", "in": 16}));
    assert_eq!(riserva["formazione"], "-");
    assert_eq!(riserva["verifica"], "OK");

    // sul foglio: colonne affiancate, utenze e conduttori, sbarre dall'arrivo all'ultima partenza
    let colonne = inserti(&s, "1", "CE_UNIF_");
    let xs: Vec<f64> = colonne.iter().filter(|c| c.0.starts_with("CE_UNIF_M") || c.0.starts_with("CE_UNIF_S")).map(|c| c.1).collect();
    assert_eq!(xs, vec![72.5, 97.5, 122.5, 147.5], "{colonne:?}");
    assert_eq!(inserti(&s, "1", "CE_UNIF_U_").len(), 3);
    assert_eq!(inserti(&s, "1", "CE_UNIF_COND").len(), 4);
    assert_eq!(inserti(&s, "1", "CE_UNIF_INTESTAZIONE").len(), 1);
    assert_eq!(sbarre(&s, "1"), vec![(72.5, 158.0)]);

    // l'arrivo somma le partenze: 13 kW
    let l = esegui(&mut s, "unifilare", json!({"quadro": "+QD"}));
    let v = l["circuiti"].as_array().unwrap();
    assert_eq!(v.len(), 4);
    assert!(v[0]["arrivo"].as_bool().unwrap());
    assert!((v[0]["potenza"].as_f64().unwrap() - 13.0).abs() < 1e-9, "{}", v[0]);
    // le partenze contano la caduta dell'arrivo
    assert!(v[2]["cdtTotale"].as_f64().unwrap() > v[2]["cdt"].as_f64().unwrap());

    // l'arrivo si ridimensiona sulle partenze finché la taratura non la fissa chi progetta;
    // in automatico si tiene un quarto della caduta ammessa (2,5 mm² darebbero più dell'1 %)
    assert_eq!((v[0]["in"].as_f64(), v[0]["formazione"].as_str(), v[0]["ok"].as_bool()), (Some(25.0), Some("5G4"), Some(true)), "{}", v[0]);
    let generale = v[0]["handle"].as_str().unwrap().to_string();
    let g = esegui(&mut s, "unifilare.modifica", json!({"handle": generale, "in": 16}));
    assert_eq!(g["verifica"], "Ib>In", "{g}");
    let g = esegui(&mut s, "unifilare.modifica", json!({"handle": generale, "dimensiona": true}));
    assert_eq!(g["ok"], true, "{g}");
    assert_eq!(g["in"], 25.0);

    // un'altra protezione cambia il blocco, i dati restano
    let h = motore["handle"].as_str().unwrap();
    let m = esegui(&mut s, "unifilare.modifica", json!({"handle": h, "protezione": "fus", "lunghezza": 5}));
    assert_eq!(m["protezione"], "fus");
    assert_eq!(m["denominazione"], "Pompa");
    assert_eq!(m["colonna"], 3);
    assert_ne!(m["handle"], motore["handle"]);
    // 20 A con fusibili: 1,5 mm² non basta (0,9 · 22 < 20), 2,5 sì
    assert_eq!((m["formazione"].as_str(), m["ok"].as_bool()), (Some("4G2,5"), Some(true)), "{m}");
    let m = esegui(&mut s, "unifilare.modifica", json!({"handle": m["handle"], "in": 40, "sezione": 4}));
    assert_eq!(m["verifica"], "In>0,9Iz");
    assert!(s.execute("unifilare.modifica", &json!({"handle": m["handle"], "cosfi": 1.5})).is_err());

    // la protezione è un componente: in distinta con la descrizione dai dati
    let m = elettrico::distinta::materiali(&mut s).unwrap();
    assert!(m.iter().any(|x| x.codice == "S204-C25" && x.costruttore == "ABB" && x.descrizione == "Magnetotermico 4P C25"), "{m:?}");
    let d = esegui(&mut s, "uscita", json!({"tipo": "circuiti", "quadro": "+QD"}));
    assert_eq!(d["fogli"].as_array().unwrap().len(), 1);
    let dir = std::env::temp_dir().join(format!("cadeo_unifilare_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let x = dir.join("circuiti.xlsx");
    let r = esegui(&mut s, "excel", json!({"tipo": "circuiti", "quadro": "+QD", "file": x.to_string_lossy()}));
    assert_eq!(r["righe"], 4);
    let _ = std::fs::remove_dir_all(&dir);

    // nel lay-out del quadro le protezioni occupano i moduli dei loro poli: F+N 2, 3F+N 4
    let ing = elettrico::layout::ingombri(&mut s, "+QD").unwrap();
    let largo = |sigla: &str| ing.iter().find(|i| i.sigla == sigla).map(|i| i.larghezza);
    assert_eq!(largo(prese["sigla"].as_str().unwrap()), Some(36.0), "{ing:?}");
    assert_eq!(largo(a["sigla"].as_str().unwrap()), Some(72.0));

    // il foglio 2 continua il quadro dalla prima colonna, con le sue sbarre
    esegui(&mut s, "cambiofoglio", json!({"foglio": 2}));
    // il foglio 2 è vuoto: il quadro va indicato
    let c = esegui(&mut s, "unifilare.circuito", json!({"quadro": "+QD", "utenza": "luce", "potenza": 0.5, "denominazione": "Luci"}));
    assert_eq!((c["foglio"].as_u64(), c["colonna"].as_u64(), c["circuito"].as_str()), (Some(2), Some(1), Some("C4")));
    assert_eq!(sbarre(&s, "2"), vec![(62.0, 83.0)]);
    // sullo stesso foglio il quadro si prende dai circuiti che ci sono
    let c2 = esegui(&mut s, "unifilare.circuito", json!({"utenza": "prese", "potenza": 1}));
    assert_eq!((c2["quadro"].as_str(), c2["circuito"].as_str()), (Some("+QD"), Some("C5")));
    assert_eq!(sbarre(&s, "2"), vec![(62.0, 108.0)]);
    // niente sigle doppie
    assert!(s.execute("unifilare.modifica", &json!({"handle": c["handle"], "nome": prese["sigla"]})).is_err());

    // cancellati i circuiti, Aggiorna Liste toglie utenze, conduttori, sbarre e intestazioni
    esegui(&mut s, "erase", json!({"handles": [c["handle"], c2["handle"]]}));
    esegui(&mut s, "aggiornaliste", json!({}));
    assert!(inserti(&s, "2", "CE_UNIF_").is_empty(), "{:?}", inserti(&s, "2", "CE_UNIF_"));
    assert!(sbarre(&s, "2").is_empty());
    assert_eq!(inserti(&s, "1", "CE_UNIF_U_").len(), 3);
}
