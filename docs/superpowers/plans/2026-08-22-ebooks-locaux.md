# Les ebooks locaux — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ajouter à Ozalid Studio une livraison locale — un PDF et un EPUB du livre entier, couverture comprise — générée d'un geste depuis l'étape Livraison.

**Architecture:** Le PDF réutilise `interieur::source` avec des marges symétriques et sans blanche de parité, précédé d'une page de couverture insérée. L'EPUB est bâti par un module `epub` entièrement pur — chapitres et octets en entrée, archive ZIP en sortie — qu'un module `ebook` orchestre en appelant Typst pour le PDF et pour le PNG de couverture. Un parseur d'emphase unique, extrait de `manuscrit`, sert les deux langages de sortie.

**Tech Stack:** Rust, Tauri 2, crate `zip` 7 (déjà présente), Typst en sidecar, front vanilla sans bundler, tests `cargo test` et `node --test`.

**Spec :** `docs/superpowers/specs/2026-08-22-ebooks-locaux-design.md`

---

## Avant de commencer

Le dépôt doit être propre et les vérifications passantes. Depuis `app/src-tauri/` :

```bash
cargo test --lib && cargo clippy --all-targets -- -D warnings && cargo fmt --check
```

Depuis `app/` :

```bash
node --test "tests/*.test.js"
```

Il faut aussi un Typst et des polices en place, sans quoi les lots 3 à 5 ne se vérifient pas :

```bash
app/outils/typst.sh --local
app/outils/polices.sh
```

**Relever la valeur du témoin maintenant**, c'est la référence de non-régression de tout le chantier :

```bash
cd app/src-tauri && cargo run --example temoin
```

Noter le compte de pages affiché. Deux tâches de ce plan touchent à la composition (tâches 1 et 9) et doivent le laisser identique.

**Convention de commit du dépôt :** message en français, à l'indicatif, une phrase qui dit ce que le livre ou l'outil sait faire de plus. Regarder `git log --oneline -10` avant le premier commit.

**Écart assumé avec la spec :** la spec découpe le chantier en quatre lots ; ce plan en
compte cinq. Le lot 1 — l'extraction du parseur d'emphase de `manuscrit` — n'y figurait
pas : il est apparu en écrivant le rendu XHTML, qui aurait sinon dupliqué la lecture des
astérisques. C'est un ajout de moyen, pas de périmètre, et il touche à la composition,
donc il passe le témoin comme le lot 3.

---

## Structure des fichiers

| Fichier | Responsabilité | Sort |
|---|---|---|
| `app/src-tauri/src/manuscrit.rs` | + `Morceau` et `morceaux()` : la coupure de l'emphase, une fois pour les deux langages | modifié |
| `app/src-tauri/src/epub.rs` | Chapitres + PNG + polices → une archive EPUB 3. Aucun disque, aucun Typst | **créé** |
| `app/src-tauri/src/ebook.rs` | Orchestration : source PDF, PNG de couverture, choix des polices, écriture | **créé** |
| `app/src-tauri/src/couverture.rs` | + `page_une()` : la 1ère sur une page insérable dans un autre document | modifié |
| `app/src-tauri/src/interieur.rs` | + `assemble()` interne et `source_ebook()` | modifié |
| `app/src-tauri/src/typst.rs` | + accesseur `polices()` | modifié |
| `app/src-tauri/src/commands.rs` | + commande `ebook_generer` | modifié |
| `app/src-tauri/src/lib.rs` | + deux `pub mod`, + une commande au `invoke_handler` | modifié |
| `app/src-tauri/examples/ebook.rs` | L'exercice sur livre réel, sans interface | **créé** |
| `app/src/index.html` | + le bloc « Ebooks » à l'étape Livraison | modifié |
| `app/src/livraison.js` | + `afficherEbooks()` et `ebooks()` | modifié |
| `app/src/app.js` | + l'écouteur du bouton, + l'oubli des sorties | modifié |
| `app/tests/ebook.test.js` | Câblage du bloc Ebooks | **créé** |
| `app/README.md` | + les deux modules au tableau, + l'exemple aux vérifications | modifié |

---

# Lot 1 — Un seul parseur d'emphase

L'intérieur rend `*mot*` en markup Typst, l'EPUB doit le rendre en `<em>`. Écrire deux
fois la lecture des astérisques, c'est se réserver le jour où l'un des deux traitera
une astérisque isolée autrement que l'autre. La coupure devient donc typée, et chaque
sortie ne fait plus que rendre.

### Task 1 : `manuscrit::morceaux`

**Files:**
- Modify: `app/src-tauri/src/manuscrit.rs` (fonction `inline`, lignes 90-117)
- Test: `app/src-tauri/src/manuscrit.rs` (module `tests` en fin de fichier)

- [ ] **Step 1 : écrire le test qui échoue**

À ajouter dans le `mod tests` de `manuscrit.rs` :

```rust
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
```

- [ ] **Step 2 : lancer le test pour le voir échouer**

```bash
cd app/src-tauri && cargo test --lib morceaux
```

Attendu : ÉCHEC à la compilation — `cannot find function 'morceaux'`, `cannot find type 'Morceau'`.

- [ ] **Step 3 : écrire l'implémentation**

Dans `manuscrit.rs`, **remplacer** la fonction `inline` (lignes 90-117) par ceci. La
fonction `ferme`, juste après, ne bouge pas.

```rust
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
```

- [ ] **Step 4 : lancer les tests pour les voir passer**

```bash
cd app/src-tauri && cargo test --lib manuscrit
```

Attendu : SUCCÈS, y compris tous les tests préexistants de `inline`. Ce sont eux qui
prouvent que le refactor est neutre.

- [ ] **Step 5 : le témoin, qui est le vrai juge**

```bash
cd app/src-tauri && cargo run --example temoin
```

Attendu : SUCCÈS, et le même compte de pages qu'au relevé initial. Un écart d'une
seule page signifie que `inline` ne rend plus exactement la même source : revenir au
step 3 plutôt que d'accepter le nouveau chiffre.

- [ ] **Step 6 : clippy et fmt**

```bash
cd app/src-tauri && cargo clippy --all-targets -- -D warnings && cargo fmt --check
```

Attendu : aucune sortie.

- [ ] **Step 7 : commit**

```bash
cd /Users/jean-paulgavini/Documents/Dev/ozalid
git add app/src-tauri/src/manuscrit.rs
git commit -m "Les astérisques du manuscrit ne se lisent plus qu'une fois"
```

---

# Lot 2 — Le module `epub`, sans rien autour

Chapitres, PNG et fichiers de police en entrée ; une archive en sortie. Ni disque, ni
Typst, ni prestataire : tout ce lot se vérifie par des tests unitaires.

### Task 2 : le squelette du module et l'échappement XML

**Files:**
- Create: `app/src-tauri/src/epub.rs`
- Modify: `app/src-tauri/src/lib.rs:1-19`

- [ ] **Step 1 : écrire le test qui échoue**

Créer `app/src-tauri/src/epub.rs` avec **seulement** l'entête et le module de tests :

```rust
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
```

Puis déclarer le module dans `lib.rs`, en respectant l'ordre alphabétique de la liste
existante — entre `envoi` et `epreuve`. `ebook` viendra à la tâche 11 ; ne pas l'ajouter
maintenant, il n'existe pas encore et `lib.rs` ne compilerait plus.

```rust
pub mod envoi;
pub mod epub;
pub mod epreuve;
```

- [ ] **Step 2 : lancer le test pour le voir échouer**

```bash
cd app/src-tauri && cargo test --lib epub
```

Attendu : ÉCHEC à la compilation — `cannot find function 'echappe' in this scope`.

- [ ] **Step 3 : écrire l'implémentation**

Insérer dans `epub.rs`, avant le `mod tests` :

```rust
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
```

- [ ] **Step 4 : lancer le test pour le voir passer**

```bash
cd app/src-tauri && cargo test --lib epub
```

Attendu : SUCCÈS, 2 tests.

- [ ] **Step 5 : commit**

```bash
cd /Users/jean-paulgavini/Documents/Dev/ozalid
git add app/src-tauri/src/epub.rs app/src-tauri/src/lib.rs
git commit -m "Le XML a désormais son propre échappement"
```

---

### Task 3 : le paragraphe et le chapitre en XHTML

**Files:**
- Modify: `app/src-tauri/src/epub.rs`

- [ ] **Step 1 : écrire les tests qui échouent**

À ajouter dans le `mod tests` de `epub.rs`. Aucun `use` supplémentaire : `Bloc` et
`Chapitre` viennent du `use super::*` déjà en tête du module de tests, une fois que le
step 3 les aura importés au niveau du module.

```rust
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
    let ch = Chapitre { numero: 1, titre: String::new(), blocs: vec![] };
    let x = chapitre_xhtml(&ch);
    assert!(!x.contains(r#"class="titre""#), "{x}");
    assert!(x.contains(r#"<span class="numero">1</span>"#), "{x}");
}

/// Un titre de chapitre contenant une esperluette casserait l'archive s'il n'était pas
/// échappé — et le manuscrit en admet une, c'est du texte ordinaire.
#[test]
fn un_titre_de_chapitre_est_echappe() {
    let ch = Chapitre { numero: 3, titre: "Pile & face".into(), blocs: vec![] };
    assert!(chapitre_xhtml(&ch).contains("Pile &amp; face"));
}
```

- [ ] **Step 2 : lancer les tests pour les voir échouer**

```bash
cd app/src-tauri && cargo test --lib epub
```

Attendu : ÉCHEC à la compilation — `cannot find function 'paragraphe'`,
`cannot find value 'SCENE_XHTML'`, `cannot find function 'chapitre_xhtml'`.

- [ ] **Step 3 : écrire l'implémentation**

Insérer dans `epub.rs`, avant le `mod tests` :

```rust
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
```

Le `self` de l'import est ce qui rend `manuscrit::morceaux(s)` appelable sous ce nom-là,
choisi pour que le partage avec l'intérieur se lise à l'endroit du code où il compte.

- [ ] **Step 4 : lancer les tests pour les voir passer**

```bash
cd app/src-tauri && cargo test --lib epub
```

Attendu : SUCCÈS, 7 tests.

- [ ] **Step 5 : commit**

```bash
cd /Users/jean-paulgavini/Documents/Dev/ozalid
git add app/src-tauri/src/epub.rs
git commit -m "Un chapitre sait se dire en XHTML"
```

---

### Task 4 : le choix des faces de police

**Files:**
- Modify: `app/src-tauri/src/epub.rs`

- [ ] **Step 1 : écrire les tests qui échouent**

À ajouter dans le `mod tests` de `epub.rs` :

