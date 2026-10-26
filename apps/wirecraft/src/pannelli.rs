//! Pannelli elettrici attorno all'area di disegno, come nei CAD elettrici diffusi: a sinistra
//! Fogli (cambio foglio), Simboli (libreria), Navigatore, Cross Reference e archivio Materiali;
//! a destra la tavolozza dei comandi di fili, numerazione, morsetti e output.
//!
//! Disegnati prima dell'interfaccia di CADCraft, che usa lo spazio rimasto; i pulsanti avviano
//! i comandi come se fossero scritti sulla riga di comando.

use cadcraft_ui_egui::CadApp;
use elettrico::{fogli, siglatura, simboli};

use crate::cavi_plc::{FinestraCavi, FinestraPlc};
use crate::dialogo::{Dialogo, Esito, SchedaMateriali};
use crate::gestione::GestioneMultifoglio;
use crate::progetti::GestioneProgetti;
use crate::liste::FinestraListe;
use crate::settaggi::FinestraSettaggi;
use crate::ribbon::Azione;
use crate::stampa::FinestraStampa;
use serde_json::{Value, json};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Scheda {
    #[default]
    Fogli,
    Simboli,
    Navigatore,
    Cross,
    Materiali,
}

struct Voce {
    handle: String,
    quadro: String,
    sigla: String,
    foglio: u32,
    pos: (f64, f64),
}

#[derive(Default)]
pub struct Pannelli {
    scheda: Scheda,
    /// (disegno, revisione) dei dati mostrati: si rileggono solo quando il disegno cambia.
    letto: Option<(u64, u64)>,
    fogli: Vec<fogli::Foglio>,
    componenti: Vec<Voce>,
    cross: Option<Value>,
    filtro: String,
    morsettiera: String,
    dialogo: Option<Dialogo>,
    materiali: SchedaMateriali,
    stampa: FinestraStampa,
    gestione: GestioneMultifoglio,
    pub progetti: GestioneProgetti,
    settaggi: FinestraSettaggi,
    liste: FinestraListe,
    cavi: FinestraCavi,
    plc: FinestraPlc,
    /// Il disegno attivo (identità e percorso): quando cambia si cerca la sua commessa.
    disegno: Option<(u64, Option<String>)>,
    commessa: Option<String>,
}

impl Pannelli {
    fn aggiorna(&mut self, cad: &mut CadApp) {
        let chiave = cad.session.state().ok().map(|st| (st.uid, st.revision));
        if chiave == self.letto {
            return;
        }
        self.fogli = fogli::elenco(&mut cad.session).unwrap_or_default();
        self.componenti = siglatura::componenti(&mut cad.session)
            .unwrap_or_default()
            .into_iter()
            .filter(|c| c.madre())
            .map(|c| Voce { handle: c.blocco.handle.clone(), quadro: c.blocco.attributo("QUADRO").to_string(), sigla: c.nome.clone(), foglio: c.foglio, pos: c.blocco.pos })
            .collect();
        self.componenti.sort_by(|a, b| a.quadro.cmp(&b.quadro).then_with(|| a.sigla.cmp(&b.sigla)));
        // la lettura non cambia il disegno: si ricorda la revisione dopo
        self.letto = cad.session.state().ok().map(|st| (st.uid, st.revision));
    }

