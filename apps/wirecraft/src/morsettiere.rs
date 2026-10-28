//! Finestra "Editor Morsettiere": le morsettiere e i connettori del progetto; i morsetti di
//! quella scelta nell'ordine della morsettiera, con livello, filo, ponte e posizione; ordine a
//! mano (su, giù) o secondo lo schema o i fili, rinumerazione, ponticelli, rinomina, CSV.

use cadcraft_ui_egui::CadApp;
use serde_json::{Value, json};

use crate::navigatore::{self, Posto};

#[derive(Default)]
pub struct FinestraMorsettiere {
    pub aperta: bool,
    morsettiere: Vec<Value>,
    scelta: Option<String>,
    /// I numeri dei morsetti (un gruppo per numero: i livelli stanno insieme) nell'ordine voluto.
    ordine: Vec<String>,
    /// L'ordine è stato cambiato a mano e non ancora applicato.
    spostato: bool,
    selezionati: Vec<String>,
    da: String,
    passo: String,
    nuovo_nome: String,
    unisci: bool,
    messaggio: Option<(bool, String)>,
}

impl FinestraMorsettiere {
    pub fn apri(&mut self, cad: &mut CadApp) {
        self.aperta = true;
        self.messaggio = None;
        self.rileggi(cad);
    }

    fn rileggi(&mut self, cad: &mut CadApp) {
        let r = cad.run("morsettiera.elenco", json!({})).unwrap_or_default();
        self.morsettiere = r["morsettiere"].as_array().cloned().unwrap_or_default();
        let nomi: Vec<String> = self.morsettiere.iter().filter_map(|m| m["morsettiera"].as_str().map(str::to_string)).collect();
        if self.scelta.as_ref().is_none_or(|s| !nomi.iter().any(|n| n.eq_ignore_ascii_case(s))) {
            self.scelta = nomi.first().cloned();
        }
        self.ordine = self.gruppi();
        self.spostato = false;
        self.selezionati.retain(|n| self.ordine.contains(n));
    }

    fn morsetti(&self) -> Vec<Value> {
        self.morsettiere.iter().find(|m| m["morsettiera"].as_str() == self.scelta.as_deref()).and_then(|m| m["morsetti"].as_array().cloned()).unwrap_or_default()
    }

    /// I numeri dei morsetti nell'ordine della morsettiera, una volta sola.
    fn gruppi(&self) -> Vec<String> {
        let mut v: Vec<String> = Vec::new();
        for m in self.morsetti() {
            let n = m["numero"].as_str().unwrap_or_default().to_string();
            if !v.contains(&n) {
                v.push(n);
            }
        }
        v
    }

    fn esito(&mut self, cad: &mut CadApp, r: Result<Value, String>) {
        self.messaggio = Some(match r {
            Ok(v) => (true, v["message"].as_str().unwrap_or("Fatto").to_string()),
            Err(e) => (false, e),
        });
        self.rileggi(cad);
    }

    /// Sposta i gruppi selezionati di un posto verso l'alto (`su`) o verso il basso.
    fn sposta(&mut self, su: bool) {
        let sel = |n: &String| self.selezionati.contains(n);
        let mut v = self.ordine.clone();
        if su {
            for i in 1..v.len() {
                if sel(&v[i]) && !sel(&v[i - 1]) {
                    v.swap(i, i - 1);
                }
            }
        } else {
            for i in (0..v.len().saturating_sub(1)).rev() {
                if sel(&v[i]) && !sel(&v[i + 1]) {
                    v.swap(i, i + 1);
                }
            }
        }
        if v != self.ordine {
            self.ordine = v;
            self.spostato = true;
        }
    }

    /// I parametri comuni della rinumerazione: morsettiera, primo numero, passo.
    fn rinumera(&self, mut p: Value) -> Value {
        p["morsettiera"] = json!(self.scelta);
        if let Ok(n) = self.da.trim().parse::<u64>() {
            p["da"] = json!(n);
        }
        if let Ok(n) = self.passo.trim().parse::<u64>() {
            p["passo"] = json!(n);
        }
        p
    }

