# Maquettes en fichiers — lot 1 : le format et les trois fournies

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Les trois maquettes cessent d'être du code Rust et deviennent des archives `.maquette` incorporées au binaire, lues par un module qui rend des `Maquette` — sans que rien ne change à l'écran ni au compte de pages.

**Architecture:** Un format d'archive calqué sur le `.ozalid` (`maquette.toml` + `images/`), lu et écrit par `maquettes.rs` qui réemploie les trois utilitaires zip de `projet.rs`. Les trois archives sont **gravées depuis les constructeurs actuels** par un test transitoire, donc identiques par construction ; un second test transitoire compare l'archive relue au constructeur, et une fois vu passer, les constructeurs partent. Le paramètre `config: Option<&Path>` est posé dès maintenant mais ignoré : le lot 2 le lit.

**Tech Stack:** Rust 2021, `serde`, `toml`, `zip 7` (deflate + stored), `tempfile` en dev.

Spec : `docs/superpowers/specs/2026-08-23-maquettes-en-fichiers-design.md`, § 1, 2, 4, 6, 7 et lot 1 du § 8.

---

## Trois décisions prises en écrivant ce plan

**1. Les trois utilitaires zip de `projet.rs` passent en `pub(crate)` plutôt que d'être
recopiés.** `nom_simple`, `ajoute` et `fichier` sont exactement ce qu'il faut à une
`.maquette`, et `nom_simple` porte un contrôle de sécurité — une entrée qui remonte hors
de son répertoire — qu'on ne veut surtout pas voir diverger en deux exemplaires. Trois
changements de visibilité, aucun déplacement de code : le module `projet` reste leur
propriétaire, et son doc-comment le dit.

**2. L'écriture d'une archive fige la date des entrées à 1980-01-01.** Sans quoi le test
de gravure réécrirait trois fichiers binaires différents à chaque `cargo test`, et
`git status` ne serait plus jamais propre. La configuration actuelle de `zip`
(`default-features = false`) donne probablement déjà cette date faute de la feature
`time`, mais la reproductibilité d'un fichier versionné doit être un fait écrit, pas la
conséquence d'une feature transitive.

**3. `maquette_choisir` ne pose pas encore les images.** La table du § 5 de la spec lui
demande de le faire, mais aucune des trois fournies n'en porte : le comportement ne
serait couvert par aucun test tant que les personnalisées n'existent pas. Il vient au
lot 2, avec le premier cas qui l'exerce.

**Un écart assumé vis-à-vis de la spec, à signaler si la revue le relève** : la signature
`toutes(config: Option<&Path>)` est posée au lot 1 comme la spec l'écrit, mais le
paramètre y est mort (`_config`). C'est ce qui évite de réécrire au lot 2 la trentaine
d'appels de test que ce lot migre déjà.

---

## Tâche 1 : une archive `.maquette` fait l'aller-retour

**Files:**
- Modify: `app/src-tauri/src/projet.rs` (visibilité de `nom_simple`, `ajoute`, `fichier`)
- Modify: `app/src-tauri/src/maquettes.rs` (le type `Maquette`, `lire`, `ecrire`)

- [ ] **Step 1: Écrire le test qui échoue**

En tête de `app/src-tauri/src/maquettes.rs`, remplacer le bloc `use` actuel par :

```rust
use std::collections::BTreeMap;
use std::io::{Read, Seek, Write};

use serde::{Deserialize, Serialize};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::couverture::*;
use crate::image::Cadrage;
```

Puis, dans le module `tests` de ce fichier, ajouter en tête du module :

```rust
    use std::io::Cursor;

    /// Une maquette est une archive, pas un TOML : elle porte des images, et ce qu'elle
    /// emporte doit revenir tel quel — la couverture entière et chaque octet des images.
    /// C'est la promesse du format, et la seule chose qui rende une personnalisée
    /// fidèle au livre depuis lequel on l'a enregistrée.
    #[test]
    fn une_maquette_fait_l_aller_retour_avec_ses_images() {
        let mut images = BTreeMap::new();
        images.insert("couverture.jpg".to_string(), vec![0xff, 0xd8, 0xff, 0xe0]);
        images.insert("quatrieme.png".to_string(), vec![0x89, b'P', b'N', b'G']);
        let avant = Maquette {
            cle: "ma-collection".into(),
            nom: "Ma collection".into(),
            fournie: false,
            couverture: blanche(),
            images,
        };

        let mut octets = Vec::new();
        ecrire(Cursor::new(&mut octets), &avant).unwrap();
        let apres = lire(Cursor::new(&octets), "ma-collection", false).unwrap();

        assert_eq!(apres, avant);
    }

    /// Le nom affiché vit dans l'archive ; la clé, elle, vient de qui la lit — le nom du
    /// fichier pour une personnalisée, la table des embarquées pour une fournie. Une
    /// archive déplacée sous un autre nom de fichier change donc de clé, pas de nom.
    #[test]
    fn la_cle_vient_du_lecteur_et_le_nom_de_l_archive() {
        let m = Maquette {
            cle: "peu-importe".into(),
            nom: "Ma collection".into(),
            fournie: false,
            couverture: folio(),
            images: BTreeMap::new(),
        };
        let mut octets = Vec::new();
        ecrire(Cursor::new(&mut octets), &m).unwrap();

        let relue = lire(Cursor::new(&octets), "autre-slug", true).unwrap();
        assert_eq!(relue.cle, "autre-slug");
        assert_eq!(relue.nom, "Ma collection");
        assert!(relue.fournie);
    }
```

- [ ] **Step 2: Lancer le test et le voir échouer**

