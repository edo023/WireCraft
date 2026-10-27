//! Designazione dei componenti secondo la CEI EN 81346: funzione (`=TR1`), ubicazione (`++ZC`)
//! e quadro (`+QG`), negli attributi `FUNZIONE_IMPIANTO`, `UBICAZIONE` e `QUADRO` delle madri,
//! dei morsetti e dei connettori (i contatti seguono la loro madre).
//!
//! - **Box**: un riquadro tratto-punto sul foglio che dà funzione, ubicazione o quadro a tutto
//!   quello che racchiude. Fra box dello stesso tipo uno dentro l'altro vale il più piccolo. Si
//!   riapplicano con `BOX.APPLICA` (e con Aggiorna Liste e Tabelle) dopo aver spostato qualcosa.
//! - **Etichette**: sopra la sigla, i livelli scelti (`=TR1++ZC+QG`); con la regola
//!   "discordanti" solo quelli diversi dal box che contiene il componente, che li dice già.

use cadcraft_engine::cmd::has_doc;
use cadcraft_engine::doc::{EntityKind, Handle};
use cadcraft_engine::geom::Vec2;
use cadcraft_engine::{Accept, CommandSpec, Input, Interactive, Prompt, Result, Session, Step};
use serde_json::{Value, json};

use crate::util::{Inserito, allinea_a_destra, blocchi_definiti, blocchi_inseriti, blocco, colore, errore, esegui, handles, imposta_attributi, in_gruppo, punto_param, su_layer_zero, testo_param};
use crate::{fogli, impostazioni, morsetti, siglatura};

type P = (f64, f64);

const PREFISSO_BOX: &str = "CE_BOX_";
pub const BLOCCO_ETICHETTA: &str = "CE_DESIGNAZIONE";

/// Il livello di designazione assegnato da un box.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Livello {
    Funzione,
    Ubicazione,
    Quadro,
}

impl Livello {
    pub const TUTTI: [Livello; 3] = [Livello::Funzione, Livello::Ubicazione, Livello::Quadro];

    pub fn codice(self) -> &'static str {
        match self {
            Livello::Funzione => "funzione",
            Livello::Ubicazione => "ubicazione",
            Livello::Quadro => "quadro",
        }
    }

    pub fn da_codice(t: &str) -> Option<Self> {
        match t.trim().to_ascii_lowercase().as_str() {
            "funzione" | "=" => Some(Livello::Funzione),
            "ubicazione" | "++" => Some(Livello::Ubicazione),
            "quadro" | "+" => Some(Livello::Quadro),
            _ => None,
        }
    }

    /// Il segno davanti al valore.
    pub fn segno(self) -> &'static str {
        match self {
            Livello::Funzione => "=",
            Livello::Ubicazione => "++",
            Livello::Quadro => "+",
        }
    }

    pub fn tag(self) -> &'static str {
        match self {
            Livello::Funzione => "FUNZIONE_IMPIANTO",
            Livello::Ubicazione => "UBICAZIONE",
            Livello::Quadro => "QUADRO",
        }
    }

    /// Colore del riquadro: si distinguono a colpo d'occhio.
    fn colore(self) -> &'static str {
        match self {
            Livello::Funzione => "5",
            Livello::Ubicazione => "30",
            Livello::Quadro => "6",
        }
    }

    /// Il valore con il suo segno davanti (`QG` → `+QG`; uno già scritto col segno resta così).
    pub fn normalizza(self, valore: &str) -> String {
        let v = valore.trim().trim_start_matches(['=', '+']).trim();
        if v.is_empty() { String::new() } else { format!("{}{v}", self.segno()) }
    }
}

/// Un box disegnato.
#[derive(Clone, Debug, PartialEq)]
pub struct Box81346 {
    pub handle: String,
    pub livello: Livello,
    pub valore: String,
    pub layer: String,
    pub min: P,
    pub max: P,
}

impl Box81346 {
    fn da_blocco(b: &Inserito) -> Option<Self> {
        let misura = b.nome.to_ascii_uppercase().strip_prefix(PREFISSO_BOX)?.to_string();
        let (w, h) = misura.split_once('X')?;
        let (w, h) = (w.parse::<f64>().ok()?, h.parse::<f64>().ok()?);
        Some(Box81346 {
            handle: b.handle.clone(),
            livello: Livello::da_codice(b.attributo("TIPO"))?,
            valore: b.attributo("VALORE").trim().to_string(),
            layer: b.layer.clone(),
            min: b.pos,
            max: (b.pos.0 + w, b.pos.1 + h),
        })
    }

