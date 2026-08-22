# Le blanc de respiration — une coupure muette dans le manuscrit

Date : 2026-08-22
Statut : validé (brainstorming)

## Objectif

Le manuscrit ne connaît qu'une seule façon de séparer deux passages d'un chapitre :
`---`, qui compose un astérisme `* * *` centré entre deux blancs (`manuscrit.rs:41`,
`interieur.rs:480`). C'est une coupure qui se voit, et c'est voulu — la marque a été
choisie parce qu'un blanc seul ne survit pas à une fin de page.

Mais toutes les coupures ne demandent pas d'être annoncées. L'auteur veut aussi
respirer : laisser tomber une ligne entre deux passages sans planter trois astérisques
au milieu de sa page. Aujourd'hui il ne le peut pas — une ligne vide est ignorée par
`decoupe` (`manuscrit.rs:375`), et il n'existe aucune autre marque.

Cette spec ajoute une **seconde coupure, muette** : le blanc de respiration. Les deux
coexistent dans le même chapitre, chacune à sa marque ; c'est l'auteur qui décide, à
chaque coupure, si elle se voit ou non.

## Décisions de cadrage (brainstorming du 22/08)

- **Une respiration discrète, pas une coupure forte.** Elle a la même intention que
  `---` — séparer deux passages — mais renonce à être vue coûte que coûte. Si elle
  tombe à une frontière de page, elle se perd : c'est le comportement de la typographie
  courante, et il est accepté ici.
- **Le blanc n'est jamais forcé.** Le faire survivre à un saut de page décalerait le
  haut de la page suivante et romprait le registre — les lignes ne seraient plus en
  regard d'une page à l'autre. Le registre passe avant la coupure.
- **La marque est `___`**, jumelle de `---` dans le Markdown standard.
- **Une ligne de blanc, pas deux.** L'écart d'une ligne de texte laissée vide, celui
  de l'édition courante. Deux lignes creuseraient le gris typographique et, sur un
  chapitre qui en compte beaucoup, mangeraient des pages — donc épaissiraient le dos.
- **L'épreuve, elle, montre la coupure.** Un blanc muet y serait invisible et le
  relecteur ne pourrait pas vérifier qu'il a bien été saisi. Elle porte un filet gris,
  comme elle porte déjà l'astérisme en gris de service.

## 1. Le format du manuscrit

Une ligne contenant exactement `___` — comparaison stricte après `trim`, comme celle
de `---` (`manuscrit.rs:365`).

| écrit | coupure | ce qui s'imprime |
|---|---|---|
| `---` | rupture de scène | un blanc, `* * *` centré, un blanc |
| `___` | blanc de respiration | une ligne vide, rien d'autre |

`refus()` n'est pas touché : `___` n'y a jamais figuré, et aucune de ses six formes ne
l'attrape. Le choix de `___` tient à trois raisons, dans cet ordre :

- C'est l'autre séparateur du Markdown standard, exactement au même rang que `---` :
  deux marques jumelles pour deux coupures, la marquée et la muette. Un manuscrit
  ouvert dans n'importe quel éditeur Markdown y montre déjà une ligne.
- Aucune collision avec les refus. `- - -` aurait exigé de percer le garde-fou qui
  écarte les puces de liste ; `***` aurait fait écrire trois astérisques pour obtenir
  un blanc, quand `---` en imprime.
- Impossible à taper par accident à la place de `---` : la faute de frappe qui
  transformerait une coupure en l'autre n'existe pas.

Vérifié le 22/08 : aucun des 70 manuscrits de `build/in/texts/` ne porte de ligne
`___`. Le format s'ouvre donc sans repaginer un seul livre déjà composé.

Le doc-comment de tête de `manuscrit.rs` énumère le format admis — « chapitres en
`## NN - Titre`, séparateurs de scène `---`, emphase `*…*` et `**…**` ». Il gagne la
nouvelle marque : c'est là, et nulle part ailleurs dans le dépôt, que le format se
lit en entier.

## 2. Le modèle

```rust
pub enum Bloc {
    Paragraphe(String),
    Scene,
    Blanc,
}
```

Un variant, pas un paramètre. `Bloc::Rupture(Marque)` généraliserait un besoin qui
n'existe pas — filet, fleuron, cul-de-lampe — au prix de casser les trois rendus.

Le variant nu porte le gain décisif : le `match` exhaustif de Rust fait **échouer la
compilation** aux trois sites de rendu — `interieur.rs:480`, `epreuve.rs:146`,
`epub.rs:143` — tant qu'ils n'ont pas traité le nouveau cas. Aucune sortie ne peut
oublier le blanc en silence, ce qui est la garantie même que `manuscrit.rs` réclame en
tête de fichier.

### Les règles de position, communes aux deux coupures

Une coupure sépare deux passages d'un même chapitre. Celle qui n'a rien à séparer
n'existe pas :

- **Ignorée en tête de chapitre** — pas de passage précédent.
- **Élaguée en fin de chapitre** — pas de passage suivant. C'est le cas réel de
  *WIP7*, dont les 64 `---` annoncent chacun le chapitre suivant.
- **Jamais doublée** — deux coupures consécutives ne séparent qu'une fois.

Ces trois règles tiennent aujourd'hui dans un test sur `Bloc::Scene`
(`manuscrit.rs:341` et `370`). Elles passent de « le dernier bloc est une rupture de
scène » à « le dernier bloc est une **rupture** », l'un ou l'autre des deux variants.

Conséquence assumée : un `---` immédiatement suivi d'un `___` ne pose qu'une coupure,
la première. La marquée gagne, parce qu'elle est arrivée d'abord — pas de règle de
priorité à retenir, juste l'ordre d'écriture.

