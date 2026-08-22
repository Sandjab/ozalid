//! Placer une image dans une zone : dimensions naturelles, et cadrage.
//!
//! La géométrie est calculée ici, en millimètres, et posée telle quelle dans la
//! source Typst. C'est le portage littéral de l'`artGeom` d'`index.html`, qui produit
//! déjà cette géométrie pour l'export — le CSS `object-fit`/`object-position`/`scale`
//! n'en est que l'équivalent à l'écran. Passer par le calcul plutôt que par la mise en
//! page de Typst rend le cadrage déterministe et testable.

/// Réglages de cadrage d'une image dans sa zone.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Cadrage {
    /// Proportions conservées : l'image tient entière dans la zone (`contain`).
    /// Sinon elle la remplit et déborde (`cover`).
    pub proportions: bool,
    /// Point d'ancrage, 0 = bord gauche/haut, 1 = bord droit/bas.
    pub x: f64,
    pub y: f64,
    pub zoom: f64,
    /// Déformation horizontale. Sans objet quand les proportions sont conservées.
    pub etirement: f64,
}

impl Default for Cadrage {
    fn default() -> Self {
        Self {
            proportions: false,
            x: 0.5,
            y: 0.5,
            zoom: 1.0,
            etirement: 1.0,
        }
    }
}

/// Position et taille de l'image dans sa zone, dans l'unité de la zone.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Geometrie {
    pub gauche: f64,
    pub haut: f64,
    pub largeur: f64,
    pub hauteur: f64,
}

/// Géométrie de l'image dans la zone, ou `None` si l'une des deux est dégénérée.
pub fn place(zone: (f64, f64), naturel: (u32, u32), c: &Cadrage) -> Option<Geometrie> {
    let (zw, zh) = zone;
    let (nw, nh) = (f64::from(naturel.0), f64::from(naturel.1));
    if zw <= 0.0 || zh <= 0.0 || nw <= 0.0 || nh <= 0.0 {
        return None;
    }
    // `contain` prend la plus petite échelle, `cover` la plus grande.
    let (a, b) = (zw / nw, zh / nh);
    let ajuste = if c.proportions { a.min(b) } else { a.max(b) };
    // La déformation est repliée dans l'échelle horizontale, et n'a de sens qu'en
    // cadrage débordant : conserver les proportions la neutralise.
    let sx = c.zoom * if c.proportions { 1.0 } else { c.etirement };

    let (dw, dh) = (nw * ajuste, nh * ajuste);
    let (gauche, haut) = ((zw - dw) * c.x, (zh - dh) * c.y);
    // Le zoom se prend autour du point d'ancrage, pas du coin de la zone.
    let (ox, oy) = (zw * c.x, zh * c.y);
    Some(Geometrie {
        gauche: ox - (ox - gauche) * sx,
        haut: oy - (oy - haut) * c.zoom,
        largeur: dw * sx,
        hauteur: dh * c.zoom,
    })
}

/// Dimensions naturelles d'une image PNG ou JPEG, sans la décoder.
pub fn dimensions(octets: &[u8]) -> Option<(u32, u32)> {
    png(octets).or_else(|| jpeg(octets))
}

/// L'extension qui convient à ces octets-là.
///
/// Relevée sur le contenu, jamais sur le nom du fichier d'origine : Typst reconnaît le
/// format d'une image **à son extension**, si bien qu'un JPEG rangé sous un nom en
/// `.png` ne se composerait pas — et l'erreur arriverait des centaines de pages plus
/// loin, sur l'exemplaire d'une personne.
pub fn extension(octets: &[u8]) -> Option<&'static str> {
    if png(octets).is_some() {
        return Some("png");
    }
    jpeg(octets).map(|_| "jpg")
}

