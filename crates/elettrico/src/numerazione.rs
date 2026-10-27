//! Numerazione dei fili.
//!
//! Una rete è un insieme di fili dello stesso foglio collegati tra loro (estremi in comune o a T);
//! i simboli interrompono i fili, quindi separano le reti. Una rete che tocca un rimando prende
//! il nome del suo segnale (L1, +24V…); le altre un numero secondo le impostazioni: foglio e
//! progressivo (`05.03`, il predefinito), progressivo su tutto il progetto, o posizionale
//! (foglio, colonna e progressivo nella colonna). Il numero sta in un blocco con l'attributo `N`,
//! a 1 mm a sinistra del tratto verticale più lungo, ruotato di 90° (come nei CAD elettrici
//! diffusi: testo alto 2, a sinistra, ruotato).
//!
//! Come nei CAD elettrici diffusi:
//! - la numerazione automatica può mantenere i numeri già scritti; i fili **bloccati**
//!   (`BLOCCATO=1`) e quelli dei rimandi non cambiano mai;
//! - la numerazione manuale dà a uno o più fili (fence) un numero, incrementato filo per filo,
//!   o una stringa libera (con l'apice davanti: `'R1`) che non segue le impostazioni.

use cadcraft_engine::cmd::has_doc;
use cadcraft_engine::{Accept, CommandSpec, Input, Interactive, Prompt, Result, Session, Step};
use serde_json::{Value, json};

use crate::fili::{self, Segmento};
use crate::fogli;
use crate::impostazioni::{self, Impostazioni, NumeriFili};
use crate::squadratura;
use crate::util::{Inserito, blocchi_definiti, blocchi_inseriti, blocco, colore, errore, esegui, handles, in_gruppo, inserisci_su_layer, numero_param, punto_param, su_layer_zero, testo_param};

pub const BLOCCO_NUMERO: &str = "CE_NUMFILO";
const TOLL: f64 = 0.01;

type P = (f64, f64);

fn vicini(a: P, b: P) -> bool {
    (a.0 - b.0).abs() < TOLL && (a.1 - b.1).abs() < TOLL
}

/// Il punto sta sul segmento (estremi compresi), per segmenti orizzontali o verticali.
pub fn su(f: &Segmento, p: P) -> bool {
    let (x0, x1) = (f.a.0.min(f.b.0), f.a.0.max(f.b.0));
    let (y0, y1) = (f.a.1.min(f.b.1), f.a.1.max(f.b.1));
    p.0 > x0 - TOLL && p.0 < x1 + TOLL && p.1 > y0 - TOLL && p.1 < y1 + TOLL && {
        // collineare
        let (dx, dy) = (f.b.0 - f.a.0, f.b.1 - f.a.1);
        ((p.0 - f.a.0) * dy - (p.1 - f.a.1) * dx).abs() < TOLL * (dx.abs() + dy.abs()).max(1.0)
    }
}

/// Due fili sono collegati se un estremo dell'uno sta sull'altro (un semplice incrocio no).
fn collegati(a: &Segmento, b: &Segmento) -> bool {
    su(a, b.a) || su(a, b.b) || su(b, a.a) || su(b, a.b)
}

/// Le reti di fili di un foglio (indici nei fili dati).
pub fn reti(fili: &[Segmento]) -> Vec<Vec<usize>> {
    let n = fili.len();
    let mut padre: Vec<usize> = (0..n).collect();
    fn radice(p: &mut [usize], mut i: usize) -> usize {
        while p[i] != i {
            p[i] = p[p[i]];
            i = p[i];
        }
        i
    }
    for i in 0..n {
        for j in i + 1..n {
            if collegati(&fili[i], &fili[j]) {
                let (a, b) = (radice(&mut padre, i), radice(&mut padre, j));
                if a != b {
                    padre[b] = a;
                }
            }
        }
    }
    let mut gruppi: std::collections::BTreeMap<usize, Vec<usize>> = Default::default();
    for i in 0..n {
        let r = radice(&mut padre, i);
        gruppi.entry(r).or_default().push(i);
    }
    gruppi.into_values().collect()
}

fn crea_blocco(s: &mut Session) -> Result<()> {
    if blocchi_definiti(s)?.iter().any(|b| b.eq_ignore_ascii_case(BLOCCO_NUMERO)) {
        return Ok(());
    }
    su_layer_zero(s, |s| {
        colore(s, "BYLAYER")?;
        let r = esegui(s, "attdef", json!({"tag": "N", "prompt": "Numero filo", "default": "", "at": [0.0, 0.0], "height": 2.0}))?;
        let mut h = handles(&r);
        if let Some(t) = h.first() {
            crate::util::allinea(s, t, cadcraft_engine::doc::HAlign::Center)?;
        }
        h.extend(handles(&esegui(s, "attdef", json!({"tag": "BLOCCATO", "prompt": "Filo bloccato (1)", "default": "", "at": [0.0, -1.0], "height": 1.0, "invisible": true}))?));
        blocco(s, BLOCCO_NUMERO, h, "Numero del filo")
    })
}

