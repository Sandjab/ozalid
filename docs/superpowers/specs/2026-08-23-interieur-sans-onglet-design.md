# L'intérieur sans onglet — la composition qui ne se demande plus

Date : 2026-08-23
Statut : à valider

## Objectif

L'étape « Intérieur » porte deux champs et deux boutons : une police, un corps
d'épreuve, « Composer l'intérieur », « Tirer une épreuve ». C'est le cinquième d'une
rangée d'onglets qui raconte la fabrication d'un livre, et c'est le seul dont le contenu
tient dans le coin d'un autre.

Cette spec la **supprime**. La police rejoint le Livre, dont elle est un attribut au même
titre que le genre ; l'épreuve rejoint le manuscrit, qu'elle sert à relire ; le compte
rendu de composition descend au pied de fenêtre, où le dos l'attendait déjà ; et le
bouton « Composer » **disparaît sans être remplacé** — la composition part d'elle-même au
chargement d'un manuscrit, puis se tient à jour toute seule.

Ce dernier point est le cœur : ce n'est pas un déménagement, c'est la disparition d'un
geste. Les quatre onglets restants — Livre, Couverture, Livraison, Envois — disent alors
exactement ce qu'ils sont : ce qu'on écrit, ce qu'on dessine, ce qu'on livre, ce qu'on
offre. La pagination n'est plus une étape, elle est une conséquence.

## Ce qui existe déjà, et qu'il ne faut pas réinventer

Trois mécanismes sont en place et portent la moitié du chantier. Les ignorer serait en
écrire un quatrième à côté.

- **`veiller()`** (`app.js:459`) recompose déjà de lui-même, débouncé à 400 ms, dès que la
  mesure du destinataire visé est périmée. Changer la police y passe : `majInterieur` →
  `interieur_modifier` → `afficherProjet` → `veiller()`. **La recomposition automatique
  demandée existe donc déjà**, pour tout sauf la première.
- **`recomposer(force)`** (`app.js:477`) sérialise les compositions — une à la fois, la
  dernière gagne — et se reprogramme quand un réglage a bougé pendant qu'elle tournait.
- **`deja_compose`** (`projet.rs:285`) est le garde-fou qui retient `veiller()` avant le
  premier clic. C'est **lui seul** que cette spec déplace : il ne se lèvera plus sur un
  clic, mais sur un manuscrit chargé.

Et deux données qu'on croyait à ajouter et qui sont déjà là :

- **`chapitres_trouves`** est servi sur `ProjetVue` (`app.js:207`) : le pied peut
  l'afficher sans que rien ne soit persisté ni recalculé.
- **Le chemin du PDF** est dérivable — `sorties_dossier(o, pr.cle)/interieur-{cle}.pdf`
  (`commands.rs:562`). Il n'a pas à entrer dans `Mesure`, donc le format `.ozalid` ne
  bouge pas et `VERSION` non plus.

## Décisions de cadrage (brainstorming du 23/08)

- **Le consentement se déplace sur le manuscrit.** La première composition part au
  chargement d'un manuscrit — le geste qui dit « ce livre m'intéresse » — et non à
  l'ouverture du projet. Un `.ozalid` qu'on rouvre pour regarder une couverture ne fait
  pas tourner Typst. Le bouton « Composer » disparaît entièrement.
- **Le compte rendu descend au pied**, réduit à une légende d'une ligne : pages,
  chapitres, gouttière, dos, et un lien vers le PDF de l'intérieur.
- **Le lien s'appelle « intérieur »**, court, et porte le chemin entier au survol.
- **L'alerte de repli de police va aux deux endroits** : un signe court au pied, qui suit
  dans toutes les étapes ; le détail — quelles familles manquent — sous le sélecteur de
  police dans Livre, à côté de sa cause.
- **L'échec monte à la bande d'alerte** (`alerter()`), avec toutes les autres erreurs de
  la fenêtre. Il n'y a plus de bouton à côté duquel l'écrire, et le message doit survivre
  au changement d'étape.
- **L'épreuve va dans Livre, sous le Manuscrit.** Elle sert à relire le manuscrit, pas à
  livrer le livre : Livraison et Envois traitent du livre fini.
- **« Page blanche de fin » disparaît.** Une parité qu'on regarde une fois ne mérite pas
  une place permanente dans une légende qui suit partout.

