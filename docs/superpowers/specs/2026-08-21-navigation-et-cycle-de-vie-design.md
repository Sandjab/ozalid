# Navigation par étapes, cycle de vie du projet et menu natif

Date : 2026-08-21
Statut : validé (brainstorming)

## Objectif

Ozalid Studio tient la chaîne entière, mais elle se présente comme une page
unique de huit sections empilées : on descend pour changer de sujet, et l'on
redescend une seconde fois, dans un second ascenseur, pour atteindre le bas du
panneau de couverture. Il n'existe par ailleurs aucun moyen de créer un projet
de zéro — on ne peut qu'ouvrir un `.ozalid` ou importer un `livre.toml` — et
aucun menu natif : les trois actions de fichier sont trois boutons au sommet de
la page.

Cette spec réorganise la coquille : quatre étapes ordonnées, un cycle de vie de
document complet, un menu natif qui en fait foi, et une direction visuelle qui
cesse de teindre ce que l'application sert à juger.

La saisie de dédicace, quatrième chantier de la demande initiale, fait l'objet
d'une spec distincte : c'est une fonctionnalité neuve — module Rust, surcharge
d'une page du PDF, nouvel élément dans le `.ozalid` — là où le présent document
ne déplace que la coquille.

## Décisions de cadrage (brainstorming du 21/08)

- **Quatre étapes**, pas cinq : Livre (identité *et* manuscrit), Intérieur,
  Couverture, Livraison.
- **La fenêtre ne défile plus.** Une seule zone défilante dans toute
  l'application : le panneau de réglages de la couverture.
- **Onglets en tête**, pas de rail vertical. Le balisage le prépare, rien ne le
  livre.
- **Nouveau projet = document vide.** Ni assistant, ni sélecteur de fichiers
  imposé.
- **Un magasin de préférences côté Rust**, justifié par les projets récents que
  le menu natif doit lire.
- **Menu Fichier + Édition + Aller.** Pas de menu Composition.
- **Drapeau de modification et garde à la fermeture**, à trois boutons.
- **Une seule liste de prestataires et un pointeur dessus** : les destinataires
  du livre à l'étape Livraison, le destinataire courant dans le pied de fenêtre.
- **Pas de format générique.** L'idée est examinée et écartée, motifs au § 3.
- **Le multi-prestataire est conservé.** Il est déjà écrit, testé et prouvé.
- **Vignette de planche par package généré.**
- **Direction visuelle « atelier neutre »** : gris chauds, blanc pour les
  surfaces de travail, rouge rendu à l'alerte.
- **Aucun moteur de composition n'est touché.** Le compte de pages ne doit pas
  bouger d'une unité.

## 1. La coquille

### Ce qui a été constaté

`app/src/index.html` empile huit `<section>` dans un `<main>` à `max-width:
46rem`. Le défilement est celui du document. `styles.css:147` pose par ailleurs
`.reglages { max-height: 34rem; overflow-y: auto }` : le panneau de couverture
défile à l'intérieur d'une page qui défile déjà. C'est la cause exacte du second
ascenseur.

`lib.rs` n'importe pas `tauri::menu` : l'application n'a aucun menu déclaré.

### Ce qui est décidé

`body` devient une grille en quatre bandes, à la hauteur exacte de la fenêtre :

```
entête     titre du livre · chemin du .ozalid · état d'enregistrement
onglets    1 · Livre   2 · Intérieur   3 · Couverture   4 · Livraison
contenu    l'étape courante, seule à occuper la place restante
pied       destinataire courant · dos de la dernière composition
```

Le contenu d'une étape ne déborde pas : ce qui ne tient pas se règle par la mise
en page, jamais par un ascenseur — à la seule exception du panneau de réglages de
la couverture, dont la longueur est irréductible et qui reste donc la seule zone
défilante de l'application.