```rust
/// Les noms de fichiers réellement posés par `app/outils/polices.sh`, groupés par
/// famille. C'est le seul endroit du projet où ils soient écrits : `fonts/` n'est pas
/// versionné, la règle de choix doit donc s'éprouver sur une liste, pas sur un
/// répertoire.
fn fichiers(famille: &str) -> Vec<String> {
    let l: &[&str] = match famille {
        "EB Garamond" => &["EBGaramond[wght].ttf", "EBGaramond-Italic[wght].ttf"],
        "Crimson Pro" => &["CrimsonPro[wght].ttf", "CrimsonPro-Italic[wght].ttf"],
        "Alegreya" => &["Alegreya[wght].ttf", "Alegreya-Italic[wght].ttf"],
        "Cardo" => &["Cardo-Regular.ttf", "Cardo-Bold.ttf", "Cardo-Italic.ttf"],
        "Vollkorn" => &["Vollkorn[wght].ttf", "Vollkorn-Italic[wght].ttf"],
        "Spectral" => &[
            "Spectral-Regular.ttf", "Spectral-Italic.ttf",
            "Spectral-Bold.ttf", "Spectral-BoldItalic.ttf",
            "Spectral-SemiBold.ttf", "Spectral-SemiBoldItalic.ttf",
        ],
        "Libre Baskerville" => &[
            "LibreBaskerville[wght].ttf", "LibreBaskerville-Italic[wght].ttf",
        ],
        _ => &[],
    };
    l.iter().map(|s| s.to_string()).collect()
}

/// Chacune des sept polices de labeur doit donner un romain et un italique. Une
/// famille qui n'en donnerait pas composerait l'EPUB dans l'écriture du lecteur sans
/// que rien d'autre ne le dise.
#[test]
fn les_sept_polices_de_labeur_donnent_un_romain_et_un_italique() {
    for famille in crate::interieur::POLICES_TEXTE {
        let f = faces(&fichiers(famille)).unwrap_or_else(|| panic!("{famille} : aucune face"));
        assert!(!f.romain.contains("Italic"), "{famille} : {}", f.romain);
        assert!(f.italique.is_some(), "{famille} : pas d'italique");
    }
}

/// Le piège de la règle : Cardo livre son romain sous « Cardo-Regular.ttf », plus long
/// que « Cardo-Bold.ttf ». Choisir le nom le plus court sans écarter le gras
/// composerait tout le livre en gras — et cela ne se verrait qu'à la lecture.
#[test]
fn le_gras_n_est_jamais_pris_pour_le_romain() {
    let f = faces(&fichiers("Cardo")).unwrap();
    assert_eq!(f.romain, "Cardo-Regular.ttf");
    assert_eq!(f.italique.as_deref(), Some("Cardo-Italic.ttf"));
}

/// Même piège du côté de l'italique : Spectral porte quatre fichiers en « Italic »,
/// dont deux gras.
#[test]
fn l_italique_gras_n_est_jamais_pris_pour_l_italique() {
    let f = faces(&fichiers("Spectral")).unwrap();
    assert_eq!(f.romain, "Spectral-Regular.ttf");
    assert_eq!(f.italique.as_deref(), Some("Spectral-Italic.ttf"));
}

/// Aucun fichier, aucune face : c'est le cas « police introuvable dans `fonts/` », qui
/// n'est pas une erreur — l'EPUB se fait alors dans l'écriture du lecteur.
#[test]
fn sans_fichier_il_n_y_a_pas_de_face() {
    assert!(faces(&[]).is_none());
}
```

- [ ] **Step 2 : lancer les tests pour les voir échouer**

```bash
cd app/src-tauri && cargo test --lib epub
```

Attendu : ÉCHEC à la compilation — `cannot find function 'faces'`, `cannot find type 'Faces'`.

- [ ] **Step 3 : écrire l'implémentation**

Insérer dans `epub.rs`, avant le `mod tests` :

```rust
/// Les deux fichiers d'une famille que l'EPUB déclare.
#[derive(Debug, Clone, PartialEq)]
pub struct Faces {
    pub romain: String,
    pub italique: Option<String>,
}

/// Le romain et l'italique parmi les fichiers d'une **même** famille.
///
/// Tout nom portant « Bold » est écarté d'abord : cela couvre `-Bold`, `-BoldItalic`,
/// `-SemiBold` et `-SemiBoldItalic` d'un seul coup. Sans cette exclusion, Cardo
/// donnerait « Cardo-Bold.ttf » pour romain — son fichier ordinaire s'appelle
/// « Cardo-Regular.ttf », plus long — et le livre entier sortirait en gras.
///
/// Le gras n'est pas embarqué : sur un fichier variable l'axe `wght` le rend, sur un
/// fichier statique la liseuse le synthétise. C'est le comportement d'un EPUB
/// ordinaire, et `**mot**` reste rare dans un roman.
pub fn faces(noms: &[String]) -> Option<Faces> {
    let choisir = |italique: bool| -> Option<String> {
        noms.iter()
            .filter(|n| !n.contains("Bold"))
            .filter(|n| n.contains("Italic") == italique)
            .min_by_key(|n| n.len())
            .cloned()
    };
    Some(Faces {
        romain: choisir(false)?,
        italique: choisir(true),
    })
}
```

- [ ] **Step 4 : lancer les tests pour les voir passer**

```bash
cd app/src-tauri && cargo test --lib epub
```

Attendu : SUCCÈS, 12 tests.

- [ ] **Step 5 : commit**

```bash
cd /Users/jean-paulgavini/Documents/Dev/ozalid
git add app/src-tauri/src/epub.rs
git commit -m "L'EPUB sait laquelle des faces est le romain"
```

---

### Task 5 : l'horodatage qu'EPUB 3 exige

**Files:**
- Modify: `app/src-tauri/src/epub.rs`

- [ ] **Step 1 : écrire les tests qui échouent**

À ajouter dans le `mod tests` de `epub.rs` :

```rust
use std::time::Duration;

/// EPUB 3 exige un `dcterms:modified` en ISO 8601 UTC à la seconde. Les trois valeurs
/// ci-dessous ont été relevées avec `date -u -r <secondes>` : l'époque, une date
/// quelconque, et le 29 février d'une année bissextile — le seul cas où un calcul de
/// calendrier écrit à la main se trompe sans qu'on s'en aperçoive.
#[test]
fn l_horodatage_suit_le_calendrier_annees_bissextiles_comprises() {
    let t = |s: u64| SystemTime::UNIX_EPOCH + Duration::from_secs(s);
    assert_eq!(horodatage(t(0)), "1970-01-01T00:00:00Z");
    assert_eq!(horodatage(t(1_755_000_000)), "2025-08-12T12:00:00Z");
    assert_eq!(horodatage(t(1_709_164_800)), "2024-02-29T00:00:00Z");
}
```

- [ ] **Step 2 : lancer le test pour le voir échouer**

```bash
cd app/src-tauri && cargo test --lib horodatage
```

Attendu : ÉCHEC à la compilation — `cannot find function 'horodatage'`.

- [ ] **Step 3 : écrire l'implémentation**

Insérer dans `epub.rs` :

```rust
use std::time::SystemTime;

/// `SystemTime` → date ISO 8601 en UTC, à la seconde, telle qu'EPUB 3 l'exige pour
/// `dcterms:modified`.
///
/// Écrit à la main plutôt que tiré d'une crate : c'est le seul endroit du projet qui
/// ait besoin d'une date, et l'algorithme tient en dix lignes. Une horloge d'avant
/// 1970 rendrait l'époque — un EPUB daté de 1970 se voit, un `unwrap` sur une machine
/// mal réglée ferait perdre un livre.
pub fn horodatage(t: SystemTime) -> String {
    let s = t
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0) as i64;
    let (a, m, j) = civil(s.div_euclid(86_400));
    let reste = s.rem_euclid(86_400);
    format!(
        "{a:04}-{m:02}-{j:02}T{:02}:{:02}:{:02}Z",
        reste / 3600,
        (reste % 3600) / 60,
        reste % 60
    )
}

/// Jours depuis 1970-01-01 → (année, mois, jour), par l'algorithme de Howard Hinnant.
/// Il place mars en tête d'année, ce qui range le 29 février en fin de cycle et évite
/// tout cas particulier de bissextile.
fn civil(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let ere = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let an = yoe + ere * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let jour = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let mois = (if mp < 10 { mp + 3 } else { mp - 9 }) as u32;
    (if mois <= 2 { an + 1 } else { an }, mois, jour)
}
```

- [ ] **Step 4 : lancer le test pour le voir passer**

```bash
cd app/src-tauri && cargo test --lib epub
```

Attendu : SUCCÈS, 13 tests.

- [ ] **Step 5 : commit**

```bash
cd /Users/jean-paulgavini/Documents/Dev/ozalid
git add app/src-tauri/src/epub.rs
git commit -m "L'archive sait dire le jour où elle a été faite"
```

---

### Task 6 : l'inventaire de l'archive

Le manifeste de l'OPF et le contenu du ZIP doivent se recouvrir exactement. Les faire
découler d'une **seule** liste est ce qui le rend vrai ; le test de la tâche 8 vérifie
que la liste est bien la seule source des deux.

**Files:**
- Modify: `app/src-tauri/src/epub.rs`

- [ ] **Step 1 : écrire les tests qui échouent**

À ajouter dans le `mod tests` de `epub.rs` :

