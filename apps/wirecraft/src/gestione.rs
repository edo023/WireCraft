//! Finestra "Gestione Fogli": elenco dei fogli con titolo e traduzione modificabili,
//! generazione di nuovi fogli, inserisci/elimina/svuota/rinomina, shift e compatta, numerazione
//! dei fogli e fogli memorizzati. Ogni pulsante esegue un comando (un passo di annullamento).

use cadcraft_ui_egui::CadApp;
use elettrico::{fogli, multifoglio};
use serde_json::{Value, json};

#[derive(Default)]
pub struct GestioneMultifoglio {
    pub aperta: bool,
    /// Righe: numero, titolo, traduzione, scelta; rilette quando cambia il disegno.
    righe: Vec<Riga>,
    letto: Option<(u64, u64)>,
    quanti: String,
    primo: String,
    incremento: String,
    posizioni: String,
    nuovo_numero: String,
    memo_nome: String,
    memo_categoria: String,
    memo_descrizione: String,
    memo_scelto: Option<usize>,
    /// Revisioni: modifica, firma, revisione scelta nell'elenco.
    rev_modifica: String,
    rev_firma: String,
    rev_scelta: Option<(u32, String)>,
    memorizzati: Vec<multifoglio::FoglioMemorizzato>,
    messaggio: Option<(bool, String)>,
}

struct Riga {
    numero: u32,
    titolo: String,
    titolo1: String,
    /// Titoli come sono nel disegno: si scrivono solo quelli cambiati.
    originale: (String, String),
    corrente: bool,
    scelta: bool,
}

impl GestioneMultifoglio {
    pub fn apri(&mut self) {
        self.aperta = true;
        self.letto = None;
        self.memorizzati = multifoglio::memorizzati();
        if self.quanti.is_empty() {
            self.quanti = "10".into();
            self.incremento = "1".into();
            self.posizioni = "1".into();
        }
    }

    fn rileggi(&mut self, cad: &mut CadApp) {
        let chiave = cad.session.state().ok().map(|st| (st.uid, st.revision));
        if chiave == self.letto {
            return;
        }
        let scelti: Vec<u32> = self.righe.iter().filter(|r| r.scelta).map(|r| r.numero).collect();
        self.righe = fogli::elenco(&mut cad.session)
            .unwrap_or_default()
            .into_iter()
            .map(|f| Riga { numero: f.numero, originale: (f.titolo.clone(), f.titolo1.clone()), titolo: f.titolo, titolo1: f.titolo1, corrente: f.corrente, scelta: scelti.contains(&f.numero) })
            .collect();
        let ultimo = self.righe.last().map_or(0, |r| r.numero);
        self.primo = (ultimo + 1).to_string();
        self.letto = cad.session.state().ok().map(|st| (st.uid, st.revision));
    }

    fn scelti(&self) -> Vec<u32> {
        self.righe.iter().filter(|r| r.scelta).map(|r| r.numero).collect()
    }

    fn esegui(&mut self, cad: &mut CadApp, id: &str, p: Value) {
        self.messaggio = Some(match cad.run(id, p) {
            Ok(r) => (true, r.get("message").and_then(Value::as_str).unwrap_or("Fatto").to_string()),
            Err(e) => (false, e.to_string()),
        });
        self.letto = None;
    }

