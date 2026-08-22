//! Commandes exposées à l'interface. Aucune logique métier ici : elles orchestrent
//! les modules, tiennent le projet ouvert et traduisent les erreurs en messages
//! affichables.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::Serialize;
use tauri::Manager;
use tauri::State;

use crate::couverture::{self, Couverture, Ressource};
use crate::ebook;
use crate::epreuve;
use crate::import;
use crate::interieur::{self, Interieur, Reglage};
use crate::manuscrit;
use crate::maquettes;
use crate::package;
use crate::planche;
use crate::preferences;
use crate::projet::{Destinataire, Livraison, Livre, Projet};
use crate::providers::{self, Provider};
use crate::typst::Typst;

/// Le projet ouvert. Un seul à la fois : c'est un éditeur de document, pas une
/// bibliothèque. `chemin` est absent tant que le projet n'a pas été enregistré.
#[derive(Default)]
pub struct Atelier {
    ouvert: Mutex<Option<Ouvert>>,
}

struct Ouvert {
    chemin: Option<PathBuf>,
    projet: Projet,
    /// Vrai dès qu'une commande a touché au projet sans qu'il ait été réécrit.
    /// C'est lui, et lui seul, qui décide si fermer perd du travail.
    modifie: bool,
    /// La dernière image générée pour un envoi, tant qu'elle n'a pas été acceptée.
    ///
    /// Elle vit **hors du projet** : un modèle de diffusion rend rarement une écriture
    /// lisible du premier coup, et l'archive n'a pas à conserver la suite des essais.
    /// Accepter la fait entrer dans le `.ozalid` ; fermer le projet la laisse là où elle
    /// était, c'est-à-dire nulle part.
    candidat: Option<(usize, Vec<u8>)>,
}

/// Vue d'un prestataire pour l'interface.
#[derive(Serialize)]
pub struct ProviderVue {
    cle: String,
    libelle: String,
    largeur: f64,
    hauteur: f64,
    fond_perdu: Option<f64>,
    /// Vrai quand le prestataire publie de quoi calculer le dos. Faux, l'interface
    /// réclame un relevé plutôt que de laisser croire à un chiffre.
    dos_publie: bool,
    papiers: Vec<PapierVue>,
}

#[derive(Serialize)]
pub struct PapierVue {
    cle: String,
    libelle: String,
}

impl From<&Provider> for ProviderVue {
    fn from(p: &Provider) -> Self {
        Self {
            cle: p.cle.into(),
            libelle: p.libelle.into(),
            largeur: p.format.0,
            hauteur: p.format.1,
            fond_perdu: p.fond_perdu,
            // Une pagination quelconque suffit à savoir si une formule existe.
            dos_publie: p.papier_defaut().dos.mm(100).is_some(),
            papiers: p
                .papiers
                .iter()
                .map(|pa| PapierVue {
                    cle: pa.cle.into(),
                    libelle: pa.libelle.into(),
                })
                .collect(),
        }
    }
}

/// Ce que l'interface affiche d'un projet ouvert.
#[derive(Serialize)]
pub struct ProjetVue {
    pub chemin: Option<String>,
    pub livre: Livre,
    pub manuscrit_source: Option<String>,
    /// Chapitres réellement trouvés dans le manuscrit embarqué.
    pub chapitres_trouves: u32,
    pub mots: u32,
    /// Vrai quand le projet ne porte aucun texte. Distinct de « zéro chapitre » :
    /// un manuscrit présent mais non composable en trouve zéro aussi, et ce n'est
    /// pas la même chose à corriger.
    pub manuscrit_absent: bool,
    /// Modifications non enregistrées.
    pub modifie: bool,
    /// Maquette de couverture du projet, si le projet en porte une.
    pub couverture: Option<Couverture>,
    pub couverture_importee: bool,
    pub images: Vec<String>,
    pub interieur: Interieur,
    /// Les destinataires du livre et celui qu'on vise. Le front les joint à la table des
    /// gabarits par leur clé : les libellés, les formats et les papiers viennent de là.
    pub livraison: Livraison,
    /// La main du livre et ses envois. Toujours sérialisée, même vide : le front y
    /// lit la liste sans avoir à se demander si la section existe.
    pub envois: crate::envoi::Envois,
}

#[derive(Serialize)]
pub struct Composition {
    pub pages: u32,
    pub gouttiere: f64,
    pub blanche: bool,
    pub chapitres: u32,
    /// Épaisseur du dos en mm, ou `null` chez un prestataire à gabarit. C'est cette
    /// valeur qui alimentera la planche : elle n'est jamais ressaisie.
    pub dos: Option<f64>,
    pub pdf: String,
    /// Familles que Typst n'a pas trouvées et a remplacées par une écriture de repli
    /// — sans échouer, donc sans que rien d'autre ne le dise. Vide, tout va bien.
    pub polices_introuvables: Vec<String>,
}

#[tauri::command]
pub fn providers_liste() -> Vec<ProviderVue> {
    providers::PROVIDERS.iter().map(ProviderVue::from).collect()
}

/// Importe un répertoire de travail de l'ancienne chaîne (son `livre.toml`).
/// Le projet devient le projet ouvert, sans être enregistré : l'utilisateur choisit
/// où poser le `.ozalid`.
#[tauri::command]
pub fn projet_importer(livre_toml: String, atelier: State<Atelier>) -> Result<ProjetVue, String> {
    let projet = import::depuis_livre_toml(Path::new(&livre_toml))?;
    poser(&atelier, None, projet, true)
}

/// Un projet vide, à remplir.
///
/// Ni assistant ni sélecteur de fichiers : c'est un document neuf, comme dans un
/// traitement de texte. Le manuscrit se choisit quand on veut, l'enregistrement se
/// fait quand on veut. Le projet n'est pas « modifié » : il n'y a encore rien à
/// perdre, et le premier champ saisi lèvera le drapeau.
#[tauri::command]
pub fn projet_nouveau(atelier: State<Atelier>) -> Result<ProjetVue, String> {
    poser(
        &atelier,
        None,
        Projet::nouveau(Livre::vide(), String::new()),
        false,
    )
}

/// Referme le projet sans rien écrire.
///
/// La garde des modifications appartient à l'appelant : cette commande ne demande
/// rien, elle exécute. Les séparer permet à l'interface de poser la même question
/// avant Nouveau, Ouvrir, Importer et la fermeture de la fenêtre.
#[tauri::command]
pub fn projet_fermer(atelier: State<Atelier>) {
    *atelier.ouvert.lock().unwrap() = None;
}

/// Réécrit le projet là où il a déjà été enregistré.
///
/// Sans chemin mémorisé, l'interface bascule sur « Enregistrer sous… » : elle seule
/// possède le sélecteur de fichiers.
#[tauri::command]
pub fn projet_enregistrer(
    app: tauri::AppHandle,
    atelier: State<Atelier>,
) -> Result<ProjetVue, String> {
    let (vue, chemin) = {
        let mut garde = atelier.ouvert.lock().unwrap();
        let o = garde.as_mut().ok_or_else(aucun_projet)?;
        let chemin = o
            .chemin
            .clone()
            .ok_or_else(|| "projet jamais enregistré : choisir où le poser.".to_string())?;
        (enregistrer_a(o, &chemin)?, chemin)
    };
    memoriser(&app, &chemin);
    Ok(vue)
}

#[tauri::command]
pub fn projet_ouvrir(
    chemin: String,
    app: tauri::AppHandle,
    atelier: State<Atelier>,
) -> Result<ProjetVue, String> {
    let c = PathBuf::from(&chemin);
    let projet = Projet::ouvrir(&c)?;
    let vue = poser(&atelier, Some(c.clone()), projet, false)?;
    memoriser(&app, &c);
    Ok(vue)
}

#[tauri::command]
pub fn projet_enregistrer_sous(
    chemin: String,
    app: tauri::AppHandle,
    atelier: State<Atelier>,
) -> Result<ProjetVue, String> {
    let c = PathBuf::from(&chemin);
    let vue = {
        let mut garde = atelier.ouvert.lock().unwrap();
        let o = garde.as_mut().ok_or_else(aucun_projet)?;
        enregistrer_a(o, &c)?
    };
    memoriser(&app, &c);
    Ok(vue)
}

