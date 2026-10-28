//! Barra a schede (ribbon) come nei CAD elettrici diffusi: schede, gruppi con pulsanti grandi
//! (icona sopra, testo sotto) e colonne di pulsanti piccoli, titolo del gruppo in basso.
//! Le icone sono nostre ([`crate::icone`]).

use cadcraft_ui_egui::theme::Tokens;
use egui::{Rect, Sense, Ui, pos2, vec2};

use crate::icone::{self, Icona};

/// Cosa fa un pulsante.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Azione {
    /// Avvia un comando come scritto sulla riga di comando (interattivo).
    Avvia(&'static str),
    /// Esegue un comando con i parametri JSON dati.
    Esegui(&'static str, &'static str),
    GestioneMultifoglio,
    /// Finestra "Gestione Progetti" (commesse).
    Progetti,
    /// Finestra "Impostazioni".
    Settaggi,
    /// Finestra "Liste e Tabelle", sul tipo di uscita dato.
    Liste(Option<&'static str>),
    /// Finestra "Gestione Cavi".
    Cavi,
    /// Finestra "Gestione PLC".
    Plc,
    /// Finestra "Traduttore".
    Traduttore,
    /// Lay-out dell'interno del quadro corrente, su un foglio nuovo.
    LayoutQuadro,
    /// Foratura della piastra del foglio corrente (CSV).
    Foratura,
    /// Finestra "Sovratemperature" (calcolo termico del quadro).
    Sovratemperature,
    /// Finestra "Tensioni": potenziali e colori dei conduttori.
    Tensioni,
    /// Finestra "Configuratore": opzioni della macchina.
    Configuratore,
    /// Finestra "Schema Unifilare": circuiti del quadro di distribuzione.
    Unifilare,
    /// Finestra "Gestore Topografico": componenti del quadro nel lay-out.
    Topografico,
    /// Finestra "Planimetria": quadri sulla pianta e lunghezze dei cavi.
    Planimetria,
    /// Targhette del tipo dato (componenti, fili, morsetti, cavi) in CSV.
    Targhette(&'static str),
    Cross,
    /// Salto al collegato della selezione: madre, contatti, rimando corrispondente.
    Salta,
    ArchivioMateriali,
    LibreriaSimboli,
    Edita,
    Distinta { per_tipo: bool },
    ImportaListino,
    /// Finestra "Editor Morsettiere".
    Morsettiera,
    /// Finestra "Stampa e PDF Fogli".
    StampaFogli,
    Simbolo(usize),
    /// Funzione non ancora disponibile (pulsante spento, con la spiegazione).
    NonDisponibile,
}

#[derive(Clone, Copy)]
enum Disegno {
    Icona(Icona),
    Simbolo(usize),
}

struct Pulsante {
    disegno: Disegno,
    testo: &'static str,
    azione: Azione,
}

enum Voce {
    Testo(Pulsante),
    Icone(Vec<Pulsante>),
}

enum Elemento {
    Grande(Pulsante),
    /// Colonna di (al massimo tre) righe di pulsanti piccoli.
    Piccoli(Vec<Voce>),
}

struct Gruppo {
    titolo: &'static str,
    freccia: bool,
    elementi: Vec<Elemento>,
}

struct Scheda {
    nome: &'static str,
    gruppi: Vec<Gruppo>,
}

fn p(icona: Icona, testo: &'static str, azione: Azione) -> Pulsante {
    Pulsante { disegno: Disegno::Icona(icona), testo, azione }
}
fn g(icona: Icona, testo: &'static str, azione: Azione) -> Elemento {
    Elemento::Grande(p(icona, testo, azione))
}
fn t(icona: Icona, testo: &'static str, azione: Azione) -> Voce {
    Voce::Testo(p(icona, testo, azione))
}
fn gruppo(titolo: &'static str, freccia: bool, elementi: Vec<Elemento>) -> Gruppo {
    Gruppo { titolo, freccia, elementi }
}
/// Pulsante grande di un simbolo della libreria (per id); un id sconosciuto dà un pulsante spento.
fn simbolo(id: &str) -> Elemento {
    match elettrico::simboli::indice(id) {
        Some(i) => {
            let testo = elettrico::simboli::LIBRERIA.get(i).map_or("", |s| s.etichetta);
            Elemento::Grande(Pulsante { disegno: Disegno::Simbolo(i), testo, azione: Azione::Simbolo(i) })
        }
        None => g(Icona::Libreria, "?", Azione::NonDisponibile),
    }
}
/// Colonna di pulsanti piccoli di simboli della libreria.
fn simboli_piccoli(ids: &[&str]) -> Elemento {
    Elemento::Piccoli(
        ids.iter()
            .filter_map(|id| {
                let i = elettrico::simboli::indice(id)?;
                let s = elettrico::simboli::LIBRERIA.get(i)?;
                Some(Voce::Testo(Pulsante { disegno: Disegno::Simbolo(i), testo: s.etichetta, azione: Azione::Simbolo(i) }))
            })
            .collect(),
    )
}

use Azione::*;

fn schede() -> Vec<Scheda> {
    vec![
        Scheda {
            nome: "WireCraft",
            gruppi: vec![
                gruppo("Progetti", false, vec![g(Icona::Progetti, "Gestione Progetti", Progetti)]),
                gruppo(
                    "Generale",
                    true,
                    vec![
                        Elemento::Piccoli(vec![t(Icona::Simbologia, "Simbologia", LibreriaSimboli), t(Icona::Settaggi, "Impostazioni", Settaggi), t(Icona::Scansione, "Verifica", Cross)]),
                        Elemento::Piccoli(vec![t(Icona::Impianto, "Schema d'impianto", Liste(Some("impianto"))), t(Icona::Quadro, "Quadro corrente", Settaggi), t(Icona::Tensioni, "Tensioni", Tensioni)]),
                        Elemento::Piccoli(vec![
                            Voce::Icone(vec![p(Icona::Pianta, "Planimetria", Planimetria)]),
                            Voce::Icone(vec![p(Icona::Aggiorna, "Aggiorna liste e tabelle", Esegui("aggiornaliste", "{}"))]),
                        ]),
                    ],
                ),
                gruppo(
                    "Fogli",
                    true,
                    vec![
                        g(Icona::Multifoglio, "Gestione Fogli", GestioneMultifoglio),
                        g(Icona::FoglioPrecedente, "Foglio Precedente", Esegui("foglioprecedente", "{}")),
                        g(Icona::CambioFoglio, "Cambio Foglio", Avvia("cambiofoglio")),
                        g(Icona::FoglioSeguente, "Foglio Seguente", Esegui("fogliosuccessivo", "{}")),
                        Elemento::Piccoli(vec![
                            t(Icona::Stampa, "Stampa e PDF", StampaFogli),
                            t(Icona::Legenda, "Legenda", Liste(Some("indice"))),
                            Voce::Icone(vec![
                                p(Icona::InserisciFogli, "Aggiungi fogli", Avvia("aggiungifogli")),
                                p(Icona::TitoloFoglio, "Titolo foglio", Avvia("titolofoglio")),
                                p(Icona::Testo, "Descrizione colonna", Avvia("descrizionecolonna")),
                                p(Icona::NumeraFogli, "Numera fogli", Esegui("numerafogli", "{}")),
                            ]),
                        ]),
                    ],
                ),
                gruppo("Cross", false, vec![g(Icona::Cross, "Cross Reference", Cross), g(Icona::RimandoColl, "Vai al Collegato", Salta)]),
                gruppo("Traduttore", false, vec![g(Icona::Legenda, "Traduttore", Traduttore)]),
                gruppo(
                    "Distinta",
                    false,
                    vec![
                        g(Icona::Archivio, "Archivio Materiali", ArchivioMateriali),
                        g(Icona::Distinta, "Distinta Materiali", Liste(Some("distinta"))),
                        Elemento::Piccoli(vec![
                            Voce::Icone(vec![p(Icona::DistintaTipo, "Distinta per tipo (CSV)", Distinta { per_tipo: true })]),
                            Voce::Icone(vec![p(Icona::DistintaSigla, "Distinta per sigla (CSV)", Distinta { per_tipo: false })]),
                            Voce::Icone(vec![p(Icona::DistintaCsv, "Importa listino materiali (CSV)", ImportaListino)]),
                        ]),
                    ],
                ),
                gruppo(
                    "Modifica Simbolo",
                    false,
                    vec![
                        g(Icona::EditaSimbolo, "Modifica Simbolo (XS)", Edita),
                        Elemento::Piccoli(vec![t(Icona::Copia, "Copia", Avvia("copy")), t(Icona::Sposta, "Sposta", Avvia("spostasimbolo")), t(Icona::Cancella, "Cancella", Avvia("cancellasimbolo"))]),
                    ],
                ),
                gruppo(
                    "PLC",
                    true,
                    vec![
                        g(Icona::Plc, "Gestione PLC", Plc),
                        g(Icona::Bit, "Segnali I/O", Plc),
                        Elemento::Piccoli(vec![
                            t(Icona::Libreria, "Libreria Simboli", LibreriaSimboli),
                            t(Icona::NumeraBit, "Numera I/O", Plc),
                            t(Icona::ArchivioPlc, "Lista I/O", Liste(Some("io"))),
                        ]),
                    ],
                ),
            ],
        },
        Scheda {
            nome: "Fili e Morsetti",
            gruppi: vec![
                gruppo(
                    "Filo",
                    true,
                    vec![
                        g(Icona::Filo, "Filo", Avvia("filo")),
                        Elemento::Piccoli(vec![t(Icona::Connessione, "Connessione", Avvia("connessione")), t(Icona::FiloV, "Filo Rapido (VV)", Avvia("vv")), t(Icona::FiloH, "Filo Rapido (HH)", Avvia("hh"))]),
                    ],
                ),
                gruppo(
                    "Collegamenti",
                    true,
                    vec![
                        g(Icona::Alimentazioni, "Alimentazioni", Avvia("alimentazioni")),
                        g(Icona::Collegamenti, "Collegamenti", Avvia("collegamenti")),
                        Elemento::Piccoli(vec![
                            t(Icona::RimandoAlim, "Rimandi Alimentazioni", Avvia("rimandialimentazioni")),
                            t(Icona::RimandoColl, "Rimandi Collegamenti", Avvia("rimandicollegamenti")),
                            t(Icona::Aggiorna, "Aggiorna Rimandi", Esegui("aggiornarimandi", "{}")),
                        ]),
                    ],
                ),
                gruppo(
                    "Numerazione Fili",
                    true,
                    vec![
                        g(Icona::NumeraFili, "Numera Fili", Avvia("numerafilo")),
                        g(Icona::NumeraFili, "Numera Fili Automatico", Esegui("numerafili", "{\"tutti\": true, \"mantieni\": true}")),
                        Elemento::Piccoli(vec![
                            t(Icona::NumeraFili, "Rinumera foglio corrente", Esegui("numerafili", "{}")),
                            t(Icona::Legenda, "Lista Numeri Usati", Esegui("listanumerifili", "{}")),
                            t(Icona::Sigle, "Siglatura Componenti", Esegui("siglecomponenti", "{}")),
                        ]),
                        Elemento::Piccoli(vec![
                            t(Icona::Filo, "Materiale Conduttori", Avvia("conduttore")),
                            t(Icona::Cancella, "Blocca/Sblocca fili", Avvia("bloccafili")),
                            t(Icona::NumeraFogli, "Numera Fogli", Esegui("numerafogli", "{}")),
                        ]),
                    ],
                ),
                gruppo(
                    "Morsetti e Connettori",
                    false,
                    vec![g(Icona::Morsetti, "Inserisci Morsetti", Avvia("morsetti")), g(Icona::Morsettiera, "Morsettiere", Morsettiera), g(Icona::Morsetti, "Inserisci Connettori", Avvia("connettori"))],
                ),
            ],
        },
        Scheda {
            nome: "Simboli",
            gruppi: vec![
                gruppo("Libreria", false, vec![g(Icona::Libreria, "Libreria Simboli", LibreriaSimboli)]),
                gruppo("Contatti", true, vec![simbolo("na"), simbolo("nc"), simbolo("sc"), simboli_piccoli(&["pot3", "naritardato", "termiconc"])]),
                gruppo("Bobine", true, vec![simbolo("km"), simbolo("ka"), simboli_piccoli(&["kt", "ktdis", "yv"])]),
                gruppo("Comandi", true, vec![simbolo("sbno"), simbolo("sbnc"), simbolo("sbem"), simboli_piccoli(&["sa", "sak", "sh"])]),
                gruppo("Segnalazione", false, vec![simbolo("hl"), simboli_piccoli(&["ha"])]),
                gruppo("Sensori", true, vec![simbolo("bq"), simboli_piccoli(&["sqno", "sqnc", "bf"])]),
                gruppo("Interruttori", true, vec![simbolo("qf3"), simbolo("qm3"), simboli_piccoli(&["qf1", "qf2", "qs3"]), simboli_piccoli(&["qd2", "fu", "fr3"])]),
                gruppo("Motori e utenze", true, vec![simbolo("m3"), simboli_piccoli(&["m3pe", "tc", "g"])]),
                gruppo("Terre", false, vec![simboli_piccoli(&["pe", "terra", "massa"])]),
            ],
        },
        Scheda {
            nome: "Strumenti",
            gruppi: vec![gruppo(
                "Moduli",
                false,
                vec![
                    g(Icona::Collegamenti, "Modulo Cavi", Cavi),
                    g(Icona::Plc, "Gestione PLC", Plc),
                    g(Icona::Legenda, "Traduttore", Traduttore),
                    g(Icona::Impianto, "Loop Diagram", Liste(Some("loop"))),
                    g(Icona::Settaggi, "Configuratore", Configuratore),
                ],
            )],
        },
        Scheda {
            nome: "Quadro",
            gruppi: vec![
                gruppo(
                    "Layout",
                    false,
                    vec![g(Icona::Quadro, "Layout Quadro", LayoutQuadro), g(Icona::Tratteggio, "Foratura Piastra", Foratura), g(Icona::Cerchio, "Pulsantiera", Liste(Some("pulsantiera"))), g(Icona::Pianta, "Gestore Topografico", Topografico), g(Icona::Tensioni, "Sovratemperature", Sovratemperature)],
                ),
                gruppo(
                    "Distribuzione",
                    false,
                    vec![
                        g(Icona::Unifilare, "Schema Unifilare", Unifilare),
                        Elemento::Piccoli(vec![t(Icona::Aggiorna, "Ricalcola Circuiti", Esegui("unifilare.aggiorna", "{}")), t(Icona::Legenda, "Tabella Circuiti", Liste(Some("circuiti")))]),
                    ],
                ),
                gruppo(
                    "CEI EN 81346",
                    false,
                    vec![
                        g(Icona::Quadro, "Box Quadro", Avvia("box.quadro")),
                        Elemento::Piccoli(vec![
                            t(Icona::Impianto, "Box Funzione", Avvia("box.funzione")),
                            t(Icona::Pianta, "Box Ubicazione", Avvia("box.ubicazione")),
                            t(Icona::Aggiorna, "Applica i Box", Esegui("box.applica", "{}")),
                        ]),
                        Elemento::Piccoli(vec![
                            t(Icona::Sigle, "Designazioni", Esegui("designazioni", r#"{"fogli": "tutti"}"#)),
                            t(Icona::Sigle, "Solo Discordanti", Esegui("designazioni", r#"{"fogli": "tutti", "regola": "discordanti"}"#)),
                            t(Icona::Cancella, "Togli Designazioni", Esegui("designazioni", r#"{"fogli": "tutti", "togli": true}"#)),
                        ]),
                    ],
                ),
            ],
        },
        Scheda {
            nome: "Disegno",
            gruppi: vec![
                gruppo(
                    "Disegna",
                    true,
                    vec![
                        g(Icona::Linea, "Linea", Avvia("line")),
                        g(Icona::Polilinea, "Polilinea", Avvia("pline")),
                        g(Icona::Cerchio, "Cerchio", Avvia("circle")),
                        g(Icona::Rettangolo, "Rettangolo", Avvia("rectang")),
                        Elemento::Piccoli(vec![t(Icona::Testo, "Testo", Avvia("mtext")), t(Icona::Tratteggio, "Tratteggio", Avvia("hatch")), t(Icona::Quota, "Quota", Avvia("dimlinear"))]),
                    ],
                ),
                gruppo(
                    "Modifica",
                    true,
                    vec![
                        Elemento::Piccoli(vec![t(Icona::Copia, "Copia", Avvia("copy")), t(Icona::Sposta, "Sposta", Avvia("move")), t(Icona::Cancella, "Cancella", Avvia("erase"))]),
                        Elemento::Piccoli(vec![t(Icona::Annulla, "Annulla", Avvia("u")), t(Icona::Ripeti, "Ripeti", Avvia("redo"))]),
                    ],
                ),
            ],
        },
        Scheda {
            nome: "Vista",
            gruppi: vec![gruppo(
                "Navigazione",
                false,
                vec![
                    g(Icona::ZoomEstensioni, "Zoom Estensioni", Esegui("zoom.extents", "{}")),
                    g(Icona::ZoomFinestra, "Zoom", Avvia("zoom")),
                    g(Icona::Pan, "Pan", Avvia("pan")),
                    g(Icona::Rigenera, "Rigenera", Esegui("regen", "{}")),
                ],
            )],
        },
        Scheda {
            nome: "Output",
            gruppi: vec![
                gruppo("File", false, vec![g(Icona::Apri, "Apri", Avvia("open")), g(Icona::Salva, "Salva", Avvia("qsave")), g(Icona::Dxf, "Salva come", Avvia("saveas"))]),
                gruppo(
                    "Stampa",
                    false,
                    vec![g(Icona::Pdf, "Stampa e PDF Fogli", StampaFogli), g(Icona::Stampa, "Stampa", Avvia("plot")), g(Icona::Pdf, "Esporta PDF", Avvia("exportpdf"))],
                ),
                gruppo(
                    "Liste e Tabelle",
                    true,
                    vec![
                        g(Icona::Legenda, "Lista Fogli", Liste(Some("indice"))),
                        g(Icona::Distinta, "Distinta Materiali", Liste(Some("distinta"))),
                        g(Icona::Morsettiera, "Morsettiere", Liste(Some("morsettiere"))),
                        g(Icona::Collegamenti, "Tabella di Cablaggio", Liste(Some("cablaggio"))),
                        Elemento::Piccoli(vec![
                            t(Icona::Legenda, "Legenda Funzioni", Liste(Some("funzioni"))),
                            t(Icona::Libreria, "Legenda Simboli", Liste(Some("simboli"))),
                            t(Icona::Aggiorna, "Aggiorna Liste e Tabelle", Esegui("aggiornaliste", "{}")),
                        ]),
                    ],
                ),
                gruppo("CSV", false, vec![g(Icona::Csv, "Distinta (CSV)", Distinta { per_tipo: true }), g(Icona::Morsettiera, "Morsettiera (CSV)", Morsettiera)]),
                gruppo(
                    "Targhette",
                    false,
                    vec![
                        Elemento::Piccoli(vec![t(Icona::Sigle, "Componenti", Targhette("componenti")), t(Icona::NumeraFili, "Fili", Targhette("fili"))]),
                        Elemento::Piccoli(vec![t(Icona::Morsetti, "Morsetti", Targhette("morsetti")), t(Icona::Collegamenti, "Cavi", Targhette("cavi"))]),
                    ],
                ),
            ],
        },
    ]
}

const ALTEZZA_CORPO: f32 = 74.0;
const ALTEZZA_TITOLO: f32 = 18.0;
const ICONA_GRANDE: f32 = 30.0;
const ICONA_PICCOLA: f32 = 16.0;
const RIGA_PICCOLA: f32 = 24.0;

pub struct Ribbon {
    schede: Vec<Scheda>,
    attiva: usize,
}

impl Default for Ribbon {
    fn default() -> Self {
        Ribbon { schede: schede(), attiva: 0 }
    }
}

/// Divide un'etichetta lunga in due righe allo spazio più vicino al centro.
fn due_righe(testo: &str) -> String {
    if testo.len() <= 9 || !testo.contains(' ') {
        return testo.to_string();
    }
    let meta = testo.len() / 2;
    let taglio = testo.match_indices(' ').map(|(i, _)| i).min_by_key(|i| i.abs_diff(meta)).unwrap_or(meta);
    format!("{}\n{}", &testo[..taglio], testo[taglio..].trim_start())
}

fn icona(ui: &Ui, r: Rect, d: Disegno, colore: egui::Color32, attivo: bool) {
    match d {
        Disegno::Icona(i) => icone::disegna(ui.painter(), r, i, colore, attivo),
        Disegno::Simbolo(k) => {
            if let Some(s) = elettrico::simboli::LIBRERIA.get(k) {
                icone::disegna_simbolo(ui.painter(), r, s, colore);
            }
        }
    }
}

fn aiuto(pu: &Pulsante) -> String {
    match pu.azione {
        NonDisponibile => format!("{} – non ancora disponibile", pu.testo.replace('\n', " ")),
        Avvia(c) | Esegui(c, _) => format!("{} ({})", pu.testo, c.to_uppercase()),
        _ => pu.testo.to_string(),
    }
}

impl Ribbon {
    /// Disegna le schede e la barra; restituisce l'azione del pulsante premuto.
    pub fn mostra(&mut self, ui: &mut Ui) -> Option<Azione> {
        let tk = Tokens::get();
        let mut premuto = None;
        egui::Panel::top("ce_ribbon_schede").exact_size(26.0).frame(egui::Frame::NONE.fill(tk.chrome_dark).inner_margin(egui::Margin::symmetric(4, 0))).show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                for (k, s) in self.schede.iter().enumerate() {
                    let font = egui::FontId::proportional(13.0);
                    let largh = ui.ctx().fonts_mut(|f| f.layout_no_wrap(s.nome.to_string(), font.clone(), tk.text).size().x) + 18.0;
                    let (r, risp) = ui.allocate_exact_size(vec2(largh, 24.0), Sense::click());
                    let attiva = k == self.attiva;
                    if attiva {
                        ui.painter().rect_filled(r, 2.0, tk.chrome);
                        ui.painter().line_segment([r.left_bottom(), r.right_bottom()], egui::Stroke::new(2.0, tk.accent));
                    } else if risp.hovered() {
                        ui.painter().rect_filled(r, 2.0, tk.hover);
                    }
                    ui.painter().text(r.center(), egui::Align2::CENTER_CENTER, s.nome, font, if attiva { tk.text } else { tk.text_dim });
                    if risp.clicked() {
                        self.attiva = k;
                    }
                }
            });
        });
        egui::Panel::top("ce_ribbon").exact_size(ALTEZZA_CORPO + ALTEZZA_TITOLO + 6.0).frame(egui::Frame::NONE.fill(tk.chrome).inner_margin(egui::Margin::symmetric(4, 3))).show(ui, |ui| {
            egui::ScrollArea::horizontal().show(ui, |ui| {
                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing = vec2(2.0, 1.0);
                    let Some(scheda) = self.schede.get(self.attiva) else { return };
                    for gr in &scheda.gruppi {
                        if let Some(a) = Self::gruppo(ui, gr, &tk) {
                            premuto = Some(a);
                        }
                    }
                });
            });
        });
        premuto
    }

    fn gruppo(ui: &mut Ui, gr: &Gruppo, tk: &Tokens) -> Option<Azione> {
        let mut premuto = None;
        let inizio = ui.cursor().min;
        ui.vertical(|ui| {
            ui.horizontal_top(|ui| {
                ui.set_min_height(ALTEZZA_CORPO);
                ui.spacing_mut().item_spacing.x = 3.0;
                for e in &gr.elementi {
                    let a = match e {
                        Elemento::Grande(pu) => Self::grande(ui, pu, tk),
                        Elemento::Piccoli(voci) => Self::colonna(ui, voci, tk),
                    };
                    if a.is_some() {
                        premuto = a;
                    }
                }
            });
            // titolo del gruppo in basso, largo quanto il gruppo
            let largh = (ui.min_rect().right() - inizio.x).max(50.0);
            let (r, _) = ui.allocate_exact_size(vec2(largh, ALTEZZA_TITOLO), Sense::hover());
            let testo = ui.painter().text(r.center(), egui::Align2::CENTER_CENTER, gr.titolo, egui::FontId::proportional(12.0), tk.text_dim);
            if gr.freccia {
                // triangolino verso il basso dopo il titolo
                let c = pos2(testo.right() + 8.0, r.center().y);
                ui.painter().add(egui::Shape::convex_polygon(vec![pos2(c.x - 4.0, c.y - 2.0), pos2(c.x + 4.0, c.y - 2.0), pos2(c.x, c.y + 2.5)], tk.text_dim, egui::Stroke::NONE));
            }
        });
        let fine = ui.min_rect().right() + 3.0;
        ui.painter().line_segment([pos2(fine, inizio.y + 4.0), pos2(fine, inizio.y + ALTEZZA_CORPO + ALTEZZA_TITOLO - 2.0)], egui::Stroke::new(1.0, tk.border));
        ui.add_space(8.0);
        premuto
    }

    fn grande(ui: &mut Ui, pu: &Pulsante, tk: &Tokens) -> Option<Azione> {
        let font = egui::FontId::proportional(12.0);
        let testo = due_righe(pu.testo);
        let galley = ui.ctx().fonts_mut(|f| f.layout_no_wrap(testo.clone(), font.clone(), tk.text));
        let largh = galley.size().x.max(ICONA_GRANDE) + 10.0;
        let (r, risp) = ui.allocate_exact_size(vec2(largh, ALTEZZA_CORPO), Sense::click());
        let attivo = pu.azione != NonDisponibile;
        if risp.hovered() && attivo {
            ui.painter().rect_filled(r, 3.0, tk.hover);
        }
        let ri = Rect::from_center_size(pos2(r.center().x, r.top() + 4.0 + ICONA_GRANDE / 2.0), vec2(ICONA_GRANDE, ICONA_GRANDE));
        icona(ui, ri, pu.disegno, tk.icon, attivo);
        let colore = if attivo { tk.text } else { tk.text_faint };
        let mut job = egui::text::LayoutJob::simple(testo, font, colore, f32::INFINITY);
        job.halign = egui::Align::Center;
        let galley = ui.ctx().fonts_mut(|f| f.layout_job(job));
        ui.painter().galley(pos2(r.center().x, ri.bottom() + 4.0), galley, colore);
        let risp = risp.on_hover_text(aiuto(pu));
        (risp.clicked() && attivo).then_some(pu.azione)
    }

    fn colonna(ui: &mut Ui, voci: &[Voce], tk: &Tokens) -> Option<Azione> {
        let mut premuto = None;
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 1.0;
            for v in voci {
                match v {
                    Voce::Testo(pu) => {
                        let font = egui::FontId::proportional(12.5);
                        let largh = ui.ctx().fonts_mut(|f| f.layout_no_wrap(pu.testo.to_string(), font.clone(), tk.text).size().x) + ICONA_PICCOLA + 14.0;
                        let (r, risp) = ui.allocate_exact_size(vec2(largh, RIGA_PICCOLA), Sense::click());
                        let attivo = pu.azione != NonDisponibile;
                        if risp.hovered() && attivo {
                            ui.painter().rect_filled(r, 3.0, tk.hover);
                        }
                        let ri = Rect::from_min_size(pos2(r.left() + 3.0, r.center().y - ICONA_PICCOLA / 2.0), vec2(ICONA_PICCOLA, ICONA_PICCOLA));
                        icona(ui, ri, pu.disegno, tk.icon, attivo);
                        ui.painter().text(pos2(ri.right() + 6.0, r.center().y), egui::Align2::LEFT_CENTER, pu.testo, font, if attivo { tk.text } else { tk.text_faint });
                        if risp.on_hover_text(aiuto(pu)).clicked() && attivo {
                            premuto = Some(pu.azione);
                        }
                    }
                    Voce::Icone(pulsanti) => {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 1.0;
                            for pu in pulsanti {
                                let (r, risp) = ui.allocate_exact_size(vec2(RIGA_PICCOLA, RIGA_PICCOLA), Sense::click());
                                let attivo = pu.azione != NonDisponibile;
                                if risp.hovered() && attivo {
                                    ui.painter().rect_filled(r, 3.0, tk.hover);
                                }
                                icona(ui, Rect::from_center_size(r.center(), vec2(ICONA_PICCOLA + 2.0, ICONA_PICCOLA + 2.0)), pu.disegno, tk.icon, attivo);
                                if risp.on_hover_text(aiuto(pu)).clicked() && attivo {
                                    premuto = Some(pu.azione);
                                }
                            }
                        });
                    }
                }
            }
        });
        premuto
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn etichette_su_due_righe() {
        assert_eq!(due_righe("Gestione Fogli"), "Gestione\nFogli");
        assert_eq!(due_righe("Foglio Precedente"), "Foglio\nPrecedente");
        assert_eq!(due_righe("Filo"), "Filo");
    }

    #[test]
    fn i_comandi_della_barra_esistono() {
        elettrico::registra();
        for s in schede() {
            for g in s.gruppi {
                for e in g.elementi {
                    let mut pulsanti = Vec::new();
                    match e {
                        Elemento::Grande(p) => pulsanti.push(p),
                        Elemento::Piccoli(v) => {
                            for x in v {
                                match x {
                                    Voce::Testo(p) => pulsanti.push(p),
                                    Voce::Icone(ps) => pulsanti.extend(ps),
                                }
                            }
                        }
                    }
                    for p in pulsanti {
                        if let Avvia(c) | Esegui(c, _) = p.azione {
                            assert!(cadcraft_engine::find_command(c).is_some(), "comando {c} ({}) inesistente", p.testo);
                        }
                        if let Esegui(_, j) = p.azione {
                            assert!(serde_json::from_str::<serde_json::Value>(j).is_ok(), "{j}");
                        }
                    }
                }
            }
        }
    }
}
