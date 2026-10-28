//! Schema unifilare dei quadri di distribuzione: un circuito per colonna, appeso al sistema di
//! sbarre, con la protezione, il cavo (tanti tratti obliqui quanti i conduttori), l'utenza e sotto
//! la tabella dei dati: denominazione, sistema, tensione, potenza, corrente d'impiego, protezione
//! (curva, In, Icn, Idn), cavo (tipo, formazione, lunghezza, portata) e caduta di tensione, con la
//! verifica del coordinamento.
//!
//! Ogni circuito è un blocco `CE_UNIF_<protezione>` (l'arrivo linea `CE_UNIF_<protezione>_ARRIVO`,
//! con la protezione sopra le sbarre) con i dati negli attributi e la sua colonna della tabella;
//! a sinistra il blocco delle intestazioni. Sbarre, conduttori e utenze si ridisegnano dai dati
//! (`UNIFILARE.AGGIORNA`, anche da Aggiorna Liste e Tabelle). La protezione è un componente come
//! gli altri: sigla, codice, distinta materiali.
//!
//! Il dimensionamento è indicativo:
//! - corrente d'impiego `Ib = P / (√3 · U · cos φ)` in trifase, `P / (U · cos φ)` in monofase;
//! - la taratura normalizzata subito sopra `Ib`;
//! - la sezione minima con portata `Iz` (IEC 60364-5-52, cavi multipolari in rame isolati in
//!   EPR/XLPE a 30 °C, per la posa scelta, per il fattore di correzione `k`) tale che
//!   `Ib ≤ In ≤ Iz` (per i fusibili `In ≤ 0,9 · Iz`) e con la caduta di tensione dall'origine
//!   entro il limite: `ΔU% = 100 · b · Ib · L · (ρ/S · cos φ + λ · sin φ) / U`, con `b = √3` in
//!   trifase e `2` in monofase, `ρ = 0,0225 Ω·mm²/m`, `λ = 0,08 mΩ/m`;
//! - il potere di interruzione `Icn` almeno pari alla corrente di cortocircuito presunta
//!   dell'arrivo.
//!
//! Le potenze delle partenze, per il coefficiente di contemporaneità, danno quella dell'arrivo.

use cadcraft_engine::cmd::has_doc;
use cadcraft_engine::doc::EntityKind;
use cadcraft_engine::{CommandSpec, Result, Session};
use serde_json::{Value, json};

use crate::simboli::Grafica::{self, Arco, Cerchio, Ellisse, Linea, Pieno, Rettangolo, Tratteggio};
use crate::simboli::{self, disegna_grafica};
use crate::util::{
    Inserito, blocchi_definiti, blocchi_inseriti, blocco, colore, errore, esegui, handles, imposta_attributi, in_gruppo, inserisci_su_layer, linea, numero_param, punto_param,
    su_layer_zero, testo, testo_param,
};
use crate::{fogli, impostazioni, siglatura, uscite};

type P = (f64, f64);

/// Ordinata delle sbarre sul foglio, ascissa della prima colonna, larghezza delle colonne (mm).
pub const Y_SBARRE: f64 = 236.0;
pub const X_PRIMA: f64 = 72.5;
pub const COLONNA: f64 = 25.0;
/// Colonne in un foglio.
pub const COLONNE: usize = 14;
const INTESTAZIONE: f64 = 40.0;
/// Dalle sbarre: la tabella, le utenze, i tratti dei conduttori (sopra per l'arrivo).
const Y_TABELLA: f64 = -56.0;
const Y_UTENZA: f64 = -42.0;
const Y_CONDUTTORI: f64 = -28.0;
const Y_CONDUTTORI_ARRIVO: f64 = 20.0;
const PREFISSO: &str = "CE_UNIF_";
const BLOCCO_INTESTAZIONE: &str = "CE_UNIF_INTESTAZIONE";
const PREFISSO_UTENZA: &str = "CE_UNIF_U_";
const PREFISSO_CONDUTTORI: &str = "CE_UNIF_COND";
const SPESSORE_SBARRE: f64 = 0.8;
const TESTO: f64 = 1.8;

/// Le righe della tabella: attributo, intestazione, altezza della riga (mm).
pub const RIGHE: [(&str, &str, f64); 18] = [
    ("CIRCUITO", "Circuito", 6.0),
    ("FUNZIONE1", "Denominazione", 40.0),
    ("SISTEMA", "Sistema", 5.0),
    ("TENSIONE", "Tensione [V]", 5.0),
    ("POTENZA", "Potenza [kW]", 5.0),
    ("COSFI", "Cos fi", 5.0),
    ("IB", "Ib [A]", 5.0),
    ("CURVA", "Curva", 5.0),
    ("IN", "In [A]", 5.0),
    ("ICN", "Icn [kA]", 5.0),
    ("IDN", "Idn [mA]", 5.0),
    ("CLASSE", "Tipo differenziale", 5.0),
    ("CAVO", "Cavo", 5.0),
    ("FORMAZIONE", "Formazione", 5.0),
    ("LUNGHEZZA", "Lunghezza [m]", 5.0),
    ("IZ", "Iz [A]", 5.0),
    ("CDT", "c.d.t. totale [%]", 5.0),
    ("VERIFICA", "Verifica", 5.0),
];

/// Attributi nascosti: quelli delle madri e i dati che non vanno in tabella.
const NASCOSTI: [&str; 16] = ["QUADRO", "FUNZIONE_IMPIANTO", "UBICAZIONE", "CODICE", "COSTRUTTORE", "TIPO", "DESCRIZIONE", "DISSIPAZIONE", "UTENZA", "SEZIONE", "POSA", "KCORR", "ICC", "KC", "AUTOIN", "AUTOS"];

// ---------- calcolo ----------

/// Tarature normalizzate degli interruttori e dei fusibili (A).
pub const TARATURE: [f64; 22] = [1.0, 2.0, 3.0, 4.0, 6.0, 10.0, 13.0, 16.0, 20.0, 25.0, 32.0, 40.0, 50.0, 63.0, 80.0, 100.0, 125.0, 160.0, 200.0, 250.0, 315.0, 400.0];
/// Sezioni dei cavi (mm²).
pub const SEZIONI: [f64; 12] = [1.5, 2.5, 4.0, 6.0, 10.0, 16.0, 25.0, 35.0, 50.0, 70.0, 95.0, 120.0];
/// Portate (A) dei cavi multipolari in rame isolati in EPR/XLPE a 30 °C (IEC 60364-5-52,
/// tabelle B.52.4, B.52.5 e B.52.12): per posa, per conduttori caricati (2, 3), per sezione.
const PORTATE: [[[f64; 12]; 2]; 3] = [
    [[22.0, 30.0, 40.0, 51.0, 69.0, 91.0, 119.0, 146.0, 175.0, 221.0, 265.0, 305.0], [19.5, 26.0, 35.0, 44.0, 60.0, 80.0, 105.0, 128.0, 154.0, 194.0, 233.0, 268.0]],
    [[24.0, 33.0, 45.0, 58.0, 80.0, 107.0, 138.0, 171.0, 209.0, 269.0, 328.0, 382.0], [22.0, 30.0, 40.0, 52.0, 71.0, 96.0, 119.0, 147.0, 179.0, 229.0, 278.0, 322.0]],
    [[26.0, 36.0, 49.0, 63.0, 86.0, 115.0, 149.0, 185.0, 225.0, 289.0, 352.0, 410.0], [23.0, 32.0, 42.0, 54.0, 75.0, 100.0, 127.0, 158.0, 192.0, 246.0, 298.0, 346.0]],
];
/// Resistività del rame alla temperatura di servizio (Ω·mm²/m) e reattanza dei cavi (Ω/m).
const RHO: f64 = 0.0225;
const LAMBDA: f64 = 0.08e-3;
const EPS: f64 = 1e-9;
/// Nel dimensionamento automatico l'arrivo si prende questa parte della caduta ammessa; il
/// resto è delle partenze.
const QUOTA_ARRIVO: f64 = 0.25;
/// Taratura di una riserva senza taratura data (A).
const TARATURA_RISERVA: f64 = 16.0;

/// La protezione di un circuito.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Protezione {
    Magnetotermico,
    MagnetotermicoDifferenziale,
    Differenziale,
    Sezionatore,
    Fusibili,
}

impl Protezione {
    pub const TUTTE: [Protezione; 5] = [Protezione::Magnetotermico, Protezione::MagnetotermicoDifferenziale, Protezione::Differenziale, Protezione::Sezionatore, Protezione::Fusibili];

