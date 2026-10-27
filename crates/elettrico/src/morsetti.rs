//! Morsetti e morsettiere.
//!
//! Un morsetto è un blocco posato su un filo, con gli attributi dei CAD elettrici diffusi (vedi
//! `docs/spac-modello-dati.md`): `MORSETTIERA` (`-XG`), `NUMM` (numero del morsetto), `NUMI` e
//! `NUMO` (filo in ingresso e in uscita, scritti dalla numerazione fili), `QUADRO`, `LIVELLO`,
//! `SEZIONE`. Si leggono anche i morsetti `_MORS*` dei progetti esistenti.
//!
//! I connettori sono pin con gli stessi dati (il connettore al posto della morsettiera, il pin
//! al posto del numero) e il sesso (`SESSO`: M/F); escono nelle stesse liste dei morsetti.

use cadcraft_engine::cmd::has_doc;
use cadcraft_engine::{Accept, CommandSpec, Input, Interactive, Prompt, Result, Session, Step};
use serde_json::{Value, json};

use crate::fili;
use crate::fogli;
use crate::squadratura;
use crate::util::{Inserito, allinea_a_destra, blocchi_definiti, blocchi_inseriti, blocco, colore, errore, esegui, handles, in_gruppo, layer_corrente, punto_param, su_layer_zero, testo_param};

pub const BLOCCO_MORSETTO: &str = "CE_MORSETTO";
pub const BLOCCO_CONNETTORE: &str = "CE_CONNETTORE";
const FILTRO: &str = "CE_MORSETTO,_MORS*,CE_CONNETTORE";
const TOLL: f64 = 0.01;

type P = (f64, f64);

/// Il blocco del morsetto: cerchio sul filo, numero del morsetto a sinistra in alto, numero
/// del filo a destra in alto, nome della morsettiera a destra in basso (nascosto sui morsetti
/// che seguono il primo di una fila). Il pin di connettore ha la presa (semicerchio) e la
/// spina (rettangolo pieno) al posto del cerchio.
fn crea_blocco(s: &mut Session, nome: &str) -> Result<()> {
    if blocchi_definiti(s)?.iter().any(|b| b.eq_ignore_ascii_case(nome)) {
        return Ok(());
    }
    let connettore = nome == BLOCCO_CONNETTORE;
    su_layer_zero(s, |s| {
        colore(s, "7")?;
        let mut h = if connettore {
            let mut h = handles(&esegui(s, "arc", json!({"center": [0.0, -0.4], "radius": 1.3, "start": 180.0, "end": 360.0}))?);
            h.extend(handles(&esegui(s, "pline", json!({"vertices": [[0.0, 0.2], [0.0, 1.6]], "width": 0.9}))?));
            h
        } else {
            handles(&esegui(s, "circle", json!({"center": [0.0, 0.0], "radius": 1.0}))?)
        };
        let att = |s: &mut Session, h: &mut Vec<String>, tag: &str, at: P, alt: f64, nascosto: bool, def: &str| -> Result<String> {
            let r = esegui(s, "attdef", json!({"tag": tag, "prompt": tag, "default": def, "at": [at.0, at.1], "height": alt, "invisible": nascosto}))?;
            let n = handles(&r);
            h.extend(n.iter().cloned());
            Ok(n.into_iter().next().unwrap_or_default())
        };
        colore(s, "3")?;
        let numero = att(s, &mut h, "NUMM", (-1.4, 0.6), 1.8, false, "")?;
        allinea_a_destra(s, &numero)?;
        colore(s, "BYLAYER")?;
        att(s, &mut h, "NUMI", (1.4, 0.6), 1.8, true, "")?;
        colore(s, "4")?;
        att(s, &mut h, "MORSETTIERA", (1.4, -2.4), 1.8, false, "")?;
        for (tag, def) in [("NUMO", ""), ("QUADRO", ""), ("LIVELLO", "1"), ("SEZIONE", ""), ("COSTRUTTORE", ""), ("TIPO", ""), ("UTENZA", ""), ("NOTEM", "")] {
            att(s, &mut h, tag, (0.0, -2.0), 1.0, true, def)?;
        }
        if connettore {
            att(s, &mut h, "SESSO", (0.0, -2.0), 1.0, true, "F")?;
        }
        blocco(s, nome, std::mem::take(&mut h), if connettore { "Pin di connettore" } else { "Morsetto" })
    })
}

