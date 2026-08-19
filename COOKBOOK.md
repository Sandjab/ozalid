# Cookbook — publier un roman

Marche à suivre complète, du manuscrit au fichier téléversé chez l'imprimeur. Un chapitre par prestataire ; **Lulu**, **BoD** et **Amazon KDP** sont outillés.

L'ordre n'est pas négociable : **l'intérieur d'abord, la couverture ensuite**. La largeur du dos se calcule à partir du nombre de pages, et ce nombre n'est connu qu'une fois l'intérieur composé. Toute recomposition qui change la pagination oblige à refaire la couverture.

## Avant de commencer

```
brew install pandoc weasyprint
pip install fpdf2 pillow
```

Les scripts lisent `livre.toml` : `gen_interieur.py` exige Python ≥ 3.11 et bascule seul sur un interpréteur récent ; `roman_pdf.py` reste sur celui qui porte `fpdf2` et se rabat au besoin sur `tomli`.

---

## Lulu — poche 108 × 175 mm

### 1. Ranger les ressources

Le manuscrit va dans `build/in/texts/`, la couverture exportée dans `build/in/covers/`. Ces répertoires sont partagés : plusieurs maquettes peuvent viser le même texte.

Format du manuscrit :

```markdown
# Titre du roman

*Roman*

---

## 01 - Titre du premier chapitre

Texte du chapitre.

---

Une scène après un blanc.
```

Le titre en `# `, les chapitres en `## NN - Titre` — la numérotation est obligatoire, le script refuse tout autre gabarit de titre. Les `---` séparent les scènes.

### 2. Créer le répertoire de travail

Un répertoire par **combinaison** texte × couverture, contenant au minimum son `livre.toml` :

```
build/mon-roman/livre.toml
```

```toml
[livre]
titre = "Mon roman"
titre_page = "Mon\nroman"
auteur = "Prénom Nom"
genre = "roman"
copyright = """© Prénom Nom, 2026.
Tous droits réservés."""
manuscrit = "in/texts/mon-roman.md"
couverture = "in/covers/mon-roman.png"
```

Les chemins partent de `build/`. Omettre `chapitres` tant que le texte bouge encore : le compte est affiché à chaque composition, et la clé, une fois posée, fait échouer le script au moindre écart.

### 3. Épreuve de lecture — facultatif

Pour faire relire avant de composer pour de bon :

```
python3 outils/roman_pdf.py build/mon-roman 12 -t 10
```

Sort `build/mon-roman/out/roman.pdf` : la couverture en pleine page, puis les 12 premiers chapitres. Ce PDF est un objet de lecture, **jamais un livrable d'imprimeur** — ni fond perdu, ni gouttière, ni liminaires, et son format est le Folio 108 × 178 mm quel que soit le prestataire visé.

### 4. Composer l'intérieur

```
python3 outils/gen_interieur.py build/mon-roman --provider lulu
```

Sort `build/mon-roman/out/lulu/interieur-lulu.pdf`, accompagné des deux HTML intermédiaires. Le script enchaîne pandoc → composition → weasyprint, puis compte les pages du PDF produit ; si la pagination sort de la tranche de gouttière supposée, il recompose une seconde fois tout seul.

**Relever le nombre de pages affiché** : c'est la seule valeur qui circule vers l'étape suivante.

```
… — 280 pages, 64 chapitres, gouttière 25.0 mm (lulu).
```

### 5. Maquetter la couverture

Ouvrir `index.html` dans un navigateur, puis :

1. **Format** : choisir « Poche Lulu — 108 × 175 mm ». Un autre format donnerait une planche aux mauvaises dimensions, sans erreur visible.
2. Onglet **1ère** : mode, image, typographie, cadre.
3. Onglet **4ème** : texte de présentation, pied, fond. Laisser la zone ISBN vide, le code-barres est apposé par le prestataire.
4. Onglet **Assemblage** : saisir le **nombre de pages relevé à l'étape 4**. Le dos se recalcule, la planche s'affiche à ses dimensions réelles.

Exporter aussi la 1ère avec « Exporter en PNG » et la ranger dans `build/in/covers/` : ce PNG embarque toute la maquette et permet de la recharger telle quelle plus tard.

### 6. Exporter la planche

Onglet Assemblage → **« Exporter la planche (PDF 300 dpi) »**. Ranger le fichier dans `build/mon-roman/out/lulu/`, à côté de l'intérieur.

Pour 280 pages, la planche mesure **239,89 × 181,35 mm**, dos compris (17,54 mm) et fond perdu compris (3,175 mm sur les quatre côtés).

