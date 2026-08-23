# Identité du livre — lot 3 : l'onglet Livre et les génériques

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** L'onglet Livre montre ses champs en deux groupes — les clés, puis les libres avec l'aide qui liste les jetons — et un projet neuf naît avec des valeurs génériques qui rendent toute maquette lisible d'emblée.

**Architecture:** Deux tâches. La première pose les valeurs par défaut des champs préexistants ; la seconde regroupe l'onglet et sert la liste des jetons depuis le Rust, seul endroit qui la connaisse. Rien ne change au format : `VERSION` reste à 3.

**Tech Stack:** Rust 2021, `serde` ; front vanilla, `node --test`.

Spec : `docs/superpowers/specs/2026-08-23-identite-du-livre-design.md`.

---

## Une décision prise en écrivant ce plan

**L'année vient de `epub::horodatage`.** Le projet n'a aucune dépendance de date : il
porte son propre calendrier, l'algorithme de Howard Hinnant dans `epub::civil`, employé
pour horodater les EPUB. Plutôt que d'en dupliquer une seconde implantation ou d'élargir
la visibilité de `civil`, l'année se prend sur les quatre premiers caractères de
`horodatage`, dont le format `AAAA-MM-JJT…` est stable et entièrement ASCII.

---

## Tâche 1 : un projet neuf naît générique

**Files:**
- Modify: `app/src-tauri/src/projet.rs`
- Modify: `app/src-tauri/src/import.rs`
- Modify: `app/src/app.js`

- [ ] **Step 1: Écrire les tests**

Dans le module `tests` de `app/src-tauri/src/projet.rs` :

```rust
    /// Un projet neuf montre la maquette telle qu'elle est : ses champs portent de
    /// vraies valeurs, que le Rust reçoit et que la composition compose partout où la
    /// maquette les montre.
    #[test]
    fn un_livre_neuf_porte_ses_generiques() {
        let l = Livre::vide();
        assert_eq!(l.titre, "Titre");
        assert_eq!(l.auteur, "Auteur");
        assert_eq!(l.genre, "Genre");
        assert_eq!(l.editeur, "Editeur");
        assert_eq!(l.collection, "Collection");
        assert_eq!(l.monogramme, "Monogramme");
        assert_eq!(l.prix, "Prix");
        assert_eq!(l.mention, "Mention");
    }

    /// La dédicace fait exception et naît vide : elle est le seul champ sans
    /// interrupteur, et `interieur.rs` lui compose une belle page et sa blanche dès
    /// qu'elle n'est pas vide. Deux pages de plus sur tout projet neuf, donc un dos plus
    /// épais, que rien à l'écran n'attribuerait à un défaut que personne n'a choisi.
    #[test]
    fn la_dedicace_est_la_seule_a_naitre_vide() {
        assert!(Livre::vide().dedicace.is_empty());
        assert_eq!(Livre::vide().dedicace(), None);
    }

    /// Le copyright cite l'auteur et porte l'année de création — figée, pas un jeton :
    /// un `%ANNEE%` résolu à chaque composition ferait dire 2028 au copyright d'un livre
    /// déposé en 2026, et le dépôt légal ne se rattrape pas.
    #[test]
    fn le_copyright_neuf_cite_l_auteur_et_date_de_cette_annee() {
        let l = Livre::vide();
        assert!(l.copyright.contains("%AUTEUR%"), "{}", l.copyright);
        assert!(l.copyright.contains("Tous droits réservés."));
        assert!(l.copyright.contains("atelier Ozalid"));
        // L'année est écrite, pas citée : elle ne doit pas bouger d'une composition à
        // l'autre.
        assert!(!l.copyright.contains("%ANNEE%"));
        assert_eq!(l.copyright.lines().count(), 3);

        // Résolu, le jeton laisse la place à l'auteur du livre.
        let mut l = l;
        l.auteur = "Ivan Pjig".into();
        assert!(l.copyright().starts_with("© Ivan Pjig, 2"));
        assert!(!l.copyright().contains('%'));
    }
```

