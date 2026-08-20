//! La planche de couverture : 4ème | dos | 1ère, au gabarit du prestataire.
//!
//! C'est la pièce où le couplage que l'application existe pour tenir devient visible :
//! la largeur du dos vient de la pagination, la pagination vient de la composition de
//! l'intérieur, et le format vient du prestataire. Rien n'y est saisi à la main tant
//! que le prestataire publie ses chiffres.
//!
//! La planche ne porte **aucun trait de coupe ni repère de pli** : Lulu, KDP et
//! Bookvault les refusent explicitement (« Do not include trim/bleed marks »), et le
//! fond perdu suffit à dire où couper. Ce qui aide l'œil vit dans l'épreuve, pas dans
//! le fichier remis à l'imprimeur.

use crate::couverture::{self, Boite, Couverture, FondQuatre, Panorama, PlaceDos, Ressource};
use crate::projet::Livre;
use crate::providers::{Papier, Provider};

/// Ce qu'un prestataire ne publie pas et qu'il a fallu relever sur son gabarit.
/// Vide chez ceux qui publient tout — c'est le cas de la plupart.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Releve {
    pub dos: Option<f64>,
    pub fond_perdu: Option<f64>,
}

/// Les dimensions physiques de la planche, en mm.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Gabarit {
    /// Format d'une couverture rognée.
    pub format: (f64, f64),
    pub dos: f64,
    pub fond_perdu: f64,
}

impl Gabarit {
    /// Gabarit d'un prestataire pour une pagination donnée.
    ///
    /// Le dos et le fond perdu viennent de la table quand le prestataire les publie ;
    /// sinon du relevé de l'utilisateur. À défaut des deux, on refuse : une planche
    /// composée sur un dos inventé se voit au massicot, jamais avant.
    pub fn pour(
        pr: &Provider,
        papier: &Papier,
        pages: u32,
        releve: Releve,
    ) -> Result<Self, String> {
        let dos = papier.dos.mm(pages).or(releve.dos).ok_or_else(|| {
            format!(
                "{} ne publie pas de formule de dos : relever l'épaisseur sur son \
                 gabarit à {pages} pages et la saisir.",
                pr.libelle
            )
        })?;
        let fond_perdu = pr.fond_perdu.or(releve.fond_perdu).ok_or_else(|| {
            format!(
                "{} ne publie pas de fond perdu : le relever sur son gabarit et le saisir.",
                pr.libelle
            )
        })?;
        Ok(Self {
            format: pr.format,
            dos,
            fond_perdu,
        })
    }

    pub fn largeur(&self) -> f64 {
        2.0 * self.format.0 + self.dos + 2.0 * self.fond_perdu
    }

    pub fn hauteur(&self) -> f64 {
        self.format.1 + 2.0 * self.fond_perdu
    }

    /// Abscisse du pli côté 4ème, depuis le bord extérieur gauche.
    fn pli(&self) -> f64 {
        self.fond_perdu + self.format.0
    }

    /// Le prolongement panoramique vu depuis une zone dont le bord gauche est à `x` de
    /// celui de la planche. L'image est cadrée une seule fois, sur la planche entière ;
    /// chaque zone n'en montre que sa part.
    fn panorama(&self, x: f64) -> Panorama {
        Panorama {
            largeur: self.largeur(),
            x_zone: -x,
        }
    }
}

fn mm(v: f64) -> String {
    format!("{v:.4}mm")
}

/// Débord du dos sous les deux faces, en prolongement panoramique, en mm.
///
/// Deux zones découpées bord à bord laissent une couture claire d'un pixel : le
/// rasteriseur adoucit chaque bord de son côté et le fond transparaît entre les deux.
/// Mesuré sur un rendu à 600 ppi, aux deux plis. Le dos est donc élargi d'un cinquième
/// de millimètre de chaque côté et **posé en premier** : les deux faces le recouvrent,
/// et comme les trois portent la même image à la même place, le débord ne se voit pas.
/// Élargir une face plutôt que le dos ne ferait que déplacer la couture sur sa voisine.
/// Hors panorama, il n'a pas lieu d'être : il poserait la couleur du dos sur la 1ère.
const COUTURE: f64 = 0.2;

