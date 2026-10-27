//! Traduttore: un dizionario di frasi in più lingue e la traduzione del progetto.
//!
//! Il dizionario è un CSV aperto (separatore `;`) con i codici delle lingue nella prima riga
//! (`IT;EN;DE;FR;ES`) e una frase per riga. La traduzione rende bilingue lo schema, come quelli
//! per l'estero: la funzione dei componenti (`FUNZIONE1` → `FUNZIONE2`), il titolo dei fogli
//! (`_TITOLO` → `_TITOLO1`) e le descrizioni delle colonne (prime due righe → ultime due).
//! Le frasi si cercano intere, senza badare a maiuscole e spazi; una frase scritta tutta in
//! maiuscolo resta in maiuscolo. Quelle senza traduzione si elencano per completare il dizionario.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use cadcraft_engine::cmd::{always, has_doc};
use cadcraft_engine::{CommandSpec, Result, Session};
use serde_json::{Value, json};

use crate::archivio::campi;
use crate::util::{blocchi_inseriti, errore, esegui, in_gruppo, testo_param};
use crate::{fogli, siglatura, squadratura};

/// Le lingue del dizionario iniziale.
const LINGUE_BASE: [&str; 5] = ["IT", "EN", "DE", "FR", "ES"];

/// Il dizionario iniziale: frasi frequenti negli schemi di macchine e quadri.
const FRASI_BASE: &[[&str; 5]] = &[
    ["ALIMENTAZIONE", "POWER SUPPLY", "STROMVERSORGUNG", "ALIMENTATION", "ALIMENTACIÓN"],
    ["ALIMENTAZIONE GENERALE", "MAIN POWER SUPPLY", "HAUPTSTROMVERSORGUNG", "ALIMENTATION GÉNÉRALE", "ALIMENTACIÓN GENERAL"],
    ["ALIMENTAZIONE AUSILIARI", "AUXILIARY POWER SUPPLY", "HILFSSTROMVERSORGUNG", "ALIMENTATION AUXILIAIRE", "ALIMENTACIÓN AUXILIAR"],
    ["ALIMENTATORE 24VDC", "24VDC POWER SUPPLY UNIT", "NETZTEIL 24VDC", "ALIMENTATION 24VCC", "FUENTE DE ALIMENTACIÓN 24VCC"],
    ["INTERRUTTORE GENERALE", "MAIN SWITCH", "HAUPTSCHALTER", "INTERRUPTEUR GÉNÉRAL", "INTERRUPTOR GENERAL"],
    ["SEZIONATORE GENERALE", "MAIN DISCONNECTOR", "HAUPTTRENNSCHALTER", "SECTIONNEUR GÉNÉRAL", "SECCIONADOR GENERAL"],
    ["MOTORE", "MOTOR", "MOTOR", "MOTEUR", "MOTOR"],
    ["MOTORE POMPA", "PUMP MOTOR", "PUMPENMOTOR", "MOTEUR POMPE", "MOTOR BOMBA"],
    ["POMPA", "PUMP", "PUMPE", "POMPE", "BOMBA"],
    ["POMPA IDRAULICA", "HYDRAULIC PUMP", "HYDRAULIKPUMPE", "POMPE HYDRAULIQUE", "BOMBA HIDRÁULICA"],
    ["NASTRO TRASPORTATORE", "CONVEYOR BELT", "FÖRDERBAND", "CONVOYEUR À BANDE", "CINTA TRANSPORTADORA"],
    ["VENTILATORE", "FAN", "LÜFTER", "VENTILATEUR", "VENTILADOR"],
    ["VENTILATORE ARMADIO", "PANEL FAN", "SCHALTSCHRANKLÜFTER", "VENTILATEUR ARMOIRE", "VENTILADOR ARMARIO"],
    ["ILLUMINAZIONE ARMADIO", "PANEL LIGHTING", "SCHALTSCHRANKBELEUCHTUNG", "ÉCLAIRAGE ARMOIRE", "ILUMINACIÓN ARMARIO"],
    ["PRESA DI SERVIZIO", "SERVICE SOCKET", "SERVICESTECKDOSE", "PRISE DE SERVICE", "TOMA DE SERVICIO"],
    ["RESISTENZA ANTICONDENSA", "ANTI-CONDENSATION HEATER", "HEIZUNG GEGEN KONDENSWASSER", "RÉSISTANCE ANTI-CONDENSATION", "RESISTENCIA ANTICONDENSACIÓN"],
    ["TERMOSTATO", "THERMOSTAT", "THERMOSTAT", "THERMOSTAT", "TERMOSTATO"],
    ["TRASFORMATORE", "TRANSFORMER", "TRANSFORMATOR", "TRANSFORMATEUR", "TRANSFORMADOR"],
    ["TRASFORMATORE AUSILIARI", "AUXILIARY TRANSFORMER", "STEUERTRANSFORMATOR", "TRANSFORMATEUR AUXILIAIRE", "TRANSFORMADOR AUXILIAR"],
    ["INVERTER", "INVERTER", "FREQUENZUMRICHTER", "VARIATEUR", "VARIADOR"],
    ["AVVIATORE", "STARTER", "ANLASSER", "DÉMARREUR", "ARRANCADOR"],
    ["PROTEZIONE MOTORE", "MOTOR PROTECTION", "MOTORSCHUTZ", "PROTECTION MOTEUR", "PROTECCIÓN MOTOR"],
    ["MARCIA", "START", "START", "MARCHE", "MARCHA"],
    ["ARRESTO", "STOP", "STOPP", "ARRÊT", "PARADA"],
    ["MARCIA MOTORE", "MOTOR START", "MOTOR START", "MARCHE MOTEUR", "MARCHA MOTOR"],
    ["ARRESTO MOTORE", "MOTOR STOP", "MOTOR STOPP", "ARRÊT MOTEUR", "PARADA MOTOR"],
    ["MARCIA AVANTI", "FORWARD", "VORWÄRTS", "MARCHE AVANT", "MARCHA ADELANTE"],
    ["MARCIA INDIETRO", "REVERSE", "RÜCKWÄRTS", "MARCHE ARRIÈRE", "MARCHA ATRÁS"],
    ["SALITA", "UP", "AUF", "MONTÉE", "SUBIDA"],
    ["DISCESA", "DOWN", "AB", "DESCENTE", "BAJADA"],
    ["APERTURA", "OPENING", "ÖFFNEN", "OUVERTURE", "APERTURA"],
    ["CHIUSURA", "CLOSING", "SCHLIESSEN", "FERMETURE", "CIERRE"],
    ["EMERGENZA", "EMERGENCY", "NOTHALT", "URGENCE", "EMERGENCIA"],
    ["ARRESTO DI EMERGENZA", "EMERGENCY STOP", "NOT-HALT", "ARRÊT D'URGENCE", "PARADA DE EMERGENCIA"],
    ["RIARMO", "RESET", "QUITTIERUNG", "RÉARMEMENT", "REARME"],
    ["RIARMO EMERGENZA", "EMERGENCY RESET", "NOT-HALT QUITTIERUNG", "RÉARMEMENT URGENCE", "REARME EMERGENCIA"],
    ["MANUALE", "MANUAL", "HAND", "MANUEL", "MANUAL"],
    ["AUTOMATICO", "AUTOMATIC", "AUTOMATIK", "AUTOMATIQUE", "AUTOMÁTICO"],
    ["SELETTORE MANUALE AUTOMATICO", "MANUAL/AUTOMATIC SELECTOR", "WAHLSCHALTER HAND/AUTOMATIK", "SÉLECTEUR MANUEL/AUTOMATIQUE", "SELECTOR MANUAL/AUTOMÁTICO"],
    ["PRESENZA TENSIONE", "POWER ON", "SPANNUNG EIN", "PRÉSENCE TENSION", "PRESENCIA TENSIÓN"],
    ["LAMPADA PRESENZA TENSIONE", "POWER ON LAMP", "LEUCHTE SPANNUNG EIN", "VOYANT PRÉSENCE TENSION", "PILOTO PRESENCIA TENSIÓN"],
    ["ANOMALIA", "FAULT", "STÖRUNG", "DÉFAUT", "AVERÍA"],
    ["ALLARME", "ALARM", "ALARM", "ALARME", "ALARMA"],
    ["SEGNALAZIONE ALLARME", "ALARM SIGNAL", "ALARMMELDUNG", "SIGNALISATION ALARME", "SEÑALIZACIÓN ALARMA"],
    ["INTERVENTO TERMICO", "THERMAL OVERLOAD TRIPPED", "MOTORSCHUTZ AUSGELÖST", "DÉCLENCHEMENT THERMIQUE", "DISPARO TÉRMICO"],
    ["SIRENA", "SIREN", "SIRENE", "SIRÈNE", "SIRENA"],
    ["LAMPEGGIANTE", "FLASHING LIGHT", "BLINKLEUCHTE", "FEU CLIGNOTANT", "LUZ INTERMITENTE"],
    ["FINECORSA", "LIMIT SWITCH", "ENDSCHALTER", "FIN DE COURSE", "FINAL DE CARRERA"],
    ["FINECORSA ALTO", "UPPER LIMIT SWITCH", "ENDSCHALTER OBEN", "FIN DE COURSE HAUT", "FINAL DE CARRERA SUPERIOR"],
    ["FINECORSA BASSO", "LOWER LIMIT SWITCH", "ENDSCHALTER UNTEN", "FIN DE COURSE BAS", "FINAL DE CARRERA INFERIOR"],
    ["SENSORE", "SENSOR", "SENSOR", "CAPTEUR", "SENSOR"],
    ["SENSORE DI PROSSIMITÀ", "PROXIMITY SENSOR", "NÄHERUNGSSCHALTER", "CAPTEUR DE PROXIMITÉ", "SENSOR DE PROXIMIDAD"],
    ["FOTOCELLULA", "PHOTOCELL", "LICHTSCHRANKE", "CELLULE PHOTOÉLECTRIQUE", "FOTOCÉLULA"],
    ["PRESSOSTATO", "PRESSURE SWITCH", "DRUCKSCHALTER", "PRESSOSTAT", "PRESOSTATO"],
    ["LIVELLO MINIMO", "MINIMUM LEVEL", "MINDESTSTAND", "NIVEAU MINIMUM", "NIVEL MÍNIMO"],
    ["LIVELLO MASSIMO", "MAXIMUM LEVEL", "HÖCHSTSTAND", "NIVEAU MAXIMUM", "NIVEL MÁXIMO"],
    ["ELETTROVALVOLA", "SOLENOID VALVE", "MAGNETVENTIL", "ÉLECTROVANNE", "ELECTROVÁLVULA"],
    ["CONTATTORE", "CONTACTOR", "SCHÜTZ", "CONTACTEUR", "CONTACTOR"],
    ["RELÈ", "RELAY", "RELAIS", "RELAIS", "RELÉ"],
    ["RELÈ DI SICUREZZA", "SAFETY RELAY", "SICHERHEITSRELAIS", "RELAIS DE SÉCURITÉ", "RELÉ DE SEGURIDAD"],
    ["TEMPORIZZATORE", "TIMER", "ZEITRELAIS", "TEMPORISATEUR", "TEMPORIZADOR"],
    ["FUSIBILI", "FUSES", "SICHERUNGEN", "FUSIBLES", "FUSIBLES"],
    ["PROTEZIONE CIRCUITI AUSILIARI", "AUXILIARY CIRCUIT PROTECTION", "SCHUTZ HILFSSTROMKREISE", "PROTECTION CIRCUITS AUXILIAIRES", "PROTECCIÓN CIRCUITOS AUXILIARES"],
    ["INGRESSI DIGITALI", "DIGITAL INPUTS", "DIGITALE EINGÄNGE", "ENTRÉES NUMÉRIQUES", "ENTRADAS DIGITALES"],
    ["USCITE DIGITALI", "DIGITAL OUTPUTS", "DIGITALE AUSGÄNGE", "SORTIES NUMÉRIQUES", "SALIDAS DIGITALES"],
    ["INGRESSI ANALOGICI", "ANALOG INPUTS", "ANALOGE EINGÄNGE", "ENTRÉES ANALOGIQUES", "ENTRADAS ANALÓGICAS"],
    ["USCITE ANALOGICHE", "ANALOG OUTPUTS", "ANALOGE AUSGÄNGE", "SORTIES ANALOGIQUES", "SALIDAS ANALÓGICAS"],
    ["MORSETTIERA", "TERMINAL BLOCK", "KLEMMLEISTE", "BORNIER", "REGLETA DE BORNES"],
    ["QUADRO ELETTRICO", "ELECTRICAL PANEL", "SCHALTSCHRANK", "ARMOIRE ÉLECTRIQUE", "CUADRO ELÉCTRICO"],
    ["PULSANTIERA", "PUSH-BUTTON PANEL", "BEDIENTABLEAU", "BOÎTE À BOUTONS", "BOTONERA"],
    ["BORDO MACCHINA", "MACHINE SIDE", "MASCHINENSEITE", "BORD MACHINE", "LADO MÁQUINA"],
    ["MESSA A TERRA", "EARTHING", "ERDUNG", "MISE À LA TERRE", "PUESTA A TIERRA"],
    ["LISTA FOGLI", "SHEET LIST", "BLATTVERZEICHNIS", "LISTE DES FOLIOS", "LISTA DE HOJAS"],
    ["SCHEMA DI POTENZA", "POWER DIAGRAM", "LEISTUNGSSCHALTPLAN", "SCHÉMA DE PUISSANCE", "ESQUEMA DE POTENCIA"],
    ["SCHEMA AUSILIARI", "CONTROL DIAGRAM", "STEUERSTROMPLAN", "SCHÉMA DE COMMANDE", "ESQUEMA DE MANDO"],
    ["DISTINTA MATERIALI", "BILL OF MATERIALS", "STÜCKLISTE", "NOMENCLATURE", "LISTA DE MATERIALES"],
];

