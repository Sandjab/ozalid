# Maquettes en fichiers — lot 3 : le dialogue

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Le dialogue des maquettes liste ce qu'on a, et porte les trois gestes qui manquent — cloner depuis n'importe laquelle, renommer et effacer les siennes — le refus sur une fournie étant tenu par le Rust, non par l'absence des boutons.

**Architecture:** Trois fonctions de module qui partagent un même garde-fou (`personnalisee`, qui refuse une fournie et une clé inconnue) et un même écrivain (`poser`, sans contrôle d'unicité, celui-ci étant fait par les appelants qui savent ce qu'ils remplacent). Trois commandes par-dessus. Puis le dialogue : la liste se construit sans `innerHTML`, le renommage s'y fait en place, et l'effacement demande confirmation sur son propre bouton plutôt que d'ouvrir une seconde modale.

**Tech Stack:** Rust 2021, `serde`, `toml`, `zip 7`, `tempfile` en dev ; front vanilla, `node --test`.

Spec : `docs/superpowers/specs/2026-08-23-maquettes-en-fichiers-design.md`, lot 3 du § 8.
Lots précédents : `…-maquettes-lot-1-archives-fournies.md`, `…-maquettes-lot-2-personnalisees.md`.

---

## Trois décisions prises en écrivant ce plan

**1. `maquette_cloner(cle)` ne prend pas de nom.** La spec écrit
`maquette_cloner(cle, nom)` et « cloner Folio propose *Folio (copie)* ». Faire saisir ce
nom demanderait au dialogue un mode — un champ qui veut dire tantôt « enregistrer »,
tantôt « cloner ceci » — pour un nom que l'utilisateur validerait tel quel neuf fois sur
dix. Le Rust le calcule donc lui-même, et **suffixe s'il est pris** : « Folio (copie) »,
puis « Folio (copie) 2 ». C'est déjà la convention du dépôt pour un nom fabriqué par le
code (`envoi::distinct`), et Renommer est à deux centimètres pour la dixième fois.

**2. Effacer demande confirmation sur son propre bouton.** La spec dessine un simple
`[✗]`. Mais une maquette effacée est du travail perdu sans reprise possible, et le bouton
est à quelques pixels de « Renommer ». Le libellé passe donc à « Confirmer » pour un
second clic, et revient au premier clic ailleurs. Deux lignes de JS, aucune modale
imbriquée — un `<dialog>` dans un `<dialog>` pour trois mots serait disproportionné.

**3. Le renommage se fait en place, et se valide par Entrée.** Le nom de la ligne devient
un champ ; perdre le focus annule. **Échap n'y est pas intercepté** : dans un `<dialog>`,
il ferme la boîte, et le détourner priverait l'utilisateur du geste qu'il connaît. Annuler
un renommage se fait donc en cliquant ailleurs.

---

## Tâche 1 : renommer et effacer, refusés sur une fournie

**Files:**
- Modify: `app/src-tauri/src/maquettes.rs`

- [ ] **Step 1: Écrire les tests**

Dans le module `tests` de `app/src-tauri/src/maquettes.rs` :

