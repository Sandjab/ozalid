//! Tire l'épreuve de relecture d'un projet `.ozalid`, sans interface.
//!
//! C'est le seul moyen de vérifier que Typst avale ce que le module émet — aucun test
//! unitaire ne compile de PDF.
//!
//! Usage : cargo run --example epreuve -- <projet.ozalid> <sortie.pdf> [corps_pt]

use std::path::{Path, PathBuf};

use ozalid_lib::epreuve;
use ozalid_lib::manuscrit;
use ozalid_lib::projet::Projet;
use ozalid_lib::typst::Typst;

fn main() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let (ozalid, sortie) = match (args.next(), args.next()) {
        (Some(a), Some(b)) => (a, b),
        _ => {
            eprintln!("usage : epreuve <projet.ozalid> <sortie.pdf> [corps_pt]");
            std::process::exit(2);
        }
    };
    let corps: f64 = args
        .next()
        .map_or(Ok(12.0), |c| c.parse())
        .map_err(|_| "corps illisible : attendu un nombre de points, par exemple 12".to_string())?;

    let projet = Projet::ouvrir(Path::new(&ozalid))?;
    let livre = &projet.meta.livre;
    let int = &projet.meta.interieur;
    // `epreuve::source` interpole la police sans échappement : la validation est ici.
    int.verifie()?;
    let chapitres = manuscrit::decoupe(&projet.texte, livre.chapitres)?;

    let pdf = PathBuf::from(&sortie);
    let src = pdf.with_extension("typ");
    std::fs::write(&src, epreuve::source(livre, int, &chapitres, corps))
        .map_err(|e| format!("{} : {e}", src.display()))?;

    // Les polices embarquées, comme `composer` et `packager` : sans elles, la police du
    // projet est introuvable et Typst compose dans la sienne, sans rien dire.
    let typst =
        Typst::new("typst").avec_polices(Path::new(env!("CARGO_MANIFEST_DIR")).join("fonts"));
    typst.compile(&src, &pdf)?;
    println!(
        "{} — {} chapitres, {} en {corps} pt",
        pdf.display(),
        chapitres.len(),
        int.police
    );
    Ok(())
}
