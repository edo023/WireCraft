//! Finestra "Gestione Progetti": le commesse delle cartelle configurate, i multifogli della
//! commessa scelta, nuovo progetto, nuovo multifoglio, apri, modifica, copia; cartelle delle
//! commesse. Si apre all'avvio quando non si apre un disegno.

use std::path::PathBuf;

use cadcraft_ui_egui::CadApp;
use elettrico::commesse::{self, Commessa};
use serde_json::{Value, json};

#[derive(Default)]
pub struct GestioneProgetti {
    pub aperta: bool,
    commesse: Vec<Commessa>,
    scelta: Option<usize>,
    multifoglio: Option<usize>,
    cerca: String,
    nuovo_progetto: Option<[String; 3]>,
    /// Configurazione con nome per il progetto nuovo (vuota: le impostazioni in uso).
    configurazione: String,
    nuovo_multifoglio: Option<[String; 4]>,
    copia: Option<String>,
    nuova_cartella: String,
    messaggio: Option<(bool, String)>,
}

impl GestioneProgetti {
    pub fn apri(&mut self) {
        self.aperta = true;
        self.ricarica();
        // la commessa del disegno attivo
        if let Some(c) = commesse::attiva() {
            self.scelta = self.commesse.iter().position(|x| x.cartella == c.cartella);
        }
    }

    fn ricarica(&mut self) {
        let scelta = self.scelta.and_then(|k| self.commesse.get(k)).map(|c| c.cartella.clone());
        self.commesse = commesse::percorsi().iter().flat_map(|p| commesse::elenco(p)).collect();
        self.scelta = scelta.and_then(|c| self.commesse.iter().position(|x| x.cartella == c));
    }

    fn esegui(&mut self, cad: &mut CadApp, id: &str, p: Value) -> bool {
        let r = cad.run(id, p);
        let ok = r.is_ok();
        self.messaggio = Some(match r {
            Ok(v) => (true, v.get("message").and_then(Value::as_str).unwrap_or("Fatto").to_string()),
            Err(e) => (false, e.to_string()),
        });
        ok
    }

