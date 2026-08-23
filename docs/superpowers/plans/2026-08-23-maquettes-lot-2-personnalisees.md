# Maquettes en fichiers — lot 2 : les personnalisées

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** La couverture qu'on vient de régler s'enregistre comme maquette dans le répertoire de configuration, sous un nom dont dérive son slug, et se recharge sur un autre livre — images comprises — sans jamais toucher à l'identité de ce livre-là.

**Architecture:** Le slug d'abord, seul à décider si deux noms sont le même nom. Puis `<config>/maquettes/` lu au mieux et `ecrire` qui échoue fort, l'unicité étant imposée sur tout l'ensemble, fournies comprises. Puis les trois commandes, qui reçoivent l'`AppHandle`. Enfin le front : le remplissage du `<select>` devient une fonction rappelable, et un `<dialog>` porte le seul geste du lot.

**Tech Stack:** Rust 2021, `serde`, `toml`, `zip 7`, `tempfile` en dev ; front vanilla, `node --test`.

Spec : `docs/superpowers/specs/2026-08-23-maquettes-en-fichiers-design.md`, lot 2 du § 8.
Lot précédent : `docs/superpowers/plans/2026-08-23-maquettes-lot-1-archives-fournies.md`.

---

## Trois décisions prises en écrivant ce plan

**1. Le `<dialog>` naît ici, avec sa seule zone d'enregistrement** — décidé avec
l'utilisateur le 23/08. La spec le range au lot 3, mais le geste du lot 2 a besoin d'un
endroit où saisir un nom, et la barre de l'étape Couverture tient tout juste sur une
ligne à 900 px (c'est écrit dans `index.html`). Un champ inline serait une interface
jetable dans la barre la plus serrée de la fenêtre. Le lot 3 remplira le dialogue :
liste, Cloner, Renommer, Effacer.

**2. `ecrire` prend la couverture et les images, non un `&Maquette`.** La spec écrit
`ecrire(config, nom, m: &Maquette)`, mais trois des cinq champs de `Maquette` y seraient
ignorés — `cle`, `nom` et `fournie` — ce qui oblige l'appelant à inventer une clé que la
fonction recalcule aussitôt. Quatre paramètres dont aucun n'est mort valent mieux. Le
`cloner` du lot 3 s'en accommode aussi bien : `ecrire(config, nouveau, &m.couverture,
&m.images)`.

**3. Le séparateur du `<select>` est une `<option disabled>`, pas un `<optgroup>`.** Le
faux DOM des tests sélectionne la première option **enfant** d'un `<select>` ; des
options rangées dans un `optgroup` ne le sont plus, et le shim mentirait sur un point
que l'application exerce à chaque geste. Une option désactivée garde la liste plate,
donne le même trait horizontal à l'œil, et ne peut pas être choisie.

---

## Tâche 1 : le slug, qui dit si deux noms sont le même nom

**Files:**
- Modify: `app/src-tauri/src/maquettes.rs`

- [ ] **Step 1: Écrire les tests et le stub**

Dans `app/src-tauri/src/maquettes.rs`, juste après `par_cle`, poser le stub :

```rust
/// Le slug d'un nom : ce qui nomme son fichier, et ce qui l'identifie.
pub fn slug(_nom: &str) -> Option<String> {
    None
}
```

Et dans le module `tests` :

```rust
    /// Le nom est l'identité, le slug nomme le fichier : accents décapés, casse
    /// ignorée, tout le reste en tirets. Deux noms qui donnent le même slug **sont**
    /// le même nom — c'est ce qui permet à l'écriture de refuser plutôt que d'écraser.
    #[test]
    fn le_slug_decape_les_accents_et_ignore_la_casse() {
        assert_eq!(slug("Ma collection").as_deref(), Some("ma-collection"));
        assert_eq!(slug("Élan  vital !").as_deref(), Some("elan-vital"));
        assert_eq!(slug("Cœur").as_deref(), Some("coeur"));
        assert_eq!(slug("Ma Collection"), slug("ma  collection…"));
        assert_eq!(slug("Folio").as_deref(), Some("folio"));
    }

    /// Un slug ne borde jamais de tiret : `folio-.maquette` se relirait en clé
    /// « folio- », qui ne serait plus le slug de son propre nom.
    #[test]
    fn le_slug_ne_borde_pas_de_tiret() {
        assert_eq!(slug("  Folio  ").as_deref(), Some("folio"));
        assert_eq!(slug("— Folio —").as_deref(), Some("folio"));
    }

    /// Un nom qui ne s'écrit avec aucune lettre latine ne peut pas nommer un fichier.
    /// Lui inventer « maquette-1 » cacherait le problème derrière un nom que
    /// l'utilisateur n'a pas choisi et ne saurait pas retrouver.
    #[test]
    fn un_nom_sans_lettre_latine_n_a_pas_de_slug() {
        assert_eq!(slug(""), None);
        assert_eq!(slug("   "), None);
        assert_eq!(slug("——"), None);
        assert_eq!(slug("日本"), None);
    }
