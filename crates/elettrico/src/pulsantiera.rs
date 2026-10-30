//! Fronte quadro (pulsantiera sulla porta): i dispositivi di comando e di segnalazione del quadro
//! (pulsanti, emergenze, selettori, lampade, sirene) sulla porta disegnata in scala, con la
//! foratura (fori Ø22,5, quote dall'angolo in basso a sinistra della porta).
//!
//! La porta si disegna su un suo foglio (`PULSANTIERA.DISEGNA`); ogni dispositivo è un blocco a
//! sé, che si sposta a mano e resta dov'è. Disegnando di nuovo si dispongono solo quelli che
//! mancano, nei posti liberi (in file dall'alto, emergenze e selettori per primi);
//! `PULSANTIERA.POSA` ne posa uno in un punto. Ogni dispositivo ricorda il suo componente
//! (`RIF`) e ne segue sigla e funzione (`PULSANTIERA.AGGIORNA`, anche da Aggiorna Liste e
//! Tabelle); quelli rimasti senza componente si segnalano. L'elenco con le quote si legge dalle
//! posizioni vere: `PULSANTIERA` (con il CSV della foratura), l'uscita `pulsantiera` e l'Excel.

use cadcraft_engine::cmd::has_doc;
use cadcraft_engine::{Accept, CommandSpec, Input, Interactive, Prompt, Result, Session, Step};
use serde_json::{Value, json};

use crate::util::{Inserito, blocchi_definiti, blocchi_inseriti, blocco, colore, errore, esegui, handles, imposta_attributi, in_gruppo, inserisci_su_layer, layer_corrente, linea, punto_param, su_layer_zero, testo, testo_param};
use crate::{fogli, impostazioni, layout, siglatura, simboli, squadratura, uscite};

type P = (f64, f64);

/// Passo tra i dispositivi e distanza della prima fila dal bordo superiore della porta (mm).
const PASSO_X: f64 = 60.0;
const PASSO_Y: f64 = 80.0;
const DALL_ALTO: f64 = 150.0;
const MARGINE: f64 = 50.0;
/// Diametro dei fori (dispositivi da 22 mm).
pub const FORO: f64 = 22.5;
/// Lato del corpo di un dispositivo (mm).
const CORPO: f64 = 30.0;

const BLOCCO_PORTA: &str = "CE_PORTA";
const BLOCCO_MANIGLIA: &str = "CE_PORTA_MANIGLIA";
const PREFISSO_DISPOSITIVO: &str = "CE_FRONTE_";

/// Un dispositivo dello schema da mettere sulla porta.
#[derive(Clone, Debug, PartialEq)]
pub struct Dispositivo {
    pub sigla: String,
    /// Il simbolo della libreria (`sbno`, `hl`…).
    pub tipo: String,
    pub funzione: String,
    pub funzione2: String,
    /// L'handle del componente nello schema.
    pub rif: String,
    /// Centro del foro, dall'angolo in basso a sinistra della porta (mm): dove sta, o dove lo si
    /// propone se non è ancora posato.
    pub pos: P,
}

/// Lo stato di un dispositivo sul fronte.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stato {
    Posato,
    DaPosare,
    /// Un dispositivo sulla porta senza più il suo componente nello schema.
    Orfano,
}

impl Stato {
    pub fn testo(self) -> &'static str {
        match self {
            Stato::Posato => "posato",
            Stato::DaPosare => "da posare",
            Stato::Orfano => "non nello schema",
        }
    }
}

const TIPI: [&str; 8] = ["sbem", "sbno", "sbnc", "sh", "sa", "sak", "hl", "ha"];

fn descrizione_tipo(t: &str) -> &'static str {
    match t {
        "sbem" => "Emergenza",
        "sbno" => "Pulsante NO",
        "sbnc" => "Pulsante NC",
        "sh" => "Pulsante luminoso",
        "sa" => "Selettore",
        "sak" => "Selettore a chiave",
        "hl" => "Lampada",
        "ha" => "Sirena",
        _ => "",
    }
}

/// Ordine di disposizione: emergenze, selettori, pulsanti, lampade, sirene.
fn ordine_tipo(t: &str) -> u8 {
    match t {
        "sbem" => 0,
        "sa" | "sak" => 1,
        "sbno" | "sbnc" | "sh" => 2,
        "hl" => 3,
        _ => 4,
    }
}

