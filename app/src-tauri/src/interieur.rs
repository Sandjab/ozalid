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

use crate::manuscrit::{echappe, echappe_chaine, inline, Bloc, Piece, Sorte, SCENE};
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

/// Ce qu'un envoi dépose sur la page de titre.
///
/// `interieur` ne connaît pas la main du livre : il reçoit ce qu'elle a décidé. Une
/// image écrite à la main et une image produite par un modèle de diffusion arrivent
/// ici de la même façon — ce module n'a pas à savoir d'où l'image vient, seulement
/// qu'elle est posée à côté de la source.
#[derive(Debug, Clone, Copy)]
pub enum Trace<'a> {
    /// Un texte, composé dans la main du livre.
    Texte { police: &'a str, texte: &'a str },
    /// Une image, déjà écrite à côté de la source, désignée par son seul nom.
    Image { fichier: &'a str },
}

/// Source Typst complète de l'intérieur.
fn assemble(
    livre: &Livre,
    int: &Interieur,
    pr: &Provider,
    r: &Reglage,
    pieces: &[Piece],
    envoi: Option<Trace>,
    avant: Option<&str>,
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

    // Les zones sont déjà validées par `decoupe` : le découpage n'a qu'à les suivre.
    let lim = pieces
        .iter()
        .take_while(|p| matches!(p.sorte, Sorte::Liminaire))
        .count();
    let (liminaires_manuscrit, reste) = pieces.split_at(lim);
    let corps = reste
        .iter()
        .take_while(|p| !matches!(p.sorte, Sorte::Annexe))
        .count();
    let (corps, annexes) = reste.split_at(corps);

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
        // Ces trois-là sont cités, non composés : la ligne de commentaire et la chaîne
        // de `#set document` demandent l'échappement de chaîne, pas celui du markup.
        echappe_chaine(&livre.titre),
        pr.cle,
        echappe_chaine(&livre.titre),
        echappe_chaine(&livre.auteur),
        pr.marge_haut,
        pr.marge_bas,
        r.gouttiere,
        pr.exterieur,
        // La police est validée en amont par `Interieur::verifie` : pas d'échappement.
        int.police,
        pr.corps_pt,
    ));

    // La page insérée vient avant tout ce que `liminaires` écrit : c'est la page 1 du
    // fichier, celle qu'un lecteur voit en ouvrant.
    if let Some(a) = avant {
        s.push_str(a);
    }

    s.push_str(&liminaires(livre, envoi, liminaires_manuscrit));

    // — Corps, folio rétabli. La numérotation court depuis le faux-titre, seul son
    //   affichage était supprimé : le premier chapitre s'ouvre donc en page 5, ou en 7
    //   quand le livre porte une dédicace. —
    s.push_str(&format!("#set page(footer: {folio})\n"));

    // `#page(…)[…]` rompt le flux de lui-même, avant et après : après une page de
    // partie, le `#pagebreak()` d'ouverture du chapitre suivant ferait une page blanche
    // de plus. Le compte de pages est le seul juge de ce détail.
    let mut apres_page = false;
    for (i, p) in corps.iter().enumerate() {
        match &p.sorte {
            Sorte::Partie(r) => {
                // Une ouverture de partie est une belle page. Le verso blanc, lui, est
                // acquis par le second `#page` — mais le recto ne l'est pas : au milieu
                // du corps, la parité dépend de la longueur du chapitre précédent, donc
                // d'un texte que l'auteur retouche. Sans ce saut, trois paragraphes
                // ajoutés au chapitre d'avant retournent le dispositif, et cela ne se
                // découvre qu'après tirage.
                //
                // Le saut n'est pas un `pagebreak(to: "odd")` : la page qu'il insère
                // hérite du folio du corps, et une page entièrement vide portant son
                // numéro au milieu du livre se remarque — aucune édition courante ne le
                // fait. La blanche est donc posée ici, sans folio, en regardant la
                // parité de la page où le flux se trouve : la partie ouvre la suivante,
                // donc c'est une page **impaire** en cours qui appelle une blanche.
                s.push_str("#context if calc.odd(here().page()) { page(footer: none)[] }\n");
                s.push_str(&format!(
                    "#page(footer: none)[\n#v(22mm)\n\
                     #align(center, text(size: 13pt)[{r}])\n"
                ));
                if !p.titre.is_empty() {
                    s.push_str(&format!(
                        "#v(3.5mm)\n\
                         #align(center, text(size: 10pt, tracking: 0.14em)[{}])\n",
                        majuscules(&p.titre)
                    ));
                }
                s.push_str("]\n#page(footer: none)[]\n");
                apres_page = true;
            }
            Sorte::Chapitre(numero) => {
                // Le premier chapitre suit le dernier saut de page des liminaires : ne
                // pas en ajouter un.
                if i > 0 && !apres_page {
                    s.push_str("#pagebreak()\n");
                }
                s.push_str(&format!(
                    "#v(22mm)\n#align(center, text(size: 13pt)[{numero}])\n"
                ));
                if !p.titre.is_empty() {
                    s.push_str(&format!(
                        "#v(3.5mm)\n\
                         #align(center, text(size: 10pt, tracking: 0.14em)[{}])\n",
                        majuscules(&p.titre)
                    ));
                }
                s.push_str("#v(11mm)\n");
                s.push_str(&blocs_typst(&p.blocs));
                apres_page = false;
            }
            // `decoupe` garantit les zones : ni liminaire ni annexe n'entre dans le corps.
            Sorte::Liminaire | Sorte::Annexe => unreachable!("zone validée par decoupe"),
        }
    }

    // Les annexes rejoignent les liminaires hors du folio : il appartient au corps.
    if !annexes.is_empty() {
        if !apres_page {
            s.push_str("#pagebreak()\n");
        }
        s.push_str("#set page(footer: none)\n");
        for (i, p) in annexes.iter().enumerate() {
            if i > 0 {
                s.push_str("#pagebreak()\n");
            }
            s.push_str(&ouverture_piece(&p.titre));
            s.push_str(&blocs_typst(&p.blocs));
        }
    }

    // Page blanche de fin, sans folio — même dispositif que la blanche des liminaires.
    if r.blanche {
        s.push_str("\n#page(footer: none)[]\n");
    }
    s.push_str(&format!("\n{MARQUEUR}\n"));
    s
}

