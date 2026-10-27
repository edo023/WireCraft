//! Libreria simboli IEC 60617 (disegni nostri) e inserimento sullo schema.
//!
//! Ogni simbolo è un blocco con gli attributi del modello dati dei CAD elettrici diffusi:
//! `NOME`, `PRES` (M = madre, FA/FC/FS/FAP3… = figlio), `PINAk` e
//! `PINBk` (morsetti dei poli), più `QUADRO`, `FUNZIONE1`, `FUNZIONE2`, `CODICE`, `COSTRUTTORE`,
//! `TIPO`, `DESCRIZIONE` per le madri e `REG` (dove sta la madre) per i figli. I simboli senza
//! `PRES` (terra, massa) sono solo grafica.
//!
//! I simboli sono verticali: il morsetto A di ogni polo sta nel punto d'inserimento (spostato di
//! `x` per i poli successivi), il morsetto B `lunghezza` mm più in basso. Inserito su un filo, il
//! simbolo lo interrompe tra i due morsetti; cancellato o spostato, il filo si richiude.

use cadcraft_engine::cmd::has_doc;
use cadcraft_engine::doc::{EntityKind, Line};
use cadcraft_engine::geom::{Vec2, Vec3};
use cadcraft_engine::{Accept, CommandSpec, Input, Interactive, Prompt, Result, Session, Step};
use serde_json::{Value, json};

use crate::util::{
    allinea_a_destra, blocchi_definiti, blocchi_inseriti, blocco, colore, errore, esegui, handles, in_gruppo, layer_corrente, linea, punto_param, su_layer_zero, testo,
    testo_param,
};
use crate::{fili, fogli, morsetti, siglatura, specchietto};

type P = (f64, f64);

/// Elementi grafici di un simbolo, in mm dal punto d'inserimento.
#[derive(Clone, Copy, Debug)]
pub enum Grafica {
    Linea(P, P),
    Tratteggio(P, P),
    Cerchio(P, f64),
    /// Centro, raggio, angolo iniziale e finale (gradi, in senso antiorario).
    Arco(P, f64, f64, f64),
    /// Centro, semiasse orizzontale e verticale.
    Ellisse(P, f64, f64),
    Rettangolo(P, P),
    /// Rettangolo pieno.
    Pieno(P, P),
    Testo(P, &'static str, f64),
}

use Grafica::{Arco, Cerchio, Ellisse, Linea, Pieno, Rettangolo, Tratteggio};

pub struct Simbolo {
    /// Identificativo nel comando (`simbolo.na`) e nella libreria.
    pub id: &'static str,
    pub comando: &'static str,
    pub blocco: &'static str,
    pub etichetta: &'static str,
    /// Gruppo nella libreria (pannello Simboli, scheda Simboli).
    pub categoria: &'static str,
    pub menu: &'static [&'static str],
    /// Prefisso della sigla (KM, QF, SB…); per i figli è quello della madre attesa.
    pub prefisso: &'static str,
    /// `M` per le madri, `FA`, `FC`, `FS`, `FAP3`… per i figli, vuoto per la sola grafica.
    pub pres: &'static str,
    /// Ascisse dei poli.
    pub poli: &'static [f64],
    /// Distanza tra morsetto A e B di un polo (0 = solo morsetti A, es. motore).
    pub lunghezza: f64,
    /// Numeri dei morsetti (A, B) di ogni polo.
    pub morsetti: &'static [(&'static str, &'static str)],
    /// Morsetti in più: attributo, posizione, numero (il NC di uno scambio, lo 0V di un sensore).
    pub extra: &'static [(&'static str, P, &'static str)],
    /// Specchietto dei contatti proposto per le madri (blocco di riferimento).
    pub rif: Option<&'static str>,
    pub grafica: fn() -> Vec<Grafica>,
}

impl Simbolo {
    pub fn madre(&self) -> bool {
        self.pres == "M"
    }
    /// Solo grafica (terra, massa): niente sigla né attributi.
    pub fn grafico(&self) -> bool {
        self.pres.is_empty()
    }
    /// I morsetti (attributo, punto) del simbolo inserito in `at`.
    pub fn pin(&self, at: P) -> Vec<(String, P)> {
        let mut v = Vec::new();
        for (k, &x) in self.poli.iter().enumerate() {
            v.push((format!("PINA{}", k + 1), (at.0 + x, at.1)));
            if self.lunghezza > 0.0 {
                v.push((format!("PINB{}", k + 1), (at.0 + x, at.1 - self.lunghezza)));
            }
        }
        for (tag, p, _) in self.extra {
            v.push(((*tag).to_string(), (at.0 + p.0, at.1 + p.1)));
        }
        v
    }
}

// ---------- grafica ----------

/// Contatto NO sul polo `x` (morsetti a 0 e -10): la lama parte dal morsetto B e sta aperta a sinistra.
fn no(x: f64) -> [Grafica; 3] {
    [Linea((x, 0.0), (x, -3.0)), Linea((x, -7.0), (x, -10.0)), Linea((x, -7.0), (x - 2.5, -2.5))]
}
/// Contatto NC sul polo `x`: la lama appoggia sul fermo a destra.
fn nc(x: f64) -> [Grafica; 4] {
    [Linea((x, 0.0), (x, -3.0)), Linea((x, -3.0), (x + 2.0, -3.0)), Linea((x, -7.0), (x, -10.0)), Linea((x, -7.0), (x + 2.6, -2.6))]
}
/// Croce dell'interruttore sul contatto fisso del polo `x`.
fn croce(x: f64) -> [Grafica; 2] {
    [Linea((x - 0.8, -2.2), (x + 0.8, -3.8)), Linea((x - 0.8, -3.8), (x + 0.8, -2.2))]
}
/// Barretta del sezionatore sul contatto fisso del polo `x`.
fn sezione(x: f64) -> Grafica {
    Linea((x - 0.9, -3.0), (x + 0.9, -3.0))
}
/// Tratteggio del comando dalla lama NO del polo `x` fino a `xa`.
fn leva_no(x: f64, xa: f64) -> Grafica {
    Tratteggio((x - 1.25, -4.75), (xa, -4.75))
}
fn leva_nc(x: f64, xa: f64) -> Grafica {
    Tratteggio((x + 1.3, -4.75), (xa, -4.75))
}
/// Comando a pulsante (barra verticale) in `xa`.
fn pulsante(xa: f64) -> [Grafica; 3] {
    [Linea((xa, -3.5), (xa, -6.0)), Linea((xa, -3.5), (xa - 0.8, -3.5)), Linea((xa, -6.0), (xa - 0.8, -6.0))]
}
/// Comando dell'interruttore automatico (scatola) a sinistra.
fn scatola() -> Grafica {
    Rettangolo((-6.0, -3.5), (-4.0, -6.0))
}
/// Sganciatore termico (bimetallo) che finisce in `xa`, a sinistra.
fn bimetallo(xa: f64) -> [Grafica; 7] {
    let x = xa - 3.0;
    [
        Linea((x - 0.5, -4.75), (x, -4.75)),
        Linea((x, -4.75), (x, -3.75)),
        Linea((x, -3.75), (x + 1.0, -3.75)),
        Linea((x + 1.0, -3.75), (x + 1.0, -5.75)),
        Linea((x + 1.0, -5.75), (x + 2.0, -5.75)),
        Linea((x + 2.0, -5.75), (x + 2.0, -4.75)),
        Linea((x + 2.0, -4.75), (x + 3.0, -4.75)),
    ]
}
/// Ritardo (paracadute) sulla lama: verso sinistra per i NO, verso destra per i NC.
fn ritardo(sinistra: bool) -> [Grafica; 3] {
    let (x0, x1, c, a0, a1) = if sinistra { (-1.3, -3.3, -3.3, 90.0, 270.0) } else { (1.3, 3.3, 3.3, 270.0, 90.0) };
    [Linea((x0, -4.2), (x1, -4.2)), Linea((x0, -5.3), (x1, -5.3)), Arco((c, -4.75), 1.3, a0, a1)]
}
fn bobina() -> [Grafica; 3] {
    [Linea((0.0, 0.0), (0.0, -3.0)), Rettangolo((-3.5, -3.0), (3.5, -7.0)), Linea((0.0, -7.0), (0.0, -10.0))]
}
fn fusibile(x: f64) -> [Grafica; 2] {
    [Linea((x, 0.0), (x, -10.0)), Rettangolo((x - 1.25, -2.0), (x + 1.25, -8.0))]
}

