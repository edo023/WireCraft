//! Icone della barra a schede, disegnate in codice (nessuna immagine esterna): linee, rettangoli,
//! cerchi e testi in una griglia di 32×32 unità, scalati al riquadro.

use egui::{Color32, Painter, Pos2, Rect, Shape, Stroke, pos2};

#[derive(Clone, Copy)]
enum C {
    Tratto,
    Rosso,
    Blu,
    Verde,
    Giallo,
    Arancio,
    Grigio,
    Carta,
}

#[derive(Clone, Copy)]
enum G {
    /// Linea (x0, y0, x1, y1).
    L(f32, f32, f32, f32, C),
    /// Rettangolo (x0, y0, x1, y1), bordo e riempimento.
    R(f32, f32, f32, f32, C, Option<C>),
    /// Cerchio (centro, raggio), pieno o no.
    O(f32, f32, f32, C, bool),
    /// Freccia (da, a).
    F(f32, f32, f32, f32, C),
    /// Testo centrato (punto, testo, altezza).
    T(f32, f32, &'static str, f32, C),
    /// Foglio con l'angolo piegato (x0, y0, x1, y1).
    Foglio(f32, f32, f32, f32),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Icona {
    Progetti,
    Simbologia,
    Settaggi,
    Scansione,
    Impianto,
    Quadro,
    Tensioni,
    Pianta,
    Aggiorna,
    Multifoglio,
    FoglioPrecedente,
    CambioFoglio,
    FoglioSeguente,
    Stampa,
    Legenda,
    InserisciFogli,
    TitoloFoglio,
    NumeraFogli,
    Cross,
    Archivio,
    Distinta,
    DistintaTipo,
    DistintaSigla,
    DistintaCsv,
    EditaSimbolo,
    Copia,
    Sposta,
    Cancella,
    Plc,
    Bit,
    Libreria,
    NumeraBit,
    ArchivioPlc,
    Filo,
    Connessione,
    Alimentazioni,
    Collegamenti,
    RimandoAlim,
    RimandoColl,
    FiloV,
    FiloH,
    NumeraFili,
    Sigle,
    Morsetti,
    Morsettiera,
    Linea,
    Polilinea,
    Cerchio,
    Rettangolo,
    Testo,
    Tratteggio,
    Quota,
    ZoomEstensioni,
    ZoomFinestra,
    Pan,
    Rigenera,
    Apri,
    Salva,
    Annulla,
    Ripeti,
    Pdf,
    Dxf,
    Csv,
    Unifilare,
}

fn colore(c: C, tratto: Color32) -> Color32 {
    match c {
        C::Tratto => tratto,
        C::Rosso => Color32::from_rgb(222, 64, 56),
        C::Blu => Color32::from_rgb(64, 146, 230),
        C::Verde => Color32::from_rgb(92, 190, 92),
        C::Giallo => Color32::from_rgb(236, 196, 64),
        C::Arancio => Color32::from_rgb(240, 140, 40),
        C::Grigio => Color32::from_rgb(140, 148, 160),
        C::Carta => Color32::from_rgb(238, 240, 244),
    }
}

use C::*;
use G::*;

fn grafica(i: Icona) -> Vec<G> {
    match i {
        Icona::Progetti => vec![R(3.0, 9.0, 29.0, 27.0, Giallo, Some(Giallo)), R(3.0, 6.0, 13.0, 10.0, Giallo, Some(Giallo)), L(9.0, 14.0, 23.0, 14.0, Rosso), L(9.0, 18.0, 23.0, 18.0, Rosso), L(9.0, 22.0, 17.0, 22.0, Rosso)],
        Icona::Simbologia => {
            let mut v = vec![R(4.0, 4.0, 28.0, 28.0, Tratto, None)];
            for k in 1..4 {
                let d = 4.0 + 6.0 * k as f32;
                v.push(L(d, 4.0, d, 28.0, Tratto));
                v.push(L(4.0, d, 28.0, d, Tratto));
            }
            v.extend([R(4.0, 4.0, 10.0, 10.0, Blu, Some(Blu)), R(16.0, 16.0, 22.0, 22.0, Rosso, Some(Rosso))]);
            v
        }
        Icona::Settaggi => vec![O(16.0, 16.0, 10.0, Grigio, true), O(16.0, 16.0, 4.0, Carta, true), L(16.0, 2.0, 16.0, 7.0, Grigio), L(16.0, 25.0, 16.0, 30.0, Grigio), L(2.0, 16.0, 7.0, 16.0, Grigio), L(25.0, 16.0, 30.0, 16.0, Grigio)],
        Icona::Scansione => vec![Foglio(4.0, 3.0, 20.0, 24.0), O(20.0, 19.0, 6.0, Blu, false), L(24.5, 23.5, 30.0, 29.0, Blu)],
        Icona::Impianto => vec![R(3.0, 6.0, 13.0, 14.0, Blu, None), R(19.0, 6.0, 29.0, 14.0, Blu, None), R(11.0, 20.0, 21.0, 28.0, Verde, None), L(8.0, 14.0, 16.0, 20.0, Tratto), L(24.0, 14.0, 16.0, 20.0, Tratto)],
        Icona::Quadro => vec![R(7.0, 3.0, 25.0, 29.0, Tratto, Some(Grigio)), L(16.0, 3.0, 16.0, 29.0, Tratto), O(13.5, 16.0, 1.2, Tratto, true), O(18.5, 16.0, 1.2, Tratto, true)],
        Icona::Tensioni => vec![L(16.0, 4.0, 29.0, 27.0, Giallo), L(29.0, 27.0, 3.0, 27.0, Giallo), L(3.0, 27.0, 16.0, 4.0, Giallo), T(16.0, 19.0, "!", 12.0, Giallo)],
        Icona::Pianta => vec![R(4.0, 4.0, 28.0, 28.0, Tratto, None), L(4.0, 16.0, 18.0, 16.0, Tratto), L(18.0, 4.0, 18.0, 22.0, Tratto), O(10.0, 10.0, 2.0, Giallo, true), O(23.0, 22.0, 2.0, Giallo, true)],
        Icona::Aggiorna => vec![F(8.0, 12.0, 24.0, 12.0, Verde), F(24.0, 20.0, 8.0, 20.0, Blu)],
        Icona::Multifoglio => vec![R(10.0, 3.0, 28.0, 23.0, Giallo, Some(Giallo)), R(7.0, 6.0, 25.0, 26.0, Arancio, Some(Arancio)), Foglio(4.0, 9.0, 22.0, 29.0)],
        Icona::FoglioPrecedente => vec![Foglio(8.0, 3.0, 26.0, 29.0), F(22.0, 16.0, 4.0, 16.0, Blu), L(12.0, 8.0, 20.0, 8.0, Rosso)],
        Icona::CambioFoglio => vec![Foglio(6.0, 3.0, 26.0, 29.0), T(16.0, 17.0, "F", 15.0, Verde)],
        Icona::FoglioSeguente => vec![Foglio(6.0, 3.0, 24.0, 29.0), F(10.0, 16.0, 29.0, 16.0, Blu), L(10.0, 8.0, 18.0, 8.0, Rosso)],
        Icona::Stampa => vec![R(8.0, 3.0, 24.0, 12.0, Tratto, Some(Carta)), R(3.0, 12.0, 29.0, 23.0, Tratto, Some(Grigio)), R(8.0, 19.0, 24.0, 29.0, Tratto, Some(Carta)), O(25.0, 15.5, 1.2, Verde, true)],
        Icona::Legenda => vec![R(4.0, 5.0, 28.0, 27.0, Tratto, None), L(4.0, 11.0, 28.0, 11.0, Tratto), L(12.0, 5.0, 12.0, 27.0, Tratto), L(15.0, 16.0, 25.0, 16.0, Verde), L(15.0, 21.0, 25.0, 21.0, Verde), R(6.0, 14.0, 10.0, 18.0, Blu, Some(Blu))],
        Icona::InserisciFogli => vec![Foglio(5.0, 3.0, 23.0, 29.0), L(22.0, 22.0, 30.0, 22.0, Rosso), L(26.0, 18.0, 26.0, 26.0, Rosso)],
        Icona::TitoloFoglio => vec![Foglio(5.0, 3.0, 27.0, 29.0), R(8.0, 22.0, 24.0, 26.0, Rosso, None), T(16.0, 13.0, "T", 11.0, Blu)],
        Icona::NumeraFogli => vec![Foglio(5.0, 3.0, 27.0, 29.0), T(16.0, 17.0, "#", 13.0, Rosso)],
        Icona::Cross => vec![R(3.0, 4.0, 13.0, 12.0, Tratto, None), R(19.0, 20.0, 29.0, 28.0, Tratto, None), F(13.0, 8.0, 24.0, 19.0, Blu), F(19.0, 24.0, 8.0, 13.0, Verde), L(5.0, 8.0, 11.0, 8.0, Rosso)],
        Icona::Archivio => vec![R(7.0, 7.0, 25.0, 25.0, Rosso, Some(Rosso)), O(16.0, 7.0, 9.0, Rosso, true), O(16.0, 25.0, 9.0, Rosso, true), L(7.0, 13.0, 25.0, 13.0, Carta), L(7.0, 19.0, 25.0, 19.0, Carta)],
        Icona::Distinta => vec![R(6.0, 4.0, 26.0, 29.0, Tratto, Some(Carta)), R(11.0, 2.0, 21.0, 6.0, Tratto, Some(Grigio)), L(10.0, 12.0, 22.0, 12.0, Blu), L(10.0, 17.0, 22.0, 17.0, Blu), L(10.0, 22.0, 18.0, 22.0, Blu), F(18.0, 25.0, 29.0, 25.0, Verde)],
        Icona::DistintaTipo => vec![R(5.0, 5.0, 27.0, 27.0, Tratto, None), L(5.0, 12.0, 27.0, 12.0, Tratto), T(16.0, 21.0, "Σ", 11.0, Verde)],
        Icona::DistintaSigla => vec![R(5.0, 5.0, 27.0, 27.0, Tratto, None), L(5.0, 12.0, 27.0, 12.0, Tratto), T(16.0, 21.0, "K", 11.0, Blu)],
        Icona::DistintaCsv => vec![R(5.0, 5.0, 27.0, 27.0, Verde, Some(Verde)), T(16.0, 17.0, "X", 14.0, Carta)],
        Icona::EditaSimbolo => vec![R(4.0, 10.0, 18.0, 28.0, Tratto, None), L(11.0, 4.0, 11.0, 10.0, Tratto), L(23.0, 23.0, 29.0, 3.0, Arancio), L(21.0, 22.0, 27.0, 2.0, Arancio), T(24.0, 27.0, "XS", 7.0, Tratto)],
        Icona::Copia => vec![R(3.0, 3.0, 19.0, 19.0, Tratto, None), R(12.0, 12.0, 28.0, 28.0, Verde, None)],
        Icona::Sposta => vec![F(16.0, 16.0, 16.0, 2.0, Rosso), F(16.0, 16.0, 16.0, 30.0, Rosso), F(16.0, 16.0, 2.0, 16.0, Rosso), F(16.0, 16.0, 30.0, 16.0, Rosso)],
        Icona::Cancella => vec![L(6.0, 26.0, 26.0, 6.0, Grigio), L(8.0, 28.0, 28.0, 8.0, Grigio), L(5.0, 5.0, 14.0, 14.0, Rosso), L(14.0, 5.0, 5.0, 14.0, Rosso)],
        Icona::Plc => vec![R(5.0, 14.0, 27.0, 26.0, Tratto, None), L(16.0, 6.0, 16.0, 14.0, Tratto), R(12.0, 3.0, 20.0, 8.0, Tratto, None), L(9.0, 14.0, 9.0, 26.0, Tratto), L(23.0, 14.0, 23.0, 26.0, Tratto), T(16.0, 31.0, "PLC", 7.0, Blu)],
        Icona::Bit => vec![T(16.0, 13.0, "I / O", 9.0, Tratto), R(3.0, 3.0, 13.0, 9.0, Giallo, None), R(19.0, 3.0, 29.0, 9.0, Verde, None)],
        Icona::Libreria => vec![R(8.0, 4.0, 24.0, 28.0, Verde, Some(Verde)), L(11.0, 4.0, 11.0, 28.0, Carta), L(14.0, 10.0, 22.0, 10.0, Carta)],
        Icona::NumeraBit => vec![T(16.0, 16.0, "0 1", 11.0, Rosso)],
        Icona::ArchivioPlc => vec![R(8.0, 8.0, 24.0, 26.0, Verde, Some(Verde)), O(16.0, 8.0, 8.0, Verde, true), L(8.0, 14.0, 24.0, 14.0, Carta), L(8.0, 20.0, 24.0, 20.0, Carta)],
        Icona::Filo => vec![L(4.0, 28.0, 4.0, 10.0, Tratto), L(4.0, 10.0, 28.0, 10.0, Tratto), O(4.0, 28.0, 2.0, Verde, true), O(28.0, 10.0, 2.0, Verde, true)],
        Icona::Connessione => vec![L(3.0, 12.0, 29.0, 12.0, Tratto), L(16.0, 12.0, 16.0, 29.0, Tratto), O(16.0, 12.0, 3.0, Tratto, true)],
        Icona::Alimentazioni => vec![L(3.0, 9.0, 24.0, 9.0, Rosso), L(3.0, 16.0, 24.0, 16.0, Rosso), L(3.0, 23.0, 24.0, 23.0, Rosso), F(22.0, 9.0, 30.0, 9.0, Rosso), F(22.0, 16.0, 30.0, 16.0, Rosso), F(22.0, 23.0, 30.0, 23.0, Rosso)],
        Icona::Collegamenti => vec![L(6.0, 3.0, 6.0, 22.0, Tratto), L(6.0, 22.0, 29.0, 22.0, Tratto), L(12.0, 3.0, 12.0, 16.0, Tratto), L(12.0, 16.0, 29.0, 16.0, Tratto), L(18.0, 3.0, 18.0, 10.0, Tratto), L(18.0, 10.0, 29.0, 10.0, Tratto)],
        Icona::RimandoAlim => vec![L(3.0, 16.0, 18.0, 16.0, Rosso), F(14.0, 16.0, 24.0, 16.0, Rosso), Foglio(22.0, 8.0, 31.0, 24.0)],
        Icona::RimandoColl => vec![L(3.0, 16.0, 18.0, 16.0, Tratto), F(14.0, 16.0, 24.0, 16.0, Tratto), Foglio(22.0, 8.0, 31.0, 24.0)],
        Icona::FiloV => vec![L(3.0, 6.0, 29.0, 6.0, Rosso), F(16.0, 6.0, 16.0, 29.0, Tratto), O(16.0, 6.0, 2.0, Tratto, true), T(25.0, 22.0, "VV", 7.0, Blu)],
        Icona::FiloH => vec![L(5.0, 3.0, 5.0, 29.0, Tratto), F(5.0, 16.0, 29.0, 16.0, Tratto), O(5.0, 16.0, 2.0, Tratto, true), T(20.0, 26.0, "HH", 7.0, Blu)],
        Icona::NumeraFili => vec![L(6.0, 3.0, 6.0, 29.0, Tratto), T(19.0, 11.0, "01", 9.0, Verde), T(19.0, 23.0, "02", 9.0, Verde)],
        Icona::Sigle => vec![R(4.0, 9.0, 28.0, 23.0, Blu, None), T(16.0, 16.5, "-K1", 9.0, Blu)],
        Icona::Morsetti => vec![L(8.0, 3.0, 8.0, 29.0, Tratto), L(20.0, 3.0, 20.0, 29.0, Tratto), O(8.0, 16.0, 3.0, Verde, false), O(20.0, 16.0, 3.0, Verde, false), L(3.0, 16.0, 27.0, 16.0, Grigio)],
        Icona::Morsettiera => vec![R(6.0, 3.0, 26.0, 29.0, Tratto, None), L(6.0, 9.5, 26.0, 9.5, Tratto), L(6.0, 16.0, 26.0, 16.0, Tratto), L(6.0, 22.5, 26.0, 22.5, Tratto), O(16.0, 6.25, 1.6, Verde, false), O(16.0, 12.75, 1.6, Verde, false), O(16.0, 19.25, 1.6, Verde, false), O(16.0, 25.75, 1.6, Verde, false)],
        Icona::Linea => vec![L(5.0, 27.0, 27.0, 5.0, Tratto), O(5.0, 27.0, 2.0, Arancio, true), O(27.0, 5.0, 2.0, Arancio, true)],
        Icona::Polilinea => vec![L(4.0, 26.0, 10.0, 8.0, Tratto), L(10.0, 8.0, 20.0, 22.0, Tratto), L(20.0, 22.0, 28.0, 6.0, Tratto), O(10.0, 8.0, 1.8, Arancio, true), O(20.0, 22.0, 1.8, Arancio, true)],
        Icona::Cerchio => vec![O(16.0, 16.0, 11.0, Tratto, false), O(16.0, 16.0, 1.8, Arancio, true)],
        Icona::Rettangolo => vec![R(4.0, 8.0, 28.0, 24.0, Tratto, None), O(4.0, 24.0, 1.8, Arancio, true), O(28.0, 8.0, 1.8, Arancio, true)],
        Icona::Testo => vec![T(16.0, 16.0, "A", 22.0, Tratto)],
        Icona::Tratteggio => {
            let mut v = vec![R(4.0, 4.0, 28.0, 28.0, Tratto, None)];
            for k in 1..6 {
                let d = 4.0 * k as f32;
                v.push(L(4.0, 4.0 + d, 4.0 + d, 4.0, Grigio));
                v.push(L(4.0 + d, 28.0, 28.0, 4.0 + d, Grigio));
            }
            v
        }
        Icona::Quota => vec![L(5.0, 6.0, 5.0, 26.0, Tratto), L(27.0, 6.0, 27.0, 26.0, Tratto), F(16.0, 16.0, 5.0, 16.0, Blu), F(16.0, 16.0, 27.0, 16.0, Blu)],
        Icona::ZoomEstensioni => vec![O(14.0, 14.0, 9.0, Tratto, false), L(20.5, 20.5, 29.0, 29.0, Tratto), R(9.0, 9.0, 19.0, 19.0, Blu, None)],
        Icona::ZoomFinestra => vec![O(14.0, 14.0, 9.0, Tratto, false), L(20.5, 20.5, 29.0, 29.0, Tratto), L(9.0, 14.0, 19.0, 14.0, Verde), L(14.0, 9.0, 14.0, 19.0, Verde)],
        Icona::Pan => vec![F(16.0, 16.0, 16.0, 3.0, Tratto), F(16.0, 16.0, 16.0, 29.0, Tratto), F(16.0, 16.0, 3.0, 16.0, Tratto), F(16.0, 16.0, 29.0, 16.0, Tratto)],
        Icona::Rigenera => vec![O(16.0, 16.0, 10.0, Verde, false), F(26.0, 16.0, 26.0, 9.0, Verde)],
        Icona::Apri => vec![R(3.0, 9.0, 29.0, 27.0, Giallo, Some(Giallo)), R(3.0, 6.0, 13.0, 10.0, Giallo, Some(Giallo)), F(16.0, 24.0, 16.0, 12.0, Tratto)],
        Icona::Salva => vec![R(4.0, 4.0, 28.0, 28.0, Blu, Some(Blu)), R(9.0, 4.0, 23.0, 12.0, Carta, Some(Carta)), R(8.0, 18.0, 24.0, 28.0, Carta, Some(Carta))],
        Icona::Annulla => vec![F(24.0, 20.0, 6.0, 12.0, Tratto)],
        Icona::Ripeti => vec![F(8.0, 20.0, 26.0, 12.0, Tratto)],
        Icona::Pdf => vec![Foglio(5.0, 3.0, 27.0, 29.0), R(7.0, 14.0, 25.0, 22.0, Rosso, Some(Rosso)), T(16.0, 18.0, "PDF", 7.0, Carta)],
        Icona::Dxf => vec![Foglio(5.0, 3.0, 27.0, 29.0), R(7.0, 14.0, 25.0, 22.0, Blu, Some(Blu)), T(16.0, 18.0, "DXF", 7.0, Carta)],
        Icona::Unifilare => {
            // le sbarre con tre partenze, ognuna con la sua protezione
            let mut v = vec![R(3.0, 5.0, 29.0, 7.5, Tratto, Some(Tratto))];
            for x in [8.0, 16.0, 24.0] {
                v.extend([L(x, 7.5, x, 12.0, Tratto), L(x - 2.5, 12.0, x, 18.0, Tratto), L(x, 18.0, x, 29.0, Blu), L(x - 1.5, 23.0, x + 1.5, 21.0, Rosso)]);
            }
            v
        }
        Icona::Csv => vec![Foglio(5.0, 3.0, 27.0, 29.0), R(7.0, 14.0, 25.0, 22.0, Verde, Some(Verde)), T(16.0, 18.0, "CSV", 7.0, Carta)],
    }
}

/// Disegna l'icona nel riquadro `r` (quadrato), spenta se `attiva` è falso.
pub fn disegna(p: &Painter, r: Rect, icona: Icona, tratto: Color32, attiva: bool) {
    let s = r.width().min(r.height()) / 32.0;
    let o = r.center() - egui::vec2(16.0 * s, 16.0 * s);
    let pt = |x: f32, y: f32| -> Pos2 { pos2(o.x + x * s, o.y + y * s) };
    let spessore = (1.4 * s).max(1.0);
    let col = |c: C| {
        let c = colore(c, tratto);
        if attiva { c } else { c.gamma_multiply(0.35) }
    };
    for g in grafica(icona) {
        match g {
            L(x0, y0, x1, y1, c) => {
                p.line_segment([pt(x0, y0), pt(x1, y1)], Stroke::new(spessore, col(c)));
            }
            R(x0, y0, x1, y1, c, riemp) => {
                let rr = Rect::from_two_pos(pt(x0, y0), pt(x1, y1));
                if let Some(f) = riemp {
                    p.rect_filled(rr, 1.0, col(f));
                }
                p.rect_stroke(rr, 1.0, Stroke::new(spessore, col(c)), egui::StrokeKind::Middle);
            }
            O(x, y, raggio, c, pieno) => {
                if pieno {
                    p.circle_filled(pt(x, y), raggio * s, col(c));
                } else {
                    p.circle_stroke(pt(x, y), raggio * s, Stroke::new(spessore, col(c)));
                }
            }
            F(x0, y0, x1, y1, c) => {
                let (a, b) = (pt(x0, y0), pt(x1, y1));
                let d = (b - a).normalized();
                let n = egui::vec2(-d.y, d.x);
                let punta = 5.0 * s;
                p.line_segment([a, b - d * punta * 0.6], Stroke::new(spessore, col(c)));
                p.add(Shape::convex_polygon(vec![b, b - d * punta + n * punta * 0.55, b - d * punta - n * punta * 0.55], col(c), Stroke::NONE));
            }
            T(x, y, t, alt, c) => {
                p.text(pt(x, y), egui::Align2::CENTER_CENTER, t, egui::FontId::proportional(alt * s), col(c));
            }
            Foglio(x0, y0, x1, y1) => {
                let piega = (x1 - x0) * 0.3;
                let contorno = vec![pt(x0, y0), pt(x1 - piega, y0), pt(x1, y0 + piega), pt(x1, y1), pt(x0, y1)];
                p.add(Shape::convex_polygon(contorno.clone(), col(Carta), Stroke::new(spessore, col(Grigio))));
                p.line_segment([pt(x1 - piega, y0), pt(x1 - piega, y0 + piega)], Stroke::new(spessore, col(Grigio)));
                p.line_segment([pt(x1 - piega, y0 + piega), pt(x1, y0 + piega)], Stroke::new(spessore, col(Grigio)));
            }
        }
    }
}

/// Icona di un simbolo della libreria: la sua grafica scalata nel riquadro.
pub fn disegna_simbolo(p: &Painter, r: Rect, sim: &elettrico::simboli::Simbolo, tratto: Color32) {
    use elettrico::simboli::Grafica;
    let grafica = (sim.grafica)();
    let (x0, x1, y0, y1) = elettrico::simboli::estensione(&grafica);
    let lato = (x1 - x0).max(y1 - y0).max(1.0);
    let s = f64::from(r.width().min(r.height()) - 4.0) / lato;
    let (cx, cy) = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
    let c = r.center();
    let pt = |q: (f64, f64)| pos2(c.x + ((q.0 - cx) * s) as f32, c.y - ((q.1 - cy) * s) as f32);
    let st = Stroke::new(1.2, tratto);
    // archi ed ellissi come spezzate
    let curva = |centro: (f64, f64), rx: f64, ry: f64, a0: f64, a1: f64| -> Vec<Pos2> {
        let a1 = if a1 <= a0 { a1 + 360.0 } else { a1 };
        (0..=24).map(|k| {
            let t = (a0 + (a1 - a0) * f64::from(k) / 24.0).to_radians();
            pt((centro.0 + rx * t.cos(), centro.1 + ry * t.sin()))
        }).collect()
    };
    for g in grafica {
        match g {
            Grafica::Linea(a, b) => {
                p.line_segment([pt(a), pt(b)], st);
            }
            Grafica::Tratteggio(a, b) => {
                p.line_segment([pt(a), pt(b)], Stroke::new(1.0, tratto.gamma_multiply(0.6)));
            }
            Grafica::Rettangolo(a, b) => {
                p.rect_stroke(Rect::from_two_pos(pt(a), pt(b)), 0.0, st, egui::StrokeKind::Middle);
            }
            Grafica::Pieno(a, b) => {
                p.rect_filled(Rect::from_two_pos(pt(a), pt(b)), 0.0, tratto);
            }
            Grafica::Cerchio(q, raggio) => {
                p.circle_stroke(pt(q), (raggio * s) as f32, st);
            }
            Grafica::Arco(q, raggio, a0, a1) => {
                p.add(Shape::line(curva(q, raggio, raggio, a0, a1), st));
            }
            Grafica::Ellisse(q, rx, ry) => {
                p.add(Shape::line(curva(q, rx, ry, 0.0, 360.0), st));
            }
            Grafica::Testo(q, t, alt) => {
                p.text(pt(q), egui::Align2::CENTER_CENTER, t, egui::FontId::proportional((alt * s) as f32), tratto);
            }
        }
    }
}
