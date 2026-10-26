//! Revisioni dei fogli: come nei CAD elettrici diffusi, creare una revisione copia il foglio in
//! un layer di archivio congelato (`5_REV_01`) e scrive numero, modifica, data e firma nel
//! simbolo delle revisioni del foglio (tre righe nel cartiglio, la più recente in alto). Si
//! può vedere un foglio com'era, annullare l'ultima revisione (il foglio torna com'era) o
//! togliere tutte le revisioni.

use cadcraft_engine::cmd::has_doc;
use cadcraft_engine::{CommandSpec, Result, Session};
use serde_json::{Value, json};

use crate::fogli;
use crate::util::{Inserito, archivio_revisione, attributo, blocchi_definiti, blocchi_inseriti, blocco, colore, errore, esegui, handles, in_gruppo, inserisci_su_layer, numero_param, su_layer_zero, testo_param};

pub const BLOCCO: &str = "CE_REVIS";
/// Righe di revisione nel cartiglio.
pub const RIGHE: usize = 3;

/// Il layer d'archivio della revisione `numero` del foglio `n`.
pub fn layer_archivio(n: u32, numero: &str) -> String {
    format!("{n}_REV_{numero}")
}

/// (foglio, numero) da un layer d'archivio.
pub fn da_layer_archivio(l: &str) -> Option<(u32, String)> {
    let (f, r) = l.split_once("_REV_")?;
    Some((f.parse().ok()?, r.to_string()))
}

fn crea_blocco(s: &mut Session) -> Result<()> {
    if blocchi_definiti(s)?.iter().any(|b| b.eq_ignore_ascii_case(BLOCCO)) {
        return Ok(());
    }
    su_layer_zero(s, |s| {
        let mut h = Vec::new();
        colore(s, "4")?;
        // le righe del cartiglio: REV 15–25, MODIFICA 25–55, DATA 55–70, FIRMA 70–85; dal basso
        for k in 0..RIGHE {
            let y = 15.0 + 7.5 * (RIGHE - 1 - k) as f64 + 2.5;
            let n = k + 1;
            attributo(s, &mut h, &format!("REV_NUM_{n}"), "Revisione", (16.0, y), 2.0)?;
            attributo(s, &mut h, &format!("REV_MOD_{n}"), "Modifica", (26.0, y), 1.8)?;
            attributo(s, &mut h, &format!("REV_DAT_{n}"), "Data", (56.0, y), 1.8)?;
            attributo(s, &mut h, &format!("REV_FIR_{n}"), "Firma", (71.0, y), 1.8)?;
        }
        blocco(s, BLOCCO, h, "Revisioni del foglio")
    })
}

/// Una revisione scritta su un foglio.
#[derive(Clone, Debug, PartialEq)]
pub struct Revisione {
    pub foglio: u32,
    pub numero: String,
    pub modifica: String,
    pub data: String,
    pub firma: String,
    /// Il foglio com'era prima della revisione è archiviato.
    pub archiviata: bool,
}

fn simboli(s: &mut Session) -> Result<Vec<(u32, Inserito)>> {
    Ok(blocchi_inseriti(s, BLOCCO)?.into_iter().filter_map(|b| fogli::numero_da_layer(&b.layer).map(|n| (n, b))).collect())
}

fn layer_archivi(s: &Session) -> Result<Vec<String>> {
    Ok(s.doc()?.layers.iter().map(|l| l.name.clone()).filter(|l| archivio_revisione(l)).collect())
}

/// Le revisioni di tutti i fogli, per foglio e dalla più vecchia.
pub fn elenco(s: &mut Session) -> Result<Vec<Revisione>> {
    let archivi = layer_archivi(s)?;
    let mut v = Vec::new();
    for (n, b) in simboli(s)? {
        for k in (1..=RIGHE).rev() {
            let numero = b.attributo(&format!("REV_NUM_{k}")).trim().to_string();
            if numero.is_empty() {
                continue;
            }
            v.push(Revisione {
                foglio: n,
                archiviata: archivi.iter().any(|l| l.eq_ignore_ascii_case(&layer_archivio(n, &numero))),
                numero,
                modifica: b.attributo(&format!("REV_MOD_{k}")).into(),
                data: b.attributo(&format!("REV_DAT_{k}")).into(),
                firma: b.attributo(&format!("REV_FIR_{k}")).into(),
            });
        }
    }
    v.sort_by(|a, b| a.foglio.cmp(&b.foglio).then_with(|| a.numero.cmp(&b.numero)));
    Ok(v)
}

/// Il prossimo numero di revisione: dopo il più alto in uso (`00`, `01`…).
pub fn prossimo_numero(rev: &[Revisione]) -> String {
    let n = rev.iter().filter_map(|r| r.numero.parse::<u32>().ok()).max().map_or(0, |m| m + 1);
    format!("{n:02}")
}

