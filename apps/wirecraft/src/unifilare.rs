//! Finestra "Schema Unifilare": i circuiti del quadro con i valori calcolati e la verifica, e il
//! modulo per disegnare un circuito (o l'arrivo linea) nella prossima colonna del foglio corrente
//! o per modificare quello scelto nell'elenco.

use cadcraft_ui_egui::CadApp;
use elettrico::unifilare::{Posa, Protezione, Sistema, Utenza};
use serde_json::{Map, Value, json};

/// I dati del modulo come testi; un numero vuoto lascia il valore proposto dal programma.
#[derive(Clone)]
struct Modulo {
    utenza: String,
    protezione: String,
    sistema: String,
    denominazione: String,
    denominazione2: String,
    circuito: String,
    sigla: String,
    tensione: String,
    potenza: String,
    cosfi: String,
    curva: String,
    nominale: String,
    auto_in: bool,
    icn: String,
    idn: String,
    classe: String,
    cavo: String,
    posa: String,
    sezione: String,
    auto_sezione: bool,
    lunghezza: String,
    codice: String,
    costruttore: String,
    icc: String,
    kc: String,
    cdt_max: String,
}

impl Default for Modulo {
    fn default() -> Self {
        Modulo {
            utenza: "prese".into(),
            protezione: "mtd".into(),
            sistema: String::new(),
            denominazione: String::new(),
            denominazione2: String::new(),
            circuito: String::new(),
            sigla: String::new(),
            tensione: String::new(),
            potenza: String::new(),
            cosfi: String::new(),
            curva: String::new(),
            nominale: String::new(),
            auto_in: true,
            icn: String::new(),
            idn: String::new(),
            classe: String::new(),
            cavo: String::new(),
            posa: "C".into(),
            sezione: String::new(),
            auto_sezione: true,
            lunghezza: String::new(),
            codice: String::new(),
            costruttore: String::new(),
            icc: String::new(),
            kc: String::new(),
            cdt_max: String::new(),
        }
    }
}

/// Un numero del risultato come testo con la virgola.
fn numero(v: &Value) -> String {
    v.as_f64().map(|x| elettrico::unifilare::numero(x, 3)).unwrap_or_default()
}

impl Modulo {
    /// Il modulo con i dati di un circuito (come li dà il comando UNIFILARE).
    fn da_circuito(c: &Value) -> Self {
        let t = |k: &str| c[k].as_str().unwrap_or_default().to_string();
        Modulo {
            utenza: t("utenza"),
            protezione: t("protezione"),
            sistema: t("sistema"),
            denominazione: t("denominazione"),
            denominazione2: t("denominazione2"),
            circuito: t("circuito"),
            sigla: t("sigla"),
            tensione: numero(&c["tensione"]),
            potenza: numero(&c["potenza"]),
            cosfi: numero(&c["cosfi"]),
            curva: t("curva"),
            nominale: numero(&c["in"]),
            auto_in: c["autoIn"].as_bool().unwrap_or(false),
            icn: numero(&c["icn"]),
            idn: numero(&c["idn"]),
            classe: t("classe"),
            cavo: t("cavo"),
            posa: t("posa"),
            sezione: numero(&c["sezione"]),
            auto_sezione: c["autoSezione"].as_bool().unwrap_or(false),
            lunghezza: numero(&c["lunghezza"]),
            codice: t("codice"),
            costruttore: t("costruttore"),
            icc: numero(&c["icc"]),
            kc: numero(&c["kc"]),
            cdt_max: numero(&c["cdtMax"]),
        }
    }

