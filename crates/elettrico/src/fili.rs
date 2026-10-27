//! Fili e collegamenti: filo, connessione, alimentazioni, collegamenti a N fili, rimandi,
//! quick filo verticale/orizzontale, aggiornamento dei rimandi tra fogli.
//!
//! Come nei CAD elettrici diffusi: un filo è una LINE sul
//! layer del foglio (le linee di colore 5 sono grafica, non fili); il nodo è il blocco `Nodo`; i
//! rimandi sono i blocchi `_ALIM_n` (alimentazioni) e `_SIGN_n` (collegamenti), uno per tipo e
//! rotazione, con gli attributi `PRES` (P/A), `SEGNALE`, `#LINEA` e `REG` (riferimento).

use cadcraft_engine::cmd::has_doc;
use cadcraft_engine::doc::{EntityKind, Line};
use cadcraft_engine::geom::{Vec2, Vec3};
use cadcraft_engine::{Accept, CommandSpec, Input, Interactive, Prompt, Result, Session, Step};
use serde_json::{Value, json};

use crate::fogli;
use crate::squadratura::{self, CARTIGLIO_ALTEZZA, CORNICE_MAX, CORNICE_MIN};
use crate::util::{
    Inserito, blocchi_definiti, blocchi_inseriti, blocco, colore, errore, esegui, handles, in_gruppo, layer_corrente, linea, numero_param, punto_param, su_layer_zero,
    testo_param,
};

/// Distanza tra fili paralleli (mm), come nei settaggi predefiniti dei CAD elettrici diffusi.
pub const PASSO: f64 = 10.0;
const TOLL: f64 = 0.01;
const MAX_FILI: u32 = 12;
/// Fili DaLayer (i layer dei fogli hanno il colore dei fili), alimentazioni in rosso.
const COLORE_FILO: Option<&str> = None;
const COLORE_ALIMENTAZIONE: Option<&str> = Some("1");
/// Il PE nei progetti dei CAD elettrici diffusi ha colore 50.
const COLORE_PE: &str = "50";
/// Le linee di questo colore sono grafica, non fili.
const COLORE_IGNORATO: u8 = 5;
/// Spessore dei fili (centesimi di mm).
const SPESSORE_FILO: i64 = 30;

pub const BLOCCO_NODO: &str = "Nodo";

type P = (f64, f64);

#[derive(Clone, Debug, PartialEq)]
pub struct Segmento {
    pub a: P,
    pub b: P,
    pub handle: String,
}

impl Segmento {
    fn orizzontale(&self) -> bool {
        (self.a.1 - self.b.1).abs() < TOLL
    }
    fn verticale(&self) -> bool {
        (self.a.0 - self.b.0).abs() < TOLL
    }
    /// Il punto sta sul segmento (estremi compresi).
    fn contiene(&self, p: P) -> bool {
        let (x0, x1) = (self.a.0.min(self.b.0), self.a.0.max(self.b.0));
        let (y0, y1) = (self.a.1.min(self.b.1), self.a.1.max(self.b.1));
        if self.orizzontale() {
            (p.1 - self.a.1).abs() < TOLL && p.0 > x0 - TOLL && p.0 < x1 + TOLL
        } else if self.verticale() {
            (p.0 - self.a.0).abs() < TOLL && p.1 > y0 - TOLL && p.1 < y1 + TOLL
        } else {
            false
        }
    }
    fn estremo(&self, p: P) -> bool {
        vicini(self.a, p) || vicini(self.b, p)
    }
}

fn vicini(a: P, b: P) -> bool {
    (a.0 - b.0).abs() < TOLL && (a.1 - b.1).abs() < TOLL
}

/// Il punto `verso` reso ortogonale rispetto a `da` (orizzontale o verticale, il più vicino).
pub fn ortogonale(da: P, verso: P) -> P {
    if (verso.0 - da.0).abs() >= (verso.1 - da.1).abs() { (verso.0, da.1) } else { (da.0, verso.1) }
}

/// I fili di un layer (foglio).
pub fn fili(s: &Session, layer: &str) -> Result<Vec<Segmento>> {
    let d = s.doc()?;
    Ok(d.model
        .iter()
        .filter(|e| e.common.layer.eq_ignore_ascii_case(layer) && e.common.color != cadcraft_engine::doc::color::Color::Index(COLORE_IGNORATO))
        .filter_map(|e| match &e.kind {
            EntityKind::Line(l) => Some(Segmento { a: (l.a.x, l.a.y), b: (l.b.x, l.b.y), handle: e.handle.hex() }),
            _ => None,
        })
        .collect())
}

/// Le posizioni dei blocchi `nome` su un layer.
fn inserimenti(s: &Session, layer: &str, nome: &str) -> Result<Vec<P>> {
    let d = s.doc()?;
    Ok(d.model
        .iter()
        .filter(|e| e.common.layer.eq_ignore_ascii_case(layer))
        .filter_map(|e| match &e.kind {
            EntityKind::Insert(i) if i.block.eq_ignore_ascii_case(nome) => Some((i.insert.x, i.insert.y)),
            _ => None,
        })
        .collect())
}

// ---------- blocchi ----------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TipoRimando {
    Partenza,
    Arrivo,
}

impl TipoRimando {
    fn da_testo(t: &str) -> Option<Self> {
        match t.trim().to_ascii_lowercase().chars().next()? {
            'p' => Some(Self::Partenza),
            'a' => Some(Self::Arrivo),
            _ => None,
        }
    }
    fn pres(self) -> &'static str {
        match self {
            Self::Partenza => "P",
            Self::Arrivo => "A",
        }
    }
}

/// Rimandi per alimentazioni (`_ALIM_n`) o per collegamenti (`_SIGN_n`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Famiglia {
    Alimentazione,
    Collegamento,
}

impl Famiglia {
    fn prefisso(self) -> &'static str {
        match self {
            Self::Alimentazione => "_ALIM_",
            Self::Collegamento => "_SIGN_",
        }
    }
    fn da_nome(nome: &str) -> Option<Self> {
        let n = nome.to_ascii_uppercase();
        if n.starts_with("_ALIM_") {
            Some(Self::Alimentazione)
        } else if n.starts_with("_SIGN_") {
            Some(Self::Collegamento)
        } else {
            None
        }
    }
}

/// Suffisso del blocco di rimando per tipo e rotazione: 0/1 a 0°, 2/3 a 90°, 20/21 a 180°,
/// 22/23 a 270° (partenza/arrivo).
fn suffisso(tipo: TipoRimando, rotazione: f64) -> u32 {
    let base = match gradi(rotazione) {
        90 => 2,
        180 => 20,
        270 => 22,
        _ => 0,
    };
    base + u32::from(tipo == TipoRimando::Arrivo)
}

