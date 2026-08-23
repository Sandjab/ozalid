# Identité du livre — lot 2 : les clés montent

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** L'éditeur, le monogramme, la collection, le prix et la mention quittent `Couverture` pour `Livre`. Le livre dit ce qui est écrit, la maquette dit où et si ça se voit.

**Architecture:** Trois tâches qui laissent chacune l'application juste. La première est additive — `Livre` gagne ses champs, personne ne les lit encore. La deuxième porte la migration et la montée de `VERSION` à 3. La troisième bascule la composition et retire les champs de `Couverture` ; elle est atomique par nature, un champ ne pouvant changer de place à moitié.

**Tech Stack:** Rust 2021, `serde`, `toml` ; front vanilla, `node --test`.

Spec : `docs/superpowers/specs/2026-08-23-identite-du-livre-design.md`.
Lot précédent : `docs/superpowers/plans/2026-08-23-identite-lot-1-substitution.md`.

---

## Deux décisions prises en écrivant ce plan

**La migration ne retire rien de la maquette.** La spec demandait que les clés migrées
soient « retirées de la maquette, pour qu'aucun `.ozalid` réécrit ne conserve deux
vérités ». Impossible à la tâche 2 : ni `Pied.editeur` ni `Pastille.texte` ne portent de
`#[serde(default)]`, et un TOML amputé de ces champs serait refusé à la relecture. Ce
n'est pas grave, c'est mieux : une fois `Couverture` allégée à la tâche 3, ces champs
deviennent inconnus, serde les ignore, et **aucune réécriture ne les conserve** — le
résultat visé, sans code de suppression à écrire ni à tester.

**Les cinq nouveaux champs naissent avec leur valeur générique**, dès la tâche 1 :
`"Editeur"`, `"Collection"`, `"Monogramme"`, `"Prix"`, `"Mention"`. La spec les
réservait au lot 3, mais les trois maquettes perdent leur `editeur: "ÉDITEUR"` à la
tâche 3 : sans valeur au livre, un projet neuf en maquette Blanche n'afficherait plus
rien au pied. Une régression transitoire entre deux lots vaut moins que cette avance.
Les champs préexistants — titre, auteur, genre, titre de page, copyright — gardent
leurs valeurs actuelles et attendent bien le lot 3.

---

## Fichiers

| Fichier | Tâche | Rôle |
|---|---|---|
| `app/src-tauri/src/projet.rs` | 1, 2 | `Livre` : cinq champs, deux accesseurs ; `VERSION = 3` et la migration |
| `app/src-tauri/src/gabarit.rs` | 1 | `JETONS` passe de trois à six |
| `app/src-tauri/src/couverture.rs` | 3 | `bloc_pied`, `bloc_pastille`, `corps_quatre`, `source_quatre` lisent le livre ; `Pied`, `Pastille`, `Quatrieme` perdent six champs |
| `app/src-tauri/src/planche.rs` | 3 | `composes` lit `livre.editeur` |
| `app/src-tauri/src/maquettes.rs` | 3 | les trois maquettes perdent leurs textes |
| `app/src-tauri/src/commands.rs` | 3 | deux appels de `source_quatre` gagnent le livre |
| `app/src/index.html` | 3 | l'onglet Livre gagne cinq contrôles |
| `app/src/app.js` | 3 | `livre()` et l'affichage |
| `app/src/couverture.js` | 3 | le `SCHEMA` perd six champs |
| `app/tests/contrats.test.js` | 3 | la garde des champs déplacés |

---

## Tâche 1 : `Livre` reçoit les cinq champs

Additive : rien ne les lit encore, rien ne peut casser.

**Files:**
- Modify: `app/src-tauri/src/projet.rs:45-110`
- Modify: `app/src-tauri/src/gabarit.rs:15-19`

- [ ] **Step 1: Écrire les tests**

Dans le module `tests` de `app/src-tauri/src/gabarit.rs` :

