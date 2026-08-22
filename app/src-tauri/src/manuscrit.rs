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

/// Un bloc de chapitre.
///
/// Une rupture de scène n'est ni un paragraphe vide ni de la mise en page : c'est une
/// coupure que l'auteur a écrite. Elle est typée pour que chaque composition décide
/// quoi en faire — l'épreuve et l'intérieur la rendent tous deux, chacun avec son
/// espace autour de la même marque.
#[derive(Debug, Clone, PartialEq)]
pub enum Bloc {
    Paragraphe(String),
    Scene,
}

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
/// Elle vit ici, à côté du bloc qu'elle rend, parce que l'épreuve et le livre la
/// composent tous deux : ce qu'on relit doit être ce qui s'imprime. Ce qui les sépare
/// est l'espace autour, que chacun règle à sa page.
///
/// Les `\*` sont échappés : en markup Typst, `*` ouvre une emphase.
pub const SCENE: &str = r"\*#h(0.8em)\*#h(0.8em)\*";

#[derive(Debug, Clone, PartialEq)]
pub struct Chapitre {
    pub numero: u32,
    pub titre: String,
    pub blocs: Vec<Bloc>,
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

/// Texte brut → contenu d'une chaîne Typst, entre guillemets droits.
///
/// Rien à voir avec [`echappe`], qui protège le *markup* : ici le texte n'est pas
/// composé, il est cité — `#set document(title: "…")` et la ligne de commentaire qui
/// ouvre chaque source. Le `#` y est un caractère comme un autre, mais le `"` referme
/// la chaîne et le saut de ligne fait sortir du commentaire ce qui le suit.
///
/// Les sauts de ligne deviennent la séquence `\n`, pas un espace : la chaîne dit
/// toujours le titre entier, et le commentaire tient sur sa ligne.
pub fn echappe_chaine(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str(r"\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str(r"\n"),
            '\r' => out.push_str(r"\r"),
            _ => out.push(c),
        }
    }
    out
}

/// Un morceau de paragraphe : du texte, ou du texte sous emphase.
///
/// La coupure du texte est ici, le rendu chez l'appelant : l'intérieur en fait du
/// markup Typst, l'EPUB des balises XHTML, et les deux ne peuvent plus diverger sur
/// ce qu'ils ont lu. Le texte porté est **brut** — c'est à chaque sortie de
/// l'échapper dans son langage, et les deux langages n'ont pas un caractère dangereux
/// en commun.
#[derive(Debug, Clone, PartialEq)]
pub enum Morceau {
    Brut(String),
    Emph(String),
    Fort(String),
}

/// Texte d'un paragraphe → suite de morceaux.
///
/// Les segments bruts consécutifs sont recollés : un paragraphe sans emphase ne fait
/// qu'un morceau.
pub fn morceaux(s: &str) -> Vec<Morceau> {
    let chars: Vec<char> = s.chars().collect();
    let mut out = Vec::new();
    let mut brut = String::new();
    let mut i = 0;
    while i < chars.len() {
        let double = chars[i] == '*' && chars.get(i + 1) == Some(&'*');
        let simple = chars[i] == '*' && !double;
        if double || simple {
            let ouvre = if double { 2 } else { 1 };
            if let Some(fin) = ferme(&chars, i + ouvre, ouvre) {
                if !brut.is_empty() {
                    out.push(Morceau::Brut(std::mem::take(&mut brut)));
                }
                let dedans: String = chars[i + ouvre..fin].iter().collect();
                out.push(if double {
                    Morceau::Fort(dedans)
                } else {
                    Morceau::Emph(dedans)
                });
                i = fin + ouvre;
                continue;
            }
        }
        brut.push(chars[i]);
        i += 1;
    }
    if !brut.is_empty() {
        out.push(Morceau::Brut(brut));
    }
    out
}