/// Crea la revisione `numero` sui fogli dati: archivia ogni foglio com'è e scrive la riga nel
/// suo simbolo delle revisioni (la più recente in alto; oltre tre, la più vecchia esce).
pub fn crea(s: &mut Session, fogli_da: &[u32], numero: &str, modifica: &str, data: &str, firma: &str) -> Result<usize> {
    crea_blocco(s)?;
    let esistenti = simboli(s)?;
    for &n in fogli_da {
        let layer = fogli::nome_layer(n);
        if !fogli::elenco(s)?.iter().any(|f| f.numero == n) {
            return Err(errore(format!("Il foglio {n} non esiste")));
        }
        let archivio = layer_archivio(n, numero);
        if layer_archivi(s)?.iter().any(|l| l.eq_ignore_ascii_case(&archivio)) {
            return Err(errore(format!("Il foglio {n} ha già la revisione {numero}")));
        }
        // copia del foglio (senza i suoi numeri, titolo e revisioni) nell'archivio congelato
        let contenuto = crate::multifoglio::contenuto(s, &layer)?;
        esegui(s, "layer.new", json!({"name": archivio, "color": 9}))?;
        esegui(s, "layer.set", json!({"name": archivio, "frozen": true, "plot": false}))?;
        if !contenuto.is_empty() {
            let r = esegui(s, "copy", json!({"handles": contenuto, "delta": [0.0, 0.0]}))?;
            let copie = handles(&r);
            if !copie.is_empty() {
                esegui(s, "laymch", json!({"handles": copie, "layer": archivio}))?;
            }
        }
        // la riga nel simbolo del foglio: le revisioni scendono di una riga
        let mut righe: Vec<[String; 4]> = Vec::new();
        let b = esistenti.iter().find(|(f, _)| *f == n).map(|(_, b)| b.clone());
        if let Some(b) = &b {
            for k in 1..=RIGHE {
                let r = [format!("REV_NUM_{k}"), format!("REV_MOD_{k}"), format!("REV_DAT_{k}"), format!("REV_FIR_{k}")].map(|t| b.attributo(&t).to_string());
                if !r[0].trim().is_empty() {
                    righe.push(r);
                }
            }
        }
        righe.insert(0, [numero.to_string(), modifica.to_string(), data.to_string(), firma.to_string()]);
        righe.truncate(RIGHE);
        let mut valori = serde_json::Map::new();
        for k in 0..RIGHE {
            let r = righe.get(k).cloned().unwrap_or_default();
            for (j, t) in ["NUM", "MOD", "DAT", "FIR"].iter().enumerate() {
                valori.insert(format!("REV_{t}_{}", k + 1), json!(r[j]));
            }
        }
        match b {
            Some(b) => {
                esegui(s, "attedit", json!({"handle": b.handle, "values": valori}))?;
            }
            None => {
                inserisci_su_layer(s, BLOCCO, (0.0, 0.0), &Value::Object(valori), &layer)?;
            }
        }
    }
    Ok(fogli_da.len())
}

/// Mostra il foglio `n` com'era nella revisione `numero` (o com'è, senza numero).
pub fn vedi(s: &mut Session, n: u32, numero: Option<&str>) -> Result<()> {
    // il foglio torna corrente, poi si congelano tutti gli archivi
    fogli::vai(s, n)?;
    for l in layer_archivi(s)? {
        esegui(s, "layer.set", json!({"name": l, "frozen": true}))?;
    }
    if let Some(numero) = numero {
        let archivio = layer_archivio(n, numero);
        if !layer_archivi(s)?.iter().any(|l| l.eq_ignore_ascii_case(&archivio)) {
            return Err(errore(format!("Il foglio {n} non ha la revisione {numero}")));
        }
        esegui(s, "layer.set", json!({"name": archivio, "frozen": false}))?;
        esegui(s, "layer.current", json!({"name": archivio}))?;
        esegui(s, "layer.set", json!({"name": fogli::nome_layer(n), "frozen": true}))?;
    }
    Ok(())
}

