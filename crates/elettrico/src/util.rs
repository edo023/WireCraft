//! Aiuti comuni: comandi JSON di CADCraft, gruppi di annullamento, parametri, richieste.

use cadcraft_engine::{Accept, EngineError, Input, Interactive, Prompt, Result, Session, Step};
use serde_json::{Value, json};

/// Esegue un comando JSON di CADCraft.
pub fn esegui(s: &mut Session, id: &str, params: Value) -> Result<Value> {
    s.execute(id, &params)
}

/// Gli handle restituiti da un comando (`handle` o `handles`).
pub fn handles(v: &Value) -> Vec<String> {
    if let Some(h) = v.get("handle").and_then(Value::as_str) {
        return vec![h.to_string()];
    }
    v.get("handles").and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_str).map(str::to_string).collect()).unwrap_or_default()
}

/// Esegue `f` come un solo passo di annullamento chiamato `etichetta`, anche se dentro esegue
/// più comandi di CADCraft.
pub fn in_gruppo<T>(s: &mut Session, etichetta: &str, f: impl FnOnce(&mut Session) -> Result<T>) -> Result<T> {
    let inizio = s.state_mut().map(|st| {
        st.begin_undo_group();
        st.undo.len()
    });
    let r = f(s);
    if let Ok(st) = s.state_mut() {
        st.end_undo_group();
        if let Ok(i) = inizio
            && let Some(passo) = st.undo.get_mut(i)
        {
            passo.label = etichetta.to_string();
        }
    }
    r
}

// ---------- disegno di blocchi ----------

pub fn colore(s: &mut Session, c: &str) -> Result<()> {
    esegui(s, "setvar", json!({"name": "CECOLOR", "value": c})).map(|_| ())
}

pub fn linea(s: &mut Session, h: &mut Vec<String>, punti: &[(f64, f64)], chiusa: bool) -> Result<()> {
    let p: Vec<Value> = punti.iter().map(|(x, y)| json!([x, y])).collect();
    h.extend(handles(&esegui(s, "line", json!({"points": p, "closed": chiusa}))?));
    Ok(())
}

pub fn testo(s: &mut Session, h: &mut Vec<String>, at: (f64, f64), t: &str, altezza: f64, giust: &str) -> Result<()> {
    h.extend(handles(&esegui(s, "text", json!({"at": [at.0, at.1], "text": t, "height": altezza, "justify": giust}))?));
    Ok(())
}

pub fn attributo(s: &mut Session, h: &mut Vec<String>, tag: &str, richiesta: &str, at: (f64, f64), altezza: f64) -> Result<()> {
    h.extend(handles(&esegui(s, "attdef", json!({"tag": tag, "prompt": richiesta, "default": "", "at": [at.0, at.1], "height": altezza}))?));
    Ok(())
}

pub fn blocco(s: &mut Session, nome: &str, h: Vec<String>, descrizione: &str) -> Result<()> {
    esegui(s, "-block", json!({"name": nome, "base": [0.0, 0.0], "handles": h, "keep": "delete", "description": descrizione})).map(|_| ())
}

/// Allinea a destra il testo di una definizione di attributo (o di un testo): il punto dato
/// diventa la fine del testo. ATTDEF non ha la giustificazione tra i parametri.
pub fn allinea_a_destra(s: &mut Session, handle: &str) -> Result<()> {
    allinea(s, handle, cadcraft_engine::doc::HAlign::Right)
}

/// Allinea orizzontalmente il testo di una definizione di attributo (o di un testo) rispetto
/// al suo punto d'inserimento.
pub fn allinea(s: &mut Session, handle: &str, come: cadcraft_engine::doc::HAlign) -> Result<()> {
    use cadcraft_engine::doc::EntityKind;
    let h = cadcraft_engine::doc::Handle::parse_hex(handle).ok_or_else(|| errore(format!("handle non valido: {handle}")))?;
    s.doc_mut()?.modify_entity(h, |e| {
        let t = match &mut e.kind {
            EntityKind::AttDef(a) => Some(&mut a.text),
            EntityKind::Text(t) => Some(t),
            _ => None,
        };
        if let Some(t) = t {
            t.halign = come;
            t.align_pt = Some(t.insert);
        }
    })?;
    Ok(())
}