/// La rotazione ridotta a 0, 90, 180 o 270.
fn gradi(rotazione: f64) -> u32 {
    let g = (rotazione.rem_euclid(360.0) / 90.0).round().rem_euclid(4.0);
    // 0..=3 dopo rem_euclid
    (g as u32) * 90
}

pub fn nome_rimando(famiglia: Famiglia, tipo: TipoRimando, rotazione: f64) -> String {
    format!("{}{}", famiglia.prefisso(), suffisso(tipo, rotazione))
}

/// Ruota (x, y) di `gradi` multipli di 90.
fn ruota(p: P, gradi: u32) -> P {
    match gradi % 360 {
        90 => (-p.1, p.0),
        180 => (-p.0, -p.1),
        270 => (p.1, -p.0),
        _ => p,
    }
}

/// Crea il blocco di rimando `famiglia`/`tipo`/`rotazione` se manca: freccia nella direzione del
/// filo, nome del potenziale (SEGNALE) prima della freccia, riferimento (REG) dopo.
fn crea_rimando(s: &mut Session, famiglia: Famiglia, tipo: TipoRimando, rotazione: f64) -> Result<String> {
    let nome = nome_rimando(famiglia, tipo, rotazione);
    if blocchi_definiti(s)?.iter().any(|b| b.eq_ignore_ascii_case(&nome)) {
        return Ok(nome);
    }
    let g = gradi(rotazione);
    let partenza = tipo == TipoRimando::Partenza;
    // geometria a 0°: partenza = freccia da (0,0) a (4,0); arrivo = da (-4,0) a (0,0)
    let (coda, punta) = if partenza { ((0.0, 0.0), (4.0, 0.0)) } else { ((-4.0, 0.0), (0.0, 0.0)) };
    let (coda, punta) = (ruota(coda, g), ruota(punta, g));
    // testi orizzontali (leggibili) in posizioni che non coprono il filo
    let (segnale, reg) = match (partenza, g) {
        (true, 90) => ((1.5, -12.0), (-2.0, 5.5)),
        (true, 180) => ((2.0, 1.0), (-17.0, -1.0)),
        (true, 270) => ((1.5, 9.0), (-2.0, -8.0)),
        (true, _) => ((-14.0, 1.0), (5.5, -1.0)),
        (false, 90) => ((1.5, 2.0), (-2.0, -8.0)),
        (false, 180) => ((-14.0, 1.0), (5.5, -1.0)),
        (false, 270) => ((1.5, -6.0), (-2.0, 5.5)),
        (false, _) => ((1.0, 1.0), (-17.0, -1.0)),
    };
    // la freccia è larga in coda e a punta nella direzione del filo: per l'arrivo la punta è
    // nel punto d'inserimento, cioè entra nel foglio
    let (larg_coda, larg_punta) = (1.6, 0.0);
    su_layer_zero(s, |s| {
        let mut h = Vec::new();
        colore(s, "1")?;
        let freccia = json!({"vertices": [{"p": [coda.0, coda.1], "startWidth": larg_coda, "endWidth": larg_punta}, {"p": [punta.0, punta.1]}]});
        h.extend(handles(&esegui(s, "pline", freccia)?));
        colore(s, "4")?;
        attr(s, &mut h, "SEGNALE", "Nome del potenziale", segnale, 2.5, false)?;
        attr(s, &mut h, "REG", "Riferimento", reg, 2.5, false)?;
        attr(s, &mut h, "PRES", "Partenza/Arrivo", (0.0, 0.0), 1.0, true)?;
        attr(s, &mut h, "#LINEA", "Descrizione", (0.0, -3.0), 1.8, true)?;
        let descr = format!("Rimando di {} ({g}°)", if partenza { "partenza" } else { "arrivo" });
        blocco(s, &nome, h, &descr)
    })?;
    Ok(nome)
}

fn crea_nodo(s: &mut Session) -> Result<()> {
    if blocchi_definiti(s)?.iter().any(|b| b.eq_ignore_ascii_case(BLOCCO_NODO)) {
        return Ok(());
    }
    su_layer_zero(s, |s| {
        colore(s, "BYBLOCK")?;
        let h = handles(&esegui(s, "donut", json!({"center": [0.0, 0.0], "inside": 0.0, "outside": 1.4}))?);
        blocco(s, BLOCCO_NODO, h, "Connessione (nodo) tra fili")
    })
}

fn attr(s: &mut Session, h: &mut Vec<String>, tag: &str, richiesta: &str, at: P, altezza: f64, nascosto: bool) -> Result<()> {
    let r = esegui(s, "attdef", json!({"tag": tag, "prompt": richiesta, "default": "", "at": [at.0, at.1], "height": altezza, "invisible": nascosto}))?;
    h.extend(handles(&r));
    Ok(())
}

// ---------- operazioni ----------

/// Disegna un filo da `a` a `b` sul layer corrente (DaLayer o del colore dato); nessun filo se i
/// punti coincidono.
fn crea_filo(s: &mut Session, a: P, b: P, col: Option<&str>) -> Result<Option<String>> {
    if vicini(a, b) {
        return Ok(None);
    }
    colore(s, col.unwrap_or("BYLAYER"))?;
    esegui(s, "setvar", json!({"name": "CELWEIGHT", "value": SPESSORE_FILO}))?;
    let mut h = Vec::new();
    linea(s, &mut h, &[a, b], false)?;
    esegui(s, "setvar", json!({"name": "CELWEIGHT", "value": -1}))?;
    colore(s, "BYLAYER")?;
    Ok(h.into_iter().next())
}

/// Il colore di un oggetto come valore di CECOLOR.
fn colore_di(s: &Session, handle: &str) -> Result<String> {
    use cadcraft_engine::doc::color::Color;
    let h = cadcraft_engine::doc::Handle::parse_hex(handle).ok_or_else(|| errore(format!("handle non valido: {handle}")))?;
    let e = s.doc()?.entity(h).ok_or_else(|| errore(format!("oggetto {handle} non trovato")))?;
    Ok(match e.common.color {
        Color::ByBlock => "BYBLOCK".into(),
        Color::Index(n) => n.to_string(),
        _ => "BYLAYER".into(),
    })
}