### 7. Téléverser chez Lulu

Créer le projet avec ces paramètres :

| Réglage | Valeur |
|---|---|
| Format | Pocketbook — 4,25 × 6,875 in / 108 × 175 mm |
| Reliure | Paperback (dos carré collé) |
| Encre et papier | Standard Black & White, 60# crème non couché — le classique pour un roman |

Puis envoyer les deux fichiers : `interieur-lulu.pdf` comme intérieur, la planche PDF comme couverture.

### 8. Contrôler avant de valider

- L'aperçu en ligne montre bien **280 pages** — un écart signifie que le PDF téléversé n'est pas celui qu'on vient de composer.
- Le texte du dos tombe dans le dos, sans mordre sur les plats. C'est le premier symptôme d'un nombre de pages erroné.
- Rien d'important ne s'approche à moins de 13 mm du bord : c'est la marge de sécurité, et ce que le massicot peut emporter.
- Le titre et l'auteur sur la couverture correspondent à ceux du `livre.toml`. La couverture est un rendu, le TOML fait foi.

### Gabarit Lulu, pour mémoire

| Grandeur | Valeur | Source |
|---|---|---|
| Format de rognage | 108 × 175 mm | `PROVIDERS` de `gen_interieur.py` |
| Fond perdu | 3,175 mm (0,125 po) | `PROVIDERS` d'`index.html` |
| Dos | pages / 17,48 + 1,524 mm | idem — 280 p. → 17,54 mm |
| Gouttière (marge intérieure) | 25 mm, pour 151 à 400 pages | guide Lulu |
| Marge extérieure | 13 mm | sécurité |
| Marges haut / bas | 14 / 15 mm | |
| Corps du texte | Baskerville 9,5 pt, interligne 1,42 | |

Le guide de référence est dans `build/in/editors/lulu-book-creation-guide.pdf`.

### Pièges

- **Hors de la tranche 151-400 pages**, `gen_interieur.py` refuse de composer plutôt que d'inventer une gouttière. Compléter `PROVIDERS` depuis le guide.
- **Le fond perdu ne prolonge pas l'image** : il reprend la couleur de fond de chaque panneau. Une photo à fond perdu s'arrête donc au trait de coupe — prévoir que le massicot puisse manger jusqu'à 3 mm de composition.
- **Recomposer, c'est refaire la couverture.** Un chapitre ajouté change la pagination, donc le dos, donc la planche.
- **Distribution commerciale** : les maquettes livrées imitent des chartes protégées (Folio, Blanche). Usage privé sans réserve ; ne pas activer la Retail Distribution avec une couverture de ce genre. Voir `NOTES.md`.

---

## BoD — 13,5 × 21,5 cm ou 12 × 19 cm

Même marche à suivre que Lulu ; seules les étapes qui changent sont détaillées ici. BoD a un
avantage décisif pour l'usage privé : **imprimer n'oblige pas à publier**, ni à prendre un ISBN.

### 1 à 3. Ressources, répertoire de travail, épreuve

Identiques au chapitre Lulu — le `livre.toml` ne dépend pas du prestataire.

### 4. Composer l'intérieur

```
python3 outils/gen_interieur.py build/mon-roman --provider bod
```

Sort `build/mon-roman/out/bod/interieur-bod.pdf`, composé en 13,5 × 21,5 cm avec les marges
des modèles Word officiels de BoD : 20 mm côté reliure, 15 mm à l'extérieur, 18,8 en tête,
28 en pied.

Contrairement à Lulu, **la marge de reliure ne dépend pas de la pagination** chez BoD : la
seconde passe converge donc toujours au premier tour.

**La parité est réglée automatiquement** : une feuille porte deux pages et BoD refuse un
compte impair à la saisie, donc le script ajoute au besoin une page blanche en fin d'ouvrage
— sans folio — et le signale dans sa ligne de résultat. Le compte affiché est celui à
reporter, page blanche comprise.

### 5 et 6. Maquetter et exporter

Dans l'app : format **« Poche BoD — 120 × 190 mm »** ou **« Roman — 135 × 215 mm »** selon le
format visé, puis onglet Assemblage, prestataire **« BoD (crème 90 g) »**, et le nombre de
pages relevé à l'étape 4.

Le fond perdu passe à 5 mm et le dos se calcule autrement : à 280 pages, 19,5 mm chez BoD
contre 17,54 mm chez Lulu. **Les deux gabarits ne sont pas interchangeables** — une planche
maquettée pour Lulu et téléversée chez BoD serait fausse sur les deux tableaux.

### 7. Téléverser chez BoD