```rust
    /// Le refus côté Rust n'est pas une redondance de l'interface qui masque les
    /// boutons : c'est la **seule** garantie réelle de l'immuabilité des fournies.
    /// L'interface n'est qu'une politesse, et une commande s'appelle sans elle.
    #[test]
    fn une_fournie_ne_se_renomme_ni_ne_s_efface() {
        let dir = tempfile::tempdir().unwrap();

        let e = renommer(dir.path(), "folio", "Ma folio").unwrap_err();
        assert!(e.contains("fournie"), "{e}");
        let e = effacer(dir.path(), "folio").unwrap_err();
        assert!(e.contains("fournie"), "{e}");

        assert!(par_cle(Some(dir.path()), "folio").is_some(), "Folio doit tenir");
    }

    /// Renommer déplace le fichier, puisque le slug le nomme — et la maquette garde
    /// tout ce qu'elle emportait. Ce qui se perdrait ici serait une couverture entière.
    #[test]
    fn renommer_deplace_le_fichier_et_garde_le_contenu() {
        let dir = tempfile::tempdir().unwrap();
        let mut images = BTreeMap::new();
        images.insert("couverture.jpg".to_string(), vec![1, 2, 3]);
        let cv = fournie("surimpression");
        ecrire(dir.path(), "Ma collection", &cv, &images).unwrap();

        renommer(dir.path(), "ma-collection", "Nuit blanche").unwrap();

        assert!(par_cle(Some(dir.path()), "ma-collection").is_none(), "l'ancien fichier tient encore");
        let m = par_cle(Some(dir.path()), "nuit-blanche").unwrap();
        assert_eq!(m.nom, "Nuit blanche");
        assert_eq!(m.couverture, cv);
        assert_eq!(m.images, images);
    }

    /// Corriger la casse ou la ponctuation d'un nom garde le même slug : la maquette ne
    /// doit pas s'y voir refuser son propre nom, ni disparaître dans l'opération.
    #[test]
    fn se_renommer_sous_le_meme_slug_est_permis() {
        let dir = tempfile::tempdir().unwrap();
        ecrire(dir.path(), "ma collection", &fournie("folio"), &BTreeMap::new()).unwrap();

        renommer(dir.path(), "ma-collection", "Ma Collection !").unwrap();

        let m = par_cle(Some(dir.path()), "ma-collection").unwrap();
        assert_eq!(m.nom, "Ma Collection !");
        assert_eq!(
            toutes(Some(dir.path())).iter().filter(|m| !m.fournie).count(),
            1,
            "le renommage a dédoublé la maquette"
        );
    }

    /// L'unicité vaut au renommage comme à l'écriture : deux maquettes de même clé
    /// rendraient la seconde inatteignable, et le renommage écraserait la première.
    #[test]
    fn renommer_vers_un_nom_pris_est_refuse() {
        let dir = tempfile::tempdir().unwrap();
        ecrire(dir.path(), "Ma collection", &fournie("folio"), &BTreeMap::new()).unwrap();
        ecrire(dir.path(), "Nuit blanche", &fournie("blanche"), &BTreeMap::new()).unwrap();

        let e = renommer(dir.path(), "nuit-blanche", "MA COLLECTION").unwrap_err();
        assert!(e.contains("Ma collection"), "{e}");

        let m = par_cle(Some(dir.path()), "nuit-blanche").unwrap();
        assert_eq!(m.nom, "Nuit blanche", "le refus a quand même renommé");
    }

    /// Effacer retire le fichier, et rien d'autre.
    #[test]
    fn effacer_retire_la_maquette_et_laisse_les_autres() {
        let dir = tempfile::tempdir().unwrap();
        ecrire(dir.path(), "Ma collection", &fournie("folio"), &BTreeMap::new()).unwrap();
        ecrire(dir.path(), "Nuit blanche", &fournie("blanche"), &BTreeMap::new()).unwrap();

        effacer(dir.path(), "ma-collection").unwrap();

        assert!(par_cle(Some(dir.path()), "ma-collection").is_none());
        assert!(par_cle(Some(dir.path()), "nuit-blanche").is_some());
        assert_eq!(toutes(Some(dir.path())).len(), 4, "fournies comprises");
    }

    /// Une clé qu'aucune maquette ne porte : le geste vient d'une liste périmée, et le
    /// dire vaut mieux que de laisser croire à un effacement qui n'a rien effacé.
    #[test]
    fn une_cle_inconnue_est_refusee_avant_toute_ecriture() {
        let dir = tempfile::tempdir().unwrap();
        let e = effacer(dir.path(), "jamais-vue").unwrap_err();
        assert!(e.contains("jamais-vue"), "{e}");
    }
```

- [ ] **Step 2: Lancer les tests et les voir échouer**

```bash
cd app/src-tauri && cargo test --lib maquettes 2>&1 | tail -12
```

Attendu : la compilation échoue — `cannot find function renommer`, `cannot find function
effacer`.

- [ ] **Step 3: Factoriser, puis écrire les deux fonctions**

Dans `app/src-tauri/src/maquettes.rs`, remplacer `ecrire` par cette suite (le corps qui
écrit sort dans `poser`, et le contrôle d'unicité dans `deja_prise` — le renommage a
besoin des deux, mais pas du même contrôle) :

```rust
/// Le chemin du fichier d'une personnalisée.
fn chemin(config: &Path, cle: &str) -> PathBuf {
    repertoire(config).join(format!("{cle}.{EXT}"))
}

/// Le nom de la maquette qui tient déjà cette clé — `soi` exceptée, pour qu'un
/// renommage ne se refuse pas à lui-même quand le slug ne change pas.
fn deja_prise(config: &Path, cle: &str, soi: Option<&str>) -> Option<String> {
    toutes(Some(config))
        .into_iter()
        .find(|m| m.cle == cle && Some(m.cle.as_str()) != soi)
        .map(|m| m.nom)
}

/// La personnalisée de cette clé, ou le refus qui dit pourquoi.
///
/// C'est ici que l'immuabilité des fournies est **réellement** tenue. L'interface qui
/// n'offre pas les boutons est une politesse : une commande s'appelle sans elle, et une
/// liste périmée nomme des clés qui n'existent plus.
fn personnalisee(config: &Path, cle: &str) -> Result<Maquette, String> {
    let m = par_cle(Some(config), cle).ok_or_else(|| format!("maquette inconnue : {cle}"))?;
    if m.fournie {
        return Err(format!(
            "« {} » est une maquette fournie : elle ne se renomme ni ne s'efface.",
            m.nom
        ));
    }
    Ok(m)
}

/// Écrit le fichier d'une personnalisée, sans rien contrôler : c'est l'appelant qui
/// sait s'il crée ou s'il remplace.
fn poser(
    config: &Path,
    cle: &str,
    nom: &str,
    couverture: &Couverture,
    images: &BTreeMap<String, Vec<u8>>,
) -> Result<(), String> {
    let dir = repertoire(config);
    std::fs::create_dir_all(&dir).map_err(|e| {
        format!(
            "répertoire des maquettes inutilisable ({}) : {e}",
            dir.display()
        )
    })?;
    let chemin = chemin(config, cle);
    let f = std::fs::File::create(&chemin)
        .map_err(|e| format!("écriture de {} : {e}", chemin.display()))?;
    ecrire_archive(
        f,
        &Maquette {
            cle: cle.into(),
            nom: nom.into(),
            fournie: false,
            couverture: couverture.clone(),
            images: images.clone(),
        },
    )
}

/// Le slug d'un nom saisi, ou le refus qui dit quoi faire.
fn slug_saisi(nom: &str) -> Result<String, String> {
    slug(nom).ok_or_else(|| {
        format!(
            "« {nom} » ne peut pas nommer une maquette : il y faut au moins une lettre ou un chiffre."
        )
    })
}
```

