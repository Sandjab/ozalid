//! Composition de l'intérieur : source Typst, et convergence gouttière/parité.
//!
//! Deux conditions doivent être satisfaites **ensemble** : la gouttière doit
//! correspondre à la tranche de pagination effective, et le compte de pages doit être
//! pair — une feuille porte deux pages, les prestataires refusent l'impair. Chacune
//! peut déplacer la pagination, d'où la reprise.
//!
//! Le compte de pages produit ici est celui que consomme la couverture pour calculer
//! le dos. Il ne transite par aucune saisie humaine : c'est la raison d'être de l'app.

use serde::{Deserialize, Serialize};

use crate::manuscrit::{echappe, inline, Chapitre};
use crate::projet::Livre;
use crate::providers::Provider;
use crate::typst::MARQUEUR;

/// Les polices que l'intérieur admet.
///
/// Volontairement plus courte que `couverture::POLICES` : ce sont les seules qui
/// tiennent trois cents pages de corps de texte, chacune avec un vrai italique. Un
/// titrage comme Oswald ferait un roman illisible, et l'erreur ne se découvrirait
/// qu'après tirage.
pub const POLICES_TEXTE: &[&str] = &[
    "EB Garamond",
    "Crimson Pro",
    "Alegreya",
    "Cardo",
    "Vollkorn",
    "Spectral",
    "Libre Baskerville",
];

fn police_defaut() -> String {
    "EB Garamond".into()
}

/// Réglages d'intérieur du projet.
///
/// Le prestataire impose le format, les marges, la gouttière et le corps ; le livre
/// choisit son caractère. C'est la raison pour laquelle la police n'est pas un champ
/// de `Provider`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Interieur {
    #[serde(default = "police_defaut")]
    pub police: String,
}

impl Default for Interieur {
    fn default() -> Self {
        Self {
            police: police_defaut(),
        }
    }
}