```bash
cd app/src-tauri && cargo test --lib maquettes 2>&1 | tail -20
```

Attendu : la compilation échoue — `cannot find type Maquette`, `cannot find function ecrire`, `cannot find function lire`.

- [ ] **Step 3: Ouvrir les trois utilitaires zip de `projet.rs`**

Dans `app/src-tauri/src/projet.rs`, changer la visibilité de trois fonctions privées et
compléter leurs doc-comments (le reste du corps est inchangé) :

```rust
/// Ce qui suit `images/`, `polices/` ou `envois/` est-il un simple nom de fichier ?
///
/// L'application n'en écrit jamais d'autres : ces trois répertoires sont plats, et
/// leurs noms sont fabriqués — `couverture.jpg`, la police copiée, l'envoi assaini par
/// `envoi::nom_image`. Mais l'archive est un document qu'on s'échange, et rien n'oblige
/// celle qu'on ouvre à venir d'ici : `package::ecrire_images` et `ecrire_polices` en
/// font des chemins par `join`, qui suit ce qui remonte jusqu'à écrire ailleurs.
///
/// La contre-oblique est refusée avec la barre : elle sépare sous Windows, et une
/// archive écrite là-bas y arriverait par le même chemin.
///
/// `maquettes` s'en sert pour les mêmes raisons : une `.maquette` est une archive du
/// même genre, qu'on s'échange aussi. Le contrôle ne doit exister qu'une fois — deux
/// exemplaires divergeraient, et c'est le plus vieux qui laisserait passer.
pub(crate) fn nom_simple(court: &str) -> bool {
    court != "." && court != ".." && !court.contains(['/', '\\'])
}

/// Ajoute une entrée à une archive. Partagé avec `maquettes`, qui écrit le même genre
/// d'archive.
pub(crate) fn ajoute<W: Write + Seek>(
```

et

```rust
/// Lit une entrée d'archive, ou rend `None` si elle n'y est pas. Partagé avec
/// `maquettes`.
pub(crate) fn fichier<R: Read + Seek>(
```

- [ ] **Step 4: Écrire le format et ses deux fonctions**

Dans `app/src-tauri/src/maquettes.rs`, remplacer le doc-comment de tête du module par :

```rust
//! Les maquettes de couverture : des **archives**, non du code.
//!
//! Une maquette ne porte que la mise en page : le titre, l'auteur, le genre, l'éditeur
//! et la collection viennent du livre. Charger une maquette ne change donc jamais ce
//! qui sera imprimé comme identité — seulement la façon dont ça paraît.
//!
//! ```text
//! maquette.toml   le nom affiché, et la couverture entière
//! images/         couverture.ext et quatrieme.ext, quand la maquette en porte
//! ```
//!
//! Trois maquettes sont **fournies** : leurs archives sont incorporées au binaire par
//! `include_bytes!`. Il n'y a donc aucun chemin à résoudre sur le poste, aucun mode
//! dégradé, aucun écart entre développement et livraison — et leur immuabilité est un
//! fait, pas une règle applicative. C'est précisément le piège connu de `fonts/`, où
//! `target/debug` ne suit pas les sources.
//!
//! **Pas de champ `version`** : comme le `.ozalid`, tout futur champ arrive avec son
//! `#[serde(default = …)]`, et une archive écrite par une version antérieure se relit.
```

Puis, après le bloc `use` et avant `fn style(…)`, poser le format :

```rust
const MAQUETTE_TOML: &str = "maquette.toml";
const IMAGES: &str = "images/";

/// Ce que porte `maquette.toml`. Le scalaire précède la table : en TOML, une valeur
/// écrite après une table lui appartiendrait.
#[derive(Serialize, Deserialize)]
struct Fichier {
    nom: String,
    couverture: Couverture,
}

/// Une maquette, fournie ou personnalisée.
///
/// La `cle` ne vit pas dans l'archive : elle vient de qui la lit — la table des
/// embarquées pour une fournie, le nom du fichier pour une personnalisée. Le **nom**,
/// lui, est l'identité, et c'est lui que l'archive porte.
#[derive(Debug, Clone, PartialEq)]
pub struct Maquette {
    pub cle: String,
    pub nom: String,
    /// Ni renommable, ni effaçable. Le refus est tenu par le Rust, pas par l'interface.
    pub fournie: bool,
    pub couverture: Couverture,
    /// Nom de fichier (sans `images/`) → contenu, comme dans `Projet`.
    pub images: BTreeMap<String, Vec<u8>>,
}

fn lire<R: Read + Seek>(source: R, cle: &str, fournie: bool) -> Result<Maquette, String> {
    let mut zip = ZipArchive::new(source).map_err(|e| format!("archive illisible : {e}"))?;
    let brut = crate::projet::fichier(&mut zip, MAQUETTE_TOML)?
        .ok_or_else(|| format!("archive sans {MAQUETTE_TOML} : ce n'est pas une maquette."))?;
    let brut =
        String::from_utf8(brut).map_err(|_| format!("{MAQUETTE_TOML} n'est pas de l'UTF-8."))?;
    let f: Fichier = toml::from_str(&brut).map_err(|e| format!("{MAQUETTE_TOML} : {e}"))?;

    let mut images = BTreeMap::new();
    let noms: Vec<String> = zip.file_names().map(str::to_owned).collect();
    for nom in noms {
        let Some(court) = nom.strip_prefix(IMAGES) else {
            continue;
        };
        // L'entrée du répertoire lui-même, que tout archiveur écrit.
        if court.is_empty() {
            continue;
        }
        if let Some(oct) = crate::projet::fichier(&mut zip, &nom)? {
            images.insert(court.to_string(), oct);
        }
    }

    Ok(Maquette {
        cle: cle.into(),
        nom: f.nom,
        fournie,
        couverture: f.couverture,
        images,
    })
}