| Réglage | Valeur |
|---|---|
| Format | 13,5 × 21,5 cm (ou 12 × 19 cm) |
| Couverture | souple, pelliculage mat, brillant ou en relief |
| Papier | **crème 90 g** — celui sur lequel repose le calcul du dos |
| Reliure | collée |

Le parcours myBoD permet de commander pour soi sans publier ni référencer le titre.

### Gabarit BoD, pour mémoire

| Grandeur | Valeur | Source |
|---|---|---|
| Fond perdu | 5 mm | guide de maquette BoD |
| Dos | pages × 0,0675 + 0,6 mm, en crème 90 g | calculateur officiel, relevé sur 4 points |
| Épaisseur des autres papiers | blanc 90 g 0,012 · photo mat 120 g 0,0126 · photo brillant 130 g 0,0101 cm/feuille | données du calculateur |
| Marge de reliure | 20 mm, quelle que soit la pagination | modèle Word « Roman » 13,5 × 21,5 |
| Marge extérieure | 15 mm | idem |
| Marges haut / bas | 18,8 / 28 mm | idem |
| Pagination | 24 à 900 pages, **nombre pair obligatoire** | validation du calculateur |
| Export | PDF/X-3:2002 | guide de maquette |

Le détail des relevés est dans `build/in/editors/bod-specs.md`.

### Pièges propres à BoD

- **Le dos dépend du papier.** La formule implémentée vaut pour le **crème 90 g**. Sur blanc
  90 g le même livre donne un dos plus mince — 17,4 mm au lieu de 19,5 mm à 280 pages. Changer
  de papier à la commande sans refaire la couverture donnerait un dos faux.
- **Nombre de pages pair**, sans exception — le script s'en charge, mais le compte à saisir
  dans l'app est bien celui qu'il affiche, page blanche comprise.
- Le fond perdu de 5 mm est plus large que chez Lulu : une composition calée sur Lulu perd
  près de 2 mm de plus au massicot.

---

## Amazon KDP — 5 × 8, 5,5 × 8,5 ou 6 × 9 pouces

Même marche à suivre que Lulu ; seules les étapes qui changent sont détaillées. KDP a la
documentation technique la plus complète du marché : gabarits de manuscrit officiels et
formules de dos publiées. Sa contrepartie est commerciale — **imprimer oblige à publier**, et
l'épreuve privée sort filigranée.

### 0. Choisir le format, une fois pour toutes

KDP propose dix-sept formats de rognage ; trois sont outillés ici, et le format choisi doit
être **le même** dans le script et dans l'app :

| Format | Millimètres | `--provider` | Format dans l'app |
|---|---|---|---|
| 5 × 8 po | 127 × 203,2 | `kdp-5x8` | KDP 5 × 8 po |
| 5,5 × 8,5 po | 139,7 × 215,9 | `kdp-55x85` | KDP 5,5 × 8,5 po |
| 6 × 9 po | 152,4 × 228,6 | `kdp-6x9` | KDP 6 × 9 po |

Le 5,5 × 8,5 est à 5 mm près le « Roman » 135 × 215 de BoD : une maquette faite pour l'un se
transpose presque telle quelle sur l'autre. Le 5 × 8 est le plus proche d'un poche français.

### 1 à 3. Ressources, répertoire de travail, épreuve

Identiques au chapitre Lulu — le `livre.toml` ne dépend pas du prestataire.

### 4. Composer l'intérieur

```
python3 outils/gen_interieur.py build/mon-roman --provider kdp-55x85
```

Sort `build/mon-roman/out/kdp-55x85/interieur-kdp-55x85.pdf`, composé avec les marges des
modèles de manuscrit officiels : **12,7 mm en tête, en pied et à l'extérieur, 19,05 mm côté
reliure**, identiques aux trois formats.

La gouttière ne bouge qu'une fois : 19,05 mm jusqu'à 700 pages, 22,23 mm au-delà, où le
minimum imposé par KDP passe devant la valeur du modèle. Comme ailleurs, la parité est réglée
seule par une page blanche de fin sans folio.

### 5 et 6. Maquetter et exporter

Dans l'app : le **format KDP correspondant** à celui composé à l'étape 4, puis onglet
Assemblage, prestataire **« KDP (crème) »** ou **« KDP (blanc) »** selon le papier commandé, et
le nombre de pages relevé.

Le dos KDP est un simple produit, **sans le terme additif** de Lulu et de BoD : l'épaisseur de
la couverture n'entre pas dans le calcul. À 178 pages en 5,5 × 8,5, le dos fait 11,30 mm sur
crème et 10,18 mm sur blanc — plus d'un millimètre d'écart, assez pour faire mordre le texte du
dos sur les plats.