`ecrire` garde son doc-comment et devient :

```rust
pub fn ecrire(
    config: &Path,
    nom: &str,
    couverture: &Couverture,
    images: &BTreeMap<String, Vec<u8>>,
) -> Result<(), String> {
    let cle = slug_saisi(nom)?;
    if let Some(prise) = deja_prise(config, &cle, None) {
        return Err(format!("« {prise} » porte déjà ce nom."));
    }
    poser(config, &cle, nom, couverture, images)
}
```

Puis, à la suite :

```rust
/// Renomme une personnalisée. Le slug nommant le fichier, le renommage le déplace —
/// sauf quand seule la casse ou la ponctuation change, auquel cas le fichier est
/// réécrit en place et la maquette ne se refuse pas son propre nom.
pub fn renommer(config: &Path, cle: &str, nom: &str) -> Result<(), String> {
    let m = personnalisee(config, cle)?;
    let neuf = slug_saisi(nom)?;
    if let Some(prise) = deja_prise(config, &neuf, Some(cle)) {
        return Err(format!("« {prise} » porte déjà ce nom."));
    }
    poser(config, &neuf, nom, &m.couverture, &m.images)?;
    if neuf != cle {
        let ancien = chemin(config, cle);
        std::fs::remove_file(&ancien)
            .map_err(|e| format!("l'ancien fichier tient encore ({}) : {e}", ancien.display()))?;
    }
    Ok(())
}

/// Efface une personnalisée. Sans reprise : ce que le fichier portait est perdu, et
/// c'est la fenêtre qui demande confirmation.
pub fn effacer(config: &Path, cle: &str) -> Result<(), String> {
    personnalisee(config, cle)?;
    let chemin = chemin(config, cle);
    std::fs::remove_file(&chemin).map_err(|e| format!("effacement de {} : {e}", chemin.display()))
}
```

- [ ] **Step 4: Lancer les tests et les voir passer**

```bash
cd app/src-tauri && cargo test --lib maquettes 2>&1 | tail -8
```

Attendu : `test result: ok.`

- [ ] **Step 5: Muter pour vérifier que les tests mordent**

Les six tests n'ont été rouges que par une erreur de compilation, ce qui ne prouve rien.
Trois mutations, à jouer et à défaire une par une :

```bash
cd app/src-tauri
# 1. Le garde-fou des fournies saute.
python3 - <<'FIN'
p = 'src/maquettes.rs'
s = open(p, encoding='utf-8').read()
s = s.replace('    if m.fournie {\n        return Err(format!(\n            "« {} » est une maquette fournie : elle ne se renomme ni ne s\'efface.",\n            m.nom\n        ));\n    }\n', '')
open(p, 'w', encoding='utf-8').write(s)
FIN
cargo test --lib maquettes 2>&1 | grep -E "(FAILED|test result)"
```

Attendu : `une_fournie_ne_se_renomme_ni_ne_s_efface` **échoue**. Défaire (`git checkout
app/src-tauri/src/maquettes.rs` perdrait le travail non commité : rétablir à la main, ou
travailler sur une copie). Puis :

```bash
# 2. Le `soi` du renommage disparaît : une maquette se refuse son propre nom.
#    deja_prise(config, &neuf, Some(cle))  ->  deja_prise(config, &neuf, None)
# Attendu : `se_renommer_sous_le_meme_slug_est_permis` échoue.

# 3. L'ancien fichier n'est plus retiré : le `if neuf != cle { … }` est commenté.
# Attendu : `renommer_deplace_le_fichier_et_garde_le_contenu` échoue.
```

Chacune doit faire échouer **son** test et lui seul. Tout rétablir avant de commiter, et
vérifier que la suite est de nouveau verte.

- [ ] **Step 6: Vérifier et commiter**

```bash
cd app/src-tauri
cargo fmt --check; echo "fmt=$?"
cargo clippy --all-targets -- -D warnings > /dev/null 2>&1; echo "clippy=$?"
cargo test 2>&1 | grep "test result: ok" | head -1
```

