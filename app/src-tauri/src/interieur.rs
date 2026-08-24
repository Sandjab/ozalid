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

/// Ce qu'un envoi dépose sur sa page.
///
/// `interieur` ne connaît ni la main du livre, ni d'où l'image vient : il reçoit ce
/// que l'envoi a décidé. Une image écrite à la main et une image produite par un
/// modèle de diffusion arrivent ici de la même façon — ce module n'a pas à savoir
/// laquelle, seulement qu'elle est posée à côté de la source.
#[derive(Debug, Clone, Copy)]
pub enum Quoi<'a> {
    /// Un texte, composé dans la main de cet envoi.
    Texte { police: &'a str, texte: &'a str },
    /// Une image, déjà écrite à côté de la source, désignée par son seul nom.
    Image { fichier: &'a str },
}

/// Un envoi et sa place sur la page.
#[derive(Debug, Clone, Copy)]
pub struct Trace<'a> {
    pub quoi: Quoi<'a>,
    pub place: &'a crate::envoi::Place,
}

/// Le rapport entre la largeur de l'objet et le corps de son écriture.
///
/// L'objet est self-similaire : l'agrandir agrandit les lettres, parce que tirer un
/// coin à la souris agrandit une signature — il n'élargit pas une colonne de texte
/// pour la laisser se recomposer. Le corps suit donc la taille.
///
/// La valeur cale le nouveau réglage sur l'ancien : jusqu'à la v4, l'envoi se composait
/// en 14 pt dans un bloc de 70 % de la justification. Sur une page de 127 mm, une
/// taille de 0,60 donne 76,2 mm de large, et 14 pt valent 4,94 mm — d'où 4,94 / 76,2.
const CORPS_SUR_LARGEUR: f64 = 0.0648;

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
  footer: none,{fg}
)
#set text(font: "{}", size: {}pt, lang: "fr", hyphenate: true,
          top-edge: 0.75em, bottom-edge: -0.25em,
          costs: (orphan: 100%, widow: 100%))
#set par(justify: true, leading: {lead}em, spacing: {lead}em, first-line-indent: 1.2em)

// Le blanc de respiration : `n` lignes sautées, sans marque. Faible au sens de Typst,
// donc supprimé à une frontière de page — le registre passe avant la coupure.
//
// La hauteur est exacte, pas approchée : `top-edge` et `bottom-edge` ci-dessus posent
// la ligne à 1em pile, l'avance d'une ligne à la suivante vaut donc 1em + leading. Le
// blanc doit en plus couvrir l'espacement de paragraphe qu'il remplace — Typst fusionne
// deux espacements faibles en gardant le plus grand —, d'où le terme supplémentaire.
// À n = 1 : 1em + 2·leading, la valeur relevée sur PDF le 22/08.
#let blanc(n) = v(n * 1em + (n + 1) * {lead}em, weak: true)

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
        fg = foreground(envoi, fw),
    ));

    // La page insérée vient avant tout ce que `liminaires` écrit : c'est la page 1 du
    // fichier, celle qu'un lecteur voit en ouvrant.
    if let Some(a) = avant {
        s.push_str(a);
    }

    s.push_str(&liminaires(livre, liminaires_manuscrit));

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
                s.push_str(&titre_sous_numero(&p.titre));
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
                s.push_str(&titre_sous_numero(&p.titre));
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

