# Le blanc de respiration — plan d'implémentation

> **Pour les agents :** SOUS-SKILL REQUISE — `superpowers:subagent-driven-development`
> (recommandé) ou `superpowers:executing-plans` pour dérouler ce plan tâche par tâche.
> Les étapes sont en cases à cocher (`- [ ]`).

**But :** ajouter au manuscrit une seconde coupure, muette — la ligne `___` — qui saute
une ligne sur les PDF sans imprimer d'astérisques.

**Architecture :** un variant `Bloc::Blanc` à côté de `Bloc::Scene`. Le `match`
exhaustif de Rust force les **quatre** sites qui balaient les blocs à traiter le
nouveau cas, donc aucune sortie ne peut l'oublier en silence. Les deux coupures
partagent les mêmes règles de position, factorisées dans `manuscrit.rs`.

**Pile :** Rust (`app/src-tauri/`), Typst 0.15.1 en sidecar, XHTML/CSS pour l'EPUB.

**Spec :** `docs/superpowers/specs/2026-08-22-blanc-de-respiration-design.md`

---

## Les fichiers touchés

| fichier | responsabilité | ce qui change |
|---|---|---|
| `app/src-tauri/src/manuscrit.rs` | format → blocs typés | le variant `Blanc`, la marque `___`, les règles de position communes |
| `app/src-tauri/src/interieur.rs` | blocs → Typst du livre | `#let blanc` au préambule, `#blanc` au rendu |
| `app/src-tauri/src/epreuve.rs` | blocs → Typst de l'épreuve | le filet gris **et** le compte de mots |
| `app/src-tauri/src/epub.rs` | blocs → XHTML | `<p class="blanc">` et sa règle CSS |

Les quatre sites que le compilateur va signaler, à connaître avant de commencer :
`interieur.rs:467`, `epreuve.rs:44` (compte de mots), `epreuve.rs:141`, `epub.rs:141`.
Un cinquième balayage, `epub.rs:753`, est un `if let Bloc::Paragraphe` : il ignore déjà
les blocs sans texte et n'a **rien** à changer — un blanc n'a pas de XML à valider.

**Convention de test du dépôt :** chaque `#[test]` porte un doc-comment `///` qui dit
*pourquoi* le comportement compte, jamais seulement ce qu'il fait. Voir `manuscrit.rs`
lignes 565, 575, 591. Les tests ci-dessous respectent cette forme ; ne pas la retirer.

Toutes les commandes `cargo` se lancent depuis `app/src-tauri/`.

---

### Tâche 1 : Le manuscrit reconnaît `___`

**Fichiers :**
- Modifier : `app/src-tauri/src/manuscrit.rs` (en-tête `//!`, enum ligne 19, `decoupe` ligne 365)
- Modifier : `app/src-tauri/src/interieur.rs:467`, `epreuve.rs:44`, `epreuve.rs:141`, `epub.rs:141` — branches provisoires
- Test : `app/src-tauri/src/manuscrit.rs`, module `tests`

- [ ] **Étape 1 : écrire le test qui échoue**

À placer dans `mod tests` de `manuscrit.rs`, juste après
`une_rupture_de_scene_est_gardee_comme_bloc` :

```rust
    /// Le blanc est une coupure que l'auteur a écrite, au même titre que la rupture de
    /// scène : typé, il traverse la découpe sans se confondre avec un paragraphe dont
    /// le texte serait « ___ » — qui, lui, s'imprimerait tel quel.
    #[test]
    fn un_blanc_de_respiration_est_garde_comme_bloc() {
        let ch = decoupe("## 01 - Un\n\nAvant.\n\n___\n\nAprès.\n", None).unwrap();
        assert_eq!(
            ch[0].blocs,
            vec![
                Bloc::Paragraphe("Avant.".into()),
                Bloc::Blanc,
                Bloc::Paragraphe("Après.".into()),
            ]
        );
    }
```

- [ ] **Étape 2 : le lancer pour le voir échouer — premier rouge**

```
cargo test un_blanc_de_respiration_est_garde_comme_bloc
```