/// Les projets récents dont le fichier existe encore.
///
/// L'écran d'accueil et le sous-menu « Ouvrir un récent » lisent cette même liste :
/// il n'y a pas deux inventaires à tenir d'accord.
#[tauri::command]
pub fn recents_liste(app: tauri::AppHandle) -> Vec<String> {
    config(&app)
        .map(|d| preferences::charger(&d).recents_existants())
        .unwrap_or_default()
}

/// Libellés des trois boutons de la garde.
///
/// Ce sont eux qui font foi au retour : avec une variante personnalisée, le plugin
/// rend `MessageDialogResult::Custom(libellé)` et non un `Yes`/`No`. Les garder en
/// constantes évite que la comparaison et l'affichage divergent.
const ENREGISTRER: &str = "Enregistrer";
const IGNORER: &str = "Ne pas enregistrer";
const ANNULER: &str = "Annuler";

/// Demande quoi faire des modifications non enregistrées.
///
/// Rend `"enregistrer"`, `"ignorer"` ou `"annuler"`, et `"ignorer"` d'emblée quand
/// il n'y a rien à perdre. La commande **ne fait rien** de la réponse : c'est
/// l'interface qui agit, parce qu'elle seule possède le sélecteur de fichiers dont
/// « Enregistrer sous… » a besoin.
///
/// `async` par nécessité : `blocking_show_with_result` bloque son fil jusqu'au clic,
/// et le plugin interdit de l'appeler depuis le fil principal — ce qui serait le cas
/// d'une commande synchrone, dont le corps s'exécute en ligne dans le gestionnaire
/// de protocole de la webview.
#[tauri::command]
pub async fn garde_modifications(
    app: tauri::AppHandle,
    atelier: State<'_, Atelier>,
) -> Result<String, String> {
    // Le verrou est relâché avant la boîte : la tenir pendant que l'utilisateur
    // réfléchit condamnerait toute autre commande.
    let modifie = {
        let garde = atelier.ouvert.lock().unwrap();
        garde.as_ref().is_some_and(|o| o.modifie)
    };
    if !modifie {
        return Ok("ignorer".into());
    }

    use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
    let reponse = app
        .dialog()
        .message("Ce projet porte des modifications qui ne sont pas enregistrées.")
        .title("Enregistrer avant de continuer ?")
        .kind(MessageDialogKind::Warning)
        .buttons(MessageDialogButtons::YesNoCancelCustom(
            ENREGISTRER.into(),
            IGNORER.into(),
            ANNULER.into(),
        ))
        .blocking_show_with_result();

    Ok(reponse_garde(reponse).to_string())
}

/// Ce que le clic de l'utilisateur veut dire.
///
/// Séparé de la boîte parce que la boîte ne se simule pas, alors que cette
/// traduction, elle, se teste — et qu'une erreur ici perdrait du travail.
fn reponse_garde(r: tauri_plugin_dialog::MessageDialogResult) -> &'static str {
    use tauri_plugin_dialog::MessageDialogResult;
    match r {
        MessageDialogResult::Custom(s) if s == ENREGISTRER => "enregistrer",
        MessageDialogResult::Custom(s) if s == IGNORER => "ignorer",
        // Filet : si une plateforme rendait les valeurs canoniques plutôt que les
        // libellés, le sens resterait le même. Tout le reste — fermeture de la
        // boîte comprise — est un refus, parce que c'est le choix qui ne perd rien.
        MessageDialogResult::Yes => "enregistrer",
        MessageDialogResult::No => "ignorer",
        _ => "annuler",
    }
}

/// L'interface a-t-elle posé ses écouteurs ?
///
/// Tant qu'elle ne l'a pas fait, retenir la fermeture rendrait l'application
/// inquittable : personne n'écouterait la demande. Un front qui n'a jamais démarré
/// n'a rien à perdre non plus — on le laisse donc partir sans question.
///
/// Ce que ce filet suppose, et qui le rend sûr : le seul chemin vers
/// `modifie = true` passe par `vue_modifiee`, elle-même appelée uniquement par des
/// commandes qui exigent un projet déjà ouvert — et un projet ne s'ouvre que par une
/// commande du front. `Atelier` naît vide (`Default`), donc tant que l'interface n'a
/// pas tourné, il n'y a rien à perdre. **Si un jour `setup()` restaure ou reprend un
/// projet automatiquement, cet invariant casse** : le filet laisserait alors partir
/// un projet modifié sans le demander.
#[derive(Default)]
pub struct Interface {
    pub prete: std::sync::atomic::AtomicBool,
}

/// L'interface annonce qu'elle écoute. Appelée une fois, au chargement.
#[tauri::command]
pub fn interface_prete(interface: State<Interface>) {
    // `Relaxed` suffit : ce drapeau ne publie rien d'autre que lui-même — l'état
    // partagé qui compte, `Atelier.ouvert`, a son propre `Mutex`. Les lecteurs ne
    // font que décider d'émettre ou non, jamais lire une valeur posée à côté.
    interface
        .prete
        .store(true, std::sync::atomic::Ordering::Relaxed);
}

/// Relit le manuscrit à sa source d'origine et remplace la copie embarquée.
///
/// Le `.ozalid` est auto-portant : le manuscrit y est copié, donc une correction faite
/// dans l'éditeur de texte n'y entre que par ce geste. Le chemin d'origine est
/// mémorisé pour que ce soit un bouton et non une navigation.
#[tauri::command]
pub fn manuscrit_reimporter(atelier: State<Atelier>) -> Result<ProjetVue, String> {
    let mut garde = atelier.ouvert.lock().unwrap();
    let o = garde.as_mut().ok_or_else(aucun_projet)?;
    let source = o.projet.meta.manuscrit.source.clone().ok_or_else(|| {
        "ce projet ne mémorise aucune source de manuscrit — en choisir une.".to_string()
    })?;
    o.projet.texte = std::fs::read_to_string(&source)
        .map_err(|e| format!("manuscrit introuvable ({source}) : {e}"))?;
    vue_modifiee(o)
}

/// Remplace le manuscrit par un fichier choisi, et mémorise son chemin.
#[tauri::command]
pub fn manuscrit_choisir(chemin: String, atelier: State<Atelier>) -> Result<ProjetVue, String> {
    let mut garde = atelier.ouvert.lock().unwrap();
    let o = garde.as_mut().ok_or_else(aucun_projet)?;
    o.projet.texte =
        std::fs::read_to_string(&chemin).map_err(|e| format!("manuscrit illisible : {e}"))?;
    o.projet.meta.manuscrit.source = Some(chemin);
    vue_modifiee(o)
}

#[tauri::command]
pub fn livre_modifier(livre: Livre, atelier: State<Atelier>) -> Result<ProjetVue, String> {
    let mut garde = atelier.ouvert.lock().unwrap();
    let o = garde.as_mut().ok_or_else(aucun_projet)?;
    o.projet.meta.livre = livre;
    vue_modifiee(o)
}

#[tauri::command]
pub fn polices_texte_liste() -> Vec<&'static str> {
    interieur::POLICES_TEXTE.to_vec()
}

#[tauri::command]
pub fn interieur_modifier(
    interieur: Interieur,
    atelier: State<Atelier>,
) -> Result<ProjetVue, String> {
    interieur.verifie()?;
    let mut garde = atelier.ouvert.lock().unwrap();
    let o = garde.as_mut().ok_or_else(aucun_projet)?;
    o.projet.meta.interieur = interieur;
    vue_modifiee(o)
}

/* ---------- destinataires ---------- */

