# Identité du livre — lot 1 : la substitution

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Les champs libres du livre — titre de la page de titre, dédicace, copyright — citent les champs clés par des jetons `%TITRE%`, `%AUTEUR%`, `%GENRE%`, résolus à la composition.

**Architecture:** Un module `gabarit` porte une fonction pure `substituer`. `Livre` expose ses champs libres par des accesseurs qui la traversent, selon le motif déjà en place — le champ `titre_page` et la méthode `titre_page()` coexistent aujourd'hui. Les tests sont posés **aux points de sortie** (source Typst de l'intérieur, conversion vers l'EPUB) et non sur `substituer` seule : c'est l'oubli d'un appelant qu'ils doivent attraper, pas l'algorithme.

**Tech Stack:** Rust 2021, `serde`, `toml`, `cargo test` ; front vanilla, `node --test`.

**Ce que ce lot ne fait pas :** aucun champ ne change de place, `VERSION` reste à 2, et les valeurs par défaut génériques (« Titre », « Auteur »…) attendent le lot 3. Un `.ozalid` écrit après ce lot reste lisible par le binaire actuel.

Spec : `docs/superpowers/specs/2026-08-23-identite-du-livre-design.md`.

---

## Fichiers

| Fichier | Rôle |
|---|---|
| `app/src-tauri/src/gabarit.rs` | **créé** — `substituer`, la table des jetons, ses tests |
| `app/src-tauri/src/lib.rs:1` | déclare `pub mod gabarit;` |
| `app/src-tauri/src/projet.rs:45-100` | `Livre` : `titre_page` et `dedicace` passent en `String`, trois accesseurs |
| `app/src-tauri/src/interieur.rs:350,404,411` | lit les accesseurs |
| `app/src-tauri/src/ebook.rs:71-78` | conversion vers `epub::Livre`, isolée pour être testable |
| `app/src-tauri/src/import.rs:517` | test existant à ajuster (`String` au lieu de `&str`) |
| `app/src/index.html:54` | l'indication de `inTitrePage` |
| `app/src/app.js:387,392,728-740` | affichage et collecte des deux champs |

**Commandes de vérification**, depuis `app/src-tauri/` sauf mention contraire :

```
cargo test --lib
cargo clippy --all-targets -- -D warnings
cargo fmt --check
cd .. && node --test "tests/*.test.js"
```

---

## Tâche 1 : `gabarit::substituer`

**Files:**
- Create: `app/src-tauri/src/gabarit.rs`
- Modify: `app/src-tauri/src/lib.rs:1`

- [ ] **Step 1: Créer le module vide et le déclarer**

Créer `app/src-tauri/src/gabarit.rs` :

```rust
//! Les jetons `%CLE%` des champs libres du livre.
//!
//! Un champ libre — le titre de la page de titre, la dédicace, le copyright — peut
//! citer un champ clé. La substitution se fait **à la composition**, jamais à la
//! saisie : le `.ozalid` conserve le texte à jetons, qui doit suivre le livre si le
//! titre change.

use crate::projet::Livre;

/// Les jetons reconnus, et le champ clé que chacun désigne.
///
/// Les clés sont littérales par définition : aucune n'est elle-même substituée, et
/// c'est ce qui rend la cascade impossible sans avoir à s'en garder.
const JETONS: [(&str, fn(&Livre) -> &str); 3] = [
    ("%TITRE%", |l| &l.titre),
    ("%AUTEUR%", |l| &l.auteur),
    ("%GENRE%", |l| &l.genre),
];
```

Ajouter dans `app/src-tauri/src/lib.rs`, en gardant l'ordre alphabétique des modules
— après `pub mod epub;` et avant `pub mod image;` :

```rust
pub mod gabarit;
```

- [ ] **Step 2: Écrire les tests, avant la fonction**

Ajouter à la fin de `app/src-tauri/src/gabarit.rs` :