    pub fn codice(self) -> &'static str {
        match self {
            Protezione::Magnetotermico => "mt",
            Protezione::MagnetotermicoDifferenziale => "mtd",
            Protezione::Differenziale => "id",
            Protezione::Sezionatore => "sez",
            Protezione::Fusibili => "fus",
        }
    }

    pub fn descrizione(self) -> &'static str {
        match self {
            Protezione::Magnetotermico => "Magnetotermico",
            Protezione::MagnetotermicoDifferenziale => "Magnetotermico differenziale",
            Protezione::Differenziale => "Interruttore differenziale",
            Protezione::Sezionatore => "Interruttore di manovra-sezionatore",
            Protezione::Fusibili => "Fusibili",
        }
    }

    pub fn da_testo(t: &str) -> Option<Self> {
        Self::TUTTE.into_iter().find(|p| p.codice().eq_ignore_ascii_case(t.trim()))
    }

    fn blocco(self, arrivo: bool) -> String {
        format!("{PREFISSO}{}{}", self.codice().to_ascii_uppercase(), if arrivo { "_ARRIVO" } else { "" })
    }

    /// La protezione e se è l'arrivo, dal nome del blocco.
    fn da_blocco(nome: &str) -> Option<(Self, bool)> {
        let n = nome.to_ascii_uppercase();
        let resto = n.strip_prefix(PREFISSO)?;
        let (codice, arrivo) = match resto.strip_suffix("_ARRIVO") {
            Some(c) => (c, true),
            None => (resto, false),
        };
        Self::TUTTE.into_iter().find(|p| p.codice().eq_ignore_ascii_case(codice)).map(|p| (p, arrivo))
    }

    fn prefisso(self) -> &'static str {
        match self {
            Protezione::Sezionatore => "QS",
            Protezione::Fusibili => "FU",
            _ => "QF",
        }
    }

    /// Ha gli sganciatori di sovracorrente con curva e potere di interruzione.
    pub fn magnetotermica(self) -> bool {
        matches!(self, Protezione::Magnetotermico | Protezione::MagnetotermicoDifferenziale)
    }

    pub fn differenziale(self) -> bool {
        matches!(self, Protezione::MagnetotermicoDifferenziale | Protezione::Differenziale)
    }

    /// Il simbolo unipolare, dal morsetto in alto a 10 mm più in basso.
    fn grafica(self) -> Vec<Grafica> {
        // il comando manuale a sinistra, legato alla lama
        let manovra = [Tratteggio((-4.0, -4.75), (-1.25, -4.75)), Linea((-4.0, -3.5), (-4.0, -6.0)), Linea((-5.5, -3.5), (-4.0, -3.5)), Linea((-5.5, -6.0), (-4.0, -6.0))];
        match self {
            Protezione::Magnetotermico => simboli::interruttore(1),
            Protezione::MagnetotermicoDifferenziale => simboli::differenziale(1),
            Protezione::Differenziale => {
                let mut v = simboli::no(0.0).to_vec();
                v.extend(manovra);
                v.extend([Ellisse((0.0, -8.6), 2.5, 0.9), Tratteggio((-2.5, -8.6), (-5.0, -8.6)), Linea((-5.0, -8.6), (-5.0, -6.0))]);
                v
            }
            Protezione::Sezionatore => {
                let mut v = simboli::no(0.0).to_vec();
                v.push(simboli::sezione(0.0));
                v.extend(manovra);
                v
            }
            Protezione::Fusibili => simboli::fusibile(0.0).to_vec(),
        }
    }
}

/// Che cosa alimenta un circuito.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Utenza {
    Prese,
    Luce,
    Motore,
    Quadro,
    Riscaldamento,
    Generica,
    Riserva,
}

impl Utenza {
    pub const TUTTE: [Utenza; 7] = [Utenza::Prese, Utenza::Luce, Utenza::Motore, Utenza::Quadro, Utenza::Riscaldamento, Utenza::Generica, Utenza::Riserva];

    pub fn codice(self) -> &'static str {
        match self {
            Utenza::Prese => "prese",
            Utenza::Luce => "luce",
            Utenza::Motore => "motore",
            Utenza::Quadro => "quadro",
            Utenza::Riscaldamento => "riscaldamento",
            Utenza::Generica => "generica",
            Utenza::Riserva => "riserva",
        }
    }

    pub fn descrizione(self) -> &'static str {
        match self {
            Utenza::Prese => "Prese",
            Utenza::Luce => "Illuminazione",
            Utenza::Motore => "Motore",
            Utenza::Quadro => "Sottoquadro",
            Utenza::Riscaldamento => "Riscaldamento",
            Utenza::Generica => "Utenza generica",
            Utenza::Riserva => "Riserva",
        }
    }

    pub fn da_testo(t: &str) -> Option<Self> {
        Self::TUTTE.into_iter().find(|u| u.codice().eq_ignore_ascii_case(t.trim()))
    }

    /// Sezione minima: 2,5 mm² per le prese, 1,5 per il resto.
    fn sezione_minima(self) -> f64 {
        if self == Utenza::Prese { 2.5 } else { 1.5 }
    }

    pub fn curva(self) -> &'static str {
        if self == Utenza::Motore { "D" } else { "C" }
    }

    /// Corrente differenziale proposta (mA): 30 per prese e luce, 300 per il resto.
    pub fn idn(self) -> f64 {
        if matches!(self, Utenza::Prese | Utenza::Luce) { 30.0 } else { 300.0 }
    }

    pub fn cosfi(self) -> f64 {
        match self {
            Utenza::Motore => 0.85,
            Utenza::Riscaldamento => 1.0,
            _ => 0.9,
        }
    }

    fn blocco(self) -> String {
        format!("{PREFISSO_UTENZA}{}", self.codice().to_ascii_uppercase())
    }

    /// Il simbolo dell'utenza, appeso al cavo nel punto d'inserimento.
    fn grafica(self) -> Vec<Grafica> {
        let k = 2.0_f64.sqrt();
        match self {
            Utenza::Prese => vec![Linea((0.0, 0.0), (0.0, -4.0)), Linea((-3.5, -4.0), (3.5, -4.0)), Arco((0.0, -4.0), 3.5, 180.0, 360.0)],
            Utenza::Luce => vec![Linea((0.0, 0.0), (0.0, -3.0)), Cerchio((0.0, -7.0), 4.0), Linea((-2.0 * k, -7.0 - 2.0 * k), (2.0 * k, -7.0 + 2.0 * k)), Linea((-2.0 * k, -7.0 + 2.0 * k), (2.0 * k, -7.0 - 2.0 * k))],
            Utenza::Motore => vec![Linea((0.0, 0.0), (0.0, -2.0)), Cerchio((0.0, -7.0), 5.0), Grafica::Testo((0.0, -7.0), "M", 3.5)],
            Utenza::Quadro => vec![Linea((0.0, 0.0), (0.0, -2.0)), Rettangolo((-5.0, -2.0), (5.0, -11.0)), Linea((-5.0, -11.0), (5.0, -2.0))],
            Utenza::Riscaldamento => {
                let mut v = vec![Linea((0.0, 0.0), (0.0, -2.0)), Rettangolo((-2.5, -2.0), (2.5, -11.0))];
                v.extend([-4.5, -6.5, -8.5].map(|y| Linea((-2.5, y), (2.5, y))));
                v
            }
            Utenza::Generica => vec![Linea((0.0, 0.0), (0.0, -8.0)), Linea((-1.5, -5.5), (0.0, -8.0)), Linea((1.5, -5.5), (0.0, -8.0))],
            Utenza::Riserva => vec![Linea((0.0, 0.0), (0.0, -3.0)), Linea((-2.5, -3.0), (2.5, -3.0)), Pieno((-0.4, -3.4), (0.4, -3.6))],
        }
    }
}

/// Il sistema di distribuzione di un circuito: fasi, neutro, conduttore di protezione.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sistema {
    pub fasi: u8,
    pub neutro: bool,
    pub pe: bool,
}

impl Sistema {
    pub const PROPOSTI: [&'static str; 6] = ["3F+N+PE", "3F+PE", "3F+N", "3F", "F+N+PE", "F+N"];

    pub fn da_testo(t: &str) -> Option<Self> {
        let t = t.trim().to_ascii_uppercase().replace(' ', "");
        let mut parti = t.split('+');
        let f = parti.next()?;
        let fasi = match f.strip_suffix('F').or_else(|| f.strip_suffix('L'))? {
            "" | "1" => 1,
            "2" => 2,
            "3" => 3,
            _ => return None,
        };
        let mut s = Sistema { fasi, neutro: false, pe: false };
        for p in parti {
            match p {
                "N" => s.neutro = true,
                "PE" => s.pe = true,
                _ => return None,
            }
        }
        Some(s)
    }

    pub fn testo(self) -> String {
        let mut t = if self.fasi == 1 { "F".to_string() } else { format!("{}F", self.fasi) };
        if self.neutro {
            t.push_str("+N");
        }
        if self.pe {
            t.push_str("+PE");
        }
        t
    }

    pub fn trifase(self) -> bool {
        self.fasi == 3
    }

    pub fn conduttori(self) -> usize {
        usize::from(self.fasi) + usize::from(self.neutro) + usize::from(self.pe)
    }

    /// Conduttori caricati per la portata: tre in trifase (neutro scarico), due in monofase.
    fn caricati(self) -> usize {
        if self.trifase() { 3 } else { 2 }
    }

    /// Tensione proposta: 400 V tra le fasi, 230 V tra fase e neutro.
    pub fn tensione(self) -> f64 {
        if self.fasi == 1 { 230.0 } else { 400.0 }
    }
}

/// Come è posato il cavo (metodi di installazione di IEC 60364-5-52).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Posa {
    B2 = 0,
    C = 1,
    E = 2,
}

impl Posa {
    pub const TUTTE: [Posa; 3] = [Posa::B2, Posa::C, Posa::E];

    pub fn codice(self) -> &'static str {
        match self {
            Posa::B2 => "B2",
            Posa::C => "C",
            Posa::E => "E",
        }
    }

    pub fn descrizione(self) -> &'static str {
        match self {
            Posa::B2 => "B2 – in tubo o canale su parete",
            Posa::C => "C – fissato su parete",
            Posa::E => "E – su passerella forata o in aria",
        }
    }

    pub fn da_testo(t: &str) -> Option<Self> {
        Self::TUTTE.into_iter().find(|p| p.codice().eq_ignore_ascii_case(t.trim()))
    }
}

/// Corrente d'impiego (A) di una potenza `kw` alla tensione `u` (tra le fasi in trifase).
pub fn corrente(kw: f64, u: f64, cosfi: f64, trifase: bool) -> f64 {
    if u <= 0.0 || cosfi <= 0.0 {
        return 0.0;
    }
    let k = if trifase { 3f64.sqrt() } else { 1.0 };
    kw * 1000.0 / (k * u * cosfi)
}

