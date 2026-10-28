//! Test di interoperabilità: verifica della lettura e del ricalcolo di riferimenti
//! incrociati e rimandi su commesse DWG multifoglio preesistenti.
//! Il percorso si imposta tramite la variabile d'ambiente `COMMESSA_DWG_INTEROP` (o fallback locale);
//! se il file non è presente, il test viene saltato normalmente.

use cadcraft_engine::Session;
use serde_json::json;

const ESEMPIO_LOCALE: &str = r"C:\SPAC Automazione CAD 2026\Commesse\SPAC Esempio 2026 - IEC\ELECTRICAL DIAGRAM.dwg";

#[test]
fn test_interoperabilita_lettura_dwg_multifoglio() {
    let _uno_alla_volta = crate::uno_alla_volta();
    let percorso = std::env::var("COMMESSA_DWG_INTEROP")
        .or_else(|_| std::env::var("SPAC_ESEMPIO_DWG"))
        .unwrap_or_else(|_| ESEMPIO_LOCALE.into());
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
    // il nostro collegamento dei rimandi deve dare gli stessi riferimenti già presenti nel disegno originale
    let attesi: Vec<(String, String)> = rimandi.iter().map(|r| (r.handle.clone(), r.riferimento.clone())).collect();
    elettrico::fili::aggiorna_rimandi(&mut s).unwrap();
    let dopo = elettrico::fili::rimandi(&mut s).unwrap();
    let uguali = attesi.iter().filter(|(h, rif)| dopo.iter().any(|d| &d.handle == h && &d.riferimento == rif)).count();
    eprintln!("rimandi: {} ({} partenze), riferimenti conformi all'originale: {uguali}/{}", rimandi.len(), partenze, attesi.len());
    for (h, rif) in &attesi {
        if let Some(d) = dopo.iter().find(|d| &d.handle == h && &d.riferimento != rif) {
            eprintln!("  diverso: {} {} foglio {}: originale {rif:?}, ricalcolato {:?}", if d.partenza { "P" } else { "A" }, d.segnale, d.foglio, d.riferimento);
        }
    }

    let fili5 = elettrico::fili::fili(&s, "5").unwrap();
    eprintln!("fili sul foglio 5: {}", fili5.len());
    assert!(!fili5.is_empty());
    assert_eq!(uguali, attesi.len(), "stessi riferimenti su tutti i rimandi nel disegno");

    // figli (contatti): REG = dove sta la madre (riferimento incrociato conforme)
    let prima = elettrico::siglatura::componenti(&mut s).unwrap();
    let madri = prima.iter().filter(|c| c.madre()).count();
    let figli: Vec<(String, String)> = prima.iter().filter(|c| c.figlio()).map(|c| (c.blocco.handle.clone(), c.blocco.attributo("REG").to_string())).collect();
    elettrico::siglatura::aggiorna_cross(&mut s).unwrap();
    let dopo = elettrico::siglatura::componenti(&mut s).unwrap();
    let uguali_figli = figli.iter().filter(|(h, reg)| dopo.iter().any(|c| &c.blocco.handle == h && c.blocco.attributo("REG") == reg)).count();
    eprintln!("madri: {madri}, figli: {}, REG conformi all'originale: {uguali_figli}", figli.len());
    for (h, reg) in &figli {
        if let Some(c) = dopo.iter().find(|c| &c.blocco.handle == h && c.blocco.attributo("REG") != reg) {
            eprintln!("  figlio diverso: {} {} foglio {}: originale {reg:?}, ricalcolato {:?}", c.pres, c.nome, c.foglio, c.blocco.attributo("REG"));
        }
    }
    // nell'esempio due bit PLC hanno REG vuoto mentre gli altri bit della stessa scheda no:
    // si ammettono differenze solo lì, i contatti elettrici devono coincidere tutti
    let diversi_non_plc = figli
        .iter()
        .filter(|(h, reg)| dopo.iter().any(|c| &c.blocco.handle == h && c.blocco.attributo("REG") != reg && !c.pres.eq_ignore_ascii_case("F_PLC")))
        .count();
    assert_eq!(diversi_non_plc, 0, "stesso REG su tutti i contatti compatibili");
    assert!(uguali_figli + 2 >= figli.len());
    let _ = s.execute("elencofogli", &json!({}));
}
