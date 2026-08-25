//! L'envoi autographe : le mot manuscrit adressé à une personne.
//!
//! À ne pas confondre avec la dédicace imprimée de `Livre::dedicace`, qui figure dans
//! tous les exemplaires. L'envoi est propre à un exemplaire, et il se pose **sur** une
//! page existante : il n'en ajoute aucune, donc il ne déplace ni la pagination, ni le
//! dos, ni la planche.

use serde::{Deserialize, Serialize};

/// Les polices manuscrites embarquées avec l'application.
///
/// Comme `POLICES_TEXTE`, la liste est fermée : Typst composerait une police inconnue
/// par repli sur son défaut **sans lever d'erreur**, et cela ne se verrait qu'après
/// tirage. Chacune a été retenue sur relevé fontTools — accents français, ligature œ,
/// guillemets, apostrophe courbe — et non sur la fiche de son fondeur.
pub const MAINS: &[&str] = &["Caveat", "Dancing Script", "Petit Formal Script"];

/// D'où vient l'écriture d'un envoi.
///
/// Elle se pose sur l'envoi et non sur le livre : rien n'oblige deux exemplaires du
/// même livre à s'écrire dans la même main.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "lowercase")]
pub enum Main {
    /// Police manuscrite : embarquée avec l'application, ou fournie par l'auteur et
    /// embarquée dans le `.ozalid`. Une seule variante pour ces deux sources — seule la
    /// provenance du fichier diffère, la composition est la même.
    Police { police: String },
    /// Une image écrite à la main, une par envoi : l'auteur écrit son mot sur une
    /// feuille, le photographie, et c'est cette image-là qui s'imprime.
    Image,
    /// Une image par envoi, produite par un modèle de diffusion à partir du gabarit du
    /// livre, dans lequel le mot de chaque envoi s'insère.
    ///
    /// Le gabarit vit sur `Envois` et non ici : c'est le style d'écriture du livre, et
    /// le réécrire pour chaque personne n'aurait pas de sens. L'adresse du modèle et la
    /// clé appartiennent à la machine, et vivent dans les préférences. Une image
    /// acceptée est figée dans l'archive comme celle du mode précédent — composer ne
    /// rappelle jamais le réseau.
    Diffusion,
}

impl Default for Main {
    fn default() -> Self {
        Self::Police {
            police: MAINS[0].into(),
        }
    }
}

/// Où l'envoi se pose sur sa page.
///
/// **En fractions de la page, jamais en millimètres** : c'est la règle de l'atelier
/// gelé — « tout réglage est en pourcentage de la largeur de couverture » — et c'est
/// ce qui rend un placement portable du poche au grand format. Les fractions portent
/// sur la page entière, marges comprises, ce qui les met en correspondance 1:1 avec le
/// canevas de l'interface, qui montre la page entière.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Place {
    /// Page physique du PDF, à partir de 1 : celle que la vignette montre. Le
    /// `counter(page)` de l'intérieur n'est jamais remis à zéro — seul son affichage
    /// est masqué jusqu'au corps —, si bien que ce numéro désigne bien la n-ième page
    /// du fichier.
    pub page: u32,
    /// Centre de l'objet, en fraction de la largeur et de la hauteur de page. Le
    /// centre et non le coin : la rotation tourne autour de lui, en CSS comme en Typst.
    pub x: f64,
    pub y: f64,
    /// Largeur de l'objet, en fraction de la largeur de page.
    pub taille: f64,
    /// Degrés, positif dans le sens horaire.
    pub angle: f64,
}

impl Default for Place {
    /// La page de titre, au bas — là où les projets d'avant cette spec portaient leur
    /// envoi. Le faux-titre est en page 1, sa blanche en 2.
    fn default() -> Self {
        Self {
            page: 3,
            x: 0.5,
            y: 0.80,
            taille: 0.60,
            angle: 0.0,
        }
    }
}

