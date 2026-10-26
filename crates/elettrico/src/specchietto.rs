//! Specchietto dei contatti (riferimento di registrazione) sotto le bobine.
//!
//! Come nei progetti dei CAD elettrici diffusi (vedi `docs/spac-modello-dati.md`): la madre ha in
//! `CODICE` il nome del blocco di riferimento; il blocco (`PRES=R`, `MADRE` = sigla) ha una riga
//! per contatto disponibile, con `PINAk`/`PINBk` (`PINCk` per lo scambio) e `REGk` = dove è
//! disegnato il contatto che usa quella riga. Ogni contatto figlio occupa righe compatibili e ne
//! prende i numeri dei morsetti. I blocchi di riferimento e la loro grafica sono nostri.

use cadcraft_engine::{Result, Session};
use serde_json::json;

use crate::siglatura::{Componente, componenti};
use crate::squadratura;
use crate::util::{allinea_a_destra, blocchi_definiti, blocco, colore, esegui, handles, inserisci_su_layer, linea, su_layer_zero};

type P = (f64, f64);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Contatto {
    /// Polo di potenza (NA).
    Potenza,
    /// Ausiliario normalmente aperto.
    Na,
    /// Ausiliario normalmente chiuso.
    Nc,
    /// Scambio: comune (A), NA (B), NC (C).
    Scambio,
}

pub struct Riferimento {
    pub blocco: &'static str,
    pub descrizione: &'static str,
    /// Per ogni riga: tipo e morsetti; per lo scambio (comune, NA, NC), per gli altri (A, B, "").
    pub righe: &'static [(Contatto, &'static str, &'static str, &'static str)],
}

use Contatto::{Na, Nc, Potenza, Scambio};

pub static RIFERIMENTI: &[Riferimento] = &[
    Riferimento {
        blocco: "CE_RIF_KM",
        descrizione: "Contattore: 3 poli di potenza, 1 NA, 1 NC",
        righe: &[(Potenza, "1", "2", ""), (Potenza, "3", "4", ""), (Potenza, "5", "6", ""), (Na, "13", "14", ""), (Nc, "21", "22", "")],
    },
    Riferimento {
        blocco: "CE_RIF_KM22",
        descrizione: "Contattore: 3 poli di potenza, 2 NA, 2 NC",
        righe: &[(Potenza, "1", "2", ""), (Potenza, "3", "4", ""), (Potenza, "5", "6", ""), (Na, "13", "14", ""), (Nc, "21", "22", ""), (Nc, "31", "32", ""), (Na, "43", "44", "")],
    },
    Riferimento { blocco: "CE_RIF_KA", descrizione: "Relè: 2 contatti di scambio", righe: &[(Scambio, "11", "14", "12"), (Scambio, "21", "24", "22")] },
    Riferimento {
        blocco: "CE_RIF_KA4",
        descrizione: "Relè: 4 contatti di scambio",
        righe: &[(Scambio, "11", "14", "12"), (Scambio, "21", "24", "22"), (Scambio, "31", "34", "32"), (Scambio, "41", "44", "42")],
    },
    Riferimento { blocco: "CE_RIF_KT", descrizione: "Temporizzatore: 2 contatti di scambio ritardati", righe: &[(Scambio, "15", "18", "16"), (Scambio, "25", "28", "26")] },
    Riferimento { blocco: "CE_RIF_FR", descrizione: "Relè termico: 1 NC (95-96), 1 NA (97-98)", righe: &[(Nc, "95", "96", ""), (Na, "97", "98", "")] },
    Riferimento { blocco: "CE_RIF_QM", descrizione: "Salvamotore: contatti ausiliari 1 NA, 1 NC", righe: &[(Na, "13", "14", ""), (Nc, "21", "22", "")] },
    Riferimento { blocco: "CE_RIF_AUX", descrizione: "Interruttore: 1 contatto ausiliario di scambio", righe: &[(Scambio, "11", "14", "12")] },
    // testine aggiuntive: si aggiungono al riferimento della madre per avere più contatti
    Riferimento { blocco: "CE_RIF_T11", descrizione: "Testina aggiuntiva 1 NA + 1 NC", righe: &[(Na, "53", "54", ""), (Nc, "61", "62", "")] },
    Riferimento { blocco: "CE_RIF_T20", descrizione: "Testina aggiuntiva 2 NA", righe: &[(Na, "53", "54", ""), (Na, "63", "64", "")] },
    Riferimento { blocco: "CE_RIF_T02", descrizione: "Testina aggiuntiva 2 NC", righe: &[(Nc, "51", "52", ""), (Nc, "61", "62", "")] },
    Riferimento {
        blocco: "CE_RIF_T22",
        descrizione: "Testina aggiuntiva 2 NA + 2 NC",
        righe: &[(Na, "53", "54", ""), (Nc, "61", "62", ""), (Nc, "71", "72", ""), (Na, "83", "84", "")],
    },
];