/// I dispositivi del quadro nello schema, nell'ordine in cui si dispongono.
fn dello_schema(s: &mut Session, quadro: &str) -> Result<Vec<Dispositivo>> {
    let default = impostazioni::quadro_corrente().unwrap_or_default();
    let senza_quadro = quadro.trim().is_empty() || default.trim().eq_ignore_ascii_case(quadro.trim());
    let mut v: Vec<Dispositivo> = Vec::new();
    for c in siglatura::componenti(s)?.into_iter().filter(|c| c.madre() && c.foglio > 0) {
        let b = &c.blocco;
        let Some(sim) = simboli::LIBRERIA.iter().find(|x| x.blocco.eq_ignore_ascii_case(&b.nome)) else { continue };
        if !TIPI.contains(&sim.id) || !layout::del_quadro(b.attributo("QUADRO"), quadro, senza_quadro) || v.iter().any(|d| d.sigla == c.nome) {
            continue;
        }
        v.push(Dispositivo { sigla: c.nome.clone(), tipo: sim.id.to_string(), funzione: b.attributo("FUNZIONE1").to_string(), funzione2: b.attributo("FUNZIONE2").to_string(), rif: b.handle.clone(), pos: (0.0, 0.0) });
    }
    v.sort_by(|a, b| (ordine_tipo(&a.tipo), a.sigla.as_str()).cmp(&(ordine_tipo(&b.tipo), b.sigla.as_str())));
    Ok(v)
}

/// La porta di un quadro disegnata su un foglio.
#[derive(Clone, Debug)]
pub struct Porta {
    pub layer: String,
    /// Angolo in basso a sinistra sul foglio.
    pub origine: P,
    pub larghezza: f64,
    pub altezza: f64,
    pub scala: f64,
}

impl Porta {
    /// Da mm sulla porta a coordinate del foglio, e ritorno.
    fn sul_foglio(&self, p: P) -> P {
        (self.origine.0 + p.0 / self.scala, self.origine.1 + p.1 / self.scala)
    }
    fn sulla_porta(&self, p: P) -> P {
        ((p.0 - self.origine.0) * self.scala, (p.1 - self.origine.1) * self.scala)
    }
}

fn numero(b: &Inserito, tag: &str) -> f64 {
    b.attributo(tag).trim().replace(',', ".").parse::<f64>().ok().filter(|x| x.is_finite() && *x > 0.0).unwrap_or(0.0)
}

/// La porta del quadro, se è disegnata.
pub fn porta(s: &mut Session, quadro: &str) -> Result<Option<Porta>> {
    Ok(blocchi_inseriti(s, BLOCCO_PORTA)?.into_iter().find(|b| b.attributo("QUADRO").trim().eq_ignore_ascii_case(quadro.trim())).map(|b| Porta {
        layer: b.layer.clone(),
        origine: b.pos,
        larghezza: numero(&b, "LARGHEZZA"),
        altezza: numero(&b, "ALTEZZA"),
        scala: numero(&b, "SCALA").max(1.0),
    }))
}

/// La misura della porta: quella data, o quella della piastra del lay-out, o 600 × 800.
fn misura_porta(s: &mut Session, quadro: &str, data: Option<P>) -> P {
    data.unwrap_or_else(|| {
        crate::util::blocchi_inseriti(s, layout::BLOCCO_PIASTRA)
            .ok()
            .and_then(|p| p.into_iter().find(|b| quadro.trim().is_empty() || b.attributo("QUADRO").trim().eq_ignore_ascii_case(quadro.trim())))
            .and_then(|b| Some((b.attributo("LARGHEZZA").trim().parse::<f64>().ok()?, b.attributo("ALTEZZA").trim().parse::<f64>().ok()?)))
            .unwrap_or((600.0, 800.0))
    })
}

/// I posti per i dispositivi su una porta: file dall'alto, ognuna centrata; `n` posti al più.
fn posti(porta: P, n: usize) -> Vec<P> {
    let colonne = (((porta.0 - 2.0 * MARGINE) / PASSO_X).floor() as usize + 1).max(1);
    let file = (((porta.1 - MARGINE - DALL_ALTO) / PASSO_Y).floor() as usize + 1).max(1);
    let mut v = Vec::new();
    for fila in 0..file {
        let in_fila = colonne.min(n.saturating_sub(fila * colonne)).max(1);
        let x0 = (porta.0 - PASSO_X * (in_fila - 1) as f64) / 2.0;
        for col in 0..in_fila {
            if v.len() < n {
                v.push((x0 + PASSO_X * col as f64, porta.1 - DALL_ALTO - PASSO_Y * fila as f64));
            }
        }
    }
    v
}

