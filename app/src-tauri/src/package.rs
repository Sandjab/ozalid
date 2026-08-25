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
    /// Épaisseur que le texte du dos réclame, **quand elle dépasse `dos`** : le titre
    /// part rogné au pli sur ce PDF-là. `None`, il tient. C'est la seule chose qu'une
    /// maquette unique pour N formats casse au lieu de simplement la déplacer — le
    /// corps du dos suit la largeur de couverture, son épaisseur suit la pagination.
    pub dos_requis: Option<f64>,
    pub fond_perdu: f64,
    /// Dimensions de la planche, en mm.
    pub planche: (f64, f64),
    pub chemins: Vec<String>,
    /// La planche en PNG, à côté du PDF. Elle ne part pas chez l'imprimeur — d'où sa
    /// place hors de `chemins` : c'est de quoi vérifier d'un coup d'œil que la planche
    /// tient, pour ce prestataire-là, avec le dos qu'il a réellement mesuré.
    pub vignette: String,
    /// Familles que Typst n'a pas trouvées et a remplacées par une écriture de repli
    /// — sans échouer, donc sans que rien d'autre ne le dise. Vide, tout va bien.
    pub polices_introuvables: Vec<String>,
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
    let mut polices_introuvables = typst.compile(&src_int, &pdf_int)?;

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
    // La planche a ses propres polices : ses substitutions s'ajoutent à celles de
    // l'intérieur, chaque famille une fois.
    for f in typst.compile(&src_pl, &pdf_pl)? {
        if !polices_introuvables.contains(&f) {
            polices_introuvables.push(f);
        }
    }

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
        dos_requis: planche::dos_insuffisant(livre, cv, g.format.0, g.dos),
        fond_perdu: g.fond_perdu,
        planche: (g.largeur(), g.hauteur()),
        chemins: vec![affiche(&pdf_int), affiche(&pdf_pl)],
        vignette: affiche(&png_pl),
        polices_introuvables,
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

/// Ce qu'un envoi dépose sur sa page, et où il s'y pose : l'image est écrite au passage
/// à côté de la source qui la nommera.
///
/// Écrire l'image ici, et non dans un balayage préalable, garantit qu'aucune image ne
/// se retrouve dans le répertoire d'un autre dédicataire : elle est déposée là où sa
/// source est composée, et elle n'est nommée que par elle.
pub fn trace<'a>(
    projet: &'a Projet,
    e: &'a crate::envoi::Envoi,
    dossier: &Path,
) -> Result<interieur::Trace<'a>, String> {
    let qui = if e.dedicataire.trim().is_empty() {
        "cet envoi"
    } else {
        &e.dedicataire
    };
    let quoi = match &e.main {
        crate::envoi::Main::Police { police } => interieur::Quoi::Texte {
            police,
            texte: &e.contenu,
        },
        // Générée ou écrite à la main, une image est une image : elle a été acceptée,
        // elle est dans l'archive, et composer ne rappelle jamais le réseau.
        crate::envoi::Main::Image | crate::envoi::Main::Diffusion => {
            let fichier = e
                .image
                .as_deref()
                .ok_or_else(|| format!("{qui} n'a pas d'image : en choisir une."))?;
            let octets = projet.images_envois.get(fichier).ok_or_else(|| {
                format!("{qui} : l'image « {fichier} » ne figure pas dans le projet.")
            })?;
            // Détouré ici et nulle part ailleurs : `trace` est le seul chemin par où
            // passent la composition d'un package et le rendu de l'objet du canevas.
            // L'écran ne peut donc pas montrer autre chose que ce qui s'imprime.
            //
            // Le nom passe en `.png` : Typst reconnaît le format d'une image à son
            // extension, et un PNG rangé sous `.jpg` ne se composerait pas.
            let (nom, octets) = match &e.detourage {
                Some(d) => {
                    let png = crate::detourage::applique(octets, d)
                        .map_err(|err| format!("{qui} : {err}"))?;
                    let tige = fichier.rsplit_once('.').map_or(fichier, |(t, _)| t);
                    (format!("{tige}.png"), std::borrow::Cow::Owned(png))
                }
                None => (
                    fichier.to_string(),
                    std::borrow::Cow::Borrowed(octets.as_slice()),
                ),
            };
            std::fs::write(dossier.join(&nom), &*octets)
                .map_err(|err| format!("{nom} : écriture impossible : {err}"))?;
            interieur::Quoi::Image {
                fichier: nom.into(),
            }
        }
    };
    Ok(interieur::Trace {
        quoi,
        place: &e.place,
    })
}