/// Interrompe il filo che passa per `a` e per `b` (i morsetti di un simbolo): restano i tratti
/// esterni, con lo stesso colore. Restituisce se c'era un filo da interrompere.
pub fn spezza(s: &mut Session, a: P, b: P) -> Result<bool> {
    let layer = layer_corrente(s)?;
    let Some(f) = fili(s, &layer)?.into_iter().find(|f| f.contiene(a) && f.contiene(b)) else { return Ok(false) };
    let col = colore_di(s, &f.handle)?;
    let d = |p: P, q: P| (p.0 - q.0).hypot(p.1 - q.1);
    // l'estremo del filo dalla parte di `a` e quello dalla parte di `b`
    let (lato_a, lato_b) = if d(f.a, a) <= d(f.a, b) { (f.a, f.b) } else { (f.b, f.a) };
    esegui(s, "erase", json!({"handles": [f.handle]}))?;
    let col = if col == "BYLAYER" { None } else { Some(col.as_str()) };
    crea_filo(s, lato_a, a, col)?;
    crea_filo(s, b, lato_b, col)?;
    Ok(true)
}

/// Inserisce un nodo di connessione in `p` (se non c'è già).
pub fn connessione(s: &mut Session, p: P) -> Result<bool> {
    let layer = layer_corrente(s)?;
    if inserimenti(s, &layer, BLOCCO_NODO)?.iter().any(|q| vicini(*q, p)) {
        return Ok(false);
    }
    crea_nodo(s)?;
    esegui(s, "insert", json!({"name": BLOCCO_NODO, "at": [p.0, p.1]}))?;
    Ok(true)
}

/// Collegamento automatico: un nodo dove un filo nuovo finisce a T su un altro filo, o dove
/// si incontrano tre o più fili. Restituisce quanti nodi ha messo.
fn connessioni_automatiche(s: &mut Session, punti: &[P]) -> Result<usize> {
    let layer = layer_corrente(s)?;
    let tutti = fili(s, &layer)?;
    let mut messi = 0;
    for &p in punti {
        let rami: usize = tutti.iter().filter(|f| f.contiene(p)).map(|f| if f.estremo(p) { 1 } else { 2 }).sum();
        if rami >= 3 && connessione(s, p)? {
            messi += 1;
        }
    }
    Ok(messi)
}

/// Un filo ortogonale per i punti dati (ogni punto è reso ortogonale al precedente).
pub fn filo(s: &mut Session, punti: &[P]) -> Result<Vec<String>> {
    let mut creati = Vec::new();
    let mut estremi = Vec::new();
    let mut prec = match punti.first() {
        Some(p) => *p,
        None => return Ok(creati),
    };
    for &q in punti.iter().skip(1) {
        let q = ortogonale(prec, q);
        if let Some(h) = crea_filo(s, prec, q, COLORE_FILO)? {
            creati.push(h);
            estremi.extend([prec, q]);
        }
        prec = q;
    }
    connessioni_automatiche(s, &estremi)?;
    Ok(creati)
}

/// Rotazione (gradi) della direzione da `a` a `b`, ridotta a 0/90/180/270.
fn angolo(a: P, b: P) -> f64 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    if dx.abs() >= dy.abs() {
        if dx >= 0.0 { 0.0 } else { 180.0 }
    } else if dy >= 0.0 {
        90.0
    } else {
        270.0
    }
}

/// Inserisce un rimando in `p`. Senza rotazione, la prende dal filo che finisce in `p`.
pub fn rimando(s: &mut Session, famiglia: Famiglia, tipo: TipoRimando, p: P, segnale: &str, descrizione: &str, rotazione: Option<f64>) -> Result<String> {
    let rot = match rotazione {
        Some(r) => r,
        None => {
            let layer = layer_corrente(s)?;
            let altro = fili(s, &layer)?.into_iter().find_map(|f| {
                if vicini(f.a, p) {
                    Some(f.b)
                } else if vicini(f.b, p) {
                    Some(f.a)
                } else {
                    None
                }
            });
            match (tipo, altro) {
                (TipoRimando::Partenza, Some(o)) => angolo(o, p),
                (TipoRimando::Arrivo, Some(o)) => angolo(p, o),
                (_, None) => 0.0,
            }
        }
    };
    let nome = crea_rimando(s, famiglia, tipo, rot)?;
    let attr = json!({"PRES": tipo.pres(), "SEGNALE": segnale, "#LINEA": descrizione, "REG": ""});
    let r = esegui(s, "insert", json!({"name": nome, "at": [p.0, p.1], "attribs": attr}))?;
    handles(&r).into_iter().next().ok_or_else(|| errore("rimando non inserito"))
}

/// Nomi predefiniti dei potenziali per `n` fili.
pub fn nomi_predefiniti(n: u32) -> Vec<String> {
    let base: &[&str] = match n {
        1 => &["L1"],
        2 => &["L1", "N"],
        3 => &["L1", "L2", "L3"],
        4 => &["L1", "L2", "L3", "N"],
        5 => &["L1", "L2", "L3", "N", "PE"],
        _ => &[],
    };
    (0..n).map(|i| base.get(i as usize).map_or_else(|| format!("F{}", i + 1), |s| (*s).to_string())).collect()
}

/// Come terminano i fili di un'alimentazione.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Estremo {
    Niente,
    Nome,
    Partenza,
    Arrivo,
}

impl Estremo {
    fn da_testo(t: &str) -> Option<Self> {
        match t.to_ascii_lowercase().as_str() {
            "nessuno" | "niente" => Some(Self::Niente),
            "nome" => Some(Self::Nome),
            "partenza" => Some(Self::Partenza),
            "arrivo" => Some(Self::Arrivo),
            _ => None,
        }
    }
}

/// N fili paralleli da `da` a `a` (orizzontali o verticali), uno sotto l'altro (o a destra),
/// con nome all'inizio e frecce di partenza alla fine (o come indicato).
pub fn alimentazioni(s: &mut Session, nomi: &[String], da: P, a: P, passo: f64, inizio: Estremo, fine: Estremo) -> Result<Vec<String>> {
    let a = ortogonale(da, a);
    let orizz = (a.1 - da.1).abs() < TOLL;
    let mut creati = Vec::new();
    for (i, nome) in nomi.iter().enumerate() {
        let k = i as f64 * passo;
        let (p0, p1) = if orizz { ((da.0, da.1 - k), (a.0, a.1 - k)) } else { ((da.0 + k, da.1), (a.0 + k, a.1)) };
        // il conduttore di protezione col suo colore, gli altri rossi
        let col = if nome.eq_ignore_ascii_case("PE") { Some(COLORE_PE) } else { COLORE_ALIMENTAZIONE };
        let Some(h) = crea_filo(s, p0, p1, col)? else { continue };
        creati.push(h);
        for (estremo, p, altro) in [(inizio, p0, p1), (fine, p1, p0)] {
            match estremo {
                Estremo::Niente => {}
                Estremo::Nome => {
                    // il nome prima dell'inizio del filo, sulla sua linea
                    let giu = if orizz { if p.0 <= altro.0 { "MR" } else { "ML" } } else if p.1 >= altro.1 { "BC" } else { "TC" };
                    let off = if orizz { (if p.0 <= altro.0 { -1.5 } else { 1.5 }, 0.0) } else { (0.0, if p.1 >= altro.1 { 1.5 } else { -1.5 }) };
                    colore(s, "4")?;
                    esegui(s, "text", json!({"at": [p.0 + off.0, p.1 + off.1], "text": nome, "height": 2.5, "justify": giu}))?;
                    colore(s, "BYLAYER")?;
                }
                Estremo::Partenza => {
                    rimando(s, Famiglia::Alimentazione, TipoRimando::Partenza, p, nome, "", Some(angolo(altro, p)))?;
                }
                Estremo::Arrivo => {
                    rimando(s, Famiglia::Alimentazione, TipoRimando::Arrivo, p, nome, "", Some(angolo(p, altro)))?;
                }
            }
        }
    }
    aggiorna_rimandi(s)?;
    Ok(creati)
}