Les trois lignes doivent dire `0`, `0` et `ok` — **ne jamais mettre clippy dans un pipe**,
le shell rendrait le statut du dernier maillon et un échec passerait inaperçu.

```bash
cd /Users/jean-paulgavini/Documents/Dev/ozalid
git add app/src-tauri/src/maquettes.rs
git commit -m "$(cat <<'EOF'
Une maquette du poste se renomme et s'efface

Le refus sur une fournie est tenu par le Rust : l'interface qui n'offre pas les
boutons est une politesse, pas une garantie. Renommer déplace le fichier, sauf
quand seule la casse change — une maquette ne doit pas se refuser son propre nom.

Claude-Session: https://claude.ai/code/session_01AyXS78gurZmV9m6yFQ8LwL
EOF
)"
```

---

## Tâche 2 : le nom d'un clone

**Files:**
- Modify: `app/src-tauri/src/maquettes.rs`

- [ ] **Step 1: Écrire les tests**

```rust
    /// Cloner est un geste, pas une saisie : le nom du clone est fabriqué par le code,
    /// et un nom fabriqué se suffixe plutôt que de se faire refuser — c'est déjà la
    /// convention du dépôt pour les envois (`envoi::distinct`). Renommer est à côté
    /// pour la fois où « (copie) » ne convient pas.
    #[test]
    fn le_nom_d_un_clone_s_ecarte_de_ce_qui_est_pris() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(nom_de_copie(Some(dir.path()), "Folio"), "Folio (copie)");

        ecrire(dir.path(), "Folio (copie)", &fournie("folio"), &BTreeMap::new()).unwrap();
        assert_eq!(nom_de_copie(Some(dir.path()), "Folio"), "Folio (copie) 2");

        ecrire(dir.path(), "Folio (copie) 2", &fournie("folio"), &BTreeMap::new()).unwrap();
        assert_eq!(nom_de_copie(Some(dir.path()), "Folio"), "Folio (copie) 3");
    }

    /// Cloner un clone ne fait pas « Folio (copie) (copie) (copie) » : le suffixe se
    /// remplace, il ne s'empile pas.
    #[test]
    fn cloner_un_clone_ne_reempile_pas_le_suffixe() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(nom_de_copie(Some(dir.path()), "Folio (copie)"), "Folio (copie) 2");
        assert_eq!(nom_de_copie(Some(dir.path()), "Folio (copie) 7"), "Folio (copie)");
    }
```

- [ ] **Step 2: Lancer les tests et les voir échouer**

```bash
cd app/src-tauri && cargo test --lib maquettes::tests::le_nom_d_un_clone 2>&1 | tail -8
```

Attendu : `cannot find function nom_de_copie`.

- [ ] **Step 3: Implanter**

```rust
/// Ce que porte un nom de clone, sans son suffixe de copie.
const COPIE: &str = " (copie)";

/// Un nom libre pour le clone de `nom` : « Folio (copie) », puis « Folio (copie) 2 ».
///
/// Le suffixe se **remplace** plutôt que de s'empiler : cloner un clone donne un
/// deuxième clone, non « Folio (copie) (copie) ». Le rang monte tant que la place est
/// prise — un nom fabriqué par le code n'a pas à se faire refuser, là où un nom saisi,
/// lui, est refusé pour que l'utilisateur sache que le sien existait déjà.
pub fn nom_de_copie(config: Option<&Path>, nom: &str) -> String {
    let souche = match nom.split_once(COPIE) {
        Some((avant, _)) => avant,
        None => nom,
    };
    let pris = |candidat: &str| {
        slug(candidat).is_some_and(|c| toutes(config).iter().any(|m| m.cle == c))
    };
    let premier = format!("{souche}{COPIE}");
    if !pris(&premier) {
        return premier;
    }
    (2..)
        .map(|n| format!("{souche}{COPIE} {n}"))
        .find(|c| !pris(c))
        .expect("la suite des entiers ne s'épuise pas")
}
```

- [ ] **Step 4: Lancer les tests et les voir passer**

```bash
cd app/src-tauri && cargo test --lib maquettes 2>&1 | tail -6
```

Attendu : `test result: ok.`

- [ ] **Step 5: Vérifier et commiter**

```bash
cd app/src-tauri
cargo fmt --check; echo "fmt=$?"
cargo clippy --all-targets -- -D warnings > /dev/null 2>&1; echo "clippy=$?"
cargo test 2>&1 | grep "test result: ok" | head -1
cd /Users/jean-paulgavini/Documents/Dev/ozalid
git add app/src-tauri/src/maquettes.rs
git commit -m "$(cat <<'EOF'
Un clone se nomme tout seul

« Folio (copie) », puis « Folio (copie) 2 » : un nom fabriqué par le code se
suffixe, là où un nom saisi se fait refuser. Cloner un clone ne réempile pas le
suffixe.

Claude-Session: https://claude.ai/code/session_01AyXS78gurZmV9m6yFQ8LwL
EOF
)"
```

---

## Tâche 3 : les trois commandes

**Files:**
- Modify: `app/src-tauri/src/commands.rs`
- Modify: `app/src-tauri/src/lib.rs`