/// Refuse un envoi placé sur une page que l'intérieur de ce prestataire n'a pas.
///
/// Le même manuscrit ne fait pas le même nombre de pages en poche et en grand format.
/// Pour les liminaires — faux-titre, blanche, titre, copyright, dédicace — les pages
/// coïncident d'un format à l'autre, et c'est là qu'un envoi va dans les faits.
/// Ailleurs, on refuse en disant quoi faire, le chiffre mesuré compris : c'est la
/// convention du dos non publié.
fn verifie_pages(liste: &[crate::envoi::Envoi], pages: u32) -> Result<(), String> {
    for (i, e) in liste.iter().enumerate() {
        if e.place.page >= 1 && e.place.page <= pages {
            continue;
        }
        let qui = if e.dedicataire.trim().is_empty() {
            format!("envoi {}", i + 1)
        } else {
            e.dedicataire.clone()
        };
        return Err(format!(
            "{qui} : envoi placé page {}, l'intérieur n'en fait que {pages}.",
            e.place.page
        ));
    }
    Ok(())
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

    // Le compte de pages n'existe qu'après la convergence : le contrôle ne peut pas
    // avoir lieu plus tôt, et refuser ici coûte une composition de moins qu'un tirage
    // faux.
    verifie_pages(&envois.liste, base.pages)?;

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
    let mut sorties = Vec::with_capacity(envois.liste.len());
    for (e, nom_dossier) in envois.liste.iter().zip(dossiers_d_envoi(&envois.liste)) {
        let dossier = racine.join(&nom_dossier);
        std::fs::create_dir_all(&dossier)
            .map_err(|err| format!("répertoire inutilisable ({}) : {err}", dossier.display()))?;

        let src = dossier.join(nom(pr, "interieur", "typ"));
        let t = trace(projet, e, &dossier)?;
        ecrire(
            &src,
            &interieur::source(livre, int, pr, &reglage, &chapitres, Some(t)),
        )?;
        let pdf = dossier.join(nom(pr, "interieur", "pdf"));
        // L'envoi peut composer dans une main que la référence n'emploie pas : ses
        // substitutions à lui s'ajoutent à celles du package de référence.
        let replis = typst.compile(&src, &pdf)?;

        // La planche ne dépend pas de l'envoi : elle est recopiée, pas recomposée.
        let mut p = base.clone();
        for f in replis {
            if !p.polices_introuvables.contains(&f) {
                p.polices_introuvables.push(f);
            }
        }
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
                ..Default::default()
            },
            crate::envoi::Envoi {
                dedicataire: "Marie-Léa".into(),
                contenu: "B.".into(),
                ..Default::default()
            },
            crate::envoi::Envoi {
                dedicataire: "..".into(),
                contenu: "C.".into(),
                ..Default::default()
            },
        ];
        assert_eq!(
            dossiers_d_envoi(&envois),
            vec!["Marie-Léa", "Marie-Léa-2", "envoi"]
        );
    }

    fn projet_en_images(image: Option<&str>) -> Projet {
        let mut p = Projet::nouveau(crate::projet::Livre::vide(), "## 01\n\nA.\n".into());
        p.meta.envois.liste = vec![crate::envoi::Envoi {
            dedicataire: "Léa".into(),
            main: crate::envoi::Main::Image,
            image: image.map(str::to_string),
            ..Default::default()
        }];
        if let Some(n) = image {
            p.images_envois.insert(n.into(), b"\x89PNG".to_vec());
        }
        p
    }

    /// L'image part avec la source qui la nomme, dans le répertoire de son dédicataire :
    /// c'est ce qui garantit qu'aucune image ne se retrouve dans l'exemplaire d'un autre.
    #[test]
    fn l_image_d_un_envoi_est_ecrite_a_cote_de_sa_source() {
        let p = projet_en_images(Some("Léa.png"));
        let dir = tempfile::tempdir().unwrap();
        let t = trace(&p, &p.meta.envois.liste[0], dir.path()).unwrap();

        assert!(matches!(
            &t.quoi,
            interieur::Quoi::Image { fichier } if fichier == "Léa.png"
        ));
        assert_eq!(
            std::fs::read(dir.path().join("Léa.png")).unwrap(),
            b"\x89PNG"
        );
    }

    /// **La promesse du figeage.** Une image générée puis acceptée est une image comme
    /// une autre : elle vit dans l'archive, et composer ne rappelle jamais le modèle. Un
    /// package se refait des mois plus tard, hors ligne, à l'identique — et le jour où
    /// le service aura fermé.
    #[test]
    fn une_image_generee_et_acceptee_compose_comme_une_autre() {
        let mut p = projet_en_images(Some("Léa.png"));
        p.meta.envois.gabarit = "une aquarelle, mention « {envoi} »".into();
        p.meta.envois.liste[0].main = crate::envoi::Main::Diffusion;
        let dir = tempfile::tempdir().unwrap();
        let t = trace(&p, &p.meta.envois.liste[0], dir.path()).unwrap();
        assert!(matches!(
            &t.quoi,
            interieur::Quoi::Image { fichier } if fichier == "Léa.png"
        ));
    }

    /// Un envoi sans image ne compose pas, et l'erreur nomme la personne : la liste peut
    /// en porter dix, et « il manque une image » n'aiderait pas à savoir laquelle. Ce
    /// refus est ici, à la composition, et non à la saisie — on écrit la liste avant de
    /// choisir les images.
    #[test]
    fn un_envoi_sans_image_refuse_de_composer_en_nommant_le_dedicataire() {
        let p = projet_en_images(None);
        let dir = tempfile::tempdir().unwrap();
        let err = trace(&p, &p.meta.envois.liste[0], dir.path()).unwrap_err();
        assert!(err.contains("Léa"), "{err}");
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

    /// Le même manuscrit ne fait pas le même nombre de pages en poche et en grand
    /// format : une page choisie à l'œil chez l'un peut n'exister chez l'autre. Rogner
    /// sur la dernière page enverrait à l'impression un exemplaire que personne n'a
    /// voulu ; le refus nomme la personne, la page et le compte, comme le fait déjà le
    /// dos non publié.
    #[test]
    fn une_page_hors_bornes_fait_refuser_la_generation() {
        let err = verifie_pages(
            &[
                crate::envoi::Envoi {
                    dedicataire: "Léa".into(),
                    place: crate::envoi::Place {
                        page: 3,
                        ..Default::default()
                    },
                    ..Default::default()
                },
                crate::envoi::Envoi {
                    dedicataire: "Marc".into(),
                    place: crate::envoi::Place {
                        page: 210,
                        ..Default::default()
                    },
                    ..Default::default()
                },
            ],
            198,
        )
        .unwrap_err();
        assert!(err.contains("Marc"), "{err}");
        assert!(err.contains("210"), "{err}");
        assert!(err.contains("198"), "{err}");
        assert!(!err.contains("Léa"), "Léa n'est pas en cause : {err}");
    }

    /// Page 0 n'existe pas : les pages de Typst comptent à partir de 1, et un zéro
    /// venu d'un TOML écrit à la main ne doit pas composer un envoi invisible.
    #[test]
    fn la_page_zero_est_refusee() {
        let err = verifie_pages(
            &[crate::envoi::Envoi {
                dedicataire: "Léa".into(),
                place: crate::envoi::Place {
                    page: 0,
                    ..Default::default()
                },
                ..Default::default()
            }],
            198,
        )
        .unwrap_err();
        assert!(err.contains("Léa"), "{err}");
    }

    /// Ce que `trace` écrit sur le disque est détouré, et porte un nom en `.png` : Typst
    /// reconnaît le format d'une image **à son extension**, et un PNG rangé sous `.jpg`
    /// ne se composerait pas — l'erreur tomberait sur l'exemplaire d'une personne.
    #[test]
    fn une_image_detouree_s_ecrit_en_png() {
        let dir = tempfile::tempdir().unwrap();
        let mut p = projet_en_images(Some("Léa.jpg"));
        // Un JPEG uni clair : tout est papier, donc tout doit sortir transparent.
        let mut jpeg = Vec::new();
        image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(
            8,
            8,
            image::Rgb([245, 243, 238]),
        ))
        .write_to(
            &mut std::io::Cursor::new(&mut jpeg),
            image::ImageFormat::Jpeg,
        )
        .unwrap();
        p.images_envois.insert("Léa.jpg".into(), jpeg);
        p.meta.envois.liste[0].detourage = Some(crate::detourage::Detourage {
            papier: 240.0,
            encre: 40.0,
        });

        let t = trace(&p, &p.meta.envois.liste[0], dir.path()).unwrap();
        let interieur::Quoi::Image { fichier } = t.quoi else {
            panic!("la trace n'est pas une image");
        };
        assert!(fichier.ends_with(".png"), "écrit sous « {fichier} »");
        let ecrit = std::fs::read(dir.path().join(&*fichier)).unwrap();
        let px = image::load_from_memory(&ecrit).unwrap().to_rgba8();
        assert_eq!(
            px.get_pixel(0, 0)[3],
            0,
            "le papier n'a pas été rendu transparent"
        );
    }

    /// Un projet d'avant ce chantier compose exactement ce qu'il composait : mêmes
    /// octets, même nom. C'est l'autre moitié de la décision « un projet ancien garde
    /// son rendu » — la première moitié est dans `envoi.rs`, et elle ne dit que le
    /// modèle.
    #[test]
    fn sans_detourage_l_image_part_telle_quelle() {
        let dir = tempfile::tempdir().unwrap();
        let mut p = projet_en_images(Some("Léa.jpg"));
        let mut jpeg = Vec::new();
        image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(
            8,
            8,
            image::Rgb([245, 243, 238]),
        ))
        .write_to(
            &mut std::io::Cursor::new(&mut jpeg),
            image::ImageFormat::Jpeg,
        )
        .unwrap();
        p.images_envois.insert("Léa.jpg".into(), jpeg.clone());
        // Le projet ancien : la photo est là, les seuils n'y sont pas.
        p.meta.envois.liste[0].detourage = None;

        let t = trace(&p, &p.meta.envois.liste[0], dir.path()).unwrap();
        let interieur::Quoi::Image { fichier } = t.quoi else {
            panic!("la trace n'est pas une image");
        };
        assert!(fichier.ends_with(".jpg"), "le nom a changé : « {fichier} »");
        assert_eq!(
            std::fs::read(dir.path().join(&*fichier)).unwrap(),
            jpeg,
            "les octets ont été retouchés sans qu'on l'ait demandé"
        );
    }
}