fn ecrire<W: Write + Seek>(sortie: W, m: &Maquette) -> Result<(), String> {
    let mut zip = ZipWriter::new(sortie);
    // La date des entrées est figée : une archive versionnée doit être la même à
    // l'octet près d'une écriture à l'autre, sinon le test qui grave les fournies
    // salirait le dépôt à chaque `cargo test`.
    let fige = |m: CompressionMethod| {
        SimpleFileOptions::default()
            .compression_method(m)
            .last_modified_time(zip::DateTime::default())
    };
    let texte_opts = fige(CompressionMethod::Deflated);
    // Les images sont déjà compressées (PNG, JPEG) : les dégonfler coûte du temps
    // pour un gain nul, parfois négatif.
    let brut_opts = fige(CompressionMethod::Stored);

    let f = Fichier {
        nom: m.nom.clone(),
        couverture: m.couverture.clone(),
    };
    let toml_brut =
        toml::to_string_pretty(&f).map_err(|e| format!("sérialisation de {MAQUETTE_TOML} : {e}"))?;
    crate::projet::ajoute(&mut zip, MAQUETTE_TOML, toml_brut.as_bytes(), texte_opts)?;
    for (nom, oct) in &m.images {
        crate::projet::ajoute(&mut zip, &format!("{IMAGES}{nom}"), oct, brut_opts)?;
    }
    zip.finish().map_err(|e| format!("clôture : {e}"))?;
    Ok(())
}
```

- [ ] **Step 5: Lancer les tests et les voir passer**

```bash
cd app/src-tauri && cargo test --lib maquettes 2>&1 | tail -20
```

Attendu : `test result: ok.` — les cinq tests du module (les trois de propriété
existants et les deux neufs).

- [ ] **Step 6: Vérifier et commiter**

```bash
cd app/src-tauri && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test 2>&1 | tail -5
cd /Users/jean-paulgavini/Documents/Dev/ozalid
git add app/src-tauri/src/maquettes.rs app/src-tauri/src/projet.rs
git commit -m "$(cat <<'EOF'
Une maquette sait s'écrire et se relire

Le format : maquette.toml et images/, une archive du même genre que le .ozalid,
dont elle réemploie les trois utilitaires zip plutôt que d'en recopier un
quatrième. La date des entrées est figée pour que le fichier soit le même à
l'octet près d'une écriture à l'autre.

Claude-Session: https://claude.ai/code/session_01AyXS78gurZmV9m6yFQ8LwL
EOF
)"
```

---

## Tâche 2 : une image de travers est refusée

**Files:**
- Modify: `app/src-tauri/src/maquettes.rs`

- [ ] **Step 1: Écrire le test qui échoue**

Dans le module `tests` de `app/src-tauri/src/maquettes.rs` :

```rust
    /// Une `.maquette` est un document qu'on s'échange, et rien n'oblige celle qu'on
    /// ouvre à venir d'ici. `package::ecrire_images` fait des chemins de ces noms par
    /// `join` : une entrée qui remonte écrirait ailleurs sur le disque. Le refus est le
    /// même que celui du `.ozalid`, et sur le même contrôle — il n'en existe qu'un.
    #[test]
    fn une_image_qui_remonte_hors_de_son_repertoire_est_refusee() {
        let mut octets = Vec::new();
        {
            let mut zip = ZipWriter::new(Cursor::new(&mut octets));
            let opts = SimpleFileOptions::default();
            let f = Fichier {
                nom: "Piégée".into(),
                couverture: folio(),
            };
            crate::projet::ajoute(
                &mut zip,
                MAQUETTE_TOML,
                toml::to_string_pretty(&f).unwrap().as_bytes(),
                opts,
            )
            .unwrap();
            crate::projet::ajoute(&mut zip, "images/../../ailleurs.png", b"x", opts).unwrap();
            zip.finish().unwrap();
        }

        let e = lire(Cursor::new(&octets), "piegee", false).unwrap_err();
        assert!(e.contains("ailleurs.png"), "{e}");
    }
```

- [ ] **Step 2: Lancer le test et le voir échouer**

```bash
cd app/src-tauri && cargo test --lib maquettes::tests::une_image_qui_remonte 2>&1 | tail -20
```

Attendu : `FAILED` — `called Result::unwrap_err() on an Ok value`. L'archive se lit
sans broncher et pose une image nommée `../../ailleurs.png`.

- [ ] **Step 3: Poser le contrôle**

Dans `lire`, entre le `if court.is_empty()` et la lecture de l'entrée :

```rust
        if !crate::projet::nom_simple(court) {
            return Err(format!(
                "archive refusée : « {nom} » n'est pas un simple nom de fichier."
            ));
        }
```

- [ ] **Step 4: Lancer le test et le voir passer**

```bash
cd app/src-tauri && cargo test --lib maquettes 2>&1 | tail -10
```

Attendu : `test result: ok.`

- [ ] **Step 5: Vérifier et commiter**

```bash
cd app/src-tauri && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test 2>&1 | tail -5
cd /Users/jean-paulgavini/Documents/Dev/ozalid
git add app/src-tauri/src/maquettes.rs
git commit -m "$(cat <<'EOF'
Une maquette venue d'ailleurs n'écrit pas ailleurs

Le même refus que le .ozalid, sur le même contrôle : une entrée d'images/ qui
remonte hors de son répertoire deviendrait un chemin par join au moment du
package.