/// Le destinataire visé, avec le gabarit et le papier de la table qui vont avec.
///
/// Le point de passage unique de tout ce qui a besoin d'un prestataire : composer,
/// apercevoir, mesurer un dos. Il n'y a plus de second endroit où le choisir.
fn vise(
    o: &Ouvert,
) -> Result<(&'static Provider, &'static providers::Papier, &Destinataire), String> {
    let d = o
        .projet
        .meta
        .livraison
        .courant()
        .ok_or("aucun destinataire : en déclarer un à l'étape Livraison.")?;
    let pr = providers::provider(&d.provider)
        .ok_or_else(|| format!("prestataire inconnu : {}", d.provider))?;
    Ok((pr, papier(pr, Some(&d.papier))?, d))
}

/// Ajoute un prestataire à la liste des destinataires, avec son papier par défaut.
#[tauri::command]
pub fn destinataire_ajouter(
    provider_cle: String,
    atelier: State<Atelier>,
) -> Result<ProjetVue, String> {
    let pr = providers::provider(&provider_cle)
        .ok_or_else(|| format!("prestataire inconnu : {provider_cle}"))?;
    let mut garde = atelier.ouvert.lock().unwrap();
    let o = garde.as_mut().ok_or_else(aucun_projet)?;
    let l = &mut o.projet.meta.livraison;
    if l.destinataires.iter().any(|d| d.provider == pr.cle) {
        return Err(format!("{} est déjà destinataire de ce livre.", pr.libelle));
    }
    l.destinataires.push(Destinataire::pour(pr));
    vue_modifiee(o)
}

/// Retire un destinataire — sauf le dernier : c'est lui qui donne son format à
/// l'aperçu, et une liste vide rendrait la Couverture inutilisable.
#[tauri::command]
pub fn destinataire_retirer(
    provider_cle: String,
    atelier: State<Atelier>,
) -> Result<ProjetVue, String> {
    let mut garde = atelier.ouvert.lock().unwrap();
    let o = garde.as_mut().ok_or_else(aucun_projet)?;
    let l = &mut o.projet.meta.livraison;
    if l.destinataires.len() < 2 {
        return Err(
            "un livre garde au moins un destinataire : c'est lui qui donne le format \
             sous lequel on regarde la couverture."
                .into(),
        );
    }
    let avant = l.destinataires.len();
    l.destinataires.retain(|d| d.provider != provider_cle);
    if l.destinataires.len() == avant {
        return Err(format!(
            "{provider_cle} n'est pas destinataire de ce livre."
        ));
    }
    // Retirer celui qu'on visait laisse le pointeur en l'air : il retombe sur le
    // premier, plutôt que de désigner un absent jusqu'au prochain geste.
    if l.courant().is_none() {
        l.courant = l.destinataires[0].provider.clone();
    }
    vue_modifiee(o)
}

/// Le papier d'un destinataire et, chez ceux qui ne publient rien, ses relevés.
#[tauri::command]
pub fn destinataire_regler(
    destinataire: Destinataire,
    atelier: State<Atelier>,
) -> Result<ProjetVue, String> {
    let pr = providers::provider(&destinataire.provider)
        .ok_or_else(|| format!("prestataire inconnu : {}", destinataire.provider))?;
    papier(pr, Some(&destinataire.papier))?;
    let mut garde = atelier.ouvert.lock().unwrap();
    let o = garde.as_mut().ok_or_else(aucun_projet)?;
    let place = o
        .projet
        .meta
        .livraison
        .destinataires
        .iter_mut()
        .find(|d| d.provider == destinataire.provider)
        .ok_or_else(|| format!("{} n'est pas destinataire de ce livre.", pr.libelle))?;
    *place = destinataire;
    vue_modifiee(o)
}

/// Déplace le pointeur : pour qui l'on compose, et sous quel format on regarde.
///
/// Le geste modifie le projet, parce que le pointeur est enregistré avec lui : rouvrir
/// un livre le rend tel qu'on l'avait laissé, visé sur le même destinataire.
#[tauri::command]
pub fn destinataire_viser(
    provider_cle: String,
    atelier: State<Atelier>,
) -> Result<ProjetVue, String> {
    let mut garde = atelier.ouvert.lock().unwrap();
    let o = garde.as_mut().ok_or_else(aucun_projet)?;
    let l = &mut o.projet.meta.livraison;
    if !l.destinataires.iter().any(|d| d.provider == provider_cle) {
        return Err(format!(
            "{provider_cle} n'est pas destinataire de ce livre."
        ));
    }
    l.courant = provider_cle;
    vue_modifiee(o)
}

/// Compose l'intérieur du projet ouvert pour le destinataire visé, et rend le compte
/// de pages avec le dos qui en découle.
#[tauri::command]
pub fn composer(atelier: State<Atelier>) -> Result<Composition, String> {
    let garde = atelier.ouvert.lock().unwrap();
    let o = garde.as_ref().ok_or_else(aucun_projet)?;
    let (pr, papier, _) = vise(o)?;

    let livre = &o.projet.meta.livre;
    let int = &o.projet.meta.interieur;
    // `interieur::source` interpole la police sans échappement : la validation est ici.
    int.verifie()?;
    let chapitres = manuscrit::decoupe(&o.projet.texte, livre.chapitres)?;

    let dossier = sorties_dossier(o, pr.cle)?;
    std::fs::create_dir_all(&dossier).map_err(|e| {
        format!(
            "répertoire de sortie inutilisable ({}) : {e}",
            dossier.display()
        )
    })?;

    let typst = typst()?;
    let src = dossier.join(format!("interieur-{}.typ", pr.cle));

    // La convergence ne mesure que le compte de pages : aucun PDF n'est produit tant
    // que le réglage n'est pas stable.
    let r = interieur::converge(pr, |reglage| {
        ecrire(
            &src,
            &interieur::source(livre, int, pr, reglage, &chapitres, None),
        )?;
        typst.pages(&src)
    })?;

    let reglage = Reglage {
        gouttiere: r.gouttiere,
        blanche: r.blanche,
    };
    ecrire(
        &src,
        &interieur::source(livre, int, pr, &reglage, &chapitres, None),
    )?;
    let pdf = dossier.join(format!("interieur-{}.pdf", pr.cle));
    let polices_introuvables = typst.compile(&src, &pdf)?;

    Ok(Composition {
        pages: r.pages,
        gouttiere: r.gouttiere,
        blanche: r.blanche,
        chapitres: chapitres.len() as u32,
        dos: papier.dos.mm(r.pages),
        pdf: pdf.to_string_lossy().into_owned(),
        polices_introuvables,
    })
}

/// Tire l'épreuve de relecture à la racine des sorties : elle ne vise aucun éditeur,
/// elle ne descend donc pas dans un répertoire de prestataire.
#[tauri::command]
pub fn epreuve_tirer(corps_pt: f64, atelier: State<Atelier>) -> Result<String, String> {
    let garde = atelier.ouvert.lock().unwrap();
    let o = garde.as_ref().ok_or_else(aucun_projet)?;
    let livre = &o.projet.meta.livre;
    let int = &o.projet.meta.interieur;
    // `epreuve::source` interpole la police sans échappement : la validation est ici.
    int.verifie()?;
    let chapitres = manuscrit::decoupe(&o.projet.texte, livre.chapitres)?;

    let dossier = sorties_racine(o)?;
    std::fs::create_dir_all(&dossier).map_err(|e| {
        format!(
            "répertoire de sortie inutilisable ({}) : {e}",
            dossier.display()
        )
    })?;
    let src = dossier.join("epreuve.typ");
    ecrire(&src, &epreuve::source(livre, int, &chapitres, corps_pt))?;
    let pdf = dossier.join("epreuve.pdf");
    // Les substitutions de police ne sont pas remontées ici : l'épreuve se lit pour
    // son texte, et composer l'intérieur — qui emploie les mêmes polices — les
    // signale déjà dans son compte rendu.
    typst()?.compile(&src, &pdf)?;
    Ok(pdf.to_string_lossy().into_owned())
}

/* ---------- couverture ---------- */

#[derive(Serialize)]
pub struct MaquetteVue {
    cle: String,
    libelle: String,
}

#[tauri::command]
pub fn maquettes_liste() -> Vec<MaquetteVue> {
    maquettes::toutes()
        .into_iter()
        .map(|(cle, libelle, _)| MaquetteVue {
            cle: cle.into(),
            libelle: libelle.into(),
        })
        .collect()
}