## 3. L'intérieur imprimé

```typst
#v(…, weak: true)
```

Rien d'autre : pas de marque, pas d'alignement, pas de bloc.

`weak: true` est précisément ce que la décision de cadrage demande. En Typst, un
espacement faible est supprimé à une frontière de page : le blanc disparaît s'il tombe
là, le registre reste tenu, et aucune page ne s'ouvre sur un trou. C'est la
transcription exacte de « si elle tombe en fin de page, elle se perd ».

La hauteur suit l'interligne du projet (`lead`, `interieur.rs:187`), jamais une valeur
en points : le blanc doit rester une ligne quand l'auteur change l'interligne ou la
police. La boîte de ligne vaut 1 em par construction (`top-edge: 0.75em, bottom-edge:
-0.25em`, README « pièges connus »), donc une ligne complète vaut `1em + lead`.

Deux espacements faibles adjacents fusionnent en gardant le plus grand, et
`par.spacing` vaut déjà `lead` : la valeur posée est donc `1em + lead * 2`.

**Relevé sur PDF le 22/08**, et non déduit : à 150 dpi, l'écart entre deux lignes vaut
28 px et celui que le blanc ouvre en vaut 57 — une ligne sautée exactement. Typst
fusionne bien ; s'il avait additionné, l'écart aurait été de 63 px.

## 4. L'épreuve

```typst
#v(3mm)
#align(center)[#line(length: 12mm, stroke: 0.4pt + rgb("#c0c0c0"))]
#v(3mm)
```

L'épreuve n'est pas le livre : A4, fer à gauche, marge d'annotation, numéros de ligne,
astérisme en gris `#808080` — elle ne ressemble à aucune page imprimée et ne promet
aucune fidélité. C'est un document de travail sur le texte.

Or une coupure muette y serait une coupure invisible : le relecteur ne pourrait pas
vérifier qu'elle a bien été saisie, ni la distinguer d'un blanc qu'il aurait cru voir.
Le filet gris la lui montre, dans la même couleur de service que l'astérisme — celle
qui ne s'imprime jamais. Le blanc reste blanc sur le livre ; l'épreuve, elle, montre
l'intention.

Gris plus clair que celui de l'astérisme (`#c0c0c0` contre `#808080`) : la coupure
muette doit se lire comme la plus légère des deux, jusque sur l'épreuve.

## 5. L'EPUB

```xhtml
<p class="blanc"> </p>
```

— où l'espace est un U+00A0.

```css
p.blanc { margin: 0; text-indent: 0; }
```

L'espace insécable n'est pas une précaution de style : les liseuses escamotent les
`<p>` vides, et le blanc disparaîtrait sans lui. Il est écrit en caractère littéral
U+00A0, comme `SCENE_XHTML` (`epub.rs:83`), pas en entité — le document est du XHTML
sans DTD, où `&nbsp;` n'est pas défini.

`margin: 0` parce que la ligne du paragraphe **est** le blanc : une marge s'y
ajouterait au lieu de le composer. C'est la différence avec `p.scene`, qui porte
`margin: 1em 0` de part et d'autre de sa marque (`epub.rs:555`).

## 6. Vérification

### Le témoin de non-régression

`cargo run --example temoin` doit rendre **exactement le même compte de pages**
qu'avant le chantier. *Candide* ne porte aucun `___`, et le nouveau variant n'ajoute
rien à la source Typst des manuscrits qui l'ignorent. Un écart, même d'une page,
signalerait que le rendu a fui hors de son cas.

### Tests unitaires — `manuscrit.rs`

Chacun **vu rouge** avant la correction (CLAUDE.md § vérifications) :

- `___` entre deux paragraphes donne `[Paragraphe, Blanc, Paragraphe]`.
- `___` en tête de chapitre ne laisse pas de bloc.
- `___` en fin de chapitre est élagué.
- Deux `___` consécutifs ne donnent qu'un `Blanc`.
- `---` suivi de `___` ne donne qu'un `Scene`, et l'inverse qu'un `Blanc`.
- `___` avant le premier `##` n'ouvre pas de chapitre fantôme.

### Tests unitaires — les sorties

- `interieur` : la source porte un `v(` faible et **aucune** occurrence de `SCENE`
  pour un chapitre qui n'a qu'un `Blanc`.
- `epreuve` : la source porte le filet ; le gris du filet n'est pas celui de
  l'astérisme.
- `epub` : le XHTML porte `<p class="blanc">`, son espace insécable, et la feuille de
  style porte `p.blanc`.

### Mesure sur le PDF

La hauteur du blanc de l'intérieur est **relevée sur un PDF composé**, pas déduite :
un manuscrit de deux paragraphes séparés par `___`, comparé au même sans. L'écart doit
valoir une ligne de texte. C'est cette mesure qui fixe la valeur du § 3.

### Avant commit

`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` depuis
`app/src-tauri/`, `node --test tests/*.test.js` depuis `app/`.

## 7. Hors périmètre

- **Aucun réglage d'interface.** La hauteur du blanc n'est pas un paramètre de projet :
  une ligne, toujours. Si le besoin apparaît, il ouvrira son propre chantier.
- **Aucune troisième marque** — filet imprimé, fleuron, cul-de-lampe. Deux coupures
  suffisent tant qu'une troisième n'est pas demandée.
- **Le blanc forcé n'est pas offert.** Celui qui veut une coupure qui survive à la fin
  de page dispose déjà de `---` : c'est exactement ce pour quoi la marque existe.
- **La documentation du format côté interface** reste inchangée : `app/README.md`
  décrit le sous-ensemble admis en prose, sans énumérer les marques.
