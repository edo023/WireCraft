//! Siglatura dei componenti e cross reference figlio → madre.
//!
//! Come nei progetti dei CAD elettrici diffusi:
//! - la **madre** (bobina, interruttore, pulsante…) ha `PRES=M` e la sigla in `NOME`;
//! - un **figlio** (contatto) ha `PRES` che inizia con `F` (`FA`, `FC`, `FS`, `FAP3`…), in
//!   `NOME` la sigla della sua madre e in `REG` dove si trova la madre (`08/4E`);
//! - la sigla segue la siglatura delle impostazioni: incrementale (`-KM1`), foglio-prefisso
//!   (`-08KM1`), prefisso-foglio (`-KM08.1`, il predefinito) con progressivo assoluto o che
//!   riparte a ogni foglio, o posizionale (`-KM08.3` = foglio 8, colonna 3).

use cadcraft_engine::cmd::has_doc;
use cadcraft_engine::{CommandSpec, Result, Session};
use serde_json::{Value, json};

use crate::fogli;
use crate::impostazioni::{self, Impostazioni, Siglatura};
use crate::squadratura;
use crate::util::{Inserito, blocchi_inseriti, esegui, in_gruppo};

pub const PREFISSO_ASSOLUTO: &str = "-";

type P = (f64, f64);

/// La sigla di un componente secondo la siglatura in uso (non posizionale): con i predefiniti
/// `-KM08.1`, senza foglio `-KM1`.
pub fn sigla(prefisso: &str, foglio: u32, n: u32) -> String {
    formatta(&impostazioni::correnti(), prefisso, foglio, n)
}

/// La sigla `n` per `prefisso` sul foglio `foglio` nella siglatura di `imp`.
pub fn formatta(imp: &Impostazioni, prefisso: &str, foglio: u32, n: u32) -> String {
    let pa = &imp.prefisso_assoluto;
    let f = format!("{foglio:0w$}", w = imp.cifre_foglio.max(1));
    if foglio == 0 {
        return format!("{pa}{prefisso}{n}");
    }
    match imp.siglatura {
        Siglatura::Incrementale => format!("{pa}{prefisso}{n}"),
        Siglatura::FoglioPrefissoAssoluta | Siglatura::FoglioPrefissoRelativa => format!("{pa}{f}{prefisso}{n}"),
        Siglatura::PrefissoFoglioAssoluta | Siglatura::PrefissoFoglioRelativa | Siglatura::Posizionale => format!("{pa}{prefisso}{f}{}{n}", imp.separatore_sigla),
    }
}

/// Scompone una sigla in (prefisso, foglio, progressivo): `-KM08.1` → ("KM", 8, 1), `-K7` → ("K", 0, 7).
pub fn scomponi(nome: &str) -> Option<(String, u32, u32)> {
    let s = nome.trim().trim_start_matches(PREFISSO_ASSOLUTO);
    let lettere: String = s.chars().take_while(|c| c.is_ascii_alphabetic()).collect();
    if lettere.is_empty() {
        return None;
    }
    let resto = s.get(lettere.len()..)?;
    match resto.split_once('.') {
        Some((f, n)) => Some((lettere, f.parse().ok()?, n.parse().ok()?)),
        None => Some((lettere, 0, resto.parse().ok()?)),
    }
}

/// Il prefisso (le lettere) di una sigla in qualunque siglatura: `-KM08.1`, `-1KM3`, `KM3.2A` → `KM`.
pub fn prefisso_di(nome: &str) -> Option<String> {
    let s = nome.trim().trim_start_matches(['-', '+', '=']);
    let s = s.trim_start_matches(|c: char| c.is_ascii_digit());
    let p: String = s.chars().take_while(|c| c.is_ascii_alphabetic()).collect();
    (!p.is_empty()).then_some(p)
}

/// Il progressivo finale di una sigla: `-KM08.12` → 12, `-1KA3` → 3, `KA1.1A` → 1.
pub fn numero_finale(nome: &str) -> Option<u32> {
    let s = nome.trim().trim_end_matches(|c: char| c.is_ascii_uppercase());
    let cifre: String = s.chars().rev().take_while(char::is_ascii_digit).collect();
    if cifre.is_empty() || cifre.len() == s.len() {
        return None;
    }
    cifre.chars().rev().collect::<String>().parse().ok()
}