struct Dizionario {
    file: Option<PathBuf>,
    lingue: Vec<String>,
    /// Una riga per frase, allineata a `lingue` (vuoto = non tradotta).
    frasi: Vec<Vec<String>>,
}

static DIZIONARIO: Mutex<Option<Dizionario>> = Mutex::new(None);

fn base() -> Dizionario {
    Dizionario { file: None, lingue: LINGUE_BASE.iter().map(|l| l.to_string()).collect(), frasi: FRASI_BASE.iter().map(|r| r.iter().map(|t| t.to_string()).collect()).collect() }
}

fn con<T>(f: impl FnOnce(&mut Dizionario) -> Result<T>) -> Result<T> {
    let mut g = DIZIONARIO.lock().map_err(|_| errore("dizionario non disponibile"))?;
    f(g.get_or_insert_with(base))
}

/// La forma con cui si confrontano le frasi: maiuscole, spazi singoli.
fn chiave(t: &str) -> String {
    t.split_whitespace().collect::<Vec<_>>().join(" ").to_uppercase()
}

fn lingua(t: &str) -> String {
    t.trim().to_uppercase()
}

/// Legge un dizionario CSV: (lingue, frasi).
pub fn leggi_csv(testo: &str) -> Result<(Vec<String>, Vec<Vec<String>>)> {
    let testo = testo.trim_start_matches('\u{feff}');
    let mut righe = testo.lines().filter(|l| !l.trim().is_empty());
    let intest = righe.next().ok_or_else(|| errore("dizionario vuoto"))?;
    let sep = [';', ',', '\t'].into_iter().max_by_key(|c| intest.matches(*c).count()).unwrap_or(';');
    let lingue: Vec<String> = campi(intest, sep).iter().map(|l| lingua(l)).collect();
    if lingue.len() < 2 || lingue.iter().any(String::is_empty) {
        return Err(errore("la prima riga deve avere i codici delle lingue (es. IT;EN;DE)"));
    }
    let frasi = righe
        .map(|r| {
            let mut f = campi(r, sep);
            f.resize(lingue.len(), String::new());
            f
        })
        .filter(|f| f.iter().any(|t| !t.is_empty()))
        .collect();
    Ok((lingue, frasi))
}