    pub fn contiene(&self, layer: &str, p: P) -> bool {
        self.layer.eq_ignore_ascii_case(layer) && p.0 >= self.min.0 && p.0 <= self.max.0 && p.1 >= self.min.1 && p.1 <= self.max.1
    }

    fn area(&self) -> f64 {
        (self.max.0 - self.min.0) * (self.max.1 - self.min.1)
    }
}

pub fn boxes(s: &mut Session) -> Result<Vec<Box81346>> {
    Ok(blocchi_inseriti(s, &format!("{PREFISSO_BOX}*"))?.iter().filter_map(Box81346::da_blocco).collect())
}

/// Il box più piccolo di quel livello che contiene il punto.
fn box_di<'a>(tutti: &'a [Box81346], livello: Livello, layer: &str, p: P) -> Option<&'a Box81346> {
    tutti.iter().filter(|b| b.livello == livello && b.contiene(layer, p)).min_by(|a, b| a.area().total_cmp(&b.area()))
}

/// Il blocco del riquadro largo `w` e alto `h`: bordo tratto-punto (colore dal blocco inserito),
/// il valore visibile nell'angolo in alto a sinistra, tipo e descrizione nascosti.
fn crea_blocco_box(s: &mut Session, w: f64, h: f64) -> Result<String> {
    let nome = format!("{PREFISSO_BOX}{w:.0}X{h:.0}");
    if blocchi_definiti(s)?.iter().any(|b| b.eq_ignore_ascii_case(&nome)) {
        return Ok(nome);
    }
    su_layer_zero(s, |s| {
        colore(s, "BYBLOCK")?;
        esegui(s, "linetype", json!({"load": "DASHDOT"}))?;
        esegui(s, "linetype", json!({"current": "DASHDOT"}))?;
        esegui(s, "setvar", json!({"name": "CELTSCALE", "value": 0.4}))?;
        let bordo = esegui(s, "pline", json!({"vertices": [[0.0, 0.0], [w, 0.0], [w, h], [0.0, h]], "closed": true}));
        esegui(s, "linetype", json!({"current": "BYLAYER"}))?;
        esegui(s, "setvar", json!({"name": "CELTSCALE", "value": 1.0}))?;
        let mut v = handles(&bordo?);
        let att = |s: &mut Session, v: &mut Vec<String>, tag: &str, at: P, alt: f64, nascosto: bool| -> Result<()> {
            v.extend(handles(&esegui(s, "attdef", json!({"tag": tag, "prompt": tag, "default": "", "at": [at.0, at.1], "height": alt, "invisible": nascosto}))?));
            Ok(())
        };
        att(s, &mut v, "VALORE", (2.0, h - 4.0), 2.5, false)?;
        att(s, &mut v, "TIPO", (2.0, h - 7.0), 1.5, true)?;
        att(s, &mut v, "DESCRIZIONE", (2.0, h - 9.0), 1.5, true)?;
        blocco(s, &nome, v, "Box CEI EN 81346")
    })?;
    Ok(nome)
}

/// Disegna un box sul foglio corrente tra gli angoli `da` e `a` e lo applica.
pub fn disegna_box(s: &mut Session, livello: Livello, valore: &str, da: P, a: P, descrizione: &str) -> Result<(String, usize)> {
    let valore = livello.normalizza(valore);
    if valore.is_empty() {
        return Err(errore(format!("Indicare {} del box", match livello {
            Livello::Funzione => "la funzione",
            Livello::Ubicazione => "l'ubicazione",
            Livello::Quadro => "il quadro",
        })));
    }
    let min = (da.0.min(a.0), da.1.min(a.1));
    let (w, h) = ((da.0 - a.0).abs().round(), (da.1 - a.1).abs().round());
    if w < 10.0 || h < 10.0 {
        return Err(errore("Il box è troppo piccolo: almeno 10 × 10 mm"));
    }
    in_gruppo(s, "Box", |s| {
        let nome = crea_blocco_box(s, w, h)?;
        colore(s, livello.colore())?;
        let r = esegui(s, "insert", json!({"name": nome, "at": [min.0, min.1], "attribs": {"VALORE": valore, "TIPO": livello.codice(), "DESCRIZIONE": descrizione}}));
        colore(s, "BYLAYER")?;
        let h = handles(&r?).into_iter().next().ok_or_else(|| errore("box non inserito"))?;
        let n = applica(s)?;
        Ok((h, n))
    })
}

