pub mod commands;
pub mod interieur;
pub mod manuscrit;
pub mod projet;
pub mod providers;
pub mod typst;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::providers_liste,
            commands::composer
        ])
        .run(tauri::generate_context!())
        .expect("démarrage de l'application impossible");
}
