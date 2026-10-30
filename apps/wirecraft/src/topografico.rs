//! Finestra "Gestore Topografico": i componenti del quadro da montare nel lay-out, posati o da
//! posare, e gli ingombri rimasti senza componente nello schema; posa del singolo componente o
//! di tutti quelli mancanti uno dopo l'altro, allineamento allo schema.

use cadcraft_engine::Input;
use cadcraft_ui_egui::CadApp;
use serde_json::{Value, json};

use crate::navigatore::{self, Posto};

#[derive(Default)]
pub struct FinestraTopografico {
    pub aperta: bool,
    quadro: String,
    /// Il foglio del lay-out del quadro (0 se non c'è).
    foglio: u32,
    elementi: Vec<Value>,
    riassunto: String,
    messaggio: Option<(bool, String)>,
    /// Il disegno e la sua revisione quando si è letto l'elenco.
    letto: Option<(u64, u64)>,
}

impl FinestraTopografico {
    pub fn apri(&mut self, cad: &mut CadApp) {
        self.aperta = true;
        self.messaggio = None;
        // il quadro del lay-out sul foglio corrente, altrimenti quello corrente
        let r = cad.run("topografico", json!({})).unwrap_or_default();
        self.quadro = r["quadro"].as_str().unwrap_or_default().to_string();
        self.rileggi(cad);
    }

    fn rileggi(&mut self, cad: &mut CadApp) {
        let r = cad.run("topografico", json!({"quadro": self.quadro.trim()})).unwrap_or_default();
        self.letto = cad.session.state().ok().map(|st| (st.uid, st.revision));
        self.elementi = r["elementi"].as_array().cloned().unwrap_or_default();
        self.foglio = r["foglio"].as_u64().and_then(|f| u32::try_from(f).ok()).unwrap_or(0);
        self.riassunto = r["message"].as_str().unwrap_or_default().to_string();
    }

    fn esito(&mut self, cad: &mut CadApp, r: Result<Value, String>) {
        self.messaggio = Some(match r {
            Ok(v) => (true, v["message"].as_str().unwrap_or("Fatto").to_string()),
            Err(e) => (false, e),
        });
        self.rileggi(cad);
    }

    /// Va sul foglio del lay-out e avvia la posa interattiva (del componente dato, o di tutti
    /// quelli mancanti uno dopo l'altro).
    fn posa(&mut self, cad: &mut CadApp, sigla: Option<&str>) {
        if self.foglio == 0 {
            self.messaggio = Some((false, "Il quadro non ha ancora il lay-out: disegnalo con Layout Quadro".into()));
            return;
        }
        let _ = cad.run("cambiofoglio", json!({"foglio": self.foglio}));
        cad.start("topografico.posa");
        let input = match sigla {
            Some(s) => Input::Text(s.to_string()),
            None => Input::Enter,
        };
        if let Err(e) = cad.session.input(input) {
            cad.session.echo(e.to_string());
        }
        self.messaggio = Some((true, "Indica il punto nel lay-out (Esc per finire)".into()));
    }

