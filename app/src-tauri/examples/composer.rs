//! Compose l'intérieur d'un projet `.ozalid`, sans interface.
//!
//! Sert à exercer la chaîne entière sur un livre réel — c'est le témoin de
//! non-régression du compte de pages, à rejouer après toute modification de la
//! composition. La fenêtre Tauri n'apporte rien à cette vérification.
//!
//! Usage : cargo run --example composer -- <projet.ozalid> <prestataire> <sortie>

use std::path::{Path, PathBuf};

use ozalid_lib::interieur::{self, Reglage};
use ozalid_lib::manuscrit;
use ozalid_lib::projet::Projet;
use ozalid_lib::providers;
use ozalid_lib::typst::Typst;

fn main() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let (ozalid, cle, sortie) = match (args.next(), args.next(), args.next()) {
        (Some(a), Some(b), Some(c)) => (a, b, c),
        _ => {
            eprintln!("usage : composer <projet.ozalid> <prestataire> <répertoire de sortie>");
            eprintln!(
                "prestataires : {}",
                providers::PROVIDERS
                    .iter()
                    .map(|p| p.cle)
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            std::process::exit(2);
        }
    };

    let pr = providers::provider(&cle).ok_or_else(|| format!("prestataire inconnu : {cle}"))?;
    let projet = Projet::ouvrir(Path::new(&ozalid))?;
    let livre = &projet.meta.livre;
    let int = &projet.meta.interieur;
    // `interieur::source` interpole la police sans échappement : la validation est ici.
    int.verifie()?;
    let chapitres = manuscrit::decoupe(&projet.texte, livre.chapitres)?;

    let dossier = PathBuf::from(&sortie);
    std::fs::create_dir_all(&dossier).map_err(|e| format!("{sortie} : {e}"))?;
    let src = dossier.join(format!("interieur-{}.typ", pr.cle));
    // Les polices embarquées, comme le fait `packager` : sans elles, la police du
    // projet est introuvable et Typst compose dans la sienne, sans rien dire — le
    // témoin de non-régression mesurerait alors un livre qui n'est pas celui-là.
    let typst =
        Typst::new("typst").avec_polices(Path::new(env!("CARGO_MANIFEST_DIR")).join("fonts"));

    let mut passes = 0;
    let r = interieur::converge(pr, |reglage| {
        passes += 1;
        std::fs::write(
            &src,
            interieur::source(livre, int, pr, reglage, &chapitres, None),
        )
        .map_err(|e| e.to_string())?;
        typst.pages(&src)
    })?;

    let reglage = Reglage {
        gouttiere: r.gouttiere,
        blanche: r.blanche,
    };
    std::fs::write(
        &src,
        interieur::source(livre, int, pr, &reglage, &chapitres, None),
    )
    .map_err(|e| e.to_string())?;
    let pdf = dossier.join(format!("interieur-{}.pdf", pr.cle));
    typst.compile(&src, &pdf)?;

    let papier = pr.papier_defaut();
    let dos = match papier.dos.mm(r.pages) {
        Some(mm) => format!("{mm:.2} mm"),
        None => "à relever sur le gabarit".into(),
    };
    println!(
        "{} — {} pages{}, {} chapitres, gouttière {} mm, dos {dos} ({}, {} mesure{})",
        pdf.display(),
        r.pages,
        if r.blanche {
            " (blanche de fin ajoutée)"
        } else {
            ""
        },
        chapitres.len(),
        r.gouttiere,
        pr.cle,
        passes,
        if passes > 1 { "s" } else { "" },
    );
    Ok(())
}
