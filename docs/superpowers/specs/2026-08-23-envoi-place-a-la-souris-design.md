# L'envoi se place à la souris, sur la page qu'on veut

Date : 2026-08-23
Statut : validé (brainstorming du 23/08)

## Objectif

Aujourd'hui l'envoi autographe est un mot posé au bas de la page de titre, à
28 mm du bord, dans une main que **le livre entier** partage. Trois choses
manquent, et elles se tiennent :

- **La main appartient au livre.** Un auteur qui veut écrire son mot à la main
  pour Léa et le faire composer pour Marc ne le peut pas : choisir « image
  écrite à la main » l'impose aux vingt exemplaires. C'est la contrainte qui
  rend la liste de dédicataires moins utile qu'elle ne devrait l'être.
- **La page est fixe.** Un envoi va sur la page de titre, et nulle part
  ailleurs. Ni la page de garde, ni le faux-titre, ni la première page du
  chapitre que le dédicataire a inspiré.
- **La place est un chiffre écrit dans le code.** `bottom + center, dy: -28mm,
  width: 70%` : ni position, ni échelle, ni inclinaison ne se règlent, et
  encore moins à la souris.

Cette spec descend la main dans l'exemplaire, ouvre le placement à toutes les
pages, et le rend à la souris sur un canevas où la page se voit en vrai.

## Décisions de cadrage (brainstorming du 23/08)

- **La main descend du livre vers l'envoi.** C'est le point de départ : sans
  lui, « image écrite à la main » reste un choix qui engage tout le tirage. Ce
  qui reste au livre, c'est ce que l'archive porte (`personnelle`) et ce qui n'a
  aucun sens à réécrire par personne (le `gabarit` de diffusion).
- **Un envoi neuf naît comme le précédent** — même main, même placement. Sans
  cette règle, vingt dédicataires demanderaient vingt fois le même réglage, et
  la ressemblance des exemplaires d'un même livre, qui était acquise, se
  paierait.
- **Le placement s'exprime en fractions de page**, jamais en millimètres. C'est
  la règle d'`index.html` — « tout réglage est en pourcentage » — et c'est ce qui
  rend un placement portable d'un format à l'autre.
- **L'échelle grossit l'objet entier, lettres comprises.** Tirer un coin à la
  souris, c'est agrandir une signature, pas élargir une colonne de texte et la
  laisser se recomposer. Conséquence assumée : le corps suit la taille.
- **Le canevas ne simule rien.** Le fond est la page rendue par Typst, l'objet
  est l'envoi rendu par Typst sur fond transparent. Ce qu'on manipule est ce qui
  s'imprimera, à l'antialiasing près — pas une approximation du navigateur.
- **Une page qui n'existe pas chez un prestataire fait refuser sa génération**,
  en nommant la page et le compte. Pas de repli silencieux sur la dernière page.

## Ce qui est vérifié, et non supposé

Deux hypothèses portaient la conception. Les deux ont été éprouvées sur le vrai
livre témoin — *Les Heures creuses*, 190 pages, kdp-5x8 — avant d'écrire cette
spec, avec le sidecar épinglé (Typst 0.15.1).

**Le `foreground` de page conditionnel ne crée aucune page.** Une source portant
un `#set page(foreground: context { if counter(page).get().first() == 3 {…} })`
au préambule et la même source sans lui composent **6 pages toutes les deux**. Le
`foreground` survit au `#set page(footer: …)` qui ouvre le corps — les `set` de
Typst fusionnent champ à champ — et aux `#page(…)[…]` des pages de partie. Ses
pourcentages se résolvent sur la **page entière, marges comprises**, ce qui les
met en correspondance 1:1 avec un canevas qui montre la page entière.

**Le coût du rendu ne contraint rien.** Toutes les vignettes en une invocation :
**0,58 s** pour 190 pages à 24 ppi (120 × 192 px), 1,9 Mo au total. Une page en
grand à 150 ppi (750 × 1200) : **0,19 s**. L'objet d'un envoi seul, à 300 ppi,
fond transparent, hauteur automatique : **20 ms**, et le pixel du coin est bien
à alpha 0. Il n'y a donc ni fenêtrage, ni chargement paresseux, ni cache
sophistiqué à concevoir.

## 1. Le modèle

