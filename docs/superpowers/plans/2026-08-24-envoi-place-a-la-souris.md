# L'envoi se place à la souris — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Descendre la main de l'envoi du livre vers l'exemplaire, et rendre le placement de l'envoi — page, position, échelle, inclinaison — réglable à la souris sur un canevas montrant la vraie page.

**Architecture:** Le placement vit dans `Envoi::place`, en fractions de page. Typst le pose en `foreground` de page conditionné au numéro de page, qui ne consomme aucun flux : la pagination, le dos et la planche restent ceux du tirage. L'interface montre la page rendue sans envoi en fond et l'objet rendu par Typst sur fond transparent par-dessus ; glisser, redimensionner et incliner sont de purs `transform` CSS sur cette image.

**Tech Stack:** Rust + Tauri 2, Typst 0.15.1 en sidecar, front vanilla sans bundler, tests `cargo test` et `node --test`.

**Spec :** `docs/superpowers/specs/2026-08-23-envoi-place-a-la-souris-design.md`

---

## Avant de commencer

Lire la spec en entier. Elle porte les décisions et leurs raisons ; ce plan ne
porte que les gestes.

Conventions de ce dépôt, non négociables (voir `CLAUDE.md`) :

- **Français** dans l'interface, les commentaires et les commits. Les termes
  techniques anglais restent tels quels (`viewport`, `chunk`, `canvas`).
- Un commentaire dit **pourquoi**, pas quoi. Le style du dépôt est dense et
  motivé : relire les voisins avant d'écrire.
- **Tout test neuf doit avoir été vu échouer.** C'est l'objet de l'étape « Run
  test to verify it fails » de chaque tâche : elle n'est pas décorative.

Commandes de vérification, depuis `app/src-tauri/` sauf mention contraire :

```
cargo fmt --check
cargo clippy --all-targets -- -D warnings     # jamais dans un pipe : `| tail` masque l'échec
cargo test
cargo run --example temoin                    # témoin de pagination
cd .. && node --test tests/*.test.js          # depuis app/
```

Mise en route si le sidecar ou les polices manquent :

```
app/outils/typst.sh --local
app/outils/polices.sh
```

**Piège connu** : `target/debug/fonts` ne suit pas `fonts/` tout seul, et le repli
de Typst est muet. Si un rendu sort dans une écriture qui n'est pas la bonne,
c'est là qu'il faut regarder.

---

## Structure des fichiers

| Fichier | Responsabilité | Sort |
|---|---|---|
| `app/src-tauri/src/envoi.rs` | `Place`, `Envoi` avec sa main, `Envois` sans elle | modifié |
| `app/src-tauri/src/projet.rs` | migration v3 → v4 dans `migre()`, `VERSION = 4` | modifié |
| `app/src-tauri/src/interieur.rs` | le `foreground` de page, le corps qui suit la taille | modifié |
| `app/src-tauri/src/package.rs` | la trace lit la main de l'envoi ; refus page hors bornes | modifié |
| `app/src-tauri/src/typst.rs` | `apercus()` : toutes les pages en une invocation | modifié |
| `app/src-tauri/src/commands.rs` | `envoi_vignettes`, `envoi_page`, `envoi_objet`, `envoi_regler` | modifié |
| `app/src-tauri/src/lib.rs` | enregistrement des commandes | modifié |
| `app/src/placement.js` | la géométrie du placement, sans DOM | **créé** |
| `app/src/envois.js` | liste, sélection, réglages, canevas, rail | modifié |
| `app/src/index.html` | les quatre bandes de l'étape | modifié |
| `app/src/styles.css` | la grille de l'étape, le canevas, les prises | modifié |
| `app/src/app.js` | câblage des nouveaux contrôles | modifié |
| `app/tests/placement.test.js` | la géométrie | **créé** |
| `app/README.md` | l'écran, le `.ozalid`, les modules | modifié |

Cinq lots, du plus sûr au plus lourd. Chacun se commite et laisse l'application
compilable.

---

# Lot A — le modèle

## Task 1 : `Place`, et la main descend dans l'envoi

**Files:**
- Modify: `app/src-tauri/src/envoi.rs`
- Test: `app/src-tauri/src/envoi.rs` (module `tests` en fin de fichier)

- [ ] **Step 1 : écrire les tests qui échouent**

Ajouter dans le `mod tests` d'`envoi.rs` :

```rust
    /// Un placement s'exprime en fractions de page et non en millimètres : c'est ce
    /// qui rend une maquette de placement portable du poche au grand format. Le
    /// défaut repose l'envoi sur la page de titre — page 3, le faux-titre étant en 1
    /// et sa blanche en 2 —, là où les projets d'avant le portaient.
    #[test]
    fn un_placement_neuf_repose_l_envoi_sur_la_page_de_titre() {
        let p = Place::default();
        assert_eq!(p.page, 3);
        assert_eq!(p.x, 0.5);
        assert!((0.0..=1.0).contains(&p.y), "y hors page : {}", p.y);
        assert!((0.0..=1.0).contains(&p.taille), "taille hors page : {}", p.taille);
        assert_eq!(p.angle, 0.0);
    }

    /// Le fait que cette spec ajoute, et le seul que rien d'autre ne protège : deux
    /// exemplaires du même livre peuvent s'écrire dans deux mains différentes. Un mot
    /// composé pour Léa et une photo d'écriture pour Marc ne s'excluent plus.
    #[test]
    fn deux_envois_du_meme_livre_ont_chacun_leur_main() {
        let e = Envois {
            liste: vec![
                Envoi {
                    dedicataire: "Léa".into(),
                    main: Main::Police { police: MAINS[0].into() },
                    contenu: "Pour Léa.".into(),
                    ..Envoi::default()
                },
                Envoi {
                    dedicataire: "Marc".into(),
                    main: Main::Image,
                    image: Some("Marc.jpg".into()),
                    ..Envoi::default()
                },
            ],
            ..Envois::default()
        };
        assert!(e.verifie().is_ok(), "{:?}", e.verifie());
    }

    /// L'erreur doit nommer le dédicataire fautif : une liste de vingt envois dont un
    /// porte une main inconnue laisserait sinon chercher lequel.
    #[test]
    fn une_main_inconnue_nomme_le_dedicataire() {
        let e = Envois {
            liste: vec![Envoi {
                dedicataire: "Marc".into(),
                main: Main::Police { police: "Comic Sans".into() },
                ..Envoi::default()
            }],
            ..Envois::default()
        };
        let err = e.verifie().unwrap_err();
        assert!(err.contains("Marc"), "{err}");
        assert!(err.contains("Comic Sans"), "{err}");
    }

    /// Un envoi sans dédicataire doit se dire quand même : « main inconnue » tout
    /// court laisserait chercher dans une liste où plusieurs lignes sont anonymes.
    #[test]
    fn un_envoi_anonyme_se_designe_par_son_rang() {
        let e = Envois {
            liste: vec![Envoi {
                dedicataire: "  ".into(),
                main: Main::Police { police: "Comic Sans".into() },
                ..Envoi::default()
            }],
            ..Envois::default()
        };
        let err = e.verifie().unwrap_err();
        assert!(err.contains('1'), "le rang manque : {err}");
    }
```

Retirer le test `une_saisie_ne_peut_pas_inventer_une_police_personnelle` : la
garde qu'il protège disparaît avec `Envois::reprend` (spec § 1 — l'interface ne
renvoie plus qu'un `Envoi`, qui ne porte aucun champ de police personnelle, si
bien que le type interdit ce que la garde refusait).

Adapter les tests existants qui construisent un `Envois { main: … }` :
`une_main_generee_se_choisit_avant_d_avoir_son_gabarit`,
`une_main_en_image_n_a_pas_de_police_a_verifier`, `un_livre_neuf_a_deja_une_main`,
`une_main_hors_liste_est_refusee`,
`la_police_personnelle_est_admise_tant_que_l_archive_la_porte`,
`l_erreur_de_main_nomme_aussi_la_police_personnelle` — la main se pose désormais
sur un `Envoi` de la liste. `un_livre_neuf_a_deja_une_main` devient :

```rust
    /// Un envoi neuf sait écrire sans qu'on lui règle quoi que ce soit, comme le livre
    /// sait déjà composer son intérieur en EB Garamond.
    #[test]
    fn un_envoi_neuf_a_deja_une_main() {
        assert_eq!(
            Envoi::default().main,
            Main::Police { police: MAINS[0].into() }
        );
    }
```

- [ ] **Step 2 : lancer les tests pour les voir échouer**

Run: `cargo test --lib envoi`
Expected: FAIL — `cannot find type Place in this scope`, et `struct Envoi has no field named main`.

- [ ] **Step 3 : écrire le modèle**

Dans `envoi.rs`, remplacer la définition de `Main::Diffusion` par une variante
sans champ, et ajouter `Place` :

```rust
    /// Une image par envoi, produite par un modèle de diffusion à partir du gabarit du
    /// livre, dans lequel le mot de chaque envoi s'insère.
    ///
    /// Le gabarit vit sur `Envois` et non ici : c'est le style d'écriture du livre, et
    /// le réécrire pour chaque personne n'aurait pas de sens. L'adresse du modèle et la
    /// clé appartiennent à la machine, et vivent dans les préférences. Une image
    /// acceptée est figée dans l'archive comme celle du mode précédent — composer ne
    /// rappelle jamais le réseau.
    Diffusion,
```

```rust
/// Où l'envoi se pose sur sa page.
///
/// **En fractions de la page, jamais en millimètres** : c'est la règle de l'atelier
/// gelé — « tout réglage est en pourcentage de la largeur de couverture » — et c'est
/// ce qui rend un placement portable du poche au grand format. Les fractions portent
/// sur la page entière, marges comprises, ce qui les met en correspondance 1:1 avec le
/// canevas de l'interface, qui montre la page entière.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Place {
    /// Page physique du PDF, à partir de 1 : celle que la vignette montre. Le
    /// `counter(page)` de l'intérieur n'est jamais remis à zéro — seul son affichage
    /// est masqué jusqu'au corps —, si bien que ce numéro désigne bien la n-ième page
    /// du fichier.
    pub page: u32,
    /// Centre de l'objet, en fraction de la largeur et de la hauteur de page. Le
    /// centre et non le coin : la rotation tourne autour de lui, en CSS comme en Typst.
    pub x: f64,
    pub y: f64,
    /// Largeur de l'objet, en fraction de la largeur de page.
    pub taille: f64,
    /// Degrés, positif dans le sens horaire.
    pub angle: f64,
}

impl Default for Place {
    /// La page de titre, au bas — là où les projets d'avant cette spec portaient leur
    /// envoi. Le faux-titre est en page 1, sa blanche en 2.
    fn default() -> Self {
        Self {
            page: 3,
            x: 0.5,
            y: 0.80,
            taille: 0.60,
            angle: 0.0,
        }
    }
}
```

`Envoi` gagne la main et le placement :

```rust
/// Un mot adressé à une personne, sur son exemplaire.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Envoi {
    pub dedicataire: String,
    /// D'où vient l'écriture de **cet** exemplaire. Elle appartenait au livre jusqu'à
    /// la v4 du format : un auteur ne pouvait pas écrire son mot à la main pour l'une
    /// et le faire composer pour l'autre.
    #[serde(default)]
    pub main: Main,
    /// Ce que la main réclame : le texte à composer. Vide quand la main est une image.
    #[serde(default)]
    pub contenu: String,
    /// Nom, sous `envois/` dans l'archive, de l'image de cet envoi.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    #[serde(default)]
    pub place: Place,
}
```

`#[derive(Default)]` sur `Envoi` continue de fonctionner : `Main` implémente déjà
`Default` à la main (`Police { MAINS[0] }`) et `Place` vient de le faire. Rien de
plus à écrire.

`Envois` perd `main` et gagne `gabarit` :

```rust
/// Les envois du livre, et ce qu'ils partagent.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Envois {
    /// Famille de la police personnelle embarquée sous `polices/`, quand le livre en
    /// porte une.
    ///
    /// Le nom figure ici pour que `projet.toml` reste lisible dézippé, mais **c'est le
    /// fichier qui fait foi** : à l'ouverture, `Projet::ouvrir` le relève dans
    /// l'archive et écrase ce que le TOML annonçait. Un nom recopié à la main dans le
    /// TOML ferait sinon composer une police que Typst ne trouverait pas — c'est-à-dire
    /// une autre.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub personnelle: Option<String>,
    /// Le patron de prompt des envois générés, partagé par tous : c'est le style
    /// d'écriture du livre, pas le mot d'une personne. `{envoi}` y marque l'endroit où
    /// le mot de chacun s'insère.
    #[serde(default)]
    pub gabarit: String,
    #[serde(default)]
    pub liste: Vec<Envoi>,
}
```

`verifie` boucle et nomme :

```rust
impl Envois {
    /// Refuse une main que Typst ne saurait pas trouver, en nommant l'envoi fautif.
    ///
    /// Sans ce contrôle, Typst composerait dans sa police par défaut **sans lever
    /// d'erreur** : `--ignore-system-fonts` empêche une substitution par le système,
    /// pas une substitution par le défaut du binaire. C'est le contrôle
    /// d'`Interieur::verifie`, pour la même raison.
    ///
    /// Le dédicataire est nommé parce qu'une liste de vingt envois dont un seul est
    /// fautif laisserait sinon chercher lequel.
    pub fn verifie(&self) -> Result<(), String> {
        for (i, e) in self.liste.iter().enumerate() {
            // Une image n'a pas de nom de police à trouver : elle se pose telle quelle.
            // Ce qui lui manque — l'image d'un envoi qui n'en a pas — se refuse à la
            // composition, pas ici : on écrit la liste avant de choisir les images.
            let Main::Police { police } = &e.main else {
                continue;
            };
            if MAINS.contains(&police.as_str()) || self.personnelle.as_deref() == Some(police) {
                continue;
            }
            let mut attendu: Vec<&str> = MAINS.to_vec();
            attendu.extend(self.personnelle.as_deref());
            return Err(format!(
                "{} : main inconnue « {police} ». Attendu : {}.",
                designe(e, i),
                attendu.join(", ")
            ));
        }
        Ok(())
    }
}

/// Comment nommer un envoi dans un message d'erreur.
///
/// Le rang plutôt que rien quand la ligne est anonyme : « main inconnue » tout court
/// laisserait chercher dans une liste où plusieurs lignes le sont.
fn designe(e: &Envoi, i: usize) -> String {
    if e.dedicataire.trim().is_empty() {
        format!("envoi {}", i + 1)
    } else {
        e.dedicataire.clone()
    }
}
```

