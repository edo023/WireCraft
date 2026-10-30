//! Finestra di inserimento e di modifica di un simbolo (sigla, quadro, funzioni, specchietto dei
//! contatti, materiali dall'archivio) e scheda dell'archivio materiali.

use cadcraft_ui_egui::CadApp;
use elettrico::archivio::{self, Articolo};
use elettrico::{fogli, impostazioni, siglatura, simboli, specchietto, squadratura};
use serde_json::{Value, json};

/// Scelte dello specchietto dei contatti: etichetta e parametro `riferimento` (None = quello
/// proposto dal simbolo, "" = nessuno).
fn scelte_riferimento() -> Vec<(String, Option<&'static str>)> {
    let mut v = vec![("Proposto dal simbolo".to_string(), None)];
    v.extend(specchietto::RIFERIMENTI.iter().map(|r| (r.descrizione.to_string(), Some(r.blocco))));
    v.push(("Nessuno".into(), Some("")));
    v
}

pub struct Dialogo {
    /// Il simbolo da inserire (indice nella libreria) oppure il componente da modificare.
    indice: Option<usize>,
    handle: Option<String>,
    madre_sim: bool,
    titolo: String,
    nome: String,
    /// La sigla proposta all'apertura: se resta quella, la sigla si calcola alla posa.
    nome_proposto: String,
    quadro: String,
    funzione1: String,
    funzione2: String,
    /// Designazione CEI EN 81346 e potenza dissipata (W).
    funzione_imp: String,
    ubicazione: String,
    dissipazione: String,
    /// Materiali: costruttore, codice, descrizione.
    materiali: Vec<[String; 3]>,
    cerca: String,
    risultati: Vec<Articolo>,
    nuovo: [String; 3],
    madre: String,
    /// Madri disponibili: sigla e descrizione (posizione, funzione).
    madri: Vec<(String, String)>,
    riferimento: usize,
}

pub enum Esito {
    Aperto,
    Chiuso,
}

fn dividi(t: &str) -> Vec<String> {
    if t.trim().is_empty() { Vec::new() } else { t.split(';').map(|x| x.trim().to_string()).collect() }
}

/// Le madri del progetto, prima quelle col prefisso atteso: (sigla, posizione e funzione).
fn madri(cad: &mut CadApp, prefisso: &str) -> Vec<(String, String)> {
    let mut v: Vec<(bool, String, String)> = siglatura::componenti(&mut cad.session)
        .unwrap_or_default()
        .into_iter()
        .filter(|c| c.madre())
        .map(|c| {
            let pos = squadratura::riferimento(c.foglio, c.blocco.pos.0, c.blocco.pos.1);
            let descr = format!("{pos}  {}  {}", c.blocco.attributo("QUADRO"), c.blocco.attributo("FUNZIONE1"));
            let altro = !siglatura::prefisso_di(&c.nome).is_some_and(|p| p.eq_ignore_ascii_case(prefisso));
            (altro, c.nome, descr)
        })
        .collect();
    v.sort();
    v.into_iter().map(|(_, n, d)| (n, d)).collect()
}

