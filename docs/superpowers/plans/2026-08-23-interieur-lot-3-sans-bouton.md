# Lot 3 — le bouton disparaît

Spec : `docs/superpowers/specs/2026-08-23-interieur-sans-onglet-design.md`
Précédents : lot 1 (déménagement), lot 2 (le pied).

## Ce que ce lot fait

« Composer l'intérieur » disparaît. La composition part **au chargement d'un manuscrit**
— le geste qui dit « ce livre m'intéresse » — puis se tient à jour toute seule, comme
elle le fait déjà. L'échec monte à la bande d'alerte.

C'est le seul lot qui change le comportement, et le seul qui se révoque d'un `revert`.

## Deux trous que l'écriture du plan a trouvés, et ce qu'on en fait

### 1. Un premier échec n'aurait aucune reprise

`deja_compose` ne se lève qu'à une composition **réussie** (`retenir_mesure`). Si la
toute première échoue — police invalide, compte de chapitres faux —, `veiller()` reste
muette : le livre n'a jamais été composé, donc rien ne part tout seul. Et il n'y a plus
de bouton. **On corrige la cause et il ne se passe rien.**

C'est le vrai défaut de la disparition du bouton, et le plan ne peut pas l'ignorer.

**Correctif : un consentement de session dans le front.** Une variable levée par le
chargement d'un manuscrit, et que `veiller()` accepte au même titre que `deja_compose`.
Corriger la police relance alors la composition, même après un premier échec.

Ce qu'il reste et qu'on assume, à écrire au README : un projet dont la **toute première
composition a échoué**, refermé puis rouvert, ne repart pas tout seul. Il faut recharger
le manuscrit — qui est aussi le geste par lequel on répare un manuscrit fautif. Le porter
dans le `.ozalid` demanderait un champ pour distinguer « on a consenti » de « on a
composé », et `dosPerime` a besoin du second : les deux ne sont pas le même fait.

### 2. Quarante-cinq secondes de rouge, sans rien avoir demandé

`#etat` disait « composition… » à côté du bouton. Le bouton parti, la spec (§ 11) laisse
le pied dire « dos périmé » pendant que Typst tourne. **Vu au lot 1 : c'est illisible
comme état d'attente** — un rouge d'alerte qui dure une minute sur un geste qu'on n'a pas
fait se lit comme une panne, pas comme un travail en cours.

**Écart avec la spec § 3, assumé : le pied gagne un cinquième état, « composition… »**,
en gris, prioritaire sur tous les autres. Ce n'est pas la barre de progression que la
spec refuse — c'est un mot, à l'endroit où le compte rendu vit désormais.

## Tâche 1 : la composition part au manuscrit

**Files:**
- Modify: `app/src/app.js`

- [ ] **Step 1: Le consentement de session**

Une variable à côté de `veilleSuspendue`, levée par le chargement d'un manuscrit et
retombée par `oublierLesSorties` — elle appartient au livre ouvert, pas à la fenêtre.
`veiller()` la lit en plus de `deja_compose`.

Écrire **pourquoi** elle existe : sans elle, un premier échec n'aurait pas de reprise.

- [ ] **Step 2: `manuscritRemplace` déclenche**

```js
function manuscritRemplace(p) {
  oublierLaComposition();
  consenti = true;
  afficherProjet(p);
  recomposer(true);
}
```

`recomposer(true)` court-circuite le garde-fou : la mesure vient d'être effacée de toute
façon, et `veiller()` n'aurait pas encore vu le consentement au moment du rendu.

- [ ] **Step 3: L'import d'un `livre.toml` aussi**

Il ne passe pas par `manuscritRemplace` (`app.js:843`) : un `livre.toml` importé apporte
son manuscrit, et c'est le même geste. **À ne pas oublier** — la spec le signale comme le
déclencheur qu'on rate.

Attention à l'ordre : `oublierLesSorties()` retombe le consentement et arme
`veilleSuspendue`. Le lever **après**.

- [ ] **Step 4: Ce qui ne doit pas partir**

- Un `.ozalid` rouvert ne compose pas. `oublierLesSorties` arme `veilleSuspendue`, et le
  consentement retombe avec lui.
- Un projet neuf n'a pas de manuscrit.
- Un `.ozalid` qui porte `deja_compose` sans mesure repart, comme aujourd'hui : ce
  comportement-là ne change pas.

## Tâche 2 : le bouton meurt, l'échec monte

**Files:**
- Modify: `app/src/index.html`
- Modify: `app/src/app.js`

- [ ] **Step 1: Le balisage**

