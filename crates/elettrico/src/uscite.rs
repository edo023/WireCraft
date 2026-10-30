//! Uscite disegnate sui fogli: lista dei fogli, distinta materiali, morsettiere, tabella di
//! cablaggio, legenda delle funzioni e dei simboli. Come nei CAD elettrici diffusi ogni uscita
//! occupa uno o più fogli consecutivi ed è **aggiornabile**: ogni pagina è un blocco
//! `CE_USCITA_<foglio>` con gli attributi `TIPO` e `PARAMETRI`, e "Aggiorna Liste e Tabelle" le
//! ridisegna con i dati del momento.

use cadcraft_engine::cmd::has_doc;
use cadcraft_engine::{CommandSpec, Result, Session};
use serde_json::{Value, json};

use crate::util::{Inserito, blocchi_inseriti, blocco, colore, errore, esegui, handles, in_gruppo, inserisci_su_layer, linea, numero_param, su_layer_zero, testo, testo_param};
use crate::{collegamenti, distinta, fogli, morsetti, multifoglio, siglatura, simboli, squadratura};

type P = (f64, f64);

/// Angolo in alto a sinistra dell'area delle tabelle: dentro la cornice, sotto la fascia delle
/// descrizioni delle colonne e sopra il cartiglio.
const ORIGINE: P = (20.0, squadratura::CORNICE_MAX.1 - squadratura::FASCIA_DESCRIZIONI - 2.5);
const LARGHEZZA_AREA: f64 = 390.0;
/// Altezza utile: fino a 5 mm sopra il cartiglio.
const ALTEZZA_AREA: f64 = ORIGINE.1 - squadratura::CORNICE_MIN.1 - squadratura::CARTIGLIO_ALTEZZA - 5.0;
/// Larghezza e altezza dell'area di una pagina di uscita (mm).
pub const AREA: P = (LARGHEZZA_AREA, ALTEZZA_AREA);
const ALTEZZA_TITOLO: f64 = 9.0;
const ALTEZZA_INTESTAZIONE: f64 = 8.0;
const TESTO: f64 = 2.5;
const PREFISSO_BLOCCO: &str = "CE_USCITA_";

/// Una cella: testo, o il simbolo di un blocco (legenda dei simboli).
#[derive(Clone, Debug, PartialEq)]
pub enum Cella {
    T(String),
    Blocco(String),
}

impl From<String> for Cella {
    fn from(t: String) -> Self {
        Cella::T(t)
    }
}

impl From<&str> for Cella {
    fn from(t: &str) -> Self {
        Cella::T(t.to_string())
    }
}

#[derive(Clone, Debug)]
pub struct Tabella {
    pub titolo: String,
    /// Titolo e larghezza (mm) delle colonne.
    pub colonne: Vec<(String, f64)>,
    pub righe: Vec<Vec<Cella>>,
    pub altezza_riga: f64,
}

impl Tabella {
    pub fn larghezza(&self) -> f64 {
        self.colonne.iter().map(|c| c.1).sum()
    }
    /// Righe in una colonna di pagina.
    fn righe_per_blocco(&self) -> usize {
        ((ALTEZZA_AREA - ALTEZZA_TITOLO - ALTEZZA_INTESTAZIONE) / self.altezza_riga).floor().max(1.0) as usize
    }
    /// Tabelle affiancate in una pagina (come la lista fogli a due colonne).
    fn blocchi_per_pagina(&self) -> usize {
        ((LARGHEZZA_AREA + 4.0) / (self.larghezza() + 4.0)).floor().clamp(1.0, 3.0) as usize
    }
    /// Le pagine: per ognuna i gruppi di righe affiancati.
    pub fn pagine(&self) -> Vec<Vec<std::ops::Range<usize>>> {
        let n = self.righe_per_blocco();
        let k = self.blocchi_per_pagina();
        let gruppi: Vec<std::ops::Range<usize>> = (0..self.righe.len().max(1)).step_by(n).map(|i| i..(i + n).min(self.righe.len())).collect();
        gruppi.chunks(k).map(<[std::ops::Range<usize>]>::to_vec).collect()
    }
}

/// Un testo accorciato per stare in `larghezza` mm (con altezza `h`).
fn adatta(t: &str, larghezza: f64, h: f64) -> String {
    let max = ((larghezza - 1.5) / (h * 0.8)).floor().max(1.0) as usize;
    if t.chars().count() <= max {
        return t.to_string();
    }
    let mut s: String = t.chars().take(max.saturating_sub(1)).collect();
    s.push('…');
    s
}

