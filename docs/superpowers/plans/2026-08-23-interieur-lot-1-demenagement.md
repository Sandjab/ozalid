# Lot 1 — la police et l'épreuve déménagent, l'onglet meurt

Spec : `docs/superpowers/specs/2026-08-23-interieur-sans-onglet-design.md`

## Ce que ce lot fait, et ce qu'il ne fait pas

Un **déménagement**, et rien d'autre. Les deux blocs de l'étape Intérieur remontent dans
l'étape Livre, la section disparaît, les onglets passent de cinq à quatre. Le bouton
« Composer l'intérieur » **survit**, relogé avec la police ; le panneau de résultat aussi.
Aucun comportement ne change — c'est ce qui rend ce lot jugeable à l'œil sans rien
risquer.

Les identifiants ne bougent pas : `inPoliceInterieur`, `btComposer`, `etat`, `resultat`,
`inEpreuveCorps`, `btEpreuve`, `cheminEpreuve` gardent leurs noms et leurs branchements.
C'est ce qui fait que la plupart des tests qui les pilotent n'ont pas une ligne à changer.

## Un écart avec le § 12 de la spec, assumé

La spec range **le témoin de dos périmé** dans le lot 2, avec le reste du pied. C'est
impossible : le témoin vit sur l'onglet Intérieur, et cet onglet meurt ici. Le laisser
sur l'onglet Livre le temps d'un lot le ferait déménager deux fois, et réécrire ses tests
deux fois.

**Il descend donc au pied dès ce lot**, à la destination que la spec lui donne. C'est la
seule chose de ce lot qui ne soit pas un pur déplacement, et c'est pourquoi elle a sa
tâche à elle (tâche 3) plutôt que d'être glissée dans une autre.

## Tâche 1 : le balisage

**Files:**
- Modify: `app/src/index.html`

- [ ] **Step 1: Les deux blocs remontent dans Livre**

Dans `index.html`, la `<section id="etapeLivre">` se termine aujourd'hui par le bloc
« Manuscrit ». Y ajouter, **après** lui et dans cet ordre, les deux blocs pris tels quels
à `<section id="etapeInterieur">` : d'abord « Épreuve », puis « Intérieur ».

L'ordre compte et il n'est pas celui de l'ancienne étape : le manuscrit, ce qu'on en tire
pour le relire, puis comment on le compose. L'épreuve suit le manuscrit dont elle sort ;
la police ferme la marche parce qu'elle regarde vers le livre fini.

Le commentaire de l'ancienne section — « Aucun sélecteur de prestataire ici : il est dans
le pied, et il vaut pour toute la fenêtre » — suit le bloc « Intérieur » : il explique
toujours pourquoi « Composer » ne demande pas pour qui.

- [ ] **Step 2: La section disparaît**

Supprimer `<section id="etapeInterieur" class="etape" role="tabpanel" hidden>` et sa
balise fermante. Rien d'autre ne doit rester dedans — vérifier qu'aucun élément n'a été
oublié entre les deux blocs déplacés.

```bash
grep -n "etapeInterieur" src/index.html   # attendu : aucune ligne
```

## Tâche 2 : les onglets, le menu, les colonnes

**Files:**
- Modify: `app/src/app.js`
- Modify: `app/src-tauri/src/menu.rs`
- Modify: `app/src/styles.css`

- [ ] **Step 1: `ETAPES` perd une entrée**

Dans `app.js`, retirer `['interieur', '2 · Intérieur', 'etapeInterieur'],` et
**renuméroter les libellés** des trois suivantes : Couverture devient `2 ·`, Livraison
`3 ·`, Envois `4 ·`.

- [ ] **Step 2: `etatEtapes` perd son entrée `interieur`**

Retirer le bloc `interieur: { … }`. La constante `dosPerime` **ne part pas avec lui** :
elle sert à la tâche 3. La laisser où elle est ne suffira pas — `etatEtapes` et `majPied`
sont deux fonctions ; voir la tâche 3 pour l'endroit où elle doit vivre.

- [ ] **Step 3: Le commentaire d'`app.js` dit la vérité**

Le commentaire au-dessus d'`ETAPES` annonce « Six fichiers, donc, pas trois ». Le compte
est faux : cinq fichiers de test pilotent l'étape en plus des six. Le corriger en nommant
les tests, sans quoi le prochain qui ajoutera une étape refera l'oubli.

- [ ] **Step 4: Le menu natif**

Dans `menu.rs`, supprimer l'entrée `aller.interieur` et **renuméroter** : Couverture
`CmdOrCtrl+2`, Livraison `CmdOrCtrl+3`, Envois `CmdOrCtrl+4`.

Côté front, **rien à faire** : le gestionnaire est dérivé d'`ETAPES`
(`app.js:946` — `ETAPES.map(([cle]) => [\`aller.${cle}\`, …])`), donc l'entrée retirée au
step 1 emporte son branchement. Seul le Rust, qui écrit ses entrées à la main, doit être
corrigé — et c'est exactement le genre d'écart que ce lot doit vérifier à l'œil : un menu
qui garde une entrée sans destination ne casse rien de visible.