```rust
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
        assert_eq!(substituer("Tous droits réservés.", &livre()), "Tous droits réservés.");
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

    /// **Le test qui compte.** Une valeur de clé qui ressemble à un jeton ne doit pas
    /// être substituée à son tour : une seconde passe ouvrirait la porte à la cascade,
    /// et un titre malencontreux ferait dire au copyright autre chose que ce qui est
    /// écrit.
    #[test]
    fn la_substitution_ne_cascade_pas() {
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
```

- [ ] **Step 3: Vérifier que ça ne compile pas**

```
cargo test --lib gabarit
```

Attendu : ÉCHEC de compilation, `cannot find function 'substituer' in this scope`.

- [ ] **Step 4: Écrire `substituer`**

Insérer dans `app/src-tauri/src/gabarit.rs`, entre `JETONS` et le module de tests :

```rust
/// Remplace les jetons connus par la valeur de leur champ clé.
///
/// **Une seule passe.** Le texte est parcouru une fois de gauche à droite : ce qu'un
/// jeton produit n'est jamais réexaminé. Un `replace` par jeton en boucle aurait
/// l'air équivalent et ne l'est pas — il resubstituerait la valeur du précédent.
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
```

- [ ] **Step 5: Vérifier que les tests passent**

```
cargo test --lib gabarit
```

Attendu : 6 tests, tous verts.

- [ ] **Step 6: Commit**

```bash
git add app/src-tauri/src/gabarit.rs app/src-tauri/src/lib.rs
git commit -m "Un champ libre peut citer une clé du livre"
```

---

## Tâche 2 : le titre de la page de titre porte son jeton

`titre_page` est aujourd'hui un `Option<String>` dont l'absence veut dire « le titre
sert ». Ce repli invisible devient le jeton `%TITRE%` : la même chose, montrée dans le
champ et retouchable.

**Files:**
- Modify: `app/src-tauri/src/projet.rs:45-100`
- Modify: `app/src-tauri/src/interieur.rs:350`
- Modify: `app/src-tauri/src/ebook.rs:71-78`
- Modify: `app/src-tauri/src/import.rs:517`

- [ ] **Step 1: Écrire les tests, dans `projet.rs`**

Ajouter au module `tests` de `app/src-tauri/src/projet.rs` :

```rust
/// Le repli d'autrefois — `titre_page` absent, le titre sert — devient un jeton. Un
/// `.ozalid` écrit avant ce lot doit donc s'ouvrir avec `%TITRE%` et composer comme
/// avant, sans que `VERSION` ait bougé.
#[test]
fn un_projet_sans_titre_de_page_recoit_le_jeton() {
    let toml = r#"
[ozalid]
version = 2

[livre]
titre = "Les Heures creuses"
auteur = "Ivan Pjig"
"#;
    let m: Metadonnees = toml::from_str(toml).expect("projet sans titre_page refusé");
    assert_eq!(m.livre.titre_page, "%TITRE%");
    assert_eq!(m.livre.titre_page(), "Les Heures creuses");
}

/// Un titre de page saisi à la main, avec ses sauts de ligne voulus, ne doit pas être
/// touché par la substitution.
#[test]
fn un_titre_de_page_ecrit_a_la_main_est_rendu_tel_quel() {
    let mut l = Livre::vide();
    l.titre = "Les Heures creuses".into();
    l.titre_page = "Les Heures\ncreuses".into();
    assert_eq!(l.titre_page(), "Les Heures\ncreuses");
}
```

- [ ] **Step 2: Vérifier l'échec**

```
cargo test --lib projet::tests::un_projet_sans_titre_de_page_recoit_le_jeton
```

Attendu : ÉCHEC de compilation — `titre_page` est un `Option<String>`, la comparaison
avec `"%TITRE%"` ne type-checke pas.

- [ ] **Step 3: Changer le champ et l'accesseur**

Dans `app/src-tauri/src/projet.rs`, remplacer le champ et sa documentation :