/// Disegna (in coordinate del blocco: origine in alto a sinistra) una pagina della tabella.
fn disegna_pagina(s: &mut Session, t: &Tabella, gruppi: &[std::ops::Range<usize>], h: &mut Vec<String>) -> Result<()> {
    colore(s, "7")?;
    let largo = t.larghezza();
    let k = gruppi.len().max(1) as f64;
    let totale = largo * k + 4.0 * (k - 1.0);
    testo(s, h, (totale / 2.0, -ALTEZZA_TITOLO / 2.0), &t.titolo, 4.0, "MC")?;
    for (j, g) in gruppi.iter().enumerate() {
        let ox = j as f64 * (largo + 4.0);
        let y0 = -ALTEZZA_TITOLO;
        let y1 = y0 - ALTEZZA_INTESTAZIONE;
        let fondo = y1 - t.altezza_riga * g.len() as f64;
        // cornice, intestazione e righe
        linea(s, h, &[(ox, y0), (ox + largo, y0), (ox + largo, fondo), (ox, fondo)], true)?;
        linea(s, h, &[(ox, y1), (ox + largo, y1)], false)?;
        let mut x = ox;
        for (titolo, w) in &t.colonne {
            for (r, riga) in titolo.split('\n').enumerate() {
                testo(s, h, (x + 1.5, y0 - 3.2 - 3.0 * r as f64), &adatta(riga, *w, TESTO), TESTO, "L")?;
            }
            x += w;
            if x < ox + largo - 0.01 {
                linea(s, h, &[(x, y0), (x, fondo)], false)?;
            }
        }
        for (k, i) in g.clone().enumerate() {
            let yr = y1 - t.altezza_riga * k as f64;
            if k > 0 {
                linea(s, h, &[(ox, yr), (ox + largo, yr)], false)?;
            }
            let mut x = ox;
            for ((_, w), cella) in t.colonne.iter().zip(t.righe.get(i).into_iter().flatten()) {
                match cella {
                    Cella::T(v) if !v.is_empty() => testo(s, h, (x + 1.5, yr - t.altezza_riga / 2.0), &adatta(v, *w, TESTO), TESTO, "ML")?,
                    Cella::Blocco(b) => {
                        if let Some(sim) = simboli::trova(b) {
                            let (x0, x1, ya, yb) = simboli::estensione(&(sim.grafica)());
                            let scala = ((t.altezza_riga - 2.0) / (yb - ya).max(1.0)).min((w - 4.0) / (x1 - x0).max(1.0)).min(1.0);
                            let at = (x + w / 2.0 - (x0 + x1) / 2.0 * scala, yr - t.altezza_riga / 2.0 - (ya + yb) / 2.0 * scala);
                            let r = esegui(s, "insert", json!({"name": sim.blocco, "at": [at.0, at.1], "scale": scala}))?;
                            h.extend(handles(&r));
                        }
                    }
                    Cella::T(_) => {}
                }
                x += w;
            }
        }
    }
    Ok(())
}

/// Le pagine di un'uscita già disegnate: (blocco inserito, foglio).
fn pagine_esistenti(s: &mut Session) -> Result<Vec<(Inserito, u32)>> {
    Ok(blocchi_inseriti(s, &format!("{PREFISSO_BLOCCO}*"))?.into_iter().filter_map(|b| fogli::numero_da_layer(&b.layer).map(|n| (b, n))).collect())
}

/// Il foglio `n` contiene oggetti che non sono numeri, titolo o pagine di uscite.
fn occupato(s: &Session, n: u32) -> Result<bool> {
    use cadcraft_engine::doc::EntityKind;
    let layer = fogli::nome_layer(n);
    Ok(s.doc()?.model.iter().any(|e| {
        e.common.layer == layer
            && !matches!(&e.kind, EntityKind::Insert(i) if {
                let b = i.block.to_ascii_uppercase();
                b.starts_with("NUMFG") || b.starts_with("TITOLO") || b.starts_with(PREFISSO_BLOCCO)
            })
    }))
}

/// Disegna la tabella sui fogli da `primo` in poi (default dopo l'ultimo): toglie le pagine di
/// uscite che c'erano su quei fogli, crea i fogli che mancano (o li inserisce se il numero è di
/// un foglio con altro disegno) e dà loro il titolo. Restituisce i fogli usati.
pub fn disegna(s: &mut Session, t: &Tabella, tipo: &str, parametri: &Value, primo: Option<u32>, titolo_foglio: &str, titolo_foglio1: &str) -> Result<Vec<u32>> {
    let pagine = t.pagine();
    disegna_con(s, pagine.len(), tipo, parametri, primo, titolo_foglio, titolo_foglio1, |s, k, h| disegna_pagina(s, t, &pagine[k], h))
}

