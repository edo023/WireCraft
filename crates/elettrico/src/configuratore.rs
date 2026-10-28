//! Configuratore macchina: le opzioni di una macchina (`P01` alimentazione 400 V, `P02` 500 V,
//! `P10` PLC di un tipo, `P11` di un altro…) con i gruppi che si escludono a vicenda, le parti
//! dello schema abbinate a ogni opzione, e il progetto generato con le sole opzioni scelte.
//!
//! Un'opzione è un gruppo con nome del disegno (`OPZ_P01`, non selezionabile: un clic prende
//! l'oggetto, non tutta l'opzione), con descrizione e gruppo di esclusione; i suoi membri sono le
//! parti abbinate più un'ancora, un blocco invisibile `CE_OPZIONE` che tiene in vita l'opzione
//! anche senza parti (il disegno toglie i gruppi vuoti). Le parti senza opzione sono comuni e
//! restano sempre; una parte abbinata a più opzioni resta se ne è scelta almeno una.

use cadcraft_engine::cmd::has_doc;
use cadcraft_engine::doc::{Drawing, Group, Handle};
use cadcraft_engine::{CommandSpec, Result, Session};
use serde_json::{Value, json};

use crate::util::{blocchi_definiti, blocchi_inseriti, blocco, errore, esegui, handles, inserisci_su_layer, su_layer_zero, testo_param};

const PREFISSO: &str = "OPZ_";
pub const BLOCCO_ANCORA: &str = "CE_OPZIONE";
/// Separatore tra gruppo di esclusione e descrizione nella descrizione del gruppo.
const SEP: &str = " | ";

/// Un'opzione della macchina.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Opzione {
    pub codice: String,
    pub descrizione: String,
    /// Le opzioni dello stesso gruppo si escludono a vicenda (vuoto: libera).
    pub gruppo: String,
    pub parti: usize,
}

fn codice_valido(c: &str) -> Result<String> {
    let c = c.trim().to_ascii_uppercase();
    if c.is_empty() || c.len() > 20 || !c.chars().all(|x| x.is_ascii_alphanumeric() || x == '_' || x == '-') {
        return Err(errore(format!("Codice d'opzione non valido: \"{c}\" (lettere, cifre, _ e -)")));
    }
    Ok(c)
}

/// Le ancore delle opzioni nel disegno.
fn ancore(d: &Drawing) -> Vec<Handle> {
    d.model.iter().filter(|e| matches!(&e.kind, cadcraft_engine::doc::EntityKind::Insert(i) if i.block.eq_ignore_ascii_case(BLOCCO_ANCORA))).map(|e| e.handle).collect()
}

fn da_gruppo(g: &Group, ancore: &[Handle]) -> Option<Opzione> {
    let codice = g.name.strip_prefix(PREFISSO)?.to_string();
    let (gruppo, descrizione) = g.description.split_once(SEP).unwrap_or(("", g.description.as_str()));
    let parti = g.members.iter().filter(|m| !ancore.contains(m)).count();
    Some(Opzione { codice, descrizione: descrizione.to_string(), gruppo: gruppo.to_string(), parti })
}

pub fn opzioni(d: &Drawing) -> Vec<Opzione> {
    let a = ancore(d);
    let mut v: Vec<Opzione> = d.groups.iter().filter_map(|g| da_gruppo(g, &a)).collect();
    v.sort_by(|a, b| a.codice.cmp(&b.codice));
    v
}

/// Il blocco dell'ancora: solo attributi nascosti, niente da vedere né da stampare.
fn crea_blocco_ancora(s: &mut Session) -> Result<()> {
    if blocchi_definiti(s)?.iter().any(|b| b.eq_ignore_ascii_case(BLOCCO_ANCORA)) {
        return Ok(());
    }
    su_layer_zero(s, |s| {
        let mut h = Vec::new();
        for tag in ["CODICE", "DESCRIZIONE", "GRUPPO"] {
            h.extend(handles(&esegui(s, "attdef", json!({"tag": tag, "prompt": tag, "default": "", "at": [0.0, 0.0], "height": 1.0, "invisible": true}))?));
        }
        blocco(s, BLOCCO_ANCORA, h, "Opzione del configuratore")
    })
}

