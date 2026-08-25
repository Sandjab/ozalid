//! Séparer l'encre du papier dans la photo d'un envoi.
//!
//! Un envoi écrit à la main est presque toujours la photo d'un mot tracé sur une
//! feuille. Le papier photographié n'est pas du blanc pur — 230 à 245, teinté, avec le
//! dégradé de l'éclairage — et ce blanc-là s'encre. Sur un papier crème, il paraît.
//!
//! Aucun disque, aucun état : des octets entrent, des octets sortent. C'est la manière
//! d'`image.rs`, et c'est ce qui rend ce module vérifiable sur des images fabriquées en
//! mémoire.

/// Les deux seuils qui séparent l'encre du papier, en luminance 0-255.
///
/// Deux et non un : sans point d'encre, un trait bien noir ressort délavé — mesuré
/// (48, 51, 123) contre (28, 32, 105) pour un stylo bleu. Voir la spec, § « Ce qui est
/// vérifié ».
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Detourage {
    /// Au-dessus, c'est le papier : alpha 0.
    pub papier: f64,
    /// En dessous, c'est l'encre pleine : alpha 1.
    pub encre: f64,
}

/// La luminance perçue d'un pixel, 0-255.
fn luminance(r: u8, g: u8, b: u8) -> f64 {
    0.2126 * r as f64 + 0.7152 * g as f64 + 0.0722 * b as f64
}

/// La photo, son fond rendu transparent, en PNG.
///
/// **La couleur du pixel n'est pas touchée.** Démultiplier l'encre pour la « retrouver »
/// derrière le papier a été mesuré : le trait en sort moins fidèle qu'avec un point noir
/// bien posé, pour un calcul plus compliqué. Seul l'alpha se calcule.
///
/// L'alpha calculé **multiplie** celui d'entrée : un PNG déjà détouré par l'auteur ne
/// doit pas redevenir opaque là où son fond est clair.
pub fn applique(octets: &[u8], d: &Detourage) -> Result<Vec<u8>, String> {
    if d.papier <= d.encre {
        return Err(format!(
            "détourage impossible : le papier ({:.0}) doit être plus clair que \
             l'encre ({:.0}).",
            d.papier, d.encre
        ));
    }
    let mut img = image::load_from_memory(octets)
        .map_err(|e| format!("image illisible : {e}"))?
        .to_rgba8();
    let ecart = d.papier - d.encre;
    for p in img.pixels_mut() {
        let [r, g, b, a] = p.0;
        let f = ((d.papier - luminance(r, g, b)) / ecart).clamp(0.0, 1.0);
        p.0 = [r, g, b, (f * a as f64).round() as u8];
    }
    let mut out = Vec::new();
    image::DynamicImage::ImageRgba8(img)
        .write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png)
        .map_err(|e| format!("encodage PNG impossible : {e}"))?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Une image unie de la couleur donnée, en PNG.
    fn uni(r: u8, g: u8, b: u8) -> Vec<u8> {
        let img = image::RgbaImage::from_pixel(4, 4, image::Rgba([r, g, b, 255]));
        let mut out = Vec::new();
        image::DynamicImage::ImageRgba8(img)
            .write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png)
            .unwrap();
        out
    }

    /// L'alpha du premier pixel d'un PNG.
    fn alpha(octets: &[u8]) -> u8 {
        image::load_from_memory(octets)
            .unwrap()
            .to_rgba8()
            .get_pixel(0, 0)[3]
    }

    const SEUILS: Detourage = Detourage {
        papier: 240.0,
        encre: 40.0,
    };

    /// Les deux bouts de la rampe. Sans eux, un détourage qui ne ferait rien passerait.
    #[test]
    fn le_papier_disparait_et_l_encre_reste() {
        assert_eq!(alpha(&applique(&uni(250, 250, 250), &SEUILS).unwrap()), 0);
        assert_eq!(alpha(&applique(&uni(10, 10, 10), &SEUILS).unwrap()), 255);
    }

    /// La rampe elle-même : un seuil binaire ferait tomber ce test, et c'est tout son
    /// objet — il hacherait le trait en escalier là où la photo l'a lissé.
    #[test]
    fn un_pixel_a_mi_chemin_sort_a_mi_alpha() {
        // Luminance 140, à mi-chemin de 240 et 40 : un gris neutre suffit, la luminance
        // d'un gris vaut sa composante.
        let a = alpha(&applique(&uni(140, 140, 140), &SEUILS).unwrap());
        assert!((a as i32 - 128).abs() <= 2, "alpha {a}, attendu 128 ± 2");
    }

    /// `papier <= encre` divise par zéro ou inverse la rampe : l'image sortirait
    /// entièrement opaque sans qu'on sache pourquoi. On refuse en nommant les deux
    /// valeurs — c'est un réglage que l'écran laisse atteindre.
    #[test]
    fn un_papier_plus_sombre_que_l_encre_se_refuse() {
        let d = Detourage {
            papier: 40.0,
            encre: 240.0,
        };
        let err = applique(&uni(200, 200, 200), &d).unwrap_err();
        assert!(err.contains("240"), "le message ne dit pas l'encre : {err}");
        assert!(
            err.contains("40"),
            "le message ne dit pas le papier : {err}"
        );
    }
}
