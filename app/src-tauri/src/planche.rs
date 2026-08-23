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

use crate::couverture::{
    self, Boite, Couverture, ElementDos, FondQuatre, Panorama, PlaceDos, Ressource,
};
use crate::projet::Livre;
use crate::providers::{Papier, Provider};
use serde::Serialize;

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

    /// La part que le fond perdu prend sur la largeur et sur la hauteur de la planche,
    /// en fraction de celle-ci.
    ///
    /// C'est la mesure dont l'aperçu a besoin pour marquer la coupe sur une image qu'il
    /// affiche à une taille quelconque : les millimètres n'y survivent pas, les
    /// proportions oui. Deux fractions et non une : une planche est bien plus large que
    /// haute, et le même fond perdu n'y pèse pas pareil.
    pub fn part_fond_perdu(&self) -> (f64, f64) {
        (
            self.fond_perdu / self.largeur(),
            self.fond_perdu / self.hauteur(),
        )
    }

    /// Abscisse du pli côté 4ème, depuis le bord extérieur gauche.
    fn pli(&self) -> f64 {
        self.fond_perdu + self.format.0
    }

    /// Les deux plis, en fraction de la largeur de la planche : celui de la 4ème, puis
    /// celui de la 1ère.
    ///
    /// L'aperçu en a besoin pour la même raison que du fond perdu : un dos dont le fond
    /// est celui du papier ne se distingue d'aucune des deux faces, et la planche paraît
    /// alors d'un seul tenant. C'est précisément la maquette où le dos est le plus facile
    /// à rater — celle où rien ne montre où le livre se plie.
    pub fn plis(&self) -> (f64, f64) {
        let l = self.largeur();
        (self.pli() / l, (self.pli() + self.dos) / l)
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

/// Ce qui sépare le texte du dos de chacun de ses deux plis, en mm.
///
/// Un dos carré collé ne se plie pas au trait près : un texte calé contre le pli passe
/// sur la face au premier exemplaire mal plié. Aucun gabarit de la table ne publie
/// cette valeur — c'est donc un choix, pris large plutôt que juste, et le seul endroit
/// où le reprendre.
const JEU_PLI: f64 = 1.0;

/// Les éléments que le dos compose réellement, avec leur texte.
///
/// Un élément éteint, ou dont le texte est vide, ne laisse pas de trou sur le dos :
/// c'est ce qui permet de composer un dos sans éditeur. [`bloc_dos`] et [`dos_requis`]
/// lisent la même liste, sans quoi une maquette serait jugée sur un auteur qu'elle ne
/// porte pas.
fn composes<'a>(
    livre: &'a Livre,
    cv: &'a Couverture,
) -> Vec<(&'static str, &'a ElementDos, &'a str)> {
    [
        ("auteur", &cv.dos.auteur, livre.auteur.trim()),
        ("titre", &cv.dos.titre, livre.titre.trim()),
        ("editeur", &cv.dos.editeur, cv.pied.editeur.trim()),
    ]
    .into_iter()
    .filter(|(_, el, texte)| el.actif && !texte.is_empty())
    .collect()
}

/// La rangée d'une place, dans l'ordre de lecture du dos : du pied vers la tête.
fn rangee(p: PlaceDos) -> usize {
    match p {
        PlaceDos::Pied => 0,
        PlaceDos::Centre => 1,
        PlaceDos::Tete => 2,
    }
}

/// Ce qu'un élément du dos occupe le long du dos, en fraction de sa longueur.
///
/// Des fractions, comme les repères de la planche et pour la même raison : l'aperçu du
/// dos s'affiche à la largeur que la fenêtre lui laisse, et seules des proportions y
/// survivent. Le sens est celui de l'aperçu couché, de gauche à droite — donc du pied
/// vers la tête, ce que la double rotation de `source_dos` produit.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BoiteDos {
    pub cle: &'static str,
    pub debut: f64,
    pub fin: f64,
}

