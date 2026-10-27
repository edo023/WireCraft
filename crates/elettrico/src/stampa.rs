//! Stampa e PDF dei fogli: una pagina per foglio, con squadratura e cartiglio del foglio 0.
//!
//! Il disegno non cambia. La stampa lavora su una copia in cui ogni foglio da stampare ha un
//! layout con una finestra sulla squadratura che congela gli altri fogli, come si farebbe a mano
//! in AutoCAD: ogni pagina contiene solo il suo foglio e porta il suo numero (`01`, `02`…).
//!
//! Il PDF **dinamico**, come nei CAD elettrici diffusi, aggiunge in coda al file (aggiornamento
//! incrementale, il PDF di CADCraft resta intatto) i segnalibri dei fogli e i collegamenti dei
//! riferimenti `05/3B` di rimandi, contatti e specchietti alla pagina del foglio citato.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use cadcraft_engine::cmd::has_doc;
use cadcraft_engine::doc::{Drawing, Entity, EntityKind, Handle, Layer, Layout, MONOCHROME_CTB, PageSetup, Space, Viewport, entity_bounds};
use cadcraft_engine::geom::{Bounds2, Vec2, Vec3};
use cadcraft_engine::{Accept, CommandSpec, Interactive, Result, Session};
use serde_json::{Value, json};

use crate::fogli::{self, numero_da_layer};
use crate::squadratura::{FOGLIO, SEPARATORE};
use crate::util::{Richiesta, errore, testo_param};

/// Layer (spento) delle finestre dei layout di stampa: la loro cornice non si stampa.
const LAYER_FINESTRE: &str = "CE_STAMPA_FINESTRE";
/// Pagine al massimo in un PDF (il limite di CADCraft).
pub const MAX_PAGINE: usize = 256;
/// Punti tipografici per millimetro.
const PT_PER_MM: f64 = 72.0 / 25.4;
/// Margine attorno al testo di un riferimento cliccabile (mm).
const MARGINE_LINK: f64 = 1.0;
/// Tolleranza per riconoscere un formato di carta dalla squadratura (mm).
const TOLL_CARTA: f64 = 5.0;

#[derive(Clone, Debug)]
pub struct Opzioni {
    /// A colori; altrimenti in bianco e nero (`monochrome.ctb`).
    pub colori: bool,
    /// Segnalibri dei fogli e riferimenti cliccabili.
    pub dinamico: bool,
    /// Titolo del documento PDF.
    pub titolo: String,
}

impl Default for Opzioni {
    fn default() -> Self {
        Opzioni { colori: false, dinamico: true, titolo: String::new() }
    }
}

pub struct Stampa {
    pub pdf: Vec<u8>,
    pub pagine: usize,
    /// Riferimenti resi cliccabili (0 se il PDF non è dinamico).
    pub collegamenti: usize,
}

/// I fogli da stampare, in ordine di numero: `"tutti"` (o niente), `"corrente"`, un elenco come
/// `"1-3,5"` o una lista di numeri. Restano solo i fogli che esistono.
pub fn scegli(scelta: Option<&Value>, esistenti: &[u32], corrente: Option<u32>) -> Result<Vec<u32>> {
    let numero = |v: &Value| v.as_u64().and_then(|n| u32::try_from(n).ok()).or_else(|| v.as_str()?.trim().parse().ok());
    let intervalli: Vec<(u32, u32)> = match scelta {
        None | Some(Value::Null) => vec![(0, u32::MAX)],
        Some(Value::Array(a)) => a.iter().filter_map(numero).map(|n| (n, n)).collect(),
        Some(v @ Value::Number(_)) => numero(v).map(|n| (n, n)).into_iter().collect(),
        Some(Value::String(t)) => match t.trim().to_lowercase().as_str() {
            "" | "tutti" | "t" => vec![(0, u32::MAX)],
            "corrente" | "c" => {
                let n = corrente.ok_or_else(|| errore("Nessun foglio corrente"))?;
                vec![(n, n)]
            }
            elenco => leggi_intervalli(elenco)?,
        },
        Some(_) => return Err(errore("fogli: \"tutti\", \"corrente\", un elenco come \"1-3,5\" o una lista di numeri")),
    };
    let scelti: Vec<u32> = esistenti.iter().copied().filter(|n| intervalli.iter().any(|(a, b)| (a..=b).contains(&n))).collect();
    if scelti.is_empty() {
        return Err(errore("Nessun foglio da stampare"));
    }
    if scelti.len() > MAX_PAGINE {
        return Err(errore(format!("Al massimo {MAX_PAGINE} fogli in un PDF: stamparli a gruppi (es. 1-{MAX_PAGINE})")));
    }
    Ok(scelti)
}

