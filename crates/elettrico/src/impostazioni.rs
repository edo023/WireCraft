//! Impostazioni (settaggi) della commessa: siglatura, numerazione dei fili, conduttore in uso,
//! quadro/funzione/ubicazione correnti.
//!
//! Come nei CAD elettrici diffusi le impostazioni stanno con la commessa, in `CFG/WireCraft.ini`
//! (un file INI leggibile); un disegno fuori da una commessa usa quelle generali del programma.
//! Da una commessa che ha solo il file dei settaggi del CAD elettrico d'origine
//! (`CFG/Spac_cfg.ini`) si leggono le voci equivalenti (siglatura, separatori, passo dei fili).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use cadcraft_engine::cmd::always;
use cadcraft_engine::{CommandSpec, Result, Session};
use serde_json::{Value, json};

use crate::util::{errore, testo_param};

/// Come si compongono le sigle dei componenti.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Siglatura {
    /// `KA1`, `KA2`…: un progressivo per prefisso su tutto il progetto.
    Incrementale,
    /// `1KA1`, `2KA4`…: foglio davanti al prefisso, progressivo unico per prefisso.
    FoglioPrefissoAssoluta,
    /// `1KA1`, `2KA1`…: foglio davanti al prefisso, progressivo che riparte a ogni foglio.
    FoglioPrefissoRelativa,
    /// `KA01.1`, `KA02.4`…: prefisso, foglio, separatore, progressivo unico per prefisso.
    PrefissoFoglioAssoluta,
    /// `KA01.1`, `KA02.1`…: prefisso, foglio, separatore, progressivo che riparte a ogni foglio.
    PrefissoFoglioRelativa,
    /// `KA01.3`: prefisso, foglio, separatore, colonna; una lettera se la colonna è già presa.
    Posizionale,
}

impl Siglatura {
    pub const TUTTE: [Siglatura; 6] = [
        Siglatura::Incrementale,
        Siglatura::FoglioPrefissoAssoluta,
        Siglatura::FoglioPrefissoRelativa,
        Siglatura::PrefissoFoglioAssoluta,
        Siglatura::PrefissoFoglioRelativa,
        Siglatura::Posizionale,
    ];
    pub fn codice(self) -> &'static str {
        match self {
            Self::Incrementale => "incrementale",
            Self::FoglioPrefissoAssoluta => "foglio-prefisso-assoluta",
            Self::FoglioPrefissoRelativa => "foglio-prefisso-relativa",
            Self::PrefissoFoglioAssoluta => "prefisso-foglio-assoluta",
            Self::PrefissoFoglioRelativa => "prefisso-foglio-relativa",
            Self::Posizionale => "posizionale",
        }
    }
    pub fn descrizione(self) -> &'static str {
        match self {
            Self::Incrementale => "Incrementale (KA1, KA2…)",
            Self::FoglioPrefissoAssoluta => "Foglio-Prefisso incrementale assoluta (1KA1, 2KA4…)",
            Self::FoglioPrefissoRelativa => "Foglio-Prefisso incrementale relativa (1KA1, 2KA1…)",
            Self::PrefissoFoglioAssoluta => "Prefisso-Foglio incrementale assoluta (KA01.1, KA02.4…)",
            Self::PrefissoFoglioRelativa => "Prefisso-Foglio incrementale relativa (KA01.1, KA02.1…)",
            Self::Posizionale => "Posizionale: prefisso, foglio e colonna (KA01.3)",
        }
    }
    pub fn da_codice(t: &str) -> Option<Self> {
        let t = t.trim().to_ascii_lowercase();
        Self::TUTTE.into_iter().find(|s| s.codice() == t).or(match t.as_str() {
            // codici dei settaggi del CAD elettrico d'origine
            "incre" => Some(Self::Incrementale),
            "fgincr" => Some(Self::FoglioPrefissoRelativa),
            "fgincra" => Some(Self::FoglioPrefissoAssoluta),
            "prefgincr" => Some(Self::PrefissoFoglioRelativa),
            "prefgincra" => Some(Self::PrefissoFoglioAssoluta),
            "posiz" | "pos" => Some(Self::Posizionale),
            _ => None,
        })
    }
    /// Il progressivo riparte a ogni foglio.
    pub fn relativa(self) -> bool {
        matches!(self, Self::FoglioPrefissoRelativa | Self::PrefissoFoglioRelativa)
    }
}

/// Come si numerano i fili non collegati a un rimando.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NumeriFili {
    /// `05.03`: foglio, separatore, progressivo che riparte a ogni foglio.
    FoglioProgressivo,
    /// `1`, `2`, `3`…: progressivo unico su tutto il progetto.
    Progressivo,
    /// `053.1`: foglio e colonna, poi un progressivo nella colonna.
    Posizionale,
}

