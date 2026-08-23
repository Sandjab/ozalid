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
    let titre_page = livre.titre_page();
    let dedicace = livre.dedicace();
    let livre_epub = epub::Livre {
        titre: &livre.titre,
        titre_page: &titre_page,
        auteur: &livre.auteur,
        genre: &livre.genre,
        copyright: &livre.copyright,
        dedicace: dedicace.as_deref(),
    };
    // Les refus de l'archive ne dépendent que du projet : les poser ici, c'est les
    // rendre avant la composition plutôt qu'après. `epub::archive` les repose de toute
    // façon — voir `epub::verifie`.
    epub::verifie(&livre_epub, &chapitres)?;

    std::fs::create_dir_all(dossier)
        .map_err(|e| format!("répertoire inutilisable ({}) : {e}", dossier.display()))?;

    // Les deux livrables de la génération précédente s'en vont avant qu'on écrive le
    // premier octet de celle-ci : une panne en cours de route laisse alors un manque,
    // qui se voit, au lieu d'un PDF neuf à côté d'un EPUB périmé, qui ne se voit pas.
    let base = nom_de_fichier(&livre.titre);
    let pdf = dossier.join(format!("{base}.pdf"));
    let fichier_epub = dossier.join(format!("{base}.epub"));
    efface(&pdf)?;
    efface(&fichier_epub)?;

    let (une, _) = package::ecrire_images(projet, dossier)?;

    // 1. Le PDF : la couverture en page 1, puis l'intérieur sans son imposition.
    let src = dossier.join("ebook.typ");
    let page = couverture::page_une(livre, cv, pr.format, une.as_ref(), dos_mm);
    ecrire(
        &src,
        &interieur::source_ebook(livre, int, pr, &chapitres, &page),
    )?;
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
        &livre_epub,
        &chapitres,
        &octets_png,
        polices.as_ref(),
        &epub::horodatage(std::time::SystemTime::now()),
    )?;
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
///
/// Les octets retenus sont **ceux qu'on vient de lire**, jamais relus. Relire, c'était
/// ouvrir deux fois les 10 Mo et 32 fichiers de `fonts/` à chaque génération, et surtout
/// se donner deux échecs muets : une seconde lecture ratée sur l'italique le faisait
/// disparaître de l'EPUB en laissant le compte rendu dire que tout allait bien, et sur le
/// romain elle annonçait « famille introuvable » alors qu'elle venait d'être trouvée.
fn polices_du_livre(famille: &str, dossiers: &[std::path::PathBuf]) -> Option<epub::Polices> {
    let mut trouves: Vec<(String, Vec<u8>)> = Vec::new();
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
                    trouves.push((nom.to_string(), octets));
                }
            }
        }
    }
    let noms: Vec<String> = trouves.iter().map(|(n, _)| n.clone()).collect();
    let faces = epub::faces(&noms)?;
    let prendre = |nom: &str| -> Option<epub::Face> {
        let (_, octets) = trouves.iter().find(|(n, _)| n == nom)?;
        Some(epub::Face {
            nom: nom.to_string(),
            octets: octets.clone(),
        })
    };
    Some(epub::Polices {
        famille: famille.to_string(),
        romain: prendre(&faces.romain)?,
        italique: faces.italique.as_deref().and_then(prendre),
    })
}

/// Retire un livrable de la génération précédente.
///
/// L'absence n'est pas une erreur : c'est le cas ordinaire, celui de la première
/// génération, et refuser là remplacerait un problème par un autre. Tout autre échec, en
/// revanche, refuse — plutôt que de passer outre en silence. Un fichier qui résiste à la
/// suppression est exactement celui qu'une panne laisserait en place, périmé, sous le nom
/// du livre : le couple dépareillé que cette suppression existe pour empêcher. Il
/// résisterait au demeurant tout autant à l'écriture, mais vingt secondes plus tard et
/// sous un message qui ne dirait pas d'où vient le blocage.
fn efface(chemin: &Path) -> Result<(), String> {
    match std::fs::remove_file(chemin) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(format!(
            "le livrable précédent ne s'efface pas ({}) : {e}",
            chemin.display()
        )),
        _ => Ok(()),
    }
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
    use crate::projet::{Livre, Projet};
    use crate::providers::provider;

    /// Un projet qui passe tous les refus : une maquette, la police de labeur par
    /// défaut, un manuscrit d'un chapitre. Les tests ci-dessous le cassent chacun d'un
    /// endroit, pour éprouver un refus à la fois.
    fn projet_temoin() -> Projet {
        let mut p = Projet::nouveau(
            Livre {
                titre: "Les Heures creuses".into(),
                auteur: "Ivan Pjig".into(),
                ..Livre::vide()
            },
            "## 1 - Le seuil\n\nPremier.\n".into(),
        );
        p.meta.couverture.maquette = crate::maquettes::par_cle("folio");
        p
    }

    /// Toute panne entre l'écriture du PDF et celle de l'EPUB laissait le PDF de la
    /// génération courante à côté de l'EPUB de la précédente : deux fichiers au bon nom,
    /// rien pour les distinguer, et c'est le périmé qu'on enverrait à un lecteur. Les
    /// deux cibles sont donc retirées avant la première écriture — une panne laisse alors
    /// un manque, qui se voit.
    #[test]
    fn une_panne_ne_laisse_pas_le_livrable_de_la_fois_d_avant() {
        let d = tempfile::tempdir().unwrap();
        let pdf = d.path().join("Les Heures creuses.pdf");
        let epub = d.path().join("Les Heures creuses.epub");
        std::fs::write(&pdf, b"le PDF d'avant").unwrap();
        std::fs::write(&epub, b"l'EPUB d'avant").unwrap();

        // Une image dont les dimensions sont illisibles fait échouer
        // `package::ecrire_images`, juste après la suppression : c'est la panne la plus
        // précoce qu'on puisse provoquer sans lancer Typst.
        let mut p = projet_temoin();
        p.images
            .insert("couverture.png".into(), b"pas une image".to_vec());

        let err = generer(
            &p,
            provider("lulu").unwrap(),
            None,
            d.path(),
            &Typst::new("typst-qui-n-existe-pas"),
        )
        .unwrap_err();
        assert!(err.contains("couverture.png"), "{err}");
        assert!(!pdf.exists(), "le PDF de la fois d'avant est resté");
        assert!(!epub.exists(), "l'EPUB de la fois d'avant est resté");
    }

    /// Les trois refus de l'archive ne dépendent que du projet. Les laisser tomber dans
    /// `epub::archive` faisait payer la composition entière — vingt secondes et un PDF
    /// neuf sous un message d'échec — pour un défaut connu avant la première écriture.
    #[test]
    fn un_manuscrit_que_l_epub_refuse_est_refuse_avant_toute_ecriture() {
        let d = tempfile::tempdir().unwrap();
        let sortie = d.path().join("ebook");
        let mut p = projet_temoin();
        p.texte = "## 1 - Le seuil\n\nUn saut\u{c} de page.\n".into();

        let err = generer(
            &p,
            provider("lulu").unwrap(),
            None,
            &sortie,
            // Un binaire qui n'existe pas : le refus doit tomber avant qu'on l'appelle.
            &Typst::new("typst-qui-n-existe-pas"),
        )
        .unwrap_err();
        assert!(err.contains("U+000C"), "{err}");
        assert!(
            !sortie.exists(),
            "le répertoire de sortie a été créé malgré le refus"
        );
    }

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
