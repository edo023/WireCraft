//! Pannelli Navigatore e Cross Reference.
//!
//! Navigatore: albero dei quadri con le madri e i loro contatti, le morsettiere con i morsetti,
//! le schede PLC con i bit, i rimandi per segnale; un clic porta all'elemento (foglio e zoom),
//! ✎ apre Edita Simbolo. Cross: la verifica del progetto con i problemi trovati, in rosso gli
//! errori, in giallo gli avvisi, in verde ciò che è in ordine; un clic porta all'elemento.

use cadcraft_ui_egui::CadApp;
use elettrico::{fili, morsetti, plc, siglatura, squadratura};
use serde_json::json;

type P = (f64, f64);

/// Un elemento a cui si può andare.
#[derive(Clone, Debug)]
pub struct Posto {
    pub testo: String,
    pub foglio: u32,
    pub pos: P,
    pub handle: String,
}

impl Posto {
    fn rif(&self) -> String {
        squadratura::riferimento(self.foglio, self.pos.0, self.pos.1)
    }
}

/// Una madre con i suoi contatti.
#[derive(Clone, Debug)]
pub struct Madre {
    pub posto: Posto,
    pub funzione: String,
    pub figli: Vec<Posto>,
}

#[derive(Default)]
pub struct Dati {
    pub quadri: Vec<(String, Vec<Madre>)>,
    pub morsettiere: Vec<(String, Vec<Posto>)>,
    pub schede: Vec<(String, Vec<Posto>)>,
    pub segnali: Vec<(String, Vec<Posto>)>,
}

/// Un problema trovato dalla verifica.
#[derive(Clone, Debug)]
pub struct Problema {
    pub grave: bool,
    pub testo: String,
    pub posto: Option<Posto>,
}

pub fn leggi(cad: &mut CadApp) -> Dati {
    let s = &mut cad.session;
    let comp = siglatura::componenti(s).unwrap_or_default();
    let mut quadri: Vec<(String, Vec<Madre>)> = Vec::new();
    let mut madri: Vec<&siglatura::Componente> = comp.iter().filter(|c| c.madre()).collect();
    madri.sort_by(|a, b| a.nome.cmp(&b.nome));
    for m in madri {
        let q = m.blocco.attributo("QUADRO").trim().to_string();
        let figli = comp
            .iter()
            .filter(|f| f.figlio() && f.nome.eq_ignore_ascii_case(&m.nome))
            .map(|f| Posto { testo: format!("{} {}", f.pres, contatti(f)), foglio: f.foglio, pos: f.blocco.pos, handle: f.blocco.handle.clone() })
            .collect();
        let madre = Madre {
            posto: Posto { testo: m.nome.clone(), foglio: m.foglio, pos: m.blocco.pos, handle: m.blocco.handle.clone() },
            funzione: m.blocco.attributo("FUNZIONE1").to_string(),
            figli,
        };
        match quadri.iter_mut().find(|(x, _)| *x == q) {
            Some((_, v)) => v.push(madre),
            None => quadri.push((q, vec![madre])),
        }
    }
    quadri.sort_by(|a, b| a.0.cmp(&b.0));
    let mut morsettiere: Vec<(String, Vec<Posto>)> = Vec::new();
    let mut ms = morsetti::morsetti(s).unwrap_or_default();
    ms.sort_by(|a, b| a.morsettiera.cmp(&b.morsettiera).then_with(|| a.numero.parse::<u64>().ok().cmp(&b.numero.parse::<u64>().ok())));
    for m in ms {
        let chiave = format!("{} {}", m.blocco.attributo("QUADRO"), m.morsettiera).trim().to_string();
        let p = Posto { testo: format!("{}  {}", m.numero, m.blocco.attributo("NUMI")), foglio: m.foglio, pos: m.blocco.pos, handle: m.blocco.handle.clone() };
        match morsettiere.iter_mut().find(|(x, _)| *x == chiave) {
            Some((_, v)) => v.push(p),
            None => morsettiere.push((chiave, vec![p])),
        }
    }
    let mut schede: Vec<(String, Vec<Posto>)> = Vec::new();
    for b in plc::bit(s).unwrap_or_default() {
        let chiave = b.blocco.attributo("NOME").to_string();
        let p = Posto { testo: format!("{}  {}", b.indirizzo(), b.blocco.attributo("COMMENTO1")), foglio: b.foglio, pos: b.blocco.pos, handle: b.blocco.handle.clone() };
        match schede.iter_mut().find(|(x, _)| *x == chiave) {
            Some((_, v)) => v.push(p),
            None => schede.push((chiave, vec![p])),
        }
    }
    let mut segnali: Vec<(String, Vec<Posto>)> = Vec::new();
    let mut rim = fili::rimandi(s).unwrap_or_default();
    rim.sort_by(|a, b| a.segnale.cmp(&b.segnale).then(a.foglio.cmp(&b.foglio)));
    for r in rim {
        let p = Posto { testo: format!("{} → {}", if r.partenza { "partenza" } else { "arrivo" }, r.riferimento), foglio: r.foglio, pos: r.pos, handle: r.handle.clone() };
        match segnali.iter_mut().find(|(x, _)| *x == r.segnale) {
            Some((_, v)) => v.push(p),
            None => segnali.push((r.segnale.clone(), vec![p])),
        }
    }
    Dati { quadri, morsettiere, schede, segnali }
}

