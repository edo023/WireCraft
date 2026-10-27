//! Lay-out del quadro (Interno Armadio) e foratura della piastra.
//!
//! Su un foglio nuovo si disegna in scala la piastra di fondo con le canaline (ai lati e tra le
//! file) e le guide DIN, e sulle guide l'ingombro dei componenti del quadro presi dallo schema
//! (le madri con quel `QUADRO`) e delle sue morsettiere. Gli ingombri sono blocchi con la sigla
//! nell'attributo `SIGLA` (non `NOME`: non sono componenti dello schema) e si spostano a mano
//! come ogni blocco; una tabella accanto alla piastra elenca
//! componenti e misure. La piastra si sceglie tra le misure commerciali in modo che tutto entri.
//!
//! La foratura legge guide e canaline come sono nel disegno (anche spostate o allungate a mano)
//! e mette i fori di fissaggio a passo costante: elenco in CSV con le quote dall'angolo in basso
//! a sinistra della piastra, e i fori disegnati sul foglio.

use cadcraft_engine::cmd::has_doc;
use cadcraft_engine::{CommandSpec, Result, Session};
use serde_json::{Value, json};

use crate::util::{Inserito, blocchi_definiti, blocchi_inseriti, blocco, colore, errore, esegui, handles, in_gruppo, inserisci_su_layer, linea, numero_param, su_layer_zero, testo, testo_param};
use crate::{fogli, impostazioni, morsetti, siglatura, simboli, squadratura};

type P = (f64, f64);

pub const BLOCCO_PIASTRA: &str = "CE_LAY_PIASTRA";
const BLOCCO_GUIDA: &str = "CE_LAY_GUIDA";
const BLOCCO_CANALINA: &str = "CE_LAY_CANALINA";
const PREFISSO_INGOMBRO: &str = "CE_LAY_ING_";
const PREFISSO_FORI: &str = "CE_LAY_FORI_";

/// Misure commerciali delle piastre (mm).
const LARGHEZZE: [f64; 6] = [400.0, 600.0, 800.0, 1000.0, 1200.0, 1600.0];
const ALTEZZE: [f64; 9] = [400.0, 600.0, 800.0, 1000.0, 1200.0, 1400.0, 1600.0, 1800.0, 2000.0];
/// Scale possibili (1:k).
const SCALE: [f64; 8] = [2.0, 2.5, 4.0, 5.0, 7.5, 10.0, 15.0, 20.0];

/// Bordo libero della piastra, canaline laterali e orizzontali, altezza di una fila.
const MARGINE: f64 = 20.0;
const CANALINA_V: f64 = 60.0;
const CANALINA_O: f64 = 60.0;
const FILA: f64 = 140.0;
const GUIDA: f64 = 35.0;
/// Spazio tra la canalina laterale e la guida, e tra due componenti.
const AGIO: f64 = 10.0;
const DISTACCO: f64 = 2.0;
/// Larghezza di un morsetto e delle due testate di una morsettiera.
const MORSETTO: f64 = 5.2;
const TESTATE: f64 = 18.0;

/// Area del foglio per piastra e tabella.
const AREA_MIN: P = (25.0, squadratura::CORNICE_MIN.1 + squadratura::CARTIGLIO_ALTEZZA + 10.0);
const AREA_MAX: P = (405.0, squadratura::CORNICE_MAX.1 - squadratura::FASCIA_DESCRIZIONI - 8.0);
const LARGHEZZA_TABELLA: f64 = 125.0;

/// L'ingombro di un elemento da montare sulle guide.
#[derive(Clone, Debug, PartialEq)]
pub struct Ingombro {
    pub sigla: String,
    pub descrizione: String,
    pub codice: String,
    pub larghezza: f64,
    pub altezza: f64,
    /// Ordine di montaggio: protezioni, alimentazione, comando, ... morsettiere per ultime.
    gruppo: u8,
}

