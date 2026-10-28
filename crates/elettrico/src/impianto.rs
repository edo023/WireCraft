//! Schema d'impianto: i quadri del progetto come blocchi, con la descrizione (dal box del
//! quadro) e quanti componenti hanno, collegati dai cavi che vanno dall'uno all'altro, con
//! sigla, formazione, lunghezza e funzione.
//!
//! Il percorso di un cavo è quello della sua definizione (da quadro, a quadro); se manca, lo
//! dicono i componenti a cui arrivano le sue vene. È un'uscita come le altre: sul foglio,
//! aggiornabile, e in tabella per Excel.

use cadcraft_engine::{Result, Session};
use serde_json::{Value, json};

use crate::norma81346::Livello;
use crate::util::{blocchi_inseriti, colore, errore, linea, testo};
use crate::{cavi, morsetti, siglatura, uscite};

/// Un quadro dello schema d'impianto.
#[derive(Clone, Debug, PartialEq)]
pub struct Nodo {
    pub nome: String,
    pub descrizione: String,
    pub componenti: usize,
}

/// I cavi che vanno da un quadro a un altro.
#[derive(Clone, Debug, PartialEq)]
pub struct Tratta {
    pub da: String,
    pub a: String,
    pub cavi: Vec<cavi::Cavo>,
}

/// Il nome di un quadro come si scrive: con il `+` davanti.
fn nome_quadro(q: &str) -> String {
    let q = q.trim().trim_start_matches('+').trim();
    if q.is_empty() { String::new() } else { format!("+{q}") }
}

/// I quadri e le tratte dei cavi del progetto.
pub fn dati(s: &mut Session) -> Result<(Vec<Nodo>, Vec<Tratta>)> {
    let componenti = siglatura::componenti(s)?;
    let quadro_di = |sigla: &str| -> Option<String> { componenti.iter().find(|c| c.madre() && c.nome.eq_ignore_ascii_case(sigla)).map(|c| nome_quadro(c.blocco.attributo("QUADRO"))) };
    let mut nodi: Vec<Nodo> = Vec::new();
    let aggiungi = |nodi: &mut Vec<Nodo>, q: &str, conta: bool| {
        if q.is_empty() {
            return;
        }
        match nodi.iter_mut().find(|n| n.nome.eq_ignore_ascii_case(q)) {
            Some(n) => n.componenti += usize::from(conta),
            None => nodi.push(Nodo { nome: q.to_string(), descrizione: String::new(), componenti: usize::from(conta) }),
        }
    };
    for c in componenti.iter().filter(|c| c.madre() && c.foglio > 0) {
        aggiungi(&mut nodi, &nome_quadro(c.blocco.attributo("QUADRO")), true);
    }
    let morsettiere = morsetti::morsetti(s)?;
    let quadro_morsettiera = |m: &str| morsettiere.iter().find(|x| x.morsettiera.eq_ignore_ascii_case(m)).map(|x| nome_quadro(x.blocco.attributo("QUADRO")));
    let tutti_cavi = cavi::cavi(s)?;
    // dove arrivano le vene, per i cavi senza percorso
    let conduttori = if tutti_cavi.iter().any(|c| c.da_quadro.trim().is_empty() || c.a_quadro.trim().is_empty()) { cavi::conduttori(s)? } else { Vec::new() };
    let mut tratte: Vec<Tratta> = Vec::new();
    for c in tutti_cavi {
        let (mut da, mut a) = (nome_quadro(&c.da_quadro), nome_quadro(&c.a_quadro));
        if da.is_empty() || a.is_empty() {
            let mut visti: Vec<String> = Vec::new();
            for v in conduttori.iter().filter(|v| v.cavo.eq_ignore_ascii_case(&c.sigla)) {
                for capo in [&v.da, &v.a] {
                    let sigla = capo.split(':').next().unwrap_or_default();
                    if let Some(q) = quadro_di(sigla).or_else(|| quadro_morsettiera(sigla)).filter(|q| !q.is_empty())
                        && !visti.iter().any(|x| x.eq_ignore_ascii_case(&q))
                    {
                        visti.push(q);
                    }
                }
            }
            if da.is_empty() {
                da = visti.iter().find(|q| !q.eq_ignore_ascii_case(&a)).cloned().unwrap_or_default();
            }
            if a.is_empty() {
                a = visti.iter().find(|q| !q.eq_ignore_ascii_case(&da)).cloned().unwrap_or_default();
            }
        }
        if da.is_empty() || a.is_empty() || da.eq_ignore_ascii_case(&a) {
            continue;
        }
        aggiungi(&mut nodi, &da, false);
        aggiungi(&mut nodi, &a, false);
        match tratte.iter_mut().find(|t| (t.da.eq_ignore_ascii_case(&da) && t.a.eq_ignore_ascii_case(&a)) || (t.da.eq_ignore_ascii_case(&a) && t.a.eq_ignore_ascii_case(&da))) {
            Some(t) => t.cavi.push(c),
            None => tratte.push(Tratta { da, a, cavi: vec![c] }),
        }
    }
    // le descrizioni dai box dei quadri
    for b in blocchi_inseriti(s, "CE_BOX_*")? {
        if Livello::da_codice(b.attributo("TIPO")) != Some(Livello::Quadro) {
            continue;
        }
        let q = nome_quadro(b.attributo("VALORE"));
        if let Some(n) = nodi.iter_mut().find(|n| n.nome.eq_ignore_ascii_case(&q))
            && n.descrizione.is_empty()
        {
            n.descrizione = b.attributo("DESCRIZIONE").trim().to_string();
        }
    }
    // prima i quadri con più componenti
    nodi.sort_by(|a, b| b.componenti.cmp(&a.componenti).then_with(|| a.nome.cmp(&b.nome)));
    Ok((nodi, tratte))
}

