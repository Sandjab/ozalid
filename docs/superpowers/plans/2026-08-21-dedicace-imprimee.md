# La page de dédicace imprimée — plan d'implémentation

> **Pour les agents :** SOUS-SKILL REQUISE — `superpowers:subagent-driven-development`
> (recommandée) ou `superpowers:executing-plans` pour exécuter ce plan tâche par tâche.
> Les étapes sont des cases à cocher (`- [ ]`).

**But :** ajouter au livre une dédicace facultative, composée en belle page après le
copyright, verso blanc — deux pages de plus quand elle est renseignée, rien du tout
quand elle ne l'est pas.

**Architecture :** un champ `Option<String>` sur `Livre`, un accesseur qui rejette le
blanc, une fonction `liminaires()` extraite de `interieur::source` qui porte les pages
1 à 6, et trois points de câblage dans le front. Aucun nouveau mécanisme : la blanche
est un `#pagebreak()` doublé, l'échappement est celui du copyright.

**Pile :** Rust (Tauri 2, serde, Typst en sidecar), JavaScript sans build, tests
`cargo test` et `node --test`.

**Spec :** `docs/superpowers/specs/2026-08-21-dedicace-imprimee-design.md`

---

## Fichiers touchés

| Fichier | Responsabilité dans ce chantier |
|---|---|
| `app/src-tauri/src/projet.rs` | Le champ `dedicace`, son accesseur, `Livre::vide()`, un helper de test |
| `app/src-tauri/src/interieur.rs` | `liminaires()` extraite, la page de dédicace, ses tests |
| `app/src-tauri/src/import.rs` | Trois constructions littérales de `Livre` à compléter |
| `app/src-tauri/src/epreuve.rs`, `planche.rs`, `couverture.rs` | Un helper de test chacun |
| `app/src-tauri/examples/temoin.rs` | La construction du livre témoin |
| `app/src/index.html` | Le champ de saisie |
| `app/src/app.js` | Affichage, collecte, écouteur |
| `app/tests/coquille.test.js` | Les deux tests du front |
| `CLAUDE.md` | `outils/` acté archive |

Toutes les commandes `cargo` se lancent depuis `app/src-tauri`, toutes les commandes
`node` depuis `app`.

---

## Tâche 1 : le champ sur `Livre`

**Fichiers :**
- Modifier : `app/src-tauri/src/projet.rs:36-74` (struct, `vide()`, accesseur), `:334`
  (helper de test)
- Modifier : `app/src-tauri/src/import.rs:56`, `:579`, `:747`
- Modifier : `app/src-tauri/src/interieur.rs:263`, `epreuve.rs:164`,
  `couverture.rs:988`, `planche.rs:282`
- Modifier : `app/src-tauri/examples/temoin.rs:41`

- [ ] **Étape 1 : écrire les tests dans `projet.rs`, module `tests`**

À ajouter à la fin du module `#[cfg(test)] mod tests` :

```rust
/// Un `.ozalid` écrit avant la dédicace s'ouvre sans un mot : le champ est
/// facultatif, `VERSION` n'a donc pas bougé. Si cette garde tombe, ce sont tous les
/// projets déjà enregistrés qui refusent de s'ouvrir.
#[test]
fn un_projet_sans_champ_dedicace_se_relit() {
    let toml = r#"
[ozalid]
version = 2
[livre]
titre = "Les Heures creuses"
auteur = "Ivan Pjig"
"#;
    let m: Metadonnees = toml::from_str(toml).expect("relecture refusée");
    assert_eq!(m.livre.dedicace, None);
}

/// Une dédicace faite d'espaces ne doit pas coûter deux pages et un dos : c'est
/// l'accesseur qui tranche, une seule fois, pour tous ses appelants.
#[test]
fn une_dedicace_de_blanc_equivaut_a_pas_de_dedicace() {
    let mut l = livre();
    assert_eq!(l.dedicace(), None);
    l.dedicace = Some("   \n  ".into());
    assert_eq!(l.dedicace(), None, "du blanc a été pris pour une dédicace");
    l.dedicace = Some("  À M.  ".into());
    assert_eq!(l.dedicace(), Some("À M."), "les bords doivent être rognés");
}
```