```rust
fn livre_temoin() -> Livre<'static> {
    Livre {
        titre: "Les Heures creuses",
        auteur: "Ivan Pjig",
        genre: "roman",
        copyright: "© 2026 Ivan Pjig\nTous droits réservés",
        dedicace: Some("À R."),
    }
}

fn chapitres_temoins() -> Vec<Chapitre> {
    vec![
        Chapitre { numero: 1, titre: "Le seuil".into(),
                   blocs: vec![Bloc::Paragraphe("Premier.".into())] },
        Chapitre { numero: 2, titre: String::new(),
                   blocs: vec![Bloc::Paragraphe("Second.".into())] },
    ]
}

/// L'inventaire porte, dans l'ordre, la couverture, les liminaires puis un fichier par
/// chapitre. C'est cet ordre qui devient celui de la lecture.
#[test]
fn l_inventaire_ouvre_sur_la_couverture_et_suit_les_chapitres() {
    let ch = chapitres_temoins();
    let e = contenu(&livre_temoin(), &ch, b"\x89PNG", None);
    let lisibles: Vec<&str> = e.iter().filter(|x| x.spine).map(|x| x.nom.as_str()).collect();
    assert_eq!(
        lisibles,
        vec!["couverture.xhtml", "liminaires.xhtml", "ch001.xhtml", "ch002.xhtml"]
    );
}

/// Le `nav` est un document XHTML, mais il n'est pas une page du livre : le laisser
/// dans le fil de lecture ferait tourner une table des matières entre la couverture et
/// le premier chapitre.
#[test]
fn la_table_des_matieres_n_est_pas_une_page_du_livre() {
    let ch = chapitres_temoins();
    let e = contenu(&livre_temoin(), &ch, b"\x89PNG", None);
    let nav = e.iter().find(|x| x.nom == "nav.xhtml").expect("pas de nav");
    assert!(!nav.spine);
    assert_eq!(nav.proprietes, Some("nav"));
}

/// Le PNG de couverture est **stocké** tel quel : il est déjà compressé, et le
/// repasser en deflate ne gagne rien pour un livre qui pèse déjà quelques mégaoctets.
#[test]
fn la_couverture_entre_dans_l_archive_sans_etre_recompressee() {
    let ch = chapitres_temoins();
    let e = contenu(&livre_temoin(), &ch, b"\x89PNG", None);
    let img = e.iter().find(|x| x.nom == "images/couverture.png").expect("pas d'image");
    assert!(!img.compresse);
    assert_eq!(img.media, "image/png");
    assert_eq!(img.proprietes, Some("cover-image"));
    assert_eq!(img.octets, b"\x89PNG");
}

/// Sans police embarquée, l'inventaire n'en porte aucune et le CSS retombe sur
/// `serif`. Ce n'est pas une erreur : le livre reste juste, seul son œil change.
#[test]
fn sans_police_l_inventaire_n_en_porte_aucune() {
    let ch = chapitres_temoins();
    let e = contenu(&livre_temoin(), &ch, b"\x89PNG", None);
    assert!(!e.iter().any(|x| x.nom.starts_with("fonts/")));
    let css = e.iter().find(|x| x.nom == "style.css").unwrap();
    let css = String::from_utf8(css.octets.clone()).unwrap();
    assert!(!css.contains("@font-face"), "{css}");
    assert!(css.contains("serif"), "{css}");
}

/// Avec une police, les deux faces entrent dans l'archive et le CSS les déclare.
#[test]
fn les_deux_faces_entrent_dans_l_archive_et_le_css_les_declare() {
    let ch = chapitres_temoins();
    let p = Polices {
        famille: "Cardo".into(),
        romain: Face { nom: "Cardo-Regular.ttf".into(), octets: b"R".to_vec() },
        italique: Some(Face { nom: "Cardo-Italic.ttf".into(), octets: b"I".to_vec() }),
    };
    let e = contenu(&livre_temoin(), &ch, b"\x89PNG", Some(&p));
    assert!(e.iter().any(|x| x.nom == "fonts/Cardo-Regular.ttf"));
    assert!(e.iter().any(|x| x.nom == "fonts/Cardo-Italic.ttf"));
    let css = e.iter().find(|x| x.nom == "style.css").unwrap();
    let css = String::from_utf8(css.octets.clone()).unwrap();
    assert_eq!(css.matches("@font-face").count(), 2, "{css}");
    assert!(css.contains("font-style: italic"), "{css}");
    assert!(css.contains(r#"url("fonts/Cardo-Regular.ttf")"#), "{css}");
}

/// La dédicace ne paraît que si le livre en porte une : une page vide se verrait.
#[test]
fn la_dedicace_ne_parait_que_si_le_livre_en_porte_une() {
    let ch = chapitres_temoins();
    let avec = contenu(&livre_temoin(), &ch, b"\x89PNG", None);
    let lim = |e: &[Entree]| {
        let x = e.iter().find(|x| x.nom == "liminaires.xhtml").unwrap();
        String::from_utf8(x.octets.clone()).unwrap()
    };
    assert!(lim(&avec).contains("À R."));

    let mut l = livre_temoin();
    l.dedicace = None;
    let sans = contenu(&l, &ch, b"\x89PNG", None);
    assert!(!lim(&sans).contains("dedicace"), "{}", lim(&sans));
}
```

- [ ] **Step 2 : lancer les tests pour les voir échouer**

```bash
cd app/src-tauri && cargo test --lib epub
```

Attendu : ÉCHEC à la compilation — `cannot find type 'Livre'`, `'Entree'`, `'Polices'`,
`'Face'`, `cannot find function 'contenu'`.

- [ ] **Step 3 : écrire l'implémentation**

Insérer dans `epub.rs`, avant le `mod tests` :

```rust
/// Ce qu'un EPUB porte du livre. Les emprunts évitent de recopier le projet pour le
/// traverser ; ce module ne garde rien.
#[derive(Debug, Clone)]
pub struct Livre<'a> {
    pub titre: &'a str,
    pub auteur: &'a str,
    pub genre: &'a str,
    pub copyright: &'a str,
    pub dedicace: Option<&'a str>,
}

/// Un fichier de police, prêt à entrer dans l'archive.
#[derive(Debug, Clone, PartialEq)]
pub struct Face {
    /// Nom du fichier, sans répertoire : il devient `OEBPS/fonts/<nom>`.
    pub nom: String,
    pub octets: Vec<u8>,
}

/// L'écriture du livre, telle que l'EPUB l'embarque.
#[derive(Debug, Clone, PartialEq)]
pub struct Polices {
    /// La famille, telle que le CSS la nommera.
    pub famille: String,
    pub romain: Face,
    pub italique: Option<Face>,
}

/// Une entrée de l'archive, sous `OEBPS/`.
///
/// C'est la **seule** liste des fichiers du livre : le manifeste de l'OPF en découle,
/// le contenu du ZIP aussi. Un fichier qu'on ajouterait à l'un sans l'autre ferait
/// rejeter l'archive par une liseuse stricte, et rien d'autre ne le dirait.
#[derive(Debug, Clone, PartialEq)]
pub struct Entree {
    pub nom: String,
    pub octets: Vec<u8>,
    pub media: &'static str,
    /// `properties` de l'OPF : « nav » pour la table des matières, « cover-image »
    /// pour la couverture. Ce sont les deux seules dont EPUB 3 ait besoin.
    pub proprietes: Option<&'static str>,
    /// Vrai si l'entrée est une page du fil de lecture.
    pub spine: bool,
    /// Faux pour ce qui est déjà compressé — le PNG, les polices.
    pub compresse: bool,
}

impl Entree {
    fn xhtml(nom: &str, corps: String, spine: bool, proprietes: Option<&'static str>) -> Self {
        Self {
            nom: nom.into(),
            octets: corps.into_bytes(),
            media: "application/xhtml+xml",
            proprietes,
            spine,
            compresse: true,
        }
    }
}

/// Nom de fichier d'un chapitre. Trois chiffres : un roman dépasse rarement 999
/// chapitres, et l'ordre alphabétique des noms reste celui de la lecture.
fn nom_chapitre(rang: usize) -> String {
    format!("ch{:03}.xhtml", rang + 1)
}

/// Tout ce que l'archive porte sous `OEBPS/`, sauf `content.opf` — qui décrit cette
/// liste et ne peut donc pas s'y décrire lui-même.
fn contenu(
    livre: &Livre,
    chapitres: &[Chapitre],
    couverture_png: &[u8],
    polices: Option<&Polices>,
) -> Vec<Entree> {
    let mut e = vec![
        Entree::xhtml("couverture.xhtml", couverture_xhtml(), true, None),
        Entree::xhtml("liminaires.xhtml", liminaires_xhtml(livre), true, None),
    ];
    for (i, ch) in chapitres.iter().enumerate() {
        e.push(Entree::xhtml(&nom_chapitre(i), chapitre_xhtml(ch), true, None));
    }
    e.push(Entree::xhtml(
        "nav.xhtml",
        nav_xhtml(chapitres),
        false,
        Some("nav"),
    ));
    e.push(Entree {
        nom: "toc.ncx".into(),
        octets: ncx(livre, chapitres).into_bytes(),
        media: "application/x-dtbncx+xml",
        proprietes: None,
        spine: false,
        compresse: true,
    });
    e.push(Entree {
        nom: "style.css".into(),
        octets: css(polices).into_bytes(),
        media: "text/css",
        proprietes: None,
        spine: false,
        compresse: true,
    });
    e.push(Entree {
        nom: "images/couverture.png".into(),
        octets: couverture_png.to_vec(),
        media: "image/png",
        proprietes: Some("cover-image"),
        spine: false,
        // Un PNG est déjà compressé : le repasser en deflate ne gagne rien.
        compresse: false,
    });
    if let Some(p) = polices {
        for f in std::iter::once(&p.romain).chain(p.italique.iter()) {
            e.push(Entree {
                nom: format!("fonts/{}", f.nom),
                octets: f.octets.clone(),
                media: "font/ttf",
                proprietes: None,
                spine: false,
                compresse: false,
            });
        }
    }
    e
}

fn couverture_xhtml() -> String {
    page(
        "Couverture",
        "<div class=\"couverture\"><img src=\"images/couverture.png\" alt=\"Couverture\"/></div>\n",
    )
}

/// La page de titre, le copyright et — quand le livre en porte une — la dédicace.
///
/// Le faux-titre et les blanches du papier ne passent pas : ils n'ont de sens que sur
/// une feuille pliée. Le reste est du livre.
fn liminaires_xhtml(livre: &Livre) -> String {
    let mut c = format!(
        "<div class=\"titre-page\">\n\
         <p class=\"auteur\">{}</p>\n\
         <h1 class=\"grand-titre\">{}</h1>\n\
         <p class=\"genre\">{}</p>\n\
         </div>\n",
        echappe(livre.auteur),
        echappe(livre.titre),
        echappe(livre.genre),
    );
    c.push_str(&format!(
        "<div class=\"copyright\">{}</div>\n",
        lignes(livre.copyright)
    ));
    if let Some(d) = livre.dedicace {
        c.push_str(&format!("<div class=\"dedicace\">{}</div>\n", lignes(d)));
    }
    page("Titre", &c)
}

/// Texte à sauts de ligne → paragraphes XHTML. Les lignes vides sont écartées : elles
/// espaçaient un pavé Typst, le CSS s'en charge ici.
fn lignes(s: &str) -> String {
    s.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(|l| format!("<p>{}</p>", echappe(l)))
        .collect()
}

fn nav_xhtml(chapitres: &[Chapitre]) -> String {
    let mut l = String::new();
    for (i, ch) in chapitres.iter().enumerate() {
        l.push_str(&format!(
            "<li><a href=\"{}\">{}</a></li>\n",
            nom_chapitre(i),
            echappe(&intitule(ch))
        ));
    }
    page(
        "Table des matières",
        &format!(
            "<nav epub:type=\"toc\" id=\"toc\">\n\
             <h1>Table des matières</h1>\n\
             <ol>\n{l}</ol>\n\
             </nav>\n"
        ),
    )
}

/// La même table, au format des liseuses antérieures à EPUB 3. Elle ne coûte que
/// quelques centaines d'octets et évite un sommaire vide sur les appareils anciens.
fn ncx(livre: &Livre, chapitres: &[Chapitre]) -> String {
    let mut points = String::new();
    for (i, ch) in chapitres.iter().enumerate() {
        points.push_str(&format!(
            "<navPoint id=\"nav{n}\" playOrder=\"{n}\">\
             <navLabel><text>{}</text></navLabel>\
             <content src=\"{}\"/></navPoint>\n",
            echappe(&intitule(ch)),
            nom_chapitre(i),
            n = i + 1
        ));
    }
    format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<ncx xmlns="http://www.daisy.org/z3986/2005/ncx/" version="2005-1">
<head><meta name="dtb:uid" content="{}"/></head>
<docTitle><text>{}</text></docTitle>
<navMap>
{points}</navMap>
</ncx>
"#,
        echappe(&identifiant(livre)),
        echappe(livre.titre)
    )
}

/// Le CSS du livre. Court à dessein : ce qui n'est pas dit reste au réglage du lecteur,
/// et c'est ce qu'on attend d'un EPUB.
fn css(polices: Option<&Polices>) -> String {
    let mut s = String::new();
    let famille = match polices {
        Some(p) => {
            for (f, style) in std::iter::once((&p.romain, "normal"))
                .chain(p.italique.iter().map(|i| (i, "italic")))
            {
                s.push_str(&format!(
                    "@font-face {{\n  font-family: \"{}\";\n  font-style: {style};\n  \
                     font-weight: 100 900;\n  src: url(\"fonts/{}\");\n}}\n",
                    p.famille, f.nom
                ));
            }
            format!("\"{}\", serif", p.famille)
        }
        None => "serif".into(),
    };
    s.push_str(&format!(
        r#"
body {{ font-family: {famille}; margin: 0 5%; line-height: 1.45;
       text-align: justify; hyphens: auto; -webkit-hyphens: auto; }}
p {{ margin: 0; text-indent: 1.2em; }}
/* Le premier paragraphe d'un chapitre n'a pas d'alinéa — comme sur le papier, où
   Typst n'indente pas le paragraphe qui ouvre un bloc. Après une rupture de scène,
   en revanche, l'alinéa revient : c'est ce qui a été relevé sur la page composée. */
h1 + p {{ text-indent: 0; }}
h1 {{ margin: 2.5em 0 2em; text-align: center; font-weight: normal; }}
h1 .numero {{ display: block; font-size: 1.2em; }}
h1 .titre {{ display: block; margin-top: 0.6em; font-size: 0.85em;
             letter-spacing: 0.14em; text-transform: uppercase; }}
p.scene {{ text-align: center; text-indent: 0; margin: 1em 0; word-spacing: 0.5em; }}
.couverture {{ margin: 0; text-align: center; }}
.couverture img {{ max-width: 100%; }}
.titre-page {{ margin-top: 25%; text-align: center; }}
.titre-page p, .titre-page h1 {{ text-indent: 0; }}
.grand-titre {{ font-size: 1.6em; font-weight: normal; letter-spacing: 0.06em; }}
.genre {{ font-style: italic; }}
.copyright {{ margin-top: 40%; font-size: 0.8em; text-align: center; }}
.copyright p {{ text-indent: 0; }}
.dedicace {{ margin-top: 25%; font-style: italic; text-align: center; }}
.dedicace p {{ text-indent: 0; }}
"#
    ));
    s
}

/// L'identifiant unique du livre.
///
/// Tiré du titre et de l'auteur, non d'un tirage au sort : deux générations du même
/// livre doivent porter le même identifiant, sans quoi une liseuse y verrait deux
/// ouvrages et garderait les deux. `envoi::assaini` est déjà la fonction du projet qui
/// décide ce qu'un titre devient quand il sert de nom.
fn identifiant(livre: &Livre) -> String {
    format!(
        "urn:ozalid:{}-{}",
        crate::envoi::assaini(livre.titre),
        crate::envoi::assaini(livre.auteur)
    )
}
```

