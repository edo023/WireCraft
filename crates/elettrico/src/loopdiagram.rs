//! Loop diagram: per ogni ingresso e uscita dei PLC il percorso fino al dispositivo in campo,
//! con i numeri dei fili, i morsetti attraversati, il cavo e gli altri capi del dispositivo
//! (l'alimentazione di un sensore, il ritorno di un attuatore).
//!
//! Il percorso parte dalla rete che arriva sul bit e passa per morsetti e rimandi (anche su altri
//! fogli) fino al primo componente: il dispositivo. È un'uscita come le altre (`USCITA {tipo:
//! loop}`), su fogli nuovi divisi in tre zone (campo, quadro, PLC) e aggiornata da Aggiorna Liste
//! e Tabelle; la stessa cosa in tabella esce con `tabella("loop")`.

use cadcraft_engine::{Result, Session};
use serde_json::{Value, json};

use crate::collegamenti::{self, Rete, TipoCapo};
use crate::numerazione::su;
use crate::util::{colore, esegui, handles, linea, testo};
use crate::{cavi, fili, plc, siglatura, squadratura, uscite};

type P = (f64, f64);

const TOLL: f64 = 0.01;
/// Altezza di un loop sulla pagina e della testata delle zone (mm).
const FASCIA: f64 = 28.0;
const TESTATA: f64 = 16.0;
/// Limiti delle zone: campo fino a 130, quadro fino a 260, poi il PLC.
const ZONA_QUADRO: f64 = 130.0;
const ZONA_PLC: f64 = 260.0;

/// Il percorso di un bit.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Loop {
    pub indirizzo: String,
    pub ingresso: bool,
    pub commento: String,
    pub commento2: String,
    pub scheda: String,
    pub pin_scheda: String,
    pub quadro: String,
    /// Dove sta il bit (`foglio/colonnaRiga`).
    pub riferimento: String,
    /// Numeri dei fili attraversati, dal PLC verso il campo.
    pub fili: Vec<String>,
    /// Morsetti attraversati (`-X1:5`), dal PLC verso il campo.
    pub morsetti: Vec<String>,
    /// Vene di cavo posate sul percorso (`-W01 BN`).
    pub cavi: Vec<String>,
    /// Il dispositivo raggiunto: sigla, morsetto, descrizione.
    pub dispositivo: Option<(String, String, String)>,
    /// Gli altri morsetti del dispositivo con la rete a cui vanno (`13`, `+24V`).
    pub altri: Vec<(String, String)>,
    /// Il bit va dritto a una linea di alimentazione (`+24V`), senza dispositivo.
    pub potenziale: Option<String>,
}

/// La rete passa per il punto `p` (su un filo o in un estremo).
fn tocca(r: &Rete, p: P) -> bool {
    r.fili.iter().any(|f| su(f, p))
}

fn vicini(a: P, b: P) -> bool {
    (a.0 - b.0).abs() < TOLL && (a.1 - b.1).abs() < TOLL
}

