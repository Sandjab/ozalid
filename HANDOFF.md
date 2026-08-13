# Handoff — Ozalid

État du projet au 13 août 2026. Document destiné à reprendre le développement sans avoir suivi la conversation d'origine.

---

## 1. Origine

Le projet part d'une question de repérage : trouver des exemples de couvertures poche où le haut porte un bandeau uni avec le titre, et où le reste est une photo pleine page. Ce dispositif a un nom en design éditorial — **bandeau de titre supérieur + image à fond perdu** — et sa référence canonique est la charte Penguin Modern Classics de Jim Stoddart (2002).

L'outil est né du besoin de tester des proportions concrètement plutôt que de raisonner dans le vide. Il a ensuite absorbé un second archétype, la collection Blanche de Gallimard, puis un troisième, la surimpression.

Le livre de travail est *Au Petit Remords*, par Ivan Pjig. Ces valeurs sont les défauts des champs texte, pas des constantes.

---

## 2. Historique des versions

### v1 — `versions/v1-bandeau.html`

Premier jet. Un seul mode : bandeau + image.

Établit les fondations conservées depuis :

- **Unités relatives.** Tous les corps, marges et espacements sont exprimés en pourcentage de la largeur de couverture, via `calc(var(--cw) * var(--x))`. Décision structurante : elle permet de changer de format sans retoucher la typographie.
- **`render()` unique.** Une seule fonction lit les contrôles et écrit des variables CSS. Pas de manipulation de style dispersée.
- **`fitCover()`** dimensionne la couverture pour tenir dans la zone d'affichage sans jamais déformer le ratio.
- Presets typographiques inspirés du Folio : auteur en grotesque bold rouge, titre en serif régulier noir, ferrage à gauche, pastille de collection en bas à droite.
- Repères de coupe et cote du bandeau affichés à l'écran, exclus de l'export.

Formats disponibles dès cette version : Folio 108×178, poche 110×180, semi-poche 128×198, grand format 140×205, roman 135×215 mm.

### v2 — `versions/v2-trois-modes.html`

Généralisation. Trois modes de mise en page au lieu d'un.

**Modes ajoutés :**

| Mode | Comportement |
|---|---|
| `band` | image sous le bandeau, avec option de cadre blanc périphérique |
| `overlay` | image en `inset: 0`, bloc texte positionné librement de 0 à 85 % de la hauteur |
| `typo` | image masquée, composition purement typographique |

Le mode pilote le positionnement de `.art` et l'affichage conditionnel de trois `fieldset` du panneau.

**Générateur de cadre.** Six paramètres indépendants : marge, couleur et épaisseur du filet externe, retrait du double filet, sa couleur, son épaisseur, écartement entre les deux traits. Réalisé en `<div>` imbriquées (`.frame > .frame-r1 > .frame-r2`) avec des bordures en px recalculées à chaque `render()`. Ce choix est délibéré : `outline` et `box-shadow` sont mal rendus par `html2canvas`.

**Voile de lisibilité** pour le mode surimpression : six variantes (haut, bas, haut+bas, uni sombre, uni clair, aucun) avec intensité réglable. Indispensable pour poser du texte clair sur une photo sans écraser l'image.

**Polices didones ajoutées** : Bodoni Moda (400–900, axe optique), Playfair Display, Prata.

**Trois presets** : `folio`, `blanche`, `overlay`. Chacun recharge l'intégralité des contrôles.

#### Analyse de la Blanche, pour mémoire

Les valeurs du preset `blanche` ne sont pas approximatives. Elles viennent d'un échantillonnage direct des pixels d'une couverture Gallimard (*Beauté*, Philippe Sollers) :

- papier `#FCF0D8`
- rouge `#C00000`
- noir pur `#000000`
- marge du cadre ≈ 9 % de la largeur, retrait du double filet ≈ 4 %

Sur la typographie, Gallimard documente lui-même sa charte : monogramme NRF passé du Garamond au Didot italique, puis Didot gras adopté pour le nom de l'auteur et le titre. Le cadre — un filet noir encadrant un double filet rouge — est un invariant de la collection depuis 1911, inspiré des Éditions de La Phalange.

**Limite non contournable** : les didones du catalogue Deberny et Peignot utilisées sur les Blanche de l'entre-deux-guerres n'ont jamais été numérisées. Bodoni Moda est un substitut de la même classification Vox-ATypI, avec un contraste et des empattements filiformes proches, mais ce n'est pas le caractère d'origine. Aucune solution disponible aujourd'hui ne l'est.

### v3 — `versions/v3-metadonnees-png.html` (courante)

