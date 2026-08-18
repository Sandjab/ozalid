# Packaging couverture pour l'auto-édition — onglets 4ème et assemblage

Date : 2026-08-18
Statut : validé (brainstorming)

## Objectif

Industrialiser dans l'app la production de la planche de couverture complète
(4ème + dos + 1ère) prête à téléverser chez un prestataire d'auto-édition, en
remplacement de la partie « couverture » de la chaîne Python de `build/lulu/`.
La composition de l'intérieur du roman reste hors périmètre : elle demeure dans
la chaîne Python (pandoc → weasyprint), l'app n'en connaît que le **nombre de
pages**, saisi à la main.

## Décisions de cadrage

- **Périmètre : couverture seule.** Composer un roman paginé dans le navigateur
  (paged.js ou équivalent) ferait changer l'app de nature ; rejeté.
- **Onglet « 1ère de couverture » = l'atelier actuel, inchangé.** La 1ère
  assemblée est exactement la maquette en cours.
- **Contenu de la 4ème** : texte de présentation, pied de 4ème (mention, prix,
  numéro de collection), zone code-barres/ISBN réservée (vide pour l'usage
  privé), fond hérité de la 1ère ou distinct.
- **Prestataires : Lulu seul au départ**, via un objet `PROVIDERS` extensible
  (même esprit que `PRESETS`). Amazon KDP et autres viendront comme simples
  entrées supplémentaires.
- **Contraintes du projet inchangées** : fichier unique ouvrable en `file://`,
  une seule clé `localStorage`, tout réglage en % de largeur de couverture,
  contrôles nommés `inXxx` pour la persistance automatique.

## 1. Navigation par onglets

Trois onglets dans la barre haute : **1ère de couverture** (atelier actuel),
**4ème de couverture**, **Assemblage**. Les onglets montrent/masquent des vues
sans rechargement, sur le modèle du sélecteur de modes (`setMode`).

## 2. Onglet « 4ème de couverture »

Un second élément couverture (`#cover4`), même format et même logique `--cw`
que la 1ère, rendu par la même passe `render()` (qui reste l'unique endroit
écrivant les styles). Contenus, tous en % de largeur :

- **Texte de présentation** : textarea (extrait ou argumentaire) ;
  police/corps/interlignage/justification/marges, par défaut hérités de la
  palette de la 1ère, débrayables.
- **Pied de 4ème** : mention éditeur, prix, numéro de collection.
- **Zone code-barres/ISBN** : cadre blanc aux dimensions du prestataire,
  activable, vide par défaut.
- **Fond** : couleur de la 1ère reprise par défaut, ou couleur/image distincte.

## 3. Onglet « Assemblage »

- Objet `PROVIDERS`, entrée **Lulu** : formule de dos
  `pages / 17,48 + 1,524 mm` (vérifiée sur le livre réel de 244 pages),
  fond perdu 3,175 mm, dimensions de planche calculées.
- **Entrée : nombre de pages**, avec rappel visible du couplage « si
  l'intérieur regénéré change de compte de pages, mettre à jour ce nombre ».
- **Dos composé dans l'app** : auteur + titre + éditeur en vertical (quart de
  tour anti-horaire, comme la pastille), réglages typo dédiés.
- **Aperçu de la planche complète** (4ème + dos + 1ère) avec traits de coupe et
  fond perdu, réutilisant tels quels les rendus des deux autres onglets.

## 4. Export print

PNG de la planche à 300 dpi (`html2canvas` avec `scale` calculé d'après les
dimensions physiques ; ≈ 2810 × 2140 px pour le format poche actuel),
encapsulé dans un **PDF aux dimensions exactes en mm** via **pdf-lib** (CDN).
C'est la « raison forte » prévue par le CLAUDE.md pour une dépendance
supplémentaire : Lulu attend un PDF. Le PDF est raster 300 dpi, déjà accepté
par Lulu (la couverture actuelle l'est).

## 5. Risques identifiés

- **`html2canvas` sur un canvas ~3000 px** : à valider en premier dans le plan
  d'implémentation (mémoire, fidélité).
- **Piège vérifié le 18/08 : à `scale: 1` exactement, `html2canvas` peint
  l'ombre d'écran de `.cover` (`box-shadow`) à l'intérieur du canvas** — voile
  gris dégradé. À toute échelle > 1, l'ombre n'est pas rendue et l'export est
  propre (l'export actuel à `scale: 3` n'a jamais été affecté ; le « voile à
  l'export » signalé le 17/08 était un artefact de protocole de test à
  `scale: 1`). L'export d'assemblage calculera son échelle : garantir
  `scale > 1` ou neutraliser les `box-shadow` dans le clone `onclone`.
- **Nombre de pages saisi à la main** : prix assumé du choix « couverture
  seule » ; le rappel du couplage est affiché dans l'onglet.

## 6. Découpage en lots

1. **Onglets + 4ème de couverture** — navigation, `#cover4`, contrôles,
   persistance.
2. **Assemblage** — `PROVIDERS`, calcul du dos, dos composé, aperçu planche
   avec traits de coupe.
3. **Export PDF 300 dpi** — échelle calculée, pdf-lib, dimensions exactes.

Chaque lot livre séparément et passe les vérifications du projet : syntaxe
(`node --check`), trois presets et trois modes, round-trip des métadonnées,
contrôle du rendu à l'export et pas seulement à l'écran.