/// `13-14`, `1-2 3-4 5-6`: i morsetti di un contatto.
fn contatti(c: &siglatura::Componente) -> String {
    let mut v = Vec::new();
    for k in 1..=4 {
        let (a, b) = (c.blocco.attributo(&format!("PINA{k}")), c.blocco.attributo(&format!("PINB{k}")));
        if a.is_empty() && b.is_empty() {
            break;
        }
        v.push(format!("{a}-{b}"));
    }
    v.join(" ")
}

/// La verifica del progetto (aggiorna anche cross reference e rimandi).
pub fn verifica(cad: &mut CadApp) -> Vec<Problema> {
    let mut v = Vec::new();
    let r = cad.run("crossreference", json!({})).unwrap_or_default();
    let s = &mut cad.session;
    let comp = siglatura::componenti(s).unwrap_or_default();
    let posto = |c: &siglatura::Componente| Posto { testo: c.nome.clone(), foglio: c.foglio, pos: c.blocco.pos, handle: c.blocco.handle.clone() };
    // contatti senza madre
    for f in comp.iter().filter(|f| f.figlio()) {
        if !comp.iter().any(|m| m.madre() && m.nome.eq_ignore_ascii_case(&f.nome)) {
            v.push(Problema { grave: true, testo: format!("Contatto {} senza madre", f.nome), posto: Some(posto(f)) });
        }
    }
    // contatti oltre la disponibilità (dal cross reference)
    for t in r.get("contattiSenzaPosto").and_then(|x| x.as_array()).into_iter().flatten().filter_map(|x| x.as_str()) {
        let p = comp.iter().find(|f| f.figlio() && t.contains(&f.nome)).map(posto);
        v.push(Problema { grave: true, testo: format!("Oltre la disponibilità di contatti: {t}"), posto: p });
    }
    // sigle doppie
    let mut madri: Vec<&siglatura::Componente> = comp.iter().filter(|c| c.madre()).collect();
    madri.sort_by(|a, b| a.nome.cmp(&b.nome));
    for w in madri.windows(2) {
        if let [a, b] = w
            && a.nome.eq_ignore_ascii_case(&b.nome)
            && a.blocco.attributo("QUADRO") == b.blocco.attributo("QUADRO")
        {
            v.push(Problema { grave: true, testo: format!("Sigla doppia {}", a.nome), posto: Some(posto(b)) });
        }
    }
    // rimandi senza corrispondenza
    for r in fili::rimandi(s).unwrap_or_default().into_iter().filter(|r| r.riferimento == fili::SENZA_COPPIA) {
        v.push(Problema {
            grave: false,
            testo: format!("Rimando {} {} senza corrispondenza", if r.partenza { "di partenza" } else { "di arrivo" }, r.segnale),
            posto: Some(Posto { testo: r.segnale.clone(), foglio: r.foglio, pos: r.pos, handle: r.handle.clone() }),
        });
    }
    // madri con specchietto senza nessun contatto (informazione)
    for m in comp.iter().filter(|c| c.madre() && !c.blocco.attributo("CODICE").is_empty()) {
        if !comp.iter().any(|f| f.figlio() && f.nome.eq_ignore_ascii_case(&m.nome)) {
            v.push(Problema { grave: false, testo: format!("{} non ha contatti disegnati", m.nome), posto: Some(posto(m)) });
        }
    }
    // numeri di filo ripetuti
    if let Ok(numeri) = elettrico::numerazione::numeri_usati(s) {
        for u in numeri.into_iter().filter(|u| u.ripetuto) {
            v.push(Problema { grave: true, testo: format!("Numero di filo ripetuto {} ({})", u.numero, u.dove.join(" ")), posto: None });
        }
    }
    v
}

/// Va al foglio dell'elemento e lo inquadra.
pub fn vai(cad: &mut CadApp, p: &Posto) {
    if p.foglio > 0 {
        let _ = cad.run("cambiofoglio", json!({"foglio": p.foglio}));
    }
    let _ = cad.run("zoom.window", json!({"p1": [p.pos.0 - 45.0, p.pos.1 - 30.0], "p2": [p.pos.0 + 45.0, p.pos.1 + 30.0]}));
    if let Some(h) = cadcraft_engine::doc::Handle::parse_hex(&p.handle) {
        cad.session.set_selection(vec![h]);
    }
}

/// Cosa chiede il pannello dopo il disegno: andare a un elemento o editarlo.
pub enum Richiesta {
    Vai(Posto),
    Edita(String),
}

