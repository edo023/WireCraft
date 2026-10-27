//! Calcolo termico del quadro: bilancio tra la potenza dissipata dai componenti e il calore che
//! l'involucro scambia con l'ambiente, con la superficie efficace secondo il tipo di
//! installazione (metodo di IEC/TR 60890 e DIN VDE 0660-500 per armadi chiusi).
//!
//! - Temperatura interna: `Ti = Te + P / (k · A)`.
//! - Oltre la massima ammessa: con l'esterno più fresco basta un ventilatore di portata
//!   `V = 3,1 · (P − k·A·(Tmax − Te)) / (Tmax − Te)` m³/h (a livello del mare); altrimenti serve
//!   un condizionatore di potenza `P + k·A·(Te − Tmax)`.
//! - Con la minima esterna: la resistenza anticondensa `k·A·(Tmin − Te,min)` (a macchina ferma).
//!
//! La potenza di un componente è l'attributo `DISSIPAZIONE` (W) se c'è, altrimenti una stima per
//! tipo di apparecchio, segnalata come tale. Il calcolo è un'uscita come le altre: tabella sul
//! foglio, aggiornata da Aggiorna Liste e Tabelle.

use cadcraft_engine::cmd::has_doc;
use cadcraft_engine::{CommandSpec, Result, Session};
use serde_json::{Value, json};

use crate::util::{blocchi_inseriti, errore, testo_param};
use crate::{impostazioni, layout, morsetti, siglatura, simboli};

/// Fattore del ventilatore (m³·K/(W·h)) fino a 100 m sul livello del mare.
const FATTORE_ARIA: f64 = 3.1;
/// Potenza di un morsetto (W).
const MORSETTO_W: f64 = 0.1;

/// Come è installato l'armadio: decide quali pareti scambiano calore.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Installazione {
    SingoloLibero = 1,
    SingoloParete = 2,
    TestaFilaLibero = 3,
    TestaFilaParete = 4,
    InFilaLibero = 5,
    InFilaParete = 6,
    InFilaPareteTettoCoperto = 7,
}

impl Installazione {
    pub const TUTTE: [Installazione; 7] = [
        Installazione::SingoloLibero,
        Installazione::SingoloParete,
        Installazione::TestaFilaLibero,
        Installazione::TestaFilaParete,
        Installazione::InFilaLibero,
        Installazione::InFilaParete,
        Installazione::InFilaPareteTettoCoperto,
    ];

    pub fn da_numero(n: u64) -> Option<Self> {
        Self::TUTTE.into_iter().find(|i| *i as u64 == n)
    }

    pub fn descrizione(self) -> &'static str {
        match self {
            Installazione::SingoloLibero => "armadio singolo, libero su tutti i lati",
            Installazione::SingoloParete => "armadio singolo, a parete",
            Installazione::TestaFilaLibero => "primo o ultimo di una fila, libero",
            Installazione::TestaFilaParete => "primo o ultimo di una fila, a parete",
            Installazione::InFilaLibero => "in mezzo a una fila, libero",
            Installazione::InFilaParete => "in mezzo a una fila, a parete",
            Installazione::InFilaPareteTettoCoperto => "in mezzo a una fila, a parete, tetto coperto",
        }
    }

    /// Superficie efficace (m²) con larghezza, altezza e profondità in metri.
    pub fn superficie(self, l: f64, a: f64, p: f64) -> f64 {
        match self {
            Installazione::SingoloLibero => 1.8 * a * (l + p) + 1.4 * l * p,
            Installazione::SingoloParete => 1.4 * l * (a + p) + 1.8 * p * a,
            Installazione::TestaFilaLibero => 1.4 * p * (a + l) + 1.8 * l * a,
            Installazione::TestaFilaParete => 1.4 * a * (l + p) + 1.4 * l * p,
            Installazione::InFilaLibero => 1.8 * l * a + 1.4 * l * p + p * a,
            Installazione::InFilaParete => 1.4 * l * (a + p) + p * a,
            Installazione::InFilaPareteTettoCoperto => 1.4 * l * a + 0.7 * l * p + p * a,
        }
    }
}