impl NumeriFili {
    pub const TUTTI: [NumeriFili; 3] = [NumeriFili::FoglioProgressivo, NumeriFili::Progressivo, NumeriFili::Posizionale];
    pub fn codice(self) -> &'static str {
        match self {
            Self::FoglioProgressivo => "foglio-progressivo",
            Self::Progressivo => "progressivo",
            Self::Posizionale => "posizionale",
        }
    }
    pub fn descrizione(self) -> &'static str {
        match self {
            Self::FoglioProgressivo => "Foglio.progressivo (05.01, 05.02…)",
            Self::Progressivo => "Progressivo su tutto il progetto (1, 2, 3…)",
            Self::Posizionale => "Posizionale: foglio, colonna e progressivo (053.1)",
        }
    }
    pub fn da_codice(t: &str) -> Option<Self> {
        let t = t.trim().to_ascii_lowercase();
        Self::TUTTI.into_iter().find(|s| s.codice() == t)
    }
}

/// Un conduttore (materiale del filo): sezione, colore, codice, costruttore.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Conduttore {
    pub sezione: String,
    pub colore: String,
    pub codice: String,
    pub costruttore: String,
}

impl Conduttore {
    /// Testo da mostrare accanto al filo: `1x1.5 BK`.
    pub fn etichetta(&self) -> String {
        [self.sezione.trim(), self.colore.trim()].iter().filter(|t| !t.is_empty()).copied().collect::<Vec<_>>().join(" ")
    }
    pub fn vuoto(&self) -> bool {
        self.sezione.trim().is_empty() && self.colore.trim().is_empty() && self.codice.trim().is_empty()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Impostazioni {
    pub siglatura: Siglatura,
    /// Separatore tra foglio e progressivo nelle sigle (`KA01.1`).
    pub separatore_sigla: String,
    /// Davanti a ogni sigla (`-KA01.1`).
    pub prefisso_assoluto: String,
    /// Cifre del numero di foglio nelle sigle e nei numeri dei fili (2 = `05`).
    pub cifre_foglio: usize,
    /// Sigle uguali ammesse in quadri diversi (il progressivo riparte per quadro).
    pub sigle_uguali_quadri: bool,
    pub numeri_fili: NumeriFili,
    pub separatore_fili: String,
    /// Cifre del progressivo dei fili (2 = `05.03`).
    pub cifre_filo: usize,
    pub prefisso_fili: String,
    pub suffisso_fili: String,
    /// Distanza tra fili paralleli (mm).
    pub passo_fili: f64,
    /// Conduttore in uso per i nuovi fili e per le alimentazioni.
    pub conduttore: Conduttore,
    pub conduttore_alimentazione: Conduttore,
    /// Applica il conduttore in uso ai nuovi collegamenti.
    pub applica_conduttore: bool,
    /// Quadro (+), funzione (=) e ubicazione (++) correnti per i nuovi simboli.
    pub quadro: String,
    pub funzione: String,
    pub ubicazione: String,
    /// Mostra il nome della morsettiera solo sul primo morsetto di una fila.
    pub morsettiera_solo_primo: bool,
    /// Nomi proposti per le alimentazioni (un gruppo per riga).
    pub nomi_alimentazioni: Vec<String>,
}

impl Default for Impostazioni {
    fn default() -> Self {
        Impostazioni {
            siglatura: Siglatura::PrefissoFoglioRelativa,
            separatore_sigla: ".".into(),
            prefisso_assoluto: "-".into(),
            cifre_foglio: 2,
            sigle_uguali_quadri: false,
            numeri_fili: NumeriFili::FoglioProgressivo,
            separatore_fili: ".".into(),
            cifre_filo: 2,
            prefisso_fili: String::new(),
            suffisso_fili: String::new(),
            passo_fili: 10.0,
            conduttore: Conduttore { sezione: "1x1.5".into(), colore: "BK".into(), codice: "H07V-K 1x1.5 BK".into(), costruttore: String::new() },
            conduttore_alimentazione: Conduttore { sezione: "1x2.5".into(), colore: "BK".into(), codice: "H07V-K 1x2.5 BK".into(), costruttore: String::new() },
            applica_conduttore: true,
            quadro: String::new(),
            funzione: String::new(),
            ubicazione: String::new(),
            morsettiera_solo_primo: true,
            nomi_alimentazioni: ["L1 L2 L3", "L1 L2 L3 N", "L1 L2 L3 N PE", "U V W", "U V W PE", "R S T", "L N", "L N PE", "L+ M", "+24V 0V"].iter().map(|s| (*s).to_string()).collect(),
        }
    }
}

struct Stato {
    file: Option<PathBuf>,
    imp: Impostazioni,
}

static STATO: Mutex<Option<Stato>> = Mutex::new(None);

fn con<T>(f: impl FnOnce(&mut Stato) -> T) -> Option<T> {
    let mut g = STATO.lock().ok()?;
    let st = g.get_or_insert_with(|| Stato { file: None, imp: Impostazioni::default() });
    Some(f(st))
}

/// Le impostazioni in uso.
pub fn correnti() -> Impostazioni {
    con(|st| st.imp.clone()).unwrap_or_default()
}

/// Il file da cui vengono le impostazioni in uso (se ce n'è uno).
pub fn file() -> Option<PathBuf> {
    con(|st| st.file.clone()).flatten()
}

/// Cambia le impostazioni in uso e le salva nel loro file (se ce n'è uno).
pub fn cambia(f: impl FnOnce(&mut Impostazioni)) -> Result<()> {
    let salva = con(|st| {
        f(&mut st.imp);
        st.file.clone().map(|p| (p, st.imp.clone()))
    })
    .flatten();
    if let Some((p, imp)) = salva {
        scrivi(&p, &imp)?;
    }
    Ok(())
}

/// Usa le impostazioni `imp` legate al file `file` (dove si salveranno i cambiamenti).
pub fn usa(file: Option<PathBuf>, imp: Impostazioni) {
    let _ = con(|st| {
        st.file = file;
        st.imp = imp;
    });
}

/// Il quadro corrente, se impostato.
pub fn quadro_corrente() -> Option<String> {
    let q = correnti().quadro;
    (!q.trim().is_empty()).then(|| q.trim().to_string())
}

// ---------- file INI ----------

/// Le sezioni di un file INI: sezione → (chiave → valore). Chiavi senza distinzione di maiuscole.
pub fn leggi_ini(testo: &str) -> BTreeMap<String, BTreeMap<String, String>> {
    let mut out: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
    let mut sezione = String::new();
    for riga in testo.trim_start_matches('\u{feff}').lines() {
        let r = riga.trim();
        if r.is_empty() || r.starts_with(';') || r.starts_with('#') {
            continue;
        }
        if let Some(s) = r.strip_prefix('[').and_then(|r| r.strip_suffix(']')) {
            sezione = s.trim().to_ascii_lowercase();
            continue;
        }
        if let Some((k, v)) = r.split_once('=') {
            out.entry(sezione.clone()).or_default().insert(k.trim().to_ascii_lowercase(), v.trim().to_string());
        }
    }
    out
}

fn vero(t: &str) -> bool {
    matches!(t.trim().to_ascii_lowercase().as_str(), "1" | "si" | "sì" | "true" | "yes" | "on")
}

/// Le impostazioni da un file INI nostro (valori mancanti: i predefiniti).
pub fn da_ini(testo: &str) -> Impostazioni {
    let ini = leggi_ini(testo);
    let mut imp = Impostazioni::default();
    let v = |s: &str, k: &str| ini.get(s).and_then(|m| m.get(k)).cloned();
    if let Some(t) = v("siglatura", "modo").and_then(|t| Siglatura::da_codice(&t)) {
        imp.siglatura = t;
    }
    if let Some(t) = v("siglatura", "separatore") {
        imp.separatore_sigla = t;
    }
    if let Some(t) = v("siglatura", "prefisso_assoluto") {
        imp.prefisso_assoluto = t;
    }
    if let Some(n) = v("siglatura", "cifre_foglio").and_then(|t| t.parse().ok()) {
        imp.cifre_foglio = usize::min(n, 4);
    }
    if let Some(t) = v("siglatura", "sigle_uguali_quadri") {
        imp.sigle_uguali_quadri = vero(&t);
    }
    if let Some(t) = v("fili", "numerazione").and_then(|t| NumeriFili::da_codice(&t)) {
        imp.numeri_fili = t;
    }
    if let Some(t) = v("fili", "separatore") {
        imp.separatore_fili = t;
    }
    if let Some(n) = v("fili", "cifre").and_then(|t| t.parse().ok()) {
        imp.cifre_filo = usize::min(n, 6);
    }
    if let Some(t) = v("fili", "prefisso") {
        imp.prefisso_fili = t;
    }
    if let Some(t) = v("fili", "suffisso") {
        imp.suffisso_fili = t;
    }
    if let Some(p) = v("fili", "passo").and_then(|t| t.replace(',', ".").parse::<f64>().ok()).filter(|p| p.is_finite() && *p > 0.0) {
        imp.passo_fili = p;
    }
    if let Some(t) = v("fili", "applica_conduttore") {
        imp.applica_conduttore = vero(&t);
    }
    if let Some(t) = v("fili", "alimentazioni") {
        imp.nomi_alimentazioni = t.split('|').map(str::trim).filter(|s| !s.is_empty()).map(str::to_string).collect();
    }
    for (sezione, c) in [("conduttore", &mut imp.conduttore), ("conduttore_alimentazione", &mut imp.conduttore_alimentazione)] {
        if let Some(m) = ini.get(sezione) {
            let g = |k: &str| m.get(k).cloned().unwrap_or_default();
            *c = Conduttore { sezione: g("sezione"), colore: g("colore"), codice: g("codice"), costruttore: g("costruttore") };
        }
    }
    if let Some(t) = v("correnti", "quadro") {
        imp.quadro = t;
    }
    if let Some(t) = v("correnti", "funzione") {
        imp.funzione = t;
    }
    if let Some(t) = v("correnti", "ubicazione") {
        imp.ubicazione = t;
    }
    if let Some(t) = v("morsetti", "morsettiera_solo_primo") {
        imp.morsettiera_solo_primo = vero(&t);
    }
    imp
}

/// Le impostazioni equivalenti dai settaggi del CAD elettrico d'origine (`Spac_cfg.ini`).
pub fn da_ini_origine(testo: &str) -> Impostazioni {
    let ini = leggi_ini(testo);
    let mut imp = Impostazioni::default();
    let v = |s: &str, k: &str| ini.get(s).and_then(|m| m.get(k)).cloned();
    if let Some(t) = v("siglatura_simboli", "tipo_siglatura").and_then(|t| Siglatura::da_codice(&t)) {
        imp.siglatura = t;
    }
    if let Some(t) = v("siglatura_simboli", "separatore") {
        imp.separatore_sigla = t;
    }
    if v("siglatura_simboli", "abilitazione_prefisso_assoluto").is_some_and(|t| !vero(&t)) {
        imp.prefisso_assoluto = String::new();
    } else if let Some(t) = v("siglatura_simboli", "prefisso_assoluto") {
        imp.prefisso_assoluto = t;
    }
    if let Some(t) = v("preferenze", "sigle_componenti_uguali") {
        imp.sigle_uguali_quadri = vero(&t);
    }
    if let Some(t) = v("numerazione_filo_morsetti", "formattazione_foglio") {
        imp.cifre_foglio = t.trim().len().clamp(1, 4);
    }
    if let Some(t) = v("numerazione_filo_morsetti", "formattazione_numero") {
        imp.cifre_filo = t.trim().len().clamp(1, 6);
    }
    if let Some(t) = v("numerazione_filo_morsetti", "separatore") {
        imp.separatore_fili = t;
    }
    if v("numerazione_filo_morsetti", "numerazione_posizionale").is_some_and(|t| vero(&t)) {
        imp.numeri_fili = NumeriFili::Posizionale;
    }
    if let Some(p) = v("configurazione_fili", "distanza_fili").and_then(|t| t.replace(',', ".").parse::<f64>().ok()).filter(|p| p.is_finite() && *p > 0.0) {
        imp.passo_fili = p;
    }
    if let Some(t) = v("numerazione_filo_morsetti", "nome_morsettiera_solo_primo") {
        imp.morsettiera_solo_primo = vero(&t);
    }
    let nomi: Vec<String> = (0..20).filter_map(|k| v("prefissi_suffissi", &format!("prefsuff_{k}"))).map(|t| t.trim().to_string()).filter(|t| !t.is_empty()).collect();
    if !nomi.is_empty() {
        imp.nomi_alimentazioni = nomi;
    }
    if let Some(m) = ini.get("dati_filo") {
        let g = |k: &str| m.get(k).cloned().unwrap_or_default();
        imp.conduttore = Conduttore { sezione: g("sezione_filo"), colore: g("colore_filo"), codice: g("tipo_filo"), costruttore: g("costruttore_filo") };
    }
    if let Some(m) = ini.get("dati_alimentazione") {
        let g = |k: &str| m.get(k).cloned().unwrap_or_default();
        imp.conduttore_alimentazione = Conduttore { sezione: g("sezione_alimentazione"), colore: g("colore_alimentazione"), codice: g("tipo_alimentazione"), costruttore: g("costruttore_alimentazione") };
    }
    imp
}

/// Le impostazioni in un file INI nostro.
pub fn a_ini(imp: &Impostazioni) -> String {
    let si = |b: bool| if b { "1" } else { "0" };
    let mut out = String::from("; Impostazioni di WireCraft\n");
    out.push_str(&format!(
        "\n[Siglatura]\nModo={}\nSeparatore={}\nPrefisso_Assoluto={}\nCifre_Foglio={}\nSigle_Uguali_Quadri={}\n",
        imp.siglatura.codice(),
        imp.separatore_sigla,
        imp.prefisso_assoluto,
        imp.cifre_foglio,
        si(imp.sigle_uguali_quadri)
    ));
    out.push_str(&format!(
        "\n[Fili]\nNumerazione={}\nSeparatore={}\nCifre={}\nPrefisso={}\nSuffisso={}\nPasso={}\nApplica_Conduttore={}\nAlimentazioni={}\n",
        imp.numeri_fili.codice(),
        imp.separatore_fili,
        imp.cifre_filo,
        imp.prefisso_fili,
        imp.suffisso_fili,
        imp.passo_fili,
        si(imp.applica_conduttore),
        imp.nomi_alimentazioni.join(" | ")
    ));
    for (sezione, c) in [("Conduttore", &imp.conduttore), ("Conduttore_Alimentazione", &imp.conduttore_alimentazione)] {
        out.push_str(&format!("\n[{sezione}]\nSezione={}\nColore={}\nCodice={}\nCostruttore={}\n", c.sezione, c.colore, c.codice, c.costruttore));
    }
    out.push_str(&format!("\n[Correnti]\nQuadro={}\nFunzione={}\nUbicazione={}\n", imp.quadro, imp.funzione, imp.ubicazione));
    out.push_str(&format!("\n[Morsetti]\nMorsettiera_Solo_Primo={}\n", si(imp.morsettiera_solo_primo)));
    out
}

pub fn scrivi(file: &Path, imp: &Impostazioni) -> Result<()> {
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir).map_err(|e| errore(format!("{}: {e}", dir.display())))?;
    }
    std::fs::write(file, a_ini(imp)).map_err(|e| errore(format!("{}: {e}", file.display())))
}

