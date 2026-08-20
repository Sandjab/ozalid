pub mod commands;
pub mod couverture;
pub mod image;
pub mod import;
pub mod interieur;
pub mod manuscrit;
pub mod maquettes;
pub mod package;
pub mod planche;
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
            commands::maquettes_liste,
            commands::polices_liste,
            commands::maquette_choisir,
            commands::couverture_modifier,
            commands::couverture_apercu,
            commands::composer,
            commands::packager
        ])
        .run(tauri::generate_context!())
        .expect("démarrage de l'application impossible");
}