/// `"1-3, 5;8"` → [(1, 3), (5, 5), (8, 8)].
fn leggi_intervalli(t: &str) -> Result<Vec<(u32, u32)>> {
    let non_valido = || errore(format!("Elenco di fogli non valido: {t} (esempio: 1-3,5)"));
    t.split([',', ';', ' '])
        .filter(|p| !p.trim().is_empty())
        .map(|p| {
            let num = |x: &str| x.trim().parse::<u32>().map_err(|_| non_valido());
            match p.split_once('-') {
                Some((a, b)) => {
                    let (a, b) = (num(a)?, num(b)?);
                    Ok((a.min(b), a.max(b)))
                }
                None => num(p).map(|n| (n, n)),
            }
        })
        .collect()
}

/// L'etichetta di un foglio (pagina del PDF e nome del suo layout): `05`.
pub fn etichetta(n: u32) -> String {
    format!("{n:02}")
}

/// La parte di model space che fa da pagina: il foglio A3 della squadratura nostra, oppure il
/// formato di carta che contiene la squadratura del disegno (blocco `MASTER*` sul foglio 0).
pub fn pagina(d: &Drawing) -> Bounds2 {
    let a3 = Bounds2::new(Vec2::ZERO, Vec2::new(FOGLIO.0, FOGLIO.1));
    let master = d
        .model
        .iter()
        .find(|e| e.common.layer == "0" && matches!(&e.kind, EntityKind::Insert(i) if i.block.to_ascii_uppercase().starts_with("MASTER")));
    let Some(e) = master else { return a3 };
    let b = entity_bounds(d, e, 0);
    if b.is_empty() || !(b.min.is_finite() && b.max.is_finite()) || a3.expand(1.0).contains_box(&b) {
        return a3;
    }
    let (lungo, corto) = (b.width().max(b.height()), b.width().min(b.height()));
    let carta = cadcraft_engine::render::PAPER_SIZES
        .iter()
        .map(|p| (p.width_mm.max(p.height_mm), p.width_mm.min(p.height_mm)))
        .filter(|(l, c)| *l + TOLL_CARTA >= lungo && *c + TOLL_CARTA >= corto && *l <= lungo * 1.2 + TOLL_CARTA)
        .min_by(|a, b| (a.0 * a.1).total_cmp(&(b.0 * b.1)));
    let (l, c) = carta.unwrap_or((lungo + 2.0 * TOLL_CARTA, corto + 2.0 * TOLL_CARTA));
    let meta = if b.width() >= b.height() { Vec2::new(l, c) } else { Vec2::new(c, l) } / 2.0;
    Bounds2::new(b.center() - meta, b.center() + meta)
}

/// Impostazione di pagina di un layout di stampa: carta grande come la pagina, senza margini, 1:1.
fn impostazione(dim: Vec2, colori: bool) -> PageSetup {
    let (corto, lungo) = (dim.x.min(dim.y), dim.x.max(dim.y));
    let vicino = |a: f64, b: f64| (a - b).abs() < 0.5;
    let nome = cadcraft_engine::render::PAPER_SIZES
        .iter()
        .find(|p| vicino(p.width_mm.min(p.height_mm), corto) && vicino(p.width_mm.max(p.height_mm), lungo))
        .map_or_else(|| format!("User ({corto:.2} x {lungo:.2} MM)"), |p| p.name.to_string());
    PageSetup {
        paper: nome,
        width_mm: corto,
        height_mm: lungo,
        landscape: dim.x >= dim.y,
        margins_mm: [0.0; 4],
        plot_area: "layout".into(),
        scale_to_fit: false,
        scale: 1.0,
        center: false,
        plot_style_table: if colori { String::new() } else { MONOCHROME_CTB.into() },
        lineweights: true,
        paper_mm: true,
        ..PageSetup::default()
    }
}

