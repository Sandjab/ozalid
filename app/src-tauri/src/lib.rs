pub mod commands;
pub mod import;
pub mod interieur;
pub mod manuscrit;
pub mod png;
pub mod projet;
pub mod providers;
pub mod typst;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(commands::Atelier::default())
        .invoke_handler(tauri::generate_handler![
            commands::providers_liste,
            commands::projet_importer,
            commands::projet_ouvrir,
            commands::projet_enregistrer,
            commands::manuscrit_choisir,
            commands::manuscrit_reimporter,
            commands::livre_modifier,
            commands::composer
        ])
        .run(tauri::generate_context!())
        .expect("démarrage de l'application impossible");
}
