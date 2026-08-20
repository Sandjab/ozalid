//! Manuscrit Markdown → chapitres → source Typst.
//!
//! Le format admis est celui du projet, et lui seul : titre en `# `, chapitres en
//! `## NN - Titre`, séparateurs de scène `---`, emphase `*…*` et `**…**`. Tout le
//! reste est **refusé** avec son numéro de ligne plutôt que composé de travers :
//! une liste ou un lien silencieusement aplati donnerait un livre imprimé faux,
//! découvert après tirage.
//!
//! Ce n'est donc pas un convertisseur Markdown général. Si le besoin apparaît, la
//! bascule vers un vrai parseur (`pulldown-cmark`) se fera derrière la même API.

#[derive(Debug, Clone, PartialEq)]
pub struct Chapitre {
    pub numero: u32,
    pub titre: String,
    pub paragraphes: Vec<String>,
}

/// Caractères qui ouvrent une syntaxe Typst dans du texte brut. Tout ce qui vient du
/// manuscrit ou du projet passe par là : un titre contenant `#` ne doit pas devenir
/// une expression Typst.
pub fn echappe(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if matches!(
            c,
            '\\' | '#' | '$' | '*' | '_' | '`' | '<' | '>' | '@' | '~'
        ) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// Texte d'un paragraphe → contenu Typst. L'emphase est restituée après échappement,
/// jamais avant : sinon les `*` du texte deviendraient des marqueurs.
pub fn inline(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::new();
    let mut brut = String::new();
    let mut i = 0;
    while i < chars.len() {
        let double = chars[i] == '*' && chars.get(i + 1) == Some(&'*');
        let simple = chars[i] == '*' && !double;
        if double || simple {
            let ouvre = if double { 2 } else { 1 };
            if let Some(fin) = ferme(&chars, i + ouvre, ouvre) {
                out.push_str(&echappe(&brut));
                brut.clear();
                let dedans: String = chars[i + ouvre..fin].iter().collect();
                let balise = if double { "strong" } else { "emph" };
                out.push_str(&format!("#{balise}[{}]", echappe(&dedans)));
                i = fin + ouvre;
                continue;
            }
        }
        brut.push(chars[i]);
        i += 1;
    }
    out.push_str(&echappe(&brut));
    out
}

/// Position du marqueur fermant, à condition qu'il tienne sur la même ligne et que
/// le contenu ne soit pas vide. Une astérisque isolée reste un caractère ordinaire.
fn ferme(chars: &[char], depuis: usize, largeur: usize) -> Option<usize> {
    let mut i = depuis;
    while i < chars.len() {
        if chars[i] == '\n' {
            return None;
        }
        if chars[i] == '*' {
            let suit_double = chars.get(i + 1) == Some(&'*');
            let est_double = suit_double;
            if (largeur == 2 && est_double) || (largeur == 1 && !est_double) {
                return if i > depuis { Some(i) } else { None };
            }
        }
        i += 1;
    }
    None
}

/// Constructions Markdown que la chaîne ne sait pas composer. Refusées, pas ignorées.
fn refus(ligne: &str) -> Option<&'static str> {
    let t = ligne.trim_start();
    if t.starts_with("###") {
        Some("titre de niveau 3 ou plus")
    } else if t.starts_with("> ") {
        Some("citation en bloc")
    } else if t.contains('`') {
        Some("code littéral")
    } else if t.starts_with("- ") || t.starts_with("+ ") || t.starts_with("* ") {
        Some("liste")
    } else if t.starts_with('|') {
        Some("tableau")
    } else if t.starts_with("![") {
        Some("image")
    } else {
        None
    }
}

/// Découpe le manuscrit. `attendu` est le contrôle d'intégrité facultatif du
/// `projet.toml` : il n'a de sens qu'au gel, quand le compte ne doit plus bouger.
pub fn decoupe(md: &str, attendu: Option<u32>) -> Result<Vec<Chapitre>, String> {
    let mut chapitres: Vec<Chapitre> = Vec::new();
    for (no, ligne) in md.lines().enumerate() {
        let no = no + 1;
        let t = ligne.trim();
        if let Some(quoi) = refus(ligne) {
            return Err(format!(
                "ligne {no} : {quoi} — non composable par la chaîne."
            ));
        }
        if let Some(reste) = t.strip_prefix("## ") {
            chapitres.push(entete(reste.trim(), no)?);
        } else if t.starts_with("# ") || t == "---" || t.is_empty() {
            // Titre du livre : le projet fait foi, pas le manuscrit. Séparateurs de
            // scène : sans rendu propre, comme dans la chaîne actuelle.
            continue;
        } else if let Some(courant) = chapitres.last_mut() {
            courant.paragraphes.push(t.to_string());
        } else {
            // Avant le premier « ## » : liminaires du manuscrit, composés par le projet.
            continue;
        }
    }
    if chapitres.is_empty() {
        return Err("aucun chapitre trouvé (attendu : « ## NN - Titre »).".into());
    }
    if let Some(n) = attendu {
        let trouves = chapitres.len() as u32;
        if trouves != n {
            return Err(format!(
                "{n} chapitres attendus (projet), {trouves} trouvés."
            ));
        }
    }
    Ok(chapitres)
}