## 1. Où va quoi

| Ce qui vit dans Intérieur | Sa destination |
|---|---|
| `<select>` Police + sa note | **Livre**, nouveau bloc « Intérieur » après « Manuscrit » |
| Bouton « Composer l'intérieur » | **supprimé** |
| `#etat` (compte rendu court) | **supprimé** — l'échec monte à `alerter()` |
| `#resultat` (pages, chapitres, gouttière, blanche, dos, chemin) | **pied**, en légende ; « blanche » supprimée |
| Alerte « police introuvable » | **pied** (signe court) **et Livre** (détail) |
| Bloc « Épreuve » entier | **Livre**, après le bloc Manuscrit |

L'ordre du bloc Livre devient : Livre (identité) → Textes dérivés → Manuscrit → Épreuve →
Intérieur. Le manuscrit, ce qu'on en tire pour le relire, et comment on le compose : les
trois derniers blocs se lisent dans cet ordre-là.

## 2. La composition sans bouton

### Le déclencheur

`manuscritRemplace()` (`app.js:825`) est le seul entonnoir par lequel un manuscrit arrive
— réimport et choix d'un autre fichier y passent tous deux. Il gagne un appel :

```
function manuscritRemplace(p) {
  oublierLaComposition();
  afficherProjet(p);
  recomposer(true);        // forcé : la mesure vient d'être effacée de toute façon
}
```

`recomposer(true)` court-circuite le garde-fou de `veiller()`, lève `deja_compose` du
côté Rust comme le faisait le premier clic, et tout ce qui suit se comporte comme
aujourd'hui.

**Un import de `livre.toml` apporte lui aussi un manuscrit** et doit déclencher la même
chose. Ce chemin ne passe pas par `manuscritRemplace` : c'est à vérifier à l'exécution et
à corriger là où le projet importé est posé.

**Un `.ozalid` rouvert ne compose pas.** Sa mesure est dans l'archive et vaut toujours —
c'est l'invariant de `Mesure` (`projet.rs:229` : « une mesure présente vaut toujours »).
S'il porte `deja_compose` sans mesure, `veiller()` le rattrape déjà, et ce comportement ne
change pas.

**Un projet neuf ne compose pas** : il n'a pas de manuscrit.

### Ce qui reste vrai

Tout le reste du dispositif est inchangé. La police modifiée périme la mesure côté Rust,
`veiller()` la voit absente et relance ; le débounce évite la rafale ; `recomposer` évite
les compositions parallèles. **Aucune de ces mécaniques n'est à écrire** — elles tournent
déjà.

### Le risque qu'on prend

Aujourd'hui, une composition ne part jamais sans qu'on l'ait demandée deux fois : le
premier clic, puis chaque geste après lui. Demain, choisir un manuscrit lance Typst. Sur
un manuscrit de trois cents pages, c'est plusieurs dizaines de secondes qui partent sans
qu'on ait rien cliqué. C'est le prix assumé de la disparition du bouton : le geste
« choisir un manuscrit » est jugé suffisamment engageant pour valoir consentement. Si
l'usage dément ce pari, le recours n'est pas de remettre le bouton mais de rendre la
composition interruptible — un autre chantier.

## 3. Le pied de fenêtre

Le pied porte aujourd'hui la visée (`#visee`) et le dos (`#piedDos`). Il portera la visée
et une **légende** : ce que la dernière composition a mesuré pour le destinataire visé.

```
Vu pour [Amazon KDP ▾]     · 214 pages · 12 chapitres · gouttière 5,2 mm · dos 14,1 mm · intérieur
```

- **pages, gouttière, dos** : lus dans `destinataireCourant().compose` et
  `dosCourant()`, comme `piedDos` le fait déjà.
- **chapitres** : `projet.chapitres_trouves`, déjà servi.
- **intérieur** : un lien, chemin entier en `title`. Il ne paraît **que si le PDF
  existe** — un lien vers un fichier effacé à la main est pire que pas de lien.

Le pied a **quatre états**, et un seul à la fois :

1. **Aucun projet** — vide, comme aujourd'hui.
2. **Jamais composé** — « dos non composé » ou « dos relevé sur le gabarit », les deux
   mentions actuelles de `majPied`. Rien d'autre : il n'y a rien de vrai à dire.