/// Une zone de la planche, découpée à ses bords : ce qui déborde du dos ne doit pas
/// mordre sur la 1ère, et réciproquement.
fn zone(dx: f64, largeur: f64, hauteur: f64, contenu: &str) -> String {
    format!(
        "#place(top + left, dx: {}, dy: 0mm, box(width: {}, height: {}, clip: true)[\n{contenu}])\n",
        mm(dx),
        mm(largeur),
        mm(hauteur),
    )
}

/// Le dos : fond sur toute la hauteur, texte en lecture de bas en haut.
///
/// Auteur, titre et éditeur s'y placent chacun où sa maquette le dit — au pied, au
/// centre ou en tête — et dans l'ordre que fixe son rang. Le texte est calé sur la
/// couverture **rognée**, pas sur la planche : le fond perdu n'est pas de la surface
/// imprimée utile.
fn bloc_dos(
    livre: &Livre,
    cv: &Couverture,
    g: &Gabarit,
    image_une: Option<&Ressource>,
    couture: f64,
) -> String {
    let (fw, fh) = g.format;
    let d = &cv.dos;
    let fond = if d.fond_propre { &d.fond } else { &cv.papier };

    let mut s = format!(
        "#place(top + left, rect(width: {}, height: {}, fill: rgb(\"{}\")))\n",
        mm(g.dos + 2.0 * couture),
        mm(g.hauteur()),
        fond.replace('"', "")
    );

    // En prolongement panoramique, la photo traverse le dos : sans cette tranche, une
    // couverture panoramique aurait une bande de papier au pli, et elle se verrait sur
    // le livre en main plus sûrement que partout ailleurs.
    if cv.quatrieme.fond == FondQuatre::Panorama {
        if let (Some((zone, geo)), Some(r)) = (
            couverture::image_une(
                cv,
                g.format,
                image_une,
                Boite::une(g.format, g.fond_perdu),
                Some(g.panorama(g.pli() - couture)),
            ),
            image_une,
        ) {
            s.push_str(&couverture::bloc_image(zone, &geo, &r.fichier));
        }
    }

    // Chaque élément est rangé à sa place, puis les éléments d'une même place sont
    // ordonnés par leur rang. Un élément éteint, ou dont le texte est vide, ne laisse
    // pas de trou : c'est ce qui permet de composer un dos sans éditeur, ou un dos qui
    // ne porte que le titre.
    let mut places: [Vec<(u8, String)>; 3] = [Vec::new(), Vec::new(), Vec::new()];
    for (el, texte) in [
        (&d.auteur, livre.auteur.trim()),
        (&d.titre, livre.titre.trim()),
        (&d.editeur, cv.pied.editeur.trim()),
    ] {
        if !el.actif || texte.is_empty() {
            continue;
        }
        let i = match el.place {
            PlaceDos::Pied => 0,
            PlaceDos::Centre => 1,
            PlaceDos::Tete => 2,
        };
        places[i].push((el.rang, format!("#{}", el.style.applique(fw, texte))));
    }
    if places.iter().all(Vec::is_empty) {
        return s;
    }

    let ecart = format!("#h({})", mm(d.ecart / 100.0 * fw));
    let cellules: Vec<String> = places
        .iter_mut()
        .map(|p| {
            p.sort_by_key(|(rang, _)| *rang);
            p.iter()
                .map(|(_, t)| t.as_str())
                .collect::<Vec<_>>()
                .join(&ecart)
        })
        .collect();

    // Cinq colonnes : pied, ressort, centre, ressort, tête. Les ressorts poussent les
    // extrémités contre les bords quel que soit le nombre d'éléments, et le centre
    // reste centré même quand une extrémité est vide.
    s.push_str(&format!(
        "#place(center + horizon, rotate(-90deg, reflow: true, \
         block(width: {}, height: {}, inset: (x: {}))[\n\
         #set align(horizon)\n\
         #grid(columns: (auto, 1fr, auto, 1fr, auto), align: horizon,\n  \
         [{}], [], [{}], [], [{}])\n]))\n",
        mm(fh),
        mm(g.dos),
        mm(d.marge / 100.0 * fw),
        cellules[0],
        cellules[1],
        cellules[2],
    ));
    s
}