Retirer `Envois::reprend` en entier.

- [ ] **Step 4 : lancer les tests**

Run: `cargo test --lib envoi`
Expected: PASS. Les autres modules ne compilent pas encore — c'est attendu, les
tâches 2, 3 et 5 les rattrapent. Si `cargo test --lib envoi` refuse de bâtir à
cause de `projet.rs` ou `package.rs`, passer à la tâche 2 et lancer les tests
d'`envoi` à la fin de la tâche 5.

- [ ] **Step 5 : commiter**

```bash
git add app/src-tauri/src/envoi.rs
git commit -m "La main descend du livre dans l'exemplaire

Un auteur peut écrire son mot à la main pour l'une et le faire composer pour
l'autre : la main était au livre, elle est à l'envoi. Le gabarit de diffusion
remonte sur Envois — c'est le style d'écriture du livre, pas le mot d'une
personne. Place arrive avec, en fractions de page.

Envois::reprend s'en va : sa garde devient impossible à violer, l'interface ne
renvoyant plus qu'un Envoi, qui ne porte aucun champ de police personnelle."
```

---

## Task 2 : la migration v3 → v4

**Files:**
- Modify: `app/src-tauri/src/projet.rs:41` (`VERSION`), `:428-470` (`migre`)
- Test: `app/src-tauri/src/projet.rs` (module `tests`)

- [ ] **Step 1 : écrire les tests qui échouent**

```rust
    /// Un `.ozalid` de la v3 porte sa main au livre. La perdre ferait composer les
    /// vingt exemplaires dans le défaut — en silence, dans une écriture que personne
    /// n'a choisie. Le TOML est écrit **littéralement** : ce sont les fichiers d'hier
    /// qu'il s'agit de relire, et les types d'hier n'existent plus pour les fabriquer.
    #[test]
    fn la_main_du_livre_v3_descend_dans_chaque_envoi() {
        let v3 = r#"
[ozalid]
version = 3
[livre]
titre = "Les Heures creuses"
auteur = "Ivan Pjig"
genre = "roman"
chapitres = 12
[envois.main]
mode = "police"
police = "Dancing Script"
[[envois.liste]]
dedicataire = "Léa"
contenu = "Pour Léa."
[[envois.liste]]
dedicataire = "Marc"
contenu = "À Marc."
"#;
        let m = migre(toml::from_str(v3).expect("TOML v3 illisible")).expect("migration refusée");
        assert_eq!(m.ozalid.version, VERSION);
        for e in &m.envois.liste {
            assert_eq!(
                e.main,
                crate::envoi::Main::Police { police: "Dancing Script".into() },
                "{} a perdu la main du livre",
                e.dedicataire
            );
        }
    }

    /// Le gabarit vivait dans la main, il appartient désormais aux envois : le perdre
    /// obligerait à réécrire le prompt d'un livre qu'on rouvre.
    #[test]
    fn le_gabarit_v3_remonte_sur_les_envois() {
        let v3 = r#"
[ozalid]
version = 3
[livre]
titre = "T"
auteur = "A"
genre = "roman"
chapitres = 1
[envois.main]
mode = "diffusion"
gabarit = "une écriture à l'encre bleue : {envoi}"
[[envois.liste]]
dedicataire = "Léa"
"#;
        let m = migre(toml::from_str(v3).expect("TOML v3 illisible")).expect("migration refusée");
        assert_eq!(m.envois.gabarit, "une écriture à l'encre bleue : {envoi}");
        assert_eq!(m.envois.liste[0].main, crate::envoi::Main::Diffusion);
    }

    /// Un envoi qui porte déjà sa main est en v4 : la main du livre n'a rien à y
    /// écraser. Sans ce contrôle, une migration rejouée écraserait le travail fait.
    #[test]
    fn un_envoi_qui_a_deja_sa_main_ne_se_la_fait_pas_ecraser() {
        let mixte = r#"
[ozalid]
version = 3
[livre]
titre = "T"
auteur = "A"
genre = "roman"
chapitres = 1
[envois.main]
mode = "police"
police = "Dancing Script"
[[envois.liste]]
dedicataire = "Léa"
[envois.liste.main]
mode = "image"
"#;
        let m = migre(toml::from_str(mixte).expect("TOML illisible")).expect("migration refusée");
        assert_eq!(m.envois.liste[0].main, crate::envoi::Main::Image);
    }

```

**Le refus d'un projet venu du futur n'est pas à écrire** : `Projet::ouvrir` le
porte déjà (`projet.rs:664`, « un projet venu du futur doit être refusé plutôt
que migré de travers »). C'est ce dont cette tâche se sert, non ce qu'elle
ajoute. Vérifier seulement qu'un test le couvre :

```bash
grep -n "lit jusqu" app/src-tauri/src/projet.rs
```

S'il n'en existe aucun, en écrire un sur le modèle des tests d'archive du module
— une archive réelle dont le `projet.toml` annonce `VERSION + 1` — et le commiter
avec cette tâche : c'est lui qui rend la montée de version sûre.

- [ ] **Step 2 : lancer les tests pour les voir échouer**

Run: `cargo test --lib projet::tests::la_main_du_livre_v3`
Expected: FAIL — la main de chaque envoi vaut `Police { "Caveat" }`, le défaut,
et non `Dancing Script`.

- [ ] **Step 3 : monter la version et écrire la migration**

`projet.rs:41` :

```rust
/// La version du format `.ozalid`.
///
/// **v4** : la main de l'envoi descend du livre dans l'exemplaire, et le gabarit de
/// diffusion remonte sur `Envois`. Contrairement aux sections facultatives ajoutées
/// depuis la v3, qu'un binaire d'avant traverse sans dommage, un champ se **déplace**
/// ici : un binaire v3 ouvrant un projet v4 ne verrait aucune main d'envoi, et son
/// `serde(default)` les lui donnerait toutes dans la même écriture — celle que
/// personne n'a choisie. Monter la version fait refuser ce projet, ce qui est vrai et
/// réparable, plutôt qu'imprimer vingt exemplaires dans la mauvaise main.
pub const VERSION: u32 = 4;
```

Dans `migre`, après la reprise des textes de la v2 et **avant** la réécriture du
numéro de version :

```rust
        // v3 → v4 : la main du livre descend dans chaque envoi, le gabarit remonte sur
        // les envois. Sur le `toml::Value` et non sur les types, pour la raison déjà
        // dite plus haut : en v4, `Envois` ne porte plus de `main`, il n'y a donc plus
        // de structure Rust capable de la lire.
        //
        // Rien n'est **retiré**, et ce n'est pas un oubli : une fois `Envois` allégée,
        // `main` y est inconnue, serde l'ignore, et aucune réécriture ne la conserve.
        if let Some(envois) = v.get_mut("envois").and_then(toml::Value::as_table_mut) {
            let ancienne = envois.get("main").cloned();
            if let Some(g) = ancienne
                .as_ref()
                .and_then(|m| m.get("gabarit"))
                .and_then(toml::Value::as_str)
            {
                envois
                    .entry("gabarit".to_string())
                    .or_insert_with(|| toml::Value::String(g.to_string()));
            }
            if let (Some(ancienne), Some(liste)) = (
                ancienne,
                envois.get_mut("liste").and_then(toml::Value::as_array_mut),
            ) {
                for e in liste {
                    // Un envoi qui porte déjà sa main est en v4 : une migration rejouée
                    // n'a pas à écraser le travail fait.
                    if let Some(t) = e.as_table_mut() {
                        t.entry("main".to_string()).or_insert(ancienne.clone());
                    }
                }
            }
        }
```

- [ ] **Step 4 : lancer les tests**

Run: `cargo test --lib projet`
Expected: PASS.

- [ ] **Step 5 : commiter**

```bash
git add app/src-tauri/src/projet.rs
git commit -m "Le .ozalid passe en version 4

La main du livre descend dans chaque envoi, le gabarit remonte sur Envois. Sur le
toml::Value et non sur les types, pour la raison de la v2 : Envois ne porte plus
de main, aucune structure Rust ne sait plus la lire.

VERSION monte, contrairement à la règle des sections facultatives : un champ se
déplace, et un binaire v3 ouvrant un projet v4 donnerait à tous les envois la
main par défaut. Le refus de Projet::ouvrir est plus honnête."
```

---

# Lot B — le Typst

## Task 3 : l'envoi se pose en `foreground` de page

**Files:**
- Modify: `app/src-tauri/src/interieur.rs:129-141` (`Trace`), `:144-291` (`assemble`), `:333-434` (`liminaires`)
- Test: `app/src-tauri/src/interieur.rs` (module `tests`)

- [ ] **Step 1 : écrire les tests qui échouent**

Adapter le fabricant `trace()` du module de tests (`interieur.rs:1156`) :

```rust
    fn place() -> Place {
        Place { page: 3, x: 0.5, y: 0.80, taille: 0.60, angle: 0.0 }
    }

    fn trace() -> Trace<'static> {
        Trace { quoi: Quoi::Texte { police: "Caveat", texte: "À Léa," }, place: PLACE }
    }
```

`PLACE` étant une constante du module de tests :

```rust
    const PLACE: &Place = &Place { page: 3, x: 0.5, y: 0.80, taille: 0.60, angle: 0.0 };
```

Puis les tests neufs :

```rust
    /// L'envoi se pose en `foreground` de page, conditionné au numéro de page. C'est
    /// ce qui lui interdit de créer une page — donc de déplacer la pagination, le dos
    /// et la planche — **sur n'importe quelle page**, et non plus sur la seule page de
    /// titre. Si ce test tombe, tous les packages d'envoi sont faux.
    #[test]
    fn un_envoi_se_pose_en_foreground_conditionne_a_sa_page() {
        let p = Place { page: 37, ..*PLACE };
        let s = source_avec(Some(Trace { quoi: Quoi::Texte { police: "Caveat", texte: "À Léa," }, place: &p }));
        assert!(s.contains("foreground:"), "pas de foreground : {s}");
        assert!(
            s.contains("counter(page).get().first() == 37"),
            "la page visée n'est pas dans la condition : {s}"
        );
        // Le flux ne doit rien recevoir : un `#pagebreak` de plus, et le compte bouge.
        let sans = source_avec(None);
        assert_eq!(
            s.matches("#pagebreak()").count(),
            sans.matches("#pagebreak()").count(),
            "l'envoi a ajouté une rupture de page"
        );
    }

    /// Le `foreground` se pose au préambule, une fois : un `#set page(…)` au milieu du
    /// document ouvrirait une page. Il doit donc paraître **avant** le premier contenu.
    #[test]
    fn le_foreground_est_au_preambule() {
        let s = source_avec(Some(trace()));
        let f = s.find("foreground:").expect("pas de foreground");
        let premier_contenu = s.find("#v(42mm)").expect("pas de faux-titre");
        assert!(f < premier_contenu, "le foreground est posé après le contenu : {s}");
    }

    /// Hors du `foreground`, la source ne bouge pas d'un octet : c'est ce qui garantit
    /// que tous les exemplaires d'un tirage partagent la même pagination.
    #[test]
    fn un_envoi_ne_touche_que_le_foreground() {
        let avec = source_avec(Some(trace()));
        let sans = source_avec(None);
        let debut = avec.find("foreground:").expect("pas de foreground");
        let fin = avec[debut..].find("\n)").expect("foreground non refermé") + debut;
        let ampute = format!("{}{}", &avec[..debut], &avec[fin..]);
        let sans_virgule = ampute.replace("\n  ,", "");
        assert_eq!(
            sans_virgule.replace(char::is_whitespace, ""),
            sans.replace(char::is_whitespace, ""),
            "l'envoi a modifié la source hors du foreground"
        );
    }

    /// L'échelle grossit l'objet entier, lettres comprises : tirer un coin à la souris
    /// agrandit une signature, il n'élargit pas une colonne de texte pour la laisser se
    /// recomposer. Le corps suit donc la taille.
    #[test]
    fn l_echelle_emporte_le_corps() {
        let petit = Place { taille: 0.30, ..*PLACE };
        let grand = Place { taille: 0.60, ..*PLACE };
        let sp = corps_de(&source_avec(Some(Trace {
            quoi: Quoi::Texte { police: "Caveat", texte: "À Léa," }, place: &petit })));
        let sg = corps_de(&source_avec(Some(Trace {
            quoi: Quoi::Texte { police: "Caveat", texte: "À Léa," }, place: &grand })));
        assert!(sg > sp * 1.9 && sg < sp * 2.1, "le corps n'a pas doublé : {sp} → {sg}");
    }

    /// L'inclinaison passe par `rotate`, dont l'origine est le centre — comme en CSS,
    /// sans quoi le canevas et Typst ne montreraient pas la même chose.
    #[test]
    fn l_inclinaison_passe_par_rotate() {
        let p = Place { angle: -4.0, ..*PLACE };
        let s = source_avec(Some(Trace {
            quoi: Quoi::Texte { police: "Caveat", texte: "À Léa," }, place: &p }));
        assert!(s.contains("rotate(-4"), "pas de rotation : {s}");
    }
