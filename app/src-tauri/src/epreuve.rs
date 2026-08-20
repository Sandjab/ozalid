//! L'épreuve de relecture : le manuscrit sur A4, pour être annoté.
//!
//! Ce n'est **pas** une simulation du livre imprimé, et elle ne le prétend pas : A4
//! recto, fer à gauche, large marge à droite, numéros de ligne. C'est un document de
//! travail sur le texte. C'est aussi ce qui l'autorise à composer les ruptures de
//! scène que l'intérieur perd encore.
//!
//! Aucun `Provider` n'entre ici, et aucune convergence : une épreuve ne va chez
//! personne, et son compte de pages n'intéresse personne.

use crate::interieur::Interieur;
use crate::manuscrit::{echappe, inline, Bloc, Chapitre};
use crate::projet::Livre;

/// Marque de rupture de scène : trois astérisques espacées.
///
/// Un blanc seul ne survit pas à une fin de page, il faut donc un signe visible. Mais
/// ce signe doit exister dans les **sept** polices de `POLICES_TEXTE`, sinon Typst le
/// compose par repli sur une autre police, sans un mot — le mécanisme même contre
/// lequel `Interieur::verifie` a été posé, et qui ne se verrait qu'après tirage.
///
/// Relevé sur les 29 fichiers de `fonts/` : `✳` (U+2733) et l'astérisme `⁂` (U+2042)
/// ne sont portés que par Cardo ; l'astérisque `*` (U+002A) est dans les 29. La marque
/// suit donc le caractère du livre au lieu de le trahir, et le jour où Cardo quitterait
/// `polices.sh` rien ne bougera.
///
/// Les `\*` sont échappés : en markup Typst, `*` ouvre une emphase.
pub const SCENE: &str = r"\*#h(0.8em)\*#h(0.8em)\*";

/// Format de la page, en mm. La marge de droite est celle où l'on écrit.
const MARGE_HAUT: f64 = 25.0;
const MARGE_BAS: f64 = 25.0;
const MARGE_GAUCHE: f64 = 30.0;
const MARGE_DROITE: f64 = 50.0;

/// Source Typst complète de l'épreuve.
pub fn source(livre: &Livre, int: &Interieur, chapitres: &[Chapitre], corps_pt: f64) -> String {
    let titre = echappe(&livre.titre);
    let auteur = echappe(&livre.auteur);
    // Interpolée brute, comme dans `interieur::source` : l'appelant doit l'avoir
    // validée par `Interieur::verifie`, qui seul connaît les polices admises.
    let police = &int.police;
    // Les mots du texte seul : ni les titres de chapitres, ni les `---` du manuscrit.
    // `commands.rs:540` compte, lui, `projet.texte.split_whitespace()` sur le Markdown
    // brut, et annonce donc toujours un peu plus. La divergence est assumée et va
    // toujours dans ce sens : le compte de la garde est celui qu'un auteur appelle des
    // mots. Les deux chiffres se voient — ne pas « corriger » l'un vers l'autre.
    let mots: usize = chapitres
        .iter()
        .flat_map(|c| &c.blocs)
        .filter_map(|b| match b {
            Bloc::Paragraphe(p) => Some(p.split_whitespace().count()),
            Bloc::Scene => None,
        })
        .sum();

    let mut s = format!(
        r##"// Épreuve de relecture — {titre}
#set document(title: "{titre}", author: "{auteur}")
#set page(
  width: 210mm, height: 297mm,
  margin: (top: {MARGE_HAUT}mm, bottom: {MARGE_BAS}mm,
           left: {MARGE_GAUCHE}mm, right: {MARGE_DROITE}mm),
  header: context {{
    let n = counter(page).get().first()
    if n <= 1 {{ return }}
    // Filtrer sur le numéro de page, et surtout pas sur `.before(here())`, qui est
    // pourtant la forme idiomatique : l'en-tête est évalué avant que le titre ouvrant
    // la page ne soit posé, si bien que `before(here())` priverait de rappel toute
    // page d'ouverture de chapitre — une page sur deux ou trois en chapitres courts.
    let ch = query(heading.where(level: 1)).filter(h => h.location().page() <= n)
    set text(size: 8.5pt, fill: rgb("#808080"))
    grid(columns: (1fr, auto),
      [{titre} — {auteur}],
      align(right)[#if ch.len() > 0 {{ ch.last().body }}])
  }},
  footer: context {{
    let n = counter(page).get().first()
    if n <= 1 {{ return }}
    set text(size: 8.5pt, fill: rgb("#808080"))
    align(center)[p. #n / #counter(page).final().first()]
  }},
)
#set text(font: "{police}", size: {corps_pt}pt, lang: "fr", hyphenate: false,
          top-edge: 0.75em, bottom-edge: -0.25em)
#set par(justify: false, leading: 0.5em, spacing: 0.5em, first-line-indent: 1.2em)
#show heading.where(level: 1): it => block(width: 100%, above: 0mm, below: 11mm)[
  #set text(size: {corps_pt}pt, weight: 400, tracking: 0.1em)
  #upper(it.body)
]

// Typst ne localise pas les noms de mois : « [month repr:long] » sort en anglais.
#let MOIS = ("janvier", "février", "mars", "avril", "mai", "juin",
             "juillet", "août", "septembre", "octobre", "novembre", "décembre")
#let aujourdhui = {{
  let d = datetime.today()
  [#d.day() #MOIS.at(d.month() - 1) #d.year()]
}}