/// Mostra o nasconde l'attributo `tag` di un blocco inserito.
pub fn mostra_attributo(s: &mut Session, handle: &str, tag: &str, visibile: bool) -> Result<()> {
    use cadcraft_engine::doc::EntityKind;
    let h = cadcraft_engine::doc::Handle::parse_hex(handle).ok_or_else(|| errore(format!("handle non valido: {handle}")))?;
    s.doc_mut()?.modify_entity(h, |e| {
        if let EntityKind::Insert(i) = &mut e.kind {
            for a in i.attribs.iter_mut().filter(|a| a.tag.eq_ignore_ascii_case(tag)) {
                a.invisible = !visibile;
            }
        }
    })?;
    Ok(())
}

/// Un morsetto letto dal disegno.
#[derive(Clone, Debug)]
pub struct Morsetto {
    pub blocco: Inserito,
    pub morsettiera: String,
    pub numero: String,
    pub foglio: u32,
}

pub fn morsetti(s: &mut Session) -> Result<Vec<Morsetto>> {
    Ok(blocchi_inseriti(s, FILTRO)?
        .into_iter()
        .filter(|b| !b.attributo("MORSETTIERA").is_empty())
        .map(|b| Morsetto {
            morsettiera: b.attributo("MORSETTIERA").to_string(),
            numero: b.attributo("NUMM").to_string(),
            foglio: fogli::numero_da_layer(&b.layer).unwrap_or(0),
            blocco: b,
        })
        .collect())
}

/// Il prossimo numero libero di una morsettiera.
fn prossimo(tutti: &[Morsetto], morsettiera: &str) -> u32 {
    tutti.iter().filter(|m| m.morsettiera.eq_ignore_ascii_case(morsettiera)).filter_map(|m| m.numero.trim().parse::<u32>().ok()).max().unwrap_or(0) + 1
}

/// Posa un morsetto (o il pin di un connettore, con il sesso) in `p` con il numero dato (o il
/// prossimo libero).
pub fn posa(s: &mut Session, morsettiera: &str, p: P, numero: Option<String>, quadro: &str, sesso: Option<&str>) -> Result<(String, String)> {
    let nome = if sesso.is_some() { BLOCCO_CONNETTORE } else { BLOCCO_MORSETTO };
    crea_blocco(s, nome)?;
    let numero = match numero {
        Some(n) => n,
        None => prossimo(&morsetti(s)?, morsettiera).to_string(),
    };
    let quadro = if quadro.trim().is_empty() { crate::impostazioni::quadro_corrente().unwrap_or_default() } else { quadro.trim().to_string() };
    let mut attr = json!({"MORSETTIERA": morsettiera, "NUMM": numero, "QUADRO": quadro, "LIVELLO": "1"});
    if let Some(x) = sesso {
        attr["SESSO"] = json!(x);
    }
    let r = esegui(s, "insert", json!({"name": nome, "at": [p.0, p.1], "attribs": attr}))?;
    let h = handles(&r).into_iter().next().ok_or_else(|| errore("morsetto non inserito"))?;
    Ok((h, numero))
}

/// I punti dove la linea `da`–`a` (orizzontale o verticale) attraversa fili perpendicolari, in
/// ordine lungo la linea.
pub fn attraversamenti(fili: &[fili::Segmento], da: P, a: P) -> Vec<P> {
    let orizzontale = (a.1 - da.1).abs() <= (a.0 - da.0).abs();
    let mut punti: Vec<P> = Vec::new();
    for f in fili {
        if orizzontale {
            let y = da.1;
            let (x0, x1) = (da.0.min(a.0), da.0.max(a.0));
            if (f.a.0 - f.b.0).abs() < TOLL && f.a.0 > x0 - TOLL && f.a.0 < x1 + TOLL && y > f.a.1.min(f.b.1) - TOLL && y < f.a.1.max(f.b.1) + TOLL {
                punti.push((f.a.0, y));
            }
        } else {
            let x = da.0;
            let (y0, y1) = (da.1.min(a.1), da.1.max(a.1));
            if (f.a.1 - f.b.1).abs() < TOLL && f.a.1 > y0 - TOLL && f.a.1 < y1 + TOLL && x > f.a.0.min(f.b.0) - TOLL && x < f.a.0.max(f.b.0) + TOLL {
                punti.push((x, f.a.1));
            }
        }
    }
    let verso = if orizzontale { (a.0 - da.0).signum() } else { (a.1 - da.1).signum() };
    punti.sort_by(|p, q| {
        let (u, v) = if orizzontale { (p.0, q.0) } else { (p.1, q.1) };
        (u * verso).total_cmp(&(v * verso))
    });
    punti.dedup_by(|p, q| (p.0 - q.0).abs() < TOLL && (p.1 - q.1).abs() < TOLL);
    punti
}

