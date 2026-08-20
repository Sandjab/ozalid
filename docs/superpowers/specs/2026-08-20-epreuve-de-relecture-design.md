# Épreuve de relecture, et la police de l'intérieur

Date : 2026-08-20
Statut : validé (brainstorming)

## Objectif

Jalon 5, premier volet : le module `epreuve` de la spec Ozalid Studio, resté à
l'état d'une ligne de tableau (« PDF A4 de relecture »). Il produit le document
sur lequel un auteur ou un correcteur annote le texte — pas une simulation du
livre imprimé.

Le brainstorming a mis au jour un second sujet, indissociable du premier : la
police de l'intérieur n'a jamais été choisie. Ce volet la choisit.

## Décisions de cadrage (brainstorming du 20/08)

- **Épreuve A4 de relecture**, et non le portage de l'épreuve poche de
  `roman_pdf.py` ni une épreuve prépresse de la planche. Les trois notions
  cohabitaient dans le dépôt sans se recouvrir ; seule celle-ci n'existe nulle
  part.
- **Numéros de ligne**, remis à zéro à chaque page. « p. 42, l. 7 » est la
  convention de l'édition, et c'est le seul repère qui désigne un mot.
- **Toujours le manuscrit entier.** Une épreuve partielle ne se relit pas : les
  répétitions et les fils narratifs sont précisément ce qui ne se voit qu'en
  entier.
- **Module autonome**, sans `Provider` ni convergence.
- **Un seul réglage** : le corps du texte.
- **Les séparateurs de scène sont rendus, à l'épreuve seulement.**
- **L'intérieur passe en EB Garamond.**
- **Le défaut des ruptures de scène perdues à l'impression n'est pas corrigé
  ici** : il est consigné en dette.

## 1. La police de l'intérieur

### Ce qui a été constaté

`interieur::source` n'émet aucun `font:`. L'intérieur est donc composé dans la
police par défaut du binaire Typst — vérifié dans le PDF produit :
`LibertinusSerif-Regular`, `-Bold`, `-Italic`.

C'est une divergence silencieuse d'avec la chaîne Python, qui composait en
**Baskerville** (`gen_interieur.py:196,208`). Elle a échappé au jalon 1 parce
que le témoin de non-régression du projet est le compte de pages : or c'est la
police qui déplace ce compte. Le témoin ne pouvait pas signaler sa propre cause.

`typst.rs` passe `--ignore-system-fonts`, donc le Baskerville du poste est hors
d'atteinte de toute façon — et c'est heureux : un poste sans lui composerait
autrement.

### Ce qui est décidé

L'intérieur passe en **EB Garamond**, déjà embarquée dans `fonts/` avec son
italique. Garalde de labeur, œil petit, économe : le choix classique du roman
français, et l'intention d'un Baskerville sans ses faiblesses en petit corps.

Vérifié sur le sidecar 0.15.1, `--ignore-system-fonts` : Typst la nomme
« EB Garamond », et les axes de la variable répondent — romain, italique vrai,
gras, demi-gras 600, gras italique, ligatures.

Compte de pages des *Heures creuses* au gabarit Lulu, même source, seule la
police changeant :

