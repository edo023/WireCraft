//! Solo su Windows: icona e informazioni di versione dentro `wirecraft.exe`. Altrove non fa nulla.

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=assets/wirecraft.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let mut res = winresource::WindowsResource::new();
    if std::path::Path::new("assets/wirecraft.ico").exists() {
        res.set_icon("assets/wirecraft.ico");
    }
    res.set("ProductName", "WireCraft")
        .set("FileDescription", "WireCraft - CAD elettrico per l'automazione industriale")
        .set("LegalCopyright", "WireCraft contributors. MIT OR Apache-2.0.")
        .set("OriginalFilename", "wirecraft.exe")
        .set("InternalName", "wirecraft");
    if let Err(e) = res.compile() {
        println!("cargo:warning=wirecraft.exe senza icona e informazioni di versione: {e}");
    }
}