/// Un componente (madre, figlio o riferimento) letto dal disegno.
#[derive(Clone, Debug)]
pub struct Componente {
    pub blocco: Inserito,
    pub pres: String,
    pub nome: String,
    pub foglio: u32,
}

impl Componente {
    /// Madre: `PRES=M`, oppure un blocco con `NOME` e senza `PRES` (schede PLC, componenti
    /// pneumatici: i loro figli vi puntano allo stesso modo).
    pub fn madre(&self) -> bool {
        self.pres.eq_ignore_ascii_case("M") || (self.pres.is_empty() && !self.nome.is_empty())
    }
    pub fn figlio(&self) -> bool {
        self.pres.to_ascii_uppercase().starts_with('F')
    }
}

/// I componenti: blocchi con PRES di madre, figlio o riferimento, o con un NOME senza PRES.
/// I rimandi (PRES P/A) restano fuori.
pub fn componenti(s: &mut Session) -> Result<Vec<Componente>> {
    Ok(blocchi_inseriti(s, "*")?
        .into_iter()
        .filter(|b| {
            let p = b.attributo("PRES").to_ascii_uppercase();
            p == "M" || p == "R" || p.starts_with('F') || (p.is_empty() && !b.attributo("NOME").trim().is_empty())
        })
        .map(|b| Componente {
            pres: b.attributo("PRES").to_string(),
            nome: b.attributo("NOME").to_string(),
            foglio: fogli::numero_da_layer(&b.layer).unwrap_or(0),
            blocco: b,
        })
        .collect())
}

/// Il quadro di un componente (per le sigle uguali in quadri diversi).
fn quadro_di(c: &Componente) -> String {
    c.blocco.attributo("QUADRO").trim().to_ascii_uppercase()
}

/// Il prossimo progressivo libero per `prefisso` sul foglio `foglio` nella siglatura in uso
/// (non posizionale): conta le madri con lo stesso prefisso, dello stesso foglio se il
/// progressivo riparte a ogni foglio, dello stesso quadro se le sigle si ripetono tra quadri.
pub fn prossimo(componenti: &[Componente], prefisso: &str, foglio: u32) -> u32 {
    prossimo_in(&impostazioni::correnti(), componenti, prefisso, foglio, &impostazioni::quadro_corrente().unwrap_or_default())
}

fn prossimo_in(imp: &Impostazioni, componenti: &[Componente], prefisso: &str, foglio: u32, quadro: &str) -> u32 {
    componenti
        .iter()
        .filter(|c| c.madre())
        .filter(|c| !imp.sigle_uguali_quadri || quadro_di(c) == quadro.trim().to_ascii_uppercase())
        .filter_map(|c| progressivo_in(imp, &c.nome, prefisso, foglio))
        .max()
        .unwrap_or(0)
        + 1
}

/// Il progressivo di una sigla scritta nella siglatura di `imp` con `prefisso`; nelle
/// siglature che ripartono a ogni foglio, solo se è del foglio `foglio`.
fn progressivo_in(imp: &Impostazioni, nome: &str, prefisso: &str, foglio: u32) -> Option<u32> {
    let s = nome.trim().strip_prefix(imp.prefisso_assoluto.as_str()).unwrap_or(nome.trim());
    let numero = |t: &str| if !t.is_empty() && t.bytes().all(|b| b.is_ascii_digit()) { t.parse::<u32>().ok() } else { None };
    let prefisso_uguale = |t: &str| t.get(..prefisso.len()).is_some_and(|p| p.eq_ignore_ascii_case(prefisso));
    let (f, n) = match imp.siglatura {
        _ if foglio == 0 => (0, numero(s.get(prefisso.len()..).filter(|_| prefisso_uguale(s))?)?),
        Siglatura::Incrementale => (0, numero(s.get(prefisso.len()..).filter(|_| prefisso_uguale(s))?)?),
        Siglatura::FoglioPrefissoAssoluta | Siglatura::FoglioPrefissoRelativa => {
            let cifre = s.bytes().take_while(u8::is_ascii_digit).count();
            let resto = s.get(cifre..)?;
            (numero(s.get(..cifre)?)?, numero(resto.get(prefisso.len()..).filter(|_| prefisso_uguale(resto))?)?)
        }
        Siglatura::PrefissoFoglioAssoluta | Siglatura::PrefissoFoglioRelativa | Siglatura::Posizionale => {
            let resto = s.get(prefisso.len()..).filter(|_| prefisso_uguale(s))?;
            let (f, n) = if imp.separatore_sigla.is_empty() { resto.split_at(imp.cifre_foglio.min(resto.len())) } else { resto.split_once(imp.separatore_sigla.as_str())? };
            (numero(f)?, numero(n)?)
        }
    };
    (!imp.siglatura.relativa() || f == foglio).then_some(n)
}

