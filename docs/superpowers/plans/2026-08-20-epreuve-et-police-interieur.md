# Épreuve A4 de relecture et police d'intérieur réglable — plan d'implémentation

> **Pour les agents :** SOUS-SKILL REQUIS — utiliser `superpowers:subagent-driven-development`
> (recommandé) ou `superpowers:executing-plans` pour exécuter ce plan tâche par tâche.
> Les étapes sont en cases à cocher (`- [ ]`).

**But :** produire un PDF A4 de relecture depuis le manuscrit du projet, et faire de la
police de l'intérieur un réglage du livre au lieu d'un défaut subi.

**Architecture :** un module `epreuve` autonome, sans `Provider` ni convergence, qui ne
partage avec `interieur` que le découpage du manuscrit. Une structure `Interieur` dans
`interieur.rs`, portée par `projet.toml`, validée contre une liste de sept serifs de
labeur. Un enum `Bloc` remplace `Vec<String>` dans `Chapitre` pour que les ruptures de
scène survivent jusqu'à l'épreuve.

**Pile :** Rust + Tauri 2, sidecar Typst 0.15.1, front vanilla sans bundler.

**Spec :** `docs/superpowers/specs/2026-08-20-epreuve-de-relecture-design.md`

---

## Fichiers touchés

| Fichier | Rôle |
|---|---|
| `app/outils/polices.sh` | modifié — quatre familles de plus |
| `app/src-tauri/src/interieur.rs` | modifié — `Interieur`, `POLICES_TEXTE`, `font:` dans la source |
| `app/src-tauri/src/manuscrit.rs` | modifié — enum `Bloc`, `Chapitre.blocs` |
| `app/src-tauri/src/projet.rs` | modifié — section `[interieur]` dans `Metadonnees` |
| `app/src-tauri/src/epreuve.rs` | **créé** — la source Typst de l'épreuve |
| `app/src-tauri/src/lib.rs` | modifié — module et commandes déclarés |
| `app/src-tauri/src/commands.rs` | modifié — `interieur_modifier`, `polices_texte_liste`, `epreuve` |
| `app/src-tauri/src/package.rs` | modifié — passe `&projet.meta.interieur` |
| `app/src-tauri/examples/composer.rs` | modifié — idem |
| `app/src-tauri/examples/epreuve.rs` | **créé** — exercer l'épreuve sans fenêtre |
| `app/src/index.html` | modifié — sections « Intérieur » et « Épreuve » |
| `app/src/app.js` | modifié — sélecteur de police, bouton d'épreuve |
| `app/tests/epreuve.test.js` | **créé** — le front de l'épreuve |
| `NOTES.md` | modifié — la dette des ruptures de scène |
| `README.md` | modifié — l'épreuve de l'app |