/// I vertici del filo `i` (0 = il percorso stesso) di un collegamento a più fili: sui tratti
/// orizzontali i fili scendono di `passo`, su quelli verticali vanno a destra di `passo`.
pub fn percorso_parallelo(percorso: &[P], i: u32, passo: f64) -> Vec<P> {
    let k = f64::from(i) * passo;
    let n = percorso.len();
    (0..n)
        .filter_map(|j| {
            let p = *percorso.get(j)?;
            let vicino = |q: Option<&P>| q.map(|q| ((q.1 - p.1).abs() < TOLL, (q.0 - p.0).abs() < TOLL));
            let (prima, dopo) = (vicino(j.checked_sub(1).and_then(|m| percorso.get(m))), vicino(percorso.get(j + 1)));
            let orizz = prima.is_some_and(|(o, _)| o) || dopo.is_some_and(|(o, _)| o);
            let vert = prima.is_some_and(|(_, v)| v) || dopo.is_some_and(|(_, v)| v);
            Some((p.0 + if vert { k } else { 0.0 }, p.1 - if orizz { k } else { 0.0 }))
        })
        .collect()
}

/// N fili paralleli lungo un percorso ortogonale.
pub fn collegamenti(s: &mut Session, n: u32, punti: &[P], passo: f64) -> Result<Vec<String>> {
    let mut percorso: Vec<P> = Vec::new();
    for &q in punti {
        let q = percorso.last().map_or(q, |&prec| ortogonale(prec, q));
        if percorso.last().is_none_or(|&prec| !vicini(prec, q)) {
            percorso.push(q);
        }
    }
    let mut creati = Vec::new();
    let mut estremi = Vec::new();
    for i in 0..n {
        let vertici = percorso_parallelo(&percorso, i, passo);
        for w in vertici.windows(2) {
            if let [a, b] = w
                && let Some(h) = crea_filo(s, *a, *b, COLORE_FILO)?
            {
                creati.push(h);
            }
        }
        estremi.extend(vertici.first().copied());
        estremi.extend(vertici.last().copied());
    }
    connessioni_automatiche(s, &estremi)?;
    Ok(creati)
}

/// Distanze (crescenti) dei fili incontrati, saltando il fascio di fili paralleli vicini (a non
/// più di un passo l'uno dall'altro) da cui si parte: il primo dopo il fascio è l'arrivo.
fn primo_oltre_il_fascio(mut distanze: Vec<f64>) -> Option<f64> {
    distanze.sort_by(f64::total_cmp);
    let mut prec = 0.0;
    for d in distanze {
        if d - prec > PASSO + TOLL {
            return Some(d);
        }
        prec = d;
    }
    None
}

/// Quick filo: da `p` in verticale verso il basso (o in orizzontale verso destra) fino al primo
/// filo oltre il fascio di partenza, altrimenti fino al cartiglio (o al bordo della cornice).
/// I fili attraversati non vengono collegati.
pub fn filo_rapido(s: &mut Session, p: P, verticale: bool) -> Result<Option<String>> {
    let layer = layer_corrente(s)?;
    let tutti = fili(s, &layer)?;
    let fine = if verticale {
        let d = tutti
            .iter()
            .filter(|f| f.orizzontale() && f.a.1 < p.1 - TOLL && p.0 > f.a.0.min(f.b.0) - TOLL && p.0 < f.a.0.max(f.b.0) + TOLL)
            .map(|f| p.1 - f.a.1)
            .collect();
        (p.0, primo_oltre_il_fascio(d).map_or(CORNICE_MIN.1 + CARTIGLIO_ALTEZZA, |d| p.1 - d))
    } else {
        let d = tutti
            .iter()
            .filter(|f| f.verticale() && f.a.0 > p.0 + TOLL && p.1 > f.a.1.min(f.b.1) - TOLL && p.1 < f.a.1.max(f.b.1) + TOLL)
            .map(|f| f.a.0 - p.0)
            .collect();
        (primo_oltre_il_fascio(d).map_or(CORNICE_MAX.0, |d| p.0 + d), p.1)
    };
    let h = crea_filo(s, p, fine, COLORE_FILO)?;
    connessioni_automatiche(s, &[p, fine])?;
    Ok(h)
}

#[derive(Clone, Debug)]
pub struct Rimando {
    pub handle: String,
    pub famiglia: Famiglia,
    pub partenza: bool,
    pub foglio: u32,
    pub pos: P,
    pub segnale: String,
    /// Descrizione della linea (`#LINEA`): due rimandi si collegano solo se coincide anche questa.
    pub linea: String,
    pub riferimento: String,
}

/// Riferimento scritto su un rimando senza corrispondenza.
pub const SENZA_COPPIA: &str = "///";

impl Rimando {
    fn da_blocco(b: &Inserito) -> Option<Self> {
        let famiglia = Famiglia::da_nome(&b.nome)?;
        // PRES dice partenza/arrivo; in mancanza vale il suffisso (pari = partenza)
        let partenza = match b.attributo("PRES").to_ascii_uppercase().as_str() {
            "P" => true,
            "A" => false,
            _ => b.nome.rsplit('_').next().and_then(|n| n.parse::<u32>().ok()).is_some_and(|n| n % 2 == 0),
        };
        Some(Rimando {
            handle: b.handle.clone(),
            famiglia,
            partenza,
            foglio: fogli::numero_da_layer(&b.layer).unwrap_or(0),
            pos: b.pos,
            segnale: b.attributo("SEGNALE").to_string(),
            linea: b.attributo("#LINEA").to_string(),
            riferimento: b.attributo("REG").to_string(),
        })
    }