#[tauri::command]
pub fn polices_liste() -> Vec<&'static str> {
    couverture::POLICES.to_vec()
}

/// Charge une maquette de départ. Elle remplace la mise en page, jamais l'identité du
/// livre : le titre et l'auteur imprimés restent ceux du projet.
#[tauri::command]
pub fn maquette_choisir(cle: String, atelier: State<Atelier>) -> Result<ProjetVue, String> {
    let m = maquettes::par_cle(&cle).ok_or_else(|| format!("maquette inconnue : {cle}"))?;
    let mut garde = atelier.ouvert.lock().unwrap();
    let o = garde.as_mut().ok_or_else(aucun_projet)?;
    o.projet.meta.couverture.maquette = Some(m);
    vue_modifiee(o)
}

#[tauri::command]
pub fn couverture_modifier(
    couverture: Couverture,
    atelier: State<Atelier>,
) -> Result<ProjetVue, String> {
    let mut garde = atelier.ouvert.lock().unwrap();
    let o = garde.as_mut().ok_or_else(aucun_projet)?;
    o.projet.meta.couverture.maquette = Some(couverture);
    vue_modifiee(o)
}

/// Nom sous lequel une image entre dans le projet, selon la face qu'elle sert.
///
/// Le nom porte le rôle — c'est ainsi que la composition le lit — et l'extension
/// vient du fichier choisi, parce que Typst distingue le PNG du JPEG.
fn nom_image(face: &str, ext: &str) -> Result<String, String> {
    match face {
        "une" => Ok(format!("couverture.{ext}")),
        "quatre" => Ok(format!("quatrieme.{ext}")),
        autre => Err(format!("face inconnue : {autre}")),
    }
}

/// Remplace l'image d'une face par un fichier choisi.
///
/// Le projet est auto-portant : l'image y est copiée, comme le manuscrit. Elle est
/// refusée ici plutôt qu'à la composition — une image dont Typst ne saura rien faire
/// n'a pas à entrer dans un `.ozalid` qui l'emporterait partout ensuite.
#[tauri::command]
pub fn image_choisir(
    face: String,
    chemin: String,
    atelier: State<Atelier>,
) -> Result<ProjetVue, String> {
    let source = Path::new(&chemin);
    let ext = source
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .filter(|e| matches!(e.as_str(), "jpg" | "jpeg" | "png"))
        .ok_or("image refusée : seuls le JPEG et le PNG se composent.")?;
    let nom = nom_image(&face, &ext)?;
    let octets = std::fs::read(source).map_err(|e| format!("image illisible : {e}"))?;
    Ressource::depuis(&nom, &octets)
        .ok_or_else(|| format!("{nom} : dimensions illisibles (ni PNG ni JPEG)."))?;

    let mut garde = atelier.ouvert.lock().unwrap();
    let o = garde.as_mut().ok_or_else(aucun_projet)?;
    poser_image(&mut o.projet.images, nom, octets);
    vue_modifiee(o)
}

/// Pose l'image d'une face et retire celle qui tenait déjà ce rôle.
///
/// Le remplacement se fait par rôle, pas par nom : une image importée s'appelle comme
/// elle veut, et deux images qui servent la même face laisseraient l'ordre alphabétique
/// décider laquelle se compose.
fn poser_image(images: &mut BTreeMap<String, Vec<u8>>, nom: String, octets: Vec<u8>) {
    let quatre = package::sert_la_quatrieme(&nom);
    images.retain(|n, _| package::sert_la_quatrieme(n) != quatre);
    images.insert(nom, octets);
}

/// Aperçu d'une face de couverture ou de la planche entière, en PNG encodé dans une
/// URL `data:`.
///
/// L'aperçu sort du **même** moteur et de la même source que le PDF final : il n'y a
/// donc pas d'écart écran/export à surveiller, contrairement à l'atelier HTML.
///
/// `dos_mm` vient de la dernière composition de l'intérieur ; il n'est jamais saisi.
/// Sans lui, la planche ne s'aperçoit pas — c'est voulu : une planche dont le dos
/// serait deviné donnerait à voir un livre qui n'existe pas.
#[tauri::command]
pub fn couverture_apercu(
    face: String,
    dos_mm: Option<f64>,
    atelier: State<Atelier>,
) -> Result<String, String> {
    let garde = atelier.ouvert.lock().unwrap();
    let o = garde.as_ref().ok_or_else(aucun_projet)?;
    let cv = o
        .projet
        .meta
        .couverture
        .maquette
        .as_ref()
        .ok_or("aucune maquette : en choisir une.")?;
    // Le format vient du destinataire visé, et le fond perdu de son relevé quand le
    // prestataire n'en publie pas : les deux sont dans le projet, plus dans un champ.
    let (pr, _, d) = vise(o)?;
    let fond_perdu_mm = d.fond_perdu_mm;

    // Répertoire de travail de l'aperçu : temporaire, jamais à côté du projet. Un
    // aperçu n'est pas une sortie, et il est réécrit à chaque réglage.
    let dossier = std::env::temp_dir().join("ozalid-apercu");
    std::fs::create_dir_all(&dossier).map_err(|e| format!("aperçu impossible : {e}"))?;

    let (une, quatre) = ecrire_images(&o.projet, &dossier)?;
    let src = match face.as_str() {
        "une" => couverture::source_une(&o.projet.meta.livre, cv, pr.format, une.as_ref(), dos_mm),
        "quatre" => {
            couverture::source_quatre(cv, pr.format, quatre.as_ref(), une.as_ref(), dos_mm)?
        }
        "planche" => {
            let dos = dos_mm.ok_or(
                "planche : composer l'intérieur d'abord, c'est la pagination qui donne le dos.",
            )?;
            let fp = pr.fond_perdu.or(fond_perdu_mm).ok_or_else(|| {
                format!(
                    "{} ne publie pas de fond perdu : le relever sur son gabarit et le saisir.",
                    pr.libelle
                )
            })?;
            let g = planche::Gabarit {
                format: pr.format,
                dos,
                fond_perdu: fp,
            };
            planche::source(&o.projet.meta.livre, cv, &g, une.as_ref(), quatre.as_ref())?
        }
        autre => return Err(format!("face inconnue : {autre}")),
    };

    let typ = dossier.join(format!("apercu-{face}.typ"));
    let png = dossier.join(format!("apercu-{face}.png"));
    ecrire(&typ, &src)?;
    typst()?.apercu(&typ, &png, 1, 150)?;

    donnee_png(&png)
}

/// Un PNG du disque, en donnée `data:` : la fenêtre ne lit pas les fichiers, une image
/// n'y entre pas autrement.
fn donnee_png(chemin: &Path) -> Result<String, String> {
    let octets = std::fs::read(chemin).map_err(|e| format!("aperçu illisible : {e}"))?;
    Ok(donnee_image(&octets))
}

/// Des octets d'image, prêts à poser dans une balise `img`.
///
/// Le type est relevé sur le contenu : la fenêtre affiche d'après lui, et un JPEG
/// annoncé en PNG resterait un cadre vide.
fn donnee_image(octets: &[u8]) -> String {
    let type_mime = match crate::image::extension(octets) {
        Some("jpg") => "image/jpeg",
        _ => "image/png",
    };
    format!(
        "data:{type_mime};base64,{}",
        base64::Engine::encode(&base64::engine::general_purpose::STANDARD, octets)
    )
}

/* ---------- packages ---------- */

/// Ce que rend la génération pour un prestataire : le package, ou l'erreur qui l'a
/// empêché. Un prestataire en échec n'interrompt pas les autres — mais il est dit.
#[derive(Serialize)]
pub struct Resultat {
    pub provider: String,
    pub libelle: String,
    pub package: Option<package::Package>,
    /// La planche du package, en PNG, prête à poser dans une balise `img`.
    ///
    /// Le chemin du fichier ne suffirait pas : la fenêtre ne lit pas le disque, et
    /// c'est déjà par une donnée en clair que l'aperçu de la Couverture voyage.
    pub vignette: Option<String>,
    pub erreur: Option<String>,
}

