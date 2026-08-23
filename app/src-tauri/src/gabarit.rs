//! Les jetons `%CLE%` des champs libres du livre.
//!
//! Un champ libre — le titre de la page de titre, la dédicace, le copyright — peut
//! citer un champ clé. La substitution se fait **à la composition**, jamais à la
//! saisie : le `.ozalid` conserve le texte à jetons, qui doit suivre le livre si le
//! titre change.

use crate::projet::Livre;

/// Un jeton et le champ clé qu'il désigne.
type Jeton = (&'static str, fn(&Livre) -> &str);

/// Les jetons reconnus, et le champ clé que chacun désigne.
///
/// Les clés sont littérales par définition : aucune n'est elle-même substituée, et
/// c'est ce qui rend toute référence cyclique impossible — il n'y a pas de chaîne à
/// parcourir, donc rien à borner.
const JETONS: [Jeton; 3] = [
    ("%TITRE%", |l| &l.titre),
    ("%AUTEUR%", |l| &l.auteur),
    ("%GENRE%", |l| &l.genre),
];

/// Remplace les jetons connus par la valeur de leur champ clé.
///
/// **Une seule passe.** Le texte est parcouru une fois de gauche à droite : ce qu'un
/// jeton produit est poussé dans la sortie et jamais réexaminé.
///
/// Ce n'est pas une garde contre les références cycliques — il ne peut pas y en avoir,
/// un jeton ne désignant qu'une clé et une clé n'étant jamais substituée. C'est une
/// garde contre la relecture de la sortie : un `replace` par jeton en boucle aurait
/// l'air équivalent et ne l'est pas, car il traiterait la valeur du jeton précédent
/// comme du texte à substituer. Un titre valant « 100 % coton » suffit à le montrer.
///
/// Un jeton inconnu est recopié tel quel.
pub fn substituer(texte: &str, livre: &Livre) -> String {
    let mut sortie = String::with_capacity(texte.len());
    let mut reste = texte;
    while let Some(i) = reste.find('%') {
        sortie.push_str(&reste[..i]);
        let a_partir_du_pour_cent = &reste[i..];
        match JETONS
            .iter()
            .find(|(jeton, _)| a_partir_du_pour_cent.starts_with(jeton))
        {
            Some((jeton, valeur)) => {
                sortie.push_str(valeur(livre));
                reste = &a_partir_du_pour_cent[jeton.len()..];
            }
            None => {
                sortie.push('%');
                reste = &a_partir_du_pour_cent[1..];
            }
        }
    }
    sortie.push_str(reste);
    sortie
}

#[cfg(test)]
mod tests {
    use super::*;

    fn livre() -> Livre {
        Livre {
            titre: "Les Heures creuses".into(),
            auteur: "Ivan Pjig".into(),
            genre: "roman".into(),
            ..Livre::vide()
        }
    }

    #[test]
    fn chaque_jeton_prend_la_valeur_de_sa_cle() {
        let l = livre();
        assert_eq!(substituer("%TITRE%", &l), "Les Heures creuses");
        assert_eq!(substituer("%AUTEUR%", &l), "Ivan Pjig");
        assert_eq!(substituer("%GENRE%", &l), "roman");
    }

    #[test]
    fn un_texte_sans_jeton_ne_bouge_pas() {
        assert_eq!(
            substituer("Tous droits réservés.", &livre()),
            "Tous droits réservés."
        );
    }

    #[test]
    fn plusieurs_jetons_dans_une_phrase() {
        assert_eq!(
            substituer("%TITRE%, un %GENRE% de %AUTEUR%.", &livre()),
            "Les Heures creuses, un roman de Ivan Pjig.",
        );
    }

    /// Un jeton inconnu traverse intact : il se voit dans l'aperçu et sur l'épreuve.
    /// Le supprimer ferait disparaître du texte sans laisser de trace.
    #[test]
    fn un_jeton_inconnu_reste_tel_quel() {
        assert_eq!(substituer("%TITER% et 100 %", &livre()), "%TITER% et 100 %");
    }

    /// **Le test qui compte.** Aucun cycle n'est possible — un jeton ne désigne qu'une
    /// clé, et une clé n'est jamais substituée. Le risque est ailleurs : une valeur de
    /// clé peut *contenir* ce qui ressemble à un jeton, sans rien désigner du tout.
    /// « 100 % coton » est un titre légitime. Relire la sortie ferait dire au copyright
    /// autre chose que ce qui est écrit dans le champ.
    #[test]
    fn une_valeur_qui_ressemble_a_un_jeton_reste_litterale() {
        let l = Livre {
            titre: "%AUTEUR%".into(),
            auteur: "Ivan Pjig".into(),
            ..Livre::vide()
        };
        assert_eq!(substituer("%TITRE%", &l), "%AUTEUR%");
    }

    /// Un pour-cent isolé, une paire vide, un jeton tronqué : rien ne doit paniquer
    /// ni manger le texte qui suit.
    #[test]
    fn les_pour_cent_isoles_survivent() {
        let l = livre();
        assert_eq!(substituer("100 % coton", &l), "100 % coton");
        assert_eq!(substituer("%%", &l), "%%");
        assert_eq!(substituer("%TITRE", &l), "%TITRE");
    }
}