fn g_no() -> Vec<Grafica> {
    no(0.0).to_vec()
}
fn g_nc() -> Vec<Grafica> {
    nc(0.0).to_vec()
}
fn g_scambio() -> Vec<Grafica> {
    // comune in basso (B), NO in alto sull'asse (A), NC in alto a sinistra (C)
    vec![Linea((0.0, -10.0), (0.0, -7.0)), Linea((0.0, -7.0), (-2.6, -2.6)), Linea((0.0, 0.0), (0.0, -3.0)), Linea((-5.0, 0.0), (-5.0, -3.0)), Linea((-5.0, -3.0), (-2.1, -3.0))]
}
fn g_no_rit() -> Vec<Grafica> {
    let mut v = no(0.0).to_vec();
    v.extend(ritardo(true));
    v
}
fn g_nc_rit() -> Vec<Grafica> {
    let mut v = nc(0.0).to_vec();
    v.extend(ritardo(false));
    v
}
fn g_pot(n: usize) -> Vec<Grafica> {
    let mut v = Vec::new();
    for k in 0..n {
        v.extend(no(10.0 * k as f64));
    }
    v.push(Tratteggio((-1.25, -4.75), (10.0 * (n as f64 - 1.0) - 1.25, -4.75)));
    v
}
fn g_pot2() -> Vec<Grafica> {
    g_pot(2)
}
fn g_pot3() -> Vec<Grafica> {
    g_pot(3)
}
fn g_termico_nc() -> Vec<Grafica> {
    let mut v = nc(0.0).to_vec();
    v.push(leva_nc(0.0, -4.0));
    v.extend(bimetallo(-4.0));
    v
}
fn g_termico_no() -> Vec<Grafica> {
    let mut v = no(0.0).to_vec();
    v.push(leva_no(0.0, -4.0));
    v.extend(bimetallo(-4.0));
    v
}
fn g_bobina() -> Vec<Grafica> {
    bobina().to_vec()
}
fn g_bobina_eccitazione() -> Vec<Grafica> {
    // ritardata all'eccitazione: riquadro con la croce accanto alla bobina
    let mut v = bobina().to_vec();
    v.extend([Rettangolo((-7.0, -3.0), (-3.5, -7.0)), Linea((-7.0, -3.0), (-3.5, -7.0)), Linea((-7.0, -7.0), (-3.5, -3.0))]);
    v
}
fn g_bobina_diseccitazione() -> Vec<Grafica> {
    // ritardata alla diseccitazione: riquadro pieno accanto alla bobina
    let mut v = bobina().to_vec();
    v.push(Pieno((-7.0, -3.0), (-3.5, -7.0)));
    v
}
fn g_elettrovalvola() -> Vec<Grafica> {
    let mut v = bobina().to_vec();
    v.extend([Linea((-3.5, -7.0), (3.5, -3.0)), Linea((3.5, -5.0), (6.0, -5.0)), Linea((6.0, -3.5), (6.0, -6.5))]);
    v
}
fn g_lampada() -> Vec<Grafica> {
    vec![Linea((0.0, 0.0), (0.0, -2.0)), Cerchio((0.0, -5.0), 3.0), Linea((-2.12, -2.88), (2.12, -7.12)), Linea((-2.12, -7.12), (2.12, -2.88)), Linea((0.0, -8.0), (0.0, -10.0))]
}
fn g_sirena() -> Vec<Grafica> {
    vec![Linea((0.0, 0.0), (0.0, -3.0)), Linea((-3.5, -3.0), (3.5, -3.0)), Arco((0.0, -3.0), 3.5, 180.0, 360.0), Linea((0.0, -6.5), (0.0, -10.0))]
}
fn g_puls_no() -> Vec<Grafica> {
    let mut v = no(0.0).to_vec();
    v.push(leva_no(0.0, -5.0));
    v.extend(pulsante(-5.0));
    v
}
fn g_puls_nc() -> Vec<Grafica> {
    let mut v = nc(0.0).to_vec();
    v.push(leva_nc(0.0, -5.0));
    v.extend(pulsante(-5.0));
    v
}
fn g_puls_luminoso() -> Vec<Grafica> {
    let mut v = g_puls_no();
    v.extend([Cerchio((-8.0, -4.75), 1.5), Linea((-9.06, -3.69), (-6.94, -5.81)), Linea((-9.06, -5.81), (-6.94, -3.69))]);
    v
}
fn g_emergenza() -> Vec<Grafica> {
    let mut v = nc(0.0).to_vec();
    v.push(leva_nc(0.0, -5.0));
    v.extend([Linea((-5.0, -2.75), (-5.0, -6.75)), Arco((-5.0, -4.75), 2.0, 90.0, 270.0)]);
    v
}
fn g_selettore() -> Vec<Grafica> {
    let mut v = no(0.0).to_vec();
    v.push(leva_no(0.0, -5.0));
    v.extend([Linea((-6.0, -3.5), (-5.0, -3.5)), Linea((-5.0, -3.5), (-5.0, -6.0)), Linea((-5.0, -6.0), (-4.0, -6.0))]);
    v
}
fn g_chiave() -> Vec<Grafica> {
    let mut v = no(0.0).to_vec();
    v.push(leva_no(0.0, -4.5));
    v.extend([Cerchio((-6.2, -4.75), 1.0), Linea((-5.2, -4.75), (-4.5, -4.75)), Linea((-4.8, -4.75), (-4.8, -5.6))]);
    v
}
fn g_finecorsa_no() -> Vec<Grafica> {
    let mut v = no(0.0).to_vec();
    v.push(leva_no(0.0, -4.2));
    v.extend([Cerchio((-5.2, -4.75), 1.0), Linea((-6.2, -6.0), (-4.2, -6.0))]);
    v
}
fn g_finecorsa_nc() -> Vec<Grafica> {
    let mut v = nc(0.0).to_vec();
    v.push(leva_nc(0.0, -4.2));
    v.extend([Cerchio((-5.2, -4.75), 1.0), Linea((-6.2, -6.0), (-4.2, -6.0))]);
    v
}
fn g_pressostato() -> Vec<Grafica> {
    let mut v = no(0.0).to_vec();
    v.push(leva_no(0.0, -4.5));
    v.extend([Rettangolo((-7.5, -3.5), (-4.5, -6.0)), Grafica::Testo((-6.0, -4.75), "P", 2.0)]);
    v
}
fn g_termostato() -> Vec<Grafica> {
    let mut v = nc(0.0).to_vec();
    v.push(leva_nc(0.0, -4.5));
    v.extend([Rettangolo((-7.5, -3.5), (-4.5, -6.0)), Grafica::Testo((-6.0, -4.75), "T", 2.0)]);
    v
}
/// Sensore a tre fili: + in alto, uscita in basso sull'asse, 0V in basso a destra.
fn sensore() -> Vec<Grafica> {
    vec![Linea((0.0, 0.0), (0.0, -2.5)), Rettangolo((-3.0, -2.5), (8.0, -7.5)), Linea((0.0, -7.5), (0.0, -10.0)), Linea((5.0, -7.5), (5.0, -10.0))]
}
fn g_prossimita() -> Vec<Grafica> {
    let mut v = sensore();
    v.extend([Linea((2.5, -3.5), (4.0, -5.0)), Linea((4.0, -5.0), (2.5, -6.5)), Linea((2.5, -6.5), (1.0, -5.0)), Linea((1.0, -5.0), (2.5, -3.5))]);
    v
}
fn g_fotocellula() -> Vec<Grafica> {
    let mut v = sensore();
    v.extend([Linea((0.5, -3.5), (3.0, -5.0)), Linea((3.0, -5.0), (2.0, -5.0)), Linea((3.0, -5.0), (2.6, -4.1)), Linea((2.5, -4.5), (5.0, -6.0)), Linea((5.0, -6.0), (4.0, -6.0)), Linea((5.0, -6.0), (4.6, -5.1))]);
    v
}
fn g_fusibile() -> Vec<Grafica> {
    fusibile(0.0).to_vec()
}
fn g_fusibili3() -> Vec<Grafica> {
    let mut v = Vec::new();
    for x in [0.0, 10.0, 20.0] {
        v.extend(fusibile(x));
    }
    v
}
/// Interruttore automatico a `n` poli (magnetotermico).
fn interruttore(n: usize) -> Vec<Grafica> {
    let mut v = Vec::new();
    for k in 0..n {
        let x = 10.0 * k as f64;
        v.extend(no(x));
        v.extend(croce(x));
    }
    v.push(scatola());
    v.push(Tratteggio((-4.0, -4.75), (10.0 * (n as f64 - 1.0) - 1.25, -4.75)));
    v
}
fn g_qf1() -> Vec<Grafica> {
    interruttore(1)
}
fn g_qf2() -> Vec<Grafica> {
    interruttore(2)
}
fn g_qf3() -> Vec<Grafica> {
    interruttore(3)
}
fn g_qf4() -> Vec<Grafica> {
    interruttore(4)
}
fn g_salvamotore() -> Vec<Grafica> {
    let mut v = interruttore(3);
    v.extend(bimetallo(-6.0));
    v
}
fn differenziale(n: usize) -> Vec<Grafica> {
    let mut v = interruttore(n);
    let largo = 10.0 * (n as f64 - 1.0);
    v.extend([Ellisse((largo / 2.0, -8.6), largo / 2.0 + 2.5, 0.9), Tratteggio((-2.5, -8.6), (-5.0, -8.6)), Linea((-5.0, -8.6), (-5.0, -6.0))]);
    v
}
fn g_qd2() -> Vec<Grafica> {
    differenziale(2)
}
fn g_qd4() -> Vec<Grafica> {
    differenziale(4)
}
fn g_sezionatore3() -> Vec<Grafica> {
    let mut v = Vec::new();
    for x in [0.0, 10.0, 20.0] {
        v.extend(no(x));
        v.push(sezione(x));
    }
    v.push(Tratteggio((-4.0, -4.75), (18.75, -4.75)));
    v.extend([Linea((-4.0, -3.5), (-4.0, -6.0)), Linea((-5.5, -3.5), (-4.0, -3.5)), Linea((-5.5, -6.0), (-4.0, -6.0))]);
    v
}
fn g_termico3() -> Vec<Grafica> {
    let mut v = Vec::new();
    for x in [0.0, 10.0, 20.0] {
        v.extend([Linea((x, 0.0), (x, -3.0)), Linea((x, -3.0), (x + 1.5, -3.0)), Linea((x + 1.5, -3.0), (x + 1.5, -7.0)), Linea((x + 1.5, -7.0), (x, -7.0)), Linea((x, -7.0), (x, -10.0))]);
    }
    v.push(Rettangolo((-2.5, -2.0), (23.5, -8.0)));
    v
}
fn motore(x_centro: f64) -> [Grafica; 3] {
    [Cerchio((x_centro, -16.0), 7.0), Grafica::Testo((x_centro, -14.5), "M", 4.0), Grafica::Testo((x_centro, -19.5), "3~", 2.5)]
}
fn g_motore3() -> Vec<Grafica> {
    let mut v = vec![
        Linea((0.0, 0.0), (0.0, -10.0)),
        Linea((0.0, -10.0), (5.05, -11.05)),
        Linea((10.0, 0.0), (10.0, -9.0)),
        Linea((20.0, 0.0), (20.0, -10.0)),
        Linea((20.0, -10.0), (14.95, -11.05)),
    ];
    v.extend(motore(10.0));
    v
}
fn g_motore3_pe() -> Vec<Grafica> {
    let mut v = g_motore3();
    v.extend([Tratteggio((30.0, 0.0), (30.0, -16.0)), Tratteggio((30.0, -16.0), (17.0, -16.0))]);
    v
}
fn g_motore1() -> Vec<Grafica> {
    vec![
        Linea((0.0, 0.0), (0.0, -9.5)),
        Linea((0.0, -9.5), (2.7, -11.3)),
        Linea((10.0, 0.0), (10.0, -9.5)),
        Linea((10.0, -9.5), (7.3, -11.3)),
        Cerchio((5.0, -15.0), 6.0),
        Grafica::Testo((5.0, -13.8), "M", 3.5),
        Grafica::Testo((5.0, -18.0), "1~", 2.2),
    ]
}
fn g_resistenza_riscaldante() -> Vec<Grafica> {
    vec![Linea((0.0, 0.0), (0.0, -2.0)), Rettangolo((-2.5, -2.0), (2.5, -8.0)), Linea((-2.5, -4.0), (2.5, -4.0)), Linea((-2.5, -6.0), (2.5, -6.0)), Linea((0.0, -8.0), (0.0, -10.0))]
}
fn g_presa() -> Vec<Grafica> {
    let mut v = Vec::new();
    for x in [0.0, 10.0, 20.0] {
        v.extend([Linea((x, 0.0), (x, -5.0)), Arco((x, -6.5), 1.5, 0.0, 180.0)]);
    }
    v.push(Tratteggio((-3.0, -9.0), (23.0, -9.0)));
    v
}
fn g_trasformatore() -> Vec<Grafica> {
    vec![
        Linea((0.0, 0.0), (0.0, -6.0)),
        Linea((0.0, -6.0), (1.54, -6.0)),
        Linea((10.0, 0.0), (10.0, -6.0)),
        Linea((10.0, -6.0), (8.46, -6.0)),
        Cerchio((5.0, -8.0), 4.0),
        Cerchio((5.0, -12.0), 4.0),
        Linea((0.0, -20.0), (0.0, -14.0)),
        Linea((0.0, -14.0), (1.54, -14.0)),
        Linea((10.0, -20.0), (10.0, -14.0)),
        Linea((10.0, -14.0), (8.46, -14.0)),
    ]
}
fn g_alimentatore() -> Vec<Grafica> {
    vec![
        Linea((0.0, 0.0), (0.0, -4.0)),
        Linea((10.0, 0.0), (10.0, -4.0)),
        Rettangolo((-3.0, -4.0), (13.0, -16.0)),
        Linea((-3.0, -16.0), (13.0, -4.0)),
        Grafica::Testo((1.0, -7.0), "~", 3.0),
        Grafica::Testo((9.0, -13.0), "=", 3.0),
        Linea((0.0, -16.0), (0.0, -20.0)),
        Linea((10.0, -16.0), (10.0, -20.0)),
    ]
}
fn g_terra_protezione() -> Vec<Grafica> {
    vec![Linea((0.0, 0.0), (0.0, -4.0)), Linea((-3.0, -4.0), (3.0, -4.0)), Linea((-2.0, -5.0), (2.0, -5.0)), Linea((-1.0, -6.0), (1.0, -6.0)), Cerchio((0.0, -4.8), 4.0)]
}
fn g_terra() -> Vec<Grafica> {
    vec![Linea((0.0, 0.0), (0.0, -4.0)), Linea((-3.0, -4.0), (3.0, -4.0)), Linea((-2.0, -5.0), (2.0, -5.0)), Linea((-1.0, -6.0), (1.0, -6.0))]
}
fn g_massa() -> Vec<Grafica> {
    vec![Linea((0.0, 0.0), (0.0, -4.0)), Linea((-3.0, -4.0), (3.0, -4.0)), Linea((-3.0, -4.0), (-4.5, -6.0)), Linea((0.0, -4.0), (-1.5, -6.0)), Linea((3.0, -4.0), (1.5, -6.0))]
}
fn g_resistore() -> Vec<Grafica> {
    vec![Linea((0.0, 0.0), (0.0, -2.0)), Rettangolo((-1.5, -2.0), (1.5, -8.0)), Linea((0.0, -8.0), (0.0, -10.0))]
}
fn g_condensatore() -> Vec<Grafica> {
    vec![Linea((0.0, 0.0), (0.0, -4.4)), Linea((-3.0, -4.4), (3.0, -4.4)), Linea((-3.0, -5.6), (3.0, -5.6)), Linea((0.0, -5.6), (0.0, -10.0))]
}
fn g_diodo() -> Vec<Grafica> {
    vec![
        Linea((0.0, 0.0), (0.0, -3.5)),
        Linea((-2.5, -3.5), (2.5, -3.5)),
        Linea((2.5, -3.5), (0.0, -6.5)),
        Linea((0.0, -6.5), (-2.5, -3.5)),
        Linea((-2.5, -6.5), (2.5, -6.5)),
        Linea((0.0, -6.5), (0.0, -10.0)),
    ]
}

