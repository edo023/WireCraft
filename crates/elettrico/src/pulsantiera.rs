//! Pulsantiera sulla porta del quadro: i dispositivi di comando e di segnalazione dello schema
//! (pulsanti, emergenze, selettori, lampade, sirene) disposti in file sulla porta, con la
//! foratura (fori Ø22, quote dall'angolo in basso a sinistra della porta).
//!
//! È un'uscita come le altre (`USCITA {tipo: pulsantiera}`): la porta in scala con il disegno di
//! ogni dispositivo e la sua sigla, e accanto l'elenco con le funzioni in due lingue e le quote;
//! la stessa cosa in tabella (Excel) con `tabella("pulsantiera")`.

use cadcraft_engine::{Result, Session};
use serde_json::{Value, json};

use crate::util::{colore, errore, esegui, handles, linea, testo};
use crate::{impostazioni, layout, siglatura, simboli, uscite};

type P = (f64, f64);

/// Passo tra i dispositivi e distanza della prima fila dal bordo superiore della porta (mm).
const PASSO_X: f64 = 60.0;
const PASSO_Y: f64 = 80.0;
const DALL_ALTO: f64 = 150.0;
const MARGINE: f64 = 50.0;
/// Diametro dei fori (dispositivi da 22 mm).
pub const FORO: f64 = 22.5;