```rust
    /// Les trois clés qui montent au lot 2 sont des jetons comme les autres.
    #[test]
    fn les_cles_de_la_maison_sont_des_jetons() {
        let l = Livre {
            editeur: "Ozalid".into(),
            collection: "Les Heures".into(),
            monogramme: "O".into(),
            ..livre()
        };
        assert_eq!(
            substituer("%EDITEUR%, %COLLECTION%, %MONOGRAMME%", &l),
            "Ozalid, Les Heures, O"
        );
    }
```

Dans le module `tests` de `app/src-tauri/src/projet.rs` :

```rust
    /// Le prix et la mention sont des champs libres : ils citent les clés, comme le
    /// copyright. Un `.ozalid` écrit avant eux s'ouvre avec leurs valeurs génériques.
    #[test]
    fn le_prix_et_la_mention_citent_les_cles() {
        let mut l = Livre::vide();
        l.collection = "Les Heures".into();
        l.prix = "18 € — %COLLECTION%".into();
        l.mention = "%EDITEUR%".into();
        l.editeur = "Ozalid".into();
        assert_eq!(l.prix(), "18 € — Les Heures");
        assert_eq!(l.mention(), "Ozalid");
    }

    /// Les cinq champs sont facultatifs dans le TOML : `VERSION` monte pour ce qui
    /// change de place, pas pour ce qui s'ajoute. Un projet qui ne les porte pas reçoit
    /// leurs valeurs génériques.
    #[test]
    fn un_projet_sans_les_cles_de_la_maison_recoit_les_generiques() {
        let toml = r#"
[ozalid]
version = 2

[livre]
titre = "Les Heures creuses"
auteur = "Ivan Pjig"
"#;
        let m: Metadonnees = toml::from_str(toml).expect("projet sans les clés refusé");
        assert_eq!(m.livre.editeur, "Editeur");
        assert_eq!(m.livre.collection, "Collection");
        assert_eq!(m.livre.monogramme, "Monogramme");
        assert_eq!(m.livre.prix, "Prix");
        assert_eq!(m.livre.mention, "Mention");
    }
```

- [ ] **Step 2: Vérifier l'échec**

```
cd app/src-tauri && cargo test --lib les_cles_de_la_maison
```

Attendu : ÉCHEC de compilation, `struct Livre has no field named editeur`.

- [ ] **Step 3: Ajouter les champs**

Dans `app/src-tauri/src/projet.rs`, à la suite de `genre` dans `struct Livre` :

```rust
    /// L'éditeur, la collection et le monogramme : des **clés**, littérales, jamais
    /// substituées. Elles nomment la maison, pas le livre, et elles ne bougent pas d'un
    /// titre à l'autre chez un auto-éditeur.
    ///
    /// Elles vivaient dans la maquette — l'éditeur dans le pied de la 1ère, que le dos
    /// relisait ; la collection sous le nom de « pastille ». Le livre dit ce qui est
    /// écrit, la maquette dit où et si ça se voit.
    #[serde(default = "editeur_defaut")]
    pub editeur: String,
    #[serde(default = "collection_defaut")]
    pub collection: String,
    #[serde(default = "monogramme_defaut")]
    pub monogramme: String,
```

et, après `copyright` :

```rust
    /// Le prix et la mention légale : des champs **libres**, qui citent les clés.
    #[serde(default = "prix_defaut")]
    pub prix: String,
    #[serde(default = "mention_defaut")]
    pub mention: String,
```

Les cinq fonctions de défaut, à côté de `titre_page_defaut` :

```rust
fn editeur_defaut() -> String {
    "Editeur".into()
}

fn collection_defaut() -> String {
    "Collection".into()
}

fn monogramme_defaut() -> String {
    "Monogramme".into()
}

fn prix_defaut() -> String {
    "Prix".into()
}

fn mention_defaut() -> String {
    "Mention".into()
}
```

Dans `Livre::vide()`, les cinq lignes correspondantes :

```rust
            editeur: editeur_defaut(),
            collection: collection_defaut(),
            monogramme: monogramme_defaut(),
            prix: prix_defaut(),
            mention: mention_defaut(),
```

Les deux accesseurs, à côté de `copyright()` :

```rust
    /// Le prix, jetons résolus.
    pub fn prix(&self) -> String {
        crate::gabarit::substituer(&self.prix, self)
    }

    /// La mention légale, jetons résolus.
    pub fn mention(&self) -> String {
        crate::gabarit::substituer(&self.mention, self)
    }
```