Attendu : **échec de compilation**, `error[E0599]: no variant or associated item named
'Blanc' found for enum 'Bloc'`. C'est le rouge attendu : le variant n'existe pas.

- [ ] **Étape 3 : ajouter le variant**

Dans `manuscrit.rs`, remplacer l'enum de la ligne 18 :

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum Bloc {
    Paragraphe(String),
    Scene,
    Blanc,
}
```

et compléter le doc-comment qui la précède, après la phrase existante sur la rupture de
scène :

```rust
/// Le blanc de respiration est la même coupure, muette : l'auteur sépare deux passages
/// sans vouloir que la page l'annonce. Les deux suivent les mêmes règles de position ;
/// seul le rendu les distingue.
```

- [ ] **Étape 4 : donner aux quatre sites une branche provisoire**

Le compilateur les refuse tant qu'ils ne traitent pas `Blanc`. Ces branches sont
**provisoires** : les tâches 4 à 6 les remplissent. Elles ne rendent rien pour l'instant,
ce qui est un état honnête — la marque est reconnue, pas encore composée.

`interieur.rs`, dans `blocs_typst`, après la branche `Bloc::Scene` (ligne 480) :

```rust
            Bloc::Blanc => {}
```

`epreuve.rs`, dans le compte de mots (ligne 46), remplacer `Bloc::Scene => None,` par :

```rust
            Bloc::Scene | Bloc::Blanc => None,
```

`epreuve.rs`, dans le rendu, après la branche `Bloc::Scene` (ligne 148) :

```rust
                Bloc::Blanc => {}
```

`epub.rs`, dans `blocs_xhtml`, après la branche `Bloc::Scene` (ligne 143) :

```rust
            Bloc::Blanc => {}