```

- [ ] **Step 2: Lancer les tests et les voir échouer**

```bash
cd app/src-tauri && cargo test --lib maquettes::tests::le_slug 2>&1 | tail -12
```

Attendu : `FAILED` — `assertion failed: None == Some("ma-collection")`.

- [ ] **Step 3: Implanter le slug**

Remplacer le stub par :

```rust
/// Les lettres latines accentuées, ramenées à leur base ASCII.
///
/// Une table plutôt qu'une dépendance de normalisation Unicode : le besoin tient dans
/// les alphabets latins, et une crate de plus pour cinquante caractères coûterait plus
/// cher que ce qu'elle rendrait. Les majuscules n'y figurent pas — la casse est abaissée
/// avant la table.
const ACCENTS: &[(char, &str)] = &[
    ('à', "a"),
    ('á', "a"),
    ('â', "a"),
    ('ã', "a"),
    ('ä', "a"),
    ('å', "a"),
    ('ç', "c"),
    ('è', "e"),
    ('é', "e"),
    ('ê', "e"),
    ('ë', "e"),
    ('ì', "i"),
    ('í', "i"),
    ('î', "i"),
    ('ï', "i"),
    ('ñ', "n"),
    ('ò', "o"),
    ('ó', "o"),
    ('ô', "o"),
    ('õ', "o"),
    ('ö', "o"),
    ('ù', "u"),
    ('ú', "u"),
    ('û', "u"),
    ('ü', "u"),
    ('ý', "y"),
    ('ÿ', "y"),
    ('æ', "ae"),
    ('œ', "oe"),
    ('ß', "ss"),
];

/// Le slug d'un nom : ce qui nomme son fichier, et ce qui l'identifie.
///
/// Accents décapés, casse ignorée, tout ce qui n'est ni lettre ni chiffre ASCII devient
/// un tiret, et deux tirets d'affilée n'en font qu'un. « Ma Collection » et
/// « ma collection… » donnent donc le même slug : ce sont le même nom, et `ecrire` le
/// refuse au lieu d'écraser.
///
/// `None` quand il ne reste rien — un nom qui ne s'écrit avec aucune lettre latine ne
/// peut pas nommer un fichier, et lui en inventer un le rendrait introuvable.
pub fn slug(nom: &str) -> Option<String> {
    let mut decape = String::with_capacity(nom.len());
    for c in nom.chars().flat_map(char::to_lowercase) {
        match ACCENTS.iter().find(|(a, _)| *a == c) {
            Some((_, base)) => decape.push_str(base),
            None => decape.push(c),
        }
    }
    let mut s = String::with_capacity(decape.len());
    for c in decape.chars() {
        if c.is_ascii_alphanumeric() {
            s.push(c);
        } else if !s.ends_with('-') {
            s.push('-');
        }
    }
    let net = s.trim_matches('-');
    (!net.is_empty()).then(|| net.to_string())
}
```

- [ ] **Step 4: Lancer les tests et les voir passer**

```bash
cd app/src-tauri && cargo test --lib maquettes 2>&1 | tail -6
```

Attendu : `test result: ok.`

- [ ] **Step 5: Vérifier et commiter**

```bash
cd app/src-tauri && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test 2>&1 | tail -3
cd /Users/jean-paulgavini/Documents/Dev/ozalid
git add app/src-tauri/src/maquettes.rs
git commit -m "$(cat <<'EOF'
Le nom est l'identité, le slug nomme le fichier

Accents décapés, casse ignorée, le reste en tirets : deux noms qui donnent le
même slug sont le même nom. Un nom sans lettre latine n'a pas de slug, et le
dira plutôt que de s'en voir inventer un.

Claude-Session: https://claude.ai/code/session_01AyXS78gurZmV9m6yFQ8LwL
EOF
)"
```

---

## Tâche 2 : les personnalisées se lisent dans le répertoire de configuration

**Files:**
- Modify: `app/src-tauri/src/maquettes.rs`

- [ ] **Step 1: Écrire les tests**

Dans le module `tests` de `app/src-tauri/src/maquettes.rs` :

```rust
    /// Écrit une archive à la main dans `<config>/maquettes/`, comme le ferait une
    /// version antérieure ou un utilisateur qui déplace ses fichiers.
    fn pose(config: &Path, fichier: &str, nom: &str) {
        let dir = config.join("maquettes");
        std::fs::create_dir_all(&dir).unwrap();
        let m = Maquette {
            cle: String::new(),
            nom: nom.into(),
            fournie: false,
            couverture: fournie("folio"),
            images: BTreeMap::new(),
        };
        ecrire_archive(std::fs::File::create(dir.join(fichier)).unwrap(), &m).unwrap();
    }

    /// Les personnalisées viennent après les fournies, dans l'ordre de leur nom : le
    /// menu propose d'abord ce qui est livré, puis ce qu'on a fait soi-même.
    #[test]
    fn les_personnalisees_suivent_les_fournies() {
        let dir = tempfile::tempdir().unwrap();
        pose(dir.path(), "zeste.maquette", "Zeste");
        pose(dir.path(), "ma-collection.maquette", "Ma collection");

        let vues: Vec<(String, bool)> = toutes(Some(dir.path()))
            .into_iter()
            .map(|m| (m.cle, m.fournie))
            .collect();
        assert_eq!(
            vues,
            [
                ("folio".to_string(), true),
                ("blanche".to_string(), true),
                ("surimpression".to_string(), true),
                ("ma-collection".to_string(), false),
                ("zeste".to_string(), false),
            ]
        );
    }

    /// La clé d'une personnalisée est le nom de son fichier : c'est lui qu'on retrouve
    /// sur le disque, et c'est par lui que le lot 3 la renommera et l'effacera.
    #[test]
    fn une_personnalisee_se_retrouve_par_sa_cle() {
        let dir = tempfile::tempdir().unwrap();
        pose(dir.path(), "ma-collection.maquette", "Ma collection");
        let m = par_cle(Some(dir.path()), "ma-collection").unwrap();
        assert_eq!(m.nom, "Ma collection");
        assert!(!m.fournie);
    }

    /// La lecture est au mieux : ce qui se perd est un point de départ, et refuser la
    /// liste entière pour un fichier de travers coûterait tous les autres.
    #[test]
    fn une_maquette_illisible_n_empeche_pas_les_autres_de_se_lister() {
        let dir = tempfile::tempdir().unwrap();
        pose(dir.path(), "bonne.maquette", "Bonne");
        let d = dir.path().join("maquettes");
        std::fs::write(d.join("cassee.maquette"), b"ceci n'est pas une archive").unwrap();
        // Ce qui ne porte pas l'extension n'est pas même regardé.
        std::fs::write(d.join("notes.txt"), b"rien a voir").unwrap();

        let cles: Vec<String> = toutes(Some(dir.path())).into_iter().map(|m| m.cle).collect();
        assert_eq!(cles, ["folio", "blanche", "surimpression", "bonne"]);
    }

    /// Répertoire de configuration inatteignable, ou aucune personnalisée encore
    /// écrite : les fournies restent servies. Même arbitrage que les projets récents.
    #[test]
    fn sans_configuration_les_fournies_restent() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(toutes(None).len(), 3, "aucun répertoire");
        assert_eq!(toutes(Some(dir.path())).len(), 3, "répertoire vide");
    }