- [ ] **Step 4: Étendre la table des jetons**

Dans `app/src-tauri/src/gabarit.rs` :

```rust
const JETONS: [Jeton; 6] = [
    ("%TITRE%", |l| &l.titre),
    ("%AUTEUR%", |l| &l.auteur),
    ("%GENRE%", |l| &l.genre),
    ("%EDITEUR%", |l| &l.editeur),
    ("%COLLECTION%", |l| &l.collection),
    ("%MONOGRAMME%", |l| &l.monogramme),
];
```

- [ ] **Step 5: Compléter les constructions littérales de `Livre`**

Les aides de test et le témoin énumèrent tous les champs. Les trouver et les compléter
avec les cinq valeurs génériques :

```
cd app/src-tauri && cargo test --lib --no-run 2>&1 | grep -A 2 "missing field"
```

Sites connus : `projet.rs`, `interieur.rs`, `couverture.rs`, `epreuve.rs`, `planche.rs`,
`import.rs` (deux sites), et `examples/temoin.rs`. Dans `import.rs:56`, la construction
depuis un `livre.toml` : la chaîne Python ne porte aucun de ces cinq champs, ils
prennent donc leurs défauts, comme la dédicace le fait déjà.

- [ ] **Step 6: Vérifier que tout passe**

```
cd app/src-tauri && cargo test --lib && cargo clippy --all-targets -- -D warnings && cargo fmt --check
```

Attendu : vert, aucun avertissement.

- [ ] **Step 7: Commit**

```bash
git add app/src-tauri
git commit -m "Le livre reçoit les clés de la maison"
```

---

## Tâche 2 : la migration en version 3

**Files:**
- Modify: `app/src-tauri/src/projet.rs:37,491-500`

- [ ] **Step 1: Écrire les tests**

Dans le module `tests` de `app/src-tauri/src/projet.rs` :