    fn stessa_linea(&self, o: &Rimando) -> bool {
        self.famiglia == o.famiglia && self.segnale.trim().eq_ignore_ascii_case(o.segnale.trim()) && self.linea.trim().eq_ignore_ascii_case(o.linea.trim())
    }
}

/// Abbina partenze e arrivi uno a uno. Ogni partenza (in ordine di foglio, dall'alto in basso)
/// prende il primo arrivo libero della stessa linea nei fogli seguenti; se non ce n'è, il più
/// vicino nei fogli precedenti; per ultimo, uno sullo stesso foglio. Restituisce le coppie
/// (indice partenza, indice arrivo).
fn abbina(tutti: &[Rimando]) -> Vec<(usize, usize)> {
    let mut partenze: Vec<usize> = (0..tutti.len()).filter(|&i| tutti[i].partenza).collect();
    let ordine = |i: &usize| {
        let r = &tutti[*i];
        (r.foglio, std::cmp::Reverse((r.pos.1 * 1000.0).round() as i64), (r.pos.0 * 1000.0).round() as i64)
    };
    partenze.sort_by_key(ordine);
    let mut liberi: Vec<usize> = (0..tutti.len()).filter(|&i| !tutti[i].partenza).collect();
    liberi.sort_by_key(ordine);
    let mut coppie = Vec::new();
    for p in partenze {
        let rp = &tutti[p];
        let scelta = {
            let candidati = || liberi.iter().copied().enumerate().filter(|(_, a)| tutti[*a].stessa_linea(rp));
            let dopo = candidati().filter(|(_, a)| tutti[*a].foglio > rp.foglio).min_by_key(|(_, a)| tutti[*a].foglio);
            let prima = || candidati().filter(|(_, a)| tutti[*a].foglio < rp.foglio).max_by_key(|(_, a)| tutti[*a].foglio);
            let stesso = || candidati().find(|(_, a)| tutti[*a].foglio == rp.foglio);
            dopo.or_else(prima).or_else(stesso)
        };
        if let Some((k, a)) = scelta {
            coppie.push((p, a));
            liberi.remove(k);
        }
    }
    coppie
}

/// Tutti i rimandi del disegno.
pub fn rimandi(s: &mut Session) -> Result<Vec<Rimando>> {
    Ok(blocchi_inseriti(s, "_ALIM_*,_SIGN_*")?.iter().filter_map(Rimando::da_blocco).collect())
}

/// Collega i rimandi di partenza e di arrivo (stessa famiglia, segnale e descrizione), uno a
/// uno (vedi [`abbina`]): ogni freccia riporta in REG `foglio/colonnaRiga` dell'altra, o `///`
/// se è rimasta senza coppia. Restituisce (rimandi collegati, descrizione di quelli senza coppia).
pub fn aggiorna_rimandi(s: &mut Session) -> Result<(usize, Vec<String>)> {
    let tutti = rimandi(s)?;
    let mut rif: Vec<String> = vec![SENZA_COPPIA.to_string(); tutti.len()];
    let coppie = abbina(&tutti);
    for &(p, a) in &coppie {
        let (rp, ra) = (&tutti[p], &tutti[a]);
        rif[p] = squadratura::riferimento(ra.foglio, ra.pos.0, ra.pos.1);
        rif[a] = squadratura::riferimento(rp.foglio, rp.pos.0, rp.pos.1);
    }
    let soli: Vec<String> = tutti
        .iter()
        .zip(&rif)
        .filter(|(_, r)| *r == SENZA_COPPIA)
        .map(|(r, _)| format!("{} {} (foglio {})", if r.partenza { "partenza" } else { "arrivo" }, r.segnale, r.foglio))
        .collect();
    let collegati = coppie.len() * 2;
    let nuovi: Vec<(String, String)> = tutti.iter().zip(rif).filter(|(r, nuovo)| r.riferimento != *nuovo).map(|(r, nuovo)| (r.handle.clone(), nuovo)).collect();
    for (h, rif) in nuovi {
        esegui(s, "attedit", json!({"handle": h, "values": {"REG": rif}}))?;
    }
    Ok((collegati, soli))
}

// ---------- comandi ----------

fn punti_param(p: &Value, chiave: &str) -> Vec<P> {
    p.get(chiave)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|q| {
            let a = q.as_array()?;
            Some((a.first()?.as_f64()?, a.get(1)?.as_f64()?))
        })
        .filter(|(x, y)| x.is_finite() && y.is_finite())
        .collect()
}

fn nomi_param(p: &Value, n: u32) -> Vec<String> {
    match p.get("nomi") {
        Some(Value::Array(a)) => a.iter().filter_map(Value::as_str).map(str::to_string).collect(),
        Some(Value::String(t)) => t.split([',', ' ', ';']).filter(|x| !x.is_empty()).map(str::to_string).collect(),
        _ => nomi_predefiniti(n),
    }
}

fn numero_fili(p: &Value) -> Result<u32> {
    let n = numero_param(p, "fili").unwrap_or(3);
    if n == 0 || n > MAX_FILI {
        return Err(errore(format!("Numero di fili non valido: {n} (da 1 a {MAX_FILI})")));
    }
    Ok(n)
}

fn run_filo(s: &mut Session, p: &Value) -> Result<Value> {
    let punti = punti_param(p, "points");
    if punti.len() < 2 {
        return Err(errore("Servono almeno due punti"));
    }
    in_gruppo(s, "Filo", |s| filo(s, &punti).map(|h| json!({"handles": h})))
}

fn run_connessione(s: &mut Session, p: &Value) -> Result<Value> {
    let at = punto_param(p, "at").ok_or_else(|| errore("Indicare il punto"))?;
    in_gruppo(s, "Connessione", |s| connessione(s, at).map(|messa| json!({"inserita": messa})))
}

