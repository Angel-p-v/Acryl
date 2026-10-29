fn main() {
    tauri_build::build();
    // Enlaces con libmpv (sistema). Se requiere libmpv-dev instalado en el host.
    pkg_config::Config::new()
        .probe("mpv")
        .expect("libmpv no encontrado en el sistema; instala libmpv-dev");
    println!("cargo:rustc-link-lib=mpv");
}