```

- [ ] **Step 2: Lancer les tests et les voir échouer**

```bash
cd app/src-tauri && cargo test --lib maquettes 2>&1 | tail -15
```

Attendu : la compilation échoue sur `ecrire_archive` (la fonction s'appelle encore
`ecrire`), puis, une fois renommée, les quatre tests échouent sur des listes de trois.

- [ ] **Step 3: Renommer `ecrire` en `ecrire_archive` et lire le répertoire**

Le nom `ecrire` est réservé à la fonction publique de la tâche 3 : renommer l'écrivain
d'archive du lot 1 et retirer sa dispense de code mort, qui n'a plus lieu d'être dès que
`ecrire` l'appellera.

```rust
/// Écrit une archive de maquette. Ni le nom du fichier ni l'unicité ne la regardent :
/// c'est `ecrire` qui en décide.
fn ecrire_archive<W: Write + Seek>(sortie: W, m: &Maquette) -> Result<(), String> {
```

(supprimer les quatre lignes de doc-comment « Écrire une archive n'a pas encore
d'appelant… » et l'attribut `#[cfg_attr(not(test), allow(dead_code))]`, et corriger les
deux appels dans les tests de la tâche 1 du lot précédent.)

Puis, sous `FOURNIES` :

```rust
/// L'extension d'une archive de maquette, sans le point.
const EXT: &str = "maquette";

/// Là où vivent les personnalisées : à côté de `preferences.toml`, parce qu'elles
/// appartiennent à la machine et non au livre. Un `.ozalid` reste auto-portant — sa
/// couverture est dans l'archive ; une maquette n'est qu'un point de départ.
fn repertoire(config: &Path) -> PathBuf {
    config.join("maquettes")
}

/// Les personnalisées, dans l'ordre de leur nom.
///
/// Un répertoire absent n'est pas une avarie : c'est l'état d'un poste où l'on n'a
/// encore rien enregistré.
fn personnalisees(config: &Path) -> Vec<Maquette> {
    let Ok(entrees) = std::fs::read_dir(repertoire(config)) else {
        return Vec::new();
    };
    let mut v = Vec::new();
    for e in entrees.flatten() {
        let chemin = e.path();
        if chemin.extension().and_then(|x| x.to_str()) != Some(EXT) {
            continue;
        }
        let Some(cle) = chemin.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        match std::fs::File::open(&chemin)
            .map_err(|err| err.to_string())
            .and_then(|f| lire(f, cle, false))
        {
            Ok(m) => v.push(m),
            Err(err) => eprintln!("maquette « {} » ignorée : {err}", chemin.display()),
        }
    }
    // Par le nom affiché, qui est l'identité — et non par la clé, que l'utilisateur ne
    // voit nulle part. L'ordre suit les codes de caractères : « Étoile » passe donc
    // après « Zeste », ce qui est un pis-aller assumé plutôt qu'une collation complète.
    v.sort_by(|a, b| a.nom.cmp(&b.nom));
    v
}
```

et compléter `toutes` :

```rust
pub fn toutes(config: Option<&Path>) -> Vec<Maquette> {
    let mut v: Vec<Maquette> = FOURNIES
        .iter()
        .filter_map(|(cle, octets)| {
            lire(std::io::Cursor::new(*octets), cle, true)
                .map_err(|e| eprintln!("maquette fournie « {cle} » illisible : {e}"))
                .ok()
        })
        .collect();
    if let Some(c) = config {
        v.extend(personnalisees(c));
    }
    v
}
```

Ajouter `PathBuf` à l'import : `use std::path::{Path, PathBuf};`.

- [ ] **Step 4: Lancer les tests et les voir passer**

```bash
cd app/src-tauri && cargo test --lib maquettes 2>&1 | tail -8
```