```

Deux aides du module de tests, à écrire à côté de `trace()` :

```rust
    /// La source d'un intérieur ordinaire, avec ou sans envoi : tout ce que ces tests
    /// comparent est ce que l'envoi y change.
    fn source_avec(envoi: Option<Trace>) -> String {
        let livre = livre();
        let int = Interieur::default();
        let pr = crate::providers::provider("kdp-5x8").expect("gabarit kdp-5x8");
        let r = Reglage { gouttiere: pr.gouttieres[0].2, blanche: false };
        source(&livre, &int, pr, &r, &pieces(), envoi)
    }

    /// Le corps composé de l'envoi, en millimètres, relevé dans la source.
    fn corps_de(s: &str) -> f64 {
        let i = s.find("size: ").expect("pas de corps d'envoi") + "size: ".len();
        let j = s[i..].find("mm").expect("corps non exprimé en mm") + i;
        s[i..j].parse().expect("corps illisible")
    }
```

`livre()` et `pieces()` existent déjà dans le module de tests — les réutiliser
tels quels ; si leurs noms diffèrent, prendre ceux du fichier.

- [ ] **Step 2 : lancer les tests pour les voir échouer**

Run: `cargo test --lib interieur`
Expected: FAIL — `Trace` n'a pas de champ `quoi`, `Quoi` n'existe pas, et aucune
source ne contient `foreground:`.

- [ ] **Step 3 : écrire le `foreground`**

`Trace` se scinde. `interieur.rs:129` :

```rust
/// Ce qu'un envoi dépose sur sa page.
///
/// `interieur` ne connaît ni la main du livre, ni d'où l'image vient : il reçoit ce
/// que l'envoi a décidé. Une image écrite à la main et une image produite par un
/// modèle de diffusion arrivent ici de la même façon — ce module n'a pas à savoir
/// laquelle, seulement qu'elle est posée à côté de la source.
#[derive(Debug, Clone, Copy)]
pub enum Quoi<'a> {
    /// Un texte, composé dans la main de cet envoi.
    Texte { police: &'a str, texte: &'a str },
    /// Une image, déjà écrite à côté de la source, désignée par son seul nom.
    Image { fichier: &'a str },
}

/// Un envoi et sa place sur la page.
#[derive(Debug, Clone, Copy)]
pub struct Trace<'a> {
    pub quoi: Quoi<'a>,
    pub place: &'a crate::envoi::Place,
}

/// Le rapport entre la largeur de l'objet et le corps de son écriture.
///
/// L'objet est self-similaire : l'agrandir agrandit les lettres, parce que tirer un
/// coin à la souris agrandit une signature — il n'élargit pas une colonne de texte
/// pour la laisser se recomposer. Le corps suit donc la taille.
///
/// La valeur cale le nouveau réglage sur l'ancien : jusqu'à la v4, l'envoi se composait
/// en 14 pt dans un bloc de 70 % de la justification. Sur une page de 127 mm, une
/// taille de 0,60 donne 76,2 mm de large, et 14 pt valent 4,94 mm — d'où 4,94 / 76,2.
const CORPS_SUR_LARGEUR: f64 = 0.0648;
```

Dans `assemble`, l'entête reçoit le `foreground`. Remplacer la ligne
`  footer: none,\n)` du `format!` de l'entête par `  footer: none,{}\n)`, et
passer `&foreground(envoi, fw)` en argument. Puis :

```rust
/// Ce que l'envoi ajoute à `#set page` : un `foreground` conditionné au numéro de page.
///
/// **`foreground` et non le flux.** Un `#place` dans le flux ne pouvait déjà pas créer
/// de page ; il fallait en revanche l'écrire là où la page visée se compose, ce qui
/// enfermait l'envoi sur la page de titre. Le `foreground`, lui, se pose une fois au
/// préambule et vise n'importe quelle page — un `#set page(…)` au milieu du document
/// ouvrirait une page, d'où le préambule et lui seul.
///
/// Il survit au `#set page(footer: …)` qui ouvre le corps, les `set` de Typst
/// fusionnant champ à champ, et aux `#page(…)[…]` des pages de partie. Ses pourcentages
/// se résolvent sur la **page entière, marges comprises** : c'est ce qui les met en
/// correspondance 1:1 avec le canevas de l'interface, qui montre la page entière.
///
/// `counter(page)` n'est jamais remis à zéro dans l'intérieur — seul son affichage est
/// masqué jusqu'au corps —, si bien que la condition porte bien sur la n-ième page du
/// fichier, celle que la vignette montre.
fn foreground(envoi: Option<Trace>, largeur_mm: f64) -> String {
    let Some(t) = envoi else {
        return String::new();
    };
    let p = t.place;
    let quoi = match t.quoi {
        Quoi::Texte { police, texte } => format!(
            r#"box(width: {taille}%)[
        #set par(justify: false, first-line-indent: 0pt, leading: 0.9em)
        #text(font: "{police}", size: {corps:.3}mm, hyphenate: false)[{mot}]
      ]"#,
            taille = p.taille * 100.0,
            // La main est validée en amont par `Envois::verifie` : pas d'échappement.
            mot = echappe(texte).replace('\n', r" \ "),
            corps = p.taille * largeur_mm * CORPS_SUR_LARGEUR,
        ),
        // Le nom du fichier est fabriqué par `envoi::nom_image` : assaini, il ne porte
        // ni guillemet qui refermerait la chaîne, ni séparateur qui la ferait sortir du
        // répertoire où l'image vient d'être écrite.
        //
        // Aucune borne de hauteur, contrairement à la v3 : elle protégeait d'un envoi
        // qui recouvrirait le titre, or le canevas montre désormais ce recouvrement, et
        // le brider corrigerait l'auteur d'une faute qu'il voit.
        Quoi::Image { fichier } => format!(
            r#"image("{fichier}", width: {}%)"#,
            p.taille * 100.0
        ),
    };
    format!(
        r#"
  foreground: context {{
    if counter(page).get().first() == {page} {{
      place(center + horizon, dx: {dx}%, dy: {dy}%, rotate({angle}deg, {quoi}))
    }}
  }},"#,
        page = p.page,
        dx = (p.x - 0.5) * 100.0,
        dy = (p.y - 0.5) * 100.0,
        angle = p.angle,
    )
}
```

Dans `liminaires`, retirer le bloc `match envoi { … }` en entier et le paramètre
`envoi` de la signature ; l'appel devient
`s.push_str(&liminaires(livre, liminaires_manuscrit));`.

- [ ] **Step 4 : lancer les tests**

Run: `cargo test --lib interieur`
Expected: PASS. Les tests de la v3 qui comptaient les `#place(` —
`un_envoi_ne_cree_aucune_page`, `un_envoi_ne_touche_que_la_page_de_titre`,
`une_image_d_envoi_ne_cree_aucune_page_non_plus` — sont remplacés par ceux de
l'étape 1 : les retirer. Garder et adapter
`l_envoi_est_compose_dans_la_main_du_livre` (renommé
`l_envoi_est_compose_dans_sa_main`), `un_envoi_n_est_pas_justifie`,
`un_envoi_ne_cesure_pas`, `un_envoi_est_echappe_et_garde_ses_sauts_de_ligne`,
`une_image_d_envoi_n_emporte_aucune_police` — ils portent sur le contenu de
l'objet, qui n'a pas changé de nature.

- [ ] **Step 5 : commiter**

```bash
git add app/src-tauri/src/interieur.rs
git commit -m "L'envoi se pose en foreground, sur la page qu'on veut

Un #place dans le flux ne pouvait déjà pas créer de page, mais il fallait
l'écrire là où la page visée se compose : l'envoi était enfermé sur la page de
titre. Le foreground se pose une fois au préambule et vise n'importe quelle page.

Il survit au set page(footer:) du corps et aux page() de partie, et ses
pourcentages portent sur la page entière — d'où la correspondance 1:1 avec le
canevas. Le corps suit la taille : agrandir une signature agrandit ses lettres."
```

---

## Task 4 : le test réel — aucune page créée, quelle que soit la page

**Files:**
- Test: `app/src-tauri/tests/` (nouveau fichier `envoi_pagination.rs`) ou module `tests` d'`interieur.rs` selon ce que le dépôt fait déjà

- [ ] **Step 1 : voir où vivent les tests qui lancent Typst**

```bash
ls app/src-tauri/tests/ 2>/dev/null
grep -rn "Typst::new\|binaires/" app/src-tauri/src/*.rs app/src-tauri/tests/*.rs 2>/dev/null | grep -i test
```

Suivre ce qui existe. S'il n'y a aucun test intégré lançant Typst, en créer un
dans `app/src-tauri/tests/envoi_pagination.rs`, marqué `#[ignore]` s'il faut le
sidecar — et **le lancer explicitement** à l'étape 3, un test ignoré ne protégeant
rien.

- [ ] **Step 2 : écrire le test**

```rust
//! L'invariant qui tient toute la chaîne des envois, vérifié en composant pour de
//! vrai.
//!
//! Compter les `#place` ou les `#pagebreak` dans la source ne prouve rien : c'est
//! Typst qui décide du nombre de pages, et lui seul. Si cet invariant tombe, le dos
//! est faux, la planche est fausse, et les exemplaires partent à l'impression sans
//! que rien ne le signale.

/// Un envoi ne crée aucune page, **quelle que soit la page visée** — la première, une
/// page de partie, la dernière.
#[test]
#[ignore = "lance le sidecar Typst : cargo test --test envoi_pagination -- --ignored"]
fn un_envoi_ne_cree_aucune_page_ou_qu_il_se_pose() {
    let (livre, int, pr, reglage, pieces, typst, dossier) = atelier();

    let src = dossier.path().join("sans.typ");
    std::fs::write(&src, interieur::source(&livre, &int, pr, &reglage, &pieces, None)).unwrap();
    let sans = typst.pages(&src).expect("pagination sans envoi");
    assert!(sans > 10, "le manuscrit témoin est trop court pour ce test : {sans}");

    for page in [1, 3, sans / 2, sans] {
        let place = Place { page, x: 0.5, y: 0.8, taille: 0.6, angle: -4.0 };
        let t = Trace {
            quoi: Quoi::Texte { police: "Caveat", texte: "À Léa,\nces heures creuses." },
            place: &place,
        };
        let src = dossier.path().join(format!("avec-{page}.typ"));
        std::fs::write(&src, interieur::source(&livre, &int, pr, &reglage, &pieces, Some(t))).unwrap();
        assert_eq!(
            typst.pages(&src).expect("pagination avec envoi"),
            sans,
            "un envoi posé page {page} a déplacé la pagination"
        );
    }
}
```

`atelier()` monte le décor : un livre, le gabarit `kdp-5x8`, les pièces d'un
manuscrit d'essai, un `Typst` pointant sur `binaires/` et `fonts/`, et un
`tempfile::TempDir`. Le construire en s'inspirant d'`examples/composer.rs`, qui
fait exactement ce montage. Si `tempfile` n'est pas déjà une dépendance de dev,
utiliser `std::env::temp_dir().join("ozalid-test-envoi")` et le nettoyer en fin
de test plutôt que d'ajouter une dépendance.

- [ ] **Step 3 : le voir échouer, puis passer**

D'abord le voir **rouge** par mutation ciblée : dans `interieur::foreground`,
remplacer temporairement `place(center + horizon, …)` par un `pagebreak()` suivi
du même `place`, et lancer :

Run: `cargo test --test envoi_pagination -- --ignored`
Expected: FAIL — « un envoi posé page 1 a déplacé la pagination ».

Défaire la mutation, relancer :

Expected: PASS pour les quatre pages.

- [ ] **Step 4 : commiter**

```bash
git add app/src-tauri/tests/envoi_pagination.rs
git commit -m "Le témoin de l'invariant compose pour de vrai

Compter les #place dans la source ne prouve rien : c'est Typst qui décide du
nombre de pages. Quatre pages visées — la première, la page de titre, le milieu,
la dernière —, et le compte doit être celui du livre sans envoi.

Vu rouge par mutation : un pagebreak glissé dans le foreground le fait tomber."
```

---

## Task 5 : la trace lit la main de l'envoi, et la page hors bornes refuse

**Files:**
- Modify: `app/src-tauri/src/package.rs:173-204` (`trace`), `:211-280` (`assembler_envois`)
- Test: `app/src-tauri/src/package.rs` (module `tests`)

- [ ] **Step 1 : écrire le test qui échoue**

```rust
    /// Le même manuscrit ne fait pas le même nombre de pages en poche et en grand
    /// format : une page choisie à l'œil chez l'un peut n'exister chez l'autre. Rogner
    /// sur la dernière page enverrait à l'impression un exemplaire que personne n'a
    /// voulu ; le refus nomme la personne, la page et le compte, comme le fait déjà le
    /// dos non publié.
    #[test]
    fn une_page_hors_bornes_fait_refuser_la_generation() {
        let err = verifie_pages(
            &[
                Envoi { dedicataire: "Léa".into(), place: Place { page: 3, ..Default::default() }, ..Default::default() },
                Envoi { dedicataire: "Marc".into(), place: Place { page: 210, ..Default::default() }, ..Default::default() },
            ],
            198,
        )
        .unwrap_err();
        assert!(err.contains("Marc"), "{err}");
        assert!(err.contains("210"), "{err}");
        assert!(err.contains("198"), "{err}");
        assert!(!err.contains("Léa"), "Léa n'est pas en cause : {err}");
    }

    /// Page 0 n'existe pas : les pages de Typst comptent à partir de 1, et un zéro
    /// venu d'un TOML écrit à la main ne doit pas composer un envoi invisible.
    #[test]
    fn la_page_zero_est_refusee() {
        let err = verifie_pages(
            &[Envoi { dedicataire: "Léa".into(), place: Place { page: 0, ..Default::default() }, ..Default::default() }],
            198,
        )
        .unwrap_err();
        assert!(err.contains("Léa"), "{err}");
    }
