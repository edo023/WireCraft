//! Finestra "Sovratemperature": armadio (misure, installazione, materiale), temperature di
//! progetto, potenze dissipate dei componenti del quadro ed esito del calcolo termico; il
//! resoconto si disegna su un foglio come uscita aggiornabile.

use cadcraft_ui_egui::CadApp;
use elettrico::termico::{Installazione, Materiale};
use serde_json::{Value, json};

#[derive(Default)]
pub struct FinestraTermico {
    pub aperta: bool,
    quadro: String,
    misure: [f64; 3],
    installazione: u8,
    materiale: String,
    t_esterna: f64,
    t_massima: f64,
    anticondensa: bool,
    t_esterna_min: f64,
    t_minima: f64,
    /// Ultimo esito del comando CALCOLOTERMICO.
    esito: Option<Value>,
    messaggio: Option<(bool, String)>,
}

impl FinestraTermico {
    /// Apre la finestra sul quadro corrente, con l'armadio proposto dal lay-out (se c'è).
    pub fn apri(&mut self, cad: &mut CadApp) {
        self.aperta = true;
        self.quadro = elettrico::impostazioni::quadro_corrente().unwrap_or_default();
        self.t_esterna = 35.0;
        self.t_massima = 40.0;
        self.t_esterna_min = -5.0;
        self.t_minima = 10.0;
        self.esito = None;
        self.messaggio = None;
        if let Ok(r) = cad.run("calcolotermico", json!({"quadro": self.quadro})) {
            let a = &r["armadio"];
            self.misure = [a["larghezza"].as_f64().unwrap_or(600.0), a["altezza"].as_f64().unwrap_or(800.0), a["profondita"].as_f64().unwrap_or(250.0)];
            self.installazione = a["installazione"].as_u64().and_then(|n| u8::try_from(n).ok()).unwrap_or(2);
            self.materiale = a["materiale"].as_str().unwrap_or("acciaio").to_string();
            self.esito = Some(r);
        }
    }

    fn parametri(&self) -> Value {
        let mut p = json!({
            "quadro": self.quadro.trim(),
            "larghezza": self.misure[0], "altezza": self.misure[1], "profondita": self.misure[2],
            "installazione": self.installazione, "materiale": self.materiale,
            "tEsterna": self.t_esterna, "tMassima": self.t_massima,
        });
        if self.anticondensa {
            p["tEsternaMin"] = json!(self.t_esterna_min);
            p["tMinima"] = json!(self.t_minima);
        }
        p
    }