// ---------- libreria ----------

const POLO: &[f64] = &[0.0];
const DUE_POLI: &[f64] = &[0.0, 10.0];
const TRE_POLI: &[f64] = &[0.0, 10.0, 20.0];
const QUATTRO_POLI: &[f64] = &[0.0, 10.0, 20.0, 30.0];
const NIENTE: &[(&str, P, &str)] = &[];

/// Base dei simboli della libreria: si cambia solo ciò che serve.
const BASE: Simbolo = Simbolo {
    id: "",
    comando: "",
    blocco: "",
    etichetta: "",
    categoria: "",
    menu: &[],
    prefisso: "",
    pres: "M",
    poli: POLO,
    lunghezza: 10.0,
    morsetti: &[("1", "2")],
    extra: NIENTE,
    rif: None,
    grafica: g_no,
};

pub const CONTATTI: &str = "Contatti";
pub const BOBINE: &str = "Bobine";
pub const COMANDI: &str = "Comandi";
pub const SENSORI: &str = "Sensori";
pub const SEGNALAZIONE: &str = "Segnalazione";
pub const PROTEZIONI: &str = "Protezioni";
pub const INTERRUTTORI: &str = "Interruttori";
pub const MOTORI: &str = "Motori";
pub const ALIMENTAZIONE: &str = "Trasformatori e alimentatori";
pub const UTILIZZATORI: &str = "Utilizzatori";
pub const COMPONENTI: &str = "Componenti";
pub const TERRE: &str = "Terre e Masse";