/// Larghezza e altezza (mm) di un componente sulla guida, dal simbolo o dalle lettere della
/// sigla; None per quello che non sta nel quadro (motori, pulsanti, sensori, lampade, valvole).
fn misura(id: Option<&str>, prefisso: &str, poli: usize) -> Option<(f64, f64, u8)> {
    let per_id = match id.unwrap_or("") {
        "qf1" => Some((18.0, 90.0)),
        "qf2" | "qd2" => Some((36.0, 90.0)),
        "qf3" => Some((54.0, 90.0)),
        "qf4" | "qd4" => Some((72.0, 90.0)),
        "qm3" => Some((45.0, 100.0)),
        "qs3" => Some((54.0, 90.0)),
        "fu" => Some((18.0, 90.0)),
        "fu3" => Some((54.0, 90.0)),
        "fr3" => Some((45.0, 80.0)),
        "km" => Some((45.0, 85.0)),
        "ka" => Some((16.0, 80.0)),
        "kt" | "ktdis" => Some((22.5, 90.0)),
        "tc" => Some((80.0, 95.0)),
        "g" => Some((60.0, 90.0)),
        "eh" => Some((40.0, 80.0)),
        "xs" => Some((54.0, 90.0)),
        "c" => Some((18.0, 60.0)),
        "v" => Some((6.2, 60.0)),
        _ => None,
    };
    let p = prefisso.to_ascii_uppercase();
    let gruppo = match p.chars().next() {
        Some('Q') => 0,
        Some('F') => 1,
        Some('T' | 'G') => 2,
        Some('U') => 3,
        Some('A') => 4,
        Some('K') => 5,
        _ => 6,
    };
    if let Some((w, h)) = per_id {
        return Some((w, h, gruppo));
    }
    let poli = poli.max(1) as f64;
    let (w, h) = match p.chars().next()? {
        'M' | 'S' | 'B' | 'H' | 'Y' | 'W' => return None,
        'X' if p != "XS" => return None,
        'Q' => (18.0 * poli, 90.0),
        'F' => (18.0 * poli, 90.0),
        'K' => (22.5, 85.0),
        'T' => (80.0, 95.0),
        'G' => (60.0, 90.0),
        'U' => (90.0, 120.0),
        'A' => (120.0, 100.0),
        'E' => (40.0, 80.0),
        'P' => (45.0, 90.0),
        _ => (18.0, 80.0),
    };
    Some((w, h, gruppo))
}

/// Il quadro di un oggetto appartiene a quello chiesto (vuoto = quadro di default).
fn del_quadro(q: &str, quadro: &str, senza_quadro: bool) -> bool {
    if q.trim().is_empty() { senza_quadro } else { q.trim().eq_ignore_ascii_case(quadro.trim()) }
}

/// Gli ingombri del quadro: componenti (madri) in ordine di montaggio, poi le morsettiere.
pub fn ingombri(s: &mut Session, quadro: &str) -> Result<Vec<Ingombro>> {
    let default = impostazioni::quadro_corrente().unwrap_or_default();
    let senza_quadro = quadro.trim().is_empty() || default.trim().eq_ignore_ascii_case(quadro.trim());
    let mut v: Vec<Ingombro> = Vec::new();
    for c in siglatura::componenti(s)?.into_iter().filter(|c| c.madre()) {
        let b = &c.blocco;
        if !del_quadro(b.attributo("QUADRO"), quadro, senza_quadro) || v.iter().any(|i| i.sigla == c.nome) {
            continue;
        }
        let sim = simboli::LIBRERIA.iter().find(|x| x.blocco.eq_ignore_ascii_case(&b.nome));
        let prefisso = siglatura::prefisso_di(&c.nome).unwrap_or_default();
        let Some((mut w, h, gruppo)) = misura(sim.map(|x| x.id), &prefisso, sim.map_or(3, |x| x.poli.len())) else { continue };
        // una larghezza scritta sul componente vale più di quella proposta
        if let Ok(x) = b.attributo("LARGHEZZA").trim().replace(',', ".").parse::<f64>()
            && x > 0.0
        {
            w = x;
        }
        let descrizione = [b.attributo("FUNZIONE1"), b.attributo("DESCRIZIONE")].into_iter().find(|t| !t.trim().is_empty()).or(sim.map(|x| x.etichetta)).unwrap_or("").to_string();
        v.push(Ingombro { sigla: c.nome.clone(), descrizione, codice: b.attributo("TIPO").to_string(), larghezza: w, altezza: h, gruppo });
    }
    v.sort_by(|a, b| a.gruppo.cmp(&b.gruppo).then_with(|| siglatura::prefisso_di(&a.sigla).cmp(&siglatura::prefisso_di(&b.sigla))).then_with(|| a.sigla.cmp(&b.sigla)));
    let mut file: Vec<(String, usize)> = Vec::new();
    for m in morsetti::morsetti(s)? {
        if m.blocco.nome.eq_ignore_ascii_case(morsetti::BLOCCO_CONNETTORE) || !del_quadro(m.blocco.attributo("QUADRO"), quadro, senza_quadro) {
            continue;
        }
        match file.iter_mut().find(|(n, _)| n.eq_ignore_ascii_case(&m.morsettiera)) {
            Some(f) => f.1 += 1,
            None => file.push((m.morsettiera.clone(), 1)),
        }
    }
    file.sort();
    for (nome, n) in file {
        v.push(Ingombro { sigla: nome, descrizione: format!("Morsettiera, {n} morsetti"), codice: String::new(), larghezza: n as f64 * MORSETTO + TESTATE, altezza: 50.0, gruppo: 9 });
    }
    Ok(v)
}

