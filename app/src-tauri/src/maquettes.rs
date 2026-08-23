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
use std::path::{Path, PathBuf};

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

/// Écrit une archive de maquette. Ni le nom du fichier ni l'unicité ne la regardent :
/// c'est `ecrire` qui en décide.
fn ecrire_archive<W: Write + Seek>(sortie: W, m: &Maquette) -> Result<(), String> {
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

/// L'extension d'une archive de maquette, sans le point.
const EXT: &str = "maquette";

/// Là où vivent les personnalisées : à côté de `preferences.toml`, parce qu'elles
/// appartiennent à la machine et non au livre. Un `.ozalid` reste auto-portant — sa
/// couverture est dans l'archive ; une maquette n'est qu'un point de départ.
fn repertoire(config: &Path) -> PathBuf {
    config.join("maquettes")
}

/// Les personnalisées, dans l'ordre de leur nom.
///
/// Un répertoire absent n'est pas une avarie : c'est l'état d'un poste où l'on n'a
/// encore rien enregistré.
fn personnalisees(config: &Path) -> Vec<Maquette> {
    let Ok(entrees) = std::fs::read_dir(repertoire(config)) else {
        return Vec::new();
    };
    let mut v = Vec::new();
    for e in entrees.flatten() {
        let chemin = e.path();
        if chemin.extension().and_then(|x| x.to_str()) != Some(EXT) {
            continue;
        }
        let Some(cle) = chemin.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        match std::fs::File::open(&chemin)
            .map_err(|err| err.to_string())
            .and_then(|f| lire(f, cle, false))
        {
            Ok(m) => v.push(m),
            Err(err) => eprintln!("maquette « {} » ignorée : {err}", chemin.display()),
        }
    }
    // Par le nom affiché, qui est l'identité — et non par la clé, que l'utilisateur ne
    // voit nulle part, ni par ce que rend `read_dir`, que rien ne spécifie. L'ordre suit
    // les codes de caractères : « Étoile » passe donc après « Zeste », pis-aller assumé
    // plutôt qu'une collation complète.
    v.sort_by(|a, b| a.nom.cmp(&b.nom));
    v
}

/// Le chemin du fichier d'une personnalisée.
fn chemin(config: &Path, cle: &str) -> PathBuf {
    repertoire(config).join(format!("{cle}.{EXT}"))
}

/// Le nom de la maquette qui tient déjà cette clé — `soi` exceptée, pour qu'un
/// renommage ne se refuse pas à lui-même quand le slug ne change pas.
fn deja_prise(config: &Path, cle: &str, soi: Option<&str>) -> Option<String> {
    toutes(Some(config))
        .into_iter()
        .find(|m| m.cle == cle && Some(m.cle.as_str()) != soi)
        .map(|m| m.nom)
}

/// La personnalisée de cette clé, ou le refus qui dit pourquoi.
///
/// C'est ici que l'immuabilité des fournies est **réellement** tenue. L'interface qui
/// n'offre pas les boutons est une politesse : une commande s'appelle sans elle, et une
/// liste périmée nomme des clés qui n'existent plus.
fn personnalisee(config: &Path, cle: &str) -> Result<Maquette, String> {
    let m = par_cle(Some(config), cle).ok_or_else(|| format!("maquette inconnue : {cle}"))?;
    if m.fournie {
        return Err(format!(
            "« {} » est une maquette fournie : elle ne se renomme ni ne s'efface.",
            m.nom
        ));
    }
    Ok(m)
}

/// Écrit le fichier d'une personnalisée, sans rien contrôler : c'est l'appelant qui
/// sait s'il crée ou s'il remplace.
fn poser(
    config: &Path,
    cle: &str,
    nom: &str,
    couverture: &Couverture,
    images: &BTreeMap<String, Vec<u8>>,
) -> Result<(), String> {
    let dir = repertoire(config);
    std::fs::create_dir_all(&dir).map_err(|e| {
        format!(
            "répertoire des maquettes inutilisable ({}) : {e}",
            dir.display()
        )
    })?;
    let chemin = chemin(config, cle);
    let f = std::fs::File::create(&chemin)
        .map_err(|e| format!("écriture de {} : {e}", chemin.display()))?;
    ecrire_archive(
        f,
        &Maquette {
            cle: cle.into(),
            nom: nom.into(),
            fournie: false,
            couverture: couverture.clone(),
            images: images.clone(),
        },
    )
}

/// Le slug d'un nom saisi, ou le refus qui dit quoi faire.
fn slug_saisi(nom: &str) -> Result<String, String> {
    slug(nom).ok_or_else(|| {
        format!(
            "« {nom} » ne peut pas nommer une maquette : il y faut au moins une lettre ou un chiffre."
        )
    })
}

/// Enregistre une couverture comme maquette personnalisée.
///
/// **L'écriture échoue fort**, là où la lecture est au mieux : un « Enregistrer » qui
/// échoue en silence perd du travail. Deux refus, et ils disent tous deux quoi faire —
/// un nom qui ne donne aucun slug, et un nom déjà pris.
///
/// L'unicité porte sur l'ensemble, fournies comprises : deux entrées de même clé
/// rendraient la seconde inatteignable par `par_cle`.
///
/// La couverture et les images sont passées telles quelles — c'est l'instantané fidèle
/// de la spec : ce que la maquette emporte est ce qui était à l'écran. La discipline
/// (des images neutres, un résumé de 4ème en jetons) appartient à l'utilisateur ;
/// filtrer demanderait au code de deviner ce qui est générique, et il devinerait mal.
pub fn ecrire(
    config: &Path,
    nom: &str,
    couverture: &Couverture,
    images: &BTreeMap<String, Vec<u8>>,
) -> Result<(), String> {
    let cle = slug_saisi(nom)?;
    if let Some(prise) = deja_prise(config, &cle, None) {
        return Err(format!("« {prise} » porte déjà ce nom."));
    }
    poser(config, &cle, nom, couverture, images)
}

/// Renomme une personnalisée.
///
/// Le slug nommant le fichier, le renommage le **déplace** — sauf quand seule la casse
/// ou la ponctuation change, auquel cas le fichier est réécrit en place et la maquette
/// ne se refuse pas son propre nom.
pub fn renommer(config: &Path, cle: &str, nom: &str) -> Result<(), String> {
    let m = personnalisee(config, cle)?;
    let neuf = slug_saisi(nom)?;
    if let Some(prise) = deja_prise(config, &neuf, Some(cle)) {
        return Err(format!("« {prise} » porte déjà ce nom."));
    }
    poser(config, &neuf, nom, &m.couverture, &m.images)?;
    if neuf != cle {
        let ancien = chemin(config, cle);
        std::fs::remove_file(&ancien)
            .map_err(|e| format!("l'ancien fichier tient encore ({}) : {e}", ancien.display()))?;
    }
    Ok(())
}

/// Efface une personnalisée. Sans reprise : ce que le fichier portait est perdu, et
/// c'est la fenêtre qui demande confirmation.
pub fn effacer(config: &Path, cle: &str) -> Result<(), String> {
    personnalisee(config, cle)?;
    let chemin = chemin(config, cle);
    std::fs::remove_file(&chemin).map_err(|e| format!("effacement de {} : {e}", chemin.display()))
}

/// Les maquettes, dans l'ordre où l'interface les propose.
///
/// **La lecture est au mieux** : une archive illisible est ignorée avec un mot sur la
/// sortie d'erreur — ce qui se perd est un point de départ, et refuser la liste entière
/// coûterait les autres. L'écriture, elle, échoue fort : elle perdrait du travail.
///
/// `config` porte le répertoire de configuration, ou `None` quand il est inatteignable —
/// les fournies restent alors servies, comme les projets récents restent listés.
pub fn toutes(config: Option<&Path>) -> Vec<Maquette> {
    let mut v: Vec<Maquette> = FOURNIES
        .iter()
        .filter_map(|(cle, octets)| {
            lire(std::io::Cursor::new(*octets), cle, true)
                .map_err(|e| eprintln!("maquette fournie « {cle} » illisible : {e}"))
                .ok()
        })
        .collect();
    if let Some(c) = config {
        v.extend(personnalisees(c));
    }
    v
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
        ecrire_archive(Cursor::new(&mut octets), &avant).unwrap();
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
        ecrire_archive(Cursor::new(&mut octets), &m).unwrap();

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

    /// Écrit une archive à la main dans `<config>/maquettes/`, comme le ferait une
    /// version antérieure ou un utilisateur qui déplace ses fichiers.
    fn pose(config: &Path, fichier: &str, nom: &str) {
        let dir = config.join("maquettes");
        std::fs::create_dir_all(&dir).unwrap();
        let m = Maquette {
            cle: String::new(),
            nom: nom.into(),
            fournie: false,
            couverture: fournie("folio"),
            images: BTreeMap::new(),
        };
        ecrire_archive(std::fs::File::create(dir.join(fichier)).unwrap(), &m).unwrap();
    }

    /// Les personnalisées viennent après les fournies, dans l'ordre de leur **nom** —
    /// le seul que l'utilisateur voit. Le menu propose d'abord ce qui est livré, puis ce
    /// qu'on a fait soi-même.
    ///
    /// Les deux fichiers sont nommés à contre-sens, sans quoi le test passerait aussi
    /// bien sans tri : `read_dir` rend ses entrées dans un ordre que rien ne spécifie —
    /// APFS les rend par empreinte, et rend précisément `zzz` avant `zeste`. C'est donc
    /// bien le tri, et non le système de fichiers, qui met « Ma collection » en tête.
    #[test]
    fn les_personnalisees_suivent_les_fournies_dans_l_ordre_de_leur_nom() {
        let dir = tempfile::tempdir().unwrap();
        pose(dir.path(), "zzz.maquette", "Zeste");
        pose(dir.path(), "zeste.maquette", "Ma collection");

        let vues: Vec<(String, String, bool)> = toutes(Some(dir.path()))
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
                ("zeste".to_string(), "Ma collection".to_string(), false),
                ("zzz".to_string(), "Zeste".to_string(), false),
            ]
        );
    }

    /// La clé d'une personnalisée est le nom de son fichier : c'est lui qu'on retrouve
    /// sur le disque, et c'est par lui que le lot 3 la renommera et l'effacera.
    #[test]
    fn une_personnalisee_se_retrouve_par_sa_cle() {
        let dir = tempfile::tempdir().unwrap();
        pose(dir.path(), "ma-collection.maquette", "Ma collection");
        let m = par_cle(Some(dir.path()), "ma-collection").unwrap();
        assert_eq!(m.nom, "Ma collection");
        assert!(!m.fournie);
    }

    /// La lecture est au mieux : ce qui se perd est un point de départ, et refuser la
    /// liste entière pour un fichier de travers coûterait tous les autres.
    #[test]
    fn une_maquette_illisible_n_empeche_pas_les_autres_de_se_lister() {
        let dir = tempfile::tempdir().unwrap();
        pose(dir.path(), "bonne.maquette", "Bonne");
        let d = dir.path().join("maquettes");
        std::fs::write(d.join("cassee.maquette"), b"ceci n'est pas une archive").unwrap();
        // Ce qui ne porte pas l'extension n'est pas même regardé.
        std::fs::write(d.join("notes.txt"), b"rien a voir").unwrap();

        let cles: Vec<String> = toutes(Some(dir.path()))
            .into_iter()
            .map(|m| m.cle)
            .collect();
        assert_eq!(cles, ["folio", "blanche", "surimpression", "bonne"]);
    }

    /// Répertoire de configuration inatteignable, ou aucune personnalisée encore
    /// écrite : les fournies restent servies. Même arbitrage que les projets récents.
    #[test]
    fn sans_configuration_les_fournies_restent() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(toutes(None).len(), 3, "aucun répertoire");
        assert_eq!(toutes(Some(dir.path())).len(), 3, "répertoire vide");
    }

    /// Le refus côté Rust n'est pas une redondance de l'interface qui masque les
    /// boutons : c'est la **seule** garantie réelle de l'immuabilité des fournies.
    /// L'interface n'est qu'une politesse, et une commande s'appelle sans elle.
    #[test]
    fn une_fournie_ne_se_renomme_ni_ne_s_efface() {
        let dir = tempfile::tempdir().unwrap();

        let e = renommer(dir.path(), "folio", "Ma folio").unwrap_err();
        assert!(e.contains("fournie"), "{e}");
        let e = effacer(dir.path(), "folio").unwrap_err();
        assert!(e.contains("fournie"), "{e}");

        assert!(
            par_cle(Some(dir.path()), "folio").is_some(),
            "Folio doit tenir"
        );
    }

    /// Renommer déplace le fichier, puisque le slug le nomme — et la maquette garde
    /// tout ce qu'elle emportait. Ce qui se perdrait ici serait une couverture entière.
    #[test]
    fn renommer_deplace_le_fichier_et_garde_le_contenu() {
        let dir = tempfile::tempdir().unwrap();
        let mut images = BTreeMap::new();
        images.insert("couverture.jpg".to_string(), vec![1, 2, 3]);
        let cv = fournie("surimpression");
        ecrire(dir.path(), "Ma collection", &cv, &images).unwrap();

        renommer(dir.path(), "ma-collection", "Nuit blanche").unwrap();

        assert!(
            par_cle(Some(dir.path()), "ma-collection").is_none(),
            "l'ancien fichier tient encore"
        );
        let m = par_cle(Some(dir.path()), "nuit-blanche").unwrap();
        assert_eq!(m.nom, "Nuit blanche");
        assert_eq!(m.couverture, cv);
        assert_eq!(m.images, images);
    }

    /// Corriger la casse ou la ponctuation d'un nom garde le même slug : la maquette ne
    /// doit pas s'y voir refuser son propre nom, ni disparaître dans l'opération.
    #[test]
    fn se_renommer_sous_le_meme_slug_est_permis() {
        let dir = tempfile::tempdir().unwrap();
        ecrire(
            dir.path(),
            "ma collection",
            &fournie("folio"),
            &BTreeMap::new(),
        )
        .unwrap();

        renommer(dir.path(), "ma-collection", "Ma Collection !").unwrap();

        let m = par_cle(Some(dir.path()), "ma-collection").unwrap();
        assert_eq!(m.nom, "Ma Collection !");
        assert_eq!(
            toutes(Some(dir.path()))
                .iter()
                .filter(|m| !m.fournie)
                .count(),
            1,
            "le renommage a dédoublé la maquette"
        );
    }

    /// L'unicité vaut au renommage comme à l'écriture : deux maquettes de même clé
    /// rendraient la seconde inatteignable, et le renommage écraserait la première.
    #[test]
    fn renommer_vers_un_nom_pris_est_refuse() {
        let dir = tempfile::tempdir().unwrap();
        ecrire(
            dir.path(),
            "Ma collection",
            &fournie("folio"),
            &BTreeMap::new(),
        )
        .unwrap();
        ecrire(
            dir.path(),
            "Nuit blanche",
            &fournie("blanche"),
            &BTreeMap::new(),
        )
        .unwrap();

        let e = renommer(dir.path(), "nuit-blanche", "MA COLLECTION").unwrap_err();
        assert!(e.contains("Ma collection"), "{e}");

        let m = par_cle(Some(dir.path()), "nuit-blanche").unwrap();
        assert_eq!(m.nom, "Nuit blanche", "le refus a quand même renommé");
    }

    /// Effacer retire le fichier, et rien d'autre.
    #[test]
    fn effacer_retire_la_maquette_et_laisse_les_autres() {
        let dir = tempfile::tempdir().unwrap();
        ecrire(
            dir.path(),
            "Ma collection",
            &fournie("folio"),
            &BTreeMap::new(),
        )
        .unwrap();
        ecrire(
            dir.path(),
            "Nuit blanche",
            &fournie("blanche"),
            &BTreeMap::new(),
        )
        .unwrap();

        effacer(dir.path(), "ma-collection").unwrap();

        assert!(par_cle(Some(dir.path()), "ma-collection").is_none());
        assert!(par_cle(Some(dir.path()), "nuit-blanche").is_some());
        assert_eq!(toutes(Some(dir.path())).len(), 4, "fournies comprises");
    }

    /// Une clé qu'aucune maquette ne porte : le geste vient d'une liste périmée, et le
    /// dire vaut mieux que de laisser croire à un effacement qui n'a rien effacé.
    #[test]
    fn une_cle_inconnue_est_refusee_avant_toute_ecriture() {
        let dir = tempfile::tempdir().unwrap();
        let e = effacer(dir.path(), "jamais-vue").unwrap_err();
        assert!(e.contains("jamais-vue"), "{e}");
    }

    /// L'aller-retour complet d'une personnalisée : ce qu'on enregistre est ce qu'on
    /// retrouve, images comprises. C'est la promesse du geste — la couverture réglée
    /// pour un livre resservira au suivant.
    #[test]
    fn une_personnalisee_enregistree_se_recharge_entiere() {
        let dir = tempfile::tempdir().unwrap();
        let mut images = BTreeMap::new();
        images.insert("couverture.jpg".to_string(), vec![0xff, 0xd8, 0xff, 0xe0]);
        let cv = fournie("surimpression");

        ecrire(dir.path(), "Ma collection", &cv, &images).unwrap();

        let m = par_cle(Some(dir.path()), "ma-collection").unwrap();
        assert_eq!(m.nom, "Ma collection");
        assert!(!m.fournie);
        assert_eq!(m.couverture, cv);
        assert_eq!(m.images, images);
    }

    /// L'unicité porte sur **tout** l'ensemble, fournies comprises : une personnalisée
    /// nommée « Folio » ferait deux entrées de même clé dans le menu, et la seconde
    /// serait inatteignable. Le refus nomme celle qui tient déjà la place.
    #[test]
    fn un_nom_deja_pris_est_refuse_fournie_comprise() {
        let dir = tempfile::tempdir().unwrap();
        let cv = fournie("folio");

        let e = ecrire(dir.path(), "Folio", &cv, &BTreeMap::new()).unwrap_err();
        assert!(e.contains("Folio"), "{e}");

        ecrire(dir.path(), "Ma collection", &cv, &BTreeMap::new()).unwrap();
        // Même slug, autre casse et autre ponctuation : c'est le même nom.
        let e = ecrire(dir.path(), "ma  collection !", &cv, &BTreeMap::new()).unwrap_err();
        assert!(e.contains("Ma collection"), "{e}");
    }

    /// Un « Enregistrer » qui échoue perd du travail : il remonte, il ne s'arrange pas
    /// en silence avec un nom que personne n'a choisi.
    #[test]
    fn un_nom_sans_slug_est_refuse_plutot_qu_arrange() {
        let dir = tempfile::tempdir().unwrap();
        let e = ecrire(dir.path(), "  ", &fournie("folio"), &BTreeMap::new()).unwrap_err();
        assert!(e.contains("lettre"), "{e}");
        assert!(
            toutes(Some(dir.path())).iter().all(|m| m.fournie),
            "rien ne doit avoir été écrit"
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