Attendu : `test result: ok.` Le test `une_maquette_illisible_n_empeche_pas_les_autres`
écrit une ligne sur la sortie d'erreur — c'est voulu, et `cargo test` ne la montre que
sur échec.

- [ ] **Step 5: Vérifier et commiter**

```bash
cd app/src-tauri && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test 2>&1 | tail -3
cd /Users/jean-paulgavini/Documents/Dev/ozalid
git add app/src-tauri/src/maquettes.rs
git commit -m "$(cat <<'EOF'
Les maquettes du poste se lisent à côté des préférences

<config>/maquettes/, lu au mieux : une archive de travers s'écarte avec un mot,
elle ne coûte pas la liste. Les personnalisées suivent les fournies, dans
l'ordre de leur nom.

Claude-Session: https://claude.ai/code/session_01AyXS78gurZmV9m6yFQ8LwL
EOF
)"
```

---

## Tâche 3 : `ecrire`, et l'unicité qui refuse

**Files:**
- Modify: `app/src-tauri/src/maquettes.rs`

- [ ] **Step 1: Écrire les tests**

Dans le module `tests` :

```rust
    /// L'aller-retour complet d'une personnalisée : ce qu'on enregistre est ce qu'on
    /// retrouve, images comprises. C'est la promesse du geste — la couverture réglée
    /// pour un livre resservira au suivant.
    #[test]
    fn une_personnalisee_enregistree_se_recharge_entiere() {
        let dir = tempfile::tempdir().unwrap();
        let mut images = BTreeMap::new();
        images.insert("couverture.jpg".to_string(), vec![0xff, 0xd8, 0xff, 0xe0]);
        let cv = fournie("surimpression");

        ecrire(dir.path(), "Ma collection", &cv, &images).unwrap();

        let m = par_cle(Some(dir.path()), "ma-collection").unwrap();
        assert_eq!(m.nom, "Ma collection");
        assert!(!m.fournie);
        assert_eq!(m.couverture, cv);
        assert_eq!(m.images, images);
    }

    /// L'unicité porte sur **tout** l'ensemble, fournies comprises : une personnalisée
    /// nommée « Folio » ferait deux entrées de même clé dans le menu, et la seconde
    /// serait inatteignable. Le refus nomme celle qui tient déjà la place.
    #[test]
    fn un_nom_deja_pris_est_refuse_fournie_comprise() {
        let dir = tempfile::tempdir().unwrap();
        let cv = fournie("folio");

        let e = ecrire(dir.path(), "Folio", &cv, &BTreeMap::new()).unwrap_err();
        assert!(e.contains("Folio"), "{e}");

        ecrire(dir.path(), "Ma collection", &cv, &BTreeMap::new()).unwrap();
        // Même slug, autre casse et autre ponctuation : c'est le même nom.
        let e = ecrire(dir.path(), "ma  collection !", &cv, &BTreeMap::new()).unwrap_err();
        assert!(e.contains("Ma collection"), "{e}");
    }

    /// Un « Enregistrer » qui échoue perd du travail : il remonte, il ne s'arrange pas
    /// en silence avec un nom que personne n'a choisi.
    #[test]
    fn un_nom_sans_slug_est_refuse_plutot_qu_arrange() {
        let dir = tempfile::tempdir().unwrap();
        let e = ecrire(dir.path(), "  ", &fournie("folio"), &BTreeMap::new()).unwrap_err();
        assert!(e.contains("lettre"), "{e}");
        assert!(
            toutes(Some(dir.path())).iter().all(|m| m.fournie),
            "rien ne doit avoir été écrit"
        );
    }
```

- [ ] **Step 2: Lancer les tests et les voir échouer**

```bash
cd app/src-tauri && cargo test --lib maquettes 2>&1 | tail -12
```

Attendu : la compilation échoue — `cannot find function ecrire in this scope`.

- [ ] **Step 3: Implanter `ecrire`**

Après `personnalisees` :

```rust
/// Enregistre une couverture comme maquette personnalisée.
///
/// **L'écriture échoue fort**, là où la lecture est au mieux : un « Enregistrer » qui
/// échoue en silence perd du travail. Deux refus, et ils disent tous deux quoi faire —
/// un nom qui ne donne aucun slug, et un nom déjà pris.
///
/// L'unicité porte sur l'ensemble, fournies comprises : deux entrées de même clé
/// rendraient la seconde inatteignable par `par_cle`.
///
/// La couverture et les images sont passées telles quelles — c'est l'instantané fidèle
/// de la spec : ce que la maquette emporte est ce qui était à l'écran. La discipline
/// (des images neutres, un résumé de 4ème en jetons) appartient à l'utilisateur ;
/// filtrer demanderait au code de deviner ce qui est générique, et il devinerait mal.
pub fn ecrire(
    config: &Path,
    nom: &str,
    couverture: &Couverture,
    images: &BTreeMap<String, Vec<u8>>,
) -> Result<(), String> {
    let cle = slug(nom).ok_or_else(|| {
        format!("« {nom} » ne peut pas nommer une maquette : il y faut au moins une lettre ou un chiffre.")
    })?;
    if let Some(prise) = toutes(Some(config)).into_iter().find(|m| m.cle == cle) {
        return Err(format!("« {} » porte déjà ce nom.", prise.nom));
    }
    let dir = repertoire(config);
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("répertoire des maquettes inutilisable ({}) : {e}", dir.display()))?;
    let chemin = dir.join(format!("{cle}.{EXT}"));
    let f = std::fs::File::create(&chemin)
        .map_err(|e| format!("écriture de {} : {e}", chemin.display()))?;
    ecrire_archive(
        f,
        &Maquette {
            cle,
            nom: nom.into(),
            fournie: false,
            couverture: couverture.clone(),
            images: images.clone(),
        },
    )
}
```