```

- [ ] **Step 2 : lancer le test pour le voir échouer**

Run: `cargo test --lib package::tests::une_page_hors_bornes`
Expected: FAIL — `cannot find function verifie_pages in this scope`.

- [ ] **Step 3 : écrire le contrôle et corriger `trace`**

Dans `package.rs` :

```rust
/// Refuse un envoi placé sur une page que l'intérieur de ce prestataire n'a pas.
///
/// Le même manuscrit ne fait pas le même nombre de pages en poche et en grand format.
/// Pour les liminaires — faux-titre, blanche, titre, copyright, dédicace — les pages
/// coïncident d'un format à l'autre, et c'est là qu'un envoi va dans les faits.
/// Ailleurs, on refuse en disant quoi faire, le chiffre mesuré compris : c'est la
/// convention du dos non publié.
fn verifie_pages(liste: &[crate::envoi::Envoi], pages: u32) -> Result<(), String> {
    for (i, e) in liste.iter().enumerate() {
        if e.place.page >= 1 && e.place.page <= pages {
            continue;
        }
        let qui = if e.dedicataire.trim().is_empty() {
            format!("envoi {}", i + 1)
        } else {
            e.dedicataire.clone()
        };
        return Err(format!(
            "{qui} : envoi placé page {}, l'intérieur n'en fait que {pages}.",
            e.place.page
        ));
    }
    Ok(())
}
```

Dans `assembler_envois`, juste après `let base = assembler(…)?;` — le compte de
pages n'est connu qu'une fois la référence composée :

```rust
    // Le compte de pages n'existe qu'après la convergence : le contrôle ne peut pas
    // avoir lieu plus tôt, et refuser ici coûte une composition de moins qu'un tirage
    // faux.
    verifie_pages(&envois.liste, base.pages)?;
```

Vérifier le nom du champ portant le compte sur `Package` (`base.pages` d'après
`envois.js`, qui affiche `r.package.pages`) et l'employer tel quel.

`trace` lit la main de l'envoi et rend une `Trace` complète :

```rust
pub fn trace<'a>(
    projet: &'a Projet,
    e: &'a crate::envoi::Envoi,
    dossier: &Path,
) -> Result<interieur::Trace<'a>, String> {
    let qui = if e.dedicataire.trim().is_empty() {
        "cet envoi"
    } else {
        &e.dedicataire
    };
    let quoi = match &e.main {
        crate::envoi::Main::Police { police } => interieur::Quoi::Texte {
            police,
            texte: &e.contenu,
        },
        // Générée ou écrite à la main, une image est une image : elle a été acceptée,
        // elle est dans l'archive, et composer ne rappelle jamais le réseau.
        crate::envoi::Main::Image | crate::envoi::Main::Diffusion => {
            let fichier = e
                .image
                .as_deref()
                .ok_or_else(|| format!("{qui} n'a pas d'image : en choisir une."))?;
            let octets = projet.images_envois.get(fichier).ok_or_else(|| {
                format!("{qui} : l'image « {fichier} » ne figure pas dans le projet.")
            })?;
            std::fs::write(dossier.join(fichier), octets)
                .map_err(|err| format!("{fichier} : écriture impossible : {err}"))?;
            interieur::Quoi::Image { fichier }
        }
    };
    Ok(interieur::Trace { quoi, place: &e.place })
}
```

- [ ] **Step 4 : lancer toute la suite**

Run: `cargo test`
Expected: PASS. C'est ici que le lot A se referme : les modules qui ne
compilaient plus depuis la tâche 1 sont rattrapés.

- [ ] **Step 5 : `fmt`, `clippy`, témoin, puis commiter**

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo run --example temoin
```

Le compte de pages du témoin doit être **identique** au précédent sur le même
manuscrit. S'il a bougé, la tâche 3 a touché le flux : reprendre avant de
commiter.

```bash
git add app/src-tauri/src/package.rs
git commit -m "Chaque envoi compose dans sa main, et sa page doit exister

trace lit la main de l'envoi et non celle du livre. Une page que l'intérieur du
prestataire n'a pas fait refuser la génération, en nommant la personne, la page
et le compte — la convention du dos non publié. Le contrôle a lieu après la
convergence : le compte de pages n'existe pas plus tôt."
```

---

# Lot C — les commandes

## Task 6 : `Typst::apercus`, toutes les pages en une invocation

**Files:**
- Modify: `app/src-tauri/src/typst.rs:107-120`

- [ ] **Step 1 : écrire la méthode**

Rendre page à page coûterait une composition complète par page — 190 fois pour un
livre ordinaire. Typst substitue `{p}` dans le nom de sortie et rasterise en
parallèle : mesuré à 0,58 s pour 190 pages à 24 ppi.

```rust
    /// Rend **toutes** les pages en PNG, en une seule invocation, et rend leurs chemins
    /// dans l'ordre.
    ///
    /// `motif` porte le `{p}` que Typst substitue par le numéro de page. Appeler
    /// `apercu` page à page coûterait une composition complète par page — 190 fois pour
    /// un livre ordinaire, là où l'invocation unique rasterise en parallèle.
    ///
    /// Les chemins sont relus dans le répertoire plutôt que fabriqués : Typst décide
    /// seul du remplissage du numéro, et deviner « 1 » quand il écrit « 001 » rendrait
    /// une liste de fichiers absents.
    pub fn apercus(&self, source: &Path, motif: &Path, ppi: u32) -> Result<Vec<PathBuf>, String> {
        self.lance(&[
            "compile",
            "--format",
            "png",
            "--ppi",
            &ppi.to_string(),
            &chemin(source)?,
            &chemin(motif)?,
        ])?;
        let dossier = motif
            .parent()
            .ok_or_else(|| format!("motif sans répertoire : {}", motif.display()))?;
        let prefixe = motif
            .file_name()
            .and_then(|n| n.to_str())
            .and_then(|n| n.split("{p}").next())
            .ok_or_else(|| format!("motif sans « {{p}} » : {}", motif.display()))?;
        let mut pages: Vec<(u32, PathBuf)> = std::fs::read_dir(dossier)
            .map_err(|e| format!("vignettes illisibles ({}) : {e}", dossier.display()))?
            .filter_map(Result::ok)
            .filter_map(|e| {
                let nom = e.file_name().into_string().ok()?;
                let n = nom.strip_prefix(prefixe)?.strip_suffix(".png")?;
                Some((n.parse().ok()?, e.path()))
            })
            .collect();
        pages.sort_by_key(|(n, _)| *n);
        Ok(pages.into_iter().map(|(_, c)| c).collect())
    }
```

- [ ] **Step 2 : écrire le test qui échoue**

Dans le module `tests` de `typst.rs`, ou dans le fichier de tests intégrés créé à
la tâche 4 s'il n'y en a pas :

```rust
    /// Une invocation, N fichiers, dans l'ordre des pages. Rendre page à page coûterait
    /// une composition complète par page.
    #[test]
    #[ignore = "lance le sidecar Typst"]
    fn toutes_les_pages_sortent_en_une_invocation() {
        let (typst, dossier) = atelier_typst();
        let src = dossier.path().join("trois.typ");
        std::fs::write(&src, "un #pagebreak() deux #pagebreak() trois").unwrap();
        let pages = typst
            .apercus(&src, &dossier.path().join("p{p}.png"), 20)
            .expect("rendu des pages");
        assert_eq!(pages.len(), 3, "{pages:?}");
        assert!(pages.iter().all(|p| p.exists()), "{pages:?}");
        assert!(
            pages[0] < pages[1] && pages[1] < pages[2],
            "les pages ne sont pas dans l'ordre : {pages:?}"
        );
    }
```

- [ ] **Step 3 : le voir échouer, puis passer**

Le voir rouge d'abord : commenter le `pages.sort_by_key(…)` et lancer

Run: `cargo test --lib typst -- --ignored`
Expected: FAIL sur l'ordre (l'ordre de `read_dir` n'est pas garanti ; si le test
passe quand même, forcer le rouge en triant à l'envers).

Rétablir, relancer : PASS.

- [ ] **Step 4 : commiter**

```bash
git add app/src-tauri/src/typst.rs
git commit -m "Toutes les pages en une invocation

Typst substitue {p} et rasterise en parallèle : 0,58 s pour les 190 pages du
livre témoin à 24 ppi, là où page à page coûterait 190 compositions complètes.
Les chemins sont relus dans le répertoire plutôt que fabriqués — Typst décide
seul du remplissage du numéro."
```

---

## Task 7 : les commandes de rendu

**Files:**
- Modify: `app/src-tauri/src/commands.rs`, `app/src-tauri/src/lib.rs:121-130`

- [ ] **Step 1 : écrire `envoi_vignettes` et `envoi_page`**

Les deux rendent depuis `interieur::source(…, None)` : le fond est la page **sans
envoi**, invariante puisqu'un envoi posé en `foreground` ne réordonne rien. Elle
sert donc à tous les dédicataires.

```rust
/// Toutes les pages de l'intérieur, en vignettes, pour le destinataire visé.
///
/// Rendues **sans envoi** : un envoi posé en `foreground` ne réordonne rien, la page de
/// fond ne dépend donc d'aucun dédicataire et la même série sert à tous.
///
/// Le répertoire est nommé par l'empreinte de la source. Une composition qui change
/// change l'empreinte, donc le répertoire : il n'y a pas d'invalidation à écrire,
/// seulement un nom à calculer. Ce qui reste est dans le temporaire du système, qui est
/// fait pour cela.
///
/// Les données partent en URL `data:` comme tous les aperçus de cette fenêtre : 190
/// pages à 24 ppi pèsent 1,9 Mo, une fois par composition.
#[tauri::command]
pub fn envoi_vignettes(atelier: State<Atelier>) -> Result<Vec<String>, String> {
    let garde = atelier.ouvert.lock().unwrap();
    let o = garde.as_ref().ok_or_else(aucun_projet)?;
    let (src, dossier) = source_de_fond(o)?;
    let motif = dossier.join("v{p}.png");
    // Aucun cache : 190 pages coûtent six dixièmes de seconde, et l'interface ne
    // demande cette série qu'à l'ouverture de l'étape. Un cache achèterait ce
    // dixième-là au prix d'une invalidation à tenir juste.
    let pages = typst()?.apercus(&src, &motif, VIGNETTE_PPI)?;
    pages.iter().map(|p| donnee_png(p)).collect()
}

/// Une page de l'intérieur, en grand, pour le canevas de placement.
#[tauri::command]
pub fn envoi_page(page: u32, atelier: State<Atelier>) -> Result<String, String> {
    let garde = atelier.ouvert.lock().unwrap();
    let o = garde.as_ref().ok_or_else(aucun_projet)?;
    let (src, dossier) = source_de_fond(o)?;
    let png = dossier.join(format!("grand-{page}.png"));
    typst()?.apercu(&src, &png, page, PAGE_PPI)?;
    donnee_png(&png)
}
```

Les deux constantes et l'aide commune :

```rust
/// 120 px de large sur une page de 127 mm : de quoi reconnaître une page dans un rail.
const VIGNETTE_PPI: u32 = 24;
/// 750 px de large sur la même page : de quoi placer un envoi à la souris.
const PAGE_PPI: u32 = 150;

/// La source de l'intérieur **sans envoi**, écrite dans un répertoire nommé par son
/// empreinte, et ce répertoire.
fn source_de_fond(o: &Ouvert) -> Result<(PathBuf, PathBuf), String> {
    let (pr, papier, d) = vise(o)?;
    let int = &o.projet.meta.interieur;
    int.verifie()?;
    let livre = &o.projet.meta.livre;
    let chapitres = manuscrit::decoupe(&o.projet.texte, livre.chapitres)?;
    let mesure = d
        .compose
        .as_ref()
        .ok_or("intérieur non composé : le placement a besoin des pages du tirage.")?;
    let reglage = Reglage {
        gouttiere: mesure.gouttiere,
        blanche: mesure.blanche,
    };
    let src = interieur::source(livre, int, pr, &reglage, &chapitres, None);
    let dossier = std::env::temp_dir()
        .join("ozalid-pages")
        .join(empreinte(&src));
    std::fs::create_dir_all(&dossier)
        .map_err(|e| format!("répertoire inutilisable ({}) : {e}", dossier.display()))?;
    let chemin = dossier.join("fond.typ");
    ecrire(&chemin, &src)?;
    let _ = papier;
    Ok((chemin, dossier))
}
```

`empreinte` : reprendre la fonction de hachage déjà employée dans le dépôt si
elle existe (`grep -rn "fn empreinte\|DefaultHasher" app/src-tauri/src/`). Sinon :

```rust
/// Une empreinte courte et stable d'une source, pour nommer son répertoire de rendus.
///
/// `DefaultHasher` suffit : ce n'est pas un contrôle d'intégrité, seulement un nom qui
/// change quand la source change. Une collision coûterait des vignettes périmées, pas
/// un mauvais tirage.
fn empreinte(s: &str) -> String {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    s.hash(&mut h);
    format!("{:016x}", h.finish())
}
```

Le nom des champs de `mesure` (`gouttiere`, `blanche`) est à vérifier sur le type
réel : `grep -n "pub struct Compose" -A 10 app/src-tauri/src/projet.rs`.

- [ ] **Step 2 : écrire `envoi_objet`**