/// Dove mettere il numero di una rete: (punto, rotazione) a 1 mm a sinistra del tratto verticale
/// più lungo, o sopra quello orizzontale più lungo.
pub fn posizione(fili: &[&Segmento]) -> Option<(P, f64)> {
    let lung = |f: &&Segmento| (f.a.0 - f.b.0).abs() + (f.a.1 - f.b.1).abs();
    let verticali = fili.iter().filter(|f| (f.a.0 - f.b.0).abs() < TOLL);
    if let Some(f) = verticali.max_by(|a, b| lung(a).total_cmp(&lung(b))) {
        return Some(((f.a.0 - 1.0, (f.a.1 + f.b.1) / 2.0), 90.0));
    }
    let f = fili.iter().max_by(|a, b| lung(a).total_cmp(&lung(b)))?;
    Some((((f.a.0 + f.b.0) / 2.0, f.a.1 + 1.0), 0.0))
}

/// L'etichetta sta su un filo della rete: a 1 mm a sinistra di un tratto verticale o 1 mm
/// sopra uno orizzontale.
fn etichetta_su(e: &Inserito, g: &[&Segmento]) -> bool {
    g.iter().any(|f| ((f.a.0 - f.b.0).abs() < TOLL && su(f, (e.pos.0 + 1.0, e.pos.1))) || ((f.a.1 - f.b.1).abs() < TOLL && su(f, (e.pos.0, e.pos.1 - 1.0))))
}

/// Ordine di lettura di una rete: dal punto più a sinistra (e più in alto).
fn chiave(g: &[&Segmento]) -> (i64, i64) {
    let x = g.iter().map(|f| f.a.0.min(f.b.0)).fold(f64::INFINITY, f64::min);
    let y = g.iter().map(|f| f.a.1.max(f.b.1)).fold(f64::NEG_INFINITY, f64::max);
    ((x * 1000.0).round() as i64, -(y * 1000.0).round() as i64)
}

/// Il punto di riferimento di una rete (il più a sinistra e in alto): per la colonna.
fn punto_rete(g: &[&Segmento]) -> P {
    let mut v: Vec<P> = g.iter().flat_map(|f| [f.a, f.b]).collect();
    v.sort_by(|a, b| a.0.total_cmp(&b.0).then(b.1.total_cmp(&a.1)));
    v.first().copied().unwrap_or((0.0, 0.0))
}

/// Il numero di un filo secondo le impostazioni: `n` sul foglio `foglio` (colonna `col` per la
/// numerazione posizionale).
pub fn formatta(imp: &Impostazioni, foglio: u32, col: u32, n: u32) -> String {
    let corpo = match imp.numeri_fili {
        NumeriFili::FoglioProgressivo => format!("{foglio:0w$}{}{n:0c$}", imp.separatore_fili, w = imp.cifre_foglio.max(1), c = imp.cifre_filo.max(1)),
        NumeriFili::Progressivo => format!("{n:0c$}", c = imp.cifre_filo.max(1)),
        NumeriFili::Posizionale => format!("{foglio:0w$}{col}{}{n}", imp.separatore_fili, w = imp.cifre_foglio.max(1)),
    };
    format!("{}{corpo}{}", imp.prefisso_fili, imp.suffisso_fili)
}

/// Il progressivo di un numero scritto con [`formatta`] (per ripartire dal più alto).
fn progressivo(imp: &Impostazioni, t: &str) -> Option<u32> {
    let t = t.strip_prefix(imp.prefisso_fili.as_str())?.strip_suffix(imp.suffisso_fili.as_str())?;
    let n = match imp.numeri_fili {
        NumeriFili::Progressivo => t,
        _ if imp.separatore_fili.is_empty() => t.get(imp.cifre_foglio..)?,
        _ => t.rsplit_once(imp.separatore_fili.as_str())?.1,
    };
    (!n.is_empty() && n.bytes().all(|b| b.is_ascii_digit())).then(|| n.parse().ok()).flatten()
}

/// Una rete del foglio con il suo nome: dai rimandi, dall'etichetta che ha già, o nuovo.
struct Rete<'a> {
    fili: Vec<&'a Segmento>,
    rimando: Option<String>,
    etichette: Vec<&'a Inserito>,
}

impl Rete<'_> {
    fn etichetta(&self) -> Option<&Inserito> {
        self.etichette.first().copied()
    }
    fn bloccata(&self) -> bool {
        self.etichette.iter().any(|e| e.attributo("BLOCCATO").trim() == "1")
    }
}