/// Un oggetto che riceve la designazione: madre, morsetto o connettore.
struct Designato {
    handle: String,
    layer: String,
    pos: P,
    valori: [String; 3],
}

fn designati(s: &mut Session) -> Result<Vec<Designato>> {
    let leggi = |b: &Inserito| Designato {
        handle: b.handle.clone(),
        layer: b.layer.clone(),
        pos: b.pos,
        valori: Livello::TUTTI.map(|l| b.attributo(l.tag()).trim().to_string()),
    };
    let mut v: Vec<Designato> = siglatura::componenti(s)?.iter().filter(|c| c.madre() && c.foglio > 0).map(|c| leggi(&c.blocco)).collect();
    v.extend(morsetti::morsetti(s)?.iter().filter(|m| m.foglio > 0).map(|m| leggi(&m.blocco)));
    Ok(v)
}

/// Dà a ogni madre, morsetto e connettore i valori dei box che lo contengono. Restituisce
/// quanti oggetti sono cambiati.
pub fn applica(s: &mut Session) -> Result<usize> {
    let tutti = boxes(s)?;
    if tutti.is_empty() {
        return Ok(0);
    }
    let mut n = 0;
    for d in designati(s)? {
        let nuovi: Vec<(&str, &str)> = Livello::TUTTI
            .iter()
            .enumerate()
            .filter_map(|(k, &l)| box_di(&tutti, l, &d.layer, d.pos).filter(|b| b.valore != d.valori[k]).map(|b| (l.tag(), b.valore.as_str())))
            .collect();
        if !nuovi.is_empty() {
            imposta_attributi(s, &d.handle, &nuovi)?;
            n += 1;
        }
    }
    Ok(n)
}

// ---------- etichette ----------

/// Quali livelli mostrare e come.
#[derive(Clone, Debug, PartialEq)]
pub struct Etichette {
    pub livelli: Vec<Livello>,
    /// Solo i livelli diversi da quelli del box che contiene il componente.
    pub discordanti: bool,
    /// Anche sui contatti (con i dati della loro madre).
    pub contatti: bool,
}

impl Default for Etichette {
    fn default() -> Self {
        Etichette { livelli: Livello::TUTTI.to_vec(), discordanti: false, contatti: true }
    }
}

/// Il testo dell'etichetta: i livelli scelti con i loro segni, nell'ordine della norma.
pub fn designazione(valori: &[String; 3], mostra: &[Livello]) -> String {
    Livello::TUTTI.iter().zip(valori).filter(|(l, v)| mostra.contains(l) && !v.trim().is_empty()).map(|(l, v)| l.normalizza(v)).collect()
}

fn crea_blocco_etichetta(s: &mut Session) -> Result<()> {
    if blocchi_definiti(s)?.iter().any(|b| b.eq_ignore_ascii_case(BLOCCO_ETICHETTA)) {
        return Ok(());
    }
    su_layer_zero(s, |s| {
        colore(s, "5")?;
        let mut v = handles(&esegui(s, "attdef", json!({"tag": "TESTO", "prompt": "Designazione", "default": "", "at": [0.0, 0.0], "height": 2.0}))?);
        if let Some(t) = v.first().cloned() {
            allinea_a_destra(s, &t)?;
        }
        v.extend(handles(&esegui(s, "attdef", json!({"tag": "RIF", "prompt": "Componente", "default": "", "at": [0.0, -2.0], "height": 1.0, "invisible": true}))?));
        blocco(s, BLOCCO_ETICHETTA, v, "Designazione CEI EN 81346")
    })
}