```rust
/// L'objet d'un envoi, rendu seul sur fond transparent, avec son rapport hauteur sur
/// largeur.
///
/// C'est ce que le canevas manipule. Le rendre par Typst plutôt que de l'imiter en CSS
/// fait que ce qu'on déplace **est** ce qui s'imprimera : même police, même corps, même
/// coupure de lignes. Une page en fond, cet objet par-dessus, et glisser n'est plus
/// qu'un `transform` — Typst n'est rappelé que quand le mot ou la main changent.
///
/// La largeur de rendu est fixe et la hauteur automatique : l'échelle est appliquée par
/// le canevas, l'objet étant self-similaire.
#[tauri::command]
pub fn envoi_objet(index: usize, atelier: State<Atelier>) -> Result<Objet, String> {
    let garde = atelier.ouvert.lock().unwrap();
    let o = garde.as_ref().ok_or_else(aucun_projet)?;
    let envois = &o.projet.meta.envois;
    envois.verifie()?;
    let e = envois
        .liste
        .get(index)
        .ok_or("envoi introuvable : la liste a changé.")?;
    let (pr, _, _) = vise(o)?;

    let dossier = std::env::temp_dir().join("ozalid-objet");
    std::fs::create_dir_all(&dossier)
        .map_err(|err| format!("répertoire inutilisable ({}) : {err}", dossier.display()))?;
    let largeur = pr.format.0 * e.place.taille;
    let src = dossier.join("objet.typ");
    ecrire(&src, &interieur::source_objet(&package::trace(&o.projet, e, &dossier)?, largeur))?;
    let png = dossier.join("objet.png");
    // L'écriture de l'auteur vit dans le `.ozalid` : sans ce dépliage, l'objet
    // composerait dans la police de repli, et le canevas montrerait autre chose que ce
    // qui s'imprimera.
    let typst = typst()?;
    let typst = match package::ecrire_polices(&o.projet, &dossier)? {
        Some(d) => typst.avec_polices(d),
        None => typst,
    };
    typst.apercu(&src, &png, 1, OBJET_PPI)?;
    let (l, h) = crate::image::dimensions(&std::fs::read(&png).map_err(|e| e.to_string())?)?;
    Ok(Objet {
        image: donnee_png(&png)?,
        ratio: h as f64 / l as f64,
    })
}

/// 300 ppi : l'objet est agrandi par le canevas, et une signature pixelisée sous la
/// souris ferait douter du rendu.
const OBJET_PPI: u32 = 300;

/// L'objet d'un envoi, tel que le canevas le manipule.
#[derive(serde::Serialize)]
pub struct Objet {
    /// Le PNG, fond transparent, prêt à poser dans une balise `img`.
    pub image: String,
    /// Hauteur sur largeur : le canevas en a besoin pour dessiner ses prises avant que
    /// l'image ne soit chargée.
    pub ratio: f64,
}
```

`crate::image::dimensions` : vérifier le nom réel dans `image.rs`
(`grep -n "pub fn" app/src-tauri/src/image.rs`) — le README annonce « dimensions
naturelles d'une image ».

Et dans `interieur.rs`, la source d'un objet seul :

```rust
/// La source d'un envoi rendu **seul**, sur fond transparent, à hauteur automatique.
///
/// C'est ce que le canevas de placement manipule : le rendre par Typst plutôt que de
/// l'imiter en CSS fait que ce qu'on déplace est ce qui s'imprimera. `fill: none` donne
/// le fond transparent, `height: auto` laisse la hauteur suivre le texte.
pub fn source_objet(t: &Trace, largeur_mm: f64) -> String {
    let quoi = match t.quoi {
        Quoi::Texte { police, texte } => format!(
            r#"#set par(justify: false, first-line-indent: 0pt, leading: 0.9em)
#set text(font: "{police}", size: {corps:.3}mm, hyphenate: false, lang: "fr")
{mot}
"#,
            corps = largeur_mm * CORPS_SUR_LARGEUR,
            mot = echappe(texte).replace('\n', r" \ "),
        ),
        Quoi::Image { fichier } => format!("#image(\"{fichier}\", width: 100%)\n"),
    };
    format!(
        "#set page(width: {largeur_mm}mm, height: auto, margin: 0pt, fill: none)\n{quoi}"
    )
}
```

- [ ] **Step 3 : enregistrer les commandes**

`lib.rs`, dans la liste de l'`invoke_handler`, à côté des autres `envoi_` :

```rust
            commands::envoi_vignettes,
            commands::envoi_page,
            commands::envoi_objet,
```

- [ ] **Step 4 : corriger `envoi_apercu`**

`commands.rs:1432` compose l'intérieur **privé de ses chapitres** (`&[]`) et rend
la page 3 : l'optimisation devient fausse dès qu'un envoi vise une autre page —
la page 37 n'existe pas dans un intérieur sans corps. Remplacer par la source
complète et la page visée. La mesure la rend indolore : 0,19 s pour les 190 pages
du livre témoin.

```rust
/// La page d'un envoi, telle qu'elle sera imprimée.
///
/// La source est celle de l'intérieur **entier**, et non plus privée de ses chapitres :
/// l'envoi vise n'importe quelle page depuis la v4, et une page 37 n'existe pas dans un
/// intérieur sans corps. Composer le livre complet coûte deux dixièmes de seconde sur
/// un manuscrit de 190 pages — moins que la surprise d'un aperçu qui ne montre pas la
/// bonne page.
#[tauri::command]
pub fn envoi_apercu(index: usize, atelier: State<Atelier>) -> Result<String, String> {
    let garde = atelier.ouvert.lock().unwrap();
    let o = garde.as_ref().ok_or_else(aucun_projet)?;
    let (pr, _, d) = vise(o)?;
    let envois = &o.projet.meta.envois;
    envois.verifie()?;
    let e = envois
        .liste
        .get(index)
        .ok_or("envoi introuvable : la liste a changé.")?;

    let int = &o.projet.meta.interieur;
    int.verifie()?;
    let livre = &o.projet.meta.livre;
    let chapitres = manuscrit::decoupe(&o.projet.texte, livre.chapitres)?;
    let mesure = d
        .compose
        .as_ref()
        .ok_or("intérieur non composé : l'aperçu a besoin des pages du tirage.")?;
    let dossier = sorties_racine(o)?.join("envois");
    std::fs::create_dir_all(&dossier)
        .map_err(|err| format!("répertoire inutilisable ({}) : {err}", dossier.display()))?;
    let src = dossier.join("apercu.typ");
    ecrire(
        &src,
        &interieur::source(
            livre,
            int,
            pr,
            &Reglage { gouttiere: mesure.gouttiere, blanche: mesure.blanche },
            &chapitres,
            Some(package::trace(&o.projet, e, &dossier)?),
        ),
    )?;
    let png = dossier.join("apercu.png");
    let typst = typst()?;
    let typst = match package::ecrire_polices(&o.projet, &dossier)? {
        Some(d) => typst.avec_polices(d),
        None => typst,
    };
    typst.apercu(&src, &png, e.place.page, 110)?;
    donnee_png(&png)
}
```

- [ ] **Step 5 : bâtir et commiter**

```bash
cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
git add app/src-tauri/src/commands.rs app/src-tauri/src/interieur.rs app/src-tauri/src/lib.rs
git commit -m "Les rendus dont le canevas a besoin

envoi_vignettes rend toutes les pages sans envoi — la page de fond ne dépend
d'aucun dédicataire, un foreground ne réordonnant rien. envoi_page en rend une en
grand, envoi_objet rend l'envoi seul sur fond transparent : ce qu'on déplacera à
la souris est ce qui s'imprimera.

envoi_apercu perd son raccourci : il composait l'intérieur privé de ses chapitres
pour rendre la page 3, ce qui devient faux dès qu'un envoi vise la page 37."
```

---

## Task 8 : `envoi_regler`, `envoi_ajouter`, `envoi_retirer`

**Files:**
- Modify: `app/src-tauri/src/commands.rs:1251-1270` (`envois_modifier`), `app/src-tauri/src/lib.rs:121`

- [ ] **Step 1 : écrire les trois commandes**

`envois_modifier` disparaît : la liste entière ne voyage plus à chaque frappe, et
le piège qu'elle portait — « une main omise reviendrait au défaut » — s'en va
avec elle.

```rust
/// Remplace un envoi par lui-même modifié : sa main, son mot, son placement.
///
/// Un envoi et non la liste entière, contrairement à `envois_modifier` qui l'a
/// précédée : elle obligeait l'interface à renvoyer ce qu'elle n'avait pas modifié, et
/// une main omise revenait au défaut. Ici le rang désigne, et le reste ne bouge pas.
#[tauri::command]
pub fn envoi_regler(
    index: usize,
    envoi: crate::envoi::Envoi,
    atelier: State<Atelier>,
) -> Result<ProjetVue, String> {
    let mut garde = atelier.ouvert.lock().unwrap();
    let o = garde.as_mut().ok_or_else(aucun_projet)?;
    let mut envois = o.projet.meta.envois.clone();
    *envois
        .liste
        .get_mut(index)
        .ok_or("envoi introuvable : la liste a changé.")? = envoi;
    o.projet.regler_envois(envois)?;
    o.modifie = true;
    Ok(vue(o))
}

/// Ajoute un envoi, qui naît comme le précédent.
///
/// Même main, même placement que le dernier de la liste : sans cette règle, vingt
/// dédicataires demanderaient vingt fois le même réglage, et la ressemblance des
/// exemplaires d'un même tirage, qui était acquise quand la main appartenait au livre,
/// se paierait.
#[tauri::command]
pub fn envoi_ajouter(dedicataire: String, atelier: State<Atelier>) -> Result<ProjetVue, String> {
    let mut garde = atelier.ouvert.lock().unwrap();
    let o = garde.as_mut().ok_or_else(aucun_projet)?;
    let mut envois = o.projet.meta.envois.clone();
    let modele = envois.liste.last();
    envois.liste.push(crate::envoi::Envoi {
        dedicataire,
        main: modele.map(|e| e.main.clone()).unwrap_or_default(),
        place: modele.map(|e| e.place).unwrap_or_default(),
        // Le mot et l'image, eux, sont ce qui distingue un exemplaire : ils ne
        // s'héritent pas.
        ..Default::default()
    });
    o.projet.regler_envois(envois)?;
    o.modifie = true;
    Ok(vue(o))
}

/// Retire un envoi. Son image reste dans l'archive : c'est `regler_envois` qui élague
/// ce que plus aucun envoi ne désigne, comme il le faisait déjà.
#[tauri::command]
pub fn envoi_retirer(index: usize, atelier: State<Atelier>) -> Result<ProjetVue, String> {
    let mut garde = atelier.ouvert.lock().unwrap();
    let o = garde.as_mut().ok_or_else(aucun_projet)?;
    let mut envois = o.projet.meta.envois.clone();
    if index >= envois.liste.len() {
        return Err("envoi introuvable : la liste a changé.".into());
    }
    envois.liste.remove(index);
    o.projet.regler_envois(envois)?;
    o.modifie = true;
    Ok(vue(o))
}

/// Le gabarit de diffusion, partagé par tous les envois du livre.
#[tauri::command]
pub fn envois_gabarit(gabarit: String, atelier: State<Atelier>) -> Result<ProjetVue, String> {
    let mut garde = atelier.ouvert.lock().unwrap();
    let o = garde.as_mut().ok_or_else(aucun_projet)?;
    let mut envois = o.projet.meta.envois.clone();
    envois.gabarit = gabarit;
    o.projet.regler_envois(envois)?;
    o.modifie = true;
    Ok(vue(o))
}
```

Vérifier la forme exacte de `o.modifie` et de `vue(o)` sur `envois_modifier`
avant de l'effacer : ce sont les lignes à reproduire, et elles peuvent différer
(`Ok(vue(o))` ou `Ok(projet_vue(o))`).

`Projet::regler_envois` appelait sans doute `Envois::reprend`, qui n'existe plus.
Le corriger : il doit désormais appeler `envois.verifie()?` directement, la
`personnelle` étant conservée par le projet lui-même.

```bash
grep -n "fn regler_envois" -A 20 app/src-tauri/src/projet.rs
```

- [ ] **Step 2 : enregistrer, retirer l'ancienne**

`lib.rs` : retirer `commands::envois_modifier`, ajouter
`commands::envoi_regler`, `commands::envoi_ajouter`, `commands::envoi_retirer`,
`commands::envois_gabarit`.

- [ ] **Step 3 : bâtir**

Run: `cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test`
Expected: PASS. Le front ne compile pas — il n'est pas compilé —, mais il appelle
encore `envois_modifier` : c'est le lot D qui le rattrape, et l'application est
cassée à l'étape Envois entre les deux. C'est assumé et borné à un commit.

- [ ] **Step 4 : commiter**

```bash
git add app/src-tauri/src/commands.rs app/src-tauri/src/lib.rs app/src-tauri/src/projet.rs
git commit -m "Un envoi se règle seul, et naît comme le précédent

envois_modifier faisait voyager la liste entière à chaque frappe, et une main
omise revenait au défaut. envoi_regler désigne par le rang ; ajouter et retirer
sont deux commandes de liste.

Un envoi neuf hérite main et placement du dernier : sans cela, vingt dédicataires
demanderaient vingt fois le même réglage. Le mot et l'image, eux, sont ce qui
distingue un exemplaire."
```

---

# Lot D — le front

## Task 9 : `placement.js`, la géométrie

**Files:**
- Create: `app/src/placement.js`
- Test: `app/tests/placement.test.js`

Ce module ne touche pas au DOM : il reçoit des nombres et en rend. C'est ce qui
le rend testable sans fenêtre, et c'est aussi ce qui rend le canevas relisible —
la géométrie d'un côté, les écouteurs de l'autre.

- [ ] **Step 1 : écrire les tests qui échouent**

`app/tests/placement.test.js` :