fn run_morsetti(s: &mut Session, p: &Value) -> Result<Value> {
    posa_fila(s, p, None)
}

fn run_connettori(s: &mut Session, p: &Value) -> Result<Value> {
    let sesso = match testo_param(p, "sesso").unwrap_or_default().trim().to_ascii_uppercase().as_str() {
        "M" | "MASCHIO" | "MALE" => "M",
        _ => "F",
    };
    posa_fila(s, p, Some(sesso))
}

/// Posa una fila di morsetti (o di pin di connettore) su un punto o sui fili attraversati.
fn posa_fila(s: &mut Session, p: &Value, sesso: Option<&str>) -> Result<Value> {
    let chiave = if sesso.is_some() { "connettore" } else { "morsettiera" };
    let morsettiera = testo_param(p, chiave)
        .or_else(|| testo_param(p, "morsettiera"))
        .filter(|m| !m.trim().is_empty())
        .ok_or_else(|| errore(if sesso.is_some() { "Indicare il connettore (es. -XC1)" } else { "Indicare la morsettiera (es. -XG)" }))?;
    let quadro = testo_param(p, "quadro").unwrap_or_default();
    in_gruppo(s, if sesso.is_some() { "Connettori" } else { "Morsetti" }, |s| {
        let punti = match (punto_param(p, "at"), punto_param(p, "da"), punto_param(p, "a")) {
            (Some(at), _, _) => vec![at],
            (None, Some(da), Some(a)) => {
                let layer = layer_corrente(s)?;
                attraversamenti(&fili::fili(s, &layer)?, da, a)
            }
            _ => return Err(errore("Indicare un punto (at) o una linea che attraversa i fili (da, a)")),
        };
        if punti.is_empty() {
            return Err(errore("La linea non attraversa nessun filo"));
        }
        let mut numero = testo_param(p, "numero");
        let mut posati = Vec::new();
        let solo_primo = crate::impostazioni::correnti().morsettiera_solo_primo;
        for (k, q) in punti.into_iter().enumerate() {
            let (h, n) = posa(s, &morsettiera, q, numero.take(), &quadro, sesso)?;
            // come nei CAD elettrici diffusi il nome della morsettiera solo sul primo della fila
            if solo_primo && k > 0 {
                mostra_attributo(s, &h, "MORSETTIERA", false)?;
            }
            posati.push(json!({"handle": h, "numero": n}));
        }
        crate::numerazione::aggiorna_morsetti(s)?;
        let numeri: Vec<String> = posati.iter().filter_map(|m| m["numero"].as_str().map(str::to_string)).collect();
        let msg = if sesso.is_some() { format!("Connettore {morsettiera}: pin {}", numeri.join(", ")) } else { format!("Morsettiera {morsettiera}: morsetti {}", numeri.join(", ")) };
        Ok(json!({"morsetti": posati, "message": msg}))
    })
}

/// Le righe della morsettiera, in ordine di numero: (morsetto, filo, posizione, quadro).
pub fn righe(s: &mut Session, morsettiera: &str) -> Result<Vec<[String; 4]>> {
    let mut m: Vec<Morsetto> = morsetti(s)?.into_iter().filter(|m| m.morsettiera.eq_ignore_ascii_case(morsettiera)).collect();
    m.sort_by(|a, b| {
        let (na, nb) = (a.numero.parse::<u64>().ok(), b.numero.parse::<u64>().ok());
        na.cmp(&nb).then_with(|| a.numero.cmp(&b.numero))
    });
    Ok(m.iter()
        .map(|m| {
            let filo = if m.blocco.attributo("NUMI").is_empty() { m.blocco.attributo("NUMO") } else { m.blocco.attributo("NUMI") };
            [m.numero.clone(), filo.to_string(), squadratura::riferimento(m.foglio, m.blocco.pos.0, m.blocco.pos.1), m.blocco.attributo("QUADRO").to_string()]
        })
        .collect())
}

