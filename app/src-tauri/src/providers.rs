//! Table unique des gabarits prestataires.
//!
//! Elle fusionne les deux tables historiques du projet, qui décrivaient les mêmes
//! prestataires sans jamais se recouper : le `PROVIDERS` d'`index.html` (couverture :
//! fond perdu, formule de dos) et celui de `outils/gen_interieur.py` (intérieur :
//! format, marges, gouttières, typographie). Un prestataire s'ajoute désormais à un
//! seul endroit, et le nombre de pages ne peut plus désigner deux formats différents.
//!
//! Toutes les valeurs proviennent des relevés déjà documentés dans ces deux fichiers
//! et dans `COOKBOOK.md`, ou de relevés faits sur les gabarits et calculateurs des
//! prestataires, cités en commentaire là où ils servent ; aucune n'est reconstituée.
//! Hors tranche connue, on refuse plutôt que d'extrapoler.

/// Épaisseur du dos. Trois formes, parce que les prestataires en publient trois.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Dos {
    /// Lulu : `pages / 17,48 + 1,524` mm. Gardée sous forme de division, comme le
    /// guide l'écrit — la convertir en facteur décimal introduirait une dérive.
    Divise { par: f64, plus: f64 },
    /// BoD et KDP : `pages × épaisseur + constante` mm. La constante vaut 0 chez KDP,
    /// qui ne compte pas l'épaisseur de la couverture.
    Multiplie { par: f64, plus: f64 },
    /// CoolLibri : aucune formule publiable (la « main » des papiers manque). Le dos
    /// se relève sur leur gabarit, il ne se calcule pas.
    Mesure,
}

impl Dos {
    /// Épaisseur en mm, ou `None` quand le prestataire ne publie pas de formule.
    pub fn mm(&self, pages: u32) -> Option<f64> {
        let p = f64::from(pages);
        match *self {
            Dos::Divise { par, plus } => Some(p / par + plus),
            Dos::Multiplie { par, plus } => Some(p * par + plus),
            Dos::Mesure => None,
        }
    }
}

/// Un papier d'intérieur. Il ne change que l'épaisseur du dos, jamais la composition :
/// c'est pourquoi il vit ici et non dans les réglages de l'intérieur.
#[derive(Debug, Clone, Copy)]
pub struct Papier {
    pub cle: &'static str,
    pub libelle: &'static str,
    /// La couleur du papier, en notation CSS, telle que le canevas la peint.
    ///
    /// **Convention d'Ozalid et non mesure** : aucun prestataire ne publie la teinte de
    /// son crème. Elle suit ce que le libellé annonce, et rien d'autre — un papier dont
    /// le nom ne dit pas « crème » est tenu pour blanc plutôt que deviné.
    ///
    /// Elle ne sert qu'à l'écran. Le PDF n'a pas de fond, et lui en donner un ferait
    /// imprimer un aplat sur toutes les pages — l'erreur même qu'on corrige ici.
    pub teinte: &'static str,
    pub dos: Dos,
}

/// Une tranche de pagination et la gouttière (marge intérieure) qu'elle impose.
pub type Tranche = (u32, u32, f64);

#[derive(Debug, Clone, Copy)]
pub struct Provider {
    pub cle: &'static str,
    pub libelle: &'static str,
    /// Format de rognage en mm (largeur, hauteur).
    pub format: (f64, f64),
    pub marge_haut: f64,
    pub marge_bas: f64,
    /// Marge extérieure (sécurité), opposée à la gouttière.
    pub exterieur: f64,
    /// Seules les tranches vérifiées dans le guide du prestataire figurent ici.
    /// Hors tranche, on refuse plutôt qu'inventer.
    pub gouttieres: &'static [Tranche],
    pub corps_pt: f64,
    pub interligne: f64,
    pub folio_pt: f64,
    /// Fond perdu en mm, ou `None` quand il se relève sur le gabarit du prestataire.
    pub fond_perdu: Option<f64>,
    pub pages_min: u32,
    pub pages_max: u32,
    /// Au moins un papier ; le premier est le défaut.
    pub papiers: &'static [Papier],
}