    pub fn mostra(&mut self, cad: &mut CadApp, ctx: &egui::Context) {
        if !self.aperta || cad.session.docs.is_empty() {
            return;
        }
        // il disegno è cambiato (una posa, una cancellazione): l'elenco segue
        if cad.session.state().ok().map(|st| (st.uid, st.revision)) != self.letto {
            self.rileggi(cad);
        }
        let mut aperta = true;
        let mut azione: Option<(&'static str, Value)> = None;
        let mut posa: Option<Option<String>> = None;
        let mut vai: Option<Posto> = None;
        let mut rileggi = false;
        egui::Window::new("Gestore Topografico").id(egui::Id::new("ce_topografico")).open(&mut aperta).default_width(720.0).default_height(460.0).show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Quadro");
                rileggi |= ui.add(egui::TextEdit::singleline(&mut self.quadro).desired_width(80.0)).lost_focus();
                ui.label(&self.riassunto);
            });
            if self.foglio == 0 {
                ui.horizontal(|ui| {
                    ui.label("Il quadro non ha ancora il lay-out.");
                    if ui.button("Disegna il lay-out").clicked() {
                        azione = Some(("layoutquadro", json!({"quadro": self.quadro.trim()})));
                    }
                });
            }
            ui.separator();
            egui::ScrollArea::vertical().id_salt("ce_topo_elenco").max_height(320.0).show(ui, |ui| {
                egui::Grid::new("ce_topo_griglia").striped(true).num_columns(6).show(ui, |ui| {
                    for t in ["", "Sigla", "Descrizione", "L × H (mm)", "Codice", ""] {
                        ui.strong(t);
                    }
                    ui.end_row();
                    for e in &self.elementi {
                        let t = |k: &str| e[k].as_str().unwrap_or_default().to_string();
                        let stato = t("stato");
                        let (segno, colore, aiuto) = match stato.as_str() {
                            "posato" => ("●", egui::Color32::from_rgb(70, 170, 80), "Nel lay-out"),
                            "da posare" => ("○", egui::Color32::from_rgb(230, 160, 60), "Da posare nel lay-out"),
                            _ => ("✖", egui::Color32::from_rgb(220, 70, 60), "Nel lay-out ma non più nello schema"),
                        };
                        ui.colored_label(colore, segno).on_hover_text(aiuto);
                        ui.label(egui::RichText::new(t("sigla")).strong());
                        ui.label(t("descrizione"));
                        let misura = format!("{} × {}", e["larghezza"].as_f64().map(|x| format!("{x:.0}")).unwrap_or_default(), e["altezza"].as_f64().map(|x| format!("{x:.0}")).unwrap_or_default());
                        if e["stimata"].as_bool() == Some(true) {
                            ui.label(format!("{misura} *")).on_hover_text("Misura stimata dal tipo di componente: scrivi la larghezza vera nel campo LARGHEZZA del componente");
                        } else {
                            ui.label(misura);
                        }
                        ui.label(t("codice"));
                        ui.horizontal(|ui| match stato.as_str() {
                            "da posare" => {
                                if ui.small_button("Posa").on_hover_text("Indica il punto nel lay-out").clicked() {
                                    posa = Some(Some(t("sigla")));
                                }
                            }
                            _ => {
                                if ui.small_button("Vai").clicked() {
                                    vai = Some(Posto {
                                        testo: t("sigla"),
                                        foglio: e["foglio"].as_u64().and_then(|f| u32::try_from(f).ok()).unwrap_or(0),
                                        pos: (e["x"].as_f64().unwrap_or(0.0), e["y"].as_f64().unwrap_or(0.0)),
                                        handle: t("handle"),
                                    });
                                }
                                if ui.small_button("Togli").on_hover_text("Toglie l'ingombro dal lay-out").clicked() {
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
                let mancanti = self.elementi.iter().filter(|e| e["stato"] == "da posare").count();
                if ui.add_enabled(mancanti > 0 && self.foglio > 0, egui::Button::new(format!("Disponi i mancanti ({mancanti})"))).on_hover_text("In coda sulle guide dove c'è posto; quelli già posati non si muovono").clicked() {
                    azione = Some(("layoutquadro", json!({"quadro": self.quadro.trim()})));
                }
                if ui.add_enabled(mancanti > 0, egui::Button::new("Posali a mano")).on_hover_text("Uno dopo l'altro: un punto per ognuno").clicked() {
                    posa = Some(None);
                }
                if self.foglio > 0 && ui.button("Rifai il lay-out").on_hover_text("Ridisegna piastra, guide e ingombri da capo sullo stesso foglio: le posizioni scelte a mano si perdono").clicked() {
                    azione = Some(("layoutquadro", json!({"quadro": self.quadro.trim(), "rifai": true})));
                }
                if ui.button("Aggiorna dallo schema").on_hover_text("Sigle e codici degli ingombri dai componenti").clicked() {
                    azione = Some(("topografico.aggiorna", json!({})));
                }
                let orfani = self.elementi.iter().filter(|e| e["stato"] == "non nello schema").count();
                if ui.add_enabled(orfani > 0, egui::Button::new(format!("Togli quelli non nello schema ({orfani})"))).clicked() {
                    azione = Some(("topografico.aggiorna", json!({"togliOrfani": true})));
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
        self.aperta = aperta;
    }
}