fn campo_csv(t: &str) -> String {
    if t.contains([';', '"', '\n']) { format!("\"{}\"", t.replace('"', "\"\"")) } else { t.to_string() }
}

fn testo_csv(d: &Dizionario) -> String {
    let mut out = d.lingue.join(";");
    out.push('\n');
    for f in &d.frasi {
        out.push_str(&f.iter().map(|t| campo_csv(t)).collect::<Vec<_>>().join(";"));
        out.push('\n');
    }
    out
}

fn salva(d: &Dizionario) -> Result<()> {
    let Some(file) = &d.file else { return Ok(()) };
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir).map_err(|e| errore(format!("{}: {e}", dir.display())))?;
    }
    std::fs::write(file, testo_csv(d)).map_err(|e| errore(format!("{}: {e}", file.display())))
}

/// Usa `file` come dizionario: lo legge se esiste, altrimenti lo crea con le frasi iniziali.
/// Restituisce le frasi.
pub fn imposta(file: &Path) -> Result<usize> {
    let letto = match std::fs::read_to_string(file) {
        Ok(t) => Some(leggi_csv(&t)?),
        Err(_) => None,
    };
    con(|d| {
        *d = match letto {
            Some((lingue, frasi)) => Dizionario { file: Some(file.to_path_buf()), lingue, frasi },
            None => {
                let nuovo = Dizionario { file: Some(file.to_path_buf()), ..base() };
                salva(&nuovo)?;
                nuovo
            }
        };
        Ok(d.frasi.len())
    })
}