impl Provider {
    /// Gouttière imposée par la tranche de pagination, en mm.
    pub fn gouttiere(&self, pages: u32) -> Result<f64, String> {
        self.gouttieres
            .iter()
            .find(|(lo, hi, _)| *lo <= pages && pages <= *hi)
            .map(|(_, _, g)| *g)
            .ok_or_else(|| {
                format!(
                    "{pages} pages : tranche de gouttière absente du gabarit {} — \
                     la compléter depuis le guide du prestataire.",
                    self.cle
                )
            })
    }

    /// Papier par défaut : le premier de la liste.
    pub fn papier_defaut(&self) -> &'static Papier {
        &self.papiers[0]
    }

    pub fn papier(&self, cle: &str) -> Option<&'static Papier> {
        self.papiers.iter().find(|p| p.cle == cle)
    }
}

const PAPIER_UNIQUE_LULU: &[Papier] = &[Papier {
    cle: "standard",
    libelle: "Papier standard",
    teinte: "#ffffff",
    // Formule Lulu, vérifiée sur un livre réel de 244 pages → 15,48 mm.
    dos: Dos::Divise {
        par: 17.48,
        plus: 1.524,
    },
}];

// BoD : dos = pages × épaisseur_feuille/2 + 0,6 mm de couverture 250 g. L'épaisseur
// dépend du papier ; retenu le crème 90 g, défaut de BoD et papier de roman.
// Relevé au calculateur officiel : 280 p → 19,5 mm, 560 p → 38,4 mm.
const PAPIER_UNIQUE_BOD: &[Papier] = &[Papier {
    cle: "creme-90",
    libelle: "Crème 90 g",
    teinte: "#f7f0e0",
    dos: Dos::Multiplie {
        par: 0.0675,
        plus: 0.6,
    },
}];

// KDP : deux papiers noir et blanc, définitifs après publication (page d'aide
// « Create a Paperback Cover »). Aucun terme additif — l'épaisseur de la couverture
// n'entre pas dans le calcul, contrairement à Lulu et BoD.
const PAPIERS_KDP: &[Papier] = &[
    Papier {
        cle: "creme",
        libelle: "Crème",
        teinte: "#f7f0e0",
        dos: Dos::Multiplie {
            par: 0.0635,
            plus: 0.0,
        },
    },
    Papier {
        cle: "blanc",
        libelle: "Blanc",
        teinte: "#ffffff",
        dos: Dos::Multiplie {
            par: 0.0572,
            plus: 0.0,
        },
    },
];

const PAPIER_MESURE: &[Papier] = &[Papier {
    cle: "mesure",
    libelle: "Dos relevé sur le gabarit",
    teinte: "#ffffff",
    dos: Dos::Mesure,
}];

// TheBookEdition : 0,060 mm par page, **quel que soit le papier et quel que soit le
// format**. Ce n'est pas une simplification de notre part — c'est ce que produit leur
// générateur de gabarit (POST sur /fr/module/bookscover/simulationcover, relevé le
// 20/08/2026). Mesuré sur la largeur des gabarits JPEG 300 dpi qu'il renvoie :
// 40 p → 232,41 mm, 100 p → 235,97, 280 p → 246,80, 500 p → 260,01, 750 p → 275,00,
// soit 2 × 110 + 2 × 5 de fond perdu + pages × 0,060 au format Poche. Les mêmes
// paginations sur le papier 120 g, et sur les formats 12x18, 14,8x21 et 21x29,7,
// donnent le même dos à moins de 0,04 mm — l'écart résiduel est l'arrondi au pixel.
const PAPIERS_TBE: &[Papier] = &[
    Papier {
        cle: "munken-80",
        libelle: "Munken 80 g",
        teinte: "#ffffff",
        dos: Dos::Multiplie {
            par: 0.060,
            plus: 0.0,
        },
    },
    Papier {
        cle: "120",
        libelle: "Papier 120 g",
        teinte: "#ffffff",
        dos: Dos::Multiplie {
            par: 0.060,
            plus: 0.0,
        },
    },
];

