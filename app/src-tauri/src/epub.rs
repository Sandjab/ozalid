//! Le livre en EPUB 3 : une archive, et rien d'autre.
//!
//! Ce module ne touche pas au disque, n'appelle pas Typst et ne connaît aucun
//! prestataire : il reçoit des chapitres et des octets, il rend des octets. C'est ce
//! qui le rend éprouvable en entier sans `fonts/`, sans sidecar et sans répertoire
//! temporaire.
//!
//! L'EPUB est **reflowable** : le lecteur choisit son corps, et la pagination n'y veut
//! plus rien dire. Rien ici ne cherche donc à reproduire la mise en page du papier —
//! seulement ce qui appartient au livre : son texte, sa coupure en chapitres, ses
//! ruptures de scène, son œil.

/// Texte brut → contenu XML.
///
/// Rien à voir avec `manuscrit::echappe`, qui protège le markup Typst : les deux
/// langages n'ont pas un caractère dangereux en commun, et les confondre laisserait
/// passer une esperluette ici ou un dièse là-bas.
///
/// L'apostrophe est échappée bien qu'elle ne soit dangereuse que dans un attribut :
/// le même échappement sert au texte et aux attributs, et une seule règle vaut mieux
/// que deux dont on choisirait mal.
fn echappe(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(c),
        }
    }
    out
}

use crate::manuscrit::{self, Bloc, Chapitre, Morceau};

/// La rupture de scène telle que l'EPUB l'écrit.
///
/// Le même caractère que sur le papier — `manuscrit::SCENE` l'a choisi parce qu'il est
/// le seul présent dans tous les fichiers de `fonts/` —, mais pas la même chaîne : la
/// constante du manuscrit est du markup Typst, `\*#h(0.8em)\*`, illisible ici.
/// L'espacement vient des espaces insécables et du CSS, non de `#h()`.
const SCENE_XHTML: &str = "*\u{a0}*\u{a0}*";

/// Texte d'un paragraphe → contenu XHTML.
///
/// La lecture des astérisques est celle de `manuscrit::morceaux`, partagée avec
/// l'intérieur : l'EPUB et le papier ne peuvent pas comprendre le même paragraphe
/// autrement. Seul le rendu diffère.
fn paragraphe(s: &str) -> String {
    manuscrit::morceaux(s)
        .into_iter()
        .map(|m| match m {
            Morceau::Brut(t) => echappe(&t),
            Morceau::Emph(t) => format!("<em>{}</em>", echappe(&t)),
            Morceau::Fort(t) => format!("<strong>{}</strong>", echappe(&t)),
        })
        .collect()
}

/// Le titre d'un chapitre tel qu'il paraît dans la table des matières.
fn intitule(ch: &Chapitre) -> String {
    if ch.titre.is_empty() {
        ch.numero.to_string()
    } else {
        format!("{} — {}", ch.numero, ch.titre)
    }
}