```rust
    /// Un `.ozalid` en version 2 porte ses textes dans la maquette. Ils remontent au
    /// livre à l'ouverture, sans resaisie : c'est la première vraie migration du
    /// format, et cinq champs courts par projet valent le code qui les sauve.
    #[test]
    fn un_projet_v2_remonte_ses_textes_de_la_maquette() {
        let mut p = Projet::nouveau(livre(), String::new());
        p.meta.couverture.maquette = crate::maquettes::par_cle("folio");
        let mut brut = toml::to_string_pretty(&p.meta).unwrap();
        // Le projet est réécrit en v2, avec les textes là où la v2 les rangeait.
        brut = brut.replace("version = 3", "version = 2");
        let v2: toml::Value = toml::from_str(&brut).unwrap();
        let v2 = pose(v2, &["couverture", "maquette", "pied", "editeur"], "OZALID");
        let v2 = pose(v2, &["couverture", "maquette", "pied", "monogramme"], "O");
        let v2 = pose(v2, &["couverture", "maquette", "quatrieme", "collection"], "Les Heures");
        let v2 = pose(v2, &["couverture", "maquette", "quatrieme", "prix"], "18 €");
        let v2 = pose(v2, &["couverture", "maquette", "quatrieme", "mention"], "Dépôt légal");

        let m = migre(v2).expect("migration refusée");
        assert_eq!(m.livre.editeur, "OZALID");
        assert_eq!(m.livre.monogramme, "O");
        assert_eq!(m.livre.collection, "Les Heures");
        assert_eq!(m.livre.prix, "18 €");
        assert_eq!(m.livre.mention, "Dépôt légal");
    }

    /// La pastille portait un nom de collection sous un autre nom — « folio » dans la
    /// maquette Folio. Elle sert de repli quand la collection explicite est vide : la
    /// laisser tomber ferait perdre la seule chose que ce champ disait.
    #[test]
    fn la_pastille_supplee_une_collection_vide() {
        let mut p = Projet::nouveau(livre(), String::new());
        p.meta.couverture.maquette = crate::maquettes::par_cle("folio");
        let brut = toml::to_string_pretty(&p.meta)
            .unwrap()
            .replace("version = 3", "version = 2");
        let v2: toml::Value = toml::from_str(&brut).unwrap();
        let v2 = pose(v2, &["couverture", "maquette", "quatrieme", "collection"], "");
        let v2 = pose(v2, &["couverture", "maquette", "pastille", "texte"], "folio");

        let m = migre(v2).expect("migration refusée");
        assert_eq!(m.livre.collection, "folio");
    }

    /// La collection explicite gagne toujours : le repli n'est qu'un repli.
    #[test]
    fn une_collection_explicite_bat_la_pastille() {
        let mut p = Projet::nouveau(livre(), String::new());
        p.meta.couverture.maquette = crate::maquettes::par_cle("folio");
        let brut = toml::to_string_pretty(&p.meta)
            .unwrap()
            .replace("version = 3", "version = 2");
        let v2: toml::Value = toml::from_str(&brut).unwrap();
        let v2 = pose(v2, &["couverture", "maquette", "quatrieme", "collection"], "Les Heures");
        let v2 = pose(v2, &["couverture", "maquette", "pastille", "texte"], "folio");

        let m = migre(v2).expect("migration refusée");
        assert_eq!(m.livre.collection, "Les Heures");
    }

    /// Un projet déjà en v3 ne doit rien remonter : ses textes sont au livre, et la
    /// maquette n'en porte plus.
    #[test]
    fn un_projet_v3_traverse_la_migration_sans_bouger() {
        let mut l = livre();
        l.editeur = "Ozalid".into();
        let p = Projet::nouveau(l, String::new());
        let v3: toml::Value = toml::from_str(&toml::to_string_pretty(&p.meta).unwrap()).unwrap();

        let m = migre(v3).expect("migration refusée");
        assert_eq!(m.livre.editeur, "Ozalid");
    }

    /// Pose une valeur à un chemin de sections, en créant ce qui manque.
    fn pose(mut v: toml::Value, chemin: &[&str], valeur: &str) -> toml::Value {
        let mut courant = &mut v;
        for cle in &chemin[..chemin.len() - 1] {
            courant = courant
                .as_table_mut()
                .unwrap()
                .entry(*cle)
                .or_insert_with(|| toml::Value::Table(Default::default()));
        }
        courant.as_table_mut().unwrap().insert(
            chemin[chemin.len() - 1].to_string(),
            toml::Value::String(valeur.into()),
        );
        v
    }
```

- [ ] **Step 2: Vérifier l'échec**

```
cd app/src-tauri && cargo test --lib un_projet_v2_remonte
```

Attendu : ÉCHEC de compilation, `cannot find function 'migre' in this scope`.

- [ ] **Step 3: Monter la version et écrire la migration**

Dans `app/src-tauri/src/projet.rs`, remplacer la constante et sa documentation :

```rust
/// Version 3 : l'éditeur, le monogramme, la collection, le prix et la mention sont au
/// livre, là où la 2 les rangeait dans la maquette. Le livre dit ce qui est écrit, la
/// maquette dit où et si ça se voit.
pub const VERSION: u32 = 3;
```

Ajouter la migration, juste avant `impl Projet` :

