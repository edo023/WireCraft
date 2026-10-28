//! Liste in Excel: il file .xlsx con titolo, intestazioni e righe.

use cadcraft_engine::Session;
use serde_json::{Value, json};
use std::io::Read;

fn esegui(s: &mut Session, id: &str, p: Value) -> Value {
    match s.execute(id, &p) {
        Ok(v) => v,
        Err(e) => panic!("{id}: {e}"),
    }
}

/// Il testo di tutte le parti XML del file Excel (fogli e stringhe condivise).
fn contenuto(file: &std::path::Path) -> String {
    let mut z = zip::ZipArchive::new(std::fs::File::open(file).unwrap()).unwrap();
    let mut out = String::new();
    for i in 0..z.len() {
        let mut f = z.by_index(i).unwrap();
        if f.name().ends_with(".xml") {
            f.read_to_string(&mut out).unwrap();
        }
    }
    out
}

#[test]
fn liste_in_excel() {
    let _uno_alla_volta = crate::uno_alla_volta();
    crate::registra();
    let mut s = Session::empty();
    s.new_drawing(true);
    esegui(&mut s, "nuovomultifoglio", json!({"fogli": 1}));
    esegui(&mut s, "cambiofoglio", json!({"foglio": 1}));
    for x in [60.0, 80.0] {
        esegui(&mut s, "filo", json!({"points": [[x, 200.0], [x, 60.0]]}));
    }
    esegui(&mut s, "simbolo.km", json!({"at": [250.0, 180.0], "quadro": "+QG", "funzione1": "Contattore pompa", "costruttore": "Officina", "tipo": "CT-09"}));
    esegui(&mut s, "morsetti", json!({"morsettiera": "-X1", "da": [50.0, 90.0], "a": [90.0, 90.0]}));
    esegui(&mut s, "numerafili", json!({"tutti": true}));
    let dir = std::env::temp_dir().join(format!("cadeo_excel_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();

    let f = dir.join("distinta.xlsx");
    let r = esegui(&mut s, "excel", json!({"tipo": "distinta", "file": f.to_string_lossy()}));
    assert_eq!(r["righe"], 1, "{r}");
    let x = contenuto(&f);
    assert!(x.contains("DISTINTA MATERIALI"), "titolo");
    assert!(x.contains("CT-09") && x.contains("Officina"), "la riga del contattore");
    assert!(x.contains("Distinta materiali"), "nome del foglio di lavoro");

    let f = dir.join("morsettiere.xlsx");
    esegui(&mut s, "excel", json!({"tipo": "morsettiere", "file": f.to_string_lossy()}));
    let x = contenuto(&f);
    assert!(x.contains("-X1") && x.contains("01.01"), "morsettiera e numeri dei fili come testo");
    // la legenda dei simboli scrive il nome del simbolo al posto del disegno
    let f = dir.join("simboli.xlsx");
    esegui(&mut s, "excel", json!({"tipo": "simboli", "file": f.to_string_lossy()}));
    assert!(contenuto(&f).contains("Bobina"));

    assert!(s.execute("excel", &json!({"tipo": "boh", "file": dir.join("x.xlsx").to_string_lossy()})).is_err());
    assert!(s.execute("excel", &json!({"tipo": "distinta"})).is_err(), "senza file");
    let _ = std::fs::remove_dir_all(&dir);
}