/// Ruota il testo di una definizione di attributo (o di un testo) di `gradi`, allineandolo a
/// destra o a sinistra rispetto al suo punto.
pub fn ruota_testo(s: &mut Session, handle: &str, gradi: f64, come: cadcraft_engine::doc::HAlign) -> Result<()> {
    use cadcraft_engine::doc::EntityKind;
    let h = cadcraft_engine::doc::Handle::parse_hex(handle).ok_or_else(|| errore(format!("handle non valido: {handle}")))?;
    s.doc_mut()?.modify_entity(h, |e| {
        let t = match &mut e.kind {
            EntityKind::AttDef(a) => Some(&mut a.text),
            EntityKind::Text(t) => Some(t),
            _ => None,
        };
        if let Some(t) = t {
            t.rotation = gradi.to_radians();
            t.halign = come;
            t.align_pt = Some(t.insert);
        }
    })?;
    Ok(())
}

/// I blocchi definiti nel disegno.
pub fn blocchi_definiti(s: &mut Session) -> Result<Vec<String>> {
    let v = esegui(s, "blocks.list", json!({}))?;
    Ok(v.get("blocks")
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(|b| b.get("name").and_then(Value::as_str)).map(str::to_string).collect())
        .unwrap_or_default())
}

/// Il layer corrente.
pub fn layer_corrente(s: &mut Session) -> Result<String> {
    let v = esegui(s, "layer", json!({}))?;
    v.get("layers")
        .and_then(Value::as_array)
        .and_then(|a| a.iter().find(|l| l.get("current").and_then(Value::as_bool) == Some(true)))
        .and_then(|l| l.get("name").and_then(Value::as_str))
        .map(str::to_string)
        .ok_or_else(|| errore("Nessun layer corrente"))
}

/// Esegue `f` con il layer `0` corrente (le entità dei blocchi stanno sul layer 0, così prendono
/// il layer del blocco inserito), poi rimette il layer di prima e il colore DaLayer.
pub fn su_layer_zero<T>(s: &mut Session, f: impl FnOnce(&mut Session) -> Result<T>) -> Result<T> {
    let prima = layer_corrente(s)?;
    esegui(s, "layer.current", json!({"name": "0"}))?;
    let r = f(s);
    colore(s, "BYLAYER")?;
    esegui(s, "layer.current", json!({"name": prima}))?;
    r
}

/// Inserisce il blocco `nome` in `at` con gli attributi dati e lo porta sul layer `layer`
/// (anche se congelato). Restituisce l'handle.
pub fn inserisci_su_layer(s: &mut Session, nome: &str, at: (f64, f64), attributi: &Value, layer: &str) -> Result<String> {
    let r = esegui(s, "insert", json!({"name": nome, "at": [at.0, at.1], "attribs": attributi}))?;
    let h = handles(&r).into_iter().next().ok_or_else(|| errore(format!("blocco {nome} non inserito")))?;
    esegui(s, "laymch", json!({"handles": [h], "layer": layer}))?;
    Ok(h)
}

/// Un blocco inserito nel disegno, con i suoi attributi.
#[derive(Clone, Debug, Default)]
pub struct Inserito {
    pub nome: String,
    pub handle: String,
    pub layer: String,
    pub pos: (f64, f64),
    pub attributi: Vec<(String, String)>,
}

impl Inserito {
    pub fn attributo(&self, tag: &str) -> &str {
        self.attributi.iter().find(|(t, _)| t.eq_ignore_ascii_case(tag)).map_or("", |(_, v)| v.as_str())
    }
}