```rust
/// Remonte au livre les textes qu'un projet en version 2 rangeait dans la maquette.
///
/// Sur le `toml::Value` et non sur les types : en v3, `Couverture` ne porte plus ces
/// champs, il n'y a donc plus de structure Rust capable de les lire. Un projet déjà en
/// v3 traverse sans rien faire.
///
/// Un champ vide côté v2 laisse sa valeur générique : ce qui n'a jamais été saisi n'a
/// rien à remonter.
fn migre(mut v: toml::Value) -> Result<Metadonnees, String> {
    let version = v
        .get("ozalid")
        .and_then(|o| o.get("version"))
        .and_then(toml::Value::as_integer)
        .unwrap_or(0);
    if version < 3 {
        // La collection explicite gagne ; la pastille, qui portait un nom de collection
        // sous un autre nom, ne sert que de repli.
        let repris = [
            ("editeur", vec!["pied", "editeur"], vec![]),
            ("monogramme", vec!["pied", "monogramme"], vec![]),
            (
                "collection",
                vec!["quatrieme", "collection"],
                vec!["pastille", "texte"],
            ),
            ("prix", vec!["quatrieme", "prix"], vec![]),
            ("mention", vec!["quatrieme", "mention"], vec![]),
        ];
        for (vers, depuis, repli) in repris {
            let valeur = lit(&v, &depuis)
                .filter(|s| !s.is_empty())
                .or_else(|| lit(&v, &repli).filter(|s| !s.is_empty()));
            if let Some(valeur) = valeur {
                if let Some(livre) = v.get_mut("livre").and_then(toml::Value::as_table_mut) {
                    livre.insert(vers.to_string(), toml::Value::String(valeur));
                }
            }
        }
        if let Some(o) = v.get_mut("ozalid").and_then(toml::Value::as_table_mut) {
            o.insert("version".into(), toml::Value::Integer(VERSION as i64));
        }
    }
    v.try_into().map_err(|e| format!("{PROJET_TOML} : {e}"))
}

/// La chaîne rangée sous `couverture.maquette.<chemin>`, si elle y est.
fn lit(v: &toml::Value, chemin: &[&str]) -> Option<String> {
    let mut courant = v.get("couverture")?.get("maquette")?;
    for cle in chemin {
        courant = courant.get(cle)?;
    }
    courant.as_str().map(str::to_owned)
}
```

- [ ] **Step 4: Brancher la migration dans `lire`**

Dans `Projet::lire`, remplacer la désérialisation directe :

```rust
        let valeur: toml::Value =
            toml::from_str(&toml_brut).map_err(|e| format!("{PROJET_TOML} : {e}"))?;
        let version = valeur
            .get("ozalid")
            .and_then(|o| o.get("version"))
            .and_then(toml::Value::as_integer)
            .unwrap_or(0);
        if version > VERSION as i64 {
            return Err(format!(
                "projet en version {version}, cette application lit jusqu'à la {VERSION}."
            ));
        }
        let mut meta: Metadonnees = migre(valeur)?;
```

Le contrôle de version qui suivait la désérialisation devient inutile — il est remonté
ici, avant la migration, pour qu'un projet venu du futur soit refusé plutôt que migré
de travers. Retirer l'ancien bloc `if meta.ozalid.version > VERSION`.

- [ ] **Step 5: Vérifier que tout passe**

```
cd app/src-tauri && cargo test --lib && cargo clippy --all-targets -- -D warnings && cargo fmt --check
```

Attendu : vert. Vérifier notamment que le test existant du projet venu du futur passe
toujours — il vise désormais la version 4.

- [ ] **Step 6: Voir la migration échouer par mutation**

Retirer momentanément le repli par la pastille — la ligne `.or_else(...)` — et lancer :

```
cd app/src-tauri && cargo test --lib la_pastille_supplee_une_collection_vide
```

Attendu : ÉCHEC, la collection valant sa générique au lieu de « folio ». Rétablir.

- [ ] **Step 7: Commit**

```bash
git add app/src-tauri
git commit -m "Un projet de la version 2 remonte ses textes au livre"
```

---

## Tâche 3 : la composition lit le livre

Atomique : un champ ne change pas de place à moitié.

**Files:**
- Modify: `app/src-tauri/src/couverture.rs:190-198,326-332,716-735,762-770,1072-1125,1150-1161`
- Modify: `app/src-tauri/src/planche.rs:148-160,423`
- Modify: `app/src-tauri/src/maquettes.rs`
- Modify: `app/src-tauri/src/commands.rs:804,964`
- Modify: `app/src/index.html`, `app/src/app.js`, `app/src/couverture.js`
- Modify: `app/tests/contrats.test.js`

- [ ] **Step 1: Écrire les tests de sortie**

Dans le module `tests` de `app/src-tauri/src/couverture.rs` :

