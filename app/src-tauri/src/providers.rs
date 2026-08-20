//! Table unique des gabarits prestataires.
//!
//! Elle fusionne les deux tables historiques du projet, qui décrivaient les mêmes
//! prestataires sans jamais se recouper : le `PROVIDERS` d'`index.html` (couverture :
//! fond perdu, formule de dos) et celui de `outils/gen_interieur.py` (intérieur :
//! format, marges, gouttières, typographie). Un prestataire s'ajoute désormais à un
//! seul endroit, et le nombre de pages ne peut plus désigner deux formats différents.
//!
//! Toutes les valeurs proviennent des relevés déjà documentés dans ces deux fichiers
//! et dans `COOKBOOK.md` ; aucune n'est reconstituée.

/// Épaisseur du dos. Trois formes, parce que les prestataires en publient trois.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Dos {
    /// Lulu : `pages / 17,48 + 1,524` mm. Gardée sous forme de division, comme le
    /// guide l'écrit — la convertir en facteur décimal introduirait une dérive.
    Divise { par: f64, plus: f64 },
    /// BoD et KDP : `pages × épaisseur + constante` mm. La constante vaut 0 chez KDP,
    /// qui ne compte pas l'épaisseur de la couverture.
    Multiplie { par: f64, plus: f64 },
    /// CoolLibri, TheBookEdition, Bookvault : aucune formule publiable (la « main »
    /// des papiers manque). Le dos se relève sur leur gabarit, il ne se calcule pas.
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
        dos: Dos::Multiplie {
            par: 0.0635,
            plus: 0.0,
        },
    },
    Papier {
        cle: "blanc",
        libelle: "Blanc",
        dos: Dos::Multiplie {
            par: 0.0572,
            plus: 0.0,
        },
    },
];

const PAPIER_MESURE: &[Papier] = &[Papier {
    cle: "mesure",
    libelle: "Dos relevé sur le gabarit",
    dos: Dos::Mesure,
}];

// Gouttières KDP : le plus grand des deux gabarits publiés — les 19,05 mm du modèle
// de manuscrit, sauf au-delà de 700 pages où le minimum de la tranche (0,875 po) passe
// devant. Identiques aux trois formats de rognage.
const GOUTTIERES_KDP: &[Tranche] = &[(24, 700, 19.05), (701, 828, 22.23)];

// CoolLibri : 20 mm sur les quatre côtés, sans distinction reliure/extérieur et sans
// variation selon la pagination (FAQ : « 2 cm de marges tout autour »). Tranche unique,
// bornée par les paginations admises en dos carré collé.
const GOUTTIERES_COOLLIBRI: &[Tranche] = &[(60, 700, 20.0)];

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
}