fn indice_lingua(d: &mut Dizionario, l: &str) -> usize {
    let l = lingua(l);
    match d.lingue.iter().position(|x| *x == l) {
        Some(i) => i,
        None => {
            d.lingue.push(l);
            for f in &mut d.frasi {
                f.push(String::new());
            }
            d.lingue.len() - 1
        }
    }
}

/// Le lingue del dizionario.
pub fn lingue() -> Vec<String> {
    con(|d| Ok(d.lingue.clone())).unwrap_or_default()
}

/// Le coppie (frase, traduzione) del dizionario da `da` ad `a`, in ordine alfabetico.
pub fn voci(da: &str, a: &str) -> Vec<(String, String)> {
    con(|d| {
        let (i, j) = (indice_lingua(d, da), indice_lingua(d, a));
        let mut v: Vec<(String, String)> = d.frasi.iter().filter_map(|f| Some((f.get(i)?.clone(), f.get(j)?.clone()))).filter(|(x, _)| !x.is_empty()).collect();
        v.sort();
        Ok(v)
    })
    .unwrap_or_default()
}

/// La traduzione di `frase` da `da` ad `a`; una frase tutta maiuscola resta maiuscola.
pub fn traduzione(frase: &str, da: &str, a: &str) -> Option<String> {
    let k = chiave(frase);
    if k.is_empty() {
        return None;
    }
    let t = con(|d| {
        let (i, j) = (indice_lingua(d, da), indice_lingua(d, a));
        Ok(d.frasi.iter().find(|f| f.get(i).is_some_and(|x| chiave(x) == k)).and_then(|f| f.get(j)).filter(|t| !t.trim().is_empty()).cloned())
    })
    .ok()
    .flatten()?;
    let maiuscola = frase.chars().any(char::is_alphabetic) && !frase.chars().any(char::is_lowercase);
    Some(if maiuscola { t.to_uppercase() } else { t })
}

