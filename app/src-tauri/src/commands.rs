//! Commandes exposées à l'interface. Aucune logique métier ici : elles orchestrent
//! les modules et traduisent leurs erreurs en messages affichables.

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::interieur::{self, Reglage};
use crate::manuscrit;
use crate::projet::Livre;
use crate::providers::{self, Provider};
use crate::typst::Typst;

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

/// Compose l'intérieur et rend le compte de pages avec le dos qui en découle.
#[tauri::command]
pub fn composer(
    manuscrit_path: String,
    livre: Livre,
    provider_cle: String,
    papier_cle: Option<String>,
    sortie: String,
) -> Result<Composition, String> {
    let pr = providers::provider(&provider_cle)
        .ok_or_else(|| format!("prestataire inconnu : {provider_cle}"))?;
    let papier = match papier_cle.as_deref() {
        Some(c) => pr
            .papier(c)
            .ok_or_else(|| format!("papier inconnu chez {} : {c}", pr.cle))?,
        None => pr.papier_defaut(),
    };

    let md = std::fs::read_to_string(&manuscrit_path)
        .map_err(|e| format!("manuscrit illisible ({manuscrit_path}) : {e}"))?;
    let chapitres = manuscrit::decoupe(&md, livre.chapitres)?;

    let typst = Typst::new(binaire_typst()?);
    let dossier = PathBuf::from(&sortie);
    std::fs::create_dir_all(&dossier)
        .map_err(|e| format!("répertoire de sortie inutilisable ({sortie}) : {e}"))?;
    let src = dossier.join(format!("interieur-{}.typ", pr.cle));

    // La convergence ne mesure que le compte de pages : aucun PDF n'est produit tant
    // que le réglage n'est pas stable.
    let r = interieur::converge(pr, |reglage| {
        ecrire(&src, &interieur::source(&livre, pr, reglage, &chapitres))?;
        typst.pages(&src)
    })?;

    let reglage = Reglage {
        gouttiere: r.gouttiere,
        blanche: r.blanche,
    };
    ecrire(&src, &interieur::source(&livre, pr, &reglage, &chapitres))?;
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