/// Dove sta la fine della sigla (`NOME`) di un blocco: l'etichetta va subito sopra.
fn sopra_la_sigla(s: &Session, handle: &str) -> Option<P> {
    let d = s.doc().ok()?;
    let e = d.entity(Handle::parse_hex(handle)?)?;
    let EntityKind::Insert(i) = &e.kind else { return None };
    let a = i.attribs.iter().find(|a| a.tag.eq_ignore_ascii_case("NOME") && !a.invisible)?;
    let p = a.text.align_pt.unwrap_or(a.text.insert);
    Some((p.x, p.y + a.text.height + 1.0))
}

/// Ridisegna le etichette di designazione sui fogli `fogli` (tutti se vuoto): toglie quelle che
/// c'erano e, se `regole` c'è, mette le nuove. Restituisce quante ne ha messe.
pub fn etichette(s: &mut Session, fogli: &[u32], regole: Option<&Etichette>) -> Result<usize> {
    let nel_campo = |layer: &str| fogli.is_empty() || fogli::numero_da_layer(layer).is_some_and(|n| fogli.contains(&n));
    let vecchie: Vec<String> = blocchi_inseriti(s, BLOCCO_ETICHETTA)?.into_iter().filter(|b| nel_campo(&b.layer)).map(|b| b.handle).collect();
    if !vecchie.is_empty() {
        esegui(s, "erase", json!({"handles": vecchie}))?;
    }
    let Some(r) = regole else { return Ok(0) };
    let tutti_box = boxes(s)?;
    let componenti = siglatura::componenti(s)?;
    let madri: Vec<&siglatura::Componente> = componenti.iter().filter(|c| c.madre()).collect();
    let valori = |b: &Inserito| Livello::TUTTI.map(|l| b.attributo(l.tag()).trim().to_string());
    // (handle, layer, posizione, valori) degli oggetti da etichettare
    let mut da_fare: Vec<(String, String, P, [String; 3])> = Vec::new();
    for c in componenti.iter().filter(|c| c.foglio > 0 && nel_campo(&c.blocco.layer)) {
        if c.madre() {
            da_fare.push((c.blocco.handle.clone(), c.blocco.layer.clone(), c.blocco.pos, valori(&c.blocco)));
        } else if r.contatti && c.figlio()
            && let Some(m) = madri.iter().find(|m| m.nome.trim().eq_ignore_ascii_case(c.nome.trim()))
        {
            da_fare.push((c.blocco.handle.clone(), c.blocco.layer.clone(), c.blocco.pos, valori(&m.blocco)));
        }
    }
    if da_fare.is_empty() {
        return Ok(0);
    }
    crea_blocco_etichetta(s)?;
    let mut n = 0;
    for (handle, layer, pos, val) in da_fare {
        let mostra: Vec<Livello> = r
            .livelli
            .iter()
            .copied()
            .filter(|&l| !r.discordanti || box_di(&tutti_box, l, &layer, pos).is_none_or(|b| !b.valore.eq_ignore_ascii_case(&l.normalizza(&val[l as usize]))))
            .collect();
        let testo = designazione(&val, &mostra);
        if testo.is_empty() {
            continue;
        }
        let at = sopra_la_sigla(s, &handle).unwrap_or((pos.0, pos.1 + 3.0));
        crate::util::inserisci_su_layer(s, BLOCCO_ETICHETTA, at, &json!({"TESTO": testo, "RIF": handle}), &layer)?;
        n += 1;
    }
    Ok(n)
}

// ---------- comandi ----------

fn livello_param(p: &Value) -> Result<Livello> {
    let t = testo_param(p, "tipo").unwrap_or_else(|| "quadro".into());
    Livello::da_codice(&t).ok_or_else(|| errore(format!("Tipo di box sconosciuto: {t} (quadro, funzione, ubicazione)")))
}

fn run_box(s: &mut Session, p: &Value) -> Result<Value> {
    let livello = livello_param(p)?;
    let (Some(da), Some(a)) = (punto_param(p, "da"), punto_param(p, "a")) else { return Err(errore("Indicare gli angoli del box (da, a)")) };
    let valore = testo_param(p, "valore").unwrap_or_default();
    let (h, n) = disegna_box(s, livello, &valore, da, a, &testo_param(p, "descrizione").unwrap_or_default())?;
    Ok(json!({"handle": h, "assegnati": n, "message": format!("Box {} {}: {n} oggetti assegnati", livello.codice(), livello.normalizza(&valore))}))
}