- [ ] **Step 4: Lancer les tests et les voir passer**

```bash
cd app/src-tauri && cargo test --lib maquettes 2>&1 | tail -8
```

Attendu : `test result: ok.`

- [ ] **Step 5: Vérifier et commiter**

```bash
cd app/src-tauri && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test 2>&1 | tail -3
cd /Users/jean-paulgavini/Documents/Dev/ozalid
git add app/src-tauri/src/maquettes.rs
git commit -m "$(cat <<'EOF'
Une couverture réglée s'enregistre pour le livre suivant

L'écriture échoue fort là où la lecture est au mieux : un nom sans slug et un
nom déjà pris sont refusés, et le refus nomme qui tient la place. L'unicité
couvre les fournies — deux clés identiques rendraient la seconde inatteignable.

Claude-Session: https://claude.ai/code/session_01AyXS78gurZmV9m6yFQ8LwL
EOF
)"
```

---

## Tâche 4 : les commandes voient les deux origines

**Files:**
- Modify: `app/src-tauri/src/commands.rs`
- Modify: `app/src-tauri/src/lib.rs` (déclaration de la commande neuve)

- [ ] **Step 1: `MaquetteVue` gagne `fournie`, et les commandes l'`AppHandle`**

Dans `app/src-tauri/src/commands.rs`, remplacer le bloc des maquettes :

```rust
#[derive(Serialize)]
pub struct MaquetteVue {
    cle: String,
    libelle: String,
    /// Ni renommable, ni effaçable. La fenêtre s'en sert pour ne pas offrir des gestes
    /// que le Rust refuserait de toute façon — l'interface est une politesse, le refus
    /// est ailleurs.
    fournie: bool,
}

#[tauri::command]
pub fn maquettes_liste(app: tauri::AppHandle) -> Vec<MaquetteVue> {
    maquettes::toutes(config(&app).as_deref())
        .into_iter()
        .map(|m| MaquetteVue {
            cle: m.cle,
            libelle: m.nom,
            fournie: m.fournie,
        })
        .collect()
}
```

et `maquette_choisir`, qui pose aussi les images :

```rust
/// Charge une maquette de départ. Elle remplace la mise en page **et les images**,
/// jamais l'identité du livre : le titre et l'auteur imprimés restent ceux du projet.
///
/// Les images se posent rôle par rôle : une maquette qui porte une photo de 1ère la
/// pose, une maquette qui n'en porte pas laisse celle du livre où elle est. Sans quoi
/// charger une maquette purement typographique effacerait la photo du livre.
#[tauri::command]
pub fn maquette_choisir(
    cle: String,
    app: tauri::AppHandle,
    atelier: State<Atelier>,
) -> Result<ProjetVue, String> {
    let m = maquettes::par_cle(config(&app).as_deref(), &cle)
        .ok_or_else(|| format!("maquette inconnue : {cle}"))?;
    let mut garde = atelier.ouvert.lock().unwrap();
    let o = garde.as_mut().ok_or_else(aucun_projet)?;
    o.projet.meta.couverture.maquette = Some(m.couverture);
    for (nom, octets) in m.images {
        poser_image(&mut o.projet.images, nom, octets);
    }
    vue_modifiee(o)
}

/// Enregistre la couverture du projet ouvert comme maquette personnalisée.
///
/// Le projet n'est pas touché : ce geste écrit à côté, dans le répertoire de
/// configuration, et ne rend donc aucune `ProjetVue`. La fenêtre rafraîchit sa liste en
/// rappelant `maquettes_liste`, seule source de vérité.
#[tauri::command]
pub fn maquette_enregistrer(
    nom: String,
    app: tauri::AppHandle,
    atelier: State<Atelier>,
) -> Result<(), String> {
    let dir = config(&app).ok_or("répertoire de configuration introuvable.")?;
    let garde = atelier.ouvert.lock().unwrap();
    let o = garde.as_ref().ok_or_else(aucun_projet)?;
    let cv = o
        .projet
        .meta
        .couverture
        .maquette
        .as_ref()
        .ok_or("aucune maquette : en choisir une avant de l'enregistrer.")?;
    maquettes::ecrire(&dir, &nom, cv, &o.projet.images)
}
```

- [ ] **Step 2: Déclarer la commande**

Dans `app/src-tauri/src/lib.rs`, ajouter `commands::maquette_enregistrer` à la liste
`generate_handler!`, juste après `commands::maquette_choisir`.

- [ ] **Step 3: Compiler et lancer la suite**

```bash
cd app/src-tauri && cargo test 2>&1 | tail -5 && cargo clippy --all-targets -- -D warnings 2>&1 | tail -2
```

Attendu : `test result: ok.` et clippy muet. Si `poser_image` est déclarée plus bas que
`maquette_choisir`, rien à faire : Rust ne demande pas l'ordre.

- [ ] **Step 4: Commiter**

