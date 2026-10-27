//! Estrazione targhette: le etichette da stampare per il cablaggio del quadro, in CSV per la
//! stampante di targhette.
//!
//! - **componenti**: la sigla e la funzione di ogni madre (una targhetta a componente);
//! - **fili**: il numero di ogni filo, una targhetta per ogni capo collegato (morsetto di un
//!   componente o di una morsettiera);
//! - **morsetti**: il numero di ogni morsetto e, in testa a ogni morsettiera, il suo nome;
//! - **cavi**: la sigla di ogni cavo, una targhetta per estremità.
//!
//! Con `quadro` solo quelle del quadro; in ordine di quadro e di testo (i numeri in ordine
//! naturale: `-X1:2` prima di `-X1:10`).

use cadcraft_engine::cmd::has_doc;
use cadcraft_engine::{CommandSpec, Result, Session};
use serde_json::{Value, json};

use crate::collegamenti::{self, TipoCapo};
use crate::util::{errore, testo_param};
use crate::{cavi, morsetti, siglatura};

/// Una targhetta: il testo, una seconda riga, il quadro, quante copie.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Targhetta {
    pub testo: String,
    pub descrizione: String,
    pub quadro: String,
    pub copie: u32,
}

pub const TIPI: [&str; 4] = ["componenti", "fili", "morsetti", "cavi"];
pub const INTESTAZIONE: [&str; 4] = ["Testo", "Descrizione", "Quadro", "Copie"];

/// Chiave d'ordine "naturale": i gruppi di cifre come numeri.
fn naturale(t: &str) -> Vec<(u8, u64, String)> {
    let mut v = Vec::new();
    let mut cifre = String::new();
    let mut lettere = String::new();
    let chiudi = |v: &mut Vec<(u8, u64, String)>, cifre: &mut String, lettere: &mut String| {
        if !lettere.is_empty() {
            v.push((0, 0, std::mem::take(lettere).to_uppercase()));
        }
        if !cifre.is_empty() {
            v.push((1, cifre.parse().unwrap_or(u64::MAX), std::mem::take(cifre)));
        }
    };
    for c in t.chars() {
        if c.is_ascii_digit() {
            if !lettere.is_empty() {
                chiudi(&mut v, &mut cifre, &mut lettere);
            }
            cifre.push(c);
        } else {
            if !cifre.is_empty() {
                chiudi(&mut v, &mut cifre, &mut lettere);
            }
            lettere.push(c);
        }
    }
    chiudi(&mut v, &mut cifre, &mut lettere);
    v
}

fn del_quadro(q: &str, quadro: Option<&str>) -> bool {
    quadro.is_none_or(|x| x.trim().is_empty() || q.trim().eq_ignore_ascii_case(x.trim()))
}

/// Aggiunge una targhetta (o le sue copie a quella con lo stesso testo e quadro), se è del quadro
/// chiesto.
fn aggiungi(v: &mut Vec<Targhetta>, solo: Option<&str>, testo: String, descrizione: String, quadro: String, copie: u32) {
    if testo.trim().is_empty() || !del_quadro(&quadro, solo) {
        return;
    }
    match v.iter_mut().find(|t| t.testo == testo && t.quadro == quadro) {
        Some(t) => t.copie += copie,
        None => v.push(Targhetta { testo, descrizione, quadro, copie }),
    }
}

