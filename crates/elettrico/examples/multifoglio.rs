//! Crea un multifoglio di prova e lo salva in DXF, e se indicato anche in PDF:
//! `cargo run --example multifoglio -- prova.dxf [prova.pdf]`.

use cadcraft_engine::Session;
use serde_json::json;

fn main() -> Result<(), String> {
    let out = std::env::args().nth(1).unwrap_or_else(|| "multifoglio.dxf".into());
    let pdf = std::env::args().nth(2);
    if !elettrico::registra() {
        return Err("comandi non registrati".into());
    }
    cadcraft_engine::cmd::file::set_io(cadcraft_engine::cmd::file::IoHooks {
        read: |b, name| cadcraft_io::read(b, name).map_err(|e| e.to_string()),
        write: |d, name| cadcraft_io::write(d, name).map_err(|e| e.to_string()),
        plot: Some(|d, space, opts| cadcraft_io::plotter::plot(d, space, opts).map_err(|e| e.to_string())),
    });
    let mut s = Session::empty();
    s.new_drawing(true);
    let mut esegui = |id: &str, p: serde_json::Value| s.execute(id, &p).map_err(|e| format!("{id}: {e}"));
    esegui(
        "nuovomultifoglio",
        json!({"fogli": 4, "commessa": "ESEMPIO", "multifoglio": "SCHEMA", "cliente": "Officina di prova", "descrizione": "Quadro comando nastro", "descrizioneMultifoglio": "Schema elettrico", "disegnatore": "AP",
               "data": "25/10/2026"}),
    )?;
    for (n, t) in [(1, "Alimentazione"), (2, "Motori"), (3, "Ausiliari"), (4, "Ingressi PLC")] {
        esegui("titolofoglio", json!({"foglio": n, "titolo": t}))?;
    }
    // foglio 1: alimentazione trifase con PE e tre calate; foglio 2: arrivo e collegamenti
    esegui("cambiofoglio", json!({"foglio": 1}))?;
    esegui("alimentazioni", json!({"nomi": "L1 L2 L3 PE", "da": [35.0, 257.5], "a": [397.5, 257.5]}))?;
    for (x, y) in [(100.0, 257.5), (110.0, 247.5), (120.0, 237.5)] {
        esegui("vv", json!({"at": [x, y]}))?;
    }
    esegui("hh", json!({"at": [100.0, 150.0]}))?;
    esegui("cambiofoglio", json!({"foglio": 2}))?;
    esegui("alimentazioni", json!({"nomi": "L1 L2 L3 PE", "da": [35.0, 257.5], "a": [397.5, 257.5], "inizio": "arrivo", "fine": "nessuno"}))?;
    esegui("collegamenti", json!({"fili": 3, "points": [[80.0, 257.5], [80.0, 180.0], [200.0, 180.0]]}))?;
    esegui("rimandicollegamenti", json!({"at": [200.0, 180.0], "segnale": "05.01", "descrizione": "Motore nastro"}))?;
    // comando: pulsante di marcia e bobina del contattore su una calata da L1
    esegui("vv", json!({"at": [300.0, 257.5]}))?;
    esegui("simbolo.sbno", json!({"at": [300.0, 220.0], "funzione1": "Marcia nastro"}))?;
    esegui("simbolo.km", json!({"at": [300.0, 150.0], "funzione1": "Contattore motore"}))?;
    // foglio 1: interruttore e contatti di potenza del contattore sulle tre calate
    esegui("cambiofoglio", json!({"foglio": 1}))?;
    esegui("simbolo.qf3", json!({"at": [100.0, 210.0]}))?;
    esegui("simbolo.pot3", json!({"at": [100.0, 170.0], "madre": "-KM02.1"}))?;
    esegui("morsetti", json!({"morsettiera": "-XM", "da": [90.0, 120.0], "a": [130.0, 120.0]}))?;
    esegui("numerafili", json!({"tutti": true}))?;
    if let Some(pdf) = &pdf {
        esegui("stampafogli", json!({"file": pdf, "colori": true}))?;
        println!("scritto {pdf}");
    }
    let doc = s.doc().map_err(|e| e.to_string())?.clone();
    let byte = cadcraft_io::write(&doc, &out).map_err(|e| e.to_string())?;
    std::fs::write(&out, byte).map_err(|e| e.to_string())?;
    println!("scritto {out}");
    Ok(())
}