fn reti_del_foglio<'a>(tutti: &'a [Segmento], etichette: &'a [Inserito], rimandi: &[fili::Rimando], layer: &str, n: u32) -> Vec<Rete<'a>> {
    let mut v: Vec<Rete<'a>> = reti(tutti)
        .into_iter()
        .map(|g| {
            let g: Vec<&Segmento> = g.into_iter().filter_map(|i| tutti.get(i)).collect();
            // il segnale di un rimando che tocca un estremo della rete (prima gli arrivi)
            let mut toccati: Vec<&fili::Rimando> = rimandi.iter().filter(|r| r.foglio == n && g.iter().any(|f| vicini(f.a, r.pos) || vicini(f.b, r.pos))).collect();
            toccati.sort_by_key(|r| r.partenza);
            let rimando = toccati.first().map(|r| r.segnale.trim().to_string()).filter(|t| !t.is_empty());
            let etichette = etichette.iter().filter(|e| e.layer == layer && etichetta_su(e, &g)).collect();
            Rete { fili: g, rimando, etichette }
        })
        .collect();
    v.sort_by_key(|r| chiave(&r.fili));
    v
}

/// Opzioni della numerazione automatica.
#[derive(Clone, Debug, Default)]
pub struct Opzioni {
    /// Primo numero (default: 1, o dopo il più alto già usato con la numerazione progressiva).
    pub primo: Option<u32>,
    /// I fili già numerati conservano il numero.
    pub mantieni: bool,
}

/// Numera i fili dei fogli `fogli` (in quest'ordine). Restituisce per foglio i nomi delle reti.
pub fn numera(s: &mut Session, fogli_da: &[u32], o: &Opzioni) -> Result<Vec<(u32, Vec<String>)>> {
    crea_blocco(s)?;
    let imp = impostazioni::correnti();
    let rimandi = fili::rimandi(s)?;
    let mut etichette = blocchi_inseriti(s, BLOCCO_NUMERO)?;
    // progressivo unico: riparte dopo il più alto in uso sugli altri fogli
    let mut globale = o.primo.unwrap_or_else(|| {
        etichette.iter().filter(|e| !fogli_da.iter().any(|n| fogli::nome_layer(*n) == e.layer)).filter_map(|e| progressivo(&imp, e.attributo("N"))).max().map_or(1, |m| m + 1)
    });
    let mut out = Vec::new();
    for &n in fogli_da {
        let layer = fogli::nome_layer(n);
        let tutti = fili::fili(s, &layer)?;
        let reti = reti_del_foglio(&tutti, &etichette, &rimandi, &layer, n);
        // numeri già presi sul foglio (mantenuti o bloccati): i nuovi li saltano
        let tenuti: Vec<String> =
            reti.iter().filter(|r| r.rimando.is_none() && (r.bloccata() || o.mantieni)).filter_map(|r| r.etichetta()).map(|e| e.attributo("N").to_string()).collect();
        let mut via: Vec<String> = Vec::new();
        let mut nuove: Vec<(P, f64, String)> = Vec::new();
        let mut nomi = Vec::new();
        let mut locale = o.primo.unwrap_or(1);
        let mut per_colonna: std::collections::HashMap<u32, u32> = Default::default();
        for r in &reti {
            let tieni = r.rimando.is_none() && r.etichetta().is_some_and(|e| !e.attributo("N").trim().is_empty()) && (r.bloccata() || o.mantieni);
            let nome = if let Some(t) = &r.rimando {
                t.clone()
            } else if tieni {
                r.etichetta().map(|e| e.attributo("N").to_string()).unwrap_or_default()
            } else {
                let (col, _) = squadratura::colonna_riga(punto_rete(&r.fili).0, punto_rete(&r.fili).1);
                loop {
                    let k = match imp.numeri_fili {
                        NumeriFili::FoglioProgressivo => {
                            locale += 1;
                            locale - 1
                        }
                        NumeriFili::Progressivo => {
                            globale += 1;
                            globale - 1
                        }
                        NumeriFili::Posizionale => {
                            let c = per_colonna.entry(col).or_insert(0);
                            *c += 1;
                            *c - 1
                        }
                    };
                    let t = formatta(&imp, n, col, k);
                    if !tenuti.contains(&t) {
                        break t;
                    }
                }
            };
            if !tieni {
                via.extend(r.etichette.iter().map(|e| e.handle.clone()));
                let solo_orizzontale = r.fili.iter().all(|f| (f.a.1 - f.b.1).abs() < TOLL);
                if let Some((p, rot)) = posizione(&r.fili).filter(|_| r.rimando.is_none() || !solo_orizzontale) {
                    nuove.push((p, rot, nome.clone()));
                }
            } else {
                // etichette doppie sulla stessa rete: ne resta una
                via.extend(r.etichette.iter().skip(1).map(|e| e.handle.clone()));
            }
            nomi.push(nome);
        }
        // etichette rimaste su fili che non ci sono più
        via.extend(etichette.iter().filter(|e| e.layer == layer && !reti.iter().any(|r| r.etichette.iter().any(|x| x.handle == e.handle))).map(|e| e.handle.clone()));
        if !via.is_empty() {
            esegui(s, "erase", json!({"handles": via}))?;
        }
        for (p, rot, nome) in nuove {
            metti_etichetta(s, &layer, p, rot, &nome, false)?;
        }
        etichette = blocchi_inseriti(s, BLOCCO_NUMERO)?;
        out.push((n, nomi));
    }
    Ok(out)
}

