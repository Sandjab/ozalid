# Les pièces du manuscrit — préface, annexes, pages de partie

Date : 2026-08-22
Statut : validé (brainstorming)

## Objectif

Le manuscrit n'admet aujourd'hui qu'une seule forme de titre : `## NN - Titre`. Tout
le reste est refusé avec son numéro de ligne — c'est le « fail loud » écrit en tête de
`manuscrit.rs`, et il a sa raison d'être : `## Chapitre premier` doit rester une
erreur, parce qu'un chapitre non numéroté ne se verrait qu'après tirage.

Mais un roman ne se compose pas que de chapitres. *WIP8* (`build/in/texts/WIP8.md`)
s'ouvre sur une préface de huit paragraphes, et bute sur `ligne 7 : titre de chapitre
« Préface » (attendu : « NN - Titre »)`. Le livre veut aussi des pages intercalaires à
titre libre — « Avant Clément », « Après Clément » — qui ouvrent une partie sur une
belle page au verso blanc.

Cette spec ouvre le format de **trois crans nommés**, sans l'ouvrir tout court : le
manuscrit reconnaît des pièces liminaires, des pièces annexes et des pages de partie,
chacune à une marque explicite. Un titre qui n'entre dans aucune des quatre formes
échoue exactement comme aujourd'hui.

## Décisions de cadrage (brainstorming du 22/08)

- **Le texte vit dans le manuscrit**, pas dans l'onglet Livre. Une préface fait
  plusieurs milliers de signes avec des paragraphes et de l'emphase : elle n'a pas sa
  place dans un champ de formulaire à côté de la dédicace, et l'auteur l'écrit là où il
  écrit le reste.
- **Liste blanche fermée**, pas de forme libre. La position d'une pièce découle de son
  mot ; l'auteur n'a rien à déclarer.
- **Un titre libre exige un marqueur explicite** : `Avant Clément` est indiscernable
  d'un chapitre mal formé, donc la page de partie s'écrit `## Partie III - Avant
  Clément`.
- **Romains pour les parties**, arabes pour les chapitres. Le romain se vérifie comme
  le reste : `III` doit suivre `II`.
- **La préface est une pièce liminaire sans folio**, comme la dédicace — pas une
  pagination romaine séparée, pas un chapitre zéro.
- **La numérotation des chapitres court au travers des parties** : 01…64 reste 01…64.
- **`livre.chapitres` ne compte que les chapitres.** *WIP8* reste à 64, aucun
  `livre.toml` à retoucher.

## 1. Le format du manuscrit

Quatre formes d'en-tête `## `, et rien d'autre :

| écrit | pièce | zone |
|---|---|---|
| `## 01 - Vingt centimes` | chapitre numéroté | corps |
| `## Préface` · `## Avant-propos` · `## Prologue` | liminaire | avant le corps |
| `## Épilogue` · `## Postface` · `## Remerciements` | annexe | après le corps |
| `## Partie III - Avant Clément` | page de partie | corps |

Le manuscrit est donc **trois zones, dans cet ordre** : liminaires, corps, annexes.

Reconnaissance des mots-clés : comparaison **insensible à la casse, accents exigés**.
`## préface` passe, `## Preface` non — le projet est en français accentué, et un mot
désaccentué est plus probablement une faute qu'une intention.

Le titre est facultatif partout où il l'est déjà : `## Partie III` seul est admis,
comme `## 7` l'est pour un chapitre (`manuscrit.rs:347`).

### Les refus

Chacun avec son numéro de ligne, dans la forme des refus existants :

1. **Titre hors des quatre formes** — inchangé : `ligne 7 : titre de chapitre
   « Chapitre premier » (attendu : « NN - Titre »).`
2. **Zone non tenue** — une pièce liminaire après un chapitre, un chapitre ou une page
   de partie après une annexe. Pas de réordonnancement silencieux : la position découle
   du mot **et** doit être écrite.
3. **Romain de partie non consécutif** ou illisible — `Partie IV` juste après
   `Partie II` est une partie perdue en route.
4. **Du texte sous une page de partie** — une page de partie ne porte que son titre ;
   un paragraphe écrit là serait silencieusement perdu à la composition, ce qui est
   précisément ce que le format refuse.

Un liminaire ou une annexe **sans texte** est admis : rien ne se perd, la page est
seulement maigre.

## 2. Le modèle

`Chapitre` cède la place à `Piece`, dans `manuscrit.rs` :

```rust
/// Ce qu'une pièce est, et où elle se compose. La position découle de la sorte :
/// aucun appelant n'a à la déduire du titre.
#[derive(Debug, Clone, PartialEq)]
pub enum Sorte {
    Chapitre(u32),
    Liminaire,
    Annexe,
    Partie(String), // le romain, tel qu'écrit
}

#[derive(Debug, Clone, PartialEq)]
pub struct Piece {
    pub sorte: Sorte,
    pub titre: String,
    pub blocs: Vec<Bloc>,
}
```