- [ ] **Step 1: Écrire les commandes**

Dans `app/src-tauri/src/commands.rs`, après `maquette_enregistrer` :

```rust
/// Clone une maquette, fournie ou non, sous un nom que le Rust fabrique.
///
/// Aucun nom n'est demandé : « Folio (copie) » convient neuf fois sur dix, et
/// « Renommer » est à côté pour la dixième. Un nom saisi ici aurait obligé le dialogue
/// à se donner un mode.
#[tauri::command]
pub fn maquette_cloner(cle: String, app: tauri::AppHandle) -> Result<(), String> {
    let dir = config(&app).ok_or("répertoire de configuration introuvable.")?;
    let m = maquettes::par_cle(Some(&dir), &cle)
        .ok_or_else(|| format!("maquette inconnue : {cle}"))?;
    let nom = maquettes::nom_de_copie(Some(&dir), &m.nom);
    maquettes::ecrire(&dir, &nom, &m.couverture, &m.images)
}

/// Renomme une personnalisée. Le refus sur une fournie est dans `maquettes`, pas ici :
/// c'est lui la garantie, l'interface ne fait que ne pas offrir le bouton.
#[tauri::command]
pub fn maquette_renommer(cle: String, nom: String, app: tauri::AppHandle) -> Result<(), String> {
    let dir = config(&app).ok_or("répertoire de configuration introuvable.")?;
    maquettes::renommer(&dir, &cle, &nom)
}

#[tauri::command]
pub fn maquette_effacer(cle: String, app: tauri::AppHandle) -> Result<(), String> {
    let dir = config(&app).ok_or("répertoire de configuration introuvable.")?;
    maquettes::effacer(&dir, &cle)
}
```

- [ ] **Step 2: Déclarer les trois**

Dans `app/src-tauri/src/lib.rs`, à la suite de `commands::maquette_enregistrer` :

```rust
            commands::maquette_cloner,
            commands::maquette_renommer,
            commands::maquette_effacer,
```

- [ ] **Step 3: Compiler et vérifier**

```bash
cd app/src-tauri
cargo fmt --check; echo "fmt=$?"
cargo clippy --all-targets -- -D warnings > /dev/null 2>&1; echo "clippy=$?"
cargo test 2>&1 | grep "test result: ok" | head -1
```

- [ ] **Step 4: Commiter**

```bash
cd /Users/jean-paulgavini/Documents/Dev/ozalid
git add app/src-tauri/src/commands.rs app/src-tauri/src/lib.rs
git commit -m "$(cat <<'EOF'
Trois gestes de plus sur les maquettes

Cloner depuis n'importe laquelle, renommer et effacer les siennes. Le clonage ne
demande pas de nom : le Rust le fabrique, et le dialogue s'épargne un mode.

Claude-Session: https://claude.ai/code/session_01AyXS78gurZmV9m6yFQ8LwL
EOF
)"
```

---

## Tâche 4 : la liste dans le dialogue

**Files:**
- Modify: `app/src/index.html`
- Modify: `app/src/couverture.js`
- Modify: `app/src/styles.css`
- Modify: `app/tests/couverture.test.js`

- [ ] **Step 1: Écrire les tests du front**

Dans `app/tests/couverture.test.js`, à la suite des trois tests du lot 2. L'aide qui
suit lit la liste construite dans le dialogue ; la poser avec les autres aides du
fichier :

```js
/** Ce que le dialogue montre : une ligne par maquette, avec ses boutons. */
const lignesMaquettes = (els) => [...els.get('listeMaquettes').children].map((l) => {
  const enfants = [...l.children];
  return {
    nom: enfants[0].textContent,
    gestes: enfants.filter((e) => e.tagName === 'BUTTON').map((b) => b.textContent),
  };
});
```

