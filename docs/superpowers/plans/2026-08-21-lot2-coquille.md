# Lot 2 — La coquille en quatre étapes

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remplacer la page unique de huit sections empilées par une coquille en quatre bandes — entête, onglets, étape courante, pied — où la fenêtre ne défile plus et où le contenu des étapes est déplacé tel quel, sans changement de fond.

**Architecture:** `body` devient une grille de quatre bandes à la hauteur exacte de la fenêtre, tenue par une variable CSS que le rail vertical d'un jour remplacerait à elle seule. Les huit `<section>` deviennent quatre `<section class="etape">` — Livre, Intérieur, Couverture, Livraison — dont une seule est montrée à la fois, plus un accueil qui prend leur place quand aucun projet n'est ouvert. Côté JS, une table `ETAPES` est la source unique des onglets, du routage du menu « Aller » et du masquage des sections. Le seul ascenseur restant est celui du panneau de réglages de la couverture.

**Tech Stack:** front vanilla sans bundler (`app/src/index.html`, `styles.css`, `app.js`), Rust + Tauri 2.11.5 pour la seule activation du menu « Aller », tests `node --test` sur le vrai `app.js` dans le faux DOM de `app/tests/dom_shim.js`.

---

## Contexte pour qui n'a jamais ouvert ce dépôt

L'application vit dans `app/`. Le Rust est dans `app/src-tauri/src/`, le front — trois fichiers, sans bundler — dans `app/src/`. Le front n'a **aucune logique métier** : il invoque des commandes Rust et affiche ce qu'elles rendent. Chaque commande qui touche au projet rend une `ProjetVue`, et `afficherProjet()` redessine le panneau entier depuis elle.

Les tests du front exécutent le **vrai** `app.js` dans un faux DOM (`app/tests/dom_shim.js`) qui lit l'état initial du **vrai** `index.html`. Ils sont la seule garde automatique du front, et la restructuration les invalide en bloc : ils sont réécrits dans ce lot, jamais laissés en attente.

Le français est la langue de l'interface, des commentaires et des messages de commit. Les commits du dépôt sont des phrases qui disent ce que le code a appris, jamais `feat:` ni `fix:`.

Commandes de vérification, à connaître avant de commencer :

```
cd app/src-tauri && cargo test --lib && cargo clippy --all-targets && cargo fmt --check
cd app && node --test "tests/*.test.js"
```

Le lot précédent — cycle de vie du projet et menu natif — est livré. Son plan
(`docs/superpowers/plans/2026-08-21-lot1-cycle-de-vie-et-menu.md`) porte en tête une
section « Écarts assumés en cours d'exécution » qu'il faut avoir lue : elle explique
notamment pourquoi `fichier.quitter` est une entrée ordinaire et pourquoi
`Interface::prete` existe.

---

## Décisions de lecture de la spec

Trois points que la spec implique sans les écrire noir sur blanc. Ils sont tranchés ici
pour que l'exécution n'ait pas à les inventer ; s'ils sont contestés, c'est **avant** la
tâche 2 qu'il faut le dire.

**1. Les boutons « Enregistrer » et « Enregistrer sous… » disparaissent de l'écran.**
La spec décrit l'entête comme portant « titre du livre · chemin du `.ozalid` · état
d'enregistrement » — aucun bouton — et l'accueil comme offrant « Nouveau projet, Ouvrir
un `.ozalid`, Importer un `livre.toml`, et la liste des projets récents » — les trois
seuls. Enregistrer devient donc un geste de menu (⌘S, ⇧⌘S), comme dans tout éditeur de
document macOS. Les fonctions `enregistrerQuelquePart()` et `enregistrerSous()` ne
changent pas : elles perdent leurs deux boutons, gardent leurs entrées de menu et leur
rôle dans la garde.

**2. L'étape Livraison est la seule zone défilante de trop, et pour un lot seulement.**
La table compte quatorze gabarits ; la liste des prestataires ne tient pas dans les
~480 px de contenu qu'offre la fenêtre à sa taille minimale (900 × 640, `tauri.conf.json`).
Le lot 3 réduit cette liste aux seuls **destinataires du livre** — une poignée — et le
problème disparaît avec lui. En attendant, `#etapeLivraison` porte son propre
`overflow-y: auto`. C'est une dette à date de péremption connue, écrite dans le CSS à
l'endroit où elle se paie, et non une exception silencieuse à « une seule zone défilante ».