    /// I parametri per UNIFILARE.CIRCUITO, .ARRIVO o .MODIFICA. I testi vanno sempre (vuoti
    /// cancellano), i numeri solo se scritti; taratura e sezione automatiche si ridimensionano.
    fn parametri(&self, quadro: &str, arrivo: bool) -> Value {
        let mut p = Map::new();
        for (k, v) in [
            ("protezione", &self.protezione),
            ("denominazione", &self.denominazione),
            ("denominazione2", &self.denominazione2),
            ("circuito", &self.circuito),
            ("nome", &self.sigla),
            ("curva", &self.curva),
            ("classe", &self.classe),
            ("cavo", &self.cavo),
            ("posa", &self.posa),
            ("codice", &self.codice),
            ("costruttore", &self.costruttore),
        ] {
            p.insert(k.into(), json!(v.trim()));
        }
        // i testi vuoti che hanno un valore proposto non si mandano
        for k in ["protezione", "curva", "classe", "cavo", "posa", "circuito", "nome"] {
            if p.get(k).and_then(Value::as_str).is_some_and(str::is_empty) {
                p.remove(k);
            }
        }
        if !arrivo {
            p.insert("utenza".into(), json!(self.utenza));
        }
        if !self.sistema.trim().is_empty() {
            p.insert("sistema".into(), json!(self.sistema.trim()));
        }
        let mut numeri = vec![("tensione", &self.tensione), ("potenza", &self.potenza), ("cosfi", &self.cosfi), ("icn", &self.icn), ("idn", &self.idn), ("lunghezza", &self.lunghezza)];
        if arrivo {
            numeri.extend([("icc", &self.icc), ("kc", &self.kc), ("cdtMax", &self.cdt_max)]);
        }
        if !self.auto_in {
            numeri.push(("in", &self.nominale));
        }
        if !self.auto_sezione {
            numeri.push(("sezione", &self.sezione));
        }
        for (k, v) in numeri {
            if !v.trim().is_empty() {
                p.insert(k.into(), json!(v.trim()));
            }
        }
        if !quadro.trim().is_empty() {
            p.insert("quadro".into(), json!(quadro.trim()));
        }
        if self.auto_in || self.auto_sezione {
            p.insert("dimensiona".into(), json!(true));
        }
        Value::Object(p)
    }
}

#[derive(Default)]
pub struct FinestraUnifilare {
    pub aperta: bool,
    quadro: String,
    circuiti: Vec<Value>,
    /// Il circuito scelto nell'elenco (handle) e se è l'arrivo.
    scelto: Option<(String, bool)>,
    modulo: Modulo,
    messaggio: Option<(bool, String)>,
}

impl FinestraUnifilare {
    pub fn apri(&mut self, cad: &mut CadApp) {
        self.aperta = true;
        self.messaggio = None;
        self.quadro = elettrico::impostazioni::quadro_corrente().unwrap_or_default();
        self.rileggi(cad);
    }

    fn rileggi(&mut self, cad: &mut CadApp) {
        let r = cad.run("unifilare", json!({"quadro": self.quadro.trim()})).unwrap_or_default();
        self.circuiti = r["circuiti"].as_array().cloned().unwrap_or_default();
        if let Some((h, _)) = &self.scelto
            && !self.circuiti.iter().any(|c| c["handle"].as_str() == Some(h))
        {
            self.scelto = None;
        }
    }

    fn esito(&mut self, cad: &mut CadApp, r: Result<Value, String>) {
        self.messaggio = Some(match &r {
            Ok(v) => (v["ok"].as_bool() != Some(false), v["message"].as_str().unwrap_or("Fatto").to_string()),
            Err(e) => (false, e.clone()),
        });
        // il circuito appena disegnato o modificato resta scelto
        if let Ok(v) = &r
            && let Some(h) = v["handle"].as_str()
        {
            self.scelto = Some((h.to_string(), v["arrivo"].as_bool().unwrap_or(false)));
            self.modulo = Modulo::da_circuito(v);
        }
        self.rileggi(cad);
    }