/// Ce que l'envoi ajoute à `#set page` : un `foreground` conditionné au numéro de page.
///
/// **`foreground` et non le flux.** Un `#place` dans le flux ne pouvait déjà pas créer
/// de page ; il fallait en revanche l'écrire là où la page visée se compose, ce qui
/// enfermait l'envoi sur la page de titre. Le `foreground`, lui, se pose une fois au
/// préambule et vise n'importe quelle page — un `#set page(…)` au milieu du document
/// ouvrirait une page, d'où le préambule et lui seul.
///
/// Il survit au `#set page(footer: …)` qui ouvre le corps, les `set` de Typst
/// fusionnant champ à champ, et aux `#page(…)[…]` des pages de partie. Ses pourcentages
/// se résolvent sur la **page entière, marges comprises** : c'est ce qui les met en
/// correspondance 1:1 avec le canevas de l'interface, qui montre la page entière.
///
/// `counter(page)` n'est jamais remis à zéro dans l'intérieur — seul son affichage est
/// masqué jusqu'au corps —, si bien que la condition porte bien sur la n-ième page du
/// fichier, celle que la vignette montre.
fn foreground(envoi: Option<Trace>, largeur_mm: f64) -> String {
    let Some(t) = envoi else {
        return String::new();
    };
    let p = t.place;
    let quoi = match t.quoi {
        Quoi::Texte { police, texte } => format!(
            r#"box(width: {taille}%)[
        #set par(justify: false, first-line-indent: 0pt, leading: 0.9em)
        #text(font: "{police}", size: {corps:.3}mm, hyphenate: false)[{mot}]
      ]"#,
            taille = p.taille * 100.0,
            // La main est validée en amont par `Envois::verifie` : pas d'échappement.
            mot = echappe(texte).replace('\n', r" \ "),
            corps = p.taille * largeur_mm * CORPS_SUR_LARGEUR,
        ),
        // Le nom du fichier est fabriqué par `envoi::nom_image` : assaini, il ne porte
        // ni guillemet qui refermerait la chaîne, ni séparateur qui la ferait sortir du
        // répertoire où l'image vient d'être écrite.
        //
        // Aucune borne de hauteur, contrairement à la v3 : elle protégeait d'un envoi
        // qui recouvrirait le titre, or le canevas montre désormais ce recouvrement, et
        // le brider corrigerait l'auteur d'une faute qu'il voit.
        Quoi::Image { fichier } => format!(r#"image("{fichier}", width: {}%)"#, p.taille * 100.0),
    };
    format!(
        r#"
  foreground: context {{
    if counter(page).get().first() == {page} {{
      place(center + horizon, dx: {dx}%, dy: {dy}%, rotate({angle}deg, {quoi}))
    }}
  }},"#,
        page = p.page,
        dx = (p.x - 0.5) * 100.0,
        dy = (p.y - 0.5) * 100.0,
        angle = p.angle,
    )
}

