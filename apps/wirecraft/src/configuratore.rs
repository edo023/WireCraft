//! Finestra "Configuratore": le opzioni della macchina con il loro gruppo di esclusione, le parti
//! dello schema abbinate dalla selezione, e il progetto generato con le sole opzioni scelte.

use cadcraft_ui_egui::CadApp;
use serde_json::{Value, json};

#[derive(Default)]
pub struct FinestraConfiguratore {
    pub aperta: bool,
    /// (codice, descrizione, gruppo, parti) come le dà il comando CONFIGURATORE.
    opzioni: Vec<(String, String, String, u64)>,
    /// Le opzioni scelte per generare il progetto.
    scelte: Vec<String>,
    nuova: [String; 3],
    messaggio: Option<(bool, String)>,
}

impl FinestraConfiguratore {
    pub fn apri(&mut self, cad: &mut CadApp) {
        self.aperta = true;
        self.messaggio = None;
        self.rileggi(cad);
    }

    fn rileggi(&mut self, cad: &mut CadApp) {
        let r = cad.run("configuratore", json!({})).unwrap_or_default();
        self.opzioni = r["opzioni"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|o| {
                let t = |k: &str| o[k].as_str().unwrap_or_default().to_string();
                (t("codice"), t("descrizione"), t("gruppo"), o["parti"].as_u64().unwrap_or(0))
            })
            .collect();
        self.scelte.retain(|c| self.opzioni.iter().any(|o| o.0 == *c));
    }

    fn esito(&mut self, cad: &mut CadApp, r: Result<Value, String>) {
        self.messaggio = Some(match r {
            Ok(v) => (true, v.get("message").and_then(Value::as_str).unwrap_or("Fatto").to_string()),
            Err(e) => (false, e),
        });
        self.rileggi(cad);
    }

    pub fn mostra(&mut self, cad: &mut CadApp, ctx: &egui::Context) {
        if !self.aperta || cad.session.docs.is_empty() {
            return;
        }
        let mut aperta = true;
        let mut azione: Option<(&'static str, Value)> = None;
        let mut genera = false;
        egui::Window::new("Configuratore").id(egui::Id::new("ce_configuratore")).open(&mut aperta).default_width(560.0).show(ctx, |ui| {
            ui.label("Le opzioni della macchina: abbina le parti dello schema (seleziona e premi Abbina), poi scegli le opzioni e genera il progetto. Le opzioni dello stesso gruppo si escludono.");
            ui.add_space(4.0);
            egui::Grid::new("ce_conf_tab").striped(true).num_columns(6).show(ui, |ui| {
                for t in ["", "Codice", "Descrizione", "Gruppo", "Parti", ""] {
                    ui.strong(t);
                }
                ui.end_row();
                for (codice, descrizione, gruppo, parti) in &self.opzioni {
                    let mut scelta = self.scelte.contains(codice);
                    if ui.checkbox(&mut scelta, "").changed() {
                        if scelta {
                            // una sola opzione per gruppo
                            if !gruppo.is_empty() {
                                let stesse: Vec<String> = self.opzioni.iter().filter(|o| o.2 == *gruppo).map(|o| o.0.clone()).collect();
                                self.scelte.retain(|c| !stesse.contains(c));
                            }
                            self.scelte.push(codice.clone());
                        } else {
                            self.scelte.retain(|c| c != codice);
                        }
                    }
                    ui.label(egui::RichText::new(codice).strong());
                    ui.label(descrizione);
                    ui.label(gruppo);
                    ui.label(parti.to_string());
                    ui.horizontal(|ui| {
                        if ui.small_button("Abbina").on_hover_text("Le parti selezionate vanno con questa opzione").clicked() {
                            azione = Some(("configuratore.abbina", json!({"codice": codice})));
                        }
                        if ui.small_button("Stacca").on_hover_text("Le parti selezionate tornano comuni").clicked() {
                            azione = Some(("configuratore.abbina", json!({"codice": codice, "togli": true})));
                        }
                        if ui.small_button("🗑").on_hover_text("Elimina l'opzione (le parti restano, comuni)").clicked() {
                            azione = Some(("configuratore.elimina", json!({"codice": codice})));
                        }
                    });
                    ui.end_row();
                }
            });
            ui.separator();
            ui.horizontal(|ui| {
                ui.add(egui::TextEdit::singleline(&mut self.nuova[0]).hint_text("Codice (P01)").desired_width(70.0));
                ui.add(egui::TextEdit::singleline(&mut self.nuova[1]).hint_text("Descrizione").desired_width(200.0));
                ui.add(egui::TextEdit::singleline(&mut self.nuova[2]).hint_text("Gruppo").desired_width(70.0));
                if ui.add_enabled(!self.nuova[0].trim().is_empty(), egui::Button::new("Aggiungi")).clicked() {
                    azione = Some(("configuratore", json!({"definisci": [{"codice": self.nuova[0].trim(), "descrizione": self.nuova[1].trim(), "gruppo": self.nuova[2].trim()}]})));
                    self.nuova = Default::default();
                }
            });
            ui.separator();
            ui.horizontal(|ui| {
                ui.label(if self.scelte.is_empty() { "Nessuna opzione scelta: solo le parti comuni".to_string() } else { format!("Scelte: {}", self.scelte.join(", ")) });
                genera = ui.button("Genera progetto…").on_hover_text("Un nuovo disegno con le sole opzioni scelte; questo resta com'è").clicked();
            });
            if let Some((ok, m)) = &self.messaggio {
                let colore = if *ok { ui.visuals().text_color() } else { egui::Color32::from_rgb(220, 70, 60) };
                ui.colored_label(colore, m);
            }
        });
        if let Some((comando, p)) = azione {
            let r = cad.run(comando, p);
            self.esito(cad, r);
        }
        if genera && let Some(f) = rfd::FileDialog::new().add_filter("DXF", &["dxf"]).set_file_name(format!("progetto {}.dxf", self.scelte.join(" "))).save_file() {
            let r = cad.run("configuratore.genera", json!({"scelte": self.scelte, "file": f.to_string_lossy()}));
            let ok = r.is_ok();
            self.esito(cad, r);
            if ok {
                cad.open_path(&f.to_string_lossy());
            }
        }
        self.aperta = aperta;
    }
}
