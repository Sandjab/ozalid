//! Les ebooks locaux : le PDF et l'EPUB, écrits à côté du projet.
//!
//! Ce module est aux sorties locales ce que `package` est aux prestataires : il
//! traverse la chaîne, il ne compose rien lui-même. Le PDF vient d'`interieur`, la
//! couverture de `couverture`, l'archive d'`epub` — et Typst fait les deux rendus.
//!
//! L'ebook ne mesure pas sa pagination : il n'a pas de dos à calculer. Sa génération
//! est donc une compilation, là où un package en enchaîne plusieurs.

use std::path::Path;

use serde::Serialize;

use crate::projet::Projet;
use crate::providers::Provider;
use crate::typst::Typst;
use crate::{couverture, envoi, epub, interieur, manuscrit, package, police};

/// Définition du PNG de couverture embarqué dans l'EPUB, en points par pouce.
///
/// À 250 ppp, une couverture de 170 mm de haut fait environ 1670 pixels — au-dessus du
/// seuil où Kindle et Kobo cessent de recadrer la vignette. Monter davantage alourdit
/// l'archive sans rien gagner à l'écran.
const PPP_COUVERTURE: u32 = 250;

/// Ce que la génération a écrit.
#[derive(Debug, Clone, Serialize)]
pub struct Ebooks {
    pub pdf: String,
    pub epub: String,
    pub octets_pdf: u64,
    pub octets_epub: u64,
    /// Familles que Typst a remplacées par une écriture de repli en composant le PDF.
    /// Vide, tout va bien.
    pub polices_introuvables: Vec<String>,
    /// Renseigné quand la police de l'intérieur n'a pas été trouvée dans les
    /// répertoires de Typst : l'EPUB est alors dans l'écriture du lecteur. Ce n'est pas
    /// une erreur — le livre reste juste, seul son œil change.
    pub police_non_embarquee: Option<String>,
}

/// Nom de fichier des deux sorties, sans extension.
fn nom_de_fichier(titre: &str) -> String {
    envoi::assaini(titre)
}

/// Écrit le PDF et l'EPUB du livre dans `dossier`.
///
/// `dos_mm` vient du destinataire visé : il ne sert qu'au cadrage panoramique de la
/// couverture. Absent, l'image se cadre sur la seule 1ère — ce que fait déjà l'aperçu à
/// l'écran, et ce n'est pas un refus de plus.
pub fn generer(
    projet: &Projet,
    pr: &Provider,
    dos_mm: Option<f64>,
    dossier: &Path,
    typst: &Typst,
) -> Result<Ebooks, String> {
    let livre = &projet.meta.livre;
    let int = &projet.meta.interieur;
    // `interieur::source_ebook` interpole la police sans échappement : la validation
    // est ici, comme dans `package::assembler`.
    int.verifie()?;
    let cv = projet
        .meta
        .couverture
        .maquette
        .as_ref()
        .ok_or("aucune maquette de couverture : en choisir une avant de générer les ebooks.")?;
    let chapitres = manuscrit::decoupe(&projet.texte, livre.chapitres)?;
    std::fs::create_dir_all(dossier)
        .map_err(|e| format!("répertoire inutilisable ({}) : {e}", dossier.display()))?;

    let (une, _) = package::ecrire_images(projet, dossier)?;
    let base = nom_de_fichier(&livre.titre);

    // 1. Le PDF : la couverture en page 1, puis l'intérieur sans son imposition.
    let src = dossier.join("ebook.typ");
    let page = couverture::page_une(livre, cv, pr.format, une.as_ref(), dos_mm);
    ecrire(
        &src,
        &interieur::source_ebook(livre, int, pr, &chapitres, &page),
    )?;
    let pdf = dossier.join(format!("{base}.pdf"));
    let polices_introuvables = typst.compile(&src, &pdf)?;

    // 2. La couverture seule, en PNG, pour l'EPUB. Même source que la page du PDF :
    //    les deux fichiers montrent la même image.
    let src_cv = dossier.join("couverture-ebook.typ");
    ecrire(
        &src_cv,
        &couverture::source_une(livre, cv, pr.format, une.as_ref(), dos_mm),
    )?;
    let png = dossier.join("couverture-ebook.png");
    typst.apercu(&src_cv, &png, 1, PPP_COUVERTURE)?;
    let octets_png = std::fs::read(&png)
        .map_err(|e| format!("couverture illisible ({}) : {e}", png.display()))?;

    // 3. L'écriture du livre, si elle est là.
    let polices = polices_du_livre(&int.police, typst.polices());
    let police_non_embarquee = polices.is_none().then(|| int.police.clone());

    // 4. L'archive.
    let arch = epub::archive(
        &epub::Livre {
            titre: &livre.titre,
            titre_page: livre.titre_page(),
            auteur: &livre.auteur,
            genre: &livre.genre,
            copyright: &livre.copyright,
            dedicace: livre.dedicace(),
        },
        &chapitres,
        &octets_png,
        polices.as_ref(),
        &epub::horodatage(std::time::SystemTime::now()),
    )?;
    let fichier_epub = dossier.join(format!("{base}.epub"));
    std::fs::write(&fichier_epub, &arch)
        .map_err(|e| format!("écriture impossible ({}) : {e}", fichier_epub.display()))?;

    Ok(Ebooks {
        pdf: pdf.to_string_lossy().into_owned(),
        epub: fichier_epub.to_string_lossy().into_owned(),
        octets_pdf: taille(&pdf),
        octets_epub: arch.len() as u64,
        polices_introuvables,
        police_non_embarquee,
    })
}

