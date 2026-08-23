//! Les maquettes de couverture : des **archives**, non du code.
//!
//! Une maquette ne porte que la mise en page : le titre, l'auteur, le genre, l'éditeur
//! et la collection viennent du livre. Charger une maquette ne change donc jamais ce
//! qui sera imprimé comme identité — seulement la façon dont ça paraît.
//!
//! ```text
//! maquette.toml   le nom affiché, et la couverture entière
//! images/         couverture.ext et quatrieme.ext, quand la maquette en porte
//! ```
//!
//! Trois maquettes sont **fournies** : leurs archives sont incorporées au binaire par
//! `include_bytes!`. Il n'y a donc aucun chemin à résoudre sur le poste, aucun mode
//! dégradé, aucun écart entre développement et livraison — et leur immuabilité est un
//! fait, pas une règle applicative. C'est précisément le piège connu de `fonts/`, où
//! `target/debug` ne suit pas les sources.
//!
//! **Pas de champ `version`** : comme le `.ozalid`, tout futur champ arrive avec son
//! `#[serde(default = …)]`, et une archive écrite par une version antérieure se relit.

use std::collections::BTreeMap;
use std::io::{Read, Seek, Write};
use std::path::Path;

use serde::{Deserialize, Serialize};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::couverture::*;

const MAQUETTE_TOML: &str = "maquette.toml";
const IMAGES: &str = "images/";

/// Ce que porte `maquette.toml`. Le scalaire précède la table : en TOML, une valeur
/// écrite après une table lui appartiendrait.
#[derive(Serialize, Deserialize)]
struct Fichier {
    nom: String,
    couverture: Couverture,
}

/// Une maquette, fournie ou personnalisée.
///
/// La `cle` ne vit pas dans l'archive : elle vient de qui la lit — la table des
/// embarquées pour une fournie, le nom du fichier pour une personnalisée. Le **nom**,
/// lui, est l'identité, et c'est lui que l'archive porte.
#[derive(Debug, Clone, PartialEq)]
pub struct Maquette {
    pub cle: String,
    pub nom: String,
    /// Ni renommable, ni effaçable. Le refus est tenu par le Rust, pas par l'interface.
    pub fournie: bool,
    pub couverture: Couverture,
    /// Nom de fichier (sans `images/`) → contenu, comme dans `Projet`.
    pub images: BTreeMap<String, Vec<u8>>,
}

fn lire<R: Read + Seek>(source: R, cle: &str, fournie: bool) -> Result<Maquette, String> {
    let mut zip = ZipArchive::new(source).map_err(|e| format!("archive illisible : {e}"))?;
    let brut = crate::projet::fichier(&mut zip, MAQUETTE_TOML)?
        .ok_or_else(|| format!("archive sans {MAQUETTE_TOML} : ce n'est pas une maquette."))?;
    let brut =
        String::from_utf8(brut).map_err(|_| format!("{MAQUETTE_TOML} n'est pas de l'UTF-8."))?;
    let f: Fichier = toml::from_str(&brut).map_err(|e| format!("{MAQUETTE_TOML} : {e}"))?;

    let mut images = BTreeMap::new();
    let noms: Vec<String> = zip.file_names().map(str::to_owned).collect();
    for nom in noms {
        let Some(court) = nom.strip_prefix(IMAGES) else {
            continue;
        };
        // L'entrée du répertoire lui-même, que tout archiveur écrit.
        if court.is_empty() {
            continue;
        }
        if !crate::projet::nom_simple(court) {
            return Err(format!(
                "archive refusée : « {nom} » n'est pas un simple nom de fichier."
            ));
        }
        if let Some(oct) = crate::projet::fichier(&mut zip, &nom)? {
            images.insert(court.to_string(), oct);
        }
    }

    Ok(Maquette {
        cle: cle.into(),
        nom: f.nom,
        fournie,
        couverture: f.couverture,
        images,
    })
}