```

- [ ] **Étape 5 : relancer le test — deuxième rouge, celui qui compte**

```
cargo test un_blanc_de_respiration_est_garde_comme_bloc
```

Attendu : compilation réussie, **test en échec** sur l'égalité — la découpe a produit
`Paragraphe("___")` au lieu de `Blanc`, parce que `___` n'est encore qu'une ligne de
texte ordinaire.

- [ ] **Étape 6 : brancher la marque dans `decoupe`**

Ajouter cette fonction dans `manuscrit.rs`, juste au-dessus de `elague_rupture_finale`
(ligne 288) :

```rust
/// Les deux marques de coupure du format, et rien d'autre. `---` se voit sur la page,
/// `___` non ; tout le reste — position, élagage, doublons — leur est commun, et c'est
/// pour cela qu'elles se lisent au même endroit.
///
/// `___` est la jumelle de `---` dans le Markdown standard : un manuscrit ouvert dans
/// n'importe quel éditeur y montre déjà une ligne, et aucune faute de frappe ne
/// transforme l'une en l'autre.
fn rupture(t: &str) -> Option<Bloc> {
    match t {
        "---" => Some(Bloc::Scene),
        "___" => Some(Bloc::Blanc),
        _ => None,
    }
}
```

Puis, dans `decoupe`, remplacer la branche `} else if t == "---" {` (ligne 365) et son
corps par :

```rust
        } else if let Some(rupture) = rupture(t) {
            // Hors chapitre, la rupture appartient aux liminaires : rien à garder. Dans
            // un chapitre, elle n'est gardée qu'à la suite d'un paragraphe : ni en tête
            // de chapitre, ni après une rupture déjà posée (deux marques consécutives ne
            // séparent qu'une fois, quelles qu'elles soient).
            if let Some(courant) = pieces.last_mut() {
                if matches!(courant.blocs.last(), Some(Bloc::Paragraphe(_))) {
                    courant.blocs.push(rupture);
                }
            }
```

- [ ] **Étape 7 : mettre à jour l'en-tête du fichier**

C'est le seul endroit du dépôt où le format admis se lit en entier. Dans le `//!` de
tête (ligne 3), remplacer :

```rust
//! Le format admis est celui du projet, et lui seul : titre en `# `, chapitres en
//! `## NN - Titre`, séparateurs de scène `---`, emphase `*…*` et `**…**`. Tout le
```

par :

```rust
//! Le format admis est celui du projet, et lui seul : titre en `# `, chapitres en
//! `## NN - Titre`, coupures `---` (marquée) et `___` (muette), emphase `*…*` et
//! `**…**`. Tout le
```

- [ ] **Étape 8 : vert**

```
cargo test un_blanc_de_respiration_est_garde_comme_bloc
```

Attendu : `test result: ok. 1 passed`.

- [ ] **Étape 9 : la suite entière ne bouge pas**

```
cargo test
```

Attendu : tous les tests passent. Les tests existants sur `---` sont intacts : la
marque marquée n'a pas changé de comportement.

- [ ] **Étape 10 : commit**

```bash
git add app/src-tauri/src/manuscrit.rs app/src-tauri/src/interieur.rs \
        app/src-tauri/src/epreuve.rs app/src-tauri/src/epub.rs
git commit -m "Le manuscrit sait qu'une ligne peut se sauter sans le dire"
```

---

### Tâche 2 : Les deux coupures suivent les mêmes règles de position

**Fichiers :**
- Modifier : `app/src-tauri/src/manuscrit.rs` (`elague_rupture_finale` ligne 288)
- Test : `app/src-tauri/src/manuscrit.rs`, module `tests`

- [ ] **Étape 1 : écrire les tests qui échouent**

À placer dans `mod tests`, à la suite du test de la tâche 1 :

```rust
    /// Un blanc qui ferme un chapitre ne sépare rien : le chapitre suivant commence sur
    /// sa propre page. Sans élagage, l'épreuve afficherait un filet parasite avant
    /// chaque saut de page — exactement le défaut corrigé pour la rupture de scène.
    #[test]
    fn un_blanc_en_fin_de_chapitre_ne_laisse_pas_de_bloc() {
        let ch = decoupe("## 01 - Un\n\nTexte.\n\n___\n\n## 02 - Deux\n\nTexte.\n", None).unwrap();
        assert_eq!(ch[0].blocs, vec![Bloc::Paragraphe("Texte.".into())]);
        assert_eq!(ch[1].blocs, vec![Bloc::Paragraphe("Texte.".into())]);
    }

    /// Un blanc qui ouvre un chapitre n'a pas de passage précédent à séparer du suivant.
    #[test]
    fn un_blanc_en_tete_de_chapitre_ne_laisse_pas_de_bloc() {
        let ch = decoupe("## 01 - Un\n\n___\n\nTexte.\n", None).unwrap();
        assert_eq!(ch[0].blocs, vec![Bloc::Paragraphe("Texte.".into())]);
    }

    /// Deux coupures consécutives ne séparent qu'une fois, et c'est la première qui
    /// vaut — quelles que soient les deux marques. L'auteur qui hésite et laisse les
    /// deux ne creuse pas sa page pour autant, et la règle se retient sans exception :
    /// c'est l'ordre d'écriture qui tranche, pas une priorité entre marques.
    #[test]
    fn deux_coupures_consecutives_ne_separent_qu_une_fois() {
        for (md, attendu) in [
            ("## 01 - Un\n\nA.\n\n___\n\n___\n\nB.\n", Bloc::Blanc),
            ("## 01 - Un\n\nA.\n\n---\n\n___\n\nB.\n", Bloc::Scene),
            ("## 01 - Un\n\nA.\n\n___\n\n---\n\nB.\n", Bloc::Blanc),
        ] {
            let ch = decoupe(md, None).unwrap();
            assert_eq!(
                ch[0].blocs,
                vec![
                    Bloc::Paragraphe("A.".into()),
                    attendu.clone(),
                    Bloc::Paragraphe("B.".into()),
                ],
                "{md}"
            );
        }
    }

    /// Un `___` avant le premier chapitre appartient aux liminaires du manuscrit, que le
    /// projet compose lui-même : il ne doit ouvrir aucun chapitre fantôme.
    #[test]
    fn un_blanc_avant_le_premier_chapitre_est_ignore() {
        let ch = decoupe("# Le Livre\n\n___\n\n## 01 - Un\n\nTexte.\n", None).unwrap();
        assert_eq!(ch.len(), 1);
        assert_eq!(ch[0].blocs, vec![Bloc::Paragraphe("Texte.".into())]);
    }
```

- [ ] **Étape 2 : les lancer pour les voir échouer**

```
cargo test _blanc_
```

Attendu : `un_blanc_en_fin_de_chapitre_ne_laisse_pas_de_bloc` **échoue** — le chapitre
garde un `Bloc::Blanc` orphelin en dernière position, parce que `elague_rupture_finale`
ne connaît que `Scene`.

Les trois autres passent déjà : la règle « seulement après un paragraphe » de la tâche 1
les couvre. Les garder quand même — ce sont les trois invariants du format, et un
refactor futur doit les faire tomber s'il les casse.

- [ ] **Étape 3 : généraliser l'élagage**

Dans `manuscrit.rs`, ajouter cette méthode juste après le bloc `impl Piece` (ligne 70) :

```rust
impl Bloc {
    /// Une coupure entre deux passages, marquée ou muette. Ce que les deux ont en
    /// commun tient ici : elles ne valent qu'entre deux passages, et n'importe quelle
    /// règle de position les traite ensemble.
    fn est_rupture(&self) -> bool {
        matches!(self, Bloc::Scene | Bloc::Blanc)
    }
}
```

Puis remplacer le corps de `elague_rupture_finale` (ligne 288) :

```rust
fn elague_rupture_finale(ch: Option<&mut Piece>) {
    if let Some(ch) = ch {
        if ch.blocs.last().is_some_and(Bloc::est_rupture) {
            ch.blocs.pop();
        }
    }
}
```

`is_some_and` est déjà la forme employée par le fichier (`manuscrit.rs:424`).

Compléter enfin son doc-comment, à la suite de la phrase existante sur *WIP7* :

```rust
/// La règle vaut pour les deux marques : un `___` en fin de chapitre ne sépare pas
/// davantage qu'un `---`.
```

- [ ] **Étape 4 : vert**

```
cargo test _blanc_
```

Attendu : `4 passed`.

- [ ] **Étape 5 : mutation ciblée — vérifier que les tests protègent vraiment**

Remettre temporairement `matches!(ch.blocs.last(), Some(Bloc::Scene))` dans
`elague_rupture_finale`, puis :

```
cargo test
```

Attendu : `un_blanc_en_fin_de_chapitre_ne_laisse_pas_de_bloc` **échoue**. Rétablir
`is_some_and(Bloc::est_rupture)` et confirmer le retour au vert avec `cargo test`.

- [ ] **Étape 6 : commit**

```bash
git add app/src-tauri/src/manuscrit.rs
git commit -m "Une coupure qui ne sépare rien n'existe pas, muette ou non"
```

---

### Tâche 3 : Mesurer la hauteur du blanc sur un PDF

Aucune modification du dépôt. Cette tâche répond à la seule question que le § 3 de la
spec laisse ouverte : **Typst fusionne-t-il deux espacements faibles adjacents en
gardant le plus grand, ou les additionne-t-il ?** De la réponse dépend la valeur à
poser en tâche 4. La spec est explicite : c'est la mesure qui tranche, pas le
raisonnement.

**Fichiers :** uniquement dans le scratchpad de session, rien à committer.

- [ ] **Étape 1 : écrire le fichier de mesure**

Le préambule reproduit celui de `interieur.rs:180-190` — mêmes `top-edge`/`bottom-edge`
(qui ramènent la boîte de ligne à 1 em), même `leading` = `spacing` = `lead`. Avec
l'interligne 1,35 de BoD, `lead` vaut 0,35.

Écrire dans `<scratchpad>/mesure-blanc.typ` :

```typst
#set page(width: 100mm, height: 150mm, margin: 10mm)
#set text(font: "EB Garamond", size: 10pt, lang: "fr",
          top-edge: 0.75em, bottom-edge: -0.25em)