```rust
    /// Titre de la page de titre, avec ses sauts de ligne voulus.
    ///
    /// Vaut `%TITRE%` par défaut, ce qui reproduit l'ancien repli — le titre sert — en
    /// le rendant visible dans le champ et retouchable. Un `.ozalid` écrit avant le
    /// jeton reçoit ce défaut ; `VERSION` n'a donc pas à bouger.
    #[serde(default = "titre_page_defaut")]
    pub titre_page: String,
```

Ajouter la fonction de défaut, à côté de `genre_defaut` :

```rust
fn titre_page_defaut() -> String {
    "%TITRE%".into()
}
```

Dans `impl Livre`, remplacer l'accesseur :

```rust
    /// Titre tel qu'il doit paraître sur la page de titre, jetons résolus.
    pub fn titre_page(&self) -> String {
        crate::gabarit::substituer(&self.titre_page, self)
    }
```

Dans `Livre::vide()`, remplacer `titre_page: None,` par :

```rust
            titre_page: titre_page_defaut(),
```

- [ ] **Step 4: Ajuster les trois appelants**

`app/src-tauri/src/interieur.rs:350` compile tel quel — `livre.titre_page()` était déjà
suivi d'un `.replace`, qui produisait déjà une `String`. Le vérifier, ne rien changer.

`app/src-tauri/src/ebook.rs`, dans la construction de `epub::Livre` : le champ attend
un `&'a str`, l'accesseur rend une `String`. Introduire une liaison juste avant :

```rust
    let titre_page = livre.titre_page();
    let livre_epub = epub::Livre {
        titre: &livre.titre,
        titre_page: &titre_page,
        auteur: &livre.auteur,
        genre: &livre.genre,
        copyright: &livre.copyright,
        dedicace: livre.dedicace(),
    };
```

`app/src-tauri/src/import.rs:517` — `assert_eq!(l.titre_page(), "Les Heures\ncreuses")`
continue de passer, `String` et `&str` se comparant. Le vérifier, ne rien changer.

L'aide `livre()` du module de tests de `interieur.rs` (vers la ligne 500) énumère tous
les champs : y remplacer `titre_page: Some("Les Heures\ncreuses".into()),` par
`titre_page: "Les Heures\ncreuses".into(),`.

Chercher tout autre appelant avant d'aller plus loin :

```
grep -rn "titre_page" app/src-tauri/src/
```

- [ ] **Step 5: Vérifier que les tests passent**

```
cargo test --lib
```

Attendu : vert, y compris les deux tests neufs.

- [ ] **Step 6: Commit**

```bash
git add app/src-tauri/src/projet.rs app/src-tauri/src/ebook.rs
git commit -m "Le titre de la page de titre dit d'où il vient"
```

---

## Tâche 3 : la dédicace

**Files:**
- Modify: `app/src-tauri/src/projet.rs:45-100,1277-1283`
- Modify: `app/src-tauri/src/interieur.rs:411`
- Modify: `app/src-tauri/src/ebook.rs:71-78`

- [ ] **Step 1: Écrire le test**

Ajouter au module `tests` de `app/src-tauri/src/projet.rs` :

```rust
/// Une dédicace peut citer le livre. Le rognage et le filtre du blanc restent en
/// place, et s'appliquent **après** la substitution : un jeton dont la clé est vide ne
/// doit pas composer une page pour rien.
#[test]
fn une_dedicace_cite_les_cles_puis_est_rognee() {
    let mut l = Livre::vide();
    l.auteur = "Ivan Pjig".into();
    l.dedicace = "  Pour %AUTEUR%.  ".into();
    assert_eq!(l.dedicace().as_deref(), Some("Pour Ivan Pjig."));

    l.auteur = String::new();
    l.dedicace = "  %AUTEUR%  ".into();
    assert_eq!(l.dedicace(), None, "une clé vide ne doit pas coûter deux pages");
}
```

- [ ] **Step 2: Vérifier l'échec**

```
cargo test --lib projet::tests::une_dedicace_cite_les_cles_puis_est_rognee
```