/// Texte d'un paragraphe → contenu Typst. L'emphase est restituée après échappement,
/// jamais avant : sinon les `*` du texte deviendraient des marqueurs.
pub fn inline(s: &str) -> String {
    morceaux(s)
        .into_iter()
        .map(|m| match m {
            Morceau::Brut(t) => echappe(&t),
            Morceau::Emph(t) => format!("#emph[{}]", echappe(&t)),
            Morceau::Fort(t) => format!("#strong[{}]", echappe(&t)),
        })
        .collect()
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

/// Une rupture de scène sépare deux passages d'un même chapitre ; une rupture qui
/// ouvre ou ferme un chapitre ne sépare rien. Cet invariant vaut quel que soit l'usage
/// que l'auteur fait de `---` dans son manuscrit — y compris l'usage réel observé sur
/// *WIP7* (build/in/texts/WIP7.md) : ses 64 `---` précèdent chacun un `## `, comme un
/// filet de fin de chapitre plutôt qu'une séparation de scène. Sans cet élagage,
/// `decoupe` laisserait un `Bloc::Scene` orphelin en fin de chaque chapitre — invisible
/// tant que l'intérieur ignore les `Scene`, mais que l'épreuve afficherait comme une
/// astérisque parasite avant chaque saut de page.
fn elague_rupture_finale(ch: Option<&mut Chapitre>) {
    if let Some(ch) = ch {
        if matches!(ch.blocs.last(), Some(Bloc::Scene)) {
            ch.blocs.pop();
        }
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
            // Le chapitre qui se ferme ne doit pas garder de rupture en dernière
            // position : elle ne séparerait rien.
            elague_rupture_finale(chapitres.last_mut());
            chapitres.push(entete(reste.trim(), no)?);
        } else if t == "---" {
            // Hors chapitre, la rupture appartient aux liminaires : rien à garder. Dans
            // un chapitre, elle n'est gardée qu'à la suite d'un paragraphe : ni en tête
            // de chapitre, ni après une rupture déjà posée (deux `---` consécutifs ne
            // séparent qu'une fois).
            if let Some(courant) = chapitres.last_mut() {
                if matches!(courant.blocs.last(), Some(Bloc::Paragraphe(_))) {
                    courant.blocs.push(Bloc::Scene);
                }
            }
        } else if t.starts_with("# ") || t.is_empty() {
            // Titre du livre : le projet fait foi, pas le manuscrit.
            continue;
        } else if let Some(courant) = chapitres.last_mut() {
            courant.blocs.push(Bloc::Paragraphe(t.to_string()));
        } else {
            // Avant le premier « ## » : liminaires du manuscrit, composés par le projet.
            continue;
        }
    }
    // Le dernier chapitre du manuscrit n'a pas de « ## » suivant pour déclencher
    // l'élagage : il faut le faire une dernière fois en sortie de boucle.
    elague_rupture_finale(chapitres.last_mut());
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
        blocs: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// La coupure de l'emphase est typée pour que l'intérieur et l'EPUB rendent la
    /// **même** lecture du manuscrit. Ce test est le contrat entre les deux : il dit ce
    /// qui a été lu, pas ce qui en est fait.
    #[test]
    fn l_emphase_est_coupee_en_morceaux_types() {
        assert_eq!(
            morceaux("un *mot* et **deux** mots"),
            vec![
                Morceau::Brut("un ".into()),
                Morceau::Emph("mot".into()),
                Morceau::Brut(" et ".into()),
                Morceau::Fort("deux".into()),
                Morceau::Brut(" mots".into()),
            ]
        );
    }

    /// Une astérisque qui ne ferme pas reste un caractère du texte : c'est ce que fait
    /// `inline` depuis toujours, et le passage par `morceaux` ne doit pas le changer.
    #[test]
    fn une_asterisque_isolee_reste_du_texte_brut() {
        assert_eq!(
            morceaux("3 * 4 = 12"),
            vec![Morceau::Brut("3 * 4 = 12".into())]
        );
    }

    /// Un manuscrit sans emphase ne produit qu'un morceau : les segments bruts se
    /// recollent au lieu de sortir caractère par caractère.
    #[test]
    fn le_texte_sans_emphase_ne_fait_qu_un_morceau() {
        assert_eq!(
            morceaux("rien à signaler"),
            vec![Morceau::Brut("rien à signaler".into())]
        );
    }

    /// La contre-oblique était sauve par accident : `echappe` la doublait pour le
    /// markup, et la chaîne s'en trouvait bien. Cité, le titre ne passe plus par lui —
    /// une contre-oblique laissée nue ouvrirait une séquence d'échappement, et le `\n`
    /// qu'un titre porte en toutes lettres deviendrait un vrai saut de ligne.
    #[test]
    fn une_contre_oblique_citee_reste_doublee() {
        assert_eq!(echappe_chaine(r"Le quai \ nord"), r"Le quai \\ nord");
        assert_eq!(echappe_chaine("quai\r\nnord"), r"quai\r\nnord");
        // Le `#` ouvre une expression en markup, jamais dans une chaîne : l'échapper
        // ici l'imprimerait dans les métadonnées du PDF.
        assert_eq!(echappe_chaine("Ivan #Pjig"), "Ivan #Pjig");
    }

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
        assert_eq!(ch[0].blocs.len(), 2);
    }

    #[test]
    fn un_chapitre_sans_titre_est_admis() {
        let ch = decoupe("## 7\n\nTexte.\n", None).unwrap();
        assert_eq!(ch[0].numero, 7);
        assert_eq!(ch[0].titre, "");
    }

    /// Le titre du livre appartient au projet, pas au manuscrit : le `#` de tête ne
    /// doit jamais se retrouver dans le corps composé. Le séparateur de scène, lui,
    /// n'est plus silencieusement perdu : il est conservé sous forme de `Bloc::Scene`.
    #[test]
    fn le_titre_du_livre_ne_devient_pas_du_corps_mais_le_separateur_est_conserve() {
        let ch = decoupe(
            "# Le Livre\n\n*Roman*\n\n## 01 - Un\n\nTexte.\n\n---\n\nSuite.\n",
            None,
        )
        .unwrap();
        assert_eq!(
            ch[0].blocs,
            vec![
                Bloc::Paragraphe("Texte.".into()),
                Bloc::Scene,
                Bloc::Paragraphe("Suite.".into()),
            ]
        );
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

    /// Une rupture de scène est une intention de l'auteur, pas une ligne vide. Elle est
    /// gardée telle quelle : l'épreuve la compose, l'intérieur l'ignore encore.
    #[test]
    fn une_rupture_de_scene_est_gardee_comme_bloc() {
        let ch = decoupe("## 01 - Un\n\nAvant.\n\n---\n\nAprès.\n", None).unwrap();
        assert_eq!(
            ch[0].blocs,
            vec![
                Bloc::Paragraphe("Avant.".into()),
                Bloc::Scene,
                Bloc::Paragraphe("Après.".into()),
            ]
        );
    }

    /// Un `---` avant le premier chapitre appartient aux liminaires du manuscrit, que
    /// le projet compose lui-même : il ne doit ouvrir aucun chapitre fantôme.
    #[test]
    fn une_rupture_avant_le_premier_chapitre_est_ignoree() {
        let ch = decoupe("# Le Livre\n\n---\n\n## 01 - Un\n\nTexte.\n", None).unwrap();
        assert_eq!(ch.len(), 1);
        assert_eq!(ch[0].blocs, vec![Bloc::Paragraphe("Texte.".into())]);
    }

    /// Une rupture qui ouvre un chapitre ne sépare rien : elle n'a pas de passage
    /// précédent dans ce chapitre à séparer d'un passage suivant.
    #[test]
    fn une_rupture_en_tete_de_chapitre_ne_laisse_pas_de_bloc() {
        let ch = decoupe("## 01 - Un\n\n---\n\nTexte.\n", None).unwrap();
        assert_eq!(ch[0].blocs, vec![Bloc::Paragraphe("Texte.".into())]);
    }

    /// Une rupture qui ferme un chapitre ne sépare rien non plus : c'est le cas réel du
    /// manuscrit *WIP7*, où chaque `---` annonce le chapitre suivant plutôt que de
    /// séparer deux scènes du même chapitre. Sans élagage, chaque chapitre du livre
    /// composé garderait un `Bloc::Scene` orphelin en dernière position.
    #[test]
    fn une_rupture_en_fin_de_chapitre_ne_laisse_pas_de_bloc() {
        let ch = decoupe(
            "## 01 - Un\n\nTexte.\n\n---\n\n## 02 - Deux\n\nSuite.\n",
            None,
        )
        .unwrap();
        assert_eq!(ch[0].blocs, vec![Bloc::Paragraphe("Texte.".into())]);
        assert_eq!(ch[1].blocs, vec![Bloc::Paragraphe("Suite.".into())]);
    }

    /// Deux ruptures consécutives ne séparent qu'une fois : la seconde ne trouve aucun
    /// paragraphe à sa suite immédiate dans le chapitre, elle est donc ignorée plutôt
    /// que d'empiler une deuxième `Bloc::Scene`.
    #[test]
    fn deux_ruptures_consecutives_n_en_font_qu_une() {
        let ch = decoupe("## 01 - Un\n\nAvant.\n\n---\n\n---\n\nAprès.\n", None).unwrap();
        assert_eq!(
            ch[0].blocs,
            vec![
                Bloc::Paragraphe("Avant.".into()),
                Bloc::Scene,
                Bloc::Paragraphe("Après.".into()),
            ]
        );
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

    /// Le manuscrit-témoin de la CI, embarqué à la compilation des tests.
    const TEMOIN: &str = include_str!("../temoin/manuscrit.md");

    /// Ce que la CI compose doit d'abord passer la porte du format. Ce test échoue en
    /// une seconde là où l'exemple `temoin` coûte une composition entière.
    #[test]
    fn le_manuscrit_temoin_est_composable() {
        let chapitres = decoupe(TEMOIN, Some(30)).expect("le témoin doit être composable");
        assert_eq!(chapitres.len(), 30);
        assert_eq!(chapitres[0].numero, 1);
        assert!(
            !chapitres[0].titre.is_empty(),
            "un chapitre sans titre : la conversion a mangé l'en-tête"
        );
    }

    /// Un checkout Windows peut convertir les fins de ligne malgré `.gitattributes` si
    /// celui-ci venait à disparaître. Les `\r` ne se verraient pas dans le découpage —
    /// `str::lines` les retire — mais ils entreraient dans les paragraphes, donc dans la
    /// source Typst, et déplaceraient peut-être la pagination sans rien dire.
    #[test]
    fn le_manuscrit_temoin_est_en_fins_de_ligne_unix() {
        assert!(
            !TEMOIN.contains('\r'),
            "le témoin porte des retours chariot : .gitattributes n'a pas joué"
        );
    }
}
