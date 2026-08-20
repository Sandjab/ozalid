//! Les trois maquettes de départ, portées depuis les `PRESETS` d'`index.html`.
//!
//! Elles ne portent que la mise en page : le titre, l'auteur et le genre viennent du
//! projet. Charger une maquette ne change donc jamais ce qui sera imprimé comme
//! identité du livre — seulement la façon dont ça paraît.

use crate::couverture::*;
use crate::image::Cadrage;

fn style(police: &str, graisse: u16, taille: f64, couleur: &str) -> Style {
    Style {
        police: police.into(),
        graisse,
        italique: false,
        taille,
        couleur: couleur.into(),
        tracking: 0.0,
        casse: Casse::Telle,
    }
}

/// Réglages de 4ème communs aux trois maquettes : l'atelier les livrait identiques.
fn quatrieme_commune() -> Quatrieme {
    Quatrieme {
        fond: FondQuatre::Herite,
        couleur: "#fcf0d8".into(),
        texte: String::new(),
        style: style("Spectral", 400, 3.0, "#191917"),
        interligne: 1.45,
        align: Align::Gauche,
        pad_x: 10.0,
        top: 12.0,
        pied_actif: true,
        mention: String::new(),
        collection: String::new(),
        prix: String::new(),
        style_pied: style("Archivo", 400, 2.4, "#191917"),
        pied_y: 4.0,
        isbn_actif: false,
        isbn_l: 34.0,
        isbn_h: 21.0,
        isbn_dx: 7.0,
        isbn_dy: 7.0,
        cadrage: Cadrage::default(),
        voile: Voile::Aucun,
        voile_opacite: 0.55,
    }
}

fn pastille_eteinte() -> Pastille {
    Pastille {
        actif: false,
        texte: String::new(),
        style: style("Archivo", 400, 3.2, "#ffffff"),
        fond: "#111111".into(),
        coin: Coin::BasDroite,
        verticale: false,
        arrondie: true,
        dx: 4.5,
        dy: 3.5,
    }
}

/// Bandeau de titre en haut, image à fond perdu dessous. Archétype Folio / Penguin
/// Modern Classics.
pub fn folio() -> Couverture {
    Couverture {
        mode: Mode::Bandeau,
        papier: "#ffffff".into(),
        align: Align::Gauche,
        pad_x: 7.0,
        bandeau: 30.0,
        bandeau_retrait: false,
        bloc_y: 13.0,
        cadre: Cadre {
            actif: false,
            marge: 9.0,
            filet1_couleur: "#000000".into(),
            filet1_epaisseur: 0.3,
            decroche: 4.0,
            filet2_couleur: "#c00000".into(),
            filet2_epaisseur: 0.25,
            ecart: 0.9,
        },
        auteur: Style {
            taille: 6.4,
            ..style("Archivo", 700, 6.4, "#c00000")
        },
        titre: style("Spectral", 400, 8.0, "#191917"),
        titre_interligne: 1.1,
        titre_ecart: 3.5,
        genre_visible: false,
        genre: style("Spectral", 400, 2.2, "#191917"),
        genre_ecart: 6.0,
        pied: Pied {
            actif: false,
            monogramme: "nrf".into(),
            editeur: "GALLIMARD".into(),
            y: 11.0,
            style_mono: Style {
                italique: true,
                ..style("Spectral", 600, 7.0, "#191917")
            },
            style_editeur: Style {
                tracking: 10.0,
                ..style("Archivo", 400, 3.2, "#191917")
            },
        },
        pastille: Pastille {
            actif: true,
            texte: "folio".into(),
            ..pastille_eteinte()
        },
        cadrage: Cadrage::default(),
        voile: Voile::Aucun,
        voile_opacite: 0.55,
        quatrieme: quatrieme_commune(),
    }
}