/// Come [`disegna`], per un'uscita di `n_pagine` disegnate da `pagina(s, k, handles)` in
/// coordinate del blocco (origine in alto a sinistra dell'area, [`AREA`] mm).
#[allow(clippy::too_many_arguments)]
pub fn disegna_con(
    s: &mut Session,
    n_pagine: usize,
    tipo: &str,
    parametri: &Value,
    primo: Option<u32>,
    titolo_foglio: &str,
    titolo_foglio1: &str,
    mut pagina: impl FnMut(&mut Session, usize, &mut Vec<String>) -> Result<()>,
) -> Result<Vec<u32>> {
    let n_pagine = n_pagine.max(1);
    let ultimo = fogli::elenco(s)?.last().map_or(0, |f| f.numero);
    let mut usati = Vec::new();
    let esistenti = pagine_esistenti(s)?;
    for (n, k) in (primo.unwrap_or(ultimo + 1)..).zip(0..n_pagine) {
        let esiste = fogli::elenco(s)?.iter().any(|f| f.numero == n);
        if esiste && occupato(s, n)? && k > 0 {
            // il foglio che segue ha altro disegno: se ne inserisce uno nuovo prima
            multifoglio::inserisci_foglio(s, n)?;
        } else if !esiste {
            fogli::crea(s, n)?;
        }
        let layer = fogli::nome_layer(n);
        let via: Vec<String> = esistenti.iter().filter(|(_, f)| *f == n).map(|(b, _)| b.handle.clone()).collect();
        if !via.is_empty() {
            esegui(s, "erase", json!({"handles": via}))?;
        }
        let nome = format!("{PREFISSO_BLOCCO}{n}");
        su_layer_zero(s, |s| {
            let mut h = Vec::new();
            pagina(s, k, &mut h)?;
            for (tag, valore) in [("TIPO", tipo.to_string()), ("PARAMETRI", parametri.to_string()), ("PAGINA", format!("{}/{}", k + 1, n_pagine))] {
                h.extend(handles(&esegui(s, "attdef", json!({"tag": tag, "prompt": tag, "default": valore, "at": [0.0, 0.0], "height": 1.0, "invisible": true}))?));
            }
            blocco(s, &nome, h, "Uscita aggiornabile")
        })?;
        inserisci_su_layer(s, &nome, ORIGINE, &json!({"TIPO": tipo, "PARAMETRI": parametri.to_string(), "PAGINA": format!("{}/{}", k + 1, n_pagine)}), &layer)?;
        if !titolo_foglio.is_empty() {
            fogli::imposta_titolo(s, n, titolo_foglio, Some(titolo_foglio1))?;
        }
        usati.push(n);
    }
    fogli::numera(s)?;
    Ok(usati)
}

// ---------- le uscite ----------

/// Tipi di uscita.
pub const TIPI: [(&str, &str); 16] = [
    ("indice", "Lista fogli"),
    ("distinta", "Distinta materiali"),
    ("morsettiere", "Morsettiere"),
    ("disegnomorsettiere", "Disegno morsettiere"),
    ("cablaggio", "Tabella di cablaggio"),
    ("cavi", "Lista cavi"),
    ("conduttoricavi", "Tabella conduttori dei cavi"),
    ("io", "Lista I/O PLC"),
    ("revisioni", "Legenda revisioni"),
    ("funzioni", "Legenda funzioni"),
    ("simboli", "Legenda simboli"),
    ("termico", "Calcolo termico del quadro"),
    ("loop", "Loop diagram"),
    ("pulsantiera", "Pulsantiera"),
    ("circuiti", "Tabella dei circuiti"),
    ("impianto", "Schema d'impianto"),
];