/// Un chapitre, dans son propre fichier.
///
/// Un seul `<h1>`, qui porte le numéro et le titre : c'est lui que la table des
/// matières vise, et deux titres de rang 1 par fichier dérouteraient les liseuses qui
/// bâtissent leur sommaire sur la structure plutôt que sur le `nav`.
fn chapitre_xhtml(ch: &Chapitre) -> String {
    let mut corps = String::from("<h1>");
    corps.push_str(&format!(r#"<span class="numero">{}</span>"#, ch.numero));
    if !ch.titre.is_empty() {
        corps.push_str(&format!(
            r#"<span class="titre">{}</span>"#,
            echappe(&ch.titre)
        ));
    }
    corps.push_str("</h1>\n");
    for b in &ch.blocs {
        match b {
            Bloc::Paragraphe(p) => corps.push_str(&format!("<p>{}</p>\n", paragraphe(p))),
            Bloc::Scene => corps.push_str(&format!("<p class=\"scene\">{SCENE_XHTML}</p>\n")),
        }
    }
    page(&intitule(ch), &corps)
}

/// L'enveloppe XHTML commune à toutes les pages de l'archive.
fn page(titre: &str, corps: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops" lang="fr" xml:lang="fr">
<head>
<meta charset="utf-8"/>
<title>{}</title>
<link rel="stylesheet" type="text/css" href="style.css"/>
</head>
<body>
{corps}</body>
</html>
"#,
        echappe(titre)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Les deux langages de sortie du projet n'ont pas un caractère dangereux en
    /// commun : `manuscrit::echappe` protège le markup Typst (`#`, `$`, `*`…), celui-ci
    /// protège le XML. Les confondre laisserait passer une esperluette dans une
    /// archive, qu'aucune liseuse n'ouvrirait.
    #[test]
    fn l_echappement_xml_protege_ce_que_le_xml_craint() {
        assert_eq!(
            echappe(r#"Rémi & <Léa> dit "oui" d'un trait"#),
            "Rémi &amp; &lt;Léa&gt; dit &quot;oui&quot; d&apos;un trait"
        );
    }

    /// Le dièse ouvre une expression Typst, pas une entité XML : l'échappement XML ne
    /// doit pas y toucher, sans quoi les deux modules finiraient par se recopier.
    #[test]
    fn l_echappement_xml_laisse_passer_ce_qui_ne_regarde_que_typst() {
        assert_eq!(echappe("#1 *gras* $x$"), "#1 *gras* $x$");
    }

    /// L'emphase du manuscrit devient une balise, et l'échappement s'applique **au
    /// contenu**, pas au marqueur : un « & » sous emphase doit sortir échappé dans son
    /// `<em>`.
    #[test]
    fn l_emphase_devient_une_balise_et_le_contenu_reste_echappe() {
        assert_eq!(
            paragraphe("il dit *oui & non* puis **rien**"),
            "il dit <em>oui &amp; non</em> puis <strong>rien</strong>"
        );
    }

    /// La rupture de scène est le même caractère que sur le papier. `manuscrit::SCENE`
    /// n'est pas réutilisable — c'est du markup Typst — mais l'astérisque qu'il porte,
    /// si : ce test amarre les deux formes l'une à l'autre, pour que changer la marque du
    /// livre sans changer celle de l'EPUB se voie.
    #[test]
    fn la_rupture_de_scene_porte_la_meme_asterisque_que_le_papier() {
        assert!(crate::manuscrit::SCENE.contains(r"\*"));
        assert_eq!(SCENE_XHTML.matches('*').count(), 3);
        assert!(!SCENE_XHTML.contains('#'));
    }

    /// Un chapitre rend un titre unique — numéro et titre dans le même `<h1>` — puis ses
    /// blocs dans l'ordre. Deux `<h1>` par fichier dérouteraient la table des matières.
    #[test]
    fn un_chapitre_rend_un_titre_unique_puis_ses_blocs() {
        let ch = Chapitre {
            numero: 12,
            titre: "Le seuil".into(),
            blocs: vec![
                Bloc::Paragraphe("Premier.".into()),
                Bloc::Scene,
                Bloc::Paragraphe("Second.".into()),
            ],
        };
        let x = chapitre_xhtml(&ch);
        assert_eq!(x.matches("<h1").count(), 1);
        assert!(x.contains(r#"<span class="numero">12</span>"#), "{x}");
        assert!(x.contains(r#"<span class="titre">Le seuil</span>"#), "{x}");
        assert!(x.contains("<p>Premier.</p>"), "{x}");
        assert!(x.contains(r#"<p class="scene">"#), "{x}");
        assert!(x.contains("<p>Second.</p>"), "{x}");
        // L'ordre du manuscrit est l'ordre du fichier.
        assert!(x.find("Premier.") < x.find("Second."));
    }

    /// Un chapitre sans titre n'écrit pas de `<span class="titre">` vide : une liseuse
    /// afficherait une ligne blanche dans sa table des matières.
    #[test]
    fn un_chapitre_sans_titre_n_ecrit_pas_de_titre_vide() {
        let ch = Chapitre {
            numero: 1,
            titre: String::new(),
            blocs: vec![],
        };
        let x = chapitre_xhtml(&ch);
        assert!(!x.contains(r#"class="titre""#), "{x}");
        assert!(x.contains(r#"<span class="numero">1</span>"#), "{x}");
    }

    /// Un titre de chapitre contenant une esperluette casserait l'archive s'il n'était pas
    /// échappé — et le manuscrit en admet une, c'est du texte ordinaire.
    #[test]
    fn un_titre_de_chapitre_est_echappe() {
        let ch = Chapitre {
            numero: 3,
            titre: "Pile & face".into(),
            blocs: vec![],
        };
        assert!(chapitre_xhtml(&ch).contains("Pile &amp; face"));
    }
}
