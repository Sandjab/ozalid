//! Génère les ebooks locaux d'un projet `.ozalid`, sans interface.
//!
//! C'est le seul moyen de vérifier ce qu'aucun test ne peut dire : que Typst avale la
//! source à couverture insérée, et qu'une liseuse ouvre l'archive.
//!
//! Usage : cargo run --example ebook -- <projet.ozalid> <sortie> [prestataire]

use std::path::{Path, PathBuf};

use ozalid_lib::ebook;
use ozalid_lib::projet::Projet;
use ozalid_lib::providers;
use ozalid_lib::typst::Typst;

fn main() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let (ozalid, sortie) = match (args.next(), args.next()) {
        (Some(a), Some(b)) => (a, b),
        _ => {
            eprintln!("usage : ebook <projet.ozalid> <sortie> [prestataire]");
            std::process::exit(2);
        }
    };
    let projet = Projet::ouvrir(Path::new(&ozalid))?;

    // Le gabarit vient du destinataire visé, comme dans l'application. L'argument n'est
    // là que pour en essayer un autre sans toucher au projet.
    let cle = args.next();
    let d = projet
        .meta
        .livraison
        .courant()
        .ok_or("aucun destinataire dans ce projet.")?;
    let cle = cle.unwrap_or_else(|| d.provider.clone());
    let pr = providers::provider(&cle).ok_or_else(|| format!("prestataire inconnu : {cle}"))?;

    // Les polices embarquées, comme `composer` et `epreuve` : sans elles, la police du
    // projet est introuvable et le PDF part en repli, l'EPUB dans l'écriture du lecteur.
    let typst =
        Typst::new("typst").avec_polices(Path::new(env!("CARGO_MANIFEST_DIR")).join("fonts"));
    let r = ebook::generer(&projet, pr, d.dos_mm, &PathBuf::from(&sortie), &typst)?;

    println!("{}  ({} Ko)", r.pdf, r.octets_pdf / 1024);
    println!("{}  ({} Ko)", r.epub, r.octets_epub / 1024);
    if let Some(p) = &r.police_non_embarquee {
        println!("police « {p} » introuvable : EPUB dans l'écriture du lecteur.");
    }
    if !r.polices_introuvables.is_empty() {
        println!(
            "composé par repli : {}. Le PDF ne suit pas la maquette.",
            r.polices_introuvables.join(", ")
        );
    }
    Ok(())
}