fn metti_etichetta(s: &mut Session, layer: &str, p: P, rot: f64, nome: &str, bloccato: bool) -> Result<String> {
    crea_blocco(s)?;
    let h = inserisci_su_layer(s, BLOCCO_NUMERO, p, &json!({"N": nome, "BLOCCATO": if bloccato { "1" } else { "" }}), layer)?;
    if rot != 0.0 {
        esegui(s, "rotate", json!({"handles": [h], "base": [p.0, p.1], "angle": rot}))?;
    }
    Ok(h)
}

/// Numera i fili del foglio `n` da capo (compatibilità: come la numerazione automatica senza
/// mantenere i numeri). Restituisce i nomi dati alle reti, nell'ordine.
pub fn numera_foglio(s: &mut Session, n: u32) -> Result<Vec<String>> {
    Ok(numera(s, &[n], &Opzioni::default())?.into_iter().next().map(|(_, v)| v).unwrap_or_default())
}

/// Le reti del foglio `n` che passano per il punto `p` o che la fence `da`–`a` attraversa, in
/// ordine lungo la fence; per ognuna, il punto del filo toccato.
fn reti_toccate(tutti: &[Segmento], p: Option<P>, fence: Option<(P, P)>) -> Vec<(Vec<usize>, P)> {
    let gruppi = reti(tutti);
    let punti: Vec<P> = match (p, fence) {
        (Some(p), _) => {
            // il filo più vicino al punto (entro 2 mm)
            let d = |f: &Segmento| {
                let (x0, x1) = (f.a.0.min(f.b.0), f.a.0.max(f.b.0));
                let (y0, y1) = (f.a.1.min(f.b.1), f.a.1.max(f.b.1));
                let q = (p.0.clamp(x0, x1), p.1.clamp(y0, y1));
                ((q.0 - p.0).hypot(q.1 - p.1), q)
            };
            tutti.iter().map(d).filter(|(dist, _)| *dist <= 2.0).min_by(|a, b| a.0.total_cmp(&b.0)).map(|(_, q)| vec![q]).unwrap_or_default()
        }
        (None, Some((da, a))) => {
            let mut v = crate::morsetti::attraversamenti(tutti, da, a);
            // la fence può essere anche verticale su fili orizzontali: attraversamenti lo gestisce
            v.dedup_by(|x, y| vicini(*x, *y));
            v
        }
        _ => Vec::new(),
    };
    let mut out: Vec<(Vec<usize>, P)> = Vec::new();
    for q in punti {
        if let Some(g) = gruppi.iter().find(|g| g.iter().any(|&i| tutti.get(i).is_some_and(|f| su(f, q))))
            && !out.iter().any(|(x, _)| x == g)
        {
            out.push((g.clone(), q));
        }
    }
    out
}