Persistance des réglages.

**Écriture.** À l'export, un chunk `tEXt` est inséré dans le PNG juste après l'IHDR, sous la clé `atelier-couverture`. Contenu : JSON encodé en base64 UTF-8, comprenant le mode, le format, la valeur de chaque contrôle, et optionnellement la photo source rééchantillonnée à 1600 px en JPEG q85.

**Implémentation.** Table CRC-32 précalculée, construction manuelle du chunk (longueur, type, données, CRC), insertion par recopie de tableau. L'encodage base64 découpe en blocs de 0x8000 octets pour ne pas faire déborder la pile d'appels sur les grandes images — `String.fromCharCode.apply` sur un tableau de plusieurs centaines de milliers d'éléments plante sinon.

**Lecture.** Parcours séquentiel des chunks jusqu'à `IEND`, recherche du `tEXt` portant la bonne clé.

**Trois voies de rechargement** : PNG exporté, fichier JSON, ou export JSON seul pour le versionnage.

#### Validation effectuée

Le round-trip a été testé, pas seulement supposé :

1. Fabrication d'un PNG minimal valide sous Node (signature, IHDR, IDAT deflate, IEND).
2. Insertion d'une configuration, relecture, comparaison — identité vérifiée.
3. Ouverture du fichier résultant avec PIL : image valide, clé `atelier-couverture` détectée dans `im.info`, configuration décodée conforme.

Le fichier reste un PNG strictement standard.

#### Automatisme important

`collectConfig()` balaie `.panel input[id^="in"], .panel select[id^="in"]`. **Tout contrôle nommé `inXxx` est donc persisté sans code supplémentaire.** Corollaire : un contrôle nommé autrement sera silencieusement absent de la sauvegarde, sans erreur ni avertissement. C'est le piège principal du code actuel.

---

## 3. Dettes et fragilités

**`html2canvas` est le maillon faible.** Bibliothèque de réimplémentation du rendu CSS, pas une capture navigateur. Elle approxime, et son comportement sur les gradients, les bordures fines et le rendu typographique ne correspond pas toujours à l'écran. Toute modification visuelle doit être vérifiée sur le PNG exporté, jamais seulement dans la fenêtre.

**Pas de gestion du fond perdu au sens imprimeur.** L'export produit exactement le format de coupe, sans les 3 à 5 mm débordants qu'attend un imprimeur. Les repères affichés sont indicatifs, pas des traits de coupe exploitables en production.

**Pas de dos ni de quatrième de couverture.** L'outil ne produit qu'un plat 1. Une couverture complète demanderait une seconde surface, un calcul d'épaisseur de dos à partir du nombre de pages et du grammage, et une logique de prolongement des filets sur le dos — ce que fait la Blanche depuis les années 1920.

**Sortie en PNG seulement.** Pas de PDF, pas de CMJN, pas de profil colorimétrique. Le rouge `#C00000` est une valeur RGB écran ; l'équivalent imprimé demanderait une conversion.

**Aucun test automatisé.** Les vérifications ont été faites manuellement. Il n'y a pas de harnais permettant de détecter une régression.

**Le positionnement vertical du bloc titre en mode `band` est bancal.** `render()` contient deux affectations successives de `block.style.top`, la seconde écrasant la première. Le résultat est correct mais le code ne l'est pas — à nettoyer.

---

## 4. Pistes

Par ordre de rapport valeur / effort décroissant :

1. **Nettoyer le double `block.style.top`** dans `render()`. Correctif de quelques lignes.
2. **Export PDF** via `jsPDF` ou une impression navigateur pilotée, avec fond perdu et traits de coupe réels.
3. **Galerie de maquettes** : conserver plusieurs configurations en mémoire pour comparer côte à côte, sans passer par un export-réimport.
4. **Couverture complète** avec dos et plat 4, calcul du dos par nombre de pages et grammage.
5. **Bibliothèque de presets élargie** : Points, Penguin Modern Classics, Minuit, Actes Sud.
6. **Détection de collision texte / zone claire** en mode surimpression : analyser la luminance de la zone recouverte par le bloc titre et suggérer un voile adapté.

---

## 5. Points d'attention juridiques

Les presets reproduisent d'assez près des chartes graphiques protégées. `Gallimard`, `NRF`, `folio` sont des marques déposées ; leur présence comme valeurs de champs par défaut est acceptable en usage personnel de maquettage, beaucoup moins si l'outil est distribué ou si les couvertures produites sont publiées.

C'est aussi la raison pour laquelle le dépôt s'appelle *Ozalid* et non un dérivé d'un nom de collection.