/// Aggiunge (o corregge) la traduzione di una frase e salva il dizionario.
pub fn aggiungi(frase: &str, da: &str, traduzione: &str, a: &str) -> Result<()> {
    let k = chiave(frase);
    if k.is_empty() {
        return Err(errore("Indicare la frase da tradurre"));
    }
    con(|d| {
        let (i, j) = (indice_lingua(d, da), indice_lingua(d, a));
        match d.frasi.iter_mut().find(|f| f.get(i).is_some_and(|x| chiave(x) == k)) {
            Some(f) => {
                if let Some(t) = f.get_mut(j) {
                    *t = traduzione.trim().to_string();
                }
            }
            None => {
                let mut f = vec![String::new(); d.lingue.len()];
                if let Some(t) = f.get_mut(i) {
                    *t = frase.trim().to_string();
                }
                if let Some(t) = f.get_mut(j) {
                    *t = traduzione.trim().to_string();
                }
                d.frasi.push(f);
            }
        }
        salva(d)
    })
}

/// Unisce al dizionario quello del CSV `testo` (le traduzioni nuove sostituiscono le vecchie).
/// Restituisce le frasi lette.
pub fn importa(testo: &str) -> Result<usize> {
    let (lingue, frasi) = leggi_csv(testo)?;
    con(|d| {
        let idx: Vec<usize> = lingue.iter().map(|l| indice_lingua(d, l)).collect();
        let prima = idx.first().copied().unwrap_or(0);
        for f in &frasi {
            let k = f.first().map(|t| chiave(t)).unwrap_or_default();
            let pos = if k.is_empty() { None } else { d.frasi.iter().position(|x| x.get(prima).is_some_and(|t| chiave(t) == k)) };
            let riga = match pos {
                Some(p) => p,
                None => {
                    d.frasi.push(vec![String::new(); d.lingue.len()]);
                    d.frasi.len() - 1
                }
            };
            for (c, t) in idx.iter().zip(f) {
                if let Some(x) = d.frasi.get_mut(riga).and_then(|r| r.get_mut(*c))
                    && !t.is_empty()
                {
                    *x = t.clone();
                }
            }
        }
        salva(d)?;
        Ok(frasi.len())
    })
}