- [ ] **Step 2: Vérifier l'échec**

```
cd app/src-tauri && cargo test --lib un_livre_neuf_porte_ses_generiques
```

Attendu : ÉCHEC, `left: "" right: "Titre"` — `Livre::vide()` pose des chaînes vides.

- [ ] **Step 3: Poser les génériques**

Dans `app/src-tauri/src/projet.rs`, `genre_defaut` change de valeur :

```rust
fn genre_defaut() -> String {
    "Genre".into()
}
```

Ajouter l'année et le copyright par défaut, à côté des autres fonctions de défaut :

```rust
/// L'année en cours, pour dater le copyright d'un projet neuf.
///
/// Prise sur `epub::horodatage`, dont le format `AAAA-MM-JJT…` est stable et entièrement
/// ASCII : le projet n'a aucune dépendance de date, il porte son propre calendrier — et
/// en avoir deux implantations serait pire que ce découpage.
fn annee_courante() -> String {
    crate::epub::horodatage(std::time::SystemTime::now())[..4].to_string()
}

/// Le copyright d'un projet neuf : l'auteur cité, l'année **écrite**.
///
/// L'année est figée à la création et non citée par un jeton : un `%ANNEE%` résolu à
/// chaque composition ferait dire 2028 au copyright d'un livre déposé en 2026.
fn copyright_defaut() -> String {
    format!(
        "© %AUTEUR%, {}.\nTous droits réservés.\nMaquette de couverture : atelier Ozalid",
        annee_courante()
    )
}
```

Dans `Livre::vide()`, les quatre champs préexistants :

```rust
            titre: "Titre".into(),
            auteur: "Auteur".into(),
            genre: genre_defaut(),
            …
            copyright: copyright_defaut(),
```

Le champ `copyright` porte `#[serde(default)]`, qui vaut la chaîne vide : le laisser
tel quel. Un `.ozalid` sans copyright en a délibérément un vide, ce n'est pas un projet
neuf — le défaut générique ne vaut qu'à la création.

- [ ] **Step 4: Le front cesse d'imposer « roman »**

Dans `app/src/app.js`, `livre()` :

```javascript
    genre: $('inGenre').value.trim(),
```

Le repli `|| 'roman'` doublait le défaut du Rust en le contredisant : vider le champ y
réécrivait « roman », que l'utilisateur venait précisément d'effacer.

- [ ] **Step 5: L'import d'un `livre.toml` suit le même défaut**

Dans `app/src-tauri/src/import.rs` :

```rust
        genre: s.genre.unwrap_or_else(crate::projet::genre_defaut),
```

et rendre `genre_defaut` visible du crate, comme `titre_page_defaut` :

```rust
pub(crate) fn genre_defaut() -> String {
```

- [ ] **Step 6: Ajuster le test du genre**

`app/src-tauri/src/commands.rs` porte `un_livre_vide_prend_le_genre_par_defaut`, qui
attend `"roman"`. Il vise désormais `"Genre"`. C'est le même test, avec la même
intention : un livre neuf ne naît pas sans genre.

- [ ] **Step 7: Vérifier**

```
cd app/src-tauri && cargo test --lib && cargo clippy --all-targets -- -D warnings && cargo fmt --check
cd app && node --test "tests/*.test.js"
cd app/src-tauri && cargo run --example temoin
```

Attendu : tout vert, et le témoin à **98 pages, dos 7,21 mm**. Il construit son livre
littéralement, donc les défauts ne l'atteignent pas — mais le vérifier est le seul moyen
de le savoir.

- [ ] **Step 8: Commit**

```bash
git add app
git commit -m "Un projet neuf montre la maquette telle qu'elle est"
```

---

## Tâche 2 : l'onglet Livre en deux groupes