/// Le file di una piastra: (y della guida dal basso, lunghezza utile).
fn file(larghezza: f64, altezza: f64) -> (Vec<f64>, f64) {
    let lunghezza = larghezza - 2.0 * (MARGINE + CANALINA_V + AGIO);
    let n = ((altezza - 2.0 * MARGINE - CANALINA_O) / (CANALINA_O + FILA)).floor().max(0.0) as usize;
    // dall'alto: canalina, fila, canalina, fila, ... canalina
    let alto = altezza - MARGINE - CANALINA_O;
    ((0..n).map(|k| alto - FILA / 2.0 - k as f64 * (FILA + CANALINA_O)).collect(), lunghezza)
}

/// Dispone gli ingombri sulle file: per ognuno (fila, x dall'inizio della guida); quelli che
/// non entrano restano fuori (None).
fn disponi(v: &[Ingombro], file: usize, lunghezza: f64) -> Vec<Option<(usize, f64)>> {
    let mut libero = vec![0.0; file];
    let mut fila = 0;
    let mut morsettiere = false;
    let mut out = Vec::new();
    for i in v {
        // le morsettiere cominciano una fila nuova, se ce n'è un'altra
        if i.gruppo == 9 && !morsettiere {
            morsettiere = true;
            if libero.get(fila).is_some_and(|x| *x > 0.0) && fila + 1 < file {
                fila += 1;
            }
        }
        while libero.get(fila).is_some_and(|x| x + i.larghezza > lunghezza + 1e-6) {
            fila += 1;
        }
        out.push(libero.get_mut(fila).map(|x| {
            let at = *x;
            *x += i.larghezza + DISTACCO;
            (fila, at)
        }));
    }
    out
}

/// La piastra commerciale più piccola in cui entra tutto (o la più grande).
fn scegli_piastra(v: &[Ingombro]) -> (f64, f64) {
    let mut misure: Vec<(f64, f64)> = LARGHEZZE.iter().flat_map(|w| ALTEZZE.iter().map(move |h| (*w, *h))).filter(|(w, h)| h >= w || *h >= 600.0).collect();
    misure.sort_by(|a, b| (a.0 * a.1).total_cmp(&(b.0 * b.1)).then_with(|| a.0.total_cmp(&b.0)));
    // entra tutto, e le morsettiere hanno una guida loro sotto i componenti
    let entra = |&(w, h): &(f64, f64)| {
        let (f, l) = file(w, h);
        let posti = disponi(v, f.len(), l);
        let fila = |morsettiera: bool| v.iter().zip(&posti).filter(move |(i, _)| (i.gruppo == 9) == morsettiera).filter_map(|(_, p)| p.map(|p| p.0));
        let separate = match (fila(false).max(), fila(true).min()) {
            (Some(c), Some(m)) => m > c,
            _ => true,
        };
        !f.is_empty() && posti.iter().all(Option::is_some) && separate
    };
    misure.iter().copied().find(entra).unwrap_or((LARGHEZZE[LARGHEZZE.len() - 1], ALTEZZE[ALTEZZE.len() - 1]))
}

/// La scala più grande (1:k) con cui la piastra entra nel foglio accanto alla tabella.
fn scegli_scala(larghezza: f64, altezza: f64) -> f64 {
    let (w, h) = (AREA_MAX.0 - AREA_MIN.0 - LARGHEZZA_TABELLA - 15.0, AREA_MAX.1 - AREA_MIN.1);
    SCALE.iter().copied().find(|k| larghezza / k <= w && altezza / k <= h).unwrap_or(SCALE[SCALE.len() - 1])
}

fn crea_blocchi_fissi(s: &mut Session) -> Result<()> {
    let definiti = blocchi_definiti(s)?;
    let c = |n: &str| definiti.iter().any(|b| b.eq_ignore_ascii_case(n));
    let attributi = |s: &mut Session, h: &mut Vec<String>, tag: &[&str]| -> Result<()> {
        for t in tag {
            h.extend(handles(&esegui(s, "attdef", json!({"tag": t, "prompt": t, "default": "", "at": [0.0, 0.0], "height": 1.0, "invisible": true}))?));
        }
        Ok(())
    };
    su_layer_zero(s, |s| {
        if !c(BLOCCO_GUIDA) {
            // guida DIN di lunghezza 1 e altezza 1 (si scala nei due sensi)
            colore(s, "8")?;
            let mut h = Vec::new();
            linea(s, &mut h, &[(0.0, -0.5), (1.0, -0.5), (1.0, 0.5), (0.0, 0.5)], true)?;
            linea(s, &mut h, &[(0.0, -0.2), (1.0, -0.2)], false)?;
            linea(s, &mut h, &[(0.0, 0.2), (1.0, 0.2)], false)?;
            attributi(s, &mut h, &["LUNGHEZZA"])?;
            blocco(s, BLOCCO_GUIDA, h, "Guida DIN")?;
        }
        if !c(BLOCCO_CANALINA) {
            colore(s, "9")?;
            let mut h = Vec::new();
            linea(s, &mut h, &[(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)], true)?;
            linea(s, &mut h, &[(0.0, 0.0), (1.0, 1.0)], false)?;
            linea(s, &mut h, &[(0.0, 1.0), (1.0, 0.0)], false)?;
            attributi(s, &mut h, &["LUNGHEZZA", "LARGHEZZA", "VERSO"])?;
            blocco(s, BLOCCO_CANALINA, h, "Canalina")?;
        }
        if !c(BLOCCO_PIASTRA) {
            colore(s, "7")?;
            let mut h = Vec::new();
            linea(s, &mut h, &[(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)], true)?;
            attributi(s, &mut h, &["QUADRO", "LARGHEZZA", "ALTEZZA", "SCALA"])?;
            blocco(s, BLOCCO_PIASTRA, h, "Piastra di fondo")?;
        }
        Ok(())
    })
}