/// Écrire une archive n'a pas encore d'appelant en production : c'est le lot 2 qui lui
/// en donne un, quand « Enregistrer la couverture actuelle » écrira une personnalisée.
/// D'ici là seuls les tests l'appellent — dont celui qui grave les trois fournies.
#[cfg_attr(not(test), allow(dead_code))]
fn ecrire<W: Write + Seek>(sortie: W, m: &Maquette) -> Result<(), String> {
    let mut zip = ZipWriter::new(sortie);
    // La date des entrées est figée : une archive versionnée doit être la même à
    // l'octet près d'une écriture à l'autre, sinon le test qui grave les fournies
    // salirait le dépôt à chaque `cargo test`.
    let fige = |m: CompressionMethod| {
        SimpleFileOptions::default()
            .compression_method(m)
            .last_modified_time(zip::DateTime::default())
    };
    let texte_opts = fige(CompressionMethod::Deflated);
    // Les images sont déjà compressées (PNG, JPEG) : les dégonfler coûte du temps
    // pour un gain nul, parfois négatif.
    let brut_opts = fige(CompressionMethod::Stored);

    let f = Fichier {
        nom: m.nom.clone(),
        couverture: m.couverture.clone(),
    };
    let toml_brut = toml::to_string_pretty(&f)
        .map_err(|e| format!("sérialisation de {MAQUETTE_TOML} : {e}"))?;
    crate::projet::ajoute(&mut zip, MAQUETTE_TOML, toml_brut.as_bytes(), texte_opts)?;
    for (nom, oct) in &m.images {
        crate::projet::ajoute(&mut zip, &format!("{IMAGES}{nom}"), oct, brut_opts)?;
    }
    zip.finish().map_err(|e| format!("clôture : {e}"))?;
    Ok(())
}

/// Les trois fournies, incorporées au binaire : rien à résoudre sur le poste, donc
/// aucun écart entre développement et livraison, et l'immuabilité est un fait.
const FOURNIES: [(&str, &[u8]); 3] = [
    ("folio", include_bytes!("../maquettes/folio.maquette")),
    ("blanche", include_bytes!("../maquettes/blanche.maquette")),
    (
        "surimpression",
        include_bytes!("../maquettes/surimpression.maquette"),
    ),
];

/// Les maquettes, dans l'ordre où l'interface les propose.
///
/// **La lecture est au mieux** : une archive illisible est ignorée avec un mot sur la
/// sortie d'erreur — ce qui se perd est un point de départ, et refuser la liste entière
/// coûterait les autres. L'écriture, elle, échoue fort : elle perdrait du travail.
///
/// `config` porte le répertoire de configuration, ou `None` quand il est inatteignable.
/// Il est ignoré tant que les personnalisées n'existent pas (lot 2) : seules les
/// fournies sont servies.
pub fn toutes(_config: Option<&Path>) -> Vec<Maquette> {
    FOURNIES
        .iter()
        .filter_map(|(cle, octets)| {
            lire(std::io::Cursor::new(*octets), cle, true)
                .map_err(|e| eprintln!("maquette fournie « {cle} » illisible : {e}"))
                .ok()
        })
        .collect()
}

pub fn par_cle(config: Option<&Path>, cle: &str) -> Option<Maquette> {
    toutes(config).into_iter().find(|m| m.cle == cle)
}

/// Les lettres latines accentuées, ramenées à leur base ASCII.
///
/// Une table plutôt qu'une dépendance de normalisation Unicode : le besoin tient dans
/// les alphabets latins, et une crate de plus pour cinquante caractères coûterait plus
/// cher que ce qu'elle rendrait. Les majuscules n'y figurent pas — la casse est abaissée
/// avant la table.
const ACCENTS: &[(char, &str)] = &[
    ('à', "a"),
    ('á', "a"),
    ('â', "a"),
    ('ã', "a"),
    ('ä', "a"),
    ('å', "a"),
    ('ç', "c"),
    ('è', "e"),
    ('é', "e"),
    ('ê', "e"),
    ('ë', "e"),
    ('ì', "i"),
    ('í', "i"),
    ('î', "i"),
    ('ï', "i"),
    ('ñ', "n"),
    ('ò', "o"),
    ('ó', "o"),
    ('ô', "o"),
    ('õ', "o"),
    ('ö', "o"),
    ('ù', "u"),
    ('ú', "u"),
    ('û', "u"),
    ('ü', "u"),
    ('ý', "y"),
    ('ÿ', "y"),
    ('æ', "ae"),
    ('œ', "oe"),
    ('ß', "ss"),
];

/// Le slug d'un nom : ce qui nomme son fichier, et ce qui l'identifie.
///
/// Accents décapés, casse ignorée, tout ce qui n'est ni lettre ni chiffre ASCII devient
/// un tiret, et deux tirets d'affilée n'en font qu'un. « Ma Collection » et
/// « ma collection… » donnent donc le même slug : ce sont le même nom, et `ecrire` le
/// refuse au lieu d'écraser.
///
/// `None` quand il ne reste rien — un nom qui ne s'écrit avec aucune lettre latine ne
/// peut pas nommer un fichier, et lui en inventer un le rendrait introuvable.
pub fn slug(nom: &str) -> Option<String> {
    let mut decape = String::with_capacity(nom.len());
    for c in nom.chars().flat_map(char::to_lowercase) {
        match ACCENTS.iter().find(|(a, _)| *a == c) {
            Some((_, base)) => decape.push_str(base),
            None => decape.push(c),
        }
    }
    let mut s = String::with_capacity(decape.len());
    for c in decape.chars() {
        if c.is_ascii_alphanumeric() {
            s.push(c);
        } else if !s.ends_with('-') {
            s.push('-');
        }
    }
    let net = s.trim_matches('-');
    (!net.is_empty()).then(|| net.to_string())
}