/// Lettere che distinguono le madri nella stessa colonna (siglatura posizionale): A…Z, AA…
fn lettera(k: usize) -> String {
    let mut k = k;
    let mut s = String::new();
    loop {
        s.insert(0, char::from(b'A' + u8::try_from(k % 26).unwrap_or(0)));
        if k < 26 {
            return s;
        }
        k = k / 26 - 1;
    }
}

/// La sigla per una nuova madre con `prefisso` sul foglio `foglio`, posata in `at`, secondo la
/// siglatura in uso; mai una sigla già usata.
pub fn nuova_sigla(componenti: &[Componente], prefisso: &str, foglio: u32, at: P) -> String {
    let imp = impostazioni::correnti();
    let quadro = impostazioni::quadro_corrente().unwrap_or_default();
    let usata = |n: &str| componenti.iter().any(|c| c.madre() && c.nome.eq_ignore_ascii_case(n) && (!imp.sigle_uguali_quadri || quadro_di(c) == quadro.to_ascii_uppercase()));
    if imp.siglatura == Siglatura::Posizionale && foglio > 0 {
        let (col, _) = squadratura::colonna_riga(at.0, at.1);
        let base = formatta(&imp, prefisso, foglio, col);
        // la colonna è libera se non c'è né la sigla né una sua variante con lettera
        let con_lettera = |n: &str| n.len() > base.len() && n.get(..base.len()).is_some_and(|b| b.eq_ignore_ascii_case(&base)) && n[base.len()..].bytes().all(|c| c.is_ascii_uppercase());
        if !usata(&base) && !componenti.iter().any(|c| c.madre() && con_lettera(&c.nome)) {
            return base;
        }
        // la sigla senza lettera vale come "A": la nuova prende la prima lettera libera dopo
        let da = usize::from(usata(&base));
        return (da..).map(|k| format!("{base}{}", lettera(k))).find(|n| !usata(n)).unwrap_or(base);
    }
    let mut n = prossimo_in(&imp, componenti, prefisso, foglio, &quadro);
    loop {
        let s = formatta(&imp, prefisso, foglio, n);
        if !usata(&s) {
            return s;
        }
        n += 1;
    }
}

/// Ordine di lettura di un foglio: per colonne da sinistra a destra, dall'alto in basso.
fn ordine_lettura(b: &Inserito) -> (i64, i64) {
    ((b.pos.0 * 1000.0).round() as i64, -(b.pos.1 * 1000.0).round() as i64)
}

