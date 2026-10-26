//! Squadratura A3 con griglia di riferimento (colonne 0–9, righe A–F), cartiglio e blocchi di
//! numerazione dei fogli.
//!
//! Disegno nostro: misure in mm, origine nell'angolo in basso a sinistra del foglio A3. Nomi dei
//! blocchi e degli attributi come nei progetti dei CAD elettrici diffusi (vedi
//! `docs/spac-modello-dati.md`), così i progetti si scambiano:
//! - `MASTER_A3` sul foglio 0: cornice, riferimenti e cartiglio con i dati della commessa;
//! - su ogni foglio `NUMFG`, `NUMFGP`, `NUMFGS`, `NUMFGT` (numero, precedente, seguente,
//!   totale) e `TITOLO1` (titolo del foglio in due lingue).

use cadcraft_engine::{Result, Session};

use crate::util::{attributo, blocchi_definiti, blocco, colore, linea, testo};

pub const BLOCCO_MASTER: &str = "MASTER_A3";
pub const BLOCCO_TITOLO: &str = "TITOLO1";

/// Blocchi di numerazione: nome, attributo, posizione, altezza del testo.
pub const NUMERAZIONE: [(&str, &str, (f64, f64), f64); 4] = [
    ("NUMFG", "NUMERO_DI_FOGLIO", (400.0, 34.0), 4.0),
    ("NUMFGP", "FOGLIO_PRECEDENTE", (380.0, 34.0), 3.0),
    ("NUMFGS", "FOGLIO_SEGUENTE", (400.0, 19.0), 3.0),
    ("NUMFGT", "TOTALE_FOGLI", (380.0, 19.0), 3.0),
];
/// Dove si inserisce il blocco del titolo del foglio.
pub const POS_TITOLO: (f64, f64) = (238.0, 23.0);

/// Il foglio A3 orizzontale.
pub const FOGLIO: (f64, f64) = (420.0, 297.0);
/// La cornice: dentro stanno lo schema e il cartiglio; fuori, la fascia dei riferimenti.
/// Stessa griglia dei CAD elettrici diffusi (da 15,15 a 415,285: colonne di 40 mm, righe di
/// 45 mm), così i riferimenti `foglio/colonnaRiga` coincidono.
pub const CORNICE_MIN: (f64, f64) = (15.0, 15.0);
pub const CORNICE_MAX: (f64, f64) = (415.0, 285.0);
pub const COLONNE: u32 = 10;
pub const RIGHE: u32 = 6;
/// Larghezza della fascia dei riferimenti attorno alla cornice.
const FASCIA: f64 = 5.0;
/// Il cartiglio occupa la parte bassa della cornice, per tutta la larghezza.
pub const CARTIGLIO_ALTEZZA: f64 = 30.0;
/// Separatore tra foglio e colonna nei riferimenti (`16/0B`).
pub const SEPARATORE: &str = "/";
/// Altezza della fascia in testa alla cornice con le descrizioni delle colonne.
pub const FASCIA_DESCRIZIONI: f64 = 17.5;
/// Blocco della descrizione di una colonna (due righe per lingua).
pub const BLOCCO_DESCRIZIONE: &str = "CE_DESCR_COL";

/// Colonna (0–9) e riga (A–F) di un punto del foglio; i punti fuori cornice vanno sul bordo più
/// vicino. Un punto esattamente sul confine tra due celle appartiene alla precedente (a x=215 la
/// colonna è 4), come nei CAD elettrici diffusi.
pub fn colonna_riga(x: f64, y: f64) -> (u32, char) {
    let larg = (CORNICE_MAX.0 - CORNICE_MIN.0) / f64::from(COLONNE);
    let alt = (CORNICE_MAX.1 - CORNICE_MIN.1) / f64::from(RIGHE);
    let cella = |v: f64, n: u32| (v.ceil() - 1.0).clamp(0.0, f64::from(n - 1));
    let c = cella((x - CORNICE_MIN.0) / larg, COLONNE);
    let r = cella((CORNICE_MAX.1 - y) / alt, RIGHE);
    // clamp ha già ristretto i valori a 0..=9 e 0..=5
    let c = c as u32;
    let r = r as u8;
    (c, char::from(b'A' + r))
}

/// Riferimento come nei CAD elettrici: `05/3B` = foglio 5, colonna 3, riga B.
pub fn riferimento(foglio: u32, x: f64, y: f64) -> String {
    let (c, r) = colonna_riga(x, y);
    format!("{foglio:02}{SEPARATORE}{c}{r}")
}