/// La libreria, nell'ordine dei menu. L'indice è il parametro dei comandi `simbolo.*`.
pub static LIBRERIA: &[Simbolo] = &[
    // contatti (figli)
    Simbolo { id: "na", comando: "simbolo.na", blocco: "CE_NA", etichetta: "Contatto NO", categoria: CONTATTI, menu: &["Simboli", "Contatto NO"], prefisso: "KA", pres: "FA", morsetti: &[("13", "14")], grafica: g_no, ..BASE },
    Simbolo { id: "nc", comando: "simbolo.nc", blocco: "CE_NC", etichetta: "Contatto NC", categoria: CONTATTI, menu: &["Simboli", "Contatto NC"], prefisso: "KA", pres: "FC", morsetti: &[("21", "22")], grafica: g_nc, ..BASE },
    Simbolo {
        id: "sc",
        comando: "simbolo.sc",
        blocco: "CE_SC",
        etichetta: "Contatto in scambio",
        categoria: CONTATTI,
        menu: &["Simboli", "Contatto CO"],
        prefisso: "KA",
        pres: "FS",
        morsetti: &[("14", "11")],
        extra: &[("PINC1", (-5.0, 0.0), "12")],
        grafica: g_scambio,
        ..BASE
    },
    Simbolo {
        id: "naritardato",
        comando: "simbolo.naritardato",
        blocco: "CE_NA_RIT",
        etichetta: "Contatto NO ritardato",
        categoria: CONTATTI,
        menu: &["Simboli", "Altri contatti", "Contatto NO ritardato"],
        prefisso: "KT",
        pres: "FA",
        morsetti: &[("15", "18")],
        grafica: g_no_rit,
        ..BASE
    },
    Simbolo {
        id: "ncritardato",
        comando: "simbolo.ncritardato",
        blocco: "CE_NC_RIT",
        etichetta: "Contatto NC ritardato",
        categoria: CONTATTI,
        menu: &["Simboli", "Altri contatti", "Contatto NC ritardato"],
        prefisso: "KT",
        pres: "FC",
        morsetti: &[("15", "16")],
        grafica: g_nc_rit,
        ..BASE
    },
    Simbolo {
        id: "pot2",
        comando: "simbolo.pot2",
        blocco: "CE_POT2",
        etichetta: "Contatti di potenza (2 poli)",
        categoria: CONTATTI,
        menu: &["Simboli", "Altri contatti", "Contatti di potenza (2 poli)"],
        prefisso: "KM",
        pres: "FAP2",
        poli: DUE_POLI,
        morsetti: &[("1", "2"), ("3", "4")],
        grafica: g_pot2,
        ..BASE
    },
    Simbolo {
        id: "pot3",
        comando: "simbolo.pot3",
        blocco: "CE_POT3",
        etichetta: "Contatti di potenza (3 poli)",
        categoria: CONTATTI,
        menu: &["Simboli", "Contatti di potenza (3 poli)"],
        prefisso: "KM",
        pres: "FAP3",
        poli: TRE_POLI,
        morsetti: &[("1", "2"), ("3", "4"), ("5", "6")],
        grafica: g_pot3,
        ..BASE
    },
    Simbolo {
        id: "termiconc",
        comando: "simbolo.termiconc",
        blocco: "CE_TERMICO_NC",
        etichetta: "Contatto NC del relè termico",
        categoria: CONTATTI,
        menu: &["Simboli", "Contatti del Relè Termico", "Contatto NC (95-96)"],
        prefisso: "FR",
        pres: "FC",
        morsetti: &[("95", "96")],
        grafica: g_termico_nc,
        ..BASE
    },
    Simbolo {
        id: "termicono",
        comando: "simbolo.termicono",
        blocco: "CE_TERMICO_NO",
        etichetta: "Contatto NO del relè termico",
        categoria: CONTATTI,
        menu: &["Simboli", "Contatti del Relè Termico", "Contatto NO (97-98)"],
        prefisso: "FR",
        pres: "FA",
        morsetti: &[("97", "98")],
        grafica: g_termico_no,
        ..BASE
    },
    // bobine
    Simbolo {
        id: "ka",
        comando: "simbolo.ka",
        blocco: "CE_BOBINA_KA",
        etichetta: "Bobina relè",
        categoria: BOBINE,
        menu: &["Simboli", "Bobine Relè", "Bobina relè"],
        prefisso: "KA",
        morsetti: &[("A1", "A2")],
        rif: Some("CE_RIF_KA"),
        grafica: g_bobina,
        ..BASE
    },
    Simbolo {
        id: "km",
        comando: "simbolo.km",
        blocco: "CE_BOBINA_KM",
        etichetta: "Bobina contattore",
        categoria: BOBINE,
        menu: &["Simboli", "Bobine Relè", "Bobina contattore"],
        prefisso: "KM",
        morsetti: &[("A1", "A2")],
        rif: Some("CE_RIF_KM"),
        grafica: g_bobina,
        ..BASE
    },
    Simbolo {
        id: "kt",
        comando: "simbolo.kt",
        blocco: "CE_BOBINA_KT",
        etichetta: "Bobina ritardata all'eccitazione",
        categoria: BOBINE,
        menu: &["Simboli", "Bobine Relè Ritardate", "Ritardata all'eccitazione"],
        prefisso: "KT",
        morsetti: &[("A1", "A2")],
        rif: Some("CE_RIF_KT"),
        grafica: g_bobina_eccitazione,
        ..BASE
    },
    Simbolo {
        id: "ktdis",
        comando: "simbolo.ktdis",
        blocco: "CE_BOBINA_KT_DIS",
        etichetta: "Bobina ritardata alla diseccitazione",
        categoria: BOBINE,
        menu: &["Simboli", "Bobine Relè Ritardate", "Ritardata alla diseccitazione"],
        prefisso: "KT",
        morsetti: &[("A1", "A2")],
        rif: Some("CE_RIF_KT"),
        grafica: g_bobina_diseccitazione,
        ..BASE
    },
    Simbolo {
        id: "yv",
        comando: "simbolo.yv",
        blocco: "CE_ELETTROVALVOLA",
        etichetta: "Elettrovalvola",
        categoria: BOBINE,
        menu: &["Simboli", "Altre Bobine Relè", "Elettrovalvola"],
        prefisso: "YV",
        morsetti: &[("A1", "A2")],
        grafica: g_elettrovalvola,
        ..BASE
    },
    // comandi
    Simbolo {
        id: "sbno",
        comando: "simbolo.sbno",
        blocco: "CE_PULS_NO",
        etichetta: "Pulsante NO",
        categoria: COMANDI,
        menu: &["Simboli", "Pulsanti", "Pulsante NO"],
        prefisso: "SB",
        morsetti: &[("13", "14")],
        grafica: g_puls_no,
        ..BASE
    },
    Simbolo {
        id: "sbnc",
        comando: "simbolo.sbnc",
        blocco: "CE_PULS_NC",
        etichetta: "Pulsante NC",
        categoria: COMANDI,
        menu: &["Simboli", "Pulsanti", "Pulsante NC"],
        prefisso: "SB",
        morsetti: &[("11", "12")],
        grafica: g_puls_nc,
        ..BASE
    },
    Simbolo {
        id: "sbem",
        comando: "simbolo.sbem",
        blocco: "CE_EMERGENZA",
        etichetta: "Pulsante di emergenza",
        categoria: COMANDI,
        menu: &["Simboli", "Pulsanti", "Pulsante di emergenza"],
        prefisso: "SB",
        morsetti: &[("11", "12")],
        grafica: g_emergenza,
        ..BASE
    },
    Simbolo {
        id: "sh",
        comando: "simbolo.sh",
        blocco: "CE_PULS_LUM",
        etichetta: "Pulsante luminoso",
        categoria: COMANDI,
        menu: &["Simboli", "Pulsanti", "Pulsante luminoso"],
        prefisso: "SH",
        morsetti: &[("13", "14")],
        grafica: g_puls_luminoso,
        ..BASE
    },
    Simbolo {
        id: "sa",
        comando: "simbolo.sa",
        blocco: "CE_SELETTORE",
        etichetta: "Selettore",
        categoria: COMANDI,
        menu: &["Simboli", "Selettori", "Selettore"],
        prefisso: "SA",
        morsetti: &[("13", "14")],
        grafica: g_selettore,
        ..BASE
    },
    Simbolo {
        id: "sak",
        comando: "simbolo.sak",
        blocco: "CE_SELETTORE_CHIAVE",
        etichetta: "Selettore a chiave",
        categoria: COMANDI,
        menu: &["Simboli", "Selettori", "Selettore a chiave"],
        prefisso: "SA",
        morsetti: &[("13", "14")],
        grafica: g_chiave,
        ..BASE
    },
    // sensori
    Simbolo {
        id: "sqno",
        comando: "simbolo.sqno",
        blocco: "CE_FINECORSA_NO",
        etichetta: "Finecorsa NO",
        categoria: SENSORI,
        menu: &["Simboli", "Comandi", "Finecorsa NO"],
        prefisso: "SQ",
        morsetti: &[("13", "14")],
        grafica: g_finecorsa_no,
        ..BASE
    },
    Simbolo {
        id: "sqnc",
        comando: "simbolo.sqnc",
        blocco: "CE_FINECORSA_NC",
        etichetta: "Finecorsa NC",
        categoria: SENSORI,
        menu: &["Simboli", "Comandi", "Finecorsa NC"],
        prefisso: "SQ",
        morsetti: &[("11", "12")],
        grafica: g_finecorsa_nc,
        ..BASE
    },
    Simbolo {
        id: "bq",
        comando: "simbolo.bq",
        blocco: "CE_PROSSIMITA",
        etichetta: "Sensore di prossimità (3 fili)",
        categoria: SENSORI,
        menu: &["Simboli", "Sensori di Prossimità", "Sensore di prossimità"],
        prefisso: "BQ",
        morsetti: &[("BN", "BK")],
        extra: &[("PINC1", (5.0, -10.0), "BU")],
        grafica: g_prossimita,
        ..BASE
    },
    Simbolo {
        id: "bf",
        comando: "simbolo.bf",
        blocco: "CE_FOTOCELLULA",
        etichetta: "Fotocellula (3 fili)",
        categoria: SENSORI,
        menu: &["Simboli", "Fotocellule", "Fotocellula"],
        prefisso: "BF",
        morsetti: &[("BN", "BK")],
        extra: &[("PINC1", (5.0, -10.0), "BU")],
        grafica: g_fotocellula,
        ..BASE
    },
    Simbolo {
        id: "sp",
        comando: "simbolo.sp",
        blocco: "CE_PRESSOSTATO",
        etichetta: "Pressostato",
        categoria: SENSORI,
        menu: &["Simboli", "Comandi", "Pressostato"],
        prefisso: "SP",
        morsetti: &[("13", "14")],
        grafica: g_pressostato,
        ..BASE
    },
    Simbolo {
        id: "st",
        comando: "simbolo.st",
        blocco: "CE_TERMOSTATO",
        etichetta: "Termostato",
        categoria: SENSORI,
        menu: &["Simboli", "Comandi", "Termostato"],
        prefisso: "ST",
        morsetti: &[("11", "12")],
        grafica: g_termostato,
        ..BASE
    },
    // segnalazione
    Simbolo {
        id: "hl",
        comando: "simbolo.hl",
        blocco: "CE_LAMPADA",
        etichetta: "Lampada di segnalazione",
        categoria: SEGNALAZIONE,
        menu: &["Simboli", "Lampade", "Lampada di segnalazione"],
        prefisso: "HL",
        morsetti: &[("X1", "X2")],
        grafica: g_lampada,
        ..BASE
    },
    Simbolo {
        id: "ha",
        comando: "simbolo.ha",
        blocco: "CE_SIRENA",
        etichetta: "Sirena / avvisatore acustico",
        categoria: SEGNALAZIONE,
        menu: &["Simboli", "Segnalatori", "Sirena"],
        prefisso: "HA",
        morsetti: &[("1", "2")],
        grafica: g_sirena,
        ..BASE
    },
    // protezioni
    Simbolo {
        id: "fu",
        comando: "simbolo.fu",
        blocco: "CE_FUSIBILE",
        etichetta: "Fusibile",
        categoria: PROTEZIONI,
        menu: &["Protezioni", "Fusibili", "Fusibile"],
        prefisso: "FU",
        grafica: g_fusibile,
        ..BASE
    },
    Simbolo {
        id: "fu3",
        comando: "simbolo.fu3",
        blocco: "CE_FUSIBILI3",
        etichetta: "Fusibili (3 poli)",
        categoria: PROTEZIONI,
        menu: &["Protezioni", "Fusibili", "Fusibili 3 poli"],
        prefisso: "FU",
        poli: TRE_POLI,
        morsetti: &[("1", "2"), ("3", "4"), ("5", "6")],
        grafica: g_fusibili3,
        ..BASE
    },
    Simbolo {
        id: "fr3",
        comando: "simbolo.fr3",
        blocco: "CE_TERMICO3",
        etichetta: "Relè termico",
        categoria: PROTEZIONI,
        menu: &["Protezioni", "Relè Termici", "Relè termico"],
        prefisso: "FR",
        poli: TRE_POLI,
        morsetti: &[("1", "2"), ("3", "4"), ("5", "6")],
        rif: Some("CE_RIF_FR"),
        grafica: g_termico3,
        ..BASE
    },
    // interruttori
    Simbolo {
        id: "qf1",
        comando: "simbolo.qf1",
        blocco: "CE_QF1",
        etichetta: "Magnetotermico 1 polo",
        categoria: INTERRUTTORI,
        menu: &["Protezioni", "Magnetotermici", "1 polo"],
        prefisso: "QF",
        grafica: g_qf1,
        ..BASE
    },
    Simbolo {
        id: "qf2",
        comando: "simbolo.qf2",
        blocco: "CE_QF2",
        etichetta: "Magnetotermico 2 poli",
        categoria: INTERRUTTORI,
        menu: &["Protezioni", "Magnetotermici", "2 poli"],
        prefisso: "QF",
        poli: DUE_POLI,
        morsetti: &[("1", "2"), ("3", "4")],
        grafica: g_qf2,
        ..BASE
    },
    Simbolo {
        id: "qf3",
        comando: "simbolo.qf3",
        blocco: "CE_QF3",
        etichetta: "Magnetotermico 3 poli",
        categoria: INTERRUTTORI,
        menu: &["Protezioni", "Magnetotermici", "3 poli"],
        prefisso: "QF",
        poli: TRE_POLI,
        morsetti: &[("1", "2"), ("3", "4"), ("5", "6")],
        grafica: g_qf3,
        ..BASE
    },
    Simbolo {
        id: "qf4",
        comando: "simbolo.qf4",
        blocco: "CE_QF4",
        etichetta: "Magnetotermico 4 poli",
        categoria: INTERRUTTORI,
        menu: &["Protezioni", "Magnetotermici", "4 poli"],
        prefisso: "QF",
        poli: QUATTRO_POLI,
        morsetti: &[("1", "2"), ("3", "4"), ("5", "6"), ("7", "8")],
        grafica: g_qf4,
        ..BASE
    },
    Simbolo {
        id: "qd2",
        comando: "simbolo.qd2",
        blocco: "CE_QD2",
        etichetta: "Magnetotermico differenziale 2 poli",
        categoria: INTERRUTTORI,
        menu: &["Protezioni", "Magnetotermici Differenziali", "2 poli"],
        prefisso: "QF",
        poli: DUE_POLI,
        morsetti: &[("1", "2"), ("3", "4")],
        grafica: g_qd2,
        ..BASE
    },
    Simbolo {
        id: "qd4",
        comando: "simbolo.qd4",
        blocco: "CE_QD4",
        etichetta: "Magnetotermico differenziale 4 poli",
        categoria: INTERRUTTORI,
        menu: &["Protezioni", "Magnetotermici Differenziali", "4 poli"],
        prefisso: "QF",
        poli: QUATTRO_POLI,
        morsetti: &[("1", "2"), ("3", "4"), ("5", "6"), ("7", "8")],
        grafica: g_qd4,
        ..BASE
    },
    Simbolo {
        id: "qm3",
        comando: "simbolo.qm3",
        blocco: "CE_QM3",
        etichetta: "Interruttore salvamotore",
        categoria: INTERRUTTORI,
        menu: &["Protezioni", "Magnetotermici", "Salvamotore 3 poli"],
        prefisso: "QM",
        poli: TRE_POLI,
        morsetti: &[("1", "2"), ("3", "4"), ("5", "6")],
        rif: Some("CE_RIF_QM"),
        grafica: g_salvamotore,
        ..BASE
    },
    Simbolo {
        id: "qs3",
        comando: "simbolo.qs3",
        blocco: "CE_QS3",
        etichetta: "Sezionatore 3 poli",
        categoria: INTERRUTTORI,
        menu: &["Protezioni", "Sezionatori", "Sezionatore 3 poli"],
        prefisso: "QS",
        poli: TRE_POLI,
        morsetti: &[("1", "2"), ("3", "4"), ("5", "6")],
        grafica: g_sezionatore3,
        ..BASE
    },
    // motori
    Simbolo {
        id: "m3",
        comando: "simbolo.m3",
        blocco: "CE_MOTORE3",
        etichetta: "Motore trifase",
        categoria: MOTORI,
        menu: &["Simboli", "Motori", "Motore trifase"],
        prefisso: "M",
        poli: TRE_POLI,
        lunghezza: 0.0,
        morsetti: &[("U", ""), ("V", ""), ("W", "")],
        grafica: g_motore3,
        ..BASE
    },
    Simbolo {
        id: "m3pe",
        comando: "simbolo.m3pe",
        blocco: "CE_MOTORE3_PE",
        etichetta: "Motore trifase con PE",
        categoria: MOTORI,
        menu: &["Simboli", "Motori", "Motore trifase con PE"],
        prefisso: "M",
        poli: QUATTRO_POLI,
        lunghezza: 0.0,
        morsetti: &[("U", ""), ("V", ""), ("W", ""), ("PE", "")],
        grafica: g_motore3_pe,
        ..BASE
    },
    Simbolo {
        id: "m1",
        comando: "simbolo.m1",
        blocco: "CE_MOTORE1",
        etichetta: "Motore monofase",
        categoria: MOTORI,
        menu: &["Simboli", "Motori", "Motore monofase"],
        prefisso: "M",
        poli: DUE_POLI,
        lunghezza: 0.0,
        morsetti: &[("U1", ""), ("U2", "")],
        grafica: g_motore1,
        ..BASE
    },
    // trasformatori e alimentatori
    Simbolo {
        id: "tc",
        comando: "simbolo.tc",
        blocco: "CE_TRASFORMATORE",
        etichetta: "Trasformatore monofase",
        categoria: ALIMENTAZIONE,
        menu: &["Simboli", "Trasformatori", "Trasformatore monofase"],
        prefisso: "T",
        poli: DUE_POLI,
        lunghezza: 20.0,
        morsetti: &[("0", "0"), ("400", "230")],
        grafica: g_trasformatore,
        ..BASE
    },
    Simbolo {
        id: "g",
        comando: "simbolo.g",
        blocco: "CE_ALIMENTATORE",
        etichetta: "Alimentatore AC/DC",
        categoria: ALIMENTAZIONE,
        menu: &["Simboli", "Trasformatori", "Alimentatore AC/DC"],
        prefisso: "G",
        poli: DUE_POLI,
        lunghezza: 20.0,
        morsetti: &[("L", "+"), ("N", "-")],
        grafica: g_alimentatore,
        ..BASE
    },
    // utilizzatori
    Simbolo {
        id: "eh",
        comando: "simbolo.eh",
        blocco: "CE_RESISTENZA_RISC",
        etichetta: "Resistenza riscaldante",
        categoria: UTILIZZATORI,
        menu: &["Simboli", "Componenti", "Resistenza riscaldante"],
        prefisso: "EH",
        grafica: g_resistenza_riscaldante,
        ..BASE
    },
    Simbolo {
        id: "xs",
        comando: "simbolo.xs",
        blocco: "CE_PRESA",
        etichetta: "Presa di servizio",
        categoria: UTILIZZATORI,
        menu: &["Simboli", "Componenti", "Presa di servizio"],
        prefisso: "XS",
        poli: TRE_POLI,
        lunghezza: 0.0,
        morsetti: &[("L", ""), ("N", ""), ("PE", "")],
        grafica: g_presa,
        ..BASE
    },
    // componenti
    Simbolo { id: "r", comando: "simbolo.r", blocco: "CE_RESISTORE", etichetta: "Resistore", categoria: COMPONENTI, menu: &["Simboli", "Componenti", "Resistore"], prefisso: "R", grafica: g_resistore, ..BASE },
    Simbolo {
        id: "c",
        comando: "simbolo.c",
        blocco: "CE_CONDENSATORE",
        etichetta: "Condensatore",
        categoria: COMPONENTI,
        menu: &["Simboli", "Componenti", "Condensatore"],
        prefisso: "C",
        grafica: g_condensatore,
        ..BASE
    },
    Simbolo {
        id: "v",
        comando: "simbolo.v",
        blocco: "CE_DIODO",
        etichetta: "Diodo",
        categoria: COMPONENTI,
        menu: &["Simboli", "Componenti", "Diodo"],
        prefisso: "V",
        morsetti: &[("A", "K")],
        grafica: g_diodo,
        ..BASE
    },
    // terre e masse: solo grafica
    Simbolo {
        id: "pe",
        comando: "simbolo.pe",
        blocco: "CE_TERRA_PE",
        etichetta: "Terra di protezione",
        categoria: TERRE,
        menu: &["Simboli", "Terre e Masse", "Terra di protezione"],
        pres: "",
        lunghezza: 0.0,
        morsetti: &[("", "")],
        grafica: g_terra_protezione,
        ..BASE
    },
    Simbolo {
        id: "terra",
        comando: "simbolo.terra",
        blocco: "CE_TERRA",
        etichetta: "Terra",
        categoria: TERRE,
        menu: &["Simboli", "Terre e Masse", "Terra"],
        pres: "",
        lunghezza: 0.0,
        morsetti: &[("", "")],
        grafica: g_terra,
        ..BASE
    },
    Simbolo {
        id: "massa",
        comando: "simbolo.massa",
        blocco: "CE_MASSA",
        etichetta: "Massa",
        categoria: TERRE,
        menu: &["Simboli", "Terre e Masse", "Massa"],
        pres: "",
        lunghezza: 0.0,
        morsetti: &[("", "")],
        grafica: g_massa,
        ..BASE
    },
];