/// Un mot adressé à une personne, sur son exemplaire.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Envoi {
    pub dedicataire: String,
    /// D'où vient l'écriture de **cet** exemplaire. Elle appartenait au livre jusqu'à
    /// la v4 du format : un auteur ne pouvait pas écrire son mot à la main pour l'une
    /// et le faire composer pour l'autre.
    #[serde(default)]
    pub main: Main,
    /// Le mot adressé à cette personne. Composé tel quel sous une main en police ; sous
    /// une main générée, c'est ce que la marque `{envoi}` du gabarit va chercher. Vide
    /// sous une main en images, qui n'a pas de texte à composer.
    #[serde(default)]
    pub contenu: String,
    /// Nom, sous `envois/` dans l'archive, de l'image de cet envoi.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    /// Les seuils qui séparent l'encre du papier sur la photo de cet envoi.
    ///
    /// Sur l'envoi et non sur le livre : chaque photo a son éclairage. `None` — les
    /// projets d'avant ce chantier — vaut « aucun détourage », et l'image se compose
    /// telle quelle. Il survit à un passage en police : le perdre obligerait à régler à
    /// nouveau après un aller-retour, et ce n'est pas ce que changer de main veut dire.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detourage: Option<crate::detourage::Detourage>,
    #[serde(default)]
    pub place: Place,
}

/// Les envois du livre, et ce qu'ils partagent.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Envois {
    /// Famille de la police personnelle embarquée sous `polices/`, quand le livre en
    /// porte une.
    ///
    /// Le nom figure ici pour que `projet.toml` reste lisible dézippé, mais **c'est le
    /// fichier qui fait foi** : à l'ouverture, `Projet::ouvrir` le relève dans
    /// l'archive et écrase ce que le TOML annonçait. Un nom recopié à la main dans le
    /// TOML ferait sinon composer une police que Typst ne trouverait pas — c'est-à-dire
    /// une autre.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub personnelle: Option<String>,
    /// Le patron de prompt des envois générés, partagé par tous : c'est le style
    /// d'écriture du livre, pas le mot d'une personne. `{envoi}` y marque l'endroit où
    /// le mot de chacun s'insère.
    #[serde(default)]
    pub gabarit: String,
    /// La couleur de l'encre et le paraphe de l'auteur, que le gabarit appelle par
    /// `{couleur}` et `{paraphe}`.
    ///
    /// Sur le livre et non sur l'exemplaire, comme le gabarit lui-même : un auteur signe
    /// ses vingt exemplaires du même stylo, et de la même main. C'est ce qui les
    /// sépare de la `main`, descendue sur l'envoi en v4 parce qu'elle, elle varie —
    /// écrire à la main pour l'une et faire composer pour l'autre a un sens.
    ///
    /// La couleur se saisit **dans la langue du gabarit** : celui de la maison est en
    /// anglais, et « bleu-noir » y produirait une phrase que le modèle lirait mal.
    #[serde(default)]
    pub couleur: String,
    #[serde(default)]
    pub paraphe: String,
    #[serde(default)]
    pub liste: Vec<Envoi>,
}

impl Envois {
    /// Refuse une main que Typst ne saurait pas trouver, en nommant l'envoi fautif.
    ///
    /// Sans ce contrôle, Typst composerait dans sa police par défaut **sans lever
    /// d'erreur** : `--ignore-system-fonts` empêche une substitution par le système,
    /// pas une substitution par le défaut du binaire. C'est le contrôle
    /// d'`Interieur::verifie`, pour la même raison.
    ///
    /// Le dédicataire est nommé parce qu'une liste de vingt envois dont un seul est
    /// fautif laisserait sinon chercher lequel.
    pub fn verifie(&self) -> Result<(), String> {
        for (i, e) in self.liste.iter().enumerate() {
            // Une image n'a pas de nom de police à trouver : elle se pose telle quelle.
            // Ce qui lui manque — l'image d'un envoi qui n'en a pas — se refuse à la
            // composition, pas ici : on écrit la liste avant de choisir les images.
            let Main::Police { police } = &e.main else {
                continue;
            };
            if MAINS.contains(&police.as_str()) || self.personnelle.as_deref() == Some(police) {
                continue;
            }
            let mut attendu: Vec<&str> = MAINS.to_vec();
            attendu.extend(self.personnelle.as_deref());
            return Err(format!(
                "{} : main inconnue « {police} ». Attendu : {}.",
                designe(e, i),
                attendu.join(", ")
            ));
        }
        Ok(())
    }
}