/// Source Typst de l'intérieur du livre, tel qu'il part à l'impression.
pub fn source(
    livre: &Livre,
    int: &Interieur,
    pr: &Provider,
    r: &Reglage,
    pieces: &[Piece],
    envoi: Option<Trace>,
) -> String {
    assemble(livre, int, pr, r, pieces, envoi, None)
}

/// L'intérieur du livre précédé de sa couverture, **sans imposition**.
///
/// La gouttière revient à la marge extérieure et la blanche de parité disparaît : ce
/// ne sont pas des réglages qu'on offre, c'est ce que veut dire « sans imposition ».
/// Les deux n'ont de sens qu'une fois le livre relié.
///
/// Aucun envoi : l'envoi autographe est une affaire de tirage papier, et il n'a pas de
/// dédicataire ici.
pub fn source_ebook(
    livre: &Livre,
    int: &Interieur,
    pr: &Provider,
    pieces: &[Piece],
    couverture: &str,
) -> String {
    let r = Reglage {
        gouttiere: pr.exterieur,
        blanche: false,
    };
    assemble(livre, int, pr, &r, pieces, None, Some(couverture))
}

/// Les pages liminaires : faux-titre, blanche, page de titre, copyright, et — quand le
/// livre en porte une — la dédicace et sa blanche, puis les pièces liminaires du
/// manuscrit.
///
/// Toutes sans folio, et sans avoir à le dire : `footer: none`, posé par l'entête que
/// `source` écrit, court jusqu'au `#set page(footer: …)` qui ouvre le corps.
fn liminaires(livre: &Livre, envoi: Option<Trace>, pieces: &[Piece]) -> String {
    let mut s = String::new();
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
"#,
        majuscules(&livre.titre),
        majuscules(&livre.auteur),
        majuscules(&livre.titre_page().replace('\n', "\u{1}")).replace('\u{1}', r" \ "),
        echappe(&livre.genre),
    ));

    // L'envoi se pose sur la page de titre, dans le blanc que son contenu laisse au
    // bas. `#place` ne consomme pas le flux : il lui est impossible de créer une page,
    // et c'est là-dessus que repose la promesse — la pagination, le dos et la planche
    // sont les mêmes pour tous les envois du livre.
    match envoi {
        Some(Trace::Texte { police, texte }) => s.push_str(&format!(
            r#"#place(bottom + center, dy: -28mm, block(width: 70%)[
  #set par(justify: false, first-line-indent: 0pt, leading: 0.9em)
  #text(font: "{police}", size: 14pt, hyphenate: false)[{}]
])
"#,
            // La main est validée en amont par `Envois::verifie` : pas d'échappement.
            echappe(texte).replace('\n', r" \ ")
        )),
        // Le nom du fichier est fabriqué par `envoi::nom_image` : assaini, il ne porte
        // ni guillemet qui refermerait la chaîne, ni séparateur qui la ferait sortir du
        // répertoire où l'image vient d'être écrite.
        //
        // La hauteur est bornée à 30 % du corps — le blanc que la page de titre laisse
        // au bas, sur tous les formats. La largeur reste maîtresse tant que la hauteur
        // tient : une image qui passait hier ne bouge pas d'un pixel ; seule celle qui
        // recouvrirait le titre est ramenée à la borne. Pas de `fit: "contain"` : le
        // cadre qu'il demande n'ancre pas son contenu en bas, l'image y flotterait.
        Some(Trace::Image { fichier }) => s.push_str(&format!(
            r#"#place(bottom + center, dy: -28mm, layout(zone => {{
  let plein = measure(image("{fichier}", width: zone.width * 70%))
  if plein.height > zone.height * 30% {{
    image("{fichier}", height: zone.height * 30%)
  }} else {{
    image("{fichier}", width: 70%)
  }}
}}))
"#
        )),
        None => {}
    }
    s.push_str("#pagebreak()\n\n");

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

    // La dédicace prend une belle page, son verso reste blanc — deux `#pagebreak()`
    // d'affilée, le dispositif de la blanche du faux-titre. Le corps s'ouvre donc en
    // page 7 au lieu de 5, et le dos en tient compte de lui-même puisqu'il découle de
    // la pagination mesurée, jamais d'une saisie.
    if let Some(d) = livre.dedicace() {
        s.push_str(&format!(
            r#"#v(48mm)
#align(right, emph(text(size: 9.5pt)[{}]))
#pagebreak()
#pagebreak()

"#,
            echappe(d).replace('\n', r" \ ")
        ));
    }

    // Les pièces liminaires du manuscrit ferment la série : `footer: none` court encore,
    // le folio ne sera rétabli qu'au premier chapitre.
    for p in pieces {
        s.push_str(&ouverture_piece(&p.titre));
        s.push_str(&blocs_typst(&p.blocs));
        s.push_str("#pagebreak()\n\n");
    }

    s
}