3. **Composé** — la légende complète ci-dessus, plus `⚠ repli` en rouge si la dernière
   composition a substitué une police.
4. **Périmé** — « dos périmé », en rouge, à la place des chiffres. C'est le témoin qui
   s'allumait sur l'onglet Intérieur ; il descend ici avec le reste, et les témoins
   d'onglets tombent de trois à deux.

**Ce qui n'y va pas** : l'échec de composition (il monte à `alerter()`) et le détail des
polices manquantes (il vit dans Livre). Le pied est une légende, pas un journal.

### La règle du README, et pourquoi elle autorise ce déplacement

Le README pose que « ce qui rend compte d'un travail long — composer, tirer une épreuve,
générer les packages — reste **à côté du bouton qui l'a lancé** ». La composition n'aura
plus de bouton. Mais la même règle porte déjà son exception : « l'aperçu de couverture
n'est ni l'un ni l'autre : **personne ne l'a demandé**, il se recompose à chaque réglage,
et ce qu'il dit de lui-même se lit sous l'image comme une légende ».

La composition entre exactement dans cette troisième catégorie. Le pied est sa légende,
comme la ligne sous l'aperçu est celle de la couverture. **La règle du README ne change
pas, elle s'applique** — et sa formulation doit être élargie pour le dire.

## 4. Le repli de police, aux deux endroits

Typst n'échoue pas quand une famille manque : il compose dans une écriture de repli, et
son avertissement part sur un `stderr` qu'aucune fenêtre ne montre. C'est un piège connu,
inscrit dans `CLAUDE.md`. Aujourd'hui il n'est dit qu'à un seul endroit — le panneau que
cette spec supprime.

- **Au pied** : `⚠ repli`, en rouge, ajouté à la légende. Court, il suit dans toutes les
  étapes et se voit depuis la Couverture, où l'on regarde le résultat.
- **Dans Livre**, sous le sélecteur de police : la phrase entière, avec les familles
  nommées. C'est là qu'on va réparer.

`polices_introuvables` n'est aujourd'hui que dans le retour de `composer()`, donc perdu à
la réouverture. Deux options, à trancher à l'écriture du plan :

- **Le garder en mémoire d'écran** (une variable, effacée par `oublierLaComposition`) :
  aucun changement de format, mais l'alerte disparaît quand on rouvre le projet — alors
  que le PDF, lui, est toujours faux.
- **L'entrer dans `Mesure`** en `#[serde(default)] Vec<String>` : l'alerte survit à la
  réouverture, ce qui est plus juste, au prix d'un champ de plus dans le `.ozalid`.
  `VERSION` ne bouge pas — le champ arrive avec son défaut, comme tous les autres.

**Recommandation : l'entrer dans `Mesure`.** Une mesure décrit ce que la composition a
produit ; un PDF composé dans une écriture de repli en fait partie. L'oublier à la
réouverture ferait dire au pied que tout va bien devant un fichier qui ne suit pas la
maquette.

## 5. L'échec, à la bande d'alerte

Sans bouton, il n'y a plus de geste de reprise : on corrige la cause — une police
invalide, un compte de chapitres qui ne tombe pas juste, un manuscrit illisible — et
`veiller()` repart de lui-même. Le message, lui, passe par `alerter()` (`app.js:288`),
comme toute erreur de la fenêtre.

C'est cohérent avec le README : « ce qui refuse une saisie monte à l'entête, la seule
bande que toutes les étapes partagent : le geste est fini, et le message doit survivre au
changement d'étape ». Une composition déclenchée depuis la Couverture qui échouerait dans
le pied serait illisible dès qu'on change d'étape.

**Un point à ne pas manquer** : `recomposer` est appelé sans `await` par `veiller`, et son
erreur ne remonte aujourd'hui à personne — `composer()` l'attrape et l'écrit dans `#etat`,
qui va disparaître. Il faut la router vers `alerter()` **et vérifier qu'elle ne s'efface
pas au geste suivant** : `alerter('')` est appelé au début de chaque `essai()`, et un
réglage anodin ferait disparaître l'échec sans que rien ne soit réparé.

## 6. Ce que le Rust doit ajouter

Peu de choses — c'est le point rassurant de ce chantier.

