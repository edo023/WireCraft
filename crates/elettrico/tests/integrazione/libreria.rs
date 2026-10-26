//! Libreria simboli: ogni simbolo si inserisce, scambi e temporizzatori nello specchietto,
//! inserimento su fence, cancellazione e spostamento che richiudono i fili.

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

fn componente(s: &mut Session, handle: &str) -> elettrico::siglatura::Componente {
    elettrico::siglatura::componenti(s).unwrap().into_iter().find(|c| c.blocco.handle == handle).unwrap()
}

fn fili(s: &Session, layer: &str) -> Vec<((f64, f64), (f64, f64))> {
    let mut v: Vec<_> = elettrico::fili::fili(s, layer).unwrap().into_iter().map(|f| (f.a, f.b)).collect();
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v
}

#[test]
fn ogni_simbolo_della_libreria_si_inserisce() {
    let _uno_alla_volta = crate::uno_alla_volta();
    let mut s = prepara(1);
    let mut x = 20.0;
    let mut y = 260.0;
    let mut madri: std::collections::HashMap<&str, String> = Default::default();
    // prima le madri, così i figli trovano la loro
    let mut ordine: Vec<&elettrico::simboli::Simbolo> = elettrico::simboli::LIBRERIA.iter().collect();
    ordine.sort_by_key(|s| !s.madre());
    for sim in ordine {
        let mut p = json!({"at": [x, y]});
        if !sim.madre() && !sim.grafico() {
            let m = madri.get(sim.prefisso).cloned().unwrap_or_else(|| "-X1".into());
            p["madre"] = json!(m);
        }
        let r = esegui(&mut s, sim.comando, p);
        let h = r["handle"].as_str().unwrap().to_string();
        if sim.grafico() {
            assert_eq!(r["nome"], "", "{}", sim.id);
        } else {
            let c = componente(&mut s, &h);
            assert_eq!(c.pres, sim.pres, "{}", sim.id);
            assert_eq!(c.blocco.nome, sim.blocco);
            for (tag, _) in sim.pin((0.0, 0.0)) {
                assert!(c.blocco.attributi.iter().any(|(t, _)| t == &tag), "{}: manca {tag}", sim.id);
            }
            if sim.madre() {
                assert!(c.nome.starts_with(&format!("-{}01.", sim.prefisso)), "{}: {}", sim.id, c.nome);
                madri.entry(sim.prefisso).or_insert(c.nome.clone());
            }
        }
        x += 40.0;
        if x > 390.0 {
            x = 20.0;
            y -= 45.0;
        }
    }
}

#[test]
fn scambio_e_temporizzatore_nello_specchietto() {
    let _uno_alla_volta = crate::uno_alla_volta();
    let mut s = prepara(2);
    let ka = esegui(&mut s, "simbolo.ka", json!({"at": [100.0, 200.0]}));
    let kt = esegui(&mut s, "simbolo.kt", json!({"at": [200.0, 200.0]}));
    assert_eq!((ka["nome"].as_str(), kt["nome"].as_str()), (Some("-KA01.1"), Some("-KT01.1")));
    esegui(&mut s, "cambiofoglio", json!({"foglio": 2}));
    let sc = esegui(&mut s, "simbolo.sc", json!({"at": [100.0, 150.0], "madre": "-KA01.1"}));
    let rit = esegui(&mut s, "simbolo.naritardato", json!({"at": [140.0, 150.0], "madre": "-KT01.1"}));
    let sc = componente(&mut s, sc["handle"].as_str().unwrap());
    // lo scambio prende i morsetti della prima riga: NA in alto, comune in basso, NC
    assert_eq!((sc.blocco.attributo("PINA1"), sc.blocco.attributo("PINB1"), sc.blocco.attributo("PINC1")), ("14", "11", "12"));
    assert_eq!(sc.blocco.attributo("REG"), "01/2B");
    let rit = componente(&mut s, rit["handle"].as_str().unwrap());
    assert_eq!((rit.blocco.attributo("PINA1"), rit.blocco.attributo("PINB1")), ("15", "18"), "il NA ritardato usa comune e NA dello scambio");
    // gli specchietti segnano dove sono i contatti
    let rif = |s: &mut Session, madre: &str| elettrico::siglatura::componenti(s).unwrap().into_iter().find(|c| c.pres == "R" && c.blocco.attributo("MADRE") == madre).unwrap();
    assert_eq!(rif(&mut s, "-KA01.1").blocco.attributo("REG0"), "02/2C");
    assert_eq!(rif(&mut s, "-KT01.1").blocco.nome, "CE_RIF_KT");
    assert_eq!(rif(&mut s, "-KT01.1").blocco.attributo("REG0"), "02/3C");
}