/// Portata (A) di un cavo della sezione data; `None` se la sezione non è in tabella.
pub fn portata(sezione: f64, posa: Posa, caricati: usize) -> Option<f64> {
    let i = SEZIONI.iter().position(|s| (s - sezione).abs() < 1e-6)?;
    PORTATE.get(posa as usize)?.get(usize::from(caricati >= 3))?.get(i).copied()
}

/// Caduta di tensione (%) di un cavo lungo `l` m percorso da `ib` A.
pub fn caduta(ib: f64, l: f64, sezione: f64, cosfi: f64, u: f64, trifase: bool) -> f64 {
    if u <= 0.0 || sezione <= 0.0 {
        return 0.0;
    }
    let sin = (1.0 - cosfi * cosfi).max(0.0).sqrt();
    let b = if trifase { 3f64.sqrt() } else { 2.0 };
    100.0 * b * ib * l * (RHO / sezione * cosfi + LAMBDA * sin) / u
}

/// Un numero con al più `decimali` cifre, senza zeri finali, con la virgola.
pub fn numero(x: f64, decimali: usize) -> String {
    let t = format!("{x:.decimali$}");
    let t = if t.contains('.') { t.trim_end_matches('0').trim_end_matches('.').to_string() } else { t };
    if t == "-0" { "0".into() } else { t.replace('.', ",") }
}

/// Un numero scritto con la virgola o con il punto.
fn leggi(t: &str) -> Option<f64> {
    t.trim().replace(',', ".").parse::<f64>().ok().filter(|x| x.is_finite())
}

/// Quello che arriva da monte a una partenza: caduta di tensione dell'arrivo, cortocircuito
/// presunto (kA, 0 se non noto), caduta massima ammessa (%).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Monte {
    pub cdt: f64,
    pub icc: f64,
    pub cdt_max: f64,
}

impl Default for Monte {
    fn default() -> Self {
        Monte { cdt: 0.0, icc: 0.0, cdt_max: 4.0 }
    }
}

/// I valori calcolati di un circuito e i problemi trovati.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Calcolo {
    pub ib: f64,
    pub iz: Option<f64>,
    /// Caduta di tensione del cavo e dall'origine dell'impianto (%).
    pub cdt: f64,
    pub cdt_totale: f64,
    pub problemi: Vec<&'static str>,
}

impl Calcolo {
    pub fn ok(&self) -> bool {
        self.problemi.is_empty()
    }
    pub fn verifica(&self) -> String {
        if self.ok() { "OK".into() } else { self.problemi.join(" ") }
    }
}

/// Un circuito dello schema unifilare (una colonna).
#[derive(Clone, Debug, PartialEq)]
pub struct Circuito {
    pub handle: String,
    pub layer: String,
    pub pos: P,
    /// L'arrivo linea del quadro (protezione generale sopra le sbarre).
    pub arrivo: bool,
    pub protezione: Protezione,
    pub sigla: String,
    pub circuito: String,
    pub denominazione: String,
    pub denominazione2: String,
    pub quadro: String,
    pub utenza: Utenza,
    pub sistema: Sistema,
    pub tensione: f64,
    pub potenza: f64,
    pub cosfi: f64,
    pub curva: String,
    /// Corrente nominale (taratura) della protezione (A).
    pub nominale: f64,
    pub icn: f64,
    pub idn: f64,
    pub classe: String,
    pub cavo: String,
    pub sezione: f64,
    pub posa: Posa,
    /// Fattore di correzione della portata (raggruppamento, temperatura).
    pub k: f64,
    pub lunghezza: f64,
    /// Solo per l'arrivo: cortocircuito presunto (kA), contemporaneità, caduta massima (%).
    pub icc: f64,
    pub kc: f64,
    pub cdt_max: f64,
    /// Il materiale della protezione (attributi `TIPO`, `COSTRUTTORE`, `DESCRIZIONE`, come per
    /// gli altri componenti): la descrizione vuota o scritta dal programma si rifà dai dati.
    pub codice: String,
    pub costruttore: String,
    pub descrizione_materiale: String,
    /// Taratura e sezione scelte dal programma: si rifanno a ogni ricalcolo.
    pub auto_in: bool,
    pub auto_sezione: bool,
}

impl Circuito {
    /// Un circuito nuovo con i valori proposti per l'utenza.
    pub fn nuovo(utenza: Utenza, arrivo: bool) -> Self {
        let sistema = if arrivo || matches!(utenza, Utenza::Motore | Utenza::Quadro) { Sistema { fasi: 3, neutro: !matches!(utenza, Utenza::Motore), pe: true } } else { Sistema { fasi: 1, neutro: true, pe: true } };
        let protezione = if arrivo { Protezione::Magnetotermico } else { Protezione::MagnetotermicoDifferenziale };
        Circuito {
            handle: String::new(),
            layer: String::new(),
            pos: (0.0, 0.0),
            arrivo,
            protezione,
            sigla: String::new(),
            circuito: String::new(),
            denominazione: if arrivo { "Generale".into() } else { utenza.descrizione().to_string() },
            denominazione2: String::new(),
            quadro: String::new(),
            utenza,
            sistema,
            tensione: sistema.tensione(),
            potenza: 0.0,
            cosfi: utenza.cosfi(),
            curva: utenza.curva().into(),
            nominale: 0.0,
            icn: 6.0,
            idn: if arrivo { 300.0 } else { utenza.idn() },
            classe: "A".into(),
            cavo: "FG16OR16".into(),
            sezione: 0.0,
            posa: Posa::C,
            k: 1.0,
            lunghezza: if arrivo { 10.0 } else { 20.0 },
            icc: 6.0,
            kc: 1.0,
            cdt_max: 4.0,
            codice: String::new(),
            costruttore: String::new(),
            descrizione_materiale: String::new(),
            auto_in: false,
            auto_sezione: false,
        }
    }

    fn riserva(&self) -> bool {
        !self.arrivo && self.utenza == Utenza::Riserva
    }

    pub fn ib(&self) -> f64 {
        corrente(self.potenza, self.tensione, self.cosfi, self.sistema.trifase())
    }

    pub fn calcola(&self, monte: &Monte) -> Calcolo {
        let ib = self.ib();
        if self.riserva() {
            return Calcolo { ib, iz: None, cdt: 0.0, cdt_totale: monte.cdt, problemi: Vec::new() };
        }
        let iz = portata(self.sezione, self.posa, self.sistema.caricati()).map(|i| i * self.k);
        let cdt = caduta(ib, self.lunghezza, self.sezione, self.cosfi, self.tensione, self.sistema.trifase());
        let cdt_totale = monte.cdt + cdt;
        let mut problemi = Vec::new();
        if self.nominale + EPS < ib {
            problemi.push("Ib>In");
        }
        match iz {
            None => problemi.push("Iz?"),
            Some(iz) => match self.protezione {
                Protezione::Fusibili if self.nominale > 0.9 * iz + EPS => problemi.push("In>0,9Iz"),
                Protezione::Magnetotermico | Protezione::MagnetotermicoDifferenziale if self.nominale > iz + EPS => problemi.push("In>Iz"),
                Protezione::Differenziale | Protezione::Sezionatore if ib > iz + EPS => problemi.push("Ib>Iz"),
                _ => {}
            },
        }
        if cdt_totale > monte.cdt_max + EPS {
            problemi.push("c.d.t.");
        }
        if self.protezione.magnetotermica() && monte.icc > 0.0 && self.icn + EPS < monte.icc {
            problemi.push("Icn<Icc");
        }
        Calcolo { ib, iz, cdt, cdt_totale, problemi }
    }

    /// Sceglie la taratura (la normalizzata subito sopra Ib) e la sezione (la più piccola che
    /// rispetta portata e caduta di tensione; la più grande se nessuna basta).
    pub fn dimensiona(&mut self, monte: &Monte, taratura: bool, sezione: bool) {
        if self.riserva() {
            if self.nominale <= 0.0 {
                self.nominale = TARATURA_RISERVA;
            }
            return;
        }
        let monte = if self.arrivo { &Monte { cdt_max: monte.cdt_max * QUOTA_ARRIVO, ..*monte } } else { monte };
        let ib = self.ib();
        if taratura {
            self.nominale = TARATURE.iter().copied().find(|t| *t + EPS >= ib).unwrap_or(TARATURE[TARATURE.len() - 1]);
        }
        if sezione {
            let minima = if self.arrivo { 1.5 } else { self.utenza.sezione_minima() };
            let va = |s: f64| {
                let mut c = self.clone();
                c.sezione = s;
                !c.calcola(monte).problemi.iter().any(|p| matches!(*p, "In>Iz" | "In>0,9Iz" | "Ib>Iz" | "c.d.t." | "Iz?"))
            };
            self.sezione = SEZIONI.iter().copied().filter(|s| *s + EPS >= minima).find(|s| va(*s)).unwrap_or(SEZIONI[SEZIONI.len() - 1]);
        }
    }

    /// La formazione del cavo: `3G2,5` con il PE nel cavo, `4x10` senza.
    pub fn formazione(&self) -> String {
        if self.riserva() || self.sezione <= 0.0 {
            return "-".into();
        }
        format!("{}{}{}", self.sistema.conduttori(), if self.sistema.pe { "G" } else { "x" }, numero(self.sezione, 1))
    }

