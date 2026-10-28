//! Finestra "Liste e Tabelle": disegna sui fogli lista fogli, distinta materiali, morsettiere,
//! tabella di cablaggio e legende (comando `USCITA`), le aggiorna tutte (`AGGIORNALISTE`) o le
//! esporta in CSV.

use cadcraft_ui_egui::CadApp;
use elettrico::uscite::TIPI;
use serde_json::{Value, json};

pub struct FinestraListe {
    pub aperta: bool,
    tipo: usize,
    quadro: String,
    morsettiera: String,
    foglio: String,
    messaggio: Option<(bool, String)>,
}

impl Default for FinestraListe {
    fn default() -> Self {
        FinestraListe { aperta: false, tipo: 1, quadro: String::new(), morsettiera: String::new(), foglio: String::new(), messaggio: None }
    }
}

impl FinestraListe {
    /// Apre la finestra sul tipo di uscita dato (`indice`, `distinta`…).
    pub fn apri(&mut self, tipo: Option<&str>) {
        if let Some(k) = tipo.and_then(|t| TIPI.iter().position(|x| x.0 == t)) {
            self.tipo = k;
        }
        self.aperta = true;
        self.messaggio = None;
    }

    fn esito(&mut self, r: Result<Value, String>) {
        self.messaggio = Some(match r {
            Ok(v) => (true, v.get("message").and_then(Value::as_str).unwrap_or("Fatto").to_string()),
            Err(e) => (false, e),
        });
    }

    pub fn mostra(&mut self, cad: &mut CadApp, ctx: &egui::Context) {
        if !self.aperta || cad.session.docs.is_empty() {
            return;
        }
        let mut aperta = true;
        egui::Window::new("Liste e Tabelle").id(egui::Id::new("ce_liste")).open(&mut aperta).default_width(460.0).show(ctx, |ui| {
            ui.label(egui::RichText::new("Uscita da disegnare sui fogli").strong());
            for (k, (_, nome)) in TIPI.iter().enumerate() {
                ui.radio_value(&mut self.tipo, k, *nome);
            }
            ui.separator();
            egui::Grid::new("ce_liste_par").num_columns(2).show(ui, |ui| {
                ui.label("Quadro (vuoto = tutti)");
                ui.add(egui::TextEdit::singleline(&mut self.quadro).hint_text("+QG").desired_width(100.0));
                ui.end_row();
                if TIPI.get(self.tipo).is_some_and(|t| t.0 == "morsettiere") {
                    ui.label("Morsettiera (vuoto = tutte)");
                    ui.add(egui::TextEdit::singleline(&mut self.morsettiera).hint_text("-XG").desired_width(100.0));
                    ui.end_row();
                }
                ui.label("Primo foglio (vuoto = dopo l'ultimo)");
                ui.add(egui::TextEdit::singleline(&mut self.foglio).desired_width(60.0));
                ui.end_row();
            });
            ui.horizontal(|ui| {
                if ui.button("Disegna sui fogli").clicked() {
                    let tipo = TIPI.get(self.tipo).map_or("distinta", |t| t.0);
                    let mut p = json!({"tipo": tipo, "quadro": self.quadro.trim(), "morsettiera": self.morsettiera.trim()});
                    if let Ok(n) = self.foglio.trim().parse::<u32>() {
                        p["foglio"] = json!(n);
                    }
                    let r = cad.run("uscita", p).map_err(|e| e.to_string());
                    self.esito(r);
                }
                if ui.button("Excel…").on_hover_text("La lista in un file Excel (.xlsx)").clicked() {
                    let (tipo, nome) = TIPI.get(self.tipo).copied().unwrap_or(("distinta", "Distinta materiali"));
                    if let Some(f) = rfd::FileDialog::new().add_filter("Excel", &["xlsx"]).set_file_name(format!("{}.xlsx", nome.to_lowercase())).save_file() {
                        let r = cad.run("excel", json!({"tipo": tipo, "file": f.to_string_lossy(), "quadro": self.quadro.trim(), "morsettiera": self.morsettiera.trim()})).map_err(|e| e.to_string());
                        self.esito(r);
                    }
                }
                if ui.button("Aggiorna Liste e Tabelle").on_hover_text("Ridisegna tutte le liste già sui fogli con i dati attuali").clicked() {
                    let r = cad.run("aggiornaliste", json!({})).map_err(|e| e.to_string());
                    self.esito(r);
                }
            });
            ui.separator();
            ui.label(egui::RichText::new("Esporta in CSV").strong());
            ui.horizontal(|ui| {
                for (testo, comando, file, extra) in [
                    ("Distinta per sigla", "distintamateriali", "distinta materiali.csv", json!({"ordine": "sigla"})),
                    ("Distinta per tipo", "distintamateriali", "distinta materiali per tipo.csv", json!({"ordine": "tipo"})),
                    ("Tabella di cablaggio", "tabellacablaggio", "tabella di cablaggio.csv", json!({})),
                ] {
                    if ui.button(testo).clicked()
                        && let Some(f) = rfd::FileDialog::new().add_filter("CSV", &["csv"]).set_file_name(file).save_file()
                    {
                        let mut p = extra;
                        p["file"] = json!(f.to_string_lossy());
                        let r = cad.run(comando, p).map_err(|e| e.to_string());
                        self.esito(r);
                    }
                }
            });
            if let Some((ok, m)) = &self.messaggio {
                ui.separator();
                let colore = if *ok { ui.visuals().text_color() } else { egui::Color32::from_rgb(220, 70, 60) };
                ui.colored_label(colore, m);
            }
        });
        self.aperta = aperta;
    }
}