Attendu : ÉCHEC de compilation — `dedicace` est un `Option<String>`, l'affectation
d'une `&str` ne type-checke pas.

- [ ] **Step 3: Changer le champ et l'accesseur**

Dans `app/src-tauri/src/projet.rs`, remplacer le champ :

```rust
    /// Dédicace imprimée, en belle page après le copyright. Vide, aucune page n'est
    /// composée : c'est `dedicace()` qui en juge, pas ses appelants.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub dedicace: String,
```

Remplacer l'accesseur :

```rust
    /// La dédicace, jetons résolus, si elle n'est pas que du blanc.
    ///
    /// Le rognage est ici et nulle part ailleurs : une dédicace réduite à une espace
    /// ajouterait sinon deux pages au livre, donc du dos, sans que rien ne se voie à
    /// l'écran. Il vient **après** la substitution, pour qu'un jeton dont la clé est
    /// vide ne compose pas davantage.
    pub fn dedicace(&self) -> Option<String> {
        let d = crate::gabarit::substituer(&self.dedicace, self);
        let d = d.trim();
        (!d.is_empty()).then(|| d.to_string())
    }
```

Dans `Livre::vide()`, remplacer `dedicace: None,` par :

```rust
            dedicace: String::new(),
```

- [ ] **Step 4: Ajuster les appelants**

`app/src-tauri/src/interieur.rs:411` — `d` est maintenant une `String` :

```rust
    if let Some(d) = livre.dedicace() {
        s.push_str(&format!(
            r#"#v(48mm)
#align(right, emph(text(size: 9.5pt)[{}]))
#pagebreak()
#pagebreak()

"#,
            echappe(&d).replace('\n', r" \ ")
        ));
```

`app/src-tauri/src/ebook.rs` — `epub::Livre.dedicace` attend `Option<&'a str>`.
Compléter la liaison posée à la tâche 2 :

```rust
    let titre_page = livre.titre_page();
    let dedicace = livre.dedicace();
    let livre_epub = epub::Livre {
        titre: &livre.titre,
        titre_page: &titre_page,
        auteur: &livre.auteur,
        genre: &livre.genre,
        copyright: &livre.copyright,
        dedicace: dedicace.as_deref(),
    };
```

Les tests existants qui affectent la dédicace passent d'un `Some("…".into())` à une
chaîne nue, et ceux qui la comparent gagnent un `.as_deref()`. Les trouver :

```
grep -rn "dedicace" app/src-tauri/src/ app/src-tauri/examples/
```

Traiter notamment `projet.rs:1277-1283` (`une_dedicace_de_blanc_equivaut_a_pas_de_dedicace`),
`interieur.rs:507,947,969,984` et `epreuve.rs:175`.

- [ ] **Step 5: Vérifier que les tests passent**

```
cargo test --lib
```

Attendu : vert. `une_dedicace_ajoute_une_belle_page_et_sa_blanche` et
`une_dedicace_vide_ou_blanche_ne_compose_rien` doivent passer **sans avoir été
retouchés dans leur intention** — seule la forme du champ change.

- [ ] **Step 6: Commit**

```bash
git add app/src-tauri/src/projet.rs app/src-tauri/src/interieur.rs app/src-tauri/src/ebook.rs app/src-tauri/src/epreuve.rs
git commit -m "La dédicace peut nommer l'auteur sans le réécrire"
```

---

## Tâche 4 : le copyright, et les tests aux points de sortie

C'est la tâche qui donne son sens au lot. `copyright` reste une `String` — seul un
accesseur s'ajoute — mais rien dans le type n'empêche un appelant de lire le champ
brut. Les tests sont donc posés là où le texte part vers un fichier.

**Files:**
- Modify: `app/src-tauri/src/projet.rs`
- Modify: `app/src-tauri/src/interieur.rs:404`
- Modify: `app/src-tauri/src/ebook.rs:71-78`

- [ ] **Step 1: Écrire les tests de sortie**

Dans `app/src-tauri/src/interieur.rs`, au module `tests` :