#set par(justify: true, leading: 0.35em, spacing: 0.35em, first-line-indent: 1.2em)

#let blanc = v(1em + 0.35em * 2, weak: true)

Premier paragraphe témoin, sans coupure après lui.

Deuxième paragraphe, collé au premier comme le veut la composition.

#blanc

Troisième paragraphe, précédé du blanc mesuré.

Quatrième paragraphe, collé au troisième.
```

- [ ] **Étape 2 : composer avec le Typst embarqué**

Le binaire est celui du sidecar — la version est épinglée, une autre ne mesurerait pas
la même chose. Depuis la racine du dépôt :

```bash
app/src-tauri/binaries/typst-aarch64-apple-darwin compile \
  --ignore-system-fonts --font-path app/src-tauri/fonts \
  <scratchpad>/mesure-blanc.typ <scratchpad>/mesure-blanc.pdf
```

Attendu : aucune sortie, le PDF est écrit. Si Typst signale une police manquante,
vérifier que `app/src-tauri/fonts` contient bien EB Garamond — c'est le piège connu du
CLAUDE.md.

- [ ] **Étape 3 : regarder le résultat**

```bash
pdftoppm -png -r 150 <scratchpad>/mesure-blanc.pdf <scratchpad>/mesure-blanc
```

Ouvrir `<scratchpad>/mesure-blanc-1.png` et **compter les lignes** : l'écart entre le
deuxième et le troisième paragraphe doit valoir exactement une ligne de texte vide, ni
plus ni moins. Le repère est l'écart entre le premier et le deuxième paragraphe, qui est
l'écart nul de référence.

- [ ] **Étape 4 : trancher la valeur**

- Écart d'une ligne pleine → Typst fusionne comme prévu, la valeur `1em + lead * 2`
  est la bonne. La tâche 4 l'utilise telle quelle.
- Écart trop grand (deux lignes ou davantage) → Typst additionne. Reprendre l'étape 1
  avec `#let blanc = v(1em + 0.35em, weak: true)`, recomposer, revérifier.
