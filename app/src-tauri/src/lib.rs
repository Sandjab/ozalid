pub mod commands;
pub mod couverture;
pub mod detourage;
pub mod diffusion;
pub mod ebook;
pub mod envoi;
pub mod epreuve;
pub mod epub;
pub mod gabarit;
pub mod image;
pub mod import;
pub mod interieur;
pub mod manuscrit;
pub mod maquettes;
pub mod menu;
pub mod package;
pub mod planche;
pub mod png;
pub mod police;
pub mod preferences;
pub mod projet;
pub mod providers;
pub mod typst;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        // Ouvrir un PDF dans le lecteur du poste. La fenêtre n'affiche aucun document :
        // ce qu'elle compose se relit ailleurs, et jusqu'ici un chemin s'y recopiait à la
        // main. Multiplateforme, ce qui compte — la CI livre aussi Windows.
        .plugin(tauri_plugin_opener::init())
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
                // Ce contournement délibéré de la garde n'est légitime que sous
                // `!prete` — le même invariant que documenté sur `Interface` : sans
                // front démarré, il n'y a rien à perdre. Ce n'est pas une sortie de
                // secours à réutiliser ailleurs.
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
            commands::police_texte_donnee,
            commands::jetons_liste,
            commands::interieur_modifier,
            commands::epreuve_tirer,
            commands::maquette_choisir,
            commands::maquette_enregistrer,
            commands::maquette_cloner,
            commands::maquette_renommer,
            commands::maquette_effacer,
            commands::couverture_modifier,
            commands::image_choisir,
            commands::image_retirer,
            commands::couverture_apercu,
            commands::couverture_calques,
            commands::couverture_dos_boites,
            commands::composer,
            commands::destinataire_ajouter,
            commands::destinataire_retirer,
            commands::destinataire_regler,
            commands::destinataire_viser,
            commands::packager,
            commands::ebook_generer,
            commands::envoi_regler,
            commands::envoi_ajouter,
            commands::envoi_retirer,
            commands::envois_gabarit,
            commands::envois_couleur,
            commands::envois_paraphe,
            commands::gabarit_defaut_lire,
            commands::gabarit_defaut_poser,
            commands::mains_liste,
            commands::police_choisir,
            commands::police_retirer,
            commands::envoi_image_choisir,
            commands::diffusion_lire,
            commands::diffusion_regler,
            commands::envoi_generer,
            commands::envoi_accepter,
            commands::envoi_apercu,
            commands::envoi_vignettes,
            commands::envoi_page,
            commands::envoi_objet,
            commands::envoyer,
            commands::recents_liste,
            commands::garde_modifications,
            commands::interface_prete
        ])
        .run(tauri::generate_context!())
        .expect("démarrage de l'application impossible");
}
