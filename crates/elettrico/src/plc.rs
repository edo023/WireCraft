//! PLC: schede di ingresso e di uscita disegnate a bit, con indirizzo, commento in due lingue e
//! morsetto di ogni bit; numerazione dei bit; elenco I/O per il programmatore (CSV avanti e
//! indietro).
//!
//! Come nei CAD elettrici diffusi la scheda è una madre (`-A12.1`, con codice e costruttore per
//! la distinta) e ogni bit è un figlio `F_PLC` con gli attributi `INPUT`/`OUTPUT` (indirizzo,
//! es. `E0.0`), `COMMENTO1`, `COMMENTO3` (seconda lingua), `PINA1` (morsetto della scheda),
//! `RACK`, `SLOT`. I simboli sono disegni nostri.

use cadcraft_engine::cmd::has_doc;
use cadcraft_engine::{CommandSpec, Result, Session};
use serde_json::{Value, json};

use crate::util::{Inserito, allinea_a_destra, blocchi_definiti, blocchi_inseriti, blocco, colore, errore, esegui, handles, in_gruppo, linea, punto_param, su_layer_zero, testo, testo_param};
use crate::{fili, fogli, siglatura, squadratura};

pub const BLOCCO_INGRESSO: &str = "CE_PLC_IN";
pub const BLOCCO_USCITA: &str = "CE_PLC_OUT";
pub const BLOCCO_SCHEDA: &str = "CE_PLC_SCHEDA";
/// Distanza tra i bit di una scheda (mm).
pub const PASSO_BIT: f64 = 20.0;
/// Lunghezza del bit: il morsetto in alto (ingresso) o in basso (uscita) e il filo verso la scheda.
const LUNGHEZZA: f64 = 10.0;

type P = (f64, f64);

/// Crea i blocchi del bit di ingresso, del bit di uscita e della scheda.
fn crea_blocchi(s: &mut Session) -> Result<()> {
    let definiti = blocchi_definiti(s)?;
    let manca = |n: &str| !definiti.iter().any(|b| b.eq_ignore_ascii_case(n));
    su_layer_zero(s, |s| {
        let attr = |s: &mut Session, h: &mut Vec<String>, tag: &str, def: &str, at: P, alt: f64, nascosto: bool| -> Result<String> {
            let r = handles(&esegui(s, "attdef", json!({"tag": tag, "prompt": tag, "default": def, "at": [at.0, at.1], "height": alt, "invisible": nascosto}))?);
            h.extend(r.iter().cloned());
            Ok(r.into_iter().next().unwrap_or_default())
        };
        for (nome, ingresso) in [(BLOCCO_INGRESSO, true), (BLOCCO_USCITA, false)] {
            if !manca(nome) {
                continue;
            }
            let mut h = Vec::new();
            colore(s, "7")?;
            // il morsetto della scheda (riquadro) in basso, il filo dal punto d'inserimento
            linea(s, &mut h, &[(0.0, 0.0), (0.0, -LUNGHEZZA + 3.0)], false)?;
            linea(s, &mut h, &[(-2.0, -LUNGHEZZA + 3.0), (2.0, -LUNGHEZZA + 3.0), (2.0, -LUNGHEZZA), (-2.0, -LUNGHEZZA)], true)?;
            // freccia verso la scheda (ingresso) o dalla scheda (uscita)
            let (punta, ali) = if ingresso { (-5.5, -4.3) } else { (-2.5, -3.7) };
            linea(s, &mut h, &[(-1.0, ali), (0.0, punta), (1.0, ali)], false)?;
            colore(s, "3")?;
            let pin = attr(s, &mut h, "PINA1", "", (0.0, -LUNGHEZZA + 0.6), 1.8, false)?;
            crate::util::allinea(s, &pin, cadcraft_engine::doc::HAlign::Center)?;
            colore(s, "1")?;
            let indirizzo = attr(s, &mut h, if ingresso { "INPUT" } else { "OUTPUT" }, "", (-1.0, -4.0), 2.5, false)?;
            allinea_a_destra(s, &indirizzo)?;
            colore(s, "4")?;
            // i commenti in verticale sotto la scheda, nella colonna del bit
            let c1 = attr(s, &mut h, "COMMENTO1", "", (-0.4, -LUNGHEZZA - 14.0), 1.8, false)?;
            crate::util::ruota_testo(s, &c1, 90.0, cadcraft_engine::doc::HAlign::Right)?;
            colore(s, "8")?;
            let c3 = attr(s, &mut h, "COMMENTO3", "", (2.2, -LUNGHEZZA - 14.0), 1.6, false)?;
            crate::util::ruota_testo(s, &c3, 90.0, cadcraft_engine::doc::HAlign::Right)?;
            for (tag, def) in [("NOME", ""), ("QUADRO", ""), ("TIPO", if ingresso { "DI" } else { "DO" }), ("RACK", "0"), ("SLOT", ""), ("REG", "")] {
                attr(s, &mut h, tag, def, (0.0, -12.0), 1.0, true)?;
            }
            attr(s, &mut h, "PRES", "F_PLC", (0.0, -12.0), 1.0, true)?;
            blocco(s, nome, h, if ingresso { "Bit di ingresso PLC" } else { "Bit di uscita PLC" })?;
        }
        Ok(())
    })
}