- [ ] **Step 4 : lancer les tests pour les voir passer**

```bash
cd app/src-tauri && cargo test --lib epub
```

Attendu : SUCCÈS, 19 tests.

- [ ] **Step 5 : clippy et fmt**

```bash
cd app/src-tauri && cargo clippy --all-targets -- -D warnings && cargo fmt --check
```

Attendu : aucune sortie.

- [ ] **Step 6 : commit**

```bash
cd /Users/jean-paulgavini/Documents/Dev/ozalid
git add app/src-tauri/src/epub.rs
git commit -m "L'archive tient l'inventaire de ce qu'elle porte"
```

---

### Task 7 : le manifeste OPF

**Files:**
- Modify: `app/src-tauri/src/epub.rs`

- [ ] **Step 1 : écrire les tests qui échouent**

À ajouter dans le `mod tests` de `epub.rs` :

```rust
/// Le manifeste porte exactement les entrées de l'inventaire, et le fil de lecture ne
/// renvoie qu'à des `id` du manifeste. Un `idref` orphelin fait rejeter l'archive.
#[test]
fn le_manifeste_porte_l_inventaire_et_le_fil_n_y_renvoie_que_des_id() {
    let ch = chapitres_temoins();
    let e = contenu(&livre_temoin(), &ch, b"\x89PNG", None);
    let x = opf(&livre_temoin(), &e, "2026-08-22T10:00:00Z");
    for entree in &e {
        assert!(
            x.contains(&format!("href=\"{}\"", entree.nom)),
            "{} n'est pas manifesté : {x}",
            entree.nom
        );
    }
    for entree in e.iter().filter(|x| x.spine) {
        assert!(
            x.contains(&format!("idref=\"{}\"", id_de(&entree.nom))),
            "{} n'est pas dans le fil : {x}",
            entree.nom
        );
    }
    // Le fil ne porte que des pages : ni le CSS, ni le PNG, ni le `nav`.
    assert!(!x.contains(&format!("idref=\"{}\"", id_de("style.css"))), "{x}");
    assert!(!x.contains(&format!("idref=\"{}\"", id_de("nav.xhtml"))), "{x}");
}

/// Les métadonnées qu'EPUB 3 exige, plus celle que les liseuses anciennes lisent pour
/// afficher une vignette.
#[test]
fn les_metadonnees_disent_le_livre_et_sa_couverture() {
    let ch = chapitres_temoins();
    let e = contenu(&livre_temoin(), &ch, b"\x89PNG", None);
    let x = opf(&livre_temoin(), &e, "2026-08-22T10:00:00Z");
    assert!(x.contains("<dc:title>Les Heures creuses</dc:title>"), "{x}");
    assert!(x.contains("<dc:creator>Ivan Pjig</dc:creator>"), "{x}");
    assert!(x.contains("<dc:language>fr</dc:language>"), "{x}");
    assert!(x.contains("urn:ozalid:Les-Heures-creuses-Ivan-Pjig"), "{x}");
    assert!(
        x.contains(r#"<meta property="dcterms:modified">2026-08-22T10:00:00Z</meta>"#),
        "{x}"
    );
    // La vignette, deux fois : `properties` pour EPUB 3, `meta name` pour le reste.
    assert!(x.contains(r#"properties="cover-image""#), "{x}");
    assert!(
        x.contains(&format!(
            r#"<meta name="cover" content="{}"/>"#,
            id_de("images/couverture.png")
        )),
        "{x}"
    );
}

/// Un `id` XML ne peut ni commencer par un chiffre ni porter de barre oblique ou de
/// point. Un nom de fichier, si.
#[test]
fn un_nom_de_fichier_devient_un_id_xml_valide() {
    assert_eq!(id_de("images/couverture.png"), "f-images-couverture-png");
    assert_eq!(id_de("ch001.xhtml"), "f-ch001-xhtml");
    assert_eq!(id_de("fonts/Cardo-Regular.ttf"), "f-fonts-Cardo-Regular-ttf");
}
```

- [ ] **Step 2 : lancer les tests pour les voir échouer**

```bash
cd app/src-tauri && cargo test --lib epub
```

Attendu : ÉCHEC à la compilation — `cannot find function 'opf'`, `cannot find function 'id_de'`.

- [ ] **Step 3 : écrire l'implémentation**

Insérer dans `epub.rs`, avant le `mod tests` :

```rust
/// Nom de fichier → `id` XML.
///
/// Un `id` ne peut ni commencer par un chiffre ni porter de barre oblique ou de point ;
/// un nom de fichier peut les trois. Le préfixe règle le chiffre, la substitution
/// règle le reste, et la bijection tient parce que deux entrées ne portent jamais le
/// même nom.
fn id_de(nom: &str) -> String {
    let corps: String = nom
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    format!("f-{corps}")
}

/// Le manifeste et le fil de lecture, dérivés de l'inventaire.
///
/// `content.opf` ne se manifeste pas lui-même : c'est lui qui décrit les autres, et la
/// spec de l'EPUB le désigne par `META-INF/container.xml`.
fn opf(livre: &Livre, entrees: &[Entree], modifie: &str) -> String {
    let mut manifeste = String::new();
    for e in entrees {
        let props = match e.proprietes {
            Some(p) => format!(" properties=\"{p}\""),
            None => String::new(),
        };
        manifeste.push_str(&format!(
            "<item id=\"{}\" href=\"{}\" media-type=\"{}\"{props}/>\n",
            id_de(&e.nom),
            e.nom,
            e.media
        ));
    }
    let mut fil = String::new();
    for e in entrees.iter().filter(|e| e.spine) {
        fil.push_str(&format!("<itemref idref=\"{}\"/>\n", id_de(&e.nom)));
    }
    format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<package xmlns="http://www.idpf.org/2007/opf" version="3.0" unique-identifier="pub-id" xml:lang="fr">
<metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
<dc:identifier id="pub-id">{ident}</dc:identifier>
<dc:title>{titre}</dc:title>
<dc:creator>{auteur}</dc:creator>
<dc:language>fr</dc:language>
<dc:rights>{droits}</dc:rights>
<meta property="dcterms:modified">{modifie}</meta>
<meta name="cover" content="{cover}"/>
</metadata>
<manifest>
{manifeste}</manifest>
<spine toc="{ncx}">
{fil}</spine>
</package>
"#,
        ident = echappe(&identifiant(livre)),
        titre = echappe(livre.titre),
        auteur = echappe(livre.auteur),
        droits = echappe(&livre.copyright.replace('\n', " ")),
        cover = id_de("images/couverture.png"),
        ncx = id_de("toc.ncx"),
    )
}
```

- [ ] **Step 4 : lancer les tests pour les voir passer**

```bash
cd app/src-tauri && cargo test --lib epub
```

Attendu : SUCCÈS, 22 tests.

- [ ] **Step 5 : commit**

```bash
cd /Users/jean-paulgavini/Documents/Dev/ozalid
git add app/src-tauri/src/epub.rs
git commit -m "Le manifeste de l'archive découle de son inventaire"
```

---

### Task 8 : l'archive, et la relecture qui la valide

**Files:**
- Modify: `app/src-tauri/src/epub.rs`

- [ ] **Step 1 : écrire les tests qui échouent**

À ajouter dans le `mod tests` de `epub.rs` :