/// I loop di tutti i bit dei PLC, nell'ordine dei bit (foglio, posizione).
pub fn loops(s: &mut Session) -> Result<Vec<Loop>> {
    let bits = plc::bit(s)?;
    if bits.is_empty() {
        return Ok(Vec::new());
    }
    let reti = collegamenti::reti(s, &[])?;
    let rimandi = fili::rimandi(s)?;
    let coppie = fili::abbina(&rimandi);
    let pose = cavi::pose(s)?;
    let descrizioni: Vec<(String, String)> = siglatura::componenti(s)?
        .into_iter()
        .filter(|c| c.madre())
        .map(|c| {
            let d = [c.blocco.attributo("FUNZIONE1"), c.blocco.attributo("DESCRIZIONE")].into_iter().find(|t| !t.trim().is_empty()).unwrap_or("").to_string();
            (c.nome, d)
        })
        .collect();
    // la rete dall'altra parte di un rimando
    let oltre = |r: &Rete, pos: P| -> Option<usize> {
        let i = rimandi.iter().position(|x| x.foglio == r.foglio && vicini(x.pos, pos))?;
        let j = coppie.iter().find_map(|&(p, a)| if p == i { Some(a) } else if a == i { Some(p) } else { None })?;
        let altro = &rimandi[j];
        reti.iter().position(|q| q.foglio == altro.foglio && q.capi.iter().any(|c| c.tipo == TipoCapo::Rimando && vicini(c.pos, altro.pos)))
    };
    // le reti delle linee di alimentazione: lì il percorso si ferma, i dispositivi appesi alla
    // linea non sono del bit
    let alimentazione: Vec<bool> = reti
        .iter()
        .map(|r| rimandi.iter().any(|x| x.famiglia == fili::Famiglia::Alimentazione && x.foglio == r.foglio && r.capi.iter().any(|c| c.tipo == TipoCapo::Rimando && vicini(c.pos, x.pos))))
        .collect();
    let mut out = Vec::new();
    for b in &bits {
        let scheda = b.blocco.attributo("NOME").to_string();
        let mut l = Loop {
            indirizzo: b.indirizzo().to_string(),
            ingresso: b.ingresso,
            commento: b.blocco.attributo("COMMENTO1").to_string(),
            commento2: b.blocco.attributo("COMMENTO3").to_string(),
            scheda: scheda.clone(),
            pin_scheda: b.blocco.attributo("PINA1").to_string(),
            quadro: b.blocco.attributo("QUADRO").to_string(),
            riferimento: squadratura::riferimento(b.foglio, b.blocco.pos.0, b.blocco.pos.1),
            ..Loop::default()
        };
        let mut coda: std::collections::VecDeque<usize> = reti.iter().enumerate().filter(|(_, r)| r.foglio == b.foglio && tocca(r, b.blocco.pos)).map(|(i, _)| i).collect();
        let mut viste: Vec<usize> = coda.iter().copied().collect();
        let mut blocco_dispositivo = String::new();
        while let Some(i) = coda.pop_front() {
            let r = &reti[i];
            if alimentazione[i] {
                if l.potenziale.is_none() && l.dispositivo.is_none() {
                    l.potenziale = Some(if r.nome.trim().is_empty() { "alimentazione".into() } else { r.nome.clone() });
                }
                continue;
            }
            if !r.nome.trim().is_empty() && !l.fili.contains(&r.nome) {
                l.fili.push(r.nome.clone());
            }
            for p in pose.iter().filter(|p| p.foglio == r.foglio && tocca(r, p.pos)) {
                let t = format!("{} {}", p.cavo, p.vena).trim().to_string();
                if !l.cavi.contains(&t) {
                    l.cavi.push(t);
                }
            }
            let mut avanti = Vec::new();
            for c in &r.capi {
                match c.tipo {
                    TipoCapo::Morsetto => {
                        let m = c.testo();
                        if !l.morsetti.contains(&m) {
                            l.morsetti.push(m);
                        }
                        avanti.extend(reti.iter().enumerate().filter(|(_, q)| q.capi.iter().any(|x| x.tipo == TipoCapo::Morsetto && x.sigla == c.sigla && x.pin == c.pin)).map(|(k, _)| k));
                    }
                    TipoCapo::Rimando => avanti.extend(oltre(r, c.pos)),
                    TipoCapo::Componente if l.dispositivo.is_none() && !c.sigla.eq_ignore_ascii_case(&scheda) => {
                        let d = descrizioni.iter().find(|(n, _)| n.eq_ignore_ascii_case(&c.sigla)).map(|(_, d)| d.clone()).unwrap_or_default();
                        l.dispositivo = Some((c.sigla.clone(), c.pin.clone(), d));
                        blocco_dispositivo = c.blocco.clone();
                    }
                    TipoCapo::Componente => {}
                }
            }
            for k in avanti {
                if !viste.contains(&k) {
                    viste.push(k);
                    coda.push_back(k);
                }
            }
        }
        // gli altri morsetti dello stesso simbolo (non di altri contatti con la stessa sigla)
        if let Some((_, pin, _)) = &l.dispositivo {
            for (k, r) in reti.iter().enumerate().filter(|(k, _)| !viste.contains(k)) {
                for c in r.capi.iter().filter(|c| c.tipo == TipoCapo::Componente && c.blocco == blocco_dispositivo && &c.pin != pin) {
                    let nome = if r.nome.trim().is_empty() { format!("rete {}", k + 1) } else { r.nome.clone() };
                    if !l.altri.iter().any(|(p, _)| *p == c.pin) {
                        l.altri.push((c.pin.clone(), nome));
                    }
                }
            }
            l.altri.sort();
        }
        out.push(l);
    }
    Ok(out)
}

/// Le righe della tabella dei loop.
pub fn righe(v: &[Loop]) -> Vec<Vec<String>> {
    v.iter()
        .map(|l| {
            let disp = l.dispositivo.as_ref().map(|(s, p, _)| if p.is_empty() { s.clone() } else { format!("{s}:{p}") }).unwrap_or_default();
            vec![
                l.indirizzo.clone(),
                l.commento.clone(),
                if l.pin_scheda.is_empty() { l.scheda.clone() } else { format!("{}:{}", l.scheda, l.pin_scheda) },
                l.fili.join(", "),
                l.morsetti.join(", "),
                l.cavi.join(", "),
                disp,
                match &l.potenziale {
                    Some(p) if l.dispositivo.is_none() => format!("potenziale {p}"),
                    _ => l.altri.iter().map(|(p, r)| format!("{p}: {r}")).collect::<Vec<_>>().join(", "),
                },
            ]
        })
        .collect()
}