/// Un dispositivo sulla porta.
#[derive(Clone, Debug, PartialEq)]
pub struct Dispositivo {
    pub sigla: String,
    /// Il simbolo della libreria (`sbno`, `hl`…).
    pub tipo: String,
    pub funzione: String,
    pub funzione2: String,
    /// Centro del foro, dall'angolo in basso a sinistra della porta (mm).
    pub pos: P,
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

/// I dispositivi del quadro e la porta che li contiene (larghezza, altezza): la porta data, o
/// quella della piastra del lay-out, o 600 × 800. Errore se non entrano.
pub fn dispositivi(s: &mut Session, quadro: &str, porta: Option<P>) -> Result<(Vec<Dispositivo>, P)> {
    let default = impostazioni::quadro_corrente().unwrap_or_default();
    let senza_quadro = quadro.trim().is_empty() || default.trim().eq_ignore_ascii_case(quadro.trim());
    let mut v: Vec<Dispositivo> = Vec::new();
    for c in siglatura::componenti(s)?.into_iter().filter(|c| c.madre() && c.foglio > 0) {
        let b = &c.blocco;
        let Some(sim) = simboli::LIBRERIA.iter().find(|x| x.blocco.eq_ignore_ascii_case(&b.nome)) else { continue };
        if !TIPI.contains(&sim.id) || !layout::del_quadro(b.attributo("QUADRO"), quadro, senza_quadro) || v.iter().any(|d| d.sigla == c.nome) {
            continue;
        }
        v.push(Dispositivo { sigla: c.nome.clone(), tipo: sim.id.to_string(), funzione: b.attributo("FUNZIONE1").to_string(), funzione2: b.attributo("FUNZIONE2").to_string(), pos: (0.0, 0.0) });
    }
    // le emergenze per prime, poi in ordine di sigla
    v.sort_by(|a, b| (a.tipo != "sbem", a.sigla.as_str()).cmp(&(b.tipo != "sbem", b.sigla.as_str())));
    let porta = porta.unwrap_or_else(|| {
        crate::util::blocchi_inseriti(s, layout::BLOCCO_PIASTRA)
            .ok()
            .and_then(|p| p.into_iter().find(|b| quadro.trim().is_empty() || b.attributo("QUADRO").trim().eq_ignore_ascii_case(quadro.trim())))
            .and_then(|b| Some((b.attributo("LARGHEZZA").trim().parse::<f64>().ok()?, b.attributo("ALTEZZA").trim().parse::<f64>().ok()?)))
            .unwrap_or((600.0, 800.0))
    });
    let colonne = (((porta.0 - 2.0 * MARGINE) / PASSO_X).floor() as usize + 1).max(1);
    let file = v.len().div_ceil(colonne);
    if file > 0 && DALL_ALTO + PASSO_Y * (file - 1) as f64 > porta.1 - MARGINE {
        return Err(errore(format!("{} dispositivi non entrano in una porta {:.0} × {:.0} mm", v.len(), porta.0, porta.1)));
    }
    let n = v.len();
    for (k, d) in v.iter_mut().enumerate() {
        let (fila, col) = (k / colonne, k % colonne);
        // ogni fila centrata sulla porta (l'ultima può essere più corta)
        let in_fila = colonne.min(n - fila * colonne).max(1);
        let x0 = (porta.0 - PASSO_X * (in_fila - 1) as f64) / 2.0;
        d.pos = (x0 + PASSO_X * col as f64, porta.1 - DALL_ALTO - PASSO_Y * fila as f64);
    }
    Ok((v, porta))
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

/// Disegna un dispositivo con centro `c` sul foglio (`r` raggio del foro sul foglio).
fn disegna_dispositivo(s: &mut Session, h: &mut Vec<String>, d: &Dispositivo, c: P, r: f64) -> Result<()> {
    let cerchio = |s: &mut Session, h: &mut Vec<String>, raggio: f64, col: &str| -> Result<()> {
        colore(s, col)?;
        h.extend(handles(&esegui(s, "circle", json!({"center": [c.0, c.1], "radius": raggio}))?));
        Ok(())
    };
    // il corpo del dispositivo (30 × 30 mm in scala)
    colore(s, "8")?;
    let q = r * 30.0 / FORO;
    linea(s, h, &[(c.0 - q, c.1 - q), (c.0 + q, c.1 - q), (c.0 + q, c.1 + q), (c.0 - q, c.1 + q)], true)?;
    match d.tipo.as_str() {
        "sbem" => {
            cerchio(s, h, r * 2.6, "2")?;
            cerchio(s, h, r * 1.7, "1")?;
        }
        "sbno" => {
            cerchio(s, h, r, "3")?;
            cerchio(s, h, r * 0.7, "3")?;
        }
        "sbnc" => {
            cerchio(s, h, r, "1")?;
            cerchio(s, h, r * 0.7, "1")?;
        }
        "sh" => {
            cerchio(s, h, r, "7")?;
            cerchio(s, h, r * 0.5, "2")?;
        }
        "sa" | "sak" => {
            cerchio(s, h, r, "7")?;
            linea(s, h, &[(c.0 - r * 0.8, c.1 - r * 0.25), (c.0 + r * 0.8, c.1 + r * 0.25)], false)?;
            if d.tipo == "sak" {
                linea(s, h, &[(c.0 - r * 0.15, c.1 - r * 0.5), (c.0 + r * 0.15, c.1 - r * 0.5), (c.0 + r * 0.15, c.1 + r * 0.5), (c.0 - r * 0.15, c.1 + r * 0.5)], true)?;
            }
        }
        "hl" => {
            cerchio(s, h, r, colore_lampada(&d.funzione))?;
            let k = r * 0.7;
            linea(s, h, &[(c.0 - k, c.1 - k), (c.0 + k, c.1 + k)], false)?;
            linea(s, h, &[(c.0 - k, c.1 + k), (c.0 + k, c.1 - k)], false)?;
        }
        _ => {
            cerchio(s, h, r, "7")?;
            cerchio(s, h, r * 0.6, "7")?;
        }
    }
    colore(s, "4")?;
    testo(s, h, (c.0, c.1 - q - 2.2), &d.sigla, 1.8, "MC")
}

/// Disegna la pulsantiera nella pagina (coordinate del blocco: origine in alto a sinistra).
fn disegna_pagina(s: &mut Session, quadro: &str, v: &[Dispositivo], porta: P, h: &mut Vec<String>) -> Result<()> {
    let k = layout::scegli_scala(porta.0, porta.1);
    let (largo, _) = uscite::AREA;
    colore(s, "7")?;
    let titolo = if quadro.is_empty() { "PULSANTIERA \\ CONTROL PANEL".to_string() } else { format!("PULSANTIERA QUADRO {quadro} \\ {quadro} CONTROL PANEL") };
    testo(s, h, (largo / 2.0, -4.5), &titolo, 4.0, "MC")?;
    // la porta in alto a sinistra, sotto il titolo
    let o = (5.0, -12.0 - porta.1 / k);
    let p = |x: f64, y: f64| (o.0 + x / k, o.1 + y / k);
    linea(s, h, &[p(0.0, 0.0), p(porta.0, 0.0), p(porta.0, porta.1), p(0.0, porta.1)], true)?;
    // la maniglia, a destra a metà altezza
    linea(s, h, &[p(porta.0 - 30.0, porta.1 / 2.0 - 60.0), p(porta.0 - 20.0, porta.1 / 2.0 - 60.0), p(porta.0 - 20.0, porta.1 / 2.0 + 60.0), p(porta.0 - 30.0, porta.1 / 2.0 + 60.0)], true)?;
    colore(s, "8")?;
    testo(s, h, (o.0, o.1 - 4.0), &format!("Porta {:.0} × {:.0} mm – scala 1:{}", porta.0, porta.1, k), 2.0, "L")?;
    for d in v {
        disegna_dispositivo(s, h, d, p(d.pos.0, d.pos.1), FORO / 2.0 / k)?;
    }
    // l'elenco a destra della porta
    let x0 = o.0 + porta.0 / k + 12.0;
    let righe = [("Sigla", 0.0), ("Funzione", 24.0), ("X", 92.0), ("Y", 104.0)];
    colore(s, "7")?;
    for (t, dx) in righe {
        testo(s, h, (x0 + dx, -16.0), t, 2.2, "L")?;
    }
    linea(s, h, &[(x0, -17.5), (x0 + 118.0, -17.5)], false)?;
    for (i, d) in v.iter().enumerate() {
        let y = -22.0 - 5.0 * i as f64;
        colore(s, "4")?;
        testo(s, h, (x0, y), &d.sigla, 2.0, "L")?;
        colore(s, "7")?;
        let f = if d.funzione2.trim().is_empty() { d.funzione.clone() } else { format!("{} / {}", d.funzione, d.funzione2) };
        testo(s, h, (x0 + 24.0, y), &f.chars().take(34).collect::<String>(), 2.0, "L")?;
        testo(s, h, (x0 + 92.0, y), &format!("{:.0}", d.pos.0), 2.0, "L")?;
        testo(s, h, (x0 + 104.0, y), &format!("{:.0}", d.pos.1), 2.0, "L")?;
    }
    Ok(())
}

fn porta_param(p: &Value) -> Option<P> {
    let n = |k: &str| p.get(k).and_then(|v| v.as_f64().or_else(|| v.as_str().and_then(|t| t.trim().parse().ok()))).filter(|x: &f64| x.is_finite() && *x >= 100.0 && *x <= 3000.0);
    Some((n("larghezza")?, n("altezza")?))
}

fn quadro_param(p: &Value) -> String {
    crate::util::testo_param(p, "quadro").filter(|q| !q.trim().is_empty()).or_else(impostazioni::quadro_corrente).unwrap_or_default()
}

/// Disegna la pulsantiera su un foglio dal foglio `primo` (default dopo l'ultimo).
pub fn uscita(s: &mut Session, p: &Value, primo: Option<u32>) -> Result<Vec<u32>> {
    let quadro = quadro_param(p);
    let (v, porta) = dispositivi(s, &quadro, porta_param(p))?;
    if v.is_empty() {
        return Err(errore(format!("Nessun pulsante, selettore o lampada nel quadro {quadro}")));
    }
    let mut par = serde_json::Map::new();
    par.insert("quadro".into(), json!(quadro));
    if let Some((l, a)) = porta_param(p) {
        par.insert("larghezza".into(), json!(l));
        par.insert("altezza".into(), json!(a));
    }
    uscite::disegna_con(s, 1, "pulsantiera", &Value::Object(par), primo, "Pulsantiera", "Control Panel", |s, _, h| disegna_pagina(s, &quadro, &v, porta, h))
}

/// La versione in tabella (sigla, funzioni, tipo, quote e diametro del foro).
pub fn tabella(s: &mut Session, p: &Value) -> Result<uscite::Tabella> {
    let quadro = quadro_param(p);
    let (v, _) = dispositivi(s, &quadro, porta_param(p))?;
    let col = |v: &[(&str, f64)]| v.iter().map(|(t, w)| ((*t).to_string(), *w)).collect::<Vec<_>>();
    Ok(uscite::Tabella {
        titolo: if quadro.is_empty() { "PULSANTIERA \\ CONTROL PANEL".into() } else { format!("PULSANTIERA QUADRO {quadro} \\ {quadro} CONTROL PANEL") },
        colonne: col(&[("Sigla\nTag", 30.0), ("Funzione\nFunction", 70.0), ("Funzione (2a lingua)\nFunction", 70.0), ("Tipo\nType", 40.0), ("X (mm)", 20.0), ("Y (mm)", 20.0), ("Ø (mm)", 18.0)]),
        righe: v
            .iter()
            .map(|d| {
                [d.sigla.clone(), d.funzione.clone(), d.funzione2.clone(), descrizione_tipo(&d.tipo).to_string(), format!("{:.0}", d.pos.0), format!("{:.0}", d.pos.1), format!("{FORO}")]
                    .into_iter()
                    .map(uscite::Cella::T)
                    .collect()
            })
            .collect(),
        altezza_riga: 6.0,
    })
}

fn run_pulsantiera(s: &mut Session, p: &Value) -> Result<Value> {
    let quadro = quadro_param(p);
    let (v, porta) = dispositivi(s, &quadro, porta_param(p))?;
    if let Some(f) = crate::util::testo_param(p, "file") {
        let righe: Vec<Vec<String>> = v.iter().map(|d| vec![d.sigla.clone(), d.funzione.clone(), format!("{:.1}", d.pos.0), format!("{:.1}", d.pos.1), format!("{FORO}")]).collect();
        crate::morsetti::scrivi_csv(&f, &["Sigla", "Funzione", "X", "Y", "Diametro"], &righe)?;
    }
    Ok(json!({
        "porta": [porta.0, porta.1],
        "dispositivi": v.iter().map(|d| json!({"sigla": d.sigla, "tipo": d.tipo, "funzione": d.funzione, "x": d.pos.0, "y": d.pos.1})).collect::<Vec<_>>(),
        "message": format!("Pulsantiera {quadro}: {} dispositivi su una porta {:.0} × {:.0} mm", v.len(), porta.0, porta.1),
    }))
}

pub fn comandi() -> Vec<cadcraft_engine::CommandSpec> {
    vec![cadcraft_engine::CommandSpec {
        id: "pulsantiera",
        label: "Pulsantiera",
        menu: &["Quadro", "Pulsantiera (foratura porta)"],
        shortcut: None,
        aliases: &[],
        params: "{quadro?, larghezza?, altezza? (porta, mm; default dalla piastra del lay-out), file?: CSV della foratura} → {porta, dispositivi: [{sigla, tipo, funzione, x, y}]}; sul foglio: USCITA {tipo: pulsantiera}",
        enabled: cadcraft_engine::cmd::has_doc,
        run: run_pulsantiera,
        undoable: false,
        interactive: None,
        transparent: false,
    }]
}
