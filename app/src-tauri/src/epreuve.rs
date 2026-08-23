//! L'épreuve de relecture : le manuscrit sur A4, pour être annoté.
//!
//! Ce n'est **pas** une simulation du livre imprimé, et elle ne le prétend pas : A4
//! recto, fer à gauche, large marge à droite, numéros de ligne. C'est un document de
//! travail sur le texte.
//!
//! Les ruptures de scène s'y composent avec la même marque que dans le livre —
//! `manuscrit::SCENE` — pour que ce qu'on relit soit ce qui s'imprime ; seul l'espace
//! autour diffère, réglé ici en mm puisque l'épreuve n'a qu'un format.
//!
//! Aucun `Provider` n'entre ici, et aucune convergence : une épreuve ne va chez
//! personne, et son compte de pages n'intéresse personne.

use crate::interieur::Interieur;
use crate::manuscrit::{echappe, echappe_chaine, inline, Bloc, Piece, Sorte, SCENE};
use crate::projet::Livre;

/// Format de la page, en mm. La marge de droite est celle où l'on écrit.
const MARGE_HAUT: f64 = 25.0;
const MARGE_BAS: f64 = 25.0;
const MARGE_GAUCHE: f64 = 30.0;
const MARGE_DROITE: f64 = 50.0;