    /// Disegna i pannelli; va chiamata prima di `CadApp::ui`.
    pub fn mostra(&mut self, cad: &mut CadApp, ui: &mut egui::Ui) {
        self.progetti.mostra(cad, ui.ctx());
        self.segui_disegno(cad);
        if cad.session.docs.is_empty() || cad.ui.start_tab {
            return;
        }
        self.aggiorna(cad);
        self.barra_di_stato(cad, ui);
        egui::Panel::left("ce_pannelli").resizable(true).default_size(250.0).show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                for (s, t) in [(Scheda::Fogli, "Fogli"), (Scheda::Simboli, "Simboli"), (Scheda::Navigatore, "Navigatore"), (Scheda::Cross, "Cross"), (Scheda::Materiali, "Materiali")] {
                    ui.selectable_value(&mut self.scheda, s, t);
                }
            });
            ui.separator();
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| match self.scheda {
                Scheda::Fogli => self.fogli(cad, ui),
                Scheda::Simboli => self.simboli(cad, ui),
                Scheda::Navigatore => self.navigatore(cad, ui),
                Scheda::Cross => self.cross(cad, ui),
                Scheda::Materiali => self.materiali.mostra(cad, ui),
            });
        });
        egui::Panel::right("ce_tavolozza").resizable(true).default_size(170.0).show(ui, |ui| {
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| self.tavolozza(cad, ui));
        });
        if let Some(d) = &mut self.dialogo
            && let Esito::Chiuso = d.mostra(cad, ui.ctx())
        {
            self.dialogo = None;
        }
        self.stampa.mostra(cad, ui.ctx(), &self.fogli);
        self.gestione.mostra(cad, ui.ctx());
        self.settaggi.mostra(cad, ui.ctx());
        self.liste.mostra(cad, ui.ctx());
        self.cavi.mostra(cad, ui.ctx());
        self.plc.mostra(cad, ui.ctx());
    }

    /// Esegue l'azione di un pulsante della barra a schede.
    pub fn esegui(&mut self, cad: &mut CadApp, azione: Azione) {
        match azione {
            Azione::Avvia(c) => cad.start(c),
            Azione::Esegui(c, j) => {
                let _ = cad.run(c, serde_json::from_str(j).unwrap_or_else(|_| json!({})));
            }
            Azione::Progetti => self.progetti.apri(),
            Azione::Settaggi => self.settaggi.apri(),
            Azione::Liste(tipo) => self.liste.apri(tipo),
            Azione::Cavi => self.cavi.apri(),
            Azione::Plc => self.plc.apri(),
            Azione::GestioneMultifoglio => {
                self.gestione.apri();
                self.scheda = Scheda::Fogli;
            }
            Azione::Cross => {
                self.cross = cad.run("crossreference", json!({})).ok();
                self.scheda = Scheda::Cross;
            }
            Azione::ArchivioMateriali => self.scheda = Scheda::Materiali,
            Azione::LibreriaSimboli => self.scheda = Scheda::Simboli,
            Azione::Edita => self.edita_selezionato(cad),
            Azione::Distinta { per_tipo } => {
                let nome = if per_tipo { "distinta materiali per tipo.csv" } else { "distinta materiali per sigla.csv" };
                if let Some(p) = rfd::FileDialog::new().add_filter("CSV", &["csv"]).set_file_name(nome).save_file() {
                    let _ = cad.run("distintamateriali", json!({"ordine": if per_tipo { "tipo" } else { "sigla" }, "file": p.to_string_lossy()}));
                }
            }
            Azione::ImportaListino => {
                if let Some(p) = rfd::FileDialog::new().add_filter("CSV", &["csv", "txt"]).pick_file() {
                    let _ = cad.run("materiali.importa", json!({"file": p.to_string_lossy()}));
                }
                self.scheda = Scheda::Materiali;
            }
            Azione::Morsettiera => {
                let nome = self.morsettiera.trim().to_string();
                if nome.is_empty() {
                    cad.set_status("Scrivere il nome della morsettiera (es. -XG) nella tavolozza a destra");
                } else if let Some(p) = rfd::FileDialog::new().add_filter("CSV", &["csv"]).set_file_name(format!("morsettiera {nome}.csv")).save_file() {
                    let _ = cad.run("morsettiera", json!({"morsettiera": nome, "file": p.to_string_lossy()}));
                }
            }
            Azione::StampaFogli => {
                if self.fogli.is_empty() {
                    cad.set_status("Il disegno non ha fogli: usa Gestione Multifoglio");
                } else {
                    self.stampa.apri(cad);
                }
            }
            Azione::Simbolo(i) => self.dialogo = Dialogo::inserimento(cad, i),
            Azione::NonDisponibile => {}
        }
    }

    /// Quando cambia il disegno attivo: commessa e impostazioni del nuovo disegno.
    fn segui_disegno(&mut self, cad: &mut CadApp) {
        let ora = cad.session.state().ok().map(|st| (st.uid, st.path.clone()));
        if ora == self.disegno {
            return;
        }
        let percorso = ora.as_ref().and_then(|(_, p)| p.clone());
        self.commessa = elettrico::commesse::attiva_disegno(percorso.as_deref().map(std::path::Path::new)).map(|c| c.nome());
        self.disegno = ora;
    }

    /// Barra di stato elettrica: foglio corrente, progetto, quadro e conduttore in uso.
    fn barra_di_stato(&mut self, cad: &mut CadApp, ui: &mut egui::Ui) {
        let imp = elettrico::impostazioni::correnti();
        let foglio = self.fogli.iter().find(|f| f.corrente).map(|f| if f.titolo.is_empty() { format!("{:02}", f.numero) } else { format!("{:02} – {}", f.numero, f.titolo) });
        let mut cambia_quadro = None;
        egui::Panel::bottom("ce_stato").exact_size(22.0).show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                ui.label(format!("Foglio: {}", foglio.unwrap_or_else(|| "—".into())));
                ui.separator();
                ui.label(format!("Progetto: {}", self.commessa.clone().unwrap_or_else(|| "—".into())));
                ui.separator();
                ui.label("Quadro:");
                let mut q = imp.quadro.clone();
                if ui.add(egui::TextEdit::singleline(&mut q).desired_width(60.0).hint_text("+QG")).on_hover_text("Quadro corrente per i nuovi simboli").changed() {
                    cambia_quadro = Some(q);
                }
                ui.separator();
                ui.label(format!("Conduttore: {}", imp.conduttore.etichetta())).on_hover_text(format!("{} {}", imp.conduttore.costruttore, imp.conduttore.codice));
                ui.separator();
                ui.label(egui::RichText::new(imp.siglatura.descrizione()).small());
            });
        });
        if let Some(q) = cambia_quadro {
            let _ = cad.run("impostazioni", json!({"quadro": q}));
        }
    }

    /// Apre la finestra di modifica per il componente selezionato.
    fn edita_selezionato(&mut self, cad: &mut CadApp) {
        let scelti: Vec<String> = cad.session.selection().iter().map(|h| h.hex()).collect();
        let trovato = scelti.iter().find_map(|h| Dialogo::modifica(cad, h));
        match trovato {
            Some(d) => self.dialogo = Some(d),
            None => cad.set_status("Selezionare un componente (simbolo) da modificare"),
        }
    }

    fn fogli(&mut self, cad: &mut CadApp, ui: &mut egui::Ui) {
        if self.fogli.is_empty() {
            ui.label("Il disegno non è un multifoglio.");
            if ui.button("Nuovo multifoglio").clicked() {
                let _ = cad.run("nuovomultifoglio", json!({}));
            }
            return;
        }
        ui.horizontal(|ui| {
            if ui.button("◀").on_hover_text("Foglio precedente").clicked() {
                let _ = cad.run("foglioprecedente", json!({}));
            }
            if ui.button("▶").on_hover_text("Foglio seguente").clicked() {
                let _ = cad.run("fogliosuccessivo", json!({}));
            }
            if ui.button("Aggiungi…").clicked() {
                cad.start("aggiungifogli");
            }
            if ui.button("Titolo…").clicked() {
                cad.start("titolofoglio");
            }
        });
        ui.separator();
        let mut vai = None;
        egui::Grid::new("ce_fogli").striped(true).num_columns(2).show(ui, |ui| {
            for f in &self.fogli {
                let testo = if f.corrente { egui::RichText::new(f.numero.to_string()).strong() } else { egui::RichText::new(f.numero.to_string()) };
                if ui.selectable_label(f.corrente, testo).clicked() {
                    vai = Some(f.numero);
                }
                if ui.selectable_label(f.corrente, f.titolo.as_str()).clicked() {
                    vai = Some(f.numero);
                }
                ui.end_row();
            }
        });
        if let Some(n) = vai {
            let _ = cad.run("cambiofoglio", json!({"foglio": n}));
        }
    }

    /// Libreria simboli: anteprime per categoria, con ricerca; un clic apre la finestra di
    /// inserimento.
    fn simboli(&mut self, cad: &mut CadApp, ui: &mut egui::Ui) {
        ui.add(egui::TextEdit::singleline(&mut self.filtro).hint_text("Cerca simbolo"));
        let filtro = self.filtro.to_lowercase();
        let colore = ui.visuals().text_color();
        let sfondo = ui.visuals().widgets.hovered.bg_fill;
        let mut scelto = None;
        for cat in simboli::categorie() {
            let lista: Vec<(usize, &simboli::Simbolo)> = simboli::LIBRERIA
                .iter()
                .enumerate()
                .filter(|(_, s)| s.categoria == cat && (filtro.is_empty() || s.etichetta.to_lowercase().contains(&filtro) || s.id.contains(&filtro)))
                .collect();
            if lista.is_empty() {
                continue;
            }
            egui::CollapsingHeader::new(cat).id_salt(("ce_lib", cat)).default_open(true).show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(2.0, 2.0);
                    for (i, sim) in lista {
                        let (r, risp) = ui.allocate_exact_size(egui::vec2(66.0, 74.0), egui::Sense::click());
                        if risp.hovered() {
                            ui.painter().rect_filled(r, 3.0, sfondo);
                        }
                        let ri = egui::Rect::from_min_size(r.min + egui::vec2(9.0, 3.0), egui::vec2(48.0, 44.0));
                        crate::icone::disegna_simbolo(ui.painter(), ri, sim, colore);
                        let testo = ui.painter().layout(sim.etichetta.to_string(), egui::FontId::proportional(9.5), colore, 64.0);
                        let at = egui::pos2(r.center().x - testo.size().x / 2.0, ri.bottom() + 2.0);
                        ui.painter().with_clip_rect(r).galley(at, testo, colore);
                        let figlio = if sim.madre() || sim.grafico() { "" } else { " – contatto" };
                        if risp.on_hover_text(format!("{}{figlio} ({})", sim.etichetta, sim.comando.to_uppercase())).clicked() {
                            scelto = Some(i);
                        }
                    }
                });
            });
        }
        if let Some(i) = scelto {
            self.dialogo = Dialogo::inserimento(cad, i);
        }
    }

    fn navigatore(&mut self, cad: &mut CadApp, ui: &mut egui::Ui) {
        ui.add(egui::TextEdit::singleline(&mut self.filtro).hint_text("Cerca sigla"));
        let filtro = self.filtro.to_lowercase();
        let mut vai = None;
        let mut edita = None;
        let mut quadro: Option<&str> = None;
        for v in self.componenti.iter().filter(|v| filtro.is_empty() || v.sigla.to_lowercase().contains(&filtro)) {
            if quadro != Some(v.quadro.as_str()) {
                ui.add_space(4.0);
                ui.label(egui::RichText::new(if v.quadro.is_empty() { "(senza quadro)" } else { v.quadro.as_str() }).small().strong());
                quadro = Some(v.quadro.as_str());
            }
            ui.horizontal(|ui| {
                if ui.small_button("✎").on_hover_text("Edita simbolo").clicked() {
                    edita = Some(v.handle.clone());
                }
                if ui.selectable_label(false, format!("{}   foglio {}", v.sigla, v.foglio)).clicked() {
                    vai = Some((v.foglio, v.pos));
                }
            });
        }
        if let Some(h) = edita {
            self.dialogo = Dialogo::modifica(cad, &h);
        }
        if let Some((n, p)) = vai {
            if n > 0 {
                let _ = cad.run("cambiofoglio", json!({"foglio": n}));
            }
            let _ = cad.run("zoom.window", json!({"p1": [p.0 - 40.0, p.1 - 30.0], "p2": [p.0 + 40.0, p.1 + 30.0]}));
        }
    }

    fn cross(&mut self, cad: &mut CadApp, ui: &mut egui::Ui) {
        if ui.button("Verifica cross reference").clicked() {
            self.cross = cad.run("crossreference", json!({})).ok();
        }
        let Some(r) = &self.cross else {
            ui.label("Controlla contatti, specchietti e rimandi di tutto il multifoglio.");
            return;
        };
        let elenco = |ui: &mut egui::Ui, titolo: &str, chiave: &str, colore: egui::Color32| {
            let v: Vec<&str> = r.get(chiave).and_then(Value::as_array).into_iter().flatten().filter_map(Value::as_str).collect();
            if v.is_empty() {
                ui.colored_label(egui::Color32::from_rgb(80, 180, 90), format!("✔ {titolo}: nessuno"));
            } else {
                ui.colored_label(colore, format!("{titolo}: {}", v.len()));
                for t in v {
                    ui.label(format!("   {t}"));
                }
            }
        };
        elenco(ui, "Contatti senza madre", "figliSenzaMadre", egui::Color32::from_rgb(220, 70, 60));
        elenco(ui, "Contatti oltre la disponibilità", "contattiSenzaPosto", egui::Color32::from_rgb(220, 70, 60));
        elenco(ui, "Rimandi senza corrispondenza", "rimandiSenzaCoppia", egui::Color32::from_rgb(220, 180, 40));
    }

    fn tavolozza(&mut self, cad: &mut CadApp, ui: &mut egui::Ui) {
        let mut pulsante = |ui: &mut egui::Ui, testo: &str, comando: &str| {
            if ui.add_sized([ui.available_width(), 22.0], egui::Button::new(testo)).on_hover_text(comando.to_uppercase()).clicked() {
                cad.start(comando);
            }
        };
        ui.label(egui::RichText::new("Fili e Collegamenti").strong());
        for (t, c) in [
            ("Filo", "filo"),
            ("Connessione", "connessione"),
            ("Alimentazioni", "alimentazioni"),
            ("Collegamenti", "collegamenti"),
            ("Rimandi per Alimentazioni", "rimandialimentazioni"),
            ("Rimandi per Collegamenti", "rimandicollegamenti"),
            ("Quick Filo Verticale (VV)", "vv"),
            ("Quick Filo Orizzontale (HH)", "hh"),
        ] {
            pulsante(ui, t, c);
        }
        ui.separator();
        ui.label(egui::RichText::new("Simboli").strong());
        // dopo i pulsanti, che tengono `cad` occupato
        let edita = ui.add_sized([ui.available_width(), 22.0], egui::Button::new("Edita Simbolo (XS)")).on_hover_text("Modifica il componente selezionato").clicked();
        ui.separator();
        ui.label(egui::RichText::new("Numerazione").strong());
        pulsante(ui, "Numera Fili Automatico", "numerafili");
        pulsante(ui, "Sigle Componenti", "siglecomponenti");
        pulsante(ui, "Numera Fogli", "numerafogli");
        ui.separator();
        ui.label(egui::RichText::new("Morsetti").strong());
        pulsante(ui, "Inser Morsetti", "morsetti");
        if edita {
            self.edita_selezionato(cad);
        }
        ui.add(egui::TextEdit::singleline(&mut self.morsettiera).hint_text("Morsettiera (es. -XG)"));
        if ui.button("Morsettiera in CSV…").clicked()
            && !self.morsettiera.trim().is_empty()
            && let Some(p) = rfd::FileDialog::new().add_filter("CSV", &["csv"]).set_file_name(format!("morsettiera {}.csv", self.morsettiera.trim())).save_file()
        {
            let _ = cad.run("morsettiera", json!({"morsettiera": self.morsettiera.trim(), "file": p.to_string_lossy()}));
        }
        ui.separator();
        ui.label(egui::RichText::new("Verifiche e Output").strong());
        if ui.add_sized([ui.available_width(), 22.0], egui::Button::new("Cross Reference")).clicked() {
            self.cross = cad.run("crossreference", json!({})).ok();
            self.scheda = Scheda::Cross;
        }
        if ui.add_sized([ui.available_width(), 22.0], egui::Button::new("Distinta materiali (CSV)…")).clicked()
            && let Some(p) = rfd::FileDialog::new().add_filter("CSV", &["csv"]).set_file_name("distinta materiali.csv").save_file()
        {
            let _ = cad.run("distintamateriali", json!({"ordine": "tipo", "file": p.to_string_lossy()}));
        }
        if ui.add_sized([ui.available_width(), 22.0], egui::Button::new("Stampa e PDF Fogli…")).clicked() {
            self.esegui(cad, Azione::StampaFogli);
        }
    }
}