/// Le testine aggiuntive (riferimenti da aggiungere a quello della madre).
pub fn testine() -> impl Iterator<Item = &'static Riferimento> {
    RIFERIMENTI.iter().filter(|r| r.blocco.starts_with("CE_RIF_T"))
}

/// Il riferimento proposto per una madre col prefisso dato.
pub fn predefinito(prefisso: &str) -> Option<&'static Riferimento> {
    let nome = match prefisso.to_ascii_uppercase().as_str() {
        "KM" => "CE_RIF_KM",
        "KA" | "K" => "CE_RIF_KA",
        "KT" => "CE_RIF_KT",
        "FR" => "CE_RIF_FR",
        "QM" => "CE_RIF_QM",
        _ => return None,
    };
    trova(nome)
}

pub fn trova(nome: &str) -> Option<&'static Riferimento> {
    RIFERIMENTI.iter().find(|r| r.blocco.eq_ignore_ascii_case(nome))
}

const PASSO_RIGA: f64 = 5.0;
/// Il riferimento sta sotto il simbolo della madre (alto 10 mm), con un piccolo margine, e a
/// sinistra del suo filo (come nei CAD elettrici diffusi): scostamento dal punto d'inserimento.
pub const SCOSTAMENTO: (f64, f64) = (-8.0, -13.0);
/// Quanto sta sotto la madre (compatibilità).
pub const SOTTO_MADRE: f64 = -SCOSTAMENTO.1;

/// Dove va lo specchietto della madre posata in `at`.
pub fn posizione(at: P) -> P {
    (at.0 + SCOSTAMENTO.0, at.1 + SCOSTAMENTO.1)
}

fn altezza(r: &Riferimento) -> f64 {
    PASSO_RIGA * r.righe.len() as f64
}