/// Génère le package de chaque destinataire du livre, chacun dans son répertoire.
///
/// Une seule maquette, N destinataires, aucun réglage retouché entre eux : chacun
/// compose son propre intérieur, donc sa propre pagination, donc son propre dos. C'est
/// la promesse de l'étape Livraison, et la liste vient du projet — plus de cases à
/// cocher qui désigneraient les prestataires une seconde fois.
#[tauri::command]
pub fn packager(atelier: State<Atelier>) -> Result<Vec<Resultat>, String> {
    let garde = atelier.ouvert.lock().unwrap();
    let o = garde.as_ref().ok_or_else(aucun_projet)?;
    let destinataires = o.projet.meta.livraison.destinataires.clone();
    if destinataires.is_empty() {
        return Err("aucun destinataire : en déclarer un.".into());
    }
    let typst = typst()?;

    let mut sorties = Vec::with_capacity(destinataires.len());
    for d in &destinataires {
        let Some(pr) = providers::provider(&d.provider) else {
            sorties.push(Resultat {
                provider: d.provider.clone(),
                libelle: d.provider.clone(),
                package: None,
                vignette: None,
                erreur: Some(format!("prestataire inconnu : {}", d.provider)),
            });
            continue;
        };
        let r = papier(pr, Some(&d.papier)).and_then(|pa| {
            let dossier = sorties_dossier(o, pr.cle)?;
            package::assembler(
                &o.projet,
                pr,
                pa,
                planche::Releve {
                    dos: d.dos_mm,
                    fond_perdu: d.fond_perdu_mm,
                },
                &dossier,
                &typst,
            )
        });
        sorties.push(match r {
            Ok(p) => Resultat {
                provider: pr.cle.into(),
                libelle: pr.libelle.into(),
                // La vignette manquante ne perd pas le package : les PDF sont écrits,
                // et c'est eux que l'imprimeur reçoit.
                vignette: donnee_png(Path::new(&p.vignette)).ok(),
                package: Some(p),
                erreur: None,
            },
            Err(e) => Resultat {
                provider: pr.cle.into(),
                libelle: pr.libelle.into(),
                package: None,
                vignette: None,
                erreur: Some(e),
            },
        });
    }
    Ok(sorties)
}

/// Génère les ebooks locaux dans `<projet>/ebook/`.
///
/// Une livraison, mais locale : elle ne vise aucun prestataire, elle emprunte seulement
/// le gabarit de celui qui est visé — c'est de là que viennent le format, le corps et
/// l'interligne, faute d'un format d'écran qui voudrait dire quelque chose.
#[tauri::command]
pub fn ebook_generer(atelier: State<Atelier>) -> Result<ebook::Ebooks, String> {
    let garde = atelier.ouvert.lock().unwrap();
    let o = garde.as_ref().ok_or_else(aucun_projet)?;
    let (pr, _, d) = vise(o)?;
    let dossier = sorties_racine(o)?.join("ebook");
    ebook::generer(&o.projet, pr, d.dos_mm, &dossier, &typst()?)
}

/* ---------- envois ---------- */

/// Ce qu'un envoi produit, du point de vue de l'interface.
#[derive(Serialize)]
pub struct ResultatEnvoi {
    pub dedicataire: String,
    /// Nom du répertoire écrit sous `envois/` — assaini, donc pas toujours celui du
    /// dédicataire. C'est celui-là qu'il faut ouvrir, et donc celui-là qu'on montre.
    pub dossier: String,
    pub package: package::Package,
    pub vignette: Option<String>,
}

/// Remplace la liste des envois et la main du livre.
///
/// Comme `livre_modifier`, la commande reçoit **l'objet entier** : ce que le front
/// n'envoie pas est effacé. C'est le même piège que la dédicace, et il se garde du
/// même côté.
#[tauri::command]
pub fn envois_modifier(
    envois: crate::envoi::Envois,
    atelier: State<Atelier>,
) -> Result<ProjetVue, String> {
    let mut garde = atelier.ouvert.lock().unwrap();
    let o = garde.as_mut().ok_or_else(aucun_projet)?;
    o.projet.regler_envois(envois)?;
    vue_modifiee(o)
}

/// Les mains offertes par l'application.
///
/// La police personnelle n'y est pas : elle appartient au livre ouvert, pas à
/// l'application, et le front la lit dans `envois.personnelle`.
#[tauri::command]
pub fn mains_liste() -> Vec<&'static str> {
    crate::envoi::MAINS.to_vec()
}

/// Embarque la police manuscrite de l'auteur dans le projet, et en fait sa main.
///
/// Le fichier est copié dans le `.ozalid`, comme le manuscrit et les photos : le projet
/// doit composer à l'identique sur une machine où cette écriture n'est installée nulle
/// part. C'est aussi pourquoi la famille est relevée dans le fichier plutôt que déduite
/// de son nom.
#[tauri::command]
pub fn police_choisir(chemin: String, atelier: State<Atelier>) -> Result<ProjetVue, String> {
    let source = Path::new(&chemin);
    // Typst ne charge d'un répertoire de polices que les fichiers dont l'extension le
    // dit. Une écriture rangée sous un autre nom n'y serait jamais lue, et l'envoi
    // partirait dans la police de repli sans qu'aucun message ne le signale.
    let nom = source
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .filter(|n| {
            let bas = n.to_lowercase();
            bas.ends_with(".ttf") || bas.ends_with(".otf")
        })
        .ok_or("police refusée : seuls les fichiers .ttf et .otf se composent.")?;
    let octets = std::fs::read(source).map_err(|e| format!("police illisible : {e}"))?;

    let mut garde = atelier.ouvert.lock().unwrap();
    let o = garde.as_mut().ok_or_else(aucun_projet)?;
    o.projet.poser_police(&nom, octets)?;
    vue_modifiee(o)
}

/// Embarque l'image écrite à la main pour un envoi.
///
/// Elle entre dans le `.ozalid` sous `envois/`, et non avec les photos de couverture :
/// là-bas, une image dont le nom ne commence pas par `quatrieme` devient la première de
/// couverture — le mot manuscrit d'un lecteur remplacerait la couverture du livre.
#[tauri::command]
pub fn envoi_image_choisir(
    index: usize,
    chemin: String,
    atelier: State<Atelier>,
) -> Result<ProjetVue, String> {
    // Aucun contrôle sur l'extension du fichier choisi : c'est le contenu qui décide,
    // et `poser_image_envoi` le relève. Une photo d'appareil renommée en `.png` reste
    // un JPEG, et Typst la lirait à son nom.
    let octets = std::fs::read(Path::new(&chemin)).map_err(|e| format!("image illisible : {e}"))?;

    let mut garde = atelier.ouvert.lock().unwrap();
    let o = garde.as_mut().ok_or_else(aucun_projet)?;
    o.projet.poser_image_envoi(index, octets)?;
    vue_modifiee(o)
}

/// Ce que l'interface sait de l'accès au modèle de diffusion.
///
/// **La clé n'y est pas.** Elle est en clair dans `preferences.toml`, avec les
/// permissions du fichier ; la renvoyer au front la ferait entrer dans une page, donc
/// dans une capture d'écran, donc dans un message. Savoir qu'elle est posée suffit à
/// régler l'accès.
#[derive(Serialize)]
pub struct AccesVue {
    pub url: String,
    pub cle_posee: bool,
}

#[tauri::command]
pub fn diffusion_lire(app: tauri::AppHandle) -> AccesVue {
    let d = config(&app).map(|c| preferences::charger(&c).diffusion);
    AccesVue {
        url: d.as_ref().map(|d| d.url.clone()).unwrap_or_default(),
        cle_posee: d.is_some_and(|d| !d.cle.trim().is_empty()),
    }
}