Claude-Session: https://claude.ai/code/session_01AyXS78gurZmV9m6yFQ8LwL
EOF
)"
```

---

## Tâche 3 : graver les trois archives fournies

Transitoire : ce test disparaît à la tâche 6, avec les constructeurs qui le nourrissent.

**Files:**
- Modify: `app/src-tauri/src/maquettes.rs`
- Create: `app/src-tauri/maquettes/folio.maquette`
- Create: `app/src-tauri/maquettes/blanche.maquette`
- Create: `app/src-tauri/maquettes/surimpression.maquette`

- [ ] **Step 1: Écrire le test de gravure**

Ajouter `use std::path::Path;` au bloc `use` de tête — c'est ici qu'il commence à servir,
et pas avant : posé à la tâche 1, il aurait fait échouer `clippy -D warnings` sur un
import inutilisé.

Puis, dans le module `tests` de `app/src-tauri/src/maquettes.rs` :

```rust
    /// **Transitoire** — part à la tâche 6, avec les constructeurs.
    ///
    /// Les trois archives fournies ne s'écrivent pas à la main : elles se gravent
    /// depuis les constructeurs, ce qui les leur rend identiques par construction. Ce
    /// test écrit dans les sources, ce qu'un test ne fait jamais autrement — c'est le
    /// prix d'une bascule qu'on veut invisible, et il ne dure que le temps du lot.
    #[test]
    fn grave_les_archives_fournies() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("maquettes");
        std::fs::create_dir_all(&dir).unwrap();
        for (cle, nom, couverture) in [
            ("folio", "Folio", folio()),
            ("blanche", "Blanche", blanche()),
            ("surimpression", "Surimpression", surimpression()),
        ] {
            let m = Maquette {
                cle: cle.into(),
                nom: nom.into(),
                fournie: true,
                couverture,
                images: BTreeMap::new(),
            };
            let f = std::fs::File::create(dir.join(format!("{cle}.maquette"))).unwrap();
            ecrire(f, &m).unwrap();
        }
    }
