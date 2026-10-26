//! Lettura di un progetto fatto con SPAC Automazione, se installato su questo computer.
//! Il percorso si cambia con `SPAC_ESEMPIO_DWG`; senza file il test non fa nulla.

use cadcraft_engine::Session;
use serde_json::json;

const ESEMPIO: &str = r"C:\SPAC Automazione CAD 2026\Commesse\SPAC Esempio 2026 - IEC\ELECTRICAL DIAGRAM.dwg";

#[test]
fn legge_fogli_titoli_rimandi_e_fili_di_un_progetto_spac() {
    let _uno_alla_volta = crate::uno_alla_volta();
    let percorso = std::env::var("SPAC_ESEMPIO_DWG").unwrap_or_else(|_| ESEMPIO.into());
    let Ok(byte) = std::fs::read(&percorso) else {
        eprintln!("{percorso} non trovato: test saltato");
        return;
    };
    static UNA_VOLTA: std::sync::Once = std::sync::Once::new();
    UNA_VOLTA.call_once(|| {
        crate::registra();
    });
    let doc = cadcraft_io::read(&byte, "esempio.dwg").unwrap();
    let mut s = Session::empty();
    s.open_drawing(doc, "esempio.dwg", None);

    let fogli = elettrico::fogli::elenco(&mut s).unwrap();
    assert!(fogli.len() >= 40, "fogli letti: {}", fogli.len());
    assert!(fogli.iter().filter(|f| !f.titolo.is_empty()).count() > 20, "titoli dai blocchi titolo");
    eprintln!("fogli: {}, es. {:?}", fogli.len(), fogli.iter().take(5).map(|f| (f.numero, f.titolo.as_str())).collect::<Vec<_>>());

    let rimandi = elettrico::fili::rimandi(&mut s).unwrap();
    let partenze = rimandi.iter().filter(|r| r.partenza).count();
    assert!(partenze > 0 && rimandi.len() > partenze, "partenze e arrivi");
    // il nostro collegamento dei rimandi deve dare gli stessi riferimenti già scritti da SPAC
    let attesi: Vec<(String, String)> = rimandi.iter().map(|r| (r.handle.clone(), r.riferimento.clone())).collect();
    elettrico::fili::aggiorna_rimandi(&mut s).unwrap();
    let dopo = elettrico::fili::rimandi(&mut s).unwrap();
    let uguali = attesi.iter().filter(|(h, rif)| dopo.iter().any(|d| &d.handle == h && &d.riferimento == rif)).count();
    eprintln!("rimandi: {} ({} partenze), riferimenti uguali a quelli di SPAC: {uguali}/{}", rimandi.len(), partenze, attesi.len());
    for (h, rif) in &attesi {
        if let Some(d) = dopo.iter().find(|d| &d.handle == h && &d.riferimento != rif) {
            eprintln!("  diverso: {} {} foglio {}: SPAC {rif:?}, noi {:?}", if d.partenza { "P" } else { "A" }, d.segnale, d.foglio, d.riferimento);
        }
    }

    let fili5 = elettrico::fili::fili(&s, "5").unwrap();
    eprintln!("fili sul foglio 5: {}", fili5.len());
    assert!(!fili5.is_empty());
    assert_eq!(uguali, attesi.len(), "stessi riferimenti di SPAC su tutti i rimandi");

    // figli (contatti): REG = dove sta la madre, come lo scrive SPAC
    let prima = elettrico::siglatura::componenti(&mut s).unwrap();
    let madri = prima.iter().filter(|c| c.madre()).count();
    let figli: Vec<(String, String)> = prima.iter().filter(|c| c.figlio()).map(|c| (c.blocco.handle.clone(), c.blocco.attributo("REG").to_string())).collect();
    elettrico::siglatura::aggiorna_cross(&mut s).unwrap();
    let dopo = elettrico::siglatura::componenti(&mut s).unwrap();
    let uguali_figli = figli.iter().filter(|(h, reg)| dopo.iter().any(|c| &c.blocco.handle == h && c.blocco.attributo("REG") == reg)).count();
    eprintln!("madri: {madri}, figli: {}, REG uguali a SPAC: {uguali_figli}", figli.len());
    for (h, reg) in &figli {
        if let Some(c) = dopo.iter().find(|c| &c.blocco.handle == h && c.blocco.attributo("REG") != reg) {
            eprintln!("  figlio diverso: {} {} foglio {}: SPAC {reg:?}, noi {:?}", c.pres, c.nome, c.foglio, c.blocco.attributo("REG"));
        }
    }
    // nell'esempio due bit PLC hanno REG vuoto mentre gli altri bit della stessa scheda no:
    // si ammettono differenze solo lì, i contatti elettrici devono coincidere tutti
    let diversi_non_plc = figli
        .iter()
        .filter(|(h, reg)| dopo.iter().any(|c| &c.blocco.handle == h && c.blocco.attributo("REG") != reg && !c.pres.eq_ignore_ascii_case("F_PLC")))
        .count();
    assert_eq!(diversi_non_plc, 0, "stesso REG di SPAC su tutti i contatti");
    assert!(uguali_figli + 2 >= figli.len());
    let _ = s.execute("elencofogli", &json!({}));
}