/// Règle l'accès au modèle. `cle` absente laisse en place celle qui est enregistrée.
///
/// Sans cela, corriger l'adresse effacerait la clé — le champ de saisie est vide à
/// l'écran, puisqu'on ne la lui redonne jamais.
#[tauri::command]
pub fn diffusion_regler(
    url: String,
    cle: Option<String>,
    app: tauri::AppHandle,
) -> Result<AccesVue, String> {
    let dir = config(&app).ok_or("répertoire de configuration introuvable.")?;
    let mut p = preferences::charger(&dir);
    p.diffusion.url = url;
    if let Some(c) = cle {
        p.diffusion.cle = c;
    }
    preferences::enregistrer(&dir, &p)?;
    Ok(diffusion_lire(app))
}

/// Demande au modèle l'image d'un envoi, et la garde de côté sans la figer.
///
/// Rendue en PNG encodé pour l'aperçu, et **pas** écrite dans le projet : un modèle de
/// diffusion rend rarement une écriture lisible du premier coup. On regarde, on
/// regénère, et c'est `envoi_accepter` qui fait entrer l'image dans l'archive.
#[tauri::command]
pub fn envoi_generer(
    index: usize,
    app: tauri::AppHandle,
    atelier: State<Atelier>,
) -> Result<String, String> {
    let acces = config(&app)
        .map(|c| preferences::charger(&c).diffusion)
        .unwrap_or_default();
    let mut garde = atelier.ouvert.lock().unwrap();
    let o = garde.as_mut().ok_or_else(aucun_projet)?;
    let crate::envoi::Main::Diffusion { gabarit } = &o.projet.meta.envois.main else {
        return Err("la main de ce livre n'est pas une image générée.".into());
    };
    let e = o
        .projet
        .meta
        .envois
        .liste
        .get(index)
        .ok_or("envoi introuvable : la liste a changé.")?;

    let octets = crate::diffusion::genere(
        &acces,
        &crate::diffusion::prompt(gabarit, &e.contenu),
        &crate::diffusion::Reseau,
    )?;
    let donnee = donnee_image(&octets);
    o.candidat = Some((index, octets));
    Ok(donnee)
}

/// Fige l'image générée : elle entre dans l'archive, et n'en bouge plus.
///
/// À partir d'ici, composer ne rappelle jamais le réseau — le package se refait des mois
/// plus tard, hors ligne, à l'identique.
#[tauri::command]
pub fn envoi_accepter(index: usize, atelier: State<Atelier>) -> Result<ProjetVue, String> {
    let mut garde = atelier.ouvert.lock().unwrap();
    let o = garde.as_mut().ok_or_else(aucun_projet)?;
    let (_, octets) = o
        .candidat
        .take()
        // Le candidat porte son index : accepter après avoir changé de ligne poserait
        // sinon l'image d'une personne sur l'exemplaire d'une autre.
        .filter(|(pour, _)| *pour == index)
        .ok_or("aucune image en attente pour cet envoi : en générer une.")?;
    o.projet.poser_image_envoi(index, octets)?;
    vue_modifiee(o)
}

/// Retire la police de l'auteur du projet.
#[tauri::command]
pub fn police_retirer(atelier: State<Atelier>) -> Result<ProjetVue, String> {
    let mut garde = atelier.ouvert.lock().unwrap();
    let o = garde.as_mut().ok_or_else(aucun_projet)?;
    o.projet.retirer_police();
    vue_modifiee(o)
}

/// La page de titre d'un envoi, telle qu'elle sera imprimée.
///
/// La source est celle de l'intérieur **privée de ses chapitres** : la page de titre ne
/// dépend pas du corps, et composer trois cents pages pour en regarder une seule ferait
/// de l'aperçu quelque chose qu'on n'ouvre jamais. La gouttière prise est la première
/// tranche du gabarit — elle ne déplace que la marge intérieure, et cet aperçu n'est
/// pas ce qui part à l'imprimeur : le PDF l'est.
#[tauri::command]
pub fn envoi_apercu(index: usize, atelier: State<Atelier>) -> Result<String, String> {
    let garde = atelier.ouvert.lock().unwrap();
    let o = garde.as_ref().ok_or_else(aucun_projet)?;
    let (pr, _, _) = vise(o)?;
    let envois = &o.projet.meta.envois;
    envois.verifie()?;
    let e = envois
        .liste
        .get(index)
        .ok_or("envoi introuvable : la liste a changé.")?;

    let int = &o.projet.meta.interieur;
    int.verifie()?;
    let dossier = sorties_racine(o)?.join("envois");
    std::fs::create_dir_all(&dossier)
        .map_err(|err| format!("répertoire inutilisable ({}) : {err}", dossier.display()))?;
    let src = dossier.join("apercu.typ");
    ecrire(
        &src,
        &interieur::source(
            &o.projet.meta.livre,
            int,
            pr,
            &Reglage {
                gouttiere: pr.gouttieres[0].2,
                blanche: false,
            },
            &[],
            Some(package::trace(&o.projet, e, &dossier)?),
        ),
    )?;
    let png = dossier.join("apercu.png");
    // L'écriture de l'auteur vit dans le `.ozalid` : sans ce dépliage, l'aperçu
    // composerait dans la police de repli, et ce serait un aperçu d'autre chose.
    let typst = typst()?;
    let typst = match package::ecrire_polices(&o.projet, &dossier)? {
        Some(d) => typst.avec_polices(d),
        None => typst,
    };
    typst.apercu(&src, &png, 3, 110)?;
    donnee_png(&png)
}

/// Compose un package par envoi, chez le prestataire visé.
///
/// Geste distinct de `packager` : l'un prépare le tirage, l'autre prépare des cadeaux,
/// et les déclencher ensemble composerait des exemplaires que personne n'a demandés.
#[tauri::command]
pub fn envoyer(atelier: State<Atelier>) -> Result<Vec<ResultatEnvoi>, String> {
    let garde = atelier.ouvert.lock().unwrap();
    let o = garde.as_ref().ok_or_else(aucun_projet)?;
    let (pr, papier, d) = vise(o)?;
    let typst = typst()?;
    let racine = sorties_racine(o)?.join("envois");

    let sorties = package::assembler_envois(
        &o.projet,
        pr,
        papier,
        planche::Releve {
            dos: d.dos_mm,
            fond_perdu: d.fond_perdu_mm,
        },
        &racine,
        &typst,
    )?;

    Ok(sorties
        .into_iter()
        .zip(o.projet.meta.envois.liste.iter())
        .map(|((dossier, p), e)| ResultatEnvoi {
            dedicataire: e.dedicataire.clone(),
            dossier,
            // La vignette manquante ne perd pas le package : les PDF sont écrits.
            vignette: donnee_png(Path::new(&p.vignette)).ok(),
            package: p,
        })
        .collect())
}

fn papier(pr: &'static Provider, cle: Option<&str>) -> Result<&'static providers::Papier, String> {
    match cle {
        Some(c) => pr
            .papier(c)
            .ok_or_else(|| format!("papier inconnu chez {} : {c}", pr.cle)),
        None => Ok(pr.papier_defaut()),
    }
}

/// Écrit les images du projet à côté de la source, et rend leurs descriptions.
fn ecrire_images(
    projet: &Projet,
    dossier: &Path,
) -> Result<(Option<Ressource>, Option<Ressource>), String> {
    package::ecrire_images(projet, dossier)
}

/// Racine des sorties : un répertoire du nom du projet, à côté du `.ozalid`, jamais
/// dedans. Un projet non enregistré n'a donc pas d'endroit où écrire — c'est voulu,
/// sinon les sorties atterriraient dans un répertoire temporaire que personne ne
/// retrouve. L'épreuve s'y range directement : elle ne vise aucun éditeur.
fn sorties_racine(o: &Ouvert) -> Result<PathBuf, String> {
    let chemin = o.chemin.as_ref().ok_or_else(|| {
        "enregistrer le projet avant de composer : les sorties se rangent à côté du \
         fichier .ozalid."
            .to_string()
    })?;
    let parent = chemin.parent().unwrap_or(Path::new("."));
    let nom = chemin
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "projet".into());
    Ok(parent.join(nom))
}