`envoi.rs`. La main quitte `Envois` pour `Envoi`, et le placement arrive.

```rust
pub struct Envoi {
    pub dedicataire: String,
    #[serde(default)] pub main: Main,
    #[serde(default)] pub contenu: String,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub image: Option<String>,
    #[serde(default)] pub place: Place,
}

/// Où l'envoi se pose, en fractions de la page — jamais en millimètres : un
/// placement doit valoir du poche au grand format.
pub struct Place {
    /// Page physique du PDF, à partir de 1 : celle que la vignette montre.
    pub page: u32,
    /// Centre de l'objet, en fraction de la largeur et de la hauteur de page.
    pub x: f64,
    pub y: f64,
    /// Largeur de l'objet, en fraction de la largeur de page.
    pub taille: f64,
    /// Degrés, positif dans le sens horaire.
    pub angle: f64,
}
```

`Place::default()` vaut `{ page: 3, x: 0.5, y: 0.80, taille: 0.60, angle: 0.0 }`.
Page 3 est la page de titre : le faux-titre est en 1, sa blanche en 2. Un projet
écrit avant cette spec retrouve donc son envoi là où il était — au décalage près
que la conversion impose, le placement d'avant étant relatif à la **justification**
et non à la page. L'écart est de quelques millimètres, il est assumé et daté ici.

`Envois` perd `main` et gagne `gabarit` :

```rust
pub struct Envois {
    #[serde(default, skip_serializing_if = "Option::is_none")] pub personnelle: Option<String>,
    /// Le patron de prompt, partagé par tous les envois en diffusion : c'est le
    /// style d'écriture du livre, pas le mot d'une personne.
    #[serde(default)] pub gabarit: String,
    #[serde(default)] pub liste: Vec<Envoi>,
}
```

`Main::Diffusion` devient une variante sans champ. `{"mode":"diffusion"}` se
relit toujours — le test `une_main_generee_se_choisit_avant_d_avoir_son_gabarit`
reste vrai, et le `#[serde(default)]` qu'il protégeait devient sans objet.

`Envois::verifie` boucle désormais sur la liste : chaque envoi dont la main est
une police doit nommer une main de `MAINS` ou la `personnelle` de l'archive.
L'erreur nomme le dédicataire — « Marc : main inconnue “Comic Sans” » —, sans
quoi on ne saurait pas quelle ligne réparer.

`Envois::reprend` **disparaît**, et sa garde avec elle — non par oubli, mais
parce qu'elle devient impossible à violer. Elle existait parce que l'interface
renvoyait l'objet `Envois` entier, `personnelle` comprise, et pouvait donc y
nommer une police que Typst ne trouverait pas. Avec `envoi_regler`, l'interface
ne renvoie qu'un `Envoi`, qui ne porte aucun champ de police personnelle : le
type interdit ce que la garde refusait. Le test
`une_saisie_ne_peut_pas_inventer_une_police_personnelle` s'en va avec elle, et
c'est le seul test que cette spec retire.

### La migration des projets existants — version 4

Un `.ozalid` écrit avant cette spec porte `[envois.main]` au livre et des envois
sans main. La migration se fait dans `migre()`, **sur le `toml::Value` et non sur
les types**, exactement pour la raison que ce module énonce déjà à propos de la
v2 : une fois `Envois` allégée, il n'existe plus de structure Rust capable de
lire ce champ.

Trois gestes sur le TOML brut, quand `version < VERSION` :

1. `envois.main.gabarit`, s'il est là, est posé en `envois.gabarit` ;
2. `envois.main` est recopié dans chaque `[[envois.liste]]` qui n'a pas de
   `main` — c'est ce qui conserve la main du livre à ses vingt exemplaires ;
3. rien n'est **retiré**. Une fois `Envois` sans `main`, serde l'ignore et
   aucune réécriture ne le conserve : le résultat est celui visé, sans code de
   suppression. Le raisonnement est celui de la v2, cité tel quel.

`Place` n'a pas besoin de la migration : `#[serde(default)]` suffit, et le défaut
repose l'envoi sur sa page de titre.