| Police | Pages |
|---|---|
| Libertinus Serif (l'actuelle, par défaut) | 278 |
| **EB Garamond** | **263** |
| Libre Baskerville | 319 |
| Spectral | 292 |

263 reste dans l'unique tranche de gouttière de Lulu (151–400), et la parité
ajoutera la blanche : **264 pages attendues**. C'est la nouvelle valeur du
témoin, à relever pour de bon après implémentation — la mesure ci-dessus a été
prise sur la source déjà composée, sans rejouer la convergence.

### Où la déclarer

Une **constante du module `interieur`**, émise dans le `#set text(...)` de
l'en-tête de source. Pas un champ de `Provider` : un prestataire impose un
format, des marges et une gouttière, jamais un caractère. Pas non plus un
réglage de projet tant que personne ne l'a demandé.

L'épreuve prend la même police, pour que les deux compositions ne divergent
pas sans qu'on l'ait voulu.

## 2. Les séparateurs de scène

### Ce qui a été constaté

`manuscrit::decoupe` jette les `---` (`manuscrit.rs:119-122`, « sans rendu
propre, comme dans la chaîne actuelle »). Deux scènes séparées par une ligne
blanche s'impriment donc collées, en alinéas consécutifs : le blanc que l'auteur
a écrit disparaît du livre.

### Ce qui est décidé

`Chapitre.paragraphes` devient une liste de **blocs typés** : paragraphe, ou
rupture de scène. C'est la manière du dépôt — `PlaceDos`, `Casse`, `Voile` sont
tous des enums plutôt que des chaînes conventionnelles.

```rust
pub enum Bloc {
    Paragraphe(String),
    Scene,
}
```

`interieur::source` ignore les `Scene` : sa sortie reste **identique à
l'octet près** à ce qu'elle produit aujourd'hui, à la police près. `epreuve`
les compose.

L'épreuve peut se le permettre sans mentir : A4, fer à gauche, 12 pt, marge
d'annotation, numéros de ligne — elle ne ressemble à aucune page du livre et ne
promet aucune fidélité. C'est un document de travail sur le texte, et une
rupture de scène appartient au texte.

Rendu retenu : un blanc vertical portant un astérisque centré. Le blanc seul ne
survit pas à une fin de page.

### Dette ouverte, non traitée ici

Que l'intérieur perde les ruptures de scène à l'impression reste un défaut, pas
une convention — antérieur à l'app, la chaîne Python faisait pareil. Le corriger
déplace le compte de pages de tous les livres déjà composés ; ça mérite son
propre passage, avec le témoin rejoué et comparé. À consigner dans `NOTES.md`.

## 3. Le module `epreuve`

### Frontière

```rust
pub fn source(livre: &Livre, chapitres: &[Chapitre], corps_pt: f64) -> String
```

Aucun `Provider`, aucune convergence, aucune parité : une épreuve ne va chez
personne, et son compte de pages n'intéresse personne. Une seule passe Typst.
Le module ne partage avec `interieur` que le découpage du manuscrit, qui est
déjà un module à part.

Ce sont deux compositions différentes qui se trouvent lire le même texte : les
fondre ferait entrer la relecture dans la fonction la plus critique du dépôt.

### La page

- **A4, recto seul.** Marges symétriques : le recto-verso ne concerne que
  l'imprimeur, une épreuve se lit à plat.
- **25 mm** en tête et au pied, **30 mm** à gauche — la marge porte les numéros
  de ligne —, **50 mm à droite pour annoter**. Colonne de texte 130 mm, environ
  68 signes.
- **Corps 12 pt par défaut**, réglable. Interligne 1,5, alinéa 1,2 em comme
  l'intérieur.
- **Fer à gauche, sans césure**, contrairement à l'intérieur qui justifie.
  Délibéré : une ligne d'épreuve doit tenir au texte, pas à la mise en page. La
  justification masque les espaces doublées et fabrique des lézardes qu'on
  corrigerait pour rien.
- **Numéros de ligne** en 7 pt gris dans la marge de gauche, **remis à zéro à
  chaque page** (`set par.line(numbering-scope: "page")`, natif en 0.15.1,
  vérifié).
- **Un chapitre par page neuve.**
- **En-tête** : `Titre — Auteur` à gauche, chapitre courant à droite.
  **Pied** : `p. 42 / 318`.

### La page de garde

Sans folio : titre, auteur, genre, `Épreuve de relecture — 20 août 2026`, le
compte de chapitres et de mots, et un avertissement — *les numéros de ligne se
rapportent à ce tirage ; une nouvelle épreuve les renumérote*. C'est ce qui rend
une épreuve annotée exploitable trois semaines plus tard.

La date vient de `datetime.today()` de Typst, pas de Rust : la source émise
reste identique d'un jour à l'autre, donc comparable en test, et aucune
dépendance de datation n'entre dans le projet.

### Branchement

Sur les rails existants, sans en inventer :

- `epreuve.rs` à côté de `interieur.rs` ;
- une commande Tauri dans `commands.rs` ;
- `examples/epreuve.rs`, pour l'exercer sans fenêtre — le seul moyen de
  vérifier que Typst avale ce qu'on émet ;
- une section « Épreuve » dans le panneau, à côté de « Packages » ;
- sortie **`epreuve.pdf` à la racine de `out/`**, hors des répertoires
  prestataires. C'est la convention que le README pose déjà : « l'épreuve ne
  vise aucun éditeur ».

## 4. Vérification

Tests unitaires, chacun sur une intention :

- les numéros de ligne repartent à chaque page — un repère `l. 7` continu sur
  tout le livre ne désigne rien ;
- chaque chapitre s'ouvre sur une page neuve ;
- le texte n'est ni justifié ni coupé ;
- une rupture de scène paraît à l'épreuve, et **pas** dans l'intérieur ;
- la garde porte la date du tirage et le compte de chapitres ;
- l'épreuve se compose sans aucun `Provider` ;
- l'intérieur déclare bien sa police, et la déclare une seule fois.

Vérifications de bout en bout, qu'aucun test unitaire ne remplace :

- `cargo run --example epreuve` sur *Les Heures creuses*, PDF compilé puis
  **regardé** : numéros de ligne, marge d'annotation, en-tête, garde ;
- le témoin de non-régression rejoué (`cargo run --example packager`), compte
  de pages relevé et comparé à 278 — attendu autour de 264 ;
- les polices réellement embarquées dans le PDF d'intérieur relues
  (`EBGaramond`, plus de `LibertinusSerif`).

## Hors périmètre

- Corriger les ruptures de scène de l'intérieur (dette consignée).
- Rendre la police réglable par projet.
- L'épreuve poche de `roman_pdf.py` et l'épreuve prépresse de la planche : deux
  autres besoins, deux autres passages si le sujet revient.
- La release Windows, second volet du jalon 5.