/// Il layer è l'archivio di una revisione di un foglio (`5_REV_01`): il suo contenuto è una
/// copia congelata e non partecipa a cross reference, liste e numerazioni.
pub fn archivio_revisione(layer: &str) -> bool {
    layer.to_ascii_uppercase().contains("_REV_")
}

/// I blocchi inseriti i cui nomi corrispondono a `filtro` (nomi separati da virgole, `*` jolly),
/// esclusi quelli degli archivi delle revisioni.
pub fn blocchi_inseriti(s: &mut Session, filtro: &str) -> Result<Vec<Inserito>> {
    let r = esegui(s, "dataextraction", json!({"blocks": filtro, "combine": false}))?;
    let col: Vec<String> = r.get("columns").and_then(Value::as_array).into_iter().flatten().filter_map(Value::as_str).map(str::to_string).collect();
    let fisse = ["Name", "Handle", "Layer", "Position X", "Position Y", "Rotation"];
    let i = |n: &str| col.iter().position(|c| c == n);
    let (Some(cn), Some(ch), Some(cl), Some(cx), Some(cy)) = (i("Name"), i("Handle"), i("Layer"), i("Position X"), i("Position Y")) else {
        return Ok(Vec::new());
    };
    let testo = |v: Option<&Value>| match v {
        Some(Value::String(t)) => t.clone(),
        Some(Value::Number(n)) => n.to_string(),
        _ => String::new(),
    };
    Ok(r.get("rows")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|riga| !archivio_revisione(&testo(riga.get(cl))))
        .map(|riga| Inserito {
            nome: testo(riga.get(cn)),
            handle: testo(riga.get(ch)),
            layer: testo(riga.get(cl)),
            pos: (riga.get(cx).and_then(Value::as_f64).unwrap_or(0.0), riga.get(cy).and_then(Value::as_f64).unwrap_or(0.0)),
            attributi: col.iter().enumerate().filter(|(_, c)| !fisse.contains(&c.as_str())).map(|(k, c)| (c.clone(), testo(riga.get(k)))).collect(),
        })
        .collect())
}

pub fn errore(msg: impl Into<String>) -> EngineError {
    EngineError::Other(msg.into())
}

/// Scrive i valori negli attributi di un blocco inserito e aggiunge, nascosti, quelli che la sua
/// definizione non ha (simboli disegnati prima che il tag esistesse, o da un altro programma).
pub fn imposta_attributi(s: &mut Session, handle: &str, valori: &[(&str, &str)]) -> Result<()> {
    use cadcraft_engine::doc::{EntityKind, Handle};
    let h = Handle::parse_hex(handle).ok_or_else(|| errore(format!("handle non valido: {handle}")))?;
    s.doc_mut()?.modify_entity(h, |e| {
        let EntityKind::Insert(i) = &mut e.kind else { return };
        for (tag, valore) in valori {
            if let Some(a) = i.attribs.iter_mut().find(|a| a.tag.eq_ignore_ascii_case(tag)) {
                a.text.value = (*valore).to_string();
            } else if let Some(modello) = i.attribs.first() {
                let mut a = modello.clone();
                a.tag = tag.to_ascii_uppercase();
                a.prompt = a.tag.clone();
                a.text.value = (*valore).to_string();
                a.invisible = true;
                a.constant = false;
                a.multiline = None;
                i.attribs.push(a);
            }
        }
    })?;
    Ok(())
}

pub fn testo_param(p: &Value, chiave: &str) -> Option<String> {
    p.get(chiave).and_then(Value::as_str).map(str::to_string)
}

pub fn numero_param(p: &Value, chiave: &str) -> Option<u32> {
    match p.get(chiave)? {
        Value::Number(n) => n.as_u64().and_then(|n| u32::try_from(n).ok()),
        Value::String(t) => t.trim().parse().ok(),
        _ => None,
    }
}

pub fn punto_param(p: &Value, chiave: &str) -> Option<(f64, f64)> {
    let a = p.get(chiave)?.as_array()?;
    let x = a.first()?.as_f64()?;
    let y = a.get(1)?.as_f64()?;
    (x.is_finite() && y.is_finite()).then_some((x, y))
}

