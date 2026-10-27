//! Finestre "Gestione Cavi" (cavi del progetto, nuovo cavo, posa delle vene sullo schema) e
//! "Gestione PLC" (bit di ingresso e uscita con indirizzi e commenti, nuova scheda, numerazione
//! dei bit, elenco I/O in CSV).

use cadcraft_ui_egui::CadApp;
use elettrico::cavi;
use serde_json::{Value, json};

fn esito(cad: &mut CadApp, id: &str, p: Value) -> (bool, String) {
    match cad.run(id, p) {
        Ok(v) => (true, v.get("message").and_then(Value::as_str).unwrap_or("Fatto").to_string()),
        Err(e) => (false, e),
    }
}

fn messaggio(ui: &mut egui::Ui, m: &Option<(bool, String)>) {
    if let Some((ok, t)) = m {
        ui.separator();
        let colore = if *ok { ui.visuals().text_color() } else { egui::Color32::from_rgb(220, 70, 60) };
        ui.colored_label(colore, t);
    }
}

#[derive(Default)]
pub struct FinestraCavi {
    pub aperta: bool,
    letto: Option<(u64, u64)>,
    elenco: Vec<Value>,
    scelto: Option<String>,
    nuovo: [String; 7],
    messaggio: Option<(bool, String)>,
}

impl FinestraCavi {
    pub fn apri(&mut self) {
        self.aperta = true;
        self.letto = None;
        if self.nuovo[1].is_empty() {
            self.nuovo[1] = "4G1,5".into();
        }
    }

    pub fn mostra(&mut self, cad: &mut CadApp, ctx: &egui::Context) {
        if !self.aperta || cad.session.docs.is_empty() {
            return;
        }
        let chiave = cad.session.state().ok().map(|st| (st.uid, st.revision));
        if chiave != self.letto {
            self.elenco = cad.session.execute("cavi", &json!({})).ok().and_then(|r| r.get("cavi").and_then(Value::as_array).cloned()).unwrap_or_default();
            self.letto = chiave;
        }
        let mut aperta = true;
        egui::Window::new("Gestione Cavi").id(egui::Id::new("ce_cavi")).open(&mut aperta).default_width(720.0).show(ctx, |ui| {
            egui::ScrollArea::vertical().id_salt("ce_cavi_lista").max_height(240.0).show(ui, |ui| {
                egui::Grid::new("ce_cavi_griglia").striped(true).num_columns(7).show(ui, |ui| {
                    for t in ["Cavo", "Formazione", "Tipo", "Funzione", "Da", "A", "Vene"] {
                        ui.label(egui::RichText::new(t).small().strong());
                    }
                    ui.end_row();
                    for c in &self.elenco {
                        let s = |k: &str| c.get(k).and_then(Value::as_str).unwrap_or("").to_string();
                        let sigla = s("sigla");
                        if ui.selectable_label(self.scelto.as_deref() == Some(sigla.as_str()), &sigla).clicked() {
                            self.scelto = Some(sigla.clone());
                        }
                        ui.label(s("formazione"));
                        ui.label(s("codice"));
                        ui.label(s("funzione"));
                        ui.label(s("daQuadro"));
                        ui.label(s("aQuadro"));
                        let usate = c.get("usate").and_then(Value::as_array).map_or(0, Vec::len);
                        let tot = c.get("vene").and_then(Value::as_array).map_or(0, Vec::len);
                        ui.label(format!("{usate}/{tot}"));
                        ui.end_row();
                    }
                });
                if self.elenco.is_empty() {
                    ui.label("Nessun cavo definito.");
                }
            });
            ui.horizontal(|ui| {
                let c = self.scelto.clone();
                if ui.add_enabled(c.is_some(), egui::Button::new("Posa sui fili…")).on_hover_text("Un punto sul filo o una fence su più fili: una vena per filo").clicked()
                    && let Some(c) = &c
                {
                    cavi::usa(c);
                    cad.start("cavo.posa");
                }
                if ui.add_enabled(c.is_some(), egui::Button::new("Elimina")).clicked()
                    && let Some(c) = &c
                {
                    self.messaggio = Some(esito(cad, "cavo.elimina", json!({"sigla": c})));
                    self.scelto = None;
                }
                if ui.button("Togli dai fili…").clicked() {
                    cad.start("cavo.togli");
                }
                if ui.button("Lista cavi sui fogli").clicked() {
                    self.messaggio = Some(esito(cad, "uscita", json!({"tipo": "cavi"})));
                }
                if ui.button("Conduttori sui fogli").clicked() {
                    self.messaggio = Some(esito(cad, "uscita", json!({"tipo": "conduttoricavi"})));
                }
                if ui.button("Conduttori (CSV)…").clicked()
                    && let Some(f) = rfd::FileDialog::new().add_filter("CSV", &["csv"]).set_file_name("conduttori cavi.csv").save_file()
                {
                    self.messaggio = Some(esito(cad, "cavo.conduttori", json!({"file": f.to_string_lossy()})));
                }
            });
            ui.separator();
            ui.label(egui::RichText::new("Nuovo cavo").strong());
            egui::Grid::new("ce_cavi_nuovo").num_columns(4).show(ui, |ui| {
                let campi = ["Sigla (vuoto = automatica)", "Formazione (4G1,5 · 3x2.5)", "Tipo / codice", "Funzione", "Da quadro", "A quadro", "Lunghezza (m)"];
                for (k, t) in campi.iter().enumerate() {
                    ui.label(*t);
                    ui.add(egui::TextEdit::singleline(&mut self.nuovo[k]).desired_width(150.0));
                    if k % 2 == 1 {
                        ui.end_row();
                    }
                }
                ui.end_row();
            });
            if ui.button("Definisci cavo").clicked() {
                let n = &self.nuovo;
                let p = json!({"sigla": n[0].trim(), "formazione": n[1].trim(), "codice": n[2].trim(), "funzione": n[3].trim(), "daQuadro": n[4].trim(), "aQuadro": n[5].trim(), "lunghezza": n[6].trim()});
                let r = esito(cad, "cavo.definisci", p);
                if r.0 {
                    self.nuovo[0].clear();
                }
                self.messaggio = Some(r);
            }
            messaggio(ui, &self.messaggio);
        });
        self.aperta = aperta;
    }
}

