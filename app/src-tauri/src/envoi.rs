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
    /// Police manuscrite embarquée avec l'application. Les lots suivants y ajouteront
    /// la police fournie par l'auteur, l'image écrite à la main et l'image générée.
    Police { police: String },
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
    /// Ce que la main réclame : ici, le texte à composer.
    #[serde(default)]
    pub contenu: String,
}

/// La main du livre et ses envois.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Envois {
    #[serde(default)]
    pub main: Main,
    #[serde(default)]
    pub liste: Vec<Envoi>,
}

impl Envois {
    /// Refuse une main hors liste.
    ///
    /// Sans ce contrôle, Typst composerait dans sa police par défaut **sans lever
    /// d'erreur** : `--ignore-system-fonts` empêche une substitution par le système,
    /// pas une substitution par le défaut du binaire. C'est le contrôle
    /// d'`Interieur::verifie`, pour la même raison.
    pub fn verifie(&self) -> Result<(), String> {
        let Main::Police { police } = &self.main;
        if MAINS.contains(&police.as_str()) {
            return Ok(());
        }
        Err(format!(
            "main inconnue : « {police} ». Attendu : {}.",
            MAINS.join(", ")
        ))
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

    /// Un livre neuf sait écrire sans qu'on lui règle quoi que ce soit, comme il sait
    /// déjà composer son intérieur en EB Garamond.
    #[test]
    fn un_livre_neuf_a_deja_une_main() {
        let Main::Police { police } = Envois::default().main;
        assert_eq!(police, MAINS[0]);
    }

    /// Une main hors liste est refusée, jamais substituée : même contrôle que
    /// `Interieur::verifie`, et pour la même raison.
    #[test]
    fn une_main_hors_liste_est_refusee() {
        let e = Envois {
            main: Main::Police {
                police: "Comic Sans".into(),
            },
            liste: vec![],
        };
        let err = e.verifie().unwrap_err();
        assert!(err.contains("Comic Sans"), "{err}");
    }
}