fn png(o: &[u8]) -> Option<(u32, u32)> {
    // Signature, puis IHDR : longueur (4), type (4), largeur (4), hauteur (4).
    if o.len() < 24 || &o[..8] != b"\x89PNG\r\n\x1a\n" || &o[12..16] != b"IHDR" {
        return None;
    }
    Some((
        u32::from_be_bytes(o[16..20].try_into().ok()?),
        u32::from_be_bytes(o[20..24].try_into().ok()?),
    ))
}

fn jpeg(o: &[u8]) -> Option<(u32, u32)> {
    if o.len() < 4 || o[0] != 0xFF || o[1] != 0xD8 {
        return None;
    }
    let mut p = 2usize;
    while p + 9 < o.len() {
        if o[p] != 0xFF {
            p += 1; // octets de bourrage entre segments
            continue;
        }
        let marqueur = o[p + 1];
        // SOF0-3, SOF5-7, SOF9-11, SOF13-15 portent les dimensions ; C4, C8 et CC
        // sont des tables, pas des cadres.
        let est_sof = (0xC0..=0xCF).contains(&marqueur) && !matches!(marqueur, 0xC4 | 0xC8 | 0xCC);
        let taille = u16::from_be_bytes([o[p + 2], o[p + 3]]) as usize;
        if est_sof {
            return Some((
                u32::from(u16::from_be_bytes([o[p + 7], o[p + 8]])),
                u32::from(u16::from_be_bytes([o[p + 5], o[p + 6]])),
            ));
        }
        if taille < 2 {
            return None;
        }
        p += 2 + taille;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    const ZONE: (f64, f64) = (100.0, 50.0);

    /// Cadrage débordant, sans zoom : l'image couvre la zone entière et le débordement
    /// se répartit selon l'ancrage. C'est le comportement par défaut d'une couverture.
    #[test]
    fn une_image_debordante_couvre_la_zone() {
        // 100×100 dans une zone 100×50 : l'échelle retenue est la plus grande (1.0),
        // donc 100×100, et 50 de débordement vertical.
        let g = place(ZONE, (100, 100), &Cadrage::default()).unwrap();
        assert_eq!(g.largeur, 100.0);
        assert_eq!(g.hauteur, 100.0);
        assert_eq!(g.gauche, 0.0);
        assert_eq!(g.haut, -25.0, "débordement centré");
    }

    /// Proportions conservées : l'image tient entière, avec du vide autour.
    #[test]
    fn les_proportions_conservees_font_tenir_l_image_entiere() {
        let c = Cadrage {
            proportions: true,
            ..Cadrage::default()
        };
        let g = place(ZONE, (100, 100), &c).unwrap();
        assert_eq!(g.largeur, 50.0);
        assert_eq!(g.hauteur, 50.0);
        assert_eq!(g.gauche, 25.0, "vide réparti de part et d'autre");
        assert_eq!(g.haut, 0.0);
    }

    /// L'ancrage déplace l'image dans sa zone : 0 = haut, 1 = bas. C'est le réglage
    /// qui sert à choisir ce qu'on garde d'une photo trop haute.
    #[test]
    fn l_ancrage_choisit_la_partie_visible() {
        for (y, attendu) in [(0.0, 0.0), (0.5, -25.0), (1.0, -50.0)] {
            let c = Cadrage {
                y,
                ..Cadrage::default()
            };
            let g = place(ZONE, (100, 100), &c).unwrap();
            assert_eq!(g.haut, attendu, "ancrage {y}");
        }
    }

    /// Le zoom se prend autour de l'ancrage : ancré en haut, le bord haut ne bouge
    /// pas. Sans cela, zoomer ferait dériver le cadrage qu'on vient de choisir.
    #[test]
    fn le_zoom_se_prend_autour_de_l_ancrage() {
        let c = Cadrage {
            y: 0.0,
            x: 0.0,
            zoom: 2.0,
            ..Cadrage::default()
        };
        let g = place(ZONE, (100, 100), &c).unwrap();
        assert_eq!(g.gauche, 0.0);
        assert_eq!(g.haut, 0.0);
        assert_eq!(g.largeur, 200.0);
        assert_eq!(g.hauteur, 200.0);
    }

    /// La déformation n'agit qu'en horizontal, et seulement hors proportions
    /// conservées — sinon elle contredirait le réglage qui la précède.
    #[test]
    fn l_etirement_est_neutralise_par_les_proportions_conservees() {
        let etire = Cadrage {
            etirement: 1.5,
            ..Cadrage::default()
        };
        let g = place(ZONE, (100, 100), &etire).unwrap();
        assert_eq!(g.largeur, 150.0);
        assert_eq!(g.hauteur, 100.0, "la hauteur ne suit pas l'étirement");

        let garde = Cadrage {
            proportions: true,
            etirement: 1.5,
            ..Cadrage::default()
        };
        let g = place(ZONE, (100, 100), &garde).unwrap();
        assert_eq!(g.largeur, g.hauteur, "étirement appliqué malgré tout");
    }

    #[test]
    fn une_zone_ou_une_image_degeneree_ne_donne_pas_de_geometrie() {
        assert!(place((0.0, 50.0), (100, 100), &Cadrage::default()).is_none());
        assert!(place(ZONE, (0, 100), &Cadrage::default()).is_none());
    }

    #[test]
    fn les_dimensions_d_un_png_se_lisent_dans_l_ihdr() {
        let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
        png.extend(13u32.to_be_bytes());
        png.extend(b"IHDR");
        png.extend(1200u32.to_be_bytes());
        png.extend(1980u32.to_be_bytes());
        png.extend([8, 6, 0, 0, 0]);
        assert_eq!(dimensions(&png), Some((1200, 1980)));
    }

    /// Les photos embarquées par l'atelier sont des JPEG : les lire est indispensable,
    /// pas optionnel.
    #[test]
    fn les_dimensions_d_un_jpeg_se_lisent_dans_le_cadre() {
        let mut j = vec![0xFF, 0xD8];
        // APP0 de 16 octets, à sauter
        j.extend([0xFF, 0xE0, 0x00, 0x10]);
        j.extend([0u8; 14]);
        // SOF0 : taille, précision, hauteur, largeur
        j.extend([0xFF, 0xC0, 0x00, 0x11, 0x08]);
        j.extend(1980u16.to_be_bytes());
        j.extend(1200u16.to_be_bytes());
        j.extend([0u8; 10]);
        assert_eq!(dimensions(&j), Some((1200, 1980)));
    }

    /// L'extension se relève sur les octets : une photo d'appareil renommée en `.png`
    /// reste un JPEG, et Typst la lirait à son nom.
    #[test]
    fn l_extension_se_releve_sur_le_contenu() {
        let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
        png.extend(13u32.to_be_bytes());
        png.extend(b"IHDR");
        png.extend(10u32.to_be_bytes());
        png.extend(10u32.to_be_bytes());
        png.extend([8, 6, 0, 0, 0]);
        assert_eq!(extension(&png), Some("png"));

        let mut j = vec![0xFF, 0xD8];
        j.extend([0xFF, 0xC0, 0x00, 0x11, 0x08]);
        j.extend(10u16.to_be_bytes());
        j.extend(10u16.to_be_bytes());
        j.extend([0u8; 10]);
        assert_eq!(extension(&j), Some("jpg"));

        assert_eq!(extension(b"GIF89a"), None);
    }

    #[test]
    fn un_fichier_qui_n_est_ni_png_ni_jpeg_ne_donne_pas_de_dimensions() {
        assert_eq!(dimensions(b""), None);
        assert_eq!(dimensions(b"GIF89a......."), None);
        // JPEG tronqué en plein segment : pas de dimensions, pas de panique.
        assert_eq!(dimensions(&[0xFF, 0xD8, 0xFF, 0xE0, 0x00]), None);
    }
}