```js
'use strict';

// La géométrie du placement, sans DOM : ce module reçoit des nombres et en rend.
// Le canevas, lui, se vérifie dans l'application — comme le rendu de la couverture.

const test = require('node:test');
const assert = require('node:assert');
const { charge } = require('./dom_shim');

const { deplace, redimensionne, incline, borne } = charge('placement.js');

const PLACE = { page: 3, x: 0.5, y: 0.8, taille: 0.6, angle: 0 };

/**
 * Un geste se mesure en **fraction du canevas**, jamais en pixels : le canevas
 * s'affiche à la taille que la fenêtre lui laisse, et un geste calé sur des pixels
 * irait deux fois plus vite dans une petite fenêtre. C'est la règle de `saisir()`.
 */
test('un glisser se mesure en fraction du canevas', () => {
  const petit = deplace(PLACE, { dx: 30, dy: 20 }, { largeur: 300, hauteur: 400 });
  const grand = deplace(PLACE, { dx: 60, dy: 40 }, { largeur: 600, hauteur: 800 });
  assert.equal(petit.x, grand.x);
  assert.equal(petit.y, grand.y);
  assert.equal(petit.x, 0.6);
  assert.equal(petit.y, 0.85);
});

/** L'objet reste sur sa page : un envoi glissé hors du papier ne s'imprimerait pas. */
test('le placement reste dans la page', () => {
  const loin = deplace(PLACE, { dx: 9000, dy: -9000 }, { largeur: 300, hauteur: 400 });
  assert.equal(loin.x, 1);
  assert.equal(loin.y, 0);
});

/**
 * La taille se prend sur la largeur, pas sur la diagonale : c'est elle que Typst
 * reçoit, et une prise qui suivrait la diagonale ferait diverger l'écran du rendu.
 */
test('la prise de coin règle la taille sur la largeur', () => {
  const p = redimensionne(PLACE, { dx: 30, dy: 0 }, { largeur: 300, hauteur: 400 });
  assert.equal(p.taille, 0.7);
  assert.equal(p.x, PLACE.x, 'le centre a bougé');
});

/** Une taille nulle ferait disparaître l'objet sans qu'on puisse le rattraper. */
test('la taille garde une borne basse attrapable', () => {
  const p = redimensionne(PLACE, { dx: -9000, dy: 0 }, { largeur: 300, hauteur: 400 });
  assert.ok(p.taille >= 0.05, `taille inattrapable : ${p.taille}`);
});

/**
 * La rotation se mesure autour du **centre** de l'objet — comme `rotate` en Typst et
 * `transform-origin: center` en CSS. Un autre pivot ferait diverger l'écran du rendu.
 */
test('l\'inclinaison tourne autour du centre', () => {
  // Une prise à l'aplomb du centre, tirée d'un quart de tour vers la droite.
  const p = incline(PLACE, { x: 0.5, y: 0.5 }, { largeur: 400, hauteur: 400 });
  assert.equal(Math.round(p.angle), 90);
  assert.equal(p.x, PLACE.x);
  assert.equal(p.y, PLACE.y);
});

/** L'angle reste lisible : 370° et 10° sont le même envoi, et le champ doit le dire. */
test('l\'angle est ramené dans un tour', () => {
  assert.equal(borne({ ...PLACE, angle: 370 }).angle, 10);
  assert.equal(borne({ ...PLACE, angle: -370 }).angle, -10);
});
```

`charge()` du `dom_shim` charge aujourd'hui `app.js` avec son décor. Vérifier sa
signature (`sed -n '1,80p' app/tests/dom_shim.js` et sa fin) et, si elle ne sait
charger qu'`app.js`, lui ajouter la possibilité de charger un fichier nommé et
d'en rendre les fonctions globales — `placement.js` ne posant aucun écouteur, il
n'a besoin d'aucun décor.

- [ ] **Step 2 : lancer les tests pour les voir échouer**

Run (depuis `app/`) : `node --test tests/placement.test.js`
Expected: FAIL — `Cannot find module` ou `deplace is not a function`.

- [ ] **Step 3 : écrire le module**

`app/src/placement.js` :

```js
'use strict';

/**
 * La géométrie du placement d'un envoi : où il est, quelle taille il fait, comment il
 * penche.
 *
 * Aucun DOM ici, et c'est ce qui compte : ce fichier reçoit des nombres et en rend, si
 * bien qu'il se vérifie sans fenêtre. Les écouteurs et le dessin sont dans `envois.js`.
 *
 * **Tout est en fraction de la page**, jamais en pixels : le canevas s'affiche à la
 * taille que la fenêtre lui laisse, et un geste calé sur des pixels irait deux fois
 * plus vite dans une petite fenêtre. C'est la règle de `saisir()` dans `couverture.js`,
 * et c'est aussi la forme que le Rust attend.
 */

/** La plus petite taille qu'on puisse encore attraper à la souris. */
const TAILLE_MIN = 0.05;

/** Ramène un nombre entre deux bornes. */
const entre = (v, min, max) => Math.min(max, Math.max(min, v));

/**
 * Un placement ramené dans ce qu'une page peut porter.
 *
 * L'angle est ramené dans un tour : 370° et 10° sont le même envoi, et le champ qui
 * l'affiche doit le dire.
 */
function borne(p) {
  return {
    ...p,
    x: entre(p.x, 0, 1),
    y: entre(p.y, 0, 1),
    taille: entre(p.taille, TAILLE_MIN, 1),
    angle: p.angle % 360,
  };
}

/** L'objet suit la souris : le déplacement du curseur, rapporté au canevas. */
function deplace(p, { dx, dy }, canevas) {
  return borne({
    ...p,
    x: p.x + dx / canevas.largeur,
    y: p.y + dy / canevas.hauteur,
  });
}

/**
 * La prise de coin règle la taille, sur la **largeur** et non sur la diagonale.
 *
 * C'est la largeur que Typst reçoit — `box(width: …%)` —, et une prise qui suivrait la
 * diagonale ferait diverger l'écran du rendu. Le facteur 2 vient du centre : tirer le
 * coin d'un pixel écarte les deux bords d'un pixel chacun.
 */
function redimensionne(p, { dx }, canevas) {
  return borne({ ...p, taille: p.taille + (2 * dx) / canevas.largeur });
}

/**
 * L'inclinaison, mesurée autour du **centre** de l'objet.
 *
 * Le centre parce que c'est le pivot de `rotate` en Typst comme de
 * `transform-origin: center` en CSS : un autre pivot ferait diverger l'écran du rendu.
 * L'origine des angles est le haut — une prise à l'aplomb du centre vaut 0°, et tirer
 * vers la droite fait tourner dans le sens horaire, comme le dit `Place::angle`.
 *
 * `prise` est la position du curseur en fraction du canevas ; le canevas sert à
 * rétablir les proportions, un canevas de page n'étant pas carré.
 */
function incline(p, prise, canevas) {
  const dx = (prise.x - p.x) * canevas.largeur;
  const dy = (prise.y - p.y) * canevas.hauteur;
  return borne({ ...p, angle: (Math.atan2(dx, -dy) * 180) / Math.PI });
}
```

- [ ] **Step 4 : lancer les tests**

Run (depuis `app/`) : `node --test tests/placement.test.js`
Expected: PASS, six tests.

- [ ] **Step 5 : commiter**

```bash
git add app/src/placement.js app/tests/placement.test.js
git commit -m "La géométrie du placement, sans DOM

Des nombres en entrée, des nombres en sortie : ce module se vérifie sans fenêtre.
Tout en fraction de page et jamais en pixels — la règle de saisir() : un geste
calé sur des pixels irait deux fois plus vite dans une petite fenêtre.

La taille se prend sur la largeur et la rotation autour du centre, parce que ce
sont le box(width:) et le rotate() que Typst recevra."
```

---

## Task 10 : le balisage et la grille

**Files:**
- Modify: `app/src/index.html:312-364`, `app/src/styles.css`

- [ ] **Step 1 : écrire le balisage**

Remplacer la `section#etapeEnvois` par les quatre bandes. Le bloc « Main » et le
bloc « Envois » disparaissent au profit d'une grille.

```html
  <section id="etapeEnvois" class="etape envois" role="tabpanel" hidden>

    <!-- La liste : le nom, et rien d'autre. Le mot, la main et l'image ont rejoint la
         colonne de droite, où ils concernent l'exemplaire ouvert. -->
    <div class="bande dedicataires">
      <h2>Dédicataires</h2>
      <div class="ligne">
        <input type="text" id="inDedicataire" placeholder="à qui ?"
               aria-label="Dédicataire à ajouter">
        <button id="btAjouterEnvoi" type="button">Ajouter</button>
      </div>
      <ul id="envois" role="listbox" aria-label="Dédicataires"></ul>
      <!-- Retirer porte sur l'exemplaire ouvert, et non sur chaque ligne : vingt
           boutons « Retirer » dans une liste sont vingt occasions de se tromper de
           personne, et celui-ci ne peut viser que celle qu'on regarde. -->
      <button id="btRetirerEnvoi" type="button" disabled>Retirer cet envoi</button>

      <!-- Ce qui appartient au livre, et non à l'exemplaire : l'écriture de l'auteur
           entre dans le `.ozalid` pour que le projet compose la même main sur une
           machine où elle n'est installée nulle part. -->
      <h2>Police personnelle</h2>
      <div class="ligne">
        <button id="btPolice" type="button">Ma police…</button>
        <button id="btPoliceRetirer" type="button" disabled>Retirer</button>
      </div>
      <p class="note" id="etatPolice"></p>
    </div>

    <!-- Le rail : toutes les pages du destinataire visé. Cliquer une vignette déplace
         l'envoi sur cette page — c'est le seul moyen d'en changer, et il n'y a donc pas
         de champ « page ». -->
    <div class="bande rail">
      <ol id="vignettes" aria-label="Pages de l'intérieur"></ol>
    </div>

    <!-- Le canevas : la page en fond, l'objet par-dessus, et trois prises. Le fond est
         rendu **sans envoi** — un foreground ne réordonne rien, la page ne dépend donc
         d'aucun dédicataire. -->
    <div class="bande scene">
      <div id="canevas" class="canevas">
        <img id="fondPage" class="fond" alt="" hidden>
        <div id="objet" class="objet" hidden>
          <img id="objetImage" alt="">
          <span id="priseTaille" class="prise taille" role="slider"
                tabindex="0" aria-label="Échelle de l'envoi"></span>
          <span id="priseAngle" class="prise angle" role="slider"
                tabindex="0" aria-label="Inclinaison de l'envoi"></span>
        </div>
      </div>
      <div class="ligne">
        <button id="btVoirPage" type="button">Voir la page</button>
        <span id="etatEnvois" class="etat"></span>
      </div>
      <img id="apercuEnvoi" class="apercu" alt="" hidden>
    </div>

    <!-- Les réglages de l'exemplaire ouvert. Ce que la main ne réclame pas ne paraît
         pas : un champ grisé sous une main en images donnerait à croire qu'on peut y
         écrire. -->
    <div class="bande reglages">
      <h2>Réglages</h2>
      <label><span>Main</span><select id="inMain"></select></label>

      <label id="champMot"><span>Mot</span>
        <textarea id="inMot" rows="4"></textarea></label>

      <div id="champImage" class="ligne" hidden>
        <button id="btImageEnvoi" type="button">Choisir une image…</button>
      </div>

      <div id="champDiffusion" hidden>
        <div class="ligne">
          <button id="btGenerer" type="button">Générer</button>
          <button id="btAccepter" type="button" disabled>Retenir</button>
        </div>
        <label><span>Gabarit du livre</span>
          <textarea id="inGabarit" rows="3"></textarea></label>
        <p class="note">Partagé par tous les envois : c'est le style d'écriture du
          livre. <code>{envoi}</code> y marque l'endroit où le mot de chacun s'insère.</p>
        <label><span>Adresse du modèle</span>
          <input type="url" id="inDiffusionUrl" placeholder="https://…"></label>
        <label><span>Clé</span>
          <input type="password" id="inDiffusionCle" placeholder="inchangée"></label>
        <div class="ligne">
          <button id="btDiffusionRegler" type="button">Enregistrer l'accès</button>
          <button id="btDiffusionOublier" type="button">Oublier la clé</button>
          <span id="etatDiffusion" class="etat"></span>
        </div>
        <p class="note">L'adresse et la clé restent sur cette machine, hors du
          <code>.ozalid</code> — un projet est fait pour être ouvert ailleurs.</p>
      </div>

      <label><span>Échelle</span>
        <input type="range" id="inTaille" min="5" max="100" step="1">
        <span class="val" id="vTaille"></span></label>
      <label><span>Inclinaison</span>
        <input type="range" id="inAngle" min="-45" max="45" step="1">
        <span class="val" id="vAngle"></span></label>

      <div class="ligne">
        <button id="btEnvoyer" type="button" disabled>Générer les envois</button>
      </div>
      <div id="resultatEnvois" class="resultat" hidden></div>
    </div>

  </section>
```

Et, avant `</body>`, la balise de script à côté des autres :

```html
<script src="placement.js"></script>
```

**Ordre** : `placement.js` avant `envois.js`. Les fichiers du front ne posent
aucun écouteur au chargement — ils définissent, `app.js` branche —, mais
`placement.js` doit exister avant qu'`app.js` ne s'exécute.

- [ ] **Step 2 : écrire la grille**

Dans `styles.css`, à la suite des styles d'étape existants :