**3. Les trois témoins de la spec, et pas un de plus.**
« manuscrit périmé (écart avec le contrôle d'intégrité) » → étape Livre. « aucune
maquette choisie » → étape Couverture. « dos périmé » → étape **Intérieur**, parce que
c'est là qu'on le répare : le dos vient de la composition. Un manuscrit **absent** n'allume
rien — la spec dit qu'« un manuscrit absent est un état, pas une erreur ».

---

## Structure des fichiers

**Créés**

| Fichier | Responsabilité |
|---|---|
| `app/tests/coquille.test.js` | Navigation par étapes : accueil, onglets inertes, une seule étape montrée, menu « Aller », témoins, pied |

**Modifiés**

| Fichier | Ce qui change |
|---|---|
| `app/tests/dom_shim.js` | Les identifiants viennent du HTML au lieu d'être énumérés ; un helper actionne le menu |
| `app/src/index.html` | Entête, nav, quatre étapes, accueil, pied ; le contenu des sections est déplacé tel quel |
| `app/src/styles.css` | Grille de la coquille, onglets, entête, pied, hauteur du panneau de réglages |
| `app/src/app.js` | Table `ETAPES`, navigation, alerte unique dans l'entête, pied, témoins |
| `app/src-tauri/src/menu.rs` | Le sous-menu « Aller » cesse d'être grisé |
| `app/tests/*.test.js` | Assertions de sections → assertions d'étapes ; les gestes devenus impossibles passent par le menu |
| `app/README.md` | La section qui décrit l'écran |

**Non touchés, et il faut qu'ils le restent :** `app/src/couverture.js` (le schéma des
réglages), tout `app/src-tauri/src/` sauf `menu.rs`, `index.html` à la racine du dépôt
(l'atelier HTML, gelé), `outils/`.

---

## Écarts assumés en cours d'exécution

**Défauts du plan lui-même, découverts à l'exécution de la tâche 2.**

*Le tableau du Step 7 est faux dans ses chiffres.* Ses numéros de ligne sont décalés de
16 à 45 lignes — il a été écrit avant que la tâche 1 ne retire les listes `IDS` — et ses
décomptes sont inventés : « 9 sites » de `secLivre.hidden === true` là où le fichier en
portait 4, « 5 sites » de clics sur les boutons d'enregistrement là où il n'y en avait
qu'un. Les descriptions, elles, étaient exactes, et c'est sur elles que le travail s'est
fait. **Un plan qui compte les sites sans les compter vraiment coûte plus qu'il ne
rapporte : décrire suffit.**

*Une justification du tableau était fausse.* Il prescrivait de supprimer sans
remplacement deux assertions `disabled` de `composition.test.js` au motif que « le test
`:191` vérifie déjà » l'état d'enregistrement. Ce test-là ne vérifiait rien de tel.
Supprimer sec aurait baissé la couverture ; une assertion sur `etatEnregistrement` a été
ajoutée à la place.

*Le commentaire du test « sans projet, Aller ne montre rien », donné par le plan,
décrivait deux dangers inexistants* — une étape vide qui se montrerait, une exception qui
remonterait dans le rappel de `listen`. Ni l'un ni l'autre : `majEtapes()` masque tout
quand `projet` est nul, et les deux identifiants existent. Réécrit pour dire ce que le
test vérifie vraiment. La garde d'`allerA()` est bien redondante avec ce masquage, et
elle est **gardée** pour une raison que le plan n'avait pas vue : dans le faux DOM,
`declenche('click')` ignore `disabled`, et c'est elle qui rend l'onglet réellement inerte
côté tests.

**Trous trouvés par mutation, comblés hors plan (tâche 2).**

1. *`tente()` n'effaçait l'alerte que par un chemin non gardé.* Retirer `alerter('')` de
   `tente()` ne faisait tomber aucun test. Le trou est **neuf** : l'erreur vivait
   auparavant dans `#etat`, qu'une section masquée emportait ; l'entête, elle, ne
   disparaît jamais, et une erreur qu'on n'y efface pas se lit comme le compte rendu du
   geste suivant. Commit `232a873`.
2. *Les onglets naissaient dans l'état du balisage.* Un `chargerProviders()` en échec
   n'appelle jamais `afficherAucunProjet`, donc jamais `majEtapes()` : les quatre onglets
   restaient d'apparence active, et le `tablist` sans onglet sélectionné. `majEtapes()`
   appelé à la fin de `construireEtapes()`. Commit `af92477`.
3. *Une erreur d'enregistrement survivait à l'enregistrement qui réussit.*
   `enregistrerQuelquePart` et `enregistrerSous` sont les deux seules écritures d'alerte
   hors `tente()` : elles écrivaient l'erreur sans jamais l'effacer. Le comportement
   préexistait — l'ancien code n'effaçait pas `#etat` non plus — mais la tâche 2 a fait de
   cette bande le canal unique et permanent. `alerter('')` posé **après** la garde
   `if (!projet) return false;` dans les deux : un ⌘S inerte n'a pas à effacer un
   « démarrage impossible » qui dit encore vrai. Commit `83c6d1a`, garde de ce placement
   en `6685c0a`.

**Décisions prises au-delà du plan (à ne pas « corriger » sans raison).**

- *La règle des deux canaux d'erreur est désormais écrite* au-dessus d'`alerter()` : ce
  qui refuse une saisie monte à l'entête, parce que le geste est fini et que le message
  doit survivre au changement d'étape ; ce qui rend compte d'un travail long — composer,
  tirer une épreuve, générer les packages — reste dans `#etat`, `#etatEpreuve`,
  `#etatPackages`, à côté du bouton qui l'a lancé. Faire remonter le reste par symétrie
  ferait perdre la différence.
- *`aria-live="polite"` sur `#alerte`*, non prévu : le canal d'erreur devenant unique et
  le focus restant dans le champ refusé, un lecteur d'écran n'annonçait jamais rien.
  Gardé par un test qui **lit `index.html` au lieu de passer par l'application** —
  exception assumée et documentée sur place : le faux DOM ne rapporte que la balise,
  `disabled`, `hidden` et `value`, et l'étendre pour un attribut ferait payer à soixante-dix
  tests le prix d'un seul.

**Trous trouvés par mutation, comblés hors plan (tâches 4 et 5).**

4. *Le pied levait après un démarrage raté.* `providerCourant()` rend `undefined` quand la
   table des gabarits n'a pas pu être lue, et `majPied()` était le **premier** appel à en
   déréférencer le résultat dans `afficherProjet` : l'application passait d'« utilisable en
   mode dégradé » à « cassée au premier geste », l'exception s'échappant même de `tente()`.
   Garde `!p` posée. Commit `e32f010`.
5. *« dos non composé » mentait chez un prestataire à dos relevé.* Chez un CoolLibri
   (`dos_publie: false`), après une composition **réussie** affichant 262 pages, le pied
   disait toujours « dos non composé » : `composer` rend `dos: null` chez ces
   prestataires-là — le dos ne se calcule pas, il se relève sur le gabarit. Défaut du plan,
   pas de l'exécution. Troisième état ajouté : « dos relevé sur le gabarit ». Commit `2a46875`.
6. *Une police **refusée** allumait « dos périmé » sur un dos intact.* `majEtapes()` était
   appelée huit lignes avant qu'`afficherProjet` ne repose le `select` de police :
   `dosCourant()` comparait le dos mesuré à une saisie que le refus venait d'annuler.
   L'utilisateur lisait une erreur **et** un témoin l'envoyant recomposer un livre juste.
   `majEtapes()` déplacée à la fin d'`afficherProjet`. Commit `6978a40`.

**Deux pièges de spécificité rouverts par le plan (tâche 6).**

Le plan prescrivait `#etapeCouverture { display: grid }`. Un sélecteur d'identifiant (1-0-0)
l'emporte sur `[hidden] { display: none }` (0-1-0) : **l'étape Couverture restait affichée en
permanence**, et cliquer sur l'onglet Livraison sélectionnait l'onglet mais servait la
Couverture. Aucun test ne pouvait l'attraper — ils vérifient l'attribut `hidden`, jamais le
`display` calculé. C'est exactement le piège que la dernière règle du fichier documentait.

La cascade mesurée dans un navigateur sur les seize éléments que le JS masque en a révélé un
second, dormant : `.releve .petit` (0-2-0) que le lot 3 armera en masquant les relevés selon
le prestataire. La classe entière est fermée par `[hidden] { display: none !important; }` —
la règle cesse de dépendre de sa position et de la spécificité des autres — et les trois
`:not([hidden])` posés entre-temps sont retirés, devenus des précautions trompeuses.

**La mise en page des étapes, refaite deux fois (tâche 6).**

Le plan donnait une colonne de 46 rem. Mesuré à 900 × 640, Livre demandait 568 px pour
398 offerts — et laissait 200 px vides à droite : c'était la **hauteur** qui manquait, jamais
la largeur. D'où une grille de colonnes de 23 rem.

Elle a dû être refaite : une grille aligne ses rangées, et Intérieur composé mettait le bloc
de résultat dans la colonne de droite pendant que la gauche restait vide sur 300 px. La
disposition gaspillait exactement la place qui lui manquait, et la barre était présente à
**1400 × 800** — donc à la taille par défaut de la fenêtre, 1040 × 780. Remplacée par un flux
en colonnes (`column-width`, `break-inside: avoid`), où Épreuve remonte dès qu'il y a la place.

Conséquence non prévue : **le filet a changé d'étage.** Un multi-colonnes contraint en hauteur
ne produit pas d'ascenseur vertical, il fragmente vers la droite — l'étape Épreuve est passée
hors écran derrière une barre horizontale. Un flux ne peut pas être à la fois ce qui coule et
ce qui défile. C'est donc `main` qui porte l'`overflow-y: auto`, et Livre et Intérieur
reprennent `height: auto`.

**La marge est fine et il faut le savoir** : à 1040 × 780, l'état composé passe avec 5 à 10 px
(mesuré par dichotomie : 775 tient, 770 défile). Le cas tient parce qu'on a resserré le rythme,
pas parce qu'il y avait de la place. Une ligne de plus au bloc de résultat le rouvre. Le seul
remède durable est de raccourcir ce bloc — lot 4.

**Versé à la suite du chantier, non fait ici.**

- *Le pattern d'onglets est incomplet* : pas d'`aria-controls` sur les onglets, pas
  d'`aria-labelledby` sur les sections, et pas de navigation aux flèches avec un seul
  onglet dans l'ordre de tabulation. C'est le plan qui l'a écrit ainsi. À reprendre au
  lot 4, la passe visuelle.
- *`#etapes button:disabled` est redondant* avec la règle globale `button:disabled`.
  Gardé par fidélité au Step 4 ; à revoir quand la tâche 5 posera les témoins.
- *Le bloc de résultat de composition est trop haut* : cinq lignes de chiffres et le chemin
  du PDF sur deux lignes. L'étape Intérieur composée déborde encore à 900 × 640 — décision
  de l'utilisateur, le filet la rattrape. Le resserrer touche au balisage, donc au lot 4.
- *L'ombre de l'aperçu* : `.scene` réserve `.9rem` là où l'ombre porte jusqu'à ~20 px. Sa
  partie dense est couverte, son dernier voile reste coupé. `1.25rem` serait exact.
- *`--onglets-flux` se lit à l'envers du bon sens* : `column` produit une barre horizontale,
  `row` produirait le rail vertical. Le nom décrit le mécanisme, pas le résultat.
- *`libelleMode()` reconstruit tout le schéma à chaque clic d'onglet* (`groupes()` remappe
  `SCHEMA` en entier pour un libellé qui ne change jamais). Coût négligeable, chemin chaud.

**Deux leçons de méthode, payées trois fois chacune.**

*Un commentaire au présent pour un état futur égare autant qu'un commentaire au passé.* Le
lot s'y est fait prendre trois fois : `--coquille` annoncée comme en place avant la tâche 6,
« le Rust offre ses quatre entrées » écrit alors que `menu.rs` les grisait encore, et la
doctrine de `[hidden]` qui exigeait une position devenue sans objet. La règle vaut dans les
deux sens.

*« Mes mutations n'ont rien trouvé » n'est pas « il n'y a rien ».* Un compte rendu a conclu
« aucune ligne n'est orpheline » sur vingt mutations ; trois lignes défensives non sondées
survivaient. Elles sont légitimes et restent — c'est la conclusion qui dépassait la mesure.

---

## Task 1: Le faux DOM lit les identifiants dans le HTML

**Pourquoi d'abord :** les cinq fichiers de test recopient chacun la même liste de
quarante-huit identifiants. La tâche 2 en supprime, en ajoute et en renomme : sans ce
préalable, chaque changement de balisage se paierait en cinq listes à corriger à la
main, et un identifiant oublié se manifesterait par un `null` sans message. Le faux DOM
lit déjà le vrai `index.html` pour connaître l'état initial de chaque élément ; qu'il en
lise aussi la liste est le prolongement exact de ce qu'il fait.

Deuxième objet de la tâche : les tests qui veulent actionner une entrée de menu
réimplantent chacun un `listen` sur mesure pour capturer le routeur. Un helper rendu par
`charge()` supprime ce bruit — et la tâche 2 va en avoir besoin partout, puisque des
gestes qui se faisaient au bouton se feront désormais au menu.

**Files:**
- Modify: `app/tests/dom_shim.js:130-213`
- Modify: `app/tests/cycle_de_vie.test.js`, `couverture.test.js`, `composition.test.js`, `epreuve.test.js`, `packages.test.js` (suppression des listes `IDS`)

- [ ] **Step 1: Écrire le test qui échoue**

Créer `app/tests/dom_shim.test.js` :

```js
'use strict';

// Le faux DOM est l'outil des autres tests : ce qu'il promet doit être vérifié une
// fois ici, plutôt que supposé quarante fois ailleurs.

const test = require('node:test');
const assert = require('node:assert');
const { charge } = require('./dom_shim');

/** Le strict minimum pour que app.js démarre et ouvre un projet sans lever. */
const PROJET = {
  chemin: '/livres/LHC.ozalid',
  livre: {
    titre: 'Les Heures creuses', titre_page: null, auteur: 'Ivan Pjig',
    genre: 'roman', copyright: '', chapitres: null,
  },
  manuscrit_source: null,
  chapitres_trouves: 12,
  mots: 42000,
  manuscrit_absent: false,
  modifie: false,
  couverture: null,
  couverture_importee: false,
  images: [],
  interieur: { police: 'EB Garamond' },
};

const invokeMuet = async (cmd) => {
  switch (cmd) {
    case 'providers_liste': return [{
      cle: 'lulu', libelle: 'Lulu — poche 108 × 175',
      largeur: 108, hauteur: 175, fond_perdu: 3.175, dos_publie: true,
      papiers: [{ cle: 'standard', libelle: 'Papier standard' }],
    }];
    case 'polices_liste': return ['Bodoni Moda'];
    case 'polices_texte_liste': return ['EB Garamond'];
    case 'maquettes_liste': return [];
    case 'recents_liste': return [];
    case 'interface_prete': return null;
    // Sans elle, la garde refuserait de laisser passer et « Nouveau projet » ne
    // ferait rien : le test du helper de menu se vérifierait lui-même.
    case 'garde_modifications': return 'ignorer';
    case 'couverture_apercu': throw new Error('pas de maquette');
    default: return PROJET;
  }
};

test('sans liste d\'identifiants, tous ceux du vrai HTML sont là', async () => {
  const { els } = await charge({ invoke: invokeMuet });

  assert.ok(els.get('inTitre'), 'un identifiant du HTML manque au faux DOM');
  assert.ok(els.get('btNouveau'));
  assert.equal(els.get('btReimporter').disabled, true,
    'l\'état initial doit toujours venir du HTML');
});

test('le helper de menu actionne le routeur de l\'application', async () => {
  const appels = [];
  const { menu } = await charge({
    invoke: async (cmd, args) => { appels.push(cmd); return invokeMuet(cmd, args); },
  });

  await menu('fichier.nouveau');

  assert.ok(appels.includes('projet_nouveau'));
});

/**
 * Un test qui pose son propre `listen` remplace celui du faux DOM. Le helper n'a alors
 * plus rien à actionner : mieux vaut le dire que rendre un geste silencieusement inerte.
 */
test('un listen sur mesure fait dire au helper pourquoi il ne peut rien', async () => {
  const { menu } = await charge({
    invoke: invokeMuet,
    listen: async () => () => {},
  });

  await assert.rejects(() => menu('fichier.nouveau'), /listen/);
});
```