```rust
/// **Point de sortie : le PDF de l'intérieur.** Aucun jeton ne doit survivre à la
/// composition — un `%AUTEUR%` qui passe ici s'imprime dans le livre.
///
/// Le test porte sur la source entière, et non sur le seul copyright : il doit casser
/// le jour où un champ libre de plus est branché sans passer par son accesseur.
#[test]
fn aucun_jeton_ne_survit_a_la_source_de_l_interieur() {
    let mut l = livre();
    l.titre = "Les Heures creuses".into();
    l.auteur = "Ivan Pjig".into();
    l.genre = "roman".into();
    l.titre_page = "%TITRE%".into();
    l.copyright = "© %AUTEUR%, 2026.\nTous droits réservés.".into();
    l.dedicace = "Pour %AUTEUR%.".into();

    let pr = provider("bod").unwrap();
    let r = Reglage { gouttiere: 20.0, blanche: false };
    let src = source(&l, &Interieur::default(), pr, &r, &chapitres(), None);

    for jeton in ["%TITRE%", "%AUTEUR%", "%GENRE%"] {
        assert!(!src.contains(jeton), "{jeton} a traversé la composition");
    }
    assert!(src.contains("Ivan Pjig"), "la valeur n'a pas remplacé le jeton");
    assert!(src.contains("Les Heures creuses"));
}
```

`livre()`, `chapitres()` et `provider` sont déjà en place dans ce module de tests
(`interieur.rs:500`, `:519`, et `use crate::providers::provider;`). `livre()` part de
`Livre { … }` en énumérant tous les champs : les tâches 2 et 3 l'ont déjà mise à jour —
`titre_page: "Les Heures\ncreuses".into()` et `dedicace: String::new()`. Le test
ci-dessus repart d'elle et écrase ce dont il a besoin.

Dans `app/src-tauri/src/ebook.rs`, au module `tests` :

```rust
/// **Point de sortie : l'EPUB.** La conversion vers `epub::Livre` est le seul endroit
/// où les champs libres du livre entrent dans le fichier du lecteur. Un jeton qui la
/// traverse arrive chez qui lit.
#[test]
fn aucun_jeton_ne_survit_a_la_conversion_epub() {
    let mut l = crate::projet::Livre::vide();
    l.titre = "Les Heures creuses".into();
    l.auteur = "Ivan Pjig".into();
    l.genre = "roman".into();
    l.titre_page = "%TITRE%".into();
    l.copyright = "© %AUTEUR%, 2026.".into();
    l.dedicace = "Pour %AUTEUR%.".into();

    let (titre_page, copyright, dedicace) = libres(&l);
    for texte in [&titre_page, &copyright, dedicace.as_ref().unwrap()] {
        for jeton in ["%TITRE%", "%AUTEUR%", "%GENRE%"] {
            assert!(!texte.contains(jeton), "{jeton} a traversé : {texte}");
        }
    }
    assert_eq!(titre_page, "Les Heures creuses");
    assert_eq!(copyright, "© Ivan Pjig, 2026.");
}
```

- [ ] **Step 2: Vérifier l'échec**

```
cargo test --lib aucun_jeton
```

Attendu : ÉCHEC de compilation — `libres` n'existe pas ; et une fois `libres` posée,
ÉCHEC du test de l'intérieur, `%AUTEUR%` traversant le copyright.

- [ ] **Step 3: Ajouter l'accesseur du copyright**

Dans `impl Livre`, `app/src-tauri/src/projet.rs` :

```rust
    /// Le copyright, jetons résolus.
    pub fn copyright(&self) -> String {
        crate::gabarit::substituer(&self.copyright, self)
    }
```

- [ ] **Step 4: Brancher les deux points de sortie**

`app/src-tauri/src/interieur.rs:404` :

```rust
        echappe(&livre.copyright()).replace('\n', r" \ ")
```

Dans `app/src-tauri/src/ebook.rs`, isoler la conversion — elle devient testable, et
c'est tout son objet :