/// Un testo accorciato a `n` caratteri.
fn corto(t: &str, n: usize) -> String {
    if t.chars().count() <= n {
        return t.to_string();
    }
    let mut s: String = t.chars().take(n.saturating_sub(1)).collect();
    s.push('…');
    s
}

/// Loop per pagina.
pub fn per_pagina() -> usize {
    (((uscite::AREA.1 - TESTATA) / FASCIA).floor() as usize).max(1)
}

/// Disegna la pagina con i loop `v` (coordinate del blocco: origine in alto a sinistra).
fn disegna_pagina(s: &mut Session, v: &[Loop], h: &mut Vec<String>) -> Result<()> {
    let (largo, alto) = uscite::AREA;
    colore(s, "7")?;
    testo(s, h, (largo / 2.0, -4.5), "LOOP DIAGRAM", 4.0, "MC")?;
    for (x, t) in [(ZONA_QUADRO / 2.0, "CAMPO \\ FIELD"), ((ZONA_QUADRO + ZONA_PLC) / 2.0, "QUADRO \\ PANEL"), ((ZONA_PLC + largo) / 2.0, "PLC")] {
        testo(s, h, (x, -12.0), t, 3.0, "MC")?;
    }
    linea(s, h, &[(0.0, -TESTATA), (largo, -TESTATA)], false)?;
    colore(s, "8")?;
    for x in [ZONA_QUADRO, ZONA_PLC] {
        linea(s, h, &[(x, -9.0), (x, -alto)], false)?;
    }
    for (i, l) in v.iter().enumerate() {
        let yt = -TESTATA - FASCIA * i as f64;
        let yc = yt - FASCIA / 2.0;
        colore(s, "8")?;
        linea(s, h, &[(0.0, yt - FASCIA), (largo, yt - FASCIA)], false)?;
        // PLC: indirizzo, scheda e morsetto, commenti
        colore(s, "7")?;
        let (px0, px1) = (ZONA_PLC + 15.0, largo - 5.0);
        linea(s, h, &[(px0, yc - 9.0), (px1, yc - 9.0), (px1, yc + 9.0), (px0, yc + 9.0)], true)?;
        colore(s, "1")?;
        testo(s, h, (px0 + 3.0, yc + 4.5), &format!("{} {}", l.indirizzo, if l.ingresso { "IN" } else { "OUT" }), 3.5, "ML")?;
        colore(s, "7")?;
        let scheda = if l.pin_scheda.is_empty() { l.scheda.clone() } else { format!("{}:{}", l.scheda, l.pin_scheda) };
        testo(s, h, (px0 + 3.0, yc), &format!("{scheda}   {}", l.riferimento), 2.2, "ML")?;
        colore(s, "4")?;
        testo(s, h, (px0 + 3.0, yc - 4.0), &corto(&l.commento, 44), 2.0, "ML")?;
        colore(s, "8")?;
        testo(s, h, (px0 + 3.0, yc - 7.0), &corto(&l.commento2, 50), 1.6, "ML")?;
        // dispositivo in campo
        let (dx0, dx1) = (30.0, 100.0);
        colore(s, "7")?;
        linea(s, h, &[(dx0, yc - 9.0), (dx1, yc - 9.0), (dx1, yc + 9.0), (dx0, yc + 9.0)], true)?;
        match &l.dispositivo {
            Some((sigla, pin, descrizione)) => {
                colore(s, "4")?;
                testo(s, h, (dx0 + 8.0, yc + 4.5), sigla, 3.0, "ML")?;
                colore(s, "7")?;
                testo(s, h, (dx0 + 8.0, yc - 1.0), &corto(descrizione, 30), 2.0, "ML")?;
                colore(s, "3")?;
                testo(s, h, (dx1 - 1.5, yc + 1.5), pin, 2.0, "MR")?;
                // gli altri capi, a sinistra
                for (k, (p, rete)) in l.altri.iter().take(2).enumerate() {
                    let y = yc + 4.0 - 8.0 * k as f64;
                    colore(s, "7")?;
                    linea(s, h, &[(4.0, y), (dx0, y)], false)?;
                    colore(s, "3")?;
                    testo(s, h, (dx0 + 1.5, y), p, 2.0, "ML")?;
                    colore(s, "7")?;
                    testo(s, h, (5.0, y + 1.6), &corto(rete, 10), 2.0, "ML")?;
                }
            }
            None => {
                colore(s, "8")?;
                let t = l.potenziale.as_ref().map_or_else(|| "non collegato".to_string(), |p| format!("potenziale {p}"));
                testo(s, h, ((dx0 + dx1) / 2.0, yc), &corto(&t, 26), 2.5, "MC")?;
            }
        }
        // il filo dal dispositivo al PLC, con i morsetti nella zona del quadro
        colore(s, "7")?;
        linea(s, h, &[(dx1, yc), (px0, yc)], false)?;
        let n = l.morsetti.len();
        let passo = (ZONA_PLC - ZONA_QUADRO - 20.0) / n.max(1) as f64;
        for (j, m) in l.morsetti.iter().enumerate() {
            // il primo morsetto è il più vicino al PLC
            let x = ZONA_PLC - 10.0 - passo * (j as f64 + 0.5);
            colore(s, "7")?;
            h.extend(handles(&esegui(s, "circle", json!({"center": [x, yc], "radius": 1.2}))?));
            colore(s, "3")?;
            testo(s, h, (x, yc + 3.0), &corto(m, 14), 2.0, "MC")?;
        }
        colore(s, "7")?;
        if let Some(f) = l.fili.first() {
            testo(s, h, (ZONA_PLC + 7.5, yc + 1.8), &corto(f, 8), 1.8, "MC")?;
        }
        if let Some(f) = l.fili.last() {
            testo(s, h, (dx1 + 15.0, yc + 1.8), &corto(f, 12), 1.8, "MC")?;
        }
        if !l.cavi.is_empty() {
            colore(s, "6")?;
            testo(s, h, (dx1 + 15.0, yc - 2.8), &corto(&l.cavi.join(", "), 14), 1.8, "MC")?;
        }
    }
    Ok(())
}