```rust
    /// **Point de sortie : la couverture.** Le pied de 1ère, la pastille et le pied de
    /// 4ème composent désormais des textes du livre. Aucun jeton ne doit y survivre, et
    /// aucune valeur de la maquette ne doit y reparaître.
    #[test]
    fn la_couverture_compose_les_textes_du_livre() {
        let mut l = livre();
        l.editeur = "Ozalid".into();
        l.monogramme = "O".into();
        l.collection = "Les Heures".into();
        l.prix = "18 € — %COLLECTION%".into();
        l.mention = "%EDITEUR%".into();

        let mut cv = maquettes::par_cle("blanche").unwrap();
        cv.pied.actif = true;
        cv.pastille.actif = true;
        cv.quatrieme.pied_actif = true;

        let une = page_une(&l, &cv, FORMAT, None, None);
        assert!(une.contains("Ozalid"), "l'éditeur du livre n'est pas au pied");
        assert!(une.contains("Les Heures"), "la collection n'est pas en pastille");

        let quatre = source_quatre(&l, &cv, FORMAT, None, None, None).unwrap();
        assert!(quatre.contains("18 € — Les Heures"), "le prix n'est pas substitué");
        assert!(quatre.contains("Ozalid"), "la mention n'est pas substituée");
        for jeton in ["%EDITEUR%", "%COLLECTION%", "%TITRE%"] {
            assert!(!quatre.contains(jeton), "{jeton} a traversé la 4ème");
            assert!(!une.contains(jeton), "{jeton} a traversé la 1ère");
        }
    }
```

Dans le module `tests` de `app/src-tauri/src/planche.rs` :

```rust
    /// **Point de sortie : le dos.** L'éditeur y venait du pied de la 1ère, ce que le
    /// commentaire de `Dos` avouait. Il vient du livre.
    #[test]
    fn le_dos_prend_l_editeur_du_livre() {
        let mut l = livre();
        l.editeur = "Ozalid".into();
        let mut cv = maquettes::par_cle("folio").unwrap();
        cv.dos.editeur.actif = true;

        let composes = composes(&l, &cv);
        let editeur = composes.iter().find(|(cle, _, _)| *cle == "editeur");
        assert_eq!(editeur.map(|(_, _, t)| *t), Some("Ozalid"));
    }
```

- [ ] **Step 2: Vérifier l'échec**

```
cd app/src-tauri && cargo test --lib la_couverture_compose_les_textes_du_livre
```

Attendu : ÉCHEC de compilation — `source_quatre` ne prend pas de livre, et `Livre` n'a
pas encore de lecteur côté couverture.

- [ ] **Step 3: `Couverture` perd ses six textes**

Dans `app/src-tauri/src/couverture.rs`, retirer de `struct Pied` :

```rust
    pub monogramme: String,
    pub editeur: String,
```

de `struct Pastille` :

```rust
    pub texte: String,
```

et de `struct Quatrieme` :

```rust
    pub mention: String,
    pub collection: String,
    pub prix: String,
```

Mettre à jour le commentaire de `struct Dos`, qui décrivait la dépendance de travers :
« l'auteur, le titre et l'éditeur viennent du livre ».

- [ ] **Step 4: Les fonctions de composition reçoivent le livre**

`bloc_pied` :

```rust
fn bloc_pied(livre: &Livre, p: &Pied, cv: &Couverture, (fw, fh): (f64, f64)) -> String {
    if !p.actif {
        return String::new();
    }
    let pad = cv.pad_x / 100.0 * fw;
    // L'écart monogramme → éditeur est de 6 % de la largeur, fixé par le CSS d'origine.
    format!(
        "#place(bottom + left, dx: {}, dy: -{}, block(width: {})[\n\
         #set align({})\n#set par(leading: 0em, spacing: 0em)\n\
         #{}\n#v({})\n#{}\n])\n",
        mm(pad),
        mm(p.y / 100.0 * fh),
        mm(fw - 2.0 * pad),
        cv.align.typst(),
        p.style_mono.applique(fw, &livre.monogramme),
        mm(0.06 * fw),
        p.style_editeur.applique(fw, &livre.editeur),
    )
}
```

`bloc_pastille` — la pastille affiche la collection, dont elle était le doublon
déguisé :

```rust
fn bloc_pastille(p: &Pastille, collection: &str, fw: f64, d: Debords) -> String {
    if !p.actif || collection.trim().is_empty() {
        return String::new();
    }
```