**`VERSION` passe de 3 à 4**, et c'est un écart délibéré à la règle du README
(« la monter interdirait aux binaires déjà distribués d'ouvrir les projets écrits
ensuite »). Cette règle vise l'ajout d'une **section facultative**, qu'un binaire
d'avant traverse sans dommage. Ici un champ se déplace : un binaire v3 qui
ouvrirait un projet v4 ne verrait aucune main d'envoi, et son `#[serde(default)]`
les lui donnerait toutes dans la même écriture — celle que personne n'a choisie.
C'est le repli silencieux contre lequel tout ce module est écrit.

Le refus est déjà en place et n'a rien à recevoir : `Projet::ouvrir` contrôle la
version **avant** de migrer, « un projet venu du futur doit être refusé plutôt
que migré de travers ». Un binaire v3 dira donc « projet en version 4, cette
application lit jusqu'à la 3 », ce qui est vrai et réparable, plutôt que
d'imprimer vingt exemplaires dans la mauvaise main.

La `personnelle` continue d'être **relevée dans le fichier embarqué** à
l'ouverture, jamais lue dans le TOML. Rien ne change là : elle reste au livre.

## 2. Le Typst

`interieur.rs`. Le `#place` de `liminaires()` disparaît, et avec lui le paramètre
`envoi` de la fonction. `Trace` monte au préambule de `assemble`, où elle devient
un `foreground` de page :

```typst
#set page(
  width: …, height: …, margin: (…),
  footer: none,
  foreground: context {
    if counter(page).get().first() == 37 {
      place(center + horizon, dx: 12%, dy: 25%,
        rotate(-6deg, box(width: 60%)[
          #set par(justify: false, first-line-indent: 0pt, leading: 0.9em)
          #text(font: "Caveat", size: 4.94mm, hyphenate: false)[À Léa, …]
        ]))
    }
  },
)
```

Les correspondances, que le Rust calcule :

| `Place` | Typst |
|---|---|
| `page` | `counter(page).get().first() == page` |
| `x`, `y` | `place(center + horizon, dx: (x − 0,5) × 100%, dy: (y − 0,5) × 100%)` |
| `taille` | `box(width: taille × 100%)` |
| `angle` | `rotate(angle deg, …)` — origine au centre, comme en CSS |

`counter(page)` n'est jamais remis à zéro dans l'intérieur : le folio est
seulement masqué jusqu'au corps. « Page 37 » désigne donc bien la 37ᵉ page du
PDF, celle que la vignette montre.

