//! Commandes exposées à l'interface. Aucune logique métier ici : elles orchestrent
//! les modules, tiennent le projet ouvert et traduisent les erreurs en messages
//! affichables.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::Serialize;
use tauri::State;

use crate::import;
use crate::interieur::{self, Reglage};
use crate::manuscrit;
use crate::projet::{Livre, Projet};
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
}

/// Vue d'un prestataire pour l'interface.
#[derive(Serialize)]
pub struct ProviderVue {
    cle: String,
    libelle: String,
    largeur: f64,
    hauteur: f64,
    fond_perdu: Option<f64>,
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
    /// Réglages de couverture repris de l'atelier, en attente du moteur Typst.
    pub couverture_importee: bool,
    pub images: Vec<String>,
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
    poser(&atelier, None, projet)
}

#[tauri::command]
pub fn projet_ouvrir(chemin: String, atelier: State<Atelier>) -> Result<ProjetVue, String> {
    let c = PathBuf::from(&chemin);
    let projet = Projet::ouvrir(&c)?;
    poser(&atelier, Some(c), projet)
}

#[tauri::command]
pub fn projet_enregistrer(chemin: String, atelier: State<Atelier>) -> Result<ProjetVue, String> {
    let mut garde = atelier.ouvert.lock().unwrap();
    let o = garde.as_mut().ok_or_else(aucun_projet)?;
    let c = PathBuf::from(&chemin);
    o.projet.enregistrer(&c)?;
    o.chemin = Some(c);
    vue(o)
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
    vue(o)
}

/// Remplace le manuscrit par un fichier choisi, et mémorise son chemin.
#[tauri::command]
pub fn manuscrit_choisir(chemin: String, atelier: State<Atelier>) -> Result<ProjetVue, String> {
    let mut garde = atelier.ouvert.lock().unwrap();
    let o = garde.as_mut().ok_or_else(aucun_projet)?;
    o.projet.texte =
        std::fs::read_to_string(&chemin).map_err(|e| format!("manuscrit illisible : {e}"))?;
    o.projet.meta.manuscrit.source = Some(chemin);
    vue(o)
}

#[tauri::command]
pub fn livre_modifier(livre: Livre, atelier: State<Atelier>) -> Result<ProjetVue, String> {
    let mut garde = atelier.ouvert.lock().unwrap();
    let o = garde.as_mut().ok_or_else(aucun_projet)?;
    o.projet.meta.livre = livre;
    vue(o)
}

/// Compose l'intérieur du projet ouvert et rend le compte de pages avec le dos qui
/// en découle.
#[tauri::command]
pub fn composer(
    provider_cle: String,
    papier_cle: Option<String>,
    atelier: State<Atelier>,
) -> Result<Composition, String> {
    let garde = atelier.ouvert.lock().unwrap();
    let o = garde.as_ref().ok_or_else(aucun_projet)?;

    let pr = providers::provider(&provider_cle)
        .ok_or_else(|| format!("prestataire inconnu : {provider_cle}"))?;
    let papier = match papier_cle.as_deref() {
        Some(c) => pr
            .papier(c)
            .ok_or_else(|| format!("papier inconnu chez {} : {c}", pr.cle))?,
        None => pr.papier_defaut(),
    };

    let livre = &o.projet.meta.livre;
    let chapitres = manuscrit::decoupe(&o.projet.texte, livre.chapitres)?;

    let dossier = sorties(o, pr.cle)?;
    std::fs::create_dir_all(&dossier).map_err(|e| {
        format!(
            "répertoire de sortie inutilisable ({}) : {e}",
            dossier.display()
        )
    })?;

    let typst = Typst::new(binaire_typst()?);
    let src = dossier.join(format!("interieur-{}.typ", pr.cle));

    // La convergence ne mesure que le compte de pages : aucun PDF n'est produit tant
    // que le réglage n'est pas stable.
    let r = interieur::converge(pr, |reglage| {
        ecrire(&src, &interieur::source(livre, pr, reglage, &chapitres))?;
        typst.pages(&src)
    })?;

    let reglage = Reglage {
        gouttiere: r.gouttiere,
        blanche: r.blanche,
    };
    ecrire(&src, &interieur::source(livre, pr, &reglage, &chapitres))?;
    let pdf = dossier.join(format!("interieur-{}.pdf", pr.cle));
    typst.compile(&src, &pdf)?;

    Ok(Composition {
        pages: r.pages,
        gouttiere: r.gouttiere,
        blanche: r.blanche,
        chapitres: chapitres.len() as u32,
        dos: papier.dos.mm(r.pages),
        pdf: pdf.to_string_lossy().into_owned(),
    })
}

/// Répertoire des sorties d'un prestataire : à côté du `.ozalid`, jamais dedans.
/// Un projet non enregistré n'a donc pas d'endroit où écrire — c'est voulu, sinon les
/// sorties atterriraient dans un répertoire temporaire que personne ne retrouve.
fn sorties(o: &Ouvert, provider: &str) -> Result<PathBuf, String> {
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
    Ok(parent.join(nom).join(provider))
}

fn poser(
    atelier: &State<Atelier>,
    chemin: Option<PathBuf>,
    projet: Projet,
) -> Result<ProjetVue, String> {
    let mut garde = atelier.ouvert.lock().unwrap();
    *garde = Some(Ouvert { chemin, projet });
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
        couverture_importee: o.projet.meta.couverture.atelier.is_some(),
        images: o.projet.images.keys().cloned().collect(),
    })
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

fn nom_sidecar() -> &'static str {
    if cfg!(windows) {
        "typst.exe"
    } else {
        "typst"
    }
}