1. **Le chemin du PDF de l'intérieur**, pour le destinataire visé, ou `None` s'il
   n'existe pas. Une commande `interieur_pdf()` ou un champ calculé sur la vue du
   destinataire. Aucune persistance.
2. **`polices_introuvables` dans `Mesure`**, si la recommandation du § 4 est retenue.
3. **`menu.rs`** : l'entrée « Aller › Intérieur » disparaît, et les raccourcis ⌘1 à ⌘5
   deviennent ⌘1 à ⌘4.
4. **Ouvrir le PDF au clic** — voir § 7, c'est la seule vraie question ouverte.

Rien à toucher dans `interieur.rs`, `composer()`, `converge()` ni la composition
elle-même : ce chantier ne déplace aucune page. **Le témoin doit rester à 98 pages,
dos 7,21 mm.**

## 7. La seule dépendance nouvelle : ouvrir un fichier

Le dépôt n'a que `tauri-plugin-dialog`. Rien n'y ouvre un fichier : le chemin de
l'épreuve est affiché en texte (`#cheminEpreuve`), à copier à la main. Un lien cliquable
est donc une capacité neuve.

- **`tauri-plugin-opener`** (plugin officiel Tauri 2, `openPath`) : quelques lignes de
  `Cargo.toml`, d'`init()` et de capacité. Multiplateforme, ce qui compte — la CI livre
  aussi Windows.
- **Sans dépendance** : le « lien » n'est qu'un texte au survol, et cliquer ne fait rien.
  C'est renier ce que le mot « lien » promet.
- **Une commande maison** avec `std::process::Command` : `open` sur macOS, `explorer` sur
  Windows. Réécrit le plugin en moins bien, et la livraison Windows le paierait.

**Recommandation : `tauri-plugin-opener`.** Et si on l'ajoute, le chemin de l'**épreuve**
devient cliquable du même geste — c'est le même besoin, et le laisser en texte mort à
côté d'un lien vivant serait une incohérence gratuite.

## 8. Les six fichiers du front, et le septième

`app.js:97` avertit lui-même de ce que coûte une étape : « six fichiers, donc, pas
trois ». Supprimer l'onglet Intérieur les touche tous.

1. **`src/index.html`** — la `<section id="etapeInterieur">` disparaît ; ses blocs
   remontent dans `etapeLivre` ; le pied gagne sa légende.
2. **`src/app.js`** — l'entrée d'`ETAPES` ; `etatEtapes.interieur` ; `majPied` ;
   `composer()` amputé de son bouton ; `manuscritRemplace` gagne son déclenchement ;
   `oublierLaComposition` perd les identifiants morts.
3. **`src-tauri/src/menu.rs`** — l'entrée « Aller › Intérieur », et la renumérotation.
4. **`tests/coquille.test.js`** — la table `ETAPES`, et tous les tests qui traversent
   cinq onglets.
5. **`src/styles.css`** — les deux sélecteurs `#etapeLivre, #etapeInterieur, #etapeEnvois`
   (lignes 234 et 250) perdent leur milieu ; le pied gagne la mise en forme de sa
   légende.
6. **`app/README.md`** — « L'écran » (cinq onglets → quatre, trois témoins → deux, la
   règle du compte rendu élargie), « Le prestataire, choisi une seule fois », et ce que
   le pied dit désormais.

Et le septième, que le commentaire d'`app.js` ne connaît pas encore : **cinq autres
fichiers de test** touchent `btComposer`, `#resultat` ou `etapeInterieur` —
`contrats.test.js`, `epreuve.test.js`, `packages.test.js`, `cycle_de_vie.test.js`,
`composition.test.js`. Le commentaire d'`app.js:97` est donc lui-même à corriger : ce
n'est pas six fichiers, c'est six plus les tests qui pilotent l'étape.

## 9. Risques

- **La composition qui part sans qu'on l'ait demandée.** C'est le pari du § 2. Le seul
  déclencheur est le chargement d'un manuscrit, jamais l'ouverture d'un projet — mais un
  manuscrit chargé par mégarde coûte une minute de Typst.
- **L'échec silencieux.** Aujourd'hui l'erreur est à côté du bouton qu'on vient de
  cliquer, donc regardée. Demain elle est en haut d'une fenêtre où l'on fait autre chose.
  C'est le vrai coût de la disparition du geste, et le § 5 en est la garde.