/// La copia del disegno da stampare: un layout per foglio, con la finestra sulla pagina che
/// congela gli altri fogli.
pub fn copia_di_stampa(d: &Drawing, fogli: &[u32], colori: bool) -> Drawing {
    let finestra = pagina(d);
    let dim = Vec2::new(finestra.width(), finestra.height());
    let mut c = d.clone();
    let layer_fogli: Vec<(u32, String)> = c.layers.iter().filter_map(|l| Some((numero_da_layer(&l.name)?, l.name.clone()))).collect();
    for l in &mut c.layers {
        if numero_da_layer(&l.name).is_some_and(|n| fogli.contains(&n)) {
            l.on = true;
            l.frozen = false;
        }
    }
    c.layers.retain(|l| l.name != LAYER_FINESTRE);
    c.layers.push(Layer { on: false, plot: false, ..Layer::new(LAYER_FINESTRE) });
    let mut layouts = Vec::with_capacity(fogli.len());
    for (i, &n) in fogli.iter().enumerate() {
        let mut l = Layout::new(&etichetta(n), u32::try_from(i + 1).unwrap_or(u32::MAX));
        l.page = impostazione(dim, colori);
        let vp = Viewport {
            center: Vec3::new(dim.x / 2.0, dim.y / 2.0, 0.0),
            width: dim.x,
            height: dim.y,
            view_center: finestra.center(),
            view_height: dim.y,
            id: 2,
            frozen_layers: layer_fogli.iter().filter(|(m, _)| *m != n).map(|(_, nome)| nome.clone()).collect(),
            ..Viewport::default()
        };
        let mut e = Entity::new(c.new_handle(), EntityKind::Viewport(vp));
        e.common.layer = LAYER_FINESTRE.into();
        l.entities.push(e);
        layouts.push(l);
    }
    c.layouts = layouts;
    c
}

/// Il foglio citato da un riferimento `05/3B` (foglio, colonna 0–9, riga A–Z).
pub fn foglio_citato(t: &str) -> Option<u32> {
    let (f, cella) = t.trim().split_once(SEPARATORE)?;
    let mut c = cella.chars();
    let ok = c.next().is_some_and(|x| x.is_ascii_digit()) && c.next().is_some_and(|x| x.is_ascii_uppercase()) && c.next().is_none();
    (ok && !f.is_empty() && f.len() <= 4 && f.bytes().all(|b| b.is_ascii_digit())).then(|| f.parse().ok()).flatten()
}

/// I riferimenti a un altro foglio stampato, per pagina: rettangolo (punti PDF) e pagina citata.
fn collegamenti(d: &Drawing, fogli: &[u32], finestra: &Bounds2) -> Vec<Collegamento> {
    let pagina_di: HashMap<String, usize> = fogli.iter().enumerate().map(|(i, n)| (fogli::nome_layer(*n), i)).collect();
    let indice: HashMap<u32, usize> = fogli.iter().enumerate().map(|(i, n)| (*n, i)).collect();
    let rettangolo = |b: Bounds2| {
        let b = b.expand(MARGINE_LINK);
        let pt = |p: Vec2| (p - finestra.min) * PT_PER_MM;
        let (a, z) = (pt(b.min), pt(b.max));
        [a.x, a.y, z.x, z.y]
    };
    let mut out = Vec::new();
    for e in d.model.iter() {
        let Some(&da) = pagina_di.get(&e.common.layer) else { continue };
        let testi: Vec<&cadcraft_engine::doc::Text> = match &e.kind {
            EntityKind::Insert(i) => i.attribs.iter().filter(|a| !a.invisible).map(|a| &a.text).collect(),
            EntityKind::Text(t) => vec![t],
            _ => continue,
        };
        for t in testi {
            let Some(&a) = foglio_citato(&t.value).and_then(|n| indice.get(&n)) else { continue };
            let b = entity_bounds(d, &Entity::new(Handle(0), EntityKind::Text(t.clone())), 0);
            if !b.is_empty() && finestra.intersects(&b) {
                out.push(Collegamento { da, rett: rettangolo(b), a });
            }
        }
    }
    out
}