```css
/* L'étape Envois : quatre bandes, dont aucune ne défile — la liste et les réglages
   sont courts, le rail défile en lui-même, et la page prend ce qui reste. */
.etape.envois {
  display: grid;
  grid-template-columns: 14rem 7rem 1fr 20rem;
  gap: var(--gouttiere);
  align-items: start;
  min-height: 0;
}

.envois .bande { min-width: 0; min-height: 0; }

/* Le rail défile seul : c'est la seule chose de cette étape dont la hauteur soit
   irréductible — un livre a deux cents pages. */
.envois .rail ol {
  overflow-y: auto;
  max-height: 100%;
  margin: 0;
  padding: 0;
  list-style: none;
}

.envois .rail img {
  display: block;
  width: 100%;
  cursor: pointer;
  border: 2px solid transparent;
}

/* La page visée : un liseré, pas un fond — une vignette est une image, et la teinter
   la ferait juger fausse. */
.envois .rail [aria-current="true"] img { border-color: var(--accent); }

/* La scène : la page posée sur le carton, comme la couverture. */
.envois .canevas {
  position: relative;
  display: inline-block;
  max-width: 100%;
  background: var(--carton);
  touch-action: none;
}

.envois .canevas .fond { display: block; width: 100%; }

/* L'objet : le PNG rendu par Typst, à la place et à l'inclinaison réglées. Sa largeur
   est un pourcentage du canevas — c'est le `box(width: …%)` que Typst recevra. */
.envois .objet {
  position: absolute;
  transform: translate(-50%, -50%) rotate(var(--angle));
  transform-origin: center;
  cursor: grab;
}

.envois .objet img { display: block; width: 100%; }
.envois .canevas[data-geste] .objet { cursor: grabbing; }

.envois .prise {
  position: absolute;
  width: 0.75rem;
  height: 0.75rem;
  background: var(--accent);
  border: 1px solid #fff;
  border-radius: 50%;
  cursor: pointer;
}

.envois .prise.taille { right: -0.4rem; bottom: -0.4rem; cursor: nwse-resize; }
.envois .prise.angle { left: 50%; top: -1.6rem; margin-left: -0.4rem; cursor: grab; }

/* La liste : une ligne par personne, celle qu'on règle marquée. */
.envois .dedicataires ul { margin: 0; padding: 0; list-style: none; }
.envois .dedicataires li { cursor: pointer; padding: 0.2rem 0.4rem; }
.envois .dedicataires [aria-selected="true"] { background: var(--surface); font-weight: 600; }
```

Les noms de variables (`--gouttiere`, `--accent`, `--carton`, `--surface`) sont à
prendre **dans `styles.css`** : le fichier a sa table de gris, et en inventer une
la forkerait. `grep -n "^  --" app/src/styles.css` les donne.

- [ ] **Step 3 : vérifier le contrat des identifiants**

`tests/contrats.test.js` lit les vrais fichiers et confronte les identifiants que
le JS demande à ceux que le HTML porte.

Run (depuis `app/`) : `node --test tests/contrats.test.js`
Expected: FAIL sur les identifiants disparus (`inGabarit` déplacé est encore là ;
`envois` a changé de balise). Corriger ce que le test dénonce ; il n'y a rien à
deviner.

- [ ] **Step 4 : commiter**

```bash
git add app/src/index.html app/src/styles.css
git commit -m "L'étape Envois passe en quatre bandes

Liste, rail de vignettes, page en grand, réglages de l'exemplaire ouvert. Aucune
ne défile sauf le rail, dont la hauteur est irréductible — un livre a deux cents
pages.

La liste ne porte plus que le nom : le mot, la main et l'image concernent
l'exemplaire, ils sont passés à droite. Le gabarit y paraît sous le libellé
« Gabarit du livre », parce qu'il est partagé et que le taire ferait croire qu'on
l'écrit pour cette personne-là."
```

---

## Task 11 : `envois.js`, la liste et les réglages

**Files:**
- Modify: `app/src/envois.js`, `app/src/app.js:1184-1230`

- [ ] **Step 1 : réécrire la liste et la sélection**

`envois.js` porte désormais un **rang sélectionné**, qui n'existait pas : la liste
ne montre plus tout à la fois.

```js
/**
 * Le dédicataire dont on règle l'exemplaire.
 *
 * L'étape ne montre plus tout à la fois : la liste dit qui, et les trois autres bandes
 * ne parlent que de celui-là. Le rang plutôt que l'objet — la liste se refait à chaque
 * retour du projet, et un objet retenu serait celui d'avant.
 */
let choisi = 0;

/** L'envoi qu'on règle, ou `null` si la liste est vide. */
function envoi() {
  return projet.envois.liste[choisi] ?? null;
}

/** D'où vient l'écriture de l'exemplaire ouvert. */
function main() {
  return envoi()?.main.mode ?? 'police';
}

/**
 * La liste des dédicataires : un nom par ligne, celui qu'on règle marqué.
 *
 * Le mot, la main et l'image ont quitté la ligne : ils concernent l'exemplaire ouvert,
 * et vingt lignes portant chacune un `textarea` faisaient de la liste un formulaire
 * qu'on ne lisait plus.
 */
function afficherEnvois() {
  const liste = projet.envois.liste;
  // Un retrait peut avoir laissé le rang au-delà de la liste : le ramener plutôt que
  // d'afficher un exemplaire qui n'existe plus.
  choisi = Math.min(choisi, Math.max(0, liste.length - 1));

  const box = $('envois');
  box.textContent = '';
  for (const [i, e] of liste.entries()) {
    const li = h('li', e.dedicataire || `envoi ${i + 1}`);
    li.setAttribute('role', 'option');
    li.setAttribute('aria-selected', String(i === choisi));
    li.addEventListener('click', () => choisir(i));
    box.append(li);
  }
  afficherReglages();
  $('btEnvoyer').disabled = liste.length === 0;
}

/** Ouvre l'exemplaire d'un autre dédicataire : les trois autres bandes le suivent. */
function choisir(i) {
  choisi = i;
  afficherEnvois();
  majObjet();
  majPage();
}
```

- [ ] **Step 2 : réécrire les réglages**

```js
/**
 * Les réglages de l'exemplaire ouvert.
 *
 * Ce que la main ne réclame pas ne paraît pas : un champ grisé sous une main en images
 * donnerait à croire qu'on peut y écrire.
 */
function afficherReglages() {
  const e = envoi();
  const rien = !e;
  for (const id of ['inMain', 'inMot', 'inTaille', 'inAngle', 'btVoirPage']) {
    $(id).disabled = rien;
  }
  if (rien) {
    $('champMot').hidden = false;
    $('champImage').hidden = true;
    $('champDiffusion').hidden = true;
    return;
  }

  afficherMain();
  $('inMot').value = e.contenu;
  $('champMot').hidden = main() !== 'police';
  $('champImage').hidden = main() !== 'image';
  $('champDiffusion').hidden = main() !== 'diffusion';

  $('btImageEnvoi').textContent = e.image ? `Image : ${e.image}` : 'Choisir une image…';
  $('btAccepter').textContent = e.image ? `Retenue : ${e.image}` : 'Retenir';
  // « Retenir » est éteint tant que rien n'a été généré pour cette ligne : c'est le
  // geste qui fige l'image dans le `.ozalid`, et il n'a pas d'objet avant qu'on ait
  // regardé. Un modèle de diffusion rend rarement une écriture lisible du premier coup.
  $('btAccepter').disabled = candidat !== choisi;

  $('inTaille').value = Math.round(e.place.taille * 100);
  $('vTaille').textContent = `${Math.round(e.place.taille * 100)} %`;
  $('inAngle').value = Math.round(e.place.angle);
  $('vAngle').textContent = `${Math.round(e.place.angle)}°`;
}

/**
 * Le choix de la main : les trois écritures de la maison, et celle de l'auteur.
 *
 * Le `select` est refait à chaque projet plutôt que rempli une fois au démarrage : la
 * police personnelle appartient au livre ouvert, elle entre et sort avec lui. Sa valeur
 * est reposée depuis l'envoi — sans quoi le menu montrerait la première main pendant
 * que l'exemplaire en compose une autre, et le premier réglage de l'écran l'imposerait.
 */
function afficherMain() {
  const sel = $('inMain');
  const perso = projet.envois.personnelle;
  const e = envoi();
  sel.replaceChildren();
  // Les écritures et les formes dans une seule liste, préfixées : la question posée est
  // « d'où vient l'écriture », et elle n'a qu'une réponse à la fois. Sans préfixe, une
  // police qui s'appellerait « image » désignerait l'autre forme.
  for (const m of mains) sel.append(new Option(m, `police:${m}`));
  if (perso) sel.append(new Option(`${perso} (votre police)`, `police:${perso}`));
  sel.append(new Option('Image écrite à la main', 'image'));
  sel.append(new Option('Image générée', 'diffusion'));
  sel.value = e.main.mode === 'police' ? `police:${e.main.police}` : e.main.mode;

  $('inGabarit').value = projet.envois.gabarit ?? '';
  $('etatPolice').textContent = perso
    ? `Police personnelle embarquée : ${perso}.`
    : 'Aucune police personnelle : les envois s\'écrivent dans une main de la maison.';
  $('btPoliceRetirer').disabled = !perso;
}

/** Remplace l'envoi ouvert par lui-même modifié. */
function reglerEnvoi(sur) {
  const e = envoi();
  if (!e) return Promise.resolve();
  return tente(async () => afficherProjet(
    await invoke('envoi_regler', { index: choisi, envoi: { ...e, ...sur } })));
}

/** Déplace l'envoi ouvert, sans repasser par le reste de ses réglages. */
function reglerPlace(sur) {
  const e = envoi();
  if (!e) return Promise.resolve();
  return reglerEnvoi({ place: { ...e.place, ...sur } });
}
```

- [ ] **Step 3 : recâbler `app.js`**

Remplacer les écouteurs de `app.js:1184-1230`. `btAjouterEnvoi` :

```js
$('btAjouterEnvoi').addEventListener('click', () => {
  const qui = $('inDedicataire').value.trim();
  if (!qui) return;
  $('inDedicataire').value = '';
  // Le rang du neuf, connu d'avance : la commande le pose en fin de liste, et
  // l'ouvrir aussitôt évite de le chercher dans une liste de vingt.
  return tente(async () => {
    afficherProjet(await invoke('envoi_ajouter', { dedicataire: qui }));
    choisir(projet.envois.liste.length - 1);
  });
});

$('inMot').addEventListener('change', () => reglerEnvoi({ contenu: $('inMot').value }));

// La main appartient à l'exemplaire depuis la v4 : la changer ne touche que lui.
$('inMain').addEventListener('change', () => {
  const choix = $('inMain').value;
  const main = choix.startsWith('police:')
    ? { mode: 'police', police: choix.slice('police:'.length) }
    : { mode: choix };
  return reglerEnvoi({ main });
});

// Le gabarit appartient au livre : il a sa commande, et ne passe pas par un envoi.
$('inGabarit').addEventListener('change', () => tente(async () =>
  afficherProjet(await invoke('envois_gabarit', { gabarit: $('inGabarit').value }))));

$('inTaille').addEventListener('input', () => reglerPlace({ taille: +$('inTaille').value / 100 }));
$('inAngle').addEventListener('input', () => reglerPlace({ angle: +$('inAngle').value }));

$('btImageEnvoi').addEventListener('click', () => choisirImageEnvoi(choisi));
$('btGenerer').addEventListener('click', () => genererEnvoi(choisi));
$('btAccepter').addEventListener('click', () => accepterEnvoi(choisi));
$('btVoirPage').addEventListener('click', () => apercuEnvoi(choisi));
```

Le bouton « Retirer » quitte la liste pour la bande des dédicataires, où il ne
peut viser que l'exemplaire ouvert :

```js
$('btRetirerEnvoi').addEventListener('click', () => tente(async () =>
  afficherProjet(await invoke('envoi_retirer', { index: choisi }))));
```

Il s'éteint quand la liste est vide : ajouter `'btRetirerEnvoi'` à la boucle
d'`afficherReglages` qui grise les contrôles sans exemplaire ouvert.

Corriger aussi `app.js:750-757` (l'oubli au changement de projet) : `choisi`
revient à 0, et les nouveaux nœuds — `vignettes`, `fondPage`, `objet` — se vident
comme `envois` et `apercuEnvoi` le font déjà.

- [ ] **Step 4 : tester la liste et la sélection**

Ajouter à `app/tests/placement.test.js` — ou dans un `envois.test.js` si le
chargement du décor complet l'exige :

```js
/**
 * La liste dit qui, et une seule ligne est ouverte : c'est ce qui distingue cette
 * étape de la liste-formulaire qu'elle remplace. Sans le marquage, on réglerait un
 * exemplaire en croyant en régler un autre.
 */
test('une seule ligne est ouverte à la fois', () => {
  const { $, afficherEnvois, choisir, projet } = atelierEnvois([
    { dedicataire: 'Léa' }, { dedicataire: 'Marc' }, { dedicataire: 'Sonia' },
  ]);
  afficherEnvois();
  const ouvertes = () => $('envois').enfants
    .filter((li) => li.attrs['aria-selected'] === 'true')
    .map((li) => li._texte);
  assert.deepEqual(ouvertes(), ['Léa']);
  choisir(2);
  assert.deepEqual(ouvertes(), ['Sonia']);
  assert.equal(projet.envois.liste.length, 3, 'la liste a changé de taille');
});

/** Un envoi sans nom se désigne quand même : trois lignes vides se confondraient. */
test('un envoi anonyme porte son rang', () => {
  const { $, afficherEnvois } = atelierEnvois([{ dedicataire: '' }]);
  afficherEnvois();
  assert.equal($('envois').enfants[0]._texte, 'envoi 1');
});
```

`atelierEnvois(liste)` monte le décor : le `dom_shim` chargé sur `envois.js`, un
`projet` global portant cette liste, et un `invoke` qui ne fait rien — `choisir`
appelle `majObjet` et `majPage`, qui invoquent le Rust. Le construire sur le
modèle du décor de `composition.test.js`, qui fabrique déjà un projet complet.

Le voir rouge d'abord : retirer la ligne
`li.setAttribute('aria-selected', String(i === choisi));` d'`afficherEnvois`, et
vérifier que le premier test tombe.

**Ce qui ne se teste pas ici** : « un clic qui ne déplace rien ne marque pas le
projet modifié ». Le geste réclame de vrais événements de pointeur, que le
`dom_shim` ne simule pas. La garde est dans le code (`if (dernier !== depart)`,
tâche 12) et se vérifie à la main — poser la souris sur la page sans bouger, puis
fermer le projet : aucune question ne doit être posée.

- [ ] **Step 5 : lancer les tests du front**

Run (depuis `app/`) : `node --test tests/*.test.js`
Expected: PASS. `contrats.test.js` dénonce tout identifiant que le JS demande et
que le HTML ne porte pas.

- [ ] **Step 6 : commiter**

```bash
git add app/src/envois.js app/src/app.js app/src/index.html
git commit -m "L'étape ne règle qu'un exemplaire à la fois

La liste dit qui, les trois autres bandes ne parlent que de celui-là. Vingt lignes
portant chacune leur textarea faisaient de la liste un formulaire qu'on ne lisait
plus.

Le rang plutôt que l'objet : la liste se refait à chaque retour du projet, et un
objet retenu serait celui d'avant."
```

---

## Task 12 : le canevas et les gestes

**Files:**
- Modify: `app/src/envois.js`

- [ ] **Step 1 : poser la page et l'objet**

```js
/**
 * La page de fond du canevas : celle que l'envoi vise, rendue **sans envoi**.
 *
 * Sans envoi parce qu'un `foreground` ne réordonne rien : la page ne dépend d'aucun
 * dédicataire, et la même image sert à tous. C'est aussi ce qui permet de glisser
 * l'objet sans rappeler Typst — le fond ne bouge pas.
 */
async function majPage() {
  const e = envoi();
  const img = $('fondPage');
  if (!e) { img.hidden = true; return; }
  await tente(async () => {
    img.src = await invoke('envoi_page', { page: e.place.page });
    img.alt = `Page ${e.place.page} de l'intérieur`;
    img.hidden = false;
  });
  poserObjet();
}