- [ ] **Step 2: Lancer le test pour le voir échouer**

```
cd app && node --test tests/dom_shim.test.js
```

Attendu : ÉCHEC — `charge()` exige aujourd'hui `ids`, et `menu` n'existe pas dans ce
qu'elle rend (`TypeError: menu is not a function`).

- [ ] **Step 3: Implémenter**

Dans `app/tests/dom_shim.js`, ajouter après `depuisHtml` :

```js
/**
 * Tous les identifiants posés dans le vrai index.html.
 *
 * Les énumérer dans chaque fichier de test revenait à tenir à la main une copie du
 * balisage : une section renommée s'y voyait en `null` sans message, cinq fois de
 * suite. Le faux DOM lit déjà le HTML pour connaître l'état initial de chaque élément ;
 * qu'il en lise aussi la liste ne fait qu'aller au bout de la même idée.
 */
function idsDuHtml(html) {
  return [...html.matchAll(/\bid="([^"]+)"/g)].map((m) => m[1]);
}
```

Puis remplacer la signature et le corps de `charge` :

```js
/**
 * Charge src/app.js dans un contexte muni d'un faux DOM.
 * `ids` : identifiants à créer ; par défaut, tous ceux du vrai index.html, avec leur
 * balise et leur état initial. `invoke` : implémentation des commandes Rust.
 */
async function charge({
  ids,
  invoke,
  open = async () => null,
  save = async () => null,
  listen,
  destroy = () => {},
}) {
  const html = fs.readFileSync(
    path.join(__dirname, '..', 'src', 'index.html'),
    'utf8'
  );
  const els = new Map(
    (ids ?? idsDuHtml(html)).map((id) => {
      const { tag, ...etat } = depuisHtml(html, id);
      return [id, Object.assign(new El(tag), { id }, etat)];
    })
  );
  // Les écouteurs que l'application pose, retenus pour que les tests puissent les
  // actionner : le menu natif et la fermeture de fenêtre n'ont pas d'autre porte.
  const ecouteurs = {};
  const listenUtilise = listen ?? (async (nom, fn) => {
    ecouteurs[nom] = fn;
    return () => {};
  });
```

(le reste du corps est inchangé jusqu'au `return`, avec `listen: listenUtilise` dans le
faux `window.__TAURI__.event`)

Dans l'objet `contexte`, remplacer `event: { listen }` par `event: { listen: listenUtilise }`.

Et remplacer le `return { els, contexte };` final par :

```js
  const declencheEvenement = async (nom, charge) => {
    const fn = ecouteurs[nom];
    if (!fn) {
      throw new Error(
        `aucun écouteur « ${nom} » : un listen sur mesure a-t-il remplacé celui du faux DOM ?`
      );
    }
    await fn(charge);
  };

  return {
    els,
    contexte,
    /** Ce que fait une entrée de menu, désignée par son identifiant côté Rust. */
    menu: (id) => declencheEvenement('menu', { payload: id }),
    /** La fenêtre demande à se fermer. */
    fermeture: () => declencheEvenement('fermeture-demandee', {}),
  };
}
```

Exporter `idsDuHtml` avec le reste : `module.exports = { El, charge, idsDuHtml };`

- [ ] **Step 4: Lancer le test pour le voir passer**

```
cd app && node --test tests/dom_shim.test.js
```

Attendu : 3 tests, 0 échec.

- [ ] **Step 5: Retirer les listes `IDS` des cinq fichiers de test**

Dans chacun de `cycle_de_vie.test.js`, `couverture.test.js`, `composition.test.js`,
`epreuve.test.js`, `packages.test.js` :

1. supprimer la déclaration `const IDS = [ … ];` ;
2. supprimer `ids: IDS,` de tous les appels à `charge({ … })`.

Puis remplacer les captures de routeur par le helper. Le motif à faire disparaître,
partout où il apparaît :

```js
  let router;
  const { els } = await charge({
    ids: IDS,
    invoke: a.invoke,
    listen: async (nom, fn) => { if (nom === 'menu') router = fn; return () => {}; },
  });
  await router({ payload: 'fichier.nouveau' });
```

devient :

```js
  const { els, menu } = await charge({ invoke: a.invoke });
  await menu('fichier.nouveau');
```

Même chose pour `fermeture-demandee` : `const { fermeture } = await charge({ … })` puis
`await fermeture()`. Une seule exception, à laisser telle quelle : le test
`l'interface ne s'annonce qu'une fois ses écouteurs posés` (`cycle_de_vie.test.js`), qui
pose délibérément son propre `listen` pour observer l'ordre — c'est son objet même.

- [ ] **Step 6: Lancer toute la suite**

```
cd app && node --test "tests/*.test.js"
```

Attendu : 63 tests (60 + les 3 neufs), 0 échec. Aucun comportement de l'application n'a
changé : c'est le filet qui vient d'être retendu, pas le code.

- [ ] **Step 7: Commit**

```bash
cd /Users/jean-paulgavini/Documents/Dev/ozalid
git add app/tests
git commit -m "Le faux DOM n'a plus à recopier ce que le HTML dit déjà"
```

---

## Task 2: Quatre étapes, un accueil, une entête qui porte l'alerte

Le cœur du lot. Les huit sections deviennent quatre étapes plus un accueil ; l'entête
porte le titre, le chemin, l'état d'enregistrement et **l'alerte** — celle-ci résout au
passage un contorsion du lot 1 : `tente()` choisissait sa cible selon qu'une section
était masquée ou non, parce qu'aucune bande n'était visible depuis partout. Il y en a une
maintenant.

Cette tâche ne change **aucune** mise en page : le document défile encore comme avant.
C'est la tâche 6 qui pose la grille. Séparer les deux permet de vérifier la navigation
sans que le CSS n'y soit pour rien.

**Files:**
- Modify: `app/src/index.html` (réécrit en entier)
- Modify: `app/src/styles.css` (renommage de la règle `section`, entête, onglets, pied)
- Modify: `app/src/app.js`
- Create: `app/tests/coquille.test.js`
- Modify: `app/tests/cycle_de_vie.test.js`, `composition.test.js`, `epreuve.test.js`

- [ ] **Step 1: Écrire les tests qui échouent**

Créer `app/tests/coquille.test.js` :