et, plus bas dans la même fonction, `echappe(&p.texte)` devient `echappe(collection)`.

Dans `corps_une`, les deux appels :

```rust
    cadre.push_str(&bloc_pied(livre, &cv.pied, cv, format));
    cadre.push_str(&bloc_pastille(&cv.pastille, &livre.collection, fw, Debords::de(b, format)));
```

`corps_quatre` gagne le livre en premier argument, comme `corps_une` :

```rust
pub fn corps_quatre(
    livre: &Livre,
    cv: &Couverture,
    format: (f64, f64),
    image_quatre: Option<&Ressource>,
    photo_une: Option<&Ressource>,
    pano: Option<Panorama>,
    b: Boite,
) -> Result<String, String> {
```

et les trois lignes de son pied viennent du livre — la collection littérale, le prix et
la mention substitués :

```rust
    if q.pied_actif {
        let lignes: Vec<String> = [livre.mention(), livre.collection.clone(), livre.prix()]
            .iter()
            .map(|v| v.trim())
            .filter(|v| !v.is_empty())
            .map(|v| format!("#{}", q.style_pied.applique(fw, v)))
            .collect();
```

`source_quatre` de même :

```rust
pub fn source_quatre(
    livre: &Livre,
    cv: &Couverture,
    format: (f64, f64),
    image_quatre: Option<&Ressource>,
    image_une: Option<&Ressource>,
    dos_mm: Option<f64>,
) -> Result<String, String> {
    let b = Boite::rognee(format);
    let pano = panorama_face(format, dos_mm, false);
    let corps = corps_quatre(livre, cv, format, image_quatre, image_une, pano, b)?;
    Ok(preambule(b.largeur, b.hauteur) + &corps)
}
```

- [ ] **Step 5: Le dos lit le livre**

Dans `app/src-tauri/src/planche.rs`, fonction `composes` :

```rust
        ("editeur", &cv.dos.editeur, livre.editeur.trim()),
```

et, plus bas, l'appel de `corps_quatre` gagne `livre` en premier argument.

- [ ] **Step 6: Les maquettes perdent leurs textes**

Dans `app/src-tauri/src/maquettes.rs`, retirer des trois maquettes :
`monogramme`, `editeur` dans chaque `Pied` ; `texte` dans `pastille_eteinte` et dans la
pastille de `folio` ; `mention`, `collection`, `prix` dans `quatrieme_commune`.

La pastille de `folio` reste **allumée** — c'est un choix de maquette — mais son texte
vient désormais du livre :

```rust
        pastille: Pastille {
            actif: true,
            ..pastille_eteinte()
        },
```

Le commentaire du pied de `folio`, qui expliquait pourquoi le monogramme était vide et
l'éditeur générique, se déplace : ce n'est plus l'affaire de la maquette. Le retirer.

- [ ] **Step 7: Les deux appels de `commands.rs`**

`app/src-tauri/src/commands.rs`, deux appels. Le second est déjà entouré de son
symétrique, qui montre où prendre le livre :

```rust
    let corps = match face.as_str() {
        "une" => couverture::source_une(&o.projet.meta.livre, &nu, format, None, dos_mm),
        _ => couverture::source_quatre(&o.projet.meta.livre, &nu, format, None, None, dos_mm)?,
    };
```

Le premier, vers la ligne 804, compose la 4ème d'un package : y ajouter `livre` en
premier argument, `livre` y étant déjà la variable en portée — la même que celle passée
à `source_une` quelques lignes plus haut.

- [ ] **Step 8: Le front**

Dans `app/src/couverture.js`, retirer les six entrées du `SCHEMA` : `pied.monogramme`,
`pied.editeur`, `pastille.texte`, `quatrieme.mention`, `quatrieme.collection`,
`quatrieme.prix`.

Dans `app/src/index.html`, l'onglet Livre reçoit cinq contrôles. Les clés d'abord, avec
le titre, l'auteur et le genre ; les libres ensuite, avec le copyright — le regroupement
en deux sections titrées est le lot 3, ici seul l'ordre compte :