/// Un dispositivo del fronte con il suo stato e, se è sulla porta, il suo blocco.
pub type Voce = (Dispositivo, Stato, Option<String>);

/// I dispositivi del fronte: quelli dello schema con la loro posizione (posati) o il posto
/// proposto (da posare), poi quelli sulla porta senza più componente. Errore se i dispositivi da
/// posare non entrano nella porta.
pub fn elenco(s: &mut Session, quadro: &str, misura: Option<P>) -> Result<(P, Vec<Voce>)> {
    let p = porta(s, quadro)?;
    let misura = p.as_ref().map_or_else(|| misura_porta(s, quadro, misura), |p| (p.larghezza, p.altezza));
    let schema = dello_schema(s, quadro)?;
    let sulla_porta: Vec<Inserito> = blocchi_inseriti(s, &format!("{PREFISSO_DISPOSITIVO}*"))?.into_iter().filter(|b| b.attributo("QUADRO").trim().eq_ignore_ascii_case(quadro.trim())).collect();
    let mut usati = vec![false; sulla_porta.len()];
    let mut v: Vec<Voce> = Vec::new();
    for d in schema {
        let k = sulla_porta
            .iter()
            .enumerate()
            .position(|(k, b)| !usati[k] && !d.rif.is_empty() && b.attributo("RIF").eq_ignore_ascii_case(&d.rif))
            .or_else(|| sulla_porta.iter().enumerate().position(|(k, b)| !usati[k] && b.attributo("SIGLA").eq_ignore_ascii_case(&d.sigla)));
        match (k, &p) {
            (Some(k), Some(porta)) => {
                usati[k] = true;
                let pos = porta.sulla_porta(sulla_porta[k].pos);
                v.push((Dispositivo { pos, ..d }, Stato::Posato, Some(sulla_porta[k].handle.clone())));
            }
            _ => v.push((d, Stato::DaPosare, None)),
        }
    }
    // i posti proposti per quelli da posare, lontani da quelli posati
    let occupati: Vec<P> = v.iter().filter(|x| x.1 == Stato::Posato).map(|x| x.0.pos).collect();
    let mancanti = v.iter().filter(|x| x.1 == Stato::DaPosare).count();
    if mancanti > 0 {
        let liberi: Vec<P> = posti(misura, usize::MAX / 2).into_iter().filter(|q| !occupati.iter().any(|o| (o.0 - q.0).abs() < PASSO_X / 2.0 && (o.1 - q.1).abs() < PASSO_Y / 2.0)).take(mancanti).collect();
        if liberi.len() < mancanti {
            return Err(errore(format!("{mancanti} dispositivi da posare non entrano in una porta {:.0} × {:.0} mm: posane qualcuno a mano o usa una porta più grande", misura.0, misura.1)));
        }
        // i posti proposti, centrati fila per fila come se la porta fosse vuota
        let liberi = if occupati.is_empty() { posti(misura, mancanti) } else { liberi };
        for (x, q) in v.iter_mut().filter(|x| x.1 == Stato::DaPosare).zip(liberi) {
            x.0.pos = q;
        }
    }
    for (b, _) in sulla_porta.iter().zip(&usati).filter(|(_, u)| !**u) {
        let pos = p.as_ref().map_or(b.pos, |p| p.sulla_porta(b.pos));
        v.push((Dispositivo { sigla: b.attributo("SIGLA").into(), tipo: b.attributo("TIPO").into(), funzione: b.attributo("FUNZIONE").into(), funzione2: String::new(), rif: b.attributo("RIF").into(), pos }, Stato::Orfano, Some(b.handle.clone())));
    }
    Ok((misura, v))
}

/// Il colore di una lampada dalla sua funzione (rossa, verde, gialla, blu, bianca).
fn colore_lampada(f: &str) -> &'static str {
    let f = f.to_lowercase();
    if f.contains("ross") || f.contains("allarm") || f.contains("guasto") {
        "1"
    } else if f.contains("verd") || f.contains("marcia") {
        "3"
    } else if f.contains("blu") {
        "150"
    } else if f.contains("bianc") {
        "7"
    } else {
        "2"
    }
}

