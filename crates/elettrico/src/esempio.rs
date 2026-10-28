//! Il progetto d'esempio: un nastro trasportatore con alimentazione, motore con avviamento
//! diretto, ausiliari, ingressi PLC e il quadro di distribuzione del reparto in unifilare, più
//! lay-out del quadro, calcolo termico, pulsantiera e tutte le liste. Lo creano il comando
//! `COMMESSA.ESEMPIO` (Gestione Progetti) e l'esempio `esempio_completo`.

use cadcraft_engine::{Result, Session};
use serde_json::{Value, json};

use crate::util::errore;

/// Disegna il progetto d'esempio nel disegno corrente (vuoto). `testata` sono i dati del
/// cartiglio come per `NUOVOMULTIFOGLIO` (commessa, multifoglio, cliente, descrizioni,
/// disegnatore, data).
pub fn disegna(s: &mut Session, testata: &Value) -> Result<()> {
    let mut e = |id: &str, p: Value| s.execute(id, &p).map_err(|e| errore(format!("{id}: {e}")));
    let mut testata = testata.clone();
    testata["fogli"] = json!(5);
    e("nuovomultifoglio", testata)?;
    for (n, t, t1) in [(1, "Alimentazione generale", "Main power supply"), (2, "Motore nastro", "Belt motor"), (3, "Ausiliari", "Auxiliary circuits"), (4, "Ingressi PLC", "PLC inputs"), (5, "Quadro di distribuzione reparto", "Distribution board")] {
        e("titolofoglio", json!({"foglio": n, "titolo": t, "titolo1": t1}))?;
    }
    e("impostazioni", json!({"quadro": "+QG"}))?;

    // foglio 1: alimentazione trifase con PE, interruttore generale, trasformatore ausiliari
    e("cambiofoglio", json!({"foglio": 1}))?;
    e("descrizionecolonna", json!({"colonna": 0, "testo": "ALIMENTAZIONE|400 VAC 50 HZ", "testo2": "POWER SUPPLY"}))?;
    e("descrizionecolonna", json!({"colonna": 2, "testo": "INTERRUTTORE GENERALE", "testo2": "MAIN SWITCH"}))?;
    e("descrizionecolonna", json!({"colonna": 6, "colonne": 2, "testo": "ALIMENTAZIONE AUSILIARI 230 VAC", "testo2": "AUXILIARY SUPPLY"}))?;
    // dal sezionatore ognuna delle tre fasi scende sulla sua linea
    e("filo", json!({"points": [[60.0, 240.0], [60.0, 170.0]]}))?;
    e("filo", json!({"points": [[70.0, 240.0], [70.0, 160.0]]}))?;
    e("filo", json!({"points": [[80.0, 240.0], [80.0, 150.0]]}))?;
    for (x, n) in [(60.0, "L1"), (70.0, "L2"), (80.0, "L3")] {
        e("rimandialimentazioni", json!({"at": [x, 240.0], "tipo": "arrivo", "segnale": n, "rotazione": 270.0}))?;
    }
    e("simbolo.qs3", json!({"at": [60.0, 215.0], "funzione1": "Interruttore generale", "funzione2": "Main switch", "costruttore": "ABB", "tipo": "OT40F3", "descrizione": "Sezionatore 40A 3P"}))?;
    e("alimentazioni", json!({"nomi": "L1 L2 L3 PE", "da": [60.0, 170.0], "a": [400.0, 170.0], "inizio": "nessuno"}))?;
    e("vv", json!({"at": [300.0, 170.0]}))?;
    e("vv", json!({"at": [310.0, 160.0]}))?;
    e("simbolo.qf2", json!({"at": [300.0, 130.0], "funzione1": "Protezione trasformatore", "costruttore": "ABB", "tipo": "S202-C4", "descrizione": "Magnetotermico 2P C4"}))?;
    e("simbolo.tc", json!({"at": [300.0, 100.0], "funzione1": "Trasformatore ausiliari", "costruttore": "Legrand", "tipo": "042810", "descrizione": "Trasformatore 400/230V 100VA"}))?;

    // foglio 2: motore con salvamotore e contattore
    e("cambiofoglio", json!({"foglio": 2}))?;
    e("descrizionecolonna", json!({"colonna": 1, "colonne": 2, "testo": "MOTORE NASTRO|4 kW", "testo2": "BELT MOTOR"}))?;
    e("alimentazioni", json!({"nomi": "L1 L2 L3 PE", "da": [20.0, 255.0], "a": [400.0, 255.0], "inizio": "arrivo", "fine": "nessuno"}))?;
    for (x, y) in [(60.0, 255.0), (70.0, 245.0), (80.0, 235.0)] {
        e("vv", json!({"at": [x, y]}))?;
    }
    e("simbolo.qm3", json!({"at": [60.0, 200.0], "funzione1": "Protezione motore nastro", "costruttore": "Schneider Electric", "tipo": "GV2ME10", "descrizione": "Salvamotore 4-6.3A"}))?;
    e("simbolo.pot3", json!({"at": [60.0, 160.0], "madre": "-KM03.1"}))?;
    e("morsetti", json!({"morsettiera": "-XM", "da": [50.0, 120.0], "a": [90.0, 120.0]}))?;
    e("simbolo.m3", json!({"at": [60.0, 80.0], "funzione1": "Motore nastro", "funzione2": "Belt motor", "costruttore": "Siemens", "tipo": "1LE1001-1BB2", "descrizione": "Motore 4kW 1400 rpm"}))?;
    e("cavo.definisci", json!({"formazione": "4G1,5", "codice": "FROR 4G1,5", "funzione": "Motore nastro", "daQuadro": "+QG", "aQuadro": "+BM", "lunghezza": "15"}))?;
    e("cavo.posa", json!({"cavo": "-W01", "da": [50.0, 90.0], "a": [90.0, 90.0]}))?;
    // il motore è a bordo macchina: il box quadro lo dice a tutto quello che racchiude
    e("box.quadro", json!({"valore": "BM", "da": [30.0, 52.0], "a": [115.0, 102.0], "descrizione": "Bordo macchina"}))?;

    // foglio 3: ausiliari 230 VAC con marcia/arresto e autoritenuta
    e("cambiofoglio", json!({"foglio": 3}))?;
    e("descrizionecolonna", json!({"colonna": 2, "testo": "MARCIA / ARRESTO NASTRO", "testo2": "BELT START / STOP"}))?;
    e("descrizionecolonna", json!({"colonna": 5, "testo": "LAMPADA MARCIA", "testo2": "RUN LAMP"}))?;
    e("alimentazioni", json!({"nomi": "L N", "da": [20.0, 255.0], "a": [400.0, 255.0]}))?;
    e("filo", json!({"points": [[100.0, 255.0], [100.0, 60.0], [380.0, 60.0], [380.0, 245.0]]}))?;
    e("simbolo.sbnc", json!({"at": [100.0, 230.0], "funzione1": "Arresto nastro", "costruttore": "Schneider Electric", "tipo": "XB5AA42", "descrizione": "Pulsante rosso 1NC"}))?;
    e("simbolo.sbno", json!({"at": [100.0, 200.0], "funzione1": "Marcia nastro", "costruttore": "Schneider Electric", "tipo": "XB5AA31", "descrizione": "Pulsante verde 1NO"}))?;
    e("simbolo.km", json!({"at": [100.0, 140.0], "funzione1": "Contattore motore nastro", "funzione2": "Belt motor contactor", "costruttore": "Schneider Electric", "tipo": "LC1D09P7", "descrizione": "Contattore 4kW 230VAC"}))?;
    e("filo", json!({"points": [[100.0, 215.0], [130.0, 215.0], [130.0, 190.0], [100.0, 190.0]]}))?;
    e("simbolo.na", json!({"at": [130.0, 210.0], "madre": "-KM03.1"}))?;
    e("simbolo.termiconc", json!({"at": [100.0, 175.0], "madre": "-QM02.1"}))?;
    e("filo", json!({"points": [[220.0, 255.0], [220.0, 60.0]]}))?;
    e("simbolo.na", json!({"at": [220.0, 200.0], "madre": "-KM03.1"}))?;
    e("simbolo.hl", json!({"at": [220.0, 140.0], "funzione1": "Lampada marcia", "costruttore": "Schneider Electric", "tipo": "XB5AVM3", "descrizione": "Lampada verde 230VAC"}))?;
    e("morsetti", json!({"morsettiera": "-XG", "da": [90.0, 80.0], "a": [230.0, 80.0]}))?;

    // foglio 4: ingressi PLC
    e("cambiofoglio", json!({"foglio": 4}))?;
    e("descrizionecolonna", json!({"colonna": 1, "colonne": 4, "testo": "INGRESSI DIGITALI PLC", "testo2": "PLC DIGITAL INPUTS"}))?;
    e("alimentazioni", json!({"nomi": "+24V 0V", "da": [20.0, 255.0], "a": [400.0, 255.0]}))?;
    for x in [60.0, 80.0, 100.0, 120.0] {
        e("filo", json!({"points": [[x, 255.0], [x, 140.0]]}))?;
    }
    e("simbolo.sqno", json!({"at": [60.0, 200.0], "funzione1": "Finecorsa avanti", "costruttore": "Telemecanique", "tipo": "XCKJ10511", "descrizione": "Finecorsa 1NO+1NC"}))?;
    e("simbolo.sqno", json!({"at": [80.0, 200.0], "funzione1": "Finecorsa indietro", "costruttore": "Telemecanique", "tipo": "XCKJ10511", "descrizione": "Finecorsa 1NO+1NC"}))?;
    e("simbolo.na", json!({"at": [100.0, 200.0], "madre": "-KM03.1"}))?;
    e(
        "plc.scheda",
        json!({"at": [60.0, 140.0], "bit": 8, "indirizzo": "E0.0", "costruttore": "Siemens", "codice": "6ES7521-1BH10-0AA0", "descrizione": "DI 16x24VDC", "commenti": [["Finecorsa avanti", "Forward limit"], ["Finecorsa indietro", "Back limit"], ["Contattore nastro", "Belt contactor"]]}),
    )?;

    // foglio 5: il quadro di distribuzione del reparto, che alimenta anche la macchina, in unifilare
    e("cambiofoglio", json!({"foglio": 5}))?;
    e("unifilare.arrivo", json!({"quadro": "+QD", "denominazione": "Generale reparto", "icc": 6, "kc": 0.8, "lunghezza": 20, "costruttore": "ABB", "codice": "S204-C"}))?;
    for (utenza, sistema, kw, nome, l, prot) in [
        ("quadro", "3F+PE", 5.5, "Quadro macchina nastro", 30.0, "mtd"),
        ("prese", "F+N+PE", 3.0, "Prese reparto", 35.0, "mtd"),
        ("luce", "F+N+PE", 1.2, "Illuminazione reparto", 40.0, "mtd"),
        ("riscaldamento", "F+N+PE", 2.0, "Stufa ufficio", 15.0, "mt"),
        ("motore", "3F+PE", 2.2, "Aspiratore", 25.0, "mt"),
        ("riserva", "3F+N+PE", 0.0, "Riserva", 0.0, "mt"),
    ] {
        e("unifilare.circuito", json!({"quadro": "+QD", "utenza": utenza, "sistema": sistema, "potenza": kw, "denominazione": nome, "lunghezza": l, "protezione": prot, "costruttore": "ABB"}))?;
    }

    e("numerafili", json!({"tutti": true}))?;
    e("siglecomponenti", json!({}))?;
    // designazione CEI EN 81346 sopra le sigle del foglio del motore, dove il box non la dice già
    e("designazioni", json!({"fogli": [2], "regola": "discordanti"}))?;
    // la seconda lingua dove manca, dal dizionario
    e("traduci", json!({"a": "EN"}))?;
    // lay-out dell'interno del quadro e foratura della piastra
    let layout = e("layoutquadro", json!({}))?;
    e("foraturapiastra", json!({"foglio": layout["foglio"]}))?;
    // calcolo termico del quadro, con l'armadio della piastra
    e("uscita", json!({"tipo": "termico", "quadro": "+QG"}))?;
    // la pulsantiera sulla porta, con la foratura
    e("uscita", json!({"tipo": "pulsantiera", "quadro": "+QG"}))?;
    for tipo in ["indice", "distinta", "morsettiere", "cablaggio", "cavi", "io", "loop", "circuiti", "funzioni", "simboli"] {
        e("uscita", json!({"tipo": tipo}))?;
    }
    e("crossreference", json!({}))?;
    // i fili col colore del conduttore: fasi marrone, nero e grigio, PE giallo-verde, +24V blu
    e("colorafili", json!({}))?;
    Ok(())
}