/// La tabella di un'uscita con i dati del disegno, e i titoli del foglio (due lingue).
pub fn tabella(s: &mut Session, tipo: &str, p: &Value) -> Result<(Tabella, &'static str, &'static str)> {
    let quadro = testo_param(p, "quadro").unwrap_or_default();
    let del_quadro = |q: &str| quadro.trim().is_empty() || q.trim().eq_ignore_ascii_case(quadro.trim());
    let col = |v: &[(&str, f64)]| v.iter().map(|(t, w)| ((*t).to_string(), *w)).collect::<Vec<_>>();
    match tipo {
        "indice" => {
            let righe = fogli::elenco(s)?
                .into_iter()
                .map(|f| {
                    let d = if f.titolo1.trim().is_empty() { f.titolo.clone() } else { format!("{} / {}", f.titolo, f.titolo1) };
                    vec![Cella::T(format!("{:02}", f.numero)), Cella::T(d)]
                })
                .collect();
            Ok((Tabella { titolo: "LISTA FOGLI \\ INDEX".into(), colonne: col(&[("Foglio\nSheet", 18.0), ("Descrizione\nDescription", 165.0)]), righe, altezza_riga: 6.0 }, "Lista fogli", "Sheets Index"))
        }
        "distinta" => {
            let righe = distinta::materiali(s)?
                .into_iter()
                .filter(|m| del_quadro(&m.quadro))
                .map(|m| vec![m.sigla.into(), m.codice.into(), m.descrizione.into(), m.costruttore.into(), m.quadro.into(), Cella::T(if m.foglio > 0 { format!("{:02}", m.foglio) } else { String::new() }), "1".into()])
                .collect();
            let titolo = if quadro.is_empty() { "DISTINTA MATERIALI \\ BILL OF MATERIALS".to_string() } else { format!("DISTINTA MATERIALI QUADRO {quadro} \\ {quadro} BILL OF MATERIALS") };
            Ok((
                Tabella {
                    titolo,
                    colonne: col(&[("Nome\nItem", 32.0), ("Tipo\nType", 45.0), ("Descrizione\nDescription", 170.0), ("Costruttore\nManufacturer", 50.0), ("Quadro\nBoard", 28.0), ("Fg\nSh", 14.0), ("Q.tà\nQ.ty", 16.0)]),
                    righe,
                    altezza_riga: 5.0,
                },
                "Distinta materiali",
                "Bill of Materials",
            ))
        }
        "morsettiere" => {
            let reti = collegamenti::reti(s, &[])?;
            let mut tutte = morsetti::morsetti(s)?;
            tutte.retain(|m| del_quadro(m.blocco.attributo("QUADRO")) && testo_param(p, "morsettiera").is_none_or(|x| x.eq_ignore_ascii_case(&m.morsettiera)));
            tutte.sort_by(|a, b| {
                (a.blocco.attributo("QUADRO"), a.morsettiera.as_str())
                    .cmp(&(b.blocco.attributo("QUADRO"), b.morsettiera.as_str()))
                    .then_with(|| a.numero.parse::<u64>().ok().cmp(&b.numero.parse::<u64>().ok()))
                    .then_with(|| a.numero.cmp(&b.numero))
            });
            let mut righe: Vec<Vec<Cella>> = Vec::new();
            let mut prec = String::new();
            for m in &tutte {
                let chiave = format!("{} {}", m.blocco.attributo("QUADRO"), m.morsettiera);
                if chiave != prec {
                    righe.push(vec![Cella::T(chiave.trim().to_string()), "".into(), "".into(), "".into(), "".into(), "".into(), "".into()]);
                    prec = chiave;
                }
                // i componenti collegati sopra e sotto il morsetto, sulla sua rete
                let rete = reti.iter().find(|r| r.foglio == m.foglio && r.capi.iter().any(|c| c.tipo == collegamenti::TipoCapo::Morsetto && c.pos == m.blocco.pos));
                let (sopra, sotto) = rete.map_or((String::new(), String::new()), |r| {
                    let altri = |su: bool| {
                        r.capi.iter().filter(|c| c.tipo == collegamenti::TipoCapo::Componente && (c.pos.1 > m.blocco.pos.1) == su).map(collegamenti::Capo::testo).collect::<Vec<_>>().join(" ")
                    };
                    (altri(true), altri(false))
                });
                righe.push(vec![
                    sopra.into(),
                    m.blocco.attributo("NUMI").into(),
                    m.numero.clone().into(),
                    m.blocco.attributo("NUMO").into(),
                    sotto.into(),
                    m.blocco.attributo("PONTE").into(),
                    squadratura::riferimento(m.foglio, m.blocco.pos.0, m.blocco.pos.1).into(),
                ]);
            }
            Ok((
                Tabella {
                    titolo: "MORSETTIERE \\ TERMINAL STRIPS".into(),
                    colonne: col(&[("Da\nFrom", 60.0), ("NumI", 22.0), ("Morsetto\nTerminal", 22.0), ("NumO", 22.0), ("A\nTo", 60.0), ("Ponte\nBridge", 16.0), ("Posizione\nPosition", 22.0)]),
                    righe,
                    altezza_riga: 5.0,
                },
                "Morsettiere",
                "Terminal Strips",
            ))
        }
        "cablaggio" => {
            let r = collegamenti::reti(s, &[])?;
            let righe = collegamenti::righe_cablaggio(&collegamenti::cablaggio(&r)).into_iter().filter(|r| del_quadro(&r[0]) || del_quadro(&r[8])).map(|r| r.into_iter().map(Cella::T).collect()).collect();
            Ok((
                Tabella {
                    titolo: "TABELLA DI CABLAGGIO \\ CABLING TABLE".into(),
                    colonne: col(&[
                        ("Da quadro\nFrom board", 28.0),
                        ("Da comp.\nFrom", 36.0),
                        ("Pin", 18.0),
                        ("Filo\nWire", 28.0),
                        ("Sezione\nSection", 26.0),
                        ("Colore\nColour", 20.0),
                        ("A comp.\nTo", 36.0),
                        ("Pin", 18.0),
                        ("A quadro\nTo board", 28.0),
                        ("Fg\nSh", 14.0),
                    ]),
                    righe,
                    altezza_riga: 5.0,
                },
                "Tabella di cablaggio",
                "Cabling Table",
            ))
        }
        "cavi" => {
            let pose = crate::cavi::pose(s)?;
            let righe = crate::cavi::cavi(s)?
                .into_iter()
                .filter(|c| del_quadro(&c.da_quadro) || del_quadro(&c.a_quadro))
                .map(|c| {
                    let usate = pose.iter().filter(|p| p.cavo.eq_ignore_ascii_case(&c.sigla)).count();
                    vec![
                        c.sigla.into(),
                        c.formazione.into(),
                        c.codice.into(),
                        c.funzione.into(),
                        c.da_quadro.into(),
                        c.a_quadro.into(),
                        Cella::T(format!("{usate}/{}", c.vene.len())),
                        c.lunghezza.into(),
                    ]
                })
                .collect();
            Ok((
                Tabella {
                    titolo: "LISTA CAVI \\ CABLES LIST".into(),
                    colonne: col(&[
                        ("Cavo\nCable", 28.0),
                        ("Formazione\nCores", 30.0),
                        ("Tipo\nType", 50.0),
                        ("Funzione\nFunction", 100.0),
                        ("Da\nFrom", 30.0),
                        ("A\nTo", 30.0),
                        ("Vene\nUsed", 20.0),
                        ("Lungh. m\nLength", 22.0),
                    ]),
                    righe,
                    altezza_riga: 5.0,
                },
                "Lista cavi",
                "Cables List",
            ))
        }
        "conduttoricavi" => {
            let righe = crate::cavi::conduttori(s)?.into_iter().map(|c| vec![c.cavo.into(), c.vena.into(), c.filo.into(), c.da.into(), c.a.into(), c.posizione.into()]).collect();
            Ok((
                Tabella {
                    titolo: "CONDUTTORI DEI CAVI \\ CABLE CORES".into(),
                    colonne: col(&[("Cavo\nCable", 28.0), ("Vena\nCore", 20.0), ("Filo\nWire", 28.0), ("Da\nFrom", 50.0), ("A\nTo", 50.0), ("Posizione\nPosition", 22.0)]),
                    righe,
                    altezza_riga: 5.0,
                },
                "Conduttori dei cavi",
                "Cable Cores",
            ))
        }
        "io" => {
            let righe = crate::plc::elenco_io(s)?.into_iter().filter(|r| del_quadro(&r[6])).map(|r| r.into_iter().map(Cella::T).collect()).collect();
            Ok((
                Tabella {
                    titolo: "LISTA I/O PLC \\ PLC I/O LIST".into(),
                    colonne: col(&[
                        ("Indirizzo\nAddress", 24.0),
                        ("I/O", 10.0),
                        ("Commento", 90.0),
                        ("Comment", 90.0),
                        ("Scheda\nCard", 26.0),
                        ("Morsetto\nTerm.", 18.0),
                        ("Quadro\nBoard", 20.0),
                        ("Pos.", 18.0),
                    ]),
                    righe,
                    altezza_riga: 5.0,
                },
                "Lista I/O PLC",
                "PLC I/O List",
            ))
        }
        "revisioni" => {
            let righe = crate::revisioni::elenco(s)?
                .into_iter()
                .map(|r| vec![Cella::T(format!("{:02}", r.foglio)), r.numero.into(), r.modifica.into(), r.data.into(), r.firma.into()])
                .collect();
            Ok((
                Tabella {
                    titolo: "LEGENDA REVISIONI \\ REVISIONS".into(),
                    colonne: col(&[("Foglio\nSheet", 18.0), ("Rev.", 14.0), ("Modifica\nChange", 110.0), ("Data\nDate", 28.0), ("Firma\nSign", 28.0)]),
                    righe,
                    altezza_riga: 5.0,
                },
                "Legenda revisioni",
                "Revisions",
            ))
        }
        "funzioni" => {
            let mut v: Vec<siglatura::Componente> = siglatura::componenti(s)?.into_iter().filter(|c| c.madre() && del_quadro(c.blocco.attributo("QUADRO"))).collect();
            v.sort_by(|a, b| a.nome.cmp(&b.nome));
            let righe = v
                .iter()
                .filter(|c| !c.blocco.attributo("FUNZIONE1").trim().is_empty() || !c.blocco.attributo("FUNZIONE2").trim().is_empty())
                .map(|c| {
                    vec![
                        c.nome.clone().into(),
                        c.blocco.attributo("FUNZIONE1").into(),
                        c.blocco.attributo("FUNZIONE2").into(),
                        squadratura::riferimento(c.foglio, c.blocco.pos.0, c.blocco.pos.1).into(),
                    ]
                })
                .collect();
            Ok((
                Tabella {
                    titolo: "LISTA FUNZIONI \\ FUNCTIONS LIST".into(),
                    colonne: col(&[("Componente\nComponent", 30.0), ("Funzione", 70.0), ("Function", 70.0), ("Ref.", 18.0)]),
                    righe,
                    altezza_riga: 5.0,
                },
                "Legenda funzioni",
                "Functions List",
            ))
        }
        "simboli" => {
            let usati: Vec<siglatura::Componente> = siglatura::componenti(s)?;
            let mut righe = Vec::new();
            for sim in simboli::LIBRERIA {
                let primo = usati.iter().filter(|c| c.blocco.nome.eq_ignore_ascii_case(sim.blocco)).min_by_key(|c| (c.foglio, (c.blocco.pos.0 * 10.0) as i64));
                if let Some(c) = primo {
                    righe.push(vec![Cella::Blocco(sim.id.into()), sim.etichetta.into(), sim.categoria.into(), squadratura::riferimento(c.foglio, c.blocco.pos.0, c.blocco.pos.1).into()]);
                }
            }
            Ok((
                Tabella {
                    titolo: "LEGENDA SIMBOLI \\ SYMBOLS LEGEND".into(),
                    colonne: col(&[("Simbolo\nSymbol", 30.0), ("Descrizione\nDescription", 80.0), ("Categoria\nCategory", 52.0), ("Ref.", 18.0)]),
                    righe,
                    altezza_riga: 14.0,
                },
                "Legenda simboli",
                "Symbols Legend",
            ))
        }
        "loop" => Ok((crate::loopdiagram::tabella(s)?, "Loop diagram", "Loop Diagram")),
        "pulsantiera" => Ok((crate::pulsantiera::tabella(s, p)?, "Pulsantiera", "Control Panel")),
        "circuiti" => Ok((crate::unifilare::tabella(s, p)?, "Tabella dei circuiti", "Circuit Schedule")),
        "disegnomorsettiere" => Ok((crate::morsettiere::tabella(s, p)?, "Disegno morsettiere", "Terminal strips")),
        "impianto" => Ok((crate::impianto::tabella(s, p)?, "Schema d'impianto", "Plant Diagram")),
        "termico" => {
            let quadro = if quadro.trim().is_empty() { crate::impostazioni::quadro_corrente().unwrap_or_default() } else { quadro.clone() };
            let (d, e, voci) = crate::termico::calcolo(s, &quadro, p)?;
            let righe = crate::termico::resoconto(&quadro, &d, &e, &voci).into_iter().map(|r| r.into_iter().map(Cella::T).collect()).collect();
            let titolo = if quadro.is_empty() { "CALCOLO TERMICO \\ THERMAL CALCULATION".to_string() } else { format!("CALCOLO TERMICO QUADRO {quadro} \\ {quadro} THERMAL CALCULATION") };
            Ok((Tabella { titolo, colonne: col(&[("Voce\nItem", 55.0), ("Valore\nValue", 50.0), ("Note\nNotes", 110.0)]), righe, altezza_riga: 6.0 }, "Calcolo termico", "Thermal Calculation"))
        }
        _ => Err(errore(format!("Uscita sconosciuta: {tipo} ({})", TIPI.map(|t| t.0).join(", ")))),
    }
}

