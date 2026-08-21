//! Le package d'un prestataire : l'intérieur, la planche, et de quoi les relire.
//!
//! Un livre, N prestataires, aucun réglage retouché entre les deux — c'est la « file
//! d'attente » du COOKBOOK, exécutée. Chaque prestataire coché déclenche sa propre
//! composition : son format, sa gouttière, sa pagination, donc son dos et sa planche.
//!
//! L'ordre des opérations n'est pas négociable : l'intérieur d'abord, parce que c'est
//! lui qui donne la pagination ; le dos ensuite, parce qu'il en découle ; la planche
//! enfin. Inverser reviendrait à ressaisir un nombre de pages à la main, ce que
//! l'application existe pour supprimer.

use std::path::Path;

use serde::Serialize;

use crate::couverture::Ressource;
use crate::interieur::{self, Reglage};
use crate::manuscrit;
use crate::planche::{self, Gabarit, Releve};
use crate::projet::Projet;
use crate::providers::{Papier, Provider};
use crate::typst::Typst;

/// Ce qu'un package contient une fois écrit sur le disque.
#[derive(Debug, Clone, Serialize)]
pub struct Package {
    pub provider: String,
    pub libelle: String,
    pub papier: String,
    pub pages: u32,
    pub gouttiere: f64,
    pub blanche: bool,
    pub dos: f64,
    pub fond_perdu: f64,
    /// Dimensions de la planche, en mm.
    pub planche: (f64, f64),
    pub chemins: Vec<String>,
}

/// Nom de fichier des sorties d'un prestataire. Le nom porte la clé du prestataire :
/// deux packages ouverts côte à côte ne peuvent pas être confondus.
fn nom(pr: &Provider, quoi: &str, ext: &str) -> String {
    format!("{quoi}-{}.{ext}", pr.cle)
}

/// Compose l'intérieur, en tire la pagination, puis la planche, et écrit le tout dans
/// `dossier`.
///
/// Le `releve` ne sert que chez les prestataires qui ne publient ni dos ni fond perdu ;
/// ailleurs, il est ignoré au profit de leur formule.
pub fn assembler(
    projet: &Projet,
    pr: &Provider,
    papier: &Papier,
    releve: Releve,
    dossier: &Path,
    typst: &Typst,
) -> Result<Package, String> {
    let int = &projet.meta.interieur;
    // `interieur::source` interpole la police sans échappement : la validation est ici.
    int.verifie()?;
    std::fs::create_dir_all(dossier)
        .map_err(|e| format!("répertoire inutilisable ({}) : {e}", dossier.display()))?;

    let livre = &projet.meta.livre;
    let chapitres = manuscrit::decoupe(&projet.texte, livre.chapitres)?;

    // 1. L'intérieur, et la pagination qui en sort.
    let src_int = dossier.join(nom(pr, "interieur", "typ"));
    let r = interieur::converge(pr, |reglage| {
        ecrire(
            &src_int,
            &interieur::source(livre, int, pr, reglage, &chapitres),
        )?;
        typst.pages(&src_int)
    })?;
    if r.pages < pr.pages_min || r.pages > pr.pages_max {
        return Err(format!(
            "{} : {} pages, hors des {} à {} que {} accepte en dos carré collé.",
            pr.cle, r.pages, pr.pages_min, pr.pages_max, pr.libelle
        ));
    }
    let reglage = Reglage {
        gouttiere: r.gouttiere,
        blanche: r.blanche,
    };
    ecrire(
        &src_int,
        &interieur::source(livre, int, pr, &reglage, &chapitres),
    )?;
    let pdf_int = dossier.join(nom(pr, "interieur", "pdf"));
    typst.compile(&src_int, &pdf_int)?;

    // 2. Le dos découle de cette pagination-là, jamais d'une saisie.
    let g = Gabarit::pour(pr, papier, r.pages, releve)?;

    // 3. La planche.
    let cv = projet
        .meta
        .couverture
        .maquette
        .as_ref()
        .ok_or("aucune maquette de couverture : en choisir une avant de packager.")?;
    let (une, quatre) = ecrire_images(projet, dossier)?;
    let src_pl = dossier.join(nom(pr, "couverture", "typ"));
    ecrire(
        &src_pl,
        &planche::source(livre, cv, &g, une.as_ref(), quatre.as_ref())?,
    )?;
    let pdf_pl = dossier.join(nom(pr, "couverture", "pdf"));
    typst.compile(&src_pl, &pdf_pl)?;

    Ok(Package {
        provider: pr.cle.into(),
        libelle: pr.libelle.into(),
        papier: papier.libelle.into(),
        pages: r.pages,
        gouttiere: r.gouttiere,
        blanche: r.blanche,
        dos: g.dos,
        fond_perdu: g.fond_perdu,
        planche: (g.largeur(), g.hauteur()),
        chemins: vec![affiche(&pdf_int), affiche(&pdf_pl)],
    })
}

/// Quelle face une image sert : c'est son nom qui le dit, et rien d'autre.
///
/// Le projet embarque ses images à plat, sans champ qui leur donnerait un rôle : la
/// convention de nom est donc la seule règle, et elle vaut aussi bien pour l'image
/// importée d'un ancien répertoire de travail que pour celle qu'on choisit dans
/// l'application.
pub fn sert_la_quatrieme(nom: &str) -> bool {
    nom.starts_with("quatrieme")
}

/// Écrit les images du projet à côté des sources, et rend leurs descriptions.
/// Typst lit ses images par chemin relatif, comme n'importe quel document.
pub fn ecrire_images(
    projet: &Projet,
    dossier: &Path,
) -> Result<(Option<Ressource>, Option<Ressource>), String> {
    let (mut une, mut quatre) = (None, None);
    for (nom, octets) in &projet.images {
        std::fs::write(dossier.join(nom), octets).map_err(|e| format!("{nom} : {e}"))?;
        let r = Ressource::depuis(nom, octets)
            .ok_or_else(|| format!("{nom} : dimensions illisibles (ni PNG ni JPEG)."))?;
        if sert_la_quatrieme(nom) {
            quatre = Some(r);
        } else {
            une = Some(r);
        }
    }
    Ok((une, quatre))
}

fn affiche(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}

fn ecrire(chemin: &Path, contenu: &str) -> Result<(), String> {
    std::fs::write(chemin, contenu)
        .map_err(|e| format!("écriture impossible ({}) : {e}", chemin.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::provider;

    /// Les deux sorties d'un package portent la clé du prestataire : dans un répertoire
    /// où plusieurs packages ont été produits, un fichier ne peut pas être remis au
    /// mauvais imprimeur.
    #[test]
    fn les_sorties_portent_la_cle_du_prestataire() {
        let pr = provider("bookvault-127x203").unwrap();
        assert_eq!(
            nom(pr, "couverture", "pdf"),
            "couverture-bookvault-127x203.pdf"
        );
        assert_eq!(
            nom(pr, "interieur", "typ"),
            "interieur-bookvault-127x203.typ"
        );
    }
}