pub fn trova(id: &str) -> Option<&'static Simbolo> {
    LIBRERIA.iter().find(|s| s.id.eq_ignore_ascii_case(id) || s.blocco.eq_ignore_ascii_case(id) || s.comando.eq_ignore_ascii_case(id))
}

/// L'indice del simbolo `id` nella libreria.
pub fn indice(id: &str) -> Option<usize> {
    LIBRERIA.iter().position(|s| s.id.eq_ignore_ascii_case(id))
}

/// Le categorie, nell'ordine della libreria.
pub fn categorie() -> Vec<&'static str> {
    let mut v: Vec<&'static str> = Vec::new();
    for s in LIBRERIA {
        if !v.contains(&s.categoria) {
            v.push(s.categoria);
        }
    }
    v
}

/// Segmenti di un tratteggio (1.2 pieno, 0.8 vuoto).
fn tratti(a: P, b: P) -> Vec<(P, P)> {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let lung = (dx * dx + dy * dy).sqrt();
    if lung < 1e-9 {
        return Vec::new();
    }
    let (ux, uy) = (dx / lung, dy / lung);
    let mut v = Vec::new();
    let mut t = 0.0;
    while t < lung {
        let fine = (t + 1.2).min(lung);
        v.push(((a.0 + ux * t, a.1 + uy * t), (a.0 + ux * fine, a.1 + uy * fine)));
        t += 2.0;
    }
    v
}

/// Gli estremi (x min, x max, y min, y max) della grafica di un simbolo.
pub fn estensione(grafica: &[Grafica]) -> (f64, f64, f64, f64) {
    let mut e = (0.0f64, 0.0f64, 0.0f64, 0.0f64);
    let mut aggiungi = |p: P| {
        e.0 = e.0.min(p.0);
        e.1 = e.1.max(p.0);
        e.2 = e.2.min(p.1);
        e.3 = e.3.max(p.1);
    };
    for g in grafica {
        match *g {
            Linea(a, b) | Tratteggio(a, b) | Rettangolo(a, b) | Pieno(a, b) => {
                aggiungi(a);
                aggiungi(b);
            }
            Cerchio(c, r) | Arco(c, r, ..) => {
                aggiungi((c.0 - r, c.1 - r));
                aggiungi((c.0 + r, c.1 + r));
            }
            Ellisse(c, rx, ry) => {
                aggiungi((c.0 - rx, c.1 - ry));
                aggiungi((c.0 + rx, c.1 + ry));
            }
            Grafica::Testo(p, ..) => aggiungi(p),
        }
    }
    e
}

/// L'ascissa del punto più a sinistra della grafica del simbolo.
fn sinistra(sim: &Simbolo) -> f64 {
    estensione(&(sim.grafica)()).0
}

/// Il blocco del simbolo, creato se manca.
fn crea_blocco(s: &mut Session, sim: &Simbolo) -> Result<()> {
    if blocchi_definiti(s)?.iter().any(|b| b.eq_ignore_ascii_case(sim.blocco)) {
        return Ok(());
    }
    su_layer_zero(s, |s| {
        let mut h = Vec::new();
        colore(s, "7")?;
        for g in (sim.grafica)() {
            match g {
                Linea(a, b) => linea(s, &mut h, &[a, b], false)?,
                Tratteggio(a, b) => {
                    for (p, q) in tratti(a, b) {
                        linea(s, &mut h, &[p, q], false)?;
                    }
                }
                Rettangolo(a, b) => linea(s, &mut h, &[a, (b.0, a.1), b, (a.0, b.1)], true)?,
                Pieno(a, b) => {
                    let y = (a.1 + b.1) / 2.0;
                    let w = (a.1 - b.1).abs();
                    let r = esegui(s, "pline", json!({"vertices": [[a.0.min(b.0), y], [a.0.max(b.0), y]], "width": w}))?;
                    h.extend(handles(&r));
                }
                Cerchio(c, r) => h.extend(handles(&esegui(s, "circle", json!({"center": [c.0, c.1], "radius": r}))?)),
                Arco(c, r, a0, a1) => h.extend(handles(&esegui(s, "arc", json!({"center": [c.0, c.1], "radius": r, "start": a0, "end": a1}))?)),
                Ellisse(c, rx, ry) => {
                    let (major, ratio) = if rx >= ry { ([rx, 0.0], ry / rx) } else { ([0.0, ry], rx / ry) };
                    h.extend(handles(&esegui(s, "ellipse", json!({"center": [c.0, c.1], "major": major, "ratio": ratio}))?));
                }
                Grafica::Testo(p, t, alt) => testo(s, &mut h, p, t, alt, "MC")?,
            }
        }
        if sim.grafico() {
            return blocco(s, sim.blocco, h, sim.etichetta);
        }
        let attr = |s: &mut Session, h: &mut Vec<String>, tag: &str, default: &str, at: P, alt: f64, nascosto: bool| -> Result<()> {
            h.extend(handles(&esegui(s, "attdef", json!({"tag": tag, "prompt": tag, "default": default, "at": [at.0, at.1], "height": alt, "invisible": nascosto}))?));
            Ok(())
        };
        // sigla e riferimento a sinistra del simbolo, allineati a destra
        let y_nome = if sim.lunghezza > 0.0 { -4.5 } else { -16.0 };
        let x_testi = sinistra(sim) - 1.5;
        colore(s, "4")?;
        let prima = h.len();
        attr(s, &mut h, "NOME", "", (x_testi, y_nome), 2.5, false)?;
        if sim.madre() {
            for tag in ["QUADRO", "FUNZIONE_IMPIANTO", "UBICAZIONE", "FUNZIONE1", "FUNZIONE2", "CODICE", "COSTRUTTORE", "TIPO", "DESCRIZIONE"] {
                attr(s, &mut h, tag, "", (x_testi, y_nome - 3.0), 1.8, true)?;
            }
        } else {
            attr(s, &mut h, "REG", "", (x_testi, y_nome - 3.5), 2.0, false)?;
        }
        for t in &h[prima..] {
            allinea_a_destra(s, t)?;
        }
        attr(s, &mut h, "PRES", sim.pres, (0.0, 0.0), 1.0, true)?;
        colore(s, "3")?; // numeri dei morsetti in verde
        for (k, (&x, (a, b))) in sim.poli.iter().zip(sim.morsetti).enumerate() {
            let n = k + 1;
            attr(s, &mut h, &format!("PINA{n}"), a, (x + 0.8, -2.2), 1.8, false)?;
            if sim.lunghezza > 0.0 {
                attr(s, &mut h, &format!("PINB{n}"), b, (x + 0.8, -sim.lunghezza + 0.4), 1.8, false)?;
            }
        }
        for (tag, p, def) in sim.extra {
            let at = if p.1 > -0.01 { (p.0 + 0.8, p.1 - 2.2) } else { (p.0 + 0.8, p.1 + 0.4) };
            attr(s, &mut h, tag, def, at, 1.8, false)?;
        }
        blocco(s, sim.blocco, h, sim.etichetta)
    })
}

/// I punti dove la fence `da`–`a` attraversa i fili del foglio corrente, raggruppati per poli
/// del simbolo: il punto d'inserimento di ogni simbolo da posare.
pub fn punti_su_fence(s: &mut Session, sim: &Simbolo, da: P, a: P) -> Result<Vec<P>> {
    let layer = layer_corrente(s)?;
    let incroci = morsetti::attraversamenti(&fili::fili(s, &layer)?, da, a);
    let mut punti = Vec::new();
    let mut presi: Vec<P> = Vec::new();
    for p in incroci {
        if presi.iter().any(|q| (q.0 - p.0).abs() < 0.01 && (q.1 - p.1).abs() < 0.01) {
            continue;
        }
        punti.push(p);
        // i poli successivi del simbolo coprono i fili alla loro distanza
        for &x in sim.poli.iter().skip(1) {
            presi.push((p.0 + x, p.1));
        }
    }
    Ok(punti)
}

