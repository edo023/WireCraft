//! Edita Fili e Morsetti (XS): finestra di modifica di un morsetto, del numero di un filo o di
//! un rimando, con i campi del tipo di oggetto scelto.

use cadcraft_ui_egui::CadApp;
use elettrico::edita::{self, Tipo};
use serde_json::{Value, json};

use crate::dialogo::Esito;

/// Un campo della finestra: etichetta, parametro del comando, valore.
struct Campo {
    etichetta: &'static str,
    parametro: &'static str,
    valore: String,
}

pub struct EditaXs {
    handle: String,
    comando: &'static str,
    titolo: String,
    campi: Vec<Campo>,
    /// Numero del filo bloccato (non cambia con la numerazione automatica).
    bloccato: Option<bool>,
}

impl EditaXs {
    /// La finestra per l'oggetto con l'handle dato, se è un morsetto, un filo o un rimando.
    pub fn apri(cad: &mut CadApp, handle: &str) -> Option<Self> {
        let (tipo, b) = edita::riconosci(&mut cad.session, handle).ok()?;
        let campo = |etichetta, parametro, tag: &str| Campo { etichetta, parametro, valore: b.attributo(tag).to_string() };
        let (comando, titolo, campi, bloccato) = match tipo {
            Tipo::Morsetto => (
                "editamorsetto",
                format!("Modifica morsetto – {}:{}", b.attributo("MORSETTIERA"), b.attributo("NUMM")),
                vec![
                    campo("Morsettiera", "morsettiera", "MORSETTIERA"),
                    campo("Numero", "numero", "NUMM"),
                    campo("Quadro (+)", "quadro", "QUADRO"),
                    campo("Livello", "livello", "LIVELLO"),
                    campo("Sezione", "sezione", "SEZIONE"),
                    campo("Costruttore", "costruttore", "COSTRUTTORE"),
                    campo("Codice", "tipo", "TIPO"),
                    campo("Utenza", "utenza", "UTENZA"),
                    campo("Note", "note", "NOTEM"),
                ],
                None,
            ),
            Tipo::NumeroFilo => ("editafilo", format!("Modifica numero filo – {}", b.attributo("N")), vec![campo("Numero del filo", "numero", "N")], Some(b.attributo("BLOCCATO").trim() == "1")),
            Tipo::Rimando => (
                "editarimando",
                format!("Modifica rimando – {}", b.attributo("SEGNALE")),
                vec![campo("Nome del potenziale", "segnale", "SEGNALE"), campo("Descrizione", "descrizione", "#LINEA")],
                None,
            ),
            _ => return None,
        };
        Some(EditaXs { handle: b.handle.clone(), comando, titolo, campi, bloccato })
    }

    fn parametri(&self) -> Value {
        let mut p = json!({"handle": self.handle});
        for c in &self.campi {
            p[c.parametro] = json!(c.valore.trim());
        }
        if let Some(b) = self.bloccato {
            p["bloccato"] = json!(b);
        }
        p
    }

    pub fn mostra(&mut self, cad: &mut CadApp, ctx: &egui::Context) -> Esito {
        let mut aperto = true;
        let mut fatto = false;
        egui::Window::new(self.titolo.clone()).id(egui::Id::new("ce_edita_xs")).open(&mut aperto).collapsible(false).resizable(false).show(ctx, |ui| {
            egui::Grid::new("ce_edita_xs_campi").num_columns(2).spacing([8.0, 6.0]).show(ui, |ui| {
                for c in &mut self.campi {
                    ui.label(c.etichetta);
                    // un numero di filo scritto a mano si blocca
                    if ui.add(egui::TextEdit::singleline(&mut c.valore).desired_width(220.0)).changed()
                        && let Some(b) = &mut self.bloccato
                    {
                        *b = true;
                    }
                    ui.end_row();
                }
                if let Some(b) = &mut self.bloccato {
                    ui.label("");
                    ui.checkbox(b, "Bloccato (la numerazione automatica non lo cambia)");
                    ui.end_row();
                }
            });
            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("OK").clicked() {
                    match cad.run(self.comando, self.parametri()) {
                        Ok(_) => fatto = true,
                        Err(e) => cad.set_status(e),
                    }
                }
                if ui.button("Annulla").clicked() {
                    fatto = true;
                }
            });
        });
        if fatto || !aperto { Esito::Chiuso } else { Esito::Aperto }
    }
}
