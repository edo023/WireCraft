//! Comandi elettrici di WireCraft, sopra CADCraft.
//!
//! Il disegno resta un normale DWG/DXF di CADCraft. Come nei CAD elettrici basati su AutoCAD:
//! - ogni **foglio** è un layer col suo numero (`1`, `2`, …) e tutti i fogli stanno alle stesse coordinate;
//!   il layer `0` è il **foglio 0** con squadratura e cartiglio, visibile sotto ogni foglio;
//! - i dati elettrici stanno negli **attributi dei blocchi**.
//!
//! Tutto passa dai comandi JSON di CADCraft ([`Session::execute`]): un aggiornamento di CADCraft
//! cambia il meno possibile qui.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

pub mod archivio;
pub mod cavi;
pub mod collegamenti;
pub mod commesse;
pub mod distinta;
pub mod edita;
pub mod fili;
pub mod fogli;
pub mod impostazioni;
pub mod layout;
pub mod morsetti;
pub mod multifoglio;
pub mod numerazione;
pub mod plc;
pub mod revisioni;
pub mod salto;
pub mod siglatura;
pub mod simboli;
pub mod specchietto;
pub mod squadratura;
pub mod stampa;
pub mod traduttore;
pub mod uscite;
mod util;

use cadcraft_engine::CommandSpec;
pub use cadcraft_engine::Session;

/// Tutti i comandi elettrici, da aggiungere al registro di CADCraft.
pub fn comandi() -> Vec<CommandSpec> {
    let mut v = Vec::new();
    v.extend(fogli::comandi());
    v.extend(multifoglio::comandi());
    v.extend(fili::comandi());
    v.extend(simboli::comandi());
    v.extend(siglatura::comandi());
    v.extend(numerazione::comandi());
    v.extend(morsetti::comandi());
    v.extend(distinta::comandi());
    v.extend(archivio::comandi());
    v.extend(stampa::comandi());
    v.extend(impostazioni::comandi());
    v.extend(commesse::comandi());
    v.extend(collegamenti::comandi());
    v.extend(uscite::comandi());
    v.extend(cavi::comandi());
    v.extend(plc::comandi());
    v.extend(revisioni::comandi());
    v.extend(edita::comandi());
    v.extend(traduttore::comandi());
    v.extend(layout::comandi());
    v.extend(salto::comandi());
    v
}

/// Registra i comandi elettrici in CADCraft. Va chiamata all'avvio, prima di aprire la finestra
/// o di eseguire comandi; restituisce false se è troppo tardi.
pub fn registra() -> bool {
    cadcraft_engine::cmd::register_extension_commands(comandi())
}