/// Il PDF dei fogli `fogli` (in quest'ordine, uno per pagina); `titoli` per i segnalibri.
pub fn pdf_fogli(d: &Drawing, fogli: &[u32], titoli: &HashMap<u32, String>, o: &Opzioni) -> Result<Stampa> {
    let plot = cadcraft_engine::cmd::file::io().and_then(|h| h.plot).ok_or_else(|| errore("La stampa non è disponibile in questa versione"))?;
    let copia = copia_di_stampa(d, fogli, o.colori);
    let nomi: Vec<String> = fogli.iter().map(|n| etichetta(*n)).collect();
    let pdf = plot(&copia, &Space::Model, &json!({"layouts": nomi, "title": o.titolo})).map_err(|e| errore(format!("Stampa: {e}")))?;
    if !o.dinamico {
        return Ok(Stampa { pdf, pagine: fogli.len(), collegamenti: 0 });
    }
    let link = collegamenti(d, fogli, &pagina(d));
    let segnalibri: Vec<String> = fogli
        .iter()
        .map(|n| match titoli.get(n).map(|t| t.trim()).filter(|t| !t.is_empty()) {
            Some(t) => format!("{} {t}", etichetta(*n)),
            None => format!("Foglio {}", etichetta(*n)),
        })
        .collect();
    // se il PDF non ha la forma attesa resta quello semplice
    Ok(match naviga(&pdf, &segnalibri, &link) {
        Some(dinamico) => Stampa { pdf: dinamico, pagine: fogli.len(), collegamenti: link.len() },
        None => Stampa { pdf, pagine: fogli.len(), collegamenti: 0 },
    })
}

/// Dove va il PDF se non è indicato: la cartella `PDF` accanto al disegno, col suo nome.
pub fn percorso_predefinito(disegno: &str) -> PathBuf {
    let p = Path::new(disegno);
    let nome = p.file_stem().map_or_else(|| "Fogli".into(), |s| s.to_string_lossy().to_string());
    p.parent().unwrap_or(Path::new("")).join("PDF").join(format!("{nome}.pdf"))
}

fn run_stampa(s: &mut Session, p: &Value) -> Result<Value> {
    let elenco = fogli::elenco(s)?;
    if elenco.is_empty() {
        return Err(errore("Il disegno non ha fogli: usa Nuovo multifoglio"));
    }
    let numeri: Vec<u32> = elenco.iter().map(|f| f.numero).collect();
    let corrente = fogli::corrente(s)?.map(|f| f.numero);
    let scelti = scegli(p.get("fogli"), &numeri, corrente)?;
    let file = match testo_param(p, "file").filter(|f| !f.trim().is_empty()) {
        Some(f) => PathBuf::from(f),
        None => {
            let disegno = s.state()?.path.clone().ok_or_else(|| errore("Salvare il disegno o indicare il file PDF"))?;
            percorso_predefinito(&disegno)
        }
    };
    let flag = |k: &str, d: bool| p.get(k).and_then(Value::as_bool).unwrap_or(d);
    let base = Opzioni::default();
    let o = Opzioni {
        colori: flag("colori", base.colori),
        dinamico: flag("dinamico", base.dinamico),
        titolo: testo_param(p, "titolo").unwrap_or_else(|| file.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default()),
    };
    let titoli: HashMap<u32, String> = elenco.iter().map(|f| (f.numero, f.titolo.clone())).collect();
    let st = pdf_fogli(s.doc()?, &scelti, &titoli, &o)?;
    if let Some(cartella) = file.parent().filter(|c| !c.as_os_str().is_empty()) {
        std::fs::create_dir_all(cartella).map_err(|e| errore(format!("{}: {e}", cartella.display())))?;
    }
    cadcraft_engine::atomic_write::write_atomic(&file, &st.pdf).map_err(|e| errore(format!("{}: {e}", file.display())))?;
    let percorso = file.to_string_lossy().to_string();
    let link = if o.dinamico { format!(", {} riferimenti cliccabili", st.collegamenti) } else { String::new() };
    Ok(json!({
        "file": percorso,
        "fogli": scelti,
        "pagine": st.pagine,
        "collegamenti": st.collegamenti,
        "message": format!("PDF: {} fogli{link} in {percorso}", st.pagine),
    }))
}