/// Materiale dell'involucro: coefficiente di scambio k (W/(m²·K)).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Materiale {
    Acciaio,
    Inox,
    Alluminio,
    Plastica,
}

impl Materiale {
    pub const TUTTI: [Materiale; 4] = [Materiale::Acciaio, Materiale::Inox, Materiale::Alluminio, Materiale::Plastica];

    pub fn codice(self) -> &'static str {
        match self {
            Materiale::Acciaio => "acciaio",
            Materiale::Inox => "inox",
            Materiale::Alluminio => "alluminio",
            Materiale::Plastica => "plastica",
        }
    }
    pub fn da_codice(t: &str) -> Option<Self> {
        Self::TUTTI.into_iter().find(|m| m.codice().eq_ignore_ascii_case(t.trim()))
    }
    pub fn descrizione(self) -> &'static str {
        match self {
            Materiale::Acciaio => "lamiera d'acciaio verniciata",
            Materiale::Inox => "acciaio inossidabile",
            Materiale::Alluminio => "alluminio",
            Materiale::Plastica => "poliestere o policarbonato",
        }
    }
    pub fn k(self) -> f64 {
        match self {
            Materiale::Acciaio => 5.5,
            Materiale::Inox => 4.5,
            Materiale::Alluminio => 12.0,
            Materiale::Plastica => 3.5,
        }
    }
}

/// L'armadio e le temperature di progetto.
#[derive(Clone, Debug, PartialEq)]
pub struct Dati {
    /// Misure esterne in mm.
    pub larghezza: f64,
    pub altezza: f64,
    pub profondita: f64,
    pub installazione: Installazione,
    pub materiale: Materiale,
    /// Temperatura esterna massima e interna massima ammessa (°C).
    pub t_esterna: f64,
    pub t_massima: f64,
    /// Temperatura esterna minima e interna minima (anticondensa), se si vuole il riscaldamento.
    pub t_esterna_min: Option<f64>,
    pub t_minima: f64,
}

impl Dati {
    /// Le misure proposte per una piastra `l` × `a` mm: armadio delle stesse misure nominali.
    pub fn per_piastra(l: f64, a: f64) -> Self {
        Dati {
            larghezza: l,
            altezza: a,
            profondita: if a <= 1000.0 { 250.0 } else { 400.0 },
            installazione: if a >= 1400.0 { Installazione::SingoloLibero } else { Installazione::SingoloParete },
            materiale: Materiale::Acciaio,
            t_esterna: 35.0,
            t_massima: 40.0,
            t_esterna_min: None,
            t_minima: 10.0,
        }
    }
}

/// Il risultato del bilancio termico.
#[derive(Clone, Debug, PartialEq)]
pub struct Esito {
    pub superficie: f64,
    pub k: f64,
    pub potenza: f64,
    pub sovratemperatura: f64,
    pub t_interna: f64,
    /// Potenza da togliere (W) per restare alla massima; 0 se non serve.
    pub da_smaltire: f64,
    /// Portata del ventilatore (m³/h), se basta ventilare.
    pub ventilatore: Option<f64>,
    /// Serve un condizionatore: l'esterno è caldo quanto il massimo ammesso o di più.
    pub condizionatore: bool,
    /// Potenza della resistenza anticondensa (W), se chiesta.
    pub riscaldamento: Option<f64>,
}