### 7. Téléverser chez KDP

| Réglage | Valeur |
|---|---|
| Format | celui choisi à l'étape 0 — pas un autre |
| Reliure | Paperback, dos carré collé |
| Encre et papier | Black & white, **crème** ou **blanc** — celui sur lequel repose le calcul du dos |
| Finition | mate, l'usage pour un roman |

Puis les deux fichiers : l'intérieur PDF, la planche PDF comme couverture.

### 8. Contrôler avant de valider

Mêmes contrôles que chez Lulu, plus un : la marge extérieure des modèles KDP est de 12,7 mm,
plus étroite que les 13 mm de Lulu et les 15 mm de BoD. L'aperçu en ligne signale toute
composition qui déborde de la zone sûre.

### Gabarit KDP, pour mémoire

| Grandeur | Valeur | Source |
|---|---|---|
| Fond perdu | 3,175 mm (0,125 po) | page d'aide « Create a Paperback Cover » |
| Dos, crème | pages × 0,0635 mm (0,0025 po) | idem — 280 p. → 17,78 mm |
| Dos, blanc | pages × 0,0572 mm (0,002252 po) | idem — 280 p. → 16,02 mm |
| Gouttière | 19,05 mm jusqu'à 700 p., puis 22,23 mm | modèles Word officiels et tableau des minimums |
| Marges haut / bas / extérieur | 12,7 mm | modèles Word officiels |
| Pagination | 24 à 828 pages | options d'impression |
| Texte sur le dos | à partir de 80 pages | page d'aide couverture |

Le détail des relevés est dans `build/in/editors/kdp-specs.md`, les modèles dans
`kdp-paperback-manuscript-blank-templates.zip` du même répertoire.

### Pièges propres à KDP

- **Le papier est définitif après publication** : il détermine l'ISBN de fabrication. Changer
  de crème à blanc impose un nouveau livre — et une couverture au dos refait.
- **En deçà de 80 pages, KDP n'imprime pas le texte du dos.** L'app l'affiche quand même : la
  planche est juste, l'imprimeur ignorera simplement ce qui s'y trouve.
- **La justification est longue sur les grands formats.** Les modèles gardent 12,7 mm de marge
  extérieure quel que soit le format : en 6 × 9, la colonne fait 120,6 mm, soit environ
  90 signes par ligne au corps 9,5 pt de l'atelier, contre 53 en poche Lulu. Le gabarit suit les
  modèles officiels ; élargir les marges ou grossir le corps se décide sur épreuve.
- **Imprimer oblige à publier.** Pas d'équivalent du parcours myBoD : les exemplaires auteur
  exigent un livre publié, et l'épreuve privée arrive filigranée.

---

## Ajouter un prestataire

Il faut compléter **deux** tables, l'une pour la couverture, l'autre pour l'intérieur :

- `PROVIDERS` dans `index.html` — fond perdu et formule du dos, ce qui suffit à la planche ;
- `PROVIDERS` dans `outils/gen_interieur.py` — format, marges, gouttières par tranche de pagination, réglages typographiques.

Ne reporter que des tranches de gouttière effectivement lues dans le guide du prestataire : le script préfère refuser une pagination hors tranche plutôt que d'extrapoler. Ranger le guide dans `build/in/editors/` pour la prochaine fois.

### File d'attente

Retenus depuis le comparatif POD du 19 août 2026 (`build/in/editors/comparator-pod-livres-*.html`), par ordre de traitement :

| Rang | Prestataire | Pourquoi lui | Réserve connue |
|---|---|---|---|
| ~~1~~ | ~~BoD~~ — **implémenté**, voir son chapitre | | |
| ~~2~~ | ~~Amazon KDP~~ — **implémenté**, voir son chapitre | | |
| 3 | TheBookEdition (Lille) | Meilleure fabrication du banc d'essai, contrôle manuel des fichiers, production française. | Le plus cher des trois grands, contrôles d'upload stricts, broché seul. |
| 4 | CoolLibri (Toulouse) | Français, papiers crème et satin à l'unité, jusqu'à 648 pages. | Documentation technique à vérifier, pas de distribution. |
| 5 | Bookvault (UK) | Finitions premium dès un exemplaire, papier intérieur 150 g. | Frais d'upload de 10 à 15 $, incertitudes post-Brexit. |

Lulu reste implémenté, mais le comparatif le classe en tier B : papier fin, rainage mou, coût à l'exemplaire le plus élevé des grands POD. Son intérêt tient à l'étendue de son catalogue de reliures.