/// L'etichetta di un cavo: sigla, formazione, lunghezza e, se c'è posto (`spazio` mm), la
/// funzione.
fn testo_cavo(c: &cavi::Cavo, spazio: f64) -> String {
    let mut t = format!("{}  {}", c.sigla, c.formazione);
    if !c.lunghezza.trim().is_empty() {
        t.push_str(&format!("  {} m", c.lunghezza.trim()));
    }
    let con_funzione = format!("{t}  {}", c.funzione.trim());
    if !c.funzione.trim().is_empty() && larghezza_testo(&con_funzione) <= spazio { con_funzione } else { t }
}

/// Larghezza (mm) di un testo alto 2 mm.
fn larghezza_testo(t: &str) -> f64 {
    t.chars().count() as f64 * 1.6
}

/// Quadri al massimo in una fila.
const MAX_NODI: usize = 8;

fn disegna_pagina(s: &mut Session, nodi: &[Nodo], tratte: &[Tratta], h: &mut Vec<String>) -> Result<()> {
    let (largo, alto) = uscite::AREA;
    colore(s, "7")?;
    testo(s, h, (largo / 2.0, -4.5), "SCHEMA D'IMPIANTO \\ PLANT DIAGRAM", 4.0, "MC")?;
    let n = nodi.len().min(MAX_NODI);
    let spazio = 12.0;
    let w = ((largo - spazio * (n.max(1) - 1) as f64) / n.max(1) as f64).min(70.0);
    let x0 = (largo - (w * n as f64 + spazio * (n.max(1) - 1) as f64)) / 2.0;
    let (alto_box, basso_box) = (-20.0, -55.0);
    let centro = |i: usize| x0 + i as f64 * (w + spazio) + w / 2.0;
    for (i, nodo) in nodi.iter().take(n).enumerate() {
        let x = x0 + i as f64 * (w + spazio);
        colore(s, "7")?;
        linea(s, h, &[(x, alto_box), (x + w, alto_box), (x + w, basso_box), (x, basso_box)], true)?;
        testo(s, h, (x + w / 2.0, alto_box - 8.0), &nodo.nome, 4.0, "MC")?;
        let max = (w / 1.9) as usize;
        let d: String = nodo.descrizione.chars().take(max).collect();
        testo(s, h, (x + w / 2.0, alto_box - 17.0), &d, 2.5, "MC")?;
        colore(s, "8")?;
        let c = if nodo.componenti == 1 { "1 componente".to_string() } else { format!("{} componenti", nodo.componenti) };
        testo(s, h, (x + w / 2.0, alto_box - 26.0), &c, 2.0, "MC")?;
    }
    // le tratte sotto i quadri, una fascia per tratta: giù dal quadro, di lato, su all'altro
    let indice = |q: &str| nodi.iter().take(n).position(|x| x.nome.eq_ignore_ascii_case(q));
    // le tratte da sinistra a destra (i, j con i < j), le più corte vicino ai quadri
    let mut ordinate: Vec<(usize, usize, &Tratta)> = tratte.iter().filter_map(|t| Some((indice(&t.da)?, indice(&t.a)?, t))).map(|(a, b, t)| (a.min(b), a.max(b), t)).collect();
    ordinate.sort_by_key(|(a, b, _)| b - a);
    // i punti di attacco sul fondo di ogni quadro: prima le tratte verso sinistra (le più
    // vicine all'esterno), poi quelle verso destra (le più lontane all'esterno), così le
    // verticali delle tratte lunghe non tagliano quelle corte
    let mut posto: Vec<(usize, usize, f64)> = Vec::new();
    for i in 0..n {
        let mut qui: Vec<(usize, (u8, i64))> = ordinate
            .iter()
            .enumerate()
            .filter_map(|(k, (a, b, _))| {
                if *a == i {
                    Some((k, (1, -((b - a) as i64))))
                } else if *b == i {
                    Some((k, (0, (b - a) as i64)))
                } else {
                    None
                }
            })
            .collect();
        qui.sort_by_key(|(_, chiave)| *chiave);
        let tot = qui.len();
        for (r, (k, _)) in qui.into_iter().enumerate() {
            posto.push((k, i, centro(i) - w / 2.0 + w * (r + 1) as f64 / (tot + 1) as f64));
        }
    }
    let x_di = |k: usize, i: usize| posto.iter().find(|(kk, ii, _)| *kk == k && *ii == i).map_or(centro(i), |p| p.2);
    // prima i percorsi: (xa, xb, quota della linea orizzontale, tratta)
    let mut percorsi: Vec<(f64, f64, f64, &Tratta)> = Vec::new();
    let mut y = basso_box - 6.0;
    for (k, (ia, ib, t)) in ordinate.iter().enumerate() {
        let fascia = 6.0 + 4.0 * t.cavi.len() as f64;
        if y - fascia < -alto + 4.0 {
            colore(s, "1")?;
            testo(s, h, (2.0, y - 4.0), "… altri collegamenti nella tabella dei cavi", 2.0, "L")?;
            break;
        }
        y -= fascia;
        percorsi.push((x_di(k, *ia), x_di(k, *ib), y, t));
    }
    for (k, (xa, xb, linea_y, t)) in percorsi.iter().enumerate() {
        colore(s, "4")?;
        linea(s, h, &[(*xa, basso_box), (*xa, *linea_y), (*xb, *linea_y), (*xb, basso_box)], false)?;
        colore(s, "7")?;
        // le etichette sopra la linea, a destra delle verticali dei percorsi più bassi che
        // passano di lì
        let spazio = (xa - xb).abs() - 4.0;
        let lungo = t.cavi.iter().map(|c| larghezza_testo(&testo_cavo(c, spazio))).fold(0.0, f64::max);
        let mut x_testo = xa.min(*xb) + 2.0;
        loop {
            let ostacolo = percorsi.iter().skip(k + 1).flat_map(|(a, b, _, _)| [*a, *b]).filter(|x| *x > x_testo - 1.0 && *x < x_testo + lungo).fold(None, |m: Option<f64>, x| Some(m.map_or(x, |m| m.max(x))));
            match ostacolo {
                Some(x) => x_testo = x + 2.0,
                None => break,
            }
        }
        for (j, c) in t.cavi.iter().enumerate() {
            testo(s, h, (x_testo, linea_y + 1.5 + 4.0 * j as f64), &testo_cavo(c, spazio), 2.0, "L")?;
        }
    }
    if nodi.len() > MAX_NODI {
        colore(s, "1")?;
        testo(s, h, (2.0, -alto + 6.0), &format!("{} quadri non disegnati: vedi la tabella", nodi.len() - MAX_NODI), 2.0, "L")?;
    }
    colore(s, "7")
}