/// Il blocco di un dispositivo alla scala 1:k (il centro nel punto d'inserimento), con la sigla
/// sotto; le lampade hanno il colore della funzione.
fn blocco_dispositivo(s: &mut Session, tipo: &str, funzione: &str, k: f64) -> Result<String> {
    let lampada = if tipo == "hl" { colore_lampada(funzione) } else { "" };
    let nome = format!("{PREFISSO_DISPOSITIVO}{}{}_{}", tipo.to_ascii_uppercase(), if lampada.is_empty() { String::new() } else { format!("_{lampada}") }, layout::misura_testo(k)).replace('.', "_");
    if blocchi_definiti(s)?.iter().any(|b| b.eq_ignore_ascii_case(&nome)) {
        return Ok(nome);
    }
    su_layer_zero(s, |s| {
        let mut h = Vec::new();
        let r = FORO / 2.0 / k;
        let cerchio = |s: &mut Session, h: &mut Vec<String>, raggio: f64, col: &str| -> Result<()> {
            colore(s, col)?;
            h.extend(handles(&esegui(s, "circle", json!({"center": [0.0, 0.0], "radius": raggio}))?));
            Ok(())
        };
        colore(s, "8")?;
        let q = CORPO / 2.0 / k;
        linea(s, &mut h, &[(-q, -q), (q, -q), (q, q), (-q, q)], true)?;
        match tipo {
            "sbem" => {
                cerchio(s, &mut h, r * 2.6, "2")?;
                cerchio(s, &mut h, r * 1.7, "1")?;
            }
            "sbno" => {
                cerchio(s, &mut h, r, "3")?;
                cerchio(s, &mut h, r * 0.7, "3")?;
            }
            "sbnc" => {
                cerchio(s, &mut h, r, "1")?;
                cerchio(s, &mut h, r * 0.7, "1")?;
            }
            "sh" => {
                cerchio(s, &mut h, r, "7")?;
                cerchio(s, &mut h, r * 0.5, "2")?;
            }
            "sa" | "sak" => {
                cerchio(s, &mut h, r, "7")?;
                linea(s, &mut h, &[(-r * 0.8, -r * 0.25), (r * 0.8, r * 0.25)], false)?;
                if tipo == "sak" {
                    linea(s, &mut h, &[(-r * 0.15, -r * 0.5), (r * 0.15, -r * 0.5), (r * 0.15, r * 0.5), (-r * 0.15, r * 0.5)], true)?;
                }
            }
            "hl" => {
                cerchio(s, &mut h, r, lampada)?;
                let d = r * 0.7;
                linea(s, &mut h, &[(-d, -d), (d, d)], false)?;
                linea(s, &mut h, &[(-d, d), (d, -d)], false)?;
            }
            _ => {
                cerchio(s, &mut h, r, "7")?;
                cerchio(s, &mut h, r * 0.6, "7")?;
            }
        }
        colore(s, "4")?;
        let t = handles(&esegui(s, "attdef", json!({"tag": "SIGLA", "prompt": "Sigla", "default": "", "at": [0.0, -q - 2.2], "height": 1.8}))?);
        if let Some(x) = t.first() {
            crate::util::allinea(s, x, cadcraft_engine::doc::HAlign::Center)?;
        }
        h.extend(t);
        colore(s, "BYLAYER")?;
        for tag in ["FUNZIONE", "TIPO", "QUADRO", "RIF"] {
            h.extend(handles(&esegui(s, "attdef", json!({"tag": tag, "prompt": tag, "default": "", "at": [0.0, 0.0], "height": 1.0, "invisible": true}))?));
        }
        blocco(s, &nome, h, "Dispositivo sul fronte quadro")
    })?;
    Ok(nome)
}

fn crea_porta(s: &mut Session) -> Result<()> {
    let definiti = blocchi_definiti(s)?;
    su_layer_zero(s, |s| {
        if !definiti.iter().any(|b| b.eq_ignore_ascii_case(BLOCCO_PORTA)) {
            // lato 1 nei due sensi: si scala con la misura della porta
            colore(s, "7")?;
            let mut h = Vec::new();
            linea(s, &mut h, &[(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)], true)?;
            for tag in ["QUADRO", "LARGHEZZA", "ALTEZZA", "SCALA"] {
                h.extend(handles(&esegui(s, "attdef", json!({"tag": tag, "prompt": tag, "default": "", "at": [0.0, 0.0], "height": 0.01, "invisible": true}))?));
            }
            blocco(s, BLOCCO_PORTA, h, "Porta del quadro")?;
        }
        if !definiti.iter().any(|b| b.eq_ignore_ascii_case(BLOCCO_MANIGLIA)) {
            // la maniglia, 10 × 120 mm, centrata nel punto d'inserimento
            colore(s, "7")?;
            let mut h = Vec::new();
            linea(s, &mut h, &[(-5.0, -60.0), (5.0, -60.0), (5.0, 60.0), (-5.0, 60.0)], true)?;
            blocco(s, BLOCCO_MANIGLIA, h, "Maniglia della porta")?;
        }
        Ok(())
    })
}