```js
'use strict';

// La coquille : ce qui est montré, et quand. Une seule étape à la fois, aucune sans
// projet, et le même code derrière l'onglet et derrière le menu. La mise en page,
// elle, se vérifie dans l'application — pas ici.

const test = require('node:test');
const assert = require('node:assert');
const { charge } = require('./dom_shim');

const LULU = {
  cle: 'lulu', libelle: 'Lulu — poche 108 × 175',
  largeur: 108, hauteur: 175, fond_perdu: 3.175, dos_publie: true,
  papiers: [{ cle: 'standard', libelle: 'Papier standard' }],
};

function projet(sur = {}) {
  return {
    chemin: '/livres/LHC.ozalid',
    livre: {
      titre: 'Les Heures creuses', titre_page: null, auteur: 'Ivan Pjig',
      genre: 'roman', copyright: '', chapitres: null,
    },
    manuscrit_source: null,
    chapitres_trouves: 12,
    mots: 42000,
    manuscrit_absent: false,
    modifie: false,
    couverture: null,
    couverture_importee: false,
    images: [],
    interieur: { police: 'EB Garamond' },
    ...sur,
  };
}

function atelier({ recents = [], sur = {} } = {}) {
  const appels = [];
  const invoke = async (cmd, args) => {
    appels.push([cmd, args]);
    switch (cmd) {
      case 'providers_liste': return [LULU];
      case 'polices_liste': return ['Bodoni Moda'];
      case 'polices_texte_liste': return ['EB Garamond'];
      case 'maquettes_liste': return [];
      case 'recents_liste': return recents;
      case 'garde_modifications': return 'ignorer';
      case 'projet_fermer': return null;
      case 'interface_prete': return null;
      case 'couverture_apercu': throw new Error('pas de maquette');
      default: return projet(sur);
    }
  };
  return { appels, invoke, noms: () => appels.map(([c]) => c) };
}

const ETAPES = ['livre', 'interieur', 'couverture', 'livraison'];
const montree = (els) =>
  ETAPES.filter((c) => els.get(`etape${c[0].toUpperCase()}${c.slice(1)}`).hidden === false);

test('sans projet, l\'accueil s\'offre et les onglets sont inertes', async () => {
  const a = atelier();
  const { els } = await charge({ invoke: a.invoke });

  assert.equal(els.get('accueil').hidden, false);
  assert.deepEqual(montree(els), [], 'une étape est montrée sans projet');
  for (const cle of ETAPES) {
    assert.equal(els.get(`onglet-${cle}`).disabled, true, `onglet ${cle} actif sans projet`);
  }
});

test('ouvrir un projet retire l\'accueil et montre la première étape', async () => {
  const a = atelier();
  const { els } = await charge({ invoke: a.invoke });

  await els.get('btNouveau').declenche('click');

  assert.equal(els.get('accueil').hidden, true);
  assert.deepEqual(montree(els), ['livre']);
  assert.equal(els.get('onglet-livre').getAttribute('aria-selected'), 'true');
  assert.equal(els.get('titreLivre').textContent, 'Les Heures creuses');
});

test('une seule étape est montrée à la fois', async () => {
  const a = atelier();
  const { els } = await charge({ invoke: a.invoke });
  await els.get('btNouveau').declenche('click');

  await els.get('onglet-couverture').declenche('click');

  assert.deepEqual(montree(els), ['couverture']);
  assert.equal(els.get('onglet-livre').getAttribute('aria-selected'), 'false');
  assert.equal(els.get('onglet-couverture').getAttribute('aria-selected'), 'true');
});

/**
 * Le menu et l'onglet doivent appeler la même fonction. Deux implémentations
 * dériveraient, et c'est la leçon que le lot 1 a déjà payée sur « Enregistrer ».
 */
test('le menu « Aller » montre la même étape que l\'onglet', async () => {
  const a = atelier();
  const { els, menu } = await charge({ invoke: a.invoke });
  await els.get('btNouveau').declenche('click');

  await menu('aller.livraison');

  assert.deepEqual(montree(els), ['livraison']);
  assert.equal(els.get('onglet-livraison').getAttribute('aria-selected'), 'true');
});

/**
 * Les onglets sont grisés sans projet ; le menu, lui, offre toujours ses entrées.
 * Sans garde ici, ⌘3 sur l'accueil montrerait une étape vide — et une exception
 * remonterait dans le rappel de `listen`, que personne n'attrape.
 */
test('sans projet, « Aller » ne montre rien et ne lève rien', async () => {
  const a = atelier();
  const { els, menu } = await charge({ invoke: a.invoke });

  await menu('aller.couverture');

  assert.equal(els.get('accueil').hidden, false);
  assert.deepEqual(montree(els), []);
});

/**
 * L'étape courante appartient au projet qu'on regardait. Rester sur la Livraison en
 * ouvrant un autre livre donnerait à lire ses packages sous le titre du nouveau.
 */
test('ouvrir un autre projet ramène à la première étape', async () => {
  const a = atelier();
  const { els, menu } = await charge({
    invoke: a.invoke,
    open: async () => '/livres/B.ozalid',
  });
  await els.get('btNouveau').declenche('click');
  await els.get('onglet-livraison').declenche('click');

  await menu('fichier.ouvrir');

  assert.deepEqual(montree(els), ['livre']);
});

test('fermer le projet rend l\'accueil et éteint les onglets', async () => {
  const a = atelier();
  const { els, menu } = await charge({ invoke: a.invoke });
  await els.get('btNouveau').declenche('click');

  await menu('fichier.fermer');

  assert.equal(els.get('accueil').hidden, false);
  assert.deepEqual(montree(els), []);
  assert.equal(els.get('onglet-livre').disabled, true);
  assert.equal(els.get('titreLivre').textContent, 'Ozalid Studio');
});

/**
 * Une erreur survenue à l'étape 4 doit se lire depuis l'étape 1 : l'entête est la
 * seule bande que toutes les étapes partagent, et c'est pour cela qu'elle la porte.
 */
test('une erreur s\'affiche dans l\'entête, visible depuis n\'importe quelle étape', async () => {
  const a = atelier();
  const invoke = async (cmd, args) => {
    if (cmd === 'livre_modifier') throw new Error('titre vide');
    return a.invoke(cmd, args);
  };
  const { els } = await charge({ invoke });
  await els.get('btNouveau').declenche('click');

  await els.get('inTitre').declenche('change');

  assert.match(els.get('alerte').textContent, /titre vide/);
  assert.equal(els.get('alerte').className, 'etat erreur');
});
```

- [ ] **Step 2: Lancer les tests pour les voir échouer**

```
cd app && node --test tests/coquille.test.js
```

Attendu : ÉCHEC de tous les tests — `els.get('accueil')` rend `null`, les identifiants
`onglet-*` et `etape*` n'existent pas.

- [ ] **Step 3: Réécrire `app/src/index.html`**

Fichier entier :

```html
<!DOCTYPE html>
<html lang="fr">
<head>
<meta charset="utf-8">
<title>Ozalid Studio</title>
<link rel="stylesheet" href="styles.css">
</head>
<body>

<header id="entete">
  <div class="identite">
    <h1 id="titreLivre">Ozalid Studio</h1>
    <p class="chemin" id="cheminProjet">aucun projet ouvert</p>
  </div>
  <div class="etats">
    <p class="etat" id="etatEnregistrement"></p>
    <p class="etat" id="alerte"></p>
  </div>
</header>

<!-- Les onglets sont construits par app.js depuis la table ETAPES. La nav est écrite
     indifférente à sa disposition : passer au rail vertical ne demanderait qu'une autre
     valeur de --coquille et un autre sens de flux, pas un autre balisage. -->
<nav id="etapes" role="tablist" aria-label="Étapes du livre"></nav>

<main id="contenu">

  <section id="accueil">
    <div class="bloc">
      <h2>Projet</h2>
      <div class="ligne">
        <button id="btNouveau" type="button">Nouveau projet</button>
        <button id="btOuvrir" type="button">Ouvrir un .ozalid…</button>
        <button id="btImporter" type="button">Importer un livre.toml…</button>
      </div>
      <div id="recents" class="recents"></div>
    </div>
  </section>

  <section id="etapeLivre" class="etape" role="tabpanel" hidden>
    <div class="bloc">
      <h2>Livre</h2>
      <label><span>Titre</span><input type="text" id="inTitre"></label>
      <label><span>Titre de la page de titre</span>
        <textarea id="inTitrePage" rows="2" placeholder="vide : le titre ci-dessus"></textarea></label>
      <label><span>Auteur</span><input type="text" id="inAuteur"></label>
      <label><span>Genre</span><input type="text" id="inGenre"></label>
      <label><span>Copyright</span><textarea id="inCopyright" rows="3"></textarea></label>
      <label><span>Chapitres attendus</span>
        <input type="number" id="inChapitres" min="1" placeholder="facultatif — contrôle d'intégrité"></label>
    </div>

    <div class="bloc">
      <h2>Manuscrit</h2>
      <p class="note" id="etatManuscrit"></p>
      <p class="chemin" id="sourceManuscrit"></p>
      <div class="ligne">
        <button id="btReimporter" type="button" disabled>Réimporter le manuscrit</button>
        <button id="btChoisirManuscrit" type="button">Choisir un autre manuscrit…</button>
      </div>
    </div>
  </section>

  <section id="etapeInterieur" class="etape" role="tabpanel" hidden>
    <div class="bloc">
      <h2>Intérieur</h2>
      <label><span>Police</span><select id="inPoliceInterieur"></select></label>
      <p class="note">La police fixe la pagination, donc l'épaisseur du dos : en changer
        périme le dos de la dernière composition. La planche reste sans dos tant que
        l'intérieur n'a pas été recomposé.</p>
    </div>

    <!-- Le choix du prestataire vit encore ici. Le lot 3 le remonte au pied, où il
         devient global et cesse d'exister en double avec les cases de la Livraison. -->
    <div class="bloc">
      <h2>Prestataire</h2>
      <label><span>Gabarit</span><select id="inProvider"></select></label>
      <label><span>Papier</span><select id="inPapier"></select></label>
      <p class="note" id="noteFormat"></p>
      <div class="ligne">
        <button id="btComposer" type="button">Composer l'intérieur</button>
        <span id="etat" class="etat"></span>
      </div>
      <div id="resultat" class="resultat" hidden></div>
    </div>

    <div class="bloc">
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
    </div>
  </section>

  <section id="etapeCouverture" class="etape" role="tabpanel" hidden>
    <div class="bloc">
      <div class="ligne">
        <span class="lab">Maquette de départ</span>
        <span id="maquettes" class="ligne"></span>
      </div>
      <div class="ligne">
        <span class="lab">Photos</span>
        <button id="btImageUne" type="button">Image de 1ère…</button>
        <button id="btImageQuatre" type="button">Image de 4ème…</button>
      </div>
      <p class="note" id="etatImages"></p>
      <p class="note" id="etatCouverture"></p>
    </div>

    <div class="couv">
      <div class="face">
        <div class="ligne onglets" id="faces"></div>
        <!-- L'aperçu est centré dans une scène qui, elle, a la hauteur disponible :
             l'image garde ses proportions et ne pousse jamais la fenêtre. -->
        <div class="scene"><img id="apercu" class="apercu" alt="Aperçu de la couverture"></div>
        <p class="note" id="etatApercu"></p>
      </div>
      <div id="reglages" class="reglages"></div>
    </div>
  </section>

  <section id="etapeLivraison" class="etape" role="tabpanel" hidden>
    <div class="bloc">
      <h2>Packages</h2>
      <p class="note">Chaque prestataire coché compose son propre intérieur, donc sa propre
        pagination, donc son propre dos et sa propre planche. Les fichiers sont écrits à côté
        du <code>.ozalid</code>, dans un répertoire par prestataire.</p>
      <div id="listePrestataires" class="prestataires"></div>
      <div class="ligne">
        <button id="btPackager" type="button">Générer les packages</button>
        <span id="etatPackages" class="etat"></span>
      </div>
      <div id="packages" class="resultat" hidden></div>
    </div>
  </section>

</main>

<footer id="pied">
  <p id="piedPrestataire"></p>
</footer>

<script src="couverture.js"></script>
<script src="app.js"></script>
</body>
</html>
```