pub fn calcola(d: &Dati, potenza: f64) -> Esito {
    let (l, a, p) = (d.larghezza / 1000.0, d.altezza / 1000.0, d.profondita / 1000.0);
    let superficie = d.installazione.superficie(l, a, p);
    let k = d.materiale.k();
    let ka = (k * superficie).max(1e-9);
    let sovratemperatura = potenza / ka;
    let t_interna = d.t_esterna + sovratemperatura;
    let margine = d.t_massima - d.t_esterna;
    let (da_smaltire, ventilatore, condizionatore) = if t_interna <= d.t_massima {
        (0.0, None, false)
    } else if margine > 0.0 {
        let q = potenza - ka * margine;
        (q, Some(FATTORE_ARIA * q / margine), false)
    } else {
        (potenza + ka * (-margine), None, true)
    };
    let riscaldamento = d.t_esterna_min.map(|t| (ka * (d.t_minima - t)).max(0.0));
    Esito { superficie, k, potenza, sovratemperatura, t_interna, da_smaltire, ventilatore, condizionatore, riscaldamento }
}

/// La potenza dissipata da un elemento del quadro.
#[derive(Clone, Debug, PartialEq)]
pub struct Voce {
    pub sigla: String,
    pub descrizione: String,
    pub watt: f64,
    /// Stimata per tipo di apparecchio (manca `DISSIPAZIONE`).
    pub stimata: bool,
}

/// Potenza tipica (W) di un apparecchio dal simbolo o dalle lettere della sigla.
fn stima(id: Option<&str>, prefisso: &str, poli: usize) -> f64 {
    let per_id = match id.unwrap_or("") {
        "qf1" | "fu" => Some(1.5),
        "qf2" | "qd2" => Some(3.0),
        "qf3" | "qs3" | "fu3" => Some(4.5),
        "qf4" | "qd4" => Some(6.0),
        "qm3" => Some(6.0),
        "fr3" => Some(4.0),
        "km" => Some(2.5),
        "ka" => Some(0.8),
        "kt" | "ktdis" => Some(1.5),
        "tc" | "g" => Some(12.0),
        "c" | "v" => Some(0.5),
        "xs" | "eh" => Some(0.0),
        _ => None,
    };
    if let Some(w) = per_id {
        return w;
    }
    let poli = poli.max(1) as f64;
    match prefisso.to_ascii_uppercase().chars().next() {
        Some('Q' | 'F') => 1.5 * poli,
        Some('K') => 2.0,
        Some('T' | 'G') => 12.0,
        Some('U') => 40.0,
        Some('A') => 8.0,
        Some('P') => 2.0,
        _ => 1.0,
    }
}

/// Le potenze dissipate nel quadro: i componenti montati nel quadro (non motori, pulsanti,
/// sensori…) e le morsettiere.
pub fn dissipazioni(s: &mut Session, quadro: &str) -> Result<Vec<Voce>> {
    let default = impostazioni::quadro_corrente().unwrap_or_default();
    let senza_quadro = quadro.trim().is_empty() || default.trim().eq_ignore_ascii_case(quadro.trim());
    let mut v: Vec<Voce> = Vec::new();
    for c in siglatura::componenti(s)?.into_iter().filter(|c| c.madre() && c.foglio > 0) {
        let b = &c.blocco;
        if !layout::del_quadro(b.attributo("QUADRO"), quadro, senza_quadro) || v.iter().any(|x| x.sigla == c.nome) {
            continue;
        }
        let sim = simboli::LIBRERIA.iter().find(|x| x.blocco.eq_ignore_ascii_case(&b.nome));
        let prefisso = siglatura::prefisso_di(&c.nome).unwrap_or_default();
        let poli = sim.map_or(3, |x| x.poli.len());
        if layout::misura(sim.map(|x| x.id), &prefisso, poli).is_none() {
            continue;
        }
        let dato = b.attributo("DISSIPAZIONE").trim().replace(',', ".").parse::<f64>().ok().filter(|w| w.is_finite() && *w >= 0.0);
        let descrizione = [b.attributo("FUNZIONE1"), b.attributo("DESCRIZIONE")].into_iter().find(|t| !t.trim().is_empty()).or(sim.map(|x| x.etichetta)).unwrap_or("").to_string();
        v.push(Voce { sigla: c.nome.clone(), descrizione, watt: dato.unwrap_or_else(|| stima(sim.map(|x| x.id), &prefisso, poli)), stimata: dato.is_none() });
    }
    v.sort_by(|a, b| a.sigla.cmp(&b.sigla));
    let mut file: Vec<(String, usize)> = Vec::new();
    for m in morsetti::morsetti(s)? {
        if !layout::del_quadro(m.blocco.attributo("QUADRO"), quadro, senza_quadro) {
            continue;
        }
        match file.iter_mut().find(|(n, _)| n.eq_ignore_ascii_case(&m.morsettiera)) {
            Some(f) => f.1 += 1,
            None => file.push((m.morsettiera.clone(), 1)),
        }
    }
    file.sort();
    v.extend(file.into_iter().map(|(nome, n)| Voce { sigla: nome, descrizione: format!("Morsettiera, {n} morsetti"), watt: n as f64 * MORSETTO_W, stimata: true }));
    Ok(v)
}

