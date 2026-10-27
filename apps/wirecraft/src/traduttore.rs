//! Finestra "Traduttore": lingue di partenza e di arrivo, frasi del progetto ancora senza
//! traduzione (da completare al volo), dizionario con ricerca, traduzione del progetto,
//! importazione ed esportazione del dizionario in CSV.

use cadcraft_ui_egui::CadApp;
use elettrico::traduttore;
use serde_json::{Value, json};

#[derive(Default)]
pub struct FinestraTraduttore {
    pub aperta: bool,
    da: String,
    a: String,
    sovrascrivi: bool,
    letto: Option<(u64, u64, String, String)>,
    /// Frasi del progetto senza traduzione, con la traduzione che si sta scrivendo.
    mancanti: Vec<(String, String)>,
    voci: Vec<(String, String)>,
    cerca: String,
    nuova: (String, String),
    messaggio: Option<(bool, String)>,
}

impl FinestraTraduttore {
    pub fn apri(&mut self) {
        self.aperta = true;
        self.letto = None;
        if self.da.is_empty() {
            self.da = "IT".into();
            self.a = "EN".into();
        }
    }

    fn rileggi(&mut self, cad: &mut CadApp) {
        let chiave = cad.session.state().ok().map(|st| (st.uid, st.revision, self.da.clone(), self.a.clone()));
        if chiave == self.letto && self.letto.is_some() {
            return;
        }
        self.voci = traduttore::voci(&self.da, &self.a);
        let mancanti = traduttore::mancanti(&mut cad.session, &self.da, &self.a).unwrap_or_default();
        // le traduzioni già scritte restano
        self.mancanti = mancanti.into_iter().map(|f| {
            let t = self.mancanti.iter().find(|(x, _)| *x == f).map(|(_, t)| t.clone()).unwrap_or_default();
            (f, t)
        }).collect();
        self.letto = chiave;
    }

    fn esito(&mut self, r: Result<Value, String>) {
        self.messaggio = Some(match r {
            Ok(v) => (true, v.get("message").and_then(Value::as_str).unwrap_or("Fatto").to_string()),
            Err(e) => (false, e),
        });
        self.letto = None;
    }

    pub fn mostra(&mut self, cad: &mut CadApp, ctx: &egui::Context) {
        if !self.aperta || cad.session.docs.is_empty() {
            return;
        }
        self.rileggi(cad);
        let mut aperta = true;
        let lingue = traduttore::lingue();
        egui::Window::new("Traduttore").id(egui::Id::new("ce_traduttore")).open(&mut aperta).default_width(640.0).show(ctx, |ui| {
            ui.horizontal(|ui| {
                for (id, etichetta, lingua) in [("ce_tr_da", "Da", &mut self.da), ("ce_tr_a", "a", &mut self.a)] {
                    ui.label(etichetta);
                    egui::ComboBox::from_id_salt(id).width(70.0).selected_text(lingua.as_str()).show_ui(ui, |ui| {
                        for l in &lingue {
                            ui.selectable_value(lingua, l.clone(), l.as_str());
                        }
                    });
                }
                ui.separator();
                ui.checkbox(&mut self.sovrascrivi, "Sovrascrivi le traduzioni già scritte");
                if ui.button("Traduci progetto").on_hover_text("Funzioni dei componenti, titoli dei fogli e descrizioni delle colonne").clicked() {
                    let r = cad.run("traduci", json!({"da": self.da, "a": self.a, "sovrascrivi": self.sovrascrivi}));
                    self.esito(r);
                }
            });
            ui.separator();
            ui.label(egui::RichText::new(format!("Frasi del progetto senza traduzione ({})", self.mancanti.len())).strong());
            let mut aggiungi: Option<usize> = None;
            egui::ScrollArea::vertical().id_salt("ce_tr_mancanti").max_height(160.0).show(ui, |ui| {
                egui::Grid::new("ce_tr_mancanti_griglia").striped(true).num_columns(3).show(ui, |ui| {
                    for (k, (f, t)) in self.mancanti.iter_mut().enumerate() {
                        ui.label(f.as_str());
                        let r = ui.add(egui::TextEdit::singleline(t).hint_text("traduzione").desired_width(240.0));
                        if (ui.small_button("➕").on_hover_text("Aggiungi al dizionario").clicked() || r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))) && !t.trim().is_empty() {
                            aggiungi = Some(k);
                        }
                        ui.end_row();
                    }
                });
                if self.mancanti.is_empty() {
                    ui.label("Tutte le frasi del progetto hanno la traduzione.");
                }
            });
            if let Some((f, t)) = aggiungi.and_then(|k| self.mancanti.get(k).cloned()) {
                let r = cad.run("traduttore.aggiungi", json!({"frase": f, "traduzione": t, "da": self.da, "a": self.a}));
                self.esito(r);
            }
            ui.separator();
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(format!("Dizionario ({} frasi)", self.voci.len())).strong());
                ui.add(egui::TextEdit::singleline(&mut self.cerca).hint_text("Cerca").desired_width(200.0));
            });
            let cerca = self.cerca.to_uppercase();
            egui::ScrollArea::vertical().id_salt("ce_tr_voci").max_height(200.0).show(ui, |ui| {
                egui::Grid::new("ce_tr_voci_griglia").striped(true).num_columns(2).show(ui, |ui| {
                    for (f, t) in self.voci.iter().filter(|(f, t)| cerca.is_empty() || f.to_uppercase().contains(&cerca) || t.to_uppercase().contains(&cerca)) {
                        ui.label(f.as_str());
                        if t.is_empty() {
                            ui.label(egui::RichText::new("—").weak());
                        } else {
                            ui.label(t.as_str());
                        }
                        ui.end_row();
                    }
                });
            });
            ui.horizontal(|ui| {
                ui.add(egui::TextEdit::singleline(&mut self.nuova.0).hint_text(format!("Frase ({})", self.da)).desired_width(220.0));
                ui.add(egui::TextEdit::singleline(&mut self.nuova.1).hint_text(format!("Traduzione ({})", self.a)).desired_width(220.0));
                if ui.button("Aggiungi").clicked() && !self.nuova.0.trim().is_empty() {
                    let (f, t) = std::mem::take(&mut self.nuova);
                    let r = cad.run("traduttore.aggiungi", json!({"frase": f, "traduzione": t, "da": self.da, "a": self.a}));
                    self.esito(r);
                }
            });
            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("Importa dizionario (CSV)…").clicked()
                    && let Some(p) = rfd::FileDialog::new().add_filter("CSV", &["csv", "txt"]).pick_file()
                {
                    let r = cad.run("traduttore.importa", json!({"file": p.to_string_lossy()}));
                    self.esito(r);
                }
                if ui.button("Esporta dizionario (CSV)…").clicked()
                    && let Some(p) = rfd::FileDialog::new().add_filter("CSV", &["csv"]).set_file_name("dizionario.csv").save_file()
                {
                    let r = cad.run("traduttore.esporta", json!({"file": p.to_string_lossy()}));
                    self.esito(r);
                }
            });
            if let Some((ok, t)) = &self.messaggio {
                let colore = if *ok { ui.visuals().text_color() } else { egui::Color32::from_rgb(220, 70, 60) };
                ui.colored_label(colore, t);
            }
        });
        self.aperta &= aperta;
    }
}