/// Numerazione manuale: dà alle reti toccate (punto o fence) il numero `testo` incrementato filo
/// per filo, o la stringa libera se comincia con l'apice. Restituisce i numeri dati.
pub fn numera_manuale(s: &mut Session, p: Option<P>, fence: Option<(P, P)>, testo: &str, blocca: bool) -> Result<Vec<String>> {
    let n = fogli::corrente(s)?.map(|f| f.numero).ok_or_else(|| errore("Nessun foglio corrente"))?;
    let layer = fogli::nome_layer(n);
    let imp = impostazioni::correnti();
    let tutti = fili::fili(s, &layer)?;
    let toccate = reti_toccate(&tutti, p, fence);
    if toccate.is_empty() {
        return Err(errore("Nessun filo nel punto indicato"));
    }
    let rimandi = fili::rimandi(s)?;
    let etichette = blocchi_inseriti(s, BLOCCO_NUMERO)?;
    let testo = testo.trim();
    let (libero, base) = match testo.strip_prefix('\'') {
        Some(t) => (Some(t.to_string()), 0),
        None => match testo.parse::<u32>() {
            Ok(k) => (None, k),
            // una stringa che comincia con una lettera vale com'è
            Err(_) => (Some(testo.to_string()), 0),
        },
    };
    if libero.as_ref().is_some_and(|t| t.is_empty()) {
        return Err(errore("Indicare il numero o la stringa del filo"));
    }
    let mut dati = Vec::new();
    let mut k = base;
    for (g, q) in toccate {
        let fili_rete: Vec<&Segmento> = g.iter().filter_map(|&i| tutti.get(i)).collect();
        let tocca_alim = rimandi.iter().any(|r| r.foglio == n && r.famiglia == fili::Famiglia::Alimentazione && fili_rete.iter().any(|f| vicini(f.a, r.pos) || vicini(f.b, r.pos)));
        if tocca_alim {
            // come nei CAD elettrici diffusi: il filo di un'alimentazione non si rinumera
            continue;
        }
        let vecchie: Vec<&Inserito> = etichette.iter().filter(|e| e.layer == layer && etichetta_su(e, &fili_rete)).collect();
        if vecchie.iter().any(|e| e.attributo("BLOCCATO").trim() == "1") && !blocca {
            continue;
        }
        let nome = match &libero {
            Some(t) => t.clone(),
            None => {
                let (col, _) = squadratura::colonna_riga(q.0, q.1);
                let t = formatta(&imp, n, col, k);
                k += 1;
                t
            }
        };
        let via: Vec<String> = vecchie.iter().map(|e| e.handle.clone()).collect();
        if !via.is_empty() {
            esegui(s, "erase", json!({"handles": via}))?;
        }
        // il numero accanto al tratto toccato
        let tratto = fili_rete.iter().find(|f| su(f, q)).copied();
        let (pos, rot) = match tratto {
            Some(f) if (f.a.0 - f.b.0).abs() < TOLL => ((f.a.0 - 1.0, q.1), 90.0),
            Some(f) => ((q.0, f.a.1 + 1.0), 0.0),
            None => posizione(&fili_rete).unwrap_or((q, 0.0)),
        };
        metti_etichetta(s, &layer, pos, rot, &nome, blocca)?;
        dati.push(nome);
    }
    aggiorna_morsetti(s)?;
    Ok(dati)
}

/// Blocca o sblocca i numeri dei fili toccati (punto o fence).
pub fn blocca(s: &mut Session, p: Option<P>, fence: Option<(P, P)>, bloccato: bool) -> Result<usize> {
    let n = fogli::corrente(s)?.map(|f| f.numero).ok_or_else(|| errore("Nessun foglio corrente"))?;
    let layer = fogli::nome_layer(n);
    let tutti = fili::fili(s, &layer)?;
    let etichette = blocchi_inseriti(s, BLOCCO_NUMERO)?;
    let mut k = 0;
    for (g, _) in reti_toccate(&tutti, p, fence) {
        let fili_rete: Vec<&Segmento> = g.iter().filter_map(|&i| tutti.get(i)).collect();
        for e in etichette.iter().filter(|e| e.layer == layer && etichetta_su(e, &fili_rete)) {
            esegui(s, "attedit", json!({"handle": e.handle, "values": {"BLOCCATO": if bloccato { "1" } else { "" }}}))?;
            k += 1;
        }
    }
    Ok(k)
}

/// Edita Fili: dà il numero `nome` (l'apice davanti si toglie) a tutte le etichette della rete
/// dell'etichetta `handle` e le blocca o sblocca. Un numero cambiato a mano resta bloccato se
/// non si dice altro. Restituisce le etichette cambiate.
pub fn cambia_numero(s: &mut Session, handle: &str, nome: Option<&str>, bloccato: Option<bool>) -> Result<usize> {
    let etichette = blocchi_inseriti(s, BLOCCO_NUMERO)?;
    let e = etichette.iter().find(|e| e.handle.eq_ignore_ascii_case(handle)).ok_or_else(|| errore("L'oggetto non è il numero di un filo"))?;
    let tutti = fili::fili(s, &e.layer)?;
    let rete: Vec<&Inserito> = reti(&tutti)
        .into_iter()
        .map(|g| g.into_iter().filter_map(|i| tutti.get(i)).collect::<Vec<_>>())
        .find(|g| etichetta_su(e, g))
        .map(|g| etichette.iter().filter(|x| x.layer == e.layer && etichetta_su(x, &g)).collect())
        .unwrap_or_else(|| vec![e]);
    let nome = nome.map(|n| n.trim().trim_start_matches('\'').to_string());
    if nome.as_ref().is_some_and(String::is_empty) {
        return Err(errore("Indicare il numero o la stringa del filo"));
    }
    let bloccato = bloccato.or(nome.as_ref().map(|_| true));
    let mut v = serde_json::Map::new();
    if let Some(n) = &nome {
        v.insert("N".into(), json!(n));
    }
    if let Some(b) = bloccato {
        v.insert("BLOCCATO".into(), json!(if b { "1" } else { "" }));
    }
    let handles: Vec<String> = rete.iter().map(|x| x.handle.clone()).collect();
    for h in &handles {
        esegui(s, "attedit", json!({"handle": h, "values": v}))?;
    }
    aggiorna_morsetti(s)?;
    Ok(handles.len())
}