    fn combo(ui: &mut egui::Ui, id: &str, valore: &mut String, scelte: &[(&str, &str)]) {
        let testo = scelte.iter().find(|s| s.0.eq_ignore_ascii_case(valore)).map_or(valore.as_str(), |s| s.1).to_string();
        egui::ComboBox::from_id_salt(id).width(190.0).selected_text(testo).show_ui(ui, |ui| {
            for (codice, descrizione) in scelte {
                ui.selectable_value(valore, (*codice).to_string(), *descrizione);
            }
        });
    }

    fn campo(ui: &mut egui::Ui, valore: &mut String, suggerimento: &str, largo: f32) {
        ui.add(egui::TextEdit::singleline(valore).hint_text(suggerimento).desired_width(largo));
    }

    fn elenco(&mut self, ui: &mut egui::Ui) -> Option<Value> {
        let mut scelto = None;
        egui::ScrollArea::vertical().id_salt("ce_unif_elenco").max_height(220.0).show(ui, |ui| {
            egui::Grid::new("ce_unif_tab").striped(true).num_columns(12).show(ui, |ui| {
                for t in ["Fg/Col", "Circuito", "Sigla", "Denominazione", "Utenza", "P [kW]", "Ib [A]", "In [A]", "Cavo", "Iz [A]", "c.d.t. %", "Verifica"] {
                    ui.strong(t);
                }
                ui.end_row();
                for c in &self.circuiti {
                    let h = c["handle"].as_str().unwrap_or_default();
                    let attivo = self.scelto.as_ref().is_some_and(|s| s.0 == h);
                    let posto = format!("{}/{}", c["foglio"], c["colonna"]);
                    if ui.selectable_label(attivo, posto).on_hover_text("Modifica questo circuito").clicked() {
                        scelto = Some(c.clone());
                    }
                    let t = |k: &str| c[k].as_str().unwrap_or_default().to_string();
                    ui.label(egui::RichText::new(t("circuito")).strong());
                    ui.label(t("sigla"));
                    ui.label(t("denominazione"));
                    ui.label(if c["arrivo"].as_bool() == Some(true) { "Arrivo linea".to_string() } else { Utenza::da_testo(&t("utenza")).map_or_else(String::new, |u| u.descrizione().to_string()) });
                    ui.label(numero(&c["potenza"]));
                    ui.label(elettrico::unifilare::numero(c["ib"].as_f64().unwrap_or(0.0), 1));
                    ui.label(format!("{}{}", if c["protezione"] == "mt" || c["protezione"] == "mtd" { t("curva") } else { String::new() }, numero(&c["in"])));
                    ui.label(format!("{} {}", t("cavo"), t("formazione")));
                    ui.label(c["iz"].as_f64().map_or_else(|| "-".into(), |x| elettrico::unifilare::numero(x, 1)));
                    ui.label(elettrico::unifilare::numero(c["cdtTotale"].as_f64().unwrap_or(0.0), 2));
                    let v = t("verifica");
                    if c["ok"].as_bool() == Some(true) {
                        ui.colored_label(egui::Color32::from_rgb(70, 160, 80), v);
                    } else {
                        ui.colored_label(egui::Color32::from_rgb(220, 70, 60), v);
                    }
                    ui.end_row();
                }
            });
        });
        scelto
    }