```html
      <label><span>Éditeur</span><input type="text" id="inEditeur"></label>
      <label><span>Collection</span><input type="text" id="inCollection"></label>
      <label><span>Monogramme</span><input type="text" id="inMonogramme"></label>
      <label><span>Prix</span><input type="text" id="inPrix"></label>
      <label><span>Mention</span><input type="text" id="inMention"></label>
```

Dans `app/src/app.js`, l'affichage — à la suite des autres champs du livre :

```javascript
  $('inEditeur').value = p.livre.editeur;
  $('inCollection').value = p.livre.collection;
  $('inMonogramme').value = p.livre.monogramme;
  $('inPrix').value = p.livre.prix;
  $('inMention').value = p.livre.mention;
```

la collecte, dans `livre()` :

```javascript
    editeur: $('inEditeur').value.trim(),
    collection: $('inCollection').value.trim(),
    monogramme: $('inMonogramme').value.trim(),
    prix: $('inPrix').value.trim(),
    mention: $('inMention').value.trim(),
```

et les cinq identifiants dans la liste des écouteurs `change`, vers la ligne 1027.

- [ ] **Step 9: La garde du contrat front**

Dans `app/tests/contrats.test.js`, ajouter les cinq champs à la fixture `PROJET.livre`,
puis :

```javascript
/**
 * Les cinq textes déplacés ne doivent plus être offerts par l'onglet Couverture : les
 * y laisser ferait deux endroits où saisir un éditeur, qui peuvent se contredire.
 */
test('les textes du livre ont quitté le schéma de la couverture', () => {
  const js = source('src', 'couverture.js');
  for (const chemin of ['pied.monogramme', 'pied.editeur', 'pastille.texte',
    'quatrieme.mention', 'quatrieme.collection', 'quatrieme.prix']) {
    assert.ok(!js.includes(`'${chemin}'`), `${chemin} est encore réglable en couverture`);
  }
});
```

- [ ] **Step 10: Vérifier que tout passe**

```
cd app/src-tauri && cargo test --lib && cargo clippy --all-targets -- -D warnings && cargo fmt --check
cd app && node --test "tests/*.test.js"
```

Attendu : tout vert.

- [ ] **Step 11: Commit**

```bash
git add app
git commit -m "Le livre dit ce qui est écrit, la maquette où ça se voit"
```

---

## Tâche 4 : la vérification d'ensemble

- [ ] **Step 1: Le témoin**

```
cd app/src-tauri && cargo run --example temoin
```

Attendu : **98 pages, dos 7,21 mm**, comme au lot 1. Le lot touche `planche.rs`, donc le
dos : c'est ici que le témoin devient une garde sérieuse, là où le lot 1 ne pouvait rien
déplacer.

- [ ] **Step 2: Un `.ozalid` réel migre**

Ouvrir `build/projects/Les Heures creuses.ozalid`, vérifier que l'éditeur, la
collection, le monogramme, le prix et la mention sont ceux que la maquette portait, puis
enregistrer et dézipper :

```
cd /tmp && unzip -p "<chemin>/Les Heures creuses.ozalid" projet.toml | head -20
```

Attendu : `version = 3`, les cinq champs sous `[livre]`, et **aucune trace** de
`pied.editeur`, `pastille.texte` ni `quatrieme.collection` sous la maquette — serde les
ignore désormais, donc la réécriture ne les conserve pas.

- [ ] **Step 3: À l'œil**

L'aperçu de couverture en maquette Blanche, pied allumé : l'éditeur et le monogramme
doivent paraître. En Folio, pied éteint : ils ne doivent pas paraître sur la 1ère, mais
l'éditeur doit être sur le dos si `dos.editeur.actif` est allumé. La pastille doit
afficher la collection.

Rappel du piège maison : en développement, `target/debug/fonts` ne suit pas `fonts/`
tout seul, et le repli de Typst est muet.

---

## Ce que ce lot laisse au lot 3

Le regroupement de l'onglet Livre en deux sections titrées — clés, puis libres avec
l'aide listant les six jetons, servie par une commande plutôt que recopiée dans le HTML.
Et les valeurs génériques des champs préexistants : titre, auteur, genre, et le
copyright daté de l'année de création. La Dédicace restera vide, seule à n'avoir pas
d'interrupteur.