/// Posa un dispositivo sulla porta nel punto `at` del foglio.
fn posa_dispositivo(s: &mut Session, porta: &Porta, quadro: &str, d: &Dispositivo, at: P) -> Result<String> {
    let nome = blocco_dispositivo(s, &d.tipo, &d.funzione, porta.scala)?;
    let h = inserisci_su_layer(s, &nome, at, &json!({"SIGLA": d.sigla, "FUNZIONE": d.funzione, "TIPO": d.tipo, "QUADRO": quadro}), &porta.layer)?;
    imposta_attributi(s, &h, &[("RIF", d.rif.as_str())])?;
    Ok(h)
}

/// Disegna la porta del quadro su un foglio (se non c'è) e vi dispone i dispositivi non ancora
/// posati. Restituisce (foglio, dispositivi posati ora).
pub fn disegna(s: &mut Session, quadro: &str, misura: Option<P>, foglio: Option<u32>) -> Result<(u32, usize)> {
    let p = match porta(s, quadro)? {
        Some(p) => p,
        None => {
            let (l, a) = misura_porta(s, quadro, misura);
            let n = foglio.unwrap_or_else(|| fogli::elenco(s).ok().and_then(|f| f.last().map(|f| f.numero + 1)).unwrap_or(1));
            if !fogli::elenco(s)?.iter().any(|f| f.numero == n) {
                fogli::crea(s, n)?;
            }
            let layer = fogli::nome_layer(n);
            let k = layout::scegli_scala(l, a);
            // la porta in basso a sinistra dell'area del foglio, sotto il titolo
            let o = (squadratura::CORNICE_MIN.0 + 15.0, squadratura::CORNICE_MAX.1 - squadratura::FASCIA_DESCRIZIONI - 12.0 - a / k);
            crea_porta(s)?;
            let m = layout::misura_testo;
            let r = esegui(s, "insert", json!({"name": BLOCCO_PORTA, "at": [o.0, o.1], "scale": [l / k, a / k], "attribs": {"QUADRO": quadro, "LARGHEZZA": m(l), "ALTEZZA": m(a), "SCALA": m(k)}}))?;
            esegui(s, "laymch", json!({"handles": handles(&r), "layer": layer}))?;
            let r = esegui(s, "insert", json!({"name": BLOCCO_MANIGLIA, "at": [o.0 + (l - 25.0) / k, o.1 + a / 2.0 / k], "scale": [1.0 / k, 1.0 / k]}))?;
            esegui(s, "laymch", json!({"handles": handles(&r), "layer": layer}))?;
            let mut h = Vec::new();
            colore(s, "7")?;
            let titolo = if quadro.trim().is_empty() { "FRONTE QUADRO \\ PANEL FRONT".to_string() } else { format!("FRONTE QUADRO {quadro} \\ {quadro} PANEL FRONT") };
            testo(s, &mut h, (o.0, o.1 + a / k + 6.0), &titolo, 4.0, "L")?;
            testo(s, &mut h, (o.0, o.1 - 5.0), &format!("Porta {} × {} mm – scala 1:{}", m(l), m(a), m(k)), 2.0, "L")?;
            colore(s, "BYLAYER")?;
            esegui(s, "laymch", json!({"handles": h, "layer": layer}))?;
            let titolo_foglio = if quadro.trim().is_empty() { "FRONTE QUADRO".to_string() } else { format!("FRONTE QUADRO {}", quadro.trim()) };
            fogli::imposta_titolo(s, n, &titolo_foglio, Some("PANEL FRONT"))?;
            fogli::numera(s)?;
            Porta { layer, origine: o, larghezza: l, altezza: a, scala: k }
        }
    };
    let (_, v) = elenco(s, quadro, None)?;
    let mut posati = 0;
    for (d, stato, _) in &v {
        if *stato == Stato::DaPosare {
            posa_dispositivo(s, &p, quadro, d, p.sul_foglio(d.pos))?;
            posati += 1;
        }
    }
    Ok((fogli::numero_da_layer(&p.layer).unwrap_or(0), posati))
}