    pub fn mostra(&mut self, cad: &mut CadApp, ctx: &egui::Context) {
        if !self.aperta || cad.session.docs.is_empty() {
            return;
        }
        let mut aperta = true;
        let mut azione: Option<(&'static str, Value)> = None;
        let mut vai: Option<Posto> = None;
        let mut csv = false;
        egui::Window::new("Editor Morsettiere").id(egui::Id::new("ce_morsettiere")).open(&mut aperta).default_width(860.0).default_height(520.0).show(ctx, |ui| {
            if self.morsettiere.is_empty() {
                ui.label("Nessuna morsettiera nel progetto: posa i morsetti con Inserisci Morsetti.");
                return;
            }
            ui.horizontal_top(|ui| {
                // a sinistra le morsettiere
                ui.vertical(|ui| {
                    ui.set_width(150.0);
                    ui.strong("Morsettiere");
                    egui::ScrollArea::vertical().id_salt("ce_mors_nomi").max_height(400.0).show(ui, |ui| {
                        let mut cambia = None;
                        for m in &self.morsettiere {
                            let nome = m["morsettiera"].as_str().unwrap_or_default();
                            let n = m["morsetti"].as_array().map_or(0, Vec::len);
                            let tipo = if m["connettore"].as_bool() == Some(true) { " (conn.)" } else { "" };
                            if ui.selectable_label(self.scelta.as_deref() == Some(nome), format!("{nome}  ·  {n}{tipo}")).clicked() {
                                cambia = Some(nome.to_string());
                            }
                        }
                        if let Some(c) = cambia {
                            self.scelta = Some(c);
                            self.ordine = self.gruppi();
                            self.spostato = false;
                            self.selezionati.clear();
                        }
                    });
                });
                ui.separator();
                ui.vertical(|ui| {
                    let morsetti = self.morsetti();
                    egui::ScrollArea::vertical().id_salt("ce_mors_tab").max_height(330.0).show(ui, |ui| {
                        egui::Grid::new("ce_mors_griglia").striped(true).num_columns(10).show(ui, |ui| {
                            for t in ["", "Morsetto", "Liv.", "Filo", "Ponte", "Posizione", "Quadro", "Sezione", "Tipo", ""] {
                                ui.strong(t);
                            }
                            ui.end_row();
                            for (k, numero) in self.ordine.iter().enumerate() {
                                for m in morsetti.iter().filter(|m| m["numero"].as_str() == Some(numero)) {
                                    let mut sel = self.selezionati.contains(numero);
                                    if ui.checkbox(&mut sel, "").changed() {
                                        if sel {
                                            self.selezionati.push(numero.clone());
                                        } else {
                                            self.selezionati.retain(|n| n != numero);
                                        }
                                    }
                                    let t = |c: &str| m[c].as_str().unwrap_or_default().to_string();
                                    // nell'ordine nuovo il numero che prenderà, se cambia
                                    let mut testo = egui::RichText::new(numero).strong();
                                    if self.spostato && numero.parse::<u64>().is_ok() {
                                        let (da, passo) = (self.da.trim().parse::<usize>().unwrap_or(1), self.passo.trim().parse::<usize>().unwrap_or(1).max(1));
                                        let posto = da + passo * (self.ordine[..=k].iter().filter(|n| n.parse::<u64>().is_ok()).count() - 1);
                                        if posto.to_string() != *numero {
                                            testo = egui::RichText::new(format!("{numero} → {posto}")).strong().color(egui::Color32::from_rgb(230, 160, 60));
                                        }
                                    }
                                    ui.label(testo);
                                    ui.label(m["livello"].to_string());
                                    ui.label(t("filo"));
                                    ui.label(t("ponte"));
                                    ui.label(t("posizione"));
                                    ui.label(t("quadro"));
                                    ui.label(t("sezione"));
                                    ui.label(t("tipo"));
                                    if ui.small_button("Vai").on_hover_text("Mostra il morsetto nello schema").clicked() {
                                        vai = Some(Posto {
                                            testo: numero.clone(),
                                            foglio: m["foglio"].as_u64().and_then(|f| u32::try_from(f).ok()).unwrap_or(0),
                                            pos: (m["x"].as_f64().unwrap_or(0.0), m["y"].as_f64().unwrap_or(0.0)),
                                            handle: t("handle"),
                                        });
                                    }
                                    ui.end_row();
                                }
                            }
                        });
                    });
                    ui.separator();
                    let nome = self.scelta.clone().unwrap_or_default();
                    let n_sel = self.selezionati.len();
                    ui.horizontal(|ui| {
                        if ui.add_enabled(n_sel > 0, egui::Button::new("▲ Su")).clicked() {
                            self.sposta(true);
                        }
                        if ui.add_enabled(n_sel > 0, egui::Button::new("▼ Giù")).clicked() {
                            self.sposta(false);
                        }
                        if ui.add_enabled(self.spostato, egui::Button::new("Applica l'ordine")).on_hover_text("Rinumera i morsetti nell'ordine dell'elenco").clicked() {
                            azione = Some(("morsettiera.ordine", self.rinumera(json!({"numeri": self.ordine}))));
                        }
                        if self.spostato && ui.button("Annulla").clicked() {
                            self.ordine = self.gruppi();
                            self.spostato = false;
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label("Rinumera");
                        if ui.button("secondo lo schema").on_hover_text("Foglio per foglio, da sinistra a destra e dall'alto").clicked() {
                            azione = Some(("morsettiera.ordine", self.rinumera(json!({"ordine": "schema"}))));
                        }
                        if ui.button("secondo i fili").on_hover_text("In ordine di numero di filo").clicked() {
                            azione = Some(("morsettiera.ordine", self.rinumera(json!({"ordine": "fili"}))));
                        }
                        if ui.button("compatta").on_hover_text("Stesso ordine, numeri senza buchi").clicked() {
                            azione = Some(("morsettiera.ordine", self.rinumera(json!({"ordine": "attuale"}))));
                        }
                        ui.label("dal numero");
                        ui.add(egui::TextEdit::singleline(&mut self.da).hint_text("1").desired_width(40.0));
                        ui.label("passo");
                        ui.add(egui::TextEdit::singleline(&mut self.passo).hint_text("1").desired_width(30.0));
                    });
                    ui.horizontal(|ui| {
                        if ui.add_enabled(n_sel >= 2, egui::Button::new("Ponticello")).on_hover_text("Collega i morsetti selezionati").clicked() {
                            azione = Some(("ponticello", json!({"morsettiera": nome, "morsetti": self.selezionati})));
                        }
                        if ui.add_enabled(n_sel >= 1, egui::Button::new("Togli ponticello")).clicked() {
                            azione = Some(("ponticello", json!({"morsettiera": nome, "morsetti": self.selezionati, "togli": true})));
                        }
                        if ui.button("Ponticelli automatici").on_hover_text("Tra morsetti vicini sullo stesso filo").clicked() {
                            azione = Some(("ponticelli.auto", json!({"morsettiera": nome})));
                        }
                        ui.separator();
                        ui.add(egui::TextEdit::singleline(&mut self.nuovo_nome).hint_text("Nuovo nome").desired_width(80.0));
                        ui.checkbox(&mut self.unisci, "unisci").on_hover_text("Se la morsettiera esiste già, i morsetti si aggiungono in fondo");
                        if ui.add_enabled(!self.nuovo_nome.trim().is_empty(), egui::Button::new("Rinomina")).clicked() {
                            azione = Some(("morsettiera.rinomina", json!({"morsettiera": nome, "nome": self.nuovo_nome.trim(), "unisci": self.unisci})));
                            self.scelta = Some(self.nuovo_nome.trim().to_string());
                            self.nuovo_nome.clear();
                        }
                        ui.separator();
                        csv = ui.button("CSV…").clicked();
                    });
                });
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
        if let Some(p) = vai {
            navigatore::vai(cad, &p);
        }
        if csv
            && let Some(nome) = self.scelta.clone()
            && let Some(f) = rfd::FileDialog::new().add_filter("CSV", &["csv"]).set_file_name(format!("morsettiera {nome}.csv")).save_file()
        {
            let r = cad.run("morsettiera", json!({"morsettiera": nome, "file": f.to_string_lossy()}));
            self.esito(cad, r);
        }
        self.aperta = aperta;
    }
}