/// Disegna l'uscita `tipo` (parametri: quadro, morsettiera) dal foglio `primo`.
pub fn uscita(s: &mut Session, tipo: &str, p: &Value, primo: Option<u32>) -> Result<Vec<u32>> {
    // il loop diagram è un disegno, non una tabella
    if tipo == "loop" && !p.get("tabella").and_then(Value::as_bool).unwrap_or(false) {
        return crate::loopdiagram::uscita(s, primo);
    }
    if tipo == "disegnomorsettiere" && !p.get("tabella").and_then(Value::as_bool).unwrap_or(false) {
        return crate::morsettiere::uscita(s, p, primo);
    }
    if tipo == "impianto" && !p.get("tabella").and_then(Value::as_bool).unwrap_or(false) {
        return crate::impianto::uscita(s, primo);
    }
    let (t, t0, t1) = tabella(s, tipo, p)?;
    let mut parametri = serde_json::Map::new();
    for k in ["quadro", "morsettiera"] {
        if let Some(v) = testo_param(p, k).filter(|v| !v.is_empty()) {
            parametri.insert(k.into(), json!(v));
        }
    }
    // i dati dell'armadio del calcolo termico
    for k in ["larghezza", "altezza", "profondita", "installazione", "materiale", "tEsterna", "tMassima", "tEsternaMin", "tMinima", "tabella"] {
        if let Some(v) = p.get(k).filter(|v| !v.is_null()) {
            parametri.insert(k.into(), v.clone());
        }
    }
    disegna(s, &t, tipo, &Value::Object(parametri), primo, t0, t1)
}