fn crea_blocco(s: &mut Session, r: &Riferimento) -> Result<()> {
    if blocchi_definiti(s)?.iter().any(|b| b.eq_ignore_ascii_case(r.blocco)) {
        return Ok(());
    }
    su_layer_zero(s, |s| {
        let mut h = Vec::new();
        let mut a_destra = Vec::new();
        for (k, (tipo, ..)) in r.righe.iter().enumerate() {
            let y = -2.5 - PASSO_RIGA * k as f64;
            colore(s, "7")?;
            // contatto piccolo in orizzontale tra x=-3 e x=3
            match tipo {
                Contatto::Potenza | Contatto::Na => {
                    linea(s, &mut h, &[(-3.0, y), (-1.0, y)], false)?;
                    linea(s, &mut h, &[(1.5, y), (3.0, y)], false)?;
                    linea(s, &mut h, &[(-1.0, y), (1.6, y + 1.4)], false)?;
                }
                Contatto::Nc => {
                    linea(s, &mut h, &[(-3.0, y), (-1.0, y)], false)?;
                    linea(s, &mut h, &[(1.5, y), (3.0, y)], false)?;
                    linea(s, &mut h, &[(1.5, y), (1.5, y - 1.3)], false)?;
                    linea(s, &mut h, &[(-1.0, y), (2.0, y - 1.6)], false)?;
                }
                Contatto::Scambio => {
                    linea(s, &mut h, &[(-3.0, y), (-1.0, y)], false)?;
                    linea(s, &mut h, &[(1.5, y + 1.2), (3.0, y + 1.2)], false)?;
                    linea(s, &mut h, &[(1.5, y - 1.2), (3.0, y - 1.2)], false)?;
                    linea(s, &mut h, &[(1.5, y - 1.2), (1.5, y - 2.2)], false)?;
                    linea(s, &mut h, &[(-1.0, y), (2.0, y - 2.0)], false)?;
                }
            }
            let att = |s: &mut Session, h: &mut Vec<String>, tag: String, at: P, alt: f64, nascosto: bool| -> Result<String> {
                let r = esegui(s, "attdef", json!({"tag": tag, "prompt": tag, "default": "", "at": [at.0, at.1], "height": alt, "invisible": nascosto}))?;
                let nuovi = handles(&r);
                h.extend(nuovi.iter().cloned());
                Ok(nuovi.into_iter().next().unwrap_or_default())
            };
            colore(s, "3")?;
            a_destra.push(att(s, &mut h, format!("PINA{k}"), (-3.5, y - 0.9), 1.8, false)?);
            if *tipo == Contatto::Scambio {
                att(s, &mut h, format!("PINB{k}"), (3.5, y + 0.4), 1.8, false)?;
                att(s, &mut h, format!("PINC{k}"), (3.5, y - 2.4), 1.8, false)?;
            } else {
                att(s, &mut h, format!("PINB{k}"), (3.5, y - 0.9), 1.8, false)?;
            }
            colore(s, "4")?;
            a_destra.push(att(s, &mut h, format!("REG{k}"), (-8.0, y - 0.9), 2.0, false)?);
        }
        colore(s, "4")?;
        for (tag, valore) in [("MADRE", String::new()), ("PRES", "R".to_string()), ("OFFSET", format!("@0,-{}", altezza(r)))] {
            let rr = esegui(s, "attdef", json!({"tag": tag, "prompt": tag, "default": valore, "at": [0.0, 0.0], "height": 1.0, "invisible": true}))?;
            h.extend(handles(&rr));
        }
        for t in a_destra {
            allinea_a_destra(s, &t)?;
        }
        blocco(s, r.blocco, h, r.descrizione)
    })
}

/// Distanza tra due riferimenti della stessa madre, uno sotto l'altro.
const STACCO: f64 = 2.0;

/// Dove va il riferimento numero `k` (da 0) di una madre in `at`, sotto i riferimenti `prima`.
fn posizione_k(at: P, prima: &[&Riferimento]) -> P {
    let p = posizione(at);
    (p.0, p.1 - prima.iter().map(|r| altezza(r) + STACCO).sum::<f64>())
}

/// Inserisce il riferimento `r` sotto la madre `sigla` posta in `at` sul layer `layer`.
pub fn inserisci(s: &mut Session, r: &Riferimento, sigla: &str, at: P, layer: &str) -> Result<String> {
    inserisci_sotto(s, r, sigla, at, layer, &[])
}

/// Inserisce il riferimento `r` della madre `sigla` (in `at`) sotto i suoi riferimenti `prima`.
pub fn inserisci_sotto(s: &mut Session, r: &Riferimento, sigla: &str, at: P, layer: &str, prima: &[&Riferimento]) -> Result<String> {
    crea_blocco(s, r)?;
    let mut valori = serde_json::Map::new();
    valori.insert("MADRE".into(), json!(sigla));
    valori.insert("PRES".into(), json!("R"));
    for (k, (_, a, b, c)) in r.righe.iter().enumerate() {
        valori.insert(format!("PINA{k}"), json!(a));
        valori.insert(format!("PINB{k}"), json!(b));
        if !c.is_empty() {
            valori.insert(format!("PINC{k}"), json!(c));
        }
    }
    inserisci_su_layer(s, r.blocco, posizione_k(at, prima), &serde_json::Value::Object(valori), layer)
}

/// Le righe che un figlio occupa: (quante, tipi accettati in ordine di preferenza).
fn bisogno(pres: &str) -> (usize, &'static [Contatto]) {
    let p = pres.to_ascii_uppercase();
    if let Some(n) = p.strip_prefix("FAP").and_then(|n| n.parse::<usize>().ok()) {
        return (n.max(1), &[Contatto::Potenza, Contatto::Na]);
    }
    match p.as_str() {
        "FA" => (1, &[Contatto::Na, Contatto::Scambio]),
        "FC" => (1, &[Contatto::Nc, Contatto::Scambio]),
        "FS" => (1, &[Contatto::Scambio]),
        _ => (0, &[]),
    }
}