/// Il file delle impostazioni di una commessa.
pub fn file_commessa(cartella: &Path) -> PathBuf {
    cartella.join("CFG").join("WireCraft.ini")
}

/// Carica le impostazioni della commessa in `cartella` (il nostro file, o quello del CAD
/// elettrico d'origine) e le usa; senza file, i predefiniti, che si salveranno nella commessa.
pub fn carica_commessa(cartella: &Path) -> Impostazioni {
    let nostro = file_commessa(cartella);
    let imp = match std::fs::read(&nostro) {
        Ok(b) => da_ini(&String::from_utf8_lossy(&b)),
        Err(_) => match std::fs::read(cartella.join("CFG").join("Spac_cfg.ini")) {
            Ok(b) => da_ini_origine(&b.iter().map(|&c| char::from(c)).collect::<String>()),
            Err(_) => Impostazioni::default(),
        },
    };
    usa(Some(nostro), imp.clone());
    imp
}

static GENERALI: Mutex<Option<PathBuf>> = Mutex::new(None);

/// Il file delle impostazioni generali (quelle fuori dalle commesse), se caricato.
pub fn file_generali() -> Option<PathBuf> {
    GENERALI.lock().ok().and_then(|g| g.clone())
}

/// Carica le impostazioni generali da `file` (senza file, i predefiniti) e le usa.
pub fn carica_generali(file: &Path) -> Impostazioni {
    let imp = std::fs::read(file).map(|b| da_ini(&String::from_utf8_lossy(&b))).unwrap_or_default();
    if let Ok(mut g) = GENERALI.lock() {
        *g = Some(file.to_path_buf());
    }
    usa(Some(file.to_path_buf()), imp.clone());
    imp
}