Le titre d'un liminaire ou d'une annexe est son mot-clé tel qu'écrit dans le manuscrit,
normalisé à la graphie de la liste (`préface` → `Préface`) : ce qui s'imprime ne dépend
pas de la casse tapée.

`decoupe(md, attendu) -> Result<Vec<Piece>, String>` rend les pièces **dans l'ordre du
manuscrit** ; l'ordre de composition en découle sans tri, les zones étant déjà validées.
`attendu` ne compte que les `Sorte::Chapitre`.

Une fonction courte parse les romains (`I` à `L` suffisent largement) et sert aux deux
usages : lire le numéro, et vérifier qu'il suit le précédent.

**L'ordre de reconnaissance compte** : la liste blanche est consultée sur le titre
entier avant tout découpage. `## Avant-propos` porte un tiret, et l'actuel
`split_once('-')` (`manuscrit.rs:265`) en ferait un chapitre de numéro « Avant ». Le
mot-clé d'abord, le préfixe `Partie ` ensuite, le `NN - Titre` en dernier.

Six appelants suivent le changement de type : `ebook.rs:70`, `package.rs:73` et `:234`,
`commands.rs:513`, `:568`, `:1239`.

**`commands.rs:1239` est le piège** : `chapitres_trouves` y vaut `c.len()`, et
l'interface signale un manuscrit périmé sur l'écart entre ce compte et
`livre.chapitres`. Il doit devenir le compte des seules `Sorte::Chapitre`, sinon
*WIP8* afficherait 68 contre 64 déclarés et se dirait périmé alors qu'il est juste.
Même correction pour `nb_chapitres` en tête d'épreuve (`epreuve.rs:122`).

## 3. L'intérieur imprimé

L'ordre de composition devient :

```
faux-titre · blanche · titre · copyright · dédicace     sans folio   (inchangé)
liminaires du manuscrit          ← préface              sans folio   (nouveau)
#set page(footer: folio)
corps : chapitres, pages de partie intercalées          folio
annexes du manuscrit             ← postface             sans folio   (nouveau)
blanche de fin                                          sans folio   (inchangé)
```

### Liminaires et annexes

`liminaires()` reçoit les pièces liminaires en plus du livre et de l'envoi, et les
compose après la dédicace — le `#set page(footer: folio)` qui suit ouvre le corps comme
avant. Les annexes, elles, arrivent alors que le folio est actif : un
`#set page(footer: none)` les précède, et court jusqu'à la blanche de fin qui n'en
portait déjà pas.

Les deux reprennent le gabarit du chapitre, le mot occupant la ligne du numéro :

```
#v(22mm)
#align(center, text(size: 10pt, tracking: 0.14em)[#upper[Préface]])
#v(14.5mm)
<paragraphes, séparateurs de scène compris>
#pagebreak()
```

Le mot est composé comme un **titre** de chapitre (10 pt, `tracking: 0.14em`,
capitales), pas comme un numéro : ce sont la casse et l'espacement qui font le titre,
les 13 pt du gabarit étant la taille d'un chiffre isolé. Le blanc de 14,5 mm est la
somme des deux blancs du gabarit (`3.5` + `11`), pour que le texte s'ouvre à la même
hauteur que celui d'un chapitre.

Les annexes sont composées après le dernier chapitre, sans folio comme les liminaires :
le folio appartient au corps, une pièce n'en porte pas — c'est la règle validée pour la
préface, appliquée des deux côtés.

### Pages de partie

Le dispositif de la dédicace (`interieur.rs:365`) : recto puis blanche. Le folio étant
actif dans le corps, les deux pages passent par `#page(footer: none)`, comme la blanche
de fin (`interieur.rs:246`) :

```
#page(footer: none)[
  #v(22mm)
  #align(center, text(size: 13pt)[III])
  #v(3.5mm)
  #align(center, text(size: 10pt, tracking: 0.14em)[#upper[Avant Clément]])
]
#page(footer: none)[]
```

Le romain prend la place du numéro, le titre celle du titre : une page de partie et une
ouverture de chapitre se ressemblent, ce qui est voulu.

**Piège à traiter, pas à supposer** : `#page(...)[…]` ouvre et ferme sa propre page,
tandis que la boucle des chapitres pose un `#pagebreak()` avant chaque chapitre au-delà
du premier (`interieur.rs:209`). Enchaînés tels quels, ils produiraient une page blanche
de trop après chaque partie. Le compte de pages tranche, pas le raisonnement : il se
vérifie sur un manuscrit d'essai portant une partie.

