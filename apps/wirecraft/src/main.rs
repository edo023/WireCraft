//! WireCraft: l'interfaccia di CADCraft con i comandi elettrici.
//!
//! Uso: `wirecraft [file…]`
//!
//! Ricavato dall'app desktop di CADCraft (`vendor/cadcraft/apps/cadcraft/src/main.rs`, MIT OR
//! Apache-2.0); qui restano solo apertura file, impostazioni, autosalvataggio e LISP.
#![cfg_attr(all(target_os = "windows", not(debug_assertions)), windows_subsystem = "windows")]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

mod cavi_plc;
mod configuratore;
mod dialogo;
mod edita_xs;
mod gestione;
mod icone;
mod liste;
mod morsettiere;
mod navigatore;
mod pannelli;
mod planimetria;
mod progetti;
mod settaggi;
mod ribbon;
mod stampa;
mod tensioni;
mod termico;
mod topografico;
mod traduttore;
mod unifilare;

use cadcraft_engine::Session;
use cadcraft_ui_egui::{CadApp, Services, i18n};

const NOME: &str = "WireCraft";
const APP_ID: &str = "org.wirecraft.app";
/// Bit di stencil del render pass (come CADCraft: ritaglio delle finestre di layout).
const STENCIL_BITS: u8 = 8;

struct App {
    cad: CadApp,
    pannelli: pannelli::Pannelli,
    ribbon: ribbon::Ribbon,
}

impl eframe::App for App {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.cad.logic(ctx);
        if std::mem::take(&mut self.cad.quit_requested) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
    fn raw_input_hook(&mut self, ctx: &egui::Context, raw: &mut egui::RawInput) {
        self.cad.raw_input_hook(raw);
        cadcraft_ui_egui::cmdline::capture_tab(ctx, raw);
    }
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // in alto, come nei CAD elettrici diffusi: barra dei menu, poi schede e barra a schede;
        // poi i pannelli elettrici; l'interfaccia di CADCraft usa lo spazio che resta
        let con_menu = self.cad.ui.in_window_menu;
        if con_menu {
            let lingua = self.cad.language();
            cadcraft_ui_egui::i18n::with_language(lingua, || cadcraft_ui_egui::menus::menu_bar(&mut self.cad, ui));
        }
        if !self.cad.session.docs.is_empty() && !self.cad.ui.start_tab
            && let Some(azione) = self.ribbon.mostra(ui)
        {
            self.pannelli.esegui(&mut self.cad, azione);
        }
        self.pannelli.mostra(&mut self.cad, ui);
        self.cad.ui.in_window_menu = false;
        self.cad.ui(ui);
        self.cad.ui.in_window_menu = con_menu;
    }
    fn on_exit(&mut self) {
        self.cad.shutdown_autosave();
    }
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        storage.set_string(cadcraft_ui_egui::options::PROFILE_KEY, self.cad.profile_json());
        storage.set_string(cadcraft_ui_egui::PREFS_KEY, self.cad.prefs_json());
        storage.set_string(cadcraft_ui_egui::autosave::PREFS_KEY, cadcraft_ui_egui::autosave::prefs_json());
    }
    fn persist_egui_memory(&self) -> bool {
        false
    }
}

/// Lettura e scrittura dei formati (il motore non fa I/O da solo).
fn install_io() {
    cadcraft_engine::cmd::file::set_io(cadcraft_engine::cmd::file::IoHooks {
        read: |b, name| cadcraft_io::read(b, name).map_err(|e| e.to_string()),
        write: |d, name| cadcraft_io::write(d, name).map_err(|e| e.to_string()),
        plot: Some(|d, space, opts| cadcraft_io::plotter::plot(d, space, opts).map_err(|e| e.to_string())),
    });
    cadcraft_engine::cmd::file::set_write_as(|d, name, o| {
        cadcraft_io::write_with(d, name, &cadcraft_io::SaveOptions { version: o.version, binary: o.binary }).map_err(|e| e.to_string())
    });
    cadcraft_engine::cmd::file::set_entity_codec(cadcraft_engine::cmd::file::EntityCodec {
        groups: cadcraft_io::entity_groups,
        entity: cadcraft_io::entity_from_groups,
    });
}

