# Revue d'ergonomie — inspecteur contextuel et manipulation directe

Date : 2026-08-14
Statut : validé (brainstorming)

## Objectif

Réorganiser l'interface de l'atelier Ozalid pour mieux utiliser l'espace et permettre le
positionnement des éléments directement sur la couverture (drag), en synchronisation totale
avec les réglages du panneau. Aucun changement du modèle de données ni du rendu exporté.

## Décisions de cadrage

- **Écrans cibles** : grand écran et laptop 13–14" — l'agencement doit s'adapter aux deux.
- **Liberté du drag** : vertical uniquement pour les blocs, plus des poignées latérales pour la
  marge. Pas de position X libre : l'horizontale reste régie par alignement + marge, le modèle
  de paramètres existant est conservé tel quel.
- **Éléments manipulables** : image, frontière du bandeau, cadre, bloc titre, pied éditeur.
- **Agencement retenu** : inspecteur contextuel (option C), préféré à deux panneaux latéraux (A)
  et au panneau à onglets (B) — c'est celui qui épouse le mieux la manipulation directe.
- **Panneau au repos** : réglages généraux + navigateur d'éléments (seconde voie de sélection).

## 1. Agencement général

- **Barre d'actions permanente en haut** :
  - À gauche : maquettes de départ — Folio, Blanche, Surimpression.
  - À droite : « Exporter PNG ▾ » — le bouton d'export, avec un petit menu contenant les trois
    cases actuelles (traits de coupe à l'écran, réglages dans le PNG, photo source jointe) ;
    « Réglages ▾ » — enregistrer JSON, reprendre depuis PNG exporté, reprendre depuis JSON.
- **Scène centrale** : la couverture, centrée, plus grande qu'aujourd'hui.
- **Panneau droit (~344 px)** : l'inspecteur (voir §2).
- **Responsive** (< 820 px) : le panneau passe sous la scène (comportement actuel conservé),
  la barre d'actions défile horizontalement si nécessaire.

## 2. L'inspecteur

**État repos (rien de sélectionné)** :

- Section **Général** : format, mode (bandeau / surimpression / sans image), papier,
  alignement, marge latérale.
- Section **Éléments** (navigateur) : Image, Bandeau, Cadre, Bloc titre, Pied éditeur.
  Chaque ligne sélectionne l'élément. Les éléments désactivés ou masqués (cadre décoché,
  pied masqué, image absente en mode typo) apparaissent grisés mais restent cliquables —
  c'est la voie d'accès aux éléments invisibles sur la couverture.

**État sélection** (clic sur la couverture ou via le navigateur) :

- Le panneau affiche uniquement les réglages de l'élément sélectionné, précédés d'un
  retour « ‹ Général ».
- Répartition des groupes actuels par élément :
  - **Image** : cadrage vertical, échelle, proportions conservées, voile + intensité,
    remplacement du fichier.
  - **Bandeau** : hauteur, cadre blanc style Folio.
  - **Cadre** : affichage, marge, filets (couleurs, épaisseurs, retrait, écartement).
  - **Bloc titre** : position verticale, textes (auteur, titre, genre), typographie auteur,
    typographie titre, écarts, mention de genre.
  - **Pied éditeur** : affichage, monogramme, éditeur, position, corps, couleurs, pastille.
- Sortie de sélection : Échap, clic sur le fond gris de la scène, ou « ‹ Général ».

**Affordances sur la couverture** : liseré léger au survol d'un élément sélectionnable,
liseré accentué + poignées sur l'élément sélectionné.

## 3. Manipulation directe, synchronisée

| Élément | Geste | Paramètre piloté |
|---|---|---|
| Bloc titre | drag vertical | `inBlockY` |
| Pied éditeur | drag vertical | `inImprintY` (distance depuis le bas) |
| Bloc titre ou pied | poignées latérales | `inPadX` (marge symétrique) |
| Image | drag vertical | `inArtY` |
| Image sélectionnée | molette / trackpad | `inZoom` |
| Bandeau | poignée sur la frontière image/papier | `inBand` |
| Cadre | poignée de coin | `inFrameM` |
| Élément sélectionné | flèches clavier | ± un pas du paramètre de position principal (`inBlockY`, `inImprintY`, `inArtY`, `inBand`, `inFrameM`) |

**Principe de synchronisation** : le drag convertit le déplacement souris (px) en % de largeur
de couverture, écrit la valeur dans l'input `inXxx` existant, puis déclenche son événement
`input`. `render()` et l'affichage des valeurs suivent — sliders et drag pilotent le même
paramètre, aucun état nouveau, aucune divergence possible.

## 4. Contraintes préservées (non négociables)

- Fichier unique `index.html`, ouvrable en `file://`, pas de dépendance nouvelle.
- `render()` reste le seul point d'écriture du style de la couverture.
- Tout réglage reste en % de largeur de couverture.
- **Tous les contrôles `inXxx` restent dans le DOM** : les groupes non affichés par
  l'inspecteur sont masqués (CSS), jamais retirés. `collectConfig` / `applyConfig`,
  `PRESETS` et le round-trip PNG fonctionnent à l'identique.
- Les sélecteurs de sérialisation (`.panel input[id^="in"]`) seront élargis pour couvrir la
  barre d'actions et ses menus (certains contrôles `inXxx` y migrent).
- Liserés, poignées et surbrillances vivent dans un calque **hors de `.cover`** (comme les
  traits de coupe) : ils n'apparaissent jamais dans l'export `html2canvas`.
- `fitCover()` appelée après tout changement de format, avant `render()` (inchangé).

## 5. Vérifications avant livraison

- `node --check` sur le JS extrait.
- Round-trip : export PNG → rechargement → tous les contrôles identiques (y compris ceux
  déplacés dans la barre et les menus).
- Trois modes × trois presets après modification de `render()` et du panneau.
- Pour chaque élément manipulable : drag → le slider reflète la valeur ; slider → l'élément
  bouge ; les bornes min/max du slider sont respectées par le drag.
- Export PNG sans artefact de sélection (liseré, poignées) ni traits de coupe.
- Vérification sur viewport étroit (< 820 px) : panneau dessous, barre utilisable.
