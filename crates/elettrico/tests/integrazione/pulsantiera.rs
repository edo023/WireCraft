//! Fronte quadro (pulsantiera sulla porta): dispositivi del quadro, disposizione proposta,
//! foratura, elenco; la porta disegnata con i dispositivi che restano dove li si mette.

use cadcraft_engine::Session;
use serde_json::{Value, json};

fn esegui(s: &mut Session, id: &str, p: Value) -> Value {
    match s.execute(id, &p) {
        Ok(v) => v,
        Err(e) => panic!("{id}: {e}"),
    }
}

#[test]
fn dispositivi_sulla_porta_e_foratura() {
    let _uno_alla_volta = crate::uno_alla_volta();
    crate::registra();
    let mut s = Session::empty();
    s.new_drawing(true);
    esegui(&mut s, "nuovomultifoglio", json!({"fogli": 1}));
    esegui(&mut s, "cambiofoglio", json!({"foglio": 1}));
    let mut sigle = Vec::new();
    for (k, id) in ["sbno", "sbnc", "sa", "hl", "sbem", "sbno"].iter().enumerate() {
        let r = esegui(&mut s, &format!("simbolo.{id}"), json!({"at": [40.0 + 50.0 * k as f64, 200.0], "quadro": "+QG", "funzione1": format!("Funzione {k}")}));
        sigle.push(r["nome"].as_str().unwrap().to_string());
    }
    // un sensore e un pulsante di un'altra pulsantiera non vanno su questa porta
    esegui(&mut s, "simbolo.sqno", json!({"at": [40.0, 100.0], "quadro": "+QG"}));
    esegui(&mut s, "simbolo.sbno", json!({"at": [100.0, 100.0], "quadro": "+PL"}));

    let r = esegui(&mut s, "pulsantiera", json!({"quadro": "+QG", "larghezza": 300, "altezza": 600}));
    let v = r["dispositivi"].as_array().unwrap();
    assert_eq!(v.len(), 6, "{r}");
    assert_eq!(v[0]["tipo"], "sbem", "l'emergenza per prima");
    // tutti dentro la porta, a passo costante, la prima fila 150 mm sotto il bordo
    assert!(v.iter().all(|d| (0.0..=300.0).contains(&d["x"].as_f64().unwrap()) && (0.0..=600.0).contains(&d["y"].as_f64().unwrap())), "{r}");
    assert_eq!(v[0]["y"], 450.0);
    let file_ = v.iter().map(|d| d["y"].as_f64().unwrap() as i64).collect::<std::collections::BTreeSet<_>>().len();
    assert_eq!(file_, 2, "6 dispositivi su una porta da 300: quattro e due");
    // l'ultima fila è centrata sulla porta
    let ultima: Vec<f64> = v.iter().filter(|d| d["y"].as_f64().unwrap() < 450.0).map(|d| d["x"].as_f64().unwrap()).collect();
    assert!((ultima.iter().sum::<f64>() / ultima.len() as f64 - 150.0).abs() < 1e-6, "{ultima:?}");

    // la foratura in CSV
    let dir = std::env::temp_dir().join(format!("cadeo_pulsantiera_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let f = dir.join("foratura.csv");
    esegui(&mut s, "pulsantiera", json!({"quadro": "+QG", "larghezza": 400, "altezza": 600, "file": f.to_string_lossy()}));
    let csv = std::fs::read_to_string(&f).unwrap();
    assert!(csv.starts_with("Sigla;Funzione;X;Y;Diametro\n"), "{csv}");
    assert_eq!(csv.lines().count(), 7);
    assert!(csv.contains(";22.5\n"));

    // troppi dispositivi per una porta piccola
    assert!(s.execute("pulsantiera", &json!({"quadro": "+QG", "larghezza": 150, "altezza": 200})).is_err());

    // sul foglio, e in tabella per Excel
    let u = esegui(&mut s, "uscita", json!({"tipo": "pulsantiera", "quadro": "+QG", "larghezza": 400, "altezza": 600}));
    let n = u["fogli"][0].as_u64().unwrap() as u32;
    assert_eq!(elettrico::fogli::elenco(&mut s).unwrap().iter().find(|x| x.numero == n).unwrap().titolo, "Pulsantiera");
    let x = dir.join("pulsantiera.xlsx");
    let r = esegui(&mut s, "excel", json!({"tipo": "pulsantiera", "quadro": "+QG", "file": x.to_string_lossy()}));
    assert_eq!(r["righe"], 6);
    esegui(&mut s, "aggiornaliste", json!({}));
    assert!(s.execute("uscita", &json!({"tipo": "pulsantiera", "quadro": "+XX"})).is_err(), "quadro senza dispositivi");
    let _ = std::fs::remove_dir_all(&dir);
}

/// (sigla, stato, x, y) dei dispositivi del fronte del quadro +QG.
fn fronte(s: &mut Session) -> Vec<(String, String, f64, f64)> {
    let r = esegui(s, "pulsantiera", json!({"quadro": "+QG"}));
    r["dispositivi"].as_array().unwrap().iter().map(|d| (d["sigla"].as_str().unwrap().to_string(), d["stato"].as_str().unwrap().to_string(), d["x"].as_f64().unwrap(), d["y"].as_f64().unwrap())).collect()
}

fn di<'a>(v: &'a [(String, String, f64, f64)], sigla: &str) -> &'a (String, String, f64, f64) {
    v.iter().find(|d| d.0 == sigla).unwrap()
}