    pub fn mostra(&mut self, cad: &mut CadApp, ctx: &egui::Context) {
        if !self.aperta {
            return;
        }
        let mut aperta = true;
        let mut chiudi = false;
        let mut esempio = false;
        egui::Window::new("Gestione Progetti").id(egui::Id::new("ce_gestione_progetti")).open(&mut aperta).default_width(900.0).default_height(560.0).show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Nuovo Progetto…").clicked() {
                    self.nuovo_progetto = Some(Default::default());
                }
                let c = self.scelta.and_then(|k| self.commesse.get(k)).cloned();
                if ui.add_enabled(c.is_some(), egui::Button::new("Nuovo Multifoglio…")).clicked() {
                    self.nuovo_multifoglio = Some(["SCHEMA".into(), "Schema elettrico".into(), String::new(), String::new()]);
                }
                if ui.add_enabled(c.is_some(), egui::Button::new("Copia Progetto…")).clicked() {
                    self.copia = c.map(|c| format!("{} copia", c.nome()));
                }
                if ui.button("Aggiorna").clicked() {
                    self.ricarica();
                }
                esempio |= ui.button("Progetto d'esempio").on_hover_text("Crea e apre un progetto completo da guardare e provare: nastro trasportatore, PLC, quadro di distribuzione, liste").clicked();
                ui.add(egui::TextEdit::singleline(&mut self.cerca).hint_text("Cerca progetto o cliente").desired_width(200.0));
            });
            self.form_progetto(cad, ui);
            self.form_multifoglio(cad, ui, &mut chiudi);
            self.form_copia(cad, ui);
            ui.separator();
            let filtro = self.cerca.to_lowercase();
            ui.columns(2, |col| {
                col[0].label(egui::RichText::new("Progetti").strong());
                egui::ScrollArea::vertical().id_salt("ce_gp_commesse").max_height(330.0).auto_shrink([false, true]).show(&mut col[0], |ui| {
                    egui::Grid::new("ce_gp_griglia_c").striped(true).num_columns(3).show(ui, |ui| {
                        ui.label(egui::RichText::new("Nome").small().strong());
                        ui.label(egui::RichText::new("Cliente").small().strong());
                        ui.label(egui::RichText::new("Descrizione").small().strong());
                        ui.end_row();
                        for (k, c) in self.commesse.iter().enumerate() {
                            let testo = format!("{} {} {}", c.nome(), c.cliente, c.descrizione).to_lowercase();
                            if !filtro.is_empty() && !testo.contains(&filtro) {
                                continue;
                            }
                            if ui.selectable_label(self.scelta == Some(k), c.nome()).on_hover_text(c.cartella.display().to_string()).clicked() {
                                self.scelta = Some(k);
                                self.multifoglio = None;
                            }
                            ui.label(&c.cliente);
                            ui.label(&c.descrizione);
                            ui.end_row();
                        }
                    });
                    if self.commesse.is_empty() {
                        ui.label("Nessun progetto nelle cartelle configurate.");
                        ui.add_space(6.0);
                        esempio |= ui.button("Crea il progetto d'esempio").on_hover_text("Un progetto completo da guardare e provare").clicked();
                    }
                });
                let c = self.scelta.and_then(|k| self.commesse.get(k)).cloned();
                col[1].label(egui::RichText::new(c.as_ref().map_or("Multifogli".to_string(), |c| format!("Multifogli di {}", c.nome()))).strong());
                let mut apri = None;
                let mut cambia: Option<Value> = None;
                if let Some(c) = &c {
                    egui::ScrollArea::vertical().id_salt("ce_gp_multifogli").max_height(300.0).auto_shrink([false, true]).show(&mut col[1], |ui| {
                        egui::Grid::new("ce_gp_griglia_m").striped(true).num_columns(5).show(ui, |ui| {
                            for t in ["Nr", "Multifoglio", "Descrizione", "Disegn.", "Elab."] {
                                ui.label(egui::RichText::new(t).small().strong());
                            }
                            ui.end_row();
                            for (k, m) in c.multifogli.iter().enumerate() {
                                ui.label(m.nr.to_string());
                                let esiste = c.file(&m.nome).exists();
                                let nome = if esiste { egui::RichText::new(&m.nome) } else { egui::RichText::new(&m.nome).weak() };
                                let r = ui.selectable_label(self.multifoglio == Some(k), nome).on_hover_text(format!("{}\n{}", c.file(&m.nome).display(), commesse::data_leggibile(&m.data)));
                                if r.clicked() {
                                    self.multifoglio = Some(k);
                                }
                                if r.double_clicked() {
                                    apri = Some(m.nome.clone());
                                }
                                ui.label(&m.descrizione);
                                ui.label(&m.disegnatore);
                                let mut elab = m.elaborare;
                                if ui.checkbox(&mut elab, "").on_hover_text("Da gestire nelle elaborazioni").changed() {
                                    cambia = Some(json!({"commessa": c.cartella.to_string_lossy(), "multifoglio": {"nome": m.nome, "elaborare": elab}}));
                                }
                                ui.end_row();
                            }
                        });
                    });
                    col[1].horizontal(|ui| {
                        let m = self.multifoglio.and_then(|k| c.multifogli.get(k));
                        if ui.add_enabled(m.is_some(), egui::Button::new("Apri")).clicked() {
                            apri = m.map(|m| m.nome.clone());
                        }
                        if ui.add_enabled(m.is_some(), egui::Button::new("▲")).on_hover_text("Sposta su").clicked()
                            && let Some(m) = m
                        {
                            cambia = Some(json!({"commessa": c.cartella.to_string_lossy(), "multifoglio": {"nome": m.nome, "sposta": -1}}));
                        }
                        if ui.add_enabled(m.is_some(), egui::Button::new("▼")).on_hover_text("Sposta giù").clicked()
                            && let Some(m) = m
                        {
                            cambia = Some(json!({"commessa": c.cartella.to_string_lossy(), "multifoglio": {"nome": m.nome, "sposta": 1}}));
                        }
                    });
                    col[1].label(egui::RichText::new(format!("Cartella: {}", c.cartella.display())).small());
                }
                if let (Some(c), Some(nome)) = (&c, apri)
                    && self.esegui_apri(cad, c, &nome)
                {
                    chiudi = true;
                }
                if let Some(p) = cambia {
                    self.esegui(cad, "commessa.modifica", p);
                    self.ricarica();
                }
            });
            ui.separator();
            egui::CollapsingHeader::new("Cartelle dei progetti").id_salt("ce_gp_cartelle").show(ui, |ui| {
                let mut togli = None;
                for p in commesse::percorsi() {
                    ui.horizontal(|ui| {
                        ui.label(p.display().to_string());
                        if ui.small_button("✖").on_hover_text("Togli dall'elenco (la cartella resta)").clicked() {
                            togli = Some(p.clone());
                        }
                    });
                }
                ui.horizontal(|ui| {
                    ui.add(egui::TextEdit::singleline(&mut self.nuova_cartella).hint_text("Cartella").desired_width(380.0));
                    if ui.button("Sfoglia…").clicked()
                        && let Some(d) = rfd::FileDialog::new().pick_folder()
                    {
                        self.nuova_cartella = d.to_string_lossy().to_string();
                    }
                    if ui.button("Aggiungi").clicked() && !self.nuova_cartella.trim().is_empty() {
                        let p = PathBuf::from(self.nuova_cartella.trim());
                        if let Err(e) = commesse::cambia_percorsi(Some(&p), None) {
                            self.messaggio = Some((false, e.to_string()));
                        }
                        self.nuova_cartella.clear();
                        self.ricarica();
                    }
                });
                if let Some(t) = togli {
                    let _ = commesse::cambia_percorsi(None, Some(&t));
                    self.ricarica();
                }
            });
            if let Some((ok, m)) = &self.messaggio {
                let colore = if *ok { ui.visuals().text_color() } else { egui::Color32::from_rgb(220, 70, 60) };
                ui.colored_label(colore, m);
            }
        });
        if esempio {
            let p = json!({"radice": commesse::percorsi().first().map(|p| p.to_string_lossy().to_string())});
            if self.esegui(cad, "commessa.esempio", p) {
                self.ricarica();
                chiudi = true;
            }
        }
        self.aperta = aperta && !chiudi;
    }

    fn esegui_apri(&mut self, cad: &mut CadApp, c: &Commessa, nome: &str) -> bool {
        let file = c.file(nome);
        if !file.exists() {
            self.messaggio = Some((false, format!("Il disegno {} non esiste", file.display())));
            return false;
        }
        cad.open_path(&file.to_string_lossy());
        true
    }

    fn form_progetto(&mut self, cad: &mut CadApp, ui: &mut egui::Ui) {
        let Some(campi) = &mut self.nuovo_progetto else { return };
        let mut fatto = false;
        let mut annulla = false;
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.label(egui::RichText::new("Nuovo Progetto").strong());
            egui::Grid::new("ce_gp_np").num_columns(2).show(ui, |ui| {
                for (k, t) in ["Nome (max 32 caratteri)", "Descrizione", "Cliente"].iter().enumerate() {
                    ui.label(*t);
                    ui.add(egui::TextEdit::singleline(&mut campi[k]).desired_width(320.0));
                    ui.end_row();
                }
                ui.label("Configurazione");
                let nomi = elettrico::impostazioni::configurazioni();
                let mostra = if self.configurazione.is_empty() { "impostazioni in uso".to_string() } else { self.configurazione.clone() };
                egui::ComboBox::from_id_salt("ce_gp_conf").selected_text(mostra).width(320.0).show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.configurazione, String::new(), "impostazioni in uso");
                    for n in nomi {
                        ui.selectable_value(&mut self.configurazione, n.clone(), n);
                    }
                });
                ui.end_row();
            });
            ui.horizontal(|ui| {
                fatto = ui.button("Crea").clicked();
                annulla = ui.button("Annulla").clicked();
            });
        });
        if fatto {
            let p = json!({"nome": campi[0].trim(), "descrizione": campi[1].trim(), "cliente": campi[2].trim(), "radice": commesse::percorsi().first().map(|p| p.to_string_lossy().to_string()), "configurazione": self.configurazione});
            let nome = campi[0].trim().to_string();
            if self.esegui(cad, "commessa.nuova", p) {
                self.nuovo_progetto = None;
                self.ricarica();
                self.scelta = self.commesse.iter().position(|c| c.nome() == nome);
                self.nuovo_multifoglio = Some(["SCHEMA".into(), "Schema elettrico".into(), String::new(), String::new()]);
            }
        } else if annulla {
            self.nuovo_progetto = None;
        }
    }

    fn form_multifoglio(&mut self, cad: &mut CadApp, ui: &mut egui::Ui, chiudi: &mut bool) {
        let Some(c) = self.scelta.and_then(|k| self.commesse.get(k)).cloned() else { return };
        let Some(campi) = &mut self.nuovo_multifoglio else { return };
        let mut fatto = false;
        let mut annulla = false;
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.label(egui::RichText::new(format!("Nuovo Multifoglio in {}", c.nome())).strong());
            egui::Grid::new("ce_gp_nm").num_columns(2).show(ui, |ui| {
                for (k, t) in ["Nome", "Descrizione", "Disegnatore", "Prefisso fogli (facoltativo)"].iter().enumerate() {
                    ui.label(*t);
                    ui.add(egui::TextEdit::singleline(&mut campi[k]).desired_width(320.0));
                    ui.end_row();
                }
            });
            ui.horizontal(|ui| {
                fatto = ui.button("Crea e apri").clicked();
                annulla = ui.button("Annulla").clicked();
            });
        });
        if fatto {
            let p = json!({"commessa": c.cartella.to_string_lossy(), "nome": campi[0].trim(), "descrizione": campi[1].trim(), "disegnatore": campi[2].trim(), "prefisso": campi[3].trim()});
            if self.esegui(cad, "commessa.nuovomultifoglio", p) {
                self.nuovo_multifoglio = None;
                self.ricarica();
                *chiudi = true;
            }
        } else if annulla {
            self.nuovo_multifoglio = None;
        }
    }

    fn form_copia(&mut self, cad: &mut CadApp, ui: &mut egui::Ui) {
        let Some(c) = self.scelta.and_then(|k| self.commesse.get(k)).cloned() else { return };
        let Some(nome) = &mut self.copia else { return };
        let mut fatto = false;
        let mut annulla = false;
        ui.horizontal(|ui| {
            ui.label(format!("Copia di {} come", c.nome()));
            ui.add(egui::TextEdit::singleline(nome).desired_width(240.0));
            fatto = ui.button("Copia").clicked();
            annulla = ui.button("Annulla").clicked();
        });
        if fatto {
            let p = json!({"commessa": c.cartella.to_string_lossy(), "nome": nome.trim()});
            if self.esegui(cad, "commessa.copia", p) {
                self.copia = None;
                self.ricarica();
            }
        } else if annulla {
            self.copia = None;
        }
    }
}
