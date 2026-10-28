//! Stampa e PDF dei fogli: una pagina per foglio, riferimenti cliccabili, test di interoperabilità.

use std::collections::HashMap;

use cadcraft_engine::Session;
use cadcraft_engine::doc::{Drawing, Space};
use serde_json::{Value, json};

fn prepara() {
    static UNA_VOLTA: std::sync::Once = std::sync::Once::new();
    UNA_VOLTA.call_once(|| {
        crate::registra();
        cadcraft_engine::cmd::file::set_io(cadcraft_engine::cmd::file::IoHooks {
            read: |b, name| cadcraft_io::read(b, name).map_err(|e| e.to_string()),
            write: |d, name| cadcraft_io::write(d, name).map_err(|e| e.to_string()),
            plot: Some(|d, space, opts| cadcraft_io::plotter::plot(d, space, opts).map_err(|e| e.to_string())),
        });
    });
}

fn esegui(s: &mut Session, id: &str, p: Value) -> Value {
    match s.execute(id, &p) {
        Ok(v) => v,
        Err(e) => panic!("{id}: {e}"),
    }
}

/// I layer disegnati nella pagina di un layout della copia di stampa.
fn layer_della_pagina(d: &Drawing, pagina: &str) -> Vec<String> {
    let list = cadcraft_engine::render::build_plot(d, &Space::Paper(pagina.into()), &cadcraft_engine::render::Options::default());
    let mut v: Vec<String> = list.prim_layers.iter().filter_map(|l| d.layers.get(*l as usize)).map(|l| l.name.clone()).collect();
    v.sort();
    v.dedup();
    v
}