// Bookvault : dos = pages × épaisseur, sans terme additif. Relevé à leur calculateur
// public (tools.bookvault.app/sizingcalculator) le 20/08/2026, reliure « Perfect
// Bound ». Le 70 g crème est linéaire à la décimale près sur sept paginations
// (40 p → 2,2 mm ; 100 → 5,6 ; 200 → 11,2 ; 280 → 15,7 ; 400 → 22,4 ; 560 → 31,4 ;
// 800 → 44,8) ; les deux autres papiers sont confirmés sur trois paginations chacun.
// Leur guide PDF cite 5,6 mm pour 100 pages de 80 g bond là où le calculateur en
// rend 5,5 : le calculateur fait foi, c'est lui qui produit les gabarits.
const PAPIERS_BOOKVAULT: &[Papier] = &[
    Papier {
        cle: "creme-70",
        libelle: "Crème 70 g",
        teinte: "#f7f0e0",
        dos: Dos::Multiplie {
            par: 0.056,
            plus: 0.0,
        },
    },
    Papier {
        cle: "bond-80",
        libelle: "Bond blanc 80 g",
        teinte: "#ffffff",
        dos: Dos::Multiplie {
            par: 0.055,
            plus: 0.0,
        },
    },
    Papier {
        cle: "creme-premium-80",
        libelle: "Crème premium 80 g",
        teinte: "#f7f0e0",
        dos: Dos::Multiplie {
            par: 0.072,
            plus: 0.0,
        },
    },
];

// Gouttières KDP : le plus grand des deux gabarits publiés — les 19,05 mm du modèle
// de manuscrit, sauf au-delà de 700 pages où le minimum de la tranche (0,875 po) passe
// devant. Identiques aux trois formats de rognage.
const GOUTTIERES_KDP: &[Tranche] = &[(24, 700, 19.05), (701, 828, 22.23)];

// CoolLibri : 20 mm sur les quatre côtés, sans distinction reliure/extérieur et sans
// variation selon la pagination (FAQ : « 2 cm de marges tout autour »). Tranche unique,
// bornée par les paginations admises en dos carré collé.
const GOUTTIERES_COOLLIBRI: &[Tranche] = &[(60, 700, 20.0)];

// TheBookEdition, page « Réussir la mise en page » : 1,25 cm de marge sur les quatre
// côtés pour les formats jusqu'à l'A5, plus 0,5 cm de reliure — d'où 17,5 mm de
// gouttière. Aucune variation selon la pagination n'est publiée : tranche unique,
// bornée par les 40 à 750 pages que leur dos carré collé admet.
const GOUTTIERES_TBE: &[Tranche] = &[(40, 750, 17.5)];

// Bookvault, guide « Your Guide to Supplying Print-Ready PDF Files » (help.bookvault.app),
// page 2 : « Allow a safety margin of 20mm on the gutter ». C'est la seule marge que
// Bookvault impose ; les trois autres restent un choix typographique, repris ci-dessous
// du format déjà en table le plus proche. Le calculateur refuse en dessous de 24 pages
// (« It needs to be at least 1.3mm (24 pages) ») et accepte encore 1000 pages.
const GOUTTIERES_BOOKVAULT: &[Tranche] = &[(24, 1000, 20.0)];

