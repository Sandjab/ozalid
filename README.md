# Ozalid

Générateur de maquettes de première de couverture. Outil HTML autonome, sans build, sans dépendance serveur.

Le nom vient du terme de prépresse désignant l'épreuve de contrôle avant tirage.

## Usage

Ouvrir `index.html` dans un navigateur. Rien à installer.

## Ce que ça fait

Trois modes de mise en page :

- **Bandeau** — bande de titre en haut, image à fond perdu en dessous (archétype Folio / Penguin Modern Classics)
- **Surimpression** — image sur toute la surface, texte par-dessus, avec voile de lisibilité réglable
- **Sans image** — composition purement typographique (archétype Blanche / NRF)

Trois maquettes de départ préchargées : `Folio`, `Blanche`, `Surimpression`. Chacune recharge l'intégralité des réglages.

Un générateur de cadre indépendant du mode reproduit le triple filet Gallimard (filet noir + double filet rouge), paramétrable sur six axes.

## Réglages embarqués dans le PNG

À l'export, la configuration complète est écrite dans un chunk `tEXt` du PNG sous la clé `atelier-couverture`, avec optionnellement la photo source rééchantillonnée. Recharger le PNG dans l'outil restaure la maquette entière.

Le fichier reste un PNG standard : lisible par n'importe quel visualiseur, PIL et `exiftool` voient le bloc.

Les réglages peuvent aussi être exportés seuls en JSON — plus léger, versionnable, lisible en diff.

## Unités

Corps de texte, filets et marges sont exprimés en pourcentage de la largeur de couverture. Changer de format ne casse aucun réglage typographique.

## Limites connues

- Une conversion vers JPEG détruit les métadonnées. Conserver le JSON en parallèle si les fichiers passent dans un pipeline de compression.
- Le rendu PNG passe par `html2canvas`, qui approxime certaines propriétés CSS. Voir `HANDOFF.md`.
- Le Didot original de la collection Blanche n'existe pas en version numérique. Bodoni Moda sert de substitut.

## Structure

```
index.html              version courante (copie de v3)
versions/               historique complet, chaque fichier autonome
HANDOFF.md              état du code, décisions, dettes, pistes
CLAUDE.md               instructions pour Claude Code
```