// — Garde : ni folio ni numéros de ligne —
#[
  #set par.line(numbering: none)
  #v(45mm)
  #align(center)[
    #text(size: 18pt, tracking: 0.04em)[{titre}]
    #v(5mm)
    #text(size: 12pt, tracking: 0.1em)[#upper[{auteur}]]
    #v(2mm)
    #emph[{genre}]
  ]
  #place(bottom + center, block(width: 120mm)[
    #set par(justify: false, first-line-indent: 0pt, leading: 0.5em)
    #set text(size: 9.5pt, fill: rgb("#555555"))
    #align(center)[
      Épreuve de relecture — #aujourdhui
      #v(1.5mm)
      {nb_chapitres} chapitres, {mots} mots
      #v(4mm)
      #emph[Les numéros de ligne se rapportent à ce tirage : une nouvelle épreuve les renumérote.]
    ]
  ])
]
#pagebreak()

#set par.line(numbering: n => text(size: 7pt, fill: rgb("#a0a0a0"))[#n],
              number-clearance: 7mm, numbering-scope: "page")
"##,
        genre = echappe(&livre.genre),
        nb_chapitres = chapitres.len(),
    );

    for (i, ch) in chapitres.iter().enumerate() {
        // Le premier chapitre suit le saut de page de la garde.
        if i > 0 {
            s.push_str("\n#pagebreak()\n");
        }
        let titre_ch = if ch.titre.is_empty() {
            ch.numero.to_string()
        } else {
            format!("{} — {}", ch.numero, echappe(&ch.titre))
        };
        s.push_str(&format!("= {titre_ch}\n"));
        for b in &ch.blocs {
            match b {
                Bloc::Paragraphe(p) => {
                    s.push_str(&inline(p));
                    s.push_str("\n\n");
                }
                Bloc::Scene => s.push_str(&format!(
                    "#v(5mm)\n#align(center)[#text(fill: rgb(\"#808080\"))[{SCENE}]]\n#v(5mm)\n\n"
                )),
            }
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manuscrit::Bloc;

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

    fn chapitres() -> Vec<Chapitre> {
        vec![
            Chapitre {
                numero: 12,
                titre: "Le quai".into(),
                blocs: vec![
                    Bloc::Paragraphe("Avant.".into()),
                    Bloc::Scene,
                    Bloc::Paragraphe("Après.".into()),
                ],
            },
            Chapitre {
                numero: 13,
                titre: "Ce qu'on garde".into(),
                blocs: vec![Bloc::Paragraphe("Suite.".into())],
            },
        ]
    }

    fn src() -> String {
        source(&livre(), &Interieur::default(), &chapitres(), 12.0)
    }

    /// « p. 42, l. 7 » ne désigne une ligne que si le compte repart à chaque page. Une
    /// numérotation continue sur trois cents pages ne sert à rien.
    #[test]
    fn les_numeros_de_ligne_repartent_a_chaque_page() {
        assert!(
            src().contains(r#"numbering-scope: "page""#),
            "numérotation de ligne non remise à zéro par page"
        );
    }

    /// Chaque chapitre s'ouvre sur une page neuve : c'est ce qui rend l'épreuve
    /// navigable, et ce qui permet de ne réimprimer qu'un chapitre corrigé.
    #[test]
    fn chaque_chapitre_s_ouvre_sur_une_page_neuve() {
        // Deux chapitres, un seul saut : le premier suit la garde, qui saute déjà.
        assert_eq!(src().matches("#pagebreak()").count(), 2);
    }

    /// Une ligne d'épreuve doit tenir au texte, pas à la mise en page. Justifier
    /// masquerait les espaces doublées ; couper les mots ferait annoter des césures qui
    /// n'existent pas dans le livre.
    #[test]
    fn le_texte_n_est_ni_justifie_ni_coupe() {
        let s = src();
        assert!(s.contains("justify: false"), "épreuve justifiée");
        assert!(s.contains("hyphenate: false"), "épreuve coupée");
    }

    /// La marge de droite **est** la raison d'être du document : c'est là qu'on écrit.
    /// La ramener à une marge de livre donnerait une épreuve inannotable, sans qu'un
    /// seul des autres tests ne s'en aperçoive — ils ne regardent que le texte.
    #[test]
    fn la_page_est_un_a4_portrait_avec_sa_marge_d_annotation() {
        let s = src();
        assert!(
            s.contains("width: 210mm, height: 297mm"),
            "l'épreuve n'est plus un A4 portrait"
        );
        assert!(
            s.contains("right: 50mm"),
            "marge d'annotation absente ou rognée : plus de place pour écrire"
        );
        for m in ["top: 25mm", "bottom: 25mm", "left: 30mm"] {
            assert!(s.contains(m), "marge « {m} » perdue");
        }
    }

    /// Une épreuve annotée circule en pages détachées, et souvent une seule revient.
    /// Sans rappel du livre et du chapitre en tête, ni « p. n / total » en pied, cette
    /// page-là n'est plus rattachable à rien. La garde seule en est exemptée.
    #[test]
    fn chaque_page_de_texte_porte_son_en_tete_et_son_pied() {
        let s = src();
        assert!(s.contains("header: context"), "épreuve sans en-tête");
        assert!(s.contains("footer: context"), "épreuve sans pied");
        assert!(
            s.contains("Les Heures creuses — Ivan Pjig"),
            "en-tête sans rappel du livre"
        );
        assert!(
            s.contains("query(heading.where(level: 1))"),
            "en-tête sans rappel de chapitre"
        );
        assert!(
            s.contains("p. #n / #counter(page).final().first()"),
            "pied sans folio rapporté au total"
        );
        // Une fois dans l'en-tête, une fois dans le pied : la garde n'a ni l'un ni
        // l'autre, et c'est ce qui la distingue d'une page de texte.
        assert_eq!(
            s.matches("if n <= 1 { return }").count(),
            2,
            "la garde n'est plus exemptée d'en-tête et de pied"
        );
    }

    /// La rupture de scène paraît — c'est toute la différence avec l'intérieur.
    #[test]
    fn une_rupture_de_scene_parait_sur_l_epreuve() {
        assert!(src().contains(SCENE), "rupture de scène perdue");
    }

    /// Une épreuve annotée sans date n'est pas exploitable : on ne sait plus de quel
    /// tirage les numéros de ligne parlent.
    #[test]
    fn la_garde_porte_la_date_et_le_compte_de_chapitres() {
        let s = src();
        assert!(s.contains("datetime.today()"), "garde sans date");
        assert!(s.contains("2 chapitres"), "garde sans compte de chapitres");
        assert!(
            s.contains("renumérote"),
            "garde sans avertissement sur les numéros de ligne"
        );
    }

    /// L'épreuve prend la police du projet : deux compositions du même livre ne doivent
    /// pas diverger sans qu'on l'ait voulu.
    #[test]
    fn l_epreuve_prend_la_police_du_projet() {
        let s = source(
            &livre(),
            &Interieur {
                police: "Alegreya".into(),
            },
            &chapitres(),
            12.0,
        );
        assert!(s.contains(r#"font: "Alegreya""#));
    }

    /// Le corps est le seul réglage propre à l'épreuve.
    #[test]
    fn le_corps_est_reglable() {
        let s = source(&livre(), &Interieur::default(), &chapitres(), 14.0);
        assert!(s.contains("size: 14pt"), "corps ignoré");
    }
}