/// La couverture d'une fournie, pour les tests des autres modules.
///
/// Une trentaine de tests partaient d'un constructeur ; ils partent maintenant d'une
/// archive, et cette aide leur évite de répéter le dépliage à chaque ligne.
/// `#[cfg(test)]` : elle n'existe pas dans le binaire livré.
#[cfg(test)]
pub(crate) fn fournie(cle: &str) -> Couverture {
    par_cle(None, cle)
        .unwrap_or_else(|| panic!("maquette fournie inconnue : {cle}"))
        .couverture
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    /// Une maquette est une archive, pas un TOML : elle porte des images, et ce qu'elle
    /// emporte doit revenir tel quel — la couverture entière et chaque octet des images.
    /// C'est la promesse du format, et la seule chose qui rende une personnalisée
    /// fidèle au livre depuis lequel on l'a enregistrée.
    #[test]
    fn une_maquette_fait_l_aller_retour_avec_ses_images() {
        let mut images = BTreeMap::new();
        images.insert("couverture.jpg".to_string(), vec![0xff, 0xd8, 0xff, 0xe0]);
        images.insert("quatrieme.png".to_string(), vec![0x89, b'P', b'N', b'G']);
        let avant = Maquette {
            cle: "ma-collection".into(),
            nom: "Ma collection".into(),
            fournie: false,
            couverture: fournie("blanche"),
            images,
        };

        let mut octets = Vec::new();
        ecrire(Cursor::new(&mut octets), &avant).unwrap();
        let apres = lire(Cursor::new(&octets), "ma-collection", false).unwrap();

        assert_eq!(apres, avant);
    }

    /// Le nom affiché vit dans l'archive ; la clé, elle, vient de qui la lit — le nom du
    /// fichier pour une personnalisée, la table des embarquées pour une fournie. Une
    /// archive déplacée sous un autre nom de fichier change donc de clé, pas de nom.
    #[test]
    fn la_cle_vient_du_lecteur_et_le_nom_de_l_archive() {
        let m = Maquette {
            cle: "peu-importe".into(),
            nom: "Ma collection".into(),
            fournie: false,
            couverture: fournie("folio"),
            images: BTreeMap::new(),
        };
        let mut octets = Vec::new();
        ecrire(Cursor::new(&mut octets), &m).unwrap();

        let relue = lire(Cursor::new(&octets), "autre-slug", true).unwrap();
        assert_eq!(relue.cle, "autre-slug");
        assert_eq!(relue.nom, "Ma collection");
        assert!(relue.fournie);
    }

    /// La parade du § 6 de la spec : les trois fournies ne sont plus du code, un TOML
    /// mal formé ne casserait donc plus la compilation mais le **démarrage**. Ce test
    /// les parse toutes les trois, et `cargo test` est exigé avant commit.
    #[test]
    fn les_trois_fournies_se_lisent_et_portent_leur_nom() {
        let vues: Vec<(String, String, bool)> = toutes(None)
            .into_iter()
            .map(|m| (m.cle, m.nom, m.fournie))
            .collect();
        assert_eq!(
            vues,
            [
                ("folio".to_string(), "Folio".to_string(), true),
                ("blanche".to_string(), "Blanche".to_string(), true),
                (
                    "surimpression".to_string(),
                    "Surimpression".to_string(),
                    true
                ),
            ]
        );
    }

    /// Le nom est l'identité, le slug nomme le fichier : accents décapés, casse
    /// ignorée, tout le reste en tirets. Deux noms qui donnent le même slug **sont**
    /// le même nom — c'est ce qui permet à l'écriture de refuser plutôt que d'écraser.
    #[test]
    fn le_slug_decape_les_accents_et_ignore_la_casse() {
        assert_eq!(slug("Ma collection").as_deref(), Some("ma-collection"));
        assert_eq!(slug("Élan  vital !").as_deref(), Some("elan-vital"));
        assert_eq!(slug("Cœur").as_deref(), Some("coeur"));
        assert_eq!(slug("Ma Collection"), slug("ma  collection…"));
        assert_eq!(slug("Folio").as_deref(), Some("folio"));
    }

    /// Un slug ne borde jamais de tiret : `folio-.maquette` se relirait en clé
    /// « folio- », qui ne serait plus le slug de son propre nom.
    #[test]
    fn le_slug_ne_borde_pas_de_tiret() {
        assert_eq!(slug("  Folio  ").as_deref(), Some("folio"));
        assert_eq!(slug("— Folio —").as_deref(), Some("folio"));
    }

    /// Un nom qui ne s'écrit avec aucune lettre latine ne peut pas nommer un fichier.
    /// Lui inventer « maquette-1 » cacherait le problème derrière un nom que
    /// l'utilisateur n'a pas choisi et ne saurait pas retrouver.
    #[test]
    fn un_nom_sans_lettre_latine_n_a_pas_de_slug() {
        assert_eq!(slug(""), None);
        assert_eq!(slug("   "), None);
        assert_eq!(slug("——"), None);
        assert_eq!(slug("日本"), None);
    }

    /// Une `.maquette` est un document qu'on s'échange, et rien n'oblige celle qu'on
    /// ouvre à venir d'ici. `package::ecrire_images` fait des chemins de ces noms par
    /// `join` : une entrée qui remonte écrirait ailleurs sur le disque. Le refus est le
    /// même que celui du `.ozalid`, et sur le même contrôle — il n'en existe qu'un.
    #[test]
    fn une_image_qui_remonte_hors_de_son_repertoire_est_refusee() {
        let mut octets = Vec::new();
        {
            let mut zip = ZipWriter::new(Cursor::new(&mut octets));
            let opts = SimpleFileOptions::default();
            let f = Fichier {
                nom: "Piégée".into(),
                couverture: fournie("folio"),
            };
            crate::projet::ajoute(
                &mut zip,
                MAQUETTE_TOML,
                toml::to_string_pretty(&f).unwrap().as_bytes(),
                opts,
            )
            .unwrap();
            crate::projet::ajoute(&mut zip, "images/../../ailleurs.png", b"x", opts).unwrap();
            zip.finish().unwrap();
        }

        let e = lire(Cursor::new(&octets), "piegee", false).unwrap_err();
        assert!(e.contains("ailleurs.png"), "{e}");
    }

    /// Chaque maquette doit être un archétype distinct : trois entrées qui rendraient
    /// la même chose ne serviraient à rien comme point de départ.
    #[test]
    fn les_trois_maquettes_sont_de_modes_distincts() {
        let modes: Vec<Mode> = toutes(None)
            .into_iter()
            .map(|m| m.couverture.mode)
            .collect();
        assert_eq!(modes.len(), 3);
        for (i, m) in modes.iter().enumerate() {
            assert!(!modes[..i].contains(m), "mode {m:?} en double");
        }
    }

    #[test]
    fn une_cle_inconnue_ne_rend_pas_de_maquette() {
        assert!(par_cle(None, "gallimard").is_none());
        assert!(par_cle(None, "folio").is_some());
    }

    /// Le pied éditeur est posé depuis le bas, en % de la hauteur ; le filet interne du
    /// cadre l'est depuis le bas aussi, mais son décroché se lit sur la **largeur**. Les
    /// deux ne varient donc pas ensemble d'un format à l'autre, et un pied qui dégage le
    /// filet en poche peut le traverser en A4.
    ///
    /// La maquette Blanche porte 13,5 % et non les 11 % du CSS de l'atelier : c'est le
    /// seul écart assumé vis-à-vis d'`index.html`, qui a le défaut et ne l'a pas vu.
    /// L'archive porte la valeur, ce test la borne sur tous les formats de la table —
    /// c'est ici, et nulle part ailleurs, que la raison de ce 13,5 est écrite.
    #[test]
    fn le_pied_editeur_ne_traverse_jamais_le_cadre() {
        let cv = fournie("blanche");
        let c = &cv.cadre;
        for pr in crate::providers::PROVIDERS {
            let (fw, fh) = pr.format;
            // Bord intérieur du filet le plus bas, mesuré depuis le bas de la couverture.
            // Le cadre étant concentrique, c'est la même distance qu'en haut.
            let filet = c.marge / 100.0 * fh
                + c.filet1_epaisseur / 100.0 * fw
                + c.decroche / 100.0 * fw
                + c.filet2_epaisseur / 100.0 * fw
                + c.ecart / 100.0 * fw
                + c.filet2_epaisseur / 100.0 * fw;
            let pied = cv.pied.y / 100.0 * fh;
            assert!(
                pied > filet + 0.5,
                "{} : pied à {pied:.2} mm du bas, filet à {filet:.2} mm",
                pr.cle
            );
        }
    }

    /// Le voile n'a de sens que sur une image : l'allumer sans image assombrirait
    /// une couverture qui n'a rien dessous.
    #[test]
    fn seule_la_maquette_a_image_pleine_page_porte_un_voile() {
        assert_eq!(fournie("folio").voile, Voile::Aucun);
        assert_eq!(fournie("blanche").voile, Voile::Aucun);
        assert_ne!(fournie("surimpression").voile, Voile::Aucun);
    }
}