**Files:**
- Modify: `app/src-tauri/src/gabarit.rs`
- Modify: `app/src-tauri/src/commands.rs`
- Modify: `app/src-tauri/src/lib.rs`
- Modify: `app/src/index.html`, `app/src/app.js`
- Modify: `app/tests/contrats.test.js`

- [ ] **Step 1: Écrire les tests**

Dans le module `tests` de `app/src-tauri/src/gabarit.rs` :

```rust
    /// La liste des jetons est servie par le Rust, seul à la connaître. La recopier
    /// dans le HTML la ferait mentir le jour où une clé s'ajoute — ce qui vient
    /// d'arriver deux fois.
    #[test]
    fn les_jetons_annonces_sont_ceux_qui_substituent() {
        let l = Livre {
            titre: "T".into(),
            auteur: "A".into(),
            genre: "G".into(),
            editeur: "E".into(),
            collection: "C".into(),
            monogramme: "M".into(),
            ..Livre::vide()
        };
        for jeton in jetons() {
            assert_ne!(
                substituer(jeton, &l),
                jeton,
                "{jeton} est annoncé mais ne substitue rien"
            );
        }
        assert_eq!(jetons().len(), JETONS.len());
    }
```

Dans `app/tests/contrats.test.js` :

```javascript
/**
 * L'aide de l'onglet Livre liste les jetons servis par le Rust, jamais une copie
 * écrite dans le HTML : la table `JETONS` a grossi deux fois en deux lots.
 */
test('l\'aide des jetons vient du Rust, pas du HTML', async () => {
  const rust = source('src-tauri', 'src', 'gabarit.rs');
  const attendus = [...rust.matchAll(/\("(%[A-Z]+%)"/g)].map((m) => m[1]);
  assert.ok(attendus.length >= 6, `moisson suspecte : ${attendus}`);

  const html = source('src', 'index.html');
  for (const jeton of attendus) {
    assert.ok(!html.includes(jeton), `${jeton} est recopié dans le HTML`);
  }

  const { els } = await charge({
    invoke: async (cmd, args) => (cmd === 'jetons_liste' ? attendus : invoke(cmd, args)),
    open: async () => null,
  });
  const aide = els.get('aideJetons').textContent;
  for (const jeton of attendus) {
    assert.ok(aide.includes(jeton), `${jeton} absent de l'aide`);
  }
});
```

- [ ] **Step 2: Vérifier l'échec**

```
cd app/src-tauri && cargo test --lib les_jetons_annonces
```

Attendu : ÉCHEC de compilation, `cannot find function 'jetons' in this scope`.

- [ ] **Step 3: Servir la liste**

Dans `app/src-tauri/src/gabarit.rs` :

```rust
/// Les jetons reconnus, dans l'ordre où l'aide les présente.
pub fn jetons() -> Vec<&'static str> {
    JETONS.iter().map(|(j, _)| *j).collect()
}
```

Dans `app/src-tauri/src/commands.rs`, à côté de `polices_texte_liste` :

```rust
#[tauri::command]
pub fn jetons_liste() -> Vec<&'static str> {
    crate::gabarit::jetons()
}
```

et l'inscrire dans le `invoke_handler` de `lib.rs`, avec les autres commandes.

- [ ] **Step 4: Les deux groupes dans le HTML**

Dans `app/src/index.html`, remplacer le bloc « Livre » par deux blocs. Les six clés
d'abord, les cinq libres ensuite, l'aide entre le titre du second groupe et ses champs.
Le paragraphe d'aide est **vide dans le HTML** — c'est `app.js` qui l'emplit depuis la
commande :

```html
    <div class="bloc">
      <h2>Livre</h2>
      <label><span>Titre</span><input type="text" id="inTitre"></label>
      <label><span>Auteur</span><input type="text" id="inAuteur"></label>
      <label><span>Genre</span><input type="text" id="inGenre"></label>
      <label><span>Éditeur</span><input type="text" id="inEditeur"></label>
      <label><span>Collection</span><input type="text" id="inCollection"></label>
      <label><span>Monogramme</span><input type="text" id="inMonogramme"></label>
    </div>

    <div class="bloc">
      <h2>Textes dérivés</h2>
      <!-- Vide à dessein : la liste des jetons vient de `jetons_liste`. La recopier ici
           la ferait mentir au premier jeton ajouté. -->
      <p class="note" id="aideJetons"></p>
      <label><span>Titre de la page de titre</span>
        <textarea id="inTitrePage" rows="2" placeholder="%TITRE%"></textarea></label>
      <label><span>Dédicace</span>
        <textarea id="inDedicace" rows="2"
                  placeholder="vide : pas de page de dédicace"></textarea></label>
      <label><span>Copyright</span><textarea id="inCopyright" rows="3"></textarea></label>
      <label><span>Prix</span><input type="text" id="inPrix"></label>
      <label><span>Mention</span><input type="text" id="inMention"></label>
      <label><span>Chapitres attendus</span>
        <input type="number" id="inChapitres" min="1" placeholder="facultatif — contrôle d'intégrité"></label>
    </div>