/// Posa sulla porta il dispositivo `sigla` nel punto `at` del foglio.
pub fn posa(s: &mut Session, quadro: &str, sigla: &str, at: P) -> Result<Value> {
    let p = porta(s, quadro)?.ok_or_else(|| errore(format!("Il quadro {quadro} non ha ancora il fronte: disegnalo con Pulsantiera")))?;
    let (_, v) = elenco(s, quadro, None)?;
    let (d, stato, _) = v.into_iter().find(|(d, _, _)| d.sigla.eq_ignore_ascii_case(sigla.trim())).ok_or_else(|| errore(format!("{sigla} non è un dispositivo del fronte del quadro {quadro}")))?;
    if stato != Stato::DaPosare {
        return Err(errore(format!("{} è già sulla porta", d.sigla)));
    }
    let h = posa_dispositivo(s, &p, quadro, &d, at)?;
    let q = p.sulla_porta(at);
    Ok(json!({"handle": h, "sigla": d.sigla, "x": q.0, "y": q.1, "message": format!("{} sulla porta a X {:.0}, Y {:.0} mm", d.sigla, q.0, q.1)}))
}

/// Allinea i dispositivi sulle porte allo schema: sigla, funzione e tipo del componente
/// ricordato. Con `togli_orfani` cancella quelli rimasti senza componente. (aggiornati, orfani)
pub fn aggiorna(s: &mut Session, togli_orfani: bool) -> Result<(usize, usize)> {
    let sulle_porte = blocchi_inseriti(s, &format!("{PREFISSO_DISPOSITIVO}*"))?;
    if sulle_porte.is_empty() {
        return Ok((0, 0));
    }
    let madri: Vec<siglatura::Componente> = siglatura::componenti(s)?.into_iter().filter(siglatura::Componente::madre).collect();
    let (mut aggiornati, mut orfani) = (0, Vec::new());
    for b in &sulle_porte {
        let rif = b.attributo("RIF").trim();
        let c = if rif.is_empty() { madri.iter().find(|c| c.nome.eq_ignore_ascii_case(b.attributo("SIGLA"))) } else { madri.iter().find(|c| c.blocco.handle.eq_ignore_ascii_case(rif)) };
        match c {
            Some(c) => {
                let funzione = c.blocco.attributo("FUNZIONE1");
                if c.nome != b.attributo("SIGLA") || funzione != b.attributo("FUNZIONE") {
                    imposta_attributi(s, &b.handle, &[("SIGLA", c.nome.as_str()), ("FUNZIONE", funzione)])?;
                    aggiornati += 1;
                }
            }
            None => orfani.push(b.handle.clone()),
        }
    }
    let n = orfani.len();
    if togli_orfani && !orfani.is_empty() {
        esegui(s, "erase", json!({"handles": orfani}))?;
    }
    Ok((aggiornati, n))
}

fn porta_param(p: &Value) -> Option<P> {
    let n = |k: &str| p.get(k).and_then(|v| v.as_f64().or_else(|| v.as_str().and_then(|t| t.trim().parse().ok()))).filter(|x: &f64| x.is_finite() && *x >= 100.0 && *x <= 3000.0);
    Some((n("larghezza")?, n("altezza")?))
}

/// Il quadro: quello dato, o quello della porta del foglio corrente, o il quadro corrente.
fn quadro_param(s: &mut Session, p: &Value) -> Result<String> {
    if let Some(q) = testo_param(p, "quadro").filter(|q| !q.trim().is_empty()) {
        return Ok(q);
    }
    let layer = layer_corrente(s)?;
    Ok(blocchi_inseriti(s, BLOCCO_PORTA)?
        .into_iter()
        .find(|b| b.layer.eq_ignore_ascii_case(&layer))
        .map(|b| b.attributo("QUADRO").to_string())
        .or_else(impostazioni::quadro_corrente)
        .unwrap_or_default())
}