/// Un numero di filo usato: dove, e se è ripetuto per errore.
#[derive(Clone, Debug, PartialEq)]
pub struct NumeroUsato {
    pub numero: String,
    /// Riferimenti `foglio/colonnaRiga` delle reti che lo portano.
    pub dove: Vec<String>,
    /// Viene da rimandi (alimentazioni, segnali): ripetuto per scelta.
    pub da_rimando: bool,
    pub ripetuto: bool,
}

/// I numeri dei fili usati in tutto il disegno, in ordine; segnala quelli ripetuti su reti non
/// collegate da rimandi.
pub fn numeri_usati(s: &mut Session) -> Result<Vec<NumeroUsato>> {
    let rimandi = fili::rimandi(s)?;
    let etichette = blocchi_inseriti(s, BLOCCO_NUMERO)?;
    let mut mappa: std::collections::BTreeMap<String, NumeroUsato> = Default::default();
    for f in fogli::elenco(s)? {
        let tutti = fili::fili(s, &f.layer)?;
        for r in reti_del_foglio(&tutti, &etichette, &rimandi, &f.layer, f.numero) {
            let (nome, da_rimando) = match (&r.rimando, r.etichetta()) {
                (Some(t), _) => (t.clone(), true),
                (None, Some(e)) if !e.attributo("N").trim().is_empty() => (e.attributo("N").to_string(), false),
                _ => continue,
            };
            let p = punto_rete(&r.fili);
            let u = mappa.entry(nome.clone()).or_insert(NumeroUsato { numero: nome, dove: Vec::new(), da_rimando, ripetuto: false });
            u.dove.push(squadratura::riferimento(f.numero, p.0, p.1));
            u.da_rimando |= da_rimando;
        }
    }
    let mut v: Vec<NumeroUsato> = mappa.into_values().collect();
    for u in &mut v {
        u.ripetuto = !u.da_rimando && u.dove.len() > 1;
    }
    Ok(v)
}

/// Scrive in NUMI/NUMO dei morsetti il nome della rete su cui stanno: il segnale di un rimando
/// che la tocca o il numero già scritto sul filo. Restituisce i morsetti aggiornati.
pub fn aggiorna_morsetti(s: &mut Session) -> Result<usize> {
    let morsetti = crate::morsetti::morsetti(s)?;
    let etichette = blocchi_inseriti(s, &format!("{BLOCCO_NUMERO},BLCOR*"))?;
    let rimandi = fili::rimandi(s)?;
    let mut fogli_con_morsetti: Vec<u32> = morsetti.iter().map(|m| m.foglio).filter(|f| *f > 0).collect();
    fogli_con_morsetti.sort_unstable();
    fogli_con_morsetti.dedup();
    let mut modifiche = Vec::new();
    for n in fogli_con_morsetti {
        let layer = fogli::nome_layer(n);
        let tutti = fili::fili(s, &layer)?;
        for g in reti(&tutti) {
            let g: Vec<&Segmento> = g.into_iter().filter_map(|i| tutti.get(i)).collect();
            let da_rimando = rimandi.iter().filter(|r| r.foglio == n).find(|r| g.iter().any(|f| vicini(f.a, r.pos) || vicini(f.b, r.pos))).map(|r| r.segnale.clone());
            let da_etichetta = || etichette.iter().filter(|e| e.layer == layer).find(|e| etichetta_su(e, &g)).map(|e| e.attributo("N").to_string());
            let Some(nome) = da_rimando.or_else(da_etichetta).filter(|n| !n.is_empty()) else { continue };
            for m in morsetti.iter().filter(|m| m.foglio == n && g.iter().any(|f| su(f, m.blocco.pos))) {
                if m.blocco.attributo("NUMI") != nome || m.blocco.attributo("NUMO") != nome {
                    modifiche.push((m.blocco.handle.clone(), nome.clone()));
                }
            }
        }
    }
    let quanti = modifiche.len();
    for (h, nome) in modifiche {
        esegui(s, "attedit", json!({"handle": h, "values": {"NUMI": nome, "NUMO": nome}}))?;
    }
    Ok(quanti)
}

