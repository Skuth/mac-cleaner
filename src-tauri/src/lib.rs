mod catalog;
mod clean;
mod scan;
mod types;

use std::collections::HashMap;
use std::sync::Mutex;

use types::Action;

/// Guarda as ações resolvidas do último scan, indexadas por id.
///
/// O front-end nunca envia caminhos — só ids. Toda ação destrutiva é
/// construída aqui no backend, então uma tela comprometida não consegue
/// pedir a remoção de um caminho arbitrário.
#[derive(Default)]
pub struct Catalog(pub Mutex<HashMap<String, Action>>);

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(Catalog::default())
        .invoke_handler(tauri::generate_handler![
            scan::scan,
            scan::disk_usage,
            scan::full_disk_access,
            scan::open_fda_settings,
            clean::clean,
            clean::reveal,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