/// Annulla la revisione `numero` sui fogli che l'hanno: il foglio torna com'era archiviato e la
/// riga esce dal simbolo. Restituisce i fogli ripristinati.
pub fn annulla(s: &mut Session, numero: &str) -> Result<Vec<u32>> {
    let mut fatti = Vec::new();
    for l in layer_archivi(s)? {
        let Some((n, r)) = da_layer_archivio(&l) else { continue };
        if !r.eq_ignore_ascii_case(numero) {
            continue;
        }
        let layer = fogli::nome_layer(n);
        // via il contenuto attuale (tranne numeri, titolo e simbolo revisioni), dentro quello archiviato
        let attuale: Vec<String> = {
            use cadcraft_engine::doc::EntityKind;
            let d = s.doc()?;
            d.model
                .iter()
                .filter(|e| e.common.layer == layer)
                .filter(|e| !matches!(&e.kind, EntityKind::Insert(i) if { let b = i.block.to_ascii_uppercase(); b.starts_with("NUMFG") || b.starts_with("TITOLO") || b == BLOCCO }))
                .map(|e| e.handle.hex())
                .collect()
        };
        if !attuale.is_empty() {
            esegui(s, "erase", json!({"handles": attuale}))?;
        }
        let archiviati: Vec<String> = {
            use cadcraft_engine::doc::EntityKind;
            let d = s.doc()?;
            d.model
                .iter()
                .filter(|e| e.common.layer.eq_ignore_ascii_case(&l))
                .filter(|e| !matches!(&e.kind, EntityKind::Insert(i) if { let b = i.block.to_ascii_uppercase(); b.starts_with("NUMFG") || b.starts_with("TITOLO") || b == BLOCCO }))
                .map(|e| e.handle.hex())
                .collect()
        };
        if !archiviati.is_empty() {
            esegui(s, "laymch", json!({"handles": archiviati, "layer": layer}))?;
        }
        let resto = crate::multifoglio::contenuto(s, &l)?;
        if !resto.is_empty() {
            esegui(s, "erase", json!({"handles": resto}))?;
        }
        if crate::util::layer_corrente(s)? == l {
            esegui(s, "layer.current", json!({"name": "0"}))?;
        }
        esegui(s, "layer.delete", json!({"name": l}))?;
        // la riga della revisione esce dal simbolo: le altre salgono
        if let Some((_, b)) = simboli(s)?.into_iter().find(|(f, _)| *f == n) {
            let mut righe: Vec<[String; 4]> = (1..=RIGHE)
                .map(|k| [format!("REV_NUM_{k}"), format!("REV_MOD_{k}"), format!("REV_DAT_{k}"), format!("REV_FIR_{k}")].map(|t| b.attributo(&t).to_string()))
                .filter(|r| !r[0].trim().is_empty() && !r[0].eq_ignore_ascii_case(numero))
                .collect();
            righe.resize(RIGHE, Default::default());
            let mut valori = serde_json::Map::new();
            for (k, r) in righe.iter().enumerate() {
                for (j, t) in ["NUM", "MOD", "DAT", "FIR"].iter().enumerate() {
                    valori.insert(format!("REV_{t}_{}", k + 1), json!(r[j]));
                }
            }
            esegui(s, "attedit", json!({"handle": b.handle, "values": valori}))?;
        }
        fatti.push(n);
    }
    if fatti.is_empty() {
        return Err(errore(format!("Nessun foglio ha la revisione {numero} archiviata")));
    }
    fatti.sort_unstable();
    crate::siglatura::aggiorna_tutto(s)?;
    crate::fili::aggiorna_rimandi(s)?;
    Ok(fatti)
}

/// Toglie tutte le revisioni: archivi e simboli (il disegno torna "prima emissione").
pub fn reset(s: &mut Session) -> Result<usize> {
    let archivi = layer_archivi(s)?;
    if archivi.iter().any(|l| crate::util::layer_corrente(s).is_ok_and(|c| &c == l)) {
        esegui(s, "layer.current", json!({"name": "0"}))?;
    }
    for l in &archivi {
        let c = crate::multifoglio::contenuto(s, l)?;
        if !c.is_empty() {
            esegui(s, "erase", json!({"handles": c}))?;
        }
        esegui(s, "layer.delete", json!({"name": l}))?;
    }
    let simb: Vec<String> = simboli(s)?.into_iter().map(|(_, b)| b.handle).collect();
    if !simb.is_empty() {
        esegui(s, "erase", json!({"handles": simb}))?;
    }
    Ok(archivi.len())
}

fn fogli_param(s: &mut Session, p: &Value) -> Result<Vec<u32>> {
    let tutti: Vec<u32> = fogli::elenco(s)?.iter().map(|f| f.numero).collect();
    Ok(match p.get("fogli") {
        Some(Value::Array(a)) => a.iter().filter_map(|v| v.as_u64().and_then(|n| u32::try_from(n).ok())).collect(),
        Some(Value::String(t)) if t.eq_ignore_ascii_case("corrente") => fogli::corrente(s)?.map(|f| vec![f.numero]).unwrap_or_default(),
        _ => tutti,
    })
}