impl Dialogo {
    /// Finestra per inserire il simbolo `indice` della libreria, con la sigla proposta.
    pub fn inserimento(cad: &mut CadApp, indice: usize) -> Option<Self> {
        let sim = simboli::LIBRERIA.get(indice)?;
        let foglio = fogli::corrente(&mut cad.session).ok().flatten().map_or(0, |f| f.numero);
        let componenti = siglatura::componenti(&mut cad.session).unwrap_or_default();
        // la sigla posizionale dipende da dove si posa: si calcola alla posa
        let posizionale = impostazioni::correnti().siglatura == impostazioni::Siglatura::Posizionale;
        let nome = if sim.madre() && !posizionale { siglatura::nuova_sigla(&componenti, sim.prefisso, foglio, (0.0, 0.0)) } else { String::new() };
        Some(Dialogo {
            indice: Some(indice),
            handle: None,
            madre_sim: sim.madre(),
            titolo: format!("Inserimento simbolo – {}", sim.etichetta),
            nome_proposto: nome.clone(),
            nome,
            quadro: impostazioni::quadro_corrente().unwrap_or_default(),
            funzione1: String::new(),
            funzione2: String::new(),
            funzione_imp: impostazioni::correnti().funzione,
            ubicazione: impostazioni::correnti().ubicazione,
            dissipazione: String::new(),
            materiali: Vec::new(),
            cerca: String::new(),
            risultati: archivio::cerca("", true, 50).unwrap_or_default(),
            nuovo: Default::default(),
            madre: String::new(),
            madri: madri(cad, sim.prefisso),
            riferimento: 0,
        })
    }

    /// Finestra per modificare il componente con l'handle dato (Edita Simbolo).
    pub fn modifica(cad: &mut CadApp, handle: &str) -> Option<Self> {
        let c = siglatura::componenti(&mut cad.session).ok()?.into_iter().find(|c| c.blocco.handle.eq_ignore_ascii_case(handle))?;
        let b = &c.blocco;
        let (cost, tipi, descr) = (dividi(b.attributo("COSTRUTTORE")), dividi(b.attributo("TIPO")), dividi(b.attributo("DESCRIZIONE")));
        let materiali = tipi.iter().enumerate().map(|(k, t)| [cost.get(k).cloned().unwrap_or_default(), t.clone(), descr.get(k).cloned().unwrap_or_default()]).collect();
        Some(Dialogo {
            indice: None,
            handle: Some(b.handle.clone()),
            madre_sim: c.madre(),
            titolo: format!("Modifica simbolo – {}", c.nome),
            nome_proposto: String::new(),
            nome: c.nome.clone(),
            quadro: b.attributo("QUADRO").to_string(),
            funzione1: b.attributo("FUNZIONE1").to_string(),
            funzione2: b.attributo("FUNZIONE2").to_string(),
            funzione_imp: b.attributo("FUNZIONE_IMPIANTO").to_string(),
            ubicazione: b.attributo("UBICAZIONE").to_string(),
            dissipazione: b.attributo("DISSIPAZIONE").to_string(),
            materiali,
            cerca: String::new(),
            risultati: archivio::cerca("", true, 50).unwrap_or_default(),
            nuovo: Default::default(),
            madre: if c.madre() { String::new() } else { c.nome.clone() },
            madri: madri(cad, &siglatura::prefisso_di(&c.nome).unwrap_or_default()),
            riferimento: 0,
        })
    }

    fn parametri(&self) -> Value {
        let unisci = |k: usize| self.materiali.iter().map(|m| m[k].trim()).collect::<Vec<_>>().join(";");
        if !self.madre_sim {
            return json!({"madre": self.madre.trim()});
        }
        let mut p = json!({
            "quadro": self.quadro.trim(), "funzione1": self.funzione1.trim(), "funzione2": self.funzione2.trim(),
            "funzioneImpianto": self.funzione_imp.trim(), "ubicazione": self.ubicazione.trim(), "dissipazione": self.dissipazione.trim(),
            "costruttore": unisci(0), "tipo": unisci(1), "descrizione": unisci(2),
        });
        // la sigla proposta (o nessuna) si lascia calcolare alla posa
        let nome = self.nome.trim();
        if !nome.is_empty() && (self.handle.is_some() || nome != self.nome_proposto.trim()) {
            p["nome"] = json!(nome);
        }
        if let Some((_, Some(r))) = scelte_riferimento().get(self.riferimento) {
            p["riferimento"] = json!(r);
        }
        p
    }