/// Inserisce il simbolo in `at` sul foglio corrente: sigla automatica per le madri, sigla
/// della madre (`madre`) per i figli; interrompe i fili sotto ai poli.
pub fn inserisci(s: &mut Session, sim: &Simbolo, at: P, p: &Value) -> Result<Value> {
    crea_blocco(s, sim)?;
    if sim.grafico() {
        let r = esegui(s, "insert", json!({"name": sim.blocco, "at": [at.0, at.1]}))?;
        let h = handles(&r).into_iter().next().ok_or_else(|| errore("simbolo non inserito"))?;
        return Ok(json!({"handle": h, "nome": "", "message": sim.etichetta}));
    }
    let foglio = fogli::corrente(s)?.map_or(0, |f| f.numero);
    let componenti = siglatura::componenti(s)?;
    let nome = if sim.madre() {
        match testo_param(p, "nome").map(|n| n.trim().to_string()).filter(|n| !n.is_empty()) {
            Some(n) => {
                // niente sigle doppie
                if componenti.iter().any(|c| c.madre() && c.nome.eq_ignore_ascii_case(&n)) {
                    return Err(errore(format!("La sigla {n} esiste già")));
                }
                n
            }
            None => {
                let prefisso = testo_param(p, "prefisso").unwrap_or_else(|| sim.prefisso.to_string());
                siglatura::nuova_sigla(&componenti, &prefisso, foglio, at)
            }
        }
    } else {
        testo_param(p, "madre").filter(|n| !n.trim().is_empty()).ok_or_else(|| errore("Indicare la sigla della madre (es. -KM08.1)"))?
    };
    if sim.lunghezza > 0.0 {
        for &x in sim.poli {
            fili::spezza(s, (at.0 + x, at.1), (at.0 + x, at.1 - sim.lunghezza))?;
        }
    }
    let mut attr = serde_json::Map::new();
    attr.insert("NOME".into(), json!(nome));
    attr.insert("PRES".into(), json!(sim.pres));
    for (param, tag) in [("quadro", "QUADRO"), ("funzione1", "FUNZIONE1"), ("funzione2", "FUNZIONE2"), ("codice", "CODICE"), ("costruttore", "COSTRUTTORE"), ("tipo", "TIPO"), ("descrizione", "DESCRIZIONE")] {
        if let Some(v) = testo_param(p, param)
            && sim.madre()
        {
            attr.insert(tag.into(), json!(v));
        }
    }
    if sim.madre() && !attr.contains_key("QUADRO")
        && let Some(q) = crate::impostazioni::quadro_corrente()
    {
        attr.insert("QUADRO".into(), json!(q));
    }
    // la madre con contatti (contattore, relè) ha il suo specchietto sotto, più le testine
    let riferimenti: Vec<&'static specchietto::Riferimento> = if sim.madre() {
        let mut v = match testo_param(p, "riferimento") {
            Some(n) => riferimenti_da(&n)?,
            None => match testo_param(p, "prefisso") {
                Some(pref) => specchietto::predefinito(&pref).into_iter().collect(),
                None => sim.rif.and_then(specchietto::trova).into_iter().collect(),
            },
        };
        v.extend(riferimenti_da(&testo_param(p, "testine").unwrap_or_default())?);
        v
    } else {
        Vec::new()
    };
    if !riferimenti.is_empty() {
        attr.insert("CODICE".into(), json!(riferimenti.iter().map(|r| r.blocco).collect::<Vec<_>>().join(";")));
    }
    let r = esegui(s, "insert", json!({"name": sim.blocco, "at": [at.0, at.1], "attribs": attr}))?;
    let h = handles(&r).into_iter().next().ok_or_else(|| errore("simbolo non inserito"))?;
    if sim.madre() {
        // funzione e ubicazione (CEI EN 81346): quelle date o le correnti
        let imp = crate::impostazioni::correnti();
        let funzione = crate::norma81346::Livello::Funzione.normalizza(&testo_param(p, "funzioneImpianto").unwrap_or(imp.funzione));
        let ubicazione = crate::norma81346::Livello::Ubicazione.normalizza(&testo_param(p, "ubicazione").unwrap_or(imp.ubicazione));
        let valori: Vec<(&str, &str)> = [("FUNZIONE_IMPIANTO", funzione.as_str()), ("UBICAZIONE", ubicazione.as_str())].into_iter().filter(|(_, v)| !v.is_empty()).collect();
        if !valori.is_empty() {
            crate::util::imposta_attributi(s, &h, &valori)?;
        }
    }
    if !riferimenti.is_empty() {
        let layer = layer_corrente(s)?;
        for (k, r) in riferimenti.iter().enumerate() {
            specchietto::inserisci_sotto(s, r, &nome, at, &layer, &riferimenti[..k])?;
        }
    }
    let (orfani, senza_posto) = siglatura::aggiorna_tutto(s)?;
    let mut msg = format!("{} {nome}", sim.etichetta);
    if !sim.madre() && orfani.iter().any(|o| o.starts_with(&nome)) {
        msg.push_str(" (madre non ancora disegnata)");
    }
    if !sim.madre() && senza_posto.iter().any(|o| o.contains(&nome)) {
        msg.push_str(" — attenzione: superata la disponibilità di contatti della madre");
    }
    Ok(json!({"handle": h, "nome": nome, "message": msg}))
}

/// I riferimenti da un elenco di nomi separati da `;` (vuoto: nessuno).
fn riferimenti_da(t: &str) -> Result<Vec<&'static specchietto::Riferimento>> {
    t.split(';').map(str::trim).filter(|x| !x.is_empty()).map(|x| specchietto::trova(x).ok_or_else(|| errore(format!("Riferimento sconosciuto: {x}")))).collect()
}

/// Inserisce il simbolo in `at`, oppure su ogni gruppo di fili attraversato dalla fence
/// `da`–`a` (un simbolo per filo, o per gruppo di fili se il simbolo ha più poli).
fn inserisci_tutti(s: &mut Session, sim: &Simbolo, p: &Value) -> Result<Value> {
    if let Some(at) = punto_param(p, "at") {
        return inserisci(s, sim, at, p);
    }
    let (Some(da), Some(a)) = (punto_param(p, "da"), punto_param(p, "a")) else { return Err(errore("Indicare il punto (at) o la fence (da, a)")) };
    let punti = punti_su_fence(s, sim, da, a)?;
    if punti.is_empty() {
        return Err(errore("La fence non attraversa nessun filo"));
    }
    let mut posati = Vec::new();
    let mut nomi = Vec::new();
    let mut extra = p.clone();
    for at in punti {
        let r = inserisci(s, sim, at, &extra)?;
        // la sigla data vale per il primo simbolo, gli altri prendono la sigla automatica
        if let Some(m) = extra.as_object_mut() {
            m.remove("nome");
        }
        nomi.push(r["nome"].as_str().unwrap_or_default().to_string());
        posati.push(r);
    }
    Ok(json!({"simboli": posati, "message": format!("{} ×{}: {}", sim.etichetta, nomi.len(), nomi.join(", "))}))
}

fn run_generico(s: &mut Session, p: &Value) -> Result<Value> {
    let id = testo_param(p, "simbolo").ok_or_else(|| errore("Indicare il simbolo (simbolo: \"km\", \"na\"…)"))?;
    let sim = trova(&id).ok_or_else(|| errore(format!("Simbolo sconosciuto: {id}. Disponibili: {}", LIBRERIA.iter().map(|s| s.id).collect::<Vec<_>>().join(", "))))?;
    in_gruppo(s, sim.etichetta, |s| inserisci_tutti(s, sim, p))
}

fn run_simbolo<const I: usize>(s: &mut Session, p: &Value) -> Result<Value> {
    let sim = LIBRERIA.get(I).ok_or_else(|| errore("simbolo non in libreria"))?;
    in_gruppo(s, sim.etichetta, |s| inserisci_tutti(s, sim, p))
}

/// Modifica un componente già disegnato (Edita Simbolo): sigla, quadro, funzioni e materiali di
/// una madre, o la madre di un contatto. Cambiando la sigla di una madre si aggiornano i suoi
/// contatti e il suo specchietto.
pub fn modifica(s: &mut Session, p: &Value) -> Result<Value> {
    let h = testo_param(p, "handle").ok_or_else(|| errore("Indicare il componente (handle)"))?;
    let tutti = siglatura::componenti(s)?;
    let c = tutti.iter().find(|c| c.blocco.handle.eq_ignore_ascii_case(&h)).ok_or_else(|| errore(format!("{h} non è un componente")))?;
    let mut valori = serde_json::Map::new();
    if c.madre() {
        for (param, tag) in [("quadro", "QUADRO"), ("funzione1", "FUNZIONE1"), ("funzione2", "FUNZIONE2"), ("costruttore", "COSTRUTTORE"), ("tipo", "TIPO"), ("descrizione", "DESCRIZIONE")] {
            if let Some(v) = testo_param(p, param) {
                valori.insert(tag.into(), json!(v));
            }
        }
        if let Some(nuovo) = testo_param(p, "nome").map(|n| n.trim().to_string()).filter(|n| !n.is_empty() && !n.eq_ignore_ascii_case(&c.nome)) {
            if tutti.iter().any(|o| o.madre() && o.nome.eq_ignore_ascii_case(&nuovo)) {
                return Err(errore(format!("La sigla {nuovo} esiste già")));
            }
            valori.insert("NOME".into(), json!(nuovo));
            for o in tutti.iter().filter(|o| !o.madre()) {
                if o.figlio() && o.nome.eq_ignore_ascii_case(&c.nome) {
                    esegui(s, "attedit", json!({"handle": o.blocco.handle, "values": {"NOME": nuovo}}))?;
                }
                if o.blocco.attributo("MADRE").eq_ignore_ascii_case(&c.nome) {
                    esegui(s, "attedit", json!({"handle": o.blocco.handle, "values": {"MADRE": nuovo}}))?;
                }
            }
        }
    } else if let Some(m) = testo_param(p, "madre").map(|n| n.trim().to_string()).filter(|n| !n.is_empty()) {
        valori.insert("NOME".into(), json!(m));
    }
    if !valori.is_empty() {
        esegui(s, "attedit", json!({"handle": h, "values": valori}))?;
    }
    // contattore e testine aggiuntive: si rifà lo specchietto
    if c.madre()
        && let Some(t) = testo_param(p, "riferimenti")
    {
        let nuovi = riferimenti_da(&t)?;
        let madre = siglatura::componenti(s)?.into_iter().find(|x| x.blocco.handle.eq_ignore_ascii_case(&h)).ok_or_else(|| errore("componente sparito"))?;
        specchietto::cambia_riferimenti(s, &madre, &nuovi)?;
    }
    siglatura::aggiorna_tutto(s)?;
    let nome = valori.get("NOME").and_then(Value::as_str).unwrap_or(&c.nome).to_string();
    Ok(json!({"nome": nome, "message": format!("Componente {nome} aggiornato")}))
}

fn run_modifica(s: &mut Session, p: &Value) -> Result<Value> {
    in_gruppo(s, "Modifica simbolo", |s| modifica(s, p))
}

// ---------- cancella e sposta mantenendo i collegamenti ----------