    fn modulo(&mut self, ui: &mut egui::Ui, arrivo: bool) {
        let m = &mut self.modulo;
        let utenze: Vec<(&str, &str)> = Utenza::TUTTE.iter().map(|u| (u.codice(), u.descrizione())).collect();
        let protezioni: Vec<(&str, &str)> = Protezione::TUTTE.iter().map(|p| (p.codice(), p.descrizione())).collect();
        let pose: Vec<(&str, &str)> = Posa::TUTTE.iter().map(|p| (p.codice(), p.descrizione())).collect();
        let mut sistemi: Vec<(&str, &str)> = vec![("", "Proposto per l'utenza")];
        sistemi.extend(Sistema::PROPOSTI.iter().map(|s| (*s, *s)));
        egui::Grid::new("ce_unif_modulo").num_columns(4).spacing([10.0, 6.0]).show(ui, |ui| {
            if !arrivo {
                ui.label("Utenza");
                Self::combo(ui, "ce_unif_utenza", &mut m.utenza, &utenze);
            } else {
                ui.label("Arrivo linea");
                ui.label("protezione generale sopra le sbarre");
            }
            ui.label("Protezione");
            Self::combo(ui, "ce_unif_prot", &mut m.protezione, &protezioni);
            ui.end_row();
            ui.label("Denominazione");
            Self::campo(ui, &mut m.denominazione, "es. Prese ufficio", 190.0);
            ui.label("2ª lingua");
            Self::campo(ui, &mut m.denominazione2, "", 190.0);
            ui.end_row();
            ui.label("Circuito");
            Self::campo(ui, &mut m.circuito, "automatico (C1, C2…)", 190.0);
            ui.label("Sigla");
            Self::campo(ui, &mut m.sigla, "automatica", 190.0);
            ui.end_row();
            ui.label("Sistema");
            Self::combo(ui, "ce_unif_sistema", &mut m.sistema, &sistemi);
            ui.label("Tensione [V]");
            Self::campo(ui, &mut m.tensione, "400 o 230", 190.0);
            ui.end_row();
            ui.label("Potenza [kW]");
            Self::campo(ui, &mut m.potenza, if arrivo { "somma delle partenze" } else { "0" }, 190.0);
            ui.label("Cos fi");
            Self::campo(ui, &mut m.cosfi, "proposto per l'utenza", 190.0);
            ui.end_row();
            ui.label("Curva");
            Self::combo(ui, "ce_unif_curva", &mut m.curva, &[("", "Proposta"), ("B", "B"), ("C", "C"), ("D", "D"), ("K", "K"), ("Z", "Z")]);
            ui.label("In [A]");
            ui.horizontal(|ui| {
                ui.add_enabled(!m.auto_in, egui::TextEdit::singleline(&mut m.nominale).desired_width(80.0));
                ui.checkbox(&mut m.auto_in, "automatica").on_hover_text("La taratura normalizzata subito sopra Ib, rifatta a ogni ricalcolo");
            });
            ui.end_row();
            ui.label("Icn [kA]");
            Self::campo(ui, &mut m.icn, "6", 190.0);
            ui.label("Idn [mA] / tipo");
            ui.horizontal(|ui| {
                ui.add(egui::TextEdit::singleline(&mut m.idn).hint_text("30 o 300").desired_width(80.0));
                egui::ComboBox::from_id_salt("ce_unif_classe").width(70.0).selected_text(if m.classe.is_empty() { "A".to_string() } else { m.classe.clone() }).show_ui(ui, |ui| {
                    for c in ["AC", "A", "F", "B"] {
                        ui.selectable_value(&mut m.classe, c.to_string(), c);
                    }
                });
            });
            ui.end_row();
            ui.label("Cavo");
            Self::campo(ui, &mut m.cavo, "FG16OR16", 190.0);
            ui.label("Posa");
            Self::combo(ui, "ce_unif_posa", &mut m.posa, &pose);
            ui.end_row();
            ui.label("Sezione [mm²]");
            ui.horizontal(|ui| {
                ui.add_enabled(!m.auto_sezione, egui::TextEdit::singleline(&mut m.sezione).desired_width(80.0));
                ui.checkbox(&mut m.auto_sezione, "automatica").on_hover_text("La più piccola con portata e caduta di tensione giuste, rifatta a ogni ricalcolo");
            });
            ui.label("Lunghezza [m]");
            Self::campo(ui, &mut m.lunghezza, "20", 190.0);
            ui.end_row();
            ui.label("Codice articolo");
            Self::campo(ui, &mut m.codice, "", 190.0);
            ui.label("Costruttore");
            Self::campo(ui, &mut m.costruttore, "", 190.0);
            ui.end_row();
            if arrivo {
                ui.label("Icc presunta [kA]");
                Self::campo(ui, &mut m.icc, "6", 190.0);
                ui.label("Contemporaneità");
                Self::campo(ui, &mut m.kc, "1", 190.0);
                ui.end_row();
                ui.label("c.d.t. massima [%]");
                Self::campo(ui, &mut m.cdt_max, "4", 190.0);
                ui.end_row();
            }
        });
    }