/// Scrive righe in CSV (separatore `;`, come lo apre Excel in italiano).
pub fn scrivi_csv(percorso: &str, intestazione: &[&str], righe: &[Vec<String>]) -> Result<()> {
    let campo = |t: &str| if t.contains([';', '"', '\n']) { format!("\"{}\"", t.replace('"', "\"\"")) } else { t.to_string() };
    let mut out = intestazione.iter().map(|t| campo(t)).collect::<Vec<_>>().join(";");
    out.push('\n');
    for r in righe {
        out.push_str(&r.iter().map(|t| campo(t)).collect::<Vec<_>>().join(";"));
        out.push('\n');
    }
    std::fs::write(percorso, out).map_err(|e| errore(format!("{percorso}: {e}")))
}

/// Disegna una tabella in `at` (in alto a sinistra) sul layer corrente.
pub fn tabella(s: &mut Session, at: P, titolo: &str, intestazione: &[&str], righe: &[Vec<String>], larghezze: &[f64]) -> Result<String> {
    let celle: Vec<Vec<String>> = righe.to_vec();
    let r = esegui(
        s,
        "table",
        json!({"at": [at.0, at.1], "rows": righe.len().max(1), "cols": intestazione.len(), "rowHeight": 5.0, "colWidth": larghezze.first().copied().unwrap_or(25.0), "cells": celle, "title": titolo, "header": intestazione}),
    )?;
    let h = handles(&r).into_iter().next().ok_or_else(|| errore("tabella non creata"))?;
    Ok(h)
}

fn run_morsettiera(s: &mut Session, p: &Value) -> Result<Value> {
    let nome = testo_param(p, "morsettiera").filter(|m| !m.trim().is_empty()).ok_or_else(|| errore("Indicare la morsettiera (es. -XG)"))?;
    let intest = ["Morsetto", "Filo", "Posizione", "Quadro"];
    in_gruppo(s, "Morsettiera", |s| {
        let righe: Vec<Vec<String>> = righe(s, &nome)?.into_iter().map(|r| r.to_vec()).collect();
        if righe.is_empty() {
            return Err(errore(format!("La morsettiera {nome} non ha morsetti")));
        }
        if let Some(f) = testo_param(p, "file") {
            scrivi_csv(&f, &intest, &righe)?;
        }
        if let Some(at) = punto_param(p, "at") {
            tabella(s, at, &format!("Morsettiera {nome}"), &intest, &righe, &[20.0])?;
        }
        Ok(json!({"righe": righe, "message": format!("Morsettiera {nome}: {} morsetti", righe.len())}))
    })
}

/// MORSETTI interattivo: morsettiera, poi due punti di una linea che attraversa i fili.
struct PosaMorsetti {
    morsettiera: Option<String>,
    da: Option<P>,
    /// Pin di connettore invece di morsetti.
    connettore: bool,
}

impl PosaMorsetti {
    fn predefinito(&self) -> &'static str {
        if self.connettore { "-XC1" } else { "-X1" }
    }
}