/// Comando a una sola richiesta (un numero, un testo o un punto) che poi esegue il comando JSON
/// `id` con il valore come parametro `chiave`.
pub struct Richiesta {
    pub nome: &'static str,
    pub messaggio: String,
    pub default: Option<String>,
    pub accetta: Accept,
    pub id: &'static str,
    pub chiave: &'static str,
}

impl Interactive for Richiesta {
    fn name(&self) -> &'static str {
        self.nome
    }
    fn prompt(&self, _s: &Session) -> Prompt {
        Prompt { default: self.default.clone(), ..Prompt::new(self.messaggio.clone(), self.accetta) }
    }
    fn input(&mut self, s: &mut Session, i: Input) -> Result<Step> {
        let valore = match i {
            Input::Text(t) | Input::Keyword(t) => json!(t.trim()),
            Input::Point(p) => json!([p.x, p.y]),
            Input::Enter => match &self.default {
                Some(d) => json!(d),
                None => return Ok(Step::Done),
            },
            Input::Cancel => return Ok(Step::Cancel),
            _ => return Ok(Step::Continue),
        };
        let r = s.execute(self.id, &json!({ self.chiave: valore }))?;
        if let Some(m) = r.get("message").and_then(Value::as_str) {
            for riga in m.lines() {
                s.echo(riga.to_string());
            }
        }
        Ok(Step::Done)
    }
}

/// Comando interattivo che chiede un punto su un filo (o una fence di due punti, con la parola
/// chiave Fence) ed esegue il comando JSON `id` con `at` (o `da` e `a`) più i parametri `extra`;
/// si ripete fino a Invio.
pub struct PuntoOFence {
    pub nome: &'static str,
    pub messaggio: &'static str,
    pub id: &'static str,
    pub extra: Value,
    /// Fence: `Some(None)` aspetta il primo punto, `Some(Some(p))` il secondo.
    pub fence: Option<Option<(f64, f64)>>,
}

impl PuntoOFence {
    pub fn nuovo(nome: &'static str, messaggio: &'static str, id: &'static str, extra: Value) -> Self {
        PuntoOFence { nome, messaggio, id, extra, fence: None }
    }
}

impl Interactive for PuntoOFence {
    fn name(&self) -> &'static str {
        self.nome
    }
    fn prompt(&self, _s: &Session) -> Prompt {
        match self.fence {
            None => Prompt::new(format!("{} (Invio per finire)", self.messaggio), Accept::POINT).kw(&["Fence"]),
            Some(None) => Prompt::new("Primo punto della fence", Accept::POINT),
            Some(Some(p)) => Prompt::new("Secondo punto della fence", Accept::POINT).base(cadcraft_engine::geom::Vec2::new(p.0, p.1)),
        }
    }
    fn input(&mut self, s: &mut Session, i: Input) -> Result<Step> {
        match i {
            Input::Cancel => Ok(Step::Cancel),
            Input::Enter => Ok(Step::Done),
            Input::Keyword(_) | Input::Text(_) => {
                self.fence = Some(None);
                Ok(Step::Continue)
            }
            Input::Point(q) => {
                let mut p = self.extra.clone();
                if !p.is_object() {
                    p = json!({});
                }
                match self.fence {
                    Some(None) => {
                        self.fence = Some(Some((q.x, q.y)));
                        return Ok(Step::Continue);
                    }
                    Some(Some(da)) => {
                        p["da"] = json!([da.0, da.1]);
                        p["a"] = json!([q.x, q.y]);
                        self.fence = None;
                    }
                    None => p["at"] = json!([q.x, q.y]),
                }
                let r = s.execute(self.id, &p)?;
                if let Some(m) = r.get("message").and_then(Value::as_str) {
                    s.echo(m.to_string());
                }
                Ok(Step::Continue)
            }
            _ => Ok(Step::Continue),
        }
    }
}