fn run_applica(s: &mut Session, _p: &Value) -> Result<Value> {
    let n = in_gruppo(s, "Applica i box", applica)?;
    Ok(json!({"assegnati": n, "message": format!("Box applicati: {n} oggetti cambiati")}))
}

fn run_etichette(s: &mut Session, p: &Value) -> Result<Value> {
    let fogli: Vec<u32> = match p.get("fogli") {
        Some(Value::String(t)) if t.eq_ignore_ascii_case("tutti") => Vec::new(),
        Some(Value::Array(a)) => a.iter().filter_map(|v| v.as_u64().and_then(|n| u32::try_from(n).ok())).collect(),
        _ => fogli::corrente(s)?.map(|f| vec![f.numero]).unwrap_or_default(),
    };
    let togli = p.get("togli").and_then(Value::as_bool).unwrap_or(false);
    let livelli: Vec<Livello> = match p.get("livelli") {
        Some(Value::Array(a)) => a.iter().filter_map(Value::as_str).filter_map(Livello::da_codice).collect(),
        Some(Value::String(t)) => t.split([',', ' ', ';']).filter_map(Livello::da_codice).collect(),
        _ => Livello::TUTTI.to_vec(),
    };
    let regole = Etichette {
        livelli,
        discordanti: testo_param(p, "regola").is_some_and(|r| r.eq_ignore_ascii_case("discordanti")),
        contatti: p.get("contatti").and_then(Value::as_bool).unwrap_or(true),
    };
    let n = in_gruppo(s, "Etichette di designazione", |s| etichette(s, &fogli, (!togli).then_some(&regole)))?;
    let dove = if fogli.is_empty() { "su tutti i fogli".to_string() } else { format!("sul foglio {}", fogli.iter().map(u32::to_string).collect::<Vec<_>>().join(", ")) };
    Ok(json!({"etichette": n, "message": if togli { format!("Etichette di designazione tolte {dove}") } else { format!("{n} etichette di designazione {dove}") }}))
}

/// BOX interattivo: il valore (proposto quello corrente), poi i due angoli.
struct BoxInterattivo {
    livello: Livello,
    valore: Option<String>,
    primo: Option<P>,
}

impl Interactive for BoxInterattivo {
    fn name(&self) -> &'static str {
        match self.livello {
            Livello::Funzione => "box.funzione",
            Livello::Ubicazione => "box.ubicazione",
            Livello::Quadro => "box.quadro",
        }
    }
    fn prompt(&self, _s: &Session) -> Prompt {
        match (&self.valore, self.primo) {
            (None, _) => {
                let imp = impostazioni::correnti();
                let corrente = match self.livello {
                    Livello::Funzione => imp.funzione,
                    Livello::Ubicazione => imp.ubicazione,
                    Livello::Quadro => imp.quadro,
                };
                let p = Prompt::new(format!("Valore del box {} (es. {}…)", self.livello.codice(), self.livello.normalizza("A1")), Accept::TEXT);
                if corrente.trim().is_empty() { p } else { p.default(self.livello.normalizza(&corrente)) }
            }
            (Some(_), None) => Prompt::new("Primo angolo del box", Accept::POINT),
            (Some(_), Some(p)) => Prompt::new("Angolo opposto", Accept::POINT).corner(Vec2::new(p.0, p.1)),
        }
    }
    fn input(&mut self, s: &mut Session, i: Input) -> Result<Step> {
        match i {
            Input::Cancel => Ok(Step::Cancel),
            Input::Text(t) | Input::Keyword(t) if self.valore.is_none() => {
                self.valore = Some(t.trim().to_string());
                Ok(Step::Continue)
            }
            Input::Enter if self.valore.is_none() => {
                match self.prompt(s).default {
                    Some(d) => {
                        self.valore = Some(d);
                        Ok(Step::Continue)
                    }
                    None => Ok(Step::Cancel),
                }
            }
            Input::Point(q) if self.valore.is_some() => match self.primo {
                None => {
                    self.primo = Some((q.x, q.y));
                    Ok(Step::Continue)
                }
                Some(da) => {
                    let (_, n) = disegna_box(s, self.livello, self.valore.as_deref().unwrap_or_default(), da, (q.x, q.y), "")?;
                    s.echo(format!("Box {}: {n} oggetti assegnati", self.livello.normalizza(self.valore.as_deref().unwrap_or_default())));
                    Ok(Step::Done)
                }
            },
            _ => Ok(Step::Continue),
        }
    }
}