- Écart trop petit → la fusion mange plus que prévu. Augmenter par pas de `0.35em`
  jusqu'à la ligne pleine.

**Consigner la valeur retenue et ce qui a été vu** : elle est reprise telle quelle en
tâche 4, et le § 3 de la spec est corrigé en tâche 7 si le raisonnement était faux.

---

### Tâche 4 : L'intérieur imprimé compose le blanc

**Fichiers :**
- Modifier : `app/src-tauri/src/interieur.rs` (préambule ligne 187, `blocs_typst` ligne 480)
- Test : `app/src-tauri/src/interieur.rs`, module `tests`

Remplacer partout ci-dessous `1em + {lead}em * 2` par la valeur retenue en tâche 3 si
la mesure a tranché autrement.

- [ ] **Étape 1 : écrire le test qui échoue**

À placer dans le `mod tests` de `interieur.rs`, près des tests existants sur les
ruptures de scène :

```rust
    /// Le blanc est un espace, pas un signe : la source ne doit porter aucune marque
    /// pour lui. C'est toute la différence avec la rupture de scène, et elle se vérifie
    /// ici plutôt qu'après tirage.
    #[test]
    fn le_blanc_de_respiration_ne_compose_aucune_marque() {
        let s = blocs_typst(&[
            Bloc::Paragraphe("Avant.".into()),
            Bloc::Blanc,
            Bloc::Paragraphe("Après.".into()),
        ]);
        assert!(s.contains("#blanc"), "{s}");
        assert!(!s.contains(SCENE), "{s}");
    }

    /// Le blanc est faible au sens de Typst : il disparaît à une frontière de page.
    /// C'est ce qui protège le registre — sans `weak`, la page suivante s'ouvrirait sur
    /// un trou et ses lignes ne seraient plus en regard de celles d'en face.
    #[test]
    fn le_blanc_de_respiration_est_un_espace_faible() {
        let pr = provider("bod").unwrap();
        let r = Reglage {
            gouttiere: 20.0,
            blanche: false,
        };
        let s = source(&livre(), &Interieur::default(), pr, &r, &pieces_avec_blanc(), None);
        assert!(s.contains("#let blanc = v("), "{s}");
        assert!(s.contains("weak: true"), "{s}");
    }
```

