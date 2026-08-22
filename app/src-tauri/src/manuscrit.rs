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

/// Ce qu'une pièce est, et où elle se compose. La position découle de la sorte : aucun
/// appelant n'a à la déduire du titre.
#[derive(Debug, Clone, PartialEq)]
pub enum Sorte {
    /// Un chapitre et son numéro, tel que le manuscrit l'écrit.
    Chapitre(u32),
    /// Une pièce qui précède le corps : préface, avant-propos, prologue.
    Liminaire,
    /// Une pièce qui le suit : épilogue, postface, remerciements.
    Annexe,
    /// Une page de partie et son romain, réimprimé tel qu'écrit.
    Partie(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Piece {
    pub sorte: Sorte,
    pub titre: String,
    pub blocs: Vec<Bloc>,
}

impl Piece {
    /// Le compte d'intégrité du projet et celui qu'affiche l'interface ne comptent que
    /// les chapitres : une préface n'est pas un chapitre en moins ni en plus.
    pub fn est_chapitre(&self) -> bool {
        matches!(self.sorte, Sorte::Chapitre(_))
    }
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

/// Un entier → son romain, forme canonique.
///
/// Les parties d'un roman se comptent sur les doigts : la table s'arrête à `L`, et
/// au-delà c'est une faute de frappe, pas une intention.
fn en_romain(mut n: u32) -> String {
    const TABLE: [(u32, &str); 7] = [
        (50, "L"),
        (40, "XL"),
        (10, "X"),
        (9, "IX"),
        (5, "V"),
        (4, "IV"),
        (1, "I"),
    ];
    let mut s = String::new();
    for (v, sym) in TABLE {
        while n >= v {
            s.push_str(sym);
            n -= v;
        }
    }
    s
}

/// Un romain de partie → sa valeur, à condition qu'il soit écrit sous sa forme
/// canonique. `IIII` vaudrait 4 pour un parseur laxiste et s'imprimerait tel quel :
/// on le refuse plutôt que de composer une page de partie fautive.
fn romain(s: &str) -> Option<u32> {
    (1..=50).find(|n| en_romain(*n) == s)
}

/// Les pièces qui précèdent le corps, et celles qui le suivent.
///
/// Liste **fermée** : c'est elle qui permet d'admettre un titre non numéroté sans
/// rouvrir le format. `## Chapitre premier` doit rester une erreur.
const LIMINAIRES: [&str; 3] = ["Préface", "Avant-propos", "Prologue"];
const ANNEXES: [&str; 3] = ["Épilogue", "Postface", "Remerciements"];

/// Un titre → la pièce qu'il nomme, s'il en nomme une.
///
/// Insensible à la casse, pas aux accents : le titre rendu est celui de la liste, pour
/// que ce qui s'imprime ne dépende pas de ce qui a été tapé.
fn mot_cle(titre: &str) -> Option<(Sorte, &'static str)> {
    let bas = titre.to_lowercase();
    if let Some(m) = LIMINAIRES.iter().find(|m| m.to_lowercase() == bas) {
        return Some((Sorte::Liminaire, *m));
    }
    ANNEXES
        .iter()
        .find(|m| m.to_lowercase() == bas)
        .map(|m| (Sorte::Annexe, *m))
}

/// Une rupture de scène sépare deux passages d'un même chapitre ; une rupture qui
/// ouvre ou ferme un chapitre ne sépare rien. Cet invariant vaut quel que soit l'usage
/// que l'auteur fait de `---` dans son manuscrit — y compris l'usage réel observé sur
/// *WIP7* (build/in/texts/WIP7.md) : ses 64 `---` précèdent chacun un `## `, comme un
/// filet de fin de chapitre plutôt qu'une séparation de scène. Sans cet élagage,
/// `decoupe` laisserait un `Bloc::Scene` orphelin en fin de chaque chapitre — invisible
/// tant que l'intérieur ignore les `Scene`, mais que l'épreuve afficherait comme une
/// astérisque parasite avant chaque saut de page.
fn elague_rupture_finale(ch: Option<&mut Piece>) {
    if let Some(ch) = ch {
        if matches!(ch.blocs.last(), Some(Bloc::Scene)) {
            ch.blocs.pop();
        }
    }
}

/// Découpe le manuscrit. `attendu` est le contrôle d'intégrité facultatif du
/// `projet.toml` : il n'a de sens qu'au gel, quand le compte ne doit plus bouger.
pub fn decoupe(md: &str, attendu: Option<u32>) -> Result<Vec<Piece>, String> {
    let mut pieces: Vec<Piece> = Vec::new();
    // Le manuscrit est trois zones dans cet ordre : liminaires, corps, annexes.
    let mut vu_corps = false;
    let mut vu_annexe = false;
    // Les parties se suivent depuis I : une partie sautée ne se verrait qu'au tirage.
    let mut derniere_partie = 0;
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
            elague_rupture_finale(pieces.last_mut());
            let piece = entete(reste.trim(), no)?;
            match piece.sorte {
                Sorte::Liminaire if vu_corps || vu_annexe => {
                    return Err(format!(
                        "ligne {no} : « {} » est une pièce liminaire, elle ne peut pas \
                         suivre un chapitre.",
                        piece.titre
                    ));
                }
                // Une pièce liminaire précède le corps sans l'ouvrir : deux liminaires
                // se suivent, et c'est le premier chapitre qui ferme la zone.
                Sorte::Liminaire => {}
                Sorte::Annexe => vu_annexe = true,
                Sorte::Partie(ref r) if vu_annexe => {
                    return Err(format!(
                        "ligne {no} : « Partie {r} » vient après une pièce annexe, qui \
                         ferme le livre."
                    ));
                }
                Sorte::Partie(ref r) => {
                    // `entete` a déjà refusé ce qui n'est pas un romain canonique.
                    let n = romain(r).expect("romain validé par entete");
                    if n != derniere_partie + 1 {
                        return Err(format!(
                            "ligne {no} : partie {r} après la partie {}, attendu {}.",
                            en_romain(derniere_partie),
                            en_romain(derniere_partie + 1)
                        ));
                    }
                    derniere_partie = n;
                    vu_corps = true;
                }
                _ if vu_annexe => {
                    return Err(format!(
                        "ligne {no} : « {} » vient après une pièce annexe, qui ferme le \
                         livre.",
                        piece.titre
                    ));
                }
                _ => vu_corps = true,
            }
            pieces.push(piece);
        } else if t == "---" {
            // Hors chapitre, la rupture appartient aux liminaires : rien à garder. Dans
            // un chapitre, elle n'est gardée qu'à la suite d'un paragraphe : ni en tête
            // de chapitre, ni après une rupture déjà posée (deux `---` consécutifs ne
            // séparent qu'une fois).
            if let Some(courant) = pieces.last_mut() {
                if matches!(courant.blocs.last(), Some(Bloc::Paragraphe(_))) {
                    courant.blocs.push(Bloc::Scene);
                }
            }
        } else if t.starts_with("# ") || t.is_empty() {
            // Titre du livre : le projet fait foi, pas le manuscrit.
            continue;
        } else if let Some(courant) = pieces.last_mut() {
            if let Sorte::Partie(r) = &courant.sorte {
                return Err(format!(
                    "ligne {no} : du texte sous « Partie {r} » — une page de partie ne \
                     porte que son titre."
                ));
            }
            courant.blocs.push(Bloc::Paragraphe(t.to_string()));
        } else {
            // Avant le premier « ## » : liminaires du manuscrit, composés par le projet.
            continue;
        }
    }
    // Le dernier chapitre du manuscrit n'a pas de « ## » suivant pour déclencher
    // l'élagage : il faut le faire une dernière fois en sortie de boucle.
    elague_rupture_finale(pieces.last_mut());
    if pieces.is_empty() {
        return Err("aucun chapitre trouvé (attendu : « ## NN - Titre »).".into());
    }
    if let Some(n) = attendu {
        let trouves = pieces.iter().filter(|p| p.est_chapitre()).count() as u32;
        if trouves != n {
            return Err(format!(
                "{n} chapitres attendus (projet), {trouves} trouvés."
            ));
        }
    }
    Ok(pieces)
}

fn entete(reste: &str, no: usize) -> Result<Piece, String> {
    if let Some((sorte, mot)) = mot_cle(reste) {
        return Ok(Piece {
            sorte,
            titre: mot.to_string(),
            blocs: Vec::new(),
        });
    }
    if let Some(apres) = reste.strip_prefix("Partie ") {
        let (num, titre) = match apres.split_once('-') {
            Some((n, t)) => (n.trim(), t.trim()),
            None => (apres.trim(), ""),
        };
        if romain(num).is_none() {
            return Err(format!(
                "ligne {no} : « {num} » n'est pas un numéro de partie (attendu : I, II, \
                 III…)."
            ));
        }
        return Ok(Piece {
            sorte: Sorte::Partie(num.to_string()),
            titre: titre.to_string(),
            blocs: Vec::new(),
        });
    }
    let (num, titre) = match reste.split_once('-') {
        Some((n, t)) => (n.trim(), t.trim()),
        None => (reste, ""),
    };
    let numero: u32 = num.parse().map_err(|_| {
        format!("ligne {no} : titre de chapitre « {reste} » (attendu : « NN - Titre »).")
    })?;
    Ok(Piece {
        sorte: Sorte::Chapitre(numero),
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

    /// Les romains sont réimprimés tels qu'écrits sur la page de partie : une forme
    /// non canonique composerait un livre fautif. Seule la forme qu'on écrirait à la
    /// main est admise.
    #[test]
    fn seuls_les_romains_canoniques_sont_lus() {
        assert_eq!(romain("I"), Some(1));
        assert_eq!(romain("IV"), Some(4));
        assert_eq!(romain("XIV"), Some(14));
        assert_eq!(romain("L"), Some(50));
        assert_eq!(romain("IIII"), None, "forme non canonique");
        assert_eq!(romain("VX"), None);
        assert_eq!(romain(""), None);
        assert_eq!(romain("3"), None);
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
        assert_eq!(ch[0].sorte, Sorte::Chapitre(1));
        assert_eq!(ch[0].titre, "Vingt centimes");
        assert_eq!(ch[0].blocs.len(), 2);
    }

    #[test]
    fn un_chapitre_sans_titre_est_admis() {
        let ch = decoupe("## 7\n\nTexte.\n", None).unwrap();
        assert_eq!(ch[0].sorte, Sorte::Chapitre(7));
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

    /// Le mot fait la pièce, et la pièce fait sa place : l'auteur n'a rien à déclarer.
    #[test]
    fn une_preface_est_une_piece_liminaire_et_une_postface_une_annexe() {
        let p = decoupe(
            "## Préface\n\nA.\n\n## 01 - Un\n\nB.\n\n## Postface\n\nC.\n",
            None,
        )
        .unwrap();
        assert_eq!(p[0].sorte, Sorte::Liminaire);
        assert_eq!(p[0].titre, "Préface");
        assert_eq!(p[1].sorte, Sorte::Chapitre(1));
        assert_eq!(p[2].sorte, Sorte::Annexe);
        assert_eq!(p[2].titre, "Postface");
    }

    /// La casse tapée ne doit pas ressortir à l'impression : le titre composé est celui
    /// de la liste. Les accents, eux, sont exigés — le projet est en français accentué,
    /// et un mot désaccentué est plus probablement une faute qu'une intention.
    #[test]
    fn le_mot_cle_est_insensible_a_la_casse_mais_pas_aux_accents() {
        let p = decoupe("## préface\n\nA.\n\n## 01 - Un\n\nB.\n", None).unwrap();
        assert_eq!(p[0].titre, "Préface", "le titre composé suit la liste");

        let err = decoupe("## Preface\n\nA.\n\n## 01 - Un\n\nB.\n", None).unwrap_err();
        assert!(err.contains("NN - Titre"), "{err}");
    }

    /// « Avant-propos » porte un tiret : reconnu après le découpage « NN - Titre », il
    /// deviendrait un chapitre de numéro « Avant ». La liste blanche passe donc avant.
    #[test]
    fn un_mot_cle_a_trait_d_union_n_est_pas_lu_comme_un_chapitre() {
        let p = decoupe("## Avant-propos\n\nA.\n\n## 01 - Un\n\nB.\n", None).unwrap();
        assert_eq!(p[0].sorte, Sorte::Liminaire);
        assert_eq!(p[0].titre, "Avant-propos");
    }

    /// La position découle du mot **et** doit être tenue : pas de réordonnancement
    /// silencieux d'un manuscrit dont l'auteur a mis la préface au milieu.
    #[test]
    fn une_piece_hors_de_sa_zone_est_refusee_avec_sa_ligne() {
        let err = decoupe("## 01 - Un\n\nA.\n\n## Préface\n\nB.\n", None).unwrap_err();
        assert!(err.contains("ligne 5"), "{err}");
        assert!(err.contains("Préface"), "{err}");

        let err = decoupe("## Postface\n\nA.\n\n## 01 - Un\n\nB.\n", None).unwrap_err();
        assert!(err.contains("ligne 5"), "{err}");
    }

    /// Une pièce liminaire n'ouvre pas le corps : elle le précède. Sans quoi une
    /// préface suivie d'un prologue — un manuscrit parfaitement ordinaire — serait
    /// refusée au motif que le prologue « suit un chapitre » qui n'existe pas.
    #[test]
    fn deux_pieces_liminaires_se_suivent() {
        let p = decoupe(
            "## Préface\n\nA.\n\n## Prologue\n\nB.\n\n## 01 - Un\n\nC.\n",
            None,
        )
        .unwrap();
        assert_eq!(p[0].sorte, Sorte::Liminaire);
        assert_eq!(p[1].sorte, Sorte::Liminaire);
        assert_eq!(p[2].sorte, Sorte::Chapitre(1));
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
        assert_eq!(chapitres[0].sorte, Sorte::Chapitre(1));
        assert!(
            !chapitres[0].titre.is_empty(),
            "un chapitre sans titre : la conversion a mangé l'en-tête"
        );
    }

    /// Un titre libre est indiscernable d'un chapitre mal formé : la page de partie se
    /// marque donc explicitement, et son romain se vérifie comme le reste.
    #[test]
    fn une_page_de_partie_porte_un_romain_et_un_titre_libre() {
        let p = decoupe(
            "## Partie I - Avant Clément\n\n## 01 - Un\n\nA.\n\n\
             ## Partie II - Après Clément\n\n## 02 - Deux\n\nB.\n",
            None,
        )
        .unwrap();
        assert_eq!(p[0].sorte, Sorte::Partie("I".into()));
        assert_eq!(p[0].titre, "Avant Clément");
        assert_eq!(p[2].sorte, Sorte::Partie("II".into()));
    }

    /// Comme `## 7`, une partie peut n'avoir que son numéro.
    #[test]
    fn une_page_de_partie_sans_titre_est_admise() {
        let p = decoupe("## Partie I\n\n## 01 - Un\n\nA.\n", None).unwrap();
        assert_eq!(p[0].sorte, Sorte::Partie("I".into()));
        assert_eq!(p[0].titre, "");
    }

    /// Une partie sautée est une partie perdue en route, et elle ne se verrait qu'au
    /// tirage.
    #[test]
    fn un_romain_de_partie_qui_ne_suit_pas_est_refuse() {
        let md = "## Partie I - Un\n\n## 01 - Un\n\nA.\n\n## Partie IV - Quatre\n\n\
                  ## 02 - Deux\n\nB.\n";
        let err = decoupe(md, None).unwrap_err();
        // « Partie IV » est en ligne 7 dans ce manuscrit (comptage vérifié) : la ligne 5
        // du texte de la tâche était une coquille de comptage.
        assert!(err.contains("ligne 7"), "{err}");
        assert!(
            err.contains("II"),
            "l'erreur doit dire ce qui était attendu : {err}"
        );

        let err = decoupe("## Partie X - Dix\n\n## 01 - Un\n\nA.\n", None).unwrap_err();
        assert!(err.contains("ligne 1"), "{err}");
    }

    /// Une page de partie ne porte que son titre : un paragraphe écrit là serait
    /// silencieusement perdu à la composition, ce que le format refuse partout ailleurs.
    #[test]
    fn du_texte_sous_une_page_de_partie_est_refuse() {
        let err = decoupe("## Partie I - Un\n\nDu texte.\n\n## 01 - Un\n\nA.\n", None).unwrap_err();
        assert!(err.contains("ligne 3"), "{err}");
    }

    /// Le contrôle d'intégrité du projet dit un nombre de **chapitres** : une préface
    /// ajoutée au manuscrit ne doit pas faire croire à un chapitre de plus.
    #[test]
    fn le_controle_d_integrite_ne_compte_que_les_chapitres() {
        let md = "## Préface\n\nA.\n\n## Partie I - Un\n\n## 01 - Un\n\nB.\n\n\
                  ## 02 - Deux\n\nC.\n\n## Postface\n\nD.\n";
        let p = decoupe(md, Some(2)).expect("deux chapitres, pièces en plus");
        assert_eq!(p.len(), 5);
        assert_eq!(p.iter().filter(|p| p.est_chapitre()).count(), 2);
        assert!(
            decoupe(md, Some(5)).is_err(),
            "les pièces ne sont pas des chapitres"
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