fn run_alimentazioni(s: &mut Session, p: &Value) -> Result<Value> {
    // con i nomi e senza il numero di fili, i fili sono tanti quanti i nomi
    let n = match (p.get("fili"), p.get("nomi")) {
        (None, Some(_)) => u32::try_from(nomi_param(p, 0).len()).unwrap_or(0).clamp(1, MAX_FILI),
        _ => numero_fili(p)?,
    };
    let nomi = nomi_param(p, n);
    if nomi.len() != n as usize {
        return Err(errore(format!("Servono {n} nomi di potenziale, ne sono stati dati {}", nomi.len())));
    }
    let da = punto_param(p, "da").ok_or_else(|| errore("Indicare il punto iniziale (da)"))?;
    let a = punto_param(p, "a").ok_or_else(|| errore("Indicare il punto finale (a)"))?;
    let passo = p.get("passo").and_then(Value::as_f64).filter(|x| x.is_finite() && *x > 0.0).unwrap_or(PASSO);
    let inizio = testo_param(p, "inizio").map_or(Some(Estremo::Nome), |t| Estremo::da_testo(&t)).ok_or_else(|| errore("inizio: nessuno|nome|partenza|arrivo"))?;
    let fine = testo_param(p, "fine").map_or(Some(Estremo::Partenza), |t| Estremo::da_testo(&t)).ok_or_else(|| errore("fine: nessuno|nome|partenza|arrivo"))?;
    in_gruppo(s, "Alimentazioni", |s| alimentazioni(s, &nomi, da, a, passo, inizio, fine).map(|h| json!({"handles": h, "message": format!("Alimentazione {}", nomi.join(" "))})))
}

fn run_collegamenti(s: &mut Session, p: &Value) -> Result<Value> {
    let n = numero_fili(p)?;
    let punti = punti_param(p, "points");
    if punti.len() < 2 {
        return Err(errore("Servono almeno due punti"));
    }
    let passo = p.get("passo").and_then(Value::as_f64).filter(|x| x.is_finite() && *x > 0.0).unwrap_or(PASSO);
    in_gruppo(s, "Collegamenti", |s| collegamenti(s, n, &punti, passo).map(|h| json!({"handles": h})))
}

fn run_rimando(s: &mut Session, p: &Value, famiglia: Famiglia) -> Result<Value> {
    let at = punto_param(p, "at").ok_or_else(|| errore("Indicare il punto"))?;
    let tipo = testo_param(p, "tipo").map_or(Some(TipoRimando::Partenza), |t| TipoRimando::da_testo(&t)).ok_or_else(|| errore("tipo: partenza|arrivo"))?;
    let pot = testo_param(p, "segnale").filter(|t| !t.trim().is_empty()).ok_or_else(|| errore("Indicare il segnale (nome del potenziale o numero filo)"))?;
    let descr = testo_param(p, "descrizione").unwrap_or_default();
    let rot = p.get("rotazione").and_then(Value::as_f64).filter(|r| r.is_finite());
    in_gruppo(s, "Rimando", |s| {
        let h = rimando(s, famiglia, tipo, at, &pot, &descr, rot)?;
        let (_, soli) = aggiorna_rimandi(s)?;
        let rif = rimandi(s)?.into_iter().find(|r| r.handle == h).map(|r| r.riferimento).unwrap_or_default();
        let msg = if rif.is_empty() || rif == SENZA_COPPIA { format!("Rimando {pot}: ancora senza corrispondenza") } else { format!("Rimando {pot} → {rif}") };
        Ok(json!({"handle": h, "riferimento": rif, "senzaCoppia": soli.len(), "message": msg}))
    })
}

fn run_rimando_alim(s: &mut Session, p: &Value) -> Result<Value> {
    run_rimando(s, p, Famiglia::Alimentazione)
}

fn run_rimando_coll(s: &mut Session, p: &Value) -> Result<Value> {
    run_rimando(s, p, Famiglia::Collegamento)
}

fn run_rapido(s: &mut Session, p: &Value, verticale: bool) -> Result<Value> {
    let at = punto_param(p, "at").ok_or_else(|| errore("Indicare il punto"))?;
    in_gruppo(s, if verticale { "Filo rapido verticale" } else { "Filo rapido orizzontale" }, |s| filo_rapido(s, at, verticale).map(|h| json!({"handle": h})))
}

fn run_vv(s: &mut Session, p: &Value) -> Result<Value> {
    run_rapido(s, p, true)
}

fn run_hh(s: &mut Session, p: &Value) -> Result<Value> {
    run_rapido(s, p, false)
}

fn run_aggiorna(s: &mut Session, _p: &Value) -> Result<Value> {
    in_gruppo(s, "Aggiorna rimandi", |s| {
        let (ok, soli) = aggiorna_rimandi(s)?;
        let mut msg = format!("Rimandi collegati: {ok}");
        if !soli.is_empty() {
            msg.push_str(&format!("\nSenza corrispondenza: {}", soli.join(", ")));
        }
        Ok(json!({"collegati": ok, "senzaCoppia": soli, "message": msg}))
    })
}

// ---------- comandi interattivi ----------

fn vec2(p: P) -> Vec2 {
    Vec2::new(p.0, p.1)
}

fn linea_anteprima(a: P, b: P) -> EntityKind {
    EntityKind::Line(Line { a: Vec3::new(a.0, a.1, 0.0), b: Vec3::new(b.0, b.1, 0.0) })
}

fn da_vec(v: Vec2) -> P {
    (v.x, v.y)
}

/// FILO: punti successivi, ogni tratto ortogonale; Invio per finire.
struct DisegnaFilo {
    ultimo: Option<P>,
}

impl Interactive for DisegnaFilo {
    fn name(&self) -> &'static str {
        "filo"
    }
    fn prompt(&self, _s: &Session) -> Prompt {
        match self.ultimo {
            None => Prompt::new("Punto iniziale del filo", Accept::POINT),
            Some(p) => Prompt::new("Punto successivo (Invio per finire)", Accept::POINT).base(vec2(p)),
        }
    }
    fn input(&mut self, s: &mut Session, i: Input) -> Result<Step> {
        match i {
            Input::Point(q) => {
                let q = da_vec(q);
                if let Some(p) = self.ultimo {
                    let q = ortogonale(p, q);
                    s.execute("filo", &json!({"points": [[p.0, p.1], [q.0, q.1]]}))?;
                    self.ultimo = Some(q);
                } else {
                    self.ultimo = Some(q);
                }
                Ok(Step::Continue)
            }
            Input::Enter => Ok(Step::Done),
            Input::Cancel => Ok(Step::Cancel),
            _ => Ok(Step::Continue),
        }
    }
    fn preview(&self, _s: &Session, cursore: Vec2) -> Vec<EntityKind> {
        self.ultimo.map(|p| vec![linea_anteprima(p, ortogonale(p, da_vec(cursore)))]).unwrap_or_default()
    }
}

fn fabbrica_filo(_s: &Session) -> Result<Box<dyn Interactive>> {
    Ok(Box::new(DisegnaFilo { ultimo: None }))
}

/// ALIMENTAZIONI e COLLEGAMENTI: numero di fili, (nomi), poi punti.
struct MultiFilo {
    alimentazione: bool,
    fili: Option<u32>,
    nomi: Option<Vec<String>>,
    punti: Vec<P>,
}