fn run_numera(s: &mut Session, p: &Value) -> Result<Value> {
    let fogli_da_numerare: Vec<u32> = match (p.get("tutti").and_then(Value::as_bool), p.get("fogli"), numero_param(p, "foglio")) {
        (Some(true), _, _) => fogli::elenco(s)?.into_iter().map(|f| f.numero).collect(),
        (_, Some(Value::Array(a)), _) => a.iter().filter_map(|v| v.as_u64().and_then(|n| u32::try_from(n).ok())).collect(),
        (_, _, Some(n)) => vec![n],
        _ => vec![fogli::corrente(s)?.map(|f| f.numero).ok_or_else(|| errore("Nessun foglio corrente"))?],
    };
    let o = Opzioni { primo: numero_param(p, "primo"), mantieni: p.get("mantieni").and_then(Value::as_bool).unwrap_or(false) };
    in_gruppo(s, "Numera fili", |s| {
        let r = numera(s, &fogli_da_numerare, &o)?;
        let totale: usize = r.iter().map(|(_, v)| v.len()).sum();
        let per_foglio: serde_json::Map<String, Value> = r.into_iter().map(|(n, v)| (n.to_string(), json!(v))).collect();
        aggiorna_morsetti(s)?;
        Ok(json!({"fili": per_foglio, "message": format!("Numerati {totale} fili")}))
    })
}

fn run_manuale(s: &mut Session, p: &Value) -> Result<Value> {
    let numero = testo_param(p, "numero").or_else(|| p.get("numero").and_then(Value::as_u64).map(|n| n.to_string())).ok_or_else(|| errore("Indicare il numero (o 'stringa)"))?;
    let at = punto_param(p, "at");
    let fence = punto_param(p, "da").zip(punto_param(p, "a"));
    if at.is_none() && fence.is_none() {
        return Err(errore("Indicare il punto sul filo (at) o la fence (da, a)"));
    }
    let blocca_ = p.get("blocca").and_then(Value::as_bool).unwrap_or(false);
    in_gruppo(s, "Numera filo", |s| {
        let dati = numera_manuale(s, at, fence, &numero, blocca_)?;
        Ok(json!({"numeri": dati, "message": if dati.is_empty() { "Nessun filo numerato (bloccato o di un'alimentazione)".to_string() } else { format!("Fili numerati: {}", dati.join(", ")) }}))
    })
}

fn run_blocca(s: &mut Session, p: &Value) -> Result<Value> {
    let at = punto_param(p, "at");
    let fence = punto_param(p, "da").zip(punto_param(p, "a"));
    let b = p.get("bloccato").and_then(Value::as_bool).unwrap_or(true);
    in_gruppo(s, if b { "Blocca fili" } else { "Sblocca fili" }, |s| {
        let k = blocca(s, at, fence, b)?;
        Ok(json!({"fili": k, "message": format!("{} {k} numeri di filo", if b { "Bloccati" } else { "Sbloccati" })}))
    })
}

fn run_lista(s: &mut Session, _p: &Value) -> Result<Value> {
    let v = numeri_usati(s)?;
    let ripetuti: Vec<&NumeroUsato> = v.iter().filter(|u| u.ripetuto).collect();
    let righe: Vec<String> = v.iter().map(|u| format!("{}{}  {}", u.numero, if u.ripetuto { "  (ripetuto)" } else { "" }, u.dove.join(" "))).collect();
    Ok(json!({
        "numeri": v.iter().map(|u| json!({"numero": u.numero, "dove": u.dove, "rimando": u.da_rimando, "ripetuto": u.ripetuto})).collect::<Vec<_>>(),
        "ripetuti": ripetuti.iter().map(|u| u.numero.clone()).collect::<Vec<_>>(),
        "message": if righe.is_empty() { "Nessun filo numerato".to_string() } else { format!("{}\nNumeri ripetuti: {}", righe.join("\n"), ripetuti.len()) },
    }))
}

/// NUMERA FILI interattivo: numero (o 'stringa), poi un punto sul filo o due punti di fence;
/// il numero cresce filo per filo finché non si preme Invio.
struct NumeraManuale {
    numero: Option<String>,
    da: Option<P>,
}