    /// Dal blocco inserito.
    fn da_inserito(b: &Inserito) -> Option<Self> {
        let (protezione, arrivo) = Protezione::da_blocco(&b.nome)?;
        let utenza = Utenza::da_testo(b.attributo("UTENZA")).unwrap_or(Utenza::Generica);
        let mut c = Circuito::nuovo(utenza, arrivo);
        let n = |tag: &str| leggi(b.attributo(tag));
        let t = |tag: &str| {
            let v = b.attributo(tag).trim();
            if v == "-" { String::new() } else { v.to_string() }
        };
        c.handle.clone_from(&b.handle);
        c.layer.clone_from(&b.layer);
        c.pos = b.pos;
        c.protezione = protezione;
        c.sigla = t("NOME");
        c.circuito = t("CIRCUITO");
        c.denominazione = t("FUNZIONE1");
        c.denominazione2 = t("FUNZIONE2");
        c.quadro = t("QUADRO");
        if let Some(s) = Sistema::da_testo(b.attributo("SISTEMA")) {
            c.sistema = s;
        }
        c.tensione = n("TENSIONE").filter(|x| *x > 0.0).unwrap_or(c.sistema.tensione());
        c.potenza = n("POTENZA").unwrap_or(0.0).max(0.0);
        c.cosfi = n("COSFI").filter(|x| *x > 0.0 && *x <= 1.0).unwrap_or(c.cosfi);
        c.curva = t("CURVA");
        c.nominale = n("IN").unwrap_or(0.0);
        c.icn = n("ICN").unwrap_or(0.0);
        c.idn = n("IDN").unwrap_or(c.idn);
        if !t("CLASSE").is_empty() {
            c.classe = t("CLASSE");
        }
        c.cavo = t("CAVO");
        c.sezione = n("SEZIONE").unwrap_or(0.0);
        c.posa = Posa::da_testo(b.attributo("POSA")).unwrap_or(Posa::C);
        c.k = n("KCORR").filter(|x| *x > 0.0).unwrap_or(1.0);
        c.lunghezza = n("LUNGHEZZA").unwrap_or(0.0).max(0.0);
        c.icc = n("ICC").unwrap_or(0.0).max(0.0);
        c.kc = n("KC").filter(|x| *x > 0.0).unwrap_or(1.0);
        c.cdt_max = n("CDTMAX").filter(|x| *x > 0.0).unwrap_or(4.0);
        c.codice = t("TIPO");
        c.costruttore = t("COSTRUTTORE");
        c.descrizione_materiale = t("DESCRIZIONE");
        c.auto_in = b.attributo("AUTOIN").trim() == "1";
        c.auto_sezione = b.attributo("AUTOS").trim() == "1";
        Some(c)
    }

    /// Gli attributi del blocco con i dati e i valori calcolati.
    fn valori(&self, r: &Calcolo) -> Vec<(&'static str, String)> {
        let trattino = |si: bool, v: String| if si { v } else { "-".into() };
        let riserva = self.riserva();
        let mut v = vec![
            ("NOME", self.sigla.clone()),
            ("CIRCUITO", self.circuito.clone()),
            ("FUNZIONE1", self.denominazione.clone()),
            ("FUNZIONE2", self.denominazione2.clone()),
            ("QUADRO", self.quadro.clone()),
            ("UTENZA", if self.arrivo { String::new() } else { self.utenza.codice().into() }),
            ("SISTEMA", self.sistema.testo()),
            ("TENSIONE", numero(self.tensione, 0)),
            ("POTENZA", numero(self.potenza, 2)),
            ("COSFI", numero(self.cosfi, 2)),
            ("IB", numero(r.ib, 1)),
            ("CURVA", trattino(self.protezione.magnetotermica(), self.curva.clone())),
            ("IN", numero(self.nominale, 1)),
            ("ICN", trattino(self.protezione.magnetotermica(), numero(self.icn, 1))),
            ("IDN", trattino(self.protezione.differenziale(), numero(self.idn, 0))),
            ("CLASSE", trattino(self.protezione.differenziale(), self.classe.clone())),
            ("CAVO", trattino(!riserva, self.cavo.clone())),
            ("FORMAZIONE", self.formazione()),
            ("LUNGHEZZA", trattino(!riserva, numero(self.lunghezza, 1))),
            ("IZ", r.iz.map_or_else(|| "-".into(), |i| numero(i, 1))),
            ("CDT", trattino(!riserva, numero(r.cdt_totale, 2))),
            ("VERIFICA", trattino(!riserva, r.verifica())),
            ("SEZIONE", numero(self.sezione, 1)),
            ("POSA", self.posa.codice().into()),
            ("KCORR", numero(self.k, 2)),
            ("TIPO", self.codice.clone()),
            ("COSTRUTTORE", self.costruttore.clone()),
            ("DESCRIZIONE", self.descrizione_del_materiale()),
            ("AUTOIN", if self.auto_in { "1" } else { "" }.into()),
            ("AUTOS", if self.auto_sezione { "1" } else { "" }.into()),
        ];
        if self.arrivo {
            v.extend([
                ("ICC", numero(self.icc, 1)),
                ("KC", numero(self.kc, 2)),
                ("CDTMAX", numero(self.cdt_max, 1)),
                ("SBARRE", format!("{} {} V", self.sistema.testo(), numero(self.tensione, 0))),
            ]);
        }
        v
    }

    /// La descrizione del materiale: quella data, o `Magnetotermico differenziale 2P C16 Idn 30
    /// mA A` dai dati.
    pub fn descrizione_del_materiale(&self) -> String {
        let d = self.descrizione_materiale.trim();
        if d.is_empty() || Protezione::TUTTE.iter().any(|p| d.starts_with(p.descrizione())) { self.descrizione(self.protezione.descrizione()) } else { d.to_string() }
    }

    /// La protezione in breve, per la tabella dei circuiti: `Magnetot. diff. 2P C16 30 mA A`.
    pub fn sintesi(&self) -> String {
        let nome = match self.protezione {
            Protezione::Magnetotermico => "Magnetotermico",
            Protezione::MagnetotermicoDifferenziale => "Magnetot. diff.",
            Protezione::Differenziale => "Differenziale",
            Protezione::Sezionatore => "Sezionatore",
            Protezione::Fusibili => "Fusibili",
        };
        self.descrizione(nome).replace(" Idn ", " ")
    }

    fn descrizione(&self, nome: &str) -> String {
        let poli = usize::from(self.sistema.fasi) + usize::from(self.sistema.neutro);
        let mut d = format!("{nome} {poli}P");
        if self.protezione.magnetotermica() {
            d.push_str(&format!(" {}{}", self.curva, numero(self.nominale, 1)));
        } else {
            d.push_str(&format!(" {} A", numero(self.nominale, 1)));
        }
        if self.protezione.differenziale() {
            d.push_str(&format!(" Idn {} mA {}", numero(self.idn, 0), self.classe));
        }
        d
    }

    /// Applica i parametri di un comando; errore sui valori non validi.
    fn applica(&mut self, p: &Value) -> Result<()> {
        if let Some(t) = testo_param(p, "protezione") {
            self.protezione = Protezione::da_testo(&t).ok_or_else(|| errore(format!("Protezione sconosciuta: {t} (mt, mtd, id, sez, fus)")))?;
        }
        if let Some(t) = testo_param(p, "sistema") {
            self.sistema = Sistema::da_testo(&t).ok_or_else(|| errore(format!("Sistema non valido: {t} (es. 3F+N+PE, F+N+PE)")))?;
        }
        if let Some(t) = testo_param(p, "posa") {
            self.posa = Posa::da_testo(&t).ok_or_else(|| errore(format!("Posa sconosciuta: {t} (B2, C, E)")))?;
        }
        for (k, campo) in [
            ("nome", &mut self.sigla),
            ("circuito", &mut self.circuito),
            ("denominazione", &mut self.denominazione),
            ("denominazione2", &mut self.denominazione2),
            ("curva", &mut self.curva),
            ("classe", &mut self.classe),
            ("cavo", &mut self.cavo),
            ("codice", &mut self.codice),
            ("descrizione", &mut self.descrizione_materiale),
            ("costruttore", &mut self.costruttore),
        ] {
            if let Some(v) = testo_param(p, k) {
                *campo = v.trim().to_string();
            }
        }
        let n = |k: &str| -> Result<Option<f64>> {
            match p.get(k) {
                None | Some(Value::Null) => Ok(None),
                Some(Value::Number(x)) => Ok(x.as_f64().filter(|x| x.is_finite())),
                Some(Value::String(t)) if t.trim().is_empty() => Ok(None),
                Some(Value::String(t)) => leggi(t).map(Some).ok_or_else(|| errore(format!("{k}: {t} non è un numero"))),
                Some(v) => Err(errore(format!("{k}: {v} non è un numero"))),
            }
        };
        let positivo = |k: &str, x: f64, max: f64| if x >= 0.0 && x <= max { Ok(x) } else { Err(errore(format!("{k}: {} fuori dai limiti (0–{})", numero(x, 3), numero(max, 0)))) };
        if let Some(x) = n("tensione")? {
            self.tensione = positivo("tensione", x, 1000.0)?;
        }
        if let Some(x) = n("potenza")? {
            self.potenza = positivo("potenza", x, 10000.0)?;
        }
        if let Some(x) = n("cosfi")? {
            if x <= 0.0 || x > 1.0 {
                return Err(errore("cosfi: tra 0 e 1"));
            }
            self.cosfi = x;
        }
        if let Some(x) = n("in")? {
            self.nominale = positivo("in", x, 6300.0)?;
        }
        if let Some(x) = n("icn")? {
            self.icn = positivo("icn", x, 200.0)?;
        }
        if let Some(x) = n("idn")? {
            self.idn = positivo("idn", x, 3000.0)?;
        }
        if let Some(x) = n("sezione")? {
            self.sezione = positivo("sezione", x, 1000.0)?;
        }
        if let Some(x) = n("lunghezza")? {
            self.lunghezza = positivo("lunghezza", x, 10000.0)?;
        }
        if let Some(x) = n("k")? {
            if x <= 0.0 || x > 2.0 {
                return Err(errore("k: tra 0 e 2"));
            }
            self.k = x;
        }
        if let Some(x) = n("icc")? {
            self.icc = positivo("icc", x, 200.0)?;
        }
        if let Some(x) = n("kc")? {
            if x <= 0.0 || x > 1.0 {
                return Err(errore("kc: tra 0 e 1"));
            }
            self.kc = x;
        }
        if let Some(x) = n("cdtMax")? {
            self.cdt_max = positivo("cdtMax", x, 20.0)?;
        }
        Ok(())
    }