/// L'elenco in tabella (sigla, funzioni, tipo, quote e diametro del foro), dalle posizioni vere.
pub fn tabella(s: &mut Session, p: &Value) -> Result<uscite::Tabella> {
    let quadro = quadro_param(s, p)?;
    let (_, v) = elenco(s, &quadro, porta_param(p))?;
    if v.is_empty() {
        return Err(errore(format!("Nessun pulsante, selettore o lampada nel quadro {quadro}")));
    }
    let col = |v: &[(&str, f64)]| v.iter().map(|(t, w)| ((*t).to_string(), *w)).collect::<Vec<_>>();
    Ok(uscite::Tabella {
        titolo: if quadro.is_empty() { "PULSANTIERA \\ CONTROL PANEL".into() } else { format!("PULSANTIERA QUADRO {quadro} \\ {quadro} CONTROL PANEL") },
        colonne: col(&[("Sigla\nTag", 30.0), ("Funzione\nFunction", 70.0), ("Funzione (2a lingua)\nFunction", 70.0), ("Tipo\nType", 40.0), ("X (mm)", 20.0), ("Y (mm)", 20.0), ("Ø (mm)", 18.0), ("Stato\nState", 30.0)]),
        righe: v
            .iter()
            .map(|(d, stato, _)| {
                [d.sigla.clone(), d.funzione.clone(), d.funzione2.clone(), descrizione_tipo(&d.tipo).to_string(), format!("{:.0}", d.pos.0), format!("{:.0}", d.pos.1), format!("{FORO}"), stato.testo().to_string()]
                    .into_iter()
                    .map(uscite::Cella::T)
                    .collect()
            })
            .collect(),
        altezza_riga: 6.0,
    })
}

fn run_pulsantiera(s: &mut Session, p: &Value) -> Result<Value> {
    let quadro = quadro_param(s, p)?;
    let (porta_mm, v) = elenco(s, &quadro, porta_param(p))?;
    if let Some(f) = testo_param(p, "file") {
        let righe: Vec<Vec<String>> = v.iter().filter(|(_, st, _)| *st != Stato::Orfano).map(|(d, _, _)| vec![d.sigla.clone(), d.funzione.clone(), format!("{:.1}", d.pos.0), format!("{:.1}", d.pos.1), format!("{FORO}")]).collect();
        crate::morsetti::scrivi_csv(&f, &["Sigla", "Funzione", "X", "Y", "Diametro"], &righe)?;
    }
    let disegnata = porta(s, &quadro)?;
    let conta = |st: Stato| v.iter().filter(|x| x.1 == st).count();
    Ok(json!({
        "quadro": quadro,
        "porta": [porta_mm.0, porta_mm.1],
        "foglio": disegnata.as_ref().and_then(|p| fogli::numero_da_layer(&p.layer)),
        "dispositivi": v.iter().map(|(d, st, h)| {
            let sul_foglio = disegnata.as_ref().map(|p| p.sul_foglio(d.pos));
            json!({"sigla": d.sigla, "tipo": d.tipo, "funzione": d.funzione, "x": d.pos.0, "y": d.pos.1, "stato": st.testo(), "handle": h, "foglio": disegnata.as_ref().and_then(|p| fogli::numero_da_layer(&p.layer)), "xFoglio": sul_foglio.map(|q| q.0), "yFoglio": sul_foglio.map(|q| q.1)})
        }).collect::<Vec<_>>(),
        "message": format!("Fronte {quadro}: {} posati, {} da posare, {} non nello schema, porta {:.0} × {:.0} mm", conta(Stato::Posato), conta(Stato::DaPosare), conta(Stato::Orfano), porta_mm.0, porta_mm.1),
    }))
}

fn run_disegna(s: &mut Session, p: &Value) -> Result<Value> {
    let quadro = quadro_param(s, p)?;
    let foglio = crate::util::numero_param(p, "foglio");
    let (n, posati) = in_gruppo(s, "Fronte quadro", |s| disegna(s, &quadro, porta_param(p), foglio))?;
    Ok(json!({"foglio": n, "posati": posati, "message": format!("Fronte del quadro {quadro} sul foglio {n}: {posati} dispositivi posati ora")}))
}

fn run_posa(s: &mut Session, p: &Value) -> Result<Value> {
    let quadro = quadro_param(s, p)?;
    let sigla = testo_param(p, "sigla").filter(|t| !t.trim().is_empty()).ok_or_else(|| errore("Indicare il dispositivo (sigla)"))?;
    let at = punto_param(p, "at").ok_or_else(|| errore("Indicare il punto (at)"))?;
    in_gruppo(s, "Dispositivo sulla porta", |s| posa(s, &quadro, &sigla, at))
}

fn run_aggiorna(s: &mut Session, p: &Value) -> Result<Value> {
    let togli = p.get("togliOrfani").and_then(Value::as_bool).unwrap_or(false);
    let (a, o) = in_gruppo(s, "Aggiorna fronte quadro", |s| aggiorna(s, togli))?;
    Ok(json!({"aggiornati": a, "orfani": o, "message": format!("Fronte quadro: {a} dispositivi aggiornati, {o} senza componente{}", if togli { " tolti" } else { "" })}))
}