**Le corps suit la taille.** L'objet est self-similaire : `size = taille ×
largeur_page × k`, avec `k` calé pour que `taille = 0,60` rende les 14 pt
d'aujourd'hui. Sur une page de 127 mm : 0,60 × 127 = 76,2 mm, et 14 pt = 4,94 mm,
donc `k = 0,0648`. La constante est nommée et commentée dans `interieur.rs`,
avec ce calcul — un nombre magique de plus dans ce module ne se relirait pas.

L'image, elle, n'a pas de corps : `box(width: taille × 100%)` suffit, et la
borne de hauteur à 30 % du corps disparaît. Elle protégeait d'un envoi qui
recouvrirait le titre ; le canevas montre désormais ce recouvrement, et le
brider contre la volonté de l'auteur serait le corriger d'une faute qu'il voit.

`source_ebook` continue de passer `None` : un ebook n'a pas de dédicataire.

`Trace` gagne le placement, en restant le type qui ignore d'où vient l'image :

```rust
pub struct Trace<'a> {
    pub quoi: Quoi<'a>,     // Texte { police, texte } | Image { fichier }
    pub place: &'a Place,
}
```

## 3. L'écran

Quatre bandes dans l'étape Envois, aucune ne défilant :

```
┌─ Envois ─────────────────────────────────────────────────────┐
│ Dédicataires │ vign │      page 3       │ Réglages           │
│ ▸ Léa        │  ▫1  │  ┌─────────────┐  │ Main [Caveat    ▾] │
│   Marc       │  ▫2  │  │             │  │ Mot                │
│   Sonia      │  ▪3  │  │   ╱À Léa,   │  │ ┌────────────────┐ │
│              │  ▫4  │  │  ╱  bien…   │  │ │À Léa, …        │ │
│ [+ Ajouter]  │  ▫5  │  └─────────────┘  │ └────────────────┘ │
│              │  ⋮   │                   │ Échelle    34 %    │
│ Police perso │      │   [Voir la page]  │ Inclinaison −4°    │
└──────────────┴──────┴───────────────────┴────────────────────┘
```

- **La liste** ne porte plus que le nom : sélectionner, ajouter, retirer. Le
  mot, la main et l'image ont rejoint la colonne de droite, où ils concernent
  l'exemplaire ouvert. Sous elle, ce qui appartient au livre : la police
  personnelle et ses deux boutons, tels qu'ils sont aujourd'hui.
- **Le rail de vignettes** : toutes les pages, la page visée marquée. Cliquer
  une vignette déplace l'envoi sur cette page — c'est le seul moyen d'en changer,
  il n'y a pas de champ « page ».
- **Le canevas** : la page en grand, l'objet posé dessus, une prise de coin pour
  l'échelle et une prise de rotation. Glisser l'objet le déplace.
- **Les réglages** : la main, puis ce que la main réclame — le mot, le choix
  d'image, ou les deux gestes de diffusion et le gabarit du livre. Les chiffres
  d'échelle et d'inclinaison s'y lisent et s'y saisissent, comme sur la
  couverture où prise et champ disent la même valeur.

Le **gabarit** ne paraît que quand l'envoi sélectionné est en diffusion, et son
libellé dit « Gabarit du livre » : il est partagé, et le taire ferait croire
qu'on l'écrit pour cette personne-là.

### Ce que le canevas montre

Le fond est la page **rendue sans envoi**. Elle est invariante : un envoi posé en
`foreground` ne réordonne rien, donc la page de fond ne dépend d'aucun envoi et
sert à tous les dédicataires.

L'objet est l'envoi **rendu par Typst**, seul, sur fond transparent, à hauteur
automatique. Pour une main en image, c'est l'image elle-même. Glisser,
redimensionner et incliner sont alors de purs `transform` CSS sur une image dont
le ratio est celui que Typst composera : le canevas est la vérité, pas une
imitation. Typst n'est rappelé que quand le mot ou la main changent — 20 ms,
débouncé comme le reste.

« Voir la page » reste, et redevient ce qu'il doit être : la confirmation par le
chemin complet, page composée avec son envoi.

### Les gestes

`couverture.js:1135` porte déjà `saisir()`, le moteur de geste de l'application :
capture du pointeur, déplacements exprimés en **fraction de la face et jamais en
pixels** — « un geste calé sur des pixels irait deux fois plus vite dans une
petite fenêtre » —, commit débouncé, et un clic qui n'a rien déplacé n'est pas
commis, pour ne pas marquer le projet modifié.

Le placement suit cet idiome, dans un fichier neuf, `placement.js` : le canevas,
les trois prises, et la géométrie. Il ne connaît que « une page en fond, un objet
de ratio connu, un `Place` » et rend un `Place` nouveau ; c'est ce qui le rend
testable sans DOM.

**Dette signalée, non contractée ici.** `saisir()` est soudé à `#cadreApercu` et
aux chemins de contrôles de la couverture — sept couplages. L'extraire dans un
module commun aux deux scènes est le bon geste, et c'est un remaniement du code
le plus délicat de l'application, sans rapport avec ce que cette spec livre. On
le note et on ne le fait pas : `placement.js` reprend l'idiome, pas le code.

## 4. Les commandes

| Commande | Rôle |
|---|---|
| `envoi_vignettes()` | rend **toutes** les pages de l'intérieur en une invocation, et rend leurs chemins |
| `envoi_page(n)` | une page en grand, 150 ppi |
| `envoi_objet(i)` | l'objet de l'envoi, PNG transparent, avec son ratio |
| `envoi_regler(i, envoi)` | un envoi entier : main, mot, placement |
| `envoi_image_choisir(i, chemin)` | inchangée |
| `envoi_generer(i)` / `envoi_accepter(i)` | inchangées, le gabarit se lisant sur `Envois` |
| `envoi_apercu(i)` | inchangée : la page composée avec son envoi |

`envois_modifier` disparaît au profit de `envoi_regler` : la liste entière ne
voyage plus à chaque frappe, et le piège qu'elle portait — « une main omise
reviendrait au défaut » — s'en va avec elle. Ajouter et retirer restent deux
commandes de liste, `envoi_ajouter` et `envoi_retirer`.