```js
/**
 * L'interface ne propose pas ce que le Rust refuserait : une fournie se clone, elle ne
 * se renomme ni ne s'efface. C'est une politesse — la garantie, elle, est dans
 * `maquettes::personnalisee`, et un test Rust la tient.
 */
test('le dialogue n\'offre ni Renommer ni Effacer sur une fournie', async () => {
  const { els } = await ouvre(maquette(), { maquettes_liste: AVEC_PERSONNALISEE });
  await els.get('btMaquettes').declenche('click');
  assert.deepEqual(lignesMaquettes(els), [
    { nom: 'Folio', gestes: ['Cloner'] },
    { nom: 'Blanche', gestes: ['Cloner'] },
    { nom: 'Ma collection', gestes: ['Cloner', 'Renommer', 'Effacer'] },
  ]);
});

/**
 * Cloner ne demande rien : un clic, une commande, et la liste se refait — sans quoi le
 * clone manquerait à la liste d'où on vient de le tirer.
 */
test('cloner une fournie demande le clonage et rafraîchit la liste', async () => {
  const clones = [];
  const { els } = await ouvre(maquette(), {
    maquette_cloner: ({ cle }) => { clones.push(cle); return null; },
    maquettes_liste: () => [
      { cle: 'folio', libelle: 'Folio', fournie: true },
      ...clones.map((c) => ({ cle: `${c}-copie`, libelle: `Folio (copie)`, fournie: false })),
    ],
  });
  await els.get('btMaquettes').declenche('click');
  await bouton(els, 'Folio', 'Cloner').declenche('click');

  assert.deepEqual(clones, ['folio']);
  assert.deepEqual(lignesMaquettes(els).map((l) => l.nom), ['Folio', 'Folio (copie)']);
});

/**
 * Renommer se fait en place : le nom devient un champ, Entrée valide. Ce qui part au
 * Rust est la clé de la ligne et le texte saisi — la clé, et non le nom d'avant, parce
 * que c'est elle qui nomme le fichier.
 */
test('renommer en place envoie la clé et le nouveau nom', async () => {
  const renommees = [];
  const { els } = await ouvre(maquette(), {
    maquette_renommer: (a) => { renommees.push(a); return null; },
    maquettes_liste: AVEC_PERSONNALISEE,
  });
  await els.get('btMaquettes').declenche('click');
  await bouton(els, 'Ma collection', 'Renommer').declenche('click');

  const champ = [...els.get('listeMaquettes').children]
    .flatMap((l) => [...l.children])
    .find((e) => e.tagName === 'INPUT');
  assert.strictEqual(champ.value, 'Ma collection', 'le champ part du nom courant');
  champ.value = 'Nuit blanche';
  await champ.declenche('keydown', { key: 'Enter' });

  assert.deepEqual(renommees, [{ cle: 'ma-collection', nom: 'Nuit blanche' }]);
});

/**
 * Effacer perd du travail sans reprise, et le bouton est à quelques pixels de
 * « Renommer » : le premier clic demande confirmation, le second efface.
 */
test('effacer demande confirmation avant de perdre la maquette', async () => {
  const effacees = [];
  const { els } = await ouvre(maquette(), {
    maquette_effacer: ({ cle }) => { effacees.push(cle); return null; },
    maquettes_liste: AVEC_PERSONNALISEE,
  });
  await els.get('btMaquettes').declenche('click');

  await bouton(els, 'Ma collection', 'Effacer').declenche('click');
  assert.deepEqual(effacees, [], 'le premier clic ne doit rien effacer');
  await bouton(els, 'Ma collection', 'Confirmer').declenche('click');
  assert.deepEqual(effacees, ['ma-collection']);
});

/**
 * Un refus du Rust — une fournie qu'on aurait quand même demandé d'effacer, une liste
 * périmée — se lit dans le dialogue, comme les refus d'enregistrement.
 */
test('un refus de geste se lit dans le dialogue', async () => {
  const { els } = await ouvre(maquette(), {
    maquette_cloner: () => { throw new Error('maquette inconnue : folio'); },
    maquettes_liste: AVEC_PERSONNALISEE,
  });
  await els.get('btMaquettes').declenche('click');
  await bouton(els, 'Folio', 'Cloner').declenche('click');
  assert.match(els.get('etatMaquettes').textContent, /maquette inconnue/);
});
```

et l'aide qui trouve un bouton dans une ligne, à poser avec `lignesMaquettes` :

```js
/** Le bouton `geste` de la ligne de `nom`, dans le dialogue des maquettes. */
const bouton = (els, nom, geste) => {
  const ligne = [...els.get('listeMaquettes').children]
    .find((l) => [...l.children].some((e) => e.textContent === nom || e.value === nom));
  assert.ok(ligne, `aucune ligne « ${nom} »`);
  const b = [...ligne.children].find((e) => e.tagName === 'BUTTON' && e.textContent === geste);
  assert.ok(b, `« ${nom} » n'offre pas « ${geste} »`);
  return b;
};
```

- [ ] **Step 2: Lancer les tests et les voir échouer**

```bash
cd app && node --test tests/couverture.test.js 2>&1 | grep -E "^ℹ (pass|fail)"
```

Attendu : `fail 5` — `listeMaquettes` n'existe pas.

- [ ] **Step 3: La liste dans `index.html`**

Dans le `<dialog id="dlgMaquettes">`, entre le `<p class="note">` et la ligne
d'enregistrement :

```html
      <div id="listeMaquettes" class="maquettes"></div>
```

- [ ] **Step 4: Construire la liste, dans `couverture.js`**

`remplirMaquettes` devient `rafraichirMaquettes` : une seule commande, deux vues — le
menu de la barre et la liste du dialogue. Remplacer la fonction du lot 2 par :

```js
/**
 * (Re)construit le menu de la barre **et** la liste du dialogue, d'un seul appel.
 *
 * La liste vit dans le Rust, qui relit le répertoire de configuration à chaque appel ;
 * la tenir à jour ici la dédoublerait. Rappelée après chaque geste, sans quoi ce qu'on
 * vient de cloner manquerait à la liste d'où on l'a tiré.
 */
