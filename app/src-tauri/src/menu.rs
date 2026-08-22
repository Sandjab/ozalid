//! Le menu natif, et le seul chemin par lequel ses entrées agissent.
//!
//! Aucune entrée n'exécute quoi que ce soit : chacune émet un événement que
//! l'interface traite avec le code de ses propres boutons. Il n'y a donc jamais
//! deux façons d'ouvrir un projet, seulement deux façons de demander la même — et
//! une garde des modifications à tenir à un seul endroit.
//!
//! Le menu Édition n'est pas décoratif : déclarer un menu sur mesure remplace celui
//! que Tauri pose par défaut, et ⌘C cesserait de fonctionner dans les champs de
//! saisie. Il en va de même du menu applicatif de macOS.

use tauri::menu::{MenuBuilder, MenuItemBuilder, SubmenuBuilder};
use tauri::{AppHandle, Manager};

use crate::preferences;

/// Nom de l'événement porté à l'interface. Sa charge utile est l'identifiant de
/// l'entrée choisie.
pub const EVENEMENT: &str = "menu";

/// Préfixe des entrées « Ouvrir un récent ». Ce qui suit est le chemin du projet :
/// l'identifiant transporte la donnée, ce qui évite de tenir un index en parallèle
/// du menu.
///
/// Le consommateur doit **retirer le préfixe** (`strip_prefix`), jamais découper sur
/// `:` — un chemin peut en contenir un, et la casse serait rare et silencieuse.
pub const RECENT: &str = "fichier.recent:";

/// Construit le menu et le pose sur l'application.
///
/// Appelée au démarrage, puis à chaque fois que la liste des récents change : le
/// menu entier est reconstruit plutôt que retouché, parce que reconstruire est sûr
/// et que ce menu est petit.
pub fn poser(app: &AppHandle) -> tauri::Result<()> {
    let mut recents = SubmenuBuilder::new(app, "Ouvrir un récent");
    let liste = liste_recents(app);
    if liste.is_empty() {
        recents = recents.item(
            &MenuItemBuilder::new("Aucun projet récent")
                .enabled(false)
                .build(app)?,
        );
    } else {
        for c in &liste {
            recents = recents.text(format!("{RECENT}{c}"), c);
        }
    }
    let recents = recents.build()?;

    let fichier = SubmenuBuilder::new(app, "Fichier")
        .item(
            &MenuItemBuilder::with_id("fichier.nouveau", "Nouveau projet")
                .accelerator("CmdOrCtrl+N")
                .build(app)?,
        )
        .item(
            &MenuItemBuilder::with_id("fichier.ouvrir", "Ouvrir un projet…")
                .accelerator("CmdOrCtrl+O")
                .build(app)?,
        )
        .item(&recents)
        .separator()
        .item(&MenuItemBuilder::with_id("fichier.importer", "Importer un livre.toml…").build(app)?)
        .separator()
        .item(
            &MenuItemBuilder::with_id("fichier.enregistrer", "Enregistrer")
                .accelerator("CmdOrCtrl+S")
                .build(app)?,
        )
        .item(
            &MenuItemBuilder::with_id("fichier.enregistrer_sous", "Enregistrer sous…")
                .accelerator("CmdOrCtrl+Shift+S")
                .build(app)?,
        )
        .separator()
        // Pas de ⌘W : sous macOS il ferme la fenêtre, et l'application n'en a qu'une.
        .item(&MenuItemBuilder::with_id("fichier.fermer", "Fermer le projet").build(app)?);

    // Sous macOS, « Quitter » vit dans le menu applicatif, où le système l'attend.
    // Ailleurs, il revient au menu Fichier — sans lui, rien n'offrirait de quitter.
    #[cfg(not(target_os = "macos"))]
    let fichier = fichier.separator().item(
        &MenuItemBuilder::with_id("fichier.quitter", "Quitter")
            .accelerator("CmdOrCtrl+Q")
            .build(app)?,
    );

    let fichier = fichier.build()?;

    // Les libellés sont écrits ici parce que personne d'autre ne le fera : les
    // entrées prédéfinies portent des chaînes anglaises fixes, que macOS ne
    // localise pas. Seul « Services » se passe de traduction, le mot étant le même.
    let edition = SubmenuBuilder::new(app, "Édition")
        .undo_with_text("Annuler")
        .redo_with_text("Rétablir")
        .separator()
        .cut_with_text("Couper")
        .copy_with_text("Copier")
        .paste_with_text("Coller")
        .select_all_with_text("Tout sélectionner")
        .build()?;

    // Jamais grisées, même sans projet ouvert : comme « Enregistrer », elles demandent
    // et c'est l'interface qui décide. Sans projet, elle ne montre rien — la garde vit
    // d'un seul côté, celui que le menu et les onglets ont en commun.
    let aller = SubmenuBuilder::new(app, "Aller")
        .item(
            &MenuItemBuilder::with_id("aller.livre", "Livre")
                .accelerator("CmdOrCtrl+1")
                .build(app)?,
        )
        .item(
            &MenuItemBuilder::with_id("aller.interieur", "Intérieur")
                .accelerator("CmdOrCtrl+2")
                .build(app)?,
        )
        .item(
            &MenuItemBuilder::with_id("aller.couverture", "Couverture")
                .accelerator("CmdOrCtrl+3")
                .build(app)?,
        )
        .item(
            &MenuItemBuilder::with_id("aller.livraison", "Livraison")
                .accelerator("CmdOrCtrl+4")
                .build(app)?,
        )
        .item(
            &MenuItemBuilder::with_id("aller.envois", "Envois")
                .accelerator("CmdOrCtrl+5")
                .build(app)?,
        )
        .build()?;

    let menu = MenuBuilder::new(app);
    // Sous macOS, le premier sous-menu devient le menu applicatif. Sans lui, ni
    // « À propos », ni « Masquer », ni ⌘Q.
    #[cfg(target_os = "macos")]
    let menu = menu.item(
        &SubmenuBuilder::new(app, "Ozalid Studio")
            .about_with_text("À propos d'Ozalid Studio", None)
            .separator()
            .services()
            .separator()
            .hide_with_text("Masquer Ozalid Studio")
            .hide_others_with_text("Masquer les autres")
            .show_all_with_text("Tout afficher")
            .separator()
            // Pas l'item prédéfini `quit` : celui-là envoie `terminate:` directement
            // au système, qui ne passe jamais par `CloseRequested` — la garde des
            // modifications ne le verrait pas passer. Une entrée ordinaire, comme
            // toutes les autres, demande au lieu d'agir.
            .item(
                &MenuItemBuilder::with_id("fichier.quitter", "Quitter Ozalid Studio")
                    .accelerator("CmdOrCtrl+Q")
                    .build(app)?,
            )
            .build()?,
    );
    let menu = menu.items(&[&fichier, &edition, &aller]).build()?;
    app.set_menu(menu)?;
    Ok(())
}

/// Les récents à porter au sous-menu. Même source que l'écran d'accueil, et même
/// élagage : un projet effacé n'y figure pas.
fn liste_recents(app: &AppHandle) -> Vec<String> {
    app.path()
        .app_config_dir()
        .ok()
        .map(|d| preferences::charger(&d).recents_existants())
        .unwrap_or_default()
}
