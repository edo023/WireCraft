//! Finestra "Fronte Quadro": i pulsanti, le lampade e i selettori del quadro sulla porta,
//! posati o da posare; la porta disegnata, la posa del singolo dispositivo o di tutti i
//! mancanti, l'allineamento allo schema, la foratura e l'elenco con le quote.

use cadcraft_engine::Input;
use cadcraft_ui_egui::CadApp;
use serde_json::{Value, json};

use crate::navigatore::{self, Posto};

#[derive(Default)]
pub struct FinestraFronte {
    pub aperta: bool,
    quadro: String,
    larghezza: String,
    altezza: String,
    /// Il foglio della porta (0 se non è disegnata).
    foglio: u32,
    porta: (f64, f64),
    dispositivi: Vec<Value>,
    riassunto: String,
    messaggio: Option<(bool, String)>,
    letto: Option<(u64, u64)>,
}

impl FinestraFronte {
    pub fn apri(&mut self, cad: &mut CadApp) {
        self.aperta = true;
        self.messaggio = None;
        // il quadro della porta sul foglio corrente, altrimenti quello corrente
        let r = cad.run("pulsantiera", json!({})).unwrap_or_default();
        self.quadro = r["quadro"].as_str().unwrap_or_default().to_string();
        self.rileggi(cad);
    }

    fn rileggi(&mut self, cad: &mut CadApp) {
        let r = cad.run("pulsantiera", json!({"quadro": self.quadro.trim()}));
        self.letto = cad.session.state().ok().map(|st| (st.uid, st.revision));
        match r {
            Ok(r) => {
                self.dispositivi = r["dispositivi"].as_array().cloned().unwrap_or_default();
                self.foglio = r["foglio"].as_u64().and_then(|f| u32::try_from(f).ok()).unwrap_or(0);
                self.porta = (r["porta"][0].as_f64().unwrap_or(600.0), r["porta"][1].as_f64().unwrap_or(800.0));
                self.riassunto = r["message"].as_str().unwrap_or_default().to_string();
            }
            Err(e) => {
                self.dispositivi.clear();
                self.riassunto = e;
            }
        }
    }

    fn esito(&mut self, cad: &mut CadApp, r: Result<Value, String>) {
        self.messaggio = Some(match r {
            Ok(v) => (true, v["message"].as_str().unwrap_or("Fatto").to_string()),
            Err(e) => (false, e),
        });
        self.rileggi(cad);
    }

