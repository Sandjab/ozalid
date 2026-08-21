# La page de dédicace imprimée

Date : 2026-08-21
Statut : validé (brainstorming)

## Objectif

Le livre composé par Ozalid Studio porte quatre pages liminaires — faux-titre,
blanche, page de titre, copyright — et rien d'autre. L'auteur qui veut dédier son
livre à quelqu'un n'a aucun endroit où l'écrire : ni champ dans le projet, ni page
dans le PDF.

Cette spec ajoute la dédicace imprimée : celle qui figure dans **tous** les
exemplaires, en belle page, composée dans la police du livre. Un champ, deux pages,
et le déplacement de pagination que cela entraîne, assumé et mesuré.

Elle ne traite pas de l'envoi autographe — le mot manuscrit adressé à une personne
sur un exemplaire précis. Les deux s'appellent « dédicace » dans l'usage courant et
ne se ressemblent en rien : l'un est une donnée du livre, l'autre une sortie
personnalisée. L'envoi fait l'objet de la **spec 2b**, à brainstormer une fois
celle-ci livrée — dans cet ordre, parce que l'envoi se posera sur une page liminaire
et qu'il vaut mieux qu'elle ait cessé de bouger.

## Décisions de cadrage (brainstorming du 21/08)

- **Deux specs, pas une** : la dédicace imprimée d'abord, l'envoi autographe ensuite.
- **Recto puis blanche** : la dédicace prend une belle page, son verso reste blanc.
  Le livre gagne **deux pages**, le corps garde son ouverture en recto.
- **Petit italique aligné à droite, dans le tiers supérieur** : la convention de
  l'édition, et la seule page liminaire que la maison ne centre pas.
- **Le champ appartient au livre**, à côté du copyright et du genre.
- **Facultative, et silencieusement absente** : rien de renseigné, rien d'ajouté,
  pagination inchangée.
- **`outils/` est acté archive** : la chaîne Python ne reçoit pas la dédicace, et
  cesse d'être exigée avant commit.

## 1. La donnée

`Livre` reçoit un champ :

```rust
#[serde(default, skip_serializing_if = "Option::is_none")]
pub dedicace: Option<String>,
```

C'est du texte d'auteur, au même titre que `copyright`, `genre` et `titre_page` :
il voyage avec le livre, il se saisit une fois, il n'a rien d'un réglage de
composition. `Interieur` ne porte que la police et n'est pas le bon logement ; le
manuscrit non plus — `manuscritRemplace` le réimporte, et la dédicace serait perdue
ou dupliquée à chaque réimport.

Le multi-ligne est accepté, comme pour le copyright.

**Absente, vide, ou faite d'espaces** : aucune page ajoutée. Le test est
`dedicace.as_deref().map(str::trim).is_some_and(|d| !d.is_empty())` — un champ où ne
subsiste qu'une espace ne doit pas coûter deux pages et un dos.

`VERSION` ne bouge pas. Un `.ozalid` écrit avant ce champ s'ouvre sans un mot et se
voit doté d'une dédicace absente, exactement comme `livraison` l'a fait au lot 3 du
chantier précédent.

**Prix annoncé** : `Livre` est construit littéralement en seize endroits de
`src-tauri` — sources, tests et `examples/temoin.rs` — dont une majorité derrière des
helpers `fn livre()` de modules de test. Tous sont à toucher. C'est mécanique, mais
ce n'est pas gratuit et le plan doit le compter.

## 2. La composition

### Ce qui a été constaté

`interieur::source` compose tout le document dans une seule fonction d'une centaine
de lignes : l'entête, quatre blocs `push_str` de liminaires, puis la boucle des
chapitres. Y ajouter un cinquième bloc conditionnel est le geste minimal, mais la
fonction cesserait de se lire d'un coup d'œil.

### Ce qui est décidé

`source()` cède ses pages liminaires à une fonction :

```rust
fn liminaires(livre: &Livre) -> String
```

qui rend le faux-titre, la blanche, la page de titre, le copyright et — le cas
échéant — la dédicace et sa blanche. `source()` retrouve une taille lisible et la
dédicace devient testable sans compiler un document entier. C'est une amélioration
ciblée sur le code qu'on touche, pas un refactor d'occasion : rien d'autre de
`interieur.rs` n'est déplacé.

Quand la dédicace est renseignée, deux pages s'insèrent après le pavé de copyright :

```typst
#v(48mm)
#align(right, emph(text(size: 9.5pt)[…]))
#pagebreak()
#pagebreak()
```

Trois points en découlent sans qu'il faille rien écrire de plus :

- **Pas de folio.** `footer: none` court jusqu'au `#set page(footer: …)` qui ouvre le
  corps ; les deux pages ajoutées sont donc muettes comme leurs voisines.
- **La blanche est un `#pagebreak()` doublé**, le dispositif déjà employé pour la
  page 2. Aucun mécanisme nouveau.
