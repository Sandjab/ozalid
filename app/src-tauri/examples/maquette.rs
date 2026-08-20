//! Rend les trois maquettes en PNG, 1ère et 4ème, pour comparaison visuelle.
//!
//! C'est la vérification qu'aucun test ne peut faire : le cadre, la position du bloc
//! titre, le voile. À rejouer après toute modification du moteur de couverture.
//!
//! Usage : cargo run --example maquette -- <projet.ozalid> <prestataire> <sortie>

use std::path::{Path, PathBuf};

use ozalid_lib::couverture::{self, Ressource};
use ozalid_lib::maquettes;
use ozalid_lib::projet::Projet;
use ozalid_lib::providers;
use ozalid_lib::typst::Typst;

fn main() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let (ozalid, cle, sortie) = match (args.next(), args.next(), args.next()) {
        (Some(a), Some(b), Some(c)) => (a, b, c),
        _ => {
            eprintln!("usage : maquette <projet.ozalid> <prestataire> <répertoire de sortie>");
            std::process::exit(2);
        }
    };

    let pr = providers::provider(&cle).ok_or_else(|| format!("prestataire inconnu : {cle}"))?;
    let projet = Projet::ouvrir(Path::new(&ozalid))?;
    let dossier = PathBuf::from(&sortie);
    std::fs::create_dir_all(&dossier).map_err(|e| format!("{sortie} : {e}"))?;

    // Les images du projet sont écrites à côté de la source : Typst les lit par chemin
    // relatif, comme n'importe quel document.
    let mut une = None;
    let mut quatre = None;
    for (nom, octets) in &projet.images {
        std::fs::write(dossier.join(nom), octets).map_err(|e| format!("{nom} : {e}"))?;
        let r = Ressource::depuis(nom, octets)
            .ok_or_else(|| format!("{nom} : dimensions illisibles (ni PNG ni JPEG)"))?;
        if nom.starts_with("quatrieme") {
            quatre = Some(r);
        } else {
            une = Some(r);
        }
    }

    let typst =
        Typst::new("typst").avec_polices(Path::new(env!("CARGO_MANIFEST_DIR")).join("fonts"));

    // La maquette du projet d'abord, quand il en porte une : c'est elle qu'on compare
    // au livre déjà publié.
    let mut a_rendre = maquettes::toutes();
    if let Some(m) = projet.meta.couverture.maquette.clone() {
        a_rendre.insert(0, ("projet", "Maquette du projet", m));
    }

    for (k, libelle, cv) in a_rendre {
        for (face, src) in [
            (
                "une",
                couverture::source_une(&projet.meta.livre, &cv, pr.format, une.as_ref()),
            ),
            (
                "quatre",
                couverture::source_quatre(
                    &cv,
                    pr.format,
                    quatre.as_ref(),
                    une.as_ref(),
                    Some(15.0),
                )?,
            ),
        ] {
            let typ = dossier.join(format!("{k}-{face}.typ"));
            let png = dossier.join(format!("{k}-{face}.png"));
            std::fs::write(&typ, src).map_err(|e| format!("{} : {e}", typ.display()))?;
            typst.apercu(&typ, &png, 1, 200)?;
            println!("{} — {libelle}, {face}", png.display());
        }
    }
    println!(
        "format {} × {} mm ({})",
        pr.format.0, pr.format.1, pr.libelle
    );
    Ok(())
}