/// Il blocco della scheda larga `n` bit (`CE_PLC_SCHEDA8`, `CE_PLC_SCHEDA16`…): un rettangolo
/// sotto i morsetti dei bit, con la sigla a sinistra; il punto d'inserimento è sotto il primo bit.
fn crea_scheda(s: &mut Session, n: u32) -> Result<String> {
    let nome = format!("{BLOCCO_SCHEDA}{n}");
    if blocchi_definiti(s)?.iter().any(|b| b.eq_ignore_ascii_case(&nome)) {
        return Ok(nome);
    }
    let destra = PASSO_BIT * f64::from(n) - PASSO_BIT / 2.0;
    su_layer_zero(s, |s| {
        let mut h = Vec::new();
        colore(s, "7")?;
        linea(s, &mut h, &[(-PASSO_BIT / 2.0, 0.0), (destra, 0.0), (destra, -12.0), (-PASSO_BIT / 2.0, -12.0)], true)?;
        colore(s, "4")?;
        let attr = |s: &mut Session, h: &mut Vec<String>, tag: &str, def: &str, at: P, alt: f64, nascosto: bool| -> Result<String> {
            let r = handles(&esegui(s, "attdef", json!({"tag": tag, "prompt": tag, "default": def, "at": [at.0, at.1], "height": alt, "invisible": nascosto}))?);
            h.extend(r.iter().cloned());
            Ok(r.into_iter().next().unwrap_or_default())
        };
        let sigla = attr(s, &mut h, "NOME", "", (-PASSO_BIT / 2.0 - 1.5, -4.0), 2.5, false)?;
        allinea_a_destra(s, &sigla)?;
        attr(s, &mut h, "DESCRIZIONE", "", (-PASSO_BIT / 2.0 + 2.0, -8.0), 2.0, false)?;
        for (tag, def) in [("QUADRO", ""), ("COSTRUTTORE", ""), ("TIPO", ""), ("FUNZIONE1", ""), ("FUNZIONE2", ""), ("CODICE", ""), ("RACK", "0"), ("SLOT", "")] {
            attr(s, &mut h, tag, def, (0.0, -14.0), 1.0, true)?;
        }
        attr(s, &mut h, "PRES", "M", (0.0, -14.0), 1.0, true)?;
        testo(s, &mut h, (destra - 2.0, -4.0), "PLC", 2.5, "MR")?;
        blocco(s, &nome, h, "Scheda PLC")
    })?;
    Ok(nome)
}

/// Un indirizzo: prefisso, byte (o parola) e bit; base 8 per i bit (come le schede a 8 bit).
#[derive(Clone, Debug, PartialEq)]
pub struct Indirizzo {
    pub prefisso: String,
    pub byte: u32,
    pub bit: u32,
    /// Bit per byte (8 di solito, 16 per certe schede).
    pub base: u32,
    pub separatore: String,
}