// ---------- configurazioni con nome ----------

static CONFIGURAZIONI: Mutex<Option<PathBuf>> = Mutex::new(None);

/// Imposta la cartella delle configurazioni con nome: una per cliente o per tipo di lavoro
/// (siglatura, numerazione dei fili, conduttori, morsetti), un file `<nome>.ini` ciascuna.
pub fn imposta_configurazioni(cartella: &Path) {
    if let Ok(mut g) = CONFIGURAZIONI.lock() {
        *g = Some(cartella.to_path_buf());
    }
}

fn cartella_configurazioni() -> Result<PathBuf> {
    CONFIGURAZIONI.lock().ok().and_then(|g| g.clone()).ok_or_else(|| errore("Cartella delle configurazioni non impostata"))
}

/// Il file della configurazione `nome` (un nome valido anche come nome di file).
fn file_configurazione(nome: &str) -> Result<PathBuf> {
    let n = nome.trim();
    if n.is_empty() || n.starts_with('.') || n.chars().any(|c| matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') || c.is_control()) {
        return Err(errore(format!("Nome di configurazione non valido: \"{nome}\"")));
    }
    Ok(cartella_configurazioni()?.join(format!("{n}.ini")))
}

/// I nomi delle configurazioni salvate, in ordine alfabetico.
pub fn configurazioni() -> Vec<String> {
    let Ok(dir) = cartella_configurazioni() else { return Vec::new() };
    let mut v: Vec<String> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("ini")))
        .filter_map(|p| p.file_stem().map(|s| s.to_string_lossy().to_string()))
        .collect();
    v.sort_by_key(|n| n.to_lowercase());
    v
}