    pub fn mostra(&mut self, cad: &mut CadApp, ctx: &egui::Context) {
        if !self.aperta || cad.session.docs.is_empty() {
            return;
        }
        self.rileggi(cad);
        let mut aperta = true;
        egui::Window::new("Gestione Fogli").id(egui::Id::new("ce_gestione_multifoglio")).open(&mut aperta).default_width(720.0).default_height(560.0).show(ctx, |ui| {
            if self.righe.is_empty() {
                ui.label("Il disegno non è ancora un multifoglio: squadratura e cartiglio sul foglio 0 e i fogli dello schema.");
                if ui.button("Crea multifoglio").clicked() {
                    let n: u32 = self.quanti.trim().parse().unwrap_or(10);
                    self.esegui(cad, "nuovomultifoglio", json!({"fogli": n}));
                }
                return;
            }
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Genera multifoglio:").strong());
                ui.label("Aggiungi");
                ui.add(egui::TextEdit::singleline(&mut self.quanti).desired_width(36.0));
                ui.label("fogli a partire dal N.");
                ui.add(egui::TextEdit::singleline(&mut self.primo).desired_width(36.0));
                ui.label("incremento");
                ui.add(egui::TextEdit::singleline(&mut self.incremento).desired_width(28.0));
                if ui.button("Aggiungi").clicked() {
                    let p = json!({"quanti": self.quanti.trim().parse::<u32>().unwrap_or(1), "da": self.primo.trim().parse::<u32>().unwrap_or(1), "incremento": self.incremento.trim().parse::<u32>().unwrap_or(1)});
                    self.esegui(cad, "aggiungifogli", p);
                }
            });
            ui.separator();
            let mut vai = None;
            egui::ScrollArea::vertical().id_salt("ce_gm_righe").max_height(300.0).auto_shrink([false, true]).show(ui, |ui| {
                egui::Grid::new("ce_gm_griglia").striped(true).num_columns(4).spacing([10.0, 3.0]).show(ui, |ui| {
                    ui.label("");
                    ui.label(egui::RichText::new("Foglio").strong());
                    ui.label(egui::RichText::new("Titolo").strong());
                    ui.label(egui::RichText::new("Traduzione").strong());
                    ui.end_row();
                    for r in &mut self.righe {
                        ui.checkbox(&mut r.scelta, "");
                        let n = egui::RichText::new(format!("{:02}", r.numero));
                        let n = if r.corrente { n.strong().color(ui.visuals().selection.stroke.color) } else { n };
                        if ui.add(egui::Label::new(n).sense(egui::Sense::click())).on_hover_text("Doppio clic: vai al foglio").double_clicked() {
                            vai = Some(r.numero);
                        }
                        ui.add(egui::TextEdit::singleline(&mut r.titolo).desired_width(280.0));
                        ui.add(egui::TextEdit::singleline(&mut r.titolo1).desired_width(220.0));
                        ui.end_row();
                    }
                });
            });
            if let Some(n) = vai {
                self.esegui(cad, "cambiofoglio", json!({"foglio": n}));
            }
            ui.horizontal(|ui| {
                if ui.button("Tutti").clicked() {
                    self.righe.iter_mut().for_each(|r| r.scelta = true);
                }
                if ui.button("Nessuno").clicked() {
                    self.righe.iter_mut().for_each(|r| r.scelta = false);
                }
                let cambiati: Vec<(u32, String, String)> =
                    self.righe.iter().filter(|r| (r.titolo.as_str(), r.titolo1.as_str()) != (r.originale.0.as_str(), r.originale.1.as_str())).map(|r| (r.numero, r.titolo.clone(), r.titolo1.clone())).collect();
                if ui.add_enabled(!cambiati.is_empty(), egui::Button::new(format!("Applica titoli ({})", cambiati.len()))).clicked() {
                    for (n, t, t1) in cambiati {
                        self.esegui(cad, "titolofoglio", json!({"foglio": n, "titolo": t, "titolo1": t1}));
                    }
                }
                if ui.button("Numera fogli").clicked() {
                    self.esegui(cad, "numerafogli", json!({}));
                }
            });
            ui.separator();
            let scelti = self.scelti();
            let uno = (scelti.len() == 1).then(|| scelti[0]);
            ui.horizontal_wrapped(|ui| {
                ui.label(egui::RichText::new(format!("Fogli scelti: {}", scelti.len())).strong());
                if ui.add_enabled(uno.is_some(), egui::Button::new("Inserisci foglio prima")).on_hover_text("Un foglio vuoto al posto di quello scelto; i seguenti vanno avanti di uno").clicked()
                    && let Some(n) = uno
                {
                    self.esegui(cad, "inseriscifoglio", json!({"foglio": n}));
                }
                if ui.add_enabled(!scelti.is_empty(), egui::Button::new("Svuota")).clicked() {
                    self.esegui(cad, "svuotafogli", json!({"fogli": scelti}));
                }
                if ui.add_enabled(!scelti.is_empty(), egui::Button::new("Elimina")).clicked() {
                    self.esegui(cad, "eliminafogli", json!({"fogli": scelti}));
                }
                ui.separator();
                ui.label("Rinomina in");
                ui.add(egui::TextEdit::singleline(&mut self.nuovo_numero).desired_width(36.0));
                if ui.add_enabled(uno.is_some(), egui::Button::new("Rinomina")).clicked()
                    && let (Some(n), Ok(a)) = (uno, self.nuovo_numero.trim().parse::<u32>())
                {
                    self.esegui(cad, "rinominafoglio", json!({"da": n, "a": a}));
                }
            });
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Shift:").strong());
                ui.add(egui::TextEdit::singleline(&mut self.posizioni).desired_width(30.0));
                ui.label("posizioni");
                let k: i64 = self.posizioni.trim().parse().unwrap_or(1).max(1);
                if ui.add_enabled(!scelti.is_empty(), egui::Button::new("◀ Indietro")).clicked() {
                    self.esegui(cad, "shiftfogli", json!({"fogli": scelti, "posizioni": -k}));
                }
                if ui.add_enabled(!scelti.is_empty(), egui::Button::new("Avanti ▶")).clicked() {
                    self.esegui(cad, "shiftfogli", json!({"fogli": scelti, "posizioni": k}));
                }
                if ui.add_enabled(uno.is_some(), egui::Button::new("Avanti dal foglio scelto")).on_hover_text("Sposta il foglio scelto e tutti i seguenti").clicked()
                    && let Some(n) = uno
                {
                    self.esegui(cad, "shiftfogli", json!({"da": n, "posizioni": k}));
                }
                if ui.button("Compatta").on_hover_text("Numeri consecutivi a partire dal più basso").clicked() {
                    self.esegui(cad, "compattafogli", json!({}));
                }
            });
            ui.separator();
            egui::CollapsingHeader::new("Revisioni").id_salt("ce_gm_rev").show(ui, |ui| {
                let elenco = cad.session.execute("revisione.elenco", &json!({})).unwrap_or_default();
                let prossima = elenco.get("prossima").and_then(Value::as_str).unwrap_or("00").to_string();
                egui::ScrollArea::vertical().id_salt("ce_gm_rev_lista").max_height(110.0).show(ui, |ui| {
                    egui::Grid::new("ce_gm_rev_griglia").striped(true).num_columns(5).show(ui, |ui| {
                        for t in ["Foglio", "Rev.", "Modifica", "Data", "Firma"] {
                            ui.label(egui::RichText::new(t).small().strong());
                        }
                        ui.end_row();
                        for r in elenco.get("revisioni").and_then(Value::as_array).into_iter().flatten() {
                            let f = r.get("foglio").and_then(Value::as_u64).unwrap_or(0) as u32;
                            let n = r.get("numero").and_then(Value::as_str).unwrap_or("").to_string();
                            let scelta = self.rev_scelta.as_ref() == Some(&(f, n.clone()));
                            if ui.selectable_label(scelta, format!("{f:02}")).clicked() {
                                self.rev_scelta = Some((f, n.clone()));
                            }
                            ui.label(&n);
                            ui.label(r.get("modifica").and_then(Value::as_str).unwrap_or(""));
                            ui.label(r.get("data").and_then(Value::as_str).unwrap_or(""));
                            ui.label(r.get("firma").and_then(Value::as_str).unwrap_or(""));
                            ui.end_row();
                        }
                    });
                });
                ui.horizontal(|ui| {
                    ui.label(format!("Nuova revisione {prossima}:"));
                    ui.add(egui::TextEdit::singleline(&mut self.rev_modifica).hint_text("Modifica").desired_width(220.0));
                    ui.add(egui::TextEdit::singleline(&mut self.rev_firma).hint_text("Firma").desired_width(60.0));
                    let scelti = self.scelti();
                    let testo = if scelti.is_empty() { "Crea su tutti i fogli".to_string() } else { format!("Crea sui {} fogli scelti", scelti.len()) };
                    if ui.button(testo).on_hover_text("Archivia i fogli come sono e scrive la revisione nel cartiglio").clicked() {
                        let mut p = json!({"modifica": self.rev_modifica.trim(), "firma": self.rev_firma.trim()});
                        if !scelti.is_empty() {
                            p["fogli"] = json!(scelti);
                        }
                        self.esegui(cad, "revisione.crea", p);
                    }
                });
                ui.horizontal(|ui| {
                    let r = self.rev_scelta.clone();
                    if ui.add_enabled(r.is_some(), egui::Button::new("Vedi com'era")).clicked()
                        && let Some((f, n)) = &r
                    {
                        self.esegui(cad, "revisione.vedi", json!({"foglio": f, "numero": n}));
                    }
                    if ui.button("Torna alla versione corrente").clicked() {
                        let f = r.as_ref().map(|x| x.0).or_else(|| self.righe.iter().find(|x| x.corrente).map(|x| x.numero)).unwrap_or(1);
                        self.esegui(cad, "revisione.vedi", json!({"foglio": f}));
                    }
                    if ui.add_enabled(r.is_some(), egui::Button::new("Annulla la revisione")).on_hover_text("I fogli tornano com'erano quando è stata creata").clicked()
                        && let Some((_, n)) = &r
                    {
                        self.esegui(cad, "revisione.annulla", json!({"numero": n}));
                        self.rev_scelta = None;
                    }
                    if ui.button("Legenda revisioni").clicked() {
                        self.esegui(cad, "uscita", json!({"tipo": "revisioni"}));
                    }
                    if ui.button("Reset").on_hover_text("Toglie tutte le revisioni").clicked() {
                        self.esegui(cad, "revisione.reset", json!({}));
                    }
                });
            });
            egui::CollapsingHeader::new("Fogli memorizzati").id_salt("ce_gm_memo").show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.add(egui::TextEdit::singleline(&mut self.memo_nome).hint_text("Nome").desired_width(160.0));
                    ui.add(egui::TextEdit::singleline(&mut self.memo_categoria).hint_text("Categoria (es. Ausiliari)").desired_width(140.0));
                    ui.add(egui::TextEdit::singleline(&mut self.memo_descrizione).hint_text("Descrizione").desired_width(200.0));
                    if ui.button("Memorizza foglio corrente").on_hover_text("Senza selezione: tutto il foglio; con una selezione: solo quella").clicked() {
                        let p = json!({"nome": self.memo_nome.trim(), "categoria": self.memo_categoria.trim(), "descrizione": self.memo_descrizione.trim()});
                        self.esegui(cad, "memorizzafoglio", p);
                        self.memorizzati = multifoglio::memorizzati();
                    }
                });
                egui::ScrollArea::vertical().id_salt("ce_gm_memo_lista").max_height(120.0).show(ui, |ui| {
                    for (k, m) in self.memorizzati.iter().enumerate() {
                        let testo = if m.categoria.is_empty() { format!("{}   {}", m.nome, m.descrizione) } else { format!("{} / {}   {}", m.categoria, m.nome, m.descrizione) };
                        if ui.selectable_label(self.memo_scelto == Some(k), testo).clicked() {
                            self.memo_scelto = Some(k);
                        }
                    }
                    if self.memorizzati.is_empty() {
                        ui.label(egui::RichText::new("Nessun foglio memorizzato").small());
                    }
                });
                let scelto = self.memo_scelto.and_then(|k| self.memorizzati.get(k)).map(|m| m.file.to_string_lossy().to_string());
                if ui.add_enabled(scelto.is_some(), egui::Button::new("Inserisci nel foglio corrente")).clicked()
                    && let Some(f) = scelto
                {
                    self.esegui(cad, "inseriscimemorizzato", json!({"file": f}));
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