/// Source Typst complète de l'épreuve.
pub fn source(livre: &Livre, int: &Interieur, pieces: &[Piece], corps_pt: f64) -> String {
    let titre = echappe(&livre.titre);
    let auteur = echappe(&livre.auteur);
    // Les mêmes, cités et non composés : la ligne de commentaire qui ouvre la source et
    // la chaîne de `#set document` ne sont pas du markup, et l'échappement du markup n'y
    // protège de rien.
    let titre_cite = echappe_chaine(&livre.titre);
    let auteur_cite = echappe_chaine(&livre.auteur);
    // Interpolée brute, comme dans `interieur::source` : l'appelant doit l'avoir
    // validée par `Interieur::verifie`, qui seul connaît les polices admises.
    let police = &int.police;
    // Les mots du texte seul : ni les titres de chapitres, ni les `---` du manuscrit.
    // `commands.rs:540` compte, lui, `projet.texte.split_whitespace()` sur le Markdown
    // brut, et annonce donc toujours un peu plus. La divergence est assumée et va
    // toujours dans ce sens : le compte de la garde est celui qu'un auteur appelle des
    // mots. Les deux chiffres se voient — ne pas « corriger » l'un vers l'autre.
    let mots: usize = pieces
        .iter()
        .flat_map(|c| &c.blocs)
        .filter_map(|b| match b {
            Bloc::Paragraphe(p) => Some(p.split_whitespace().count()),
            Bloc::Scene | Bloc::Blanc => None,
        })
        .sum();

    let mut s = format!(
        r##"// Épreuve de relecture — {titre_cite}
#set document(title: "{titre_cite}", author: "{auteur_cite}")
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
        nb_chapitres = pieces.iter().filter(|p| p.est_chapitre()).count(),
    );

    for (i, p) in pieces.iter().enumerate() {
        // La première pièce suit le saut de page de la garde.
        if i > 0 {
            s.push_str("\n#pagebreak()\n");
        }
        let titre_ch = match &p.sorte {
            Sorte::Chapitre(n) if p.titre.is_empty() => n.to_string(),
            Sorte::Chapitre(n) => format!("{n} — {}", echappe(&p.titre)),
            Sorte::Partie(r) if p.titre.is_empty() => format!("Partie {r}"),
            Sorte::Partie(r) => format!("Partie {r} — {}", echappe(&p.titre)),
            // Le titre d'un liminaire ou d'une annexe est son mot-clé : il vient de la
            // liste, pas du manuscrit, mais rien n'oblige à le croire sur parole.
            Sorte::Liminaire | Sorte::Annexe => echappe(&p.titre),
        };
        s.push_str(&format!("= {titre_ch}\n"));
        for b in &p.blocs {
            match b {
                Bloc::Paragraphe(p) => {
                    s.push_str(&inline(p));
                    s.push_str("\n\n");
                }
                Bloc::Scene => s.push_str(&format!(
                    "#v(5mm)\n#align(center)[#text(fill: rgb(\"#808080\"))[{SCENE}]]\n#v(5mm)\n\n"
                )),
                // Le livre laisse ce blanc muet ; l'épreuve, non. Elle numérote les
                // lignes et compose déjà l'astérisme en gris de service : un filet de
                // la même famille, plus clair, dit la coupure au relecteur sans rien
                // promettre de la page imprimée.
                Bloc::Blanc => s.push_str(
                    "#v(3mm)\n#align(center)[#line(length: 12mm, \
                     stroke: 0.4pt + rgb(\"#c0c0c0\"))]\n#v(3mm)\n\n",
                ),
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
            titre_page: crate::projet::titre_page_defaut(),
            auteur: "Ivan Pjig".into(),
            genre: "roman".into(),
            copyright: String::new(),
            dedicace: None,
            chapitres: None,
        }
    }

    fn chapitres() -> Vec<Piece> {
        vec![
            Piece {
                sorte: Sorte::Chapitre(12),
                titre: "Le quai".into(),
                blocs: vec![
                    Bloc::Paragraphe("Avant.".into()),
                    Bloc::Scene,
                    Bloc::Paragraphe("Après.".into()),
                ],
            },
            Piece {
                sorte: Sorte::Chapitre(13),
                titre: "Ce qu'on garde".into(),
                blocs: vec![Bloc::Paragraphe("Suite.".into())],
            },
        ]
    }

    fn src() -> String {
        source(&livre(), &Interieur::default(), &chapitres(), 12.0)
    }

    fn pieces_avec_blanc() -> Vec<Piece> {
        vec![Piece {
            sorte: Sorte::Chapitre(1),
            titre: "Un".into(),
            blocs: vec![
                Bloc::Paragraphe("Avant.".into()),
                Bloc::Blanc,
                Bloc::Paragraphe("Après.".into()),
            ],
        }]
    }

    /// L'épreuve est un document de travail, pas le livre : une coupure muette y serait
    /// invisible, et le relecteur ne pourrait pas vérifier qu'elle a bien été saisie.
    /// Le filet la lui montre, dans le gris de service qui ne s'imprime jamais — plus
    /// clair que celui de l'astérisme, parce que la coupure est la plus légère des deux.
    #[test]
    fn le_blanc_de_respiration_porte_un_filet_sur_l_epreuve() {
        let s = source(&livre(), &Interieur::default(), &pieces_avec_blanc(), 12.0);
        assert!(s.contains("#line(length: 12mm"), "{s}");
        assert!(s.contains("#c0c0c0"), "{s}");
        // La marque de la rupture de scène n'a rien à faire là : c'est l'autre coupure.
        assert!(!s.contains(SCENE), "{s}");
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

    /// Le même défaut qu'à l'intérieur, et pour la même raison : le titre entre ici
    /// aussi dans la chaîne de `#set document` et dans la ligne de commentaire qui ouvre
    /// la source. Un guillemet droit referme la chaîne, un saut de ligne fait sortir du
    /// commentaire ce qui suit — et l'épreuve est justement ce qu'on tire d'un livre
    /// dont le titre n'est pas encore arrêté.
    #[test]
    fn un_titre_a_guillemets_ne_referme_pas_la_chaine_du_document() {
        let mut l = livre();
        l.titre = "Le \"quai\"\nnord".into();
        let s = source(&l, &Interieur::default(), &chapitres(), 12.0);
        let doc = s
            .lines()
            .find(|l| l.starts_with("#set document"))
            .expect("ligne #set document");
        assert_eq!(
            doc,
            r#"#set document(title: "Le \"quai\"\nnord", author: "Ivan Pjig")"#
        );
        let entete = s.lines().next().expect("ligne de commentaire");
        assert!(
            entete.starts_with("// Épreuve") && entete.contains(r"quai\"),
            "commentaire d'en-tête coupé par le titre : {entete}"
        );
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

    /// Une pièce se relit comme le reste — mais son bandeau ne peut pas porter un
    /// numéro de chapitre qu'elle n'a pas.
    #[test]
    fn le_bandeau_d_une_piece_ne_porte_pas_de_numero() {
        let pieces = vec![
            Piece {
                sorte: Sorte::Liminaire,
                titre: "Préface".into(),
                blocs: vec![Bloc::Paragraphe("Entrez.".into())],
            },
            Piece {
                sorte: Sorte::Partie("III".into()),
                titre: "Avant Clément".into(),
                blocs: Vec::new(),
            },
        ];
        let s = source(&livre(), &Interieur::default(), &pieces, 12.0);
        assert!(s.contains("= Préface"), "{s}");
        assert!(s.contains("= Partie III — Avant Clément"), "{s}");
    }
}