    pub fn mostra(&mut self, cad: &mut CadApp, ctx: &egui::Context) {
        if !self.aperta || cad.session.docs.is_empty() {
            return;
        }
        let mut aperta = true;
        let mut azione: Option<(&'static str, Value)> = None;
        let mut rileggi = false;
        egui::Window::new("Schema Unifilare").id(egui::Id::new("ce_unifilare")).open(&mut aperta).default_width(900.0).show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Quadro");
                rileggi |= ui.add(egui::TextEdit::singleline(&mut self.quadro).hint_text("tutti").desired_width(80.0)).lost_focus();
                if ui.button("Ricalcola").on_hover_text("Ricalcola i circuiti e ridisegna sbarre, conduttori e utenze").clicked() {
                    azione = Some(("unifilare.aggiorna", json!({})));
                }
                ui.label("Un circuito per colonna: il nuovo va nella prossima colonna libera del foglio corrente.");
            });
            ui.separator();
            if let Some(c) = self.elenco(ui) {
                self.scelto = c["handle"].as_str().map(|h| (h.to_string(), c["arrivo"].as_bool().unwrap_or(false)));
                self.modulo = Modulo::da_circuito(&c);
            }
            ui.separator();
            let arrivo_scelto = self.scelto.as_ref().is_some_and(|s| s.1);
            self.modulo(ui, arrivo_scelto);
            ui.separator();
            // un circuito nuovo dal modulo di quello scelto prende circuito e sigla nuovi
            let nuovo = |f: &Self, arrivo: bool| {
                let mut p = f.modulo.parametri(&f.quadro, arrivo);
                let scelto = f.scelto.as_ref().and_then(|(h, _)| f.circuiti.iter().find(|c| c["handle"].as_str() == Some(h)));
                if let Some(o) = p.as_object_mut() {
                    o.remove("dimensiona");
                    if arrivo {
                        o.remove("utenza");
                    }
                    for (k, campo) in [("nome", "sigla"), ("circuito", "circuito")] {
                        if scelto.is_some_and(|c| o.get(k).is_some_and(|v| *v == c[campo])) {
                            o.remove(k);
                        }
                    }
                }
                p
            };
            ui.horizontal(|ui| {
                if ui.button("Disegna circuito").on_hover_text("Un circuito nuovo con questi dati").clicked() {
                    azione = Some(("unifilare.circuito", nuovo(self, false)));
                }
                if ui.button("Disegna arrivo linea").on_hover_text("La protezione generale del quadro, nella prima colonna").clicked() {
                    azione = Some(("unifilare.arrivo", nuovo(self, true)));
                }
                if let Some((h, arrivo)) = self.scelto.clone() {
                    if ui.button("Applica al circuito scelto").clicked() {
                        let mut p = self.modulo.parametri("", arrivo);
                        p["handle"] = json!(h);
                        azione = Some(("unifilare.modifica", p));
                    }
                    if ui.button("Nuovo").on_hover_text("Svuota il modulo").clicked() {
                        self.scelto = None;
                        self.modulo = Modulo::default();
                    }
                }
            });
            if let Some((ok, m)) = &self.messaggio {
                let colore = if *ok { ui.visuals().text_color() } else { egui::Color32::from_rgb(220, 70, 60) };
                ui.colored_label(colore, m);
            }
        });
        if let Some((comando, p)) = azione {
            let r = cad.run(comando, p);
            self.esito(cad, r);
        } else if rileggi {
            self.rileggi(cad);
        }
        self.aperta = aperta;
    }
}