```rust
use std::io::{Cursor, Read};

fn relire(octets: &[u8]) -> zip::ZipArchive<Cursor<Vec<u8>>> {
    zip::ZipArchive::new(Cursor::new(octets.to_vec())).expect("archive illisible")
}

/// `mimetype` doit être la **première** entrée et n'être pas compressée : c'est la
/// seule dont la spec de l'EPUB fixe la place et la méthode, et une liseuse stricte
/// refuse l'archive sinon. Le défaut est invisible tant qu'on n'ouvre le fichier que
/// dans Calibre, indulgent.
#[test]
fn le_mimetype_ouvre_l_archive_et_n_est_pas_compresse() {
    let ch = chapitres_temoins();
    let a = archive(&livre_temoin(), &ch, b"\x89PNG", None, "2026-08-22T10:00:00Z").unwrap();
    let mut z = relire(&a);
    let e = z.by_index(0).unwrap();
    assert_eq!(e.name(), "mimetype");
    assert_eq!(e.compression(), zip::CompressionMethod::Stored);
    drop(e);
    let mut s = String::new();
    z.by_name("mimetype").unwrap().read_to_string(&mut s).unwrap();
    assert_eq!(s, "application/epub+zip");
}

/// Ce que l'archive porte sous `OEBPS/` et ce que le manifeste déclare doivent se
/// recouvrir **exactement**, `content.opf` excepté. C'est le défaut qui fait rejeter
/// un EPUB par une liseuse stricte sans qu'aucun autre test ne le voie.
#[test]
fn l_archive_et_le_manifeste_se_recouvrent_exactement() {
    let ch = chapitres_temoins();
    let p = Polices {
        famille: "Cardo".into(),
        romain: Face { nom: "Cardo-Regular.ttf".into(), octets: b"R".to_vec() },
        italique: Some(Face { nom: "Cardo-Italic.ttf".into(), octets: b"I".to_vec() }),
    };
    let a = archive(&livre_temoin(), &ch, b"\x89PNG", Some(&p), "2026-08-22T10:00:00Z").unwrap();
    let mut z = relire(&a);

    let dans_l_archive: std::collections::BTreeSet<String> = (0..z.len())
        .map(|i| z.by_index(i).unwrap().name().to_string())
        .filter(|n| n.starts_with("OEBPS/") && n != "OEBPS/content.opf")
        .map(|n| n["OEBPS/".len()..].to_string())
        .collect();

    let mut opf = String::new();
    z.by_name("OEBPS/content.opf").unwrap().read_to_string(&mut opf).unwrap();
    let manifestes: std::collections::BTreeSet<String> = opf
        .lines()
        .filter_map(|l| l.split("href=\"").nth(1))
        .filter_map(|l| l.split('"').next())
        .map(str::to_string)
        .collect();

    assert_eq!(dans_l_archive, manifestes);
    assert!(dans_l_archive.contains("fonts/Cardo-Italic.ttf"));
}

/// `META-INF/container.xml` désigne l'OPF : c'est par lui que toute liseuse entre dans
/// l'archive, et un chemin faux la rend illisible sans autre message.
#[test]
fn le_container_designe_l_opf() {
    let ch = chapitres_temoins();
    let a = archive(&livre_temoin(), &ch, b"\x89PNG", None, "2026-08-22T10:00:00Z").unwrap();
    let mut z = relire(&a);
    let mut s = String::new();
    z.by_name("META-INF/container.xml").unwrap().read_to_string(&mut s).unwrap();
    assert!(s.contains(r#"full-path="OEBPS/content.opf""#), "{s}");
}

/// Un livre sans chapitre ne produit pas d'archive : ce serait une couverture et deux
/// pages liminaires, et le refus vaut mieux que le fichier qu'on découvrirait vide.
#[test]
fn un_livre_sans_chapitre_est_refuse() {
    let err = archive(&livre_temoin(), &[], b"\x89PNG", None, "2026-08-22T10:00:00Z")
        .unwrap_err();
    assert!(err.contains("chapitre"), "{err}");
}
```

- [ ] **Step 2 : lancer les tests pour les voir échouer**

```bash
cd app/src-tauri && cargo test --lib epub
```

Attendu : ÉCHEC à la compilation — `cannot find function 'archive'`.

- [ ] **Step 3 : écrire l'implémentation**

Insérer dans `epub.rs` :

```rust
use std::io::{Cursor, Write};

use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

/// Le type MIME de l'EPUB, en clair et non compressé, en tête d'archive.
const MIMETYPE: &str = "application/epub+zip";

/// Le seul chemin fixe de la spec : c'est là que toute liseuse entre, et c'est lui qui
/// désigne l'OPF.
const CONTAINER: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
<rootfiles>
<rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/>
</rootfiles>
</container>
"#;

/// Le livre en EPUB 3, en mémoire.
///
/// `modifie` est l'horodatage qu'EPUB 3 exige — voir [`horodatage`]. Il est passé plutôt
/// que lu ici : ce module ne consulte pas d'horloge, sans quoi ses tests dépendraient
/// du jour où on les lance.
pub fn archive(
    livre: &Livre,
    chapitres: &[Chapitre],
    couverture_png: &[u8],
    polices: Option<&Polices>,
    modifie: &str,
) -> Result<Vec<u8>, String> {
    if chapitres.is_empty() {
        return Err("aucun chapitre : il n'y a pas de livre à mettre en EPUB.".into());
    }
    let entrees = contenu(livre, chapitres, couverture_png, polices);
    let opf = opf(livre, &entrees, modifie);

    let mut buf = Vec::new();
    {
        let mut zip = ZipWriter::new(Cursor::new(&mut buf));
        let stocke = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
        let deflate = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        // L'ordre des trois premières entrées n'est pas un style : la spec veut
        // `mimetype` en tête, non compressé, et `META-INF/container.xml` est le seul
        // chemin qu'une liseuse cherche sans qu'on le lui dise.
        pose(&mut zip, "mimetype", MIMETYPE.as_bytes(), stocke)?;
        pose(&mut zip, "META-INF/container.xml", CONTAINER.as_bytes(), deflate)?;
        pose(&mut zip, "OEBPS/content.opf", opf.as_bytes(), deflate)?;
        for e in &entrees {
            let opts = if e.compresse { deflate } else { stocke };
            pose(&mut zip, &format!("OEBPS/{}", e.nom), &e.octets, opts)?;
        }
        zip.finish().map_err(|e| format!("clôture de l'EPUB : {e}"))?;
    }
    Ok(buf)
}

fn pose<W: Write + std::io::Seek>(
    zip: &mut ZipWriter<W>,
    nom: &str,
    contenu: &[u8],
    opts: SimpleFileOptions,
) -> Result<(), String> {
    zip.start_file(nom, opts)
        .map_err(|e| format!("{nom} : {e}"))?;
    zip.write_all(contenu).map_err(|e| format!("{nom} : {e}"))
}
```

- [ ] **Step 4 : lancer les tests pour les voir passer**

```bash
cd app/src-tauri && cargo test --lib epub
```

Attendu : SUCCÈS, 26 tests.

- [ ] **Step 5 : clippy et fmt**

```bash
cd app/src-tauri && cargo clippy --all-targets -- -D warnings && cargo fmt --check
```

Attendu : aucune sortie.

- [ ] **Step 6 : commit**

```bash
cd /Users/jean-paulgavini/Documents/Dev/ozalid
git add app/src-tauri/src/epub.rs
git commit -m "Le livre entre dans une archive qu'une liseuse sait ouvrir"
```

---

# Lot 3 — La couverture insérable et l'intérieur sans imposition

### Task 9 : `couverture::page_une` et `interieur::source_ebook`

**Files:**
- Modify: `app/src-tauri/src/couverture.rs` (après `source_une`, ligne 849-856)
- Modify: `app/src-tauri/src/interieur.rs` (fonction `source`, lignes 144-241)

- [ ] **Step 1 : écrire les tests qui échouent**

À ajouter dans le `mod tests` de `couverture.rs` :

```rust
/// La 1ère, posée dans un autre document, ne doit pas emporter ses `#set` : ceux de
/// `source_une` valent pour le document entier — `par(leading: 0em, justify: false)`,
/// notamment — et écraseraient ceux de l'intérieur pour toutes les pages qui suivent.
/// Le livre sortirait sans interligne et au fer à gauche, plusieurs centaines de pages
/// durant.
#[test]
fn la_couverture_inseree_ne_pose_aucun_reglage_de_document() {
    let p = page_une(&livre(), &maquettes::folio(), FORMAT, None, None);
    assert!(p.starts_with("#page("), "{p}");
    // Un `#set` en tête de ligne vaut pour le document ; à l'intérieur du bloc de
    // page, il ne vaut que pour elle.
    assert!(!p.lines().any(|l| l.starts_with("#set ")), "{p}");
    assert!(p.contains("margin: 0mm"), "{p}");
}
```

Et dans le `mod tests` de `interieur.rs` :

```rust
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
    assert!(!s.contains("#page(footer: none)[]"), "blanche de parité présente : {s}");
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
    let r = Reglage { gouttiere: 15.0, blanche: true };
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
```

> **Aides déjà en place, à ne pas redéfinir :** le `mod tests` de `interieur.rs` porte
> `fn livre()`, `fn chapitres()` et `use crate::providers::provider;` ; celui de
> `couverture.rs` porte `fn livre()`, `const FORMAT: (f64, f64) = (110.0, 180.0)` et
> `use crate::maquettes;`. Les tests ci-dessus s'appuient sur elles telles quelles.

- [ ] **Step 2 : lancer les tests pour les voir échouer**

```bash
cd app/src-tauri && cargo test --lib page_une && cargo test --lib source_ebook
```

Attendu : ÉCHEC à la compilation — `cannot find function 'page_une'`,
`cannot find function 'source_ebook'`.

- [ ] **Step 3 : écrire `couverture::page_une`**

Dans `couverture.rs`, juste après `source_une` :

```rust
/// La 1ère, sur une page insérée dans un autre document.
///
/// Même corps que [`source_une`], mais les réglages de texte et de paragraphe sont
/// portés par le **bloc de la page** au lieu du document : les `#set` de [`preambule`]
/// valent jusqu'à la fin de la source, et l'intérieur qui suivrait perdrait son
/// interligne et sa justification sur des centaines de pages.
///
/// La boîte est rognée, sans fond perdu : un ebook ne se coupe pas.
pub fn page_une(
    livre: &Livre,
    cv: &Couverture,
    format: (f64, f64),
    image: Option<&Ressource>,
    dos_mm: Option<f64>,
) -> String {
    let b = Boite::rognee(format);
    let pano = panorama_face(format, dos_mm, true);
    format!(
        "#page(width: {}, height: {}, margin: 0mm, footer: none)[\n  \
         #set text(lang: \"fr\", top-edge: 0.75em, bottom-edge: -0.25em)\n  \
         #set par(leading: 0em, spacing: 0em, justify: false)\n{}]\n",
        mm(b.largeur),
        mm(b.hauteur),
        corps_une(livre, cv, format, image, b, pano)
    )
}
```

- [ ] **Step 4 : écrire le refactor de `interieur`**

Dans `interieur.rs`, **renommer** `pub fn source` en `fn assemble` et lui ajouter un
dernier paramètre `avant: Option<&str>`. Le corps ne change qu'à un endroit : juste
après le `push_str` du préambule (celui qui se termine par `#set par(justify: true, …)`)
et **avant** `s.push_str(&liminaires(livre, envoi));`, insérer :

```rust
    // La page insérée vient avant tout ce que `liminaires` écrit : c'est la page 1 du
    // fichier, celle qu'un lecteur voit en ouvrant.
    if let Some(a) = avant {
        s.push_str(a);
    }
```

Puis ajouter, juste après `assemble` :

```rust
/// Source Typst de l'intérieur du livre, tel qu'il part à l'impression.
pub fn source(
    livre: &Livre,
    int: &Interieur,
    pr: &Provider,
    r: &Reglage,
    chapitres: &[Chapitre],
    envoi: Option<Trace>,
) -> String {
    assemble(livre, int, pr, r, chapitres, envoi, None)
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
    chapitres: &[Chapitre],
    couverture: &str,
) -> String {
    let r = Reglage {
        gouttiere: pr.exterieur,
        blanche: false,
    };
    assemble(livre, int, pr, &r, chapitres, None, Some(couverture))
}
```

- [ ] **Step 5 : lancer les tests pour les voir passer**

```bash
cd app/src-tauri && cargo test --lib
```

Attendu : SUCCÈS, y compris tous les tests préexistants de `interieur` et `couverture`.

- [ ] **Step 6 : le témoin**

```bash
cd app/src-tauri && cargo run --example temoin
```

Attendu : SUCCÈS, **au même compte de pages** qu'au relevé initial. C'est ce qui
prouve que l'extraction d'`assemble` n'a rien déplacé dans le livre papier.

- [ ] **Step 7 : clippy et fmt**