/// Richiude il filo interrotto da un polo: se un filo finisce in `a` (dalla parte opposta a `b`)
/// e un altro finisce in `b` (dalla parte opposta ad `a`), sullo stesso asse, diventano un filo
/// solo. Restituisce se li ha uniti.
fn richiudi(s: &mut Session, layer: &str, a: P, b: P) -> Result<bool> {
    let tutti = fili::fili(s, layer)?;
    let vicini = |p: P, q: P| (p.0 - q.0).abs() < 0.01 && (p.1 - q.1).abs() < 0.01;
    let verticale = (a.0 - b.0).abs() < 0.01;
    // l'altro estremo di un filo che finisce in `p`, sull'asse a–b e dalla parte opposta a `q`
    let fuori = |p: P, q: P| {
        tutti.iter().find_map(|f| {
            let altro = if vicini(f.a, p) {
                f.b
            } else if vicini(f.b, p) {
                f.a
            } else {
                return None;
            };
            let in_asse = if verticale { (altro.0 - p.0).abs() < 0.01 } else { (altro.1 - p.1).abs() < 0.01 };
            let opposto = if verticale { (altro.1 - p.1) * (q.1 - p.1) < 0.0 } else { (altro.0 - p.0) * (q.0 - p.0) < 0.0 };
            (in_asse && opposto).then(|| (f.handle.clone(), altro))
        })
    };
    let (Some((ha, fa)), Some((hb, fb))) = (fuori(a, b), fuori(b, a)) else { return Ok(false) };
    let col = {
        use cadcraft_engine::doc::color::Color;
        let h = cadcraft_engine::doc::Handle::parse_hex(&ha).ok_or_else(|| errore("handle non valido"))?;
        match s.doc()?.entity(h).map(|e| e.common.color) {
            Some(Color::Index(n)) => n.to_string(),
            _ => "BYLAYER".into(),
        }
    };
    esegui(s, "erase", json!({"handles": [ha, hb]}))?;
    let prima = layer_corrente(s)?;
    esegui(s, "layer.current", json!({"name": layer}))?;
    colore(s, &col)?;
    esegui(s, "setvar", json!({"name": "CELWEIGHT", "value": 30}))?;
    let mut h = Vec::new();
    linea(s, &mut h, &[fa, fb], false)?;
    esegui(s, "setvar", json!({"name": "CELWEIGHT", "value": -1}))?;
    colore(s, "BYLAYER")?;
    esegui(s, "layer.current", json!({"name": prima}))?;
    Ok(true)
}

/// Toglie i componenti dati (handle) richiudendo i fili sotto ai loro poli; con le madri se ne
/// vanno anche i loro specchietti. Restituisce le sigle tolte.
pub fn cancella(s: &mut Session, handle: &[String]) -> Result<Vec<String>> {
    let tutti = blocchi_inseriti(s, "*")?;
    let mut tolti = Vec::new();
    let mut via = Vec::new();
    for h in handle {
        let Some(b) = tutti.iter().find(|b| b.handle.eq_ignore_ascii_case(h)) else { continue };
        via.push(b.handle.clone());
        let nome = b.attributo("NOME").to_string();
        if b.attributo("PRES").eq_ignore_ascii_case("M") {
            for r in tutti.iter().filter(|r| r.attributo("PRES").eq_ignore_ascii_case("R") && r.attributo("MADRE").eq_ignore_ascii_case(&nome)) {
                via.push(r.handle.clone());
            }
        }
        if !nome.is_empty() {
            tolti.push(nome);
        }
        if let Some(sim) = trova(&b.nome).filter(|sim| sim.lunghezza > 0.0) {
            esegui(s, "erase", json!({"handles": [b.handle]}))?;
            for &x in sim.poli {
                let a = (b.pos.0 + x, b.pos.1);
                richiudi(s, &b.layer, a, (a.0, a.1 - sim.lunghezza))?;
            }
        }
    }
    let resto: Vec<&String> = via.iter().filter(|h| s.doc().ok().and_then(|d| cadcraft_engine::doc::Handle::parse_hex(h).and_then(|h| d.entity(h))).is_some()).collect();
    if !resto.is_empty() {
        esegui(s, "erase", json!({"handles": resto}))?;
    }
    siglatura::aggiorna_tutto(s)?;
    Ok(tolti)
}

/// Sposta un componente in `at`: richiude i fili dove stava e interrompe quelli dove arriva;
/// lo specchietto di una madre lo segue.
pub fn sposta(s: &mut Session, handle: &str, at: P) -> Result<()> {
    let b = blocchi_inseriti(s, "*")?.into_iter().find(|b| b.handle.eq_ignore_ascii_case(handle)).ok_or_else(|| errore(format!("{handle} non trovato")))?;
    let sim = trova(&b.nome);
    // il blocco si toglie di mezzo (spostandolo) prima di richiudere, così non resta sul filo
    esegui(s, "move", json!({"handles": [b.handle], "from": [b.pos.0, b.pos.1], "to": [at.0, at.1]}))?;
    if let Some(sim) = sim.filter(|sim| sim.lunghezza > 0.0) {
        for &x in sim.poli {
            let a = (b.pos.0 + x, b.pos.1);
            richiudi(s, &b.layer, a, (a.0, a.1 - sim.lunghezza))?;
        }
        let prima = layer_corrente(s)?;
        esegui(s, "layer.current", json!({"name": b.layer}))?;
        for &x in sim.poli {
            fili::spezza(s, (at.0 + x, at.1), (at.0 + x, at.1 - sim.lunghezza))?;
        }
        esegui(s, "layer.current", json!({"name": prima}))?;
    }
    siglatura::aggiorna_tutto(s)?;
    Ok(())
}

fn handles_param(p: &Value) -> Vec<String> {
    match p.get("handles").or_else(|| p.get("handle")) {
        Some(Value::Array(a)) => a.iter().filter_map(Value::as_str).map(str::to_string).collect(),
        Some(Value::String(h)) => vec![h.clone()],
        _ => Vec::new(),
    }
}

fn run_cancella(s: &mut Session, p: &Value) -> Result<Value> {
    let h = handles_param(p);
    let h = if h.is_empty() { s.selection().iter().map(|h| h.hex()).collect() } else { h };
    if h.is_empty() {
        return Err(errore("Selezionare i simboli da cancellare"));
    }
    in_gruppo(s, "Cancella simboli", |s| {
        let tolti = cancella(s, &h)?;
        Ok(json!({"tolti": tolti, "message": if tolti.is_empty() { "Cancellato".to_string() } else { format!("Cancellati: {}", tolti.join(", ")) }}))
    })
}

fn run_sposta(s: &mut Session, p: &Value) -> Result<Value> {
    let h = testo_param(p, "handle").ok_or_else(|| errore("Indicare il componente (handle)"))?;
    let at = punto_param(p, "at").ok_or_else(|| errore("Indicare il punto d'arrivo (at)"))?;
    in_gruppo(s, "Sposta simbolo", |s| {
        sposta(s, &h, at)?;
        Ok(json!({"message": "Simbolo spostato"}))
    })
}

/// CANCELLA SIMBOLO interattivo: gli oggetti selezionati, o da selezionare.
struct CancellaSimboli;

impl Interactive for CancellaSimboli {
    fn name(&self) -> &'static str {
        "cancellasimbolo"
    }
    fn prompt(&self, _s: &Session) -> Prompt {
        Prompt::new("Selezionare i simboli da cancellare", Accept::SELECT)
    }
    fn begin(&mut self, s: &mut Session) -> Result<Step> {
        if s.selection().is_empty() {
            return Ok(Step::Continue);
        }
        let r = s.execute("cancellasimbolo", &json!({}))?;
        if let Some(m) = r.get("message").and_then(Value::as_str) {
            s.echo(m.to_string());
        }
        Ok(Step::Done)
    }
    fn input(&mut self, s: &mut Session, i: Input) -> Result<Step> {
        match i {
            Input::Pick(h) if !h.is_empty() => {
                let h: Vec<String> = h.iter().map(|h| h.hex()).collect();
                let r = s.execute("cancellasimbolo", &json!({"handles": h}))?;
                if let Some(m) = r.get("message").and_then(Value::as_str) {
                    s.echo(m.to_string());
                }
                Ok(Step::Done)
            }
            Input::Cancel => Ok(Step::Cancel),
            Input::Enter => Ok(Step::Done),
            _ => Ok(Step::Continue),
        }
    }
}

fn fabbrica_cancella(_s: &Session) -> Result<Box<dyn Interactive>> {
    Ok(Box::new(CancellaSimboli))
}

/// SPOSTA SIMBOLO interattivo: il simbolo (selezionato o da scegliere), poi il punto d'arrivo.
struct SpostaSimbolo {
    handle: Option<(String, P, &'static Simbolo)>,
}

impl SpostaSimbolo {
    fn scegli(&mut self, s: &mut Session, h: &[String]) -> Result<()> {
        let tutti = blocchi_inseriti(s, "*")?;
        self.handle = h.iter().find_map(|h| {
            let b = tutti.iter().find(|b| b.handle.eq_ignore_ascii_case(h))?;
            Some((b.handle.clone(), b.pos, trova(&b.nome)?))
        });
        if self.handle.is_none() {
            return Err(errore("Selezionare un simbolo della libreria"));
        }
        Ok(())
    }
}

impl Interactive for SpostaSimbolo {
    fn name(&self) -> &'static str {
        "spostasimbolo"
    }
    fn prompt(&self, _s: &Session) -> Prompt {
        match &self.handle {
            None => Prompt::new("Selezionare il simbolo da spostare", Accept::SELECT),
            Some((_, p, _)) => Prompt::new("Punto d'arrivo", Accept::POINT).base(Vec2::new(p.0, p.1)),
        }
    }
    fn begin(&mut self, s: &mut Session) -> Result<Step> {
        let scelti: Vec<String> = s.selection().iter().map(|h| h.hex()).collect();
        if !scelti.is_empty() {
            let _ = self.scegli(s, &scelti);
        }
        Ok(Step::Continue)
    }
    fn input(&mut self, s: &mut Session, i: Input) -> Result<Step> {
        match i {
            Input::Cancel => Ok(Step::Cancel),
            Input::Enter => Ok(Step::Done),
            Input::Pick(h) if self.handle.is_none() => {
                let h: Vec<String> = h.iter().map(|h| h.hex()).collect();
                self.scegli(s, &h)?;
                Ok(Step::Continue)
            }
            Input::Point(q) => {
                let Some((h, ..)) = &self.handle else { return Ok(Step::Continue) };
                s.execute("spostasimbolo", &json!({"handle": h, "at": [q.x, q.y]}))?;
                Ok(Step::Done)
            }
            _ => Ok(Step::Continue),
        }
    }
    fn preview(&self, _s: &Session, cursore: Vec2) -> Vec<EntityKind> {
        self.handle.as_ref().map(|(_, _, sim)| anteprima(sim, (cursore.x, cursore.y))).unwrap_or_default()
    }
}

fn fabbrica_sposta(_s: &Session) -> Result<Box<dyn Interactive>> {
    Ok(Box::new(SpostaSimbolo { handle: None }))
}

// ---------- inserimento interattivo ----------

/// Parametri preparati dalla finestra di inserimento per la prossima posa interattiva di un
/// simbolo: (comando, parametri).
static PREPARATI: std::sync::Mutex<Option<(String, Value)>> = std::sync::Mutex::new(None);

/// I parametri (sigla, quadro, funzioni, materiali, madre…) che userà la prossima posa
/// interattiva del simbolo `comando`, come se fossero dati al comando.
pub fn prepara(comando: &str, parametri: Value) {
    if let Ok(mut g) = PREPARATI.lock() {
        *g = Some((comando.to_string(), parametri));
    }
}