```

`inChapitres` reste dans le second bloc faute d'un troisième : il n'est ni une clé ni un
champ dérivé, mais un contrôle d'intégrité du manuscrit. Son étiquette le dit déjà.

- [ ] **Step 5: Emplir l'aide au démarrage**

Dans `app/src/app.js`, dans `chargerProviders`, à côté des autres listes servies par le
Rust :

```javascript
  $('aideJetons').textContent =
    `Ces champs peuvent citer les précédents : ${(await invoke('jetons_liste')).join(' ')}`;
```

- [ ] **Step 6: Vérifier**

```
cd app/src-tauri && cargo test --lib && cargo clippy --all-targets -- -D warnings && cargo fmt --check
cd app && node --test "tests/*.test.js"
```

Attendu : tout vert.

- [ ] **Step 7: Voir la garde échouer par mutation**

Recopier `%TITRE%` en dur dans le paragraphe d'aide du HTML, et lancer :

```
cd app && node --test tests/contrats.test.js
```

Attendu : ÉCHEC, « %TITRE% est recopié dans le HTML ». Rétablir.

- [ ] **Step 8: Commit**

```bash
git add app
git commit -m "L'onglet Livre sépare ce qui nomme de ce qui dérive"
```

---

## Tâche 3 : la vérification d'ensemble

- [ ] **Step 1: Le témoin**

```
cd app/src-tauri && cargo run --example temoin
```

Attendu : **98 pages, dos 7,21 mm**.

- [ ] **Step 2: À l'œil**

Un projet neuf, ouvert sur chacune des trois maquettes. Les valeurs génériques doivent
paraître là où la maquette les montre et nulle part ailleurs : en Folio, pied éteint,
l'éditeur et le monogramme ne se voient pas sur la 1ère ; en Blanche, pied allumé, ils
paraissent — sans rien resaisir. Aucune page de dédicace ne doit être composée.

Puis taper `%AUTEUR%` dans le prix, et vérifier qu'il se résout dans le pied de 4ème.

Rappel du piège maison : en développement, `target/debug/fonts` ne suit pas `fonts/`
tout seul, et le repli de Typst est muet.

---

## Ce que ce lot referme

Le chantier de l'identité du livre est complet : six clés littérales, cinq champs
dérivés qui les citent, un format en version 3, et une seule frontière à retenir — le
livre dit ce qui est écrit, la maquette dit où et si ça se voit.

Reste le chantier des maquettes : fichiers livrés, fournies contre personnalisées, CRUD,
clonage, avec images et cadrage. Sa spec est à écrire ; tout ce qui a été décidé au
brainstorming du 23/08 tient, à la section « ce qu'une maquette emporte » près, que la
discipline de l'utilisateur remplace.
