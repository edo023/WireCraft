//! Materiali dei componenti e distinta materiali.
//!
//! Come nei CAD elettrici diffusi: i materiali di una madre stanno nei suoi attributi
//! `COSTRUTTORE`, `TIPO` (codice) e `DESCRIZIONE`, più materiali separati da `;` nello stesso ordine.
//! La distinta si ordina per sigla (una riga per materiale di ogni componente) o per tipo
//! (materiali uguali raggruppati, con quantità e sigle).

use cadcraft_engine::cmd::has_doc;
use cadcraft_engine::{CommandSpec, Result, Session};
use serde_json::{Value, json};

use crate::morsetti::{scrivi_csv, tabella};
use crate::siglatura::componenti;
use crate::util::{errore, esegui, in_gruppo, punto_param, testo_param};

#[derive(Clone, Debug, PartialEq)]
pub struct Materiale {
    pub sigla: String,
    pub quadro: String,
    pub costruttore: String,
    pub codice: String,
    pub descrizione: String,
    /// Foglio del componente (0 = fuori dai fogli).
    pub foglio: u32,
}

fn dividi(t: &str) -> Vec<String> {
    if t.trim().is_empty() { Vec::new() } else { t.split(';').map(|x| x.trim().to_string()).collect() }
}

/// I materiali di tutte le madri, in ordine di sigla.
pub fn materiali(s: &mut Session) -> Result<Vec<Materiale>> {
    let mut v = Vec::new();
    for c in componenti(s)?.iter().filter(|c| c.madre()) {
        let (cost, tipi, descr) = (dividi(c.blocco.attributo("COSTRUTTORE")), dividi(c.blocco.attributo("TIPO")), dividi(c.blocco.attributo("DESCRIZIONE")));
        for (k, codice) in tipi.iter().enumerate().filter(|(_, t)| !t.is_empty()) {
            v.push(Materiale {
                sigla: c.nome.clone(),
                quadro: c.blocco.attributo("QUADRO").to_string(),
                costruttore: cost.get(k).cloned().unwrap_or_default(),
                codice: codice.clone(),
                descrizione: descr.get(k).cloned().unwrap_or_default(),
                foglio: c.foglio,
            });
        }
    }
    v.sort_by(|a, b| a.sigla.cmp(&b.sigla).then_with(|| a.codice.cmp(&b.codice)));
    Ok(v)
}

/// Righe della distinta: per sigla o raggruppate per tipo (costruttore + codice).
pub fn righe(materiali: &[Materiale], per_tipo: bool) -> (Vec<&'static str>, Vec<Vec<String>>) {
    if !per_tipo {
        let r = materiali.iter().map(|m| vec![m.sigla.clone(), m.codice.clone(), m.descrizione.clone(), m.costruttore.clone(), m.quadro.clone(), "1".into()]).collect();
        return (vec!["Sigla", "Codice", "Descrizione", "Costruttore", "Quadro", "Q.tà"], r);
    }
    let mut gruppi: Vec<(String, String, String, Vec<String>)> = Vec::new();
    for m in materiali {
        match gruppi.iter_mut().find(|g| g.0.eq_ignore_ascii_case(&m.costruttore) && g.1.eq_ignore_ascii_case(&m.codice)) {
            Some(g) => g.3.push(m.sigla.clone()),
            None => gruppi.push((m.costruttore.clone(), m.codice.clone(), m.descrizione.clone(), vec![m.sigla.clone()])),
        }
    }
    gruppi.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    let r = gruppi.into_iter().map(|(c, k, d, sigle)| vec![c, k, d, sigle.len().to_string(), sigle.join(", ")]).collect();
    (vec!["Costruttore", "Codice", "Descrizione", "Q.tà", "Sigle"], r)
}