Ajouter l'échantillon juste après l'aide `chapitres()` du module (ligne 505), dont il
reprend la forme :

```rust
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
```

L'aide `livre()` existe déjà (ligne 492), et `provider` est importé en tête du module.
L'appel de `source` ci-dessus reprend celui de
`la_source_porte_le_gabarit_du_prestataire_et_le_marqueur` (ligne 592), au corpus près.

- [ ] **Étape 2 : les lancer pour les voir échouer**

```
cargo test le_blanc_de_respiration
```

Attendu : les deux **échouent** — `blocs_typst` ne rend rien pour `Blanc` (branche
provisoire de la tâche 1) et le préambule ne définit aucun `#let blanc`.

- [ ] **Étape 3 : définir le blanc au préambule**

Dans `assemble`, à la fin du `format!` du préambule, juste après la ligne
`#set par(justify: true, leading: {lead}em, …)` (ligne 187), ajouter :

```
#let blanc = v(1em + {lead}em * 2, weak: true)
```

`lead` est déjà interpolé dans ce même `format!` (ligne 154), il n'y a rien à propager.
C'est aussi pourquoi le blanc se définit ici et non dans `blocs_typst` : ni cette
fonction ni `liminaires` n'ont l'interligne en portée, et les deux composent des blocs
dans la **même** source — `source` comme `source_ebook` passent par `assemble`, donc le
`#let` est visible partout ensuite.

Ajouter au-dessus, dans le même littéral, le commentaire Typst qui dit pourquoi :

```
// Le blanc de respiration : une ligne sautée, sans marque. Faible au sens de Typst,
// donc supprimé à une frontière de page — le registre passe avant la coupure.
```

- [ ] **Étape 4 : rendre le blanc**

Dans `blocs_typst`, remplacer la branche provisoire `Bloc::Blanc => {}` par :

```rust
            // Le blanc n'a pas de marque, donc rien à centrer : il est tout entier
            // dans l'espace. Sa hauteur est définie une fois au préambule, là où
            // l'interligne est connue — une ligne de texte laissée vide.
            Bloc::Blanc => s.push_str("#blanc\n\n"),
```

- [ ] **Étape 5 : vert**

```
cargo test le_blanc_de_respiration
```

Attendu : `2 passed`.

- [ ] **Étape 6 : voir le blanc sur une vraie page**

```
cargo run --example temoin <scratchpad>/temoin-blanc
```

Attendu : `98 pages`, le compte inchangé — *Candide* ne porte aucun `___`. Un écart, même
d'une page, signalerait que le rendu a fui hors de son cas et **arrête la tâche**.

- [ ] **Étape 7 : commit**

```bash
git add app/src-tauri/src/interieur.rs
git commit -m "L'intérieur saute la ligne que le manuscrit demande"
```

---

### Tâche 5 : L'épreuve montre la coupure muette

**Fichiers :**
- Modifier : `app/src-tauri/src/epreuve.rs` (rendu ligne 148)
- Test : `app/src-tauri/src/epreuve.rs`, module `tests`

Le compte de mots (ligne 46) a déjà sa forme définitive depuis la tâche 1 : un blanc
n'apporte aucun mot, `None` est la bonne réponse et le restera.

- [ ] **Étape 1 : écrire le test qui échoue**

Dans le `mod tests` de `epreuve.rs`, près du test existant sur la rupture de scène :

```rust
    /// L'épreuve est un document de travail, pas le livre : une coupure muette y serait
    /// invisible, et le relecteur ne pourrait pas vérifier qu'elle a bien été saisie.
    /// Le filet la lui montre, dans le gris de service qui ne s'imprime jamais — plus
    /// clair que celui de l'astérisme, parce que la coupure est la plus légère des deux.
    #[test]
    fn le_blanc_de_respiration_porte_un_filet_sur_l_epreuve() {
        let s = source(&livre(), &Interieur::default(), &pieces_avec_blanc(), 12.0);
        assert!(s.contains("#line(length: 12mm"), "{s}");
        assert!(s.contains("#c0c0c0"), "{s}");
        assert!(!s.contains("#808080\"))[\\*"), "{s}");
    }
```