/// Crea l'opzione o ne cambia descrizione e gruppo di esclusione.
pub fn definisci(s: &mut Session, codice: &str, descrizione: &str, gruppo: &str) -> Result<Opzione> {
    let codice = codice_valido(codice)?;
    let gruppo = gruppo.trim().to_ascii_uppercase();
    let valori = json!({"CODICE": codice, "DESCRIZIONE": descrizione.trim(), "GRUPPO": gruppo});
    let ancora = match blocchi_inseriti(s, BLOCCO_ANCORA)?.into_iter().find(|b| b.attributo("CODICE").eq_ignore_ascii_case(&codice)) {
        Some(b) => {
            esegui(s, "attedit", json!({"handle": b.handle, "values": valori}))?;
            b.handle
        }
        None => {
            crea_blocco_ancora(s)?;
            inserisci_su_layer(s, BLOCCO_ANCORA, (0.0, 0.0), &valori, "0")?
        }
    };
    let ancora = Handle::parse_hex(&ancora).ok_or_else(|| errore("ancora senza handle"))?;
    let d = s.doc_mut()?;
    let nome = format!("{PREFISSO}{codice}");
    let descrizione_gruppo = format!("{gruppo}{SEP}{}", descrizione.trim());
    match d.groups.iter_mut().find(|g| g.name.eq_ignore_ascii_case(&nome)) {
        Some(g) => {
            g.description = descrizione_gruppo;
            if !g.members.contains(&ancora) {
                g.members.insert(0, ancora);
            }
        }
        None => d.groups.push(Group { name: nome.clone(), description: descrizione_gruppo, selectable: false, members: vec![ancora] }),
    }
    opzioni(d).into_iter().find(|o| o.codice == codice).ok_or_else(|| errore("opzione sparita"))
}

fn gruppo_di<'a>(d: &'a mut Drawing, codice: &str) -> Result<&'a mut Group> {
    let nome = format!("{PREFISSO}{}", codice.trim().to_ascii_uppercase());
    d.groups.iter_mut().find(|g| g.name.eq_ignore_ascii_case(&nome)).ok_or_else(|| errore(format!("Opzione {} non definita", codice.trim())))
}

/// Abbina (o, con `togli`, stacca) le parti `handles` all'opzione. Restituisce le parti
/// dell'opzione dopo il cambio.
pub fn abbina(s: &mut Session, codice: &str, handles: &[String], togli: bool) -> Result<usize> {
    let d = s.doc_mut()?;
    let a = ancore(d);
    let h: Vec<Handle> = handles.iter().filter_map(|x| Handle::parse_hex(x)).filter(|h| d.entity(*h).is_some() && !a.contains(h)).collect();
    if h.is_empty() && !togli {
        return Err(errore("Selezionare le parti dello schema da abbinare all'opzione"));
    }
    let g = gruppo_di(d, codice)?;
    if togli {
        g.members.retain(|m| !h.contains(m));
    } else {
        for x in h {
            if !g.members.contains(&x) {
                g.members.push(x);
            }
        }
    }
    Ok(g.members.iter().filter(|m| !a.contains(m)).count())
}

/// Elimina l'opzione: le sue parti restano, comuni; se ne va anche l'ancora.
pub fn elimina(s: &mut Session, codice: &str) -> Result<()> {
    let nome = format!("{PREFISSO}{}", codice.trim().to_ascii_uppercase());
    let d = s.doc_mut()?;
    let prima = d.groups.len();
    d.groups.retain(|g| !g.name.eq_ignore_ascii_case(&nome));
    if d.groups.len() == prima {
        return Err(errore(format!("Opzione {} non definita", codice.trim())));
    }
    let via: Vec<String> = blocchi_inseriti(s, BLOCCO_ANCORA)?.into_iter().filter(|b| b.attributo("CODICE").eq_ignore_ascii_case(codice.trim())).map(|b| b.handle).collect();
    if !via.is_empty() {
        esegui(s, "erase", json!({"handles": via}))?;
    }
    Ok(())
}