```bash
cd /Users/jean-paulgavini/Documents/Dev/ozalid
git add app/src-tauri/src/commands.rs app/src-tauri/src/lib.rs
git commit -m "$(cat <<'EOF'
Les commandes voient les maquettes des deux origines

maquettes_liste dit lesquelles sont fournies, maquette_choisir pose aussi les
images — rôle par rôle, pour qu'une maquette sans photo n'efface pas celle du
livre — et maquette_enregistrer écrit à côté du projet, sans le toucher.

Claude-Session: https://claude.ai/code/session_01AyXS78gurZmV9m6yFQ8LwL
EOF
)"
```

---

## Tâche 5 : le `<select>` se remplit, le dialogue enregistre

**Files:**
- Modify: `app/src/index.html`
- Modify: `app/src/couverture.js`
- Modify: `app/src/app.js:349-352`
- Modify: `app/src/styles.css`
- Modify: `app/tests/dom_shim.js` (`showModal` / `close`)
- Modify: `app/tests/couverture.test.js` (les tests du lot)

- [ ] **Step 1: Écrire les tests du front**

Dans `app/tests/couverture.test.js`, en fin de fichier. Le fichier a déjà tout ce qu'il
faut : `ouvre(couverture, sur, dialogues)` charge l'application sur un projet ouvert, et
son deuxième paramètre **surcharge** une commande — les surcharges passent avant les
réponses par défaut. Son `maquettes_liste` par défaut (trois fournies sans `fournie`) est
à compléter d'un `fournie: true` sur les trois, sans quoi le séparateur se poserait
devant Folio.

```js
/** Les trois fournies, plus une personnalisée : de quoi exercer le séparateur. */
const AVEC_PERSONNALISEE = () => [
  { cle: 'folio', libelle: 'Folio', fournie: true },
  { cle: 'blanche', libelle: 'Blanche', fournie: true },
  { cle: 'ma-collection', libelle: 'Ma collection', fournie: false },
];

/**
 * Le menu est un geste, pas un état : les personnalisées s'y rangent après les
 * fournies, derrière un séparateur qu'on ne peut pas choisir. Une option désactivée
 * plutôt qu'un `optgroup` — le faux DOM sélectionne la première option *enfant* d'un
 * select, et des options rangées dans un groupe ne le seraient plus.
 */
test('le menu des maquettes range les personnalisées sous un séparateur', async () => {
  const { els } = await ouvre(maquette(), { maquettes_liste: AVEC_PERSONNALISEE });
  const options = [...els.get('inMaquette').children].map((o) => ({
    texte: o.textContent, valeur: o.value, inerte: !!o.disabled,
  }));
  assert.deepEqual(options, [
    { texte: 'Repartir d\'une maquette…', valeur: '', inerte: false },
    { texte: 'Folio', valeur: 'folio', inerte: false },
    { texte: 'Blanche', valeur: 'blanche', inerte: false },
    { texte: '──────────', valeur: '', inerte: true },
    { texte: 'Ma collection', valeur: 'ma-collection', inerte: false },
  ]);
});

/**
 * Le geste du lot : le nom saisi part au Rust, et la liste se refait derrière — sans
 * quoi la maquette qu'on vient d'enregistrer manquerait au menu jusqu'au prochain
 * démarrage.
 */
test('enregistrer une maquette la fait paraître au menu', async () => {
  const enregistrees = [];
  const { els } = await ouvre(maquette(), {
    maquette_enregistrer: ({ nom }) => { enregistrees.push(nom); return null; },
    maquettes_liste: () => [
      { cle: 'folio', libelle: 'Folio', fournie: true },
      ...enregistrees.map((n) => ({ cle: 'x', libelle: n, fournie: false })),
    ],
  });

  els.get('btMaquettes').declenche('click');
  els.get('inMaquetteNom').value = 'Ma collection';
  await els.get('btMaquetteEnregistrer').declenche('click');

  assert.deepEqual(enregistrees, ['Ma collection']);
  assert.ok(
    [...els.get('inMaquette').children].some((o) => o.textContent === 'Ma collection'),
    'la maquette enregistrée doit paraître au menu'
  );
  assert.equal(els.get('inMaquetteNom').value, '', 'le champ se vide après le geste');
  assert.ok(els.get('dlgMaquettes').open, 'le dialogue reste ouvert : on en enregistre souvent deux');
});

/**
 * Un refus du Rust — nom déjà pris, nom sans slug — doit se lire *dans* le dialogue :
 * l'alerte de la fenêtre est derrière lui, et le geste paraîtrait avoir marché.
 */
test('un refus d\'enregistrement se lit dans le dialogue', async () => {
  const { els } = await ouvre(maquette(), {
    maquette_enregistrer: () => { throw new Error('« Folio » porte déjà ce nom.'); },
    maquettes_liste: AVEC_PERSONNALISEE,
  });
  els.get('btMaquettes').declenche('click');
  els.get('inMaquetteNom').value = 'Folio';
  await els.get('btMaquetteEnregistrer').declenche('click');
  assert.match(els.get('etatMaquettes').textContent, /porte déjà ce nom/);
});
```

Et, dans le `maquettes_liste` par défaut d'`ouvre`, ajouter `fournie: true` aux trois
entrées.

- [ ] **Step 2: Lancer les tests et les voir échouer**

```bash
cd app && node --test tests/couverture.test.js 2>&1 | tail -20
```

Attendu : trois échecs — l'option de séparateur manque, et `inMaquetteNom` /
`btMaquetteEnregistrer` / `etatMaquettes` n'existent pas dans `index.html`.

- [ ] **Step 3: Le bouton et le dialogue dans `index.html`**

