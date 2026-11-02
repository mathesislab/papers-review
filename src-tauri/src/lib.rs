//! papers-review: the windows of the review projects.

use tauri::{AppHandle, WebviewUrl, WebviewWindowBuilder};

/// The start window: the projects opened last, and Open.
const START: &str = "start";

fn show_start(app: &AppHandle) {
    let _ = WebviewWindowBuilder::new(app, START, WebviewUrl::App("/".into()))
        .title("papers-review")
        .inner_size(560.0, 440.0)
        .resizable(false)
        .center()
        .build();
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            show_start(app.handle());
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running papers-review");
}