impl Indirizzo {
    /// `E0.0`, `I1.7`, `%I0.3`…
    pub fn leggi(t: &str, base: u32) -> Option<Self> {
        let t = t.trim();
        let i = t.find(|c: char| c.is_ascii_digit())?;
        let (prefisso, resto) = t.split_at(i);
        let j = resto.find(|c: char| !c.is_ascii_digit()).unwrap_or(resto.len());
        let (byte, resto) = resto.split_at(j);
        let sep: String = resto.chars().take_while(|c| !c.is_ascii_digit()).collect();
        let bit = resto.get(sep.len()..).unwrap_or("");
        Some(Indirizzo { prefisso: prefisso.into(), byte: byte.parse().ok()?, bit: if bit.is_empty() { 0 } else { bit.parse().ok()? }, base: base.max(1), separatore: if sep.is_empty() { ".".into() } else { sep } })
    }
    pub fn testo(&self) -> String {
        format!("{}{}{}{}", self.prefisso, self.byte, self.separatore, self.bit)
    }
    /// L'indirizzo seguente: dopo l'ultimo bit del byte si passa al byte dopo.
    pub fn seguente(&self) -> Self {
        let mut n = self.clone();
        n.bit += 1;
        if n.bit >= n.base {
            n.bit = 0;
            n.byte += 1;
        }
        n
    }
}

/// Un bit disegnato.
#[derive(Clone, Debug)]
pub struct Bit {
    pub blocco: Inserito,
    pub ingresso: bool,
    pub foglio: u32,
}

impl Bit {
    pub fn indirizzo(&self) -> &str {
        self.blocco.attributo(if self.ingresso { "INPUT" } else { "OUTPUT" })
    }
}

/// I bit dei PLC nel disegno (i nostri e quelli dei progetti esistenti, `INP*`/`OUT*`).
pub fn bit(s: &mut Session) -> Result<Vec<Bit>> {
    let mut v: Vec<Bit> = blocchi_inseriti(s, "*")?
        .into_iter()
        .filter(|b| b.attributo("PRES").eq_ignore_ascii_case("F_PLC"))
        .map(|b| {
            let ingresso = !b.attributo("INPUT").is_empty() || b.nome.eq_ignore_ascii_case(BLOCCO_INGRESSO) || b.nome.to_ascii_uppercase().starts_with("INP");
            Bit { foglio: fogli::numero_da_layer(&b.layer).unwrap_or(0), blocco: b, ingresso }
        })
        .collect();
    v.sort_by_key(|b| (b.foglio, (b.blocco.pos.0 * 100.0) as i64));
    Ok(v)
}