```rust
/// Les trois champs libres que l'EPUB reçoit, jetons résolus.
///
/// Isolée pour être testable : `epub::Livre` emprunte ses champs, la substitution doit
/// donc produire des valeurs qui vivent plus longtemps que lui, et c'est exactement
/// l'endroit où un oubli enverrait un `%AUTEUR%` dans le fichier du lecteur.
fn libres(livre: &crate::projet::Livre) -> (String, String, Option<String>) {
    (livre.titre_page(), livre.copyright(), livre.dedicace())
}
```

et l'appeler à la place des liaisons posées aux tâches 2 et 3 :

```rust
    let (titre_page, copyright, dedicace) = libres(livre);
    let livre_epub = epub::Livre {
        titre: &livre.titre,
        titre_page: &titre_page,
        auteur: &livre.auteur,
        genre: &livre.genre,
        copyright: &copyright,
        dedicace: dedicace.as_deref(),
    };
```

- [ ] **Step 5: Vérifier que les tests passent**

```
cargo test --lib
```

Attendu : vert.

- [ ] **Step 6: Voir le test de sortie échouer sur une mutation**

Un test qui n'a jamais été rouge ne protège rien. Remettre momentanément
`echappe(&livre.copyright)` — le champ, sans les parenthèses — dans `interieur.rs`, et
lancer :

```
cargo test --lib aucun_jeton_ne_survit_a_la_source_de_l_interieur
```

Attendu : ÉCHEC, `%AUTEUR% a traversé la composition`. Rétablir l'accesseur, relancer,
vert.

- [ ] **Step 7: Commit**

```bash
git add app/src-tauri/src/projet.rs app/src-tauri/src/interieur.rs app/src-tauri/src/ebook.rs
git commit -m "Aucun jeton ne franchit un point de sortie"
```

---

## Tâche 5 : l'interface

**Files:**
- Modify: `app/src/index.html:53-54`
- Modify: `app/src/app.js:387,392,728-740`
- Modify: `app/tests/contrats.test.js`

- [ ] **Step 1: Écrire le test de contrat**

Dans `app/tests/contrats.test.js`, ajouter au projet d'exemple `PROJET` le champ
`titre_page: '%TITRE%'` en remplacement de `titre_page: null`, puis :

```javascript
test('le titre de page et la dédicace voyagent en chaînes, jamais en null', () => {
  // `livre()` renvoie ce que `livre_modifier` reçoit : le Rust attend désormais des
  // `String`, un `null` y serait refusé à la désérialisation — et un champ oublié
  // effacerait la donnée, `livre_modifier` remplaçant tout ce qu'il tient.
  const { document } = charge();
  document.getElementById('inTitrePage').value = '';
  document.getElementById('inDedicace').value = '';
  const l = app.livre();
  assert.strictEqual(l.titre_page, '');
  assert.strictEqual(l.dedicace, '');
});
```

Adapter `charge()` et l'accès à `livre()` au motif des tests voisins — vérifier
comment `couverture.test.js` requiert son module et ce que `dom_shim` expose.

- [ ] **Step 2: Vérifier l'échec**

```
cd app && node --test tests/contrats.test.js
```

Attendu : ÉCHEC — `livre()` renvoie `null` pour les deux champs.

- [ ] **Step 3: Corriger la collecte**

Dans `app/src/app.js`, fonction `livre()` :

```javascript
function livre() {
  const chap = $('inChapitres').value.trim();
  return {
    titre: $('inTitre').value.trim(),
    titre_page: $('inTitrePage').value.trim(),
    auteur: $('inAuteur').value.trim(),
    genre: $('inGenre').value.trim() || 'roman',
    copyright: $('inCopyright').value,
    // Non rognée : c'est le Rust qui rogne, en un seul endroit — et il substitue
    // avant de rogner, ce que le front ne saurait pas faire.
    dedicace: $('inDedicace').value,
    chapitres: chap === '' ? null : Number(chap),
  };
}
```

- [ ] **Step 4: Corriger l'affichage**

