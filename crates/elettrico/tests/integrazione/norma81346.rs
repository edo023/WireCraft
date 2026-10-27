//! Designazione CEI EN 81346: box di funzione, ubicazione e quadro; etichette sopra la sigla.

use cadcraft_engine::Session;
use serde_json::{Value, json};

fn esegui(s: &mut Session, id: &str, p: Value) -> Value {
    match s.execute(id, &p) {
        Ok(v) => v,
        Err(e) => panic!("{id}: {e}"),
    }
}

/// (funzione, ubicazione, quadro) della madre `nome`.
fn designazione(s: &mut Session, nome: &str) -> (String, String, String) {
    let c = elettrico::siglatura::componenti(s).unwrap().into_iter().find(|c| c.madre() && c.nome == nome).unwrap();
    let a = |t: &str| c.blocco.attributo(t).to_string();
    (a("FUNZIONE_IMPIANTO"), a("UBICAZIONE"), a("QUADRO"))
}

/// (RIF, TESTO) delle etichette di designazione.
fn etichette(s: &mut Session) -> Vec<(String, String)> {
    let r = esegui(s, "dataextraction", json!({"blocks": elettrico::norma81346::BLOCCO_ETICHETTA, "combine": false}));
    let col: Vec<String> = r["columns"].as_array().unwrap().iter().map(|c| c.as_str().unwrap().to_string()).collect();
    let k = |nome: &str| col.iter().position(|c| c == nome);
    let mut v: Vec<(String, String)> = r["rows"]
        .as_array()
        .unwrap()
        .iter()
        .map(|riga| {
            let campo = |nome: &str| k(nome).and_then(|i| riga[i].as_str()).unwrap_or_default().to_string();
            (campo("RIF"), campo("TESTO"))
        })
        .collect();
    v.sort();
    v
}