/// Il dizionario come CSV.
pub fn esporta() -> String {
    con(|d| Ok(testo_csv(d))).unwrap_or_default()
}

/// Un testo del progetto da tradurre.
#[derive(Clone, Debug, PartialEq)]
pub struct Testo {
    /// Dove sta: `-KM1`, `Foglio 3`, `Foglio 3, colonna 2`.
    pub dove: String,
    pub originale: String,
    /// La traduzione già scritta nello schema.
    pub tradotto: String,
}

/// Una modifica da fare per tradurre un testo.
enum Scrittura {
    Attributi(String, Value),
    Titolo(u32, String, String),
}

/// I testi traducibili del progetto, con il modo di scrivere la traduzione.
fn testi(s: &mut Session) -> Result<Vec<(Testo, Box<dyn Fn(&str) -> Scrittura>)>> {
    type Scrivi = Box<dyn Fn(&str) -> Scrittura>;
    let mut v: Vec<(Testo, Scrivi)> = Vec::new();
    for c in siglatura::componenti(s)? {
        let f1 = c.blocco.attributo("FUNZIONE1").trim().to_string();
        if f1.is_empty() {
            continue;
        }
        let h = c.blocco.handle.clone();
        v.push((Testo { dove: c.nome.clone(), originale: f1, tradotto: c.blocco.attributo("FUNZIONE2").trim().to_string() }, Box::new(move |t| Scrittura::Attributi(h.clone(), json!({"FUNZIONE2": t})))));
    }
    for f in fogli::elenco(s)? {
        if f.titolo.trim().is_empty() {
            continue;
        }
        let (n, t0) = (f.numero, f.titolo.clone());
        v.push((Testo { dove: format!("Foglio {n}"), originale: f.titolo.trim().to_string(), tradotto: f.titolo1.trim().to_string() }, Box::new(move |t| Scrittura::Titolo(n, t0.clone(), t.to_string()))));
    }
    let unisci = |a: &str, b: &str| [a.trim(), b.trim()].iter().filter(|x| !x.is_empty()).copied().collect::<Vec<_>>().join(" ");
    for b in blocchi_inseriti(s, squadratura::BLOCCO_DESCRIZIONE)? {
        let originale = unisci(b.attributo("RIGA1"), b.attributo("RIGA2"));
        if originale.is_empty() {
            continue;
        }
        let dove = format!("Foglio {}, colonna {}", fogli::numero_da_layer(&b.layer).unwrap_or(0), squadratura::colonna_riga(b.pos.0, b.pos.1).0);
        let due = !b.attributo("RIGA2").trim().is_empty();
        let h = b.handle.clone();
        v.push((
            Testo { dove, originale, tradotto: unisci(b.attributo("RIGA3"), b.attributo("RIGA4")) },
            Box::new(move |t| {
                let (r3, r4) = if due { fogli::due_righe(t, 0) } else { (t.to_string(), String::new()) };
                Scrittura::Attributi(h.clone(), json!({"RIGA3": r3, "RIGA4": r4}))
            }),
        ));
    }
    Ok(v)
}

