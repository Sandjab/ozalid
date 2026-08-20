//! Import d'un livre déjà fabriqué avec l'ancienne chaîne.
//!
//! Un `livre.toml` désigne son manuscrit et sa couverture ; la couverture, si elle
//! sort de l'atelier HTML, porte ses propres réglages dans un chunk PNG. Tout ce
//! qu'il faut pour reconstituer un projet est donc là, sans rien ressaisir — et les
//! livres déjà publiés deviennent du matériel de test réel.
//!
//! Convention de chemins reprise telle quelle : ceux du `livre.toml` partent du
//! **parent** du répertoire de travail (`build/`), un chemin absolu est pris tel quel.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::png::{self, ReglagesAtelier};
use crate::projet::{Atelier, Livre, Projet};

#[derive(Debug, Deserialize)]
struct Fichier {
    livre: Section,
}

#[derive(Debug, Deserialize)]
struct Section {
    titre: String,
    #[serde(default)]
    titre_page: Option<String>,
    auteur: String,
    #[serde(default)]
    genre: Option<String>,
    #[serde(default)]
    copyright: String,
    #[serde(default)]
    chapitres: Option<u32>,
    #[serde(default)]
    manuscrit: Option<String>,
    #[serde(default)]
    couverture: Option<String>,
}

/// Ce qu'un `livre.toml` désigne, en plus de l'identité du livre.
#[derive(Debug, Clone, PartialEq)]
pub struct Designations {
    pub manuscrit: String,
    pub couverture: Option<String>,
}

/// Identité et désignations lues dans un `livre.toml`.
pub fn lire_livre_toml(contenu: &str) -> Result<(Livre, Designations), String> {
    let f: Fichier = toml::from_str(contenu).map_err(|e| format!("livre.toml illisible : {e}"))?;
    let s = f.livre;
    let livre = Livre {
        titre: s.titre,
        titre_page: s.titre_page,
        auteur: s.auteur,
        genre: s.genre.unwrap_or_else(|| "roman".into()),
        copyright: s.copyright,
        chapitres: s.chapitres,
    };
    let designations = Designations {
        // Même défaut que la chaîne Python, pour que les répertoires existants passent.
        manuscrit: s.manuscrit.unwrap_or_else(|| "text.md".into()),
        couverture: s.couverture,
    };
    Ok((livre, designations))
}

/// Assemble le projet à partir de ce qui a été lu. Sans accès disque : c'est ici que
/// vivent les décisions, l'orchestration est dans [`depuis_livre_toml`].
pub fn assemble(
    livre: Livre,
    texte: String,
    source_manuscrit: Option<String>,
    reglages: Option<ReglagesAtelier>,
) -> Result<Projet, String> {
    let mut p = Projet::nouveau(livre, texte);
    p.meta.manuscrit.source = source_manuscrit;

    if let Some(r) = reglages {
        let mut champs = BTreeMap::new();
        for (k, v) in r.fields {
            champs.insert(k.clone(), valeur_toml(&k, v)?);
        }
        p.meta.couverture.atelier = Some(Atelier {
            mode: r.mode,
            format: r.format,
            champs,
        });
        for (url, nom) in [(r.image, "couverture"), (r.image4, "quatrieme")] {
            if let Some(url) = url {
                let (ext, octets) = png::data_url(&url)?;
                p.images.insert(format!("{nom}.{ext}"), octets);
            }
        }
    }
    Ok(p)
}

/// Les réglages de l'atelier sont des scalaires : chaînes, booléens de cases à cocher,
/// nombres. Tout autre type signalerait un format qu'on ne comprend pas — le refuser
/// vaut mieux que l'aplatir en chaîne et le relire de travers au jalon 3.
fn valeur_toml(cle: &str, v: serde_json::Value) -> Result<toml::Value, String> {
    match v {
        serde_json::Value::String(s) => Ok(toml::Value::String(s)),
        serde_json::Value::Bool(b) => Ok(toml::Value::Boolean(b)),
        serde_json::Value::Number(n) => n
            .as_f64()
            .map(toml::Value::Float)
            .ok_or_else(|| format!("réglage « {cle} » : nombre non représentable")),
        autre => Err(format!(
            "réglage « {cle} » de type inattendu : {autre} — import interrompu."
        )),
    }
}

