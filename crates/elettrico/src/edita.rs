//! Edita Fili e Morsetti (XS): modifica dei dati di un morsetto, di un numero di filo o di un
//! rimando già disegnati, con i controlli del caso (numeri doppi nella morsettiera) e
//! l'aggiornamento di ciò che ne dipende (fili sui morsetti, riferimenti dei rimandi).

use cadcraft_engine::cmd::has_doc;
use cadcraft_engine::{CommandSpec, Result, Session};
use serde_json::{Value, json};

use crate::util::{Inserito, blocchi_inseriti, errore, esegui, in_gruppo, testo_param};
use crate::{fili, morsetti, numerazione};

/// Che cosa è un oggetto disegnato, per la finestra di modifica.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tipo {
    Componente,
    Morsetto,
    NumeroFilo,
    Rimando,
    Cavo,
    Bit,
    Altro,
}

impl Tipo {
    pub fn codice(self) -> &'static str {
        match self {
            Tipo::Componente => "componente",
            Tipo::Morsetto => "morsetto",
            Tipo::NumeroFilo => "filo",
            Tipo::Rimando => "rimando",
            Tipo::Cavo => "cavo",
            Tipo::Bit => "bit",
            Tipo::Altro => "altro",
        }
    }
}

/// Il blocco con l'handle dato e il suo tipo.
pub fn riconosci(s: &mut Session, handle: &str) -> Result<(Tipo, Inserito)> {
    let b = blocchi_inseriti(s, "*")?.into_iter().find(|b| b.handle.eq_ignore_ascii_case(handle)).ok_or_else(|| errore(format!("{handle} non è un blocco")))?;
    let nome = b.nome.to_ascii_uppercase();
    let pres = b.attributo("PRES").to_ascii_uppercase();
    let tipo = if pres == "F_PLC" {
        Tipo::Bit
    } else if pres == "M" || pres.starts_with('F') || (!b.attributo("NOME").is_empty() && pres.is_empty()) {
        Tipo::Componente
    } else if nome == morsetti::BLOCCO_MORSETTO || nome == morsetti::BLOCCO_CONNETTORE || nome.starts_with("_MORS") {
        Tipo::Morsetto
    } else if nome == numerazione::BLOCCO_NUMERO || nome.starts_with("BLCOR") {
        Tipo::NumeroFilo
    } else if nome.starts_with("_ALIM_") || nome.starts_with("_SIGN_") {
        Tipo::Rimando
    } else if nome == crate::cavi::BLOCCO_SEGNALINO {
        Tipo::Cavo
    } else {
        Tipo::Altro
    };
    Ok((tipo, b))
}

fn run_riconosci(s: &mut Session, p: &Value) -> Result<Value> {
    let h = testo_param(p, "handle").ok_or_else(|| errore("Indicare l'oggetto (handle)"))?;
    let (t, b) = riconosci(s, &h)?;
    let attributi: serde_json::Map<String, Value> = b.attributi.iter().map(|(k, v)| (k.clone(), json!(v))).collect();
    Ok(json!({"tipo": t.codice(), "blocco": b.nome, "attributi": attributi, "message": format!("{} {}", t.codice(), b.nome)}))
}