    /// Va sul foglio della porta e avvia la posa interattiva (del dispositivo dato, o dei mancanti).
    fn posa(&mut self, cad: &mut CadApp, sigla: Option<&str>) {
        if self.foglio == 0 {
            self.messaggio = Some((false, "Disegna prima la porta".into()));
            return;
        }
        let _ = cad.run("cambiofoglio", json!({"foglio": self.foglio}));
        cad.start("pulsantiera.posa");
        let input = match sigla {
            Some(s) => Input::Text(s.to_string()),
            None => Input::Enter,
        };
        if let Err(e) = cad.session.input(input) {
            cad.session.echo(e.to_string());
        }
        self.messaggio = Some((true, "Indica il punto sulla porta (Esc per finire)".into()));
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
        let mut rileggi = false;
        let mut foratura = false;
        egui::Window::new("Fronte Quadro").id(egui::Id::new("ce_fronte")).open(&mut aperta).default_width(640.0).show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Quadro");
                rileggi |= ui.add(egui::TextEdit::singleline(&mut self.quadro).desired_width(70.0)).lost_focus();
                ui.label(&self.riassunto);
            });
            ui.horizontal(|ui| {
                if self.foglio == 0 {
                    ui.label("Porta (mm)");
                    ui.add(egui::TextEdit::singleline(&mut self.larghezza).hint_text(format!("{:.0}", self.porta.0)).desired_width(50.0));
                    ui.label("×");
                    ui.add(egui::TextEdit::singleline(&mut self.altezza).hint_text(format!("{:.0}", self.porta.1)).desired_width(50.0));
                    if ui.button("Disegna la porta").on_hover_text("Su un foglio nuovo, con i dispositivi disposti in file: poi si spostano a mano e restano dove li metti").clicked() {
                        let mut p = json!({"quadro": self.quadro.trim()});
                        if let (Ok(l), Ok(a)) = (self.larghezza.trim().parse::<f64>(), self.altezza.trim().parse::<f64>()) {
                            p["larghezza"] = json!(l);
                            p["altezza"] = json!(a);
                        }
                        azione = Some(("pulsantiera.disegna", p));
                    }
                } else {
                    ui.label(format!("Porta {:.0} × {:.0} mm sul foglio {}", self.porta.0, self.porta.1, self.foglio));
                    let mancanti = self.dispositivi.iter().filter(|d| d["stato"] == "da posare").count();
                    if ui.add_enabled(mancanti > 0, egui::Button::new(format!("Disponi i mancanti ({mancanti})"))).on_hover_text("Nei posti liberi; quelli già sulla porta non si muovono").clicked() {
                        azione = Some(("pulsantiera.disegna", json!({"quadro": self.quadro.trim()})));
                    }
                    if ui.add_enabled(mancanti > 0, egui::Button::new("Posali a mano")).on_hover_text("Uno dopo l'altro: un punto per ognuno").clicked() {
                        posa = Some(None);
                    }
                }
            });
            ui.separator();
            egui::ScrollArea::vertical().id_salt("ce_fronte_elenco").max_height(300.0).show(ui, |ui| {
                egui::Grid::new("ce_fronte_griglia").striped(true).num_columns(6).show(ui, |ui| {
                    for t in ["", "Sigla", "Funzione", "X", "Y", ""] {
                        ui.strong(t);
                    }
                    ui.end_row();
                    for d in &self.dispositivi {
                        let t = |k: &str| d[k].as_str().unwrap_or_default().to_string();
                        let stato = t("stato");
                        let (segno, colore, aiuto) = match stato.as_str() {
                            "posato" => ("●", egui::Color32::from_rgb(70, 170, 80), "Sulla porta"),
                            "da posare" => ("○", egui::Color32::from_rgb(230, 160, 60), "Da posare: X e Y sono il posto proposto"),
                            _ => ("✖", egui::Color32::from_rgb(220, 70, 60), "Sulla porta ma non più nello schema"),
                        };
                        ui.colored_label(colore, segno).on_hover_text(aiuto);
                        ui.label(egui::RichText::new(t("sigla")).strong());
                        ui.label(t("funzione"));
                        ui.label(format!("{:.0}", d["x"].as_f64().unwrap_or(0.0)));
                        ui.label(format!("{:.0}", d["y"].as_f64().unwrap_or(0.0)));
                        ui.horizontal(|ui| {
                            if stato == "da posare" {
                                if self.foglio > 0 && ui.small_button("Posa").on_hover_text("Indica il punto sulla porta").clicked() {
                                    posa = Some(Some(t("sigla")));
                                }
                            } else {
                                if ui.small_button("Vai").clicked() {
                                    vai = Some(Posto {
                                        testo: t("sigla"),
                                        foglio: self.foglio,
                                        pos: (d["xFoglio"].as_f64().unwrap_or(0.0), d["yFoglio"].as_f64().unwrap_or(0.0)),
                                        handle: t("handle"),
                                    });
                                }
                                if ui.small_button("Togli").on_hover_text("Toglie il dispositivo dalla porta (resta nello schema)").clicked() {
                                    azione = Some(("erase", json!({"handles": [t("handle")]})));
                                }
                            }
                        });
                        ui.end_row();
                    }
                });
            });
            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("Aggiorna dallo schema").on_hover_text("Sigle e funzioni dei dispositivi dai componenti").clicked() {
                    azione = Some(("pulsantiera.aggiorna", json!({})));
                }
                let orfani = self.dispositivi.iter().filter(|d| d["stato"] == "non nello schema").count();
                if ui.add_enabled(orfani > 0, egui::Button::new(format!("Togli quelli non nello schema ({orfani})"))).clicked() {
                    azione = Some(("pulsantiera.aggiorna", json!({"togliOrfani": true})));
                }
                foratura = ui.button("Foratura (CSV)…").on_hover_text("Le quote dei fori Ø22,5 dall'angolo in basso a sinistra della porta").clicked();
                if ui.button("Elenco sul foglio").on_hover_text("Sigle, funzioni e quote in una tabella su un foglio nuovo").clicked() {
                    azione = Some(("uscita", json!({"tipo": "pulsantiera", "quadro": self.quadro.trim()})));
                }
            });
            if let Some((ok, m)) = &self.messaggio {
                let colore = if *ok { ui.visuals().text_color() } else { egui::Color32::from_rgb(220, 70, 60) };
                ui.colored_label(colore, m);
            }
        });
        if let Some((comando, p)) = azione {
            let r = cad.run(comando, p);
            self.esito(cad, r);
        } else if rileggi {
            self.rileggi(cad);
        }
        if let Some(sigla) = posa {
            self.posa(cad, sigla.as_deref());
        }
        if let Some(p) = vai {
            navigatore::vai(cad, &p);
        }
        if foratura && let Some(f) = rfd::FileDialog::new().add_filter("CSV", &["csv"]).set_file_name(format!("foratura porta {}.csv", self.quadro.trim())).save_file() {
            let r = cad.run("pulsantiera", json!({"quadro": self.quadro.trim(), "file": f.to_string_lossy()}));
            self.esito(cad, r);
        }
        self.aperta = aperta;
    }
}