/// Le nuove sigle delle madri secondo la siglatura di `imp`, nell'ordine di lettura dei fogli:
/// (handle, vecchia, nuova) per quelle che cambiano.
pub fn risigla(imp: &Impostazioni, tutti: &[Componente]) -> Vec<(String, String, String)> {
    let mut madri: Vec<&Componente> = tutti.iter().filter(|c| c.madre() && prefisso_di(&c.nome).is_some()).collect();
    madri.sort_by_key(|c| (c.foglio, ordine_lettura(&c.blocco)));
    let mut cambi = Vec::new();
    let mut contatori: std::collections::HashMap<(String, u32, String), u32> = Default::default();
    // posizionale: le madri di ogni (prefisso, foglio, colonna, quadro), nell'ordine di lettura
    let mut colonne: std::collections::BTreeMap<(String, u32, u32, String), Vec<&Componente>> = Default::default();
    for m in madri {
        let Some(p) = prefisso_di(&m.nome) else { continue };
        let quadro = if imp.sigle_uguali_quadri { quadro_di(m) } else { String::new() };
        if imp.siglatura == Siglatura::Posizionale && m.foglio > 0 {
            let (col, _) = squadratura::colonna_riga(m.blocco.pos.0, m.blocco.pos.1);
            colonne.entry((p.to_ascii_uppercase(), m.foglio, col, quadro)).or_default().push(m);
            continue;
        }
        let foglio = if imp.siglatura.relativa() { m.foglio } else { 0 };
        let n = contatori.entry((p.to_ascii_uppercase(), foglio, quadro)).or_insert(0);
        *n += 1;
        let nuova = formatta(imp, &p, m.foglio, *n);
        if nuova != m.nome {
            cambi.push((m.blocco.handle.clone(), m.nome.clone(), nuova));
        }
    }
    for ((_, foglio, col, _), gruppo) in colonne {
        for (k, m) in gruppo.iter().enumerate() {
            let p = prefisso_di(&m.nome).unwrap_or_default();
            let base = formatta(imp, &p, foglio, col);
            let nuova = if gruppo.len() > 1 { format!("{base}{}", lettera(k)) } else { base };
            if nuova != m.nome {
                cambi.push((m.blocco.handle.clone(), m.nome.clone(), nuova));
            }
        }
    }
    cambi
}

/// Rinumera le madri secondo la siglatura in uso e porta le nuove sigle sui figli e sui
/// riferimenti. Restituisce le sigle cambiate (vecchia, nuova).
pub fn sigle_componenti(s: &mut Session) -> Result<Vec<(String, String)>> {
    let tutti = componenti(s)?;
    let cambi = risigla(&impostazioni::correnti(), &tutti);
    // le madri e poi i figli/riferimenti che puntavano alle vecchie sigle (una sola passata, così
    // gli scambi di sigla tra due madri non si confondono)
    for (h, _, nuova) in &cambi {
        esegui(s, "attedit", json!({"handle": h, "values": {"NOME": nuova}}))?;
    }
    for c in tutti.iter().filter(|c| !c.madre()) {
        let madre = c.blocco.attributo("MADRE");
        if let Some((_, _, nuova)) = cambi.iter().find(|(_, v, _)| v == &c.nome) {
            esegui(s, "attedit", json!({"handle": c.blocco.handle, "values": {"NOME": nuova}}))?;
        }
        if !madre.is_empty()
            && let Some((_, _, nuova)) = cambi.iter().find(|(_, v, _)| v == madre)
        {
            esegui(s, "attedit", json!({"handle": c.blocco.handle, "values": {"MADRE": nuova}}))?;
        }
    }
    aggiorna_tutto(s)?;
    Ok(cambi.into_iter().map(|(_, v, n)| (v, n)).collect())
}

/// Scrive in REG di ogni figlio dove si trova la sua madre. Restituisce i figli senza madre.
pub fn aggiorna_cross(s: &mut Session) -> Result<Vec<String>> {
    let tutti = componenti(s)?;
    let mut orfani = Vec::new();
    let mut nuovi = Vec::new();
    for f in tutti.iter().filter(|c| c.figlio()) {
        let madre = tutti.iter().find(|m| m.madre() && m.nome.eq_ignore_ascii_case(&f.nome));
        let rif = match madre {
            Some(m) => squadratura::riferimento(m.foglio, m.blocco.pos.0, m.blocco.pos.1),
            None => {
                orfani.push(format!("{} (foglio {})", f.nome, f.foglio));
                String::new()
            }
        };
        if rif != f.blocco.attributo("REG") {
            nuovi.push((f.blocco.handle.clone(), rif));
        }
    }
    for (h, rif) in nuovi {
        esegui(s, "attedit", json!({"handle": h, "values": {"REG": rif}}))?;
    }
    Ok(orfani)
}

/// Cross reference completo dei componenti: REG dei figli e specchietti delle madri.
/// Restituisce (figli senza madre, figli senza una riga libera nello specchietto).
pub fn aggiorna_tutto(s: &mut Session) -> Result<(Vec<String>, Vec<String>)> {
    let orfani = aggiorna_cross(s)?;
    let senza_posto = crate::specchietto::aggiorna(s)?;
    Ok((orfani, senza_posto))
}