    fn json(&self, r: &Calcolo) -> Value {
        json!({
            "handle": self.handle,
            "foglio": fogli::numero_da_layer(&self.layer).unwrap_or(0),
            "colonna": colonna_di(self.pos.0) + 1,
            "arrivo": self.arrivo,
            "quadro": self.quadro,
            "sigla": self.sigla,
            "circuito": self.circuito,
            "denominazione": self.denominazione,
            "denominazione2": self.denominazione2,
            "utenza": self.utenza.codice(),
            "protezione": self.protezione.codice(),
            "sistema": self.sistema.testo(),
            "tensione": self.tensione,
            "potenza": self.potenza,
            "cosfi": self.cosfi,
            "curva": self.curva,
            "in": self.nominale,
            "icn": self.icn,
            "idn": self.idn,
            "classe": self.classe,
            "cavo": self.cavo,
            "sezione": self.sezione,
            "formazione": self.formazione(),
            "posa": self.posa.codice(),
            "k": self.k,
            "lunghezza": self.lunghezza,
            "icc": self.icc,
            "kc": self.kc,
            "cdtMax": self.cdt_max,
            "codice": self.codice,
            "costruttore": self.costruttore,
            "autoIn": self.auto_in,
            "autoSezione": self.auto_sezione,
            "ib": (r.ib * 100.0).round() / 100.0,
            "iz": r.iz,
            "cdt": (r.cdt * 1000.0).round() / 1000.0,
            "cdtTotale": (r.cdt_totale * 1000.0).round() / 1000.0,
            "verifica": r.verifica(),
            "ok": r.ok(),
        })
    }
}

/// La colonna (da 0) di un'ascissa del foglio.
fn colonna_di(x: f64) -> usize {
    ((x - X_PRIMA) / COLONNA).round().clamp(0.0, (COLONNE - 1) as f64) as usize
}

fn stesso_quadro(a: &str, b: &str) -> bool {
    a.trim().eq_ignore_ascii_case(b.trim())
}

/// I circuiti del disegno, per quadro, foglio e colonna (l'arrivo per primo).
pub fn circuiti(s: &mut Session) -> Result<Vec<Circuito>> {
    let mut v: Vec<Circuito> = blocchi_inseriti(s, &format!("{PREFISSO}*"))?.iter().filter_map(Circuito::da_inserito).collect();
    v.sort_by(|a, b| {
        let chiave = |c: &Circuito| (c.quadro.to_ascii_uppercase(), !c.arrivo, fogli::numero_da_layer(&c.layer).unwrap_or(0));
        chiave(a).cmp(&chiave(b)).then(a.pos.0.total_cmp(&b.pos.0))
    });
    Ok(v)
}

/// Calcola tutti i circuiti: la potenza degli arrivi dalle loro partenze (per la
/// contemporaneità), le partenze con la caduta e il cortocircuito del loro arrivo. Taratura e
/// sezione automatiche si rifanno: prima gli arrivi, poi le partenze.
pub fn calcola_tutti(v: &mut [Circuito]) -> Vec<Calcolo> {
    let mut monti: Vec<(String, Monte)> = Vec::new();
    for i in 0..v.len() {
        if !v[i].arrivo {
            continue;
        }
        let (mut p, mut q) = (0.0, 0.0);
        let mut partenze = 0;
        for c in v.iter().filter(|c| !c.arrivo && stesso_quadro(&c.quadro, &v[i].quadro)) {
            let tan = (1.0 - c.cosfi * c.cosfi).max(0.0).sqrt() / c.cosfi;
            p += c.potenza;
            q += c.potenza * tan;
            partenze += 1;
        }
        let a = &mut v[i];
        if partenze > 0 {
            a.potenza = p * a.kc;
            if p > 0.0 {
                a.cosfi = p / p.hypot(q);
            }
        }
        let origine = Monte { cdt: 0.0, icc: a.icc, cdt_max: a.cdt_max };
        let (auto_in, auto_sezione) = (a.auto_in, a.auto_sezione);
        a.dimensiona(&origine, auto_in, auto_sezione);
        let r = a.calcola(&origine);
        if !monti.iter().any(|(q, _)| stesso_quadro(q, &a.quadro)) {
            monti.push((a.quadro.clone(), Monte { cdt: r.cdt_totale, icc: a.icc, cdt_max: a.cdt_max }));
        }
    }
    for c in v.iter_mut().filter(|c| !c.arrivo) {
        let (auto_in, auto_sezione) = (c.auto_in, c.auto_sezione);
        c.dimensiona(&monte_di(&monti, &c.quadro), auto_in, auto_sezione);
    }
    v.iter()
        .map(|c| {
            if c.arrivo {
                c.calcola(&Monte { cdt: 0.0, icc: c.icc, cdt_max: c.cdt_max })
            } else {
                c.calcola(&monte_di(&monti, &c.quadro))
            }
        })
        .collect()
}

fn monte_di(monti: &[(String, Monte)], quadro: &str) -> Monte {
    monti.iter().find(|(q, _)| stesso_quadro(q, quadro)).map(|(_, m)| *m).unwrap_or_default()
}

// ---------- blocchi ----------

fn attdef(s: &mut Session, h: &mut Vec<String>, tag: &str, at: P, alt: f64, nascosto: bool, default: &str) -> Result<String> {
    let r = esegui(s, "attdef", json!({"tag": tag, "prompt": tag, "default": default, "at": [at.0, at.1], "height": alt, "invisible": nascosto}))?;
    let nuovi = handles(&r);
    let primo = nuovi.first().cloned().unwrap_or_default();
    h.extend(nuovi);
    Ok(primo)
}

/// Il fondo della tabella (dalle sbarre).
fn fondo_tabella() -> f64 {
    Y_TABELLA - RIGHE.iter().map(|r| r.2).sum::<f64>()
}

/// Le righe della tabella tra `x0` e `x1`.
fn griglia(s: &mut Session, h: &mut Vec<String>, x0: f64, x1: f64) -> Result<()> {
    let mut y = Y_TABELLA;
    linea(s, h, &[(x0, y), (x1, y)], false)?;
    for (_, _, alt) in RIGHE {
        y -= alt;
        linea(s, h, &[(x0, y), (x1, y)], false)?;
    }
    linea(s, h, &[(x0, Y_TABELLA), (x0, y)], false)?;
    linea(s, h, &[(x1, Y_TABELLA), (x1, y)], false)
}

/// Il blocco di una colonna: protezione, cavo fino all'utenza (l'arrivo: dalla rete alle
/// sbarre) e la colonna della tabella con gli attributi.
fn crea_colonna(s: &mut Session, prot: Protezione, arrivo: bool) -> Result<String> {
    let nome = prot.blocco(arrivo);
    if blocchi_definiti(s)?.iter().any(|b| b.eq_ignore_ascii_case(&nome)) {
        return Ok(nome);
    }
    su_layer_zero(s, |s| {
        let mut h = Vec::new();
        colore(s, "7")?;
        // il punto sulle sbarre
        disegna_grafica(s, &mut h, &[Pieno((-0.6, 0.0), (0.6, -1.2))], (0.0, 0.6))?;
        let y_sigla = if arrivo {
            // dalla rete: la freccia verso il basso
            linea(s, &mut h, &[(0.0, 30.0), (0.0, 14.0)], false)?;
            linea(s, &mut h, &[(-1.2, 28.5), (0.0, 26.0), (1.2, 28.5)], false)?;
            disegna_grafica(s, &mut h, &prot.grafica(), (0.0, 14.0))?;
            linea(s, &mut h, &[(0.0, 4.0), (0.0, 0.0)], false)?;
            8.0
        } else {
            linea(s, &mut h, &[(0.0, 0.0), (0.0, -5.0)], false)?;
            disegna_grafica(s, &mut h, &prot.grafica(), (0.0, -5.0))?;
            linea(s, &mut h, &[(0.0, -15.0), (0.0, Y_UTENZA)], false)?;
            -11.0
        };
        griglia(s, &mut h, -COLONNA / 2.0, COLONNA / 2.0)?;
        colore(s, "4")?;
        attdef(s, &mut h, "NOME", (2.0, y_sigla), 2.2, false, "")?;
        if arrivo {
            attdef(s, &mut h, "SBARRE", (COLONNA / 2.0 + 2.0, 1.4), 2.2, false, "")?;
        }
        colore(s, "7")?;
        let mut y = Y_TABELLA;
        for (tag, _, alt) in RIGHE {
            let basso = y - alt;
            if tag == "FUNZIONE1" {
                // la denominazione in verticale, la seconda lingua accanto
                for (t, x, a) in [("FUNZIONE1", 0.6, 2.0), ("FUNZIONE2", 4.6, TESTO)] {
                    let at = attdef(s, &mut h, t, (x, basso + 1.5), a, false, "")?;
                    crate::util::ruota_testo(s, &at, 90.0, cadcraft_engine::doc::HAlign::Left)?;
                }
            } else {
                let at = attdef(s, &mut h, tag, (0.0, basso + (alt - TESTO) / 2.0), TESTO, false, "")?;
                crate::util::allinea(s, &at, cadcraft_engine::doc::HAlign::Center)?;
            }
            y = basso;
        }
        let fondo = fondo_tabella();
        attdef(s, &mut h, "PRES", (0.0, fondo - 3.0), 1.0, true, "M")?;
        for tag in NASCOSTI.iter().chain(if arrivo { &["CDTMAX"][..] } else { &[][..] }) {
            attdef(s, &mut h, tag, (0.0, fondo - 3.0), 1.0, true, "")?;
        }
        let descrizione = format!("Unifilare: {}{}", prot.descrizione(), if arrivo { " (arrivo linea)" } else { "" });
        blocco(s, &nome, h, &descrizione)?;
        Ok(nome.clone())
    })
}