fn cartella_prova(nome: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("cadeo-stampa-{nome}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn un_pdf_con_una_pagina_per_foglio_e_i_riferimenti_cliccabili() {
    let _uno_alla_volta = crate::uno_alla_volta();
    prepara();
    let mut s = Session::empty();
    s.new_drawing(true);
    esegui(&mut s, "nuovomultifoglio", json!({"fogli": 3}));
    esegui(&mut s, "titolofoglio", json!({"titolo": "Potenza", "foglio": 1}));
    esegui(&mut s, "simbolo.km", json!({"at": [100.0, 200.0]}));
    esegui(&mut s, "cambiofoglio", json!({"foglio": 2}));
    esegui(&mut s, "simbolo.na", json!({"at": [150.0, 200.0], "madre": "-KM01.1"}));
    esegui(&mut s, "cambiofoglio", json!({"foglio": 3}));
    esegui(&mut s, "circle", json!({"center": [200.0, 150.0], "radius": 20.0}));

    // ogni pagina mostra squadratura e cartiglio (layer 0) e solo il suo foglio
    let copia = elettrico::stampa::copia_di_stampa(s.doc().unwrap(), &[1, 2, 3], false);
    assert_eq!(copia.layouts.iter().map(|l| l.name.as_str()).collect::<Vec<_>>(), ["01", "02", "03"]);
    for (pagina, foglio) in [("01", "1"), ("02", "2"), ("03", "3")] {
        let layer = layer_della_pagina(&copia, pagina);
        assert!(layer.iter().any(|l| l == "0") && layer.iter().any(|l| l == foglio), "pagina {pagina}: {layer:?}");
        assert!(!layer.iter().any(|l| l != "0" && l != foglio), "pagina {pagina} con altri fogli: {layer:?}");
    }
    // il disegno non cambia: niente layout di stampa, fogli congelati come prima
    assert!(s.doc().unwrap().layouts.iter().all(|l| l.name != "01"));

    let dir = cartella_prova("tre-fogli");
    let file = dir.join("schema.pdf");
    let r = esegui(&mut s, "stampafogli", json!({"fogli": "tutti", "file": file.to_string_lossy()}));
    assert_eq!(r["pagine"], 3, "{r}");
    let pdf = std::fs::read(&file).unwrap();
    assert!(pdf.starts_with(b"%PDF-"));
    let testo = String::from_utf8_lossy(&pdf);
    assert!(testo.contains("/Type /Pages /Kids [") && testo.contains("/Count 3 >>"), "tre pagine");
    for etichetta in ["(01)", "(02)", "(03)"] {
        assert!(testo.contains(&format!("/P {etichetta}")), "etichetta di pagina {etichetta}");
    }
    // il contatto sul foglio 2 porta alla bobina (foglio 1), lo specchietto sul foglio 1 al contatto
    assert!(r["collegamenti"].as_u64().unwrap() >= 2, "{r}");
    assert!(testo.contains("/Subtype /Link") && testo.contains("/Dest ["));
    assert!(testo.contains("/PageMode /UseOutlines"), "segnalibri dei fogli");

    // solo il foglio corrente, PDF semplice
    let r = esegui(&mut s, "stampafogli", json!({"fogli": "corrente", "file": dir.join("foglio3.pdf").to_string_lossy(), "dinamico": false}));
    assert_eq!((r["pagine"].as_u64(), r["fogli"].clone()), (Some(1), json!([3])));
    let uno = std::fs::read(dir.join("foglio3.pdf")).unwrap();
    assert!(!String::from_utf8_lossy(&uno).contains("/Outlines"));

    // un disegno mai salvato senza file indicato: errore chiaro
    let e = s.execute("stampafogli", &json!({})).unwrap_err().to_string();
    assert!(e.contains("Salvare il disegno"), "{e}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// Test di interoperabilità: stampa PDF di una commessa multifoglio DWG complessa, se presente.
#[test]
fn stampa_progetto_dwg_multifoglio() {
    let _uno_alla_volta = crate::uno_alla_volta();
    let percorso = std::env::var("COMMESSA_DWG_INTEROP")
        .or_else(|_| std::env::var("SPAC_ESEMPIO_DWG"))
        .unwrap_or_else(|_| r"C:\SPAC Automazione CAD 2026\Commesse\SPAC Esempio 2026 - IEC\ELECTRICAL DIAGRAM.dwg".into());
    let Ok(byte) = std::fs::read(&percorso) else {
        eprintln!("{percorso} non trovato: test saltato");
        return;
    };
    prepara();
    let mut s = Session::empty();
    s.open_drawing(cadcraft_io::read(&byte, "esempio.dwg").unwrap(), "esempio.dwg", None);
    let fogli = elettrico::fogli::elenco(&mut s).unwrap();
    let numeri: Vec<u32> = fogli.iter().map(|f| f.numero).take(elettrico::stampa::MAX_PAGINE).collect();
    let titoli: HashMap<u32, String> = fogli.iter().map(|f| (f.numero, f.titolo.clone())).collect();
    let pagina = elettrico::stampa::pagina(s.doc().unwrap());
    eprintln!("pagina: {:?} – {:?} ({} x {} mm)", pagina.min, pagina.max, pagina.width(), pagina.height());

    let t = std::time::Instant::now();
    let o = elettrico::stampa::Opzioni { titolo: "Esempio Multifoglio DWG".into(), ..Default::default() };
    let st = elettrico::stampa::pdf_fogli(s.doc().unwrap(), &numeri, &titoli, &o).unwrap();
    let tempo = t.elapsed();
    // sempre la stessa cartella: il PDF e l'immagine restano da guardare dopo il test
    let dir = std::env::temp_dir().join("cadeo-stampa-interop");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("esempio.pdf"), &st.pdf).unwrap();
    eprintln!("{} fogli, {} riferimenti cliccabili, {} kB in {tempo:?}: {}", st.pagine, st.collegamenti, st.pdf.len() / 1024, dir.join("esempio.pdf").display());
    assert_eq!(st.pagine, numeri.len());
    assert!(st.collegamenti > 50, "rimandi e contatti cliccabili");
    assert!(String::from_utf8_lossy(&st.pdf).contains(&format!("/Count {} >>", numeri.len())), "una pagina per foglio");

    // un foglio di potenza come immagine, per guardarlo
    if let Some(f) = fogli.iter().find(|f| f.numero == 5) {
        for (colori, nome) in [(false, "foglio5.png"), (true, "foglio5-colori.png")] {
            let copia = elettrico::stampa::copia_di_stampa(s.doc().unwrap(), &[f.numero], colori);
            let png = cadcraft_io::plotter::plot(&copia, &Space::Paper(elettrico::stampa::etichetta(f.numero)), &json!({"output": "png", "resolution": 150})).unwrap();
            std::fs::write(dir.join(nome), png).unwrap();
            eprintln!("immagine del foglio 5: {}", dir.join(nome).display());
        }
    }
}