fn services() -> Services {
    Services {
        pick_open: Some(Box::new(|| {
            rfd::FileDialog::new()
                .add_filter(i18n::t("Drawings (DWG, DXF)"), &["dwg", "dxf", "dwt"])
                .add_filter(i18n::t("All files"), &["*"])
                .pick_file()
                .map(|p| p.to_string_lossy().to_string())
        })),
        pick_save: Some(Box::new(|name: &str| {
            let mut filtri = [("Drawing (DXF)", "dxf"), ("Drawing (DWG)", "dwg"), ("PDF", "pdf"), ("SVG", "svg"), ("PNG", "png")];
            let lower = name.to_ascii_lowercase();
            filtri.sort_by_key(|(_, ext)| !lower.ends_with(&format!(".{ext}")));
            filtri
                .iter()
                .fold(rfd::FileDialog::new().set_file_name(name), |d, (label, ext)| d.add_filter(i18n::t(label), &[*ext]))
                .save_file()
                .map(|p| p.to_string_lossy().to_string())
        })),
        pick_lisp: Some(Box::new(|dir: &str| {
            let dialog = rfd::FileDialog::new().add_filter(i18n::t("AutoLISP (LSP)"), &["lsp"]).add_filter(i18n::t("All files"), &["*"]);
            let dialog = if dir.is_empty() { dialog } else { dialog.set_directory(dir) };
            dialog.pick_file().map(|p| p.to_string_lossy().to_string())
        })),
        no_file_system: false,
    }
}

/// Cartella delle impostazioni: `WIRECRAFT_CONFIG_DIR`, altrimenti `%APPDATA%\WireCraft`
/// (Windows), `~/Library/Application Support/WireCraft` (macOS), `~/.config/wirecraft`.
fn config_dir() -> Option<std::path::PathBuf> {
    let set = |k: &str| std::env::var_os(k).filter(|v| !v.is_empty()).map(std::path::PathBuf::from);
    if let Some(dir) = set("WIRECRAFT_CONFIG_DIR") {
        return Some(dir);
    }
    if cfg!(target_os = "macos") {
        set("HOME").map(|h| h.join("Library/Application Support").join(NOME))
    } else if cfg!(windows) {
        set("APPDATA").map(|a| a.join(NOME))
    } else {
        set("XDG_CONFIG_HOME").filter(|p| p.is_absolute()).or_else(|| set("HOME").map(|h| h.join(".config"))).map(|c| c.join("wirecraft"))
    }
}

/// Le cartelle delle commesse: al primo avvio `Documenti/WireCraft/Commesse`. Le cartelle
/// delle commesse esistenti si aggiungono dalla finestra Gestione Progetti.
fn prepara_commesse(file: &std::path::Path) {
    elettrico::commesse::imposta_file_percorsi(file);
    if file.exists() {
        return;
    }
    let documenti = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")).map(|h| std::path::PathBuf::from(h).join("Documents").join(NOME).join("Commesse"));
    if let Some(d) = documenti {
        if let Err(e) = std::fs::create_dir_all(&d) {
            log::warn!("cartella commesse {}: {e}", d.display());
        }
        let _ = elettrico::commesse::cambia_percorsi(Some(&d), None);
    }
}

/// L'icona della finestra: il logo di WireCraft.
fn icona() -> std::sync::Arc<egui::IconData> {
    std::sync::Arc::new(eframe::icon_data::from_png_bytes(include_bytes!("../assets/icona.png")).unwrap_or_default())
}