Et, dans le test `un_projet_complet_survit_a_l_aller_retour` qui existe déjà, ajouter
la dédicace au projet écrit — le `.ozalid` est le document de l'utilisateur, une perte
silencieuse s'y garde là où toutes les autres se gardent, pas dans un test à part.
Après `p.meta.interieur.police = "Cardo".into();` :

```rust
        p.meta.livre.dedicace = Some("À M., qui a tenu la lampe.".into());
```

et, parmi les assertions qui suivent `let r = aller_retour(&p);` :

```rust
        assert_eq!(r.meta.livre.dedicace(), Some("À M., qui a tenu la lampe."));
```

- [ ] **Étape 2 : lancer les tests et constater l'échec**

```bash
cd app/src-tauri && cargo test un_projet_sans_champ_dedicace_se_relit
```

Attendu : **échec de compilation**, `no field 'dedicace' on type 'Livre'`. En Rust
c'est la forme normale du rouge pour un champ qui n'existe pas encore ; ne pas passer
à l'étape suivante avant de l'avoir vu.

- [ ] **Étape 3 : ajouter le champ et l'accesseur**

Dans `projet.rs`, struct `Livre`, après `copyright` (l'ordre des champs suit l'ordre
des pages du livre) :

```rust
    /// Dédicace imprimée, en belle page après le copyright. Absente ou vide, aucune
    /// page n'est composée : c'est `dedicace()` qui en juge, pas ses appelants.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dedicace: Option<String>,
```

Dans `Livre::vide()`, après `copyright: String::new(),` :

```rust
            dedicace: None,
```

Dans `impl Livre`, après `titre_page()` :

```rust
    /// La dédicace, si le livre en porte une qui ne soit pas que du blanc.
    ///
    /// Le rognage est ici et nulle part ailleurs : une dédicace réduite à une espace
    /// ajouterait sinon deux pages au livre, donc du dos, sans que rien ne se voie à
    /// l'écran.
    pub fn dedicace(&self) -> Option<&str> {
        self.dedicace
            .as_deref()
            .map(str::trim)
            .filter(|d| !d.is_empty())
    }
```

- [ ] **Étape 4 : compléter les neuf autres constructions littérales**

Ajouter `dedicace: None,` après la ligne `copyright: …` dans chacune :

```
app/src-tauri/src/import.rs:56      (l'import d'un livre.toml)
app/src-tauri/src/import.rs:579     (helper de test)
app/src-tauri/src/import.rs:747     (helper de test)
app/src-tauri/src/interieur.rs:263  (fn livre())
app/src-tauri/src/epreuve.rs:164    (fn livre())
app/src-tauri/src/couverture.rs:988 (fn livre())
app/src-tauri/src/projet.rs:334     (fn livre())
app/src-tauri/src/planche.rs:282    (fn livre())
app/src-tauri/examples/temoin.rs:41 (le livre témoin — laisser sans dédicace)
```

Les numéros de ligne bougent au fil des éditions ; les retrouver par
`grep -rn "Livre {" src examples` et vérifier qu'il n'en reste aucune sans le champ.

**`examples/temoin.rs` reste sans dédicace** : c'est ce qui garde le témoin à 98 pages.

- [ ] **Étape 5 : lancer les tests et constater le vert**

```bash
cd app/src-tauri && cargo test
```

Attendu : tout passe, y compris les deux nouveaux tests. Aucun test existant ne doit
changer de résultat — ce n'est encore qu'un champ inerte.

- [ ] **Étape 6 : commit**

```bash
git add app/src-tauri
git commit -m "Le livre peut porter une dédicace, que le blanc ne compte pas"
```

---

## Tâche 2 : extraire `liminaires()`, sans rien changer au document

Extraction pure : la source produite doit rester **identique à l'octet près**. Le
filet est double — les tests existants d'`interieur.rs`, et le témoin.

**Fichiers :**
- Modifier : `app/src-tauri/src/interieur.rs:130-248` (`source`)

- [ ] **Étape 1 : relever le témoin avant de toucher à quoi que ce soit**

```bash
cd app/src-tauri && cargo run --example temoin
```

Attendu : `98 pages`, la valeur de `PAGES_ATTENDUES`. Noter aussi le dos affiché : il
devra être identique à la fin du chantier. Si ce relevé n'est pas à 98 avant même
d'avoir édité une ligne, s'arrêter et le signaler — le reste du plan repose dessus.