impl Envois {
    /// Ajoute un envoi, qui naît comme le précédent.
    ///
    /// Même main, même placement que le dernier de la liste. Sans cette règle, vingt
    /// dédicataires demanderaient vingt fois le même réglage, et la ressemblance des
    /// exemplaires d'un même tirage — acquise tant que la main appartenait au livre — se
    /// paierait à chaque ligne.
    ///
    /// Le mot et l'image ne s'héritent pas : ce sont eux qui distinguent un exemplaire,
    /// et hériter le mot enverrait à Marc celui de Léa.
    pub fn ajouter(&mut self, dedicataire: String) {
        let modele = self.liste.last();
        self.liste.push(Envoi {
            dedicataire,
            main: modele.map(|e| e.main.clone()).unwrap_or_default(),
            place: modele.map(|e| e.place).unwrap_or_default(),
            ..Default::default()
        });
    }
}

/// Comment nommer un envoi dans un message d'erreur.
///
/// Le rang plutôt que rien quand la ligne est anonyme : « main inconnue » tout court
/// laisserait chercher dans une liste où plusieurs lignes le sont.
fn designe(e: &Envoi, i: usize) -> String {
    if e.dedicataire.trim().is_empty() {
        format!("envoi {}", i + 1)
    } else {
        e.dedicataire.clone()
    }
}

/// Nom de répertoire tiré d'un dédicataire.
///
/// C'est la seule chaîne saisie par l'utilisateur qui devienne un chemin : tout ce qui
/// n'est ni lettre, ni chiffre, ni espace, ni tiret devient un tiret, et ce qui ne
/// laisse rien devient « envoi ». Un dédicataire nommé « .. » ne doit pas écrire hors
/// du dossier du projet.
pub fn assaini(dedicataire: &str) -> String {
    let brut: String = dedicataire
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == ' ' || c == '-' {
                c
            } else {
                '-'
            }
        })
        .collect();
    // Deux caractères refusés d'affilée ne font qu'un tiret : « Marie D./Léa » donne
    // « Marie D-Léa », et non « Marie D--Léa », qu'on ne saurait relire.
    let mut serre = String::with_capacity(brut.len());
    for c in brut.chars() {
        if c == '-' && serre.ends_with('-') {
            continue;
        }
        serre.push(c);
    }
    let net = serre.trim().trim_matches('-').trim();
    if net.is_empty() {
        "envoi".into()
    } else {
        net.into()
    }
}

/// Nom de l'image d'un envoi dans l'archive, sous `envois/`.
///
/// Le nom est tiré du dédicataire, assaini comme un répertoire : il entre dans un `zip`,
/// puis dans une chaîne Typst, et un chemin déguisé en nom de personne n'a rien à faire
/// ni dans l'un ni dans l'autre. Le suffixe est posé **avant** l'extension : Typst
/// reconnaît le format d'une image à son extension, et « Léa.png-2 » ne serait plus une
/// image du tout.
pub fn nom_image(dedicataire: &str, ext: &str, pris: &[String]) -> String {
    let base = assaini(dedicataire);
    (1..)
        .map(|n| {
            if n < 2 {
                format!("{base}.{ext}")
            } else {
                format!("{base}-{n}.{ext}")
            }
        })
        .find(|c| !pris.iter().any(|p| p == c))
        .expect("la suite des entiers ne s'épuise pas")
}