#[derive(Default)]
pub struct FinestraPlc {
    pub aperta: bool,
    letto: Option<(u64, u64)>,
    /// Righe dell'elenco I/O con i commenti modificabili.
    righe: Vec<(Vec<String>, String, String)>,
    tipo_uscite: bool,
    bit16: bool,
    indirizzo: String,
    sigla: String,
    costruttore: String,
    codice: String,
    descrizione: String,
    numera_sigla: String,
    numera_da: String,
    filtro: String,
    messaggio: Option<(bool, String)>,
}

impl FinestraPlc {
    pub fn apri(&mut self) {
        self.aperta = true;
        self.letto = None;
        if self.indirizzo.is_empty() {
            self.indirizzo = "E0.0".into();
        }
    }

    pub fn mostra(&mut self, cad: &mut CadApp, ctx: &egui::Context) {
        if !self.aperta || cad.session.docs.is_empty() {
            return;
        }
        let chiave = cad.session.state().ok().map(|st| (st.uid, st.revision));
        if chiave != self.letto {
            let r = cad.session.execute("plc.elenco", &json!({})).ok();
            self.righe = r
                .and_then(|r| r.get("righe").and_then(Value::as_array).cloned())
                .unwrap_or_default()
                .into_iter()
                .map(|r| {
                    let v: Vec<String> = r.as_array().into_iter().flatten().map(|x| x.as_str().unwrap_or("").to_string()).collect();
                    let (c1, c3) = (v.get(2).cloned().unwrap_or_default(), v.get(3).cloned().unwrap_or_default());
                    (v, c1, c3)
                })
                .collect();
            self.letto = chiave;
        }
        let mut aperta = true;
        egui::Window::new("Gestione PLC").id(egui::Id::new("ce_plc")).open(&mut aperta).default_width(820.0).show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Bit I/O").strong());
                ui.add(egui::TextEdit::singleline(&mut self.filtro).hint_text("Cerca indirizzo o commento").desired_width(220.0));
            });
            let filtro = self.filtro.to_lowercase();
            let mut cambi: Vec<(String, String, String)> = Vec::new();
            egui::ScrollArea::vertical().id_salt("ce_plc_lista").max_height(300.0).show(ui, |ui| {
                egui::Grid::new("ce_plc_griglia").striped(true).num_columns(7).show(ui, |ui| {
                    for t in ["Indirizzo", "I/O", "Commento", "Comment", "Scheda", "Morsetto", "Posizione"] {
                        ui.label(egui::RichText::new(t).small().strong());
                    }
                    ui.end_row();
                    for (v, c1, c3) in &mut self.righe {
                        if !filtro.is_empty() && !v.join(" ").to_lowercase().contains(&filtro) {
                            continue;
                        }
                        let g = |k: usize| v.get(k).cloned().unwrap_or_default();
                        ui.label(egui::RichText::new(g(0)).strong());
                        ui.label(g(1));
                        let a = ui.add(egui::TextEdit::singleline(c1).desired_width(200.0)).lost_focus();
                        let b = ui.add(egui::TextEdit::singleline(c3).desired_width(200.0)).lost_focus();
                        if (a || b) && (*c1 != g(2) || *c3 != g(3)) {
                            cambi.push((g(0), c1.clone(), c3.clone()));
                        }
                        ui.label(g(4));
                        ui.label(g(5));
                        ui.label(g(7));
                        ui.end_row();
                    }
                });
                if self.righe.is_empty() {
                    ui.label("Nessun bit PLC nel disegno.");
                }
            });
            for (ind, c1, c3) in cambi {
                self.messaggio = Some(esito(cad, "plc.commento", json!({"indirizzo": ind, "commento": c1, "commento2": c3})));
            }
            ui.separator();
            ui.label(egui::RichText::new("Nuova scheda").strong());
            ui.horizontal(|ui| {
                ui.radio_value(&mut self.tipo_uscite, false, "Ingressi");
                ui.radio_value(&mut self.tipo_uscite, true, "Uscite");
                ui.separator();
                ui.radio_value(&mut self.bit16, false, "8 bit");
                ui.radio_value(&mut self.bit16, true, "16 bit");
                ui.label("Primo indirizzo");
                ui.add(egui::TextEdit::singleline(&mut self.indirizzo).desired_width(60.0));
                ui.label("Sigla");
                ui.add(egui::TextEdit::singleline(&mut self.sigla).hint_text("automatica").desired_width(70.0));
            });
            ui.horizontal(|ui| {
                ui.add(egui::TextEdit::singleline(&mut self.costruttore).hint_text("Costruttore").desired_width(120.0));
                ui.add(egui::TextEdit::singleline(&mut self.codice).hint_text("Codice").desired_width(160.0));
                ui.add(egui::TextEdit::singleline(&mut self.descrizione).hint_text("Descrizione").desired_width(200.0));
                if ui.button("Posa sullo schema…").clicked() {
                    elettrico::plc::prepara(json!({
                        "tipo": if self.tipo_uscite { "uscite" } else { "ingressi" }, "bit": if self.bit16 { 16 } else { 8 }, "indirizzo": self.indirizzo.trim(),
                        "sigla": self.sigla.trim(), "costruttore": self.costruttore.trim(), "codice": self.codice.trim(), "descrizione": self.descrizione.trim(),
                    }));
                    cad.start("plc.scheda");
                }
            });
            ui.separator();
            ui.horizontal(|ui| {
                ui.label("Numera I/O della scheda");
                ui.add(egui::TextEdit::singleline(&mut self.numera_sigla).hint_text("-A02.1").desired_width(70.0));
                ui.label("dal");
                ui.add(egui::TextEdit::singleline(&mut self.numera_da).hint_text("E2.0").desired_width(60.0));
                if ui.button("Numera").clicked() {
                    self.messaggio = Some(esito(cad, "plc.numerabit", json!({"sigla": self.numera_sigla.trim(), "indirizzo": self.numera_da.trim()})));
                }
            });
            ui.horizontal(|ui| {
                if ui.button("Lista I/O sui fogli").clicked() {
                    self.messaggio = Some(esito(cad, "uscita", json!({"tipo": "io"})));
                }
                if ui.button("Esporta elenco I/O (CSV)…").clicked()
                    && let Some(f) = rfd::FileDialog::new().add_filter("CSV", &["csv"]).set_file_name("elenco IO.csv").save_file()
                {
                    self.messaggio = Some(esito(cad, "plc.elenco", json!({"file": f.to_string_lossy()})));
                }
                if ui.button("Importa commenti (CSV)…").clicked()
                    && let Some(f) = rfd::FileDialog::new().add_filter("CSV", &["csv", "txt"]).pick_file()
                {
                    self.messaggio = Some(esito(cad, "plc.importa", json!({"file": f.to_string_lossy()})));
                }
            });
            messaggio(ui, &self.messaggio);
        });
        self.aperta = aperta;
    }
}