- [ ] **Step 5: Les colonnes**

Dans `styles.css`, les deux sélecteurs perdent leur milieu :

```
#etapeLivre, #etapeInterieur, #etapeEnvois   →   #etapeLivre, #etapeEnvois
#etapeLivre > .bloc, #etapeInterieur > .bloc, #etapeEnvois > .bloc
                                             →   #etapeLivre > .bloc, #etapeEnvois > .bloc
```

Le commentaire au-dessus de la première règle décrit un débordement de « l'étape
Intérieur composée » à 900 × 640 et annonce sa résorption. Il parle d'une étape qui
n'existe plus : le réécrire pour dire ce qui est vrai maintenant — **c'est l'étape Livre
qui porte cinq blocs**, et c'est elle qui déborde.

## Tâche 3 : le témoin de dos périmé descend au pied

**Files:**
- Modify: `app/src/app.js`
- Modify: `app/src/styles.css`

C'est la seule tâche de ce lot qui change quelque chose de visible.

- [ ] **Step 1: `majPied` gagne l'état « périmé »**

`majPied` calcule aujourd'hui trois états dans `piedDos` : « dos relevé sur le gabarit »,
« dos non composé », « dos N mm ». Il en gagne un quatrième, prioritaire sur les autres et
en rouge :

```
dos périmé
```

Sa condition est celle qui vit dans `etatEtapes` : `projet.livraison.deja_compose &&
!destinataireCourant()?.compose`. Elle doit désormais servir aux deux endroits — la
mettre dans une fonction nommée plutôt que la recopier, la duplication étant précisément
ce qui a déjà menti deux fois dans ce dépôt sur la liste des jetons.

Garder le commentaire qui l'explique — « `deja_compose` fait toute la différence : sans
lui, un livre jamais composé et un livre dont la mesure vient d'être périmée se
ressembleraient trait pour trait » — il n'a rien perdu de sa valeur en changeant
d'endroit.

- [ ] **Step 2: Le rouge**

`#piedDos` prend `var(--rouge)` dans cet état. **Le pied est sur fond clair**, pas sur le
fond noir des boutons du dialogue des maquettes : le piège de la session précédente — un
rouge illisible sur noir — ne s'applique pas ici. À regarder quand même.

- [ ] **Step 3: Le README perd un témoin**

Dans `app/README.md`, la phrase « Trois témoins et pas un de plus » et la liste qui la
précède : le dos périmé quitte les témoins d'onglets pour le pied. Il en reste **deux** —
le manuscrit qui ne correspond plus au contrôle d'intégrité (Livre) et la couverture sans
maquette (Couverture).

## Tâche 4 : les tests

**Files:**
- Modify: `app/tests/coquille.test.js`
- Modify: `app/tests/composition.test.js`
- Modify: `app/tests/contrats.test.js`
- Modify: `app/tests/epreuve.test.js`

Les tests qui pilotent `btComposer`, `inPoliceInterieur`, `btEpreuve` et `cheminEpreuve`
par leur identifiant **ne changent pas** : les éléments ont déménagé, pas été renommés.
Ne toucher qu'aux quatre fichiers ci-dessous, qui nomment la structure.

- [ ] **Step 1: `coquille.test.js`**

- La table `ETAPES` perd `'interieur'`.
- Les tests de navigation qui atterrissent sur l'étape Intérieur (`montree(els)` valant
  `['interieur']`, l'`aria-selected` et le focus de `onglet-interieur`) visent désormais
  une autre étape. **Ne pas les supprimer** : ils vérifient le pattern `tablist`, pas
  l'étape. Les rediriger sur Couverture.
- Les tests du témoin (`sous(els, 'interieur')`, `alerte(els, 'interieur')`) changent de
  cible : ils lisent maintenant `piedDos`. Les assertions « EB Garamond » disparaissent
  — la police n'a plus de sous-libellé —, les assertions « dos périmé » restent et
  s'adressent au pied.

- [ ] **Step 2: `composition.test.js`**

La liste `['etapeLivre', 'etapeInterieur', …]` du test « rien n'est proposé tant qu'aucun
projet n'est ouvert » perd son deuxième élément.

- [ ] **Step 3: `contrats.test.js`**

Le test « deux colonnes tiennent dans la fenêtre minimale » lit la règle CSS par une
expression régulière littérale :

```js
css.match(/#etapeLivre, #etapeInterieur, #etapeEnvois \{[^}]*\}/s)
```

Elle ne trouvera plus rien, et le test échouera sur son propre message — « la règle des
étapes en colonnes a changé de forme », ce qui sera exact. Mettre la regex à jour.