/// Le impostazioni di un file INI: nostro o con i settaggi del CAD elettrico d'origine.
pub fn da_file(file: &Path) -> Result<Impostazioni> {
    let b = std::fs::read(file).map_err(|e| errore(format!("{}: {e}", file.display())))?;
    let testo = String::from_utf8(b.clone()).unwrap_or_else(|_| b.iter().map(|&c| char::from(c)).collect());
    Ok(if leggi_ini(&testo).contains_key("siglatura_simboli") { da_ini_origine(&testo) } else { da_ini(&testo) })
}

/// Le impostazioni della configurazione `nome`.
pub fn configurazione(nome: &str) -> Result<Impostazioni> {
    let f = file_configurazione(nome)?;
    if !f.exists() {
        return Err(errore(format!("Configurazione \"{}\" non trovata", nome.trim())));
    }
    da_file(&f)
}

/// Salva le impostazioni in uso come configurazione `nome` (sostituisce quella con lo stesso nome).
pub fn salva_configurazione(nome: &str) -> Result<PathBuf> {
    let f = file_configurazione(nome)?;
    scrivi(&f, &correnti())?;
    Ok(f)
}

/// Usa le impostazioni `nuove` al posto di quelle in uso, tranne quadro, funzione e ubicazione
/// correnti, che sono del progetto aperto; si salvano nel file in uso (la commessa).
pub fn applica(nuove: Impostazioni) -> Result<()> {
    cambia(|i| {
        let (quadro, funzione, ubicazione) = (std::mem::take(&mut i.quadro), std::mem::take(&mut i.funzione), std::mem::take(&mut i.ubicazione));
        *i = Impostazioni { quadro, funzione, ubicazione, ..nuove };
    })
}