impl MultiFilo {
    fn anteprima_percorso(&self, cursore: P) -> Vec<P> {
        let mut v = self.punti.clone();
        if let Some(&p) = v.last() {
            v.push(ortogonale(p, cursore));
        }
        v
    }
}

impl Interactive for MultiFilo {
    fn name(&self) -> &'static str {
        if self.alimentazione { "alimentazioni" } else { "collegamenti" }
    }
    fn prompt(&self, _s: &Session) -> Prompt {
        match (self.fili, &self.nomi, self.punti.last()) {
            (None, _, _) => Prompt { default: Some("3".into()), ..Prompt::new("Numero di fili", Accept::NUMBER) },
            (Some(n), None, _) if self.alimentazione => {
                Prompt { default: Some(nomi_predefiniti(n).join(" ")), ..Prompt::new("Nomi dei potenziali (separati da spazi)", Accept::TEXT) }
            }
            (_, _, None) => Prompt::new("Punto iniziale", Accept::POINT),
            (_, _, Some(&p)) if self.alimentazione => Prompt::new("Punto finale", Accept::POINT).base(vec2(p)),
            (_, _, Some(&p)) => Prompt::new("Punto successivo (Invio per finire)", Accept::POINT).base(vec2(p)),
        }
    }
    fn input(&mut self, s: &mut Session, i: Input) -> Result<Step> {
        if let Input::Cancel = i {
            return Ok(Step::Cancel);
        }
        if self.fili.is_none() {
            let t = match i {
                Input::Text(t) => t,
                Input::Enter => "3".into(),
                _ => return Ok(Step::Continue),
            };
            let n: u32 = t.trim().parse().map_err(|_| errore("Indicare un numero di fili"))?;
            if n == 0 || n > MAX_FILI {
                return Err(errore(format!("Da 1 a {MAX_FILI} fili")));
            }
            self.fili = Some(n);
            return Ok(Step::Continue);
        }
        let n = self.fili.unwrap_or(3);
        if self.alimentazione && self.nomi.is_none() {
            let nomi = match i {
                Input::Text(t) => t.split([',', ' ', ';']).filter(|x| !x.is_empty()).map(str::to_string).collect::<Vec<_>>(),
                Input::Enter => nomi_predefiniti(n),
                _ => return Ok(Step::Continue),
            };
            if nomi.len() != n as usize {
                return Err(errore(format!("Servono {n} nomi")));
            }
            self.nomi = Some(nomi);
            return Ok(Step::Continue);
        }
        match i {
            Input::Point(q) => {
                let q = da_vec(q);
                let q = self.punti.last().map_or(q, |&p| ortogonale(p, q));
                self.punti.push(q);
                if self.alimentazione && self.punti.len() == 2 {
                    let (da, a) = (self.punti[0], self.punti[1]);
                    let r = s.execute("alimentazioni", &json!({"fili": n, "nomi": self.nomi.clone().unwrap_or_default(), "da": [da.0, da.1], "a": [a.0, a.1]}))?;
                    if let Some(m) = r.get("message").and_then(Value::as_str) {
                        s.echo(m.to_string());
                    }
                    return Ok(Step::Done);
                }
                Ok(Step::Continue)
            }
            Input::Enter if !self.alimentazione => {
                if self.punti.len() >= 2 {
                    let punti: Vec<Value> = self.punti.iter().map(|p| json!([p.0, p.1])).collect();
                    s.execute("collegamenti", &json!({"fili": n, "points": punti}))?;
                }
                Ok(Step::Done)
            }
            _ => Ok(Step::Continue),
        }
    }
    fn preview(&self, _s: &Session, cursore: Vec2) -> Vec<EntityKind> {
        let Some(n) = self.fili else { return Vec::new() };
        let percorso = self.anteprima_percorso(da_vec(cursore));
        if percorso.len() < 2 {
            return Vec::new();
        }
        let mut v = Vec::new();
        for i in 0..n {
            let vertici = if self.alimentazione {
                let (da, a) = (percorso[0], percorso[percorso.len() - 1]);
                let k = f64::from(i) * PASSO;
                if (a.1 - da.1).abs() < TOLL { vec![(da.0, da.1 - k), (a.0, a.1 - k)] } else { vec![(da.0 + k, da.1), (a.0 + k, a.1)] }
            } else {
                percorso_parallelo(&percorso, i, PASSO)
            };
            v.extend(vertici.windows(2).filter_map(|w| match w {
                [a, b] => Some(linea_anteprima(*a, *b)),
                _ => None,
            }));
        }
        v
    }
}

fn fabbrica_alimentazioni(_s: &Session) -> Result<Box<dyn Interactive>> {
    Ok(Box::new(MultiFilo { alimentazione: true, fili: None, nomi: None, punti: Vec::new() }))
}

fn fabbrica_collegamenti(_s: &Session) -> Result<Box<dyn Interactive>> {
    Ok(Box::new(MultiFilo { alimentazione: false, fili: None, nomi: None, punti: Vec::new() }))
}

/// RIMANDI: tipo, segnale (nome del potenziale o numero filo), punto (estremo del filo).
struct ChiediRimando {
    id: &'static str,
    tipo: Option<TipoRimando>,
    potenziale: Option<String>,
}

impl Interactive for ChiediRimando {
    fn name(&self) -> &'static str {
        self.id
    }
    fn prompt(&self, _s: &Session) -> Prompt {
        match (&self.tipo, &self.potenziale) {
            (None, _) => Prompt { default: Some("Partenza".into()), ..Prompt::new("Tipo di rimando", Accept::TEXT).kw(&["Partenza", "Arrivo"]) },
            (Some(_), None) => Prompt::new("Segnale (nome del potenziale o numero filo)", Accept::TEXT),
            (Some(_), Some(_)) => Prompt::new("Estremo del filo", Accept::POINT),
        }
    }
    fn input(&mut self, s: &mut Session, i: Input) -> Result<Step> {
        if let Input::Cancel = i {
            return Ok(Step::Cancel);
        }
        if self.tipo.is_none() {
            let t = match i {
                Input::Text(t) | Input::Keyword(t) => t,
                Input::Enter => "Partenza".into(),
                _ => return Ok(Step::Continue),
            };
            self.tipo = Some(TipoRimando::da_testo(&t).ok_or_else(|| errore("Partenza o Arrivo"))?);
            return Ok(Step::Continue);
        }
        if self.potenziale.is_none() {
            if let Input::Text(t) = i
                && !t.trim().is_empty()
            {
                self.potenziale = Some(t.trim().to_string());
            }
            return Ok(Step::Continue);
        }
        if let Input::Point(q) = i {
            let tipo = if self.tipo == Some(TipoRimando::Arrivo) { "arrivo" } else { "partenza" };
            let r = s.execute(self.id, &json!({"at": [q.x, q.y], "tipo": tipo, "segnale": self.potenziale.clone().unwrap_or_default()}))?;
            if let Some(m) = r.get("message").and_then(Value::as_str) {
                s.echo(m.to_string());
            }
            return Ok(Step::Done);
        }
        Ok(Step::Continue)
    }
}

