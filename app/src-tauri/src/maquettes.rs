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
use crate::image::Cadrage;

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

fn style(police: &str, graisse: u16, taille: f64, couleur: &str) -> Style {
    Style {
        police: police.into(),
        graisse,
        italique: false,
        taille,
        couleur: couleur.into(),
        tracking: 0.0,
        casse: Casse::Telle,
    }
}

/// Réglages de 4ème communs aux trois maquettes : l'atelier les livrait identiques.
fn quatrieme_commune() -> Quatrieme {
    Quatrieme {
        fond: FondQuatre::Herite,
        couleur: "#fcf0d8".into(),
        texte: String::new(),
        style: style("Spectral", 400, 3.0, "#191917"),
        interligne: 1.45,
        align: Align::Gauche,
        pad_x: 10.0,
        top: 12.0,
        pied_actif: true,
        style_pied: style("Archivo", 400, 2.4, "#191917"),
        pied_y: 4.0,
        isbn_actif: false,
        isbn_l: 34.0,
        isbn_h: 21.0,
        isbn_dx: 7.0,
        isbn_dy: 7.0,
        cadrage: Cadrage::default(),
        voile: Voile::Aucun,
        voile_opacite: 0.55,
    }
}

/// Le dos des trois maquettes : auteur et titre en tête, éditeur au pied, comme un
/// poche en rayon. Seule la couleur d'encre change d'une maquette à l'autre,
/// selon la couleur du papier — le reste se règle élément par élément dans le panneau.
fn dos(couleur: &str) -> Dos {
    let d = Dos::defaut();
    let encre = |e: ElementDos| ElementDos {
        style: style("Archivo", 600, 2.6, couleur),
        ..e
    };
    Dos {
        auteur: encre(d.auteur),
        titre: encre(d.titre),
        editeur: encre(d.editeur),
        collection: encre(d.collection),
        ..Dos::defaut()
    }
}

fn pastille_eteinte() -> Pastille {
    Pastille {
        actif: false,
        style: style("Archivo", 400, 3.2, "#ffffff"),
        fond: "#111111".into(),
        coin: Coin::BasDroite,
        verticale: false,
        arrondie: true,
        dx: 4.5,
        dy: 3.5,
    }
}

/// Bandeau de titre en haut, image à fond perdu dessous. Archétype Folio / Penguin
/// Modern Classics.
pub fn folio() -> Couverture {
    Couverture {
        mode: Mode::Bandeau,
        papier: "#ffffff".into(),
        align: Align::Gauche,
        pad_x: 7.0,
        bandeau: 30.0,
        bandeau_retrait: false,
        bloc_y: 13.0,
        cadre: Cadre {
            actif: false,
            marge: 9.0,
            filet1_couleur: "#000000".into(),
            filet1_epaisseur: 0.3,
            decroche: 4.0,
            filet2_couleur: "#c00000".into(),
            filet2_epaisseur: 0.25,
            ecart: 0.9,
        },
        auteur: Style {
            taille: 6.4,
            ..style("Archivo", 700, 6.4, "#c00000")
        },
        titre: style("Spectral", 400, 8.0, "#191917"),
        titre_interligne: 1.1,
        titre_ecart: 3.5,
        genre_visible: false,
        genre: style("Spectral", 400, 2.2, "#191917"),
        genre_ecart: 6.0,
        // Le pied s'inspire de chartes réelles : bandeau monogramme + nom d'éditeur en
        // capitales espacées. Le nom et le monogramme, eux, viennent du livre — la
        // maquette ne dit que la façon dont ils paraissent.
        pied: Pied {
            actif: false,
            y: 11.0,
            style_mono: Style {
                italique: true,
                ..style("Spectral", 600, 7.0, "#191917")
            },
            style_editeur: Style {
                tracking: 10.0,
                ..style("Archivo", 400, 3.2, "#191917")
            },
        },
        pastille: Pastille {
            actif: true,
            ..pastille_eteinte()
        },
        cadrage: Cadrage::default(),
        voile: Voile::Aucun,
        voile_opacite: 0.55,
        quatrieme: quatrieme_commune(),
        dos: dos("#191917"),
    }
}

/// Composition purement typographique, triple filet. Archétype Blanche / NRF.
pub fn blanche() -> Couverture {
    Couverture {
        mode: Mode::Typo,
        papier: "#fcf0d8".into(),
        align: Align::Centre,
        pad_x: 16.0,
        bandeau: 30.0,
        bandeau_retrait: false,
        bloc_y: 13.0,
        cadre: Cadre {
            actif: true,
            marge: 9.0,
            filet1_couleur: "#000000".into(),
            filet1_epaisseur: 0.3,
            decroche: 4.0,
            filet2_couleur: "#c00000".into(),
            filet2_epaisseur: 0.25,
            ecart: 0.9,
        },
        auteur: Style {
            tracking: 6.0,
            casse: Casse::Capitales,
            ..style("Bodoni Moda", 700, 3.6, "#000000")
        },
        titre: Style {
            tracking: 1.0,
            casse: Casse::Capitales,
            ..style("Bodoni Moda", 700, 9.0, "#c00000")
        },
        titre_interligne: 1.05,
        titre_ecart: 11.0,
        genre_visible: true,
        genre: Style {
            tracking: 12.0,
            ..style("Bodoni Moda", 400, 2.2, "#191917")
        },
        genre_ecart: 6.0,
        pied: Pied {
            actif: true,
            // 13,5 % et non les 11 % du CSS d'origine : à 11 %, le pied éditeur passe
            // sous le filet interne du cadre et le traverse. C'est le seul écart assumé
            // vis-à-vis d'`index.html` dans les maquettes — l'atelier a le même défaut,
            // il n'a pas été reproduit. Le test `le_pied_editeur_ne_traverse_jamais_le_cadre`
            // borne la valeur sur tous les formats de la table.
            y: 13.5,
            style_mono: Style {
                italique: true,
                ..style("Bodoni Moda", 600, 7.0, "#191917")
            },
            style_editeur: Style {
                tracking: 10.0,
                ..style("Bodoni Moda", 400, 3.2, "#191917")
            },
        },
        pastille: pastille_eteinte(),
        cadrage: Cadrage::default(),
        voile: Voile::Aucun,
        voile_opacite: 0.55,
        quatrieme: quatrieme_commune(),
        dos: dos("#191917"),
    }
}