impl Interactive for NumeraManuale {
    fn name(&self) -> &'static str {
        "numerafilo"
    }
    fn prompt(&self, _s: &Session) -> Prompt {
        match (&self.numero, self.da) {
            (None, _) => Prompt::new("Numero del filo (o 'stringa)", Accept::TEXT),
            (Some(n), None) => Prompt::new(format!("Filo da numerare {n} (punto sul filo, o Fence)"), Accept::POINT).kw(&["Fence"]),
            (Some(_), Some(p)) => Prompt::new("Secondo punto della fence", Accept::POINT).base(cadcraft_engine::geom::Vec2::new(p.0, p.1)),
        }
    }
    fn input(&mut self, s: &mut Session, i: Input) -> Result<Step> {
        match i {
            Input::Cancel => Ok(Step::Cancel),
            Input::Enter => Ok(Step::Done),
            Input::Text(t) if self.numero.is_none() => {
                if !t.trim().is_empty() {
                    self.numero = Some(t.trim().to_string());
                }
                Ok(Step::Continue)
            }
            Input::Keyword(_) | Input::Text(_) => {
                // "Fence": il prossimo punto è il primo della fence
                self.da = Some((f64::NAN, f64::NAN));
                Ok(Step::Continue)
            }
            Input::Point(q) => {
                let Some(numero) = self.numero.clone() else { return Ok(Step::Continue) };
                let q = (q.x, q.y);
                let p = match self.da {
                    Some(d) if d.0.is_nan() => {
                        self.da = Some(q);
                        return Ok(Step::Continue);
                    }
                    Some(d) => {
                        self.da = None;
                        json!({"numero": numero, "da": [d.0, d.1], "a": [q.0, q.1]})
                    }
                    None => json!({"numero": numero, "at": [q.0, q.1]}),
                };
                let r = s.execute("numerafilo", &p)?;
                if let Some(m) = r.get("message").and_then(Value::as_str) {
                    s.echo(m.to_string());
                }
                // il prossimo filo prende il numero seguente (le stringhe restano uguali)
                let dati = r.get("numeri").and_then(Value::as_array).map_or(0, Vec::len);
                if let Ok(k) = numero.parse::<u32>() {
                    self.numero = Some((k + u32::try_from(dati).unwrap_or(0)).to_string());
                }
                Ok(Step::Continue)
            }
            _ => Ok(Step::Continue),
        }
    }
}

fn fabbrica_blocca(_s: &Session) -> Result<Box<dyn Interactive>> {
    Ok(Box::new(crate::util::PuntoOFence::nuovo("bloccafili", "Filo da bloccare", "bloccafili", json!({"bloccato": true}))))
}

fn fabbrica_manuale(_s: &Session) -> Result<Box<dyn Interactive>> {
    Ok(Box::new(NumeraManuale { numero: None, da: None }))
}

pub fn comandi() -> Vec<CommandSpec> {
    let spec = |id, label, menu, params, run, interactive| CommandSpec { id, label, menu, shortcut: None, aliases: &[], params, enabled: has_doc, run, undoable: false, interactive, transparent: false };
    vec![
        spec(
            "numerafili",
            "Numerazione Automatica Fili",
            &["Elettrico", "Numerazione Automatica Fili"],
            "{foglio?: numero (default il corrente) | fogli?: [numeri] | tutti?: true, primo?: numero di partenza, mantieni?: bool (i fili già numerati conservano il numero)} → {fili: {foglio: [nomi]}}",
            run_numera as cadcraft_engine::cmd::Run,
            None,
        ),
        spec(
            "numerafilo",
            "Numera Filo",
            &["Elettrico", "Numera Filo"],
            "{numero: \"12\" (incrementa filo per filo) | \"'R1\" (stringa libera), at: [x,y] | da + a (fence), blocca?: bool} → {numeri}",
            run_manuale,
            Some(fabbrica_manuale as cadcraft_engine::cmd::Factory),
        ),
        spec("bloccafili", "Blocca / Sblocca Fili", &["Elettrico", "Strumenti Fili", "Blocca / Sblocca Fili"], "{at | da + a, bloccato?: bool (default true)}", run_blocca, Some(fabbrica_blocca)),
        spec("listanumerifili", "Lista numeri usati", &["Elettrico", "Strumenti Fili", "Lista numeri usati"], "{} → {numeri: [{numero, dove, rimando, ripetuto}], ripetuti}", run_lista, None),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seg(a: P, b: P) -> Segmento {
        Segmento { a, b, handle: String::new() }
    }

    #[test]
    fn reti_con_t_e_incroci() {
        let f = vec![
            seg((0.0, 100.0), (100.0, 100.0)), // barra
            seg((50.0, 100.0), (50.0, 50.0)),  // calata a T dalla barra
            seg((70.0, 120.0), (70.0, 60.0)),  // incrocia la barra senza collegarsi
            seg((50.0, 40.0), (50.0, 10.0)),   // dopo un simbolo
        ];
        let r = reti(&f);
        assert_eq!(r.len(), 3);
        assert!(r.contains(&vec![0, 1]));
    }

    #[test]
    fn formati_dei_numeri() {
        let mut imp = Impostazioni::default();
        assert_eq!(formatta(&imp, 5, 3, 7), "05.07");
        assert_eq!(progressivo(&imp, "05.07"), Some(7));
        imp.numeri_fili = NumeriFili::Progressivo;
        imp.cifre_filo = 3;
        imp.prefisso_fili = "W".into();
        assert_eq!(formatta(&imp, 5, 3, 7), "W007");
        assert_eq!(progressivo(&imp, "W007"), Some(7));
        imp.numeri_fili = NumeriFili::Posizionale;
        imp.prefisso_fili.clear();
        assert_eq!(formatta(&imp, 1, 1, 0), "011.0");
    }
}