fn fabbrica_rimando_alim(_s: &Session) -> Result<Box<dyn Interactive>> {
    Ok(Box::new(ChiediRimando { id: "rimandialimentazioni", tipo: None, potenziale: None }))
}

fn fabbrica_rimando_coll(_s: &Session) -> Result<Box<dyn Interactive>> {
    Ok(Box::new(ChiediRimando { id: "rimandicollegamenti", tipo: None, potenziale: None }))
}

/// Comandi a un solo punto (connessione, VV, HH), ripetuti fino a Invio.
struct UnPunto {
    nome: &'static str,
    messaggio: &'static str,
}

impl Interactive for UnPunto {
    fn name(&self) -> &'static str {
        self.nome
    }
    fn prompt(&self, _s: &Session) -> Prompt {
        Prompt::new(self.messaggio, Accept::POINT)
    }
    fn input(&mut self, s: &mut Session, i: Input) -> Result<Step> {
        match i {
            Input::Point(q) => {
                s.execute(self.nome, &json!({"at": [q.x, q.y]}))?;
                Ok(Step::Continue)
            }
            Input::Enter => Ok(Step::Done),
            Input::Cancel => Ok(Step::Cancel),
            _ => Ok(Step::Continue),
        }
    }
}

fn fabbrica_connessione(_s: &Session) -> Result<Box<dyn Interactive>> {
    Ok(Box::new(UnPunto { nome: "connessione", messaggio: "Punto di connessione (Invio per finire)" }))
}

fn fabbrica_vv(_s: &Session) -> Result<Box<dyn Interactive>> {
    Ok(Box::new(UnPunto { nome: "vv", messaggio: "Punto di partenza del filo verticale (Invio per finire)" }))
}

fn fabbrica_hh(_s: &Session) -> Result<Box<dyn Interactive>> {
    Ok(Box::new(UnPunto { nome: "hh", messaggio: "Punto di partenza del filo orizzontale (Invio per finire)" }))
}

const MENU: &str = "Elettrico";

fn spec(
    id: &'static str,
    label: &'static str,
    voce: &'static [&'static str],
    aliases: &'static [&'static str],
    params: &'static str,
    run: cadcraft_engine::cmd::Run,
    interactive: Option<cadcraft_engine::cmd::Factory>,
) -> CommandSpec {
    CommandSpec { id, label, menu: voce, shortcut: None, aliases, params, enabled: has_doc, run, undoable: false, interactive, transparent: false }
}

pub fn comandi() -> Vec<CommandSpec> {
    vec![
        spec("filo", "Filo", &[MENU, "Filo"], &[], "{points: [[x,y], ...]} (tratti resi ortogonali) → {handles}", run_filo, Some(fabbrica_filo)),
        spec("connessione", "Connessione", &[MENU, "Connessione"], &[], "{at: [x,y]} → {inserita}", run_connessione, Some(fabbrica_connessione)),
        spec(
            "alimentazioni",
            "Alimentazioni",
            &[MENU, "Alimentazioni"],
            &[],
            "{fili?: 1..12 (default 3), nomi?: [..] | \"L1 L2 L3\", da: [x,y], a: [x,y], passo?: mm (10), inizio?: nome|arrivo|partenza|nessuno (nome), fine?: partenza|arrivo|nome|nessuno (partenza)} → {handles}",
            run_alimentazioni,
            Some(fabbrica_alimentazioni),
        ),
        spec(
            "collegamenti",
            "Collegamenti",
            &[MENU, "Collegamenti"],
            &[],
            "{fili?: 1..12 (default 3), points: [[x,y], ...], passo?: mm (10)} → {handles}",
            run_collegamenti,
            Some(fabbrica_collegamenti),
        ),
        spec(
            "rimandialimentazioni",
            "Rimandi per Alimentazioni",
            &[MENU, "Rimandi per Alimentazioni"],
            &[],
            "{at: [x,y], tipo?: partenza|arrivo, segnale, descrizione?, rotazione?: gradi (default dal filo)} → {handle, riferimento}",
            run_rimando_alim,
            Some(fabbrica_rimando_alim),
        ),
        spec(
            "rimandicollegamenti",
            "Rimandi per Collegamenti",
            &[MENU, "Rimandi per Collegamenti"],
            &[],
            "{at: [x,y], tipo?: partenza|arrivo, segnale, descrizione?, rotazione?: gradi (default dal filo)} → {handle, riferimento}",
            run_rimando_coll,
            Some(fabbrica_rimando_coll),
        ),
        spec("vv", "Filo Rapido Verticale (VV)", &[MENU, "Filo Rapido Verticale (VV)"], &[], "{at: [x,y]} → {handle}", run_vv, Some(fabbrica_vv)),
        spec("hh", "Filo Rapido Orizzontale (HH)", &[MENU, "Filo Rapido Orizzontale (HH)"], &[], "{at: [x,y]} → {handle}", run_hh, Some(fabbrica_hh)),
        spec("aggiornarimandi", "Aggiorna Rimandi", &[MENU, "Aggiorna Rimandi"], &[], "{} → {collegati, senzaCoppia}", run_aggiorna, None),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percorso_parallelo_ad_angolo() {
        // giù e poi a destra: il secondo filo sta a destra nel tratto verticale e sotto in quello orizzontale
        let p = [(0.0, 100.0), (0.0, 50.0), (80.0, 50.0)];
        assert_eq!(percorso_parallelo(&p, 0, 5.0), p.to_vec());
        assert_eq!(percorso_parallelo(&p, 1, 5.0), vec![(5.0, 100.0), (5.0, 45.0), (80.0, 45.0)]);
    }

    #[test]
    fn ortogonale_sceglie_l_asse_dominante() {
        assert_eq!(ortogonale((0.0, 0.0), (10.0, 3.0)), (10.0, 0.0));
        assert_eq!(ortogonale((0.0, 0.0), (2.0, -9.0)), (0.0, -9.0));
    }

    #[test]
    fn nomi_dei_potenziali() {
        assert_eq!(nomi_predefiniti(3), ["L1", "L2", "L3"]);
        assert_eq!(nomi_predefiniti(7)[6], "F7");
    }
}