fn chiedi_fogli(_s: &Session) -> Result<Box<dyn Interactive>> {
    Ok(Box::new(Richiesta {
        nome: "stampafogli",
        messaggio: "Fogli da stampare (Tutti, Corrente o elenco come 1-3,5)".into(),
        default: Some("tutti".into()),
        accetta: Accept::TEXT,
        id: "stampafogli",
        chiave: "fogli",
    }))
}

pub fn comandi() -> Vec<CommandSpec> {
    vec![CommandSpec {
        id: "stampafogli",
        label: "STAMPA e PDF Fogli",
        menu: &["Fogli", "Stampa e PDF…"],
        shortcut: None,
        aliases: &[],
        params: "{fogli?: \"tutti\" | \"corrente\" | \"1-3,5\" | [numeri], file?: percorso .pdf (default: cartella PDF accanto al disegno), colori?: bool (default bianco e nero), dinamico?: bool (default sì: segnalibri e riferimenti cliccabili), titolo?} → {file, fogli, pagine, collegamenti}",
        enabled: has_doc,
        run: run_stampa,
        undoable: false,
        interactive: Some(chiedi_fogli),
        transparent: false,
    }]
}

// ---------- PDF dinamico: aggiornamento incrementale del PDF di CADCraft ----------

/// Un riferimento cliccabile: rettangolo `[x1, y1, x2, y2]` (punti) sulla pagina `da`, che porta
/// alla pagina `a` (indici da 0).
#[derive(Clone, Debug, PartialEq)]
pub struct Collegamento {
    pub da: usize,
    pub rett: [f64; 4],
    pub a: usize,
}

fn trova(h: &[u8], ago: &[u8], da: usize) -> Option<usize> {
    h.get(da..)?.windows(ago.len()).position(|w| w == ago).map(|i| i + da)
}

/// L'intero dopo `chiave` (`/Root 12 0 R` → 12).
fn numero_dopo(t: &str, chiave: &str) -> Option<usize> {
    let i = t.find(chiave)? + chiave.len();
    t.get(i..)?.trim_start().split(|c: char| !c.is_ascii_digit()).next()?.parse().ok()
}

/// I riferimenti di un array dopo `chiave` (`/Kids [3 0 R 5 0 R]` → [3, 5]).
fn riferimenti(t: &str, chiave: &str) -> Option<Vec<usize>> {
    let i = t.find(chiave)? + chiave.len();
    let resto = t.get(i..)?.trim_start().strip_prefix('[')?;
    let parti: Vec<&str> = resto.get(..resto.find(']')?)?.split_whitespace().collect();
    parti.chunks(3).map(|c| if let [n, "0", "R"] = c { n.parse().ok() } else { None }).collect()
}

/// Aggiunge `voci` in fondo a un dizionario `<< … >>`.
fn aggiungi_voci(dizionario: &str, voci: &str) -> Option<String> {
    let fine = dizionario.rfind(">>")?;
    Some(format!("{} {voci} >>", dizionario.get(..fine)?.trim_end()))
}

/// Testo PDF in UTF-16 (titoli dei segnalibri con lettere accentate).
fn testo_pdf(t: &str) -> String {
    let mut s = String::from("<FEFF");
    for u in t.encode_utf16() {
        let _ = write!(s, "{u:04X}");
    }
    s.push('>');
    s
}