/// Majuscules typographiques : `upper()` de Typst plutôt qu'une bascule en Rust, pour
/// que la casse suive la langue du document (le CSS faisait `text-transform`).
fn majuscules(s: &str) -> String {
    format!("#upper[{}]", echappe(s))
}

/// L'ouverture d'une pièce à texte — préface, postface.
///
/// Le mot occupe la ligne du numéro, mais composé comme un **titre** de chapitre : ce
/// sont la casse et l'espacement qui font le titre, les 13 pt du gabarit étant la
/// taille d'un chiffre isolé. Le blanc de 14,5 mm est la somme des deux blancs du
/// gabarit (3,5 + 11) : le texte s'ouvre à la même hauteur que celui d'un chapitre.
fn ouverture_piece(titre: &str) -> String {
    format!(
        "#v(22mm)\n#align(center, text(size: 10pt, tracking: 0.14em)[{}])\n#v(14.5mm)\n",
        majuscules(titre)
    )
}

/// Les blocs d'une pièce, composés. Partagé par les chapitres et les pièces à texte :
/// une préface se lit dans la même page qu'un chapitre.
fn blocs_typst(blocs: &[Bloc]) -> String {
    let mut s = String::new();
    for b in blocs {
        match b {
            Bloc::Paragraphe(p) => {
                s.push_str(&inline(p));
                s.push_str("\n\n");
            }
            // Le blanc est en em, non en mm : il suit le corps du prestataire comme
            // l'interligne, là où l'épreuve, qui n'a qu'un format, se règle en mm.
            // Il s'ajoute à l'espace de paragraphe, de part et d'autre — une rupture
            // se voit d'un coup d'œil sur la page, sans la trouer.
            //
            // Le paragraphe qui suit garde son alinéa, comme après n'importe quel
            // blanc : relevé sur la page composée, pas déduit. La marque le rend
            // sans conséquence — c'est elle qui dit la coupure, pas le retrait.
            Bloc::Scene => s.push_str(&format!("#v(1em)\n#align(center)[{SCENE}]\n#v(1em)\n\n")),
        }
    }
    s
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
            dedicace: None,
            chapitres: None,
        }
    }

    fn chapitres() -> Vec<Piece> {
        vec![Piece {
            sorte: Sorte::Chapitre(1),
            titre: "Un".into(),
            blocs: vec![Bloc::Paragraphe("Texte.".into())],
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
        let s = source(&livre(), &Interieur::default(), pr, &r, &[], None);
        assert!(s.contains("width: 135mm, height: 215mm"));
        assert!(s.contains("inside: 20mm"), "gouttière absente");
        assert!(s.contains("outside: 15mm"));
        assert!(s.contains("costs: (orphan: 100%, widow: 100%)"), "veuves");
        assert!(s.trim_end().ends_with(MARQUEUR), "marqueur de pagination");
    }

    /// L'ebook est le livre **sans son imposition** : la gouttière revient à la marge
    /// extérieure, et il n'y a pas de blanche de parité. Les deux n'ont de sens qu'une fois
    /// le livre relié — à l'écran, l'une décale le texte une page sur deux et l'autre ajoute
    /// une page vide.
    #[test]
    fn l_ebook_compose_sans_gouttiere_ni_blanche_de_parite() {
        let pr = provider("lulu").unwrap();
        let s = source_ebook(
            &livre(),
            &Interieur::default(),
            pr,
            &chapitres(),
            "#page[couverture]\n",
        );
        assert!(
            s.contains(&format!("inside: {}mm", pr.exterieur)),
            "gouttière non ramenée à la marge extérieure : {s}"
        );
        assert!(
            !s.contains("#page(footer: none)[]"),
            "blanche de parité présente : {s}"
        );
    }

    /// La couverture est la **première** page : avant le faux-titre, donc avant tout ce que
    /// `liminaires` écrit.
    #[test]
    fn la_couverture_precede_les_liminaires() {
        let s = source_ebook(
            &livre(),
            &Interieur::default(),
            provider("lulu").unwrap(),
            &chapitres(),
            "#page[COUVERTURE]\n",
        );
        let couverture = s.find("COUVERTURE").expect("couverture absente");
        let faux_titre = s.find("#v(42mm)").expect("faux-titre absent");
        assert!(couverture < faux_titre, "{s}");
    }

    /// L'intérieur d'impression ne bouge pas : `source` reste ce qu'elle était, sans page
    /// insérée. C'est ce test qui dit que le refactor n'a pas fui dans le livre papier.
    #[test]
    fn l_interieur_d_impression_ne_porte_aucune_couverture() {
        let r = Reglage {
            gouttiere: 15.0,
            blanche: true,
        };
        let s = source(
            &livre(),
            &Interieur::default(),
            provider("lulu").unwrap(),
            &r,
            &chapitres(),
            None,
        );
        assert!(s.contains("inside: 15mm"), "{s}");
        assert!(s.contains("#page(footer: none)[]"), "{s}");
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
            None,
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
            None,
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
            None,
        );
        assert!(s.contains(r"Les Heures \ creuses"), "saut de ligne perdu");
        assert!(s.contains(r"Ivan \#Pjig"), "auteur non échappé");
    }

    /// Le titre et l'auteur n'arrivent pas qu'en markup : ils entrent aussi *dans une
    /// chaîne* Typst, celle de `#set document`, et dans la ligne de commentaire qui
    /// ouvre la source. Un guillemet droit y referme la chaîne — le compilateur répond
    /// `expected comma` — et un saut de ligne fait sortir du commentaire ce qui suit,
    /// qui s'imprime alors en tête du livre. L'échappement du markup ne protège ni de
    /// l'un ni de l'autre : il laisse passer le `"` et ne touche pas aux sauts de ligne.
    #[test]
    fn un_titre_a_guillemets_ne_referme_pas_la_chaine_du_document() {
        let pr = provider("lulu").unwrap();
        let mut l = livre();
        l.titre = "Le \"quai\"\nnord".into();
        l.auteur = "Ivan \"Pjig\"".into();
        let s = source(
            &l,
            &Interieur::default(),
            pr,
            &Reglage {
                gouttiere: 25.0,
                blanche: false,
            },
            &[],
            None,
        );
        let doc = s
            .lines()
            .find(|l| l.starts_with("#set document"))
            .expect("ligne #set document");
        assert_eq!(
            doc,
            r#"#set document(title: "Le \"quai\"\nnord", author: "Ivan \"Pjig\"")"#
        );
        let entete = s.lines().next().expect("ligne de commentaire");
        assert!(
            entete.starts_with("// Intérieur") && entete.contains(r"quai\"),
            "commentaire d'en-tête coupé par le titre : {entete}"
        );
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
        let s = source(&livre(), &int, pr, &r, &chapitres(), None);
        assert_eq!(s.matches("font:").count(), 1);
        assert!(s.contains(r#"font: "Cardo""#), "police du projet ignorée");
    }

    /// La rupture que l'auteur a écrite s'imprime. Elle a longtemps été perdue —
    /// deux scènes se composaient collées, en alinéas consécutifs — et le test qui
    /// figeait cette dette est celui-ci, retourné : ce qui était « à l'identique »
    /// exige désormais une différence, et la marque.
    ///
    /// La même que l'épreuve compose, pour que ce qu'on relit soit ce qui s'imprime.
    #[test]
    fn une_rupture_de_scene_s_imprime() {
        let pr = provider("lulu").unwrap();
        let r = Reglage {
            gouttiere: 25.0,
            blanche: false,
        };
        let int = Interieur::default();
        let sans = vec![Piece {
            sorte: Sorte::Chapitre(1),
            titre: "Un".into(),
            blocs: vec![
                Bloc::Paragraphe("Avant.".into()),
                Bloc::Paragraphe("Après.".into()),
            ],
        }];
        let avec = vec![Piece {
            sorte: Sorte::Chapitre(1),
            titre: "Un".into(),
            blocs: vec![
                Bloc::Paragraphe("Avant.".into()),
                Bloc::Scene,
                Bloc::Paragraphe("Après.".into()),
            ],
        }];
        let s = source(&livre(), &int, pr, &r, &avec, None);
        assert_ne!(
            source(&livre(), &int, pr, &r, &sans, None),
            s,
            "la rupture de scène est encore perdue"
        );
        assert!(s.contains(SCENE), "marque de rupture absente");
    }

    /// Le premier chapitre suit déjà le saut de page du copyright : un saut de plus
    /// laisserait une page blanche parasite, qui décalerait toute la pagination.
    #[test]
    fn le_premier_chapitre_n_ajoute_pas_de_saut_de_page() {
        let pr = provider("lulu").unwrap();
        let chs = vec![
            Piece {
                sorte: Sorte::Chapitre(1),
                titre: "Un".into(),
                blocs: vec![Bloc::Paragraphe("A.".into())],
            },
            Piece {
                sorte: Sorte::Chapitre(2),
                titre: "Deux".into(),
                blocs: vec![Bloc::Paragraphe("B.".into())],
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
            None,
        );
        let corps = s.split("#set page(footer: context").nth(1).unwrap();
        assert_eq!(
            corps.matches("#pagebreak()").count(),
            1,
            "un seul saut, entre les deux chapitres"
        );
    }

    /// Une dédicace renseignée coûte exactement deux pages : la belle page et sa
    /// blanche. Une seule, et le premier chapitre s'ouvrirait au verso ; trois, et le
    /// livre gagne un feuillet que personne n'a demandé — dans les deux cas le dos est
    /// faux, et il ne se découvre qu'après tirage.
    #[test]
    fn une_dedicace_ajoute_une_belle_page_et_sa_blanche() {
        let sans = liminaires(&livre(), None, &[]);
        let mut l = livre();
        l.dedicace = Some("À M., qui a tenu la lampe.".into());
        let avec = liminaires(&l, None, &[]);

        assert_eq!(
            avec.matches("#pagebreak()").count(),
            sans.matches("#pagebreak()").count() + 2,
            "la dédicace ne coûte pas deux pages"
        );
        assert!(
            avec.contains("#align(right, emph(text(size: 9.5pt)[À M., qui a tenu la lampe.]))"),
            "la dédicace n'est pas composée en petit italique à droite : {avec}"
        );
    }

    /// Absente, vide ou faite d'espaces : la même source, à l'octet près. C'est ce qui
    /// garantit qu'un livre déjà composé ne change pas de pagination — donc pas de dos —
    /// du seul fait que le champ existe désormais.
    #[test]
    fn une_dedicace_vide_ou_blanche_ne_compose_rien() {
        let sans = liminaires(&livre(), None, &[]);
        for creux in ["", "   ", "\n \n"] {
            let mut l = livre();
            l.dedicace = Some(creux.into());
            assert_eq!(
                liminaires(&l, None, &[]),
                sans,
                "« {creux:?} » a été pris pour une dédicace"
            );
        }
    }

    /// Les deux pièges déjà gardés pour le titre de page : le markup Typst doit être
    /// échappé, et les sauts de ligne voulus doivent survivre. Un `#` non échappé fait
    /// échouer la compilation du livre entier, plusieurs centaines de pages plus loin.
    #[test]
    fn une_dedicace_est_echappee_et_garde_ses_sauts_de_ligne() {
        let mut l = livre();
        l.dedicace = Some("À #M.,\nqui a tenu la lampe.".into());
        let s = liminaires(&l, None, &[]);

        assert!(s.contains(r"À \#M.,"), "dédicace non échappée : {s}");
        assert!(
            s.contains(r"\ qui a tenu la lampe."),
            "saut de ligne perdu : {s}"
        );
    }

    /// La préface est une pièce liminaire : elle se compose avant le rétablissement du
    /// folio, donc ses pages n'en portent pas — la règle validée au cadrage.
    #[test]
    fn une_preface_se_compose_avant_le_folio() {
        let mut pieces = vec![Piece {
            sorte: Sorte::Liminaire,
            titre: "Préface".into(),
            blocs: vec![Bloc::Paragraphe("Entrez.".into())],
        }];
        pieces.extend(chapitres());
        let s = source(
            &livre(),
            &Interieur::default(),
            provider("lulu").unwrap(),
            &Reglage {
                gouttiere: 25.0,
                blanche: false,
            },
            &pieces,
            None,
        );
        let preface = s.find("Préface").expect("la préface doit être composée");
        let folio = s
            .find("#set page(footer: context")
            .expect("le folio du corps");
        assert!(
            preface < folio,
            "la préface passe après le rétablissement du folio"
        );
        assert!(s.contains("Entrez."), "le texte de la préface est perdu");
    }

    /// Une page de partie prend une belle page au verso blanc, sans folio : deux
    /// `#page(footer: none)`. Et comme `#page` rompt le flux de lui-même, le chapitre
    /// qui suit ne doit pas ajouter un `#pagebreak()` — il laisserait une page blanche
    /// de plus, invisible à la lecture du code et payée au tirage.
    ///
    /// La comparaison porte sur un corps d'un seul chapitre : la partie **et** le
    /// chapitre qui la suit doivent, à eux deux, ne coûter aucun saut de plus.
    #[test]
    fn une_page_de_partie_prend_une_belle_page_sans_folio_et_sans_saut_en_trop() {
        let pieces = vec![
            Piece {
                sorte: Sorte::Chapitre(1),
                titre: "Un".into(),
                blocs: vec![Bloc::Paragraphe("Texte.".into())],
            },
            Piece {
                sorte: Sorte::Partie("I".into()),
                titre: "Avant Clément".into(),
                blocs: Vec::new(),
            },
            Piece {
                sorte: Sorte::Chapitre(2),
                titre: "Deux".into(),
                blocs: vec![Bloc::Paragraphe("Suite.".into())],
            },
        ];
        let pr = provider("lulu").unwrap();
        let r = Reglage {
            gouttiere: 25.0,
            blanche: false,
        };
        let avec = source(&livre(), &Interieur::default(), pr, &r, &pieces, None);
        let sans = source(&livre(), &Interieur::default(), pr, &r, &chapitres(), None);
        assert_eq!(
            avec.matches("#page(footer: none)").count(),
            sans.matches("#page(footer: none)").count() + 2,
            "la partie doit ajouter exactement deux pages sans folio"
        );
        assert_eq!(
            avec.matches("#pagebreak()").count(),
            sans.matches("#pagebreak()").count(),
            "le chapitre qui suit la partie ne doit pas ajouter de saut"
        );
        // La casse est laissée à Typst (`#upper`), pour qu'elle suive la langue du
        // document : c'est le titre passé à `majuscules` qu'on vérifie, pas son rendu.
        assert!(
            avec.contains("#upper[Avant Clément]"),
            "titre de partie absent : {avec}"
        );
    }

    /// Une ouverture de partie est une belle page — un recto, jamais un verso. La parité,
    /// au milieu du corps, dépend de la longueur du chapitre qui précède, donc d'un texte
    /// que l'auteur retouche : sans saut de parité, trois paragraphes ajoutés au chapitre
    /// d'avant font paraître la partie au verso et sa blanche au recto, le dispositif
    /// exactement à l'envers. Le compte de pages ne le dit pas — les deux cas coûtent deux
    /// pages — et cela ne se découvrirait qu'après tirage.
    #[test]
    fn une_page_de_partie_est_forcee_sur_une_belle_page() {
        let pieces = vec![
            Piece {
                sorte: Sorte::Chapitre(1),
                titre: "Un".into(),
                blocs: vec![Bloc::Paragraphe("Texte.".into())],
            },
            Piece {
                sorte: Sorte::Partie("I".into()),
                titre: "Avant Clément".into(),
                blocs: Vec::new(),
            },
        ];
        let s = source(
            &livre(),
            &Interieur::default(),
            provider("lulu").unwrap(),
            &Reglage {
                gouttiere: 25.0,
                blanche: false,
            },
            &pieces,
            None,
        );
        assert!(
            s.contains("if calc.odd(here().page()) { page(footer: none)[] }"),
            "la page de partie n'est pas calée sur un recto : {s}"
        );
        // La blanche du calage est posée sans folio, comme les deux pages de la partie :
        // une page vide numérotée au milieu du livre se remarque.
        assert!(
            !s.contains("#pagebreak(to: \"odd\")"),
            "le calage par saut de parité laisserait une blanche foliotée : {s}"
        );
    }

    /// Le folio appartient au corps : une postface n'en porte pas, comme la préface.
    #[test]
    fn une_annexe_se_compose_sans_folio() {
        let mut pieces = chapitres();
        pieces.push(Piece {
            sorte: Sorte::Annexe,
            titre: "Postface".into(),
            blocs: vec![Bloc::Paragraphe("Après coup.".into())],
        });
        let s = source(
            &livre(),
            &Interieur::default(),
            provider("lulu").unwrap(),
            &Reglage {
                gouttiere: 25.0,
                blanche: false,
            },
            &pieces,
            None,
        );
        let coupe = s
            .find("#set page(footer: none)")
            .expect("le folio doit être coupé");
        let postface = s.find("Postface").expect("la postface doit être composée");
        assert!(
            coupe < postface,
            "la postface se compose avant la coupure du folio"
        );
        assert!(s.contains("Après coup."));
    }

    fn trace() -> Trace<'static> {
        Trace::Texte {
            police: "Caveat",
            texte: "À Léa, qui a lu la première version.",
        }
    }

    /// L'envoi se pose par `#place`, qui ne consomme pas le flux : il lui est
    /// impossible de créer une page. Ce n'est pas une précaution, c'est la propriété
    /// sur laquelle repose toute la promesse — même pagination, même dos, même planche
    /// pour tous les envois. Si ce test tombe, tous les packages d'envoi sont faux.
    #[test]
    fn un_envoi_ne_cree_aucune_page() {
        let sans = liminaires(&livre(), None, &[]);
        let avec = liminaires(&livre(), Some(trace()), &[]);

        assert_eq!(
            avec.matches("#pagebreak()").count(),
            sans.matches("#pagebreak()").count(),
            "l'envoi a déplacé une page"
        );
        // Compter les `#place(` plutôt que d'en chercher un : le pavé de copyright en
        // pose déjà un, si bien qu'un `contains` serait vrai même sans envoi — un test
        // qui ne peut pas échouer.
        assert_eq!(
            avec.matches("#place(").count(),
            sans.matches("#place(").count() + 1,
            "l'envoi ne se pose pas par #place : {avec}"
        );
        // Et surtout : rien qui consomme le flux. Un `#v` de plus pousserait le
        // contenu vers le bas, ce que le compte de sauts de page ne verrait pas.
        assert_eq!(
            avec.matches("#v(").count(),
            sans.matches("#v(").count(),
            "l'envoi pousse le flux au lieu de se poser dessus : {avec}"
        );
    }

    /// Hors de la page de titre, la source ne bouge pas d'un octet. Un envoi qui
    /// modifierait le corps changerait la pagination sans qu'aucun compte ne le
    /// signale.
    #[test]
    fn un_envoi_ne_touche_que_la_page_de_titre() {
        let pr = provider("lulu").unwrap();
        let r = Reglage {
            gouttiere: 25.0,
            blanche: false,
        };
        let sans = source(&livre(), &Interieur::default(), pr, &r, &chapitres(), None);
        let avec = source(
            &livre(),
            &Interieur::default(),
            pr,
            &r,
            &chapitres(),
            Some(trace()),
        );

        let corps = |s: &str| {
            s.split("#set page(footer: context")
                .nth(1)
                .unwrap()
                .to_string()
        };
        assert_eq!(corps(&sans), corps(&avec), "le corps a changé");
    }

    /// La main choisie doit être celle qui compose : sans le `font:`, Typst écrirait
    /// l'envoi dans la police de labeur du livre, et le mot ne ressemblerait plus à un
    /// mot écrit à la main.
    #[test]
    fn l_envoi_est_compose_dans_la_main_du_livre() {
        let s = liminaires(&livre(), Some(trace()), &[]);
        assert!(s.contains(r#"font: "Caveat""#), "main absente : {s}");
    }

    /// Le document est justifié — c'est bon pour trois cents pages de roman, et faux
    /// pour un mot écrit à la main : aucune main n'aligne son bord droit. Sans ce
    /// `justify: false`, l'envoi sort en pavé, ce qui trahit l'écriture manuscrite au
    /// premier coup d'œil et ne se voit dans aucun compte.
    #[test]
    fn un_envoi_n_est_pas_justifie() {
        let s = liminaires(&livre(), Some(trace()), &[]);
        assert!(s.contains("justify: false"), "envoi justifié : {s}");
    }

    /// Le document césure — c'est bon pour un roman justifié, et faux pour un mot écrit
    /// à la main : personne ne coupe « dif-fèrent » en tournant la ligne. Relevé sur un
    /// envoi réellement composé, pas supposé.
    #[test]
    fn un_envoi_ne_cesure_pas() {
        let s = liminaires(&livre(), Some(trace()), &[]);
        assert!(s.contains("hyphenate: false"), "envoi césuré : {s}");
    }

    /// L'image se pose par le même `#place` que le texte, et pour la même raison : elle
    /// ne consomme pas le flux, donc elle ne peut pas créer de page. Une image écrite à
    /// la main est bien plus haute qu'un mot de deux lignes — si elle poussait quoi que
    /// ce soit, la pagination du livre entier suivrait, et le dos avec.
    #[test]
    fn une_image_d_envoi_ne_cree_aucune_page_non_plus() {
        let sans = liminaires(&livre(), None, &[]);
        let avec = liminaires(
            &livre(),
            Some(Trace::Image {
                fichier: "Léa.png"
            }),
            &[],
        );

        assert_eq!(
            avec.matches("#pagebreak()").count(),
            sans.matches("#pagebreak()").count(),
            "l'image a déplacé une page"
        );
        assert_eq!(
            avec.matches("#place(").count(),
            sans.matches("#place(").count() + 1,
            "l'image ne se pose pas par #place : {avec}"
        );
        assert_eq!(
            avec.matches("#v(").count(),
            sans.matches("#v(").count(),
            "l'image pousse le flux au lieu de se poser dessus : {avec}"
        );
        assert!(
            avec.contains(r#"image("Léa.png", width: 70%)"#),
            "l'image n'est pas posée : {avec}"
        );
    }

    /// Une image trop haute recouvrait le titre — vu à l'aperçu, comme prévu par la
    /// spec, et tranché ensuite : le blanc du bas fait 30 % du corps, l'image s'y
    /// borne. La largeur reste maîtresse tant que la hauteur tient — une image déjà
    /// acceptée ne bouge pas d'un pixel — et la hauteur ne prend la main que sur
    /// celles qui déborderaient.
    #[test]
    fn une_image_trop_haute_est_bornee_au_blanc_du_bas() {
        let s = liminaires(
            &livre(),
            Some(Trace::Image {
                fichier: "Léa.png"
            }),
            &[],
        );
        assert!(
            s.contains(r#"image("Léa.png", height: zone.height * 30%)"#),
            "aucune borne de hauteur : {s}"
        );
        assert!(
            s.contains(r#"if plein.height > zone.height * 30%"#),
            "la borne s'applique même quand la largeur suffit : {s}"
        );
    }

    /// Une image ne s'écrit pas dans une police : lui en imposer une reviendrait à
    /// composer du texte là où il n'y en a pas, et le mot manuscrit passerait au
    /// travers.
    #[test]
    fn une_image_d_envoi_n_emporte_aucune_police() {
        let s = liminaires(
            &livre(),
            Some(Trace::Image {
                fichier: "Léa.png"
            }),
            &[],
        );
        assert!(!s.contains("font:"), "une police s'est glissée : {s}");
    }

    /// Même piège que le titre de page et que la dédicace : le markup Typst doit être
    /// échappé, les sauts de ligne voulus doivent survivre.
    #[test]
    fn un_envoi_est_echappe_et_garde_ses_sauts_de_ligne() {
        let t = Trace::Texte {
            police: "Caveat",
            texte: "À #Léa,\navec mon amitié.",
        };
        let s = liminaires(&livre(), Some(t), &[]);

        assert!(s.contains(r"À \#Léa,"), "envoi non échappé : {s}");
        assert!(
            s.contains(r"\ avec mon amitié."),
            "saut de ligne perdu : {s}"
        );
    }
}