Après le `<select id="inMaquette">` de la barre de l'étape Couverture :

```html
      <button id="btMaquettes" type="button">Maquettes…</button>
```

Et, juste avant la fermeture de `</section>` de l'étape Couverture, le dialogue —
premier `<dialog>` de ce front, et c'est la primitive standard : elle gère seule le
focus et Échap, ce qu'aucune boîte faite à la main ne fait correctement.

```html
    <!-- Gérer ses maquettes suppose un projet ouvert : le bouton ne vit que dans
         l'étape Couverture, conséquence assumée du choix « on dessine une couverture
         sur un vrai livre et un vrai format ». Le lot 3 y ajoutera la liste, Cloner,
         Renommer et Effacer. -->
    <dialog id="dlgMaquettes" class="dialogue">
      <h2>Maquettes</h2>
      <p class="note">
        Une maquette emporte la couverture entière — le cadrage, les photos et le résumé
        de 4ème. Préférer des images neutres et un résumé en jetons : ils se résoudront
        pour chaque livre où la maquette servira.
      </p>
      <div class="ligne">
        <label for="inMaquetteNom">Enregistrer la couverture actuelle</label>
        <input id="inMaquetteNom" type="text" placeholder="Nom de la maquette">
        <button id="btMaquetteEnregistrer" type="button">Enregistrer</button>
      </div>
      <p class="etat" id="etatMaquettes"></p>
      <div class="ligne fin">
        <button id="btMaquettesFermer" type="button">Fermer</button>
      </div>
    </dialog>
```

- [ ] **Step 4: Le remplissage devient une fonction, dans `couverture.js`**

Retirer d'`app.js` (lignes 349-352) les trois lignes qui remplissent le `<select>` et
poser à la place :

```js
  await remplirMaquettes();
  $('inMaquette').addEventListener('change', choisirMaquette);
  $('btMaquettes').addEventListener('click', () => {
    $('etatMaquettes').textContent = '';
    $('dlgMaquettes').showModal();
  });
  $('btMaquettesFermer').addEventListener('click', () => $('dlgMaquettes').close());
  $('btMaquetteEnregistrer').addEventListener('click', enregistrerMaquette);
```

Dans `couverture.js`, à côté de `choisirMaquette` :

```js
/**
 * (Re)remplit le menu des maquettes.
 *
 * Rappelée après chaque geste du dialogue : la liste vit dans le Rust, qui lit le
 * répertoire de configuration à chaque appel. La refaire ici serait la dédoubler.
 *
 * Les personnalisées suivent les fournies, derrière une option inerte qui fait le
 * séparateur — le Rust les rend déjà dans cet ordre, la fenêtre n'a qu'à repérer où
 * l'origine change.
 */
async function remplirMaquettes() {
  const sel = $('inMaquette');
  sel.replaceChildren();
  sel.append(new Option('Repartir d\'une maquette…', ''));
  let separateur = false;
  for (const m of await invoke('maquettes_liste')) {
    if (!m.fournie && !separateur) {
      const trait = new Option('──────────', '');
      trait.disabled = true;
      sel.append(trait);
      separateur = true;
    }
    sel.append(new Option(m.libelle, m.cle));
  }
  sel.value = '';
}

/**
 * Enregistre la couverture réglée comme maquette.
 *
 * Le compte rendu se lit dans le dialogue et non dans l'alerte de la fenêtre : celle-ci
 * est derrière lui, et un refus y passerait inaperçu — le geste paraîtrait avoir marché.
 */
async function enregistrerMaquette() {
  const nom = $('inMaquetteNom').value.trim();
  try {
    await invoke('maquette_enregistrer', { nom });
    $('inMaquetteNom').value = '';
    $('etatMaquettes').textContent = `« ${nom} » enregistrée.`;
    $('etatMaquettes').className = 'etat';
    await remplirMaquettes();
  } catch (e) {
    $('etatMaquettes').textContent = String(e);
    $('etatMaquettes').className = 'etat erreur';
  }
}
```

- [ ] **Step 5: Le faux DOM apprend `showModal` et `close`**

Dans `app/tests/dom_shim.js`, dans la classe `El`, à côté des autres méthodes :

```js
  /**
   * Le `<dialog>` du dialogue des maquettes. `open` est l'attribut que le vrai DOM
   * pose : les tests s'en servent pour dire si la boîte est ouverte, sans singer la
   * pile de modales ni le piège à focus, dont l'application ne dépend pas.
   */
  showModal() {
    this.open = true;
  }

  close() {
    this.open = false;
  }
```

- [ ] **Step 6: Le style du dialogue, dans `styles.css`**