/// Rend `nom` unique parmi `pris`, en le suffixant.
///
/// Deux dédicataires qui se réduisent au même répertoire écraseraient l'un l'autre :
/// le second exemplaire partirait avec le mot du premier.
pub fn distinct(nom: &str, pris: &[String]) -> String {
    if !pris.iter().any(|p| p == nom) {
        return nom.into();
    }
    (2..)
        .map(|n| format!("{nom}-{n}"))
        .find(|c| !pris.iter().any(|p| p == c))
        .expect("la suite des entiers ne s'épuise pas")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Un dédicataire est un nom de personne, pas un chemin. « Marie D./Léa » ne doit
    /// créer aucun sous-répertoire, et « .. » ne doit pas sortir du dossier du projet
    /// — c'est la seule chaîne saisie par l'utilisateur qui devienne un chemin.
    #[test]
    fn un_dedicataire_ne_peut_pas_devenir_un_chemin() {
        assert_eq!(assaini("Marie D./Léa"), "Marie D-Léa");
        assert_eq!(assaini(".."), "envoi");
        assert_eq!(assaini("../../etc"), "etc");
        assert_eq!(assaini("  "), "envoi");
        assert_eq!(assaini("Léa"), "Léa");
    }

    /// Deux dédicataires qui se réduisent au même répertoire écraseraient l'un
    /// l'autre : le second exemplaire partirait avec le mot du premier.
    #[test]
    fn deux_noms_qui_se_confondent_recoivent_des_repertoires_distincts() {
        let noms = ["Marie/Léa", "Marie-Léa", "Marie:Léa"];
        let mut vus: Vec<String> = Vec::new();
        for n in noms {
            let d = distinct(&assaini(n), &vus);
            vus.push(d);
        }
        assert_eq!(vus, ["Marie-Léa", "Marie-Léa-2", "Marie-Léa-3"]);
    }

    /// L'image d'un envoi entre dans l'archive, puis dans une chaîne Typst : son nom ne
    /// peut donc être ni un chemin, ni celui d'un autre envoi. Deux dédicataires
    /// homonymes recevraient sinon la même image — celle du premier.
    #[test]
    fn deux_images_d_envoi_ne_se_confondent_pas_et_gardent_leur_extension() {
        let mut pris: Vec<String> = Vec::new();
        for qui in ["Marie/Léa", "Marie-Léa", "Marie:Léa"] {
            let n = nom_image(qui, "png", &pris);
            pris.push(n);
        }
        assert_eq!(
            pris,
            ["Marie-Léa.png", "Marie-Léa-2.png", "Marie-Léa-3.png"]
        );
        assert_eq!(nom_image("..", "jpg", &pris), "envoi.jpg");
    }

    /// On choisit la main **avant** d'écrire le gabarit : le mode doit donc pouvoir
    /// voyager sans lui, à l'aller comme au retour. Le refus s'était vu à l'écran — la
    /// commande rendait « missing field gabarit » sur le simple choix de la forme.
    #[test]
    fn une_main_generee_se_choisit_avant_d_avoir_son_gabarit() {
        let sans: Main = serde_json::from_str(r#"{"mode":"diffusion"}"#)
            .expect("une main générée sans gabarit est refusée");
        assert_eq!(sans, Main::Diffusion);
        let e = Envois {
            liste: vec![Envoi {
                main: sans,
                ..Envoi::default()
            }],
            ..Envois::default()
        };
        assert!(e.verifie().is_ok());
    }

    /// Une main qui pose une image n'a aucun nom de police à trouver : la refuser
    /// interdirait le mode entier. Ce qui lui manque — l'image d'un envoi qui n'en a pas
    /// — se refuse à la composition, là où on peut encore le corriger.
    #[test]
    fn une_main_en_image_n_a_pas_de_police_a_verifier() {
        let e = Envois {
            liste: vec![Envoi {
                main: Main::Image,
                ..Envoi::default()
            }],
            ..Envois::default()
        };
        assert!(e.verifie().is_ok());
    }

    /// Un envoi neuf sait écrire sans qu'on lui règle quoi que ce soit, comme le livre
    /// sait déjà composer son intérieur en EB Garamond.
    #[test]
    fn un_envoi_neuf_a_deja_une_main() {
        assert_eq!(
            Envoi::default().main,
            Main::Police {
                police: MAINS[0].into()
            }
        );
    }

    /// Une main hors liste est refusée, jamais substituée : même contrôle que
    /// `Interieur::verifie`, et pour la même raison.
    #[test]
    fn une_main_hors_liste_est_refusee() {
        let e = Envois {
            liste: vec![Envoi {
                main: Main::Police {
                    police: "Comic Sans".into(),
                },
                ..Envoi::default()
            }],
            ..Envois::default()
        };
        let err = e.verifie().unwrap_err();
        assert!(err.contains("Comic Sans"), "{err}");
    }

    /// La police de l'auteur n'est dans aucune liste fermée — c'est tout son objet. Elle
    /// est admise parce que l'archive la porte, et refusée dès que l'archive ne la porte
    /// plus : sans quoi un `.ozalid` privé de sa police composerait ses envois par repli,
    /// en silence, dans une écriture que personne n'a choisie.
    #[test]
    fn la_police_personnelle_est_admise_tant_que_l_archive_la_porte() {
        let mut e = Envois {
            personnelle: Some("Ma Main".into()),
            liste: vec![Envoi {
                main: Main::Police {
                    police: "Ma Main".into(),
                },
                ..Envoi::default()
            }],
            ..Envois::default()
        };
        assert!(e.verifie().is_ok(), "police personnelle refusée");

        e.personnelle = None;
        let err = e.verifie().unwrap_err();
        assert!(err.contains("Ma Main"), "{err}");
    }

    /// L'erreur doit nommer ce qui est offert, la police personnelle comprise : sans
    /// elle, le message dirait de choisir parmi trois mains alors que le livre en a
    /// quatre.
    #[test]
    fn l_erreur_de_main_nomme_aussi_la_police_personnelle() {
        let e = Envois {
            personnelle: Some("Ma Main".into()),
            liste: vec![Envoi {
                main: Main::Police {
                    police: "Comic Sans".into(),
                },
                ..Envoi::default()
            }],
            ..Envois::default()
        };
        let err = e.verifie().unwrap_err();
        assert!(err.contains("Ma Main"), "{err}");
        assert!(err.contains(MAINS[0]), "{err}");
    }

    /// Un envoi neuf naît comme le précédent : même main, même placement.
    ///
    /// Sans cette règle, vingt dédicataires demanderaient vingt fois le même réglage.
    /// C'est ce qui rend les exemplaires d'un tirage semblables entre eux depuis que la
    /// main a quitté le livre.
    #[test]
    fn un_envoi_neuf_nait_comme_le_precedent() {
        let mut e = Envois::default();
        e.ajouter("Léa".into());
        e.liste[0].main = Main::Image;
        e.liste[0].place = Place {
            page: 37,
            x: 0.3,
            y: 0.4,
            taille: 0.5,
            angle: -6.0,
        };
        e.liste[0].contenu = "Pour Léa.".into();
        e.liste[0].image = Some("Lea.jpg".into());

        e.ajouter("Marc".into());
        assert_eq!(e.liste[1].main, Main::Image, "la main ne s'est pas héritée");
        assert_eq!(e.liste[1].place, e.liste[0].place, "le placement non plus");
        // Le mot et l'image distinguent un exemplaire : les hériter enverrait à Marc
        // celui de Léa.
        assert_eq!(e.liste[1].contenu, "", "Marc a reçu le mot de Léa");
        assert_eq!(e.liste[1].image, None, "Marc a reçu l'image de Léa");
        assert_eq!(e.liste[1].dedicataire, "Marc");
    }

    /// Le premier envoi d'un livre n'a personne de qui hériter : il prend les défauts,
    /// qui sont ceux d'un livre neuf.
    #[test]
    fn le_premier_envoi_prend_les_defauts() {
        let mut e = Envois::default();
        e.ajouter("Léa".into());
        assert_eq!(e.liste[0].main, Main::default());
        assert_eq!(e.liste[0].place, Place::default());
    }

    /// Un placement s'exprime en fractions de page et non en millimètres : c'est ce
    /// qui rend une maquette de placement portable du poche au grand format. Le
    /// défaut repose l'envoi sur la page de titre — page 3, le faux-titre étant en 1
    /// et sa blanche en 2 —, là où les projets d'avant le portaient.
    #[test]
    fn un_placement_neuf_repose_l_envoi_sur_la_page_de_titre() {
        // Les cinq valeurs, et non des bornes : ce que ce défaut promet, c'est qu'un
        // projet d'avant la v4 retrouve son envoi **là où il était**. Se contenter de
        // « y est dans la page » laisserait le remonter du bas au haut sans rien casser.
        assert_eq!(
            Place::default(),
            Place {
                page: 3,
                x: 0.5,
                y: 0.80,
                taille: 0.60,
                angle: 0.0,
            }
        );
    }

    /// Le fait que cette spec ajoute, et le seul que rien d'autre ne protège : deux
    /// exemplaires du même livre peuvent s'écrire dans deux mains différentes. Un mot
    /// composé pour Léa et une photo d'écriture pour Marc ne s'excluent plus.
    #[test]
    fn deux_envois_du_meme_livre_ont_chacun_leur_main() {
        let e = Envois {
            liste: vec![
                Envoi {
                    dedicataire: "Léa".into(),
                    main: Main::Police {
                        police: MAINS[0].into(),
                    },
                    contenu: "Pour Léa.".into(),
                    ..Envoi::default()
                },
                Envoi {
                    dedicataire: "Marc".into(),
                    main: Main::Image,
                    image: Some("Marc.jpg".into()),
                    ..Envoi::default()
                },
            ],
            ..Envois::default()
        };
        assert!(e.verifie().is_ok(), "{:?}", e.verifie());
    }

    /// L'erreur doit nommer le dédicataire fautif : une liste de vingt envois dont un
    /// porte une main inconnue laisserait sinon chercher lequel.
    #[test]
    fn une_main_inconnue_nomme_le_dedicataire() {
        let e = Envois {
            liste: vec![Envoi {
                dedicataire: "Marc".into(),
                main: Main::Police {
                    police: "Comic Sans".into(),
                },
                ..Envoi::default()
            }],
            ..Envois::default()
        };
        let err = e.verifie().unwrap_err();
        assert!(err.contains("Marc"), "{err}");
        assert!(err.contains("Comic Sans"), "{err}");
    }

    /// Le contrôle regarde toute la liste, et pas seulement sa première ligne : une
    /// photo d'écriture en tête ne dispense pas les suivantes d'être vérifiées. C'est
    /// le seul endroit où le passage d'une main unique à une main par envoi peut se
    /// perdre en silence — et l'exemplaire fautif partirait dans l'écriture de repli.
    #[test]
    fn un_envoi_en_image_ne_couvre_pas_les_suivants() {
        let e = Envois {
            liste: vec![
                Envoi {
                    main: Main::Image,
                    ..Envoi::default()
                },
                Envoi {
                    dedicataire: "Marc".into(),
                    main: Main::Police {
                        police: "Comic Sans".into(),
                    },
                    ..Envoi::default()
                },
            ],
            ..Envois::default()
        };
        let err = e.verifie().unwrap_err();
        assert!(err.contains("Marc"), "{err}");
    }

    /// Un envoi sans dédicataire doit se dire quand même : « main inconnue » tout
    /// court laisserait chercher dans une liste où plusieurs lignes sont anonymes.
    #[test]
    fn un_envoi_anonyme_se_designe_par_son_rang() {
        let e = Envois {
            liste: vec![Envoi {
                dedicataire: "  ".into(),
                main: Main::Police {
                    police: "Comic Sans".into(),
                },
                ..Envoi::default()
            }],
            ..Envois::default()
        };
        let err = e.verifie().unwrap_err();
        // En tête, et non quelque part : un « 1 » venu du nom d'une main ou d'un
        // numéro de version ferait passer ce test pour un message qui ne désigne rien.
        assert!(err.starts_with("envoi 1 :"), "le rang manque : {err}");
    }

    /// Un projet d'avant ce chantier n'a pas de détourage, et n'en reçoit pas d'office :
    /// on ne change pas le tirage que quelqu'un a déjà relu. Le champ est un `Option`
    /// pour cette seule raison, et `VERSION` ne bouge donc pas.
    #[test]
    fn un_envoi_ancien_n_a_pas_de_detourage() {
        let e: Envoi = toml::from_str("dedicataire = \"Léa\"\ncontenu = \"\"\n").unwrap();
        assert_eq!(e.detourage, None);
    }
}