/// I dati dell'armadio dai parametri; quelli mancanti dalla piastra del lay-out del quadro (se
/// c'è) o da un armadio 600 × 800.
pub fn dati(s: &mut Session, quadro: &str, p: &Value) -> Result<Dati> {
    let piastra = blocchi_inseriti(s, layout::BLOCCO_PIASTRA)?
        .into_iter()
        .find(|b| quadro.trim().is_empty() || b.attributo("QUADRO").trim().eq_ignore_ascii_case(quadro.trim()))
        .and_then(|b| Some((b.attributo("LARGHEZZA").trim().parse::<f64>().ok()?, b.attributo("ALTEZZA").trim().parse::<f64>().ok()?)));
    let (l, a) = piastra.unwrap_or((600.0, 800.0));
    let mut d = Dati::per_piastra(l, a);
    let num = |k: &str| p.get(k).and_then(|v| v.as_f64().or_else(|| v.as_str().and_then(|t| t.trim().replace(',', ".").parse().ok()))).filter(|x: &f64| x.is_finite());
    if let Some(x) = num("larghezza") {
        d.larghezza = x;
    }
    if let Some(x) = num("altezza") {
        d.altezza = x;
    }
    if let Some(x) = num("profondita") {
        d.profondita = x;
    }
    if [d.larghezza, d.altezza, d.profondita].iter().any(|x| *x < 50.0 || *x > 5000.0) {
        return Err(errore("Misure dell'armadio fuori misura (50–5000 mm)"));
    }
    if let Some(n) = num("installazione") {
        d.installazione = Installazione::da_numero(n as u64).ok_or_else(|| errore("Installazione: da 1 a 7"))?;
    }
    if let Some(m) = testo_param(p, "materiale") {
        d.materiale = Materiale::da_codice(&m).ok_or_else(|| errore(format!("Materiale sconosciuto: {m} (acciaio, inox, alluminio, plastica)")))?;
    }
    if let Some(x) = num("tEsterna") {
        d.t_esterna = x;
    }
    if let Some(x) = num("tMassima") {
        d.t_massima = x;
    }
    d.t_esterna_min = num("tEsternaMin");
    if let Some(x) = num("tMinima") {
        d.t_minima = x;
    }
    Ok(d)
}

fn uno(x: f64) -> String {
    format!("{x:.1}").replace('.', ",")
}