/// Il blocco delle intestazioni della tabella (a sinistra della prima colonna).
fn crea_intestazione(s: &mut Session) -> Result<()> {
    if blocchi_definiti(s)?.iter().any(|b| b.eq_ignore_ascii_case(BLOCCO_INTESTAZIONE)) {
        return Ok(());
    }
    su_layer_zero(s, |s| {
        let mut h = Vec::new();
        colore(s, "7")?;
        griglia(s, &mut h, 0.0, INTESTAZIONE)?;
        let mut y = Y_TABELLA;
        for (_, titolo, alt) in RIGHE {
            let basso = y - alt;
            let ty = if alt > 10.0 { basso + alt / 2.0 - TESTO / 2.0 } else { basso + (alt - TESTO) / 2.0 };
            testo(s, &mut h, (2.0, ty), titolo, TESTO, "L")?;
            y = basso;
        }
        blocco(s, BLOCCO_INTESTAZIONE, h, "Unifilare: intestazioni della tabella")
    })
}

/// Un blocco di sola grafica, creato se manca.
fn crea_grafica(s: &mut Session, nome: &str, grafica: &[Grafica], descrizione: &str) -> Result<()> {
    if blocchi_definiti(s)?.iter().any(|b| b.eq_ignore_ascii_case(nome)) {
        return Ok(());
    }
    su_layer_zero(s, |s| {
        let mut h = Vec::new();
        colore(s, "7")?;
        disegna_grafica(s, &mut h, grafica, (0.0, 0.0))?;
        blocco(s, nome, h, descrizione)
    })
}

/// I tratti obliqui dei conduttori, centrati sul punto d'inserimento.
fn grafica_conduttori(n: usize) -> Vec<Grafica> {
    (0..n)
        .map(|k| {
            let y = (k as f64 - (n as f64 - 1.0) / 2.0) * 1.2;
            Linea((-1.5, y - 1.0), (1.5, y + 1.0))
        })
        .collect()
}

// ---------- disegno ----------

/// Ridisegna conduttori, utenze, sbarre e intestazioni di un foglio dai suoi circuiti.
fn ridisegna(s: &mut Session, layer: &str, v: &[&Circuito]) -> Result<()> {
    let righe: Vec<f64> = {
        let mut r: Vec<f64> = Vec::new();
        for c in v {
            if !r.iter().any(|y| (y - c.pos.1).abs() < 0.01) {
                r.push(c.pos.1);
            }
        }
        r
    };
    let via: Vec<String> = s
        .doc()?
        .model
        .iter()
        .filter(|e| e.common.layer.eq_ignore_ascii_case(layer))
        .filter(|e| match &e.kind {
            EntityKind::Insert(i) => {
                let n = i.block.to_ascii_uppercase();
                n.starts_with(PREFISSO_UTENZA) || n.starts_with(PREFISSO_CONDUTTORI) || (n == BLOCCO_INTESTAZIONE && !righe.iter().any(|y| (y - i.insert.y).abs() < 0.01))
            }
            // le sbarre: orizzontali, del loro spessore, all'altezza dei circuiti (di default senza circuiti)
            EntityKind::LwPolyline(p) => {
                let su = |y: f64| if righe.is_empty() { (y - Y_SBARRE).abs() < 0.01 } else { righe.iter().any(|r| (y - r).abs() < 0.01) };
                (p.const_width - SPESSORE_SBARRE).abs() < 1e-6 && p.vertices.len() == 2 && p.vertices.iter().all(|q| su(q.p.y))
            }
            _ => false,
        })
        .map(|e| e.handle.hex())
        .collect();
    if !via.is_empty() {
        esegui(s, "erase", json!({"handles": via}))?;
    }
    let intestazioni = blocchi_inseriti(s, BLOCCO_INTESTAZIONE)?;
    for c in v {
        let n = c.sistema.conduttori();
        let cond = format!("{PREFISSO_CONDUTTORI}{n}");
        crea_grafica(s, &cond, &grafica_conduttori(n), "Unifilare: numero dei conduttori")?;
        let y = if c.arrivo { Y_CONDUTTORI_ARRIVO } else { Y_CONDUTTORI };
        inserisci_su_layer(s, &cond, (c.pos.0, c.pos.1 + y), &json!({}), layer)?;
        if !c.arrivo {
            crea_grafica(s, &c.utenza.blocco(), &c.utenza.grafica(), &format!("Unifilare: {}", c.utenza.descrizione()))?;
            inserisci_su_layer(s, &c.utenza.blocco(), (c.pos.0, c.pos.1 + Y_UTENZA), &json!({}), layer)?;
        }
    }
    for y in righe {
        let xs: Vec<f64> = v.iter().filter(|c| (c.pos.1 - y).abs() < 0.01).map(|c| c.pos.0).collect();
        let (x0, x1) = (xs.iter().copied().fold(f64::INFINITY, f64::min), xs.iter().copied().fold(f64::NEG_INFINITY, f64::max));
        // le sbarre: dall'arrivo (o da prima della prima partenza) a dopo l'ultima
        let arrivo_a_sinistra = v.iter().any(|c| c.arrivo && (c.pos.1 - y).abs() < 0.01 && (c.pos.0 - x0).abs() < 0.01);
        let inizio = if arrivo_a_sinistra { x0 } else { x0 - COLONNA / 2.0 + 2.0 };
        colore(s, "7")?;
        let r = esegui(s, "pline", json!({"vertices": [[inizio, y], [x1 + COLONNA / 2.0 - 2.0, y]], "width": SPESSORE_SBARRE}));
        colore(s, "BYLAYER")?;
        esegui(s, "laymch", json!({"handles": handles(&r?), "layer": layer}))?;
        // le intestazioni a sinistra della prima colonna
        let x = x0 - COLONNA / 2.0 - INTESTAZIONE;
        let presente = intestazioni.iter().any(|b| b.layer.eq_ignore_ascii_case(layer) && (b.pos.1 - y).abs() < 0.01 && (b.pos.0 - x).abs() < 0.01);
        if !presente {
            let vecchie: Vec<String> = intestazioni.iter().filter(|b| b.layer.eq_ignore_ascii_case(layer) && (b.pos.1 - y).abs() < 0.01).map(|b| b.handle.clone()).collect();
            if !vecchie.is_empty() {
                esegui(s, "erase", json!({"handles": vecchie}))?;
            }
            crea_intestazione(s)?;
            inserisci_su_layer(s, BLOCCO_INTESTAZIONE, (x, y), &json!({}), layer)?;
        }
    }
    Ok(())
}

/// Ricalcola tutti i circuiti, scrive i valori e ridisegna i fogli. Restituisce i circuiti.
pub fn aggiorna(s: &mut Session) -> Result<usize> {
    let mut v = circuiti(s)?;
    let r = calcola_tutti(&mut v);
    for (c, r) in v.iter().zip(&r) {
        let valori = c.valori(r);
        let coppie: Vec<(&str, &str)> = valori.iter().map(|(t, x)| (*t, x.as_str())).collect();
        imposta_attributi(s, &c.handle, &coppie)?;
    }
    let mut layer: Vec<String> = Vec::new();
    for c in &v {
        if !layer.iter().any(|l| l.eq_ignore_ascii_case(&c.layer)) {
            layer.push(c.layer.clone());
        }
    }
    for l in layer {
        let sul_foglio: Vec<&Circuito> = v.iter().filter(|c| c.layer.eq_ignore_ascii_case(&l)).collect();
        ridisegna(s, &l, &sul_foglio)?;
    }
    // i fogli rimasti senza circuiti perdono intestazioni e sbarre
    for b in blocchi_inseriti(s, BLOCCO_INTESTAZIONE)? {
        if !v.iter().any(|c| c.layer.eq_ignore_ascii_case(&b.layer)) {
            ridisegna(s, &b.layer, &[])?;
        }
    }
    Ok(v.len())
}

fn quadro_param(p: &Value) -> String {
    testo_param(p, "quadro").filter(|q| !q.trim().is_empty()).or_else(impostazioni::quadro_corrente).unwrap_or_default()
}

/// Il parametro `k` c'è e non è vuoto.
fn dato(p: &Value, k: &str) -> bool {
    p.get(k).is_some_and(|v| !v.is_null() && v.as_str().is_none_or(|t| !t.trim().is_empty()))
}

/// Il prossimo numero di circuito del quadro (`C1`, `C2`…).
fn prossimo_circuito(v: &[Circuito], quadro: &str) -> String {
    let n = v.iter().filter(|c| stesso_quadro(&c.quadro, quadro)).filter_map(|c| c.circuito.trim().strip_prefix('C').and_then(|n| n.parse::<u32>().ok())).max().unwrap_or(0);
    format!("C{}", n + 1)
}

