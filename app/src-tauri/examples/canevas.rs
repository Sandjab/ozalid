//! Exerce ce que le canevas de placement regarde, sans fenêtre.
//!
//! Les trois rendus — le rail de vignettes, la page en grand, l'objet seul — passent par
//! des commandes Tauri, donc par un projet ouvert dans l'atelier. Cet exemple refait le
//! même chemin sur les fonctions de composition, ce qui suffit à vérifier que Typst
//! accepte les sources et que les rendus sortent aux bonnes dimensions.
//!
//! Usage : cargo run --example canevas -- <projet.ozalid> <prestataire>

use std::path::Path;

use ozalid_lib::envoi::Place;
use ozalid_lib::interieur::{self, Quoi, Reglage, Trace};
use ozalid_lib::manuscrit;
use ozalid_lib::projet::Projet;
use ozalid_lib::providers;
use ozalid_lib::typst::Typst;

fn main() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let (ozalid, cle) = match (args.next(), args.next()) {
        (Some(a), Some(b)) => (a, b),
        _ => {
            eprintln!("usage : canevas <projet.ozalid> <prestataire>");
            std::process::exit(2);
        }
    };
    let pr = providers::provider(&cle).ok_or_else(|| format!("prestataire inconnu : {cle}"))?;
    let projet = Projet::ouvrir(Path::new(&ozalid))?;
    let livre = &projet.meta.livre;
    let int = &projet.meta.interieur;
    int.verifie()?;
    let chapitres = manuscrit::decoupe(&projet.texte, livre.chapitres)?;
    let typst =
        Typst::new("typst").avec_polices(Path::new(env!("CARGO_MANIFEST_DIR")).join("fonts"));

    let dossier = std::env::temp_dir().join("ozalid-canevas");
    std::fs::create_dir_all(&dossier).map_err(|e| e.to_string())?;

    let r = Reglage {
        gouttiere: pr.gouttieres[0].2,
        blanche: false,
    };
    let fond = interieur::source(livre, int, pr, &r, &chapitres, None);
    let src = dossier.join("fond.typ");
    std::fs::write(&src, &fond).map_err(|e| e.to_string())?;

    let debut = std::time::Instant::now();
    let vignettes = typst.apercus(&src, &dossier.join("v{p}.png"), 24)?;
    println!(
        "rail      : {} vignettes en {:?}",
        vignettes.len(),
        debut.elapsed()
    );

    let debut = std::time::Instant::now();
    let grande = dossier.join("grand.png");
    typst.apercu(&src, &grande, 3, 150)?;
    println!("page 3    : {:?}", debut.elapsed());

    let place = Place {
        page: 3,
        x: 0.5,
        y: 0.8,
        taille: 0.6,
        angle: -4.0,
    };
    let t = Trace {
        quoi: Quoi::Texte {
            police: "Caveat",
            texte: "À Léa,\nces heures creuses.",
        },
        place: &place,
    };
    let debut = std::time::Instant::now();
    let objet_src = dossier.join("objet.typ");
    std::fs::write(
        &objet_src,
        interieur::source_objet(&t, pr.format.0 * place.taille),
    )
    .map_err(|e| e.to_string())?;
    let objet = dossier.join("objet.png");
    typst.apercu(&objet_src, &objet, 1, 300)?;
    let octets = std::fs::read(&objet).map_err(|e| e.to_string())?;
    let (l, h) = ozalid_lib::image::dimensions(&octets).ok_or("objet non mesurable")?;
    println!(
        "objet     : {l} × {h}, rapport {:.3}, en {:?}",
        h as f64 / l as f64,
        debut.elapsed()
    );
    println!("rendus dans {}", dossier.display());
    Ok(())
}
