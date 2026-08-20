# Ozalid — origine et notes durables

Ce document ne décrit plus l'état du code : il a été vidé de son instantané du
13 août 2026, périmé depuis. Pour l'état courant, lire le `README.md` (usage,
outils, organisation du dépôt) et `docs/superpowers/` (specs et plans, chacun
avec son journal d'exécution). Ne restent ici que les choses qui ne vieillissent
pas : d'où vient le projet, ce qu'on a appris de la Blanche, pourquoi
`html2canvas` impose sa discipline, et où le terrain est juridiquement meuble.

---

## 1. Origine

Le projet part d'une question de repérage : trouver des exemples de couvertures poche où le haut porte un bandeau uni avec le titre, et où le reste est une photo pleine page. Ce dispositif a un nom en design éditorial — **bandeau de titre supérieur + image à fond perdu** — et sa référence canonique est la charte Penguin Modern Classics de Jim Stoddart (2002).

L'outil est né du besoin de tester des proportions concrètement plutôt que de raisonner dans le vide. Il a ensuite absorbé un second archétype, la collection Blanche de Gallimard, puis un troisième, la surimpression.

Le livre de travail est *Au Petit Remords*, par Ivan Pjig. Ces valeurs sont les défauts des champs texte, pas des constantes.

Le manuscrit de travail, lui, est *Les Heures creuses*, du même auteur : c'est sur lui que se mesure la pagination, donc toute non-régression de la composition. Un trait de ce texte est à connaître avant d'en tirer des conclusions : ses soixante-quatre `---` précèdent tous un `## `. Ce sont des filets de chapitre, pas des ruptures de scène, et le livre n'en contient aucune à l'intérieur d'un chapitre — alors que le format documenté appelle `---` un « séparateur de scène ». L'épreuve de relecture n'en composera donc aucune sur ce livre : ce n'est pas la fonctionnalité qui manque, c'est le matériel qui n'en porte pas.

---

## 2. Analyse de la Blanche, pour mémoire

Les valeurs du preset `blanche` ne sont pas approximatives. Elles viennent d'un échantillonnage direct des pixels d'une couverture Gallimard (*Beauté*, Philippe Sollers) :

- papier `#FCF0D8`
- rouge `#C00000`
- noir pur `#000000`
- marge du cadre ≈ 9 % de la largeur, retrait du double filet ≈ 4 %

Sur la typographie, Gallimard documente lui-même sa charte : monogramme NRF passé du Garamond au Didot italique, puis Didot gras adopté pour le nom de l'auteur et le titre. Le cadre — un filet noir encadrant un double filet rouge — est un invariant de la collection depuis 1911, inspiré des Éditions de La Phalange.

**Limite non contournable** : les didones du catalogue Deberny et Peignot utilisées sur les Blanche de l'entre-deux-guerres n'ont jamais été numérisées. Bodoni Moda est un substitut de la même classification Vox-ATypI, avec un contraste et des empattements filiformes proches, mais ce n'est pas le caractère d'origine. Aucune solution disponible aujourd'hui ne l'est.

---

## 3. `html2canvas`, le maillon faible

Bibliothèque de réimplémentation du rendu CSS, pas une capture navigateur. Elle approxime, et son comportement sur les gradients, les bordures fines et le rendu typographique ne correspond pas toujours à l'écran. Toute modification visuelle doit être vérifiée sur le fichier exporté, jamais seulement dans la fenêtre.

C'est cette contrainte qui explique une forme du code qui paraîtrait sinon tordue : le cadre est fait de `<div>` imbriquées (`.frame > .frame-r1 > .frame-r2`) aux bordures recalculées en px à chaque `render()`, parce que `outline` et `box-shadow` sont mal rendus à la capture.

---

## 4. Dette de code encore ouverte

**Le positionnement vertical du bloc titre en mode `band` est bancal.** `render()` contient deux affectations successives de `block.style.top` (`index.html:1020-1021`), la seconde écrasant la première. Le résultat est correct mais le code ne l'est pas — à nettoyer.

**Les ruptures de scène n'atteignent pas le livre imprimé.** Le manuscrit les note `---`. Depuis le passage aux blocs typés, `manuscrit::decoupe` les conserve et l'épreuve de relecture les compose, mais `interieur::source` les ignore encore : deux scènes séparées s'impriment collées, en alinéas consécutifs, et le blanc que l'auteur a écrit disparaît. Les rendre déplacerait le compte de pages de tous les livres déjà composés — le témoin de non-régression du projet — ce qui mérite un passage à part, mesuré. Un test, `l_interieur_compose_a_l_identique_avec_ou_sans_rupture_de_scene`, fige l'état actuel : il tombera le jour où on s'y mettra, et c'est voulu.

**Un guillemet droit dans le titre casse la composition.** `manuscrit::echappe` protège le markup Typst — il fait précéder `#`, `*`, `_` et leurs semblables d'une contre-oblique — mais laisse passer le `"`. Or le titre et l'auteur arrivent aussi *à l'intérieur d'une chaîne* Typst, dans `#set document(title: "…", author: "…")` : un titre portant un guillemet droit y referme la chaîne, et le compilateur répond `expected comma`. Le saut de ligne appelle le même correctif pour une autre raison : le titre est également interpolé dans la ligne de commentaire qui ouvre la source, et ce qui suit le saut sort du commentaire et s'imprime en tête du livre. La contre-oblique, elle, est déjà sauve par accident — `echappe` la double, ce qui reste valide en chaîne. Le défaut est identique dans `interieur.rs` et dans `epreuve.rs`, donc le correctif appartient à `manuscrit.rs` : une seconde fonction, pour l'échappement de chaîne, distincte de celle qui protège le markup. Jamais rencontré parce qu'aucun titre de test n'en porte.

---

## 5. Points d'attention juridiques

Les presets reproduisent d'assez près des chartes graphiques protégées. `Gallimard`, `NRF`, `folio` sont des marques déposées ; leur présence comme valeurs de champs par défaut est acceptable en usage personnel de maquettage, beaucoup moins si l'outil est distribué ou si les couvertures produites sont publiées.

C'est aussi la raison pour laquelle le dépôt s'appelle *Ozalid* et non un dérivé d'un nom de collection.