/// Le scelte valide: opzioni esistenti, al massimo una per gruppo di esclusione.
pub fn controlla(tutte: &[Opzione], scelte: &[String]) -> Result<()> {
    for c in scelte {
        if !tutte.iter().any(|o| o.codice.eq_ignore_ascii_case(c)) {
            return Err(errore(format!("Opzione {c} non definita")));
        }
    }
    for o in tutte.iter().filter(|o| !o.gruppo.is_empty()) {
        let stesse: Vec<&str> = tutte.iter().filter(|x| x.gruppo == o.gruppo && scelte.iter().any(|c| c.eq_ignore_ascii_case(&x.codice))).map(|x| x.codice.as_str()).collect();
        if stesse.len() > 1 {
            return Err(errore(format!("Le opzioni {} si escludono (gruppo {})", stesse.join(" e "), o.gruppo)));
        }
    }
    Ok(())
}

/// Il disegno con le sole opzioni scelte: via le parti abbinate solo a opzioni non scelte (e le
/// opzioni stesse, che nel progetto generato non servono più). Restituisce anche le parti tolte.
pub fn configurato(d: &Drawing, scelte: &[String]) -> Result<(Drawing, usize)> {
    let tutte = opzioni(d);
    controlla(&tutte, scelte)?;
    let scelta = |g: &Group| scelte.iter().any(|c| g.name.eq_ignore_ascii_case(&format!("{PREFISSO}{c}")));
    let opz: Vec<&Group> = d.groups.iter().filter(|g| g.name.to_ascii_uppercase().starts_with(PREFISSO)).collect();
    let a = ancore(d);
    let via: Vec<Handle> = d
        .model
        .handles()
        .into_iter()
        .filter(|h| {
            let di: Vec<&&Group> = opz.iter().filter(|g| g.members.contains(h)).collect();
            a.contains(h) || (!di.is_empty() && !di.iter().any(|g| scelta(g)))
        })
        .collect();
    let mut c = d.clone();
    for h in &via {
        c.model.remove(*h);
    }
    c.groups.retain(|g| !g.name.to_ascii_uppercase().starts_with(PREFISSO));
    for g in &mut c.groups {
        g.members.retain(|m| !via.contains(m));
    }
    // le ancore non sono parti dello schema
    Ok((c, via.len() - a.len()))
}

fn json_opzione(o: &Opzione) -> Value {
    json!({"codice": o.codice, "descrizione": o.descrizione, "gruppo": o.gruppo, "parti": o.parti})
}

fn handles_param(s: &Session, p: &Value) -> Vec<String> {
    match p.get("handles").and_then(Value::as_array) {
        Some(a) => a.iter().filter_map(Value::as_str).map(str::to_string).collect(),
        None => s.selection().iter().map(|h| h.hex()).collect(),
    }
}

fn codice_param(p: &Value) -> Result<String> {
    testo_param(p, "codice").filter(|c| !c.trim().is_empty()).ok_or_else(|| errore("Indicare il codice dell'opzione (es. P01)"))
}

fn run_opzioni(s: &mut Session, p: &Value) -> Result<Value> {
    if let Some(a) = p.get("definisci").and_then(Value::as_array) {
        for x in a {
            let c = x.get("codice").and_then(Value::as_str).unwrap_or("");
            definisci(s, c, x.get("descrizione").and_then(Value::as_str).unwrap_or(""), x.get("gruppo").and_then(Value::as_str).unwrap_or(""))?;
        }
    }
    let v = opzioni(s.doc()?);
    Ok(json!({"opzioni": v.iter().map(json_opzione).collect::<Vec<_>>(), "message": format!("{} opzioni", v.len())}))
}