impl Interieur {
    /// Refuse une police absente de la liste.
    ///
    /// Sans ce contrôle, Typst composerait dans sa police par défaut **sans lever
    /// d'erreur** : `--ignore-system-fonts` empêche une substitution par le système,
    /// pas une substitution par le défaut du binaire.
    pub fn verifie(&self) -> Result<(), String> {
        if POLICES_TEXTE.contains(&self.police.as_str()) {
            return Ok(());
        }
        Err(format!(
            "police d'intérieur inconnue : « {} ». Attendu : {}.",
            self.police,
            POLICES_TEXTE.join(", ")
        ))
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Reglage {
    pub gouttiere: f64,
    /// Page blanche de fin, sans folio, pour ramener le compte à un nombre pair.
    pub blanche: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Resultat {
    pub pages: u32,
    pub gouttiere: f64,
    pub blanche: bool,
}

/// Nombre de reprises avant d'admettre que la composition n'a pas de point fixe.
/// La bascule de parité converge en un tour puisqu'elle change le compte de 1
/// exactement ; il ne reste à absorber que les changements de tranche.
const REPRISES: usize = 4;

/// Cherche le réglage stable, en ne mesurant que le compte de pages.
///
/// `mesure` compose et rend le compte, sans produire de PDF : la convergence ne coûte
/// donc aucun fichier jeté. Elle est injectée pour que la boucle soit testable sans
/// binaire Typst — c'est de la logique métier, pas de l'orchestration de processus.
pub fn converge(
    pr: &Provider,
    mut mesure: impl FnMut(&Reglage) -> Result<u32, String>,
) -> Result<Resultat, String> {
    let mut r = Reglage {
        // Hypothèse de départ : la première tranche du gabarit.
        gouttiere: pr.gouttieres[0].2,
        blanche: false,
    };
    for _ in 0..REPRISES {
        let pages = mesure(&r)?;
        // Sort proprement si la tranche est inconnue, plutôt que d'inventer.
        let g = pr.gouttiere(pages)?;
        if (g - r.gouttiere).abs() > f64::EPSILON {
            r.gouttiere = g;
            continue;
        }
        if pages % 2 == 1 {
            r.blanche = !r.blanche;
            continue;
        }
        return Ok(Resultat {
            pages,
            gouttiere: r.gouttiere,
            blanche: r.blanche,
        });
    }
    Err("la composition ne converge pas (gouttière ou parité oscillantes).".into())
}

/// Source Typst complète de l'intérieur.
pub fn source(
    livre: &Livre,
    int: &Interieur,
    pr: &Provider,
    r: &Reglage,
    chapitres: &[Chapitre],
) -> String {
    let (fw, fh) = pr.format;
    // `leading` Typst = espace entre lignes ; `line-height` CSS = distance entre lignes
    // de base. Les deux ne coïncident qu'une fois la boîte de ligne ramenée à 1em par
    // top-edge/bottom-edge — sans quoi l'interligne dépend de la police choisie.
    let lead = pr.interligne - 1.0;
    let folio = format!(
        r#"context align(center, text(size: {}pt, counter(page).display()))"#,
        pr.folio_pt
    );

    let mut s = String::new();
    s.push_str(&format!(
        r#"// Intérieur — {} ({})
#set document(title: "{}", author: "{}")
#set page(
  width: {fw}mm, height: {fh}mm,
  margin: (top: {}mm, bottom: {}mm, inside: {}mm, outside: {}mm),
  footer: none,
)
#set text(font: "{}", size: {}pt, lang: "fr", hyphenate: true,
          top-edge: 0.75em, bottom-edge: -0.25em,
          costs: (orphan: 100%, widow: 100%))
#set par(justify: true, leading: {lead}em, spacing: {lead}em, first-line-indent: 1.2em)

"#,
        echappe(&livre.titre),
        pr.cle,
        echappe(&livre.titre),
        echappe(&livre.auteur),
        pr.marge_haut,
        pr.marge_bas,
        r.gouttiere,
        pr.exterieur,
        // La police est validée en amont par `Interieur::verifie` : pas d'échappement.
        int.police,
        pr.corps_pt,
    ));

    // — Liminaires, sans folio : faux-titre, blanche, page de titre, copyright —
    s.push_str(&format!(
        r#"#v(42mm)
#align(center, text(size: 11pt, tracking: 0.12em)[{}])
#pagebreak()
#pagebreak()

#v(30mm)
#align(center, text(size: 10.5pt, tracking: 0.1em)[{}])
#v(14mm)
#align(center, text(size: 15pt, tracking: 0.06em)[{}])
#v(10mm)
#align(center, emph(text(size: 10pt)[{}]))
#pagebreak()

"#,
        majuscules(&livre.titre),
        majuscules(&livre.auteur),
        majuscules(&livre.titre_page().replace('\n', "\u{1}")).replace('\u{1}', r" \ "),
        echappe(&livre.genre),
    ));

    // Le pavé de copyright est calé en bas de la justification. La chaîne Python le
    // posait à 143 mm du haut du corps — une valeur juste pour le poche Lulu et
    // arbitraire ailleurs ; le bas de la justification est la même intention, exprimée
    // indépendamment du format.
    s.push_str(&format!(
        r#"#place(bottom + center, block(width: 100%)[
  #set par(leading: 0.5em, spacing: 0.5em, first-line-indent: 0pt, justify: false)
  #align(center, text(size: 8pt)[{}])
])
#pagebreak()

"#,
        echappe(&livre.copyright).replace('\n', r" \ ")
    ));

    // — Corps, folio rétabli. La numérotation court depuis le faux-titre, seul son
    //   affichage était supprimé : le premier chapitre s'ouvre donc en page 5. —
    s.push_str(&format!("#set page(footer: {folio})\n"));

    for (i, ch) in chapitres.iter().enumerate() {
        // Le premier chapitre suit le saut de page du copyright : ne pas en ajouter un.
        if i > 0 {
            s.push_str("#pagebreak()\n");
        }
        s.push_str(&format!(
            "#v(22mm)\n#align(center, text(size: 13pt)[{}])\n",
            ch.numero
        ));
        if !ch.titre.is_empty() {
            s.push_str(&format!(
                "#v(3.5mm)\n#align(center, text(size: 10pt, tracking: 0.14em)[{}])\n",
                majuscules(&ch.titre)
            ));
        }
        s.push_str("#v(11mm)\n");
        for p in &ch.paragraphes {
            s.push_str(&inline(p));
            s.push_str("\n\n");
        }
    }

    // Page blanche de fin, sans folio — même dispositif que la blanche des liminaires.
    if r.blanche {
        s.push_str("\n#page(footer: none)[]\n");
    }
    s.push_str(&format!("\n{MARQUEUR}\n"));
    s
}