/// Le righe del resoconto: (voce, valore, nota).
pub fn resoconto(quadro: &str, d: &Dati, e: &Esito, voci: &[Voce]) -> Vec<[String; 3]> {
    let mut r: Vec<[String; 3]> = vec![
        ["Quadro".into(), if quadro.is_empty() { "—".into() } else { quadro.to_string() }, String::new()],
        ["Armadio L × A × P".into(), format!("{:.0} × {:.0} × {:.0} mm", d.larghezza, d.altezza, d.profondita), d.installazione.descrizione().into()],
        ["Materiale".into(), format!("k = {} W/m²K", uno(d.materiale.k())), d.materiale.descrizione().into()],
        ["Superficie efficace".into(), format!("{} m²", format!("{:.2}", e.superficie).replace('.', ",")), "IEC/TR 60890".into()],
        ["Potenza dissipata".into(), format!("{} W", uno(e.potenza)), format!("{} voci, {} stimate", voci.len(), voci.iter().filter(|v| v.stimata).count())],
        ["Temperatura esterna".into(), format!("{} °C", uno(d.t_esterna)), String::new()],
        ["Sovratemperatura".into(), format!("{} K", uno(e.sovratemperatura)), String::new()],
        ["Temperatura interna".into(), format!("{} °C", uno(e.t_interna)), format!("massima ammessa {} °C", uno(d.t_massima))],
    ];
    let esito = if e.condizionatore {
        ["Esito".into(), format!("condizionatore {:.0} W", e.da_smaltire.ceil()), "l'esterno è già alla temperatura massima".into()]
    } else if let Some(v) = e.ventilatore {
        ["Esito".into(), format!("ventilatore {:.0} m³/h", v.ceil()), format!("da smaltire {} W", uno(e.da_smaltire))]
    } else {
        ["Esito".into(), "nessun raffreddamento".into(), "la dissipazione naturale basta".into()]
    };
    r.push(esito);
    if let (Some(w), Some(t)) = (e.riscaldamento, d.t_esterna_min) {
        r.push(["Anticondensa".into(), format!("{:.0} W", w.ceil()), format!("esterno {} °C, interno almeno {} °C", uno(t), uno(d.t_minima))]);
    }
    r.push([String::new(), String::new(), String::new()]);
    for v in voci {
        r.push([v.sigla.clone(), format!("{} W", uno(v.watt)), if v.stimata { format!("{} (stima)", v.descrizione) } else { v.descrizione.clone() }]);
    }
    r
}

fn esito_json(d: &Dati, e: &Esito, voci: &[Voce]) -> Value {
    json!({
        "armadio": {"larghezza": d.larghezza, "altezza": d.altezza, "profondita": d.profondita, "installazione": d.installazione as u8, "materiale": d.materiale.codice()},
        "superficie": e.superficie,
        "k": e.k,
        "potenza": e.potenza,
        "sovratemperatura": e.sovratemperatura,
        "tInterna": e.t_interna,
        "tMassima": d.t_massima,
        "daSmaltire": e.da_smaltire,
        "ventilatore": e.ventilatore,
        "condizionatore": e.condizionatore,
        "riscaldamento": e.riscaldamento,
        "voci": voci.iter().map(|v| json!({"sigla": v.sigla, "descrizione": v.descrizione, "watt": v.watt, "stimata": v.stimata})).collect::<Vec<_>>(),
    })
}

/// Calcolo completo per il quadro `quadro` con i parametri `p`.
pub fn calcolo(s: &mut Session, quadro: &str, p: &Value) -> Result<(Dati, Esito, Vec<Voce>)> {
    let d = dati(s, quadro, p)?;
    let voci = dissipazioni(s, quadro)?;
    let e = calcola(&d, voci.iter().map(|v| v.watt).sum());
    Ok((d, e, voci))
}

fn run_calcolo(s: &mut Session, p: &Value) -> Result<Value> {
    let quadro = testo_param(p, "quadro").or_else(impostazioni::quadro_corrente).unwrap_or_default();
    let (d, e, voci) = calcolo(s, &quadro, p)?;
    let mut r = esito_json(&d, &e, &voci);
    r["quadro"] = json!(quadro);
    r["message"] = json!(format!("Quadro {}: {:.0} W, {:.1} °C dentro{}", if quadro.is_empty() { "—" } else { &quadro }, e.potenza, e.t_interna, match (e.condizionatore, e.ventilatore) {
        (true, _) => format!(", serve un condizionatore da {:.0} W", e.da_smaltire.ceil()),
        (false, Some(v)) => format!(", serve un ventilatore da {:.0} m³/h", v.ceil()),
        _ => ", nessun raffreddamento".to_string(),
    }));
    Ok(r)
}