À la fin de la feuille, en suivant les variables déjà déclarées en tête du fichier (les
relever plutôt que d'écrire des couleurs en dur) :

```css
/* Le dialogue des maquettes. `::backdrop` est ce que la primitive donne gratuitement :
   le fond assombri qui dit que la fenêtre est en attente. */
.dialogue {
  border: 1px solid var(--bord);
  border-radius: 6px;
  padding: 1rem 1.2rem;
  min-width: 32rem;
  max-width: 44rem;
  background: var(--fond);
  color: var(--encre);
}

.dialogue::backdrop {
  background: rgb(0 0 0 / 35%);
}

.dialogue h2 {
  margin: 0 0 0.4rem;
  font-size: 1.05rem;
}

.dialogue .ligne {
  display: flex;
  gap: 0.5rem;
  align-items: center;
  margin-top: 0.8rem;
}

.dialogue .ligne.fin {
  justify-content: flex-end;
}

.dialogue input[type='text'] {
  flex: 1;
}
```

Si `--bord`, `--fond` ou `--encre` n'existent pas sous ces noms, prendre ceux du fichier
— les relever avec `grep -n '^\s*--' app/src/styles.css | head -30` et n'en inventer
aucun.

- [ ] **Step 7: Lancer les tests et les voir passer**

```bash
cd app && node --test tests/*.test.js 2>&1 | tail -8
```

Attendu : `fail 0`, et le compte de tests en hausse de trois.

- [ ] **Step 8: Vérifier et commiter**

```bash
cd app/src-tauri && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test 2>&1 | tail -3
cd /Users/jean-paulgavini/Documents/Dev/ozalid
git add app/src app/tests
git commit -m "$(cat <<'EOF'
La couverture réglée s'enregistre depuis un dialogue

Premier <dialog> de ce front : la primitive standard gère seule le focus et
Échap. Il ne porte qu'un geste — le lot 3 y ajoutera la liste et les trois
autres. Le menu se remplit désormais par une fonction, rappelée après chaque
geste, et range les personnalisées derrière un séparateur inerte.

Claude-Session: https://claude.ai/code/session_01AyXS78gurZmV9m6yFQ8LwL
EOF
)"
```

---

## Tâche 6 : le témoin, la fenêtre, le README

**Files:**
- Modify: `app/README.md`

- [ ] **Step 1: Le témoin**

```bash
cd app/src-tauri && cargo run --example temoin 2>&1 | tail -3
```

Attendu : **98 pages, dos 7,21 mm**. Rien de ce lot ne touche la composition, donc tout
écart est un défaut à comprendre avant d'aller plus loin.

- [ ] **Step 2: À l'œil, dans la fenêtre**

C'est la vérification que le § 7 de la spec réclame, et qu'aucun test ne fait :

```bash
cd app/src-tauri && cargo tauri dev
```

1. Ouvrir un projet réel, aller à l'étape Couverture, régler quelque chose de visible.
2. « Maquettes… » → saisir un nom → « Enregistrer ». Le nom paraît au menu, sous le
   séparateur.
3. Vérifier le fichier : `ls ~/Library/Application\ Support/*ozalid*/maquettes/`.
4. Réessayer le même nom : le refus se lit **dans le dialogue**, pas dans l'alerte.
5. Ouvrir un **autre** projet, charger la maquette enregistrée : la mise en page doit
   être identique, les images posées, et le titre, l'auteur, l'éditeur et la collection
   **du nouveau livre** intacts.
6. Charger ensuite Blanche (qui ne porte aucune image) : la photo du livre doit rester.

Si la fenêtre s'ouvre entièrement blanche après un `touch lib.rs` + `cargo build`, ce
n'est ni le CSS ni le JS : `cargo clean -p ozalid-studio` puis `cargo build`.

- [ ] **Step 3: Le README**

Dans `app/README.md`, section « Le fichier .maquette », après le paragraphe sur les
fournies :

```markdown
Les **personnalisées** vivent dans `<config>/maquettes/`, à côté de `preferences.toml` :
elles appartiennent à la machine, non au livre — un `.ozalid` reste auto-portant, sa
couverture étant dans l'archive, et une maquette n'est qu'un point de départ. Le nom
saisi est l'identité ; le slug qui en dérive — accents décapés, casse ignorée, le reste
en tirets — nomme le fichier et sert de clé. Deux noms qui donnent le même slug sont le
même nom : l'écriture refuse et dit qui tient la place, fournies comprises.

Une maquette emporte **tout**, cadrage et images compris, et charger une maquette pose
ses images à la place de celles du projet, rôle par rôle : une maquette qui ne porte pas
de photo de 1ère laisse celle du livre où elle est. La discipline — des images neutres,
un résumé de 4ème en jetons — appartient à l'utilisateur : filtrer demanderait au code de
deviner ce qui est générique, et il devinerait mal. Rien ne borne le nombre ni le poids
des maquettes ; le répertoire se regarde et s'élague à la main.
```

Et dans le tableau des modules, la ligne `maquettes` devient :

```markdown
| `maquettes` | Le format `.maquette`, les fournies embarquées, les personnalisées du poste, et le slug |
```

- [ ] **Step 4: Commiter**

```bash
cd /Users/jean-paulgavini/Documents/Dev/ozalid
git add app/README.md
git commit -m "$(cat <<'EOF'
Le README dit où vivent les maquettes du poste

Témoin relevé : 98 pages, dos 7,21 mm, inchangé.

Claude-Session: https://claude.ai/code/session_01AyXS78gurZmV9m6yFQ8LwL
EOF
)"
```

---

## Ce que ce lot ne fait pas

- **Cloner, renommer, effacer** — lot 3, avec la liste dans le dialogue. Le refus côté
  Rust sur une fournie n'existe donc pas encore : rien ne peut encore le demander.
- **`fournie` n'est lu par personne dans la fenêtre**, sinon pour placer le séparateur.
  Il porte au lot 3 les boutons que le dialogue n'offre pas sur une fournie.
- **Aucun quota, aucun plafond.** Le poids d'une maquette est celui de ses photos ; le
  dire dans le README vaut mieux qu'un plafond arbitraire (§ 6 de la spec).