/// Un PDF a tabella xref unica, come lo scrive CADCraft.
struct Pdf<'a> {
    byte: &'a [u8],
    offsets: Vec<usize>,
    size: usize,
    root: usize,
    info: Option<usize>,
    xref: usize,
}

impl<'a> Pdf<'a> {
    fn leggi(byte: &'a [u8]) -> Option<Self> {
        let sx = byte.windows(9).rposition(|w| w == b"startxref")?;
        let xref: usize = std::str::from_utf8(byte.get(sx + 9..)?).ok()?.split_whitespace().next()?.parse().ok()?;
        let tabella = std::str::from_utf8(byte.get(xref..sx)?).ok()?;
        let mut righe = tabella.lines();
        if righe.next()? != "xref" {
            return None;
        }
        let mut intest = righe.next()?.split_whitespace();
        let (primo, n): (usize, usize) = (intest.next()?.parse().ok()?, intest.next()?.parse().ok()?);
        let offsets: Vec<usize> = righe.by_ref().take(n).map(|r| r.get(..10)?.parse().ok()).collect::<Option<_>>()?;
        if primo != 0 || offsets.len() != n {
            return None;
        }
        let trailer = tabella.get(tabella.find("trailer")?..)?;
        Some(Pdf { byte, offsets, size: numero_dopo(trailer, "/Size")?, root: numero_dopo(trailer, "/Root")?, info: numero_dopo(trailer, "/Info"), xref })
    }

    /// Il testo dell'oggetto `n` tra `n 0 obj` ed `endobj` (dizionari, non flussi).
    fn oggetto(&self, n: usize) -> Option<&'a str> {
        let o = *self.offsets.get(n)?;
        let testa = format!("{n} 0 obj");
        let inizio = o + testa.len();
        if self.byte.get(o..inizio)? != testa.as_bytes() {
            return None;
        }
        let fine = trova(self.byte, b"endobj", inizio)?;
        std::str::from_utf8(self.byte.get(inizio..fine)?).ok().map(str::trim)
    }
}

