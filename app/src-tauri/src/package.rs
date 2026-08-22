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

use std::path::{Path, PathBuf};

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
    /// La planche en PNG, à côté du PDF. Elle ne part pas chez l'imprimeur — d'où sa
    /// place hors de `chemins` : c'est de quoi vérifier d'un coup d'œil que la planche
    /// tient, pour ce prestataire-là, avec le dos qu'il a réellement mesuré.
    pub vignette: String,
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
            &interieur::source(livre, int, pr, reglage, &chapitres, None),
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
        &interieur::source(livre, int, pr, &reglage, &chapitres, None),
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

    // 4. La même planche en vignette, depuis la même source : ce qu'on regarde est ce
    // qui part à l'impression, et non une approximation qu'on espère fidèle. 72 ppp
    // suffisent à juger un débord ; c'est le PDF qui fait foi pour le reste.
    let png_pl = dossier.join(nom(pr, "couverture", "png"));
    typst.apercu(&src_pl, &png_pl, 1, 72)?;

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
        vignette: affiche(&png_pl),
    })
}

/// Les noms de répertoire des envois, dans l'ordre de la liste.
///
/// Séparé d'`assembler_envois` pour être éprouvé sans toucher au disque ni à Typst :
/// c'est ici que se joue le fait qu'un exemplaire ne parte pas avec le mot d'un autre.
fn dossiers_d_envoi(envois: &[crate::envoi::Envoi]) -> Vec<String> {
    let mut pris: Vec<String> = Vec::with_capacity(envois.len());
    for e in envois {
        let d = crate::envoi::distinct(&crate::envoi::assaini(&e.dedicataire), &pris);
        pris.push(d);
    }
    pris
}

/// Compose un package par envoi, tous chez le même prestataire.
///
/// **La convergence n'a lieu qu'une fois.** L'envoi se pose par `#place`, qui ne peut
/// pas créer de page : la gouttière, la parité, le compte de pages, le dos et la
/// planche sont donc les mêmes pour tous. Converger M fois ne coûterait pas seulement
/// M fois le temps — cela laisserait croire que le résultat pourrait différer.
pub fn assembler_envois(
    projet: &Projet,
    pr: &Provider,
    papier: &Papier,
    releve: Releve,
    racine: &Path,
    typst: &Typst,
) -> Result<Vec<(String, Package)>, String> {
    let envois = &projet.meta.envois;
    envois.verifie()?;
    if envois.liste.is_empty() {
        return Err("aucun envoi : en écrire un avant de générer.".into());
    }

    // Le package de référence, sans envoi : c'est lui qui converge, calcule le dos et
    // compose la planche. Les envois n'en reprennent que le réglage et les fichiers.
    let reference = racine.join(".reference");
    let base = assembler(projet, pr, papier, releve, &reference, typst)?;

    // La police de l'auteur n'entre en scène qu'ici : le package de référence ne porte
    // aucun envoi, donc aucune écriture manuscrite. Elle est dépliée une fois pour tous
    // les envois, et Typst la cherchera là.
    let typst = &match ecrire_polices(projet, racine)? {
        Some(dossier) => typst.clone().avec_polices(dossier),
        None => typst.clone(),
    };

    let livre = &projet.meta.livre;
    let int = &projet.meta.interieur;
    let chapitres = manuscrit::decoupe(&projet.texte, livre.chapitres)?;
    let reglage = Reglage {
        gouttiere: base.gouttiere,
        blanche: base.blanche,
    };
    let crate::envoi::Main::Police { police } = &envois.main;

    let mut sorties = Vec::with_capacity(envois.liste.len());
    for (e, nom_dossier) in envois.liste.iter().zip(dossiers_d_envoi(&envois.liste)) {
        let dossier = racine.join(&nom_dossier);
        std::fs::create_dir_all(&dossier)
            .map_err(|err| format!("répertoire inutilisable ({}) : {err}", dossier.display()))?;

        let src = dossier.join(nom(pr, "interieur", "typ"));
        ecrire(
            &src,
            &interieur::source(
                livre,
                int,
                pr,
                &reglage,
                &chapitres,
                Some(interieur::Trace {
                    police,
                    texte: &e.contenu,
                }),
            ),
        )?;
        let pdf = dossier.join(nom(pr, "interieur", "pdf"));
        typst.compile(&src, &pdf)?;

        // La planche ne dépend pas de l'envoi : elle est recopiée, pas recomposée.
        let mut p = base.clone();
        p.chemins = vec![
            affiche(&pdf),
            copier(&reference, &dossier, &nom(pr, "couverture", "pdf"))?,
        ];
        p.vignette = copier(&reference, &dossier, &nom(pr, "couverture", "png"))?;
        sorties.push((nom_dossier, p));
    }
    Ok(sorties)
}

/// Recopie un fichier de la référence vers le répertoire d'un envoi, et rend son chemin.
fn copier(depuis: &Path, vers: &Path, fichier: &str) -> Result<String, String> {
    let cible = vers.join(fichier);
    std::fs::copy(depuis.join(fichier), &cible)
        .map_err(|e| format!("{fichier} : copie impossible : {e}"))?;
    Ok(affiche(&cible))
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

/// Déplie la police personnelle du projet, et rend le répertoire où Typst la trouvera.
///
/// Typst ne lit ses polices que dans des répertoires : l'écriture de l'auteur vit dans
/// le `.ozalid`, elle doit donc atterrir sur le disque avant qu'on puisse composer. Un
/// répertoire à part, et non celui des sorties : `--font-path` est fouillé
/// récursivement, et lui donner le répertoire des envois lui ferait ouvrir un à un tous
/// les PDF qu'on vient d'y écrire.
pub fn ecrire_polices(projet: &Projet, dossier: &Path) -> Result<Option<PathBuf>, String> {
    if projet.polices.is_empty() {
        return Ok(None);
    }
    let cible = dossier.join(".polices");
    std::fs::create_dir_all(&cible)
        .map_err(|e| format!("répertoire inutilisable ({}) : {e}", cible.display()))?;
    for (nom, octets) in &projet.polices {
        std::fs::write(cible.join(nom), octets).map_err(|e| format!("{nom} : {e}"))?;
    }
    Ok(Some(cible))
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

    /// Les répertoires d'envoi portent le nom du dédicataire, assaini et rendu unique.
    /// Deux dédicataires qui se confondraient enverraient au second le mot du premier.
    #[test]
    fn les_repertoires_d_envoi_sont_distincts_et_sans_chemin() {
        let envois = [
            crate::envoi::Envoi {
                dedicataire: "Marie/Léa".into(),
                contenu: "A.".into(),
            },
            crate::envoi::Envoi {
                dedicataire: "Marie-Léa".into(),
                contenu: "B.".into(),
            },
            crate::envoi::Envoi {
                dedicataire: "..".into(),
                contenu: "C.".into(),
            },
        ];
        assert_eq!(
            dossiers_d_envoi(&envois),
            vec!["Marie-Léa", "Marie-Léa-2", "envoi"]
        );
    }

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