Trois changements de fond à noter, et rien d'autre n'a bougé dans le contenu des étapes :
le `<h2>Couverture</h2>` disparaît (l'onglet nomme déjà l'étape, et son bloc ne porte
pas d'autre titre) ; le sous-titre `<p class="sous">` de l'ancienne entête disparaît avec
elle ; les boutons « Enregistrer » et « Enregistrer sous… » sont retirés (décision 1).

- [ ] **Step 4: Adapter `app/src/styles.css`**

Renommer la règle des sections et ajouter les bandes. Remplacer :

```css
header {
  padding: 1.4rem 2rem 1rem;
  border-bottom: 1px solid var(--trait);
}
```

par :

```css
#entete {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: 1.5rem;
  padding: 1rem 2rem .8rem;
  border-bottom: 1px solid var(--trait);
}

#entete .etats { text-align: right; }

/* Les onglets des étapes. La disposition tient dans le flux de la grille : un rail
   vertical en changerait le sens, pas le balisage. */
#etapes {
  display: grid;
  grid-auto-flow: var(--onglets-flux);
  grid-auto-columns: 1fr;
  gap: .4rem;
  padding: .5rem 2rem 0;
  border-bottom: 1px solid var(--trait);
}

#etapes button {
  display: grid;
  gap: .1rem;
  justify-items: start;
  padding: .4rem .7rem;
  background: transparent;
  color: var(--encre);
  border-color: transparent;
  border-bottom: 2px solid transparent;
  border-radius: 3px 3px 0 0;
}

#etapes button[aria-selected="true"] {
  border-bottom-color: var(--encre);
  background: #fff;
}

#etapes button:disabled { opacity: .4; cursor: default; }

#pied {
  padding: .5rem 2rem;
  border-top: 1px solid var(--trait);
  color: #7a7368;
  font-size: .85rem;
}

#pied p { margin: 0; }
```

Remplacer la règle `main` et la règle `section` :

```css
main { max-width: 46rem; padding: 0 2rem; }

section { padding: 1.2rem 0; border-bottom: 1px solid var(--trait); }
section:last-child { border-bottom: none; }
```

par :

```css
main { padding: 0 2rem; }

.etape, #accueil { max-width: 46rem; }

.bloc { padding: 1.2rem 0; border-bottom: 1px solid var(--trait); }
.bloc:last-child { border-bottom: none; }
```

Enfin, ajouter dans `:root` la variable du flux des onglets, à côté des couleurs :

```css
  --onglets-flux: column;
```

- [ ] **Step 5: Adapter `app/src/app.js`**

**a.** Insérer un bloc neuf juste après la déclaration de `nb` (ligne ~35), avant
`/* ---------- prestataires ---------- */` :

```js
/* ---------- coquille ---------- */

/**
 * Les quatre étapes, dans l'ordre où le livre se fait : leur clé — celle des entrées
 * `aller.*` du menu, au préfixe près — leur libellé d'onglet, et la section montrée.
 *
 * La table est la seule source : les onglets, le routage du menu et le masquage des
 * sections en sortent tous. Ajouter une étape, c'est une ligne ici, une section dans
 * `index.html` et une entrée dans `menu.rs` — jamais trois listes à tenir d'accord.
 */
const ETAPES = [
  ['livre', '1 · Livre', 'etapeLivre'],
  ['interieur', '2 · Intérieur', 'etapeInterieur'],
  ['couverture', '3 · Couverture', 'etapeCouverture'],
  ['livraison', '4 · Livraison', 'etapeLivraison'],
];

/** L'étape montrée. Sans projet, aucune ne l'est : l'accueil prend leur place. */
let etape = 'livre';

function construireEtapes() {
  for (const [cle, libelle] of ETAPES) {
    const b = h('button', libelle);
    b.type = 'button';
    b.id = `onglet-${cle}`;
    b.setAttribute('role', 'tab');
    b.addEventListener('click', () => allerA(cle));
    $('etapes').append(b);
  }
}

/**
 * Montre une étape.
 *
 * Sans projet, le geste ne fait rien : les onglets sont inertes, mais le menu « Aller »,
 * lui, ne l'est pas. C'est ici que les deux chemins se rejoignent, et c'est le même
 * partage des rôles qu'« Enregistrer » — la protection vit du côté qu'ils ont en commun.
 */
function allerA(cle) {
  if (!projet) return;
  etape = cle;
  majEtapes();
}

/**
 * Onglets, étapes et accueil remis d'accord avec ce qui est ouvert.
 *
 * Une seule étape est montrée à la fois, et aucune sans projet : l'accueil est un état
 * de l'application, pas un écran de plus posé devant les autres.
 */
function majEtapes() {
  $('accueil').hidden = !!projet;
  for (const [cle, , section] of ETAPES) {
    const onglet = $(`onglet-${cle}`);
    onglet.disabled = !projet;
    onglet.setAttribute('aria-selected', String(!!projet && cle === etape));
    $(section).hidden = !projet || cle !== etape;
  }
}

/**
 * L'erreur va dans l'entête, la seule bande que toutes les étapes partagent.
 *
 * Une erreur de la Livraison doit se lire depuis le Livre : elle ne peut donc pas vivre
 * dans une section que le changement d'étape emporte.
 */
function alerter(message) {
  $('alerte').textContent = message;
  $('alerte').className = message ? 'etat erreur' : 'etat';
}
```

**b.** Dans `afficherProjet`, remplacer :

```js
  $('cheminProjet').textContent = p.chemin ?? 'projet non enregistré';
  $('btEnregistrer').disabled = !p.chemin;
  $('btEnregistrerSous').disabled = false;
  $('etatEnregistrement').textContent = p.modifie
    ? 'modifié'
    : (p.chemin ? 'enregistré' : 'jamais enregistré');
  $('recents').hidden = true;
  for (const s of ['secLivre', 'secManuscrit', 'secInterieur', 'secCouverture',
                   'secComposer', 'secPackages', 'secEpreuve']) {
    $(s).hidden = false;
  }
```

par :

```js
  $('titreLivre').textContent = p.livre.titre || 'Sans titre';
  $('cheminProjet').textContent = p.chemin ?? 'projet non enregistré';
  $('etatEnregistrement').textContent = p.modifie
    ? 'modifié'
    : (p.chemin ? 'enregistré' : 'jamais enregistré');
  majEtapes();
```

**c.** Remplacer `tente` en entier :

```js
async function tente(fn) {
  try {
    alerter('');
    await fn();
  } catch (e) {
    alerter(String(e));
    // `afficherProjet` ne touche pas à l'alerte : le message qu'on vient d'écrire y
    // survit au redessin.
    if (projet) afficherProjet(projet);
  }
}
```

(le doc-comment au-dessus reste, moins son paragraphe sur `#etat` masqué, qui n'a plus
d'objet)

**d.** Dans `oublierLesSorties`, remplacer le corps par :

```js
function oublierLesSorties() {
  dosCompose = null;
  // L'étape courante est une sortie comme une autre : elle appartenait au projet qu'on
  // regardait. Rester sur la Livraison en ouvrant un autre livre donnerait à lire ses
  // packages sous le titre du nouveau.
  etape = 'livre';
  for (const id of ['resultat', 'packages']) {
    $(id).replaceChildren();
    $(id).hidden = true;
  }
  $('cheminEpreuve').textContent = '';
  $('etat').textContent = '';
  $('etat').className = 'etat';
  alerter('');
}
```

**e.** Dans `afficherAucunProjet`, remplacer :

```js
  $('cheminProjet').textContent = 'aucun projet ouvert';
  $('etatEnregistrement').textContent = '';
  $('btEnregistrer').disabled = true;
  $('btEnregistrerSous').disabled = true;
  for (const s of ['secLivre', 'secManuscrit', 'secInterieur', 'secCouverture',
                   'secComposer', 'secPackages', 'secEpreuve']) {
    $(s).hidden = true;
  }
```

par :

```js
  $('titreLivre').textContent = 'Ozalid Studio';
  $('cheminProjet').textContent = 'aucun projet ouvert';
  $('etatEnregistrement').textContent = '';
  majEtapes();
```

**f.** Dans `afficherRecents`, supprimer la dernière ligne `box.hidden = !liste.length;`
et le `if (liste.length)` reste tel quel : les récents vivent désormais dans l'accueil,
que `majEtapes()` masque en bloc. Le `box.replaceChildren()` suffit à ne rien montrer
quand la liste est vide.

**g.** Dans `enregistrerQuelquePart` et `enregistrerSous`, remplacer les deux blocs

```js
      $('etat').textContent = String(e);
      $('etat').className = 'etat erreur';
```

par `alerter(String(e));`.

**h.** Dans `MENU`, ajouter les quatre étapes :

```js
const MENU = {
  'fichier.nouveau': nouveau,
  'fichier.ouvrir': ouvrir,
  'fichier.importer': importer,
  'fichier.enregistrer': enregistrerQuelquePart,
  'fichier.enregistrer_sous': enregistrerSous,
  'fichier.fermer': fermer,
  'fichier.quitter': quitter,
  // Les quatre étapes viennent de la table : le menu et les onglets appellent la même
  // fonction, et les identifiants du Rust s'en déduisent au lieu d'être recopiés.
  ...Object.fromEntries(ETAPES.map(([cle]) => [`aller.${cle}`, () => allerA(cle)])),
};
```

**i.** Dans le câblage final, supprimer les deux lignes

```js
$('btEnregistrer').addEventListener('click', enregistrerQuelquePart);
$('btEnregistrerSous').addEventListener('click', enregistrerSous);
```

ajouter `construireEtapes();` juste avant `construireFaces();`, et remplacer les deux
`catch` de démarrage (`menu inopérant` et `démarrage impossible`) par :

```js
    alerter(`menu inopérant : ${e}`);
```

et

```js
    alerter(`démarrage impossible : ${e}`);
```

- [ ] **Step 6: Lancer les tests de la coquille**

```
cd app && node --test tests/coquille.test.js
```

Attendu : 8 tests, 0 échec.

- [ ] **Step 7: Réécrire les assertions devenues fausses dans les tests existants**

```
cd app && node --test "tests/*.test.js"
```

Les échecs attendus, et ce qu'il faut y faire :

| Fichier | Assertion | Devient |
|---|---|---|
| `cycle_de_vie.test.js` (9 sites) | `els.get('secLivre').hidden === true` | `els.get('accueil').hidden === false` |
| `cycle_de_vie.test.js` | `els.get('secLivre').hidden === false` | `els.get('etapeLivre').hidden === false` |
| `cycle_de_vie.test.js` (5 sites) | clics sur `btEnregistrer` / `btEnregistrerSous` | `await menu('fichier.enregistrer')` / `await menu('fichier.enregistrer_sous')` |
| `cycle_de_vie.test.js` | `un projet jamais enregistré n'offre que « Enregistrer sous… »` : assertions `disabled` | l'état d'enregistrement seul (`etatEnregistrement === 'jamais enregistré'`) plus, par le menu, `fichier.enregistrer` sur un projet sans chemin qui appelle bien le sélecteur |
| `cycle_de_vie.test.js:346`, `:375` | `btOuvrir` cliqué avec un projet ouvert | `await menu('fichier.ouvrir')` — l'accueil est masqué, le bouton ne serait pas cliquable |
| `cycle_de_vie.test.js` | `els.get('etat')` dans le test du projet illisible | `els.get('alerte')` |
| `composition.test.js:112` | boucle sur `secLivre`, `secManuscrit`, `secComposer` | `els.get('accueil').hidden === false` et les quatre `etape*` masquées |
| `composition.test.js:115,130,194,195` | `btEnregistrer` / `btEnregistrerSous` `.disabled` | supprimées ; `etatEnregistrement` porte déjà l'information et le test `:191` la vérifie |
| `composition.test.js:129` | `secComposer.hidden === false` | `etapeLivre.hidden === false` (l'étape d'arrivée après ouverture) |
| `composition.test.js:262,263` | `els.get('etat')` sur une erreur de composition | inchangé — `composer()` écrit toujours dans `#etat`, qui vit dans l'étape Intérieur d'où le geste part |
| `composition.test.js:281,282` | `etatEnregistrement` sur un fichier illisible | `alerte` |
| `composition.test.js:283` | `secLivre.hidden === true` | `accueil.hidden === false` |
| `epreuve.test.js:80` | `secInterieur.hidden === false` | `etapeLivre.hidden === false` puis, après `menu('aller.interieur')`, `etapeInterieur.hidden === false` |
| `epreuve.test.js:128,129` | `els.get('etat')` sur une police refusée | `els.get('alerte')` — le refus passe par `tente()` |

Les clics sur `btNouveau`, `btOuvrir` et `btImporter` **qui ouvrent le premier projet**
restent des clics : l'accueil est visible à ce moment-là, le geste est réel.

- [ ] **Step 8: Lancer toute la suite**

```
cd app && node --test "tests/*.test.js"
```

Attendu : 0 échec.

- [ ] **Step 9: Commit**

```bash
cd /Users/jean-paulgavini/Documents/Dev/ozalid
git add app/src app/tests
git commit -m "Le livre se fait en quatre étapes, et l'écran n'en montre qu'une"
```

---

## Task 3: Le menu « Aller » cesse d'être grisé

Le lot 1 a posé les quatre entrées `aller.*` désactivées, avec un commentaire qui dit
pourquoi : « les étapes qu'elles désignent n'existent pas encore ». Elles existent.

**Files:**
- Modify: `app/src-tauri/src/menu.rs:103-131`

- [ ] **Step 1: Retirer les `.enabled(false)` et refaire le commentaire**

Remplacer le bloc :

```rust
    // Désactivées : les étapes qu'elles désignent n'existent pas encore, et le lot
    // suivant les branchera. Une commande sans effet ressemble à une panne ; grisée,
    // elle annonce un chantier.
    let aller = SubmenuBuilder::new(app, "Aller")
        .item(
            &MenuItemBuilder::with_id("aller.livre", "Livre")
                .accelerator("CmdOrCtrl+1")
                .enabled(false)
                .build(app)?,
        )
```

(et les trois suivantes) par :

```rust
    // Jamais grisées, même sans projet ouvert : comme « Enregistrer », elles demandent
    // et c'est l'interface qui décide. Sans projet, elle ne montre rien — la garde vit
    // d'un seul côté, celui que le menu et les onglets ont en commun.
    let aller = SubmenuBuilder::new(app, "Aller")
        .item(
            &MenuItemBuilder::with_id("aller.livre", "Livre")
                .accelerator("CmdOrCtrl+1")
                .build(app)?,
        )
        .item(
            &MenuItemBuilder::with_id("aller.interieur", "Intérieur")
                .accelerator("CmdOrCtrl+2")
                .build(app)?,
        )
        .item(
            &MenuItemBuilder::with_id("aller.couverture", "Couverture")
                .accelerator("CmdOrCtrl+3")
                .build(app)?,
        )
        .item(
            &MenuItemBuilder::with_id("aller.livraison", "Livraison")
                .accelerator("CmdOrCtrl+4")
                .build(app)?,
        )
        .build()?;
```

- [ ] **Step 2: Vérifier**

```
cd app/src-tauri && cargo clippy --all-targets && cargo fmt --check && cargo test --lib
```

Attendu : 0 avertissement, format propre, 134 tests passés. (Le menu n'est pas testable
sans fenêtre : il se vérifie à l'écran, tâche 7.)

- [ ] **Step 3: Commit**

```bash
cd /Users/jean-paulgavini/Documents/Dev/ozalid
git add app/src-tauri/src/menu.rs
git commit -m "Aller quelque part suppose qu'il y ait où aller"
```

---

## Task 4: Le pied nomme le prestataire et dit ce que vaut le dos

Le pied porte « pour qui l'on regarde » et « ce que vaut le dos ». Le prestataire y est
encore lu dans le `select` de l'étape Intérieur : le lot 3 déplacera la source, pas
l'affichage.

**Files:**
- Modify: `app/src/app.js`
- Modify: `app/tests/coquille.test.js`

- [ ] **Step 1: Écrire les tests qui échouent**

Ajouter à `app/tests/coquille.test.js` :

```js
/* ---------- le pied ---------- */

const COMPOSITION = {
  pages: 262, chapitres: 12, gouttiere: 25, blanche: true,
  dos: 16.513, pdf: '/livres/LHC/lulu/interieur-lulu.pdf',
};

test('le pied nomme le prestataire et dit le dos non composé', async () => {
  const a = atelier();
  const { els } = await charge({ invoke: a.invoke });
  await els.get('btNouveau').declenche('click');

  assert.equal(els.get('piedPrestataire').textContent,
    'Vu pour : Lulu — poche 108 × 175 · dos non composé');
});

/**
 * Le dos affiché au pied vient de la pagination mesurée, jamais d'une saisie. C'est la
 * même règle que pour l'aperçu de planche, et pour la même raison : un dos inventé se
 * voit au massicot, jamais avant.
 */
test('une fois l\'intérieur composé, le pied porte le dos mesuré', async () => {
  const a = atelier();
  const invoke = async (cmd, args) => {
    if (cmd === 'composer') return COMPOSITION;
    return a.invoke(cmd, args);
  };
  const { els } = await charge({ invoke });
  await els.get('btNouveau').declenche('click');

  await els.get('btComposer').declenche('click');

  assert.equal(els.get('piedPrestataire').textContent,
    'Vu pour : Lulu — poche 108 × 175 · dos 16,5 mm');
});

test('sans projet, le pied ne prétend rien', async () => {
  const a = atelier();
  const { els } = await charge({ invoke: a.invoke });

  assert.equal(els.get('piedPrestataire').textContent, '');
});
```

- [ ] **Step 2: Lancer pour voir échouer**

```
cd app && node --test tests/coquille.test.js
```

Attendu : ÉCHEC — `piedPrestataire` reste vide.

- [ ] **Step 3: Implémenter**

Dans `app.js`, ajouter à la fin du bloc `/* ---------- coquille ---------- */` :

```js
/**
 * Le pied : pour qui l'on regarde, et ce que vaut le dos.
 *
 * Le prestataire y est nommé une fois pour toute la fenêtre. Le dos n'y paraît que s'il
 * vaut pour ce qui est montré — c'est `dosCourant()` qui en répond — parce qu'un dos
 * périmé écrit en bas de l'écran est exactement ce qu'on ne relirait pas.
 */
function majPied() {
  if (!projet) {
    $('piedPrestataire').textContent = '';
    return;
  }
  const dos = dosCourant();
  $('piedPrestataire').textContent = `Vu pour : ${providerCourant().libelle} · `
    + (dos === null ? 'dos non composé' : `dos ${nb(dos, 1)} mm`);
}
```

Puis appeler `majPied()` :

- à la fin de `afficherProjet` (après `demanderApercu();`) ;
- à la fin de `afficherAucunProjet` (après `await afficherRecents();`) ;
- dans l'écouteur `change` de `inProvider`, après `majPapiers();` ;
- dans l'écouteur `change` de `inPapier`, avant `demanderApercu()` — remplacer
  `$('inPapier').addEventListener('change', demanderApercu);` par :

```js
// Le papier ne change ni le format ni la maquette : il ne touche que le dos, et c'est
// pour cela seul que l'aperçu et le pied doivent repartir.
$('inPapier').addEventListener('change', () => {
  majPied();
  demanderApercu();
});
```

- dans `composer()`, dans le `finally`, juste après `bt.disabled = false;` — c'est le
  seul endroit qui couvre à la fois la réussite (le dos vient d'arriver) et l'échec (le
  dos précédent, s'il y en avait un, n'a pas bougé).

- [ ] **Step 4: Lancer pour voir passer**

```
cd app && node --test "tests/*.test.js"
```

Attendu : 0 échec.

- [ ] **Step 5: Commit**

```bash
cd /Users/jean-paulgavini/Documents/Dev/ozalid
git add app/src/app.js app/tests/coquille.test.js
git commit -m "Le bas de l'écran dit pour qui l'on regarde, et ce que vaut le dos"
```

---

## Task 5: Les témoins d'attention sur les onglets

Trois témoins, ceux de la spec, et un sous-libellé qui dit l'état de chaque étape.

**Files:**
- Modify: `app/src/couverture.js` (un helper de libellé)
- Modify: `app/src/app.js`
- Modify: `app/src/styles.css`
- Modify: `app/tests/coquille.test.js`

- [ ] **Step 1: Écrire les tests qui échouent**

Ajouter à `app/tests/coquille.test.js` :

```js
/* ---------- témoins ---------- */

const KDP = {
  cle: 'kdp-6x9', libelle: 'KDP 6 × 9',
  largeur: 152.4, hauteur: 228.6, fond_perdu: 3.175, dos_publie: true,
  papiers: [{ cle: 'blanc', libelle: 'Blanc' }],
};

const sous = (els, cle) => els.get(`sous-${cle}`).textContent;
const alerte = (els, cle) => els.get(`onglet-${cle}`).className === 'alerte';

test('l\'onglet Livre dit l\'état du manuscrit sans crier', async () => {
  const a = atelier();
  const { els } = await charge({ invoke: a.invoke });
  await els.get('btNouveau').declenche('click');

  assert.equal(sous(els, 'livre'), '12 chapitres');
  assert.equal(alerte(els, 'livre'), false);
});

/**
 * L'écart avec le contrôle d'intégrité est le seul signe qu'un manuscrit périmé
 * laisse : le gabarit, la police et le papier, eux, n'ont pas bougé.
 */
test('un écart de contrôle d\'intégrité allume le témoin du Livre', async () => {
  const a = atelier({ sur: { chapitres_trouves: 2, livre: {
    titre: 'Les Heures creuses', titre_page: null, auteur: 'Ivan Pjig',
    genre: 'roman', copyright: '', chapitres: 64,
  } } });
  const { els } = await charge({ invoke: a.invoke });
  await els.get('btNouveau').declenche('click');

  assert.equal(sous(els, 'livre'), '2 chapitres, 64 attendus');
  assert.equal(alerte(els, 'livre'), true);
});

/** Un manuscrit absent est un état de projet neuf, pas une anomalie à signaler. */
test('un manuscrit absent se dit, sans allumer de témoin', async () => {
  const a = atelier({ sur: { manuscrit_absent: true, chapitres_trouves: 0, mots: 0 } });
  const { els } = await charge({ invoke: a.invoke });
  await els.get('btNouveau').declenche('click');

  assert.equal(sous(els, 'livre'), 'aucun manuscrit');
  assert.equal(alerte(els, 'livre'), false);
});

test('sans maquette, l\'onglet Couverture le dit et s\'allume', async () => {
  const a = atelier();
  const { els } = await charge({ invoke: a.invoke });
  await els.get('btNouveau').declenche('click');

  assert.equal(sous(els, 'couverture'), 'aucune maquette');
  assert.equal(alerte(els, 'couverture'), true);
});

test('une maquette en place nomme son mode et éteint le témoin', async () => {
  const a = atelier({ sur: { couverture: { mode: 'bandeau' } } });
  const { els } = await charge({ invoke: a.invoke });
  await els.get('btNouveau').declenche('click');

  assert.equal(sous(els, 'couverture'), 'Bandeau');
  assert.equal(alerte(els, 'couverture'), false);
});

/**
 * Changer de gabarit périme le dos : le même manuscrit ne fait pas le même nombre de
 * pages en poche et en grand format. Le témoin dit où le réparer — à l'Intérieur, la
 * seule étape qui recompose.
 */
test('un dos périmé par un changement de gabarit allume le témoin de l\'Intérieur', async () => {
  const a = atelier();
  const invoke = async (cmd, args) => {
    if (cmd === 'providers_liste') return [LULU, KDP];
    if (cmd === 'composer') return COMPOSITION;
    return a.invoke(cmd, args);
  };
  const { els } = await charge({ invoke });
  await els.get('btNouveau').declenche('click');
  await els.get('btComposer').declenche('click');
  assert.equal(alerte(els, 'interieur'), false, 'un dos frais ne périme rien');

  els.get('inProvider').value = 'kdp-6x9';
  await els.get('inProvider').declenche('change');

  assert.equal(sous(els, 'interieur'), 'dos périmé');
  assert.equal(alerte(els, 'interieur'), true);
});

test('sans composition, l\'onglet Intérieur nomme la police et n\'alerte pas', async () => {
  const a = atelier();
  const { els } = await charge({ invoke: a.invoke });
  await els.get('btNouveau').declenche('click');

  assert.equal(sous(els, 'interieur'), 'EB Garamond');
  assert.equal(alerte(els, 'interieur'), false);
});
```

- [ ] **Step 2: Lancer pour voir échouer**

```
cd app && node --test tests/coquille.test.js
```

Attendu : ÉCHEC — `sous-livre` n'existe pas.

- [ ] **Step 3: Implémenter le libellé de mode dans `couverture.js`**

Ajouter avant `module.exports` :

```js
/**
 * Le libellé public d'un mode de page, lu dans le schéma.
 *
 * Recopié ailleurs, il dériverait du jour où un mode change de nom : le schéma est la
 * seule liste où ces trois mots sont écrits.
 */
function libelleMode(mode) {
  const champ = groupes()
    .flatMap((g) => g.champs)
    .find((c) => c.chemin === 'mode');
  return champ.options.find(([v]) => v === mode)?.[1] ?? mode;
}
```

et l'exporter : `module.exports = { SCHEMA, groupes, lire, ecrire, libelleMode };`

- [ ] **Step 4: Implémenter les témoins dans `app.js`**

**a.** Remplacer `construireEtapes` :

```js
function construireEtapes() {
  for (const [cle, libelle] of ETAPES) {
    const b = h('button');
    b.type = 'button';
    b.id = `onglet-${cle}`;
    b.setAttribute('role', 'tab');
    b.append(h('span', libelle, 'nom'));
    // Le sous-libellé porte l'état de l'étape ; il est retrouvable par son identifiant
    // plutôt que par son rang, pour qu'ajouter un élément à l'onglet ne le déplace pas.
    const sous = h('span', '', 'sous');
    sous.id = `sous-${cle}`;
    b.append(sous);
    b.addEventListener('click', () => allerA(cle));
    $('etapes').append(b);
  }
}
```

**b.** Ajouter, avant `majEtapes` :

```js
/**
 * Ce que chaque onglet dit de son étape : un sous-libellé qui énonce où en est le
 * projet, et un témoin quand l'étape réclame attention.
 *
 * Trois témoins, et pas un de plus. Un manuscrit qui ne correspond plus au contrôle
 * d'intégrité ; une couverture sans maquette ; un dos qui ne vaut plus pour ce qui est
 * affiché, et qui s'allume à l'Intérieur parce que c'est là qu'on le répare. Un
 * manuscrit absent n'en est pas un : c'est l'état d'un projet neuf, pas une anomalie.
 */
function etatEtapes(p) {
  const attendu = p.livre.chapitres;
  const ecart = attendu !== null && attendu !== undefined && attendu !== p.chapitres_trouves;
  // Un dos existe et ne vaut plus : ni « jamais composé », qui ne réclame rien, ni
  // « à jour ».
  const dosPerime = dosCompose !== null && dosCourant() === null;
  return {
    livre: {
      sous: ecart
        ? `${p.chapitres_trouves} chapitres, ${attendu} attendus`
        : (p.manuscrit_absent ? 'aucun manuscrit' : `${p.chapitres_trouves} chapitres`),
      alerte: ecart,
    },
    interieur: {
      sous: dosPerime ? 'dos périmé' : p.interieur.police,
      alerte: dosPerime,
    },
    couverture: {
      sous: p.couverture ? libelleMode(p.couverture.mode) : 'aucune maquette',
      alerte: !p.couverture,
    },
    // Rien de vrai à dire avant qu'un package n'ait été généré, et le pied porte déjà
    // le dos : mieux vaut se taire que meubler.
    livraison: { sous: '', alerte: false },
  };
}
```

**c.** Compléter `majEtapes` :

```js
function majEtapes() {
  const etats = projet ? etatEtapes(projet) : null;
  $('accueil').hidden = !!projet;
  for (const [cle, , section] of ETAPES) {
    const onglet = $(`onglet-${cle}`);
    onglet.disabled = !projet;
    onglet.setAttribute('aria-selected', String(!!projet && cle === etape));
    $(section).hidden = !projet || cle !== etape;
    const e = etats?.[cle];
    onglet.className = e?.alerte ? 'alerte' : '';
    $(`sous-${cle}`).textContent = e ? e.sous : '';
  }
}
```

**d.** Le témoin du dos dépend de `dosCompose` et du gabarit courant, que
`afficherProjet` ne voit pas passer. Ajouter `majEtapes();` :

- dans l'écouteur `change` de `inProvider`, après `majPied();` ;
- dans l'écouteur `change` de `inPapier`, après `majPied();` ;
- dans `composer()`, dans le `finally`, après `majPied();`.

- [ ] **Step 5: Le style du témoin**

Dans `styles.css`, à la suite des règles `#etapes` :

```css
#etapes .nom { font-size: .9rem; }
#etapes .sous { font-size: .75rem; color: #7a7368; }

/* Le témoin est un point, pas une couleur de fond : discret, mais lu d'un coup d'œil
   depuis n'importe quelle étape. */
#etapes button.alerte .sous { color: var(--rouge); }
#etapes button.alerte .sous::before { content: "● "; }
```

- [ ] **Step 6: Lancer toute la suite**

```
cd app && node --test "tests/*.test.js"
```

Attendu : 0 échec.

- [ ] **Step 7: Commit**

```bash
cd /Users/jean-paulgavini/Documents/Dev/ozalid
git add app/src app/tests/coquille.test.js
git commit -m "Chaque onglet dit où en est son étape, et se signale quand elle réclame"
```

---

## Task 6: La fenêtre cesse de défiler

Le CSS seul. Aucune assertion automatique ne le couvre — la mise en page se vérifie
dans l'application, à la tâche 7. Le faire à part garantit qu'aucun test cassé ici ne
vienne d'ailleurs.

**Files:**
- Modify: `app/src/styles.css`

- [ ] **Step 1: Poser la grille de la coquille**

Dans `:root`, ajouter à côté de `--onglets-flux` :

```css
  /* La coquille, en quatre bandes à la hauteur exacte de la fenêtre. Passer au rail
     vertical serait une autre valeur ici et un `--onglets-flux: row` : le balisage,
     lui, ne bougerait pas. */
  --coquille:
    "entete"  auto
    "etapes"  auto
    "contenu" minmax(0, 1fr)
    "pied"    auto / 1fr;
```

Remplacer la règle `body` :

```css
body {
  margin: 0;
  padding: 0 0 3rem;
  font: 14px/1.5 -apple-system, "Segoe UI", system-ui, sans-serif;
  color: var(--encre);
  background: var(--papier);
}
```

par :

```css
body {
  margin: 0;
  /* La fenêtre ne défile plus : ce qui ne tient pas se règle par la mise en page, et
     la seule zone défilante de l'application est le panneau de réglages de la
     couverture — sa longueur, elle, est irréductible. */
  height: 100vh;
  overflow: hidden;
  display: grid;
  grid-template: var(--coquille);
  font: 14px/1.5 -apple-system, "Segoe UI", system-ui, sans-serif;
  color: var(--encre);
  background: var(--papier);
}

#entete { grid-area: entete; }
#etapes { grid-area: etapes; }
#contenu { grid-area: contenu; }
#pied { grid-area: pied; }
```

- [ ] **Step 2: Donner sa hauteur à l'étape montrée**

Remplacer :

```css
main { padding: 0 2rem; }

.etape, #accueil { max-width: 46rem; }
```

par :

```css
main {
  padding: 0 2rem;
  min-height: 0;
  overflow: hidden;
}

/* Une seule étape est montrée à la fois : elle prend toute la bande. `min-height: 0`
   sans quoi une grille imbriquée refuserait de se laisser comprimer et repousserait
   le pied hors de l'écran. */
.etape, #accueil { height: 100%; min-height: 0; }

/* Ces trois-là tiennent dans la fenêtre, et doivent continuer à y tenir : l'`auto`
   n'est pas un ascenseur offert, c'est un filet. Un contrôle qu'on ne peut plus
   atteindre est un piège ; une barre qui apparaît est un défaut de mise en page qui
   se voit. Si elle apparaît, c'est la mise en page qu'on corrige, pas le filet qu'on
   garde. */
#accueil, #etapeLivre, #etapeInterieur { max-width: 46rem; overflow-y: auto; }

/* L'étape Couverture donne sa hauteur au panneau : un bloc de réglages en haut, la
   couverture et son panneau dessous. */
#etapeCouverture {
  display: grid;
  grid-template-rows: auto minmax(0, 1fr);
}

/* Dette à date de péremption connue : la table compte quatorze gabarits, et la liste
   ne tient pas dans la fenêtre à sa taille minimale. Le lot 3 la réduit aux seuls
   destinataires du livre — une poignée — et cet ascenseur disparaîtra avec lui. */
#etapeLivraison { max-width: 46rem; overflow-y: auto; }
```

- [ ] **Step 3: Le panneau de couverture, seule zone défilante assumée**

Remplacer :

```css
.couv {
  display: grid;
  grid-template-columns: minmax(14rem, 22rem) 1fr;
  gap: 1.5rem;
  margin-top: 1rem;
  align-items: start;
}
```

par :

```css
.couv {
  display: grid;
  grid-template-columns: minmax(14rem, 22rem) 1fr;
  gap: 1.5rem;
  padding: 1rem 0;
  min-height: 0;
}

/* La face : ses onglets en haut, son état en bas, et l'aperçu qui prend le reste. */
.face {
  display: grid;
  grid-template-rows: auto minmax(0, 1fr) auto;
  gap: .8rem;
  min-height: 0;
}

/* La scène a la hauteur ; l'image y garde ses proportions. Sans elle, une planche
   haute pousserait la fenêtre, et un `object-fit` laisserait le fond blanc et l'ombre
   déborder de la couverture. */
.scene {
  display: flex;
  align-items: flex-start;
  justify-content: center;
  min-height: 0;
  overflow: hidden;
}
```

Remplacer :

```css
.apercu {
  display: block;
  width: 100%;
  margin-top: .8rem;
  background: #fff;
  box-shadow: 0 8px 20px -8px rgba(0, 0, 0, .4);
}

.reglages {
  max-height: 34rem;
  overflow-y: auto;
  padding-right: .6rem;
}
```

par :

```css
.apercu {
  display: block;
  max-width: 100%;
  max-height: 100%;
  width: auto;
  height: auto;
  background: #fff;
  box-shadow: 0 8px 20px -8px rgba(0, 0, 0, .4);
}

/* La seule zone défilante de l'application, et elle défile sur la hauteur de la
   fenêtre — plus sur un `max-height` en rem qui ignorait l'écran qu'on a. */
.reglages {
  min-height: 0;
  overflow-y: auto;
  padding-right: .6rem;
}
```

- [ ] **Step 4: Vérifier qu'aucun test n'a bougé**

```
cd app && node --test "tests/*.test.js"
```

Attendu : 0 échec (le CSS n'est pas testé ; c'est la non-régression du JS qu'on vérifie).

- [ ] **Step 5: Vérifier à l'écran**

```
cd app/src-tauri && cargo run
```

Sur un projet réel, à 900 px puis à 1400 px de large :

- la fenêtre elle-même ne défile pas ;
- les étapes Livre, Intérieur et Couverture tiennent sans ascenseur ;
- le panneau de réglages défile, et lui seul, sur toute la hauteur disponible ;
- l'aperçu de planche ne déborde ni en hauteur ni en largeur, ombre comprise ;
- le pied reste visible en permanence.

Si l'écran de veille empêche l'application de créer sa fenêtre :
`caffeinate -u -t 300` et `killall ScreenSaverEngine` avant de lancer.

- [ ] **Step 6: Commit**

```bash
cd /Users/jean-paulgavini/Documents/Dev/ozalid
git add app/src/styles.css
git commit -m "La fenêtre tient dans la fenêtre, et un seul panneau défile"
```

---

## Task 7: Vérification de bout en bout

**Files:**
- Modify: `app/README.md`

- [ ] **Step 1: Toute la garde automatique**

```
cd app/src-tauri && cargo test --lib && cargo clippy --all-targets && cargo fmt --check
cd app && node --test "tests/*.test.js"
```

Attendu : 134 tests Rust, 0 avertissement clippy, format propre, et 0 échec côté front.
Noter le nombre de tests front obtenu : il remplace les 60 du lot 1 comme référence.

- [ ] **Step 2: Le témoin de non-régression**

```
cd app/src-tauri && cargo run --example temoin
```

Attendu, **à l'unité près** : `98 pages, dos 7,21 mm`, code de sortie 0. Ce lot ne
touche à aucun moteur de composition ; le moindre écart ici est un défaut, pas une
amélioration.

- [ ] **Step 3: Les vérifications à l'écran**

Sur un projet réel, avec un vrai manuscrit :

- [ ] les quatre étapes à 900 px et à 1400 px de large ;
- [ ] le panneau de couverture utilisable à 700 px de contenu (fenêtre réduite au minimum) ;
- [ ] **une seule zone défilante**, l'étape Livraison exceptée (dette du lot 3) — en
      particulier, **aucune barre sur Livre ni sur Intérieur à 900 × 640** : si elle
      paraît, c'est la mise en page qu'il faut reprendre, pas le filet qu'il faut garder ;
- [ ] les trois maquettes × les trois faces, sur les trois étapes concernées ;
- [ ] ⌘1 à ⌘4 changent d'étape, et font ce que font les onglets ;
- [ ] ⌘1 à ⌘4 sans projet ne montrent rien et ne cassent rien ;
- [ ] ⌘C et ⌘V dans un champ de saisie (le menu Édition survit) ;
- [ ] ⌘S et ⇧⌘S, seuls chemins vers l'enregistrement désormais ;
- [ ] la garde à la fermeture et ses trois boutons ;
- [ ] une erreur qui **refuse une saisie** se lit depuis n'importe quelle étape — elle
      monte à l'entête et survit au changement d'étape. *(Cette case disait d'abord « une
      erreur de composition se lit depuis l'étape Livre » : c'était faux, et c'est la case
      qui avait tort. Une erreur de composition reste dans `#etat`, à côté du bouton qui
      l'a lancée — c'est la règle des deux canaux, écrite au-dessus d'`alerter()`.)*
- [ ] le témoin du Livre s'allume sur un manuscrit périmé, celui de la Couverture sans
      maquette, celui de l'Intérieur après un changement de gabarit ;
- [ ] le pied nomme le prestataire et le dos ;
- [ ] fermer le projet rend l'accueil, avec ses récents.

- [ ] **Step 4: Le second témoin, sur un livre réel**

Composer un package complet sur un projet réel avant/après ce lot et comparer le compte
de pages. C'est la garde la plus forte et la moins chère.

- [ ] **Step 5: Mettre le README à jour**

Dans `app/README.md`, la section qui décrit l'écran : la page unique de huit sections
devient quatre étapes, un accueil, une entête et un pied. Mentionner que l'enregistrement
est un geste de menu, et que le sous-menu « Aller » navigue.

- [ ] **Step 6: Commit**

```bash
cd /Users/jean-paulgavini/Documents/Dev/ozalid
git add app/README.md
git commit -m "Le README décrit l'écran qu'on a, pas celui qu'on avait"
```

---

## Ce que ce lot ne fait pas

- **Le prestataire unifié** (lot 3) : le `select` de gabarit reste dans l'étape
  Intérieur, les cases à cocher restent dans la Livraison, et le pied lit le premier.
  Il y a donc toujours deux désignations de prestataire — c'est le lot 3 qui les fond.
- **La palette « atelier neutre »** (lot 4) : le crème `#fcf0d8` et le rouge `#c00000`
  décoratif restent en place. Le rouge du témoin, lui, est déjà à sa place — c'est une
  alerte.
- **Le rail vertical** : `--coquille` et `--onglets-flux` le préparent, rien ne le livre,
  et aucune préférence de coquille n'est offerte.
- **Les vignettes de planche par package** (lot 3).