## 4. L'épreuve

`epreuve.rs:130` compose le bandeau de titre. Il devient :

| pièce | bandeau |
|---|---|
| chapitre | `12 — Un visage utile` *(inchangé)* |
| liminaire, annexe | `Préface` |
| page de partie | `Partie III — Avant Clément` |

Les liminaires et les annexes se relisent comme le reste du texte, numérotation de
lignes comprise : c'est du texte d'auteur. Une page de partie n'a pas de corps, elle
n'apporte que son bandeau.

## 5. L'EPUB

Un EPUB n'a ni belle page, ni verso blanc, ni folio : les pages de partie y deviennent
des sections à titre seul, et rien d'autre ne change de nature.

- `intitule()` (`epub.rs:102`) suit le tableau du § 4, sans le mot « Partie » pour la
  table des matières : `III — Avant Clément`.
- `chapitre_xhtml()` (`epub.rs:115`) devient `piece_xhtml()`. Le
  `<span class="numero">` ne porte que le numéro d'un chapitre ou le romain d'une
  partie ; un liminaire n'a que son `<span class="titre">`.
- `nav_xhtml()` (`epub.rs:442`) et `ncx()` (`epub.rs:464`) prennent les pièces : toutes
  figurent dans la table des matières, dans l'ordre du livre.
- `nom_chapitre(rang)` (`epub.rs:320`) nomme déjà les fichiers par rang : rien à y
  changer, une pièce est un rang.
- La vérification XML (`epub.rs:740`) désigne la pièce fautive par son intitulé plutôt
  que par `chapitre {numero}`.

`ebook.rs` n'a rien en propre : il passe par l'EPUB.

## 6. Vérification

### Le témoin de non-régression

`cargo run --example temoin` doit rendre **exactement le même compte de pages
qu'avant**. Le manuscrit témoin ne porte aucune pièce ; si son dos bouge d'un dixième
de millimètre, la refonte a cassé le chemin nominal.

Le témoin **n'est pas modifié** pour l'occasion : c'est justement parce qu'il ne bouge
pas qu'il sert de repère.

### Le compte de *WIP8*, lui, bouge

Préface et pages de partie ajoutent des pages, donc le dos change. C'est le
comportement voulu, pas un effet de bord : le dos découle de la pagination mesurée,
jamais d'une saisie.

### Tests unitaires — `manuscrit.rs`

Chacun doit avoir été **vu échouer** avant d'être vert (TDD ou mutation ciblée) :

- les quatre formes d'en-tête sont reconnues, avec leur `Sorte` ;
- `## préface` est reconnu, `## Preface` refusé ;
- `## Chapitre premier` reste refusé avec sa ligne — le test existant
  (`manuscrit.rs:463`) ne doit pas avoir à changer ;
- une pièce liminaire après un chapitre est refusée avec sa ligne ;
- un chapitre après une annexe est refusé avec sa ligne ;
- `Partie IV` après `Partie II` est refusée ;
- un paragraphe sous une page de partie est refusé avec sa ligne ;
- `attendu` ne compte que les chapitres : un manuscrit `préface + 2 chapitres` passe
  `decoupe(md, Some(2))`.

### Tests unitaires — les sorties

- `interieur.rs` : la préface est composée **avant** le `#set page(footer:` du corps ;
  une page de partie produit deux `#page(footer: none)` et le chapitre suivant ne laisse
  pas de page blanche supplémentaire (compte de `#pagebreak()` et de `#page(`, comme les
  tests de dédicace `interieur.rs:787`) ;
- `epreuve.rs` : le bandeau d'un liminaire ne porte pas de numéro ;
- `epub.rs` : un liminaire n'émet pas de `<span class="numero">`, et toutes les pièces
  figurent au `nav` et au `ncx`.

### Avant commit

`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`,
`node --test tests/*.test.js`, puis le témoin.

Et l'essai qui a motivé le chantier : *WIP8* s'importe, se compose, et sa préface paraît
là où elle doit paraître.

## 7. Hors périmètre

- **La sous-partie.** `###` reste refusé.
- **Le sommaire imprimé.** Le livre n'en a pas ; les pièces ne lui en donnent pas un.
- **La pagination romaine des liminaires.** Écartée au cadrage : la préface n'a pas de
  folio du tout.
- **Un champ « préface » dans l'onglet Livre.** Écarté au cadrage : le texte vit dans
  le manuscrit.
- **L'ouverture des chapitres en belle page.** Le projet fait un `#pagebreak()` simple
  et continue de le faire ; ce chantier ne change pas cette convention.
- **`outils/`.** La chaîne Python est archive et ne reçoit pas les pièces.