- [ ] **Étape 2 : déplacer les deux blocs de liminaires dans une fonction**

Dans `interieur.rs`, ajouter avant `fn majuscules` :

```rust
/// Les pages liminaires : faux-titre, blanche, page de titre, copyright.
///
/// Toutes sans folio, et sans avoir à le dire : `footer: none` posé par l'entête court
/// jusqu'au `#set page(footer: …)` qui ouvre le corps.
fn liminaires(livre: &Livre) -> String {
    let mut s = String::new();
    s.push_str(&format!(
        r#"#v(42mm)
#align(center, text(size: 11pt, tracking: 0.12em)[{}])
#pagebreak()
#pagebreak()

#v(30mm)
#align(center, text(size: 10.5pt, tracking: 0.1em)[{}])
#v(14mm)
#align(center, text(size: 15pt, tracking: 0.06em)[{}])
#v(10mm)
#align(center, emph(text(size: 10pt)[{}]))
#pagebreak()

"#,
        majuscules(&livre.titre),
        majuscules(&livre.auteur),
        majuscules(&livre.titre_page().replace('\n', "\u{1}")).replace('\u{1}', r" \ "),
        echappe(&livre.genre),
    ));

    // Le pavé de copyright est calé en bas de la justification. La chaîne Python le
    // posait à 143 mm du haut du corps — une valeur juste pour le poche Lulu et
    // arbitraire ailleurs ; le bas de la justification est la même intention, exprimée
    // indépendamment du format.
    s.push_str(&format!(
        r#"#place(bottom + center, block(width: 100%)[
  #set par(leading: 0.5em, spacing: 0.5em, first-line-indent: 0pt, justify: false)
  #align(center, text(size: 8pt)[{}])
])
#pagebreak()

"#,
        echappe(&livre.copyright).replace('\n', r" \ ")
    ));

    s
}
```

Dans `source()`, remplacer les deux `s.push_str(&format!(…))` correspondants — celui
qui commence par `// — Liminaires, sans folio` et celui du pavé de copyright — par :

```rust
    // — Liminaires, sans folio : faux-titre, blanche, page de titre, copyright —
    s.push_str(&liminaires(livre));
```

Le commentaire `// — Corps, folio rétabli…` et tout ce qui suit ne bougent pas.

- [ ] **Étape 3 : vérifier que rien n'a bougé**

```bash
cd app/src-tauri && cargo test && cargo run --example temoin
```

Attendu : tous les tests passent, et le témoin rend **98 pages** avec le même dos
qu'à l'étape 1. Un écart d'une seule page signifie que l'extraction a perdu ou ajouté
un saut de ligne : revenir dessus, ne pas ajuster `PAGES_ATTENDUES`.

- [ ] **Étape 4 : commit**

```bash
git add app/src-tauri/src/interieur.rs
git commit -m "Les pages liminaires se lisent d'un bloc, et le témoin ne bouge pas"
```

---

## Tâche 3 : la page de dédicace

**Fichiers :**
- Modifier : `app/src-tauri/src/interieur.rs` (`liminaires`, module `tests`)

- [ ] **Étape 1 : écrire les trois tests**

À ajouter à la fin du module `#[cfg(test)] mod tests` d'`interieur.rs` :