/// Image sur toute la surface, texte par-dessus, voile de lisibilité.
pub fn surimpression() -> Couverture {
    Couverture {
        mode: Mode::Surimpression,
        papier: "#000000".into(),
        align: Align::Centre,
        pad_x: 12.0,
        bandeau: 30.0,
        bandeau_retrait: false,
        bloc_y: 9.0,
        cadre: Cadre {
            actif: true,
            marge: 6.0,
            filet1_couleur: "#f2ece0".into(),
            filet1_epaisseur: 0.25,
            decroche: 1.4,
            filet2_couleur: "#f2ece0".into(),
            filet2_epaisseur: 0.15,
            ecart: 0.6,
        },
        auteur: Style {
            tracking: 14.0,
            casse: Casse::Capitales,
            ..style("Archivo", 600, 3.4, "#f4efe4")
        },
        titre: Style {
            tracking: -1.0,
            ..style("Playfair Display", 500, 11.0, "#ffffff")
        },
        titre_interligne: 1.02,
        titre_ecart: 6.0,
        genre_visible: false,
        genre: style("Playfair Display", 400, 2.2, "#f4efe4"),
        genre_ecart: 6.0,
        pied: Pied {
            actif: false,
            y: 11.0,
            style_mono: Style {
                italique: true,
                ..style("Playfair Display", 600, 7.0, "#f4efe4")
            },
            style_editeur: Style {
                tracking: 10.0,
                ..style("Archivo", 400, 3.2, "#f4efe4")
            },
        },
        pastille: pastille_eteinte(),
        // Ancrage bas : sur un portrait, garder le haut du cadre plutôt que le centre.
        cadrage: Cadrage {
            y: 0.62,
            zoom: 1.05,
            ..Cadrage::default()
        },
        voile: Voile::Deux,
        voile_opacite: 0.62,
        quatrieme: quatrieme_commune(),
        dos: dos("#f4efe4"),
    }
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
            couverture: blanche(),
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
            couverture: folio(),
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

    /// **Transitoire** — part à la tâche 6.
    ///
    /// La bascule doit être invisible : ce que l'archive rend doit être exactement ce
    /// que le constructeur rendait, champ pour champ. C'est ce test-là qui autorise à
    /// retirer les constructeurs.
    #[test]
    fn les_archives_fournies_valent_les_constructeurs() {
        for (cle, attendue) in [
            ("folio", folio()),
            ("blanche", blanche()),
            ("surimpression", surimpression()),
        ] {
            assert_eq!(par_cle(None, cle).unwrap().couverture, attendue, "{cle}");
        }
    }

    /// **Transitoire** — part à la tâche 6, avec les constructeurs.
    ///
    /// Les trois archives fournies ne s'écrivent pas à la main : elles se gravent
    /// depuis les constructeurs, ce qui les leur rend identiques par construction. Ce
    /// test écrit dans les sources, ce qu'un test ne fait jamais autrement — c'est le
    /// prix d'une bascule qu'on veut invisible, et il ne dure que le temps du lot.
    #[test]
    fn grave_les_archives_fournies() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("maquettes");
        std::fs::create_dir_all(&dir).unwrap();
        for (cle, nom, couverture) in [
            ("folio", "Folio", folio()),
            ("blanche", "Blanche", blanche()),
            ("surimpression", "Surimpression", surimpression()),
        ] {
            let m = Maquette {
                cle: cle.into(),
                nom: nom.into(),
                fournie: true,
                couverture,
                images: BTreeMap::new(),
            };
            let f = std::fs::File::create(dir.join(format!("{cle}.maquette"))).unwrap();
            ecrire(f, &m).unwrap();
        }
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
                couverture: folio(),
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
    /// filet en poche peut le traverser en A4. Ce test tient la maquette Blanche sur
    /// tous les formats de la table — c'est là que le défaut d'`index.html` se voyait.
    #[test]
    fn le_pied_editeur_ne_traverse_jamais_le_cadre() {
        let cv = blanche();
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
        assert_eq!(folio().voile, Voile::Aucun);
        assert_eq!(blanche().voile, Voile::Aucun);
        assert_ne!(surimpression().voile, Voile::Aucun);
    }
}