fn preparati(comando: &str) -> serde_json::Map<String, Value> {
    let Ok(mut g) = PREPARATI.lock() else { return Default::default() };
    match g.take() {
        Some((c, Value::Object(m))) if c == comando => m,
        _ => Default::default(),
    }
}

struct InserisciSimbolo {
    indice: usize,
    madre: Option<String>,
    /// Parametri della finestra di inserimento; la sigla vale solo per la prima posa.
    extra: serde_json::Map<String, Value>,
    /// Inserimento su fence: il primo punto, quando scelto.
    fence: Option<Option<P>>,
}

/// La grafica del simbolo in `at`, come linee (anteprima).
pub fn anteprima(sim: &Simbolo, at: P) -> Vec<EntityKind> {
    let l = |a: P, b: P| EntityKind::Line(Line { a: Vec3::new(at.0 + a.0, at.1 + a.1, 0.0), b: Vec3::new(at.0 + b.0, at.1 + b.1, 0.0) });
    let poligono = |v: &mut Vec<EntityKind>, c: P, rx: f64, ry: f64, a0: f64, a1: f64| {
        let a1 = if a1 <= a0 { a1 + 360.0 } else { a1 };
        let n = 16;
        let pt = |k: u32| {
            let t = (a0 + (a1 - a0) * f64::from(k) / f64::from(n)).to_radians();
            (c.0 + rx * t.cos(), c.1 + ry * t.sin())
        };
        v.extend((0..n).map(|k| l(pt(k), pt(k + 1))));
    };
    let mut v = Vec::new();
    for g in (sim.grafica)() {
        match g {
            Linea(a, b) | Tratteggio(a, b) => v.push(l(a, b)),
            Rettangolo(a, b) | Pieno(a, b) => v.extend([l(a, (b.0, a.1)), l((b.0, a.1), b), l(b, (a.0, b.1)), l((a.0, b.1), a)]),
            Cerchio(c, r) => poligono(&mut v, c, r, r, 0.0, 360.0),
            Arco(c, r, a0, a1) => poligono(&mut v, c, r, r, a0, a1),
            Ellisse(c, rx, ry) => poligono(&mut v, c, rx, ry, 0.0, 360.0),
            Grafica::Testo(..) => {}
        }
    }
    v
}

impl Interactive for InserisciSimbolo {
    fn name(&self) -> &'static str {
        LIBRERIA.get(self.indice).map_or("simbolo", |s| s.comando)
    }
    fn prompt(&self, _s: &Session) -> Prompt {
        let Some(sim) = LIBRERIA.get(self.indice) else { return Prompt::new("Simbolo", Accept::POINT) };
        if !sim.madre() && !sim.grafico() && self.madre.is_none() {
            return Prompt::new(format!("Sigla della madre ({}…)", siglatura::PREFISSO_ASSOLUTO.to_string() + sim.prefisso), Accept::TEXT);
        }
        match self.fence {
            Some(None) => Prompt::new("Primo punto della fence", Accept::POINT),
            Some(Some(p)) => Prompt::new("Secondo punto della fence", Accept::POINT).base(Vec2::new(p.0, p.1)),
            None => Prompt::new(format!("{}: punto d'inserimento (Invio per finire)", sim.etichetta), Accept::POINT).kw(&["Fence"]),
        }
    }
    fn input(&mut self, s: &mut Session, i: Input) -> Result<Step> {
        let Some(sim) = LIBRERIA.get(self.indice) else { return Ok(Step::Cancel) };
        let chiede_madre = !sim.madre() && !sim.grafico() && self.madre.is_none();
        match i {
            Input::Cancel => Ok(Step::Cancel),
            Input::Enter => Ok(Step::Done),
            Input::Text(t) if chiede_madre => {
                let t = t.trim();
                if !t.is_empty() {
                    let t = if t.starts_with(siglatura::PREFISSO_ASSOLUTO) { t.to_string() } else { format!("{}{t}", siglatura::PREFISSO_ASSOLUTO) };
                    self.madre = Some(t.to_ascii_uppercase());
                }
                Ok(Step::Continue)
            }
            Input::Keyword(k) | Input::Text(k) if k.trim().to_ascii_lowercase().starts_with('f') => {
                self.fence = Some(None);
                Ok(Step::Continue)
            }
            Input::Point(q) => {
                let mut p = Value::Object(self.extra.clone());
                if let Some(m) = &self.madre {
                    p["madre"] = json!(m);
                }
                match self.fence {
                    Some(None) => {
                        self.fence = Some(Some((q.x, q.y)));
                        return Ok(Step::Continue);
                    }
                    Some(Some(da)) => {
                        p["da"] = json!([da.0, da.1]);
                        p["a"] = json!([q.x, q.y]);
                        self.fence = None;
                    }
                    None => p["at"] = json!([q.x, q.y]),
                }
                let r = s.execute(sim.comando, &p)?;
                if let Some(m) = r.get("message").and_then(Value::as_str) {
                    s.echo(m.to_string());
                }
                // le pose successive prendono la sigla automatica
                self.extra.remove("nome");
                Ok(Step::Continue)
            }
            _ => Ok(Step::Continue),
        }
    }
    fn preview(&self, _s: &Session, cursore: Vec2) -> Vec<EntityKind> {
        match (LIBRERIA.get(self.indice), self.fence) {
            (_, Some(Some(da))) => vec![EntityKind::Line(Line { a: Vec3::new(da.0, da.1, 0.0), b: Vec3::new(cursore.x, cursore.y, 0.0) })],
            (_, Some(None)) => Vec::new(),
            (Some(sim), None) if sim.madre() || sim.grafico() || self.madre.is_some() => anteprima(sim, (cursore.x, cursore.y)),
            _ => Vec::new(),
        }
    }
}

fn fabbrica<const I: usize>(_s: &Session) -> Result<Box<dyn Interactive>> {
    let comando = LIBRERIA.get(I).map_or("", |s| s.comando);
    let extra = preparati(comando);
    let madre = extra.get("madre").and_then(Value::as_str).map(str::trim).filter(|m| !m.is_empty()).map(str::to_string);
    Ok(Box::new(InserisciSimbolo { indice: I, madre, extra, fence: None }))
}

const PARAMETRI_SIMBOLO: &str = "{at: [x,y] | da: [x,y] + a: [x,y] (fence: un simbolo per filo o gruppo di fili attraversato), nome?: sigla (madri; default automatica), madre?: sigla della madre (figli), prefisso?, riferimento?: specchietto (CE_RIF_KM, CE_RIF_KA, CE_RIF_KT…; \"\" = nessuno), testine?: \"CE_RIF_T11;…\" (testine aggiuntive), quadro?, funzione1?, funzione2?, costruttore?, tipo?, descrizione?} → {handle, nome} | {simboli}";

fn spec_simbolo<const I: usize>() -> Option<CommandSpec> {
    let sim = LIBRERIA.get(I)?;
    Some(CommandSpec {
        id: sim.comando,
        label: sim.etichetta,
        menu: sim.menu,
        shortcut: None,
        aliases: &[],
        params: PARAMETRI_SIMBOLO,
        enabled: has_doc,
        run: run_simbolo::<I>,
        undoable: false,
        interactive: Some(fabbrica::<I>),
        transparent: false,
    })
}

macro_rules! specs_simboli {
    ($($i:literal)*) => { vec![$(spec_simbolo::<$i>()),*] };
}

/// Posti per i comandi dei simboli: più dei simboli in libreria (vedi il test).
const POSTI_SIMBOLI: usize = 80;

pub fn comandi() -> Vec<CommandSpec> {
    let posti: Vec<Option<CommandSpec>> = specs_simboli!(
        0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24 25 26 27 28 29 30 31 32 33 34 35 36 37 38 39
        40 41 42 43 44 45 46 47 48 49 50 51 52 53 54 55 56 57 58 59 60 61 62 63 64 65 66 67 68 69 70 71 72 73 74 75 76 77 78 79
    );
    debug_assert_eq!(posti.len(), POSTI_SIMBOLI);
    let mut v: Vec<CommandSpec> = posti.into_iter().flatten().collect();
    let spec = |id, label, menu, aliases, params, run, interactive| CommandSpec { id, label, menu, shortcut: None, aliases, params, enabled: has_doc, run, undoable: false, interactive, transparent: false };
    v.push(spec(
        "editasimbolo",
        "Modifica Simbolo (XS)",
        &["Elettrico", "Modifica Simbolo (XS)"],
        &["xs"],
        "{handle, nome?, quadro?, funzione1?, funzione2?, costruttore?, tipo?, descrizione?, riferimenti?: \"CE_RIF_KM;CE_RIF_T11\" (specchietto e testine aggiuntive) (madri) | madre? (contatti)} → {nome}",
        run_modifica as cadcraft_engine::cmd::Run,
        None,
    ));
    v.push(spec(
        "cancellasimbolo",
        "Cancella Simbolo",
        &["Elettrico", "Cancella Simbolo"],
        &[],
        "{handles?: [hex] (default la selezione)} → {tolti} — i fili interrotti si richiudono, lo specchietto di una madre se ne va con lei",
        run_cancella,
        Some(fabbrica_cancella),
    ));
    v.push(spec(
        "spostasimbolo",
        "Sposta Simbolo",
        &["Elettrico", "Sposta Simbolo"],
        &[],
        "{handle, at: [x,y]} — richiude i fili dove stava, interrompe quelli dove arriva",
        run_sposta,
        Some(fabbrica_sposta),
    ));
    v.push(spec(
        "simbolo",
        "Inserisci simbolo",
        &[],
        &[],
        "{simbolo: id della libreria (na, nc, sc, km, qf3…), at: [x,y] | da + a (fence), nome?, madre?, quadro?, funzione1?, …} → {handle, nome}",
        run_generico,
        None,
    ));
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn libreria_coerente() {
        assert!(LIBRERIA.len() <= POSTI_SIMBOLI, "servono più posti per i comandi dei simboli");
        assert_eq!(comandi().len(), LIBRERIA.len() + 4, "un comando per simbolo, più edita, cancella, sposta e simbolo generico");
        for (i, s) in LIBRERIA.iter().enumerate() {
            assert_eq!(s.poli.len(), s.morsetti.len(), "{}: un paio di morsetti per polo", s.id);
            assert_eq!(s.comando, format!("simbolo.{}", s.id));
            assert!(s.blocco.starts_with("CE_"));
            assert!(!s.categoria.is_empty() && !s.etichetta.is_empty());
            assert!(!(s.grafica)().is_empty(), "{}: senza grafica", s.id);
            assert!(LIBRERIA.iter().skip(i + 1).all(|o| o.id != s.id && o.blocco != s.blocco), "{}: doppio", s.id);
            if let Some(r) = s.rif {
                assert!(specchietto::trova(r).is_some(), "{}: specchietto {r} inesistente", s.id);
            }
        }
    }

    #[test]
    fn tratteggio() {
        assert_eq!(tratti((0.0, 0.0), (5.0, 0.0)).len(), 3);
    }

    #[test]
    fn morsetti_del_simbolo() {
        let sc = trova("sc").unwrap_or(&LIBRERIA[0]);
        assert_eq!(sc.pin((100.0, 200.0)), vec![("PINA1".into(), (100.0, 200.0)), ("PINB1".into(), (100.0, 190.0)), ("PINC1".into(), (95.0, 200.0))]);
    }
}