async function rafraichirMaquettes() {
  const maquettes = await invoke('maquettes_liste');

  const sel = $('inMaquette');
  sel.replaceChildren();
  sel.append(new Option('Repartir d\'une maquette…', ''));
  let separateur = false;
  for (const m of maquettes) {
    if (!m.fournie && !separateur) {
      const trait = new Option('──────────', '');
      trait.disabled = true;
      sel.append(trait);
      separateur = true;
    }
    sel.append(new Option(m.libelle, m.cle));
  }
  sel.value = '';

  const liste = $('listeMaquettes');
  liste.replaceChildren();
  for (const m of maquettes) liste.append(ligneMaquette(m));
}

/**
 * Une ligne de la liste : le nom, ce qu'elle est, et ses gestes.
 *
 * Sans `innerHTML` — le nom d'une maquette vient d'un fichier qu'on n'a pas écrit.
 * Une fournie n'offre que Cloner : c'est une politesse, la garantie est dans le Rust,
 * qui refuse de renommer et d'effacer ce qui est livré avec lui.
 */
function ligneMaquette(m) {
  const ligne = document.createElement('div');
  ligne.className = 'ligne maquette';

  const nom = document.createElement('span');
  nom.className = 'nom';
  nom.textContent = m.libelle;
  ligne.append(nom);

  if (m.fournie) {
    const dit = document.createElement('span');
    dit.className = 'note';
    dit.textContent = 'fournie';
    ligne.append(dit);
  }

  ligne.append(geste('Cloner', () => invoke('maquette_cloner', { cle: m.cle })));
  if (!m.fournie) {
    ligne.append(gesteRenommer(m, ligne, nom));
    ligne.append(gesteEffacer(m));
  }
  return ligne;
}

/** Un bouton du dialogue : il agit, rend compte, et refait la liste. */
function geste(libelle, action) {
  const b = document.createElement('button');
  b.type = 'button';
  b.textContent = libelle;
  b.addEventListener('click', async () => {
    try {
      await action();
      $('etatMaquettes').textContent = '';
      $('etatMaquettes').className = 'etat';
      await rafraichirMaquettes();
    } catch (e) {
      $('etatMaquettes').textContent = String(e);
      $('etatMaquettes').className = 'etat erreur';
    }
  });
  return b;
}

/**
 * Renommer, en place : le nom devient un champ, Entrée valide, perdre le focus annule.
 *
 * Échap n'est pas intercepté — dans un `<dialog>` il ferme la boîte, et le détourner
 * priverait l'utilisateur du geste qu'il connaît.
 */
function gesteRenommer(m, ligne, nom) {
  const b = document.createElement('button');
  b.type = 'button';
  b.textContent = 'Renommer';
  b.addEventListener('click', () => {
    const champ = document.createElement('input');
    champ.type = 'text';
    champ.className = 'nom';
    champ.value = m.libelle;
    champ.addEventListener('keydown', async (e) => {
      if (e.key !== 'Enter') return;
      try {
        await invoke('maquette_renommer', { cle: m.cle, nom: champ.value.trim() });
        $('etatMaquettes').textContent = '';
        $('etatMaquettes').className = 'etat';
        await rafraichirMaquettes();
      } catch (err) {
        $('etatMaquettes').textContent = String(err);
        $('etatMaquettes').className = 'etat erreur';
      }
    });
    champ.addEventListener('blur', () => rafraichirMaquettes());
    ligne.replaceChild(champ, nom);
    champ.focus();
  });
  return b;
}

/** Effacer, en deux temps : ce qui se perd ici ne se retrouve pas. */
function gesteEffacer(m) {
  const b = document.createElement('button');
  b.type = 'button';
  b.textContent = 'Effacer';
  b.addEventListener('click', () => {
    // Le premier clic arme, le second efface. Un bouton non attaché ne reçoit pas de
    // clic : ce geste-ci ne peut donc pas déléguer à `geste`, il refait son corps.
    if (b.textContent === 'Effacer') {
      b.textContent = 'Confirmer';
      b.className = 'danger';
      return;
    }
    return rendCompte(() => invoke('maquette_effacer', { cle: m.cle }));
  });
  return b;
}
```

Les quatre gestes — Cloner, Renommer, Effacer, et l'`enregistrerMaquette` du lot 2 —
partagent leur compte rendu : l'extraire, et l'employer partout.

```js
/**
 * Fait un geste du dialogue, en rend compte, et refait la liste.
 *
 * Le compte rendu se lit **dans** le dialogue et non dans l'alerte de la fenêtre :
 * celle-ci est derrière lui, et un refus y passerait inaperçu.
 */
