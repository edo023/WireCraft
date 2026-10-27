//! Finestra "Stampa e PDF Fogli": quali fogli, colori, PDF dinamico e file, poi il comando
//! `STAMPAFOGLI`. Le scelte restano per la stampa successiva.

use cadcraft_ui_egui::CadApp;
use elettrico::{fogli, stampa};
use serde_json::json;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Scelta {
    #[default]
    Tutti,
    Corrente,
    Intervallo,
}

pub struct FinestraStampa {
    pub aperta: bool,
    scelta: Scelta,
    intervallo: String,
    colori: bool,
    fili_colorati: bool,
    dinamico: bool,
    apri: bool,
    file: String,
    /// Il disegno per cui è stato proposto `file`: cambiando disegno si propone il suo.
    disegno: Option<String>,
    errore: Option<String>,
}

impl Default for FinestraStampa {
    fn default() -> Self {
        let o = stampa::Opzioni::default();
        FinestraStampa {
            aperta: false,
            scelta: Scelta::Tutti,
            intervallo: String::new(),
            colori: o.colori,
            fili_colorati: o.fili_colorati,
            dinamico: o.dinamico,
            apri: true,
            file: String::new(),
            disegno: None,
            errore: None,
        }
    }
}

/// Apre un file con il programma predefinito del sistema.
fn apri_file(percorso: &str) -> std::io::Result<()> {
    let comando = if cfg!(windows) {
        "explorer"
    } else if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    std::process::Command::new(comando).arg(percorso).spawn().map(|_| ())
}

impl FinestraStampa {
    pub fn apri(&mut self, cad: &CadApp) {
        let disegno = cad.session.state().ok().and_then(|st| st.path.clone());
        if disegno != self.disegno {
            self.file = disegno.as_deref().map(|d| stampa::percorso_predefinito(d).to_string_lossy().to_string()).unwrap_or_default();
            self.disegno = disegno;
        }
        self.errore = None;
        self.aperta = true;
    }

    fn parametri(&self) -> serde_json::Value {
        let fogli = match self.scelta {
            Scelta::Tutti => "tutti",
            Scelta::Corrente => "corrente",
            Scelta::Intervallo => self.intervallo.trim(),
        };
        json!({"fogli": fogli, "file": self.file.trim(), "colori": self.colori, "filiColorati": self.fili_colorati, "dinamico": self.dinamico})
    }

    fn sfoglia(&mut self) {
        let attuale = std::path::Path::new(self.file.trim());
        let mut d = rfd::FileDialog::new().add_filter("PDF", &["pdf"]);
        if let Some(nome) = attuale.file_name() {
            d = d.set_file_name(nome.to_string_lossy());
        }
        if let Some(cartella) = attuale.parent().filter(|c| c.is_dir()) {
            d = d.set_directory(cartella);
        }
        if let Some(p) = d.save_file() {
            self.file = p.to_string_lossy().to_string();
        }
    }

    fn stampa(&mut self, cad: &mut CadApp) {
        if self.file.trim().is_empty() {
            self.sfoglia();
            if self.file.trim().is_empty() {
                return;
            }
        }
        match cad.run("stampafogli", self.parametri()) {
            Ok(r) => {
                let file = r.get("file").and_then(serde_json::Value::as_str).unwrap_or_default().to_string();
                if self.apri
                    && let Err(e) = apri_file(&file)
                {
                    cad.set_status(format!("PDF scritto, ma non si apre: {e}"));
                }
                self.aperta = false;
            }
            Err(e) => self.errore = Some(e),
        }
    }

    pub fn mostra(&mut self, cad: &mut CadApp, ctx: &egui::Context, elenco: &[fogli::Foglio]) {
        if !self.aperta {
            return;
        }
        let mut aperta = true;
        let mut vai = false;
        let mut annulla = false;
        egui::Window::new("Stampa e PDF Fogli").id(egui::Id::new("ce_stampa_fogli")).open(&mut aperta).collapsible(false).resizable(false).default_width(460.0).show(ctx, |ui| {
            let (primo, ultimo) = (elenco.first().map_or(0, |f| f.numero), elenco.last().map_or(0, |f| f.numero));
            let corrente = elenco.iter().find(|f| f.corrente).map(|f| f.numero);
            ui.label(egui::RichText::new("Fogli").strong());
            ui.radio_value(&mut self.scelta, Scelta::Tutti, format!("Tutti ({} fogli, {primo}–{ultimo})", elenco.len()));
            ui.radio_value(&mut self.scelta, Scelta::Corrente, corrente.map_or_else(|| "Foglio corrente".to_string(), |n| format!("Foglio corrente ({n})")));
            ui.horizontal(|ui| {
                ui.radio_value(&mut self.scelta, Scelta::Intervallo, "Fogli");
                let r = ui.add(egui::TextEdit::singleline(&mut self.intervallo).hint_text("es. 4-7, 10").desired_width(160.0));
                if r.gained_focus() {
                    self.scelta = Scelta::Intervallo;
                }
            });
            ui.add_space(6.0);
            ui.label(egui::RichText::new("Opzioni").strong());
            ui.add_enabled(!self.fili_colorati, egui::Checkbox::new(&mut self.colori, "A colori (altrimenti bianco e nero)"));
            ui.checkbox(&mut self.fili_colorati, "Fili colorati come il conduttore, il resto in nero").on_hover_text("Prima colora i fili: Tensioni → Colora i fili");
            ui.checkbox(&mut self.dinamico, "PDF dinamico: segnalibri dei fogli e riferimenti cliccabili")
                .on_hover_text("I riferimenti come 05/3B di rimandi, contatti e specchietti portano alla pagina del foglio citato");
            ui.checkbox(&mut self.apri, "Apri il PDF al termine");
            ui.add_space(6.0);
            ui.label(egui::RichText::new("File PDF").strong());
            ui.horizontal(|ui| {
                ui.add(egui::TextEdit::singleline(&mut self.file).hint_text("cartella PDF accanto al disegno").desired_width(340.0));
                if ui.button("Sfoglia…").clicked() {
                    self.sfoglia();
                }
            });
            if let Some(e) = &self.errore {
                ui.add_space(4.0);
                ui.colored_label(egui::Color32::from_rgb(220, 70, 60), e);
            }
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                vai = ui.button("Stampa PDF").clicked();
                annulla = ui.button("Annulla").clicked();
            });
        });
        if vai {
            self.stampa(cad);
        }
        if annulla || !aperta {
            self.aperta = false;
        }
    }
}