/// La source d'un envoi rendu **seul**, sur fond transparent, à hauteur automatique.
///
/// C'est ce que le canevas de placement manipule. Le rendre par Typst plutôt que de
/// l'imiter en CSS fait que ce qu'on déplace **est** ce qui s'imprimera — même police,
/// même corps, même coupure de lignes. La page en fond ne bouge pas, un `foreground` ne
/// réordonnant rien : glisser, redimensionner et incliner ne sont plus alors que des
/// `transform`, et Typst n'est rappelé que quand le mot ou la main changent.
///
/// `fill: none` donne le fond transparent, `height: auto` laisse la hauteur suivre le
/// texte. La largeur est celle que l'objet occupera sur la page : c'est elle qui décide
/// des coupures de lignes, et la rendre à une autre largeur donnerait un objet dont le
/// rapport ne serait pas celui du rendu.
pub fn source_objet(t: &Trace, largeur_mm: f64) -> String {
    let quoi = match t.quoi {
        Quoi::Texte { police, texte } => format!(
            r#"#set par(justify: false, first-line-indent: 0pt, leading: 0.9em)
#set text(font: "{police}", size: {corps:.3}mm, hyphenate: false, lang: "fr")
{mot}
"#,
            corps = largeur_mm * CORPS_SUR_LARGEUR,
            // La main est validée en amont par `Envois::verifie` : pas d'échappement.
            mot = echappe(texte).replace('\n', r" \ "),
        ),
        Quoi::Image { fichier } => format!("#image(\"{fichier}\", width: 100%)\n"),
    };
    format!("#set page(width: {largeur_mm}mm, height: auto, margin: 0pt, fill: none)\n{quoi}")
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
fn liminaires(livre: &Livre, pieces: &[Piece]) -> String {
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
        echappe(&livre.copyright()).replace('\n', r" \ ")
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
            echappe(&d).replace('\n', r" \ ")
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

/// Le titre sous le numéro d'une partie ou d'un chapitre — même casse, même espacement
/// que l'un ou l'autre, puisque c'est le même gabarit qui les compose. Absent si la
/// pièce n'a pas de titre : c'est le cas admis par le format (`## 7`, `## Partie I`).
fn titre_sous_numero(titre: &str) -> String {
    if titre.is_empty() {
        return String::new();
    }
    format!(
        "#v(3.5mm)\n#align(center, text(size: 10pt, tracking: 0.14em)[{}])\n",
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
            // Le blanc n'a pas de marque, donc rien à centrer : il est tout entier
            // dans l'espace. Sa hauteur est définie une fois au préambule, là où
            // l'interligne est connue — une ligne de texte laissée vide.
            Bloc::Blanc(n) => s.push_str(&format!("#blanc({n})\n\n")),
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::envoi::Place;
    use crate::providers::provider;
    use crate::typst::Typst;
    use std::cell::RefCell;
    use std::path::Path;

    fn livre() -> Livre {
        Livre {
            titre: "Les Heures creuses".into(),
            titre_page: "Les Heures\ncreuses".into(),
            auteur: "Ivan Pjig".into(),
            genre: "roman".into(),
            editeur: "Editeur".into(),
            collection: "Collection".into(),
            monogramme: "Monogramme".into(),
            copyright: "© Ivan Pjig, 2026.\nTous droits réservés.".into(),
            prix: "Prix".into(),
            mention: "Mention".into(),
            dedicace: String::new(),
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

    fn pieces_avec_blanc() -> Vec<Piece> {
        vec![Piece {
            sorte: Sorte::Chapitre(1),
            titre: "Un".into(),
            blocs: vec![
                Bloc::Paragraphe("Avant.".into()),
                Bloc::Blanc(1),
                Bloc::Paragraphe("Après.".into()),
            ],
        }]
    }

    /// Le blanc est un espace, pas un signe : la source ne doit porter aucune marque
    /// pour lui. C'est toute la différence avec la rupture de scène, et elle se vérifie
    /// ici plutôt qu'après tirage.
    #[test]
    fn le_blanc_de_respiration_ne_compose_aucune_marque() {
        let s = blocs_typst(&[
            Bloc::Paragraphe("Avant.".into()),
            Bloc::Blanc(1),
            Bloc::Paragraphe("Après.".into()),
        ]);
        assert!(s.contains("#blanc"), "{s}");
        assert!(!s.contains(SCENE), "{s}");
    }

    /// Le blanc est faible au sens de Typst : il disparaît à une frontière de page.
    /// C'est ce qui protège le registre — sans `weak`, la page suivante s'ouvrirait sur
    /// un trou et ses lignes ne seraient plus en regard de celles d'en face.
    ///
    /// Sa hauteur vaut `n` lignes, relevé sur PDF : Typst fusionne deux espacements
    /// faibles adjacents en gardant le plus grand, d'où le terme qui couvre l'espacement
    /// de paragraphe remplacé. Mesuré à 10 pt, `leading` et `spacing` à 0,65em, en
    /// lisant `here().position().y` après le blanc : n = 1 → 57 pt, n = 2 → 73,5 pt,
    /// n = 3 → 90 pt, soit 16,5 pt — une avance de ligne — par ligne demandée, et 90 pt
    /// aussi pour trois vraies lignes de texte à la place du blanc. À n = 1, la valeur
    /// est celle de l'ancienne formule `1em + lead * 2` : aucun manuscrit ne bouge.
    #[test]
    fn le_blanc_de_respiration_est_un_espace_faible() {
        let pr = provider("bod").unwrap();
        let r = Reglage {
            gouttiere: 20.0,
            blanche: false,
        };
        let s = source(
            &livre(),
            &Interieur::default(),
            pr,
            &r,
            &pieces_avec_blanc(),
            None,
        );
        assert!(s.contains("#let blanc(n) = v("), "{s}");
        assert!(s.contains("weak: true"), "{s}");
    }

    /// Un blanc de plusieurs lignes se compose en **un** espacement, jamais en plusieurs
    /// marques à la file : Typst fusionne deux espacements faibles adjacents en gardant
    /// le plus grand, et trois `#blanc(1)` n'auraient sauté qu'une ligne.
    #[test]
    fn un_blanc_de_trois_lignes_ne_compose_qu_un_espacement() {
        let s = blocs_typst(&[
            Bloc::Paragraphe("Avant.".into()),
            Bloc::Blanc(3),
            Bloc::Paragraphe("Après.".into()),
        ]);
        assert!(s.contains("#blanc(3)"), "{s}");
        assert_eq!(s.matches("#blanc(").count(), 1, "{s}");
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
        l.titre_page = "Les Heures\ncreuses".into();
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
        let sans = liminaires(&livre(), &[]);
        let mut l = livre();
        l.dedicace = "À M., qui a tenu la lampe.".into();
        let avec = liminaires(&l, &[]);

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
        let sans = liminaires(&livre(), &[]);
        for creux in ["", "   ", "\n \n"] {
            let mut l = livre();
            l.dedicace = creux.into();
            assert_eq!(
                liminaires(&l, &[]),
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
        l.dedicace = "À #M.,\nqui a tenu la lampe.".into();
        let s = liminaires(&l, &[]);

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

    /// La place d'un envoi ordinaire : le bas de la page de titre, là où les projets
    /// d'avant cette spec portaient le leur — le seul endroit qu'ils savaient viser.
    const PLACE: &Place = &Place {
        page: 3,
        x: 0.5,
        y: 0.80,
        taille: 0.60,
        angle: 0.0,
    };

    fn trace() -> Trace<'static> {
        Trace {
            quoi: Quoi::Texte {
                police: "Caveat",
                texte: "À Léa, qui a lu la première version.",
            },
            place: PLACE,
        }
    }

    fn image(fichier: &str) -> Trace<'_> {
        Trace {
            quoi: Quoi::Image { fichier },
            place: PLACE,
        }
    }

    /// La source d'un intérieur ordinaire, avec ou sans envoi : tout ce que ces tests
    /// comparent est ce que l'envoi y change.
    fn source_avec(envoi: Option<Trace>) -> String {
        let pr = provider("kdp-5x8").expect("gabarit kdp-5x8");
        let r = Reglage {
            gouttiere: pr.gouttieres[0].2,
            blanche: false,
        };
        source(&livre(), &Interieur::default(), pr, &r, &chapitres(), envoi)
    }

    /// Le corps composé de l'envoi, en millimètres, relevé dans la source.
    ///
    /// C'est le premier `size:` du fichier : le `foreground` se pose dans le `#set
    /// page` du préambule, donc avant le `#set text` du labeur.
    fn corps_de(s: &str) -> f64 {
        let i = s.find("size: ").expect("pas de corps d'envoi") + "size: ".len();
        let j = s[i..].find("mm").expect("corps non exprimé en mm") + i;
        s[i..j].parse().expect("corps illisible")
    }

    /// Le seul `foreground` de la source, isolé du reste.
    ///
    /// Les tests de contenu doivent s'y borner : la source entière porte déjà un
    /// `justify: false` — le pavé de copyright — et un `font:` — la police de labeur —,
    /// si bien qu'un `contains` sur elle serait vrai sans le moindre envoi. Un test qui
    /// ne peut pas échouer ne protège rien.
    fn foreground_de(s: &str) -> String {
        let debut = s.find("foreground:").expect("pas de foreground");
        let fin = s[debut..].find("\n)").expect("foreground non refermé") + debut;
        s[debut..fin].to_string()
    }

    /// L'envoi se pose en `foreground` de page, conditionné au numéro de page. C'est
    /// ce qui lui interdit de créer une page — donc de déplacer la pagination, le dos
    /// et la planche — **sur n'importe quelle page**, et non plus sur la seule page de
    /// titre. Si ce test tombe, tous les packages d'envoi sont faux.
    #[test]
    fn un_envoi_se_pose_en_foreground_conditionne_a_sa_page() {
        let p = Place { page: 37, ..*PLACE };
        let s = source_avec(Some(Trace {
            quoi: Quoi::Texte {
                police: "Caveat",
                texte: "À Léa,",
            },
            place: &p,
        }));
        assert!(s.contains("foreground:"), "pas de foreground : {s}");
        assert!(
            s.contains("counter(page).get().first() == 37"),
            "la page visée n'est pas dans la condition : {s}"
        );
        // Le flux ne doit rien recevoir : un `#pagebreak` de plus, et le compte bouge.
        let sans = source_avec(None);
        assert_eq!(
            s.matches("#pagebreak()").count(),
            sans.matches("#pagebreak()").count(),
            "l'envoi a ajouté une rupture de page"
        );
    }

    /// Le `foreground` se pose au préambule, une fois : un `#set page(…)` au milieu du
    /// document ouvrirait une page. Il doit donc paraître **avant** le premier contenu.
    #[test]
    fn le_foreground_est_au_preambule() {
        let s = source_avec(Some(trace()));
        let f = s.find("foreground:").expect("pas de foreground");
        let premier_contenu = s.find("#v(42mm)").expect("pas de faux-titre");
        assert!(
            f < premier_contenu,
            "le foreground est posé après le contenu : {s}"
        );
    }

    /// Hors du `foreground`, la source ne bouge pas d'un octet : c'est ce qui garantit
    /// que tous les exemplaires d'un tirage partagent la même pagination.
    #[test]
    fn un_envoi_ne_touche_que_le_foreground() {
        let avec = source_avec(Some(trace()));
        let sans = source_avec(None);
        let debut = avec.find("foreground:").expect("pas de foreground");
        let fin = avec[debut..].find("\n)").expect("foreground non refermé") + debut;
        let ampute = format!("{}{}", &avec[..debut], &avec[fin..]);
        assert_eq!(
            ampute.replace(char::is_whitespace, ""),
            sans.replace(char::is_whitespace, ""),
            "l'envoi a modifié la source hors du foreground"
        );
    }

    /// L'échelle grossit l'objet entier, lettres comprises : tirer un coin à la souris
    /// agrandit une signature, il n'élargit pas une colonne de texte pour la laisser se
    /// recomposer. Le corps suit donc la taille.
    #[test]
    fn l_echelle_emporte_le_corps() {
        let petit = Place {
            taille: 0.30,
            ..*PLACE
        };
        let grand = Place {
            taille: 0.60,
            ..*PLACE
        };
        let sp = corps_de(&source_avec(Some(Trace {
            quoi: Quoi::Texte {
                police: "Caveat",
                texte: "À Léa,",
            },
            place: &petit,
        })));
        let sg = corps_de(&source_avec(Some(Trace {
            quoi: Quoi::Texte {
                police: "Caveat",
                texte: "À Léa,",
            },
            place: &grand,
        })));
        assert!(
            sg > sp * 1.9 && sg < sp * 2.1,
            "le corps n'a pas doublé : {sp} → {sg}"
        );
    }

    /// L'inclinaison passe par `rotate`, dont l'origine est le centre — comme en CSS,
    /// sans quoi le canevas et Typst ne montreraient pas la même chose.
    #[test]
    fn l_inclinaison_passe_par_rotate() {
        let p = Place {
            angle: -4.0,
            ..*PLACE
        };
        let s = source_avec(Some(Trace {
            quoi: Quoi::Texte {
                police: "Caveat",
                texte: "À Léa,",
            },
            place: &p,
        }));
        assert!(s.contains("rotate(-4"), "pas de rotation : {s}");
    }

    /// La main choisie doit être celle qui compose : sans le `font:`, Typst écrirait
    /// l'envoi dans la police de labeur du livre, et le mot ne ressemblerait plus à un
    /// mot écrit à la main.
    #[test]
    fn l_envoi_est_compose_dans_sa_main() {
        let s = foreground_de(&source_avec(Some(trace())));
        assert!(s.contains(r#"font: "Caveat""#), "main absente : {s}");
    }

    /// Le document est justifié — c'est bon pour trois cents pages de roman, et faux
    /// pour un mot écrit à la main : aucune main n'aligne son bord droit. Sans ce
    /// `justify: false`, l'envoi sort en pavé, ce qui trahit l'écriture manuscrite au
    /// premier coup d'œil et ne se voit dans aucun compte.
    #[test]
    fn un_envoi_n_est_pas_justifie() {
        let s = foreground_de(&source_avec(Some(trace())));
        assert!(s.contains("justify: false"), "envoi justifié : {s}");
    }

    /// Le document césure — c'est bon pour un roman justifié, et faux pour un mot écrit
    /// à la main : personne ne coupe « dif-fèrent » en tournant la ligne. Relevé sur un
    /// envoi réellement composé, pas supposé.
    #[test]
    fn un_envoi_ne_cesure_pas() {
        let s = foreground_de(&source_avec(Some(trace())));
        assert!(s.contains("hyphenate: false"), "envoi césuré : {s}");
    }

    /// Une image ne s'écrit pas dans une police : lui en imposer une reviendrait à
    /// composer du texte là où il n'y en a pas, et le mot manuscrit passerait au
    /// travers.
    #[test]
    fn une_image_d_envoi_n_emporte_aucune_police() {
        let s = foreground_de(&source_avec(Some(image("Léa.png"))));
        assert!(!s.contains("font:"), "une police s'est glissée : {s}");
        assert!(
            s.contains(r#"image("Léa.png", width: 60%)"#),
            "l'image n'est pas posée à sa taille : {s}"
        );
    }

    /// Même piège que le titre de page et que la dédicace : le markup Typst doit être
    /// échappé, les sauts de ligne voulus doivent survivre.
    #[test]
    fn un_envoi_est_echappe_et_garde_ses_sauts_de_ligne() {
        let t = Trace {
            quoi: Quoi::Texte {
                police: "Caveat",
                texte: "À #Léa,\navec mon amitié.",
            },
            place: PLACE,
        };
        let s = foreground_de(&source_avec(Some(t)));

        assert!(s.contains(r"À \#Léa,"), "envoi non échappé : {s}");
        assert!(
            s.contains(r"\ avec mon amitié."),
            "saut de ligne perdu : {s}"
        );
    }

    /// **Point de sortie : le PDF de l'intérieur.** Aucun jeton ne doit survivre à la
    /// composition — un `%AUTEUR%` qui passe ici s'imprime dans le livre.
    ///
    /// Le test porte sur la source entière, et non sur le seul copyright : il doit
    /// casser le jour où un champ libre de plus est branché sans passer par son
    /// accesseur.
    #[test]
    fn aucun_jeton_ne_survit_a_la_source_de_l_interieur() {
        let mut l = livre();
        l.titre_page = "%TITRE%".into();
        l.copyright = "© %AUTEUR%, 2026.\nTous droits réservés.".into();
        l.dedicace = "Pour %AUTEUR%.".into();

        let pr = provider("bod").unwrap();
        let r = Reglage {
            gouttiere: 20.0,
            blanche: false,
        };
        let src = source(&l, &Interieur::default(), pr, &r, &chapitres(), None);

        for jeton in ["%TITRE%", "%AUTEUR%", "%GENRE%"] {
            assert!(!src.contains(jeton), "{jeton} a traversé la composition");
        }
        assert!(
            src.contains("Ivan Pjig"),
            "la valeur n'a pas remplacé le jeton"
        );
        assert!(src.contains("Les Heures creuses"));
    }

    /// L'objet rendu seul et l'envoi composé sur la page doivent employer **le même
    /// corps**.
    ///
    /// C'est toute la promesse du canevas : ce qu'on déplace à la souris est ce qui
    /// s'imprimera. Deux corps différents donneraient un objet dont les coupures de
    /// lignes, donc le rapport hauteur sur largeur, ne seraient pas ceux du rendu — et
    /// l'écart ne se verrait qu'après tirage.
    #[test]
    fn l_objet_du_canevas_et_l_envoi_compose_ont_le_meme_corps() {
        let pr = provider("kdp-5x8").expect("gabarit kdp-5x8");
        let place = Place {
            taille: 0.55,
            ..*PLACE
        };
        let t = Trace {
            quoi: Quoi::Texte {
                police: "Caveat",
                texte: "À Léa,",
            },
            place: &place,
        };
        let sur_la_page = corps_de(&source(
            &livre(),
            &Interieur::default(),
            pr,
            &Reglage {
                gouttiere: pr.gouttieres[0].2,
                blanche: false,
            },
            &chapitres(),
            Some(t),
        ));
        // La largeur passée à l'objet est celle qu'il occupera sur la page : c'est le
        // contrat que `envoi_objet` honore côté commandes.
        let seul = corps_de(&source_objet(&t, pr.format.0 * place.taille));
        assert_eq!(seul, sur_la_page, "le canevas ne montrera pas le rendu");
    }

    /* ---------- le témoin de l'invariant, composé pour de vrai ---------- */

    /// Un PNG minuscule mais valide : 2 × 2 pixels, deux gris.
    ///
    /// Fabriqué en dur plutôt que lu sur le disque : la variante image doit s'exercer
    /// sans dépendre d'un fichier du dépôt, et une image qu'on peut compter en octets
    /// ne cache rien.
    const PNG: &[u8] = &[
        0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x02, 0x08, 0x02, 0x00, 0x00, 0x00, 0xfd,
        0xd4, 0x9a, 0x73, 0x00, 0x00, 0x00, 0x11, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9c, 0x63, 0x60,
        0x60, 0x60, 0x68, 0x68, 0x68, 0x60, 0x80, 0x50, 0x00, 0x10, 0x8e, 0x03, 0x01, 0x6b, 0xa0,
        0x19, 0xc2, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
    ];

    /// Un manuscrit assez long pour que « page 37 » veuille dire quelque chose.
    ///
    /// `chapitres()` fait six pages : y placer un envoi ne dirait rien du cas qui compte,
    /// celui d'une page du corps, loin des liminaires où l'ancien `#place` savait déjà
    /// vivre. Quarante chapitres d'une page chacun donnent de quoi viser au milieu.
    fn manuscrit_long() -> Vec<Piece> {
        (1..=40)
            .map(|n| Piece {
                sorte: Sorte::Chapitre(n),
                titre: format!("Chapitre {n}"),
                blocs: (0..6)
                    .map(|_| {
                        Bloc::Paragraphe(
                            "Le vent tournait dans la cour, et les heures avec lui. \
                             On attendait sans savoir quoi, comme on attend toujours."
                                .into(),
                        )
                    })
                    .collect(),
            })
            .collect()
    }

    /// Le sidecar Typst et ses polices, tels que les exemples les montent.
    fn typst_de_test() -> Typst {
        Typst::new("typst").avec_polices(Path::new(env!("CARGO_MANIFEST_DIR")).join("fonts"))
    }

    /// Compose et rend le nombre de pages.
    fn pages_de(typst: &Typst, dossier: &Path, nom: &str, s: &str) -> u32 {
        std::fs::write(dossier.join(format!("{nom}.typ")), s).expect("source non écrite");
        typst
            .pages(&dossier.join(format!("{nom}.typ")))
            .expect("pagination refusée")
    }

    /// Une page rendue en PNG, telle qu'on la verrait.
    fn page_rendue(typst: &Typst, dossier: &Path, nom: &str, page: u32) -> Vec<u8> {
        let png = dossier.join(format!("{nom}-{page}.png"));
        typst
            .apercu(&dossier.join(format!("{nom}.typ")), &png, page, 40)
            .expect("rendu refusé");
        std::fs::read(&png).expect("rendu illisible")
    }

    /// **L'invariant qui tient toute la chaîne**, vérifié en composant pour de vrai.
    ///
    /// Compter les `#place` ou les `#pagebreak` dans la source ne prouve rien : c'est
    /// Typst qui décide du nombre de pages, et lui seul. Si cet invariant tombe, la
    /// pagination change, donc le dos, donc la planche — et les exemplaires partent à
    /// l'impression avec une couverture fausse, sans que rien ne le signale.
    ///
    /// Quatre pages visées, choisies pour ce qu'elles ont de différent : la première,
    /// la page de titre où l'ancien `#place` savait déjà vivre, une page du corps, et
    /// la dernière. Plus la variante image, dont la largeur en pourcentage se résout
    /// dans un `place` imbriqué dans un `foreground` — un chemin que le texte
    /// n'exerce pas.
    #[test]
    #[ignore = "lance le sidecar Typst : cargo test -- --ignored"]
    fn un_envoi_ne_cree_aucune_page_ou_qu_il_se_pose() {
        let typst = typst_de_test();
        let dossier = tempfile::tempdir().expect("répertoire de travail");
        let pr = provider("kdp-5x8").expect("gabarit kdp-5x8");
        let r = Reglage {
            gouttiere: pr.gouttieres[0].2,
            blanche: false,
        };
        let livre = livre();
        let int = Interieur::default();
        let pieces = manuscrit_long();
        let sans = pages_de(
            &typst,
            dossier.path(),
            "sans",
            &source(&livre, &int, pr, &r, &pieces, None),
        );
        assert!(
            sans > 30,
            "le manuscrit de ce test est trop court pour viser une page du corps : {sans}"
        );

        std::fs::write(dossier.path().join("mot.png"), PNG).expect("image non écrite");

        for page in [1, 3, sans / 2, sans] {
            let place = Place {
                page,
                x: 0.42,
                y: 0.73,
                taille: 0.55,
                angle: -4.0,
            };
            for (nom, quoi) in [
                (
                    "texte",
                    Quoi::Texte {
                        police: "Caveat",
                        texte: "À Léa,\nces heures creuses.",
                    },
                ),
                ("image", Quoi::Image { fichier: "mot.png" }),
            ] {
                let s = source(
                    &livre,
                    &int,
                    pr,
                    &r,
                    &pieces,
                    Some(Trace {
                        quoi,
                        place: &place,
                    }),
                );
                let cle = format!("{nom}-{page}");
                assert_eq!(
                    pages_de(&typst, dossier.path(), &cle, &s),
                    sans,
                    "un envoi en {nom} posé page {page} a déplacé la pagination"
                );
                // Le compte de pages seul ne prouverait rien : il serait tout aussi
                // identique si l'envoi ne s'imprimait nulle part. La page visée doit
                // donc différer de la même page sans envoi — et elle seule.
                assert_ne!(
                    page_rendue(&typst, dossier.path(), &cle, page),
                    page_rendue(&typst, dossier.path(), "sans", page),
                    "un envoi en {nom} visant la page {page} ne s'y voit pas"
                );
                let ailleurs = if page == 1 { 2 } else { 1 };
                assert_eq!(
                    page_rendue(&typst, dossier.path(), &cle, ailleurs),
                    page_rendue(&typst, dossier.path(), "sans", ailleurs),
                    "un envoi visant la page {page} a débordé sur la {ailleurs}"
                );
            }
        }
    }
}