fn fabbrica_quadro(_s: &Session) -> Result<Box<dyn Interactive>> {
    Ok(Box::new(BoxInterattivo { livello: Livello::Quadro, valore: None, primo: None }))
}
fn fabbrica_funzione(_s: &Session) -> Result<Box<dyn Interactive>> {
    Ok(Box::new(BoxInterattivo { livello: Livello::Funzione, valore: None, primo: None }))
}
fn fabbrica_ubicazione(_s: &Session) -> Result<Box<dyn Interactive>> {
    Ok(Box::new(BoxInterattivo { livello: Livello::Ubicazione, valore: None, primo: None }))
}

fn run_box_quadro(s: &mut Session, p: &Value) -> Result<Value> {
    run_box_di(s, p, "quadro")
}
fn run_box_funzione(s: &mut Session, p: &Value) -> Result<Value> {
    run_box_di(s, p, "funzione")
}
fn run_box_ubicazione(s: &mut Session, p: &Value) -> Result<Value> {
    run_box_di(s, p, "ubicazione")
}
fn run_box_di(s: &mut Session, p: &Value, tipo: &str) -> Result<Value> {
    let mut p = p.clone();
    if let Some(m) = p.as_object_mut() {
        m.insert("tipo".into(), json!(tipo));
    }
    run_box(s, &p)
}

pub fn comandi() -> Vec<CommandSpec> {
    let spec = |id, label, menu, params, run, interactive| CommandSpec { id, label, menu, shortcut: None, aliases: &[], params, enabled: has_doc, run, undoable: false, interactive, transparent: false };
    vec![
        spec("box", "Box", &[], "{tipo: quadro|funzione|ubicazione, valore: \"+QG\", da: [x,y], a: [x,y], descrizione?} → {handle, assegnati}", run_box as cadcraft_engine::cmd::Run, None),
        spec("box.quadro", "Box Quadro", &["Quadro", "Box Quadro"], "{valore, da, a, descrizione?}", run_box_quadro, Some(fabbrica_quadro as cadcraft_engine::cmd::Factory)),
        spec("box.funzione", "Box Funzione", &["Quadro", "Box Funzione"], "{valore, da, a, descrizione?}", run_box_funzione, Some(fabbrica_funzione as cadcraft_engine::cmd::Factory)),
        spec("box.ubicazione", "Box Ubicazione", &["Quadro", "Box Ubicazione"], "{valore, da, a, descrizione?}", run_box_ubicazione, Some(fabbrica_ubicazione as cadcraft_engine::cmd::Factory)),
        spec("box.applica", "Applica i Box", &["Quadro", "Applica i Box"], "{} — funzione, ubicazione e quadro dai box a ciò che racchiudono → {assegnati}", run_applica, None),
        spec(
            "designazioni",
            "Etichette di Designazione",
            &["Quadro", "Etichette di Designazione…"],
            "{livelli?: [funzione, ubicazione, quadro] (o \"=,++,+\"), regola?: tutti|discordanti, contatti?: bool, fogli?: \"tutti\" | [n] (default il foglio corrente), togli?: bool} → {etichette}",
            run_etichette,
            None,
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn segni_e_testo_della_designazione() {
        assert_eq!(Livello::Quadro.normalizza("QG"), "+QG");
        assert_eq!(Livello::Ubicazione.normalizza("++ZC"), "++ZC");
        assert_eq!(Livello::Ubicazione.normalizza("+ZC"), "++ZC");
        assert_eq!(Livello::Funzione.normalizza(" =TR1 "), "=TR1");
        assert_eq!(Livello::Funzione.normalizza(""), "");
        let v = ["TR1".to_string(), "++ZC".to_string(), "+QG".to_string()];
        assert_eq!(designazione(&v, &Livello::TUTTI), "=TR1++ZC+QG");
        assert_eq!(designazione(&v, &[Livello::Quadro, Livello::Funzione]), "=TR1+QG");
        assert_eq!(designazione(&[String::new(), String::new(), "+QG".into()], &Livello::TUTTI), "+QG");
    }
}