async function rendCompte(action, dit = '') {
  try {
    await action();
    $('etatMaquettes').textContent = dit;
    $('etatMaquettes').className = 'etat';
    await rafraichirMaquettes();
  } catch (e) {
    $('etatMaquettes').textContent = String(e);
    $('etatMaquettes').className = 'etat erreur';
  }
}
```

`geste` et `gesteRenommer` s'écrivent alors avec `rendCompte`, et
`enregistrerMaquette` du lot 2 devient
`rendCompte(() => invoke('maquette_enregistrer', { nom }), \`« ${nom} » enregistrée.\`)`
— avec le vidage du champ à l'intérieur de l'action, pour qu'un refus le laisse rempli.

Enfin, dans `app.js`, remplacer l'appel `await remplirMaquettes();` par
`await rafraichirMaquettes();`, et dans `enregistrerMaquette` de même.

- [ ] **Step 5: Le style des lignes, dans `styles.css`**

À la suite des règles `.dialogue` du lot 2 :

```css
.maquettes { margin-top: .8rem; border-top: 1px solid var(--trait); }
.ligne.maquette { margin: 0; padding: .4rem 0; border-bottom: 1px solid var(--trait); }
.ligne.maquette .nom { flex: 1; }
.ligne.maquette .note { margin: 0; }
.ligne.maquette button.danger { color: var(--rouge); }
```

- [ ] **Step 6: Lancer les tests et les voir passer**

```bash
cd app && node --test tests/*.test.js 2>&1 | grep -E "^ℹ (tests|pass|fail)"
```

Attendu : `fail 0`, cinq tests de plus qu'au lot 2 (197).

- [ ] **Step 7: Vérifier et commiter**

```bash
cd app/src-tauri
cargo fmt --check; echo "fmt=$?"
cargo clippy --all-targets -- -D warnings > /dev/null 2>&1; echo "clippy=$?"
cargo test 2>&1 | grep "test result: ok" | head -1
cd /Users/jean-paulgavini/Documents/Dev/ozalid
git add app/src app/tests
git commit -m "$(cat <<'EOF'
Le dialogue montre les maquettes et ce qu'on peut en faire

Une ligne par maquette : Cloner partout, Renommer et Effacer sur les siennes.
Le renommage se fait en place, l'effacement demande confirmation sur son propre
bouton — ce qui se perd là ne se retrouve pas.

Claude-Session: https://claude.ai/code/session_01AyXS78gurZmV9m6yFQ8LwL
EOF
)"
```

---

## Tâche 5 : le témoin, la fenêtre, le README

**Files:**
- Modify: `app/README.md`

- [ ] **Step 1: Le témoin**

```bash
cd app/src-tauri && cargo run --example temoin 2>&1 | tail -2
```

Attendu : **98 pages, dos 7,21 mm**.

- [ ] **Step 2: À l'œil, dans la fenêtre**

Le front est embarqué à la compilation : `touch src/lib.rs && cargo build` avant de
lancer, sans quoi le binaire garde l'ancien `src/`.

1. Ouvrir un projet, étape Couverture, « Maquettes… ».
2. La liste montre les trois fournies (mention « fournie », un seul bouton) et
   « Café du matin », enregistrée au lot 2, avec ses trois gestes.
3. Cloner Folio : « Folio (copie) » paraît, dans la liste et au menu. Cloner encore :
   « Folio (copie) 2 ».
4. Renommer « Folio (copie) » en « Ma collection » : le nom devient un champ, Entrée
   valide. Vérifier le fichier :
   `ls ~/Library/Application\ Support/cloud.gavini.ozalid/maquettes/`.
5. Effacer les clones : premier clic « Confirmer », second efface. La ligne part.
6. Vérifier qu'aucune fournie n'offre Renommer ni Effacer.

- [ ] **Step 3: Le README**

Dans `app/README.md`, section « Le fichier .maquette », après le paragraphe sur les
personnalisées :

```markdown
Toute maquette se **clone**, fournie comprise — c'est ainsi qu'on part d'une fournie pour
en faire la sienne. Le clone se nomme tout seul (« Folio (copie) », puis
« Folio (copie) 2 ») : un nom fabriqué par le code se suffixe, là où un nom saisi se fait
refuser. Renommer et effacer ne valent que pour les personnalisées, et le **Rust** le
refuse — le dialogue qui n'offre pas ces boutons sur une fournie n'est qu'une politesse,
et une commande s'appelle sans lui.
```

- [ ] **Step 4: Commiter**

```bash
cd /Users/jean-paulgavini/Documents/Dev/ozalid
git add app/README.md
git commit -m "$(cat <<'EOF'
Le README dit ce qu'on fait d'une maquette

Témoin relevé : 98 pages, dos 7,21 mm, inchangé. Le chantier des maquettes en
fichiers est complet.

Claude-Session: https://claude.ai/code/session_01AyXS78gurZmV9m6yFQ8LwL
EOF
)"
```

---

## Ce que ce lot ne fait pas

- **Aucun quota, aucune reprise après effacement.** Une maquette effacée est perdue ; la
  confirmation est tout le filet. Le § 6 de la spec l'assume : l'utilisateur voit ses
  fichiers et les gère.
- **Le dialogue ne montre pas ce qu'une maquette porte** — ni aperçu, ni poids, ni
  nombre d'images. Un nom et des gestes. Si le besoin apparaît, il ouvrira son chantier.
- **Rien ne change au format ni à la composition.** `VERSION` du `.ozalid` ne bouge pas,
  le témoin non plus.
