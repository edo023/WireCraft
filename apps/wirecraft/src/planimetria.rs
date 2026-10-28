//! Finestra "Planimetria": i quadri del progetto sulla pianta dell'impianto (posati o da
//! posare) e il calcolo della lunghezza dei cavi dai percorsi, con risalite e scorta.

use cadcraft_engine::Input;
use cadcraft_ui_egui::CadApp;
use serde_json::{Value, json};

use crate::navigatore::{self, Posto};

pub struct FinestraPlanimetria {
    pub aperta: bool,
    quadri: Vec<Value>,
    scorta: String,
    risalita: String,
    solo_mancanti: bool,
    messaggio: Option<(bool, String)>,
    letto: Option<(u64, u64)>,
}

impl Default for FinestraPlanimetria {
    fn default() -> Self {
        FinestraPlanimetria { aperta: false, quadri: Vec::new(), scorta: "10".into(), risalita: "2".into(), solo_mancanti: false, messaggio: None, letto: None }
    }
}

impl FinestraPlanimetria {
    pub fn apri(&mut self, cad: &mut CadApp) {
        self.aperta = true;
        self.messaggio = None;
        self.rileggi(cad);
    }

    fn rileggi(&mut self, cad: &mut CadApp) {
        let r = cad.run("planimetria", json!({})).unwrap_or_default();
        self.quadri = r["quadri"].as_array().cloned().unwrap_or_default();
        self.letto = cad.session.state().ok().map(|st| (st.uid, st.revision));
    }

    fn esito(&mut self, cad: &mut CadApp, r: Result<Value, String>) {
        self.messaggio = Some(match r {
            Ok(v) => (true, v["message"].as_str().unwrap_or("Fatto").to_string()),
            Err(e) => (false, e),
        });
        self.rileggi(cad);
    }

    /// Avvia la posa interattiva sul foglio corrente (del quadro dato, o dei mancanti).
    fn posa(&mut self, cad: &mut CadApp, quadro: Option<&str>) {
        cad.start("planimetria.posa");
        let input = match quadro {
            Some(q) => Input::Text(q.to_string()),
            None => Input::Enter,
        };
        if let Err(e) = cad.session.input(input) {
            cad.session.echo(e.to_string());
        }
        self.messaggio = Some((true, "Indica il punto sulla pianta del foglio corrente (Esc per finire)".into()));
    }

    pub fn mostra(&mut self, cad: &mut CadApp, ctx: &egui::Context) {
        if !self.aperta || cad.session.docs.is_empty() {
            return;
        }
        if cad.session.state().ok().map(|st| (st.uid, st.revision)) != self.letto {
            self.rileggi(cad);
        }
        let mut aperta = true;
        let mut azione: Option<(&'static str, Value)> = None;
        let mut posa: Option<Option<String>> = None;
        let mut vai: Option<Posto> = None;
        egui::Window::new("Planimetria").id(egui::Id::new("ce_planimetria")).open(&mut aperta).default_width(560.0).show(ctx, |ui| {
            ui.label("Disegna o importa la pianta dell'impianto su un foglio, poi posa i quadri: le lunghezze dei cavi si calcolano dai percorsi.");
            ui.horizontal(|ui| {
                if ui.button("Nuovo foglio di planimetria").clicked() {
                    azione = Some(("planimetria.nuovo", json!({})));
                }
                let mancanti = self.quadri.iter().filter(|q| q["posato"] == false).count();
                if ui.add_enabled(mancanti > 0, egui::Button::new(format!("Posa i mancanti ({mancanti})"))).on_hover_text("Sul foglio corrente, uno dopo l'altro").clicked() {
                    posa = Some(None);
                }
            });
            ui.separator();
            egui::Grid::new("ce_plan_quadri").striped(true).num_columns(4).show(ui, |ui| {
                for t in ["Quadro", "Descrizione", "Sulla pianta", ""] {
                    ui.strong(t);
                }
                ui.end_row();
                for q in &self.quadri {
                    let t = |k: &str| q[k].as_str().unwrap_or_default().to_string();
                    ui.label(egui::RichText::new(t("quadro")).strong());
                    ui.label(t("descrizione"));
                    if q["posato"] == true {
                        ui.colored_label(egui::Color32::from_rgb(70, 170, 80), format!("foglio {}, scala 1:{}", q["foglio"], q["scala"]));
                        if ui.small_button("Vai").clicked() {
                            vai = Some(Posto {
                                testo: t("quadro"),
                                foglio: q["foglio"].as_u64().and_then(|f| u32::try_from(f).ok()).unwrap_or(0),
                                pos: (q["x"].as_f64().unwrap_or(0.0), q["y"].as_f64().unwrap_or(0.0)),
                                handle: t("handle"),
                            });
                        }
                    } else {
                        ui.colored_label(egui::Color32::from_rgb(230, 160, 60), "da posare");
                        if ui.small_button("Posa").on_hover_text("Indica il punto sulla pianta del foglio corrente").clicked() {
                            posa = Some(Some(t("quadro")));
                        }
                    }
                    ui.end_row();
                }
            });
            ui.separator();
            ui.horizontal(|ui| {
                ui.label("Scorta %");
                ui.add(egui::TextEdit::singleline(&mut self.scorta).desired_width(36.0));
                ui.label("Risalita per estremità (m)");
                ui.add(egui::TextEdit::singleline(&mut self.risalita).desired_width(36.0));
                ui.checkbox(&mut self.solo_mancanti, "solo i cavi senza lunghezza");
            });
            if ui.button("Calcola le lunghezze dei cavi").on_hover_text("Percorso ortogonale tra i quadri, in scala, più risalite e scorta; scritto nei cavi").clicked() {
                azione = Some(("planimetria.lunghezze", json!({"scorta": self.scorta.trim(), "risalita": self.risalita.trim(), "soloMancanti": self.solo_mancanti})));
            }
            if let Some((ok, m)) = &self.messaggio {
                let colore = if *ok { ui.visuals().text_color() } else { egui::Color32::from_rgb(220, 70, 60) };
                ui.colored_label(colore, m);
            }
        });
        if let Some((comando, p)) = azione {
            let r = cad.run(comando, p);
            self.esito(cad, r);
        }
        if let Some(q) = posa {
            self.posa(cad, q.as_deref());
        }
        if let Some(p) = vai {
            navigatore::vai(cad, &p);
        }
        self.aperta = aperta;
    }
}