pub const PROVIDERS: &[Provider] = &[
    Provider {
        cle: "lulu",
        libelle: "Lulu — poche 108 × 175",
        format: (108.0, 175.0),
        marge_haut: 14.0,
        marge_bas: 15.0,
        exterieur: 13.0,
        gouttieres: &[(151, 400, 25.0)],
        corps_pt: 9.5,
        interligne: 1.42,
        folio_pt: 8.0,
        fond_perdu: Some(3.175), // 0,125 po
        pages_min: 32,
        pages_max: 800,
        papiers: PAPIER_UNIQUE_LULU,
    },
    Provider {
        cle: "bod",
        libelle: "BoD — 13,5 × 21,5 cm",
        format: (135.0, 215.0),
        marge_haut: 18.8,
        marge_bas: 28.0,
        exterieur: 15.0,
        // BoD ne module pas la marge de reliure selon l'épaisseur — tranche unique,
        // couvrant les 24 à 900 pages que sa couverture souple admet.
        gouttieres: &[(24, 900, 20.0)],
        corps_pt: 9.5,
        interligne: 1.42,
        folio_pt: 8.0,
        fond_perdu: Some(5.0),
        pages_min: 24,
        pages_max: 900,
        papiers: PAPIER_UNIQUE_BOD,
    },
    Provider {
        cle: "kdp-5x8",
        libelle: "Amazon KDP — 5 × 8 po",
        format: (127.0, 203.2),
        marge_haut: 12.7,
        marge_bas: 12.7,
        exterieur: 12.7,
        gouttieres: GOUTTIERES_KDP,
        corps_pt: 9.5,
        interligne: 1.42,
        folio_pt: 8.0,
        fond_perdu: Some(3.175),
        pages_min: 24,
        pages_max: 828,
        papiers: PAPIERS_KDP,
    },
    Provider {
        cle: "kdp-55x85",
        libelle: "Amazon KDP — 5,5 × 8,5 po",
        format: (139.7, 215.9),
        marge_haut: 12.7,
        marge_bas: 12.7,
        exterieur: 12.7,
        gouttieres: GOUTTIERES_KDP,
        corps_pt: 9.5,
        interligne: 1.42,
        folio_pt: 8.0,
        fond_perdu: Some(3.175),
        pages_min: 24,
        pages_max: 828,
        papiers: PAPIERS_KDP,
    },
    Provider {
        cle: "kdp-6x9",
        libelle: "Amazon KDP — 6 × 9 po",
        format: (152.4, 228.6),
        marge_haut: 12.7,
        marge_bas: 12.7,
        exterieur: 12.7,
        gouttieres: GOUTTIERES_KDP,
        corps_pt: 9.5,
        interligne: 1.42,
        folio_pt: 8.0,
        fond_perdu: Some(3.175),
        pages_min: 24,
        pages_max: 828,
        papiers: PAPIERS_KDP,
    },
    Provider {
        cle: "coollibri-110x170",
        libelle: "CoolLibri — 11 × 17 cm",
        format: (110.0, 170.0),
        marge_haut: 20.0,
        marge_bas: 20.0,
        exterieur: 20.0,
        gouttieres: GOUTTIERES_COOLLIBRI,
        corps_pt: 9.5,
        interligne: 1.42,
        folio_pt: 8.0,
        fond_perdu: None,
        pages_min: 60,
        pages_max: 700,
        papiers: PAPIER_MESURE,
    },
    Provider {
        cle: "coollibri-148x210",
        libelle: "CoolLibri — A5",
        format: (148.0, 210.0),
        marge_haut: 20.0,
        marge_bas: 20.0,
        exterieur: 20.0,
        gouttieres: GOUTTIERES_COOLLIBRI,
        corps_pt: 9.5,
        interligne: 1.42,
        folio_pt: 8.0,
        fond_perdu: None,
        pages_min: 60,
        pages_max: 700,
        papiers: PAPIER_MESURE,
    },
    Provider {
        cle: "coollibri-160x240",
        libelle: "CoolLibri — 16 × 24 cm",
        format: (160.0, 240.0),
        marge_haut: 20.0,
        marge_bas: 20.0,
        exterieur: 20.0,
        gouttieres: GOUTTIERES_COOLLIBRI,
        corps_pt: 9.5,
        interligne: 1.42,
        folio_pt: 8.0,
        fond_perdu: None,
        pages_min: 60,
        pages_max: 700,
        papiers: PAPIER_MESURE,
    },
    // TheBookEdition — trois des neuf formats admis en dos carré collé, ceux qui
    // servent au roman. Fond perdu de 5 mm : leur générateur rend une planche haute
    // de la hauteur du livre + 10 mm, sur les cinq formats mesurés. Marges du guide
    // de mise en page (12,5 mm, plus 5 mm de reliure).
    Provider {
        cle: "tbe-110x170",
        libelle: "TheBookEdition — Poche 11 × 17",
        format: (110.0, 170.0),
        marge_haut: 12.5,
        marge_bas: 12.5,
        exterieur: 12.5,
        gouttieres: GOUTTIERES_TBE,
        corps_pt: 9.5,
        interligne: 1.42,
        folio_pt: 8.0,
        fond_perdu: Some(5.0),
        pages_min: 40,
        pages_max: 750,
        papiers: PAPIERS_TBE,
    },
    Provider {
        cle: "tbe-120x180",
        libelle: "TheBookEdition — Manga 12 × 18",
        format: (120.0, 180.0),
        marge_haut: 12.5,
        marge_bas: 12.5,
        exterieur: 12.5,
        gouttieres: GOUTTIERES_TBE,
        corps_pt: 9.5,
        interligne: 1.42,
        folio_pt: 8.0,
        fond_perdu: Some(5.0),
        pages_min: 40,
        pages_max: 750,
        papiers: PAPIERS_TBE,
    },
    Provider {
        // 148,5 et non 148 : c'est la largeur que déclare leur table des formats,
        // et c'est elle qui dimensionne le gabarit de couverture.
        cle: "tbe-1485x210",
        libelle: "TheBookEdition — A5 14,8 × 21",
        format: (148.5, 210.0),
        marge_haut: 12.5,
        marge_bas: 12.5,
        exterieur: 12.5,
        gouttieres: GOUTTIERES_TBE,
        corps_pt: 9.5,
        interligne: 1.42,
        folio_pt: 8.0,
        fond_perdu: Some(5.0),
        pages_min: 40,
        pages_max: 750,
        papiers: PAPIERS_TBE,
    },
    // Bookvault — trois des onze formats de leur calculateur. Fond perdu de 3 mm, sur
    // les quatre côtés de la planche (guide PDF, « Paperback Book - Cover Setup »).
    Provider {
        cle: "bookvault-127x203",
        libelle: "Bookvault — Novel 127 × 203",
        format: (127.0, 203.0),
        // Format à 0,2 mm du KDP 5 × 8 déjà en table : ses marges sont reprises telles
        // quelles, faute de valeur publiée par Bookvault hors gouttière.
        marge_haut: 12.7,
        marge_bas: 12.7,
        exterieur: 12.7,
        gouttieres: GOUTTIERES_BOOKVAULT,
        corps_pt: 9.5,
        interligne: 1.42,
        folio_pt: 8.0,
        fond_perdu: Some(3.0),
        pages_min: 24,
        pages_max: 1000,
        papiers: PAPIERS_BOOKVAULT,
    },
    Provider {
        cle: "bookvault-129x198",
        libelle: "Bookvault — B Format 129 × 198",
        marge_haut: 12.7,
        marge_bas: 12.7,
        exterieur: 12.7,
        format: (129.0, 198.0),
        gouttieres: GOUTTIERES_BOOKVAULT,
        corps_pt: 9.5,
        interligne: 1.42,
        folio_pt: 8.0,
        fond_perdu: Some(3.0),
        pages_min: 24,
        pages_max: 1000,
        papiers: PAPIERS_BOOKVAULT,
    },
    Provider {
        cle: "bookvault-148x210",
        libelle: "Bookvault — A5 148 × 210",
        format: (148.0, 210.0),
        // Format identique au CoolLibri A5 : mêmes marges que lui.
        marge_haut: 20.0,
        marge_bas: 20.0,
        exterieur: 20.0,
        gouttieres: GOUTTIERES_BOOKVAULT,
        corps_pt: 9.5,
        interligne: 1.42,
        folio_pt: 8.0,
        fond_perdu: Some(3.0),
        pages_min: 24,
        pages_max: 1000,
        papiers: PAPIERS_BOOKVAULT,
    },
];