/// Disegna una scheda con `n` bit da `at` (il primo bit), verso destra, passo [`PASSO_BIT`]:
/// la scheda (madre) sotto i bit, ogni bit col suo indirizzo e il suo morsetto. I fili che
/// arrivano sui bit vengono interrotti al morsetto del bit.
pub fn scheda(s: &mut Session, at: P, n: u32, ingresso: bool, primo: &Indirizzo, primo_morsetto: u32, p: &Value) -> Result<(String, Vec<String>)> {
    if n == 0 || n > 64 {
        return Err(errore("Da 1 a 64 bit per scheda"));
    }
    crea_blocchi(s)?;
    let foglio = fogli::corrente(s)?.map_or(0, |f| f.numero);
    let componenti = siglatura::componenti(s)?;
    let sigla = testo_param(p, "sigla").filter(|t| !t.trim().is_empty()).unwrap_or_else(|| siglatura::nuova_sigla(&componenti, "A", foglio, at));
    let quadro = testo_param(p, "quadro").or_else(crate::impostazioni::quadro_corrente).unwrap_or_default();
    let nome_scheda = crea_scheda(s, n)?;
    let r = esegui(
        s,
        "insert",
        json!({
            "name": nome_scheda, "at": [at.0, at.1 - LUNGHEZZA],
            "attribs": {"NOME": sigla, "QUADRO": quadro, "PRES": "M", "DESCRIZIONE": testo_param(p, "descrizione").unwrap_or_default(),
                        "COSTRUTTORE": testo_param(p, "costruttore").unwrap_or_default(), "TIPO": testo_param(p, "codice").unwrap_or_default(),
                        "RACK": testo_param(p, "rack").unwrap_or_else(|| "0".into()), "SLOT": testo_param(p, "slot").unwrap_or_default()},
        }),
    )?;
    handles(&r).into_iter().next().ok_or_else(|| errore("scheda non inserita"))?;
    let commenti: Vec<(String, String)> = p
        .get("commenti")
        .and_then(Value::as_array)
        .map(|a| a.iter().map(|c| (c.get(0).and_then(Value::as_str).unwrap_or("").to_string(), c.get(1).and_then(Value::as_str).unwrap_or("").to_string())).collect())
        .unwrap_or_default();
    let mut indirizzo = primo.clone();
    let mut bits = Vec::new();
    for k in 0..n {
        let q = (at.0 + PASSO_BIT * f64::from(k), at.1);
        fili::spezza(s, q, (q.0, q.1 - LUNGHEZZA))?;
        let (c1, c3) = commenti.get(k as usize).cloned().unwrap_or_default();
        let r = esegui(
            s,
            "insert",
            json!({
                "name": if ingresso { BLOCCO_INGRESSO } else { BLOCCO_USCITA }, "at": [q.0, q.1],
                "attribs": {
                    "NOME": sigla, "QUADRO": quadro, "PRES": "F_PLC", (if ingresso { "INPUT" } else { "OUTPUT" }): indirizzo.testo(),
                    "PINA1": (primo_morsetto + k).to_string(), "COMMENTO1": c1, "COMMENTO3": c3, "RACK": testo_param(p, "rack").unwrap_or_else(|| "0".into()),
                    "SLOT": testo_param(p, "slot").unwrap_or_default(), "TIPO": if ingresso { "DI" } else { "DO" },
                },
            }),
        )?;
        bits.extend(handles(&r));
        indirizzo = indirizzo.seguente();
    }
    aggiorna_reg(s)?;
    Ok((sigla, bits))
}

/// Scrive in REG di ogni bit dove sta la sua scheda.
pub fn aggiorna_reg(s: &mut Session) -> Result<()> {
    let schede: Vec<Inserito> = blocchi_inseriti(s, "*")?.into_iter().filter(|b| b.attributo("PRES").eq_ignore_ascii_case("M") && b.nome.to_ascii_uppercase().starts_with(BLOCCO_SCHEDA)).collect();
    for b in bit(s)? {
        let madre = schede.iter().find(|m| m.attributo("NOME").eq_ignore_ascii_case(b.blocco.attributo("NOME")));
        let reg = madre.map(|m| squadratura::riferimento(fogli::numero_da_layer(&m.layer).unwrap_or(0), m.pos.0, m.pos.1)).unwrap_or_default();
        if b.blocco.nome.to_ascii_uppercase().starts_with("CE_PLC") && b.blocco.attributo("REG") != reg {
            esegui(s, "attedit", json!({"handle": b.blocco.handle, "values": {"REG": reg}}))?;
        }
    }
    Ok(())
}

/// Rinumera gli indirizzi dei bit della scheda `sigla` (in ordine da sinistra) da `primo`.
pub fn numera_bit(s: &mut Session, sigla: &str, primo: &Indirizzo) -> Result<Vec<String>> {
    let mut ind = primo.clone();
    let mut out = Vec::new();
    for b in bit(s)?.into_iter().filter(|b| b.blocco.attributo("NOME").eq_ignore_ascii_case(sigla)) {
        let tag = if b.ingresso { "INPUT" } else { "OUTPUT" };
        esegui(s, "attedit", json!({"handle": b.blocco.handle, "values": {tag: ind.testo()}}))?;
        out.push(ind.testo());
        ind = ind.seguente();
    }
    if out.is_empty() {
        return Err(errore(format!("La scheda {sigla} non ha bit")));
    }
    Ok(out)
}