/// Importe le répertoire de travail qui contient ce `livre.toml`.
pub fn depuis_livre_toml(chemin: &Path) -> Result<Projet, String> {
    let contenu =
        std::fs::read_to_string(chemin).map_err(|e| format!("{} : {e}", chemin.display()))?;
    let (livre, d) = lire_livre_toml(&contenu)?;

    let repertoire = chemin
        .parent()
        .ok_or_else(|| format!("{} : pas de répertoire parent", chemin.display()))?;
    let racine = repertoire.parent().unwrap_or(repertoire);

    // Chemin absolu : il est mémorisé dans le projet pour « Réimporter le manuscrit »,
    // et le `.ozalid` se déplace. Un chemin relatif au répertoire d'exécution de
    // l'import ne voudrait plus rien dire dès la première réouverture.
    let manuscrit = absolu(&resout(racine, &d.manuscrit), "manuscrit")?;
    let texte = std::fs::read_to_string(&manuscrit)
        .map_err(|e| format!("manuscrit illisible ({}) : {e}", manuscrit.display()))?;

    let reglages = match &d.couverture {
        Some(c) => {
            let png_path = absolu(&resout(racine, c), "couverture")?;
            let octets = std::fs::read(&png_path)
                .map_err(|e| format!("couverture illisible ({}) : {e}", png_path.display()))?;
            png::reglages(&octets)?
        }
        None => None,
    };

    assemble(
        livre,
        texte,
        Some(manuscrit.to_string_lossy().into_owned()),
        reglages,
    )
}

fn resout(racine: &Path, chemin: &str) -> PathBuf {
    // `join` ignore la racine quand le chemin est absolu : exactement la règle voulue.
    racine.join(chemin)
}