pub fn provider(cle: &str) -> Option<&'static Provider> {
    PROVIDERS.iter().find(|p| p.cle == cle)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(cle: &str) -> &'static Provider {
        provider(cle).unwrap_or_else(|| panic!("prestataire inconnu : {cle}"))
    }

    /// Le dos est ce que l'app promet à l'imprimeur : chaque formule est ancrée sur
    /// un relevé réel, pas sur sa propre arithmétique. Si l'un de ces chiffres bouge,
    /// c'est le guide du prestataire qui a changé — pas un détail d'implémentation.
    #[test]
    fn dos_lulu_ancre_sur_le_livre_reel_de_244_pages() {
        let dos = p("lulu").papier_defaut().dos.mm(244).unwrap();
        assert!(
            (dos - 15.48).abs() < 0.01,
            "244 pages → {dos} mm, attendu 15,48"
        );
    }

    #[test]
    fn dos_bod_ancre_sur_le_calculateur_officiel() {
        let d = p("bod").papier_defaut().dos;
        assert!((d.mm(280).unwrap() - 19.5).abs() < 0.05);
        assert!((d.mm(560).unwrap() - 38.4).abs() < 0.05);
    }

    #[test]
    fn dos_kdp_depend_du_papier_et_seulement_du_papier() {
        let kdp = p("kdp-6x9");
        let creme = kdp.papier("creme").unwrap().dos.mm(280).unwrap();
        let blanc = kdp.papier("blanc").unwrap().dos.mm(280).unwrap();
        assert!((creme - 17.78).abs() < 0.01, "crème → {creme} mm");
        assert!((blanc - 16.02).abs() < 0.01, "blanc → {blanc} mm");
        // Le papier ne change que le dos : les trois formats KDP partagent la même
        // composition d'intérieur, donc la même pagination.
        for f in ["kdp-5x8", "kdp-55x85", "kdp-6x9"] {
            assert_eq!(p(f).gouttieres, GOUTTIERES_KDP);
        }
    }

    /// Chez TheBookEdition, le dos ne dépend que de la pagination : leur générateur rend
    /// le même gabarit sur les deux papiers et sur les quatre formats mesurés. Faire
    /// dépendre le dos du papier ici produirait une planche que leur gabarit refuse.
    #[test]
    fn dos_tbe_ancre_sur_les_gabarits_releves() {
        let poche = p("tbe-110x170");
        for (pages, attendu) in [(40, 2.4), (280, 16.8), (750, 45.0)] {
            let dos = poche.papier_defaut().dos.mm(pages).unwrap();
            assert!((dos - attendu).abs() < 0.05, "{pages} p → {dos} mm");
        }
        for papier in poche.papiers {
            assert_eq!(papier.dos.mm(280), poche.papier_defaut().dos.mm(280));
        }
        for cle in ["tbe-120x180", "tbe-1485x210"] {
            assert_eq!(
                p(cle).papier_defaut().dos.mm(280),
                poche.papier_defaut().dos.mm(280)
            );
        }
    }

    /// Bookvault, à l'inverse, module le dos par le papier : le crème premium fait un
    /// livre visiblement plus épais que le bond blanc à pagination égale.
    #[test]
    fn dos_bookvault_ancre_sur_le_calculateur_papier_par_papier() {
        let bv = p("bookvault-127x203");
        for (cle, pages, attendu) in [
            ("creme-70", 280, 15.7),
            ("creme-70", 800, 44.8),
            ("bond-80", 100, 5.5),
            ("creme-premium-80", 400, 28.8),
        ] {
            let dos = bv.papier(cle).unwrap().dos.mm(pages).unwrap();
            assert!((dos - attendu).abs() < 0.05, "{cle} à {pages} p → {dos} mm");
        }
    }

    /// Le fond perdu est ce qui sépare une planche imprimable d'une planche rejetée.
    /// Chaque valeur vient du gabarit du prestataire, aucune n'est un défaut commun.
    #[test]
    fn le_fond_perdu_est_celui_du_gabarit_de_chaque_prestataire() {
        assert_eq!(p("tbe-110x170").fond_perdu, Some(5.0));
        assert_eq!(p("bookvault-127x203").fond_perdu, Some(3.0));
        assert_eq!(p("coollibri-110x170").fond_perdu, None);
    }

    /// La gouttière se lit dans la tranche, elle ne s'interpole pas : une page de plus
    /// peut la faire basculer, et c'est précisément ce qui oblige à recomposer.
    #[test]
    fn la_gouttiere_bascule_a_la_frontiere_de_tranche() {
        let kdp = p("kdp-6x9");
        assert_eq!(kdp.gouttiere(700).unwrap(), 19.05);
        assert_eq!(kdp.gouttiere(701).unwrap(), 22.23);
    }

    /// Hors tranche connue, on refuse. Inventer une gouttière produirait un intérieur
    /// que le prestataire rejetterait sans que rien ne l'ait signalé.
    #[test]
    fn hors_tranche_le_gabarit_refuse_au_lieu_d_inventer() {
        let err = p("lulu").gouttiere(100).unwrap_err();
        assert!(err.contains("100 pages"), "message peu explicite : {err}");
        assert!(err.contains("lulu"));
    }

    /// Les prestataires à gabarit ne publient pas de formule : l'app ne doit pas
    /// pouvoir en fabriquer une, quelle que soit la pagination.
    #[test]
    fn un_prestataire_a_gabarit_ne_calcule_jamais_de_dos() {
        let cl = p("coollibri-148x210");
        assert_eq!(cl.papier_defaut().dos.mm(280), None);
        assert_eq!(cl.papier_defaut().dos.mm(9999), None);
        assert_eq!(cl.fond_perdu, None, "le fond perdu se relève aussi");
    }

    #[test]
    fn chaque_prestataire_a_un_papier_par_defaut_et_des_bornes_coherentes() {
        for pr in PROVIDERS {
            assert!(!pr.papiers.is_empty(), "{} sans papier", pr.cle);
            assert!(pr.pages_min < pr.pages_max, "{} : bornes inversées", pr.cle);
            assert!(!pr.gouttieres.is_empty(), "{} sans tranche", pr.cle);
            for (lo, hi, g) in pr.gouttieres {
                assert!(lo <= hi, "{} : tranche inversée", pr.cle);
                assert!(*g > 0.0, "{} : gouttière nulle", pr.cle);
            }
        }
    }

    /// La largeur utile doit rester positive : une gouttière plus large que le format
    /// donnerait une colonne de texte négative, et Typst composerait n'importe quoi.
    #[test]
    fn la_colonne_de_texte_reste_positive_sur_toute_la_pagination() {
        for pr in PROVIDERS {
            for (lo, _, g) in pr.gouttieres {
                let utile = pr.format.0 - g - pr.exterieur;
                assert!(
                    utile > 30.0,
                    "{} à {lo} pages : colonne de {utile} mm",
                    pr.cle
                );
            }
        }
    }

    /// Chaque papier dit sa couleur, en notation CSS : c'est le front qui la peint, et
    /// une conversion en chemin serait une occasion de se tromper. La valeur est une
    /// convention d'Ozalid, pas une mesure — aucun prestataire ne publie la teinte de
    /// son crème.
    #[test]
    fn chaque_papier_annonce_sa_teinte() {
        for p in PROVIDERS {
            for pa in p.papiers {
                assert!(
                    pa.teinte.len() == 7 && pa.teinte.starts_with('#'),
                    "{} / {} : teinte « {} » illisible en CSS",
                    p.cle,
                    pa.cle,
                    pa.teinte
                );
            }
        }
    }
}