/// Posa interattiva: il dispositivo (Invio = il prossimo da posare) e il punto sulla porta.
struct PosaInterattiva {
    sigla: Option<String>,
    prossimo: Option<String>,
}

impl PosaInterattiva {
    fn prossimo(s: &mut Session) -> Option<String> {
        let quadro = quadro_param(s, &json!({})).ok()?;
        elenco(s, &quadro, None).ok()?.1.into_iter().find(|x| x.1 == Stato::DaPosare).map(|x| x.0.sigla)
    }
}

impl Interactive for PosaInterattiva {
    fn name(&self) -> &'static str {
        "PULSANTIERA.POSA"
    }
    fn prompt(&self, _s: &Session) -> Prompt {
        match &self.sigla {
            Some(sigla) => Prompt::new(format!("Punto di {sigla} sulla porta"), Accept::POINT),
            None => Prompt { default: self.prossimo.clone(), ..Prompt::new("Dispositivo da posare (Invio: il prossimo)", Accept::TEXT) },
        }
    }
    fn input(&mut self, s: &mut Session, i: Input) -> Result<Step> {
        match (i, &self.sigla) {
            (Input::Cancel, _) => Ok(Step::Cancel),
            (Input::Point(q), Some(sigla)) => {
                let quadro = quadro_param(s, &json!({}))?;
                match in_gruppo(s, "Dispositivo sulla porta", |s| posa(s, &quadro, sigla, (q.x, q.y))) {
                    Ok(r) => s.echo(r["message"].as_str().unwrap_or_default().to_string()),
                    Err(e) => s.echo(e.to_string()),
                }
                self.sigla = None;
                self.prossimo = Self::prossimo(s);
                Ok(Step::Continue)
            }
            (Input::Text(t) | Input::Keyword(t), None) if !t.trim().is_empty() => {
                self.sigla = Some(t.trim().to_string());
                Ok(Step::Continue)
            }
            (Input::Enter | Input::Text(_), None) => match self.prossimo.clone().or_else(|| Self::prossimo(s)) {
                Some(p) => {
                    self.sigla = Some(p);
                    Ok(Step::Continue)
                }
                None => {
                    s.echo("Tutti i dispositivi sono sulla porta".to_string());
                    Ok(Step::Done)
                }
            },
            (Input::Enter, Some(_)) => {
                self.sigla = None;
                Ok(Step::Continue)
            }
            _ => Ok(Step::Continue),
        }
    }
}

fn fabbrica(_s: &Session) -> Result<Box<dyn Interactive>> {
    Ok(Box::new(PosaInterattiva { sigla: None, prossimo: None }))
}

pub fn comandi() -> Vec<CommandSpec> {
    let spec = |id, label, menu, params, run| CommandSpec { id, label, menu, shortcut: None, aliases: &[], params, enabled: has_doc, run, undoable: false, interactive: None, transparent: false };
    vec![
        spec(
            "pulsantiera",
            "Pulsantiera",
            &["Quadro", "Pulsantiera (fronte quadro)", "Elenco e foratura"],
            "{quadro?, larghezza?, altezza? (porta, mm; default quella disegnata o la piastra del lay-out), file?: CSV della foratura} → {porta, foglio?, dispositivi: [{sigla, tipo, funzione, x, y, stato: posato|da posare|non nello schema}]}; in tabella: USCITA {tipo: pulsantiera}",
            run_pulsantiera as cadcraft_engine::cmd::Run,
        ),
        spec(
            "pulsantiera.disegna",
            "Disegna il fronte quadro",
            &["Quadro", "Pulsantiera (fronte quadro)", "Disegna"],
            "{quadro?, larghezza?, altezza?, foglio?} — la porta su un foglio (se non c'è) e i dispositivi non ancora posati nei posti liberi; quelli già posati restano dove sono",
            run_disegna,
        ),
        CommandSpec {
            interactive: Some(fabbrica),
            ..spec("pulsantiera.posa", "Dispositivo sulla porta", &["Quadro", "Pulsantiera (fronte quadro)", "Posa un dispositivo"], "{sigla, at: [x,y], quadro?} — il dispositivo nel punto della porta; interattivo: Invio = il prossimo da posare", run_posa)
        },
        spec("pulsantiera.aggiorna", "Aggiorna il fronte quadro", &["Quadro", "Pulsantiera (fronte quadro)", "Aggiorna dallo schema"], "{togliOrfani?: bool} — sigla e funzione dei dispositivi dallo schema → {aggiornati, orfani}", run_aggiorna),
    ]
}