/// Sorties d'un prestataire : un répertoire par prestataire, sous la racine.
fn sorties_dossier(o: &Ouvert, provider: &str) -> Result<PathBuf, String> {
    Ok(sorties_racine(o)?.join(provider))
}

/// Répertoire de configuration de l'application, s'il est atteignable.
fn config(app: &tauri::AppHandle) -> Option<PathBuf> {
    app.path().app_config_dir().ok()
}

/// Mémorise un projet dans les récents.
///
/// **Au mieux** : un échec s'écrit sur la sortie d'erreur, visible en développement,
/// invisible pour qui lance le binaire empaqueté. C'est assumé : ce qui se perd ici
/// est une liste de raccourcis, pas un livre, et faire remonter cet échec jusqu'à
/// l'interface coûterait plus qu'il ne vaut.
fn memoriser(app: &tauri::AppHandle, chemin: &Path) {
    let Some(dir) = config(app) else {
        eprintln!("préférences : répertoire de configuration introuvable, récents non mémorisés.");
        return;
    };
    let mut p = preferences::charger(&dir);
    p.ajouter_recent(chemin);
    if let Err(e) = preferences::enregistrer(&dir, &p) {
        eprintln!("préférences : {e}");
        return;
    }
    // Le sous-menu des récents vient d'être périmé par cette écriture : le
    // reconstruire ici évite d'avoir à s'en souvenir à chaque point d'appel.
    if let Err(e) = crate::menu::poser(app) {
        eprintln!("menu : reconstruction impossible : {e}");
    }
}

fn poser(
    atelier: &State<Atelier>,
    chemin: Option<PathBuf>,
    projet: Projet,
    modifie: bool,
) -> Result<ProjetVue, String> {
    let mut garde = atelier.ouvert.lock().unwrap();
    *garde = Some(Ouvert {
        chemin,
        projet,
        modifie,
        candidat: None,
    });
    vue(garde.as_ref().unwrap())
}

fn vue(o: &Ouvert) -> Result<ProjetVue, String> {
    // Le compte de chapitres affiché est celui du manuscrit embarqué, pas celui que le
    // projet déclare : c'est l'écart entre les deux qui signale un manuscrit périmé.
    let chapitres_trouves = manuscrit::decoupe(&o.projet.texte, None)
        .map(|c| c.len() as u32)
        .unwrap_or(0);
    Ok(ProjetVue {
        chemin: o.chemin.as_ref().map(|c| c.to_string_lossy().into_owned()),
        livre: o.projet.meta.livre.clone(),
        manuscrit_source: o.projet.meta.manuscrit.source.clone(),
        chapitres_trouves,
        mots: o.projet.texte.split_whitespace().count() as u32,
        manuscrit_absent: o.projet.texte.trim().is_empty(),
        modifie: o.modifie,
        couverture: o.projet.meta.couverture.maquette.clone(),
        couverture_importee: o.projet.meta.couverture.maquette.is_some(),
        images: o.projet.images.keys().cloned().collect(),
        interieur: o.projet.meta.interieur.clone(),
        livraison: o.projet.meta.livraison.clone(),
        envois: o.projet.meta.envois.clone(),
    })
}

/// La vue d'un projet qu'on vient de modifier.
///
/// Deux fonctions plutôt qu'un drapeau posé à la main dans chaque commande : le
/// point d'appel dit ce qu'il a fait, et oublier de le dire se voit à la lecture.
fn vue_modifiee(o: &mut Ouvert) -> Result<ProjetVue, String> {
    o.modifie = true;
    vue(o)
}

/// La vue d'un projet qu'on vient d'écrire sur le disque.
fn vue_enregistree(o: &mut Ouvert) -> Result<ProjetVue, String> {
    o.modifie = false;
    vue(o)
}

/// Écrit le projet à un chemin, et le retient comme le sien.
///
/// Le noyau commun d'« Enregistrer » et d'« Enregistrer sous… » : les deux ne
/// diffèrent que par la façon dont le chemin est trouvé.
fn enregistrer_a(o: &mut Ouvert, chemin: &Path) -> Result<ProjetVue, String> {
    o.projet.enregistrer(chemin)?;
    o.chemin = Some(chemin.to_path_buf());
    vue_enregistree(o)
}

fn aucun_projet() -> String {
    "aucun projet ouvert.".to_string()
}

fn ecrire(chemin: &Path, contenu: &str) -> Result<(), String> {
    std::fs::write(chemin, contenu)
        .map_err(|e| format!("écriture impossible ({}) : {e}", chemin.display()))
}

/// Binaire Typst à utiliser.
///
/// En release, seul le sidecar embarqué fait foi : se rabattre sur un Typst du système
/// rendrait la pagination dépendante de la machine, exactement ce que l'embarquement
/// doit empêcher. En développement, le Typst du PATH est accepté pour ne pas imposer
/// de vendorisation à chaque itération.
fn binaire_typst() -> Result<PathBuf, String> {
    let sidecar = std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(|d| d.join(nom_sidecar())))
        .filter(|p| p.is_file());
    match sidecar {
        Some(p) => Ok(p),
        None if cfg!(debug_assertions) => Ok(PathBuf::from("typst")),
        None => Err("Typst embarqué introuvable : l'application est mal empaquetée.".into()),
    }
}

/// Typst prêt à composer, polices embarquées comprises.
fn typst() -> Result<Typst, String> {
    let b = binaire_typst()?;
    let voisin = b.parent().map(Path::to_path_buf).unwrap_or_default();
    let candidats = [
        voisin.join("fonts"),
        // Empaquetage macOS : les ressources sont dans Contents/Resources, pas à côté
        // du binaire. Le chemin réel en release se vérifie au jalon 5.
        voisin.join("../Resources/fonts"),
        // Développement : les polices vivent dans les sources.
        Path::new(env!("CARGO_MANIFEST_DIR")).join("fonts"),
    ];
    let dossier = candidats
        .into_iter()
        .find(|p| p.is_dir())
        .ok_or("polices embarquées introuvables : lancer app/outils/polices.sh.")?;
    Ok(Typst::new(b).avec_polices(dossier))
}