Ajouter l'échantillon au même module :

```rust
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
```

L'appel de `source` reprend celui de l'aide `src()` du module (ligne 191),
`source(&livre(), &Interieur::default(), &chapitres(), 12.0)`, avec l'échantillon qui
porte le blanc à la place de `chapitres()`. L'aide `livre()` existe déjà (ligne 160).

- [ ] **Étape 2 : le lancer pour le voir échouer**

```
cargo test le_blanc_de_respiration_porte_un_filet
```

Attendu : **échec** — la source ne porte aucun `#line`, la branche est encore vide.

- [ ] **Étape 3 : composer le filet**

Dans le rendu de `epreuve.rs`, remplacer la branche provisoire `Bloc::Blanc => {}` par :

```rust
                // Le livre laisse ce blanc muet ; l'épreuve, non. Elle numérote les
                // lignes et compose déjà l'astérisme en gris de service : un filet de
                // la même famille, plus clair, dit la coupure au relecteur sans rien
                // promettre de la page imprimée.
                Bloc::Blanc => s.push_str(
                    "#v(3mm)\n#align(center)[#line(length: 12mm, \
                     stroke: 0.4pt + rgb(\"#c0c0c0\"))]\n#v(3mm)\n\n",
                ),
```

- [ ] **Étape 4 : vert**

```
cargo test le_blanc_de_respiration_porte_un_filet
```

Attendu : `1 passed`.

- [ ] **Étape 5 : voir l'épreuve**

L'exemple compose l'épreuve d'un projet existant :

```
cargo run --example epreuve -- <projet.ozalid> <scratchpad>/epreuve-blanc.pdf
```

Le projet doit porter un manuscrit contenant un `___` — celui écrit en tâche 7 étape 1
convient, et cette étape peut attendre qu'il existe. Convertir ensuite la page en image
et la regarder :

```bash
pdftoppm -png -r 150 <scratchpad>/epreuve-blanc.pdf <scratchpad>/epreuve-blanc
```

Attendu : un filet gris clair, centré, nettement plus discret que l'astérisme d'une
rupture de scène, et qui n'écrase pas le texte autour.

- [ ] **Étape 6 : commit**

```bash
git add app/src-tauri/src/epreuve.rs
git commit -m "L'épreuve montre au relecteur la coupure que le livre tait"
```

---

### Tâche 6 : L'EPUB rend le blanc

**Fichiers :**
- Modifier : `app/src-tauri/src/epub.rs` (`blocs_xhtml` ligne 143, CSS ligne 555)
- Test : `app/src-tauri/src/epub.rs`, module `tests`

- [ ] **Étape 1 : écrire le test qui échoue**

Dans le `mod tests` de `epub.rs`, près de
`la_rupture_de_scene_porte_la_meme_asterisque_que_le_papier` (ligne 811) :

```rust
    /// Les liseuses escamotent les `<p>` vides : sans l'espace insécable, le blanc
    /// disparaîtrait de l'EPUB alors qu'il est sur le papier. Le caractère est écrit en
    /// littéral U+00A0, comme l'astérisme — le document est du XHTML sans DTD, où
    /// `&nbsp;` n'est pas défini et ferait échouer la lecture.
    #[test]
    fn le_blanc_de_respiration_survit_aux_liseuses() {
        let x = blocs_xhtml(&[
            Bloc::Paragraphe("Avant.".into()),
            Bloc::Blanc,
            Bloc::Paragraphe("Après.".into()),
        ]);
        assert!(x.contains("<p class=\"blanc\">\u{a0}</p>"), "{x}");
        assert!(!x.contains("&nbsp;"), "{x}");
    }
```

- [ ] **Étape 2 : le lancer pour le voir échouer**

```
cargo test le_blanc_de_respiration_survit_aux_liseuses
```

Attendu : **échec** — le XHTML ne porte pas de `<p class="blanc">`.

- [ ] **Étape 3 : rendre le blanc**

Dans `blocs_xhtml`, remplacer la branche provisoire `Bloc::Blanc => {}` par :