/// La source qui mesure les trois textes du dos, et ne compose rien.
///
/// Typst est le seul à savoir ce qu'un texte occupe : la longueur d'une ligne dépend de
/// chaque glyphe, là où sa hauteur d'encre ne dépend que de la famille — c'est pourquoi
/// [`dos_requis`] se contente d'une table et pourquoi ceci n'en peut pas. La source ne
/// rend aucune page : `typst eval` la mesure et rend un objet JSON, en quelques
/// millisecondes.
pub fn source_mesures(livre: &Livre, cv: &Couverture, format: (f64, f64)) -> String {
    let (fw, _) = format;
    let champs: Vec<String> = composes(livre, cv)
        .iter()
        .map(|(cle, el, texte)| {
            format!(
                "{cle}: measure({}).width / 1mm",
                el.style.applique(fw, texte)
            )
        })
        .collect();
    format!(
        "#set page(width: 1000mm, height: 20mm, margin: 0mm)\n\
         #set text(lang: \"fr\", top-edge: 0.75em, bottom-edge: -0.25em)\n\
         #context [\n  #metadata(({})) <mesures>\n]\n",
        champs.join(", ")
    )
}

/// Où chaque élément du dos tombe, à partir des longueurs que Typst a mesurées.
///
/// La règle est celle de [`bloc_dos`], relue ici depuis l'autre bout : cinq colonnes —
/// pied, ressort, centre, ressort, tête — dans un bloc long comme le dos et retiré de
/// sa marge aux deux extrémités. Le pied se cale contre le début, la tête contre la
/// fin, le centre reste centré, et les éléments d'une même place se suivent séparés de
/// l'écart. Une place dont la longueur dépasse ce que le dos offre déborde ici comme
/// elle déborde là-bas : rien n'est ramené dans les bornes, sans quoi la prise se
/// poserait où le texte n'est pas.
pub fn boites_dos(
    livre: &Livre,
    cv: &Couverture,
    format: (f64, f64),
    mesures: &std::collections::BTreeMap<String, f64>,
) -> Vec<BoiteDos> {
    let (fw, fh) = format;
    let marge = cv.dos.marge / 100.0 * fw;
    let ecart = cv.dos.ecart / 100.0 * fw;

    let mut places: [Vec<(u8, &'static str, f64)>; 3] = [Vec::new(), Vec::new(), Vec::new()];
    for (cle, el, _) in composes(livre, cv) {
        let l = mesures.get(cle).copied().unwrap_or(0.0);
        places[rangee(el.place)].push((el.rang, cle, l));
    }

    for p in places.iter_mut() {
        p.sort_by_key(|(rang, _, _)| *rang);
    }
    let longueurs: Vec<f64> = places
        .iter()
        .map(|p| {
            p.iter().map(|(_, _, l)| l).sum::<f64>() + ecart * (p.len().saturating_sub(1)) as f64
        })
        .collect();
    // Le bloc est retiré de sa marge aux deux bouts, et les deux ressorts se partagent
    // à parts égales ce qui reste. Le centre n'est donc **pas** centré sur le dos dès
    // que le pied et la tête ne pèsent pas pareil : il l'est sur ce que les deux
    // ressorts laissent. Le croire centré le décale de la moitié de leur différence —
    // sept millimètres sur un dos de poche, et la prise se pose à côté du titre.
    let libre = fh - 2.0 * marge - longueurs.iter().sum::<f64>();

    let mut out = Vec::new();
    for (i, p) in places.iter().enumerate() {
        if p.is_empty() {
            continue;
        }
        let mut u = match i {
            0 => marge,
            1 => marge + longueurs[0] + libre / 2.0,
            _ => fh - marge - longueurs[2],
        };
        for (_, cle, l) in p.iter() {
            out.push(BoiteDos {
                cle,
                debut: u / fh,
                fin: (u + l) / fh,
            });
            u += l + ecart;
        }
    }
    out
}

/// Épaisseur de dos que la maquette réclame, en mm : sa ligne la plus haute, plus le
/// jeu de pli de part et d'autre. Zéro quand le dos ne porte aucun texte.
///
/// Le corps des textes de dos est un pourcentage de la **largeur de couverture** —
/// c'est ce qui rend une maquette portable d'un format à l'autre. L'épaisseur du dos,
/// elle, vient de la pagination et du papier. Les deux ne s'accordent par aucune
/// construction : la même maquette réclame un dos plus épais en grand format qu'en
/// poche, quand un grand format pagine justement plus court. C'est cet écart-là qu'on
/// mesure, et c'est le prix de « une maquette pour tous les formats ».
///
/// Les éléments d'un même dos se rangent côte à côte le long du dos, jamais l'un sous
/// l'autre : c'est donc le plus haut qui commande, pas leur somme.
pub fn dos_requis(livre: &Livre, cv: &Couverture, largeur: f64) -> f64 {
    let els = composes(livre, cv);
    if els.is_empty() {
        return 0.0;
    }
    els.iter()
        .map(|(_, el, _)| el.style.encre_mm(largeur))
        .fold(0.0, f64::max)
        + 2.0 * JEU_PLI
}

/// L'épaisseur réclamée, **et seulement quand le dos ne l'offre pas**. `None`, le texte
/// tient.
///
/// Sans cette mesure, un dos trop mince ne se voit nulle part : [`zone`] compose avec
/// `clip: true`, donc le titre est coupé net, sans erreur et sans message, sur le PDF
/// qui part à l'impression.
pub fn dos_insuffisant(livre: &Livre, cv: &Couverture, largeur: f64, dos: f64) -> Option<f64> {
    let requis = dos_requis(livre, cv, largeur);
    (requis > dos).then_some(requis)
}

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
    // ordonnés par leur rang. Ceux qui ne composent pas — éteints ou sans texte — sont
    // déjà écartés par `composes` : c'est ce qui permet de composer un dos sans
    // éditeur, ou un dos qui ne porte que le titre.
    let mut places: [Vec<(u8, String)>; 3] = [Vec::new(), Vec::new(), Vec::new()];
    for (_, el, texte) in composes(livre, cv) {
        places[rangee(el.place)].push((el.rang, format!("#{}", el.style.applique(fw, texte))));
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

/// Source Typst du dos seul, couché sur une page d'un quart de tour.
///
/// Un aperçu de réglage, pas une sortie : **pas de fond perdu**. Ce qui se règle ici est
/// le dos rogné, celui qu'on aura sous les yeux le livre en main, et le gabarit fictif
/// que voici le dit — un fond perdu nul, et le panorama calé sur cette planche-là. C'est
/// aussi ce qui rend la face disponible chez un prestataire qui ne publie pas son fond
/// perdu, là où la planche entière le réclame.
///
/// Le dos est **composé** à sa taille — treize millimètres restent treize millimètres —
/// et c'est la page qui est couchée. Debout, il ne tiendrait à l'écran que par sa
/// hauteur, et sa largeur, seule dimension qui compte pour régler trois textes, se
/// retrouverait à trente-neuf pixels : trois de plus que sur la planche, pour une face
/// entière. Couché, il prend la largeur de la fenêtre et en fait soixante-deux.
///
/// Coucher ici plutôt qu'à l'affichage n'est pas un détail de commodité : une image
/// tournée en CSS garde la boîte de mise en page qu'elle avait debout, et la scène se
/// dimensionnerait sur une hauteur que l'œil ne voit plus.
///
/// Le quart de tour est horaire, et c'est le seul qui remette le dos à l'endroit : ses
/// textes se lisent de bas en haut — `bloc_dos` les tourne de -90° — et l'inverse les
/// rendrait tête-bêche, à lire de droite à gauche. Pied à gauche, tête à droite.
pub fn source_dos(
    livre: &Livre,
    cv: &Couverture,
    format: (f64, f64),
    dos: f64,
    image_une: Option<&Ressource>,
) -> String {
    let g = Gabarit {
        format,
        dos,
        fond_perdu: 0.0,
    };
    let hauteur = g.hauteur();
    // Le box tourne autour de son coin haut-gauche : il part alors vers la gauche de la
    // page, et `dx` l'y ramène entier. Aucune couture à prévoir — elle n'existe que là
    // où deux zones se touchent.
    format!(
        "// Dos seul, couché — {} × {} mm\n\
         #set page(width: {}, height: {}, margin: 0mm)\n\
         #set text(lang: \"fr\", top-edge: 0.75em, bottom-edge: -0.25em)\n\
         #set par(leading: 0em, spacing: 0em, justify: false)\n\n\
         #place(top + left, dx: {}, dy: 0mm, \
         rotate(90deg, origin: top + left, \
         box(width: {}, height: {}, clip: true)[\n{}]))\n",
        mm(dos),
        mm(hauteur),
        mm(hauteur),
        mm(dos),
        mm(hauteur),
        mm(dos),
        mm(hauteur),
        bloc_dos(livre, cv, &g, image_une, 0.0),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::maquettes;
    use crate::providers::provider;

    fn livre() -> Livre {
        Livre {
            titre: "Les Heures creuses".into(),
            titre_page: crate::projet::titre_page_defaut(),
            auteur: "Ivan Pjig".into(),
            genre: "roman".into(),
            copyright: String::new(),
            dedicace: None,
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

    /// L'aperçu marque la coupe en pourcentage de l'image qu'il habille : c'est une
    /// fraction, pas des millimètres, et il en faut **deux**. Une planche fait près de
    /// 250 mm de large pour 180 de haut ; la même fraction sur les deux dimensions
    /// marquerait la coupe à côté d'elle-même.
    #[test]
    fn la_part_du_fond_perdu_differe_en_largeur_et_en_hauteur() {
        let g = gabarit("tbe-110x170", 280);
        let (x, y) = g.part_fond_perdu();
        assert!((x - 5.0 / 246.8).abs() < 1e-6, "part en largeur : {x}");
        assert!((y - 5.0 / 180.0).abs() < 1e-6, "part en hauteur : {y}");
        assert!(
            x < y,
            "la planche est plus large que haute : {x} devrait être < {y}"
        );
    }

    /// La face Dos compose sur un gabarit à fond perdu nul (voir `source_dos`). Rien à
    /// y marquer — et surtout pas un trait sur le bord même de l'image, qui se lirait
    /// comme une coupe à zéro millimètre du texte.
    #[test]
    fn un_gabarit_sans_fond_perdu_ne_donne_aucune_part() {
        let g = Gabarit {
            format: (108.0, 175.0),
            dos: 13.0,
            fond_perdu: 0.0,
        };
        assert_eq!(g.part_fond_perdu(), (0.0, 0.0));
    }

    /// Les deux plis encadrent le dos, et l'écart entre eux **est** le dos : c'est ce
    /// qui fait de ces deux traits la seule chose qui montre, sur un fond uni, où le
    /// livre se plie. Un pli mesuré depuis le mauvais bord placerait le dos ailleurs
    /// qu'où il est composé, et l'aperçu mentirait plus qu'il n'aiderait.
    #[test]
    fn les_deux_plis_encadrent_le_dos() {
        let g = gabarit("tbe-110x170", 280);
        let (a, b) = g.plis();
        assert!(
            (a - (5.0 + 110.0) / 246.8).abs() < 1e-6,
            "pli de la 4ème : {a}"
        );
        assert!(
            ((b - a) * g.largeur() - g.dos).abs() < 1e-6,
            "l'écart doit valoir le dos"
        );
        assert!(
            a < 0.5 && b > 0.5,
            "le dos passe par le milieu : {a} et {b}"
        );
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

    /// Le dos s'aperçoit à sa taille, pas à une taille commode.
    ///
    /// Trois textes doivent tenir dans treize millimètres : c'est cette contrainte-là
    /// qu'on regarde en réglant leur corps. Une boîte élargie « pour mieux voir » ferait
    /// paraître composable un dos qui déborde au tirage. Le grossissement vient de la
    /// page couchée et de l'écran qui l'étire, jamais de la composition.
    #[test]
    fn le_dos_seul_est_compose_a_la_largeur_du_dos() {
        let g = gabarit("lulu", 244);
        let s = source_dos(&livre(), &maquettes::folio(), g.format, g.dos, None);
        let boite = format!("box(width: {}, height: {}", mm(g.dos), mm(g.format.1));
        assert!(s.contains(&boite), "boîte attendue {boite} dans :\n{s}");
        // La page, elle, est cette boîte couchée : le dos y court en largeur.
        let page = format!("#set page(width: {}, height: {}", mm(g.format.1), mm(g.dos));
        assert!(s.contains(&page), "page attendue {page} dans :\n{s}");
    }

    /// Les prises du dos tombent là où les textes se composent.
    ///
    /// La règle est celle des cinq colonnes de `bloc_dos`, relue depuis l'autre bout :
    /// le pied contre le début, la tête contre la fin, l'écart entre deux voisins d'une
    /// même place. C'est le seul endroit où cette mise en page est écrite deux fois —
    /// une fois pour Typst, une fois pour la souris — et ce test est ce qui les tient
    /// d'accord. Si elles divergent, la prise se pose à côté du texte et le geste
    /// devient une devinette.
    ///
    /// Les longueurs sont données ici plutôt que mesurées : ce qui se vérifie est
    /// l'arithmétique de la place, pas la métrique des polices — celle-là, seul Typst
    /// la connaît, et `source_mesures` la lui demande.
    #[test]
    fn les_boites_du_dos_suivent_les_cinq_colonnes() {
        let mut cv = maquettes::folio();
        cv.pied.editeur = "OZALID".into();
        cv.dos.marge = 3.0;
        cv.dos.ecart = 2.0;
        cv.dos.auteur = ElementDos {
            actif: true,
            place: PlaceDos::Pied,
            rang: 1,
            ..cv.dos.auteur.clone()
        };
        cv.dos.titre = ElementDos {
            actif: true,
            place: PlaceDos::Pied,
            rang: 2,
            ..cv.dos.titre.clone()
        };
        cv.dos.editeur = ElementDos {
            actif: true,
            place: PlaceDos::Tete,
            rang: 1,
            ..cv.dos.editeur.clone()
        };
        let mesures = [
            ("auteur".to_string(), 20.0),
            ("titre".to_string(), 50.0),
            ("editeur".to_string(), 10.0),
        ]
        .into_iter()
        .collect();

        // Poche Lulu : 108 de large, 175 de dos. Marge 3,24 mm, écart 2,16 mm.
        let pres = |b: &[BoiteDos], c: &str, deb: f64, fin: f64, quoi: &str| {
            let x = b.iter().find(|x| x.cle == c).unwrap();
            assert!(
                (x.debut * 175.0 - deb).abs() < 0.01 && (x.fin * 175.0 - fin).abs() < 0.01,
                "{quoi} : [{}, {}] attendu [{deb}, {fin}]",
                x.debut * 175.0,
                x.fin * 175.0
            );
        };

        // Deux au pied, un à la tête. Le pied part de la marge, ses deux textes sont
        // séparés de l'écart, la tête est calée contre la marge de l'autre bout.
        let b = boites_dos(&livre(), &cv, (108.0, 175.0), &mesures);
        pres(
            &b,
            "auteur",
            3.24,
            23.24,
            "l'auteur ne part pas de la marge du pied",
        );
        pres(
            &b,
            "titre",
            25.4,
            75.4,
            "le titre ne suit pas l'auteur d'un écart",
        );
        pres(
            &b,
            "editeur",
            161.76,
            171.76,
            "l'éditeur n'est pas calé sur la tête",
        );

        // Le même dos, l'éditeur passé au centre : la tête est vide, et le centre n'est
        // donc plus centré sur le dos. Les deux ressorts se partagent ce que le pied
        // laisse — 168,52 − 72,16 − 10 —, d'où 3,24 + 72,16 + 43,18.
        cv.dos.editeur.place = PlaceDos::Centre;
        let b = boites_dos(&livre(), &cv, (108.0, 175.0), &mesures);
        pres(
            &b,
            "editeur",
            118.58,
            128.58,
            "le centre est cru centré sur le dos",
        );
    }

    /// Le corps du texte de dos est un pourcentage de la **largeur de couverture** ;
    /// l'épaisseur du dos, elle, vient de la pagination. Rien n'accorde les deux : la
    /// même maquette réclame un dos plus épais en grand format qu'en poche. C'est
    /// l'hypothèse « une maquette pour tous les formats » qui se paie ici, et c'est
    /// pour ça que la mesure suit la largeur au lieu d'être un seuil fixe.
    #[test]
    fn le_dos_requis_suit_la_largeur_de_couverture() {
        let cv = maquettes::folio();
        let r = |largeur| dos_requis(&livre(), &cv, largeur) - 2.0 * JEU_PLI;
        // Le jeu de pli mis à part, ce qui reste est du corps : il double quand la
        // couverture double.
        assert!(
            (r(216.0) - 2.0 * r(108.0)).abs() < 1e-9,
            "108 mm réclame {}, 216 mm {}",
            r(108.0),
            r(216.0)
        );
    }

    /// Un livre mince dans un grand format : le dos ne tient pas le texte que la
    /// maquette y compose. Sans cette mesure, rien ne le dit — `zone` compose avec
    /// `clip: true`, donc le titre part **rogné, sans erreur**, sur le PDF remis à
    /// l'imprimeur. C'est le seul cas où une maquette unique produit un fichier faux
    /// et non un fichier différent.
    #[test]
    fn un_dos_trop_mince_pour_son_texte_est_signale() {
        let cv = maquettes::folio();
        let mince = gabarit("kdp-6x9", 80);
        let epais = gabarit("kdp-6x9", 400);
        let insuffisant = |g: &Gabarit| dos_insuffisant(&livre(), &cv, g.format.0, g.dos);
        assert!(
            insuffisant(&mince).is_some_and(|r| r > mince.dos),
            "80 pages : dos de {} mm, réclamé {:?}",
            mince.dos,
            insuffisant(&mince)
        );
        assert!(
            insuffisant(&epais).is_none(),
            "400 pages : dos de {} mm, réclamé {:?}",
            epais.dos,
            insuffisant(&epais)
        );
    }

    /// La mesure ne juge que ce que `source_dos` compose. Un élément éteint, ou dont le
    /// texte est vide, ne laisse pas de trou sur le dos : il ne doit pas non plus lui
    /// réclamer d'épaisseur, sinon une maquette sans mention d'éditeur serait refusée
    /// sur le corps d'un éditeur qu'elle ne porte pas.
    #[test]
    fn un_element_qui_ne_compose_pas_ne_reclame_pas_depaisseur() {
        let mut cv = maquettes::folio();
        cv.pied.editeur = "Folio".into();
        cv.dos.editeur.actif = true;
        cv.dos.editeur.style.taille = 20.0;
        let avec = dos_requis(&livre(), &cv, 108.0);

        cv.dos.editeur.actif = false;
        let sans = dos_requis(&livre(), &cv, 108.0);
        assert!(sans < avec, "éteint {sans} mm, allumé {avec} mm");

        // Le texte vide compte comme éteint : c'est la règle de `source_dos`.
        cv.dos.editeur.actif = true;
        cv.pied.editeur = String::new();
        assert!(
            (dos_requis(&livre(), &cv, 108.0) - sans).abs() < 1e-9,
            "éditeur sans texte : {} mm au lieu de {sans} mm",
            dos_requis(&livre(), &cv, 108.0)
        );
    }

    /// Le quart de tour emmène la boîte hors de la page, à gauche : sans le `dx` qui l'y
    /// ramène, la face Dos serait une page blanche — et une page blanche ressemble
    /// exactement à un dos dont on aurait éteint les trois textes.
    ///
    /// Ce que ce test ne dit pas, et qui se regarde à l'œil : que le dos couché se lise
    /// dans le bon sens, pied à gauche et tête à droite.
    #[test]
    fn le_dos_couche_est_ramene_dans_sa_page() {
        let g = gabarit("lulu", 244);
        let s = source_dos(&livre(), &maquettes::folio(), g.format, g.dos, None);
        let attendu = format!(
            "dx: {}, dy: 0mm, rotate(90deg, origin: top + left",
            mm(g.format.1)
        );
        assert!(s.contains(&attendu), "attendu {attendu} dans :\n{s}");
    }

    /// La face Dos n'a pas de fond perdu à montrer, et c'est ce qui la rend disponible
    /// là où la planche ne l'est pas : chez un prestataire qui ne le publie pas. Si le
    /// fond perdu revenait dans cette page, la face deviendrait aussi exigeante que la
    /// planche, et pour rien — on ne règle pas des textes sur de la marge à couper.
    #[test]
    fn le_dos_seul_ignore_le_fond_perdu() {
        let g = gabarit("lulu", 244);
        assert!(g.fond_perdu > 0.0, "le gabarit de test doit en avoir un");
        let s = source_dos(&livre(), &maquettes::folio(), g.format, g.dos, None);
        assert!(
            !s.contains(&mm(g.hauteur())),
            "la hauteur de planche n'a rien à faire ici :\n{s}"
        );
    }

    /// Le même couplage que pour la planche, redit sur cette face : c'est la pagination
    /// qui donne le dos. Un aperçu du dos qui ne bougerait pas avec le manuscrit
    /// laisserait régler des textes sur une largeur périmée.
    #[test]
    fn une_pagination_plus_longue_elargit_le_dos_seul() {
        let court = gabarit("lulu", 244);
        let long = gabarit("lulu", 400);
        // La page est couchée : c'est sa hauteur qui porte le dos.
        let page = |g: &Gabarit| {
            let s = source_dos(&livre(), &maquettes::folio(), g.format, g.dos, None);
            let i = s.find(", height: ").unwrap() + ", height: ".len();
            s[i..].split("mm").next().unwrap().parse::<f64>().unwrap()
        };
        let ecart = long.dos - court.dos;
        assert!(ecart > 0.0, "400 pages font un dos plus épais que 244");
        assert!(
            (page(&long) - page(&court) - ecart).abs() < 0.001,
            "le dos seul doit s'élargir de {ecart} mm"
        );
    }

    /// Le dos seul montre le dos de la planche, pas une seconde écriture du même dos.
    /// Deux compositions distinctes dériveraient l'une de l'autre au premier réglage
    /// ajouté, et la face servirait à régler ce que la planche ne rendrait pas.
    #[test]
    fn le_dos_seul_et_celui_de_la_planche_portent_la_meme_grille() {
        let g = gabarit("lulu", 244);
        let cv = maquettes::folio();
        // La zone du dos est écrite en premier dans la planche : sa grille est donc la
        // première des trois.
        let grille = |s: &str| {
            let i = s.find("#grid(columns:").expect("le dos porte une grille");
            s[i..].split("]))").next().unwrap().to_string()
        };
        let planche = source(&livre(), &cv, &g, None, None).unwrap();
        let seul = source_dos(&livre(), &cv, g.format, g.dos, None);
        assert_eq!(grille(&seul), grille(&planche));
        assert!(seul.contains("Les Heures creuses"), "{seul}");
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