/// Disegna un circuito (o l'arrivo linea) nella prossima colonna libera del foglio corrente.
fn nuovo(s: &mut Session, p: &Value, arrivo: bool) -> Result<Value> {
    let foglio = fogli::corrente(s)?.ok_or_else(|| errore("Nessun foglio: crea prima un foglio"))?;
    let tutti = circuiti(s)?;
    // senza quadro: quello dei circuiti già sul foglio, poi il quadro corrente
    let quadro = testo_param(p, "quadro")
        .filter(|q| !q.trim().is_empty())
        .or_else(|| tutti.iter().find(|c| c.layer.eq_ignore_ascii_case(&foglio.layer)).map(|c| c.quadro.clone()))
        .unwrap_or_else(|| quadro_param(p));
    let occupate: Vec<usize> = tutti.iter().filter(|c| c.layer.eq_ignore_ascii_case(&foglio.layer)).map(|c| colonna_di(c.pos.0)).collect();
    let ha_arrivo = tutti.iter().any(|c| c.arrivo && stesso_quadro(&c.quadro, &quadro));
    if arrivo && ha_arrivo {
        return Err(errore(format!("Il quadro {quadro} ha già l'arrivo linea")));
    }
    let colonna = match (numero_param(p, "colonna"), punto_param(p, "at")) {
        (Some(k), _) => (k as usize).checked_sub(1).ok_or_else(|| errore("Le colonne partono da 1"))?,
        (None, Some(at)) => colonna_di(at.0),
        // le partenze dopo l'ultima colonna; sul primo foglio si lascia la prima all'arrivo
        _ if arrivo => 0,
        _ => occupate.iter().max().map_or(usize::from(!ha_arrivo), |m| m + 1),
    };
    if colonna >= COLONNE {
        return Err(errore(format!("Il foglio {} è pieno ({COLONNE} colonne): continua sul foglio successivo", foglio.numero)));
    }
    if occupate.contains(&colonna) {
        return Err(errore(format!("La colonna {} del foglio {} è già occupata", colonna + 1, foglio.numero)));
    }
    let utenza = match testo_param(p, "utenza") {
        Some(t) => Utenza::da_testo(&t).ok_or_else(|| errore(format!("Utenza sconosciuta: {t} ({})", Utenza::TUTTE.map(Utenza::codice).join(", "))))?,
        None => Utenza::Generica,
    };
    let mut c = Circuito::nuovo(utenza, arrivo);
    // i valori proposti seguono il sistema dato
    if let Some(sis) = testo_param(p, "sistema").as_deref().and_then(Sistema::da_testo) {
        c.tensione = sis.tensione();
    }
    c.applica(p)?;
    c.quadro.clone_from(&quadro);
    let at = (X_PRIMA + COLONNA * colonna as f64, Y_SBARRE);
    let componenti = siglatura::componenti(s)?;
    if c.sigla.is_empty() {
        c.sigla = siglatura::nuova_sigla(&componenti, c.protezione.prefisso(), foglio.numero, at);
    } else if componenti.iter().any(|x| x.madre() && x.nome.eq_ignore_ascii_case(&c.sigla) && stesso_quadro(x.blocco.attributo("QUADRO"), &quadro)) {
        return Err(errore(format!("La sigla {} esiste già", c.sigla)));
    }
    if c.circuito.is_empty() {
        c.circuito = if arrivo { "GEN".into() } else { prossimo_circuito(&tutti, &quadro) };
    }
    // dimensionato con quello che arriva da monte, se l'arrivo c'è già
    let monte = if arrivo {
        Monte { cdt: 0.0, icc: c.icc, cdt_max: c.cdt_max }
    } else {
        let mut con = tutti.clone();
        let r = calcola_tutti(&mut con);
        con.iter().zip(&r).find(|(a, _)| a.arrivo && stesso_quadro(&a.quadro, &quadro)).map(|(a, r)| Monte { cdt: r.cdt_totale, icc: a.icc, cdt_max: a.cdt_max }).unwrap_or_default()
    };
    c.auto_in = !dato(p, "in");
    c.auto_sezione = !dato(p, "sezione");
    c.dimensiona(&monte, c.auto_in, c.auto_sezione);
    let nome = crea_colonna(s, c.protezione, arrivo)?;
    c.handle = inserisci_su_layer(s, &nome, at, &json!({}), &foglio.layer)?;
    c.layer.clone_from(&foglio.layer);
    c.pos = at;
    let r = c.calcola(&monte);
    let valori = c.valori(&r);
    let coppie: Vec<(&str, &str)> = valori.iter().map(|(t, x)| (*t, x.as_str())).collect();
    imposta_attributi(s, &c.handle, &coppie)?;
    aggiorna(s)?;
    risultato(s, &c.handle, if arrivo { "Arrivo linea" } else { "Circuito" })
}

/// Il circuito `handle` ricalcolato, come risultato di un comando.
fn risultato(s: &mut Session, handle: &str, cosa: &str) -> Result<Value> {
    let mut v = circuiti(s)?;
    let r = calcola_tutti(&mut v);
    let (c, r) = v.iter().zip(&r).find(|(c, _)| c.handle.eq_ignore_ascii_case(handle)).ok_or_else(|| errore("circuito non trovato"))?;
    let mut j = c.json(r);
    j["message"] = json!(format!(
        "{cosa} {} {}: Ib {} A, {} {} A, cavo {} {} (Iz {} A), c.d.t. {} % — {}",
        c.circuito,
        c.sigla,
        numero(r.ib, 1),
        if c.protezione.magnetotermica() { c.curva.as_str() } else { "In" },
        numero(c.nominale, 1),
        c.cavo,
        c.formazione(),
        r.iz.map_or_else(|| "-".into(), |i| numero(i, 1)),
        numero(r.cdt_totale, 2),
        r.verifica()
    ));
    Ok(j)
}

/// Modifica un circuito: i dati dati, la protezione (cambia il blocco), il ridimensionamento.
fn modifica(s: &mut Session, p: &Value) -> Result<Value> {
    let h = testo_param(p, "handle").ok_or_else(|| errore("Indicare il circuito (handle)"))?;
    let tutti = circuiti(s)?;
    let mut c = tutti.iter().find(|c| c.handle.eq_ignore_ascii_case(&h)).cloned().ok_or_else(|| errore(format!("{h} non è un circuito dello schema unifilare")))?;
    let prima = c.protezione;
    if let Some(n) = testo_param(p, "nome").map(|n| n.trim().to_string()).filter(|n| !n.is_empty() && !n.eq_ignore_ascii_case(&c.sigla))
        && siglatura::componenti(s)?.iter().any(|x| x.madre() && x.nome.eq_ignore_ascii_case(&n) && stesso_quadro(x.blocco.attributo("QUADRO"), &c.quadro))
    {
        return Err(errore(format!("La sigla {n} esiste già")));
    }
    if let Some(t) = testo_param(p, "utenza") {
        c.utenza = Utenza::da_testo(&t).ok_or_else(|| errore(format!("Utenza sconosciuta: {t}")))?;
    }
    c.applica(p)?;
    // taratura e sezione date restano quelle; ridimensionare le rende di nuovo automatiche
    let rifai = p.get("dimensiona").and_then(Value::as_bool) == Some(true);
    if dato(p, "in") {
        c.auto_in = false;
    } else if rifai {
        c.auto_in = true;
    }
    if dato(p, "sezione") {
        c.auto_sezione = false;
    } else if rifai {
        c.auto_sezione = true;
    }
    if c.protezione != prima {
        // un'altra protezione è un altro blocco, nello stesso posto
        let nome = crea_colonna(s, c.protezione, c.arrivo)?;
        let nuovo = inserisci_su_layer(s, &nome, c.pos, &json!({}), &c.layer)?;
        esegui(s, "erase", json!({"handles": [c.handle]}))?;
        c.handle = nuovo;
    }
    let valori = c.valori(&Calcolo::default());
    let coppie: Vec<(&str, &str)> = valori.iter().map(|(t, x)| (*t, x.as_str())).collect();
    imposta_attributi(s, &c.handle, &coppie)?;
    aggiorna(s)?;
    risultato(s, &c.handle, "Circuito")
}

/// La tabella dei circuiti (sul foglio e in Excel).
pub fn tabella(s: &mut Session, p: &Value) -> Result<uscite::Tabella> {
    let quadro = testo_param(p, "quadro").unwrap_or_default();
    let mut v = circuiti(s)?;
    let r = calcola_tutti(&mut v);
    let col = |v: &[(&str, f64)]| v.iter().map(|(t, w)| ((*t).to_string(), *w)).collect::<Vec<_>>();
    Ok(uscite::Tabella {
        titolo: if quadro.trim().is_empty() { "TABELLA DEI CIRCUITI \\ CIRCUIT SCHEDULE".into() } else { format!("TABELLA DEI CIRCUITI QUADRO {quadro} \\ {quadro} CIRCUIT SCHEDULE") },
        colonne: col(&[
            ("Quadro\nBoard", 18.0),
            ("Circuito\nCircuit", 18.0),
            ("Sigla\nTag", 18.0),
            ("Denominazione\nDescription", 56.0),
            ("Sistema\nSystem", 20.0),
            ("P [kW]", 16.0),
            ("Ib [A]", 16.0),
            ("Protezione\nProtective device", 58.0),
            ("Cavo\nCable", 40.0),
            ("L [m]", 14.0),
            ("Iz [A]", 14.0),
            ("c.d.t. [%]", 16.0),
            ("Verifica\nCheck", 30.0),
        ]),
        righe: v
            .iter()
            .zip(&r)
            .filter(|(c, _)| quadro.trim().is_empty() || stesso_quadro(&c.quadro, &quadro))
            .map(|(c, r)| {
                let riserva = c.riserva();
                [
                    c.quadro.clone(),
                    c.circuito.clone(),
                    c.sigla.clone(),
                    if c.denominazione2.is_empty() { c.denominazione.clone() } else { format!("{} / {}", c.denominazione, c.denominazione2) },
                    c.sistema.testo(),
                    numero(c.potenza, 2),
                    numero(r.ib, 1),
                    c.sintesi(),
                    if riserva { "-".into() } else { format!("{} {}", c.cavo, c.formazione()) },
                    if riserva { "-".into() } else { numero(c.lunghezza, 1) },
                    r.iz.map_or_else(|| "-".into(), |i| numero(i, 1)),
                    if riserva { "-".into() } else { numero(r.cdt_totale, 2) },
                    if riserva { "-".into() } else { r.verifica() },
                ]
                .into_iter()
                .map(uscite::Cella::T)
                .collect()
            })
            .collect(),
        altezza_riga: 6.0,
    })
}

