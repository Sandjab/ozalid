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

/// D'où vient l'écriture des envois de ce livre.
///
/// Le livre fixe sa main, l'envoi apporte son contenu : tous les exemplaires d'un même
/// livre se ressemblent, comme dans la réalité.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "lowercase")]
pub enum Main {
    /// Police manuscrite : embarquée avec l'application, ou fournie par l'auteur et
    /// embarquée dans le `.ozalid`. Une seule variante pour ces deux sources — seule la
    /// provenance du fichier diffère, la composition est la même. Le lot suivant y
    /// ajoutera l'image générée.
    Police { police: String },
    /// Une image écrite à la main, une par envoi : l'auteur écrit son mot sur une
    /// feuille, le photographie, et c'est cette image-là qui s'imprime.
    Image,
}

impl Default for Main {
    fn default() -> Self {
        Self::Police {
            police: MAINS[0].into(),
        }
    }
}

/// Un mot adressé à une personne, sur son exemplaire.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Envoi {
    pub dedicataire: String,
    /// Ce que la main réclame : le texte à composer. Vide quand la main est une image.
    #[serde(default)]
    pub contenu: String,
    /// Nom, sous `envois/` dans l'archive, de l'image de cet envoi. Deux champs pour
    /// trois formes plutôt qu'un champ par forme : la ligne d'envoi change de nature
    /// avec la main, elle ne s'encombre pas de ce que la main ne réclame pas.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
}

/// La main du livre et ses envois.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Envois {
    #[serde(default)]
    pub main: Main,
    /// Famille de la police personnelle embarquée sous `polices/`, quand le livre en
    /// porte une.
    ///
    /// Le nom figure ici pour que `projet.toml` reste lisible dézippé, mais **c'est le
    /// fichier qui fait foi** : à l'ouverture, `normalise` le relève dans l'archive et
    /// écrase ce que le TOML annonçait. Un nom recopié à la main dans le TOML ferait
    /// sinon composer une police que Typst ne trouverait pas — c'est-à-dire une autre.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub personnelle: Option<String>,
    #[serde(default)]
    pub liste: Vec<Envoi>,
}

impl Envois {
    /// Refuse une main que Typst ne saurait pas trouver.
    ///
    /// Sans ce contrôle, Typst composerait dans sa police par défaut **sans lever
    /// d'erreur** : `--ignore-system-fonts` empêche une substitution par le système,
    /// pas une substitution par le défaut du binaire. C'est le contrôle
    /// d'`Interieur::verifie`, pour la même raison.
    pub fn verifie(&self) -> Result<(), String> {
        // Une image n'a pas de nom de police à trouver : elle se pose telle quelle.
        // Ce qui lui manque — l'image d'un envoi qui n'en a pas — se refuse à la
        // composition, pas ici : on écrit la liste avant de choisir les images.
        let Main::Police { police } = &self.main else {
            return Ok(());
        };
        if MAINS.contains(&police.as_str()) || self.personnelle.as_deref() == Some(police) {
            return Ok(());
        }
        let mut attendu: Vec<&str> = MAINS.to_vec();
        attendu.extend(self.personnelle.as_deref());
        Err(format!(
            "main inconnue : « {police} ». Attendu : {}.",
            attendu.join(", ")
        ))
    }

    /// La saisie de l'interface, reprise sans ce qu'elle n'a pas à dire.
    ///
    /// La police personnelle n'est pas un réglage : c'est ce que l'archive porte, relevé
    /// dans son fichier. Laisser la saisie la nommer ferait déclarer bonne, par le
    /// contrôle qui suit, une main que Typst ne trouverait pas — et l'envoi partirait
    /// chez le dédicataire dans l'écriture de repli.
    pub fn reprend(&self, saisie: Envois) -> Result<Envois, String> {
        let e = Envois {
            personnelle: self.personnelle.clone(),
            ..saisie
        };
        e.verifie()?;
        Ok(e)
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

    /// Une main qui pose une image n'a aucun nom de police à trouver : la refuser
    /// interdirait le mode entier. Ce qui lui manque — l'image d'un envoi qui n'en a pas
    /// — se refuse à la composition, là où on peut encore le corriger.
    #[test]
    fn une_main_en_image_n_a_pas_de_police_a_verifier() {
        let e = Envois {
            main: Main::Image,
            personnelle: None,
            liste: vec![],
        };
        assert!(e.verifie().is_ok());
    }

    /// Un livre neuf sait écrire sans qu'on lui règle quoi que ce soit, comme il sait
    /// déjà composer son intérieur en EB Garamond.
    #[test]
    fn un_livre_neuf_a_deja_une_main() {
        assert_eq!(
            Envois::default().main,
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
            main: Main::Police {
                police: "Comic Sans".into(),
            },
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
            main: Main::Police {
                police: "Ma Main".into(),
            },
            personnelle: Some("Ma Main".into()),
            liste: vec![],
        };
        assert!(e.verifie().is_ok(), "police personnelle refusée");

        e.personnelle = None;
        let err = e.verifie().unwrap_err();
        assert!(err.contains("Ma Main"), "{err}");
    }

    /// L'interface renvoie l'objet entier, police personnelle comprise puisqu'elle l'a
    /// reçu ainsi. Le nom qu'elle porte n'engage qu'elle : seul le fichier de l'archive
    /// dit ce que Typst saura trouver.
    #[test]
    fn une_saisie_ne_peut_pas_inventer_une_police_personnelle() {
        let porte = Envois {
            personnelle: Some("Ma Main".into()),
            ..Envois::default()
        };
        let saisie = Envois {
            main: Main::Police {
                police: "Écriture d'Emma".into(),
            },
            personnelle: Some("Écriture d'Emma".into()),
            liste: vec![],
        };
        let err = porte.reprend(saisie).unwrap_err();
        assert!(err.contains("Écriture d'Emma"), "{err}");

        let bonne = Envois {
            main: Main::Police {
                police: "Ma Main".into(),
            },
            personnelle: None,
            liste: vec![],
        };
        assert_eq!(
            porte.reprend(bonne).unwrap().personnelle.as_deref(),
            Some("Ma Main"),
            "la police de l'archive a été perdue en chemin"
        );
    }

    /// L'erreur doit nommer ce qui est offert, la police personnelle comprise : sans
    /// elle, le message dirait de choisir parmi trois mains alors que le livre en a
    /// quatre.
    #[test]
    fn l_erreur_de_main_nomme_aussi_la_police_personnelle() {
        let e = Envois {
            main: Main::Police {
                police: "Comic Sans".into(),
            },
            personnelle: Some("Ma Main".into()),
            liste: vec![],
        };
        let err = e.verifie().unwrap_err();
        assert!(err.contains("Ma Main"), "{err}");
        assert!(err.contains(MAINS[0]), "{err}");
    }
}