```

- [ ] **Step 2: Lancer le test et regarder ce qu'il a écrit**

```bash
cd app/src-tauri && cargo test --lib maquettes::tests::grave 2>&1 | tail -5
ls -l maquettes/
unzip -p maquettes/blanche.maquette maquette.toml | head -12
```

Attendu : trois fichiers de quelques kilo-octets, et un TOML qui commence par
`nom = "Blanche"` suivi de `[couverture]`.

- [ ] **Step 3: Vérifier que la gravure est reproductible**

```bash
cd app/src-tauri && shasum maquettes/*.maquette > /tmp/avant.sha && cargo test --lib maquettes::tests::grave >/dev/null 2>&1 && shasum -c /tmp/avant.sha
```

Attendu : trois `OK`. Si un fichier diffère, la date des entrées n'est pas figée —
reprendre `ecrire` (tâche 1, étape 4) avant d'aller plus loin : sans cela, chaque
`cargo test` salira le dépôt.

- [ ] **Step 4: Commiter les trois archives**

```bash
cd /Users/jean-paulgavini/Documents/Dev/ozalid
git add app/src-tauri/maquettes app/src-tauri/src/maquettes.rs
git status --short
git commit -m "$(cat <<'EOF'
Les trois maquettes fournies deviennent des fichiers

Gravées depuis les constructeurs par un test transitoire, ce qui les leur rend
identiques par construction plutôt que recopiées à la main.

Claude-Session: https://claude.ai/code/session_01AyXS78gurZmV9m6yFQ8LwL
EOF
)"
```

`git status --short` doit montrer exactement quatre fichiers ajoutés : les trois
archives et le module. Si `maquettes/` n'apparaît pas, vérifier qu'aucune règle du
`.gitignore` ne l'écarte.

---

## Tâche 4 : `toutes` et `par_cle` servent les archives

**Files:**
- Modify: `app/src-tauri/src/maquettes.rs`
- Modify: `app/src-tauri/src/commands.rs:651`
- Modify: `app/src-tauri/src/couverture.rs:1596,1630`
- Modify: `app/src-tauri/src/planche.rs:1183`
- Modify: `app/src-tauri/src/ebook.rs:254`
- Modify: `app/src-tauri/src/projet.rs:924`

- [ ] **Step 1: Poser les stubs et les appelants**

Remplacer les deux fonctions `toutes` et `par_cle` de `maquettes.rs` par :

```rust
/// Les trois fournies, incorporées au binaire : rien à résoudre sur le poste, donc
/// aucun écart entre développement et livraison, et l'immuabilité est un fait.
const FOURNIES: [(&str, &[u8]); 3] = [
    ("folio", include_bytes!("../maquettes/folio.maquette")),
    ("blanche", include_bytes!("../maquettes/blanche.maquette")),
    (
        "surimpression",
        include_bytes!("../maquettes/surimpression.maquette"),
    ),
];

/// Les maquettes, dans l'ordre où l'interface les propose.
///
/// **La lecture est au mieux** : une archive illisible est ignorée avec un mot sur la
/// sortie d'erreur — ce qui se perd est un point de départ, et refuser la liste entière
/// coûterait les autres. L'écriture, elle, échoue fort : elle perdrait du travail.
///
/// `config` porte le répertoire de configuration, ou `None` quand il est inatteignable.
/// Il est ignoré tant que les personnalisées n'existent pas (lot 2) : seules les
/// fournies sont servies.
pub fn toutes(_config: Option<&Path>) -> Vec<Maquette> {
    Vec::new()
}

pub fn par_cle(config: Option<&Path>, cle: &str) -> Option<Maquette> {
    toutes(config).into_iter().find(|m| m.cle == cle)
}
```

Migrer du même coup les cinq appelants de l'ancienne `par_cle`, sans quoi rien ne
compile :

- `commands.rs:651` :

```rust
    let m = maquettes::par_cle(None, &cle).ok_or_else(|| format!("maquette inconnue : {cle}"))?;
    let mut garde = atelier.ouvert.lock().unwrap();
    let o = garde.as_mut().ok_or_else(aucun_projet)?;
    o.projet.meta.couverture.maquette = Some(m.couverture);
```

- `commands.rs`, dans `maquettes_liste` :

```rust
#[tauri::command]
pub fn maquettes_liste() -> Vec<MaquetteVue> {
    maquettes::toutes(None)
        .into_iter()
        .map(|m| MaquetteVue {
            cle: m.cle,
            libelle: m.nom,
        })
        .collect()
}
```

- `couverture.rs:1596` et `couverture.rs:1630`, `planche.rs:1183` : remplacer
  `maquettes::par_cle("blanche").unwrap()` par `maquettes::fournie("blanche")` (et de
  même pour `"folio"`) — l'aide de test arrive à l'étape 3 de cette tâche.
- `ebook.rs:254` : `p.meta.couverture.maquette = Some(crate::maquettes::fournie("folio"));`
- `projet.rs:924` : idem.

- [ ] **Step 2: Écrire les deux tests qui échouent**

Dans le module `tests` de `maquettes.rs` :

```rust
    /// La parade du § 6 de la spec : les trois fournies ne sont plus du code, un TOML
    /// mal formé ne casserait donc plus la compilation mais le **démarrage**. Ce test
    /// les parse toutes les trois, et `cargo test` est exigé avant commit.
    #[test]
    fn les_trois_fournies_se_lisent_et_portent_leur_nom() {
        let vues: Vec<(String, String, bool)> = toutes(None)
            .into_iter()
            .map(|m| (m.cle, m.nom, m.fournie))
            .collect();
        assert_eq!(
            vues,
            [
                ("folio".to_string(), "Folio".to_string(), true),
                ("blanche".to_string(), "Blanche".to_string(), true),
                (
                    "surimpression".to_string(),
                    "Surimpression".to_string(),
                    true
                ),
            ]
        );
    }

    /// **Transitoire** — part à la tâche 6.
    ///
    /// La bascule doit être invisible : ce que l'archive rend doit être exactement ce
    /// que le constructeur rendait, champ pour champ. C'est ce test-là qui autorise à
    /// retirer les constructeurs.
    #[test]
    fn les_archives_fournies_valent_les_constructeurs() {
        for (cle, attendue) in [
            ("folio", folio()),
            ("blanche", blanche()),
            ("surimpression", surimpression()),
        ] {
            assert_eq!(par_cle(None, cle).unwrap().couverture, attendue, "{cle}");
        }
    }
```

- [ ] **Step 3: Ajouter l'aide de test et le voir échouer**

Toujours dans `maquettes.rs`, mais **hors** du module `tests` (les autres modules s'en
servent), juste après `par_cle` :

```rust
/// La couverture d'une fournie, pour les tests des autres modules.
///
/// Une trentaine de tests partaient d'un constructeur ; ils partent maintenant d'une
/// archive, et cette aide leur évite de répéter le dépliage à chaque ligne. `#[cfg(test)]`
/// : elle n'existe pas dans le binaire livré.
#[cfg(test)]
pub(crate) fn fournie(cle: &str) -> Couverture {
    par_cle(None, cle)
        .unwrap_or_else(|| panic!("maquette fournie inconnue : {cle}"))
        .couverture
}
```

```bash
cd app/src-tauri && cargo test --lib maquettes 2>&1 | tail -20
```

Attendu : deux échecs — `les_trois_fournies_se_lisent_et_portent_leur_nom` sur une liste
vide, et `les_archives_fournies_valent_les_constructeurs` sur `unwrap()` d'un `None`
(plus tous les tests des autres modules qui passent par `fournie`).

- [ ] **Step 4: Implanter `toutes`**

```rust
pub fn toutes(_config: Option<&Path>) -> Vec<Maquette> {
    FOURNIES
        .iter()
        .filter_map(|(cle, octets)| {
            lire(std::io::Cursor::new(*octets), cle, true)
                .map_err(|e| eprintln!("maquette fournie « {cle} » illisible : {e}"))
                .ok()
        })
        .collect()
}
```

- [ ] **Step 5: Lancer toute la suite et la voir passer**

```bash
cd app/src-tauri && cargo test 2>&1 | tail -10
```

Attendu : `test result: ok.` sur les 337 tests d'avant plus les neufs.

- [ ] **Step 6: Vérifier et commiter**

```bash
cd app/src-tauri && cargo fmt --check && cargo clippy --all-targets -- -D warnings
cd /Users/jean-paulgavini/Documents/Dev/ozalid
git add app/src-tauri/src
git commit -m "$(cat <<'EOF'
Les maquettes se lisent dans le binaire, plus dans le code

toutes() et par_cle() servent les trois archives embarquées. La lecture est au
mieux : une archive illisible s'écarte avec un mot, elle ne coûte pas la liste.

Claude-Session: https://claude.ai/code/session_01AyXS78gurZmV9m6yFQ8LwL
EOF
)"
```

---

## Tâche 5 : les appelants quittent les constructeurs

**Files:**
- Modify: `app/src-tauri/src/couverture.rs`, `planche.rs`, `projet.rs`, `ebook.rs` (tests)
- Modify: `app/src-tauri/src/import.rs:313-320` (production)
- Modify: `app/src-tauri/examples/temoin.rs:60`
- Modify: `app/src-tauri/examples/maquette.rs:51-56`

- [ ] **Step 1: Migrer les tests, mécaniquement**

```bash
cd app/src-tauri/src
sed -i '' \
  -e 's/maquettes::folio()/maquettes::fournie("folio")/g' \
  -e 's/maquettes::blanche()/maquettes::fournie("blanche")/g' \
  -e 's/maquettes::surimpression()/maquettes::fournie("surimpression")/g' \
  couverture.rs planche.rs projet.rs ebook.rs
grep -rn 'maquettes::\(folio\|blanche\|surimpression\)()' . ../examples
```

Attendu du `grep` : plus rien dans `src/`, et deux lignes restantes dans `import.rs`
(traitées à l'étape 2) plus les exemples (étape 3). Si `import.rs` apparaît dans le
`sed`, l'annuler : ses appels sont du code de production, pas des tests, et `fournie`
n'existe pas hors test.

- [ ] **Step 2: Migrer `import.rs`, qui est du code de production**

Dans `app/src-tauri/src/import.rs`, remplacer le `match` de `traduit` :

```rust
    let fournie = |cle: &str| -> Result<Couverture, String> {
        maquettes::par_cle(None, cle)
            .map(|m| m.couverture)
            .ok_or_else(|| format!("maquette fournie « {cle} » illisible."))
    };
    let mut cv = match r.mode.as_str() {
        "band" => fournie("folio")?,
        "overlay" => fournie("surimpression")?,
        "typo" => fournie("blanche")?,
        // Bloc sans mode : le bandeau est le défaut de l'atelier.
        "" => fournie("folio")?,
        autre => return Err(format!("mode de couverture inconnu : « {autre} ».")),
    };
```

L'erreur remonte plutôt que de paniquer : `traduit` rend déjà un `Result`, et une
archive fournie illisible est exactement le mode de panne que le § 6 de la spec redoute.

- [ ] **Step 3: Migrer les deux exemples**

`app/src-tauri/examples/temoin.rs:60` :

```rust
    projet.meta.couverture.maquette = Some(
        maquettes::par_cle(None, "blanche")
            .expect("maquette fournie « blanche »")
            .couverture,
    );
```

`app/src-tauri/examples/maquette.rs`, autour de la ligne 51 — l'exemple itérait sur des
triplets, il itère maintenant sur des `Maquette` :

```rust
    // La maquette du projet d'abord, quand il en porte une : c'est elle qu'on compare
    // au livre déjà publié.
    let mut a_rendre = maquettes::toutes(None);
    if let Some(cv) = projet.meta.couverture.maquette.clone() {
        a_rendre.insert(
            0,
            maquettes::Maquette {
                cle: "projet".into(),
                nom: "Maquette du projet".into(),
                fournie: false,
                couverture: cv,
                images: Default::default(),
            },
        );
    }

    for m in a_rendre {
        let (k, libelle, cv) = (m.cle.as_str(), m.nom.as_str(), &m.couverture);
```

Le corps de la boucle est inchangé : il lisait déjà `k`, `libelle` et `cv`. Vérifier
qu'aucun `&cv` n'y devient `&&Couverture` — le compilateur le dira.

- [ ] **Step 4: Compiler et lancer toute la suite**

```bash
cd app/src-tauri && cargo test 2>&1 | tail -10 && cargo clippy --all-targets -- -D warnings
```

Attendu : `test result: ok.` et clippy muet. Le test transitoire
`les_archives_fournies_valent_les_constructeurs` tient toujours : les constructeurs
existent encore, plus personne d'autre ne les appelle.

- [ ] **Step 5: Vérifier et commiter**

```bash
cd app/src-tauri && cargo fmt --check
cd /Users/jean-paulgavini/Documents/Dev/ozalid
git add app/src-tauri/src app/src-tauri/examples
git commit -m "$(cat <<'EOF'
Plus personne ne construit une maquette en Rust

Les tests, l'import de l'atelier et les deux exemples passent par les archives.
L'import remonte une erreur plutôt que de paniquer : une fournie illisible est
le mode de panne que la bascule introduit.

Claude-Session: https://claude.ai/code/session_01AyXS78gurZmV9m6yFQ8LwL
EOF
)"
```

---

## Tâche 6 : les constructeurs et les tests transitoires partent

**Files:**
- Modify: `app/src-tauri/src/maquettes.rs`

- [ ] **Step 1: Retirer ce qui a servi de moule**

Dans `app/src-tauri/src/maquettes.rs`, supprimer :

- les fonctions `style`, `quatrieme_commune`, `dos`, `pastille_eteinte`, `folio`,
  `blanche` et `surimpression` ;
- les `use crate::couverture::*;` et `use crate::image::Cadrage;` s'ils ne servent plus
  qu'à elles — garder `use crate::couverture::Couverture;` ;
- les deux tests transitoires `grave_les_archives_fournies` et
  `les_archives_fournies_valent_les_constructeurs`.

Dans les deux tests de la tâche 1 (`une_maquette_fait_l_aller_retour_avec_ses_images`,
`la_cle_vient_du_lecteur_et_le_nom_de_l_archive`) et celui de la tâche 2, remplacer
`blanche()` et `folio()` par `fournie("blanche")` et `fournie("folio")`.

- [ ] **Step 2: Faire vivre ce que les constructeurs disaient**

Le commentaire le plus précieux du fichier part avec `blanche()` : le pied éditeur y est
posé à 13,5 % et non aux 11 % du CSS d'origine, sans quoi il traverse le filet interne du
cadre. Le test qui borne la valeur existe déjà — lui rattacher l'explication, en tête de
`le_pied_editeur_ne_traverse_jamais_le_cadre` :

```rust
    /// Le pied éditeur est posé depuis le bas, en % de la hauteur ; le filet interne du
    /// cadre l'est depuis le bas aussi, mais son décroché se lit sur la **largeur**. Les
    /// deux ne varient donc pas ensemble d'un format à l'autre, et un pied qui dégage le
    /// filet en poche peut le traverser en A4.
    ///
    /// La maquette Blanche porte 13,5 % et non les 11 % du CSS de l'atelier : c'est le
    /// seul écart assumé vis-à-vis d'`index.html`, qui a le défaut et ne l'a pas vu.
    /// L'archive porte la valeur, ce test la borne sur tous les formats de la table —
    /// c'est ici, et nulle part ailleurs, que la raison de ce 13,5 est écrite.
    #[test]
    fn le_pied_editeur_ne_traverse_jamais_le_cadre() {
        let cv = fournie("blanche");
```

- [ ] **Step 3: Adapter les trois tests de propriété aux archives**

```rust
    /// Chaque maquette doit être un archétype distinct : trois entrées qui rendraient
    /// la même chose ne serviraient à rien comme point de départ.
    #[test]
    fn les_trois_maquettes_sont_de_modes_distincts() {
        let modes: Vec<Mode> = toutes(None).into_iter().map(|m| m.couverture.mode).collect();
        assert_eq!(modes.len(), 3);
        for (i, m) in modes.iter().enumerate() {
            assert!(!modes[..i].contains(m), "mode {m:?} en double");
        }
    }

    #[test]
    fn une_cle_inconnue_ne_rend_pas_de_maquette() {
        assert!(par_cle(None, "gallimard").is_none());
        assert!(par_cle(None, "folio").is_some());
    }

    /// Le voile n'a de sens que sur une image : l'allumer sans image assombrirait
    /// une couverture qui n'a rien dessous.
    #[test]
    fn seule_la_maquette_a_image_pleine_page_porte_un_voile() {
        assert_eq!(fournie("folio").voile, Voile::Aucun);
        assert_eq!(fournie("blanche").voile, Voile::Aucun);
        assert_ne!(fournie("surimpression").voile, Voile::Aucun);
    }
```

`Mode` et `Voile` viennent de `crate::couverture` : ajouter au module `tests`
`use crate::couverture::{Mode, Voile};` si le `use super::*` ne les apporte plus.

- [ ] **Step 4: Voir les tests de propriété échouer sur une mutation ciblée**

Ces trois-là ne sont pas neufs mais ils portent désormais sur des archives, et un test
qui n'a jamais été rouge sous sa nouvelle forme ne protège rien. Muter une archive et
vérifier qu'ils la refusent :

```bash
cd app/src-tauri
cp maquettes/folio.maquette /tmp/folio.sauvegarde
mkdir -p /tmp/mut && cd /tmp/mut && rm -rf ./* && unzip -q /Users/jean-paulgavini/Documents/Dev/ozalid/app/src-tauri/maquettes/folio.maquette
sed -i '' 's/^mode = "bandeau"/mode = "typo"/' maquette.toml
zip -q -X /Users/jean-paulgavini/Documents/Dev/ozalid/app/src-tauri/maquettes/folio.maquette maquette.toml
cd /Users/jean-paulgavini/Documents/Dev/ozalid/app/src-tauri && cargo test --lib maquettes 2>&1 | tail -20
```

Attendu : `les_trois_maquettes_sont_de_modes_distincts` **échoue** — « mode Typo en
double ». Vérifier au passage la valeur réellement écrite dans le TOML pour `mode` si le
`sed` ne mord pas (`unzip -p … maquette.toml | grep '^mode'`).

Puis remettre l'archive :

```bash
cp /tmp/folio.sauvegarde /Users/jean-paulgavini/Documents/Dev/ozalid/app/src-tauri/maquettes/folio.maquette
cd /Users/jean-paulgavini/Documents/Dev/ozalid && git status --short app/src-tauri/maquettes
```

Attendu : rien — l'archive est revenue à l'octet près.

- [ ] **Step 5: Lancer toute la suite**

```bash
cd app/src-tauri && cargo test 2>&1 | tail -10 && cargo clippy --all-targets -- -D warnings && cargo fmt --check
```

Attendu : `test result: ok.`, clippy muet, `fmt` muet.

- [ ] **Step 6: Commiter**

```bash
cd /Users/jean-paulgavini/Documents/Dev/ozalid
git add app/src-tauri/src/maquettes.rs
git commit -m "$(cat <<'EOF'
Les constructeurs ont fini de servir de moule

Folio, Blanche et Surimpression ne vivent plus que dans leurs archives. Les
tests de propriété portent sur ce qui est lu ; le pourquoi du pied à 13,5 %
rejoint le test qui le borne, seul endroit où il reste écrit.

Claude-Session: https://claude.ai/code/session_01AyXS78gurZmV9m6yFQ8LwL
EOF
)"
```

---

## Tâche 7 : le témoin, le front et le README

**Files:**
- Modify: `app/README.md`

- [ ] **Step 1: Le témoin de non-régression**

C'est la garde du lot : le témoin compose en Blanche, donc si l'archive ne rend pas ce
que le constructeur rendait, le compte de pages ou le dos bougent.

```bash
cd app/src-tauri && cargo run --example temoin 2>&1 | tail -20
```

Attendu, à l'identique du dernier relevé : **98 pages, dos 7,21 mm**. Tout écart arrête
le lot — c'est exactement ce que la bascule ne doit pas faire.

- [ ] **Step 2: Les tests du front**

Rien du front ne change à ce lot (`MaquetteVue` garde ses deux champs), mais la suite
doit rester verte :

```bash
cd app && node --test tests/*.test.js 2>&1 | tail -8
```

Attendu : `pass 189`, `fail 0` — ou le compte du jour, `fail 0` dans tous les cas.

- [ ] **Step 3: Rendre les maquettes en PNG et les regarder**

La vérification qu'aucun test ne fait — la position du cadre, l'assiette du bloc titre,
le voile :

```bash
cd app/src-tauri
cargo run --example maquette -- "../../build/projects/Les Heures creuses.ozalid" lulu /tmp/maquettes-lot1
open /tmp/maquettes-lot1
```

Attendu : quatre jeux de PNG — la maquette du projet, puis Folio, Blanche et
Surimpression — et rien n'y a bougé. `build/` n'étant pas versionné, si ce projet a
disparu, en prendre un autre (`find ../../build -name '*.ozalid'`) ; s'il n'y en a
aucun, le dire au compte rendu plutôt que de sauter l'étape.

- [ ] **Step 4: Mettre le README à jour**

Dans `app/README.md`, la ligne du tableau des modules :

```markdown
| `maquettes` | Le format `.maquette`, les trois fournies embarquées, et leur lecture |
```

Puis, à la fin de la section « Le fichier .ozalid » (elle commence ligne 179 et se
termine avant le titre de niveau 2 suivant), ajouter une section sœur :

```markdown
## Le fichier .maquette

Une archive du même genre, et pour la même raison — une maquette porte des images, elle
ne peut donc pas être un TOML seul :

```
maquette.toml   le nom affiché, et la couverture entière
images/         couverture.ext et quatrieme.ext, quand la maquette en porte
```

Elle emporte la couverture **telle qu'elle est à l'écran** : les modes, le cadre, les
styles, la pastille, le dos, le voile, le cadrage et le résumé de 4ème. Pas l'identité du
livre — l'éditeur, la collection, le monogramme, le prix et la mention sont au livre
depuis le chantier précédent, et une maquette ne peut donc plus les emporter. Le résumé
de 4ème, lui, reconnaît les jetons : une maquette peut porter un `%TITRE%, un %GENRE% de
%AUTEUR%.` qui se résout pour chaque livre où on la charge.

Les trois **fournies** — Folio, Blanche, Surimpression — vivent dans
`app/src-tauri/maquettes/` et sont incorporées au binaire par `include_bytes!` : il n'y a
aucun fichier à résoudre sur le poste, aucun écart entre développement et livraison, et
leur immuabilité est un fait plutôt qu'une règle applicative. Ce sont des **sources**,
au même titre qu'un `.rs` : elles ne se regénèrent plus depuis rien, les constructeurs
qui les ont gravées ayant été retirés.
```

Attention en recopiant : le bloc de code imbriqué ci-dessus (`maquette.toml`, `images/`)
est délimité par trois accents graves dans le README final, comme celui du `.ozalid`
juste au-dessus.

- [ ] **Step 5: Commiter**

```bash
cd /Users/jean-paulgavini/Documents/Dev/ozalid
git add app/README.md
git commit -m "$(cat <<'EOF'
Le README dit ce qu'est devenue une maquette

Témoin relevé après la bascule : 98 pages, dos 7,21 mm, inchangé.

Claude-Session: https://claude.ai/code/session_01AyXS78gurZmV9m6yFQ8LwL
EOF
)"
```

---

## Ce que l'exécution a corrigé au plan

Écrit après coup, pour que les lots 2 et 3 ne rejouent pas les mêmes trous :

- **Les tâches 1 à 5 ont fait un seul commit.** `clippy -D warnings` refuse le code
  mort : tant que rien n'appelle `lire` en production — c'est-à-dire tant que `toutes`
  n'est pas implantée, tâche 4 — le module ne compile pas en mode livraison. Découper
  plus fin aurait demandé des commits qui ne passent pas les vérifications du projet.
- **`ecrire` porte `#[cfg_attr(not(test), allow(dead_code))]`**, pour la même raison :
  son premier appelant de production est le « Enregistrer la couverture actuelle » du
  lot 2, qui lèvera l'attribut.
- **`use std::path::Path` n'arrive qu'à la tâche 3**, sans quoi il est un import
  inutilisé et clippy le refuse.
- **Les deux tests de propriété du module ont été migrés à la tâche 4, pas à la 6** : la
  signature de `toutes` change, ils ne compilaient plus. Bénéfice imprévu — ils ont été
  vus rouges sur le stub qui rend une liste vide, ce qui vaut mieux que la mutation
  d'archive prévue en tâche 6 (faite quand même : `mode = "typo"` dans `folio.maquette`
  rend bien `les_trois_maquettes_sont_de_modes_distincts` rouge).
- **`examples/maquette.rs`** : `cv` y devient un `&Couverture`, donc les deux `&cv` des
  appels à `source_une` / `source_quatre` sont à retirer (`needless_borrow`).

---

## Ce que ce lot ne fait pas

À dire à la revue, pour qu'on ne les prenne pas pour des oublis :

- **`toutes` ignore son paramètre `config`.** Le lot 2 y lit `<config>/maquettes/`.
- **`maquette_choisir` ne pose pas les images.** Aucune fournie n'en porte ; le
  comportement arrive au lot 2 avec le premier cas qui l'exerce.
- **Aucun slug n'est calculé.** Les trois clés sont celles de la table `FOURNIES`. La
  dérivation nom → slug, et le refus de deux noms qui donnent le même, sont au lot 2.
- **Le front est intact.** `MaquetteVue` ne gagne `fournie` qu'au lot 2, quand quelque
  chose en dépendra.