- **Le corps s'ouvre en page 7** au lieu de 5. La numérotation courait déjà depuis le
  faux-titre : seul son affichage change de point de départ, et il n'y en a pas.

Le texte passe par `echappe()`, et ses sauts de ligne ressortent en ` \ ` — le
traitement du pavé de copyright, repris tel quel. Le corps de **9,5 pt** est en dur,
comme les cinq autres valeurs des liminaires (11, 10,5, 15, 10 et 8 pt) : la maison
ne les indexe pas sur le prestataire, et l'indexer ici seul serait une incohérence.

**Les valeurs `48mm` et `9,5pt` sont choisies par cohérence avec les liminaires
voisins, pas mesurées.** Elles sont le point de départ, à confronter à une page
réellement compilée pendant l'implémentation ; les ajuster n'est pas un écart à la
spec, les laisser sans regarder en serait un.

## 3. L'interface

Un champ dans l'étape **Livre**, après le Copyright :

```html
<label><span>Dédicace</span>
  <textarea id="inDedicace" rows="2"
            placeholder="vide : pas de page de dédicace"></textarea></label>
```

L'ordre du panneau suit l'ordre des pages du livre : le copyright est en page 4, la
dédicace en page 5. Le placeholder dit la conséquence de laisser le champ vide plutôt
que de décrire le champ — un champ intitulé « Dédicace » n'a pas besoin qu'on répète
son nom, mais il a besoin qu'on dise qu'il ne fabrique rien tant qu'il est vide.

La lecture et l'écriture se posent aux deux endroits où `inCopyright` est déjà traité
— le chargement d'un projet et la collecte. Le drapeau de modification doit se lever
à la saisie comme pour les autres champs ; si le mécanisme est un écouteur nommé
plutôt que global, le plan l'y ajoute explicitement.

## 4. Vérification

### Le témoin de non-régression

`cargo run --example temoin` doit rendre **98 pages** — la valeur que
`PAGES_ATTENDUES` fixe — et le dos relevé au lot précédent, inchangés : le témoin ne
renseigne pas de dédicace, donc rien ne doit bouger. C'est la garde la plus
solide de ce chantier, parce qu'elle traverse la chaîne entière au lieu d'inspecter
une chaîne de caractères.

### Le témoin avec dédicace

Le même livre, dédicace renseignée, compilé pour de vrai : **100 pages**. Ni 99 — ce
qui signalerait la blanche perdue — ni 101, qui trahirait une page de trop. Deux
pages exactement, et le dos qui suit.

Ce contrôle réclame Typst, donc il ne vit pas dans `cargo test`. Il se fait au moins
une fois à l'implémentation, et son résultat s'écrit dans le compte rendu.

### Tests unitaires sur `liminaires()`

- Dédicace absente, `Some("")` et `Some("   ")` : les trois rendent **la même source,
  à l'octet près**, et c'est celle que produit la version d'avant ce chantier. Une
  seule des trois qui diffère, et un livre déjà composé change de dos sans prévenir.
- Dédicace renseignée : deux `#pagebreak()` de plus, et le texte présent.
- Un `#` dans la dédicace ressort échappé, un saut de ligne ressort en ` \ `. Ce sont
  les deux pièges déjà gardés pour le titre de page ; ils se reposent identiquement.

Chaque test doit être vu échouer sur une mutation ciblée avant d'être cru.

### Le `.ozalid`

Round-trip : dédicace écrite, relue, identique. Et un `.ozalid` dépourvu du champ
s'ouvre sans erreur, dédicace absente.

### Le reste

`cargo fmt`, `cargo clippy`, `cargo test`, `node --check` sur `app.js`. À l'écran :
la saisie du champ, la génération d'un package, et le PDF ouvert sur sa page 5.

## 5. Hors périmètre

- **L'épreuve de relecture ne porte pas la dédicace.** Elle n'affiche aucun
  liminaire — ni faux-titre, ni page de titre, ni copyright — et se présente comme un
  document de travail sur le texte, pas comme le livre. Lui ajouter la seule dédicace
  serait un liminaire orphelin.
- **`outils/` est archive.** La chaîne Python ne reçoit pas la dédicace ; le
  `CLAUDE.md` le déclare et retire des vérifications avant commit l'exigence de
  régénérer un intérieur avec `gen_interieur.py`. Le répertoire reste au dépôt pour
  l'historique. C'est le seul changement de `CLAUDE.md` que cette spec autorise.
- **L'import d'un `livre.toml` historique** n'apporte pas de dédicace : le champ n'y
  existe pas, il ressort absent. Rien à écrire pour cela, mais un test d'import
  existant doit continuer de passer.
- **L'envoi autographe** : spec 2b.
- **L'atelier `index.html` de la racine** : il ne compose pas d'intérieur, il n'est
  pas concerné.
- **Les dettes de `NOTES.md`** — ruptures de scène perdues à l'impression, `try/catch`
  dupliqué, sorties non vidées au réimport. Aucun rapport.