/// Ridisegna tutte le uscite già disegnate con i dati del momento. Restituisce (tipo, fogli).
pub fn aggiorna_liste(s: &mut Session) -> Result<Vec<(String, Vec<u32>)>> {
    let mut gruppi: Vec<(String, String, u32)> = Vec::new();
    for (b, n) in pagine_esistenti(s)? {
        let tipo = b.attributo("TIPO").to_string();
        let par = b.attributo("PARAMETRI").to_string();
        match gruppi.iter_mut().find(|(t, p, _)| *t == tipo && *p == par) {
            Some(g) => g.2 = g.2.min(n),
            None => gruppi.push((tipo, par, n)),
        }
    }
    gruppi.sort_by_key(|g| g.2);
    let mut out = Vec::new();
    for (tipo, par, primo) in gruppi {
        // le pagine vecchie dell'uscita se ne vanno: la nuova può essere più corta o più lunga
        let vecchie: Vec<String> = pagine_esistenti(s)?.into_iter().filter(|(b, _)| b.attributo("TIPO") == tipo && b.attributo("PARAMETRI") == par).map(|(b, _)| b.handle).collect();
        if !vecchie.is_empty() {
            esegui(s, "erase", json!({"handles": vecchie}))?;
        }
        let p: Value = serde_json::from_str(&par).unwrap_or_else(|_| json!({}));
        let fogli_usati = uscita(s, &tipo, &p, Some(primo))?;
        out.push((tipo, fogli_usati));
    }
    Ok(out)
}