Les vignettes et la page en grand se rendent depuis
`interieur::source(…, None)`, dans un répertoire temporaire nommé par l'empreinte
de cette source. Une composition qui change change l'empreinte, donc le
répertoire : il n'y a pas d'invalidation à écrire, seulement un nom à calculer.
Le répertoire d'avant reste sur le disque temporaire du système, qui est fait
pour cela.

`Typst` gagne une méthode, à côté d'`apercu` :

```rust
/// Rend toutes les pages en PNG, en une seule invocation : le motif porte le
/// `{p}` que Typst substitue. Rendre page à page coûterait une composition
/// complète par page — 190 fois pour un livre ordinaire.
pub fn apercus(&self, source: &Path, motif: &Path, ppi: u32) -> Result<Vec<PathBuf>, String>
```

## 5. La page qui n'existe pas

Le même manuscrit ne fait pas le même nombre de pages en poche et en grand
format. Une page choisie à l'œil chez KDP n'est pas la même chez Lulu, et
au-delà du compte le plus court, elle n'existe pas du tout.

Pour les liminaires — faux-titre, blanche, titre, copyright, dédicace — les
pages coïncident d'un format à l'autre, et c'est là qu'un envoi va dans les
faits. Ailleurs, `package::assembler_envois` **refuse**, en nommant la
personne, la page et le compte :

> Léa : envoi placé page 210, l'intérieur Lulu n'en fait que 198.

C'est la convention de la maison, celle du dos non publié : « la génération
refuse en disant quoi faire », le chiffre mesuré compris. Rogner sur la dernière
page enverrait à l'impression un exemplaire que personne n'a voulu.

Le rail de vignettes montre les pages du **destinataire visé au pied de
fenêtre** : changer de destinataire change le rail, et un envoi hors bornes s'y
voit avant la génération.

## 6. Ce qui se vérifie

Aucun test n'est écrit qui n'ait été **vu échouer** — TDD ou mutation ciblée.

**Rust.**

- Un envoi placé page 37 ne change pas le compte de pages. Test réel, Typst
  lancé, avec et sans : c'est l'invariant qui tient le dos, la planche et la
  promesse entière, et le vérifier sur la seule source ne prouverait rien.
- `Place::default()` pose l'envoi page 3 : un projet d'avant retrouve sa page de
  titre.
- Un `projet.toml` en v3 portant `[envois.main]` au livre donne cette main à
  chaque envoi, et le fichier réécrit est en v4 sans la porter. Le test se
  monte sur un TOML v3 littéral, pas sur les types du jour : c'est le fichier
  d'hier qu'il s'agit de relire, et les types d'hier n'existent plus.
- Un `gabarit` niché dans `[envois.main]` remonte sur `Envois`.
- Un envoi déjà pourvu d'une main ne se la fait pas écraser par celle du livre.
- `verifie` nomme le dédicataire fautif, pas seulement la main.
- Une page hors bornes fait refuser la génération, et le message porte les deux
  chiffres.
- Deux envois de mains différentes dans le même livre composent chacun dans la
  sienne — c'est le fait que cette spec ajoute, et rien d'autre ne le protège.

**Front** (`node --test`, via `dom_shim`).

- Un glisser de N px sur un canevas de W px déplace de `N/W` : la géométrie est
  en fractions, et un canevas de taille différente ne doit pas changer le geste.
- La rotation tourne autour du centre de l'objet.
- `x`, `y` et `taille` restent dans leurs bornes.
- Un clic qui ne déplace rien ne marque pas le projet modifié.
- Sélectionner un dédicataire change la page affichée et l'objet.

**Chaîne.** `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
`cargo test`, `node --test tests/*.test.js`, et `cargo run --example temoin` —
le compte de pages affiché, comparé au précédent sur le même manuscrit.

## Ce que cette spec ne fait pas

- **Un envoi par page, et une page par envoi.** Pas de mot en page de garde
  *et* de signature au colophon.
- **Aucun placement au livre.** Il n'y a pas de « placement par défaut » à
  côté de la main : l'héritage du précédent envoi rend le réglage collectif
  sans introduire un second niveau à afficher, à tester et à expliquer.
- **Aucune extraction de `saisir()`.** Voir la dette signalée au § 3.
- **Aucun ancrage structurel.** Un envoi vise une page physique, pas « le début
  du chapitre 3 ». La limite est celle du § 5, et elle est assumée.