- **Le pied surchargé.** Cinq mentions, un lien et une alerte sur une ligne qui doit tenir
  à 900 px, sous un `<select>` de destinataires. À vérifier à l'œil, à la fenêtre la plus
  étroite, avec un titre de prestataire long.
- **L'onglet Livre qui gonfle.** Il passe de trois blocs à cinq. Il défile déjà ; il
  défilera plus. À regarder avant de commiter — la mise en page en colonnes de
  `styles.css:234` est faite pour ça, mais elle n'a jamais eu cinq blocs à ranger.
- **Le déclencheur qu'on oublie.** L'import d'un `livre.toml` ne passe pas par
  `manuscritRemplace` : s'il n'est pas branché, importer un livre ne composera rien et le
  livre restera sans dos jusqu'au premier geste. À vérifier à l'écran, pas seulement au
  test.

## 10. Vérification

- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`,
  `node --test tests/*.test.js`. **Jamais dans un pipe** — piège connu du dépôt.
- **`cargo run --example temoin` : 98 pages, dos 7,21 mm.** Ce chantier ne touche pas à la
  composition ; tout écart est un bug, pas une conséquence.
- **Un test neuf par comportement neuf, vu échouer.** En particulier : la composition qui
  part au manuscrit et pas à l'ouverture (mutation : retirer l'appel, le test doit
  rougir) ; le lien absent quand le PDF n'existe pas ; le pied dans ses quatre états.
- **À l'œil, dans la fenêtre** — `touch src/lib.rs && cargo build` d'abord :
  1. Projet neuf, choisir un manuscrit : la composition part seule, le pied se remplit.
  2. Changer la police : le pied repasse en « dos périmé » puis se remplit à nouveau,
     sans qu'on ait cliqué.
  3. Rouvrir un `.ozalid` déjà composé : **rien ne part**, le pied dit ce que l'archive
     porte.
  4. Cliquer « intérieur » : le PDF s'ouvre. Effacer le PDF à la main, rouvrir le projet :
     le lien n'est plus là.
  5. Une police absente des répertoires embarqués : `⚠ repli` au pied, le détail dans
     Livre.
  6. Un compte de chapitres attendu faux : le message monte à l'entête et **y reste** en
     changeant d'étape.
  7. La fenêtre à 900 px : le pied tient sur une ligne.

## 11. Ce que cette spec ne fait pas

- **Elle ne rend pas la composition interruptible.** Une composition partie va au bout.
  C'est acceptable tant qu'elle est débouncée et sérialisée, et c'est ce qui rend le pari
  du § 2 réversible : si l'automatisme gêne, c'est l'interruption qu'il faudra écrire,
  pas le bouton qu'il faudra remettre.
- **Elle ne touche ni à la composition, ni au format, ni au dos.** `VERSION` du `.ozalid`
  ne bouge pas ; `Mesure` ne gagne au plus qu'un champ à défaut. Le témoin ne bouge pas.
- **Elle ne réorganise pas les autres étapes.** Couverture, Livraison et Envois ne sont
  pas touchées, sauf par la renumérotation des raccourcis.
- **Elle ne donne pas de barre de progression** à la composition automatique. Le pied dit
  « dos périmé » pendant qu'elle tourne, ce qui est vrai et suffit.

## 12. Les lots

1. **La police et l'épreuve déménagent, l'onglet meurt.** Purement du front et du menu :
   les deux blocs remontent dans Livre, `ETAPES` perd une entrée, les six fichiers
   suivent. Le bouton « Composer » **survit**, relogé dans Livre — le lot est vérifiable à
   l'écran sans rien changer au comportement.
2. **Le pied prend la légende.** `majPied` gagne ses quatre états, `#resultat` disparaît,
   le témoin de dos périmé descend, le lien « intérieur » paraît. `tauri-plugin-opener`
   entre ici, avec le chemin de l'épreuve rendu cliquable du même geste.
3. **Le bouton disparaît.** `manuscritRemplace` déclenche, l'import de `livre.toml`
   aussi, l'échec monte à `alerter()`. C'est le lot qui change vraiment le comportement,
   et il est seul — ce qui le rend révocable d'un `revert`.

Le découpage est fait pour ça : les deux premiers lots sont des déménagements qu'on juge
à l'œil, le troisième est le pari. Chacun se commite et se regarde.