    pub fn mostra(&mut self, cad: &mut CadApp, ctx: &egui::Context) {
        if !self.aperta || cad.session.docs.is_empty() {
            return;
        }
        let mut aperta = true;
        let (mut calcola, mut disegna) = (false, false);
        egui::Window::new("Sovratemperature").id(egui::Id::new("ce_termico")).open(&mut aperta).default_width(600.0).show(ctx, |ui| {
            egui::Grid::new("ce_termico_dati").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                ui.label("Quadro");
                ui.add(egui::TextEdit::singleline(&mut self.quadro).desired_width(100.0).hint_text("+QG"));
                ui.end_row();
                ui.label("Armadio L × A × P (mm)");
                ui.horizontal(|ui| {
                    for m in &mut self.misure {
                        ui.add(egui::DragValue::new(m).range(50.0..=5000.0).speed(10.0));
                    }
                });
                ui.end_row();
                ui.label("Installazione");
                let attuale = Installazione::da_numero(u64::from(self.installazione)).map_or("", Installazione::descrizione);
                egui::ComboBox::from_id_salt("ce_termico_inst").width(320.0).selected_text(attuale).show_ui(ui, |ui| {
                    for i in Installazione::TUTTE {
                        ui.selectable_value(&mut self.installazione, i as u8, i.descrizione());
                    }
                });
                ui.end_row();
                ui.label("Materiale");
                let attuale = Materiale::da_codice(&self.materiale).map_or("", Materiale::descrizione);
                egui::ComboBox::from_id_salt("ce_termico_mat").width(320.0).selected_text(attuale).show_ui(ui, |ui| {
                    for m in Materiale::TUTTI {
                        ui.selectable_value(&mut self.materiale, m.codice().to_string(), format!("{} (k = {} W/m²K)", m.descrizione(), m.k()));
                    }
                });
                ui.end_row();
                ui.label("Temperatura esterna / massima interna (°C)");
                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(&mut self.t_esterna).range(-40.0..=70.0).speed(0.5));
                    ui.add(egui::DragValue::new(&mut self.t_massima).range(0.0..=80.0).speed(0.5));
                });
                ui.end_row();
                ui.checkbox(&mut self.anticondensa, "Anticondensa: esterna minima / interna minima");
                ui.add_enabled_ui(self.anticondensa, |ui| {
                    ui.horizontal(|ui| {
                        ui.add(egui::DragValue::new(&mut self.t_esterna_min).range(-50.0..=40.0).speed(0.5));
                        ui.add(egui::DragValue::new(&mut self.t_minima).range(-20.0..=40.0).speed(0.5));
                    });
                });
                ui.end_row();
            });
            ui.horizontal(|ui| {
                calcola = ui.button("Calcola").clicked();
                disegna = ui.button("Disegna sul foglio").on_hover_text("Il resoconto su un foglio nuovo, aggiornato da Aggiorna Liste e Tabelle").clicked();
                if let Some((ok, m)) = &self.messaggio {
                    let colore = if *ok { ui.visuals().text_color() } else { egui::Color32::from_rgb(220, 70, 60) };
                    ui.colored_label(colore, m);
                }
            });
            ui.separator();
            if let Some(r) = &self.esito {
                let n = |k: &str| r[k].as_f64().unwrap_or_default();
                egui::Grid::new("ce_termico_esito").num_columns(2).show(ui, |ui| {
                    for (voce, valore) in [
                        ("Potenza dissipata", format!("{:.1} W", n("potenza"))),
                        ("Superficie efficace", format!("{:.2} m²", n("superficie"))),
                        ("Sovratemperatura", format!("{:.1} K", n("sovratemperatura"))),
                        ("Temperatura interna", format!("{:.1} °C", n("tInterna"))),
                    ] {
                        ui.label(voce);
                        ui.label(egui::RichText::new(valore).strong());
                        ui.end_row();
                    }
                    ui.label("Esito");
                    let (testo, allarme) = if r["condizionatore"] == true {
                        (format!("serve un condizionatore da {:.0} W", n("daSmaltire").ceil()), true)
                    } else if let Some(v) = r["ventilatore"].as_f64() {
                        (format!("serve un ventilatore da {:.0} m³/h", v.ceil()), true)
                    } else {
                        ("la dissipazione naturale basta".to_string(), false)
                    };
                    let colore = if allarme { egui::Color32::from_rgb(220, 140, 40) } else { egui::Color32::from_rgb(60, 170, 90) };
                    ui.colored_label(colore, testo);
                    ui.end_row();
                    if let Some(w) = r["riscaldamento"].as_f64() {
                        ui.label("Anticondensa");
                        ui.label(format!("{:.0} W", w.ceil()));
                        ui.end_row();
                    }
                });
                ui.add_space(4.0);
                ui.label(egui::RichText::new("Potenze dissipate (stimate dove manca il dato del componente: Modifica Simbolo → Potenza dissipata)").small());
                egui::ScrollArea::vertical().max_height(180.0).show(ui, |ui| {
                    egui::Grid::new("ce_termico_voci").striped(true).num_columns(3).show(ui, |ui| {
                        for v in r["voci"].as_array().into_iter().flatten() {
                            ui.label(v["sigla"].as_str().unwrap_or_default());
                            ui.label(format!("{:.1} W{}", v["watt"].as_f64().unwrap_or_default(), if v["stimata"] == true { " (stima)" } else { "" }));
                            ui.label(v["descrizione"].as_str().unwrap_or_default());
                            ui.end_row();
                        }
                    });
                });
            }
        });
        if calcola {
            match cad.run("calcolotermico", self.parametri()) {
                Ok(r) => {
                    self.messaggio = None;
                    self.esito = Some(r);
                }
                Err(e) => self.messaggio = Some((false, e)),
            }
        }
        if disegna {
            let mut p = self.parametri();
            p["tipo"] = json!("termico");
            self.messaggio = Some(match cad.run("uscita", p) {
                Ok(v) => (true, v.get("message").and_then(Value::as_str).unwrap_or("Resoconto disegnato").to_string()),
                Err(e) => (false, e),
            });
        }
        self.aperta = aperta;
    }
}