/// Le pagine già disegnate della stessa uscita (tipo, quadro, morsettiera, disegno o tabella):
/// (handle, foglio).
fn stessa_uscita(s: &mut Session, tipo: &str, p: &Value) -> Result<Vec<(String, u32)>> {
    let chiave = |v: &Value, k: &str| v.get(k).and_then(Value::as_str).unwrap_or_default().trim().to_ascii_uppercase();
    let tabella = |v: &Value| v.get("tabella").and_then(Value::as_bool).unwrap_or(false);
    Ok(pagine_esistenti(s)?
        .into_iter()
        .filter(|(b, _)| b.attributo("TIPO") == tipo)
        .filter(|(b, _)| {
            let par: Value = serde_json::from_str(b.attributo("PARAMETRI")).unwrap_or_else(|_| json!({}));
            ["quadro", "morsettiera"].iter().all(|k| chiave(&par, k) == chiave(p, k)) && tabella(&par) == tabella(p)
        })
        .map(|(b, n)| (b.handle, n))
        .collect())
}

fn run_uscita(s: &mut Session, p: &Value) -> Result<Value> {
    let tipo = testo_param(p, "tipo").ok_or_else(|| errore(format!("Indicare il tipo di uscita: {}", TIPI.map(|t| t.0).join(", "))))?;
    in_gruppo(s, "Uscita", |s| {
        // la stessa uscita già sui fogli si ridisegna al suo posto, non se ne fa una seconda: le
        // sue pagine sui fogli ridisegnati se ne vanno col disegno, quelle avanzate dopo
        let dato = numero_param(p, "foglio");
        let primo = match dato {
            Some(n) => Some(n),
            None => stessa_uscita(s, &tipo, p)?.iter().map(|(_, n)| *n).min(),
        };
        let f = uscita(s, &tipo, p, primo)?;
        if dato.is_none() {
            let avanzi: Vec<String> = stessa_uscita(s, &tipo, p)?.into_iter().filter(|(_, n)| !f.contains(n)).map(|(h, _)| h).collect();
            if !avanzi.is_empty() {
                esegui(s, "erase", json!({"handles": avanzi}))?;
            }
        }
        let nome = TIPI.iter().find(|t| t.0 == tipo).map_or(tipo.as_str(), |t| t.1);
        Ok(json!({"fogli": f, "message": format!("{nome}: fogli {}", f.iter().map(|n| format!("{n:02}")).collect::<Vec<_>>().join(", "))}))
    })
}

fn run_aggiorna(s: &mut Session, _p: &Value) -> Result<Value> {
    in_gruppo(s, "Aggiorna liste e tabelle", |s| {
        crate::norma81346::applica(s)?;
        crate::unifilare::aggiorna(s)?;
        crate::topografico::aggiorna(s, false)?;
        crate::pulsantiera::aggiorna(s, false)?;
        crate::siglatura::aggiorna_tutto(s)?;
        crate::fili::aggiorna_rimandi(s)?;
        let v = aggiorna_liste(s)?;
        let msg = if v.is_empty() {
            "Nessuna lista o tabella da aggiornare (cross reference aggiornato)".to_string()
        } else {
            v.iter().map(|(t, f)| format!("{t}: {} fogli", f.len())).collect::<Vec<_>>().join(", ")
        };
        Ok(json!({"uscite": v.iter().map(|(t, f)| json!({"tipo": t, "fogli": f})).collect::<Vec<_>>(), "message": msg}))
    })
}