#[test]
fn inserimento_su_fence() {
    let _uno_alla_volta = crate::uno_alla_volta();
    let mut s = prepara(1);
    for x in [50.0, 60.0, 70.0] {
        esegui(&mut s, "filo", json!({"points": [[x, 250.0], [x, 100.0]]}));
    }
    // fusibile unipolare: uno per filo, con sigle consecutive
    let r = esegui(&mut s, "simbolo.fu", json!({"da": [40.0, 220.0], "a": [80.0, 220.0]}));
    let nomi: Vec<&str> = r["simboli"].as_array().unwrap().iter().map(|x| x["nome"].as_str().unwrap()).collect();
    assert_eq!(nomi, ["-FU01.1", "-FU01.2", "-FU01.3"]);
    // interruttore tripolare: uno solo per i tre fili
    let r = esegui(&mut s, "simbolo.qf3", json!({"da": [40.0, 180.0], "a": [80.0, 180.0]}));
    assert_eq!(r["simboli"].as_array().unwrap().len(), 1);
    let f = fili(&s, "1");
    for x in [50.0, 60.0, 70.0] {
        assert!(f.contains(&((x, 210.0), (x, 180.0))), "polo {x}: tra fusibile e interruttore");
        assert!(f.contains(&((x, 170.0), (x, 100.0))), "polo {x}: sotto l'interruttore");
    }
    assert!(s.execute("simbolo.fu", &json!({"da": [300.0, 220.0], "a": [350.0, 220.0]})).is_err(), "fence senza fili");
}

#[test]
fn cancella_e_sposta_richiudono_i_fili() {
    let _uno_alla_volta = crate::uno_alla_volta();
    let mut s = prepara(1);
    esegui(&mut s, "filo", json!({"points": [[300.0, 250.0], [300.0, 100.0]]}));
    let km = esegui(&mut s, "simbolo.km", json!({"at": [300.0, 200.0]}));
    let km = km["handle"].as_str().unwrap().to_string();
    assert_eq!(fili(&s, "1").len(), 2);
    // spostata più in basso sullo stesso filo: il filo si richiude in alto e si apre in basso
    esegui(&mut s, "spostasimbolo", json!({"handle": km, "at": [300.0, 150.0]}));
    assert_eq!(fili(&s, "1"), vec![((300.0, 140.0), (300.0, 100.0)), ((300.0, 250.0), (300.0, 150.0))]);
    let rif = elettrico::siglatura::componenti(&mut s).unwrap().into_iter().find(|c| c.pres == "R").unwrap();
    assert_eq!(rif.blocco.pos, elettrico::specchietto::posizione((300.0, 150.0)), "lo specchietto segue la bobina");
    // cancellata: il filo torna intero e lo specchietto se ne va
    let r = esegui(&mut s, "cancellasimbolo", json!({"handles": [km]}));
    assert_eq!(r["tolti"], json!(["-KM01.1"]));
    assert_eq!(fili(&s, "1"), vec![((300.0, 250.0), (300.0, 100.0))]);
    assert!(elettrico::siglatura::componenti(&mut s).unwrap().is_empty());
    // un solo annulla rimette tutto
    esegui(&mut s, "undo", json!({"count": 1}));
    assert_eq!(elettrico::siglatura::componenti(&mut s).unwrap().len(), 2);
    assert_eq!(fili(&s, "1").len(), 2);
}

#[test]
fn testine_aggiuntive_ampliano_lo_specchietto() {
    let _uno_alla_volta = crate::uno_alla_volta();
    let mut s = prepara(1);
    let km = esegui(&mut s, "simbolo.km", json!({"at": [100.0, 200.0]}));
    let km = km["handle"].as_str().unwrap().to_string();
    esegui(&mut s, "simbolo.na", json!({"at": [200.0, 200.0], "madre": "-KM01.1"}));
    let secondo = esegui(&mut s, "simbolo.na", json!({"at": [250.0, 200.0], "madre": "-KM01.1"}));
    assert!(secondo["message"].as_str().unwrap().contains("superata"), "il contattore ha un solo NA");
    // aggiunta una testina 1NA+1NC: il secondo NA trova posto e prende 53-54
    esegui(&mut s, "editasimbolo", json!({"handle": km, "riferimenti": "CE_RIF_KM;CE_RIF_T11"}));
    let x = esegui(&mut s, "crossreference", json!({}));
    assert!(x["contattiSenzaPosto"].as_array().unwrap().is_empty(), "{x}");
    let tutti = elettrico::siglatura::componenti(&mut s).unwrap();
    let na2 = tutti.iter().find(|c| c.pres == "FA" && c.blocco.pos == (250.0, 200.0)).unwrap();
    assert_eq!((na2.blocco.attributo("PINA1"), na2.blocco.attributo("PINB1")), ("53", "54"));
    let rif: Vec<&elettrico::siglatura::Componente> = tutti.iter().filter(|c| c.pres == "R").collect();
    assert_eq!(rif.len(), 2);
    let t11 = rif.iter().find(|c| c.blocco.nome == "CE_RIF_T11").unwrap();
    let km_r = rif.iter().find(|c| c.blocco.nome == "CE_RIF_KM").unwrap();
    assert!(t11.blocco.pos.1 < km_r.blocco.pos.1, "la testina sotto il contattore");
    assert_eq!(t11.blocco.attributo("REG0"), "01/5B");
    let madre = tutti.iter().find(|c| c.nome == "-KM01.1" && c.madre()).unwrap();
    assert_eq!(madre.blocco.attributo("CODICE"), "CE_RIF_KM;CE_RIF_T11");
    // la madre cancellata si porta via entrambi gli specchietti
    esegui(&mut s, "cancellasimbolo", json!({"handles": [km]}));
    assert!(elettrico::siglatura::componenti(&mut s).unwrap().iter().all(|c| c.pres != "R"));
}
