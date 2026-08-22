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
}