**Ce test est le garde-fou de la largeur minimale** : il refait le compte des colonnes
contre `tauri.conf.json`. Il doit continuer à passer sans qu'on touche aux nombres.

- [ ] **Step 4: `epreuve.test.js`**

`assert.strictEqual(els.get('etapeInterieur').hidden, false)` vise une section qui
n'existe plus. Le test vérifie qu'un projet ouvert montre la police à sa valeur : le
rediriger sur `etapeLivre`.

- [ ] **Step 5: Voir rougir**

Avant de corriger quoi que ce soit, lancer la suite **une fois** sur le balisage déjà
modifié et lire les échecs : ce sont eux qui disent ce que les tests protégeaient
vraiment. Un test qu'on corrige sans l'avoir vu échouer est un test qu'on a peut-être
vidé.

```bash
node --test tests/*.test.js
```

## Tâche 5 : le README, les vérifications, le commit

**Files:**
- Modify: `app/README.md`

- [ ] **Step 1: « L'écran »**

Dans la section « L'écran » d'`app/README.md` :

- « une rangée de cinq **onglets** — Livre, Intérieur, Couverture, Livraison, Envois » →
  quatre onglets, sans Intérieur.
- « Les cinq onglets se traversent aux flèches » → quatre.
- Le sous-libellé « le nombre de chapitres, la police, la maquette » → la police n'en a
  plus.
- Les témoins : trois → deux (voir tâche 3, step 3).
- « Le sous-menu **« Aller »** navigue entre les cinq étapes (⌘1 à ⌘5) » → quatre étapes,
  ⌘1 à ⌘4.

**Ne pas encore toucher** à la phrase sur le compte rendu d'un travail long ni à ce que
le pied dit : elles ne deviendront fausses qu'au lot 3 et au lot 2 respectivement.

- [ ] **Step 2: Les vérifications**

Depuis `app/` et `app/src-tauri/`, **jamais dans un pipe** :

```bash
cd app/src-tauri && cargo fmt --check
cd app/src-tauri && cargo clippy --all-targets -- -D warnings
cd app/src-tauri && cargo test
cd app && node --test tests/*.test.js
```

`menu.rs` a changé, donc :

```bash
cd app/src-tauri && cargo run --example temoin
```

Attendu : **98 pages, dos 7,21 mm**. Ce lot ne touche pas à la composition ; tout écart
est un bug.

- [ ] **Step 3: À l'œil, dans la fenêtre**

Le front est embarqué à la compilation : `touch src/lib.rs && cargo build` avant de
lancer, sans quoi le binaire garde l'ancien `src/`.

1. Quatre onglets, numérotés 1 à 4. ⌘1 à ⌘4 y mènent ; ⌘5 ne fait rien.
2. L'étape Livre porte cinq blocs dans l'ordre : Livre, Textes dérivés, Manuscrit,
   Épreuve, Intérieur.
3. **À 900 px de large**, l'étape Livre : les colonnes tiennent, et ce qui déborde tombe
   dans le filet de `main` — pas dans un ascenseur qui coupe un bloc en deux.
4. Composer : le panneau de résultat paraît dans Livre, à sa place.
5. Tirer une épreuve : le chemin paraît sous son bouton.
6. Changer la police après avoir composé : le pied passe à **« dos périmé »** en rouge,
   et aucun onglet ne s'allume.
7. Recomposer : le pied revient au dos chiffré.

- [ ] **Step 4: Commiter**

```bash
cd /Users/jean-paulgavini/Documents/Dev/ozalid
git add -A
git commit -m "$(cat <<'EOF'
La police et l'épreuve rejoignent le livre

L'étape Intérieur portait deux champs et deux boutons ; ses deux blocs remontent
dans Livre, après le manuscrit dont ils sortent. Quatre onglets, ⌘1 à ⌘4.

Le témoin de dos périmé descend au pied — son onglet n'existe plus, et le pied
portait déjà le dos. Il en reste deux sur les onglets.

Rien d'autre ne change : les identifiants sont les mêmes, « Composer » et le
panneau de résultat sont là, relogés. Témoin relevé : 98 pages, dos 7,21 mm.

Claude-Session: https://claude.ai/code/session_01AyXS78gurZmV9m6yFQ8LwL
EOF
)"
```

## Ce que ce lot laisse au suivant

- **Le pied ne porte encore aucun chiffre.** Pages, chapitres, gouttière et le lien
  « intérieur » sont le lot 2, avec `tauri-plugin-opener` et la question de
  `polices_introuvables` dans `Mesure` — les deux points laissés ouverts par la spec.
- **`#resultat` vit toujours**, dans Livre. Il meurt au lot 2, et le débordement de
  l'étape Livre à 900 × 640 s'en ira avec lui.
- **« Composer » vit toujours.** Il ne meurt qu'au lot 3, avec le déclenchement
  automatique au chargement du manuscrit. C'est le seul lot qui change le comportement,
  et il reste révocable d'un `revert`.