/// I testi del progetto con la loro traduzione nello schema.
pub fn testi_del_progetto(s: &mut Session) -> Result<Vec<Testo>> {
    Ok(testi(s)?.into_iter().map(|(t, _)| t).collect())
}

/// Le frasi del progetto che il dizionario non sa tradurre da `da` ad `a`, senza ripetizioni.
pub fn mancanti(s: &mut Session, da: &str, a: &str) -> Result<Vec<String>> {
    let mut v: Vec<String> = Vec::new();
    for t in testi_del_progetto(s)? {
        if traduzione(&t.originale, da, a).is_none() && !v.iter().any(|x| chiave(x) == chiave(&t.originale)) {
            v.push(t.originale);
        }
    }
    v.sort();
    Ok(v)
}

/// Esito della traduzione del progetto.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Esito {
    pub tradotti: usize,
    /// Frasi senza traduzione nel dizionario.
    pub mancanti: Vec<String>,
}

/// Traduce il progetto da `da` ad `a`: scrive la seconda lingua dove manca (o sempre, con
/// `sovrascrivi`).
pub fn traduci(s: &mut Session, da: &str, a: &str, sovrascrivi: bool) -> Result<Esito> {
    let mut e = Esito::default();
    let mut scritture = Vec::new();
    for (t, scrivi) in testi(s)? {
        if !t.tradotto.is_empty() && !sovrascrivi {
            continue;
        }
        match traduzione(&t.originale, da, a) {
            Some(tr) if tr != t.tradotto => scritture.push(scrivi(&tr)),
            Some(_) => {}
            None => {
                if !e.mancanti.iter().any(|x| chiave(x) == chiave(&t.originale)) {
                    e.mancanti.push(t.originale);
                }
            }
        }
    }
    e.mancanti.sort();
    e.tradotti = scritture.len();
    for w in scritture {
        match w {
            Scrittura::Attributi(h, v) => {
                esegui(s, "attedit", json!({"handle": h, "values": v}))?;
            }
            Scrittura::Titolo(n, titolo, tr) => fogli::imposta_titolo(s, n, &titolo, Some(&tr))?,
        }
    }
    Ok(e)
}

// ---------- comandi ----------

fn lingue_param(p: &Value) -> (String, String) {
    (testo_param(p, "da").map_or_else(|| "IT".into(), |l| lingua(&l)), testo_param(p, "a").map_or_else(|| "EN".into(), |l| lingua(&l)))
}

fn run_traduci(s: &mut Session, p: &Value) -> Result<Value> {
    let (da, a) = lingue_param(p);
    let sovrascrivi = p.get("sovrascrivi").and_then(Value::as_bool).unwrap_or(false);
    in_gruppo(s, "Traduzione", |s| {
        let e = traduci(s, &da, &a, sovrascrivi)?;
        let msg = if e.mancanti.is_empty() { format!("Tradotti {} testi ({da} → {a})", e.tradotti) } else { format!("Tradotti {} testi ({da} → {a}); {} frasi senza traduzione", e.tradotti, e.mancanti.len()) };
        Ok(json!({"tradotti": e.tradotti, "mancanti": e.mancanti, "message": msg}))
    })
}