    fn conferma(&self, cad: &mut CadApp) -> Result<(), String> {
        let mut p = self.parametri();
        match (&self.handle, self.indice.and_then(|i| simboli::LIBRERIA.get(i))) {
            (Some(h), _) => {
                p["handle"] = json!(h);
                cad.run("editasimbolo", p).map(|_| ())
            }
            (None, Some(sim)) => {
                if !sim.madre() && self.madre.trim().is_empty() {
                    return Err("Indicare la sigla della madre".into());
                }
                simboli::prepara(sim.comando, p);
                cad.start(sim.comando);
                Ok(())
            }
            _ => Ok(()),
        }
    }

    pub fn mostra(&mut self, cad: &mut CadApp, ctx: &egui::Context) -> Esito {
        let mut aperto = true;
        let mut fatto = false;
        let mut errore: Option<String> = None;
        egui::Window::new(self.titolo.clone()).id(egui::Id::new("ce_dialogo_simbolo")).open(&mut aperto).collapsible(false).resizable(true).default_width(520.0).show(ctx, |ui| {
            egui::Grid::new("ce_dialogo_campi").num_columns(2).spacing([8.0, 6.0]).show(ui, |ui| {
                if self.madre_sim {
                    ui.label("Nome (sigla)");
                    ui.add(egui::TextEdit::singleline(&mut self.nome).hint_text("automatica"));
                    ui.end_row();
                    ui.label("Quadro (+)");
                    ui.text_edit_singleline(&mut self.quadro);
                    ui.end_row();
                    ui.label("Funzione (=)");
                    ui.text_edit_singleline(&mut self.funzione_imp);
                    ui.end_row();
                    ui.label("Ubicazione (++)");
                    ui.text_edit_singleline(&mut self.ubicazione);
                    ui.end_row();
                    ui.label("Funzione 1");
                    ui.text_edit_singleline(&mut self.funzione1);
                    ui.end_row();
                    ui.label("Funzione 2");
                    ui.text_edit_singleline(&mut self.funzione2);
                    ui.end_row();
                    ui.label("Potenza dissipata (W)");
                    ui.add(egui::TextEdit::singleline(&mut self.dissipazione).hint_text("stimata se vuota").desired_width(80.0));
                    ui.end_row();
                    if self.handle.is_none() {
                        ui.label("Riferimento contatti");
                        let scelte = scelte_riferimento();
                        egui::ComboBox::from_id_salt("ce_rif").width(300.0).selected_text(scelte.get(self.riferimento).map_or("", |r| r.0.as_str())).show_ui(ui, |ui| {
                            for (k, (t, _)) in scelte.iter().enumerate() {
                                ui.selectable_value(&mut self.riferimento, k, t.as_str());
                            }
                        });
                        ui.end_row();
                    }
                } else {
                    ui.label("Sigla della madre");
                    egui::ComboBox::from_id_salt("ce_madre").width(300.0).selected_text(self.madre.clone()).show_ui(ui, |ui| {
                        for (m, d) in &self.madri {
                            ui.selectable_value(&mut self.madre, m.clone(), format!("{m}    {d}"));
                        }
                    });
                    ui.end_row();
                    ui.label("");
                    ui.text_edit_singleline(&mut self.madre);
                    ui.end_row();
                }
            });
            if self.madre_sim {
                ui.separator();
                ui.label(egui::RichText::new("Materiali").strong());
                let mut togli = None;
                egui::Grid::new("ce_dialogo_mat").striped(true).num_columns(4).show(ui, |ui| {
                    for t in ["Costruttore", "Codice", "Descrizione", ""] {
                        ui.label(egui::RichText::new(t).small());
                    }
                    ui.end_row();
                    for (k, m) in self.materiali.iter().enumerate() {
                        ui.label(&m[0]);
                        ui.label(&m[1]);
                        ui.label(&m[2]);
                        if ui.small_button("✖").on_hover_text("Togli").clicked() {
                            togli = Some(k);
                        }
                        ui.end_row();
                    }
                    ui.add(egui::TextEdit::singleline(&mut self.nuovo[0]).hint_text("Costruttore").desired_width(110.0));
                    ui.add(egui::TextEdit::singleline(&mut self.nuovo[1]).hint_text("Codice").desired_width(110.0));
                    ui.add(egui::TextEdit::singleline(&mut self.nuovo[2]).hint_text("Descrizione").desired_width(170.0));
                    if ui.small_button("➕").on_hover_text("Aggiungi").clicked() && !self.nuovo[1].trim().is_empty() {
                        self.materiali.push(std::mem::take(&mut self.nuovo));
                    }
                    ui.end_row();
                });
                if let Some(k) = togli {
                    self.materiali.remove(k);
                }
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label("Archivio materiali");
                    if ui.add(egui::TextEdit::singleline(&mut self.cerca).hint_text("Cerca (costruttore, codice, descrizione)")).changed() {
                        self.risultati = archivio::cerca(&self.cerca, false, 50).unwrap_or_default();
                    }
                });
                if archivio::quanti() == 0 {
                    ui.label(egui::RichText::new("Archivio vuoto: importa un listino dalla scheda Materiali.").small());
                }
                egui::ScrollArea::vertical().id_salt("ce_dialogo_arch").max_height(150.0).show(ui, |ui| {
                    for a in &self.risultati {
                        let stella = if a.preferito { "★ " } else { "" };
                        if ui.selectable_label(false, format!("{stella}{}  {}  {}", a.costruttore, a.codice, a.descrizione)).on_hover_text("Clic per associare").clicked() {
                            self.materiali.push([a.costruttore.clone(), a.codice.clone(), a.descrizione.clone()]);
                        }
                    }
                });
            }
            ui.separator();
            ui.horizontal(|ui| {
                let ok = if self.handle.is_some() { "OK" } else { "OK – posa sullo schema" };
                if ui.button(ok).clicked() {
                    match self.conferma(cad) {
                        Ok(()) => fatto = true,
                        Err(e) => errore = Some(e),
                    }
                }
                if ui.button("Annulla").clicked() {
                    fatto = true;
                }
            });
        });
        if let Some(e) = errore {
            cad.set_status(e);
        }
        if fatto || !aperto { Esito::Chiuso } else { Esito::Aperto }
    }
}