/// Aggiunge segnalibri (uno per pagina) e collegamenti interni al PDF, in coda al file con una
/// nuova sezione xref. `None` se il PDF non ha la forma attesa.
pub fn naviga(byte: &[u8], segnalibri: &[String], collegamenti: &[Collegamento]) -> Option<Vec<u8>> {
    let pdf = Pdf::leggi(byte)?;
    let catalogo = pdf.oggetto(pdf.root)?;
    let pagine = riferimenti(pdf.oggetto(numero_dopo(catalogo, "/Pages")?)?, "/Kids")?;
    let dest = |p: usize| pagine.get(p).map(|k| format!("[{k} 0 R /Fit]"));
    let mut nuovi: Vec<(usize, String)> = Vec::new();
    let mut prossimo = pdf.size;
    for (i, &pagina) in pagine.iter().enumerate() {
        let mut annots = Vec::new();
        for c in collegamenti.iter().filter(|c| c.da == i) {
            let Some(d) = dest(c.a) else { continue };
            let [x1, y1, x2, y2] = c.rett;
            nuovi.push((prossimo, format!("<< /Type /Annot /Subtype /Link /Rect [{x1:.2} {y1:.2} {x2:.2} {y2:.2}] /Border [0 0 0] /Dest {d} >>")));
            annots.push(format!("{prossimo} 0 R"));
            prossimo += 1;
        }
        if annots.is_empty() {
            continue;
        }
        let corpo = pdf.oggetto(pagina)?;
        let annots = annots.join(" ");
        let corpo = if corpo.contains("/Annots [") { corpo.replacen("/Annots [", &format!("/Annots [{annots} "), 1) } else { aggiungi_voci(corpo, &format!("/Annots [{annots}]"))? };
        nuovi.push((pagina, corpo));
    }
    let mut cat = catalogo.replace(" /PageMode /UseOC", "");
    let voci: Vec<(usize, &String)> = segnalibri.iter().enumerate().take(pagine.len()).collect();
    if !voci.is_empty() {
        let radice = prossimo;
        let (primo, ultimo) = (radice + 1, radice + voci.len());
        for (n, (i, titolo)) in (primo..).zip(&voci) {
            let mut corpo = format!("<< /Title {} /Parent {radice} 0 R /Dest {}", testo_pdf(titolo), dest(*i)?);
            if n > primo {
                let _ = write!(corpo, " /Prev {} 0 R", n - 1);
            }
            if n < ultimo {
                let _ = write!(corpo, " /Next {} 0 R", n + 1);
            }
            corpo.push_str(" >>");
            nuovi.push((n, corpo));
        }
        nuovi.push((radice, format!("<< /Type /Outlines /First {primo} 0 R /Last {ultimo} 0 R /Count {} >>", voci.len())));
        prossimo = ultimo + 1;
        cat = aggiungi_voci(&cat, &format!("/Outlines {radice} 0 R /PageMode /UseOutlines"))?;
    }
    nuovi.push((pdf.root, cat));
    nuovi.sort_by_key(|(n, _)| *n);

    let mut out = byte.to_vec();
    if !out.ends_with(b"\n") {
        out.push(b'\n');
    }
    let mut posizioni = Vec::with_capacity(nuovi.len());
    for (n, corpo) in &nuovi {
        posizioni.push((*n, out.len()));
        out.extend_from_slice(format!("{n} 0 obj\n{corpo}\nendobj\n").as_bytes());
    }
    let xref = out.len();
    // la voce libera 0 in testa, come negli aggiornamenti di Acrobat: certi lettori sospettano di
    // una tabella che comincia da un altro numero
    let mut x = String::from("xref\n0 1\n0000000000 65535 f \n");
    for gruppo in posizioni.chunk_by(|a, b| b.0 == a.0 + 1) {
        let primo = gruppo.first().map_or(0, |g| g.0);
        let _ = writeln!(x, "{primo} {}", gruppo.len());
        for (_, o) in gruppo {
            let _ = writeln!(x, "{o:010} 00000 n ");
        }
    }
    let info = pdf.info.map(|i| format!(" /Info {i} 0 R")).unwrap_or_default();
    let _ = write!(x, "trailer\n<< /Size {prossimo} /Root {} 0 R{info} /Prev {} >>\nstartxref\n{xref}\n%%EOF\n", pdf.root, pdf.xref);
    out.extend_from_slice(x.as_bytes());
    Some(out)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn scelta_dei_fogli() {
        let esistenti = [1, 2, 3, 5, 8];
        let v = |s: Value| scegli(Some(&s), &esistenti, Some(3)).unwrap_or_default();
        assert_eq!(scegli(None, &esistenti, None).unwrap_or_default(), esistenti);
        assert_eq!(v(json!("tutti")), esistenti);
        assert_eq!(v(json!("corrente")), [3]);
        assert_eq!(v(json!("2-5, 8")), [2, 3, 5, 8]);
        assert_eq!(v(json!("5-2;1")), [1, 2, 3, 5]);
        assert_eq!(v(json!([8, "1", 4])), [1, 8]);
        assert!(scegli(Some(&json!("4")), &esistenti, None).is_err());
        assert!(scegli(Some(&json!("1-x")), &esistenti, None).is_err());
        assert!(scegli(Some(&json!("corrente")), &esistenti, None).is_err());
    }

    #[test]
    fn riferimenti_ai_fogli() {
        assert_eq!(foglio_citato("05/3B"), Some(5));
        assert_eq!(foglio_citato(" 112/0F "), Some(112));
        assert_eq!(foglio_citato("///"), None);
        assert_eq!(foglio_citato("05/3"), None);
        assert_eq!(foglio_citato("L1"), None);
        assert_eq!(foglio_citato("05/3b"), None);
    }

    #[test]
    fn percorso_del_pdf() {
        let p = percorso_predefinito("C:/Commesse/Prova/SCHEMA.dxf");
        assert_eq!(p, Path::new("C:/Commesse/Prova").join("PDF").join("SCHEMA.pdf"));
    }

    /// Un PDF minimo come lo scrive CADCraft: catalogo, albero delle pagine, due pagine, info.
    fn pdf_di_prova() -> Vec<u8> {
        let corpi = [
            "<< /Type /Catalog /Pages 2 0 R /PageLabels << /Nums [0 << /P (01) >> 1 << /P (02) >>] >> >>",
            "<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 >>",
            "<< /Type /Page /Parent 2 0 R /Resources << >> /MediaBox [0 0 1190.55 841.89] >>",
            "<< /Type /Page /Parent 2 0 R /Resources << >> /MediaBox [0 0 1190.55 841.89] >>",
            "<< /Producer (CADCraft) /Title (Prova) >>",
        ];
        let mut out = b"%PDF-1.4\n".to_vec();
        let mut off = Vec::new();
        for (i, c) in corpi.iter().enumerate() {
            off.push(out.len());
            out.extend_from_slice(format!("{} 0 obj\n{c}\nendobj\n", i + 1).as_bytes());
        }
        let xref = out.len();
        let mut x = format!("xref\n0 {}\n0000000000 65535 f \n", off.len() + 1);
        for o in &off {
            let _ = writeln!(x, "{o:010} 00000 n ");
        }
        let _ = write!(x, "trailer\n<< /Size 6 /Root 1 0 R /Info 5 0 R >>\nstartxref\n{xref}\n%%EOF\n");
        out.extend_from_slice(x.as_bytes());
        out
    }

    #[test]
    fn pdf_dinamico_aggiunge_segnalibri_e_collegamenti() {
        let base = pdf_di_prova();
        let link = [Collegamento { da: 0, rett: [10.0, 20.0, 30.0, 25.0], a: 1 }];
        let out = naviga(&base, &["01 Potenza".into(), "02 Ausiliari è".into()], &link).unwrap_or_default();
        assert!(out.starts_with(&base), "il PDF di partenza resta intatto");
        let coda = std::str::from_utf8(&out[base.len()..]).unwrap_or_default();
        // 6 oggetti, 1 collegamento, radice e 2 segnalibri; la xref di prima resta valida
        let xref_prima = numero_dopo(std::str::from_utf8(&base).unwrap_or_default(), "startxref").unwrap();
        assert!(coda.contains(&format!("<< /Size 10 /Root 1 0 R /Info 5 0 R /Prev {xref_prima} >>")), "{coda}");
        // ogni oggetto della nuova xref sta dove dice la tabella
        let mut righe = coda[coda.find("xref\n").unwrap_or(0)..].lines().skip(1);
        while let Some(intest) = righe.next() {
            if intest.starts_with("trailer") {
                break;
            }
            let mut p = intest.split_whitespace();
            let (primo, quanti): (usize, usize) = (p.next().unwrap().parse().unwrap(), p.next().unwrap().parse().unwrap());
            for n in primo..primo + quanti {
                let o: usize = righe.next().unwrap()[..10].parse().unwrap();
                if n == 0 {
                    continue;
                }
                assert!(out[o..].starts_with(format!("{n} 0 obj\n").as_bytes()), "oggetto {n} a {o}");
            }
        }
        let pagina1 = coda.split("3 0 obj\n").nth(1).unwrap_or_default();
        assert!(pagina1.starts_with("<< /Type /Page") && pagina1.contains("/Annots [6 0 R]"), "{pagina1}");
        assert!(coda.contains("/Rect [10.00 20.00 30.00 25.00] /Border [0 0 0] /Dest [4 0 R /Fit]"));
        assert!(coda.contains("/Outlines 7 0 R /PageMode /UseOutlines"));
        assert!(coda.contains(&testo_pdf("02 Ausiliari è")));
        assert!(coda.contains("/Parent 7 0 R /Dest [3 0 R /Fit] /Next 9 0 R"));
    }

    #[test]
    fn pdf_estraneo_resta_com_e() {
        assert!(naviga(b"%PDF-1.7 senza xref", &[], &[]).is_none());
    }
}