fn main() -> eframe::Result {
    // Prima di tutto: i comandi elettrici entrano nel registro di CADCraft.
    if !elettrico::registra() {
        eprintln!("{NOME}: comandi elettrici non registrati");
    }
    let files: Vec<String> = std::env::args().skip(1).collect();
    install_io();
    let surface = cadcraft_ui_egui::gpu::surface_config(std::env::var("CADCRAFT_VSYNC").ok().as_deref(), cfg!(all(unix, not(target_os = "macos"))));
    let mut options = eframe::NativeOptions {
        wgpu_options: eframe::egui_wgpu::WgpuConfiguration::default().with_surface_config(surface),
        viewport: egui::ViewportBuilder::default()
            .with_title(NOME)
            .with_inner_size([1600.0, 1000.0])
            .with_min_inner_size([900.0, 560.0])
            .with_drag_and_drop(true)
            .with_app_id(APP_ID)
            .with_icon(icona()),
        persist_window: false,
        stencil_buffer: STENCIL_BITS,
        ..Default::default()
    };
    // Selezioni grandi: il buffer degli indici può superare il limite predefinito di wgpu.
    if let eframe::egui_wgpu::WgpuSetup::CreateNew(create) = &mut options.wgpu_options.wgpu_setup {
        let base = std::sync::Arc::clone(&create.device_descriptor);
        create.device_descriptor = std::sync::Arc::new(move |adapter| {
            let mut d = base(adapter);
            d.required_limits.max_buffer_size = d.required_limits.max_buffer_size.max(adapter.limits().max_buffer_size);
            d
        });
    }
    eframe::run_native(
        NOME,
        options,
        Box::new(move |cc| {
            let mut app = CadApp::new(Session::empty(), services());
            app.system_languages = sys_locale::get_locales().collect();
            if let Some(profile) = cc.storage.and_then(|s| s.get_string(cadcraft_ui_egui::options::PROFILE_KEY)) {
                app.load_profile(&profile);
            }
            if let Some(prefs) = cc.storage.and_then(|s| s.get_string(cadcraft_ui_egui::PREFS_KEY)) {
                app.load_prefs(&prefs);
            }
            if let Some(prefs) = cc.storage.and_then(|s| s.get_string(cadcraft_ui_egui::autosave::PREFS_KEY)) {
                cadcraft_ui_egui::autosave::load_prefs(&prefs);
            }
            if let Some(dir) = config_dir() {
                app.enable_autosave(&dir.join("autosave"));
                if let Err(e) = elettrico::archivio::imposta(&dir.join("materiali.csv")) {
                    log::warn!("archivio materiali: {e}");
                }
                if let Err(e) = elettrico::traduttore::imposta(&dir.join("dizionario.csv")) {
                    log::warn!("dizionario: {e}");
                }
                if let Err(e) = elettrico::tensioni::imposta(&dir.join("tensioni.csv")) {
                    log::warn!("tensioni: {e}");
                }
                elettrico::multifoglio::imposta_libreria(&dir.join("Fogli memorizzati"));
                elettrico::impostazioni::imposta_configurazioni(&dir.join("Configurazioni"));
                prepara_commesse(&dir.join("percorsi-commesse.txt"));
                elettrico::impostazioni::carica_generali(&dir.join("impostazioni.ini"));
                let support = dir.join("support");
                if let Err(e) = std::fs::create_dir_all(&support) {
                    log::warn!("cartella LISP {} non creata: {e}", support.display());
                }
                app.session.set_lisp_support_dir(Some(support));
            }
            if surface.present_mode == eframe::wgpu::PresentMode::AutoNoVsync {
                app.frame_cap = Some(cadcraft_ui_egui::gpu::FrameCap::new(240));
            }
            if let Some(rs) = &cc.wgpu_render_state {
                app.set_wgpu(rs, eframe::egui_wgpu::depth_format_from_bits(0, STENCIL_BITS));
            }
            for f in &files {
                app.open_path(f);
            }
            // i file si aprono in differita: un disegno vuoto solo se non ne è stato chiesto nessuno
            if files.is_empty() && app.session.docs.is_empty() {
                app.session.new_drawing(true);
            }
            // la barra a schede sostituisce la barra strumenti e i set di strumenti di CADCraft
            app.ui.show_toolbar = false;
            app.ui.show_toolsets = false;
            let mut pannelli = pannelli::Pannelli::default();
            // come all'avvio dei CAD elettrici diffusi: senza un disegno da aprire, i progetti
            if files.is_empty() {
                pannelli.progetti.apri();
            }
            Ok(Box::new(App { cad: app, pannelli, ribbon: ribbon::Ribbon::default() }))
        }),
    )
}