fn run_crea(s: &mut Session, p: &Value) -> Result<Value> {
    let f = fogli_param(s, p)?;
    if f.is_empty() {
        return Err(errore("Nessun foglio da revisionare"));
    }
    let numero = match testo_param(p, "numero").filter(|n| !n.trim().is_empty()) {
        Some(n) => n,
        None => prossimo_numero(&elenco(s)?),
    };
    let modifica = testo_param(p, "modifica").unwrap_or_default();
    let data = testo_param(p, "data").unwrap_or_else(|| crate::commesse::data_leggibile(&crate::commesse::adesso()).split(' ').next().unwrap_or_default().to_string());
    let firma = testo_param(p, "firma").unwrap_or_default();
    in_gruppo(s, "Crea revisione", |s| {
        let n = crea(s, &f, &numero, &modifica, &data, &firma)?;
        Ok(json!({"numero": numero, "fogli": f, "message": format!("Revisione {numero} su {n} fogli: {modifica}")}))
    })
}

fn run_elenco(s: &mut Session, _p: &Value) -> Result<Value> {
    let v = elenco(s)?;
    let righe: Vec<String> = v.iter().map(|r| format!("{:02}  rev {}  {}  {}  {}", r.foglio, r.numero, r.data, r.firma, r.modifica)).collect();
    Ok(json!({
        "revisioni": v.iter().map(|r| json!({"foglio": r.foglio, "numero": r.numero, "modifica": r.modifica, "data": r.data, "firma": r.firma, "archiviata": r.archiviata})).collect::<Vec<_>>(),
        "prossima": prossimo_numero(&v),
        "message": if righe.is_empty() { "Nessuna revisione".to_string() } else { righe.join("\n") },
    }))
}

fn run_vedi(s: &mut Session, p: &Value) -> Result<Value> {
    let n = match numero_param(p, "foglio") {
        Some(n) => n,
        None => fogli::corrente(s)?.map(|f| f.numero).ok_or_else(|| errore("Nessun foglio corrente"))?,
    };
    let numero = testo_param(p, "numero").filter(|t| !t.trim().is_empty());
    vedi(s, n, numero.as_deref())?;
    Ok(json!({"message": match numero { Some(r) => format!("Foglio {n} com'era nella revisione {r} (solo da vedere)"), None => format!("Foglio {n}: versione corrente") }}))
}

fn run_annulla(s: &mut Session, p: &Value) -> Result<Value> {
    let numero = testo_param(p, "numero").ok_or_else(|| errore("Indicare la revisione da annullare"))?;
    in_gruppo(s, "Annulla revisione", |s| {
        let f = annulla(s, &numero)?;
        Ok(json!({"fogli": f, "message": format!("Revisione {numero} annullata su {} fogli", f.len())}))
    })
}

fn run_reset(s: &mut Session, _p: &Value) -> Result<Value> {
    in_gruppo(s, "Reset revisioni", |s| {
        let n = reset(s)?;
        Ok(json!({"message": format!("Tolte tutte le revisioni ({n} fogli archiviati)")}))
    })
}

pub fn comandi() -> Vec<CommandSpec> {
    let spec = |id, label, menu, params, run| CommandSpec { id, label, menu, shortcut: None, aliases: &[], params, enabled: has_doc, run, undoable: false, interactive: None, transparent: false };
    vec![
        spec(
            "revisione.crea",
            "Crea Revisione",
            &["FOGLI", "REVISIONE Fogli", "Crea revisione…"],
            "{numero?: (default il seguente: 00, 01…), modifica, data?: (oggi), firma?, fogli?: [numeri] | \"corrente\" (default tutti)}",
            run_crea as cadcraft_engine::cmd::Run,
        ),
        spec("revisione.elenco", "Elenco Revisioni", &["FOGLI", "REVISIONE Fogli", "Elenco revisioni"], "{} → {revisioni, prossima}", run_elenco),
        spec("revisione.vedi", "Vedi Revisione", &["FOGLI", "REVISIONE Fogli", "Vedi revisione…"], "{foglio?, numero?: (senza: la versione corrente)}", run_vedi),
        spec("revisione.annulla", "Annulla Revisione", &["FOGLI", "REVISIONE Fogli", "Annulla revisione…"], "{numero} — il foglio torna com'era archiviato", run_annulla),
        spec("revisione.reset", "Reset Revisioni", &["FOGLI", "REVISIONE Fogli", "Reset revisioni"], "{} — toglie archivi e simboli delle revisioni", run_reset),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layer_e_numeri() {
        assert_eq!(layer_archivio(5, "01"), "5_REV_01");
        assert_eq!(da_layer_archivio("12_REV_03"), Some((12, "03".into())));
        assert!(fogli::numero_da_layer("5_REV_01").is_none());
        let r = |n: &str| Revisione { foglio: 1, numero: n.into(), modifica: String::new(), data: String::new(), firma: String::new(), archiviata: true };
        assert_eq!(prossimo_numero(&[]), "00");
        assert_eq!(prossimo_numero(&[r("00"), r("03")]), "04");
    }
}