fn run_frasi(s: &mut Session, p: &Value) -> Result<Value> {
    let (da, a) = lingue_param(p);
    let mancanti = if s.doc().is_ok() { mancanti(s, &da, &a)? } else { Vec::new() };
    let voci: Vec<Value> = voci(&da, &a).into_iter().map(|(x, y)| json!([x, y])).collect();
    Ok(json!({"lingue": lingue(), "voci": voci, "mancanti": mancanti, "message": format!("{} frasi nel dizionario", voci.len())}))
}

fn run_aggiungi(_s: &mut Session, p: &Value) -> Result<Value> {
    let (da, a) = lingue_param(p);
    let frase = testo_param(p, "frase").unwrap_or_default();
    let tr = testo_param(p, "traduzione").unwrap_or_default();
    aggiungi(&frase, &da, &tr, &a)?;
    Ok(json!({"message": format!("{frase} → {tr}")}))
}

fn run_importa(_s: &mut Session, p: &Value) -> Result<Value> {
    let file = testo_param(p, "file").ok_or_else(|| errore("Indicare il file del dizionario (CSV)"))?;
    let testo = std::fs::read_to_string(&file).map_err(|e| errore(format!("{file}: {e}")))?;
    let n = importa(&testo)?;
    Ok(json!({"frasi": n, "message": format!("Importate {n} frasi")}))
}

fn run_esporta(_s: &mut Session, p: &Value) -> Result<Value> {
    let file = testo_param(p, "file").ok_or_else(|| errore("Indicare il file (CSV)"))?;
    std::fs::write(&file, esporta()).map_err(|e| errore(format!("{file}: {e}")))?;
    Ok(json!({"message": format!("Dizionario salvato in {file}")}))
}

pub fn comandi() -> Vec<CommandSpec> {
    let spec = |id, label, menu, params, enabled, run| CommandSpec { id, label, menu, shortcut: None, aliases: &[], params, enabled, run, undoable: false, interactive: None, transparent: false };
    vec![
        spec(
            "traduci",
            "TRADUZIONE",
            &["UTIL", "TRADUZIONE"],
            "{da?: \"IT\", a?: \"EN\", sovrascrivi?: false} → {tradotti, mancanti: [frasi senza traduzione]}",
            has_doc as cadcraft_engine::cmd::Enabled,
            run_traduci as cadcraft_engine::cmd::Run,
        ),
        spec("traduttore.frasi", "Dizionario", &[], "{da?, a?} → {lingue, voci: [[frase, traduzione]], mancanti}", always, run_frasi),
        spec("traduttore.aggiungi", "Aggiungi al dizionario", &[], "{frase, traduzione, da?, a?}", always, run_aggiungi),
        spec("traduttore.importa", "Importa dizionario", &["UTIL", "Utility Traduttore", "Importa dizionario"], "{file: CSV con le lingue nella prima riga}", always, run_importa),
        spec("traduttore.esporta", "Esporta dizionario", &["UTIL", "Utility Traduttore", "Esporta dizionario"], "{file}", always, run_esporta),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legge_il_dizionario_e_allinea_le_righe() {
        let (l, f) = leggi_csv("it;en;de\nPOMPA;PUMP\n\n\"MARCIA; AVANTI\";FORWARD;VORWÄRTS\n").unwrap();
        assert_eq!(l, ["IT", "EN", "DE"]);
        assert_eq!(f, vec![vec!["POMPA".to_string(), "PUMP".into(), String::new()], vec!["MARCIA; AVANTI".into(), "FORWARD".into(), "VORWÄRTS".into()]]);
        assert!(leggi_csv("solo\nPOMPA").is_err());
    }

    #[test]
    fn chiave_senza_maiuscole_e_spazi() {
        assert_eq!(chiave("  Ventilatore   armadio "), "VENTILATORE ARMADIO");
    }
}
