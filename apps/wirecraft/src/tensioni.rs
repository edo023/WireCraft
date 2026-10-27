//! Finestra "Tensioni": la tabella dei potenziali con tensione e colore del conduttore, e i fili
//! del disegno colorati come il conduttore (o di nuovo col colore del layer).

use cadcraft_ui_egui::CadApp;
use elettrico::tensioni;
use serde_json::{Value, json};

/// Le sigle dei colori IEC 60757 con il loro nome.
const COLORI: [(&str, &str); 14] = [
    ("", "—"),
    ("BK", "nero"),
    ("BN", "marrone"),
    ("RD", "rosso"),
    ("OG", "arancione"),
    ("YE", "giallo"),
    ("GN", "verde"),
    ("BU", "blu"),
    ("VT", "viola"),
    ("GY", "grigio"),
    ("WH", "bianco"),
    ("PK", "rosa"),
    ("TQ", "turchese"),
    ("GNYE", "giallo-verde"),
];

#[derive(Default)]
pub struct FinestraTensioni {
    pub aperta: bool,
    /// (potenziale, tensione, colore) in modifica.
    voci: Vec<[String; 3]>,
    messaggio: Option<(bool, String)>,
}

impl FinestraTensioni {
    pub fn apri(&mut self) {
        self.aperta = true;
        self.voci = tensioni::voci().into_iter().map(|p| [p.nome, p.tensione, p.colore]).collect();
        self.messaggio = None;
    }

    fn esito(&mut self, r: Result<Value, String>) {
        self.messaggio = Some(match r {
            Ok(v) => (true, v.get("message").and_then(Value::as_str).unwrap_or("Tabella salvata").to_string()),
            Err(e) => (false, e),
        });
    }

    pub fn mostra(&mut self, cad: &mut CadApp, ctx: &egui::Context) {
        if !self.aperta {
            return;
        }
        let mut aperta = true;
        let (mut salva, mut colora, mut togli) = (false, false, false);
        egui::Window::new("Tensioni").id(egui::Id::new("ce_tensioni")).open(&mut aperta).default_width(460.0).show(ctx, |ui| {
            ui.label("Il colore del conduttore di ogni potenziale (nome della linea o del rimando). Vale per le reti senza un conduttore specializzato.");
            ui.add_space(4.0);
            let mut via = None;
            egui::ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
                egui::Grid::new("ce_tensioni_tab").striped(true).num_columns(4).show(ui, |ui| {
                    ui.strong("Potenziale");
                    ui.strong("Tensione");
                    ui.strong("Colore");
                    ui.label("");
                    ui.end_row();
                    for (k, v) in self.voci.iter_mut().enumerate() {
                        ui.add(egui::TextEdit::singleline(&mut v[0]).desired_width(80.0));
                        ui.add(egui::TextEdit::singleline(&mut v[1]).desired_width(90.0).hint_text("es. 24 VDC"));
                        let nome = COLORI.iter().find(|(c, _)| *c == v[2]).map_or(v[2].clone(), |(c, n)| if c.is_empty() { (*n).to_string() } else { format!("{c} {n}") });
                        egui::ComboBox::from_id_salt(("ce_tensioni_col", k)).width(130.0).selected_text(nome).show_ui(ui, |ui| {
                            for (c, n) in COLORI {
                                ui.selectable_value(&mut v[2], c.to_string(), if c.is_empty() { n.to_string() } else { format!("{c} {n}") });
                            }
                        });
                        if ui.small_button("🗑").on_hover_text("Togli").clicked() {
                            via = Some(k);
                        }
                        ui.end_row();
                    }
                });
            });
            if let Some(k) = via {
                self.voci.remove(k);
            }
            ui.horizontal(|ui| {
                if ui.button("Aggiungi").clicked() {
                    self.voci.push([String::new(), String::new(), String::new()]);
                }
                salva = ui.button("Salva").clicked();
            });
            ui.separator();
            ui.horizontal(|ui| {
                colora = ui.button("Colora i fili").on_hover_text("I fili del disegno col colore del loro conduttore").clicked();
                togli = ui.button("Togli i colori").on_hover_text("I fili di nuovo col colore del layer").clicked();
            });
            if let Some((ok, m)) = &self.messaggio {
                let colore = if *ok { ui.visuals().text_color() } else { egui::Color32::from_rgb(220, 70, 60) };
                ui.colored_label(colore, m);
            }
        });
        if salva || colora {
            let voci: Vec<Value> = self.voci.iter().filter(|v| !v[0].trim().is_empty()).map(|v| json!({"nome": v[0].trim(), "tensione": v[1].trim(), "colore": v[2]})).collect();
            let r = cad.run("tensioni", json!({"voci": voci}));
            let ok = r.is_ok();
            self.esito(r);
            if ok && colora {
                let r = cad.run("colorafili", json!({}));
                self.esito(r);
            }
        }
        if togli {
            let r = cad.run("colorafili", json!({"togli": true}));
            self.esito(r);
        }
        self.aperta = aperta;
    }
}