pub const INTESTAZIONE_IO: [&str; 8] = ["Indirizzo", "Tipo", "Commento", "Commento (2a lingua)", "Scheda", "Morsetto", "Quadro", "Posizione"];

/// L'elenco I/O: una riga per bit.
pub fn elenco_io(s: &mut Session) -> Result<Vec<Vec<String>>> {
    Ok(bit(s)?
        .iter()
        .map(|b| {
            vec![
                b.indirizzo().to_string(),
                if b.ingresso { "I".into() } else { "O".into() },
                b.blocco.attributo("COMMENTO1").into(),
                b.blocco.attributo("COMMENTO3").into(),
                b.blocco.attributo("NOME").into(),
                b.blocco.attributo("PINA1").into(),
                b.blocco.attributo("QUADRO").into(),
                squadratura::riferimento(b.foglio, b.blocco.pos.0, b.blocco.pos.1),
            ]
        })
        .collect())
}

/// Importa i commenti dei bit da un CSV (indirizzo; commento; commento 2a lingua): i bit con
/// lo stesso indirizzo prendono i commenti. Restituisce i bit aggiornati.
pub fn importa_commenti(s: &mut Session, testo: &str) -> Result<usize> {
    let righe: Vec<Vec<String>> = testo.lines().skip(1).map(|r| crate::archivio::campi(r, ';')).collect();
    let mut n = 0;
    for b in bit(s)? {
        let Some(r) = righe.iter().find(|r| r.first().is_some_and(|i| i.eq_ignore_ascii_case(b.indirizzo()))) else { continue };
        // il CSV esportato ha le colonne di INTESTAZIONE_IO; uno minimo: indirizzo; commento; commento2
        let (c1, c3) = if r.len() >= 8 { (r.get(2), r.get(3)) } else { (r.get(1), r.get(2)) };
        esegui(s, "attedit", json!({"handle": b.blocco.handle, "values": {"COMMENTO1": c1.cloned().unwrap_or_default(), "COMMENTO3": c3.cloned().unwrap_or_default()}}))?;
        n += 1;
    }
    Ok(n)
}

fn indirizzo_param(p: &Value, ingresso: bool) -> Result<Indirizzo> {
    let base = p.get("base").and_then(Value::as_u64).map_or(8, |b| u32::try_from(b).unwrap_or(8));
    let t = testo_param(p, "indirizzo").unwrap_or_else(|| if ingresso { "E0.0".into() } else { "A0.0".into() });
    Indirizzo::leggi(&t, base).ok_or_else(|| errore(format!("Indirizzo non valido: {t} (es. E0.0, %I0.0, I:000/00)")))
}

fn run_scheda(s: &mut Session, p: &Value) -> Result<Value> {
    let at = punto_param(p, "at").ok_or_else(|| errore("Indicare il punto del primo bit (at)"))?;
    let ingresso = !testo_param(p, "tipo").is_some_and(|t| t.to_ascii_lowercase().starts_with('u') || t.to_ascii_lowercase().starts_with('o'));
    let n = p.get("bit").and_then(Value::as_u64).map_or(8, |n| u32::try_from(n).unwrap_or(8));
    let primo = indirizzo_param(p, ingresso)?;
    let morsetto = p.get("morsetto").and_then(Value::as_u64).map_or(1, |m| u32::try_from(m).unwrap_or(1));
    in_gruppo(s, "Scheda PLC", |s| {
        let (sigla, bits) = scheda(s, at, n, ingresso, &primo, morsetto, p)?;
        Ok(json!({"sigla": sigla, "bit": bits, "message": format!("Scheda {sigla}: {n} {} da {}", if ingresso { "ingressi" } else { "uscite" }, primo.testo())}))
    })
}