/// Disegna lo schema d'impianto su un foglio dal foglio `primo` (default dopo l'ultimo).
pub fn uscita(s: &mut Session, primo: Option<u32>) -> Result<Vec<u32>> {
    let (nodi, tratte) = dati(s)?;
    if nodi.is_empty() {
        return Err(errore("Nessun quadro nel progetto: assegna il quadro ai componenti"));
    }
    uscite::disegna_con(s, 1, "impianto", &json!({}), primo, "Schema d'impianto", "Plant Diagram", |s, _, h| disegna_pagina(s, &nodi, &tratte, h))
}

/// I collegamenti tra i quadri in tabella (Excel).
pub fn tabella(s: &mut Session, _p: &Value) -> Result<uscite::Tabella> {
    let (_, tratte) = dati(s)?;
    let col = |v: &[(&str, f64)]| v.iter().map(|(t, w)| ((*t).to_string(), *w)).collect::<Vec<_>>();
    Ok(uscite::Tabella {
        titolo: "SCHEMA D'IMPIANTO \\ PLANT DIAGRAM".into(),
        colonne: col(&[("Da quadro\nFrom", 25.0), ("A quadro\nTo", 25.0), ("Cavo\nCable", 22.0), ("Formazione\nCores", 28.0), ("Lunghezza [m]\nLength", 25.0), ("Funzione\nFunction", 90.0), ("Codice\nCode", 50.0)]),
        righe: tratte
            .iter()
            .flat_map(|t| t.cavi.iter().map(move |c| [t.da.clone(), t.a.clone(), c.sigla.clone(), c.formazione.clone(), c.lunghezza.clone(), c.funzione.clone(), c.codice.clone()]))
            .map(|r| r.into_iter().map(uscite::Cella::T).collect())
            .collect(),
        altezza_riga: 6.0,
    })
}