```rust
/// Une dédicace renseignée coûte exactement deux pages : la belle page et sa blanche.
/// Une seule, et le premier chapitre s'ouvrirait au verso ; trois, et le livre gagne
/// un feuillet que personne n'a demandé — dans les deux cas le dos est faux.
#[test]
fn une_dedicace_ajoute_une_belle_page_et_sa_blanche() {
    let sans = liminaires(&livre());
    let mut l = livre();
    l.dedicace = Some("À M., qui a tenu la lampe.".into());
    let avec = liminaires(&l);

    assert_eq!(
        avec.matches("#pagebreak()").count(),
        sans.matches("#pagebreak()").count() + 2,
        "la dédicace ne coûte pas deux pages"
    );
    assert!(
        avec.contains("#align(right, emph(text(size: 9.5pt)[À M., qui a tenu la lampe.]))"),
        "la dédicace n'est pas composée en petit italique à droite : {avec}"
    );
}

/// Absente, vide ou faite d'espaces : la même source, à l'octet près. C'est ce qui
/// garantit qu'un livre déjà composé ne change pas de pagination — donc pas de dos —
/// du seul fait que le champ existe désormais.
#[test]
fn une_dedicace_vide_ou_blanche_ne_compose_rien() {
    let sans = liminaires(&livre());
    for creux in ["", "   ", "\n \n"] {
        let mut l = livre();
        l.dedicace = Some(creux.into());
        assert_eq!(
            liminaires(&l),
            sans,
            "« {creux:?} » a été pris pour une dédicace"
        );
    }
}

/// Les deux pièges déjà gardés pour le titre de page : le markup Typst doit être
/// échappé, et les sauts de ligne voulus doivent survivre. Un `#` non échappé fait
/// échouer la compilation du livre entier, plusieurs centaines de pages plus loin.
#[test]
fn une_dedicace_est_echappee_et_garde_ses_sauts_de_ligne() {
    let mut l = livre();
    l.dedicace = Some("À #M.,\nqui a tenu la lampe.".into());
    let s = liminaires(&l);

    assert!(s.contains(r"À \#M.,"), "dédicace non échappée : {s}");
    assert!(s.contains(r"\ qui a tenu la lampe."), "saut de ligne perdu : {s}");
}
```

- [ ] **Étape 2 : lancer les tests et constater l'échec**

```bash
cd app/src-tauri && cargo test dedicace
```

Attendu : `une_dedicace_ajoute_une_belle_page_et_sa_blanche` et
`une_dedicace_est_echappee_et_garde_ses_sauts_de_ligne` échouent (aucune page n'est
composée) ; `une_dedicace_vide_ou_blanche_ne_compose_rien` passe déjà — c'est normal,
il garde un comportement qu'on est sur le point de pouvoir casser.

- [ ] **Étape 3 : composer la page**

Dans `liminaires()`, avant le `s` final :

```rust
    // La dédicace prend une belle page, son verso reste blanc — deux `#pagebreak()`
    // d'affilée, le dispositif de la blanche des liminaires. Le corps s'ouvre donc en
    // page 7 au lieu de 5, et le dos en tient compte puisqu'il découle de la
    // pagination mesurée.
    if let Some(d) = livre.dedicace() {
        s.push_str(&format!(
            r#"#v(48mm)
#align(right, emph(text(size: 9.5pt)[{}]))
#pagebreak()
#pagebreak()

"#,
            echappe(d).replace('\n', r" \ ")
        ));
    }
```

- [ ] **Étape 4 : lancer les tests et constater le vert**

```bash
cd app/src-tauri && cargo test
```

Attendu : tout passe.

- [ ] **Étape 5 : voir les tests échouer sur des mutations ciblées**

Un test qui n'a jamais échoué ne prouve rien. Appliquer chaque mutation, lancer
`cargo test`, vérifier l'échec annoncé, **puis annuler la mutation**.

| Mutation | Échec attendu |
|---|---|
| Retirer le second `#pagebreak()` | `une_dedicace_ajoute_une_belle_page_et_sa_blanche` |
| Remplacer `livre.dedicace()` par `livre.dedicace.as_deref()` | `une_dedicace_vide_ou_blanche_ne_compose_rien` |
| Retirer `echappe(` autour de `d` | `une_dedicace_est_echappee_et_garde_ses_sauts_de_ligne` |

- [ ] **Étape 6 : compiler une vraie page**

Poser temporairement une dédicace dans `examples/temoin.rs` :

```rust
        dedicace: Some("À M., qui a tenu la lampe.".into()),
```

puis :

```bash
cd app/src-tauri && cargo run --example temoin
```

Attendu : **100 pages**, ni 99 ni 101, et un dos supérieur à celui relevé en tâche 2.
Ouvrir le PDF produit et regarder la page 5 : petit italique, aligné à droite, dans le
tiers supérieur, page 6 blanche, chapitre Un en page 7.

**C'est le seul moment où les valeurs `48mm` et `9,5pt` se jugent.** La spec les donne
comme point de départ, choisies par cohérence avec les liminaires voisins et non
mesurées : les ajuster ici est prévu ; ne pas regarder ne l'est pas.

Puis **annuler la modification de `temoin.rs`** et relancer :

```bash
cd app/src-tauri && cargo run --example temoin
```

Attendu : retour à **98 pages** et au dos de la tâche 2.