fn run_numera(s: &mut Session, p: &Value) -> Result<Value> {
    let sigla = testo_param(p, "sigla").ok_or_else(|| errore("Indicare la sigla della scheda"))?;
    let ingresso = bit(s)?.iter().find(|b| b.blocco.attributo("NOME").eq_ignore_ascii_case(&sigla)).is_none_or(|b| b.ingresso);
    let primo = indirizzo_param(p, ingresso)?;
    in_gruppo(s, "Numera bit", |s| {
        let v = numera_bit(s, &sigla, &primo)?;
        Ok(json!({"indirizzi": v, "message": format!("Scheda {sigla}: {} … {}", v.first().cloned().unwrap_or_default(), v.last().cloned().unwrap_or_default())}))
    })
}

fn run_elenco(s: &mut Session, p: &Value) -> Result<Value> {
    let righe = elenco_io(s)?;
    if let Some(f) = testo_param(p, "file") {
        crate::morsetti::scrivi_csv(&f, &INTESTAZIONE_IO, &righe)?;
    }
    Ok(json!({"intestazione": INTESTAZIONE_IO, "righe": righe, "message": format!("{} bit", righe.len())}))
}

fn run_commento(s: &mut Session, p: &Value) -> Result<Value> {
    let indirizzo = testo_param(p, "indirizzo").ok_or_else(|| errore("Indicare l'indirizzo del bit"))?;
    in_gruppo(s, "Commento bit", |s| {
        let b = bit(s)?.into_iter().find(|b| b.indirizzo().eq_ignore_ascii_case(&indirizzo)).ok_or_else(|| errore(format!("Nessun bit {indirizzo}")))?;
        let mut v = serde_json::Map::new();
        for (k, tag) in [("commento", "COMMENTO1"), ("commento2", "COMMENTO3")] {
            if let Some(t) = testo_param(p, k) {
                v.insert(tag.into(), json!(t));
            }
        }
        esegui(s, "attedit", json!({"handle": b.blocco.handle, "values": v}))?;
        Ok(json!({"message": format!("Bit {indirizzo} aggiornato")}))
    })
}

fn run_importa(s: &mut Session, p: &Value) -> Result<Value> {
    let f = testo_param(p, "file").ok_or_else(|| errore("Indicare il file CSV"))?;
    let b = std::fs::read(&f).map_err(|e| errore(format!("{f}: {e}")))?;
    let testo = String::from_utf8(b.clone()).unwrap_or_else(|_| b.iter().map(|&c| char::from(c)).collect());
    in_gruppo(s, "Importa commenti PLC", |s| {
        let n = importa_commenti(s, &testo)?;
        Ok(json!({"bit": n, "message": format!("Commenti importati su {n} bit")}))
    })
}

/// Parametri della finestra per la prossima scheda posata in modo interattivo.
static PREPARATI: std::sync::Mutex<Option<Value>> = std::sync::Mutex::new(None);

/// I parametri (tipo, bit, indirizzo, sigla, materiali…) della prossima scheda da posare.
pub fn prepara(p: Value) {
    if let Ok(mut g) = PREPARATI.lock() {
        *g = Some(p);
    }
}

/// SCHEDA I/O interattiva: il punto del primo bit, con i parametri preparati dalla finestra.
struct PosaScheda {
    parametri: Value,
}

impl cadcraft_engine::Interactive for PosaScheda {
    fn name(&self) -> &'static str {
        "plc.scheda"
    }
    fn prompt(&self, _s: &Session) -> cadcraft_engine::Prompt {
        cadcraft_engine::Prompt::new("Punto del primo bit della scheda", cadcraft_engine::Accept::POINT)
    }
    fn input(&mut self, s: &mut Session, i: cadcraft_engine::Input) -> Result<cadcraft_engine::Step> {
        use cadcraft_engine::{Input, Step};
        match i {
            Input::Point(q) => {
                let mut p = self.parametri.clone();
                if !p.is_object() {
                    p = json!({});
                }
                p["at"] = json!([q.x, q.y]);
                let r = s.execute("plc.scheda", &p)?;
                if let Some(m) = r.get("message").and_then(Value::as_str) {
                    s.echo(m.to_string());
                }
                Ok(Step::Done)
            }
            Input::Cancel => Ok(Step::Cancel),
            Input::Enter => Ok(Step::Done),
            _ => Ok(Step::Continue),
        }
    }
    fn preview(&self, _s: &Session, c: cadcraft_engine::geom::Vec2) -> Vec<cadcraft_engine::doc::EntityKind> {
        use cadcraft_engine::doc::{EntityKind, Line};
        use cadcraft_engine::geom::Vec3;
        let n = self.parametri.get("bit").and_then(Value::as_u64).unwrap_or(8) as f64;
        let l = |a: P, b: P| EntityKind::Line(Line { a: Vec3::new(a.0, a.1, 0.0), b: Vec3::new(b.0, b.1, 0.0) });
        let (x0, x1, y0, y1) = (c.x - PASSO_BIT / 2.0, c.x + PASSO_BIT * n - PASSO_BIT / 2.0, c.y - LUNGHEZZA, c.y - LUNGHEZZA - 12.0);
        let mut v = vec![l((x0, y0), (x1, y0)), l((x1, y0), (x1, y1)), l((x1, y1), (x0, y1)), l((x0, y1), (x0, y0))];
        for k in 0..n as u32 {
            let x = c.x + PASSO_BIT * f64::from(k);
            v.push(l((x, c.y), (x, y0)));
        }
        v
    }
}