/// Composition purement typographique, triple filet. Archétype Blanche / NRF.
pub fn blanche() -> Couverture {
    Couverture {
        mode: Mode::Typo,
        papier: "#fcf0d8".into(),
        align: Align::Centre,
        pad_x: 16.0,
        bandeau: 30.0,
        bandeau_retrait: false,
        bloc_y: 13.0,
        cadre: Cadre {
            actif: true,
            marge: 9.0,
            filet1_couleur: "#000000".into(),
            filet1_epaisseur: 0.3,
            decroche: 4.0,
            filet2_couleur: "#c00000".into(),
            filet2_epaisseur: 0.25,
            ecart: 0.9,
        },
        auteur: Style {
            tracking: 6.0,
            casse: Casse::Capitales,
            ..style("Bodoni Moda", 700, 3.6, "#000000")
        },
        titre: Style {
            tracking: 1.0,
            casse: Casse::Capitales,
            ..style("Bodoni Moda", 700, 9.0, "#c00000")
        },
        titre_interligne: 1.05,
        titre_ecart: 11.0,
        genre_visible: true,
        genre: Style {
            tracking: 12.0,
            ..style("Bodoni Moda", 400, 2.2, "#191917")
        },
        genre_ecart: 6.0,
        pied: Pied {
            actif: true,
            monogramme: "nrf".into(),
            editeur: "GALLIMARD".into(),
            y: 11.0,
            style_mono: Style {
                italique: true,
                ..style("Bodoni Moda", 600, 7.0, "#191917")
            },
            style_editeur: Style {
                tracking: 10.0,
                ..style("Bodoni Moda", 400, 3.2, "#191917")
            },
        },
        pastille: pastille_eteinte(),
        cadrage: Cadrage::default(),
        voile: Voile::Aucun,
        voile_opacite: 0.55,
        quatrieme: quatrieme_commune(),
    }
}

/// Image sur toute la surface, texte par-dessus, voile de lisibilité.
pub fn surimpression() -> Couverture {
    Couverture {
        mode: Mode::Surimpression,
        papier: "#000000".into(),
        align: Align::Centre,
        pad_x: 12.0,
        bandeau: 30.0,
        bandeau_retrait: false,
        bloc_y: 9.0,
        cadre: Cadre {
            actif: true,
            marge: 6.0,
            filet1_couleur: "#f2ece0".into(),
            filet1_epaisseur: 0.25,
            decroche: 1.4,
            filet2_couleur: "#f2ece0".into(),
            filet2_epaisseur: 0.15,
            ecart: 0.6,
        },
        auteur: Style {
            tracking: 14.0,
            casse: Casse::Capitales,
            ..style("Archivo", 600, 3.4, "#f4efe4")
        },
        titre: Style {
            tracking: -1.0,
            ..style("Playfair Display", 500, 11.0, "#ffffff")
        },
        titre_interligne: 1.02,
        titre_ecart: 6.0,
        genre_visible: false,
        genre: style("Playfair Display", 400, 2.2, "#f4efe4"),
        genre_ecart: 6.0,
        pied: Pied {
            actif: false,
            monogramme: "nrf".into(),
            editeur: "GALLIMARD".into(),
            y: 11.0,
            style_mono: Style {
                italique: true,
                ..style("Playfair Display", 600, 7.0, "#f4efe4")
            },
            style_editeur: Style {
                tracking: 10.0,
                ..style("Archivo", 400, 3.2, "#f4efe4")
            },
        },
        pastille: pastille_eteinte(),
        // Ancrage bas : sur un portrait, garder le haut du cadre plutôt que le centre.
        cadrage: Cadrage {
            y: 0.62,
            zoom: 1.05,
            ..Cadrage::default()
        },
        voile: Voile::Deux,
        voile_opacite: 0.62,
        quatrieme: quatrieme_commune(),
    }
}

/// Les maquettes, par clé, dans l'ordre où l'interface les propose.
pub fn toutes() -> Vec<(&'static str, &'static str, Couverture)> {
    vec![
        ("folio", "Folio", folio()),
        ("blanche", "Blanche", blanche()),
        ("surimpression", "Surimpression", surimpression()),
    ]
}

pub fn par_cle(cle: &str) -> Option<Couverture> {
    toutes()
        .into_iter()
        .find(|(k, _, _)| *k == cle)
        .map(|(_, _, c)| c)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Chaque maquette doit être un archétype distinct : trois entrées qui rendraient
    /// la même chose ne serviraient à rien comme point de départ.
    #[test]
    fn les_trois_maquettes_sont_de_modes_distincts() {
        let modes: Vec<Mode> = toutes().into_iter().map(|(_, _, c)| c.mode).collect();
        assert_eq!(modes.len(), 3);
        for (i, m) in modes.iter().enumerate() {
            assert!(!modes[..i].contains(m), "mode {m:?} en double");
        }
    }

    #[test]
    fn une_cle_inconnue_ne_rend_pas_de_maquette() {
        assert!(par_cle("gallimard").is_none());
        assert!(par_cle("folio").is_some());
    }

    /// Le voile n'a de sens que sur une image : l'allumer sans image assombrirait
    /// une couverture qui n'a rien dessous.
    #[test]
    fn seule_la_maquette_a_image_pleine_page_porte_un_voile() {
        assert_eq!(folio().voile, Voile::Aucun);
        assert_eq!(blanche().voile, Voile::Aucun);
        assert_ne!(surimpression().voile, Voile::Aucun);
    }
}
