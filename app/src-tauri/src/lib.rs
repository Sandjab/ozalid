pub mod commands;
pub mod couverture;
pub mod epreuve;
pub mod image;
pub mod import;
pub mod interieur;
pub mod manuscrit;
pub mod maquettes;
pub mod menu;
pub mod package;
pub mod planche;
pub mod png;
pub mod preferences;
pub mod projet;
pub mod providers;
pub mod typst;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(commands::Atelier::default())
        .setup(|app| {
            menu::poser(app.handle())?;
            Ok(())
        })
        // Le menu n'agit pas : il demande. L'interface exécute, avec le code de ses
        // propres boutons, et la garde des modifications n'a qu'un seul endroit où
        // vivre.
        .on_menu_event(|app, ev| {
            use tauri::Emitter;
            if let Err(e) = app.emit(menu::EVENEMENT, ev.id().as_ref()) {
                eprintln!("menu : événement non transmis à l'interface : {e}");
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::providers_liste,
            commands::projet_importer,
            commands::projet_ouvrir,
            commands::projet_enregistrer_sous,
            commands::projet_nouveau,
            commands::projet_fermer,
            commands::projet_enregistrer,
            commands::manuscrit_choisir,
            commands::manuscrit_reimporter,
            commands::livre_modifier,
            commands::maquettes_liste,
            commands::polices_liste,
            commands::polices_texte_liste,
            commands::interieur_modifier,
            commands::epreuve_tirer,
            commands::maquette_choisir,
            commands::couverture_modifier,
            commands::image_choisir,
            commands::couverture_apercu,
            commands::composer,
            commands::packager,
            commands::recents_liste
        ])
        .run(tauri::generate_context!())
        .expect("démarrage de l'application impossible");
}