- [ ] **Étape 7 : commit**

```bash
git add app/src-tauri/src/interieur.rs
git commit -m "La dédicace prend sa belle page, et le livre en compte le prix"
```

---

## Tâche 4 : le champ dans l'atelier

Le piège de ce chantier est ici : `livre_modifier` remplace le `Livre` entier par ce
que le front envoie, et `dedicace` est `#[serde(default)]`. Un objet JavaScript sans
`dedicace` ne provoque aucune erreur — il efface la dédicace. Le second test existe
pour ça.

**Fichiers :**
- Modifier : `app/src/index.html:57` (après le Copyright)
- Modifier : `app/src/app.js:325`, `:566-573`, `:1188`
- Modifier : `app/tests/coquille.test.js`

- [ ] **Étape 1 : écrire les deux tests**

À ajouter à la fin de `app/tests/coquille.test.js` :

```js
test('la dédicace saisie part avec le livre', async () => {
  const a = atelier();
  const { els } = await charge({ invoke: a.invoke });
  await els.get('btNouveau').declenche('click');

  els.get('inDedicace').value = 'À M., qui a tenu la lampe.';
  await els.get('inDedicace').declenche('change');

  const envoi = a.appels.findLast(([c]) => c === 'livre_modifier');
  assert.ok(envoi, 'aucun livre_modifier : le champ n\'a pas d\'écouteur');
  assert.equal(envoi[1].livre.dedicace, 'À M., qui a tenu la lampe.');
});

/**
 * `livre_modifier` remplace le livre entier, et le champ est facultatif côté Rust :
 * un livre envoyé sans sa dédicace ne lève rien, il l'efface. Modifier son titre
 * suffirait donc à perdre la dédicace, sans un message.
 */
test('modifier un autre champ n\'efface pas la dédicace', async () => {
  const a = atelier({
    sur: {
      livre: {
        titre: 'Les Heures creuses', titre_page: null, auteur: 'Ivan Pjig',
        genre: 'roman', copyright: '', chapitres: null, dedicace: 'À M.',
      },
    },
  });
  const { els } = await charge({ invoke: a.invoke });
  await els.get('btNouveau').declenche('click');

  els.get('inTitre').value = 'Les Heures pleines';
  await els.get('inTitre').declenche('change');

  const envoi = a.appels.findLast(([c]) => c === 'livre_modifier');
  assert.equal(envoi[1].livre.dedicace, 'À M.', 'la dédicace a été effacée en douce');
});
```

- [ ] **Étape 2 : lancer les tests et constater l'échec**

```bash
cd app && node --test "tests/coquille.test.js"
```

Attendu : les deux échouent. Le premier sur `inDedicace` introuvable dans le faux DOM
— il lit les identifiants du vrai HTML, et le champ n'y est pas encore. Le second sur
`dedicace` absent de l'envoi.

- [ ] **Étape 3 : le champ dans le HTML**

Dans `app/src/index.html`, après la ligne du Copyright :

```html
      <label><span>Dédicace</span>
        <textarea id="inDedicace" rows="2"
                  placeholder="vide : pas de page de dédicace"></textarea></label>
```

- [ ] **Étape 4 : les trois points de câblage dans `app.js`**

Après `$('inCopyright').value = p.livre.copyright;` :

```js
  $('inDedicace').value = p.livre.dedicace ?? '';
```

Le `??` est nécessaire : `skip_serializing_if` retire le champ du JSON quand la
dédicace est absente.

Dans `function livre()`, après `copyright: …` :

```js
    dedicace: $('inDedicace').value.trim() === '' ? null : $('inDedicace').value,
```

Le champ part **non rogné** quand il porte du texte : c'est le Rust qui rogne, en un
seul endroit. Le `trim()` ne sert ici qu'à distinguer le vide du renseigné.

Enfin, dans la liste des identifiants qui pose l'écouteur `change` :

```js
for (const id of ['inTitre', 'inTitrePage', 'inAuteur', 'inGenre', 'inCopyright',
  'inDedicace', 'inChapitres']) {
```

- [ ] **Étape 5 : lancer les tests et constater le vert**

```bash
cd app && node --test "tests/*.test.js" && node --check src/app.js
```

Attendu : tous les tests passent — dont `dom_shim.test.js`, qui vérifie que les
identifiants du HTML sont tous connus du faux DOM.

