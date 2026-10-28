//! Test d'integrazione dei comandi elettrici, senza interfaccia: un solo binario (si compila
//! una volta), i test uno alla volta perché le impostazioni sono condivise da tutto il programma.
#![allow(clippy::unwrap_used, clippy::panic)]

mod commesse;
mod compatibilita_spac;
mod configuratore;
mod cavi_plc;
mod cross;
mod edita;
mod esempio;
mod excel;
mod fili;
mod fogli;
mod impostazioni;
mod layout;
mod libreria;
mod loopdiagram;
mod morsettiere;
mod multifoglio;
mod norma81346;
mod numerazione;
mod ponticelli;
mod pulsantiera;
mod robustezza;
mod revisioni;
mod salto;
mod simboli;
mod stampa;
mod targhette;
mod tensioni;
mod termico;
mod topografico;
mod traduttore;
mod unifilare;
mod uscite;

/// Registra i comandi elettrici e i formati di file (con la stampa) una volta sola.
pub fn registra() {
    static UNA_VOLTA: std::sync::Once = std::sync::Once::new();
    UNA_VOLTA.call_once(|| {
        assert!(elettrico::registra(), "registrazione comandi");
        cadcraft_engine::cmd::file::set_io(cadcraft_engine::cmd::file::IoHooks {
            read: |b, name| cadcraft_io::read(b, name).map_err(|e| e.to_string()),
            write: |d, name| cadcraft_io::write(d, name).map_err(|e| e.to_string()),
            plot: Some(|d, space, opts| cadcraft_io::plotter::plot(d, space, opts).map_err(|e| e.to_string())),
        });
    });
}

/// Esegue i test uno alla volta: alcuni cambiano le impostazioni in uso.
pub fn uno_alla_volta() -> std::sync::MutexGuard<'static, ()> {
    static LUCCHETTO: std::sync::Mutex<()> = std::sync::Mutex::new(());
    LUCCHETTO.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}