```bash
cd app/src-tauri && cargo clippy --all-targets -- -D warnings && cargo fmt --check
```

Attendu : aucune sortie.

- [ ] **Step 8 : commit**

```bash
cd /Users/jean-paulgavini/Documents/Dev/ozalid
git add app/src-tauri/src/couverture.rs app/src-tauri/src/interieur.rs
git commit -m "La couverture sait se poser en tête de l'intérieur"
```

---

# Lot 4 — `ebook`, l'orchestration

### Task 10 : l'accès aux répertoires de polices

**Files:**
- Modify: `app/src-tauri/src/typst.rs:38-41`

- [ ] **Step 1 : écrire le test qui échoue**

À ajouter dans le `mod tests` de `typst.rs` :

```rust
/// `ebook` doit lire les fichiers de police pour en embarquer deux dans l'EPUB : les
/// répertoires que Typst connaît sont les seuls qui fassent foi — s'en chercher
/// d'autres embarquerait une police que la composition n'a pas employée.
#[test]
fn les_repertoires_de_polices_sont_lisibles_du_dehors() {
    let t = Typst::new("/x/typst").avec_polices("/a/fonts").avec_polices("/b/fonts");
    let p: Vec<_> = t.polices().iter().map(|p| p.display().to_string()).collect();
    assert_eq!(p, vec!["/a/fonts", "/b/fonts"]);
}
```

- [ ] **Step 2 : lancer le test pour le voir échouer**

```bash
cd app/src-tauri && cargo test --lib typst
```

Attendu : ÉCHEC à la compilation — `no method named 'polices'`.

- [ ] **Step 3 : écrire l'implémentation**

Dans `typst.rs`, après `avec_polices` :

```rust
    /// Les répertoires de polices, pour qui a besoin des fichiers eux-mêmes.
    ///
    /// L'EPUB embarque la police du livre : elle doit être **celle que Typst a
    /// employée**, donc venir des mêmes répertoires. En chercher d'autres embarquerait
    /// une écriture que la composition n'a pas vue.
    pub fn polices(&self) -> &[PathBuf] {
        &self.polices
    }
```

- [ ] **Step 4 : lancer le test pour le voir passer**

```bash
cd app/src-tauri && cargo test --lib typst
```

Attendu : SUCCÈS, 4 tests.

- [ ] **Step 5 : commit**

```bash
cd /Users/jean-paulgavini/Documents/Dev/ozalid
git add app/src-tauri/src/typst.rs
git commit -m "Les polices de la composition se laissent enfin lire"
```

---

### Task 11 : le module `ebook`

**Files:**
- Create: `app/src-tauri/src/ebook.rs`
- Modify: `app/src-tauri/src/lib.rs`

- [ ] **Step 1 : écrire le test qui échoue**

Créer `app/src-tauri/src/ebook.rs` avec l'entête et le seul test qui se tienne sans
disque ni Typst — le reste de ce module ne se vérifie qu'en le lançant :

```rust
//! Les ebooks locaux : le PDF et l'EPUB, écrits à côté du projet.
//!
//! Ce module est aux sorties locales ce que `package` est aux prestataires : il
//! traverse la chaîne, il ne compose rien lui-même. Le PDF vient d'`interieur`, la
//! couverture de `couverture`, l'archive d'`epub` — et Typst fait les deux rendus.
//!
//! L'ebook ne mesure pas sa pagination : il n'a pas de dos à calculer. Sa génération
//! est donc une compilation, là où un package en enchaîne plusieurs.

#[cfg(test)]
mod tests {
    use super::*;

    /// Les deux fichiers portent le nom du livre, assaini comme un répertoire d'envoi :
    /// c'est la fonction du projet qui décide ce qu'un titre devient sur un disque, et
    /// il n'y en a pas deux. Un titre réduit à de la ponctuation ne doit pas donner un
    /// fichier sans nom.
    #[test]
    fn les_fichiers_portent_le_nom_du_livre_assaini() {
        assert_eq!(nom_de_fichier("Les Heures creuses"), "Les Heures creuses");
        assert_eq!(nom_de_fichier("L'été / l'hiver"), "L-été - l-hiver");
        assert_eq!(nom_de_fichier("..."), "envoi");
    }
}
```

> **Note :** la troisième assertion documente une bizarrerie assumée — `envoi::assaini`
> retombe sur « envoi » quand il ne reste rien, parce qu'elle a été écrite pour les
> envois. C'est laid pour un ebook, mais inventer un second repli ferait deux règles
> pour un cas qui ne se produit pas : un livre a un titre.

- [ ] **Step 2 : lancer le test pour le voir échouer**

Déclarer d'abord le module dans `lib.rs`, entre `diffusion` et `envoi` :

```rust
pub mod diffusion;
pub mod ebook;
pub mod envoi;
```

```bash
cd app/src-tauri && cargo test --lib ebook
```

Attendu : ÉCHEC à la compilation — `cannot find function 'nom_de_fichier'`.

- [ ] **Step 3 : écrire l'implémentation**

Insérer dans `ebook.rs`, avant le `mod tests` :

```rust
use std::path::Path;

use serde::Serialize;

use crate::projet::Projet;
use crate::providers::Provider;
use crate::typst::Typst;
use crate::{couverture, envoi, epub, interieur, manuscrit, package, police};

/// Définition du PNG de couverture embarqué dans l'EPUB, en points par pouce.
///
/// À 250 ppp, une couverture de 170 mm de haut fait environ 1670 pixels — au-dessus du
/// seuil où Kindle et Kobo cessent de recadrer la vignette. Monter davantage alourdit
/// l'archive sans rien gagner à l'écran.
const PPP_COUVERTURE: u32 = 250;

/// Ce que la génération a écrit.
#[derive(Debug, Clone, Serialize)]
pub struct Ebooks {
    pub pdf: String,
    pub epub: String,
    pub octets_pdf: u64,
    pub octets_epub: u64,
    /// Familles que Typst a remplacées par une écriture de repli en composant le PDF.
    /// Vide, tout va bien.
    pub polices_introuvables: Vec<String>,
    /// Renseigné quand la police de l'intérieur n'a pas été trouvée dans les
    /// répertoires de Typst : l'EPUB est alors dans l'écriture du lecteur. Ce n'est pas
    /// une erreur — le livre reste juste, seul son œil change.
    pub police_non_embarquee: Option<String>,
}

/// Nom de fichier des deux sorties, sans extension.
fn nom_de_fichier(titre: &str) -> String {
    envoi::assaini(titre)
}

/// Écrit le PDF et l'EPUB du livre dans `dossier`.
///
/// `dos_mm` vient du destinataire visé : il ne sert qu'au cadrage panoramique de la
/// couverture. Absent, l'image se cadre sur la seule 1ère — ce que fait déjà l'aperçu à
/// l'écran, et ce n'est pas un refus de plus.
pub fn generer(
    projet: &Projet,
    pr: &Provider,
    dos_mm: Option<f64>,
    dossier: &Path,
    typst: &Typst,
) -> Result<Ebooks, String> {
    let livre = &projet.meta.livre;
    let int = &projet.meta.interieur;
    // `interieur::source_ebook` interpole la police sans échappement : la validation
    // est ici, comme dans `package::assembler`.
    int.verifie()?;
    let cv = projet
        .meta
        .couverture
        .maquette
        .as_ref()
        .ok_or("aucune maquette de couverture : en choisir une avant de générer les ebooks.")?;
    let chapitres = manuscrit::decoupe(&projet.texte, livre.chapitres)?;
    std::fs::create_dir_all(dossier)
        .map_err(|e| format!("répertoire inutilisable ({}) : {e}", dossier.display()))?;

    let (une, _) = package::ecrire_images(projet, dossier)?;
    let base = nom_de_fichier(&livre.titre);

    // 1. Le PDF : la couverture en page 1, puis l'intérieur sans son imposition.
    let src = dossier.join("ebook.typ");
    let page = couverture::page_une(livre, cv, pr.format, une.as_ref(), dos_mm);
    ecrire(
        &src,
        &interieur::source_ebook(livre, int, pr, &chapitres, &page),
    )?;
    let pdf = dossier.join(format!("{base}.pdf"));
    let polices_introuvables = typst.compile(&src, &pdf)?;

    // 2. La couverture seule, en PNG, pour l'EPUB. Même source que la page du PDF :
    //    les deux fichiers montrent la même image.
    let src_cv = dossier.join("couverture-ebook.typ");
    ecrire(
        &src_cv,
        &couverture::source_une(livre, cv, pr.format, une.as_ref(), dos_mm),
    )?;
    let png = dossier.join("couverture-ebook.png");
    typst.apercu(&src_cv, &png, 1, PPP_COUVERTURE)?;
    let octets_png = std::fs::read(&png)
        .map_err(|e| format!("couverture illisible ({}) : {e}", png.display()))?;

    // 3. L'écriture du livre, si elle est là.
    let polices = polices_du_livre(&int.police, typst.polices());
    let police_non_embarquee = polices.is_none().then(|| int.police.clone());

    // 4. L'archive.
    let arch = epub::archive(
        &epub::Livre {
            titre: &livre.titre,
            auteur: &livre.auteur,
            genre: &livre.genre,
            copyright: &livre.copyright,
            dedicace: livre.dedicace(),
        },
        &chapitres,
        &octets_png,
        polices.as_ref(),
        &epub::horodatage(std::time::SystemTime::now()),
    )?;
    let fichier_epub = dossier.join(format!("{base}.epub"));
    std::fs::write(&fichier_epub, &arch)
        .map_err(|e| format!("écriture impossible ({}) : {e}", fichier_epub.display()))?;

    Ok(Ebooks {
        pdf: pdf.to_string_lossy().into_owned(),
        epub: fichier_epub.to_string_lossy().into_owned(),
        octets_pdf: taille(&pdf),
        octets_epub: arch.len() as u64,
        polices_introuvables,
        police_non_embarquee,
    })
}

/// Le romain et l'italique de la police du livre, lus dans les répertoires de Typst.
///
/// `None` si la famille n'y est pas : l'EPUB se fait alors dans l'écriture du lecteur,
/// et le compte rendu le dit. Ce n'est pas une erreur — contrairement à la composition,
/// où une police absente donnerait un livre imprimé faux.
fn polices_du_livre(famille: &str, dossiers: &[std::path::PathBuf]) -> Option<epub::Polices> {
    let mut trouves: Vec<(String, std::path::PathBuf)> = Vec::new();
    for d in dossiers {
        let Ok(entrees) = std::fs::read_dir(d) else {
            continue;
        };
        for e in entrees.flatten() {
            let chemin = e.path();
            let Ok(octets) = std::fs::read(&chemin) else {
                continue;
            };
            // Un fichier qui n'est pas une police, ou qui ne porte pas le français, est
            // simplement ignoré : `police::examine` refuse, et ce refus-là n'a rien à
            // dire ici — il n'y a pas d'envoi en jeu.
            let Ok(p) = police::examine(&octets) else {
                continue;
            };
            if p.famille == famille {
                if let Some(nom) = chemin.file_name().and_then(|n| n.to_str()) {
                    trouves.push((nom.to_string(), chemin.clone()));
                }
            }
        }
    }
    let noms: Vec<String> = trouves.iter().map(|(n, _)| n.clone()).collect();
    let faces = epub::faces(&noms)?;
    let lire = |nom: &str| -> Option<epub::Face> {
        let (_, chemin) = trouves.iter().find(|(n, _)| n == nom)?;
        Some(epub::Face {
            nom: nom.to_string(),
            octets: std::fs::read(chemin).ok()?,
        })
    };
    Some(epub::Polices {
        famille: famille.to_string(),
        romain: lire(&faces.romain)?,
        italique: faces.italique.as_deref().and_then(lire),
    })
}

fn ecrire(chemin: &Path, contenu: &str) -> Result<(), String> {
    std::fs::write(chemin, contenu)
        .map_err(|e| format!("écriture impossible ({}) : {e}", chemin.display()))
}

/// Taille d'un fichier, ou zéro. Un compte rendu qui échouerait parce qu'il n'a pas su
/// lire une taille serait absurde : le fichier, lui, est écrit.
fn taille(chemin: &Path) -> u64 {
    std::fs::metadata(chemin).map(|m| m.len()).unwrap_or(0)
}
```