fn nom_sidecar() -> &'static str {
    if cfg!(windows) {
        "typst.exe"
    } else {
        "typst"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Les libellés des boutons font foi au retour : le plugin rend le texte du
    /// bouton, pas un `Yes`/`No`. Une comparaison qui dériverait de l'affichage
    /// enverrait « Enregistrer » sur « ignorer », et le travail serait perdu.
    #[test]
    fn la_reponse_de_la_garde_se_lit_par_ses_libelles() {
        use tauri_plugin_dialog::MessageDialogResult as R;
        assert_eq!(reponse_garde(R::Custom(ENREGISTRER.into())), "enregistrer");
        assert_eq!(reponse_garde(R::Custom(IGNORER.into())), "ignorer");
        assert_eq!(reponse_garde(R::Custom(ANNULER.into())), "annuler");
        assert_eq!(reponse_garde(R::Yes), "enregistrer");
        assert_eq!(reponse_garde(R::No), "ignorer");
        assert_eq!(reponse_garde(R::Cancel), "annuler");
        // Fermer la boîte sans choisir ne doit rien perdre.
        assert_eq!(reponse_garde(R::Custom("autre chose".into())), "annuler");
    }

    /// Tauri ne renomme que les *arguments* d'une commande, jamais les champs d'une
    /// struct : le destinataire que l'interface renvoie à `destinataire_regler` voyage
    /// donc en snake_case, comme le `Livre` qu'elle renvoie déjà. Le lire en camelCase
    /// ferait échouer chaque relevé de gabarit saisi, sans que rien ne dise pourquoi.
    #[test]
    fn le_destinataire_de_l_interface_se_lit() {
        let json = r#"{
            "provider": "coollibri-148x210",
            "papier": "mesure",
            "dos_mm": 18.4,
            "fond_perdu_mm": 4
        }"#;
        let d: Destinataire = serde_json::from_str(json).unwrap();
        assert_eq!(d.provider, "coollibri-148x210");
        assert_eq!(d.papier, "mesure");
        assert_eq!(d.dos_mm, Some(18.4));
        assert_eq!(d.fond_perdu_mm, Some(4.0));
    }

    /// Un relevé qu'on n'a pas encore fait est absent, pas nul : le champ vide de
    /// l'interface doit arriver ici comme une absence, faute de quoi la planche se
    /// composerait sur un dos de zéro millimètre.
    #[test]
    fn un_releve_absent_reste_absent() {
        let d: Destinataire =
            serde_json::from_str(r#"{"provider": "lulu", "papier": "standard"}"#).unwrap();
        assert_eq!(d.dos_mm, None);
        assert_eq!(d.fond_perdu_mm, None);
    }

    /// Choisir l'image d'une face remplace celle qui s'y composait, quel que soit le
    /// nom qu'elle portait — un projet importé nomme ses photos comme il l'entend — et
    /// laisse l'autre face intacte.
    #[test]
    fn une_face_ne_garde_qu_une_image() {
        let mut images = BTreeMap::from([
            ("photo.jpg".to_string(), vec![1]),
            ("quatrieme.jpg".to_string(), vec![2]),
        ]);

        poser_image(&mut images, "couverture.png".into(), vec![3]);
        assert_eq!(
            images.keys().collect::<Vec<_>>(),
            ["couverture.png", "quatrieme.jpg"],
            "l'image de 1ère n'a pas été remplacée, ou la 4ème a été emportée"
        );

        poser_image(&mut images, "quatrieme.png".into(), vec![4]);
        assert_eq!(
            images.keys().collect::<Vec<_>>(),
            ["couverture.png", "quatrieme.png"]
        );
    }

    /// Le nom porte le rôle : c'est tout ce que la composition lit pour savoir quelle
    /// face une image sert.
    #[test]
    fn le_nom_d_une_image_dit_la_face_qu_elle_sert() {
        assert_eq!(nom_image("une", "jpg").unwrap(), "couverture.jpg");
        assert_eq!(nom_image("quatre", "png").unwrap(), "quatrieme.png");
        assert!(package::sert_la_quatrieme(
            &nom_image("quatre", "png").unwrap()
        ));
        assert!(!package::sert_la_quatrieme(
            &nom_image("une", "png").unwrap()
        ));
        assert!(nom_image("planche", "png").is_err());
    }

    /// La clé du modèle est en clair dans `preferences.toml`, avec les permissions du
    /// fichier : c'est un choix, et il ne tient que si elle ne va nulle part ailleurs.
    /// Ce test tombe le jour où quelqu'un ajoute le champ à la vue — c'est-à-dire le
    /// jour où la clé entrerait dans une page, donc dans une capture d'écran.
    #[test]
    fn la_vue_de_l_acces_au_modele_ne_porte_pas_la_cle() {
        let v = AccesVue {
            url: "https://exemple.test/images".into(),
            cle_posee: true,
        };
        let json = serde_json::to_string(&v).unwrap();
        assert_eq!(json.matches("cle").count(), 1, "un second « cle » : {json}");
        assert!(json.contains("cle_posee"), "{json}");
    }

    fn ouvert_neuf() -> Ouvert {
        Ouvert {
            chemin: None,
            projet: Projet::nouveau(Livre::vide(), String::new()),
            modifie: false,
            candidat: None,
        }
    }

    /// Le drapeau est ce qui décide si fermer l'application perd du travail. Il ne
    /// doit se lever que par une mutation, et retomber par une écriture — jamais
    /// par une simple relecture du projet.
    #[test]
    fn le_drapeau_de_modification_suit_les_mutations_et_les_ecritures() {
        let mut o = ouvert_neuf();
        assert!(
            !vue(&o).unwrap().modifie,
            "un projet neuf n'est pas modifié"
        );
        assert!(!vue(&o).unwrap().modifie, "relire ne modifie pas");

        assert!(vue_modifiee(&mut o).unwrap().modifie);
        assert!(vue(&o).unwrap().modifie, "le drapeau reste levé");

        assert!(!vue_enregistree(&mut o).unwrap().modifie);
    }

    /// Un manuscrit absent et un manuscrit sans chapitre composable rendent tous
    /// deux zéro chapitre. L'interface doit pouvoir dire « aucun manuscrit » plutôt
    /// que « 0 chapitre » : ce n'est pas la même chose à corriger.
    #[test]
    fn un_manuscrit_vide_se_declare_absent_et_non_vide_de_chapitres() {
        let vide = ouvert_neuf();
        let v = vue(&vide).unwrap();
        assert!(v.manuscrit_absent);
        assert_eq!(v.chapitres_trouves, 0);

        let mut plein = ouvert_neuf();
        plein.projet.texte = "## 01 - Un\n\nTexte.\n".into();
        let v = vue(&plein).unwrap();
        assert!(!v.manuscrit_absent);
        assert_eq!(v.chapitres_trouves, 1);

        // Du texte qui ne porte aucun « ## » : présent, mais sans chapitre.
        let mut sans_chapitre = ouvert_neuf();
        sans_chapitre.projet.texte = "juste une phrase\n".into();
        let v = vue(&sans_chapitre).unwrap();
        assert!(!v.manuscrit_absent, "présent, même s'il ne compose pas");
        assert_eq!(v.chapitres_trouves, 0);

        // Des espaces et des sauts de ligne ne sont pas un manuscrit : c'est ce que
        // `trim` établit, et rien ne le dirait si on le retirait.
        let mut blancs = ouvert_neuf();
        blancs.projet.texte = "  \n\n\t \n".into();
        assert!(vue(&blancs).unwrap().manuscrit_absent);
    }

    /// Écrire, c'est aussi retenir où : un « Enregistrer » suivant doit réécrire au
    /// même endroit sans rien redemander.
    #[test]
    fn enregistrer_retient_le_chemin_ecrit() {
        let dir = tempfile::tempdir().unwrap();
        let chemin = dir.path().join("livre.ozalid");
        let mut o = ouvert_neuf();
        o.modifie = true;

        let v = enregistrer_a(&mut o, &chemin).unwrap();
        assert!(!v.modifie, "le drapeau retombe à l'écriture");
        assert_eq!(o.chemin.as_deref(), Some(chemin.as_path()));
        assert!(chemin.is_file(), "l'archive est bien sur le disque");
        // Relire, et pas seulement écrire : un projet neuf porte un manuscrit vide,
        // et l'archive doit tout de même le contenir pour être relisible.
        assert_eq!(Projet::ouvrir(&chemin).unwrap().texte, "");
    }

    /// Une écriture refusée ne doit ni faire retomber le drapeau, ni faire croire que
    /// le projet a changé d'adresse. C'est le cas où l'on croirait avoir sauvegardé.
    #[test]
    fn une_ecriture_refusee_ne_deplace_ni_le_projet_ni_le_drapeau() {
        let dir = tempfile::tempdir().unwrap();
        // Un répertoire existant ne peut pas être ouvert en création de fichier :
        // c'est un échec d'écriture qui n'exige ni permission ni disque plein.
        let impossible = dir.path().join("sous-repertoire");
        std::fs::create_dir(&impossible).unwrap();

        let ancien = dir.path().join("ancien.ozalid");
        let mut o = ouvert_neuf();
        o.modifie = true;
        o.chemin = Some(ancien.clone());

        assert!(enregistrer_a(&mut o, &impossible).is_err());
        assert!(o.modifie, "le drapeau reste levé");
        assert_eq!(o.chemin.as_deref(), Some(ancien.as_path()));
    }

    /// Le genre par défaut ne doit vivre qu'à un endroit : un projet neuf et un
    /// projet relu d'un TOML sans genre doivent porter le même.
    #[test]
    fn un_livre_vide_prend_le_genre_par_defaut() {
        let l = Livre::vide();
        assert_eq!(l.genre, "roman");
        assert!(l.titre.is_empty());
        assert!(l.auteur.is_empty());
        assert_eq!(l.chapitres, None);
        assert_eq!(l.titre_page, None);
    }
}