// ---------- comandi ----------

fn run_circuito(s: &mut Session, p: &Value) -> Result<Value> {
    in_gruppo(s, "Circuito unifilare", |s| nuovo(s, p, false))
}

fn run_arrivo(s: &mut Session, p: &Value) -> Result<Value> {
    in_gruppo(s, "Arrivo linea", |s| nuovo(s, p, true))
}

fn run_modifica(s: &mut Session, p: &Value) -> Result<Value> {
    in_gruppo(s, "Modifica circuito unifilare", |s| modifica(s, p))
}

fn run_aggiorna(s: &mut Session, _p: &Value) -> Result<Value> {
    in_gruppo(s, "Aggiorna unifilare", |s| {
        let n = aggiorna(s)?;
        Ok(json!({"circuiti": n, "message": format!("Schema unifilare: {n} circuiti ricalcolati")}))
    })
}

fn run_elenco(s: &mut Session, p: &Value) -> Result<Value> {
    let quadro = testo_param(p, "quadro").unwrap_or_default();
    let mut v = circuiti(s)?;
    let r = calcola_tutti(&mut v);
    let lista: Vec<Value> = v.iter().zip(&r).filter(|(c, _)| quadro.trim().is_empty() || stesso_quadro(&c.quadro, &quadro)).map(|(c, r)| c.json(r)).collect();
    let ko = lista.iter().filter(|c| c["ok"] == false).count();
    let msg = if lista.is_empty() { "Nessun circuito nello schema unifilare".to_string() } else { format!("{} circuiti, {ko} da verificare", lista.len()) };
    Ok(json!({"circuiti": lista, "message": msg}))
}

pub fn comandi() -> Vec<CommandSpec> {
    let spec = |id, label, menu, params, run| CommandSpec { id, label, menu, shortcut: None, aliases: &[], params, enabled: has_doc, run, undoable: false, interactive: None, transparent: false };
    let dati = "utenza?: prese|luce|motore|quadro|riscaldamento|generica|riserva, protezione?: mt|mtd|id|sez|fus, sistema?: 3F+N+PE|F+N+PE…, tensione?, potenza? (kW), cosfi?, denominazione?, denominazione2?, circuito?, nome?, curva?, in? (default: dimensionata), icn?, idn?, classe?, cavo?, sezione? (default: dimensionata), posa?: B2|C|E, k?, lunghezza? (m), codice? (articolo), costruttore?, descrizione?";
    vec![
        spec("unifilare", "Circuiti dello schema unifilare", &["Quadro", "Unifilare", "Elenco circuiti"], "{quadro?} → {circuiti: [{handle, foglio, colonna, sigla, circuito, ib, in, sezione, iz, cdtTotale, verifica, ok…}]}", run_elenco as cadcraft_engine::cmd::Run),
        spec(
            "unifilare.arrivo",
            "Arrivo linea",
            &["Quadro", "Unifilare", "Arrivo linea"],
            "{quadro?, icc? (kA presunti), kc? (contemporaneità), cdtMax? (%), come UNIFILARE.CIRCUITO} — la protezione generale sopra le sbarre, nella prima colonna del foglio corrente",
            run_arrivo,
        ),
        spec("unifilare.circuito", "Circuito unifilare", &["Quadro", "Unifilare", "Nuovo circuito"], dati, run_circuito),
        spec("unifilare.modifica", "Modifica circuito unifilare", &["Quadro", "Unifilare", "Modifica circuito"], "{handle, dimensiona?: true (rifà taratura e sezione), …i dati di UNIFILARE.CIRCUITO}", run_modifica),
        spec("unifilare.aggiorna", "Aggiorna schema unifilare", &["Quadro", "Unifilare", "Ricalcola"], "{} — ricalcola i circuiti e ridisegna sbarre, conduttori e utenze", run_aggiorna),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vicino(a: f64, b: f64, toll: f64) -> bool {
        (a - b).abs() < toll
    }

    #[test]
    fn corrente_e_caduta() {
        // 10 kW trifase a 400 V, cos φ 0,9
        let ib = corrente(10.0, 400.0, 0.9, true);
        assert!(vicino(ib, 16.04, 0.01), "{ib}");
        // 3 kW monofase a 230 V, cos φ 1
        assert!(vicino(corrente(3.0, 230.0, 1.0, false), 13.04, 0.01));
        // 50 m di 2,5 mm² con 16 A trifase: circa 2,8 %
        let c = caduta(ib, 50.0, 2.5, 0.9, 400.0, true);
        assert!(vicino(c, 2.83, 0.01), "{c}");
    }

    #[test]
    fn portate_dalla_tabella() {
        assert_eq!(portata(2.5, Posa::C, 3), Some(30.0));
        assert_eq!(portata(2.5, Posa::C, 2), Some(33.0));
        assert_eq!(portata(16.0, Posa::B2, 3), Some(80.0));
        assert_eq!(portata(3.0, Posa::C, 3), None);
    }

    #[test]
    fn sistemi() {
        let s = Sistema::da_testo("3f + n + pe").unwrap_or(Sistema { fasi: 0, neutro: false, pe: false });
        assert_eq!(s, Sistema { fasi: 3, neutro: true, pe: true });
        assert_eq!(s.testo(), "3F+N+PE");
        assert_eq!(s.conduttori(), 5);
        assert_eq!(Sistema::da_testo("F+N+PE").map(Sistema::testo).as_deref(), Some("F+N+PE"));
        assert!(Sistema::da_testo("3F+X").is_none());
    }

    #[test]
    fn dimensionamento() {
        // 10 kW trifase, 50 m: 20 A, e la caduta porta da 1,5 a 2,5 mm²
        let mut c = Circuito::nuovo(Utenza::Motore, false);
        c.potenza = 10.0;
        c.lunghezza = 50.0;
        c.cosfi = 0.9;
        c.dimensiona(&Monte::default(), true, true);
        assert_eq!(c.nominale, 20.0);
        assert_eq!(c.sezione, 2.5);
        assert_eq!(c.formazione(), "4G2,5");
        assert!(c.calcola(&Monte::default()).ok());
        // corto: basta 1,5
        c.lunghezza = 10.0;
        c.dimensiona(&Monte::default(), true, true);
        assert_eq!(c.sezione, 1.5);
        // le prese hanno almeno 2,5 mm²
        let mut p = Circuito::nuovo(Utenza::Prese, false);
        p.potenza = 1.0;
        p.dimensiona(&Monte::default(), true, true);
        assert_eq!((p.nominale, p.sezione), (6.0, 2.5));
        assert_eq!(p.formazione(), "3G2,5");
    }

    #[test]
    fn verifiche() {
        let mut c = Circuito::nuovo(Utenza::Generica, false);
        c.sistema = Sistema { fasi: 3, neutro: false, pe: true };
        c.tensione = 400.0;
        c.potenza = 10.0;
        c.cosfi = 0.9;
        c.nominale = 16.0;
        c.sezione = 1.5;
        c.lunghezza = 10.0;
        let r = c.calcola(&Monte::default());
        assert_eq!(r.problemi, vec!["Ib>In"]);
        c.nominale = 25.0;
        assert_eq!(c.calcola(&Monte::default()).problemi, vec!["In>Iz"]);
        // i fusibili vogliono In ≤ 0,9 Iz
        c.protezione = Protezione::Fusibili;
        c.nominale = 20.0;
        assert_eq!(c.calcola(&Monte::default()).problemi, vec!["In>0,9Iz"]);
        // potere di interruzione sotto il cortocircuito dell'arrivo, caduta oltre il limite
        c.protezione = Protezione::Magnetotermico;
        c.icn = 4.5;
        let r = c.calcola(&Monte { cdt: 3.9, icc: 6.0, cdt_max: 4.0 });
        assert_eq!(r.problemi, vec!["c.d.t.", "Icn<Icc"]);
    }

    #[test]
    fn arrivo_dalle_partenze() {
        let mut a = Circuito::nuovo(Utenza::Generica, true);
        a.kc = 0.8;
        a.nominale = 40.0;
        a.sezione = 10.0;
        let mut m = Circuito::nuovo(Utenza::Motore, false);
        m.potenza = 10.0;
        m.cosfi = 0.8;
        let mut r = Circuito::nuovo(Utenza::Riscaldamento, false);
        r.potenza = 10.0;
        let mut v = vec![a, m, r];
        calcola_tutti(&mut v);
        assert!(vicino(v[0].potenza, 16.0, 1e-9), "{}", v[0].potenza);
        // P = 20, Q = 7,5: cos φ = 20 / 21,36
        assert!(vicino(v[0].cosfi, 0.9363, 1e-3), "{}", v[0].cosfi);
    }

    #[test]
    fn numeri_con_la_virgola() {
        assert_eq!(numero(2.5, 1), "2,5");
        assert_eq!(numero(16.0, 1), "16");
        assert_eq!(numero(0.85, 2), "0,85");
        assert_eq!(leggi("2,5"), Some(2.5));
    }
}
