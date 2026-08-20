//! Compose le manuscrit-témoin et vérifie que la pagination n'a pas bougé.
//!
//! Le témoin est *Candide* (Voltaire, 1759), du domaine public, récupéré depuis Project
//! Gutenberg et mis au format du projet. Il n'est pas là pour se lire : `build/` n'étant
//! pas versionné, c'est le seul livre que l'intégration continue puisse composer.
//!
//! Ce qu'il prouve, et qu'aucun test unitaire ne peut prouver : Typst compose le même
//! nombre de pages sur macOS et sur Windows. Un écart invaliderait la promesse centrale
//! du projet — un dos calculé sur une plateforme ne vaudrait que pour elle.
//!
//! Le gabarit est `bod`, et non `lulu` : la table Lulu ne porte pas de tranche de
//! gouttière sous 151 pages, et la compléter pour les besoins d'un test reviendrait à
//! laisser le test dicter la production.
//!
//! Usage : cargo run --example temoin [répertoire de sortie]

use std::path::{Path, PathBuf};

use ozalid_lib::maquettes;
use ozalid_lib::package;
use ozalid_lib::planche::Releve;
use ozalid_lib::projet::{Livre, Projet};
use ozalid_lib::providers;
use ozalid_lib::typst::Typst;

const PROVIDER: &str = "bod";

/// Pagination attendue du témoin.
///
/// Relevée sur macOS avec Typst 0.15.1 et EB Garamond, au corps et à l'interligne que
/// `providers` fixe pour BoD. Elle dépend de chacun de ces éléments : la déplacer est un
/// acte délibéré, à revalider sur un livre réel — jamais un ajustement pour faire passer
/// l'intégration continue.
const PAGES_ATTENDUES: u32 = 98;

fn main() -> Result<(), String> {
    let sortie = std::env::args()
        .nth(1)
        .map_or_else(|| std::env::temp_dir().join("ozalid-temoin"), PathBuf::from);

    let livre = Livre {
        titre: "Candide".into(),
        titre_page: None,
        auteur: "Voltaire".into(),
        genre: "conte philosophique".into(),
        copyright: "Texte du domaine public.".into(),
        chapitres: Some(30),
    };
    let mut projet = Projet::nouveau(livre, include_str!("../temoin/manuscrit.md").to_string());
    // La Blanche est purement typographique : le témoin traverse la planche entière sans
    // qu'une seule image ait à être versionnée.
    projet.meta.couverture.maquette = Some(maquettes::blanche());

    let pr = providers::provider(PROVIDER).ok_or("prestataire inconnu : bod")?;
    let typst =
        Typst::new("typst").avec_polices(Path::new(env!("CARGO_MANIFEST_DIR")).join("fonts"));
    let p = package::assembler(
        &projet,
        pr,
        pr.papier_defaut(),
        // BoD publie son dos et son fond perdu : le relevé est ignoré.
        Releve::default(),
        &sortie,
        &typst,
    )?;

    println!(
        "{} — {} pages, gouttière {:.1} mm, dos {:.2} mm, planche {:.2} × {:.2} mm{}",
        p.libelle,
        p.pages,
        p.gouttiere,
        p.dos,
        p.planche.0,
        p.planche.1,
        if p.blanche {
            ", blanche de parité"
        } else {
            ""
        }
    );
    if p.pages != PAGES_ATTENDUES {
        return Err(format!(
            "pagination déplacée : {} pages, {PAGES_ATTENDUES} attendues.\n\
             Si le changement est voulu — police, gabarit, version de Typst —, relever la \
             nouvelle valeur et la figer dans PAGES_ATTENDUES. Sinon, cette plateforme ne \
             compose pas comme l'autre, et aucun dos calculé ici ne vaut ailleurs.",
            p.pages
        ));
    }
    Ok(())
}
