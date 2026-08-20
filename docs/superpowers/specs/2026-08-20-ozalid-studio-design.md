# Ozalid Studio — app de bureau macOS + Windows, chaîne complète

Date : 2026-08-20
Statut : validé (brainstorming)

## Objectif

Faire tenir dans une seule application de bureau, macOS et Windows, la chaîne
entière qui va du manuscrit aux packages prestataires : composition de
l'intérieur, maquette de couverture, assemblage de la planche, génération des
packages. Un wizard mène le livre de bout en bout ; le ou les prestataires ne
se choisissent qu'à la dernière étape.

Le problème résolu n'est pas le confort. Aujourd'hui `outils/gen_interieur.py`
connaît la pagination, `index.html` connaît le gabarit prestataire, et rien ne
relie les deux sauf la mémoire de l'utilisateur — la spec du 18/08 assumait
explicitement ce prix (« nombre de pages saisi à la main : prix assumé du choix
*couverture seule* »). Or la pagination change à chaque retouche du manuscrit :
une planche exportée hier peut être fausse aujourd'hui sans qu'aucun garde-fou
ne le signale. L'app supprime le couplage manuel par construction.

## Décisions de cadrage

- **Stack : Tauri 2 + Rust, front vanilla sans bundler**, releases GitHub CI.
  La stack de `../superpopaul` transposée sans invention.
- **Composition par Typst**, binaire statique embarqué en sidecar (une cible par
  plateforme). Ni Python, ni pandoc, ni WeasyPrint : les dépendances natives de
  WeasyPrint (Pango/GTK) ne s'empaquettent pas proprement sous Windows, et
  Windows n'est testable que via les releases CI.
- **Typst compose aussi la couverture, aperçu compris.** L'aperçu affiché est le
  rendu Typst rasterisé : WYSIWYG strict, texte vectoriel dans le PDF, et rendu
  identique sur les deux OS. Une webview Tauri est WKWebView sur macOS et
  WebView2 sur Windows : conserver `html2canvas` aurait fait dépendre la planche
  de la machine qui l'exporte.
- **`index.html` et `outils/` sont gelés.** Ils continuent de servir en l'état,
  ils n'évoluent plus. Aucune parité de rendu n'est exigée entre l'ancien moteur
  CSS et le nouveau moteur Typst : la divergence est assumée et datée. Toute
  évolution ultérieure va dans l'app.
- **Un livre est un fichier `.ozalid`**, archive auto-portante qu'on ouvre,
  déplace et sauvegarde comme un document. L'arborescence `build/` n'est plus la
  structure de référence de l'app.
- **Le manuscrit est embarqué dans l'archive**, avec ré-import explicite. Le
  chemin de la dernière source est mémorisé : « Réimporter le manuscrit » est un
  bouton, pas une navigation dans un sélecteur de fichiers.
- **Français** dans l'interface, les commentaires et les commits, comme le reste
  du projet.

## 1. Emplacement et modules

L'app vit dans le repo `ozalid`, répertoire `app/`, aux côtés d'`index.html` et
d'`outils/` gelés. Modules Rust étanches, testables sans UI ; l'interface
n'a aucune logique métier, elle invoque des commandes et affiche des événements.

| Module | Rôle |
|---|---|
| `projet` | Lecture/écriture du `.ozalid`, import depuis `livre.toml` + PNG |
| `manuscrit` | Markdown → Typst, contrôle d'intégrité |
| `providers` | Table unique des gabarits prestataires |
| `typst` | Invocation du sidecar, compile PDF et PNG d'aperçu, remontée d'erreurs |
| `interieur` | Composition, compte de pages, seconde passe, correction de parité |
| `couverture` | Génération de la source Typst de la planche |
| `epreuve` | PDF A4 de relecture |
| `package` | Assemblage des packages prestataires sur disque |

Pandoc disparaît sans second binaire pour le remplacer : la conversion
Markdown → Typst se fait en Rust (`pulldown-cmark`). Le format de manuscrit est
contraint — titre en `#`, chapitres en `## NN - Titre`, séparateurs de scène
`---` — ce qui rend le contrôle d'intégrité (compte de chapitres) testable
unitairement.

### `providers` — la table unique

Le projet maintient aujourd'hui deux tables décrivant les mêmes prestataires
sans jamais se recouper : le `PROVIDERS` d'`index.html` pour la couverture,
celui de `gen_interieur.py` pour l'intérieur. L'app n'en a qu'une : format,
marges, gouttières par tranche de pagination, formule de dos, fond perdu. Un
prestataire s'ajoute à un seul endroit.

Le compte de pages ne transite plus par un humain : `interieur` le produit,
`couverture` le consomme.

## 2. Le fichier `.ozalid`

Archive zip, extension `.ozalid` :

```
projet.toml     identité du livre + réglages de couverture + chemin source du manuscrit
manuscrit.md
images/         1ère, 4ème
```

`projet.toml` garde la forme et l'esprit de `livre.toml` — titre, auteur, genre,
copyright, compte de chapitres pour le contrôle d'intégrité — augmenté des
réglages de couverture. Dézippée, l'archive reste lisible et diffable.

Un `.ozalid` déplacé sur une autre machine reste complet : c'est la raison du
choix « manuscrit embarqué ».

**Les sorties ne sont pas dans l'archive.** Un `.ozalid` ne contient que les
sources ; les packages sont écrits à côté du fichier, dans un répertoire
`<nom-du-livre>/<éditeur>/`. L'archive reste légère, versionnable, et une
sortie périmée ne survit jamais à un déplacement du projet.