impl Interactive for PosaMorsetti {
    fn name(&self) -> &'static str {
        if self.connettore { "connettori" } else { "morsetti" }
    }
    fn prompt(&self, _s: &Session) -> Prompt {
        match (&self.morsettiera, self.da) {
            (None, _) if self.connettore => Prompt { default: Some(self.predefinito().into()), ..Prompt::new("Nome del connettore", Accept::TEXT) },
            (None, _) => Prompt { default: Some(self.predefinito().into()), ..Prompt::new("Nome della morsettiera", Accept::TEXT) },
            (Some(_), None) => Prompt::new("Primo punto della linea che attraversa i fili", Accept::POINT),
            (Some(_), Some(p)) => Prompt::new("Secondo punto", Accept::POINT).base(cadcraft_engine::geom::Vec2::new(p.0, p.1)),
        }
    }
    fn input(&mut self, s: &mut Session, i: Input) -> Result<Step> {
        match i {
            Input::Cancel => Ok(Step::Cancel),
            Input::Text(t) if self.morsettiera.is_none() => {
                let t = t.trim();
                self.morsettiera = Some(if t.is_empty() { self.predefinito().into() } else if t.starts_with('-') { t.to_ascii_uppercase() } else { format!("-{}", t.to_ascii_uppercase()) });
                Ok(Step::Continue)
            }
            Input::Enter if self.morsettiera.is_none() => {
                self.morsettiera = Some(self.predefinito().into());
                Ok(Step::Continue)
            }
            Input::Point(q) if self.da.is_none() => {
                self.da = Some((q.x, q.y));
                Ok(Step::Continue)
            }
            Input::Point(q) => {
                let da = self.da.unwrap_or((q.x, q.y));
                let r = s.execute(self.name(), &json!({"morsettiera": self.morsettiera.clone().unwrap_or_default(), "da": [da.0, da.1], "a": [q.x, q.y]}))?;
                if let Some(m) = r.get("message").and_then(Value::as_str) {
                    s.echo(m.to_string());
                }
                self.da = None;
                Ok(Step::Continue)
            }
            Input::Enter => Ok(Step::Done),
            _ => Ok(Step::Continue),
        }
    }
}

fn fabbrica(_s: &Session) -> Result<Box<dyn Interactive>> {
    Ok(Box::new(PosaMorsetti { morsettiera: None, da: None, connettore: false }))
}

fn fabbrica_connettori(_s: &Session) -> Result<Box<dyn Interactive>> {
    Ok(Box::new(PosaMorsetti { morsettiera: None, da: None, connettore: true }))
}

pub fn comandi() -> Vec<CommandSpec> {
    vec![
        CommandSpec {
            id: "morsetti",
            label: "Inser MORSETTI",
            menu: &["ELETTRICO", "Inser MORSETTI"],
            shortcut: None,
            aliases: &[],
            params: "{morsettiera: \"-XG\", at: [x,y] | da: [x,y] + a: [x,y] (linea che attraversa i fili), numero?: primo numero, quadro?} → {morsetti: [{handle, numero}]}",
            enabled: has_doc,
            run: run_morsetti,
            undoable: false,
            interactive: Some(fabbrica),
            transparent: false,
        },
        CommandSpec {
            id: "connettori",
            label: "Inser CONNETTORI",
            menu: &["ELETTRICO", "Inser CONNETTORI"],
            shortcut: None,
            aliases: &[],
            params: "{connettore: \"-XC1\", sesso?: M|F (default F), at: [x,y] | da + a (linea che attraversa i fili), numero?: primo pin, quadro?} → {morsetti: [{handle, numero}]}",
            enabled: has_doc,
            run: run_connettori,
            undoable: false,
            interactive: Some(fabbrica_connettori),
            transparent: false,
        },
        CommandSpec {
            id: "morsettiera",
            label: "MORSETTIERE e CONNETTORI",
            menu: &["ELETTRICO", "MORSETTIERE e CONNETTORI"],
            shortcut: None,
            aliases: &[],
            params: "{morsettiera: \"-XG\", at?: [x,y] (tabella nel foglio corrente), file?: percorso .csv} → {righe: [[morsetto, filo, posizione, quadro]]}",
            enabled: has_doc,
            run: run_morsettiera,
            undoable: false,
            interactive: None,
            transparent: false,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seg(a: P, b: P) -> fili::Segmento {
        fili::Segmento { a, b, handle: String::new() }
    }

    #[test]
    fn la_linea_trova_i_fili_attraversati_in_ordine() {
        let f = vec![seg((70.0, 200.0), (70.0, 100.0)), seg((50.0, 200.0), (50.0, 100.0)), seg((60.0, 300.0), (60.0, 250.0)), seg((0.0, 150.0), (40.0, 150.0))];
        assert_eq!(attraversamenti(&f, (40.0, 150.0), (80.0, 150.0)), vec![(50.0, 150.0), (70.0, 150.0)]);
        assert_eq!(attraversamenti(&f, (80.0, 150.0), (40.0, 150.0)), vec![(70.0, 150.0), (50.0, 150.0)]);
    }
}