fn fabbrica_scheda(_s: &Session) -> Result<Box<dyn cadcraft_engine::Interactive>> {
    let parametri = PREPARATI.lock().ok().and_then(|mut g| g.take()).unwrap_or_else(|| json!({"bit": 8}));
    Ok(Box::new(PosaScheda { parametri }))
}

pub fn comandi() -> Vec<CommandSpec> {
    let spec = |id, label, menu, params, run| CommandSpec { id, label, menu, shortcut: None, aliases: &[], params, enabled: has_doc, run, undoable: false, interactive: None, transparent: false };
    vec![
        CommandSpec { interactive: Some(fabbrica_scheda), ..spec(
            "plc.scheda",
            "Scheda I/O PLC",
            &["UTIL", "Gestione PLC", "Scheda I/O…"],
            "{at: [x,y] (primo bit), tipo?: ingressi|uscite, bit?: 8|16 (default 8), indirizzo?: \"E0.0\", base?: bit per byte (8), morsetto?: primo morsetto (1), sigla?, quadro?, descrizione?, costruttore?, codice?: (per la distinta), rack?, slot?, commenti?: [[c1, c2]..]} → {sigla, bit}",
            run_scheda as cadcraft_engine::cmd::Run,
        ) },
        spec("plc.numerabit", "Numera BIT", &["UTIL", "Gestione PLC", "Numera BIT…"], "{sigla: scheda, indirizzo: primo (\"E2.0\"), base?: 8} → {indirizzi}", run_numera),
        spec("plc.elenco", "Gestione BIT (elenco I/O)", &["UTIL", "Gestione PLC", "Elenco I/O (CSV)"], "{file?: .csv} → {intestazione, righe}", run_elenco),
        spec("plc.commento", "Commento di un bit", &[], "{indirizzo, commento?, commento2?}", run_commento),
        spec("plc.importa", "Importa commenti I/O", &["UTIL", "Gestione PLC", "Importa commenti (CSV)"], "{file: .csv (indirizzo; commento; commento 2a lingua)}", run_importa),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indirizzi() {
        let i = Indirizzo::leggi("E0.6", 8).unwrap_or_else(|| Indirizzo { prefisso: String::new(), byte: 0, bit: 0, base: 8, separatore: ".".into() });
        assert_eq!(i.testo(), "E0.6");
        assert_eq!(i.seguente().testo(), "E0.7");
        assert_eq!(i.seguente().seguente().testo(), "E1.0");
        let j = Indirizzo::leggi("%I3.15", 16).unwrap_or_else(|| i.clone());
        assert_eq!((j.prefisso.as_str(), j.byte, j.bit), ("%I", 3, 15));
        assert_eq!(j.seguente().testo(), "%I4.0");
        assert_eq!(Indirizzo::leggi("I:000/00", 16).map(|x| x.seguente().testo()), Some("I:0/1".into()));
        assert!(Indirizzo::leggi("ABC", 8).is_none());
    }
}