## 3. Le wizard

1. **Livre** — nouveau, ouvrir, ou importer un livre existant
2. **Manuscrit** — import, contrôle d'intégrité, structure détectée
3. **Intérieur** — composition sur un format de référence, pour lire et juger.
   Le compte de pages s'affiche ici, **à titre indicatif**
4. **Couverture** — 1ère et 4ème, aperçu Typst
5. **Assemblage** — planche complète, dos calculé, jamais saisi
6. **Prestataires** — cocher les prestataires voulus

### Pourquoi le prestataire peut se choisir en dernier

Le prestataire impose le format de l'intérieur (Lulu 108 × 175, CoolLibri
110 × 170, KDP 6 × 9…), donc en apparence il faut le connaître avant de
composer. La contradiction se lève parce que tous les réglages de couverture
sont **en pourcentage de la largeur** : la maquette est déjà indépendante du
format.

À l'étape 6, chaque prestataire coché déclenche sa propre composition : son
format, sa gouttière, sa pagination, donc son dos et sa planche. Un livre,
N packages, aucun réglage retouché. C'est la « file d'attente » du COOKBOOK,
exécutée.

Le compte de pages de l'étape 3 est indicatif parce qu'il dépend du format de
référence choisi pour la relecture, pas du format final. Ce format de référence
se choisit explicitement à l'étape 3 dans la liste des formats connus de
`providers`, et il est mémorisé dans `projet.toml`. L'interface le nomme comme
tel : c'est un format de travail, pas un engagement sur le prestataire.

## 4. Le moteur couverture en Typst

À reproduire : trois modes de 1ère (Bandeau, Surimpression, Sans image), le
générateur de cadre à six axes, quatre fonds de 4ème dont le prolongement
panoramique, et les trois maquettes préchargées (Folio, Blanche, Surimpression).

Les primitives existent : `place` et `clip` pour le panoramique, `rect` à filets
pour le cadre, et les pourcentages sont natifs en Typst — la règle « tout en %
de la largeur » se transpose littéralement.

Aperçu : `typst compile --format png` à basse résolution, affiché dans la
webview, avec le même debounce que le `render()` actuel. L'aperçu et le PDF
final sortent de la même source, donc l'écart écran/export disparaît.

## 5. Risques identifiés

- **La pagination va changer.** Moteur de justification différent : le même
  manuscrit ne fera pas le même nombre de pages qu'avec pandoc + WeasyPrint. Le
  témoin de non-régression actuel (le compte de pages sur un manuscrit donné)
  repart de zéro. Le critère de validation n'est pas « le même compte qu'avant »
  mais « marges et gouttières conformes au guide du prestataire ».
- **La césure française.** C'est ce qui a le plus de chances de mal rendre dans
  un roman. À vérifier tôt sur un manuscrit réel, pas sur un texte de
  remplissage.
- **Les polices.** Les Google Fonts utilisées par `index.html` doivent être
  embarquées comme fichiers dans l'app. Licences à vérifier une par une.
- **La fidélité du cadre.** Le triple filet et ses six axes doivent retomber au
  même endroit. Aucun test automatique ne peut l'attester : comparaison visuelle
  contre les exports actuels, à l'œil.
- **Deux moteurs de couverture coexistent dans le repo** (CSS gelé, Typst
  vivant). Le gel d'`index.html` est ce qui évite d'avoir à tenir leur parité ;
  s'en écarter réintroduirait un coût permanent qu'aucun test ne couvre.

## 6. Tests

- `cargo test` dans `app/src-tauri/` : `providers` (formules de dos, tranches de
  gouttières), `manuscrit` (parsing, contrôle d'intégrité), `projet` (round-trip
  `.ozalid`, import `livre.toml` + PNG), `interieur` (choix de tranche, seconde
  passe, correction de parité).
- `node --test` côté front pour le câblage de l'UI, avec un faux DOM exécutant
  le vrai code, sur le modèle de superpopaul.
- **Le test qui porte la raison d'être du projet** : le dos affiché correspond
  toujours au PDF intérieur effectivement produit. Rallonger le manuscrit doit
  faire bouger le dos sans intervention. Un test incapable d'échouer quand ce
  câblage casse ne vaut rien.
- Procédure manuelle assumée, non automatisable : comparaison visuelle de la
  planche avant release.

## 7. Ordre de construction

| Jalon | Contenu | Ce qu'on sait à la fin |
|---|---|---|
| **0** | Spike Typst à la main : maquette Folio + intérieur Lulu, comparés aux sorties actuelles | Si le projet est faisable |
| **1** | Tauri + `providers` + `manuscrit` + `interieur` | Un intérieur composé, un compte de pages affiché |
| **2** | `.ozalid` + import d'un livre existant | Les livres publiés servent de matériel de test réel |
| **3** | Moteur couverture Typst complet | Le gros morceau est derrière |
| **4** | Assemblage + packages multi-prestataires | La chaîne entière tourne |
| **5** | Épreuve de lecture + release Windows CI | Windows validé |

Le jalon 0 précède toute ligne de Rust : si Typst ne sait pas faire le cadre, ou
si la césure est mauvaise, cela se découvre en une heure et non en trois
semaines.

L'épreuve de lecture arrive en dernier parce que c'est la seule pièce dont
l'absence ne bloque rien.
