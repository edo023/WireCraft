//! Impostazioni della commessa e modalità di siglatura. Un solo test: le impostazioni sono
//! condivise da tutto il programma.

use cadcraft_engine::Session;
use serde_json::{Value, json};

fn esegui(s: &mut Session, id: &str, p: Value) -> Value {
    match s.execute(id, &p) {
        Ok(v) => v,
        Err(e) => panic!("{id}: {e}"),
    }
}

fn madri(s: &mut Session) -> Vec<String> {
    let mut v: Vec<(u32, (i64, i64), String)> = elettrico::siglatura::componenti(s)
        .unwrap()
        .into_iter()
        .filter(|c| c.madre())
        .map(|c| (c.foglio, ((c.blocco.pos.0 * 10.0) as i64, -(c.blocco.pos.1 * 10.0) as i64), c.nome))
        .collect();
    v.sort();
    v.into_iter().map(|(.., n)| n).collect()
}

#[test]
fn modalita_di_siglatura_e_file_della_commessa() {
    let _uno_alla_volta = crate::uno_alla_volta();
    crate::registra();
    let dir = std::env::temp_dir().join(format!("cadeo_impostazioni_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("CFG")).unwrap();
    // una commessa con i soli settaggi del CAD d'origine: siglatura incrementale
    std::fs::write(dir.join("CFG").join("Spac_cfg.ini"), "[Siglatura_Simboli]\nTipo_Siglatura=incre\nPrefisso_Assoluto=-\n").unwrap();
    let imp = elettrico::impostazioni::carica_commessa(&dir);
    assert_eq!(imp.siglatura, elettrico::impostazioni::Siglatura::Incrementale);

    let mut s = Session::empty();
    s.new_drawing(true);
    esegui(&mut s, "nuovomultifoglio", json!({"fogli": 2}));
    esegui(&mut s, "simbolo.km", json!({"at": [100.0, 200.0]}));
    esegui(&mut s, "simbolo.km", json!({"at": [200.0, 200.0]}));
    esegui(&mut s, "cambiofoglio", json!({"foglio": 2}));
    esegui(&mut s, "simbolo.km", json!({"at": [100.0, 200.0]}));
    esegui(&mut s, "simbolo.km", json!({"at": [100.0, 120.0]}));
    assert_eq!(madri(&mut s), ["-KM1", "-KM2", "-KM3", "-KM4"]);

    // le altre modalità, ri-siglando lo stesso schema
    for (modo, attese) in [
        ("foglio-prefisso-assoluta", ["-01KM1", "-01KM2", "-02KM3", "-02KM4"]),
        ("foglio-prefisso-relativa", ["-01KM1", "-01KM2", "-02KM1", "-02KM2"]),
        ("prefisso-foglio-assoluta", ["-KM01.1", "-KM01.2", "-KM02.3", "-KM02.4"]),
        ("prefisso-foglio-relativa", ["-KM01.1", "-KM01.2", "-KM02.1", "-KM02.2"]),
        // posizionale: colonna 2 e 4 sul foglio 1, due madri nella colonna 2 del foglio 2
        ("posizionale", ["-KM01.2", "-KM01.4", "-KM02.2A", "-KM02.2B"]),
    ] {
        esegui(&mut s, "impostazioni", json!({"siglatura": modo}));
        esegui(&mut s, "siglecomponenti", json!({}));
        assert_eq!(madri(&mut s), attese, "{modo}");
    }
    // una nuova madre in posizionale nella colonna 2 del foglio 2 prende la lettera libera
    let r = esegui(&mut s, "simbolo.km", json!({"at": [110.0, 60.0]}));
    assert_eq!(r["nome"], "-KM02.2C");
    // le impostazioni cambiate sono salvate nella commessa, nel nostro file
    let salvate = std::fs::read_to_string(elettrico::impostazioni::file_commessa(&dir)).unwrap();
    assert!(salvate.contains("Modo=posizionale"), "{salvate}");
    assert_eq!(elettrico::impostazioni::carica_commessa(&dir).siglatura, elettrico::impostazioni::Siglatura::Posizionale);

    // quadro corrente: va nei nuovi simboli
    esegui(&mut s, "impostazioni", json!({"siglatura": "prefisso-foglio-relativa", "quadro": "+QG"}));
    let r = esegui(&mut s, "simbolo.ka", json!({"at": [300.0, 200.0]}));
    let c = elettrico::siglatura::componenti(&mut s).unwrap().into_iter().find(|c| c.nome == r["nome"].as_str().unwrap()).unwrap();
    assert_eq!(c.blocco.attributo("QUADRO"), "+QG");
    esegui(&mut s, "impostazioni", json!({"quadro": ""}));
    // si torna alle impostazioni predefinite, senza file
    elettrico::impostazioni::usa(None, elettrico::impostazioni::Impostazioni::default());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn configurazioni_con_nome_per_cliente() {
    let _uno_alla_volta = crate::uno_alla_volta();
    crate::registra();
    use elettrico::impostazioni::{self, Impostazioni, Siglatura};
    let dir = std::env::temp_dir().join(format!("cadeo_configurazioni_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    impostazioni::imposta_configurazioni(&dir.join("Configurazioni"));
    let commessa = dir.join("commessa");
    std::fs::create_dir_all(&commessa).unwrap();
    impostazioni::carica_commessa(&commessa);
    let mut s = Session::empty();
    s.new_drawing(true);

    // la configurazione di un cliente: siglatura incrementale, fili con prefisso
    esegui(&mut s, "impostazioni", json!({"siglatura": "incrementale", "prefissoFili": "W", "quadro": "+QG"}));
    esegui(&mut s, "configurazione.salva", json!({"nome": "Cliente Rossi"}));
    // un'altra, con i settaggi di partenza
    esegui(&mut s, "impostazioni", json!({"siglatura": "posizionale", "prefissoFili": ""}));
    esegui(&mut s, "configurazione.salva", json!({"nome": "base"}));
    assert_eq!(esegui(&mut s, "configurazioni", json!({}))["configurazioni"], json!(["base", "Cliente Rossi"]));

    // usarla cambia tutto tranne quadro, funzione e ubicazione del progetto
    esegui(&mut s, "impostazioni", json!({"quadro": "+BM"}));
    esegui(&mut s, "configurazione.usa", json!({"nome": "Cliente Rossi"}));
    let imp = impostazioni::correnti();
    assert_eq!((imp.siglatura, imp.prefisso_fili.as_str(), imp.quadro.as_str()), (Siglatura::Incrementale, "W", "+BM"));
    // ed è salvata nella commessa aperta
    assert_eq!(impostazioni::carica_commessa(&commessa).siglatura, Siglatura::Incrementale);

    // nomi non validi e configurazioni che non esistono
    assert!(s.execute("configurazione.salva", &json!({"nome": "../fuori"})).is_err());
    assert!(s.execute("configurazione.usa", &json!({"nome": "Nessuno"})).is_err());

    // esporta su file e reimporta con un nome
    let file = dir.join("esportate.ini");
    esegui(&mut s, "configurazione.esporta", json!({"file": file.to_string_lossy()}));
    esegui(&mut s, "configurazione.importa", json!({"file": file.to_string_lossy(), "nome": "Copia"}));
    assert_eq!(impostazioni::configurazione("Copia").unwrap().prefisso_fili, "W");
    // i settaggi del CAD elettrico d'origine si importano allo stesso modo
    let origine = dir.join("Spac_cfg.ini");
    std::fs::write(&origine, "[Siglatura_Simboli]\nTipo_Siglatura=posiz\n").unwrap();
    esegui(&mut s, "configurazione.importa", json!({"file": origine.to_string_lossy()}));
    assert_eq!(impostazioni::correnti().siglatura, Siglatura::Posizionale);

    // un progetto nuovo nasce con la configurazione scelta
    let c = esegui(&mut s, "commessa.nuova", json!({"radice": dir.to_string_lossy(), "nome": "Impianto Rossi", "configurazione": "Cliente Rossi"}));
    let cartella = std::path::PathBuf::from(c["cartella"].as_str().unwrap());
    assert_eq!(impostazioni::da_file(&impostazioni::file_commessa(&cartella)).unwrap().siglatura, Siglatura::Incrementale);
    assert!(s.execute("commessa.nuova", &json!({"radice": dir.to_string_lossy(), "nome": "Altro", "configurazione": "Nessuno"})).is_err());
    assert!(!dir.join("Altro").exists(), "con una configurazione sbagliata la commessa non si crea");

    esegui(&mut s, "configurazione.elimina", json!({"nome": "base"}));
    assert_eq!(impostazioni::configurazioni(), ["Cliente Rossi", "Copia"]);
    impostazioni::usa(None, Impostazioni::default());
    let _ = std::fs::remove_dir_all(&dir);
}