/// Le targhette del tipo `tipo` (vedi [`TIPI`]).
pub fn targhette(s: &mut Session, tipo: &str, quadro: Option<&str>) -> Result<Vec<Targhetta>> {
    let mut v: Vec<Targhetta> = Vec::new();
    match tipo {
        "componenti" => {
            for c in siglatura::componenti(s)?.into_iter().filter(|c| c.madre() && c.foglio > 0) {
                let b = &c.blocco;
                let d = [b.attributo("FUNZIONE1"), b.attributo("DESCRIZIONE")].into_iter().find(|t| !t.trim().is_empty()).unwrap_or("").to_string();
                // una sola targhetta anche se la madre compare più volte (testine, rimandi)
                if !v.iter().any(|t| t.testo == c.nome) {
                    aggiungi(&mut v, quadro, c.nome.clone(), d, b.attributo("QUADRO").to_string(), 1);
                }
            }
        }
        "fili" => {
            for r in collegamenti::reti(s, &[])? {
                let capi: Vec<&collegamenti::Capo> = r.capi.iter().filter(|c| c.tipo != TipoCapo::Rimando).collect();
                if r.nome.trim().is_empty() || capi.is_empty() {
                    continue;
                }
                // un capo per quadro: le targhette vanno dove si cabla
                for c in capi {
                    aggiungi(&mut v, quadro, r.nome.clone(), r.conduttore.etichetta(), c.quadro.clone(), 1);
                }
            }
        }
        "morsetti" => {
            let tutti = morsetti::morsetti(s)?;
            for m in tutti.iter().filter(|m| m.foglio > 0) {
                let q = m.blocco.attributo("QUADRO").to_string();
                aggiungi(&mut v, quadro, format!("{}:{}", m.morsettiera, m.numero), String::new(), q.clone(), 1);
                let testata = m.morsettiera.clone();
                if !v.iter().any(|t| t.testo == testata && t.quadro == q) {
                    aggiungi(&mut v, quadro, testata, "Morsettiera".into(), q, 1);
                }
            }
        }
        "cavi" => {
            for c in cavi::cavi(s)? {
                let d = [c.formazione.as_str(), c.funzione.as_str()].iter().filter(|t| !t.trim().is_empty()).copied().collect::<Vec<_>>().join(" ");
                // un'estremità per parte: nel quadro di partenza e in quello d'arrivo
                aggiungi(&mut v, quadro, c.sigla.clone(), d.clone(), c.da_quadro.clone(), 1);
                aggiungi(&mut v, quadro, c.sigla.clone(), d, c.a_quadro.clone(), 1);
            }
        }
        _ => return Err(errore(format!("Targhette sconosciute: {tipo} ({})", TIPI.join(", ")))),
    }
    v.sort_by(|a, b| a.quadro.to_uppercase().cmp(&b.quadro.to_uppercase()).then_with(|| naturale(&a.testo).cmp(&naturale(&b.testo))));
    Ok(v)
}

pub fn righe(v: &[Targhetta]) -> Vec<Vec<String>> {
    v.iter().map(|t| vec![t.testo.clone(), t.descrizione.clone(), t.quadro.clone(), t.copie.to_string()]).collect()
}

fn run_targhette(s: &mut Session, p: &Value) -> Result<Value> {
    let tipo = testo_param(p, "tipo").unwrap_or_else(|| "componenti".into());
    let quadro = testo_param(p, "quadro");
    let v = targhette(s, &tipo, quadro.as_deref())?;
    let righe = righe(&v);
    if let Some(f) = testo_param(p, "file") {
        morsetti::scrivi_csv(&f, &INTESTAZIONE, &righe)?;
    }
    let copie: u32 = v.iter().map(|t| t.copie).sum();
    Ok(json!({"intestazione": INTESTAZIONE, "righe": righe, "targhette": copie, "message": format!("Targhette {tipo}: {} testi, {copie} da stampare", v.len())}))
}

pub fn comandi() -> Vec<CommandSpec> {
    vec![CommandSpec {
        id: "targhette",
        label: "Estrazione Targhette",
        menu: &["Progetto", "Estrazione Targhette…"],
        shortcut: None,
        aliases: &[],
        params: "{tipo: componenti|fili|morsetti|cavi, quadro?, file?: .csv} → {righe: [testo, descrizione, quadro, copie], targhette}",
        enabled: has_doc,
        run: run_targhette,
        undoable: false,
        interactive: None,
        transparent: false,
    }]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordine_naturale() {
        let mut v = vec!["-X1:10", "-X1:2", "-X10:1", "-X1:1", "02.10", "02.9"];
        v.sort_by_key(|t| naturale(t));
        assert_eq!(v, ["-X1:1", "-X1:2", "-X1:10", "-X10:1", "02.9", "02.10"]);
    }
}