/// Majuscules typographiques : `upper()` de Typst plutôt qu'une bascule en Rust, pour
/// que la casse suive la langue du document (le CSS faisait `text-transform`).
fn majuscules(s: &str) -> String {
    format!("#upper[{}]", echappe(s))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::provider;
    use std::cell::RefCell;

    fn livre() -> Livre {
        Livre {
            titre: "Les Heures creuses".into(),
            titre_page: Some("Les Heures\ncreuses".into()),
            auteur: "Ivan Pjig".into(),
            genre: "roman".into(),
            copyright: "© Ivan Pjig, 2026.\nTous droits réservés.".into(),
            chapitres: None,
        }
    }

    fn chapitres() -> Vec<Chapitre> {
        vec![Chapitre {
            numero: 1,
            titre: "Un".into(),
            paragraphes: vec!["Texte.".into()],
        }]
    }

    /// Une composition déjà stable ne doit pas être recomposée : une reprise inutile
    /// coûte une passe de mise en page sur tout le livre.
    #[test]
    fn une_composition_stable_converge_du_premier_coup() {
        let pr = provider("lulu").unwrap();
        let appels = RefCell::new(0);
        let r = converge(pr, |_| {
            *appels.borrow_mut() += 1;
            Ok(272)
        })
        .unwrap();
        assert_eq!(r.pages, 272);
        assert_eq!(r.gouttiere, 25.0);
        assert!(!r.blanche);
        assert_eq!(*appels.borrow(), 1);
    }

    /// Un compte impair est corrigé par la blanche de fin, et le compte retenu est
    /// celui de la composition **avec** la blanche — pas celui d'avant.
    #[test]
    fn un_compte_impair_ajoute_la_blanche_et_repart_du_nouveau_compte() {
        let pr = provider("lulu").unwrap();
        let n = RefCell::new(0);
        let r = converge(pr, |reglage| {
            *n.borrow_mut() += 1;
            Ok(if reglage.blanche { 272 } else { 271 })
        })
        .unwrap();
        assert!(r.blanche);
        assert_eq!(r.pages, 272);
        assert_eq!(*n.borrow(), 2);
    }

    /// Le cas qui justifie la boucle : la gouttière dépend de la pagination, et la
    /// changer déplace la pagination. Le réglage retenu doit être cohérent avec le
    /// compte final, pas avec l'hypothèse de départ.
    #[test]
    fn un_changement_de_tranche_recompose_avec_la_bonne_gouttiere() {
        let pr = provider("kdp-6x9").unwrap();
        let r = converge(pr, |reglage| {
            // Avec la gouttière étroite le livre tient en 700 pages ; l'élargir le
            // fait passer dans la tranche suivante, qui impose l'autre gouttière.
            Ok(if reglage.gouttiere < 20.0 { 702 } else { 720 })
        })
        .unwrap();
        assert_eq!(r.gouttiere, 22.23);
        assert_eq!(r.pages, 720);
    }

    /// Hors tranche connue, la convergence s'arrête sur le message du gabarit : elle
    /// ne doit pas boucler ni retenir une gouttière inventée.
    #[test]
    fn une_pagination_hors_tranche_interrompt_la_convergence() {
        let pr = provider("lulu").unwrap();
        let err = converge(pr, |_| Ok(100)).unwrap_err();
        assert!(err.contains("100 pages"), "{err}");
    }

    /// Une oscillation doit finir par échouer plutôt que tourner sans fin — sans quoi
    /// l'app se figerait sur un manuscrit pathologique.
    #[test]
    fn une_oscillation_est_bornee_et_signalee() {
        let pr = provider("lulu").unwrap();
        let tour = RefCell::new(0u32);
        let err = converge(pr, |_| {
            let mut t = tour.borrow_mut();
            *t += 1;
            Ok(if (*t).is_multiple_of(2) { 271 } else { 273 })
        })
        .unwrap_err();
        assert!(err.contains("ne converge pas"), "{err}");
    }

    #[test]
    fn la_source_porte_le_gabarit_du_prestataire_et_le_marqueur() {
        let pr = provider("bod").unwrap();
        let r = Reglage {
            gouttiere: 20.0,
            blanche: false,
        };
        let s = source(&livre(), &Interieur::default(), pr, &r, &[]);
        assert!(s.contains("width: 135mm, height: 215mm"));
        assert!(s.contains("inside: 20mm"), "gouttière absente");
        assert!(s.contains("outside: 15mm"));
        assert!(s.contains("costs: (orphan: 100%, widow: 100%)"), "veuves");
        assert!(s.trim_end().ends_with(MARQUEUR), "marqueur de pagination");
    }

    /// La blanche de fin doit être sans folio : un numéro sur une page vide de fin est
    /// un défaut d'impression visible.
    #[test]
    fn la_blanche_de_fin_est_sans_folio() {
        let pr = provider("lulu").unwrap();
        let sans = source(
            &livre(),
            &Interieur::default(),
            pr,
            &Reglage {
                gouttiere: 25.0,
                blanche: false,
            },
            &[],
        );
        let avec = source(
            &livre(),
            &Interieur::default(),
            pr,
            &Reglage {
                gouttiere: 25.0,
                blanche: true,
            },
            &[],
        );
        assert!(!sans.contains("#page(footer: none)[]"));
        assert!(avec.contains("#page(footer: none)[]"));
    }

    /// Le titre de la page de titre garde ses sauts de ligne voulus, et rien de ce qui
    /// vient du projet ne peut ouvrir une expression Typst.
    #[test]
    fn le_titre_de_page_garde_ses_sauts_de_ligne_et_reste_echappe() {
        let pr = provider("lulu").unwrap();
        let mut l = livre();
        l.titre_page = Some("Les Heures\ncreuses".into());
        l.auteur = "Ivan #Pjig".into();
        let s = source(
            &l,
            &Interieur::default(),
            pr,
            &Reglage {
                gouttiere: 25.0,
                blanche: false,
            },
            &[],
        );
        assert!(s.contains(r"Les Heures \ creuses"), "saut de ligne perdu");
        assert!(s.contains(r"Ivan \#Pjig"), "auteur non échappé");
    }

    /// Une police que Typst ne connaît pas ne lève aucune erreur à la composition : il
    /// compose dans sa police par défaut, en silence. C'est ainsi que l'intérieur est
    /// resté en Libertinus Serif pendant quatre jalons. Le refus est donc ici, en
    /// amont, ou il n'est nulle part.
    #[test]
    fn une_police_hors_liste_est_refusee_et_non_substituee() {
        let i = Interieur {
            police: "Comic Sans MS".into(),
        };
        let e = i.verifie().unwrap_err();
        assert!(
            e.contains("Comic Sans MS"),
            "l'erreur ne nomme pas la police : {e}"
        );
        assert!(
            e.contains("EB Garamond"),
            "l'erreur ne dit pas ce qui est attendu : {e}"
        );
    }

    /// Les sept polices offertes doivent toutes passer : une liste qui contient une
    /// entrée que la validation refuse est une porte fermée sur elle-même.
    #[test]
    fn les_polices_offertes_sont_toutes_acceptees() {
        for p in POLICES_TEXTE {
            let i = Interieur {
                police: (*p).into(),
            };
            assert!(i.verifie().is_ok(), "{p} offerte mais refusée");
        }
    }

    /// La police doit être déclarée, et une seule fois. Deux `#set text(font: …)` dans
    /// la même source, c'est le second qui gagne — donc un réglage qui paraît obéi
    /// alors qu'il ne l'est pas.
    #[test]
    fn la_source_declare_la_police_du_projet_une_seule_fois() {
        let pr = provider("lulu").unwrap();
        let r = Reglage {
            gouttiere: 25.0,
            blanche: false,
        };
        let int = Interieur {
            police: "Cardo".into(),
        };
        let s = source(&livre(), &int, pr, &r, &chapitres());
        assert_eq!(s.matches("font:").count(), 1);
        assert!(s.contains(r#"font: "Cardo""#), "police du projet ignorée");
    }

    /// Le premier chapitre suit déjà le saut de page du copyright : un saut de plus
    /// laisserait une page blanche parasite, qui décalerait toute la pagination.
    #[test]
    fn le_premier_chapitre_n_ajoute_pas_de_saut_de_page() {
        let pr = provider("lulu").unwrap();
        let chs = vec![
            Chapitre {
                numero: 1,
                titre: "Un".into(),
                paragraphes: vec!["A.".into()],
            },
            Chapitre {
                numero: 2,
                titre: "Deux".into(),
                paragraphes: vec!["B.".into()],
            },
        ];
        let s = source(
            &livre(),
            &Interieur::default(),
            pr,
            &Reglage {
                gouttiere: 25.0,
                blanche: false,
            },
            &chs,
        );
        let corps = s.split("#set page(footer: context").nth(1).unwrap();
        assert_eq!(
            corps.matches("#pagebreak()").count(),
            1,
            "un seul saut, entre les deux chapitres"
        );
    }
}