/// Crea i blocchi che mancano. Va chiamata con il layer `0` corrente.
pub fn crea_blocchi(s: &mut Session) -> Result<()> {
    let esistenti = blocchi_definiti(s)?;
    let manca = |n: &str| !esistenti.iter().any(|b| b.eq_ignore_ascii_case(n));
    if manca(BLOCCO_MASTER) {
        crea_master(s)?;
    }
    for (nome, tag, _, altezza) in NUMERAZIONE {
        if manca(nome) {
            colore(s, "4")?;
            let mut h = Vec::new();
            attributo(s, &mut h, tag, tag, (0.0, 0.0), altezza)?;
            blocco(s, nome, h, "Numerazione del foglio")?;
        }
    }
    if manca(BLOCCO_DESCRIZIONE) {
        let mut h = Vec::new();
        // quattro righe centrate: due nella prima lingua, due nella seconda
        for (k, tag) in ["RIGA1", "RIGA2", "RIGA3", "RIGA4"].iter().enumerate() {
            colore(s, if k < 2 { "7" } else { "8" })?;
            let y = 5.25 - 3.5 * k as f64 - 1.0;
            attributo(s, &mut h, tag, tag, (0.0, y), 2.2)?;
            if let Some(t) = h.last().cloned() {
                crate::util::allinea(s, &t, cadcraft_engine::doc::HAlign::Center)?;
            }
        }
        blocco(s, BLOCCO_DESCRIZIONE, h, "Descrizione di una colonna")?;
    }
    if manca(BLOCCO_TITOLO) {
        colore(s, "4")?;
        let mut h = Vec::new();
        attributo(s, &mut h, "_TITOLO", "Titolo del foglio", (0.0, 0.0), 3.0)?;
        attributo(s, &mut h, "_TITOLO1", "Titolo del foglio (seconda lingua)", (0.0, -5.0), 2.5)?;
        blocco(s, BLOCCO_TITOLO, h, "Titolo del foglio")?;
    }
    colore(s, "BYLAYER")
}

/// Colonne del cartiglio (x assolute).
mod cart {
    pub const REVISIONI: (f64, f64) = (15.0, 85.0);
    pub const FIRME: (f64, f64) = (85.0, 145.0);
    pub const CLIENTE: (f64, f64) = (145.0, 235.0);
    pub const IMPIANTO: (f64, f64) = (235.0, 335.0);
    pub const COMMESSA: (f64, f64) = (335.0, 375.0);
    pub const FOGLI: (f64, f64) = (375.0, 415.0);
    pub const BASSO: f64 = 15.0;
    pub const ALTO: f64 = 45.0;
    pub const META: f64 = 30.0;
    pub const RIGA: f64 = 7.5;
}