fn entete(reste: &str, no: usize) -> Result<Chapitre, String> {
    let (num, titre) = match reste.split_once('-') {
        Some((n, t)) => (n.trim(), t.trim()),
        None => (reste, ""),
    };
    let numero: u32 = num.parse().map_err(|_| {
        format!("ligne {no} : titre de chapitre « {reste} » (attendu : « NN - Titre »).")
    })?;
    Ok(Chapitre {
        numero,
        titre: titre.to_string(),
        paragraphes: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn un_chapitre_numerote_et_titre_est_decoupe() {
        let ch = decoupe(
            "## 01 - Vingt centimes\n\nUne phrase.\n\nUne autre.\n",
            None,
        )
        .unwrap();
        assert_eq!(ch.len(), 1);
        assert_eq!(ch[0].numero, 1);
        assert_eq!(ch[0].titre, "Vingt centimes");
        assert_eq!(ch[0].paragraphes.len(), 2);
    }

    #[test]
    fn un_chapitre_sans_titre_est_admis() {
        let ch = decoupe("## 7\n\nTexte.\n", None).unwrap();
        assert_eq!(ch[0].numero, 7);
        assert_eq!(ch[0].titre, "");
    }

    /// Le titre du livre appartient au projet, pas au manuscrit : le `#` de tête ne
    /// doit jamais se retrouver dans le corps composé.
    #[test]
    fn le_titre_du_livre_et_les_separateurs_ne_deviennent_pas_du_corps() {
        let ch = decoupe(
            "# Le Livre\n\n*Roman*\n\n---\n\n## 01 - Un\n\nTexte.\n",
            None,
        )
        .unwrap();
        assert_eq!(ch[0].paragraphes, vec!["Texte."]);
    }

    /// Le contrôle d'intégrité sert au gel : il doit échouer sur un chapitre perdu,
    /// sans quoi une coupure de manuscrit passerait inaperçue jusqu'au tirage.
    #[test]
    fn le_controle_d_integrite_detecte_un_chapitre_manquant() {
        let md = "## 01 - Un\n\nA.\n\n## 02 - Deux\n\nB.\n";
        assert!(decoupe(md, Some(2)).is_ok());
        let err = decoupe(md, Some(3)).unwrap_err();
        assert!(err.contains("3 chapitres attendus"), "{err}");
    }

    /// Fail loud : mieux vaut refuser que composer de travers un manuscrit dont la
    /// chaîne ne sait pas rendre une construction.
    #[test]
    fn une_construction_non_composable_est_refusee_avec_sa_ligne() {
        for (md, quoi) in [
            ("## 01 - Un\n\n- puce\n", "liste"),
            ("## 01 - Un\n\n> citation\n", "citation"),
            ("## 01 - Un\n\n### Sous-titre\n", "titre de niveau 3"),
            ("## 01 - Un\n\nvoir `x`\n", "code"),
        ] {
            let err = decoupe(md, None).unwrap_err();
            assert!(err.contains("ligne 3"), "{md} → {err}");
            assert!(err.contains(quoi), "{md} → {err}");
        }
    }

    #[test]
    fn un_entete_mal_forme_est_refuse() {
        let err = decoupe("## Chapitre premier\n\nTexte.\n", None).unwrap_err();
        assert!(err.contains("NN - Titre"), "{err}");
    }

    /// L'échappement passe avant l'emphase : un `#` du texte ne doit pas ouvrir une
    /// expression Typst, et un `*` isolé reste une astérisque.
    #[test]
    fn le_texte_brut_ne_peut_pas_injecter_de_syntaxe_typst() {
        assert_eq!(inline("le #chapitre coûte 3$"), "le \\#chapitre coûte 3\\$");
        assert_eq!(inline("2 * 3 = 6"), "2 \\* 3 = 6");
    }

    #[test]
    fn l_emphase_simple_et_double_devient_du_contenu_typst() {
        assert_eq!(inline("un *mot* ici"), "un #emph[mot] ici");
        assert_eq!(inline("une **pancarte**."), "une #strong[pancarte].");
        // Emphase contenant un caractère spécial : échappée à l'intérieur aussi.
        assert_eq!(inline("*a#b*"), "#emph[a\\#b]");
    }

    /// Une emphase non refermée sur la ligne est du texte, pas une balise ouverte :
    /// sinon un astérisque oublié ferait avaler la fin du chapitre.
    #[test]
    fn une_emphase_non_refermee_reste_du_texte() {
        assert_eq!(inline("il a dit *bonjour"), "il a dit \\*bonjour");
        assert_eq!(inline("3 ** 2"), "3 \\*\\* 2");
    }
}
