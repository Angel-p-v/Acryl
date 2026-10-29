//! Punto de entrada de Acryl

mod lib;

use acryl::Config;
use acryl::ui::main_window::MainWindow;
use gtk4::prelude::*;
use gtk4::Application;

fn main() {
    env_logger::init();

    let config = match Config::load() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error cargando configuración: {}", e);
            std::process::exit(1);
        }
    };

    let app = Application::builder()
        .application_id("com.angelpv.acryl")
        .build();

    app.connect_activate(move |app| {
        let config = config.clone();
        let window = MainWindow::new(app, config);
        window.present();
    });

    app.run_with_args(&std::env::args().collect::<Vec<_>>());
}