pub fn mostra_navigatore(ui: &mut egui::Ui, d: &Dati, filtro: &mut String) -> Option<Richiesta> {
    let mut r = None;
    ui.add(egui::TextEdit::singleline(filtro).hint_text("Cerca sigla, morsetto, indirizzo, segnale"));
    let f = filtro.to_lowercase();
    let passa = |t: &str| f.is_empty() || t.to_lowercase().contains(&f);
    let riga = |ui: &mut egui::Ui, p: &Posto, extra: &str, r: &mut Option<Richiesta>| {
        let testo = if extra.is_empty() { format!("{}   {}", p.testo, p.rif()) } else { format!("{}   {}   {extra}", p.testo, p.rif()) };
        if ui.selectable_label(false, testo).on_hover_text("Vai all'elemento").clicked() {
            *r = Some(Richiesta::Vai(p.clone()));
        }
    };
    egui::CollapsingHeader::new(format!("Quadri ({})", d.quadri.iter().map(|q| q.1.len()).sum::<usize>())).id_salt("nav_quadri").default_open(true).show(ui, |ui| {
        for (q, madri) in &d.quadri {
            let titolo = if q.is_empty() { "(senza quadro)".to_string() } else { q.clone() };
            egui::CollapsingHeader::new(titolo).id_salt(("nav_q", q)).default_open(!f.is_empty() || d.quadri.len() == 1).show(ui, |ui| {
                for m in madri.iter().filter(|m| passa(&m.posto.testo) || passa(&m.funzione) || m.figli.iter().any(|x| passa(&x.testo))) {
                    ui.horizontal(|ui| {
                        if ui.small_button("✎").on_hover_text("Edita simbolo (XS)").clicked() {
                            r = Some(Richiesta::Edita(m.posto.handle.clone()));
                        }
                        egui::CollapsingHeader::new(format!("{}   {}   {}", m.posto.testo, m.posto.rif(), m.funzione)).id_salt(("nav_m", &m.posto.handle)).show(ui, |ui| {
                            if ui.selectable_label(false, format!("madre   {}", m.posto.rif())).clicked() {
                                r = Some(Richiesta::Vai(m.posto.clone()));
                            }
                            for c in &m.figli {
                                riga(ui, c, "", &mut r);
                            }
                            if m.figli.is_empty() {
                                ui.label(egui::RichText::new("nessun contatto").small().weak());
                            }
                        });
                    });
                }
            });
        }
    });
    for (titolo, gruppi, id) in [("Morsettiere", &d.morsettiere, "nav_mors"), ("PLC", &d.schede, "nav_plc"), ("Rimandi", &d.segnali, "nav_rim")] {
        if gruppi.is_empty() {
            continue;
        }
        egui::CollapsingHeader::new(format!("{titolo} ({})", gruppi.len())).id_salt(id).show(ui, |ui| {
            for (nome, el) in gruppi.iter() {
                if !passa(nome) && !el.iter().any(|x| passa(&x.testo)) {
                    continue;
                }
                egui::CollapsingHeader::new(format!("{nome} ({})", el.len())).id_salt((id, nome)).default_open(!f.is_empty()).show(ui, |ui| {
                    for p in el {
                        riga(ui, p, "", &mut r);
                    }
                });
            }
        });
    }
    r
}

pub fn mostra_cross(ui: &mut egui::Ui, problemi: &Option<Vec<Problema>>) -> (bool, Option<Posto>) {
    let mut esegui = false;
    let mut vai = None;
    ui.horizontal(|ui| {
        esegui = ui.button("Cross Reference").on_hover_text("Aggiorna contatti, specchietti e rimandi e verifica il progetto").clicked();
    });
    let Some(v) = problemi else {
        ui.label("Verifica contatti, specchietti, rimandi, sigle e numeri dei fili di tutto il multifoglio.");
        return (esegui, vai);
    };
    let (rosso, giallo, verde) = (egui::Color32::from_rgb(220, 70, 60), egui::Color32::from_rgb(220, 180, 40), egui::Color32::from_rgb(80, 180, 90));
    let errori = v.iter().filter(|p| p.grave).count();
    let avvisi = v.len() - errori;
    if v.is_empty() {
        ui.colored_label(verde, "● Nessun problema: il progetto è in ordine");
    } else {
        ui.horizontal(|ui| {
            ui.colored_label(if errori > 0 { rosso } else { verde }, format!("● {errori} errori"));
            ui.colored_label(if avvisi > 0 { giallo } else { verde }, format!("● {avvisi} avvisi"));
        });
    }
    ui.separator();
    for p in v {
        let colore = if p.grave { rosso } else { giallo };
        ui.horizontal(|ui| {
            ui.colored_label(colore, "●");
            let testo = match &p.posto {
                Some(x) => format!("{}   {}", p.testo, x.rif()),
                None => p.testo.clone(),
            };
            if ui.selectable_label(false, testo).clicked()
                && let Some(x) = &p.posto
            {
                vai = Some(x.clone());
            }
        });
    }
    (esegui, vai)
}