#[test]
fn il_fronte_resta_come_lo_si_dispone() {
    let _uno_alla_volta = crate::uno_alla_volta();
    crate::registra();
    let mut s = Session::empty();
    s.new_drawing(true);
    esegui(&mut s, "nuovomultifoglio", json!({"fogli": 1}));
    esegui(&mut s, "cambiofoglio", json!({"foglio": 1}));
    let mut sigle = Vec::new();
    let mut handle = Vec::new();
    for (k, id) in ["sbno", "sbnc", "hl", "sbem"].iter().enumerate() {
        let r = esegui(&mut s, &format!("simbolo.{id}"), json!({"at": [40.0 + 50.0 * k as f64, 200.0], "quadro": "+QG", "funzione1": format!("Funzione {k}")}));
        sigle.push(r["nome"].as_str().unwrap().to_string());
        handle.push(r["handle"].as_str().unwrap().to_string());
    }
    // la porta su un foglio suo, con tutti i dispositivi
    let d = esegui(&mut s, "pulsantiera.disegna", json!({"quadro": "+QG", "larghezza": 400, "altezza": 600}));
    assert_eq!((d["foglio"].as_u64(), d["posati"].as_u64()), (Some(2), Some(4)), "{d}");
    let v = fronte(&mut s);
    assert!(v.iter().all(|x| x.1 == "posato"), "{v:?}");
    assert_eq!(v[0].0, sigle[3], "l'emergenza per prima");

    // spostato a mano, resta dov'è: anche disegnando di nuovo e aggiornando le liste
    let r = esegui(&mut s, "pulsantiera", json!({"quadro": "+QG"}));
    let lampada = r["dispositivi"].as_array().unwrap().iter().find(|x| x["sigla"] == sigle[2].as_str()).unwrap().clone();
    // la porta da 400 × 600 è in scala 1:4: 25 mm sul foglio sono 100 mm sulla porta
    esegui(&mut s, "move", json!({"handles": [lampada["handle"]], "delta": [25.0, -25.0]}));
    let spostata = (lampada["x"].as_f64().unwrap() + 100.0, lampada["y"].as_f64().unwrap() - 100.0);
    let v = fronte(&mut s);
    let l = di(&v, &sigle[2]);
    assert!((l.2 - spostata.0).abs() < 1e-6 && (l.3 - spostata.1).abs() < 1e-6, "{l:?} invece di {spostata:?}");
    assert_eq!(esegui(&mut s, "pulsantiera.disegna", json!({"quadro": "+QG"}))["posati"], 0);
    esegui(&mut s, "aggiornaliste", json!({}));
    let l = di(&fronte(&mut s), &sigle[2]).clone();
    assert!((l.2 - spostata.0).abs() < 1e-6 && (l.3 - spostata.1).abs() < 1e-6, "dopo l'aggiornamento: {l:?}");

    // la foratura dice dove sono davvero
    let dir = std::env::temp_dir().join(format!("cadeo_fronte_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let f = dir.join("foratura.csv");
    esegui(&mut s, "pulsantiera", json!({"quadro": "+QG", "file": f.to_string_lossy()}));
    let csv = std::fs::read_to_string(&f).unwrap();
    assert!(csv.contains(&format!("{};Funzione 2;{:.1};{:.1}", sigle[2], spostata.0, spostata.1)), "{csv}");
    let _ = std::fs::remove_dir_all(&dir);

    // un dispositivo nuovo nello schema si dispone in un posto libero, gli altri non si muovono
    esegui(&mut s, "cambiofoglio", json!({"foglio": 1}));
    let nuova = esegui(&mut s, "simbolo.hl", json!({"at": [300.0, 200.0], "quadro": "+QG", "funzione1": "Allarme"}))["nome"].as_str().unwrap().to_string();
    assert_eq!(di(&fronte(&mut s), &nuova).1, "da posare");
    assert_eq!(esegui(&mut s, "pulsantiera.disegna", json!({"quadro": "+QG"}))["posati"], 1);
    let v = fronte(&mut s);
    let n = di(&v, &nuova);
    assert!(v.iter().filter(|x| x.0 != nuova).all(|x| (x.2 - n.2).abs() > 1.0 || (x.3 - n.3).abs() > 1.0), "nessuno sovrapposto: {v:?}");
    assert!((di(&v, &sigle[2]).2 - spostata.0).abs() < 1e-6);

    // rinominato nello schema: la porta segue; cancellato: segnalato, poi tolto
    esegui(&mut s, "editasimbolo", json!({"handle": handle[0], "nome": "-SB99", "funzione1": "Marcia"}));
    assert_eq!(esegui(&mut s, "pulsantiera.aggiorna", json!({}))["aggiornati"], 1);
    assert_eq!(di(&fronte(&mut s), "-SB99").1, "posato");
    esegui(&mut s, "cancellasimbolo", json!({"handles": [handle[1]]}));
    assert_eq!(esegui(&mut s, "pulsantiera.aggiorna", json!({}))["orfani"], 1);
    assert_eq!(di(&fronte(&mut s), &sigle[1]).1, "non nello schema");
    esegui(&mut s, "pulsantiera.aggiorna", json!({"togliOrfani": true}));
    assert!(fronte(&mut s).iter().all(|x| x.0 != sigle[1]));

    // tolto dalla porta e riposato a mano, dal foglio del fronte (Invio = il prossimo da posare)
    let r = esegui(&mut s, "pulsantiera", json!({"quadro": "+QG"}));
    let em = r["dispositivi"].as_array().unwrap().iter().find(|x| x["sigla"] == sigle[3].as_str()).unwrap().clone();
    esegui(&mut s, "erase", json!({"handles": [em["handle"]]}));
    esegui(&mut s, "cambiofoglio", json!({"foglio": 2}));
    s.start("pulsantiera.posa").unwrap();
    s.input(cadcraft_engine::Input::Enter).unwrap();
    s.input(cadcraft_engine::Input::Point(cadcraft_engine::geom::Vec2::new(em["xFoglio"].as_f64().unwrap(), em["yFoglio"].as_f64().unwrap()))).unwrap();
    s.input(cadcraft_engine::Input::Cancel).unwrap();
    let e = di(&fronte(&mut s), &sigle[3]).clone();
    assert_eq!(e.1, "posato");
    assert!((e.2 - em["x"].as_f64().unwrap()).abs() < 1e-6, "{e:?} {em}");
}