**Ordre imposé :** la tâche 1 avant la 2 (la liste ne peut pas nommer des polices
absentes du disque), la 2 avant la 3, la 4 avant la 5 (l'épreuve compose des `Bloc`).

---

## Tâche 1 : Les quatre familles dans le jeu embarqué

**Fichiers :**
- Modifier : `app/outils/polices.sh:1-2` et `:22-43`

- [ ] **Étape 1 : ajouter les neuf fichiers à `FICHIERS`**

Dans `app/outils/polices.sh`, à la fin du tableau `FICHIERS` (après la ligne
`"oswald/Oswald[wght].ttf"`) :

```bash
  # Polices de labeur de l'intérieur. Cardo n'a pas de version variable : ses trois
  # coupes sont des fichiers statiques.
  "crimsonpro/CrimsonPro[wght].ttf"
  "crimsonpro/CrimsonPro-Italic[wght].ttf"
  "alegreya/Alegreya[wght].ttf"
  "alegreya/Alegreya-Italic[wght].ttf"
  "vollkorn/Vollkorn[wght].ttf"
  "vollkorn/Vollkorn-Italic[wght].ttf"
  "cardo/Cardo-Regular.ttf"
  "cardo/Cardo-Italic.ttf"
  "cardo/Cardo-Bold.ttf"
```

Et corriger la première ligne du commentaire d'en-tête, qui ne dit plus la vérité :

```bash
# Récupère les polices de l'application dans src-tauri/fonts/ — couverture et
# intérieur.
```

- [ ] **Étape 2 : lancer le script**

```bash
cd /Users/jean-paulgavini/Documents/Dev/ozalid/app && ./outils/polices.sh
```

Attendu : neuf noms de fichiers listés, puis une ligne de total à 29 fichiers.

- [ ] **Étape 3 : vérifier que Typst voit les quatre familles**

```bash
cd app/src-tauri && ./binaries/typst-aarch64-apple-darwin fonts \
  --font-path fonts --ignore-system-fonts | sort
```

Attendu : la liste contient `Alegreya`, `Cardo`, `Crimson Pro`, `Vollkorn` — sous ces
noms exacts, ce sont eux qui iront dans `POLICES_TEXTE`. **Si un nom diffère, c'est le
nom rendu par Typst qui fait foi, pas celui du fichier.**

- [ ] **Étape 4 : composer un spécimen et le regarder**

```bash
cd app/src-tauri && cat > /tmp/spec.typ <<'EOF'
#set page(width: 150mm, height: 120mm, margin: 10mm)
#for f in ("EB Garamond", "Crimson Pro", "Alegreya", "Cardo", "Vollkorn", "Spectral", "Libre Baskerville") {
  set text(font: f, size: 10pt, lang: "fr")
  [#f — romain, #emph[italique], #strong[gras]. Œuvre, châssis, 1913.\ ]
}
EOF
./binaries/typst-aarch64-apple-darwin compile --font-path fonts \
  --ignore-system-fonts --ppi 200 --format png /tmp/spec.typ /tmp/spec.png
```

Ouvrir `/tmp/spec.png` et vérifier **les sept lignes** : chacune doit montrer un romain,
un vrai italique (pas un romain penché) et un gras. Une police qui manque se voit
immédiatement : Typst substitue en silence, la ligne paraît dans un autre caractère.

- [ ] **Étape 5 : commit**

```bash
git add app/outils/polices.sh
git commit -m "polices : Crimson Pro, Alegreya, Cardo et Vollkorn pour l'intérieur"
```

---

## Tâche 2 : `Interieur`, la liste et sa validation

**Fichiers :**
- Modifier : `app/src-tauri/src/interieur.rs:11-14` (imports) et fin de fichier
- Modifier : `app/src-tauri/src/projet.rs:88-96` et `:107-119`
- Test : `app/src-tauri/src/interieur.rs` (module `tests`), `app/src-tauri/src/projet.rs` (module `tests`)

- [ ] **Étape 1 : écrire les tests qui échouent**

Dans le module `tests` de `app/src-tauri/src/interieur.rs`, ajouter :

```rust
    /// Une police que Typst ne connaît pas ne lève aucune erreur à la composition : il
    /// compose dans sa police par défaut, en silence. C'est ainsi que l'intérieur est
    /// resté en Libertinus Serif pendant quatre jalons. Le refus est donc ici, en
    /// amont, ou il n'est nulle part.
    #[test]
    fn une_police_hors_liste_est_refusee_et_non_substituee() {
        let i = Interieur {
            police: "Comic Sans MS".into(),
        };
        let e = i.verifie().unwrap_err();
        assert!(e.contains("Comic Sans MS"), "l'erreur ne nomme pas la police : {e}");
        assert!(e.contains("EB Garamond"), "l'erreur ne dit pas ce qui est attendu : {e}");
    }

    /// Les sept polices offertes doivent toutes passer : une liste qui contient une
    /// entrée que la validation refuse est une porte fermée sur elle-même.
    #[test]
    fn les_polices_offertes_sont_toutes_acceptees() {
        for p in POLICES_TEXTE {
            let i = Interieur {
                police: (*p).into(),
            };
            assert!(i.verifie().is_ok(), "{p} offerte mais refusée");
        }
    }
```

Dans le module `tests` de `app/src-tauri/src/projet.rs`, ajouter :

```rust
    /// Un `.ozalid` écrit avant que la police ne soit réglable doit s'ouvrir, pas être
    /// refusé — même principe que le dos rendu réglable élément par élément.
    #[test]
    fn un_projet_sans_section_interieur_prend_la_police_par_defaut() {
        let toml = r#"
[ozalid]
version = 1

[livre]
titre = "Les Heures creuses"
auteur = "Ivan Pjig"
"#;
        let m: Metadonnees = toml::from_str(toml).expect("projet sans [interieur] refusé");
        assert_eq!(m.interieur.police, "EB Garamond");
    }
```

- [ ] **Étape 2 : lancer les tests pour les voir échouer**

```bash
cd app/src-tauri && cargo test --lib police
```

Attendu : ÉCHEC de compilation — `cannot find type Interieur in this scope`.

- [ ] **Étape 3 : écrire `Interieur` dans `interieur.rs`**

En tête de `app/src-tauri/src/interieur.rs`, après les `use` existants :

```rust
use serde::{Deserialize, Serialize};
```

Puis, juste après le bloc de documentation du module et avant `struct Reglage` :

```rust
/// Les polices que l'intérieur admet.
///
/// Volontairement plus courte que `couverture::POLICES` : ce sont les seules qui
/// tiennent trois cents pages de corps de texte, chacune avec un vrai italique. Un
/// titrage comme Oswald ferait un roman illisible, et l'erreur ne se découvrirait
/// qu'après tirage.
pub const POLICES_TEXTE: &[&str] = &[
    "EB Garamond",
    "Crimson Pro",
    "Alegreya",
    "Cardo",
    "Vollkorn",
    "Spectral",
    "Libre Baskerville",
];

fn police_defaut() -> String {
    "EB Garamond".into()
}

/// Réglages d'intérieur du projet.
///
/// Le prestataire impose le format, les marges, la gouttière et le corps ; le livre
/// choisit son caractère. C'est la raison pour laquelle la police n'est pas un champ
/// de `Provider`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Interieur {
    #[serde(default = "police_defaut")]
    pub police: String,
}

impl Default for Interieur {
    fn default() -> Self {
        Self {
            police: police_defaut(),
        }
    }
}

impl Interieur {
    /// Refuse une police absente de la liste.
    ///
    /// Sans ce contrôle, Typst composerait dans sa police par défaut **sans lever
    /// d'erreur** : `--ignore-system-fonts` empêche une substitution par le système,
    /// pas une substitution par le défaut du binaire.
    pub fn verifie(&self) -> Result<(), String> {
        if POLICES_TEXTE.contains(&self.police.as_str()) {
            return Ok(());
        }
        Err(format!(
            "police d'intérieur inconnue : « {} ». Attendu : {}.",
            self.police,
            POLICES_TEXTE.join(", ")
        ))
    }
}
```

- [ ] **Étape 4 : brancher la section sur `projet.toml`**

Dans `app/src-tauri/src/projet.rs`, ajouter le champ à `Metadonnees` (après
`couverture`) :

```rust
    #[serde(default)]
    pub interieur: crate::interieur::Interieur,
```

Et dans `Projet::nouveau`, après `couverture: Couverture::default(),` :

```rust
                interieur: crate::interieur::Interieur::default(),
```

- [ ] **Étape 5 : lancer les tests**

```bash
cd app/src-tauri && cargo test --lib
```

Attendu : SUCCÈS, 102 tests (99 + 3).

- [ ] **Étape 6 : commit**

```bash
git add app/src-tauri/src/interieur.rs app/src-tauri/src/projet.rs
git commit -m "La police de l'intérieur devient un réglage du projet, validé"
```

---

## Tâche 3 : L'intérieur déclare la police du projet

Cette tâche **déplace le témoin de non-régression**. Le compte de pages des *Heures
creuses* au gabarit Lulu passe de 278 à 264 attendus (263 mesurés + la blanche de
parité). C'est voulu, mesuré, et à relever de nouveau en fin de tâche.

**Fichiers :**
- Modifier : `app/src-tauri/src/interieur.rs:71` (signature) et `:83-106` (l'en-tête de source)
- Modifier : `app/src-tauri/src/package.rs:67-84`
- Modifier : `app/src-tauri/src/commands.rs:206-215`
- Modifier : `app/src-tauri/examples/composer.rs:46-58`

- [ ] **Étape 1 : écrire le test qui échoue**

Dans le module `tests` de `app/src-tauri/src/interieur.rs` :

```rust
    /// La police doit être déclarée, et une seule fois. Deux `#set text(font: …)` dans
    /// la même source, c'est le second qui gagne — donc un réglage qui paraît obéi
    /// alors qu'il ne l'est pas.
    #[test]
    fn la_source_declare_la_police_du_projet_une_seule_fois() {
        let pr = provider("lulu").unwrap();
        let r = Reglage {
            gouttiere: 25.0,
            blanche: false,
        };
        let int = Interieur {
            police: "Cardo".into(),
        };
        let s = source(&livre(), &int, pr, &r, &chapitres());
        assert_eq!(s.matches("font:").count(), 1, "police déclarée {} fois", s.matches("font:").count());
        assert!(s.contains(r#"font: "Cardo""#), "police du projet ignorée");
    }
```

Ce test a besoin d'un jeu de chapitres. Ajouter au module `tests`, à côté de `livre()` :

```rust
    fn chapitres() -> Vec<Chapitre> {
        vec![Chapitre {
            numero: 1,
            titre: "Un".into(),
            paragraphes: vec!["Texte.".into()],
        }]
    }
```

> **Note pour la tâche 4 :** ce helper devra passer à `blocs: vec![Bloc::Paragraphe("Texte.".into())]`.

- [ ] **Étape 2 : lancer le test pour le voir échouer**

```bash
cd app/src-tauri && cargo test --lib la_source_declare
```

Attendu : ÉCHEC de compilation — `source` prend 4 arguments, 5 fournis.

- [ ] **Étape 3 : changer la signature et l'en-tête de source**

Dans `app/src-tauri/src/interieur.rs`, remplacer la signature :

```rust
pub fn source(
    livre: &Livre,
    int: &Interieur,
    pr: &Provider,
    r: &Reglage,
    chapitres: &[Chapitre],
) -> String {
```

Et dans le premier `s.push_str(&format!(...))`, ajouter `font:` à la ligne `#set text` —
la police est validée en amont, donc aucun échappement n'est nécessaire, mais c'est bien
`Interieur::verifie` qui en répond :

```rust
#set text(font: "{}", size: {}pt, lang: "fr", hyphenate: true,
          top-edge: 0.75em, bottom-edge: -0.25em,
          costs: (orphan: 100%, widow: 100%))
```

en insérant `int.police` comme **premier** argument positionnel du `format!`, avant
`echappe(&livre.titre)`.

- [ ] **Étape 4 : mettre à jour les cinq points d'appel**

`app/src-tauri/src/package.rs:67-68` — `assembler` a déjà `projet` sous la main :

```rust
    let int = &projet.meta.interieur;
    int.verifie()?;
    let r = interieur::converge(pr, |reglage| {
        ecrire(&src_int, &interieur::source(livre, int, pr, reglage, &chapitres))?;
        typst.pages(&src_int)
    })?;
```

`app/src-tauri/src/package.rs:81-84` :

```rust
    ecrire(&src_int, &interieur::source(livre, int, pr, &reglage, &chapitres))?;
```

`app/src-tauri/src/commands.rs:190` — ajouter, juste après `let livre = …` :

```rust
    let int = &o.projet.meta.interieur;
    int.verifie()?;
```

puis `:207` et `:215` deviennent :

```rust
        ecrire(&src, &interieur::source(livre, int, pr, reglage, &chapitres))?;
```
```rust
    ecrire(&src, &interieur::source(livre, int, pr, &reglage, &chapitres))?;
```

`app/src-tauri/examples/composer.rs` — ajouter après `let livre = &projet.meta.livre;` :

```rust
    let int = &projet.meta.interieur;
    int.verifie()?;
```

puis les deux appels prennent `int` en second argument.

- [ ] **Étape 5 : lancer les tests**

```bash
cd app/src-tauri && cargo test --lib && cargo clippy --all-targets && cargo fmt --check
```

Attendu : SUCCÈS, 103 tests, clippy sans avertissement, fmt propre.

- [ ] **Étape 6 : relever le nouveau témoin**

```bash
cd app/src-tauri
cargo run --quiet --example importer -- ../../build/LHC/livre.toml /tmp/lhc.ozalid
cargo run --quiet --example packager -- /tmp/lhc.ozalid /tmp/pkg lulu
```

Attendu : la ligne de résultat annonce **264 pages** (et non 278), gouttière 25,0 mm,
dos autour de 16,6 mm. Si le compte diffère de plus d'une page ou deux de 264,
**s'arrêter et le signaler** : la mesure de la spec a été prise sans rejouer la
convergence, un écart franc veut dire autre chose.

- [ ] **Étape 7 : vérifier la police réellement embarquée dans le PDF**

```bash
strings /tmp/pkg/lulu/interieur-lulu.pdf | grep -o "BaseFont */[A-Za-z0-9+#-]*" | sort -u
```

Attendu : des `EBGaramond-*`, et **plus aucun `LibertinusSerif`**. C'est la seule
vérification qui aurait attrapé la dérive d'origine.

- [ ] **Étape 8 : commit**

```bash
git add app/src-tauri/src app/src-tauri/examples
git commit -m "L'intérieur compose dans la police du projet, EB Garamond par défaut"
```

Le message de commit doit porter l'ancien et le nouveau compte de pages : c'est la trace
qui permettra de comprendre, dans six mois, pourquoi le témoin a bougé.

---

## Tâche 4 : `Bloc` — les ruptures de scène conservées

**Fichiers :**
- Modifier : `app/src-tauri/src/manuscrit.rs:12-17` (type), `:117-128` (découpe), `:150-160` et `:165-195` (tests)
- Modifier : `app/src-tauri/src/interieur.rs:163-168` (la boucle) et son module `tests`

- [ ] **Étape 1 : écrire les tests qui échouent**

Dans le module `tests` de `app/src-tauri/src/manuscrit.rs` :

```rust
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
```

Dans le module `tests` de `app/src-tauri/src/interieur.rs` :

```rust
    /// L'intérieur ignore les ruptures de scène — c'est la dette consignée dans la
    /// spec. Le test la fige : le jour où on la corrigera, il tombera, et il faudra
    /// alors relever le nouveau compte de pages sciemment.
    #[test]
    fn l_interieur_compose_a_l_identique_avec_ou_sans_rupture_de_scene() {
        let pr = provider("lulu").unwrap();
        let r = Reglage {
            gouttiere: 25.0,
            blanche: false,
        };
        let int = Interieur::default();
        let sans = vec![Chapitre {
            numero: 1,
            titre: "Un".into(),
            blocs: vec![
                Bloc::Paragraphe("Avant.".into()),
                Bloc::Paragraphe("Après.".into()),
            ],
        }];
        let avec = vec![Chapitre {
            numero: 1,
            titre: "Un".into(),
            blocs: vec![
                Bloc::Paragraphe("Avant.".into()),
                Bloc::Scene,
                Bloc::Paragraphe("Après.".into()),
            ],
        }];
        assert_eq!(
            source(&livre(), &int, pr, &r, &sans),
            source(&livre(), &int, pr, &r, &avec),
            "la rupture de scène a changé l'intérieur"
        );
    }
```

- [ ] **Étape 2 : lancer les tests pour les voir échouer**

```bash
cd app/src-tauri && cargo test --lib scene
```

Attendu : ÉCHEC de compilation — `cannot find type Bloc`, et `Chapitre` n'a pas de champ
`blocs`.

- [ ] **Étape 3 : écrire le type**

Dans `app/src-tauri/src/manuscrit.rs`, remplacer la définition de `Chapitre` :

```rust
/// Un bloc de chapitre.
///
/// Une rupture de scène n'est ni un paragraphe vide ni de la mise en page : c'est une
/// coupure que l'auteur a écrite. Elle est typée pour que chaque composition décide
/// quoi en faire — l'épreuve la rend, l'intérieur ne la rend pas encore.
#[derive(Debug, Clone, PartialEq)]
pub enum Bloc {
    Paragraphe(String),
    Scene,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Chapitre {
    pub numero: u32,
    pub titre: String,
    pub blocs: Vec<Bloc>,
}
```

- [ ] **Étape 4 : garder la rupture à la découpe**

Dans `decoupe`, remplacer la chaîne de conditions (`manuscrit.rs:117-128`) :

```rust
        if let Some(reste) = t.strip_prefix("## ") {
            chapitres.push(entete(reste.trim(), no)?);
        } else if t == "---" {
            // Hors chapitre, la rupture appartient aux liminaires : rien à garder.
            if let Some(courant) = chapitres.last_mut() {
                courant.blocs.push(Bloc::Scene);
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
```

Dans `entete` (`manuscrit.rs:155`), remplacer `paragraphes: Vec::new(),` par
`blocs: Vec::new(),`.

Dans les tests existants du module, remplacer `ch[0].paragraphes.len()` par
`ch[0].blocs.len()` et `assert_eq!(ch[0].paragraphes, vec!["Texte."]);` par
`assert_eq!(ch[0].blocs, vec![Bloc::Paragraphe("Texte.".into())]);`.

- [ ] **Étape 5 : adapter la boucle de l'intérieur**

Dans `app/src-tauri/src/interieur.rs`, l'import devient :

```rust
use crate::manuscrit::{echappe, inline, Bloc, Chapitre};
```

et la boucle des paragraphes (`interieur.rs:165-168`) :

```rust
        // Les ruptures de scène sont ignorées ici : le livre imprimé les perd, dette
        // consignée dans NOTES.md. Les corriger déplacerait le compte de pages de tous
        // les livres déjà composés, ce qui mérite son propre passage.
        for b in &ch.blocs {
            let Bloc::Paragraphe(p) = b else { continue };
            s.push_str(&inline(p));
            s.push_str("\n\n");
        }
```

Adapter aussi le helper `chapitres()` du module `tests` et les deux `Chapitre` littéraux
de `interieur.rs:345-355`, qui passent de `paragraphes: vec!["A.".into()]` à
`blocs: vec![Bloc::Paragraphe("A.".into())]`.

- [ ] **Étape 6 : lancer les tests**

```bash
cd app/src-tauri && cargo test --lib && cargo clippy --all-targets && cargo fmt --check
```

Attendu : SUCCÈS, 106 tests.

- [ ] **Étape 7 : vérifier que le témoin n'a pas bougé**

```bash
cd app/src-tauri && cargo run --quiet --example packager -- /tmp/lhc.ozalid /tmp/pkg2 lulu
```

Attendu : **le même compte de pages qu'à la tâche 3** (264). Si le compte bouge, une
rupture de scène s'est glissée dans la composition de l'intérieur : c'est un défaut, pas
un progrès, et il faut le corriger avant d'aller plus loin.

- [ ] **Étape 8 : commit**

```bash
git add app/src-tauri/src
git commit -m "Les ruptures de scène survivent à la découpe, en blocs typés"
```

---

## Tâche 5 : Le module `epreuve`

La source Typst de cette tâche a été **prototypée et composée** avant l'écriture du
plan : en-tête avec rappel de chapitre, pied `p. n / total`, numéros de ligne remis à
zéro par page, garde datée en français. Les pièges déjà rencontrés y sont corrigés.

**Fichiers :**
- Créer : `app/src-tauri/src/epreuve.rs`
- Modifier : `app/src-tauri/src/lib.rs:1-13`

- [ ] **Étape 1 : déclarer le module**

Dans `app/src-tauri/src/lib.rs`, après `pub mod couverture;` :

```rust
pub mod epreuve;
```

- [ ] **Étape 2 : écrire les tests qui échouent**

Créer `app/src-tauri/src/epreuve.rs` avec, pour l'instant, seulement le module `tests` :

```rust
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
```

- [ ] **Étape 3 : lancer les tests pour les voir échouer**

```bash
cd app/src-tauri && cargo test --lib epreuve
```

Attendu : ÉCHEC de compilation — `cannot find function source`.

- [ ] **Étape 4 : écrire le module**

En tête de `app/src-tauri/src/epreuve.rs`, avant le module `tests` :

```rust
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

/// Marque de rupture de scène. Un blanc seul ne survit pas à une fin de page.
pub const SCENE: &str = "✳";

/// Format de la page, en mm. La marge de droite est celle où l'on écrit.
const MARGE_HAUT: f64 = 25.0;
const MARGE_BAS: f64 = 25.0;
const MARGE_GAUCHE: f64 = 30.0;
const MARGE_DROITE: f64 = 50.0;

/// Source Typst complète de l'épreuve.
pub fn source(livre: &Livre, int: &Interieur, chapitres: &[Chapitre], corps_pt: f64) -> String {
    let titre = echappe(&livre.titre);
    let auteur = echappe(&livre.auteur);
    let police = &int.police;
    let mots: usize = chapitres
        .iter()
        .flat_map(|c| &c.blocs)
        .filter_map(|b| match b {
            Bloc::Paragraphe(p) => Some(p.split_whitespace().count()),
            Bloc::Scene => None,
        })
        .sum();

    let mut s = format!(
        r#"// Épreuve de relecture — {titre}
#set document(title: "{titre}", author: "{auteur}")
#set page(
  width: 210mm, height: 297mm,
  margin: (top: {MARGE_HAUT}mm, bottom: {MARGE_BAS}mm,
           left: {MARGE_GAUCHE}mm, right: {MARGE_DROITE}mm),
  header: context {{
    let n = counter(page).get().first()
    if n <= 1 {{ return }}
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
"#,
        genre = echappe(&livre.genre),
        nb_chapitres = chapitres.len(),
    );

    for (i, ch) in chapitres.iter().enumerate() {
        // Le premier chapitre suit le saut de page de la garde.
        if i > 0 {
            s.push_str("\n#pagebreak()\n");
        }
        let titre_ch = if ch.titre.is_empty() {
            format!("{}", ch.numero)
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
```

> **Piège déjà rencontré, ne pas le réintroduire :** dans l'en-tête, le rappel de
> chapitre doit filtrer sur `h.location().page() <= n` et **non** sur
> `selector(...).before(here())`. Avec `before(here())`, le titre du chapitre qui ouvre
> la page n'est pas encore posé quand l'en-tête est évalué : la page d'ouverture de
> chaque chapitre sort alors sans rappel. Vérifié au prototype.

- [ ] **Étape 5 : lancer les tests**

```bash
cd app/src-tauri && cargo test --lib epreuve && cargo clippy --all-targets && cargo fmt --check
```

Attendu : SUCCÈS, 7 tests d'épreuve.

- [ ] **Étape 6 : commit**

```bash
git add app/src-tauri/src/epreuve.rs app/src-tauri/src/lib.rs
git commit -m "Le module epreuve : le manuscrit sur A4, numéroté et annotable"
```

---

## Tâche 6 : Branchement — exemple CLI, commandes, interface

**Fichiers :**
- Créer : `app/src-tauri/examples/epreuve.rs`
- Modifier : `app/src-tauri/src/commands.rs` (trois commandes), `app/src-tauri/src/lib.rs:19-34`
- Modifier : `app/src/index.html:36-48`, `app/src/app.js`
- Créer : `app/tests/epreuve.test.js`

- [ ] **Étape 1 : l'exemple CLI**

Créer `app/src-tauri/examples/epreuve.rs` :

```rust
//! Tire l'épreuve de relecture d'un projet `.ozalid`, sans interface.
//!
//! C'est le seul moyen de vérifier que Typst avale ce que le module émet — aucun test
//! unitaire ne compile de PDF.
//!
//! Usage : cargo run --example epreuve -- <projet.ozalid> <sortie.pdf> [corps_pt]

use std::path::{Path, PathBuf};

use ozalid_lib::epreuve;
use ozalid_lib::manuscrit;
use ozalid_lib::projet::Projet;
use ozalid_lib::typst::Typst;

fn main() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let (ozalid, sortie) = match (args.next(), args.next()) {
        (Some(a), Some(b)) => (a, b),
        _ => {
            eprintln!("usage : epreuve <projet.ozalid> <sortie.pdf> [corps_pt]");
            std::process::exit(2);
        }
    };
    let corps: f64 = args.next().map_or(Ok(12.0), |c| c.parse()).map_err(|_| {
        "corps illisible : attendu un nombre de points, par exemple 12".to_string()
    })?;

    let projet = Projet::ouvrir(Path::new(&ozalid))?;
    let livre = &projet.meta.livre;
    let int = &projet.meta.interieur;
    int.verifie()?;
    let chapitres = manuscrit::decoupe(&projet.texte, livre.chapitres)?;

    let pdf = PathBuf::from(&sortie);
    let src = pdf.with_extension("typ");
    std::fs::write(&src, epreuve::source(livre, int, &chapitres, corps))
        .map_err(|e| format!("{} : {e}", src.display()))?;

    let typst = Typst::new("binaries/typst-aarch64-apple-darwin").avec_polices("fonts");
    typst.compile(&src, &pdf)?;
    println!(
        "{} — {} chapitres, {} en {corps} pt",
        pdf.display(),
        chapitres.len(),
        int.police
    );
    Ok(())
}
```

- [ ] **Étape 2 : composer une épreuve réelle et la regarder**

```bash
cd app/src-tauri && cargo run --example epreuve -- /tmp/lhc.ozalid /tmp/epreuve.pdf
```

Puis rendre trois pages en PNG et **les ouvrir** :

```bash
./binaries/typst-aarch64-apple-darwin compile --font-path fonts --ignore-system-fonts \
  --pages 1-3 --ppi 130 --format png /tmp/epreuve.typ "/tmp/ep-{p}.png"
```

À vérifier à l'œil, page par page :
- page 1, la garde : ni folio, ni numéro de ligne, date **en français**, compte de
  chapitres et de mots, l'avertissement en italique en bas ;
- page 2 : le rappel de chapitre **présent en haut à droite dès la page d'ouverture**,
  les numéros de ligne à gauche partant de 1, la marge de droite vide sur 50 mm, le pied
  `p. 2 / n` ;
- une page portant une rupture de scène : l'astérisque centré, avec du blanc autour.

- [ ] **Étape 3 : les trois commandes Tauri**

Dans `app/src-tauri/src/commands.rs`, à la suite de `polices_liste` :

```rust
#[tauri::command]
pub fn polices_texte_liste() -> Vec<&'static str> {
    interieur::POLICES_TEXTE.to_vec()
}

#[tauri::command]
pub fn interieur_modifier(
    interieur: Interieur,
    atelier: State<Atelier>,
) -> Result<ProjetVue, String> {
    interieur.verifie()?;
    let mut garde = atelier.ouvert.lock().unwrap();
    let o = garde.as_mut().ok_or_else(aucun_projet)?;
    o.projet.meta.interieur = interieur;
    vue(o)
}

/// Tire l'épreuve de relecture à la racine des sorties : elle ne vise aucun éditeur,
/// elle ne descend donc pas dans un répertoire de prestataire.
#[tauri::command]
pub fn epreuve_tirer(corps_pt: f64, atelier: State<Atelier>) -> Result<String, String> {
    let garde = atelier.ouvert.lock().unwrap();
    let o = garde.as_ref().ok_or_else(aucun_projet)?;
    let livre = &o.projet.meta.livre;
    let int = &o.projet.meta.interieur;
    int.verifie()?;
    let chapitres = manuscrit::decoupe(&o.projet.texte, livre.chapitres)?;

    let dossier = sorties_racine(o)?;
    std::fs::create_dir_all(&dossier)
        .map_err(|e| format!("répertoire de sortie inutilisable ({}) : {e}", dossier.display()))?;
    let src = dossier.join("epreuve.typ");
    ecrire(&src, &epreuve::source(livre, int, &chapitres, corps_pt))?;
    let pdf = dossier.join("epreuve.pdf");
    typst()?.compile(&src, &pdf)?;
    Ok(pdf.to_string_lossy().into_owned())
}
```

`sorties_racine` n'existe pas encore. Remplacer `sorties_dossier`
(`commands.rs:446-458`) par ces deux fonctions — la seconde n'est plus que la première
suivie d'un `join` :

```rust
/// Racine des sorties : un répertoire du nom du projet, à côté du `.ozalid`. L'épreuve
/// s'y range directement — elle ne vise aucun éditeur.
fn sorties_racine(o: &Ouvert) -> Result<PathBuf, String> {
    let chemin = o.chemin.as_ref().ok_or_else(|| {
        "enregistrer le projet avant de composer : les sorties se rangent à côté du \
         fichier .ozalid."
            .to_string()
    })?;
    let parent = chemin.parent().unwrap_or(Path::new("."));
    let nom = chemin
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "projet".into());
    Ok(parent.join(nom))
}

/// Sorties d'un prestataire : un répertoire par prestataire, sous la racine.
fn sorties_dossier(o: &Ouvert, provider: &str) -> Result<PathBuf, String> {
    Ok(sorties_racine(o)?.join(provider))
}
```

Ajouter `Interieur` aux `use` de `commands.rs` (`use crate::interieur::{self, Interieur, Reglage};`
selon ce qui y est déjà importé), et `use crate::epreuve;`.

Puis déclarer les trois commandes dans `app/src-tauri/src/lib.rs`, dans
`generate_handler!`, après `commands::polices_liste` :

```rust
            commands::polices_texte_liste,
            commands::interieur_modifier,
            commands::epreuve_tirer,
```

Et exposer `interieur` dans `ProjetVue` (`commands.rs:78-89`) :

```rust
    pub interieur: Interieur,
```

renseigné dans `vue()` :

```rust
        interieur: o.projet.meta.interieur.clone(),
```

- [ ] **Étape 4 : vérifier que tout compile**

```bash
cd app/src-tauri && cargo test --lib && cargo clippy --all-targets && cargo fmt --check
```

Attendu : SUCCÈS, clippy sans avertissement.

- [ ] **Étape 5 : les deux sections de l'interface**

Dans `app/src/index.html`, après la section `secManuscrit` (ligne 47) :

```html
  <section id="secInterieur" hidden>
    <h2>Intérieur</h2>
    <label><span>Police</span><select id="inPoliceInterieur"></select></label>
    <p class="note">La police fixe la pagination, donc l'épaisseur du dos : en changer
      recompose l'intérieur et redimensionne la planche.</p>
  </section>
```

Et après la section `secPackages` :

```html
  <section id="secEpreuve" hidden>
    <h2>Épreuve</h2>
    <p class="note">Le manuscrit sur A4, fer à gauche, avec les numéros de ligne et une
      marge pour annoter. Ce n'est pas le livre : c'est de quoi le relire. Les numéros de
      ligne ne valent que pour ce tirage-là.</p>
    <label><span>Corps</span>
      <input type="number" id="inEpreuveCorps" min="8" max="18" step="0.5" value="12"></label>
    <div class="ligne">
      <button id="btEpreuve" type="button">Tirer une épreuve</button>
      <span id="etatEpreuve" class="etat"></span>
    </div>
    <p class="chemin" id="cheminEpreuve"></p>
  </section>
```

- [ ] **Étape 6 : le front**

Dans `app/src/app.js`, dans `chargerProviders()`, après
`polices = await invoke('polices_liste');` :

```js
  for (const p of await invoke('polices_texte_liste')) {
    $('inPoliceInterieur').append(new Option(p, p));
  }
```

Dans `afficherProjet` (là où `$('inTitre').value = p.livre.titre;` et consorts, vers
`app.js:79-84`), ajouter :

```js
  $('inPoliceInterieur').value = p.interieur.police;
```

et ouvrir les deux nouvelles sections en les ajoutant à la liste d'`app.js:75` :

```js
  for (const s of ['secLivre', 'secManuscrit', 'secInterieur', 'secCouverture',
                   'secComposer', 'secPackages', 'secEpreuve']) {
    $(s).hidden = false;
  }
```

Puis les deux écouteurs, à côté de ceux déjà branchés :

```js
$('inPoliceInterieur').addEventListener('change', () => tente(async () =>
  afficherProjet(await invoke('interieur_modifier', {
    interieur: { police: $('inPoliceInterieur').value },
  }))));

$('btEpreuve').addEventListener('click', () => tente(async () => {
  $('etatEpreuve').textContent = 'composition…';
  const pdf = await invoke('epreuve_tirer', { corpsPt: Number($('inEpreuveCorps').value) });
  $('etatEpreuve').textContent = '';
  $('cheminEpreuve').textContent = pdf;
}));
```

> Lire d'abord comment `tente` et `etat` sont employés pour `btComposer` et `btPackager`
> (`app.js`), et calquer : la gestion d'erreur et la remise à zéro de l'état doivent être
> les mêmes. Ne pas inventer un second motif.

- [ ] **Étape 7 : réparer les trois `faux` existants — à faire avant d'écrire quoi que ce soit**

`polices_texte_liste` est appelée au démarrage, dans `chargerProviders()`. Or les trois
fichiers de tests front (`composition.test.js`, `couverture.test.js`,
`packages.test.js`) définissent chacun leur propre `faux`, qui lève
`commande inattendue : …` sur toute commande qu'il ne connaît pas. **Les 31 tests
existants tombent donc tous** tant que les trois ne l'ont pas apprise.

Dans chacun des trois fichiers, à côté de la ligne `polices_liste` du `faux` :

```js
    if (cmd === 'polices_texte_liste') return ['EB Garamond', 'Alegreya', 'Cardo'];
```

Et dans chacun, ajouter au fixture `PROJET` (et à tout autre projet simulé du fichier) :

```js
  interieur: { police: 'Alegreya' },
```

Ajouter enfin `'secInterieur'`, `'inPoliceInterieur'`, `'secEpreuve'`,
`'inEpreuveCorps'`, `'btEpreuve'`, `'etatEpreuve'` et `'cheminEpreuve'` au tableau `IDS`
de chacun des trois fichiers : le shim ne crée que les éléments qu'on lui nomme.

Lancer `cd app && node --test "tests/*.test.js"` : les 31 tests doivent repasser au vert
**avant** d'en écrire de nouveaux.

- [ ] **Étape 8 : écrire les tests de l'épreuve**

Créer `app/tests/epreuve.test.js`, en reprenant l'en-tête de `composition.test.js`
(`IDS`, `LULU`, `PROJET`, `faux`) :

```js
test("la police d'intérieur du projet est celle qui paraît au panneau", async () => {
  const { els } = await charge({
    ids: IDS,
    invoke: faux([LULU], { projet_importer: PROJET }),
    open: async () => '/dev/ozalid/build/LHC/livre.toml',
  });
  await els.get('btImporter').declenche('click');
  assert.strictEqual(els.get('inPoliceInterieur').value, 'Alegreya');
});

// Le réglage doit atteindre le Rust : un sélecteur qui change d'apparence sans rien
// enregistrer laisserait composer dans une autre police que celle qu'on voit.
test("changer la police enregistre le réglage dans le projet", async () => {
  let recu = null;
  const { els } = await charge({
    ids: IDS,
    invoke: faux([LULU], {
      projet_importer: PROJET,
      interieur_modifier: (args) => {
        recu = args.interieur;
        return { ...PROJET, interieur: args.interieur };
      },
    }),
    open: async () => '/dev/ozalid/build/LHC/livre.toml',
  });
  await els.get('btImporter').declenche('click');
  els.get('inPoliceInterieur').value = 'Cardo';
  await els.get('inPoliceInterieur').declenche('change');
  assert.deepStrictEqual(recu, { police: 'Cardo' });
});

// L'épreuve ne dépend d'aucune pagination ni d'aucun prestataire : elle doit pouvoir
// être tirée dès qu'un manuscrit est là, sans intérieur composé au préalable.
test("l'épreuve se tire sans intérieur composé", async () => {
  let corps = null;
  const { els } = await charge({
    ids: IDS,
    invoke: faux([LULU], {
      projet_importer: PROJET,
      epreuve_tirer: (args) => {
        corps = args.corpsPt;
        return '/livres/LHC/epreuve.pdf';
      },
    }),
    open: async () => '/dev/ozalid/build/LHC/livre.toml',
  });
  await els.get('btImporter').declenche('click');
  await els.get('btEpreuve').declenche('click');
  assert.strictEqual(corps, 12);
  assert.strictEqual(els.get('cheminEpreuve').textContent, '/livres/LHC/epreuve.pdf');
  assert.strictEqual(els.get('etatEpreuve').textContent, '');
});
```

- [ ] **Étape 9 : lancer les tests front**

```bash
cd app && node --test "tests/*.test.js"
```

Attendu : SUCCÈS, 34 tests (31 + 3).

- [ ] **Étape 10 : commit**

```bash
git add app/src-tauri app/src app/tests
git commit -m "L'épreuve dans l'interface, et la police d'intérieur au panneau"
```

---

## Tâche 7 : Vérification de bout en bout et documentation

**Fichiers :**
- Modifier : `NOTES.md` (section 4, dette de code)
- Modifier : `README.md` (section Épreuve de lecture)
- Modifier : `app/README.md` (état des jalons)

- [ ] **Étape 1 : consigner la dette dans `NOTES.md`**

Dans la section « 4. Dette de code encore ouverte », ajouter :

```markdown
**Les ruptures de scène n'atteignent pas le livre imprimé.** Le manuscrit les note
`---` ; depuis les blocs typés, la découpe les conserve et l'épreuve de relecture les
compose, mais `interieur::source` les ignore encore. Deux scènes séparées s'impriment
donc collées, en alinéas consécutifs : le blanc que l'auteur a écrit disparaît. Les
rendre déplacerait le compte de pages de tous les livres déjà composés — le témoin de
non-régression du projet — ce qui mérite un passage à part, mesuré. Un test
(`l_interieur_compose_a_l_identique_avec_ou_sans_rupture_de_scene`) fige l'état actuel :
il tombera le jour où on s'y mettra, et c'est voulu.
```

- [ ] **Étape 2 : mettre le README à jour**

Dans `README.md`, la section « Épreuve de lecture » décrit `roman_pdf.py`, qui produit
une épreuve **poche** de N chapitres. Elle n'est pas remplacée par l'épreuve de l'app :
ce sont deux documents différents. Ajouter, sous la section existante, un paragraphe qui
distingue les deux — l'épreuve poche fait lire, l'épreuve A4 fait corriger — et renvoyer
à `app/README.md` pour la seconde.

Dans `app/README.md`, remplacer « Reste l'épreuve de lecture et la release Windows » par
l'état réel après cette tâche.

- [ ] **Étape 3 : la vérification complète**

```bash
cd app/src-tauri && cargo test --lib && cargo clippy --all-targets && cargo fmt --check
cd .. && node --test "tests/*.test.js"
cd src-tauri && cargo run --quiet --example packager -- /tmp/lhc.ozalid /tmp/pkg-final lulu tbe-110x170 bookvault-127x203
cargo run --quiet --example epreuve -- /tmp/lhc.ozalid /tmp/epreuve-finale.pdf
```

Attendu : tous les tests au vert, les trois packages composés, l'épreuve compilée.
**Relever le compte de pages Lulu** et le comparer à celui de la tâche 3 : il ne doit pas
avoir bougé depuis.

- [ ] **Étape 4 : la vérification qu'aucun test ne remplace**

Rendre et **regarder** :
- une page d'épreuve avec rupture de scène ;
- la garde ;
- la première page de l'intérieur Lulu, pour constater le changement de caractère.

- [ ] **Étape 5 : commit**

```bash
git add NOTES.md README.md app/README.md
git commit -m "Documentation : l'épreuve A4, et la dette des ruptures de scène"
```

---

## Ce qui reste hors de ce plan

- Corriger les ruptures de scène de l'intérieur (dette consignée en tâche 7).
- Rendre le corps et l'interligne de l'intérieur réglables : ils restent au prestataire.
- Ajouter les quatre nouvelles familles à la liste de couverture.
- La release Windows, second volet du jalon 5.