fn run_sigle(s: &mut Session, _p: &Value) -> Result<Value> {
    in_gruppo(s, "Sigle componenti", |s| {
        let cambi = sigle_componenti(s)?;
        let msg = if cambi.is_empty() {
            "Sigle già in ordine".to_string()
        } else {
            format!("Sigle cambiate: {}", cambi.iter().map(|(v, n)| format!("{v} → {n}")).collect::<Vec<_>>().join(", "))
        };
        Ok(json!({"cambi": cambi, "message": msg}))
    })
}

fn run_cross(s: &mut Session, _p: &Value) -> Result<Value> {
    in_gruppo(s, "Cross reference", |s| {
        let (orfani, senza_posto) = aggiorna_tutto(s)?;
        let (rimandi, soli) = crate::fili::aggiorna_rimandi(s)?;
        let mut msg = format!("Cross reference aggiornato: {rimandi} rimandi collegati");
        if !orfani.is_empty() {
            msg.push_str(&format!("\nFigli senza madre: {}", orfani.join(", ")));
        }
        if !senza_posto.is_empty() {
            msg.push_str(&format!("\nContatti oltre quelli disponibili: {}", senza_posto.join(", ")));
        }
        if !soli.is_empty() {
            msg.push_str(&format!("\nRimandi senza corrispondenza: {}", soli.join(", ")));
        }
        Ok(json!({"figliSenzaMadre": orfani, "contattiSenzaPosto": senza_posto, "rimandiSenzaCoppia": soli, "message": msg}))
    })
}

pub fn comandi() -> Vec<CommandSpec> {
    vec![
        CommandSpec {
            id: "siglecomponenti",
            label: "SIGLE Componenti",
            menu: &["ELETTRICO", "SIGLE Componenti"],
            shortcut: None,
            aliases: &[],
            params: "{} → {cambi: [[vecchia, nuova]]}",
            enabled: has_doc,
            run: run_sigle,
            undoable: false,
            interactive: None,
            transparent: false,
        },
        CommandSpec {
            id: "crossreference",
            label: "CROSS REFERENCE",
            menu: &["UTIL", "CROSS REFERENCE"],
            shortcut: None,
            aliases: &[],
            params: "{} → {figliSenzaMadre, contattiSenzaPosto, rimandiSenzaCoppia}",
            enabled: has_doc,
            run: run_cross,
            undoable: false,
            interactive: None,
            transparent: false,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sigle() {
        assert_eq!(sigla("KM", 8, 1), "-KM08.1");
        assert_eq!(sigla("K", 0, 7), "-K7");
        assert_eq!(scomponi("-KM08.1"), Some(("KM".into(), 8, 1)));
        assert_eq!(scomponi("-QF15.12"), Some(("QF".into(), 15, 12)));
        assert_eq!(scomponi("-K7"), Some(("K".into(), 0, 7)));
        assert_eq!(scomponi("-Q2.a"), None);
        assert_eq!(scomponi(""), None);
        assert_eq!(prefisso_di("-08KM3").as_deref(), Some("KM"));
        assert_eq!(prefisso_di("KA1.1A").as_deref(), Some("KA"));
        assert_eq!(numero_finale("-KM08.12"), Some(12));
        assert_eq!(numero_finale("KA1.1B"), Some(1));
        assert_eq!(numero_finale("-KM"), None);
        assert_eq!(lettera(0), "A");
        assert_eq!(lettera(27), "AB");
    }

    #[test]
    fn formati_di_siglatura() {
        let imp = |siglatura| Impostazioni { siglatura, ..Impostazioni::default() };
        assert_eq!(formatta(&imp(Siglatura::Incrementale), "KA", 5, 3), "-KA3");
        assert_eq!(formatta(&imp(Siglatura::FoglioPrefissoRelativa), "KA", 5, 3), "-05KA3");
        assert_eq!(formatta(&imp(Siglatura::PrefissoFoglioAssoluta), "KA", 5, 3), "-KA05.3");
        assert_eq!(formatta(&imp(Siglatura::Posizionale), "KA", 5, 3), "-KA05.3");
    }
}