#[test]
fn box_assegnano_e_etichette_mostrano() {
    let _uno_alla_volta = crate::uno_alla_volta();
    crate::registra();
    let mut s = Session::empty();
    s.new_drawing(true);
    esegui(&mut s, "nuovomultifoglio", json!({"fogli": 2}));
    esegui(&mut s, "cambiofoglio", json!({"foglio": 1}));
    let qf = esegui(&mut s, "simbolo.qf3", json!({"at": [60.0, 200.0], "quadro": "+QG"}));
    let km1 = esegui(&mut s, "simbolo.km", json!({"at": [150.0, 200.0], "quadro": "+QG"}));
    let km2 = esegui(&mut s, "simbolo.km", json!({"at": [300.0, 200.0], "quadro": "+QG"}));
    let (qf, km1, km2) = (qf["nome"].as_str().unwrap().to_string(), km1["nome"].as_str().unwrap().to_string(), km2["nome"].as_str().unwrap().to_string());
    for x in [60.0, 80.0, 100.0] {
        esegui(&mut s, "filo", json!({"points": [[x, 120.0], [x, 60.0]]}));
    }
    esegui(&mut s, "morsetti", json!({"morsettiera": "-X1", "da": [50.0, 90.0], "a": [110.0, 90.0]}));

    // un box quadro attorno al secondo contattore
    let r = esegui(&mut s, "box.quadro", json!({"valore": "BM", "da": [270.0, 160.0], "a": [340.0, 240.0]}));
    assert_eq!(r["assegnati"], 1, "{r}");
    assert_eq!(designazione(&mut s, &km2).2, "+BM");
    assert_eq!(designazione(&mut s, &km1).2, "+QG", "fuori dal box resta com'era");
    // l'ubicazione di tutto il foglio, una funzione a sinistra e una più piccola dentro
    esegui(&mut s, "box", json!({"tipo": "ubicazione", "valore": "ZC", "da": [20.0, 50.0], "a": [400.0, 260.0]}));
    esegui(&mut s, "box.funzione", json!({"valore": "=TR1", "da": [40.0, 150.0], "a": [200.0, 240.0]}));
    esegui(&mut s, "box.funzione", json!({"valore": "TR2", "da": [130.0, 160.0], "a": [175.0, 230.0]}));
    assert_eq!(designazione(&mut s, &qf), ("=TR1".into(), "++ZC".into(), "+QG".into()));
    assert_eq!(designazione(&mut s, &km1), ("=TR2".into(), "++ZC".into(), "+QG".into()), "vale il box più piccolo");
    assert_eq!(designazione(&mut s, &km2), (String::new(), "++ZC".into(), "+BM".into()));
    let morsetti = elettrico::morsetti::morsetti(&mut s).unwrap();
    assert!(morsetti.iter().all(|m| m.blocco.attributo("UBICAZIONE") == "++ZC"), "anche i morsetti");
    // niente da cambiare la seconda volta
    assert_eq!(esegui(&mut s, "box.applica", json!({}))["assegnati"], 0);
    // valori mancanti e box troppo piccoli
    assert!(s.execute("box.quadro", &json!({"valore": "", "da": [0.0, 0.0], "a": [50.0, 50.0]})).is_err());
    assert!(s.execute("box.quadro", &json!({"valore": "X", "da": [0.0, 0.0], "a": [5.0, 50.0]})).is_err());

    // un contatto del primo contattore sul foglio 2: mostra i dati della madre
    esegui(&mut s, "cambiofoglio", json!({"foglio": 2}));
    let na = esegui(&mut s, "simbolo.na", json!({"at": [150.0, 200.0], "madre": km1}))["handle"].as_str().unwrap().to_string();
    let r = esegui(&mut s, "designazioni", json!({"fogli": "tutti"}));
    assert_eq!(r["etichette"], 4, "tre madri e il contatto: {r}");
    let e = etichette(&mut s);
    assert!(e.contains(&(na.clone(), "=TR2++ZC+QG".to_string())), "{e:?}");
    // solo quello che il box non dice già: per il primo contattore, il quadro
    let r = esegui(&mut s, "designazioni", json!({"fogli": [1], "regola": "discordanti"}));
    assert_eq!(r["etichette"], 2, "{r}");
    let testi: Vec<String> = etichette(&mut s).into_iter().map(|(_, t)| t).collect();
    assert!(testi.contains(&"+QG".to_string()), "{testi:?}");
    assert!(testi.contains(&"=TR2++ZC+QG".to_string()), "il contatto sul foglio 2 non è stato toccato: {testi:?}");
    // solo il quadro, e poi via tutte
    esegui(&mut s, "designazioni", json!({"fogli": "tutti", "livelli": "+"}));
    assert!(etichette(&mut s).iter().all(|(_, t)| t.starts_with('+') && !t.starts_with("++")));
    esegui(&mut s, "designazioni", json!({"fogli": "tutti", "togli": true}));
    assert!(etichette(&mut s).is_empty());

    // spostato nel box del quadro BM, Aggiorna Liste e Tabelle gli dà il quadro nuovo
    esegui(&mut s, "cambiofoglio", json!({"foglio": 1}));
    let h = elettrico::siglatura::componenti(&mut s).unwrap().into_iter().find(|c| c.nome == qf).unwrap().blocco.handle;
    esegui(&mut s, "move", json!({"handles": [h], "delta": [250.0, 0.0]}));
    esegui(&mut s, "aggiornaliste", json!({}));
    assert_eq!(designazione(&mut s, &qf).2, "+BM");

    // funzione e ubicazione correnti vanno nei simboli nuovi
    esegui(&mut s, "impostazioni", json!({"funzione": "TR3", "ubicazione": "ZS"}));
    let ka = esegui(&mut s, "simbolo.ka", json!({"at": [380.0, 30.0]}))["nome"].as_str().unwrap().to_string();
    let (f, u, _) = designazione(&mut s, &ka);
    assert_eq!((f.as_str(), u.as_str()), ("=TR3", "++ZS"));
    esegui(&mut s, "impostazioni", json!({"funzione": "", "ubicazione": ""}));
}