pub fn comandi() -> Vec<CommandSpec> {
    vec![CommandSpec {
        id: "calcolotermico",
        label: "Sovratemperature",
        menu: &["Quadro", "Sovratemperature (calcolo termico)"],
        shortcut: None,
        aliases: &[],
        params: "{quadro?, larghezza?, altezza?, profondita? (mm, default dalla piastra del lay-out), installazione?: 1..7, materiale?: acciaio|inox|alluminio|plastica, tEsterna? (35), tMassima? (40), tEsternaMin?, tMinima? (10)} → {potenza, tInterna, ventilatore, condizionatore, riscaldamento, voci}; sul foglio: USCITA {tipo: termico}",
        enabled: has_doc,
        run: run_calcolo,
        undoable: false,
        interactive: None,
        transparent: false,
    }]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn armadio(l: f64, a: f64, p: f64, i: Installazione) -> Dati {
        Dati { larghezza: l, altezza: a, profondita: p, installazione: i, ..Dati::per_piastra(l, a) }
    }

    #[test]
    fn superfici_secondo_l_installazione() {
        // 600 × 600 × 250 a parete: 1,4·0,6·(0,6+0,25) + 1,8·0,25·0,6
        let a = Installazione::SingoloParete.superficie(0.6, 0.6, 0.25);
        assert!((a - (1.4 * 0.6 * 0.85 + 1.8 * 0.25 * 0.6)).abs() < 1e-12);
        // chi è libero su tutti i lati scambia di più di chi sta in mezzo a una fila col tetto coperto
        let libero = Installazione::SingoloLibero.superficie(0.8, 2.0, 0.6);
        assert!(Installazione::TUTTE.iter().all(|i| i.superficie(0.8, 2.0, 0.6) <= libero + 1e-12));
        assert!(Installazione::InFilaPareteTettoCoperto.superficie(0.8, 2.0, 0.6) < Installazione::InFilaParete.superficie(0.8, 2.0, 0.6));
    }

    #[test]
    fn ventilatore_condizionatore_e_anticondensa() {
        let d = armadio(600.0, 600.0, 250.0, Installazione::SingoloParete);
        let ka = 5.5 * Installazione::SingoloParete.superficie(0.6, 0.6, 0.25);
        // poca potenza: niente raffreddamento
        let e = calcola(&Dati { t_esterna: 30.0, t_massima: 40.0, ..d.clone() }, 20.0);
        assert!((e.t_interna - (30.0 + 20.0 / ka)).abs() < 1e-9);
        assert_eq!((e.ventilatore, e.condizionatore, e.da_smaltire), (None, false, 0.0));
        // tanta potenza con l'esterno più fresco: ventilatore
        let e = calcola(&Dati { t_esterna: 30.0, t_massima: 40.0, ..d.clone() }, 300.0);
        let q = 300.0 - ka * 10.0;
        assert!((e.da_smaltire - q).abs() < 1e-9);
        assert!((e.ventilatore.unwrap_or_default() - 3.1 * q / 10.0).abs() < 1e-9);
        // l'esterno già al massimo: condizionatore
        let e = calcola(&Dati { t_esterna: 40.0, t_massima: 35.0, ..d.clone() }, 100.0);
        assert!(e.condizionatore && e.ventilatore.is_none());
        assert!((e.da_smaltire - (100.0 + ka * 5.0)).abs() < 1e-9);
        // inverno: resistenza per stare a 10 °C con -5 °C fuori
        let e = calcola(&Dati { t_esterna_min: Some(-5.0), t_minima: 10.0, ..d }, 0.0);
        assert!((e.riscaldamento.unwrap_or_default() - ka * 15.0).abs() < 1e-9);
    }

    #[test]
    fn stime_per_tipo() {
        assert_eq!(stima(Some("km"), "KM", 3), 2.5);
        assert_eq!(stima(None, "QF", 2), 3.0);
        assert_eq!(stima(None, "U", 3), 40.0);
        assert_eq!(stima(Some("xs"), "XS", 1), 0.0);
    }
}