- [ ] **Step 4 : lancer le test pour le voir passer**

```bash
cd app/src-tauri && cargo test --lib ebook
```

Attendu : SUCCÈS, 1 test.

- [ ] **Step 5 : clippy et fmt**

```bash
cd app/src-tauri && cargo clippy --all-targets -- -D warnings && cargo fmt --check
```

Attendu : aucune sortie.

- [ ] **Step 6 : commit**

```bash
cd /Users/jean-paulgavini/Documents/Dev/ozalid
git add app/src-tauri/src/ebook.rs app/src-tauri/src/lib.rs
git commit -m "Le livre sait sortir en PDF et en EPUB"
```

---

### Task 12 : l'exercice sur livre réel

**Files:**
- Create: `app/src-tauri/examples/ebook.rs`

- [ ] **Step 1 : écrire l'exemple**

```rust
//! Génère les ebooks locaux d'un projet `.ozalid`, sans interface.
//!
//! C'est le seul moyen de vérifier ce qu'aucun test ne peut dire : que Typst avale la
//! source à couverture insérée, et qu'une liseuse ouvre l'archive.
//!
//! Usage : cargo run --example ebook -- <projet.ozalid> <sortie> [prestataire]

use std::path::{Path, PathBuf};

use ozalid_lib::ebook;
use ozalid_lib::projet::Projet;
use ozalid_lib::providers;
use ozalid_lib::typst::Typst;

fn main() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let (ozalid, sortie) = match (args.next(), args.next()) {
        (Some(a), Some(b)) => (a, b),
        _ => {
            eprintln!("usage : ebook <projet.ozalid> <sortie> [prestataire]");
            std::process::exit(2);
        }
    };
    let projet = Projet::ouvrir(Path::new(&ozalid))?;

    // Le gabarit vient du destinataire visé, comme dans l'application. L'argument n'est
    // là que pour en essayer un autre sans toucher au projet.
    let cle = args.next();
    let d = projet
        .meta
        .livraison
        .courant()
        .ok_or("aucun destinataire dans ce projet.")?;
    let cle = cle.unwrap_or_else(|| d.provider.clone());
    let pr = providers::provider(&cle).ok_or_else(|| format!("prestataire inconnu : {cle}"))?;

    let typst =
        Typst::new("typst").avec_polices(Path::new(env!("CARGO_MANIFEST_DIR")).join("fonts"));
    let r = ebook::generer(&projet, pr, d.dos_mm, &PathBuf::from(&sortie), &typst)?;

    println!("{}  ({} Ko)", r.pdf, r.octets_pdf / 1024);
    println!("{}  ({} Ko)", r.epub, r.octets_epub / 1024);
    if let Some(p) = &r.police_non_embarquee {
        println!("police « {p} » introuvable : EPUB dans l'écriture du lecteur.");
    }
    if !r.polices_introuvables.is_empty() {
        println!(
            "composé par repli : {}. Le PDF ne suit pas la maquette.",
            r.polices_introuvables.join(", ")
        );
    }
    Ok(())
}
```

- [ ] **Step 2 : compiler**

```bash
cd app/src-tauri && cargo build --example ebook
```

Attendu : SUCCÈS.

- [ ] **Step 3 : lancer sur un projet réel**

Prendre un `.ozalid` de `build/` qui porte une maquette de couverture.

```bash
cd app/src-tauri
cargo run --example ebook -- <chemin/vers/projet.ozalid> /tmp/ebook-essai
```

Attendu : deux chemins affichés, et deux fichiers présents dans `/tmp/ebook-essai/`.

- [ ] **Step 4 : regarder, ce qu'aucun test ne fait**

Ouvrir le PDF :

```bash
open /tmp/ebook-essai/*.pdf
```

Vérifier — et ne pas passer à la suite si l'un des cinq points manque :
- la **couverture est la page 1**, pleine page, sans marge blanche autour ;
- l'interligne et la justification du texte sont ceux du livre imprimé, **pas** ceux de
  la couverture (c'est le piège que `page_une` évite : si le texte sort au fer à gauche
  et sans interligne, les `#set` ont fui) ;
- les marges sont **symétriques** — comparer une page paire et une page impaire ;
- **aucune page vide** en fin de volume ;
- les liminaires sont là : faux-titre, page de titre, copyright.

Ouvrir l'EPUB dans Calibre et dans Apple Livres :

```bash
open /tmp/ebook-essai/*.epub
```

Vérifier :
- la **vignette de couverture** paraît dans la bibliothèque ;
- la **table des matières** est navigable et mène au bon chapitre ;
- les **italiques** du manuscrit sont là ;
- le texte est dans la **police du livre**, pas celle du lecteur (comparer avec un autre
  EPUB de la bibliothèque) ;
- les **ruptures de scène** sont centrées, trois astérisques espacées.

- [ ] **Step 5 : commit**

```bash
cd /Users/jean-paulgavini/Documents/Dev/ozalid
git add app/src-tauri/examples/ebook.rs
git commit -m "Les ebooks se tirent aussi sans ouvrir la fenêtre"
```

---

# Lot 5 — L'écran

### Task 13 : la commande

**Files:**
- Modify: `app/src-tauri/src/commands.rs` (après `packager`)
- Modify: `app/src-tauri/src/lib.rs` (`invoke_handler`)

- [ ] **Step 1 : écrire la commande**

Dans `commands.rs`, ajouter `use crate::ebook;` aux imports, puis, après la commande
`packager` :

```rust
/// Génère les ebooks locaux dans `<projet>/ebook/`.
///
/// Une livraison, mais locale : elle ne vise aucun prestataire, elle emprunte seulement
/// le gabarit de celui qui est visé — c'est de là que viennent le format, le corps et
/// l'interligne, faute d'un format d'écran qui voudrait dire quelque chose.
#[tauri::command]
pub fn ebook_generer(atelier: State<Atelier>) -> Result<ebook::Ebooks, String> {
    let garde = atelier.ouvert.lock().unwrap();
    let o = garde.as_ref().ok_or_else(aucun_projet)?;
    let (pr, _, d) = vise(o)?;
    let dossier = sorties_racine(o)?.join("ebook");
    ebook::generer(&o.projet, pr, d.dos_mm, &dossier, &typst()?)
}
```

Dans `lib.rs`, ajouter la commande au `invoke_handler`, après `commands::packager` :

```rust
            commands::packager,
            commands::ebook_generer,
```

- [ ] **Step 2 : compiler**

```bash
cd app/src-tauri && cargo build && cargo clippy --all-targets -- -D warnings && cargo fmt --check
```

Attendu : SUCCÈS, aucune sortie de clippy.

- [ ] **Step 3 : commit**

```bash
cd /Users/jean-paulgavini/Documents/Dev/ozalid
git add app/src-tauri/src/commands.rs app/src-tauri/src/lib.rs
git commit -m "L'interface peut demander ses ebooks"
```

---

### Task 14 : le bloc à l'étape Livraison

**Files:**
- Modify: `app/src/index.html:134-155`
- Modify: `app/src/livraison.js` (après `packager`)
- Modify: `app/src/app.js:437`, `:446`, `:849`
- Test: `app/tests/ebook.test.js` (créé)

- [ ] **Step 1 : écrire le test qui échoue**

Créer `app/tests/ebook.test.js`. Les constantes `LULU`, `PROJET` et la fonction
`faux()` sont celles de `tests/epreuve.test.js`, lignes 10 à ~60 : **les recopier
telles quelles depuis ce fichier** — elles décrivent le même projet témoin, et deux
versions divergentes du même faux Rust seraient pires qu'une duplication.

Le protocole est celui de tout le répertoire : `const { els } = await charge({...})`,
puis `await els.get('id').declenche('click')`, puis lecture de `els.get('id')`. Le
projet doit être chargé d'abord, par le bouton d'import — sans lui, l'écran n'a pas de
livre et les boutons de composition n'ont rien à faire.

```js
'use strict';

// Câblage du bloc Ebooks : le geste qui descend jusqu'au Rust, le compte rendu qui
// s'affiche à côté du bouton, et l'avertissement de police non embarquée. Ce que
// contiennent le PDF et l'EPUB se vérifie en les ouvrant, pas ici.

const test = require('node:test');
const assert = require('node:assert');
const { charge } = require('./dom_shim');

// … recopier ici LULU, PROJET et faux() depuis tests/epreuve.test.js …

const EBOOKS = {
  pdf: '/livres/LHC/ebook/Les Heures creuses.pdf',
  epub: '/livres/LHC/ebook/Les Heures creuses.epub',
  octets_pdf: 2400000,
  octets_epub: 1100000,
  polices_introuvables: [],
  police_non_embarquee: null,
};

/** Un projet chargé et le bouton des ebooks actionné, avec la réponse voulue. */
async function genere(ebook_generer) {
  const { els } = await charge({
    invoke: faux([LULU], { projet_importer: PROJET, ebook_generer }),
    open: async () => '/dev/ozalid/build/LHC/livre.toml',
  });
  await els.get('btImporter').declenche('click');
  await els.get('btEbooks').declenche('click');
  return els;
}

/**
 * Les deux chemins sont ce que l'utilisateur vient chercher : sans eux, il ne sait pas
 * où sont les fichiers qu'il vient de demander.
 */
test('le bouton Ebooks appelle le Rust et affiche les deux chemins', async () => {
  let appels = 0;
  const els = await genere(() => { appels += 1; return EBOOKS; });
  assert.strictEqual(appels, 1);
  const texte = els.get('ebooks').textContent;
  assert.match(texte, /Les Heures creuses\.pdf/);
  assert.match(texte, /Les Heures creuses\.epub/);
  assert.strictEqual(els.get('ebooks').hidden, false);
  assert.strictEqual(els.get('etatEbooks').textContent, '');
});

/**
 * Une police absente de `fonts/` n'est pas une erreur : le livre reste juste, seul son
 * œil change. Le dire en rouge à la place des chemins ferait croire à un échec, et on
 * chercherait des fichiers qui sont pourtant là.
 */
test('une police non embarquée est dite, sans que la génération échoue', async () => {
  const els = await genere(() => ({ ...EBOOKS, police_non_embarquee: 'Vollkorn' }));
  assert.match(els.get('ebooks').textContent, /Vollkorn/);
  assert.match(els.get('ebooks').textContent, /\.epub/);
  assert.strictEqual(els.get('etatEbooks').className, 'etat');
});

/**
 * Un refus est le compte rendu d'un travail long : il reste à côté du bouton qui l'a
 * lancé. Le faire monter à l'entête le ferait lire comme une panne de l'application.
 */
test('un refus du Rust se lit à côté du bouton, pas en haut de l\'écran', async () => {
  const els = await genere(() => {
    throw 'aucune maquette de couverture : en choisir une avant de générer les ebooks.';
  });
  assert.match(els.get('etatEbooks').textContent, /aucune maquette/);
  assert.strictEqual(els.get('etatEbooks').className, 'etat erreur');
  assert.strictEqual(els.get('alerte').textContent, '');
});

/**
 * Un compte rendu qui survivrait à l'ouverture d'un autre livre donnerait à lire les
 * chemins du livre A sous le titre du livre B.
 */
test('ouvrir un autre projet efface le compte rendu', async () => {
  const { els } = await charge({
    invoke: faux([LULU], {
      projet_importer: PROJET,
      ebook_generer: () => EBOOKS,
      projet_nouveau: { ...PROJET, chemin: null, livre: { ...PROJET.livre, titre: '' } },
    }),
    open: async () => '/dev/ozalid/build/LHC/livre.toml',
  });
  await els.get('btImporter').declenche('click');
  await els.get('btEbooks').declenche('click');
  assert.strictEqual(els.get('ebooks').hidden, false);
  await els.get('btNouveau').declenche('click');
  assert.strictEqual(els.get('ebooks').hidden, true);
  assert.strictEqual(els.get('ebooks').textContent, '');
});
```