/// Le romain et l'italique de la police du livre, lus dans les répertoires de Typst.
///
/// `None` si la famille n'y est pas : l'EPUB se fait alors dans l'écriture du lecteur,
/// et le compte rendu le dit. Ce n'est pas une erreur — contrairement à la composition,
/// où une police absente donnerait un livre imprimé faux.
fn polices_du_livre(famille: &str, dossiers: &[std::path::PathBuf]) -> Option<epub::Polices> {
    let mut trouves: Vec<(String, std::path::PathBuf)> = Vec::new();
    for d in dossiers {
        let Ok(entrees) = std::fs::read_dir(d) else {
            continue;
        };
        for e in entrees.flatten() {
            let chemin = e.path();
            let Ok(octets) = std::fs::read(&chemin) else {
                continue;
            };
            // Un fichier qui n'est pas une police, ou qui ne porte pas le français, est
            // simplement ignoré : `police::examine` refuse, et ce refus-là n'a rien à
            // dire ici — il n'y a pas d'envoi en jeu.
            let Ok(p) = police::examine(&octets) else {
                continue;
            };
            if p.famille == famille {
                if let Some(nom) = chemin.file_name().and_then(|n| n.to_str()) {
                    trouves.push((nom.to_string(), chemin.clone()));
                }
            }
        }
    }
    let noms: Vec<String> = trouves.iter().map(|(n, _)| n.clone()).collect();
    let faces = epub::faces(&noms)?;
    let lire = |nom: &str| -> Option<epub::Face> {
        let (_, chemin) = trouves.iter().find(|(n, _)| n == nom)?;
        Some(epub::Face {
            nom: nom.to_string(),
            octets: std::fs::read(chemin).ok()?,
        })
    };
    Some(epub::Polices {
        famille: famille.to_string(),
        romain: lire(&faces.romain)?,
        italique: faces.italique.as_deref().and_then(lire),
    })
}

fn ecrire(chemin: &Path, contenu: &str) -> Result<(), String> {
    std::fs::write(chemin, contenu)
        .map_err(|e| format!("écriture impossible ({}) : {e}", chemin.display()))
}

/// Taille d'un fichier, ou zéro. Un compte rendu qui échouerait parce qu'il n'a pas su
/// lire une taille serait absurde : le fichier, lui, est écrit.
fn taille(chemin: &Path) -> u64 {
    std::fs::metadata(chemin).map(|m| m.len()).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Les deux fichiers portent le nom du livre, assaini comme un répertoire d'envoi :
    /// c'est la fonction du projet qui décide ce qu'un titre devient sur un disque, et
    /// il n'y en a pas deux. Un titre réduit à de la ponctuation ne doit pas donner un
    /// fichier sans nom.
    #[test]
    fn les_fichiers_portent_le_nom_du_livre_assaini() {
        assert_eq!(nom_de_fichier("Les Heures creuses"), "Les Heures creuses");
        assert_eq!(nom_de_fichier("L'été / l'hiver"), "L-été - l-hiver");
        assert_eq!(nom_de_fichier("..."), "envoi");
    }
}