fn absolu(chemin: &Path, quoi: &str) -> Result<PathBuf, String> {
    std::fs::canonicalize(chemin)
        .map_err(|e| format!("{quoi} introuvable ({}) : {e}", chemin.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Le répertoire courant est global au processus, et les tests s'exécutent en
    /// parallèle : tout test qui le déplace doit prendre ce verrou, sinon il fait
    /// dérailler les autres de façon intermittente.
    static CWD: std::sync::Mutex<()> = std::sync::Mutex::new(());

    const TOML: &str = r#"
[livre]
titre = "Les Heures creuses"
titre_page = "Les Heures\ncreuses"
auteur = "Ivan Pjig"
genre = "roman"
copyright = """© Ivan Pjig, 2026.
Tous droits réservés."""
chapitres = 64
manuscrit = "in/texts/WIP7.md"
couverture = "in/covers/LHC-Photo.png"
"#;

    #[test]
    fn un_livre_toml_complet_donne_identite_et_designations() {
        let (l, d) = lire_livre_toml(TOML).unwrap();
        assert_eq!(l.titre, "Les Heures creuses");
        assert_eq!(l.titre_page(), "Les Heures\ncreuses");
        assert_eq!(l.chapitres, Some(64));
        assert!(l.copyright.contains("Tous droits réservés"));
        assert_eq!(d.manuscrit, "in/texts/WIP7.md");
        assert_eq!(d.couverture.as_deref(), Some("in/covers/LHC-Photo.png"));
    }

    /// Le genre et le manuscrit ont des défauts hérités de la chaîne Python : les
    /// répertoires de travail existants doivent s'importer sans être retouchés.
    #[test]
    fn les_defauts_de_la_chaine_existante_sont_respectes() {
        let (l, d) = lire_livre_toml("[livre]\ntitre = \"T\"\nauteur = \"A\"\n").unwrap();
        assert_eq!(l.genre, "roman");
        assert_eq!(d.manuscrit, "text.md");
        assert_eq!(d.couverture, None);
    }

    #[test]
    fn un_livre_toml_sans_auteur_est_refuse() {
        let err = lire_livre_toml("[livre]\ntitre = \"T\"\n").unwrap_err();
        assert!(err.contains("livre.toml illisible"), "{err}");
    }

    /// Les chemins partent du parent du répertoire de travail — « in/texts/x.md »
    /// désigne une ressource partagée, pas un fichier du répertoire lui-même.
    #[test]
    fn un_chemin_relatif_part_du_parent_du_repertoire_de_travail() {
        let racine = Path::new("/dev/ozalid/build");
        assert_eq!(
            resout(racine, "in/texts/WIP7.md"),
            PathBuf::from("/dev/ozalid/build/in/texts/WIP7.md")
        );
    }

    #[test]
    fn un_chemin_absolu_est_pris_tel_quel() {
        let racine = Path::new("/dev/ozalid/build");
        assert_eq!(
            resout(racine, "/ailleurs/roman.md"),
            PathBuf::from("/ailleurs/roman.md")
        );
    }

    /// Un PNG de l'atelier apporte les réglages ET les photos source : c'est ce qui
    /// évite de tout ressaisir. Les types des réglages sont préservés au passage.
    #[test]
    fn les_reglages_et_la_photo_du_png_entrent_dans_le_projet() {
        let b64 = {
            use base64::Engine;
            base64::engine::general_purpose::STANDARD.encode([0xFF, 0xD8, 0xFF])
        };
        let r = ReglagesAtelier {
            app: png::MOT_CLE.into(),
            v: 1,
            mode: "band".into(),
            format: vec![108.0, 178.0],
            fields: BTreeMap::from([
                ("inFormat".into(), serde_json::json!("108,178")),
                ("inFrameOn".into(), serde_json::json!(false)),
                ("inPadX".into(), serde_json::json!(7)),
            ]),
            image: Some(format!("data:image/jpeg;base64,{b64}")),
            image4: None,
        };
        let p = assemble(
            Livre {
                titre: "T".into(),
                titre_page: None,
                auteur: "A".into(),
                genre: "roman".into(),
                copyright: String::new(),
                chapitres: None,
            },
            "## 01\n\nA.\n".into(),
            Some("/travail/roman.md".into()),
            Some(r),
        )
        .unwrap();

        let a = p.meta.couverture.atelier.unwrap();
        assert_eq!(a.mode, "band");
        assert_eq!(a.champs["inFrameOn"], toml::Value::Boolean(false));
        assert_eq!(a.champs["inPadX"], toml::Value::Float(7.0));
        // La photo suit son type déclaré, pas l'extension du PNG hôte.
        assert_eq!(p.images["couverture.jpg"], vec![0xFF, 0xD8, 0xFF]);
        assert!(!p.images.contains_key("quatrieme.png"));
        assert_eq!(
            p.meta.manuscrit.source.as_deref(),
            Some("/travail/roman.md")
        );
    }

    /// Un réglage d'un type qu'on ne comprend pas interrompt l'import. L'aplatir en
    /// chaîne donnerait une maquette silencieusement fausse au jalon 3.
    #[test]
    fn un_reglage_de_type_inattendu_interrompt_l_import() {
        let err = valeur_toml("inTruc", serde_json::json!(["a", "b"])).unwrap_err();
        assert!(err.contains("inTruc"), "{err}");
        assert!(err.contains("import interrompu"), "{err}");
    }

    /// Le chemin du manuscrit est mémorisé pour « Réimporter », et le `.ozalid` se
    /// déplace : un chemin relatif au répertoire d'où l'import a été lancé ne
    /// désignerait plus rien à la réouverture.
    #[test]
    fn la_source_memorisee_du_manuscrit_est_absolue() {
        let tmp = tempfile::tempdir().unwrap();
        let racine = tmp.path();
        std::fs::create_dir_all(racine.join("in/texts")).unwrap();
        std::fs::create_dir_all(racine.join("travail")).unwrap();
        std::fs::write(racine.join("in/texts/roman.md"), "## 01 - Un\n\nTexte.\n").unwrap();
        let toml = racine.join("travail/livre.toml");
        std::fs::write(
            &toml,
            "[livre]\ntitre = \"T\"\nauteur = \"A\"\nmanuscrit = \"in/texts/roman.md\"\n",
        )
        .unwrap();

        // Chemin relatif volontairement passé à l'import, comme depuis une ligne de
        // commande lancée ailleurs.
        let _verrou = CWD.lock().unwrap_or_else(|e| e.into_inner());
        let precedent = std::env::current_dir().unwrap();
        std::env::set_current_dir(racine).unwrap();
        let p = depuis_livre_toml(Path::new("travail/livre.toml"));
        std::env::set_current_dir(precedent).unwrap();

        let source = p.unwrap().meta.manuscrit.source.unwrap();
        assert!(
            Path::new(&source).is_absolute(),
            "source relative : {source}"
        );
        assert!(source.ends_with("in/texts/roman.md"), "{source}");
    }

    /// Un manuscrit désigné mais absent doit être signalé avec son chemin : c'est la
    /// panne courante quand un répertoire de travail a été déplacé.
    #[test]
    fn un_manuscrit_absent_est_signale_avec_son_chemin() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("travail")).unwrap();
        let toml = tmp.path().join("travail/livre.toml");
        std::fs::write(
            &toml,
            "[livre]\ntitre = \"T\"\nauteur = \"A\"\nmanuscrit = \"in/texts/absent.md\"\n",
        )
        .unwrap();
        let err = depuis_livre_toml(&toml).unwrap_err();
        assert!(err.contains("manuscrit introuvable"), "{err}");
        assert!(err.contains("absent.md"), "{err}");
    }

    #[test]
    fn un_livre_sans_png_de_l_atelier_s_importe_sans_couverture() {
        let p = assemble(
            Livre {
                titre: "T".into(),
                titre_page: None,
                auteur: "A".into(),
                genre: "roman".into(),
                copyright: String::new(),
                chapitres: None,
            },
            "## 01\n\nA.\n".into(),
            None,
            None,
        )
        .unwrap();
        assert!(p.meta.couverture.atelier.is_none());
        assert!(p.images.is_empty());
    }
}