/**
 * L'objet manipulé : l'envoi rendu par Typst, sur fond transparent.
 *
 * Rendu par Typst et non imité en CSS : ce qu'on déplace **est** ce qui s'imprimera —
 * même police, même corps, même coupure de lignes. Typst n'est rappelé qu'ici, quand le
 * mot ou la main changent ; glisser, redimensionner et incliner ne sont ensuite que des
 * `transform`.
 */
async function majObjet() {
  const e = envoi();
  const bloc = $('objet');
  if (!e) { bloc.hidden = true; return; }
  await tente(async () => {
    const o = await invoke('envoi_objet', { index: choisi });
    $('objetImage').src = o.image;
    $('objetImage').alt = `Envoi pour ${e.dedicataire || 'ce dédicataire'}`;
    bloc.hidden = false;
    poserObjet();
  });
}

/**
 * Pose l'objet à sa place sur le canevas.
 *
 * En pourcentages du canevas et non en pixels : c'est ce que le Rust reçoit, et c'est
 * ce qui fait qu'un canevas plus petit montre le même placement.
 */
function poserObjet() {
  const e = envoi();
  if (!e) return;
  const s = $('objet').style;
  s.setProperty('left', `${e.place.x * 100}%`);
  s.setProperty('top', `${e.place.y * 100}%`);
  s.setProperty('width', `${e.place.taille * 100}%`);
  s.setProperty('--angle', `${e.place.angle}deg`);
}
```

`styles.css` de la tâche 10 lit `left`, `top`, `width` et `--angle` : le
`dom_shim` ne connaît que `setProperty` sur `style`, ce qui suffit.

- [ ] **Step 2 : câbler les trois gestes**

Sur le modèle de `saisir()` (`couverture.js:1133`) : capture du pointeur,
déplacements en fraction, et un clic qui n'a rien déplacé qui ne se commet pas.

```js
/**
 * Un geste sur le canevas : ce qu'il tient, et ce que chaque pixel en fait.
 *
 * Le modèle est `saisir()` dans `couverture.js`, dont c'est l'idiome : le déplacement
 * se mesure en **fraction du canevas** et jamais en pixels — le canevas s'affiche à la
 * taille que la fenêtre lui laisse —, et un clic qui n'a rien déplacé n'est pas commis,
 * pour ne pas marquer le projet modifié pour avoir posé la souris sur sa propre page.
 *
 * Le code n'est pas partagé avec lui, et c'est délibéré : `saisir()` est soudé à
 * `#cadreApercu` et aux chemins de contrôles de la couverture. L'extraire est le bon
 * geste, et c'est un remaniement du code le plus délicat de l'application, sans rapport
 * avec ce que cette étape livre.
 */
function saisirPlacement(el, calcule) {
  el.addEventListener('pointerdown', (ev) => {
    if (ev.button) return;
    const e = envoi();
    if (!e) return;
    const cadre = $('canevas').getBoundingClientRect();
    if (!cadre.width || !cadre.height) return;
    ev.preventDefault();
    ev.stopPropagation();
    el.setPointerCapture(ev.pointerId);

    const depart = e.place;
    const canevas = { largeur: cadre.width, hauteur: cadre.height };
    let dernier = depart;
    $('canevas').setAttribute('data-geste', 'oui');

    const bouger = (m) => {
      dernier = calcule(depart, {
        dx: m.clientX - ev.clientX,
        dy: m.clientY - ev.clientY,
        x: (m.clientX - cadre.left) / cadre.width,
        y: (m.clientY - cadre.top) / cadre.height,
      }, canevas);
      // Le projet n'est pas touché pendant le geste : seul l'écran suit. C'est ce qui
      // rend le glisser instantané, l'objet étant déjà rendu.
      projet.envois.liste[choisi].place = dernier;
      poserObjet();
    };
    const lacher = () => {
      el.removeEventListener('pointermove', bouger);
      el.removeEventListener('pointerup', lacher);
      el.removeEventListener('pointercancel', lacher);
      $('canevas').removeAttribute('data-geste');
      // Un clic qui n'a rien déplacé ne se commet pas : il marquerait le projet
      // modifié, donc réveillerait la garde à la fermeture, pour avoir posé la souris
      // sur sa propre page.
      if (dernier !== depart) reglerPlace(dernier);
    };
    el.addEventListener('pointermove', bouger);
    el.addEventListener('pointerup', lacher);
    el.addEventListener('pointercancel', lacher);
  });
}

/** Câble les trois prises du canevas. Une fois, au démarrage. */
function cablerPlacement() {
  saisirPlacement($('objetImage'), (p, d, c) => deplace(p, d, c));
  saisirPlacement($('priseTaille'), (p, d, c) => redimensionne(p, d, c));
  saisirPlacement($('priseAngle'), (p, d, c) => incline(p, { x: d.x, y: d.y }, c));
}
```

`cablerPlacement()` s'appelle depuis `app.js`, là où `cablerPrises()` l'est déjà
pour la couverture.

- [ ] **Step 3 : vérifier dans l'application**

`cargo tauri dev`, ouvrir un `.ozalid` avec un manuscrit composé, aller aux
Envois, ajouter un dédicataire, et **vérifier de l'œil** :

- l'objet suit la souris au 1:1, sans décalage ni saut au premier pixel ;
- la prise de coin agrandit les lettres avec le bloc ;
- la prise d'angle tourne autour du centre de l'objet ;
- redimensionner la fenêtre ne change pas le placement ;
- « Voir la page » montre l'objet **au même endroit** que le canevas — c'est le
  contrôle qui vaut tous les tests, et le seul qui prouve la correspondance 1:1.

Si le rendu de « Voir la page » diverge du canevas, ne pas ajuster à vue : la
cause est dans la correspondance `place` → Typst de la tâche 3, et la corriger là.

- [ ] **Step 4 : commiter**

```bash
git add app/src/envois.js app/src/app.js
git commit -m "L'envoi se place à la souris

La page en fond, l'objet rendu par Typst par-dessus, et trois prises. Glisser
n'est qu'un transform : le fond ne bouge pas — un foreground ne réordonne rien —
et l'objet est déjà rendu, si bien que Typst n'est rappelé que quand le mot ou la
main changent.

L'idiome est celui de saisir() : fractions et jamais pixels, et un clic qui n'a
rien déplacé ne se commet pas. Le code n'est pas partagé, saisir() étant soudé à
#cadreApercu — l'extraction est notée, pas faite."
```

---

## Task 13 : le rail de vignettes

**Files:**
- Modify: `app/src/envois.js`, `app/src/app.js`

- [ ] **Step 1 : écrire le rail**

```js
/**
 * Les vignettes de toutes les pages de l'intérieur.
 *
 * Rendues en une invocation et gardées en mémoire : 190 pages coûtent six dixièmes de
 * seconde et 1,9 Mo, une fois par composition. Les redemander à chaque changement de
 * dédicataire ferait payer ce prix vingt fois pour des images identiques — la page de
 * fond ne dépend d'aucun envoi.
 */
let pages = null;

async function majVignettes() {
  const ol = $('vignettes');
  if (!pages) {
    $('etatEnvois').className = 'etat';
    $('etatEnvois').textContent = 'rendu des pages…';
    try {
      pages = await invoke('envoi_vignettes');
      $('etatEnvois').textContent = '';
    } catch (e) {
      $('etatEnvois').textContent = String(e);
      $('etatEnvois').className = 'etat erreur';
      return;
    }
  }
  const visee = envoi()?.place.page ?? 0;
  ol.textContent = '';
  for (const [i, src] of pages.entries()) {
    const n = i + 1;
    const li = h('li');
    li.setAttribute('aria-current', String(n === visee));
    const img = h('img');
    img.src = src;
    img.alt = `Page ${n}`;
    // Cliquer une vignette déplace l'envoi sur cette page : c'est le seul moyen d'en
    // changer, et c'est pourquoi il n'y a pas de champ « page ».
    li.addEventListener('click', () => reglerPlace({ page: n }).then(majPage));
    li.append(img);
    ol.append(li);
  }
}
```

- [ ] **Step 2 : oublier les vignettes quand la composition change**

Une composition qui change change les pages. Dans `app.js`, là où le projet est
oublié (`app.js:750`) et là où une composition se termine, poser `pages = null`.

```js
/** Les vignettes de l'intérieur sont périmées : la prochaine ouverture les refera. */
function oublierPages() {
  pages = null;
}
```

Appelée depuis l'oubli de projet **et** depuis le retour de la composition
automatique — la veille décrite au README, qui recompose dès qu'une mesure
disparaît. Chercher où elle rend la main :

```bash
grep -n "composer\|majComposition\|debounce" app/src/app.js | head
```

- [ ] **Step 3 : vérifier dans l'application**

`cargo tauri dev` : le rail se remplit, la page visée porte son liseré, cliquer
une vignette déplace l'envoi et le canevas suit. Puis modifier le manuscrit et
vérifier que le rail se refait — s'il montre encore les pages d'avant, l'oubli de
l'étape 2 n'est pas branché au bon endroit.

- [ ] **Step 4 : commiter**

```bash
git add app/src/envois.js app/src/app.js
git commit -m "Le rail montre toutes les pages, et cliquer y pose l'envoi

Rendues en une invocation et gardées : 190 pages coûtent six dixièmes de seconde,
et la page de fond ne dépend d'aucun envoi — les redemander à chaque changement
de dédicataire ferait payer ce prix vingt fois pour des images identiques.

Une composition qui change les oublie : elles seraient sinon les pages d'un livre
qui n'existe plus."
```

---

# Lot E — la finition

## Task 14 : le README dit ce que l'étape est devenue

**Files:**
- Modify: `app/README.md`

- [ ] **Step 1 : mettre à jour les quatre passages**

1. **« L'écran »** — l'étape Envois n'est plus une liste : quatre bandes, la
   main par exemplaire, le placement à la souris.
2. **« Le fichier .ozalid »** — la version passe à 4, et dire pourquoi : un champ
   se déplace, un binaire v3 donnerait à tous les envois la main par défaut.
3. **« Modules »** — la ligne `envoi` mentionne le placement ; ajouter
   `placement` côté front si le tableau couvre le front (vérifier).
4. **Les mesures** — l'invariant « un envoi ne crée aucune page » vaut désormais
   sur n'importe quelle page, et il est vérifié en composant pour de vrai.

- [ ] **Step 2 : commiter**

```bash
git add app/README.md
git commit -m "Le README dit où l'envoi se place désormais"
```

---

## Task 15 : la passe de vérification complète

- [ ] **Step 1 : la chaîne entière**

```bash
cd app/src-tauri
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo test --test envoi_pagination -- --ignored
cargo run --example temoin
cd .. && node --test tests/*.test.js
```

Le compte de pages du témoin doit être **identique** à celui d'avant ce chantier.
S'il a bougé, l'invariant est cassé quelque part : reprendre la tâche 3.

- [ ] **Step 2 : le tour à la main dans l'application**

Sur un `.ozalid` réel, composé :

- ouvrir un projet **v3** : ses envois gardent leur main, le fichier réenregistré
  est en v4 ;
- deux dédicataires, l'un en police, l'autre en image écrite à la main : chacun
  compose dans la sienne ;
- placer un envoi sur une page du corps, générer les packages, ouvrir le PDF :
  l'envoi est **sur cette page**, et le compte de pages est celui du tirage ;
- changer de destinataire au pied de fenêtre pour un format plus court, et
  vérifier que la génération **refuse** en nommant la personne, la page et le
  compte.

- [ ] **Step 3 : rendre compte**

Dire ce qui a été vérifié et sur quoi — « composé sur *Les Heures creuses*,
kdp-5x8, 190 pages » —, et non « ça marche ». Ce qui n'a pas été exercé se dit
aussi.

---

## Notes pour qui exécute

- **`envoi_vignettes` porte une branche morte volontaire** à la tâche 7, étape 1,
  avec l'instruction de la supprimer. Ne pas la recopier telle quelle.
- **Trois noms sont à vérifier sur le code réel** avant usage : le champ du compte
  de pages sur `Package` (tâche 5), les champs de la mesure enregistrée
  (`compose.gouttiere`, `compose.blanche` — tâche 7), et `crate::image::dimensions`
  (tâche 7). Le plan les nomme d'après le README et le front ; le code fait foi.
- **`dom_shim.charge()`** ne sait peut-être charger qu'`app.js` : la tâche 9 le
  signale et demande de l'étendre. C'est un vrai petit chantier, pas une ligne.
- **L'application est cassée à l'étape Envois entre les tâches 8 et 11.** C'est
  borné et assumé ; ne pas la livrer dans cet état.
- **Clippy jamais dans un pipe** : `| tail` masque l'échec et le commit passe
  quand même.