fn run_distinta(s: &mut Session, p: &Value) -> Result<Value> {
    let per_tipo = testo_param(p, "ordine").is_some_and(|o| o.eq_ignore_ascii_case("tipo"));
    in_gruppo(s, "Distinta materiali", |s| {
        let m = materiali(s)?;
        let (intest, righe) = righe(&m, per_tipo);
        if let Some(f) = testo_param(p, "file") {
            scrivi_csv(&f, &intest, &righe)?;
        }
        if let Some(at) = punto_param(p, "at") {
            if righe.is_empty() {
                return Err(errore("Nessun materiale associato ai componenti"));
            }
            tabella(s, at, "Distinta materiali", &intest, &righe, &[30.0])?;
        }
        Ok(json!({"intestazione": intest, "righe": righe, "message": format!("Distinta materiali: {} righe", righe.len())}))
    })
}

/// Associa (o aggiunge) un materiale al componente con la sigla data.
fn run_materiale(s: &mut Session, p: &Value) -> Result<Value> {
    let sigla = testo_param(p, "sigla").ok_or_else(|| errore("Indicare la sigla del componente"))?;
    let codice = testo_param(p, "codice").filter(|c| !c.trim().is_empty()).ok_or_else(|| errore("Indicare il codice del materiale"))?;
    let costruttore = testo_param(p, "costruttore").unwrap_or_default();
    let descrizione = testo_param(p, "descrizione").unwrap_or_default();
    let aggiungi = p.get("aggiungi").and_then(Value::as_bool).unwrap_or(false);
    in_gruppo(s, "Materiale", |s| {
        let tutti = componenti(s)?;
        let c = tutti.iter().find(|c| c.madre() && c.nome.eq_ignore_ascii_case(&sigla)).ok_or_else(|| errore(format!("Componente {sigla} non trovato")))?;
        let unisci = |tag: &str, nuovo: &str| {
            let vecchio = c.blocco.attributo(tag);
            if aggiungi && !c.blocco.attributo("TIPO").is_empty() { format!("{vecchio};{nuovo}") } else { nuovo.to_string() }
        };
        let valori = json!({"COSTRUTTORE": unisci("COSTRUTTORE", &costruttore), "TIPO": unisci("TIPO", &codice), "DESCRIZIONE": unisci("DESCRIZIONE", &descrizione)});
        esegui(s, "attedit", json!({"handle": c.blocco.handle, "values": valori}))?;
        Ok(json!({"sigla": sigla, "message": format!("{sigla}: {costruttore} {codice}")}))
    })
}

pub fn comandi() -> Vec<CommandSpec> {
    vec![
        CommandSpec {
            id: "distintamateriali",
            label: "Distinta Materiali",
            menu: &["Progetto", "Distinta Materiali"],
            shortcut: None,
            aliases: &[],
            params: "{ordine?: sigla|tipo, at?: [x,y] (tabella nel foglio corrente), file?: percorso .csv} → {intestazione, righe}",
            enabled: has_doc,
            run: run_distinta,
            undoable: false,
            interactive: None,
            transparent: false,
        },
        CommandSpec {
            id: "materiale",
            label: "Associa materiale",
            menu: &["Progetto", "Associa materiale"],
            shortcut: None,
            aliases: &[],
            params: "{sigla, codice, costruttore?, descrizione?, aggiungi?: bool (aggiunge ai materiali già associati)}",
            enabled: has_doc,
            run: run_materiale,
            undoable: false,
            interactive: None,
            transparent: false,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(sigla: &str, codice: &str) -> Materiale {
        Materiale { sigla: sigla.into(), quadro: "+QG".into(), costruttore: "Eaton".into(), codice: codice.into(), descrizione: String::new(), foglio: 1 }
    }

    #[test]
    fn distinta_per_tipo_raggruppa() {
        let v = vec![m("-KM08.1", "199220"), m("-KM08.2", "199220"), m("-KM08.1", "281200")];
        let (_, r) = righe(&v, true);
        assert_eq!(r[0], vec!["Eaton", "199220", "", "2", "-KM08.1, -KM08.2"]);
        assert_eq!(r.len(), 2);
    }
}