- [ ] **Étape 6 : voir les tests échouer sur des mutations ciblées**

| Mutation | Échec attendu |
|---|---|
| Retirer `dedicace` de `function livre()` | les deux tests |
| Retirer `'inDedicace'` de la liste d'écouteurs | `la dédicace saisie part avec le livre` |

Annuler chaque mutation après l'avoir vue échouer.

- [ ] **Étape 7 : commit**

```bash
git add app/src/index.html app/src/app.js app/tests/coquille.test.js
git commit -m "La dédicace se saisit dans le livre, et survit aux autres champs"
```

---

## Tâche 5 : `outils/` acté archive

**Fichiers :**
- Modifier : `CLAUDE.md`

- [ ] **Étape 1 : relire le passage concerné**

Deux endroits parlent d'`outils/` : le paragraphe d'entête (« La chaîne Python de
composition de l'intérieur vit dans `outils/` ») et la dernière puce des
« Vérifications avant commit » (`python3 -m py_compile outils/*.py`, et la
régénération d'un intérieur complet par `gen_interieur.py`).

- [ ] **Étape 2 : dire que la chaîne est morte**

Dans le paragraphe d'entête (ligne 3), remplacer exactement :

```
La chaîne Python de composition de l'intérieur vit dans `outils/` (voir README) ;
```

par :

```
`outils/` conserve la chaîne Python (pandoc + WeasyPrint) qui composait l'intérieur avant le passage à Typst : c'est de l'archive, plus maintenue — l'app fait foi ;
```

Dans « Vérifications avant commit », remplacer la dernière puce, celle qui commence
par `python3 -m py_compile outils/*.py`, par :

```markdown
- `cargo run --example temoin` si un fichier de `app/src-tauri/` a changé : le compte
  de pages affiché est le témoin de non-régression, à comparer au précédent sur le
  même manuscrit.
```

Le répertoire `outils/` n'est **pas** supprimé : il reste au dépôt pour l'historique.

**Ne rien changer d'autre dans `CLAUDE.md`**, même si le reste du fichier est visiblement
périmé — l'entête y décrit encore l'atelier HTML de la racine comme étant « l'app ». La
spec n'autorise que ce changement-ci ; le reste est à soumettre à l'utilisateur
séparément.

- [ ] **Étape 3 : commit**

```bash
git add CLAUDE.md
git commit -m "La chaîne Python est de l'archive, et le témoin dit le vrai"
```

---

## Tâche 6 : vérification d'ensemble

- [ ] **Étape 1 : la chaîne complète**

```bash
cd app/src-tauri && cargo fmt --check && cargo clippy -- -D warnings && cargo test
cd app && node --test "tests/*.test.js" && node --check src/app.js && node --check src/couverture.js
cd app/src-tauri && cargo run --example temoin
```

Attendu : propre partout, et **98 pages** avec le dos relevé en tâche 2.

- [ ] **Étape 2 : à l'écran**

```bash
caffeinate -u -t 1 && killall ScreenSaverEngine 2>/dev/null; cd app && cargo tauri dev
```

À vérifier dans l'application :

1. L'étape Livre montre le champ Dédicace sous le Copyright, avec son placeholder.
2. Saisir une dédicace, quitter le champ : l'état passe à « modifié ».
3. Enregistrer, fermer, rouvrir le projet : la dédicace est revenue.
4. Modifier le titre : la dédicace est **toujours là** dans le champ.
5. Générer un package : le PDF d'intérieur porte la dédicace en page 5, la page 6 est
   blanche, le chapitre Un s'ouvre en page 7.
6. Vider le champ, régénérer : le compte de pages retombe à sa valeur d'avant.

`cargo tauri dev` recharge le webview dès que `src/*.css` ou `src/*.js` change : le
projet ouvert se referme mais le Rust garde son état modifié, et un clic sur un récent
déclenche alors la garde. Répondre « Ne pas enregistrer » et revérifier le SHA-256 du
`.ozalid` de travail.

- [ ] **Étape 3 : compte rendu**

Écrire ce qui a été vérifié **avec les valeurs relevées** : le compte de pages avec et
sans dédicace, le dos dans les deux cas, les mutations vues échouer, et les valeurs
typographiques finalement retenues si elles diffèrent de `48mm` / `9,5pt`.