La `<div class="ligne">` qui porte `btComposer` et `#etat` disparaît. La note qui explique
que changer la police périme le dos **reste** — elle dit toujours vrai, et mieux qu'avant :
c'est elle qui annonce que le changement va lancer quelque chose. La retoucher pour le
dire.

- [ ] **Step 2: `composer()` sans bouton ni témoin**

Plus de `bt.disabled`, plus de `#etat`. L'erreur part à `alerter()`.

Le `finally` garde `majPied()` et `majEtapes()`.

- [ ] **Step 3: `oublierLaComposition` perd `etat`**

Et le commentaire qui explique pourquoi les canaux partent ensemble doit rester juste.

- [ ] **Step 4: Le pied dit « composition… »**

`majPied` lit `compositionEnCours` et le place avant tous les autres états, en gris.
`composer()` appelle donc `majPied()` **au début** aussi, pas seulement dans son
`finally` — sans quoi le mot ne paraîtrait jamais.

- [ ] **Step 5: L'échec qui s'efface, et pourquoi ce n'est pas un défaut**

`essai()` appelle `alerter('')` à chaque tentative : un réglage quelconque efface donc le
message. **Ce n'est pas un trou, c'est une propriété** — tout geste qui l'efface relance
aussi la composition (la mesure est toujours absente, `veiller()` repart), qui le
réécrira si la cause tient. Le message se répare tout seul, avec le décalage du débounce.

À écrire en commentaire, sinon quelqu'un ajoutera une garde qui casserait ça.

## Tâche 3 : les tests

**Files:**
- Modify: `app/tests/packages.test.js` (14 clics)
- Modify: `app/tests/composition.test.js` (8 clics)
- Modify: `app/tests/coquille.test.js` (9 clics)

- [ ] **Step 1: Voir rougir**

- [ ] **Step 2: Un geste au lieu d'un bouton**

Les trente et un `btComposer.declenche('click')` deviennent le geste qui compose
désormais : recharger le manuscrit. Les deux faux le modélisent déjà —
`manuscrit_reimporter` y efface la mesure, et `composer` la repose.

Un helper par fichier, nommé pour dire l'intention :

```js
/** Le geste qui compose depuis que le bouton n'existe plus : charger un manuscrit. */
const faireComposer = (els) => els.get('btReimporter').declenche('click');
```

**Attention** : `manuscritRemplace` appelle `oublierLaComposition()`, qui vide
`#packages`, `#ebooks` et `#resultatEnvois`. Un test qui composait *après* avoir généré
perdrait ses comptes rendus. Vérifier l'ordre dans chaque test touché plutôt que de
substituer en aveugle.

- [ ] **Step 3: Ce qui doit être neuf, et vu échouer**

- Charger un manuscrit compose, sans qu'on ait cliqué (mutation : retirer l'appel).
- **Ouvrir un `.ozalid` ne compose pas** — le test qui protège le pari.
- Importer un `livre.toml` compose (le déclencheur qu'on rate).
- Un premier échec, puis la cause corrigée : la composition repart. C'est le test du
  correctif n° 1, et il n'aurait aucun équivalent sans lui.
- L'échec se lit dans `#alerte`.
- Le pied dit « composition… » pendant, et l'état vrai après.

## Tâche 4 : vérifications, œil, README, commit

- [ ] **Step 1** : `cargo fmt --check`, `clippy -D warnings`, `cargo test`,
  `node --test tests/*.test.js`, `cargo run --example temoin` — **jamais dans un pipe**.
  Le Rust n'est pas touché par ce lot ; le témoin doit être **98 pages, dos 7,21 mm**.

- [ ] **Step 2 : à l'œil** (`touch src/lib.rs && cargo build` d'abord)

1. Projet neuf, choisir un manuscrit : la composition part seule, le pied dit
   « composition… » puis se remplit. Aucun bouton nulle part.
2. Rouvrir un `.ozalid` composé : **rien ne part**, le pied dit ce que l'archive porte.
3. Un compte de chapitres attendu faux : le message monte à l'entête. Le corriger : la
   composition repart d'elle-même.
4. La note sous la police annonce bien que changer la police relance.

- [ ] **Step 3 : le README**

« L'écran » : le compte rendu d'un travail long ne liste plus « composer ». Et la
section du cycle de vie doit dire ce qui déclenche une composition — le manuscrit, puis
la veille — et la réserve du § 1 ci-dessus.

- [ ] **Step 4 : commiter**

## Ce que ce lot ne fait pas

- **La composition n'est pas interruptible.** Une composition partie va au bout. Si
  l'automatisme gêne, c'est l'interruption qu'il faudra écrire, pas le bouton qu'il
  faudra remettre.
- **Un premier échec ne survit pas à la fermeture du projet** comme un état réparable :
  voir la réserve du § 1.