/// Disegna il loop diagram dal foglio `primo` (default dopo l'ultimo).
pub fn uscita(s: &mut Session, primo: Option<u32>) -> Result<Vec<u32>> {
    let v = loops(s)?;
    let n = per_pagina();
    let pagine: Vec<&[Loop]> = if v.is_empty() { vec![&[]] } else { v.chunks(n).collect() };
    uscite::disegna_con(s, pagine.len(), "loop", &json!({}), primo, "Loop diagram", "Loop Diagram", |s, k, h| disegna_pagina(s, pagine[k], h))
}

/// La versione in tabella.
pub fn tabella(s: &mut Session) -> Result<uscite::Tabella> {
    let righe = righe(&loops(s)?).into_iter().map(|r| r.into_iter().map(uscite::Cella::T).collect()).collect();
    let col = |v: &[(&str, f64)]| v.iter().map(|(t, w)| ((*t).to_string(), *w)).collect::<Vec<_>>();
    Ok(uscite::Tabella {
        titolo: "LOOP DIAGRAM".into(),
        colonne: col(&[
            ("Indirizzo\nAddress", 22.0),
            ("Commento\nComment", 70.0),
            ("Scheda\nCard", 28.0),
            ("Fili\nWires", 30.0),
            ("Morsetti\nTerminals", 60.0),
            ("Cavo\nCable", 32.0),
            ("Dispositivo\nDevice", 34.0),
            ("Altri capi\nOther ends", 50.0),
        ]),
        righe,
        altezza_riga: 6.0,
    })
}

pub fn json_loop(l: &Loop) -> Value {
    json!({
        "indirizzo": l.indirizzo, "ingresso": l.ingresso, "commento": l.commento, "scheda": l.scheda, "pinScheda": l.pin_scheda,
        "riferimento": l.riferimento, "fili": l.fili, "morsetti": l.morsetti, "cavi": l.cavi,
        "dispositivo": l.dispositivo.as_ref().map(|(s, p, d)| json!({"sigla": s, "pin": p, "descrizione": d})),
        "altri": l.altri.iter().map(|(p, r)| json!({"pin": p, "rete": r})).collect::<Vec<_>>(),
        "potenziale": l.potenziale,
    })
}

fn run_loops(s: &mut Session, _p: &Value) -> Result<Value> {
    let v = loops(s)?;
    let senza = v.iter().filter(|l| l.dispositivo.is_none() && l.potenziale.is_none()).count();
    Ok(json!({"loop": v.iter().map(json_loop).collect::<Vec<_>>(), "message": format!("{} ingressi e uscite, {senza} senza dispositivo collegato", v.len())}))
}

pub fn comandi() -> Vec<cadcraft_engine::CommandSpec> {
    vec![cadcraft_engine::CommandSpec {
        id: "loopdiagram",
        label: "Loop Diagram",
        menu: &["Progetto", "Gestione PLC", "Loop Diagram (elenco)"],
        shortcut: None,
        aliases: &[],
        params: "{} → {loop: [{indirizzo, commento, scheda, fili, morsetti, cavi, dispositivo, altri}]}; sui fogli: USCITA {tipo: loop}",
        enabled: cadcraft_engine::cmd::has_doc,
        run: run_loops,
        undoable: false,
        interactive: None,
        transparent: false,
    }]
}