Chaque onglet porte un témoin discret lorsque son étape réclame attention :
manuscrit périmé (écart avec le contrôle d'intégrité), aucune maquette choisie,
dos périmé.

Sans projet ouvert, les onglets sont inertes et le contenu montre un accueil :
Nouveau projet, Ouvrir un `.ozalid`, Importer un `livre.toml`, et la liste des
projets récents.

Le balisage de la `<nav>` est écrit **indifférent à sa disposition** : quatre
boutons portant chacun un nom et un sous-libellé d'état, la grille tenue par des
variables CSS. Passer au rail vertical serait un bloc de règles et une classe sur
`body`. Aucune préférence de coquille n'est livrée : le besoin n'est pas établi,
et la maintenir doublerait ce qui doit être regardé à chaque changement visuel.

Conséquence à tenir dès maintenant : **le panneau de couverture doit rester
utilisable à 700 px de large**, largeur qu'aurait le contenu avec un rail sur une
fenêtre à sa taille minimale. C'est la dette payée d'avance, et elle est bonne à
payer de toute façon.

### Direction visuelle

Trois directions ont été comparées sur la même étape : le crème de la Blanche
tenu (`#fcf0d8` / `#191917` / `#c00000`, la palette actuelle), un atelier neutre,
et une table sombre.

**L'atelier neutre est retenu.** Motif : l'application sert à juger une
couverture, et un fond crème n'est pas neutre — un blanc posé dessus paraît bleu,
un beige paraît neutre. Le rouge `#c00000`, aujourd'hui décoratif dans la
coquille, est rendu à son seul emploi : l'alerte. La couverture devient le seul
objet coloré de l'écran.

Ce que le choix coûte : l'application perd le caractère qui la distinguait d'un
formulaire. Il est à retrouver dans la typographie, les espacements et le soin
des états — pas dans le fond.

La table sombre est écartée pour une raison précise : `#c00000` y est illisible
et devrait céder à un orangé, soit un écart avec la couleur même que l'outil
compose.

## 2. Les quatre étapes

### 1 · Livre

Identité et manuscrit dans un seul écran — six champs et trois boutons y tiennent
sans effort, et le manuscrit est ce qui porte le titre et l'auteur.

- titre, titre de la page de titre, auteur, genre, copyright, chapitres attendus ;
- source mémorisée du manuscrit, son état (chapitres, mots), l'écart avec le
  contrôle d'intégrité quand il y en a un, Réimporter, Choisir un autre
  manuscrit.

Sur un projet neuf, l'encart manuscrit **dit ce qui manque** au lieu d'afficher
zéro chapitre : un manuscrit absent est un état, pas une erreur.

### 2 · Intérieur

- la police du livre ;
- **Composer**, qui compose pour le destinataire courant et rend pages,
  chapitres, gouttière, page blanche de parité et dos ;
- **Tirer une épreuve**, avec son corps.

Aucun sélecteur de prestataire ici : il est dans le pied, et il est global.

L'épreuve rejoint l'intérieur plutôt que la livraison : les deux composent le
texte, l'une pour relire, l'autre pour imprimer. Elle sort à la racine du projet,
jamais dans un répertoire de prestataire — le code le dit déjà.

### 3 · Couverture

Maquette de départ, photos de 1ère et de 4ème, les trois faces (1ère, 4ème,
Planche), l'aperçu et le panneau de réglages. Inchangée dans son fond ; c'est sa
place dans la fenêtre qui change.

La face Planche annonce pour quel gabarit son dos vaut, ou refuse de s'afficher
tant que rien n'a été composé — comme aujourd'hui.

### 4 · Livraison

- la liste des **destinataires** du livre : ajouter, retirer, leur papier, et
  pour ceux qui ne publient pas de formule, leur relevé de dos et de fond perdu ;
- **Générer** ;
- un résultat par destinataire : sa vignette de planche, ses chiffres (pages,
  gouttière, dos, dimensions de planche, fond perdu) et ses chemins de fichiers.

## 3. Le prestataire choisi une seule fois

### Ce qui a été constaté

Le prestataire est aujourd'hui désigné à deux endroits sans rapport : un `select`
dans la section « Prestataire », qui sert à composer l'intérieur de travail et
dont `dosCourant()` (`app.js:353`) se sert pour valider le dos de l'aperçu de
planche ; et des cases à cocher dans « Packages », qui désignent les livraisons.
Rien ne dit lequel est lequel.

### L'hypothèse d'un format générique, et pourquoi elle est écartée

L'idée examinée : choisir dans Intérieur un format générique — petit, moyen,
grand — et le décliner ensuite par prestataire, en travaillant sur des
proportions.

Les rapports largeur/hauteur des quatorze gabarits de la table forment
effectivement trois grappes :

| Rapport | Gabarits |
|---|---|
| 0,617 – 0,628 | Lulu 108×175, KDP 5×8, Bookvault 127×203, BoD 135×215 |
| 0,647 – 0,667 | CoolLibri 110×170, TBE 110×170, KDP 5,5×8,5, Bookvault 129×198, TBE 120×180, KDP 6×9, CoolLibri 160×240 |
| 0,705 – 0,707 | CoolLibri A5, TBE 148,5×210, Bookvault 148×210 |

L'intuition n'est donc pas fausse sur les données. Elle est sans objet, et
dangereuse, pour deux raisons distinctes :

**Sans objet pour les faces.** La maquette est *déjà* exprimée en pourcentage de
la largeur de couverture ; elle transpose d'un rapport à l'autre sans qu'aucun
format intermédiaire n'ait à exister. Un format générique n'ajouterait rien à ce
qui fonctionne.

**Dangereuse pour le dos.** Le dos est en millimètres, jamais en pourcentage :
il vient du compte de pages, qui vient des marges et de la gouttière du gabarit
réel. Un format générique devrait, pour en produire un, **inventer une formule** —
ce que le projet s'interdit explicitement à deux endroits :

> « Seules les tranches vérifiées dans le guide du prestataire figurent ici.
> Hors tranche, on refuse plutôt qu'inventer. » — `providers.rs:63`

> « Une planche composée sur un dos inventé se voit au massicot, jamais
> avant. » — `planche.rs:39`

Et le dos n'est pas qu'une largeur : l'auteur, le titre et l'éditeur s'y
composent élément par élément, chacun avec sa taille, sa place et son rang. Les
régler sur un dos approché, c'est les régler sur rien.

### L'hypothèse d'un seul prestataire par `.ozalid`, et pourquoi elle est écartée

Le multi-prestataire n'est pas un pari à prendre : il est écrit, testé et prouvé.
`packager(choix: Vec<Choix>)` compose chaque prestataire indépendamment, dans son
répertoire, avec des fichiers portant sa clé — un test le fige. L'exemple
`packager` traverse la chaîne entière pour trois gabarits d'un coup, et le README
en fait la preuve que Typst compile vraiment ce que le moteur émet. Rien dans le
modèle de données n'est propre à un prestataire : marges, gouttière, corps, fond
perdu et formule de dos viennent tous de la table, jamais de l'utilisateur.

Passer au mono ne serait pas une simplification mais un retrait, et il coûterait
au mauvais endroit : un `.ozalid` par destinataire, c'est la même maquette
recopiée N fois, donc un espacement de titre corrigé N fois à la main. C'est
exactement ce que l'application a été construite pour abolir — « un livre, N
prestataires, aucun réglage retouché entre les deux » (`package.rs:4`).

### Ce qui est décidé

Une seule liste, et un pointeur dessus.

- **La liste** — les destinataires du livre, déclarés à l'étape 4 et nulle part
  ailleurs, avec leur papier et leurs relevés.
- **Le pointeur** — le *destinataire courant*, dans le pied de fenêtre, à côté de
  l'état du dos. La forme du libellé, sur un exemple :
  `Vu pour : Lulu — poche 108 × 175 · dos 19,7 mm`, ou
  `Vu pour : Lulu — poche 108 × 175 · dos non composé`. Il ne peut désigner qu'un
  membre de la liste.

L'étape 2 compose pour lui, l'étape 3 rend ses aperçus à son format, l'étape 4
génère pour toute la liste. Il n'y a plus deux choix de prestataire.

Précision qui impose le pointeur : les faces 1ère et 4ème ne réclament aucune
*composition*, mais elles réclament un *format* — `couverture_apercu` prend un
`providerCle` pour connaître le rapport de la page. Un prestataire courant est
donc nécessaire même pour regarder une première de couverture.

Un projet neuf naît avec un destinataire — le premier gabarit de la table — pour
que le pointeur ne soit jamais vide.

Côté `projet.toml`, une section `[livraison]` nouvelle porte la liste et le
destinataire courant, en `#[serde(default)]` : un `.ozalid` écrit avant elle
s'ouvre sans rien dire et se voit doté du premier gabarit de la table, avec son
`papier_defaut()` — ce que faisait déjà le `select` en se positionnant sur sa
première option.

`projet::VERSION` **ne bouge pas**. La règle de refus est `version > VERSION` :
ajouter une section facultative ne rend illisible aucun fichier existant, et la
monter interdirait aux binaires déjà distribués d'ouvrir les projets écrits
ensuite — pour un champ qu'ils sauraient simplement ignorer. Le bump reste
réservé à ce qui change de sens, comme la 1 → 2 l'a fait pour la maquette.

### La vignette de planche

Chaque package généré affiche sa planche en vignette, à côté de ses chiffres.
C'est là que « est-ce que ça tient » se vérifie : sur du vrai, pour chaque
prestataire, avec son dos mesuré — et non sur une approximation qu'on espère
fidèle.

Le rendu existe déjà : `couverture_apercu` compose la planche par le même chemin
que le PDF. C'est un rendu de plus par prestataire, pas un moteur de plus.

## 4. Cycle de vie du projet

### Nouveau projet

`projet_nouveau()` construit un `Projet` vide : livre aux champs vides (genre
`roman` par défaut), manuscrit absent, aucune maquette, un destinataire par
défaut. `Projet::nouveau(livre, texte)` existe déjà et suffit.

Un point à traiter explicitement : `ProjetVue` compte chapitres et mots à chaque
retour. Un manuscrit vide doit s'y présenter comme une absence, pas comme un
découpage à zéro chapitre — et l'étape 1 doit le dire ainsi.

### Enregistrer

`projet_enregistrer_courant()` écrit au chemin mémorisé. Sans chemin, le front
bascule sur « Enregistrer sous… », qui est le comportement actuel.

L'`Atelier` gagne un drapeau `modifie`, levé par toute commande qui touche au
projet, abaissé à l'écriture, exposé dans `ProjetVue` pour que l'entête le
montre : `enregistré` / `modifié` / `jamais enregistré`.

### La garde

`WindowEvent::CloseRequested` : si `modifie`, `api.prevent_close()`, puis une
boîte native à trois boutons — **Enregistrer**, **Ne pas enregistrer**,
**Annuler**. Le même dialogue précède Nouveau, Ouvrir, Importer et Fermer le
projet.

Vérifié : `tauri-plugin-dialog` est résolu en 2.7.2 (`Cargo.lock`), qui expose
`MessageDialogButtons::YesNoCancelCustom(String, String, String)` côté Rust et
son équivalent côté JS. Aucune modale maison n'est nécessaire.

Contrainte à respecter : la variante bloquante du plugin ne doit pas être appelée
sur le fil principal. C'est la forme à callback qui sert dans le gestionnaire de
fenêtre.

## 5. Le menu natif

Un module `menu.rs` construit le menu de l'application :

**Fichier** — Nouveau ⌘N · Ouvrir… ⌘O · Ouvrir un récent ▸ · Importer un
`livre.toml`… · Enregistrer ⌘S · Enregistrer sous… ⇧⌘S · Fermer le projet.

**Édition** — les `PredefinedMenuItem` standard : annuler, rétablir, couper,
copier, coller, tout sélectionner. Déclarer un menu sur mesure les supprime
sinon, et ⌘C cesse de fonctionner dans les champs de saisie sous macOS. Le menu
applicatif de macOS (À propos, Masquer, Quitter) est à reconstruire pour la même
raison.

**Aller** — les quatre étapes, ⌘1 à ⌘4.

« Fermer le projet » ne prend pas ⌘W : sous macOS ce raccourci ferme la fenêtre,
et l'application n'en a qu'une.

Chaque entrée **émet un événement au front**, qui exécute exactement le code des
boutons de l'accueil. Le menu et la souris passent par la même implémentation ; il
n'existe pas deux façons d'ouvrir un projet. Les boutons de l'écran d'accueil sont
des raccourcis du menu, pas une seconde vérité.

## 6. Les préférences

Un module `preferences.rs` écrit un `preferences.toml` dans le répertoire de
configuration de l'application. Aucune dépendance nouvelle : `serde` et `toml`
sont déjà là.

Un seul champ pour l'instant :

```toml
recents = ["/Users/…/heures-creuses.ozalid", "…"]
```

Plafonné à dix, dédoublonné, les chemins disparus élagués à la lecture. Le
sous-menu « Ouvrir un récent » et l'écran d'accueil lisent la même liste, et elle
est reconstruite après chaque ouverture et chaque enregistrement.

L'écriture est **au mieux** : ne pas pouvoir enregistrer une préférence se
signale, mais n'empêche jamais de travailler. Une liste de projets récents perdue
ne coûte rien ; un projet qu'on ne peut plus ouvrir coûterait tout.

Le magasin est écrit pour accueillir d'autres préférences — la coquille au choix
en est la première candidate — sans que rien d'autre que `recents` n'y soit livré
aujourd'hui.

## 7. Vérification

### Automatique

```
cd app/src-tauri && cargo test --lib && cargo clippy --all-targets && cargo fmt --check
cd app && node --test "tests/*.test.js"
```

Tests neufs côté Rust :

- aller-retour des préférences : dédoublonnage, plafond de dix, élagage des
  chemins disparus ;
- un projet neuf vide reste valide à l'écriture et à la relecture ;
- un `.ozalid` sans `[livraison]` prend le premier gabarit de la table ;
- le drapeau `modifie` se lève à chaque mutation et retombe à l'écriture.

### Le point de tension

Les tests du front exécutent le vrai `app.js` dans un faux DOM qui lit l'état
initial du vrai `index.html`. La restructuration les invalide en bloc — et ils
sont la seule garde automatique du front. Ils sont réécrits dans le même lot,
jamais laissés en attente : onglets inertes sans projet, changement de
destinataire courant qui relance l'aperçu, drapeau de modification, entrées de
menu qui appellent le même code que les boutons.

### À l'œil, dans l'application

La doctrine du README ne change pas : tout ce qui se voit se vérifie dans
l'application.

- les quatre étapes à 900 px et à 1400 px de large ;
- le panneau de couverture utilisable à 700 px ;
- **une seule zone défilante, jamais deux** ;
- les trois maquettes × les trois faces ;
- chaque entrée de menu fait ce que son bouton fait ;
- ⌘C et ⌘V dans un champ de saisie ;
- la garde à la fermeture, et ses trois boutons ;
- un projet récent dont le fichier a été supprimé disparaît de la liste.

### Le témoin de non-régression

Cette spec ne touche à aucun moteur de composition. **Le compte de pages ne doit
pas bouger d'une unité.** `cargo run --example temoin` et un `packager` sur un
livre réel, avant et après. C'est la garde la plus forte, et la moins chère.

## 8. La passe visuelle

Pas `frontend-design` : il pousse vers une interface distinctive et expressive, à
rebours d'un outil de labeur qui doit s'effacer devant l'objet qu'il montre.

L'intention est écrite ici — atelier neutre, gris chauds, blanc pour les surfaces
de travail, rouge rendu à l'alerte. La finition se fait **en fin de chantier**,
sur du code déjà structuré, avec `impeccable:polish` puis `impeccable:audit`.
Corriger vaut mieux qu'inventer une seconde fois.

## 9. Ordre d'exécution suggéré

Le chantier est trop large pour un seul lot. Quatre, dans cet ordre — chacun
laisse l'application en état de marche, et chacun est vérifiable seul :

1. **Le cycle de vie et le menu.** `projet_nouveau`, drapeau `modifie`,
   `projet_enregistrer_courant`, garde à la fermeture, `preferences.rs`,
   `menu.rs`, écran d'accueil avec les récents. La page unique ne bouge pas
   encore : c'est ce qui rend ce lot vérifiable sans rien casser d'autre.
2. **La coquille.** Grille en quatre bandes, onglets, entête, pied, étapes
   inertes sans projet, réécriture des tests du front. Le contenu des étapes est
   déplacé tel quel, sans changement de fond.
3. **Le prestataire unifié.** Section `[livraison]`, destinataires à l'étape 4,
   destinataire courant dans le pied, vignettes de planche par package.
4. **La passe visuelle.** Palette atelier neutre, puis `impeccable:polish` et
   `impeccable:audit`.

Le témoin de non-régression — le compte de pages — se rejoue à la fin de chaque
lot, pas seulement à la fin du chantier.

## 10. Hors périmètre

- **La dédicace** : spec 2, à brainstormer séparément. Elle se posera dans
  l'étape 4, juste avant la génération.
- **Le rail vertical et sa préférence de coquille** : le balisage le prépare,
  rien ne le livre.
- **Les dettes de `NOTES.md`** — ruptures de scène perdues à l'impression,
  guillemet droit dans le titre, double affectation de `block.style.top` dans
  `index.html`. Aucun rapport avec ce chantier.
- **`index.html` et `outils/`** : gelés, non touchés.
- **L'icône** : reste provisoire.