pub fn comandi() -> Vec<CommandSpec> {
    let spec = |id, label, menu, params, run| CommandSpec { id, label, menu, shortcut: None, aliases: &[], params, enabled: has_doc, run, undoable: false, interactive: None, transparent: false };
    vec![
        spec(
            "uscita",
            "Liste e Tabelle sui fogli",
            &["Progetto", "Liste e Tabelle sui fogli…"],
            "{tipo: indice|distinta|morsettiere|cablaggio|funzioni|simboli, foglio?: primo foglio (default dopo l'ultimo), quadro?, morsettiera?} → {fogli}",
            run_uscita as cadcraft_engine::cmd::Run,
        ),
        spec("aggiornaliste", "Aggiorna Liste e Tabelle", &["Elettrico", "Aggiorna Liste e Tabelle"], "{} — cross reference, rimandi e uscite già disegnate", run_aggiorna),
        spec(
            "excel",
            "Liste in Excel",
            &["Progetto", "Liste in Excel…"],
            "{tipo: come USCITA, file: .xlsx, quadro?, morsettiera?} — la lista in un foglio Excel → {righe}",
            run_excel,
        ),
    ]
}

/// Il nome di un foglio di lavoro Excel: al massimo 31 caratteri, senza `[]:*?/\`.
fn nome_foglio_excel(t: &str) -> String {
    let n: String = t.chars().filter(|c| !matches!(c, '[' | ']' | ':' | '*' | '?' | '/' | '\\')).take(31).collect();
    if n.trim().is_empty() { "Lista".into() } else { n.trim().to_string() }
}

/// Scrive la tabella in un file Excel: titolo, intestazioni in grassetto, una riga per riga;
/// colonne larghe come sui fogli. I testi restano testi (i numeri dei fili come `01.10`).
pub fn excel(t: &Tabella, nome: &str, file: &str) -> Result<()> {
    use rust_xlsxwriter::{Format, FormatBorder, Workbook};
    let e = |x: rust_xlsxwriter::XlsxError| errore(format!("Excel: {x}"));
    let mut libro = Workbook::new();
    let foglio = libro.add_worksheet();
    foglio.set_name(nome_foglio_excel(nome)).map_err(e)?;
    let titolo = Format::new().set_bold().set_font_size(14);
    let intestazione = Format::new().set_bold().set_border_bottom(FormatBorder::Thin);
    foglio.write_string_with_format(0, 0, t.titolo.replace('\\', "/"), &titolo).map_err(e)?;
    for (k, (testo, larghezza)) in t.colonne.iter().enumerate() {
        let c = u16::try_from(k).map_err(|_| errore("Troppe colonne"))?;
        foglio.write_string_with_format(2, c, testo.replace('\n', " / "), &intestazione).map_err(e)?;
        // da mm sul foglio a caratteri di Excel, mai troppo strette
        foglio.set_column_width(c, (larghezza / 2.0).max(8.0)).map_err(e)?;
    }
    for (i, riga) in t.righe.iter().enumerate() {
        let r = u32::try_from(i + 3).map_err(|_| errore("Troppe righe"))?;
        for (k, cella) in riga.iter().enumerate() {
            let c = u16::try_from(k).map_err(|_| errore("Troppe colonne"))?;
            let testo = match cella {
                Cella::T(v) => v.clone(),
                Cella::Blocco(b) => simboli::trova(b).map_or_else(|| b.clone(), |s| s.etichetta.to_string()),
            };
            if !testo.is_empty() {
                foglio.write_string(r, c, testo).map_err(e)?;
            }
        }
    }
    libro.save(file).map_err(e)
}

fn run_excel(s: &mut Session, p: &Value) -> Result<Value> {
    let tipo = testo_param(p, "tipo").ok_or_else(|| errore(format!("Indicare il tipo di lista: {}", TIPI.map(|t| t.0).join(", "))))?;
    let file = testo_param(p, "file").filter(|f| !f.trim().is_empty()).ok_or_else(|| errore("Indicare il file .xlsx"))?;
    let (t, nome, _) = tabella(s, &tipo, p)?;
    excel(&t, nome, &file)?;
    Ok(json!({"righe": t.righe.len(), "file": file, "message": format!("{nome}: {} righe in {file}", t.righe.len())}))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pagine_e_testi_lunghi() {
        let t = Tabella { titolo: String::new(), colonne: vec![("A".into(), 20.0), ("B".into(), 160.0)], righe: vec![vec![]; 100], altezza_riga: 6.0 };
        let p = t.pagine();
        // 33 righe per colonna, due colonne per pagina: 66 righe a pagina
        assert_eq!(t.righe_per_blocco(), 33);
        assert_eq!(p.len(), 2);
        assert_eq!(p[0], vec![0..33, 33..66]);
        assert_eq!(p[1], vec![66..99, 99..100]);
        assert_eq!(adatta("Contattore", 30.0, 2.5), "Contattore");
        assert!(adatta("Interruttore magnetotermico differenziale", 20.0, 2.5).ends_with('…'));
    }
}