> **Note :** le dernier test suppose un bouton d'accueil `btNouveau` et une commande
> `projet_nouveau`. Vérifier leurs noms exacts dans `tests/cycle_de_vie.test.js`, qui
> exerce déjà ce chemin, et reprendre les siens — c'est le fichier qui fait foi pour le
> cycle de vie.

- [ ] **Step 2 : lancer le test pour le voir échouer**

```bash
cd app && node --test tests/ebook.test.js
```

Attendu : ÉCHEC — `btEbooks` est `null`, `Cannot read properties of null (reading 'click')`.

- [ ] **Step 3 : le HTML**

Dans `app/src/index.html`, à l'intérieur de `<section id="etapeLivraison">`, **après**
le `</div>` du bloc « Destinataires » (ligne 153) et avant le `</section>` :

```html
    <div class="bloc">
      <h2>Ebooks</h2>
      <p class="note">Le livre entier pour un écran : la couverture, les liminaires et
        tous les chapitres, en PDF et en EPUB. Le PDF est le livre sans son imposition —
        marges symétriques, pas de page blanche de parité. Les fichiers sont écrits dans
        <code>ebook/</code>, à côté du <code>.ozalid</code>. Le format vient du
        destinataire visé, en bas de la fenêtre.</p>
      <div class="ligne">
        <button id="btEbooks" type="button">Générer les ebooks</button>
        <span id="etatEbooks" class="etat"></span>
      </div>
      <div id="ebooks" class="resultat" hidden></div>
    </div>
```

- [ ] **Step 4 : le JS**

Dans `app/src/livraison.js`, après la fonction `packager` :

```js
/** Une taille de fichier, en unités qu'on lit d'un coup d'œil. */
function poids(octets) {
  return octets >= 1024 * 1024
    ? `${nb(octets / (1024 * 1024), 1)} Mo`
    : `${Math.round(octets / 1024)} Ko`;
}

/**
 * Le compte rendu des ebooks : les deux chemins, leur poids, et ce qui s'est passé de
 * travers sans faire échouer la génération.
 *
 * La police non embarquée n'est pas une erreur : le livre reste juste, seul son œil
 * change. Elle se lit donc dans le compte rendu, à côté des chemins, et non en rouge à
 * la place d'un résultat qui existe.
 */
function afficherEbooks(r) {
  const box = $('ebooks');
  box.replaceChildren();
  for (const [chemin, octets] of [[r.pdf, r.octets_pdf], [r.epub, r.octets_epub]]) {
    box.append(h('p', `${chemin}   (${poids(octets)})`, 'chemin'));
  }
  if (r.police_non_embarquee) {
    box.append(h('p', `Police « ${r.police_non_embarquee} » introuvable : l'EPUB est `
      + `dans l'écriture du lecteur. Le texte, lui, est celui du livre.`, 'note'));
  }
  // Celle-ci, en revanche, touche le PDF : c'est le fichier qu'on lira, et il ne suit
  // pas la maquette.
  if (r.polices_introuvables.length) {
    box.append(h('p', 'Police introuvable, composé dans une écriture de repli : '
      + `${r.polices_introuvables.join(', ')}. Le PDF ne suit pas la maquette.`,
    'note alerte'));
  }
  box.hidden = false;
}

async function ebooks() {
  const bt = $('btEbooks');
  bt.disabled = true;
  $('ebooks').hidden = true;
  $('etatEbooks').className = 'etat';
  $('etatEbooks').textContent = 'composition du PDF et de l’EPUB…';
  try {
    afficherEbooks(await invoke('ebook_generer'));
    $('etatEbooks').textContent = '';
  } catch (e) {
    $('etatEbooks').textContent = String(e);
    $('etatEbooks').className = 'etat erreur';
  } finally {
    bt.disabled = false;
  }
}
```

Dans `app/src/app.js` :

- ligne 437, ajouter `'ebooks'` à la liste des comptes rendus à vider :

```js
  for (const id of ['resultat', 'packages', 'ebooks', 'resultatEnvois']) {
```

- ligne 446, ajouter `'etatEbooks'` :

```js
  for (const id of ['etat', 'etatEpreuve', 'etatPackages', 'etatEbooks', 'etatEnvois']) {
```

- ligne 849, poser l'écouteur à côté de celui de `btPackager` :

```js
$('btEbooks').addEventListener('click', ebooks);
```

- [ ] **Step 5 : lancer les tests pour les voir passer**

```bash
cd app && node --test "tests/*.test.js"
```

Attendu : SUCCÈS, y compris les tests préexistants.

- [ ] **Step 6 : commit**

```bash
cd /Users/jean-paulgavini/Documents/Dev/ozalid
git add app/src/index.html app/src/livraison.js app/src/app.js app/tests/ebook.test.js
git commit -m "L'étape Livraison sait aussi livrer au lecteur"
```

---

### Task 15 : l'essai dans la fenêtre

**Files:** aucun — c'est une vérification.

- [ ] **Step 1 : lancer l'application**

```bash
cd app/src-tauri && cargo tauri dev
```

- [ ] **Step 2 : ouvrir un projet réel, aller à la Livraison, générer**

Vérifier — et corriger avant de passer à la suite si l'un des points manque :
- le bloc « Ebooks » est **sous** les destinataires, et ne fait pas déborder l'étape ;
- pendant la génération, le bouton est éteint et l'état dit « composition… » ;
- les deux chemins et leurs poids paraissent **à côté du bouton**, jamais dans l'entête ;
- fermer le projet et en ouvrir un autre efface le compte rendu.

- [ ] **Step 3 : vérifier les refus**

- Sur un projet **sans maquette de couverture** : le message « aucune maquette de
  couverture… » se lit en rouge à côté du bouton, et l'entête reste vide.
- Sur un projet **non enregistré** : le message parle du `.ozalid` à enregistrer.

- [ ] **Step 4 : commit s'il y a eu correction**

Sinon, passer à la tâche suivante.

---

### Task 16 : la documentation

**Files:**
- Modify: `app/README.md`

- [ ] **Step 1 : le tableau des modules**

Dans `app/README.md`, section « Modules », ajouter deux lignes — `epub` après `envoi`,
`ebook` après `package`, en suivant l'ordre du tableau existant :

```markdown
| `epub` | Le livre en EPUB 3 : chapitres, couverture et police → une archive |
| `ebook` | Les sorties locales : le PDF sans imposition et l'EPUB, à côté du projet |
```

- [ ] **Step 2 : l'état du jalon**

Dans le paragraphe « **État : jalon 5** », remplacer « packages multi-prestataires, »
par :

```
packages multi-prestataires, ebooks locaux — PDF et EPUB, couverture comprise —,
```

- [ ] **Step 3 : les vérifications**

Dans la section « Vérifications », ajouter la ligne à la liste des exercices sur livre
réel, après celle de `epreuve` :

```
cargo run --example ebook -- <projet.ozalid> <sortie>
```

Puis, après le paragraphe qui commente `epreuve`, ajouter :

```markdown
`ebook` écrit le PDF et l'EPUB sans interface. Le PDF se regarde pour trois choses que
nul test ne dit : la couverture ouvre-t-elle le fichier, les marges sont-elles
symétriques d'une page paire à une page impaire, ne reste-t-il aucune page vide à la
fin. L'EPUB, lui, se juge dans une liseuse — vignette dans la bibliothèque, table des
matières navigable, italiques présentes, et le texte dans la police du livre et non
dans celle du lecteur.
```

- [ ] **Step 4 : le fichier .ozalid**

Dans la section « Le fichier .ozalid », après la phrase qui dit que seule l'épreuve
reste à la racine, ajouter :

```markdown
Les ebooks ont eux aussi leur répertoire, `ebook/`, frère de ceux des prestataires :
ils ne visent personne en particulier, mais ils sont deux fichiers et non un, et les
laisser à la racine mêlerait le livre du lecteur à l'épreuve du relecteur.
```

- [ ] **Step 5 : commit**

```bash
cd /Users/jean-paulgavini/Documents/Dev/ozalid
git add app/README.md
git commit -m "Le README dit ce que le livre sait devenir à l'écran"
```

---

### Task 17 : la passe complète

**Files:** aucun — c'est une vérification.

- [ ] **Step 1 : toute la batterie**

```bash
cd app/src-tauri && cargo test --lib && cargo clippy --all-targets -- -D warnings && cargo fmt --check
cd ../ && node --test "tests/*.test.js"
```

Attendu : SUCCÈS partout, aucune sortie de clippy ni de fmt.

- [ ] **Step 2 : le témoin, une dernière fois**

```bash
cd app/src-tauri && cargo run --example temoin
```

Attendu : SUCCÈS, **au compte de pages du relevé initial**.

- [ ] **Step 3 : les exercices sur livre réel**

```bash
cd app/src-tauri
cargo run --example packager -- <projet.ozalid> /tmp/pk lulu
cargo run --example ebook -- <projet.ozalid> /tmp/eb
```

Attendu : les deux réussissent. Le premier prouve que le chantier n'a rien cassé du
chemin d'impression.

---

## Ce que ce plan ne fait pas

Repris de la spec, § 9 — à ne pas ajouter en chemin :

- **Le mobi**, et toute conversion vers un format Amazon.
- **Les signets PDF** : les chapitres ne sont pas des `#heading`, et en faire
  demanderait de reprouver la pagination au témoin.
- **`epubcheck`** : aucune plateforme n'est visée. Les tests tiennent les points
  structurels, rien ne valide l'archive automatiquement.
- **Un ebook par dédicataire** : `source_ebook` passe délibérément `None` à `assemble`.
- **Un format d'ebook réglable** : le gabarit vient du destinataire visé.