fn misura_testo(x: f64) -> String {
    let t = format!("{x:.1}");
    t.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// Il blocco dell'ingombro `l`×`a` mm alla scala 1:k (rettangolo con la sigla).
fn blocco_ingombro(s: &mut Session, l: f64, a: f64, k: f64) -> Result<String> {
    let nome = format!("{PREFISSO_INGOMBRO}{}x{}_{}", misura_testo(l), misura_testo(a), misura_testo(k)).replace('.', "_");
    if blocchi_definiti(s)?.iter().any(|b| b.eq_ignore_ascii_case(&nome)) {
        return Ok(nome);
    }
    let (w, h) = (l / k, a / k);
    su_layer_zero(s, |s| {
        colore(s, "4")?;
        let mut v = Vec::new();
        linea(s, &mut v, &[(0.0, -h / 2.0), (w, -h / 2.0), (w, h / 2.0), (0.0, h / 2.0)], true)?;
        colore(s, "2")?;
        let alt = 1.8_f64.min(h / 4.0).max(0.8);
        let r = esegui(s, "attdef", json!({"tag": "SIGLA", "prompt": "Sigla", "default": "", "at": [w / 2.0, 0.0], "height": alt}))?;
        let hn = handles(&r);
        // nei componenti stretti la sigla va in verticale
        if let Some(t) = hn.first() {
            if w < 6.0 * alt {
                crate::util::ruota_testo(s, t, 90.0, cadcraft_engine::doc::HAlign::Center)?;
            } else {
                crate::util::allinea(s, t, cadcraft_engine::doc::HAlign::Center)?;
            }
        }
        v.extend(hn);
        colore(s, "BYLAYER")?;
        for t in ["QUADRO", "CODICE", "LARGHEZZA", "ALTEZZA"] {
            v.extend(handles(&esegui(s, "attdef", json!({"tag": t, "prompt": t, "default": "", "at": [0.0, 0.0], "height": 1.0, "invisible": true}))?));
        }
        blocco(s, &nome, v, "Ingombro componente")
    })?;
    Ok(nome)
}

fn inserisci_scalato(s: &mut Session, nome: &str, at: P, scala: P, attributi: &Value, layer: &str) -> Result<String> {
    let r = esegui(s, "insert", json!({"name": nome, "at": [at.0, at.1], "scale": [scala.0, scala.1], "attribs": attributi}))?;
    let h = handles(&r).into_iter().next().ok_or_else(|| errore(format!("blocco {nome} non inserito")))?;
    esegui(s, "laymch", json!({"handles": [h], "layer": layer}))?;
    Ok(h)
}

/// Esito del lay-out.
#[derive(Clone, Debug, PartialEq)]
pub struct Esito {
    pub foglio: u32,
    pub larghezza: f64,
    pub altezza: f64,
    pub scala: f64,
    pub posati: usize,
    /// Sigle che non entrano nella piastra.
    pub fuori: Vec<String>,
}

/// Il foglio del lay-out: `n` se è vuoto o è già il lay-out del quadro (si rifà), altrimenti un
/// foglio nuovo dopo l'ultimo.
fn foglio_per(s: &mut Session, n: Option<u32>, quadro: &str) -> Result<u32> {
    let elenco = fogli::elenco(s)?;
    if let Some(n) = n {
        let layer = fogli::nome_layer(n);
        if !elenco.iter().any(|f| f.numero == n) {
            fogli::crea(s, n)?;
            return Ok(n);
        }
        let suo = blocchi_inseriti(s, BLOCCO_PIASTRA)?.iter().any(|b| b.layer == layer && b.attributo("QUADRO").eq_ignore_ascii_case(quadro));
        let vuoto = !s.doc()?.model.iter().any(|e| e.common.layer == layer && !matches!(&e.kind, cadcraft_engine::doc::EntityKind::Insert(i) if { let b = i.block.to_ascii_uppercase(); b.starts_with("NUMFG") || b.starts_with("TITOLO") }));
        if suo || vuoto {
            let via: Vec<String> = s.doc()?.model.iter().filter(|e| e.common.layer == layer && matches!(&e.kind, cadcraft_engine::doc::EntityKind::Insert(i) if i.block.to_ascii_uppercase().starts_with("CE_LAY_"))).map(|e| e.handle.hex()).collect();
            if !via.is_empty() {
                esegui(s, "erase", json!({"handles": via}))?;
            }
            return Ok(n);
        }
        crate::multifoglio::inserisci_foglio(s, n)?;
        return Ok(n);
    }
    let n = elenco.last().map_or(1, |f| f.numero + 1);
    fogli::crea(s, n)?;
    Ok(n)
}

/// Disegna il lay-out del quadro sul foglio `n` (o su uno nuovo), con la piastra data o scelta.
pub fn disegna(s: &mut Session, quadro: &str, n: Option<u32>, piastra: Option<(f64, f64)>) -> Result<Esito> {
    let v = ingombri(s, quadro)?;
    if v.is_empty() {
        return Err(errore(format!("Nessun componente da montare nel quadro {quadro}")));
    }
    let (lw, lh) = piastra.unwrap_or_else(|| scegli_piastra(&v));
    let k = scegli_scala(lw, lh);
    let (righe, lunghezza) = file(lw, lh);
    let posti = disponi(&v, righe.len(), lunghezza);
    let n = foglio_per(s, n, quadro)?;
    let layer = fogli::nome_layer(n);
    crea_blocchi_fissi(s)?;
    // la piastra in basso a sinistra dell'area
    let o = (AREA_MIN.0, AREA_MIN.1 + ((AREA_MAX.1 - AREA_MIN.1) - lh / k).max(0.0));
    let q = |x: f64, y: f64| (o.0 + x / k, o.1 + y / k);
    let m = |x: f64| misura_testo(x);
    inserisci_scalato(s, BLOCCO_PIASTRA, o, (lw / k, lh / k), &json!({"QUADRO": quadro, "LARGHEZZA": m(lw), "ALTEZZA": m(lh), "SCALA": m(k)}), &layer)?;
    // canaline: due verticali ai lati, orizzontali sopra e sotto ogni fila
    let alto_v = lh - 2.0 * MARGINE;
    for x in [MARGINE, lw - MARGINE - CANALINA_V] {
        inserisci_scalato(s, BLOCCO_CANALINA, q(x, MARGINE), (CANALINA_V / k, alto_v / k), &json!({"LUNGHEZZA": m(alto_v), "LARGHEZZA": m(CANALINA_V), "VERSO": "V"}), &layer)?;
    }
    let lungo_o = lw - 2.0 * (MARGINE + CANALINA_V);
    let mut ys: Vec<f64> = righe.iter().map(|y| y + FILA / 2.0).collect();
    if let Some(ultima) = righe.last() {
        ys.push(ultima - FILA / 2.0 - CANALINA_O);
    }
    for y in ys {
        inserisci_scalato(s, BLOCCO_CANALINA, q(MARGINE + CANALINA_V, y), (lungo_o / k, CANALINA_O / k), &json!({"LUNGHEZZA": m(lungo_o), "LARGHEZZA": m(CANALINA_O), "VERSO": "O"}), &layer)?;
    }
    let x0 = MARGINE + CANALINA_V + AGIO;
    for y in &righe {
        inserisci_scalato(s, BLOCCO_GUIDA, q(x0, *y), (lunghezza / k, GUIDA / k), &json!({"LUNGHEZZA": m(lunghezza)}), &layer)?;
    }
    let mut posati = 0;
    let mut fuori = Vec::new();
    for (i, posto) in v.iter().zip(&posti) {
        match posto.and_then(|(f, x)| righe.get(f).map(|y| (x, *y))) {
            Some((x, y)) => {
                let nome = blocco_ingombro(s, i.larghezza, i.altezza, k)?;
                inserisci_su_layer(s, &nome, q(x0 + x, y), &json!({"SIGLA": i.sigla, "QUADRO": quadro, "CODICE": i.codice, "LARGHEZZA": m(i.larghezza), "ALTEZZA": m(i.altezza)}), &layer)?;
                posati += 1;
            }
            None => fuori.push(i.sigla.clone()),
        }
    }
    // didascalia della piastra e tabella dei componenti
    let mut h = Vec::new();
    testo(s, &mut h, (o.0, o.1 - 4.0), &format!("Piastra {} x {} mm – scala 1:{}", m(lw), m(lh), m(k)), 2.5, "L")?;
    tabella(s, &mut h, (AREA_MAX.0 - LARGHEZZA_TABELLA, AREA_MAX.1), &v, &posti)?;
    esegui(s, "laymch", json!({"handles": h, "layer": layer}))?;
    let titolo = if quadro.trim().is_empty() { "LAYOUT INTERNO QUADRO".to_string() } else { format!("LAYOUT INTERNO QUADRO {}", quadro.trim()) };
    let titolo1 = if quadro.trim().is_empty() { "PANEL LAYOUT".to_string() } else { format!("PANEL LAYOUT {}", quadro.trim()) };
    fogli::imposta_titolo(s, n, &titolo, Some(&titolo1))?;
    fogli::numera(s)?;
    Ok(Esito { foglio: n, larghezza: lw, altezza: lh, scala: k, posati, fuori })
}

/// La tabella dei componenti accanto alla piastra (in alto a destra dell'area).
fn tabella(s: &mut Session, h: &mut Vec<String>, at: P, v: &[Ingombro], posti: &[Option<(usize, f64)>]) -> Result<()> {
    let colonne = [("Sigla", 22.0), ("Descrizione", 63.0), ("L x H", 22.0), ("Fila", 18.0)];
    let riga = 5.0;
    let max = (((at.1 - AREA_MIN.1) / riga).floor() as usize).saturating_sub(2);
    let righe: Vec<[String; 4]> = v
        .iter()
        .zip(posti)
        .map(|(i, p)| [i.sigla.clone(), i.descrizione.clone(), format!("{}x{}", misura_testo(i.larghezza), misura_testo(i.altezza)), p.map_or_else(|| "fuori".to_string(), |(f, _)| (f + 1).to_string())])
        .take(max)
        .collect();
    colore(s, "7")?;
    let largo: f64 = colonne.iter().map(|c| c.1).sum();
    let fondo = at.1 - riga * (righe.len() + 1) as f64;
    linea(s, h, &[at, (at.0 + largo, at.1), (at.0 + largo, fondo), (at.0, fondo)], true)?;
    let mut x = at.0;
    for (t, w) in colonne {
        testo(s, h, (x + 1.2, at.1 - riga / 2.0), t, 2.2, "ML")?;
        x += w;
        if x < at.0 + largo - 0.01 {
            linea(s, h, &[(x, at.1), (x, fondo)], false)?;
        }
    }
    for (r, celle) in righe.iter().enumerate() {
        let y = at.1 - riga * (r + 1) as f64;
        linea(s, h, &[(at.0, y), (at.0 + largo, y)], false)?;
        let mut x = at.0;
        for ((_, w), t) in colonne.iter().zip(celle) {
            let max = ((w - 2.0) / 1.7).floor().max(1.0) as usize;
            let t: String = if t.chars().count() > max { t.chars().take(max - 1).chain(['…']).collect() } else { t.clone() };
            testo(s, h, (x + 1.2, y - riga / 2.0), &t, 2.0, "ML")?;
            x += w;
        }
    }
    colore(s, "BYLAYER")
}

// ---------- foratura ----------

/// Un foro della piastra: quote (mm) dall'angolo in basso a sinistra, diametro, elemento.
#[derive(Clone, Debug, PartialEq)]
pub struct Foro {
    pub x: f64,
    pub y: f64,
    pub diametro: f64,
    pub elemento: String,
}

/// Opzioni della foratura.
#[derive(Clone, Copy, Debug)]
pub struct OpzioniFori {
    pub diametro: f64,
    pub passo_guide: f64,
    pub passo_canaline: f64,
    /// Distanza del primo foro dall'estremo.
    pub dall_estremo: f64,
}

impl Default for OpzioniFori {
    fn default() -> Self {
        OpzioniFori { diametro: 4.2, passo_guide: 100.0, passo_canaline: 250.0, dall_estremo: 25.0 }
    }
}

/// Le posizioni dei fori lungo un tratto lungo `l`: agli estremi (a `e`) e a passo `p`.
fn lungo(l: f64, e: f64, p: f64) -> Vec<f64> {
    if l <= 2.0 * e {
        return vec![l / 2.0];
    }
    let utile = l - 2.0 * e;
    let n = (utile / p).ceil().max(1.0) as usize;
    (0..=n).map(|i| e + utile * i as f64 / n as f64).collect()
}

fn numero(b: &Inserito, tag: &str) -> f64 {
    b.attributo(tag).trim().replace(',', ".").parse().unwrap_or(0.0)
}

/// I fori della piastra del foglio `n`.
pub fn fori(s: &mut Session, n: u32, o: &OpzioniFori) -> Result<(Inserito, Vec<Foro>)> {
    let layer = fogli::nome_layer(n);
    let piastra = blocchi_inseriti(s, BLOCCO_PIASTRA)?.into_iter().find(|b| b.layer == layer).ok_or_else(|| errore(format!("Il foglio {n} non ha il lay-out di un quadro")))?;
    let k = numero(&piastra, "SCALA").max(0.1);
    let mm = |p: P| ((p.0 - piastra.pos.0) * k, (p.1 - piastra.pos.1) * k);
    let mut v = Vec::new();
    for g in blocchi_inseriti(s, BLOCCO_GUIDA)?.into_iter().filter(|b| b.layer == layer) {
        let (x, y) = mm(g.pos);
        for d in lungo(numero(&g, "LUNGHEZZA"), o.dall_estremo, o.passo_guide) {
            v.push(Foro { x: x + d, y, diametro: o.diametro, elemento: "Guida DIN".into() });
        }
    }
    for c in blocchi_inseriti(s, BLOCCO_CANALINA)?.into_iter().filter(|b| b.layer == layer) {
        let (x, y) = mm(c.pos);
        let (l, w) = (numero(&c, "LUNGHEZZA"), numero(&c, "LARGHEZZA"));
        let verticale = c.attributo("VERSO").eq_ignore_ascii_case("V");
        for d in lungo(l, o.dall_estremo, o.passo_canaline) {
            let (fx, fy) = if verticale { (x + w / 2.0, y + d) } else { (x + d, y + w / 2.0) };
            v.push(Foro { x: fx, y: fy, diametro: o.diametro, elemento: "Canalina".into() });
        }
    }
    let arrotonda = |t: f64| (t * 10.0).round() / 10.0;
    for f in &mut v {
        f.x = arrotonda(f.x);
        f.y = arrotonda(f.y);
    }
    v.sort_by(|a, b| b.y.total_cmp(&a.y).then_with(|| a.x.total_cmp(&b.x)));
    v.dedup_by(|a, b| (a.x - b.x).abs() < 0.5 && (a.y - b.y).abs() < 0.5);
    Ok((piastra, v))
}

/// Disegna i fori sul foglio (un blocco per foglio, rifatto ogni volta).
fn disegna_fori(s: &mut Session, n: u32, piastra: &Inserito, v: &[Foro]) -> Result<()> {
    let layer = fogli::nome_layer(n);
    let nome = format!("{PREFISSO_FORI}{n}");
    let via: Vec<String> = blocchi_inseriti(s, &nome)?.into_iter().map(|b| b.handle).collect();
    if !via.is_empty() {
        esegui(s, "erase", json!({"handles": via}))?;
    }
    let k = numero(piastra, "SCALA").max(0.1);
    su_layer_zero(s, |s| {
        colore(s, "1")?;
        let mut h = Vec::new();
        for f in v {
            let r = (f.diametro / 2.0 / k).max(0.35);
            h.extend(handles(&esegui(s, "circle", json!({"center": [f.x / k, f.y / k], "radius": r}))?));
        }
        blocco(s, &nome, h, "Foratura piastra")
    })?;
    inserisci_su_layer(s, &nome, piastra.pos, &json!({}), &layer)?;
    Ok(())
}

// ---------- comandi ----------

/// Il quadro più usato dai componenti dello schema.
fn quadro_piu_usato(s: &mut Session) -> Result<Option<String>> {
    let mut conta: Vec<(String, usize)> = Vec::new();
    for c in siglatura::componenti(s)?.into_iter().filter(|c| c.madre()) {
        let q = c.blocco.attributo("QUADRO").trim().to_string();
        if q.is_empty() {
            continue;
        }
        match conta.iter_mut().find(|(x, _)| x.eq_ignore_ascii_case(&q)) {
            Some(x) => x.1 += 1,
            None => conta.push((q, 1)),
        }
    }
    Ok(conta.into_iter().max_by_key(|(_, n)| *n).map(|(q, _)| q))
}

fn run_layout(s: &mut Session, p: &Value) -> Result<Value> {
    let quadro = match testo_param(p, "quadro").filter(|q| !q.trim().is_empty()).or_else(impostazioni::quadro_corrente).filter(|q| !q.trim().is_empty()) {
        Some(q) => q,
        None => quadro_piu_usato(s)?.unwrap_or_default(),
    };
    let piastra = match (p.get("larghezza").and_then(Value::as_f64), p.get("altezza").and_then(Value::as_f64)) {
        (Some(w), Some(h)) if w >= 200.0 && h >= 300.0 => Some((w, h)),
        (None, None) => None,
        _ => return Err(errore("La piastra deve essere almeno 200 x 300 mm (larghezza e altezza)")),
    };
    let n = numero_param(p, "foglio");
    in_gruppo(s, "Lay-out quadro", |s| {
        let e = disegna(s, &quadro, n, piastra)?;
        let mut msg = format!("Lay-out {quadro} sul foglio {}: piastra {}x{} mm, scala 1:{}, {} elementi", e.foglio, e.larghezza, e.altezza, misura_testo(e.scala), e.posati);
        if !e.fuori.is_empty() {
            msg.push_str(&format!("; non entrano: {}", e.fuori.join(", ")));
        }
        Ok(json!({"foglio": e.foglio, "larghezza": e.larghezza, "altezza": e.altezza, "scala": e.scala, "posati": e.posati, "fuori": e.fuori, "message": msg}))
    })
}

fn run_fori(s: &mut Session, p: &Value) -> Result<Value> {
    let n = match numero_param(p, "foglio") {
        Some(n) => n,
        None => fogli::corrente(s)?.map(|f| f.numero).ok_or_else(|| errore("Nessun foglio corrente"))?,
    };
    let d = OpzioniFori::default();
    let o = OpzioniFori {
        diametro: p.get("diametro").and_then(Value::as_f64).filter(|x| *x > 0.0).unwrap_or(d.diametro),
        passo_guide: p.get("passoGuide").and_then(Value::as_f64).filter(|x| *x >= 20.0).unwrap_or(d.passo_guide),
        passo_canaline: p.get("passoCanaline").and_then(Value::as_f64).filter(|x| *x >= 20.0).unwrap_or(d.passo_canaline),
        dall_estremo: d.dall_estremo,
    };
    in_gruppo(s, "Foratura piastra", |s| {
        let (piastra, v) = fori(s, n, &o)?;
        if p.get("disegna").and_then(Value::as_bool).unwrap_or(true) {
            disegna_fori(s, n, &piastra, &v)?;
        }
        let righe: Vec<Vec<String>> = v.iter().enumerate().map(|(i, f)| vec![(i + 1).to_string(), misura_testo(f.x), misura_testo(f.y), misura_testo(f.diametro), f.elemento.clone()]).collect();
        if let Some(file) = testo_param(p, "file") {
            morsetti::scrivi_csv(&file, &["N", "X", "Y", "Diametro", "Elemento"], &righe)?;
        }
        Ok(json!({"fori": righe, "message": format!("Piastra {}x{} mm: {} fori", piastra.attributo("LARGHEZZA"), piastra.attributo("ALTEZZA"), v.len())}))
    })
}

pub fn comandi() -> Vec<CommandSpec> {
    let spec = |id, label, menu, params, run| CommandSpec { id, label, menu, shortcut: None, aliases: &[], params, enabled: has_doc, run, undoable: false, interactive: None, transparent: false };
    vec![
        spec(
            "layoutquadro",
            "Interno Armadio",
            &["LAYOUT", "Interno Armadio"],
            "{quadro?: \"+QG\" (default il quadro corrente), foglio?: numero (default dopo l'ultimo), larghezza?, altezza?: piastra in mm (default la più piccola in cui entra tutto)} → {foglio, larghezza, altezza, scala, posati, fuori}",
            run_layout as cadcraft_engine::cmd::Run,
        ),
        spec(
            "foraturapiastra",
            "Foratura Piastra",
            &["LAYOUT", "Foratura Piastra"],
            "{foglio?: foglio del lay-out (default il corrente), diametro?: 4.2, passoGuide?: 100, passoCanaline?: 250, file?: CSV, disegna?: true} → {fori: [[n, x, y, diametro, elemento]]}",
            run_fori,
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ing(sigla: &str, l: f64, gruppo: u8) -> Ingombro {
        Ingombro { sigla: sigla.into(), descrizione: String::new(), codice: String::new(), larghezza: l, altezza: 90.0, gruppo }
    }

    #[test]
    fn misure_dai_simboli_e_dalle_sigle() {
        assert_eq!(misura(Some("qf3"), "QF", 3), Some((54.0, 90.0, 0)));
        assert_eq!(misura(Some("km"), "KM", 3), Some((45.0, 85.0, 5)));
        assert_eq!(misura(None, "QF", 2), Some((36.0, 90.0, 0)));
        assert_eq!(misura(Some("m3"), "M", 3), None, "il motore non sta nel quadro");
        assert_eq!(misura(None, "SB", 1), None);
        assert_eq!(misura(None, "X", 1), None);
    }

    #[test]
    fn file_e_disposizione() {
        let (f, l) = file(600.0, 800.0);
        assert_eq!(f.len(), 3);
        assert_eq!(l, 600.0 - 2.0 * (20.0 + 60.0 + 10.0));
        assert_eq!(f[0], 800.0 - 20.0 - 60.0 - 70.0);
        let v = vec![ing("-QF1", 300.0, 0), ing("-KM1", 200.0, 5), ing("-X1", 100.0, 9)];
        // la seconda non entra nella prima fila; la morsettiera va su una fila nuova
        assert_eq!(disponi(&v, 3, l), vec![Some((0, 0.0)), Some((1, 0.0)), Some((2, 0.0))]);
        // con due file la morsettiera segue il contattore
        assert_eq!(disponi(&v, 2, l)[2], Some((1, 202.0)));
        assert_eq!(disponi(&[ing("-QF1", 500.0, 0)], 2, l), vec![None], "più lungo della guida");
        assert_eq!(lungo(420.0, 25.0, 100.0), vec![25.0, 117.5, 210.0, 302.5, 395.0]);
        assert_eq!(lungo(30.0, 25.0, 100.0), vec![15.0]);
    }

    #[test]
    fn piastra_e_scala() {
        let poco = vec![ing("-QF1", 54.0, 0)];
        assert_eq!(scegli_piastra(&poco), (400.0, 400.0));
        let tanto: Vec<Ingombro> = (0..30).map(|i| ing(&format!("-KM{i}"), 45.0, 5)).collect();
        let (w, h) = scegli_piastra(&tanto);
        let (f, l) = file(w, h);
        assert!(disponi(&tanto, f.len(), l).iter().all(Option::is_some));
        assert_eq!(scegli_scala(600.0, 800.0), 4.0);
    }
}