pub fn elimina_configurazione(nome: &str) -> Result<()> {
    let f = file_configurazione(nome)?;
    std::fs::remove_file(&f).map_err(|e| errore(format!("{}: {e}", f.display())))
}

// ---------- comando ----------

fn conduttore_json(c: &Conduttore) -> Value {
    json!({"sezione": c.sezione, "colore": c.colore, "codice": c.codice, "costruttore": c.costruttore})
}

fn conduttore_da(v: &Value, base: &Conduttore) -> Conduttore {
    let g = |k: &str, d: &str| v.get(k).and_then(Value::as_str).map_or_else(|| d.to_string(), str::to_string);
    Conduttore { sezione: g("sezione", &base.sezione), colore: g("colore", &base.colore), codice: g("codice", &base.codice), costruttore: g("costruttore", &base.costruttore) }
}

pub fn a_json(imp: &Impostazioni) -> Value {
    json!({
        "siglatura": imp.siglatura.codice(),
        "separatoreSigla": imp.separatore_sigla,
        "prefissoAssoluto": imp.prefisso_assoluto,
        "cifreFoglio": imp.cifre_foglio,
        "sigleUgualiQuadri": imp.sigle_uguali_quadri,
        "numeriFili": imp.numeri_fili.codice(),
        "separatoreFili": imp.separatore_fili,
        "cifreFilo": imp.cifre_filo,
        "prefissoFili": imp.prefisso_fili,
        "suffissoFili": imp.suffisso_fili,
        "passoFili": imp.passo_fili,
        "conduttore": conduttore_json(&imp.conduttore),
        "conduttoreAlimentazione": conduttore_json(&imp.conduttore_alimentazione),
        "applicaConduttore": imp.applica_conduttore,
        "quadro": imp.quadro,
        "funzione": imp.funzione,
        "ubicazione": imp.ubicazione,
        "morsettieraSoloPrimo": imp.morsettiera_solo_primo,
        "nomiAlimentazioni": imp.nomi_alimentazioni,
    })
}