fn run_morsetto(s: &mut Session, p: &Value) -> Result<Value> {
    let h = testo_param(p, "handle").ok_or_else(|| errore("Indicare il morsetto (handle)"))?;
    in_gruppo(s, "Modifica morsetto", |s| {
        let (t, b) = riconosci(s, &h)?;
        if t != Tipo::Morsetto {
            return Err(errore("L'oggetto non è un morsetto"));
        }
        let morsettiera = testo_param(p, "morsettiera").unwrap_or_else(|| b.attributo("MORSETTIERA").to_string());
        let numero = testo_param(p, "numero").unwrap_or_else(|| b.attributo("NUMM").to_string());
        let doppio = morsetti::morsetti(s)?.into_iter().any(|m| m.blocco.handle != b.handle && m.morsettiera.eq_ignore_ascii_case(&morsettiera) && m.numero == numero && m.blocco.attributo("LIVELLO") == b.attributo("LIVELLO"));
        if doppio {
            return Err(errore(format!("Il morsetto {numero} esiste già nella morsettiera {morsettiera}")));
        }
        let mut v = serde_json::Map::new();
        v.insert("MORSETTIERA".into(), json!(morsettiera));
        v.insert("NUMM".into(), json!(numero));
        for (k, tag) in [("quadro", "QUADRO"), ("sezione", "SEZIONE"), ("costruttore", "COSTRUTTORE"), ("tipo", "TIPO"), ("utenza", "UTENZA"), ("note", "NOTEM"), ("livello", "LIVELLO")] {
            if let Some(x) = testo_param(p, k) {
                v.insert(tag.into(), json!(x));
            }
        }
        esegui(s, "attedit", json!({"handle": b.handle, "values": v}))?;
        Ok(json!({"message": format!("Morsetto {morsettiera}:{numero} aggiornato")}))
    })
}

fn run_filo(s: &mut Session, p: &Value) -> Result<Value> {
    let h = testo_param(p, "handle").ok_or_else(|| errore("Indicare il numero del filo (handle)"))?;
    in_gruppo(s, "Modifica filo", |s| {
        let (t, b) = riconosci(s, &h)?;
        if t != Tipo::NumeroFilo {
            return Err(errore("L'oggetto non è il numero di un filo"));
        }
        let numero = testo_param(p, "numero");
        let bloccato = p.get("bloccato").and_then(Value::as_bool);
        let n = numerazione::cambia_numero(s, &b.handle, numero.as_deref(), bloccato)?;
        Ok(json!({"etichette": n, "message": "Numero del filo aggiornato"}))
    })
}

fn run_rimando(s: &mut Session, p: &Value) -> Result<Value> {
    let h = testo_param(p, "handle").ok_or_else(|| errore("Indicare il rimando (handle)"))?;
    in_gruppo(s, "Modifica rimando", |s| {
        let (t, b) = riconosci(s, &h)?;
        if t != Tipo::Rimando {
            return Err(errore("L'oggetto non è un rimando"));
        }
        let mut v = serde_json::Map::new();
        if let Some(x) = testo_param(p, "segnale").filter(|x| !x.trim().is_empty()) {
            v.insert("SEGNALE".into(), json!(x.trim()));
        }
        if let Some(x) = testo_param(p, "descrizione") {
            v.insert("#LINEA".into(), json!(x));
        }
        esegui(s, "attedit", json!({"handle": b.handle, "values": v}))?;
        let (ok, soli) = fili::aggiorna_rimandi(s)?;
        Ok(json!({"collegati": ok, "senzaCoppia": soli.len(), "message": "Rimando aggiornato"}))
    })
}

pub fn comandi() -> Vec<CommandSpec> {
    let spec = |id, label, menu, params, run| CommandSpec { id, label, menu, shortcut: None, aliases: &[], params, enabled: has_doc, run, undoable: false, interactive: None, transparent: false };
    vec![
        spec("xs.riconosci", "Che oggetto è", &[], "{handle} → {tipo: componente|morsetto|filo|rimando|cavo|bit|altro, blocco, attributi}", run_riconosci as cadcraft_engine::cmd::Run),
        spec(
            "editamorsetto",
            "Modifica Morsetto",
            &["Elettrico", "Modifica Fili e Morsetti", "Morsetto"],
            "{handle, morsettiera?, numero?, quadro?, sezione?, costruttore?, tipo?, utenza?, note?, livello?}",
            run_morsetto,
        ),
        spec(
            "editafilo",
            "Modifica Numero Filo",
            &["Elettrico", "Modifica Fili e Morsetti", "Numero filo"],
            "{handle: etichetta del numero, numero? (tutta la rete; cambiato resta bloccato), bloccato?}",
            run_filo,
        ),
        spec("editarimando", "Modifica Rimando", &["Elettrico", "Modifica Fili e Morsetti", "Rimando"], "{handle, segnale?, descrizione?}", run_rimando),
    ]
}
