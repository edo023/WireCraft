//! Un progetto d'esempio completo: alimentazione, motore con avviamento diretto, comandi,
//! morsettiera, cavi, PLC, lay-out del quadro e le liste sui fogli; salvato in DXF e stampato
//! in PDF.
//!
//! `cargo run --example esempio_completo -- esempio.dxf [esempio.pdf]`

use cadcraft_engine::Session;
use serde_json::json;

fn main() -> Result<(), String> {
    let out = std::env::args().nth(1).unwrap_or_else(|| "esempio.dxf".into());
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
    elettrico::esempio::disegna(
        &mut s,
        &json!({"commessa": "ESEMPIO", "multifoglio": "SCHEMA", "cliente": "Officina di prova", "descrizione": "Nastro trasportatore", "descrizioneMultifoglio": "Schema elettrico", "disegnatore": "AP", "data": "26/10/2026"}),
    )
    .map_err(|e| e.to_string())?;
    let doc = s.doc().map_err(|e| e.to_string())?.clone();
    let byte = cadcraft_io::write(&doc, &out).map_err(|e| e.to_string())?;
    std::fs::write(&out, byte).map_err(|e| e.to_string())?;
    println!("scritto {out}");
    if let Some(pdf) = &pdf {
        let r = s.execute("stampafogli", &json!({"file": pdf, "filiColorati": true})).map_err(|e| e.to_string())?;
        println!("{}", r["message"].as_str().unwrap_or(""));
    }
    Ok(())
}