/// Scheda dell'archivio materiali: ricerca, preferiti, importazione di listini e della libreria
/// di base, nuovi articoli; un articolo si posa nello schema dal suo codice o si associa al
/// componente selezionato.
#[derive(Default)]
pub struct SchedaMateriali {
    cerca: String,
    solo_preferiti: bool,
    risultati: Option<Vec<Articolo>>,
    nuovo: [String; 3],
}

/// La sigla della madre del componente selezionato (o della madre del contatto selezionato).
fn sigla_selezionata(cad: &mut CadApp) -> Option<String> {
    let scelto = cad.session.selection().first()?.hex();
    let c = siglatura::componenti(&mut cad.session).ok()?.into_iter().find(|c| c.blocco.handle.eq_ignore_ascii_case(&scelto))?;
    Some(c.nome)
}

impl SchedaMateriali {
    pub fn mostra(&mut self, cad: &mut CadApp, ui: &mut egui::Ui) {
        let mut ricarica = self.risultati.is_none();
        ui.label(format!("Articoli in archivio: {}", archivio::quanti()));
        ricarica |= ui.add(egui::TextEdit::singleline(&mut self.cerca).hint_text("Cerca (codice, descrizione, costruttore)")).changed();
        ricarica |= ui.checkbox(&mut self.solo_preferiti, "Solo preferiti").changed();
        ui.horizontal_wrapped(|ui| {
            if ui.button("Importa listino (CSV)…").on_hover_text("Colonne riconosciute: Costruttore, Codice, Descrizione, Categoria, Simbolo, Larghezza").clicked()
                && let Some(p) = rfd::FileDialog::new().add_filter("CSV", &["csv", "txt"]).pick_file()
            {
                match cad.run("materiali.importa", json!({"file": p.to_string_lossy()})) {
                    Ok(r) => cad.set_status(r["message"].as_str().unwrap_or_default().to_string()),
                    Err(e) => cad.set_status(e),
                }
                ricarica = true;
            }
            if ui.button("Libreria di base").on_hover_text("Articoli generici (magnetotermici, contattori, relè, alimentatori, pulsanti…) con simbolo e larghezza").clicked() {
                match cad.run("materiali.base", json!({})) {
                    Ok(r) => cad.set_status(r["message"].as_str().unwrap_or_default().to_string()),
                    Err(e) => cad.set_status(e),
                }
                ricarica = true;
            }
        });
        ui.collapsing("Nuovo articolo", |ui| {
            ui.add(egui::TextEdit::singleline(&mut self.nuovo[0]).hint_text("Costruttore"));
            ui.add(egui::TextEdit::singleline(&mut self.nuovo[1]).hint_text("Codice"));
            ui.add(egui::TextEdit::singleline(&mut self.nuovo[2]).hint_text("Descrizione"));
            if ui.button("Aggiungi").clicked() && !self.nuovo[1].trim().is_empty() {
                let [c, k, d] = std::mem::take(&mut self.nuovo);
                let _ = cad.run("materiali.aggiungi", json!({"costruttore": c, "codice": k, "descrizione": d}));
                ricarica = true;
            }
        });
        if ricarica {
            self.risultati = archivio::cerca(&self.cerca, self.solo_preferiti, 300).ok();
        }
        ui.separator();
        let mut cambia = None;
        let mut inserisci = None;
        let mut associa = None;
        for a in self.risultati.iter().flatten() {
            ui.horizontal(|ui| {
                if ui.small_button(if a.preferito { "★" } else { "☆" }).on_hover_text("Preferito").clicked() {
                    cambia = Some((a.costruttore.clone(), a.codice.clone(), !a.preferito));
                }
                if !a.simbolo.is_empty() && ui.small_button("Inserisci").on_hover_text("Posa il simbolo con questo materiale: un punto per componente, Invio per finire").clicked() {
                    inserisci = Some(a.clone());
                }
                if ui.small_button("Associa").on_hover_text("Al componente selezionato nel disegno").clicked() {
                    associa = Some(a.clone());
                }
                let simbolo = simboli::trova(&a.simbolo).map_or("—", |s| s.etichetta);
                let larghezza = if a.larghezza > 0.0 { format!("{} mm", a.larghezza) } else { "—".into() };
                ui.label(format!("{}  {}", a.costruttore, a.codice)).on_hover_text(format!("{}\nSimbolo: {simbolo}\nLarghezza sulla guida: {larghezza}", a.descrizione));
            });
        }
        if let Some((c, k, p)) = cambia {
            if let Err(e) = archivio::segna_preferito(&c, &k, p) {
                cad.set_status(e.to_string());
            }
            self.risultati = archivio::cerca(&self.cerca, self.solo_preferiti, 300).ok();
        }
        if let Some(a) = inserisci {
            cad.start("materiale.inserisci");
            if let Err(e) = cad.session.input(cadcraft_engine::Input::Text(format!("{}|{}", a.costruttore, a.codice))) {
                cad.set_status(e.to_string());
            }
        }
        if let Some(a) = associa {
            match sigla_selezionata(cad) {
                Some(sigla) => match cad.run("materiale", json!({"sigla": sigla, "codice": a.codice, "costruttore": a.costruttore, "descrizione": a.descrizione})) {
                    Ok(r) => cad.set_status(r["message"].as_str().unwrap_or_default().to_string()),
                    Err(e) => cad.set_status(e),
                },
                None => cad.set_status("Seleziona prima il componente nel disegno"),
            }
        }
    }
}