/// Morsetti che il figlio prende dalla riga: (A, B, C).
fn morsetti_per(pres: &str, riga: &(Contatto, &str, &str, &str)) -> (String, String, String) {
    let (tipo, a, b, c) = *riga;
    match (tipo, pres.to_ascii_uppercase().as_str()) {
        (Contatto::Scambio, "FC") => (a.into(), c.into(), String::new()),
        // il simbolo dello scambio ha in alto il NA (A) e il NC (C), in basso il comune (B)
        (Contatto::Scambio, "FS") => (b.into(), a.into(), c.into()),
        _ => (a.into(), b.into(), String::new()),
    }
}

/// Una riga degli specchietti di una madre: (specchietto, indice nella sua tabella, riga).
type RigaDi = (usize, usize, &'static (Contatto, &'static str, &'static str, &'static str));

/// Assegna i contatti figli alle righe degli specchietti delle loro madri (il riferimento e le
/// testine aggiuntive, uno sotto l'altro, nell'ordine del `CODICE` della madre), scrive in ogni
/// riga dove è usato il contatto e dà ai figli i numeri dei morsetti; riporta gli specchietti
/// nostri sotto la loro madre. Restituisce i figli che non hanno trovato una riga libera.
pub fn aggiorna(s: &mut Session) -> Result<Vec<String>> {
    let tutti = componenti(s)?;
    let mut figli: Vec<&Componente> = tutti.iter().filter(|c| c.figlio()).collect();
    figli.sort_by_key(|c| (c.foglio, (c.blocco.pos.0 * 1000.0).round() as i64, -(c.blocco.pos.1 * 1000.0).round() as i64));
    let mut senza_posto = Vec::new();
    let mut modifiche: Vec<(String, serde_json::Map<String, serde_json::Value>)> = Vec::new();
    let mut spostamenti: Vec<(String, P, P, String)> = Vec::new();
    // i riferimenti nostri raggruppati per madre
    let mut gruppi: Vec<(String, Vec<(&Componente, &'static Riferimento)>)> = Vec::new();
    for spec in tutti.iter().filter(|c| c.pres.eq_ignore_ascii_case("R")) {
        let Some(r) = trova(&spec.blocco.nome) else { continue };
        let sigla = spec.blocco.attributo("MADRE").to_string();
        match gruppi.iter_mut().find(|(m, _)| m.eq_ignore_ascii_case(&sigla)) {
            Some((_, v)) => v.push((spec, r)),
            None => gruppi.push((sigla, vec![(spec, r)])),
        }
    }
    for (sigla, mut specchi) in gruppi {
        let madre = tutti.iter().find(|m| m.madre() && m.nome.eq_ignore_ascii_case(&sigla));
        // nell'ordine del CODICE della madre, poi dall'alto in basso
        let codici: Vec<String> = madre.map(|m| m.blocco.attributo("CODICE").split(';').map(|c| c.trim().to_ascii_uppercase()).collect()).unwrap_or_default();
        specchi.sort_by_key(|(c, r)| (codici.iter().position(|x| *x == r.blocco).unwrap_or(usize::MAX), -(c.blocco.pos.1 * 1000.0).round() as i64));
        if let Some(m) = madre {
            for (k, (spec, _)) in specchi.iter().enumerate() {
                let prima: Vec<&Riferimento> = specchi.iter().take(k).map(|(_, r)| *r).collect();
                let voluto = posizione_k(m.blocco.pos, &prima);
                if (voluto.0 - spec.blocco.pos.0).abs() > 1e-6 || (voluto.1 - spec.blocco.pos.1).abs() > 1e-6 || spec.blocco.layer != m.blocco.layer {
                    spostamenti.push((spec.blocco.handle.clone(), spec.blocco.pos, voluto, m.blocco.layer.clone()));
                }
            }
        }
        // tutte le righe della madre: (specchietto, indice nella sua tabella, riga)
        let righe: Vec<RigaDi> = specchi.iter().enumerate().flat_map(|(j, (_, r))| r.righe.iter().enumerate().map(move |(k, riga)| (j, k, riga))).collect();
        let mut libere: Vec<bool> = vec![true; righe.len()];
        let mut valori: Vec<serde_json::Map<String, serde_json::Value>> = specchi
            .iter()
            .map(|(_, r)| (0..r.righe.len()).map(|k| (format!("REG{k}"), json!(""))).collect())
            .collect();
        for f in figli.iter().filter(|f| f.nome.eq_ignore_ascii_case(&sigla)) {
            let (quante, tipi) = bisogno(&f.pres);
            if quante == 0 {
                continue;
            }
            // righe libere del tipo preferito, poi degli altri accettati
            let mut scelte: Vec<usize> = Vec::new();
            for t in tipi {
                for (i, (_, _, riga)) in righe.iter().enumerate() {
                    if scelte.len() < quante && libere[i] && riga.0 == *t && !scelte.contains(&i) {
                        scelte.push(i);
                    }
                }
            }
            if scelte.len() < quante {
                senza_posto.push(format!("{} {} (foglio {})", f.pres, f.nome, f.foglio));
                continue;
            }
            let rif = squadratura::riferimento(f.foglio, f.blocco.pos.0, f.blocco.pos.1);
            let mut morsetti = serde_json::Map::new();
            for (n, &i) in scelte.iter().enumerate() {
                libere[i] = false;
                let (j, k, riga) = righe[i];
                if let Some(v) = valori.get_mut(j) {
                    v.insert(format!("REG{k}"), json!(rif));
                }
                let (a, b, c) = morsetti_per(&f.pres, riga);
                morsetti.insert(format!("PINA{}", n + 1), json!(a));
                morsetti.insert(format!("PINB{}", n + 1), json!(b));
                if !c.is_empty() {
                    morsetti.insert(format!("PINC{}", n + 1), json!(c));
                }
            }
            if morsetti.iter().any(|(t, v)| f.blocco.attributo(t) != v.as_str().unwrap_or("")) {
                modifiche.push((f.blocco.handle.clone(), morsetti));
            }
        }
        for ((spec, _), v) in specchi.iter().zip(valori) {
            if v.iter().any(|(t, x)| spec.blocco.attributo(t) != x.as_str().unwrap_or("")) {
                modifiche.push((spec.blocco.handle.clone(), v));
            }
        }
    }
    for (h, da, a, layer) in spostamenti {
        esegui(s, "move", json!({"handles": [h], "from": [da.0, da.1], "to": [a.0, a.1]}))?;
        esegui(s, "laymch", json!({"handles": [h], "layer": layer}))?;
    }
    for (h, valori) in modifiche {
        esegui(s, "attedit", json!({"handle": h, "values": valori}))?;
    }
    Ok(senza_posto)
}

/// Cambia i riferimenti di una madre (il contattore e le testine aggiuntive): toglie quelli che
/// ha, mette i nuovi uno sotto l'altro e scrive i nomi nel suo `CODICE`.
pub fn cambia_riferimenti(s: &mut Session, madre: &Componente, nuovi: &[&'static Riferimento]) -> Result<()> {
    let tutti = componenti(s)?;
    let vecchi: Vec<String> = tutti.iter().filter(|c| c.pres.eq_ignore_ascii_case("R") && c.blocco.attributo("MADRE").eq_ignore_ascii_case(&madre.nome)).map(|c| c.blocco.handle.clone()).collect();
    if !vecchi.is_empty() {
        esegui(s, "erase", json!({"handles": vecchi}))?;
    }
    for (k, r) in nuovi.iter().enumerate() {
        inserisci_sotto(s, r, &madre.nome, madre.blocco.pos, &madre.blocco.layer, &nuovi[..k])?;
    }
    let codice: Vec<&str> = nuovi.iter().map(|r| r.blocco).collect();
    esegui(s, "attedit", json!({"handle": madre.blocco.handle, "values": {"CODICE": codice.join(";")}}))?;
    Ok(())
}