/// Applica i valori di `p` (stesse chiavi di [`a_json`]) alle impostazioni.
pub fn applica_json(imp: &mut Impostazioni, p: &Value) -> Result<()> {
    if let Some(t) = testo_param(p, "siglatura") {
        imp.siglatura = Siglatura::da_codice(&t).ok_or_else(|| errore(format!("siglatura: {}", Siglatura::TUTTE.map(Siglatura::codice).join("|"))))?;
    }
    if let Some(t) = testo_param(p, "numeriFili") {
        imp.numeri_fili = NumeriFili::da_codice(&t).ok_or_else(|| errore(format!("numeriFili: {}", NumeriFili::TUTTI.map(NumeriFili::codice).join("|"))))?;
    }
    for (k, campo) in [
        ("separatoreSigla", &mut imp.separatore_sigla),
        ("prefissoAssoluto", &mut imp.prefisso_assoluto),
        ("separatoreFili", &mut imp.separatore_fili),
        ("prefissoFili", &mut imp.prefisso_fili),
        ("suffissoFili", &mut imp.suffisso_fili),
        ("quadro", &mut imp.quadro),
        ("funzione", &mut imp.funzione),
        ("ubicazione", &mut imp.ubicazione),
    ] {
        if let Some(t) = testo_param(p, k) {
            *campo = t;
        }
    }
    for (k, campo) in [("cifreFoglio", &mut imp.cifre_foglio), ("cifreFilo", &mut imp.cifre_filo)] {
        if let Some(n) = p.get(k).and_then(Value::as_u64) {
            *campo = usize::try_from(n.clamp(1, 6)).unwrap_or(2);
        }
    }
    for (k, campo) in [("sigleUgualiQuadri", &mut imp.sigle_uguali_quadri), ("applicaConduttore", &mut imp.applica_conduttore), ("morsettieraSoloPrimo", &mut imp.morsettiera_solo_primo)] {
        if let Some(b) = p.get(k).and_then(Value::as_bool) {
            *campo = b;
        }
    }
    if let Some(x) = p.get("passoFili").and_then(Value::as_f64).filter(|x| x.is_finite() && *x > 0.0) {
        imp.passo_fili = x;
    }
    if let Some(c) = p.get("conduttore") {
        imp.conduttore = conduttore_da(c, &imp.conduttore);
    }
    if let Some(c) = p.get("conduttoreAlimentazione") {
        imp.conduttore_alimentazione = conduttore_da(c, &imp.conduttore_alimentazione);
    }
    if let Some(a) = p.get("nomiAlimentazioni").and_then(Value::as_array) {
        imp.nomi_alimentazioni = a.iter().filter_map(Value::as_str).map(str::to_string).collect();
    }
    Ok(())
}

fn run_impostazioni(_s: &mut Session, p: &Value) -> Result<Value> {
    let mut imp = correnti();
    applica_json(&mut imp, p)?;
    let cambiate = imp != correnti();
    if cambiate {
        let nuove = imp.clone();
        cambia(|i| *i = nuove)?;
    }
    let mut r = a_json(&imp);
    r["file"] = json!(file().map(|f| f.to_string_lossy().to_string()));
    r["message"] = json!(if cambiate { "Impostazioni aggiornate" } else { "Impostazioni" });
    Ok(r)
}

fn nome_param(p: &Value) -> Result<String> {
    crate::util::testo_param(p, "nome").filter(|n| !n.trim().is_empty()).ok_or_else(|| errore("Indicare il nome della configurazione"))
}

fn run_configurazioni(_s: &mut Session, _p: &Value) -> Result<Value> {
    Ok(json!({"configurazioni": configurazioni(), "cartella": cartella_configurazioni().ok().map(|c| c.to_string_lossy().to_string())}))
}

fn run_salva(_s: &mut Session, p: &Value) -> Result<Value> {
    let nome = nome_param(p)?;
    let f = salva_configurazione(&nome)?;
    Ok(json!({"file": f.to_string_lossy(), "message": format!("Configurazione \"{}\" salvata", nome.trim())}))
}

fn run_usa(_s: &mut Session, p: &Value) -> Result<Value> {
    let nome = nome_param(p)?;
    applica(configurazione(&nome)?)?;
    let mut r = a_json(&correnti());
    r["message"] = json!(format!("Configurazione \"{}\" in uso", nome.trim()));
    Ok(r)
}

fn run_elimina(_s: &mut Session, p: &Value) -> Result<Value> {
    let nome = nome_param(p)?;
    elimina_configurazione(&nome)?;
    Ok(json!({"message": format!("Configurazione \"{}\" eliminata", nome.trim())}))
}

fn run_esporta(_s: &mut Session, p: &Value) -> Result<Value> {
    let f = crate::util::testo_param(p, "file").map(PathBuf::from).ok_or_else(|| errore("Indicare il file"))?;
    scrivi(&f, &correnti())?;
    Ok(json!({"file": f.to_string_lossy(), "message": format!("Impostazioni esportate in {}", f.display())}))
}