```rust
            // L'espace insécable n'est pas une précaution de style : les liseuses
            // suppriment les paragraphes vides, et le blanc s'en irait avec.
            Bloc::Blanc => s.push_str("<p class=\"blanc\">\u{a0}</p>\n"),
```

- [ ] **Étape 4 : ajouter la règle CSS**

Dans la feuille de style (ligne 555), juste après la règle `p.scene` :

```
p.blanc {{ margin: 0; text-indent: 0; }}
```

Les accolades sont doublées : la feuille est écrite dans un `format!`. `margin: 0`
parce que la ligne du paragraphe **est** le blanc — une marge s'y ajouterait au lieu de
le composer, là où `p.scene` a besoin des siennes pour dégager sa marque.

- [ ] **Étape 5 : vérifier que la règle CSS accompagne la classe**

Une classe sans règle serait un blanc silencieusement rendu par le style par défaut de
la liseuse. Ajouter ce test à la suite du précédent :

```rust
    /// Une classe sans règle laisserait la liseuse appliquer son style de paragraphe :
    /// alinéa et marges reviendraient, et le blanc vaudrait alors plus qu'une ligne.
    #[test]
    fn la_feuille_de_style_porte_la_regle_du_blanc() {
        let f = css(None);
        assert!(f.contains("p.blanc { margin: 0"), "{f}");
    }
```

`css(None)` est la fonction qui produit la feuille (`epub.rs:519`) ; `None` signifie
« aucune police embarquée », ce qui ne change rien aux règles de paragraphe.

Les accolades sont **simples** dans l'assertion et doubles dans la source : `css` est
écrite dans un `format!`, et le test lit la feuille rendue, pas son littéral.

```
cargo test le_blanc_de_respiration_survit_aux_liseuses la_feuille_de_style_porte_la_regle_du_blanc
```

Attendu : `2 passed`.

- [ ] **Étape 6 : commit**

```bash
git add app/src-tauri/src/epub.rs
git commit -m "L'EPUB garde le blanc que les liseuses effaceraient"
```

---

### Tâche 7 : Vérifications complètes et mise à jour de la spec

**Fichiers :**
- Modifier : `docs/superpowers/specs/2026-08-22-blanc-de-respiration-design.md` (§ 3), si la mesure a démenti le calcul

- [ ] **Étape 1 : la chaîne complète sur un manuscrit qui porte les deux coupures**

Écrire dans le scratchpad un manuscrit de deux chapitres mêlant `---` et `___`, l'importer
dans un projet de test, et composer intérieur, épreuve et EPUB. Vérifier sur les trois
sorties que la marquée porte ses astérisques et que la muette n'en porte aucun.

- [ ] **Étape 2 : le témoin**

```
cargo run --example temoin
```

Attendu : **98 pages**, la valeur de `PAGES_ATTENDUES` (`examples/temoin.rs:34`). Un
écart arrête tout : il voudrait dire que le blanc s'est glissé dans des livres qui ne le
demandent pas.

- [ ] **Étape 3 : les vérifications avant commit du CLAUDE.md**

```
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

puis, depuis `app/` :

```
node --test tests/*.test.js
```

Attendu : les quatre propres. `cargo fmt --check` ne sort rien.

- [ ] **Étape 4 : corriger la spec si la mesure l'a démentie**

Si la tâche 3 a retenu une autre valeur que `1em + lead × 2`, remplacer dans le § 3 de
la spec le paragraphe qui l'annonçait par la valeur mesurée, et dire ce qui a été
observé — Typst additionne ou fusionne. La spec doit décrire le code tel qu'il est, pas
tel qu'il était prévu. Si la mesure a confirmé le calcul, retirer la phrase « à
confirmer par mesure » et la remplacer par « relevé sur PDF le 22/08 ».

- [ ] **Étape 5 : commit final**

```bash
git add docs/superpowers/specs/2026-08-22-blanc-de-respiration-design.md
git commit -m "La spec du blanc dit la hauteur relevée, non celle attendue"
```

Ne **pas** pousser : le dépôt travaille en merge direct sur `main`, et la poussée se
demande.
