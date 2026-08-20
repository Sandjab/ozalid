//! Importe un répertoire de travail de l'ancienne chaîne et en fait un `.ozalid`.
//!
//! Sert à faire entrer les livres déjà publiés dans l'application — et, ce faisant, à
//! disposer de matériel de test réel plutôt que de manuscrits fabriqués.
//!
//! Usage : cargo run --example importer -- <livre.toml> <sortie.ozalid>

use std::path::Path;

use ozalid_lib::import;
use ozalid_lib::manuscrit;
use ozalid_lib::projet::Projet;

fn main() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let (toml, sortie) = match (args.next(), args.next()) {
        (Some(a), Some(b)) => (a, b),
        _ => {
            eprintln!("usage : importer <livre.toml> <sortie.ozalid>");
            std::process::exit(2);
        }
    };

    let projet = import::depuis_livre_toml(Path::new(&toml))?;
    let cible = Path::new(&sortie);
    projet.enregistrer(cible)?;

    // Relire ce qu'on vient d'écrire : un import qui ne se rouvre pas ne vaut rien.
    let relu = Projet::ouvrir(cible)?;
    let chapitres = manuscrit::decoupe(&relu.texte, relu.meta.livre.chapitres)?;

    let l = &relu.meta.livre;
    let taille = std::fs::metadata(cible).map(|m| m.len()).unwrap_or(0);
    println!(
        "{} — « {} », {}, {} chapitres, {} mots, {:.2} Mo",
        cible.display(),
        l.titre,
        l.auteur,
        chapitres.len(),
        relu.texte.split_whitespace().count(),
        taille as f64 / 1_048_576.0,
    );
    match &relu.meta.couverture.maquette {
        Some(m) => println!(
            "  couverture : mode {:?}, titre en {} {}, papier {}, cadre {}",
            m.mode,
            m.titre.police,
            m.titre.graisse,
            m.papier,
            if m.cadre.actif { "actif" } else { "éteint" },
        ),
        None => println!("  couverture : aucun réglage de l'atelier dans le PNG"),
    }
    if relu.images.is_empty() {
        println!("  images : aucune photo source embarquée");
    } else {
        for (nom, oct) in &relu.images {
            println!("  image : {nom} ({} Ko)", oct.len() / 1024);
        }
    }
    Ok(())
}