/// Importa un file di impostazioni (nostro o del CAD elettrico d'origine): con `nome` lo salva
/// tra le configurazioni, senza lo applica subito.
fn run_importa(_s: &mut Session, p: &Value) -> Result<Value> {
    let f = crate::util::testo_param(p, "file").map(PathBuf::from).ok_or_else(|| errore("Indicare il file"))?;
    let imp = da_file(&f)?;
    match crate::util::testo_param(p, "nome").filter(|n| !n.trim().is_empty()) {
        Some(nome) => {
            let dest = file_configurazione(&nome)?;
            scrivi(&dest, &imp)?;
            Ok(json!({"file": dest.to_string_lossy(), "message": format!("Configurazione \"{}\" importata", nome.trim())}))
        }
        None => {
            applica(imp)?;
            let mut r = a_json(&correnti());
            r["message"] = json!(format!("Impostazioni importate da {}", f.display()));
            Ok(r)
        }
    }
}

pub fn comandi() -> Vec<CommandSpec> {
    let spec = |id, label, menu, params, run| CommandSpec { id, label, menu, shortcut: None, aliases: &[], params, enabled: always, run, undoable: false, interactive: None, transparent: false };
    vec![
        impostazioni_spec(),
        spec("configurazioni", "Configurazioni", &[], "{} → {configurazioni: [nomi], cartella}", run_configurazioni as cadcraft_engine::cmd::Run),
        spec("configurazione.salva", "Salva Configurazione", &["Elettrico", "Configurazioni", "Salva con nome…"], "{nome} — le impostazioni in uso come configurazione con nome", run_salva),
        spec("configurazione.usa", "Usa Configurazione", &["Elettrico", "Configurazioni", "Usa…"], "{nome} — tutto tranne quadro, funzione e ubicazione correnti", run_usa),
        spec("configurazione.elimina", "Elimina Configurazione", &["Elettrico", "Configurazioni", "Elimina…"], "{nome}", run_elimina),
        spec("configurazione.esporta", "Esporta Impostazioni", &["Elettrico", "Configurazioni", "Esporta su file…"], "{file}", run_esporta),
        spec("configurazione.importa", "Importa Impostazioni", &["Elettrico", "Configurazioni", "Importa da file…"], "{file, nome?} — senza nome le applica, con nome le salva tra le configurazioni", run_importa),
    ]
}

fn impostazioni_spec() -> CommandSpec {
    CommandSpec {
        id: "impostazioni",
        label: "Impostazioni",
        menu: &["Elettrico", "Impostazioni…"],
        shortcut: None,
        aliases: &["settaggi"],
        params: "{siglatura?: incrementale|foglio-prefisso-assoluta|foglio-prefisso-relativa|prefisso-foglio-assoluta|prefisso-foglio-relativa|posizionale, separatoreSigla?, prefissoAssoluto?, cifreFoglio?, sigleUgualiQuadri?, numeriFili?: foglio-progressivo|progressivo|posizionale, separatoreFili?, cifreFilo?, prefissoFili?, suffissoFili?, passoFili?, conduttore?: {sezione, colore, codice, costruttore}, conduttoreAlimentazione?, applicaConduttore?, quadro?, funzione?, ubicazione?, morsettieraSoloPrimo?, nomiAlimentazioni?: [..]} → le impostazioni in uso (senza parametri: le legge)",
        enabled: always,
        run: run_impostazioni,
        undoable: false,
        interactive: None,
        transparent: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ini_nostro_avanti_e_indietro() {
        let mut imp = Impostazioni { siglatura: Siglatura::Posizionale, quadro: "+QG".into(), passo_fili: 7.5, ..Impostazioni::default() };
        imp.conduttore.colore = "BU".into();
        assert_eq!(da_ini(&a_ini(&imp)), imp);
    }

    #[test]
    fn settaggi_d_origine() {
        let t = "[Siglatura_Simboli]\nTipo_Siglatura=incre\nSeparatore=.\nAbilitazione_Prefisso_Assoluto=1\nPrefisso_Assoluto=-\n[Configurazione_Fili]\nDistanza_Fili=7.5\n[Prefissi_Suffissi]\nPrefSuff_0=L1 L2 L3\nPrefSuff_1=\n[Numerazione_Filo_Morsetti]\nFormattazione_Foglio=01\nFormattazione_Numero=001\n";
        let imp = da_ini_origine(t);
        assert_eq!(imp.siglatura, Siglatura::Incrementale);
        assert_eq!(imp.passo_fili, 7.5);
        assert_eq!(imp.nomi_alimentazioni, vec!["L1 L2 L3"]);
        assert_eq!((imp.cifre_foglio, imp.cifre_filo), (2, 3));
    }
}
