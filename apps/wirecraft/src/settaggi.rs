//! Finestra "Impostazioni": siglatura dei componenti, numerazione dei fili, conduttori in uso,
//! morsetti e nomi delle alimentazioni. Le impostazioni sono quelle della commessa del disegno
//! aperto (o quelle generali) e si salvano con "Applica".

use cadcraft_ui_egui::CadApp;
use elettrico::impostazioni::{self, Impostazioni, NumeriFili, Siglatura};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Sezione {
    #[default]
    Siglatura,
    Fili,
    Conduttori,
    Morsetti,
    Alimentazioni,
}

#[derive(Default)]
pub struct FinestraSettaggi {
    pub aperta: bool,
    sezione: Sezione,
    bozza: Option<Impostazioni>,
    nomi: String,
    messaggio: Option<(bool, String)>,
}

impl FinestraSettaggi {
    pub fn apri(&mut self) {
        let imp = impostazioni::correnti();
        self.nomi = imp.nomi_alimentazioni.join("\n");
        self.bozza = Some(imp);
        self.aperta = true;
        self.messaggio = None;
    }

    pub fn mostra(&mut self, cad: &mut CadApp, ctx: &egui::Context) {
        if !self.aperta {
            return;
        }
        let Some(mut b) = self.bozza.take() else { return };
        let mut aperta = true;
        let mut applica = false;
        egui::Window::new("Impostazioni").id(egui::Id::new("ce_settaggi")).open(&mut aperta).default_width(620.0).default_height(440.0).show(ctx, |ui| {
            let file = impostazioni::file().map_or_else(|| "impostazioni del programma (non salvate)".to_string(), |f| f.display().to_string());
            ui.label(egui::RichText::new(format!("File: {file}")).small());
            ui.separator();
            ui.horizontal_top(|ui| {
                ui.vertical(|ui| {
                    ui.set_width(150.0);
                    for (s, t) in [
                        (Sezione::Siglatura, "Siglatura simboli"),
                        (Sezione::Fili, "Numerazione fili"),
                        (Sezione::Conduttori, "Conduttori in uso"),
                        (Sezione::Morsetti, "Morsetti"),
                        (Sezione::Alimentazioni, "Alimentazioni"),
                    ] {
                        ui.selectable_value(&mut self.sezione, s, t);
                    }
                });
                ui.separator();
                ui.vertical(|ui| match self.sezione {
                    Sezione::Siglatura => {
                        ui.label(egui::RichText::new("Modalità di siglatura").strong());
                        for m in Siglatura::TUTTE {
                            ui.radio_value(&mut b.siglatura, m, m.descrizione());
                        }
                        ui.add_space(6.0);
                        egui::Grid::new("ce_set_sigla").num_columns(2).show(ui, |ui| {
                            ui.label("Prefisso assoluto");
                            ui.add(egui::TextEdit::singleline(&mut b.prefisso_assoluto).desired_width(40.0));
                            ui.end_row();
                            ui.label("Separatore foglio/progressivo");
                            ui.add(egui::TextEdit::singleline(&mut b.separatore_sigla).desired_width(40.0));
                            ui.end_row();
                            ui.label("Cifre del numero di foglio");
                            ui.add(egui::DragValue::new(&mut b.cifre_foglio).range(1..=4));
                            ui.end_row();
                        });
                        ui.checkbox(&mut b.sigle_uguali_quadri, "Sigle uguali ammesse in quadri diversi");
                        let esempio = elettrico::siglatura::formatta(&b, "KA", 5, 3);
                        ui.label(format!("Esempio (foglio 5, terzo KA): {esempio}"));
                        ui.label(egui::RichText::new("Per applicare la nuova siglatura allo schema: Siglatura Componenti.").small());
                    }
                    Sezione::Fili => {
                        ui.label(egui::RichText::new("Numerazione automatica dei fili").strong());
                        for m in NumeriFili::TUTTI {
                            ui.radio_value(&mut b.numeri_fili, m, m.descrizione());
                        }
                        egui::Grid::new("ce_set_fili").num_columns(2).show(ui, |ui| {
                            ui.label("Separatore");
                            ui.add(egui::TextEdit::singleline(&mut b.separatore_fili).desired_width(40.0));
                            ui.end_row();
                            ui.label("Cifre del progressivo");
                            ui.add(egui::DragValue::new(&mut b.cifre_filo).range(1..=6));
                            ui.end_row();
                            ui.label("Prefisso");
                            ui.add(egui::TextEdit::singleline(&mut b.prefisso_fili).desired_width(60.0));
                            ui.end_row();
                            ui.label("Suffisso");
                            ui.add(egui::TextEdit::singleline(&mut b.suffisso_fili).desired_width(60.0));
                            ui.end_row();
                            ui.label("Distanza tra fili paralleli (mm)");
                            ui.add(egui::DragValue::new(&mut b.passo_fili).range(1.0..=50.0).speed(0.5));
                            ui.end_row();
                        });
                    }
                    Sezione::Conduttori => {
                        ui.checkbox(&mut b.applica_conduttore, "Applica il conduttore in uso ai nuovi collegamenti");
                        for (titolo, c) in [("Conduttore dei fili", &mut b.conduttore), ("Conduttore delle alimentazioni", &mut b.conduttore_alimentazione)] {
                            ui.add_space(6.0);
                            ui.label(egui::RichText::new(titolo).strong());
                            egui::Grid::new(("ce_set_cond", titolo)).num_columns(2).show(ui, |ui| {
                                for (t, v) in [("Sezione (es. 1x1.5)", &mut c.sezione), ("Colore (es. BK)", &mut c.colore), ("Codice", &mut c.codice), ("Costruttore", &mut c.costruttore)] {
                                    ui.label(t);
                                    ui.add(egui::TextEdit::singleline(v).desired_width(200.0));
                                    ui.end_row();
                                }
                            });
                        }
                    }
                    Sezione::Morsetti => {
                        ui.checkbox(&mut b.morsettiera_solo_primo, "Nome della morsettiera solo sul primo morsetto di una fila");
                        ui.add_space(6.0);
                        egui::Grid::new("ce_set_correnti").num_columns(2).show(ui, |ui| {
                            ui.label("Quadro corrente (+)");
                            ui.add(egui::TextEdit::singleline(&mut b.quadro).desired_width(100.0));
                            ui.end_row();
                            ui.label("Funzione corrente (=)");
                            ui.add(egui::TextEdit::singleline(&mut b.funzione).desired_width(100.0));
                            ui.end_row();
                            ui.label("Ubicazione corrente (++)");
                            ui.add(egui::TextEdit::singleline(&mut b.ubicazione).desired_width(100.0));
                            ui.end_row();
                        });
                    }
                    Sezione::Alimentazioni => {
                        ui.label("Nomi proposti per le alimentazioni (un gruppo per riga, nomi separati da spazi):");
                        ui.add(egui::TextEdit::multiline(&mut self.nomi).desired_rows(10).desired_width(300.0));
                    }
                });
            });
            ui.separator();
            ui.horizontal(|ui| {
                applica = ui.button("Applica").clicked();
                if ui.button("Predefiniti").on_hover_text("Rimette i valori predefiniti (poi Applica)").clicked() {
                    b = Impostazioni::default();
                    self.nomi = b.nomi_alimentazioni.join("\n");
                }
                if let Some((ok, m)) = &self.messaggio {
                    let colore = if *ok { ui.visuals().text_color() } else { egui::Color32::from_rgb(220, 70, 60) };
                    ui.colored_label(colore, m);
                }
            });
        });
        if applica {
            b.nomi_alimentazioni = self.nomi.lines().map(str::trim).filter(|l| !l.is_empty()).map(str::to_string).collect();
            let p = impostazioni::a_json(&b);
            self.messaggio = Some(match cad.run("impostazioni", p) {
                Ok(_) => (true, "Impostazioni salvate".to_string()),
                Err(e) => (false, e.to_string()),
            });
        }
        self.bozza = Some(b);
        self.aperta = aperta;
    }
}