/// Source Typst de la planche entière, sur une page unique aux dimensions du gabarit.
pub fn source(
    livre: &Livre,
    cv: &Couverture,
    g: &Gabarit,
    image_une: Option<&Ressource>,
    image_quatre: Option<&Ressource>,
) -> Result<String, String> {
    let fp = g.fond_perdu;
    let (largeur, hauteur) = (g.largeur(), g.hauteur());
    let c = if cv.quatrieme.fond == FondQuatre::Panorama {
        COUTURE
    } else {
        0.0
    };

    let bq = Boite::quatre(g.format, fp);
    let bu = Boite::une(g.format, fp);
    // Chaque zone reçoit le panorama vu de son propre bord gauche : la 4ème depuis 0,
    // la 1ère depuis l'autre côté du dos.
    let quatre = couverture::corps_quatre(
        cv,
        g.format,
        image_quatre,
        image_une,
        Some(g.panorama(0.0)),
        bq,
    )?;
    let une = couverture::corps_une(
        livre,
        cv,
        g.format,
        image_une,
        bu,
        Some(g.panorama(g.pli() + g.dos)),
    );

    let mut s = format!(
        "// Planche — {} × {} mm, dos {} mm, fond perdu {} mm\n\
         #set page(width: {}, height: {}, margin: 0mm)\n\
         #set text(lang: \"fr\", top-edge: 0.75em, bottom-edge: -0.25em)\n\
         #set par(leading: 0em, spacing: 0em, justify: false)\n\n",
        mm(largeur),
        mm(hauteur),
        mm(g.dos),
        mm(fp),
        mm(largeur),
        mm(hauteur),
    );
    // Le dos d'abord, débordant sous les deux faces ; les faces ensuite, qui le
    // recouvrent. Sans panorama, `c` est nul et l'ordre n'a plus d'effet.
    s.push_str(&zone(
        g.pli() - c,
        g.dos + 2.0 * c,
        hauteur,
        &bloc_dos(livre, cv, g, image_une, c),
    ));
    s.push_str(&zone(0.0, bq.largeur, hauteur, &quatre));
    s.push_str(&zone(g.pli() + g.dos, bu.largeur, hauteur, &une));
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::maquettes;
    use crate::providers::provider;

    fn livre() -> Livre {
        Livre {
            titre: "Les Heures creuses".into(),
            titre_page: None,
            auteur: "Ivan Pjig".into(),
            genre: "roman".into(),
            copyright: String::new(),
            chapitres: None,
        }
    }

    fn gabarit(cle: &str, pages: u32) -> Gabarit {
        let pr = provider(cle).unwrap();
        Gabarit::pour(pr, pr.papier_defaut(), pages, Releve::default()).unwrap()
    }

    fn photo() -> Ressource {
        Ressource {
            fichier: "couverture.jpg".into(),
            largeur: 1200,
            hauteur: 1980,
        }
    }

    /// Abscisses des trois zones de la planche, dans l'ordre 4ème, dos, 1ère : ce sont
    /// les seuls placements posés à `dy: 0mm`, tout le reste vit à l'intérieur d'elles.
    fn abscisses_des_zones(s: &str) -> Vec<f64> {
        s.match_indices(", dy: 0mm, box(")
            .map(|(i, _)| {
                s[..i]
                    .rsplit("dx: ")
                    .next()
                    .unwrap()
                    .split("mm")
                    .next()
                    .unwrap()
                    .parse::<f64>()
                    .unwrap()
            })
            .collect()
    }

    /// **Le test qui porte la raison d'être du projet.** Rallonger le manuscrit change
    /// la pagination, donc le dos, donc la largeur de la planche et la position de la
    /// 1ère — sans que personne ne ressaisisse quoi que ce soit. Si ce câblage casse,
    /// l'application ne vaut plus que l'atelier HTML qu'elle remplace.
    #[test]
    fn une_pagination_plus_longue_elargit_la_planche_et_deplace_la_premiere() {
        let court = gabarit("lulu", 244);
        let long = gabarit("lulu", 400);
        let ecart = long.dos - court.dos;
        assert!(ecart > 8.0, "dos passé de {} à {}", court.dos, long.dos);
        assert!((long.largeur() - court.largeur() - ecart).abs() < 1e-9);

        let cv = maquettes::folio();
        let dx = |g: &Gabarit| {
            let s = source(&livre(), &cv, g, Some(&photo()), None).unwrap();
            // Les trois zones de la planche sont les seules posées à `dy: 0mm` ; la
            // 1ère de couverture est la plus à droite des trois.
            abscisses_des_zones(&s).into_iter().fold(f64::MIN, f64::max)
        };
        assert!((dx(&long) - dx(&court) - ecart).abs() < 0.01);
    }

    /// La planche mesure exactement deux couvertures, un dos et deux fonds perdus.
    /// Un millimètre de trop et le prestataire refuse le fichier.
    #[test]
    fn la_planche_mesure_le_gabarit_du_prestataire() {
        let g = gabarit("tbe-110x170", 280);
        assert!((g.dos - 16.8).abs() < 0.01, "dos {}", g.dos);
        assert_eq!(g.fond_perdu, 5.0);
        assert!((g.largeur() - 246.8).abs() < 0.01, "{}", g.largeur());
        assert!((g.hauteur() - 180.0).abs() < 0.01, "{}", g.hauteur());
    }

    /// Chez un prestataire à gabarit, rien ne peut être calculé : l'application doit le
    /// dire et réclamer le relevé, jamais improviser une épaisseur.
    #[test]
    fn un_prestataire_a_gabarit_reclame_le_releve_au_lieu_d_inventer() {
        let pr = provider("coollibri-148x210").unwrap();
        let err = Gabarit::pour(pr, pr.papier_defaut(), 280, Releve::default()).unwrap_err();
        assert!(err.contains("dos"), "{err}");

        let err = Gabarit::pour(
            pr,
            pr.papier_defaut(),
            280,
            Releve {
                dos: Some(17.0),
                fond_perdu: None,
            },
        )
        .unwrap_err();
        assert!(err.contains("fond perdu"), "{err}");

        let g = Gabarit::pour(
            pr,
            pr.papier_defaut(),
            280,
            Releve {
                dos: Some(17.0),
                fond_perdu: Some(3.0),
            },
        )
        .unwrap();
        assert_eq!(g.dos, 17.0);
    }

    /// Le relevé ne doit jamais prendre le pas sur la formule du prestataire : sinon une
    /// valeur saisie une fois survivrait à un changement de pagination.
    #[test]
    fn la_formule_du_prestataire_prime_sur_le_releve() {
        let pr = provider("lulu").unwrap();
        let g = Gabarit::pour(
            pr,
            pr.papier_defaut(),
            244,
            Releve {
                dos: Some(99.0),
                fond_perdu: Some(99.0),
            },
        )
        .unwrap();
        assert!((g.dos - 15.48).abs() < 0.01, "{}", g.dos);
        assert!((g.fond_perdu - 3.175).abs() < 0.01);
    }

    /// Le dos porte l'auteur, le titre et l'éditeur, tournés d'un quart de tour. Le
    /// titre y vient du livre, comme partout ailleurs.
    #[test]
    fn le_dos_porte_l_identite_du_livre_tournee() {
        let cv = maquettes::blanche();
        let s = source(&livre(), &cv, &gabarit("lulu", 244), None, None).unwrap();
        assert!(s.contains("rotate(-90deg"), "dos non tourné");
        assert!(s.contains("Les Heures creuses"));
        assert!(s.contains("Ivan Pjig"));
        assert!(s.contains("ÉDITEUR"), "éditeur du pied absent du dos");
    }

    /// Les trois éléments du dos se règlent séparément : le rang les ordonne au sein
    /// d'une place, la place les envoie d'un bout à l'autre. Une maquette qui met le
    /// titre en tête et l'auteur au pied doit produire exactement cela.
    #[test]
    fn la_place_et_le_rang_ordonnent_les_elements_du_dos() {
        let mut cv = maquettes::blanche();
        let g = gabarit("lulu", 244);

        // Par défaut : éditeur au pied, puis auteur et titre en tête. La source liste
        // les places dans l'ordre pied, centre, tête.
        let s = bloc_dos(&livre(), &cv, &g, None, 0.0);
        let ordre = |s: &str| {
            ["Ivan Pjig", "Les Heures creuses", "ÉDITEUR"].map(|t| s.find(t).unwrap_or(usize::MAX))
        };
        let [auteur, titre, editeur] = ordre(&s);
        assert!(editeur < auteur, "l'éditeur n'est pas au pied");
        assert!(auteur < titre, "titre avant auteur en tête");

        // Rangs inversés : le titre passe devant l'auteur, sans changer de place.
        cv.dos.auteur.rang = 2;
        cv.dos.titre.rang = 1;
        let [auteur, titre, _] = ordre(&bloc_dos(&livre(), &cv, &g, None, 0.0));
        assert!(titre < auteur, "le rang n'ordonne rien");

        // L'auteur renvoyé au pied y rejoint l'éditeur, et le titre reste seul en tête.
        cv.dos.auteur.place = PlaceDos::Pied;
        cv.dos.auteur.rang = 2;
        let [auteur, titre, editeur] = ordre(&bloc_dos(&livre(), &cv, &g, None, 0.0));
        assert!(
            editeur < auteur && auteur < titre,
            "l'auteur n'est pas descendu au pied"
        );
    }

    /// Chaque élément porte son propre style : c'est ce qui permet un titre en
    /// capitales et un éditeur discret sur le même dos.
    #[test]
    fn chaque_element_du_dos_a_son_style() {
        let mut cv = maquettes::folio();
        cv.dos.auteur.style.couleur = "#c00000".into();
        cv.dos.titre.style.casse = crate::couverture::Casse::Capitales;
        cv.dos.editeur.style.taille = 1.8;

        let s = bloc_dos(&livre(), &cv, &gabarit("lulu", 244), None, 0.0);
        assert!(s.contains("#c00000"), "couleur d'auteur ignorée");
        assert!(
            s.contains("#upper[Les Heures creuses]"),
            "casse du titre ignorée"
        );
        assert!(
            s.contains(&format!("size: {}", mm(1.8 / 100.0 * 108.0))),
            "corps de l'éditeur ignoré"
        );
    }

    /// Éteindre un élément le retire sans laisser d'espace : un dos sans mention
    /// d'éditeur est un cas courant, pas une anomalie.
    #[test]
    fn un_element_eteint_ne_parait_pas_sur_le_dos() {
        let mut cv = maquettes::folio();
        cv.dos.editeur.actif = false;
        let s = bloc_dos(&livre(), &cv, &gabarit("lulu", 244), None, 0.0);
        assert!(!s.contains("ÉDITEUR"), "éditeur éteint pourtant composé");
        assert!(
            s.contains("Les Heures creuses"),
            "le reste du dos a disparu"
        );
    }

    /// Un dos sans texte reste un dos : la bande de fond doit être peinte même quand
    /// le livre n'a ni auteur ni éditeur à y porter.
    #[test]
    fn un_dos_sans_texte_garde_son_fond() {
        let mut cv = maquettes::folio();
        cv.pied.editeur = String::new();
        let mut l = livre();
        l.titre = String::new();
        l.auteur = String::new();
        let s = bloc_dos(&l, &cv, &gabarit("lulu", 244), None, 0.0);
        assert!(s.contains("rect("), "{s}");
        assert!(!s.contains("rotate"), "texte émis sans rien à écrire");
    }

    /// La planche ne porte aucun repère : c'est une exigence des prestataires, pas une
    /// préférence. En ajouter ferait rejeter le fichier.
    #[test]
    fn la_planche_ne_porte_aucun_trait_de_coupe() {
        let s = source(
            &livre(),
            &maquettes::surimpression(),
            &gabarit("kdp-6x9", 300),
            Some(&photo()),
            None,
        )
        .unwrap();
        assert!(!s.contains("line("), "trait tracé sur la planche");
        assert!(!s.contains("dash"), "repère en tirets sur la planche");
    }

    /// Une image à fond perdu doit courir jusqu'au bord de la planche, fond perdu
    /// compris : sinon le massicot découvre une bande de papier au bord de la photo.
    #[test]
    fn l_image_a_fond_perdu_couvre_le_fond_perdu() {
        let g = gabarit("lulu", 244);
        let s = source(
            &livre(),
            &maquettes::surimpression(),
            &g,
            Some(&photo()),
            None,
        )
        .unwrap();
        let (fw, fh) = g.format;
        // La zone image de chaque face déborde du fond perdu : une largeur de plus que
        // la couverture rognée, une hauteur de deux de plus.
        let attendue = format!("width: {}", mm(fw + g.fond_perdu));
        assert!(s.contains(&attendue), "zone image non étendue : {attendue}");
        assert!(s.contains(&format!("height: {}", mm(fh + 2.0 * g.fond_perdu))));
    }

    /// Le prolongement panoramique est la seule composition où les trois zones doivent
    /// se raccorder au millimètre. Le critère n'est pas « la 4ème est décalée de tant » :
    /// c'est que la photo occupe **la même place sur la planche** qu'elle soit portée
    /// par la 4ème, par le dos ou par la 1ère. Un écart ici, et la photo saute au pli.
    #[test]
    fn le_panorama_pose_la_meme_image_au_meme_endroit_dans_les_trois_zones() {
        let g = gabarit("lulu", 244);
        let mut cv = maquettes::folio();
        cv.quatrieme.fond = FondQuatre::Panorama;
        let s = source(&livre(), &cv, &g, Some(&photo()), None).unwrap();

        let x = abscisses_absolues_des_images(&s);
        assert_eq!(x.len(), 3, "4ème, dos et 1ère doivent porter la photo");
        for (i, v) in x.iter().enumerate() {
            assert!(
                (v - x[0]).abs() < 0.01,
                "zone {i} : photo à {v} mm au lieu de {} mm",
                x[0]
            );
        }
    }

    /// Position absolue de chaque image sur la planche : abscisse de la zone de planche,
    /// plus celle de la zone image dans la face, plus celle de l'image dans sa zone —
    /// les trois `dx` qui précèdent chaque `image(`.
    fn abscisses_absolues_des_images(s: &str) -> Vec<f64> {
        s.match_indices("image(\"")
            .map(|(i, _)| {
                let avant: Vec<f64> = s[..i]
                    .split("dx: ")
                    .skip(1)
                    .map(|d| d.split("mm").next().unwrap().parse::<f64>().unwrap())
                    .collect();
                avant.iter().rev().take(3).sum()
            })
            .collect()
    }
}
