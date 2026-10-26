//! Gestione multifoglio: inserire, eliminare, rinominare, shift e compatta dei fogli con i
//! riferimenti che seguono; copia tra fogli; fogli memorizzati.

use cadcraft_engine::Session;
use serde_json::{Value, json};

fn prepara(fogli: u32) -> Session {
    static UNA_VOLTA: std::sync::Once = std::sync::Once::new();
    UNA_VOLTA.call_once(|| {
        crate::registra();
        cadcraft_engine::cmd::file::set_io(cadcraft_engine::cmd::file::IoHooks {
            read: |b, name| cadcraft_io::read(b, name).map_err(|e| e.to_string()),
            write: |d, name| cadcraft_io::write(d, name).map_err(|e| e.to_string()),
            plot: None,
        });
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

fn numeri(s: &mut Session) -> Vec<u32> {
    elettrico::fogli::elenco(s).unwrap().iter().map(|f| f.numero).collect()
}

fn componente(s: &mut Session, nome: &str, pres: &str) -> elettrico::siglatura::Componente {
    elettrico::siglatura::componenti(s).unwrap().into_iter().find(|c| (c.nome == nome || c.blocco.attributo("MADRE") == nome) && c.pres == pres).unwrap_or_else(|| panic!("{nome} {pres}"))
}

/// (numero, precedente, seguente, totale) scritti sul foglio `layer`.
fn numerazione(s: &mut Session, layer: &str) -> (String, String, String, String) {
    let r = esegui(s, "dataextraction", json!({"blocks": "NUMFG,NUMFGP,NUMFGS,NUMFGT", "combine": false}));
    let col: Vec<&str> = r["columns"].as_array().unwrap().iter().map(|c| c.as_str().unwrap()).collect();
    let i = |n: &str| col.iter().position(|c| *c == n).unwrap();
    let v = |tag: &str| {
        r["rows"].as_array().unwrap().iter().find(|x| x[i("Layer")] == layer && x[i(tag)].as_str().is_some_and(|t| !t.is_empty())).map(|x| x[i(tag)].as_str().unwrap().to_string()).unwrap_or_default()
    };
    (v("NUMERO_DI_FOGLIO"), v("FOGLIO_PRECEDENTE"), v("FOGLIO_SEGUENTE"), v("TOTALE_FOGLI"))
}

/// Bobina sul foglio 2, contatto e arrivo del rimando sul foglio 4.
fn schema(s: &mut Session) {
    esegui(s, "titolofoglio", json!({"foglio": 2, "titolo": "Comandi"}));
    esegui(s, "titolofoglio", json!({"foglio": 4, "titolo": "Potenza"}));
    esegui(s, "cambiofoglio", json!({"foglio": 2}));
    esegui(s, "simbolo.km", json!({"at": [100.0, 200.0]}));
    esegui(s, "filo", json!({"points": [[300.0, 250.0], [400.0, 250.0]]}));
    esegui(s, "rimandicollegamenti", json!({"at": [400.0, 250.0], "segnale": "X1"}));
    esegui(s, "cambiofoglio", json!({"foglio": 4}));
    esegui(s, "simbolo.na", json!({"at": [60.0, 200.0], "madre": "-KM02.1"}));
    esegui(s, "filo", json!({"points": [[30.0, 250.0], [100.0, 250.0]]}));
    esegui(s, "rimandicollegamenti", json!({"at": [30.0, 250.0], "segnale": "X1", "tipo": "arrivo"}));
}

#[test]
fn inserisci_elimina_shift_e_compatta_con_i_riferimenti() {
    let _uno_alla_volta = crate::uno_alla_volta();
    let mut s = prepara(5);
    schema(&mut s);
    let rif_partenza = |s: &mut Session| elettrico::fili::rimandi(s).unwrap().into_iter().find(|r| r.partenza).unwrap().riferimento;
    assert_eq!(rif_partenza(&mut s), "04/0A");

    // un foglio nuovo al posto del 3: il 4 diventa 5 e i riferimenti lo seguono
    esegui(&mut s, "inseriscifoglio", json!({"foglio": 3, "titolo": "Nuovo"}));
    assert_eq!(numeri(&mut s), [1, 2, 3, 4, 5, 6]);
    let fogli = elettrico::fogli::elenco(&mut s).unwrap();
    assert_eq!(fogli.iter().find(|f| f.numero == 5).unwrap().titolo, "Potenza");
    assert_eq!(fogli.iter().find(|f| f.numero == 3).unwrap().titolo, "Nuovo");
    assert!(fogli.iter().find(|f| f.numero == 3).unwrap().corrente, "si va sul foglio inserito");
    assert_eq!(rif_partenza(&mut s), "05/0A");
    assert_eq!(componente(&mut s, "-KM02.1", "R").blocco.attributo("REG3"), "05/1B");
    assert_eq!(componente(&mut s, "-KM02.1", "FA").foglio, 5);

    // eliminato il 3, poi shift del gruppo 4-5 indietro di uno
    esegui(&mut s, "eliminafogli", json!({"fogli": [3]}));
    assert_eq!(numeri(&mut s), [1, 2, 4, 5, 6]);
    assert!(s.execute("shiftfogli", &json!({"fogli": [4, 5], "posizioni": 1})).is_err(), "il 6 è occupato");
    esegui(&mut s, "shiftfogli", json!({"fogli": [4, 5], "posizioni": -1}));
    assert_eq!(numeri(&mut s), [1, 2, 3, 4, 6]);
    assert_eq!(rif_partenza(&mut s), "04/0A");
    // dal foglio 3 in poi avanti di 5, poi compatta
    esegui(&mut s, "shiftfogli", json!({"da": 3, "posizioni": 5}));
    assert_eq!(numeri(&mut s), [1, 2, 8, 9, 11]);
    esegui(&mut s, "compattafogli", json!({}));
    assert_eq!(numeri(&mut s), [1, 2, 3, 4, 5]);
    assert_eq!(rif_partenza(&mut s), "04/0A");
    // numerazione nel cartiglio: il foglio 5 è l'ultimo di 5
    let n5 = numerazione(&mut s, "5");
    assert_eq!(n5, ("05".into(), "04".into(), "/".into(), "05".into()));

    esegui(&mut s, "rinominafoglio", json!({"da": 5, "a": 20}));
    assert_eq!(numeri(&mut s), [1, 2, 3, 4, 20]);
    assert!(s.execute("rinominafoglio", &json!({"da": 20, "a": 2})).is_err(), "il 2 esiste");

    // svuota: il foglio resta ma senza il suo contenuto; i rimandi rimasti senza coppia
    esegui(&mut s, "svuotafogli", json!({"fogli": "4"}));
    assert_eq!(numeri(&mut s), [1, 2, 3, 4, 20]);
    assert_eq!(rif_partenza(&mut s), elettrico::fili::SENZA_COPPIA);
    // tutto si annulla un passo alla volta
    esegui(&mut s, "undo", json!({"count": 1}));
    assert_eq!(rif_partenza(&mut s), "04/0A");
}

#[test]
fn copia_tra_fogli_e_fogli_memorizzati() {
    let _uno_alla_volta = crate::uno_alla_volta();
    let mut s = prepara(5);
    schema(&mut s);
    esegui(&mut s, "cambiofoglio", json!({"foglio": 2}));
    let km = componente(&mut s, "-KM02.1", "M").blocco.handle;
    // la bobina copiata sul foglio 3 e 5 prende sigle nuove e il suo specchietto
    esegui(&mut s, "copiatrafogli", json!({"handles": [km], "fogli": [3, 5]}));
    assert_eq!(componente(&mut s, "-KM03.1", "M").foglio, 3);
    assert_eq!(componente(&mut s, "-KM05.1", "R").foglio, 5);
    assert_eq!(componente(&mut s, "-KM02.1", "M").foglio, 2, "l'originale resta");

    // il foglio 4 memorizzato e reinserito sul foglio 1: il contatto punta ancora a -KM02.1
    let dir = std::env::temp_dir().join(format!("cadeo_fogli_memo_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    elettrico::multifoglio::imposta_libreria(&dir);
    esegui(&mut s, "cambiofoglio", json!({"foglio": 2}));
    esegui(&mut s, "memorizzafoglio", json!({"nome": "Comando contattore", "categoria": "Ausiliari", "descrizione": "Bobina con filo e rimando"}));
    let elenco = esegui(&mut s, "fogli.memorizzati", json!({}));
    assert_eq!(elenco["fogli"][0]["categoria"], "Ausiliari");
    assert_eq!(elenco["fogli"][0]["descrizione"], "Bobina con filo e rimando");
    esegui(&mut s, "cambiofoglio", json!({"foglio": 1}));
    let r = esegui(&mut s, "inseriscimemorizzato", json!({"nome": "Comando contattore"}));
    assert!(r["handles"].as_array().unwrap().len() >= 4, "{r}");
    let km1 = componente(&mut s, "-KM01.1", "M");
    assert_eq!(km1.blocco.pos, (100.0, 200.0));
    assert_eq!(km1.blocco.layer, "1");
    assert_eq!(componente(&mut s, "-KM01.1", "R").foglio, 1, "lo specchietto della copia segue la nuova sigla");
    let numeri_fg1 = esegui(&mut s, "dataextraction", json!({"blocks": "NUMFG,TITOLO*", "combine": false}));
    let col: Vec<&str> = numeri_fg1["columns"].as_array().unwrap().iter().map(|c| c.as_str().unwrap()).collect();
    let layer = col.iter().position(|c| *c == "Layer").unwrap();
    let sul_primo = numeri_fg1["rows"].as_array().unwrap().iter().filter(|r| r[layer] == "1").count();
    assert_eq!(sul_primo, 2, "numero e titolo del foglio 2 non si memorizzano");
    let _ = std::fs::remove_dir_all(&dir);
}