fn crea_master(s: &mut Session) -> Result<()> {
    let mut h = Vec::new();
    let (x0, y0) = CORNICE_MIN;
    let (x1, y1) = CORNICE_MAX;
    // cornice, fascia e tacche della griglia
    colore(s, "2")?;
    linea(s, &mut h, &[(x0, y0), (x1, y0), (x1, y1), (x0, y1)], true)?;
    linea(s, &mut h, &[(x0 - FASCIA, y0 - FASCIA), (x1 + FASCIA, y0 - FASCIA), (x1 + FASCIA, y1 + FASCIA), (x0 - FASCIA, y1 + FASCIA)], true)?;
    let larg = (x1 - x0) / f64::from(COLONNE);
    let alt = (y1 - y0) / f64::from(RIGHE);
    for i in 1..COLONNE {
        let x = x0 + larg * f64::from(i);
        linea(s, &mut h, &[(x, y1), (x, y1 + FASCIA)], false)?;
        linea(s, &mut h, &[(x, y0 - FASCIA), (x, y0)], false)?;
    }
    for j in 1..RIGHE {
        let y = y1 - alt * f64::from(j);
        linea(s, &mut h, &[(x0 - FASCIA, y), (x0, y)], false)?;
        linea(s, &mut h, &[(x1, y), (x1 + FASCIA, y)], false)?;
    }
    colore(s, "7")?;
    for i in 0..COLONNE {
        let x = x0 + larg * (f64::from(i) + 0.5);
        let t = i.to_string();
        testo(s, &mut h, (x, y1 + FASCIA / 2.0), &t, 2.5, "MC")?;
        testo(s, &mut h, (x, y0 - FASCIA / 2.0), &t, 2.5, "MC")?;
    }
    for j in 0..RIGHE {
        let y = y1 - alt * (f64::from(j) + 0.5);
        let t = char::from(b'A' + u8::try_from(j).unwrap_or(0)).to_string();
        testo(s, &mut h, (x0 - FASCIA / 2.0, y), &t, 2.5, "MC")?;
        testo(s, &mut h, (x1 + FASCIA / 2.0, y), &t, 2.5, "MC")?;
    }
    // fascia delle descrizioni delle colonne, in testa
    let yf = y1 - FASCIA_DESCRIZIONI;
    linea(s, &mut h, &[(x0, yf), (x1, yf)], false)?;
    for i in 1..COLONNE {
        let x = x0 + larg * f64::from(i);
        linea(s, &mut h, &[(x, yf), (x, y1)], false)?;
    }
    // cartiglio
    colore(s, "2")?;
    linea(s, &mut h, &[(x0, cart::ALTO), (x1, cart::ALTO)], false)?;
    for x in [cart::FIRME.0, cart::CLIENTE.0, cart::IMPIANTO.0, cart::COMMESSA.0, cart::FOGLI.0, 395.0] {
        linea(s, &mut h, &[(x, cart::BASSO), (x, cart::ALTO)], false)?;
    }
    for k in 1..4 {
        let y = cart::BASSO + cart::RIGA * f64::from(k);
        linea(s, &mut h, &[(cart::REVISIONI.0, y), (cart::REVISIONI.1, y)], false)?;
        linea(s, &mut h, &[(cart::FIRME.0, y), (cart::FIRME.1, y)], false)?;
    }
    for x in [25.0, 55.0, 70.0] {
        linea(s, &mut h, &[(x, cart::BASSO), (x, cart::ALTO)], false)?;
    }
    for (a, b) in [cart::IMPIANTO, cart::COMMESSA, cart::FOGLI] {
        linea(s, &mut h, &[(a, cart::META), (b, cart::META)], false)?;
    }
    colore(s, "1")?; // etichette in rosso
    let etichetta = |x: f64, y: f64| (x + 1.0, y + 4.8);
    let intest = cart::BASSO + 3.0 * cart::RIGA;
    for (x, t) in [(15.0, "REV"), (25.0, "MODIFICA"), (55.0, "DATA"), (70.0, "FIRMA")] {
        testo(s, &mut h, etichetta(x, intest), t, 1.8, "L")?;
    }
    for (k, t) in ["APPROVATO", "CONTROLLATO", "DISEGNATO", "DATA"].iter().enumerate() {
        testo(s, &mut h, etichetta(cart::FIRME.0, cart::BASSO + cart::RIGA * k as f64), t, 1.8, "L")?;
    }
    testo(s, &mut h, etichetta(cart::CLIENTE.0, intest), "CLIENTE", 1.8, "L")?;
    testo(s, &mut h, etichetta(cart::IMPIANTO.0, intest), "IMPIANTO", 1.8, "L")?;
    testo(s, &mut h, (cart::IMPIANTO.0 + 1.0, cart::META - 3.0), "TITOLO FOGLIO", 1.8, "L")?;
    testo(s, &mut h, etichetta(cart::COMMESSA.0, intest), "COMMESSA", 1.8, "L")?;
    testo(s, &mut h, (cart::COMMESSA.0 + 1.0, cart::META - 3.0), "MULTIFOGLIO", 1.8, "L")?;
    testo(s, &mut h, etichetta(cart::FOGLI.0, intest), "PREC.", 1.8, "L")?;
    testo(s, &mut h, etichetta(395.0, intest), "FOGLIO", 1.8, "L")?;
    testo(s, &mut h, (cart::FOGLI.0 + 1.0, cart::META - 3.0), "TOTALE", 1.8, "L")?;
    testo(s, &mut h, (396.0, cart::META - 3.0), "SEGUE", 1.8, "L")?;
    colore(s, "4")?; // valori in ciano
    for (k, (tag, rich)) in [("APPROVED", "Approvato"), ("CHECKED", "Controllato"), ("DRAFTSMAN", "Disegnato"), ("DATE", "Data")].iter().enumerate() {
        attributo(s, &mut h, tag, rich, (cart::FIRME.0 + 22.0, cart::BASSO + cart::RIGA * k as f64 + 2.0), 2.0)?;
    }
    attributo(s, &mut h, "CUSTOMER", "Cliente", (cart::CLIENTE.0 + 2.0, 32.0), 3.0)?;
    attributo(s, &mut h, "CUSTOMER-LINE1", "Cliente (riga 2)", (cart::CLIENTE.0 + 2.0, 25.0), 2.5)?;
    attributo(s, &mut h, "CUSTOMER-LINE2", "Cliente (riga 3)", (cart::CLIENTE.0 + 2.0, 19.0), 2.5)?;
    attributo(s, &mut h, "JOB-DESCRIPTION", "Descrizione della commessa", (cart::IMPIANTO.0 + 2.0, 37.0), 2.5)?;
    attributo(s, &mut h, "DRAWING-DESCRIPTION", "Descrizione del multifoglio", (cart::IMPIANTO.0 + 2.0, 32.0), 2.5)?;
    attributo(s, &mut h, "JOB-ORDER", "Commessa", (cart::COMMESSA.0 + 2.0, 34.0), 2.5)?;
    attributo(s, &mut h, "FILE-NAME", "Multifoglio", (cart::COMMESSA.0 + 2.0, 19.0), 2.5)?;
    blocco(s, BLOCCO_MASTER, h, "Squadratura A3 con riferimenti e cartiglio")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn riferimenti_ai_bordi_della_griglia() {
        assert_eq!(colonna_riga(15.5, 284.0), (0, 'A'));
        assert_eq!(colonna_riga(414.0, 16.0), (9, 'F'));
        assert_eq!(colonna_riga(130.0, 200.0), (2, 'B'));
        // fuori cornice: il bordo più vicino
        assert_eq!(colonna_riga(-50.0, 500.0), (0, 'A'));
        assert_eq!(riferimento(5, 130.0, 200.0), "05/2B");
        // sul confine: la cella precedente (casi presi da un progetto reale)
        assert_eq!(riferimento(4, 215.0, 120.0), "04/4D");
        assert_eq!(riferimento(7, 190.0, 240.0), "07/4A");
        assert_eq!(colonna_riga(15.0, 285.0), (0, 'A'));
    }
}