fn run_abbina(s: &mut Session, p: &Value) -> Result<Value> {
    let codice = codice_param(p)?;
    let h = handles_param(s, p);
    let togli = p.get("togli").and_then(Value::as_bool).unwrap_or(false);
    let n = abbina(s, &codice, &h, togli)?;
    Ok(json!({"parti": n, "message": format!("Opzione {}: {n} parti", codice.trim().to_ascii_uppercase())}))
}

fn run_elimina(s: &mut Session, p: &Value) -> Result<Value> {
    let codice = codice_param(p)?;
    elimina(s, &codice)?;
    Ok(json!({"message": format!("Opzione {} eliminata (le parti restano, comuni)", codice.trim().to_ascii_uppercase())}))
}

fn run_genera(s: &mut Session, p: &Value) -> Result<Value> {
    let scelte: Vec<String> = match p.get("scelte") {
        Some(Value::Array(a)) => a.iter().filter_map(Value::as_str).map(|x| x.trim().to_ascii_uppercase()).collect(),
        Some(Value::String(t)) => t.split([',', ' ', ';']).filter(|x| !x.is_empty()).map(str::to_ascii_uppercase).collect(),
        _ => Vec::new(),
    };
    let file = testo_param(p, "file").filter(|f| !f.trim().is_empty()).ok_or_else(|| errore("Indicare il file del progetto da generare"))?;
    let (c, tolte) = configurato(s.doc()?, &scelte)?;
    let io = cadcraft_engine::cmd::file::io().ok_or_else(|| errore("Il salvataggio dei file non è disponibile"))?;
    let nome = std::path::Path::new(&file).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "progetto.dxf".into());
    let byte = (io.write)(&c, &nome).map_err(errore)?;
    std::fs::write(&file, byte).map_err(|e| errore(format!("{file}: {e}")))?;
    let elenco = if scelte.is_empty() { "nessuna opzione".to_string() } else { scelte.join(", ") };
    Ok(json!({"file": file, "tolte": tolte, "message": format!("Progetto con {elenco}: {tolte} parti tolte, salvato in {file}")}))
}

pub fn comandi() -> Vec<CommandSpec> {
    let spec = |id, label, menu, params, run| CommandSpec { id, label, menu, shortcut: None, aliases: &[], params, enabled: has_doc, run, undoable: false, interactive: None, transparent: false };
    vec![
        spec(
            "configuratore",
            "Configuratore",
            &["Progetto", "Configuratore", "Opzioni"],
            "{definisci?: [{codice, descrizione, gruppo?}]} → {opzioni: [{codice, descrizione, gruppo, parti}]}",
            run_opzioni as cadcraft_engine::cmd::Run,
        ),
        spec("configuratore.abbina", "Abbina all'Opzione", &["Progetto", "Configuratore", "Abbina"], "{codice, handles? (default la selezione), togli?: bool} → {parti}", run_abbina),
        spec("configuratore.elimina", "Elimina Opzione", &[], "{codice}", run_elimina),
        spec("configuratore.genera", "Genera Progetto", &["Progetto", "Configuratore", "Genera…"], "{scelte: [codici] | \"P01, P10\", file: .dxf} — il progetto con le sole opzioni scelte → {tolte}", run_genera),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn o(c: &str, g: &str) -> Opzione {
        Opzione { codice: c.into(), descrizione: String::new(), gruppo: g.into(), parti: 0 }
    }

    #[test]
    fn esclusioni() {
        let v = [o("P01", "ALIM"), o("P02", "ALIM"), o("P10", "PLC"), o("P20", "")];
        assert!(controlla(&v, &["P01".into(), "P10".into(), "P20".into()]).is_ok());
        assert!(controlla(&v, &["P01".into(), "P02".into()]).is_err());
        assert!(controlla(&v, &["P99".into()]).is_err());
        assert!(controlla(&v, &[]).is_ok());
        assert!(codice_valido("p01").is_ok_and(|c| c == "P01"));
        assert!(codice_valido("P 1").is_err());
    }
}
