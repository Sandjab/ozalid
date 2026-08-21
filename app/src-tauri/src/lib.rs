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
        .manage(commands::Interface::default())
        .setup(|app| {
            menu::poser(app.handle())?;
            Ok(())
        })
        // Le menu n'agit pas : il demande. L'interface exécute, avec le code de ses
        // propres boutons, et la garde des modifications n'a qu'un seul endroit où
        // vivre. Seul « Quitter » a un filet : envoyé avant que l'interface n'ait
        // posé ses écouteurs, personne ne le recevrait, et l'application deviendrait
        // inquittable — on quitte alors directement, il n'y a de toute façon rien à
        // perdre.
        .on_menu_event(|app, ev| {
            use tauri::Manager;
            let prete = app
                .state::<commands::Interface>()
                .prete
                .load(std::sync::atomic::Ordering::Relaxed);
            if ev.id().as_ref() == "fichier.quitter" && !prete {
                app.exit(0);
                return;
            }
            use tauri::Emitter;
            if let Err(e) = app.emit(menu::EVENEMENT, ev.id().as_ref()) {
                eprintln!("menu : événement non transmis à l'interface : {e}");
            }
        })
        // Fermer la fenêtre, c'est fermer l'application : la même garde doit s'y
        // appliquer. Elle ne peut pas être posée ici — la réponse « Enregistrer »
        // demande un sélecteur de fichiers, que seule l'interface possède — donc on
        // retient la fermeture et on lui passe la main. Même filet qu'au menu : sans
        // interface pour l'écouter, retenir la fermeture coincerait l'utilisateur.
        .on_window_event(|fenetre, ev| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = ev {
                use tauri::Manager;
                if !fenetre
                    .state::<commands::Interface>()
                    .prete
                    .load(std::sync::atomic::Ordering::Relaxed)
                {
                    return;
                }
                use tauri::Emitter;
                api.prevent_close();
                if let Err(e) = fenetre.emit("fermeture-demandee", ()) {
                    // L'interface est injoignable : mieux vaut fermer que coincer
                    // l'utilisateur dans une fenêtre qui refuse de partir.
                    eprintln!("fermeture : interface injoignable ({e}), fermeture forcée.");
                    let _ = fenetre.destroy();
                }
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
            commands::recents_liste,
            commands::garde_modifications,
            commands::interface_prete
        ])
        .run(tauri::generate_context!())
        .expect("démarrage de l'application impossible");
}
