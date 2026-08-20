//! Produit les packages d'un projet pour un ou plusieurs prestataires.
//!
//! C'est la chaîne entière en une commande : intérieur composé, pagination mesurée,
//! dos calculé, planche assemblée. Sans interface, donc utilisable pour vérifier que
//! Typst compile ce que le moteur émet — ce qu'aucun test unitaire ne peut faire.
//!
//! Usage : cargo run --example packager -- <projet.ozalid> <sortie> <prestataire…>

use std::path::{Path, PathBuf};

use ozalid_lib::package;
use ozalid_lib::planche::Releve;
use ozalid_lib::projet::Projet;
use ozalid_lib::providers;
use ozalid_lib::typst::Typst;

fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() < 3 {
        eprintln!("usage : packager <projet.ozalid> <répertoire de sortie> <prestataire…>");
        std::process::exit(2);
    }
    let projet = Projet::ouvrir(Path::new(&args[0]))?;
    let racine = PathBuf::from(&args[1]);
    let typst =
        Typst::new("typst").avec_polices(Path::new(env!("CARGO_MANIFEST_DIR")).join("fonts"));

    for cle in &args[2..] {
        let pr = providers::provider(cle).ok_or_else(|| format!("prestataire inconnu : {cle}"))?;
        // Un relevé de secours pour les prestataires à gabarit, afin que l'exemple
        // puisse les traverser aussi ; l'interface, elle, le demande à l'utilisateur.
        let releve = Releve {
            dos: Some(17.0),
            fond_perdu: Some(3.0),
        };
        let p = package::assembler(
            &projet,
            pr,
            pr.papier_defaut(),
            releve,
            &racine.join(pr.cle),
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
        for c in &p.chemins {
            println!("   {c}");
        }
    }
    Ok(())
}