Dans `app/src/app.js`, vers la ligne 387 :

```javascript
  $('inTitrePage').value = p.livre.titre_page;
  $('inAuteur').value = p.livre.auteur;
  $('inGenre').value = p.livre.genre;
  $('inCopyright').value = p.livre.copyright;
  // Le champ est absent du JSON quand la dédicace est vide : `skip_serializing_if`.
  $('inDedicace').value = p.livre.dedicace ?? '';
```

- [ ] **Step 5: Changer l'indication du champ**

Dans `app/src/index.html`, ligne 54, l'ancienne indication décrit un repli qui
n'existe plus :

```html
        <textarea id="inTitrePage" rows="2" placeholder="%TITRE%"></textarea></label>
```

**Changement de comportement assumé** : vider ce champ donne désormais une page de
titre sans titre, là où le vide valait « le titre ci-dessus ». C'est visible à
l'aperçu et réversible en retapant le jeton, et c'est le prix de rendre le repli
explicite.

- [ ] **Step 6: Vérifier que les tests passent**

```
cd app && node --test "tests/*.test.js"
```

Attendu : vert.

- [ ] **Step 7: Commit**

```bash
git add app/src/index.html app/src/app.js app/tests/contrats.test.js
git commit -m "Le champ montre le jeton plutôt qu'un repli invisible"
```

---

## Tâche 6 : la vérification d'ensemble

- [ ] **Step 1: La suite complète**

```
cd app/src-tauri && cargo test --lib && cargo clippy --all-targets -- -D warnings && cargo fmt --check
cd app && node --test "tests/*.test.js"
```

Attendu : tout vert, aucun avertissement clippy.

- [ ] **Step 2: Le témoin, avant et après**

Le compte de pages est la garde de non-régression du projet. Le relever sur `main`
avant d'exécuter ce plan, et le comparer ici :

```
cd app/src-tauri && cargo run --example temoin
```

Attendu : **le même compte de pages qu'avant le lot**. Un écart signifie qu'un champ
libre a changé de valeur composée — le plus probable étant la dédicace, dont la
présence vaut deux pages.

- [ ] **Step 3: Ouvrir un projet réel**

Lancer l'application, ouvrir `build/projects/Les Heures creuses.ozalid`, et vérifier :

- l'onglet Livre affiche le titre de page tel qu'il était, ou `%TITRE%` s'il était vide ;
- la dédicace est inchangée ;
- l'aperçu de couverture est identique à ce qu'il était ;
- taper `%AUTEUR%` dans le copyright et regarder l'épreuve : le nom paraît, pas le jeton.

Rappel du piège connu : en développement, `target/debug/fonts` ne suit pas `fonts/`
tout seul, et le repli de Typst est muet.

- [ ] **Step 4: Enregistrer, rouvrir**

Enregistrer le projet, le rouvrir, vérifier que le titre de page et la dédicace sont
revenus à l'identique. Vérifier à la main, dans l'archive dézippée, que `projet.toml`
porte bien le texte **à jetons** et non sa valeur résolue :

```
cd /tmp && unzip -o "<chemin>/Les Heures creuses.ozalid" projet.toml && grep -n "titre_page\|copyright" projet.toml
```

Attendu : `titre_page = "%TITRE%"` si c'est ce qui a été saisi. Une valeur résolue dans
le fichier signifierait que la substitution a lieu à la saisie et non à la
composition — le contraire de ce que la spec décide.

---

## Ce que ce lot laisse au suivant

Le lot 2 fera monter `editeur`, `monogramme`, `collection`, `prix` et `mention` de
`Couverture` vers `Livre`, ajoutera les trois jetons correspondants à `JETONS`, fera
afficher la Collection par la pastille, et portera **la seule migration** du format —
`VERSION` à 3. La table `JETONS` et les tests de sortie posés ici sont ce sur quoi il
s'appuiera : ajouter une clé y sera une ligne, et les tests diront si un appelant l'a
oubliée.
