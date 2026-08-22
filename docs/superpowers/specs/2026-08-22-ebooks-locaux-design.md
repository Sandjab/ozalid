# Les ebooks locaux

Date : 2026-08-22
Statut : validé (brainstorming)

## Objectif

La chaîne sait produire des packages prestataires : un intérieur composé, une planche
au gabarit, un dos qui découle de la pagination. Elle ne sait rien produire qui se lise
sur un écran.

Cette spec ajoute une **livraison locale** : le livre entier — couverture comprise —
en deux fichiers qu'on ouvre, qu'on relit, qu'on envoie à un lecteur. Un PDF et un
EPUB, générés d'un geste, écrits à côté du projet.

Ce n'est pas une sixième étape de la fabrication : les cinq onglets racontent l'ordre
où le livre se fait, et l'ebook est une sortie de plus à la fin. Il vit donc à la
Livraison, comme l'épreuve vit à l'Intérieur.

## Décisions de cadrage (brainstorming du 22/08)

- **Pas de mobi.** Voir § 1 : le format est mort et sa production coûterait soit une
  dépendance système non versionnée, soit un producteur PalmDB/KF8 écrit à la main.
- **Le PDF est le livre sans son imposition** : même format, même police, mêmes
  liminaires, mais marges symétriques et pas de blanche de parité.
- **Le gabarit vient du destinataire pointé**, celui du pied de fenêtre. Rien à
  saisir, rien à ajouter à la table des prestataires.
- **L'EPUB embarque la police de l'intérieur.** Elle est OFL, donc redistribuable.
- **Un bloc à l'étape Livraison**, avec son bouton et son compte rendu à côté.
- **Usage visé** : relecture personnelle et diffusion directe aux lecteurs. Aucun
  dépôt sur plateforme, donc pas d'ISBN ni de passage obligé par un validateur.

## 1. Pourquoi pas de mobi

Amazon a retiré le MOBI en août 2022 : Send-to-Kindle et KDP le **refusent**
aujourd'hui, et acceptent l'EPUB directement. Un `.mobi` produit maintenant ne
s'ouvrirait nulle part où l'EPUB ne s'ouvre pas déjà.

Le produire supposerait, au choix :

- `kindlegen`, retiré de la distribution par Amazon — indisponible ;
- `ebook-convert` de Calibre, une dépendance système lourde, à installer sur chaque
  poste, non versionnée, dont la version changerait le rendu d'une machine à l'autre —
  exactement ce que l'embarquement du sidecar Typst existe pour empêcher ;
- un producteur PalmDB/KF8 écrit dans le projet, chantier hors de proportion avec
  l'usage.

Aucune des trois ne se justifie. La décision est prise, pas reportée : rien dans le
design ci-dessous ne réserve de place pour un troisième format.

## 2. Ce qu'on écrit sur le disque

Les sorties d'un projet vivent à côté de l'archive, jamais dedans. Les ebooks suivent
la règle et prennent leur propre répertoire, frère de ceux des prestataires :

```
<projet>/ebook/<titre-assaini>.pdf
<projet>/ebook/<titre-assaini>.epub
```

`<titre-assaini>` passe par `envoi::assaini`, déjà en place pour nommer les répertoires
d'envoi : c'est la fonction du projet qui décide ce qu'un nom d'auteur ou de titre
devient sur un disque, et il n'y en aura pas deux.

Un projet non enregistré ne peut donc pas générer d'ebook, faute d'endroit où écrire —
même règle, et même message, que les packages.

## 3. Le PDF

### Ce qui change par rapport à l'impression

`interieur::source` prend déjà tous ses réglages en paramètre. L'ebook l'appelle avec
deux valeurs différentes, et c'est tout :

| | Impression | Ebook |
|---|---|---|
| `gouttiere` | issue de `interieur::converge` | `pr.exterieur` — marges symétriques |
| `blanche` | issue de `interieur::converge` | `false` |
| pagination | mesurée, elle donne le dos | inutile : aucun appel à `typst.pages` |

La gouttière et la blanche de parité n'ont de sens qu'une fois le livre relié. À
l'écran, l'une décale le texte une page sur deux et l'autre ajoute une page vide : les
deux sont de l'imposition, pas du livre.

Les **liminaires restent intacts** — faux-titre, sa blanche, page de titre, copyright,
dédicace et sa blanche. Ce sont des conventions du livre, pas de l'imposition, et les
retirer donnerait un autre livre.

L'ebook ne mesure pas sa pagination : il n'a pas de dos à calculer. La génération est
donc une seule compilation, là où un package en fait plusieurs.

### La couverture en page 1

`couverture::source_une` produit une source Typst **autonome** : un préambule qui pose
la page, puis le corps. Ce préambule contient des `#set` de document — `text`, `par` —
qui, insérés tels quels dans l'intérieur, écraseraient les siens pour tout ce qui
suit.

`couverture` expose donc une seconde forme de la même face :

```rust
/// La 1ère, sur une page insérée dans un autre document.
///
/// Même corps que `source_une`, mais les réglages de texte et de paragraphe y sont
/// portés par le bloc de la page, non par le document : l'intérieur qui suit garde
/// les siens.
pub fn page_une(
    livre: &Livre,
    cv: &Couverture,
    format: (f64, f64),
    image: Option<&Ressource>,
    dos_mm: Option<f64>,
) -> String
```

Elle rend un `#page(width: …, height: …, margin: 0mm, footer: none)[…]` portant les
mêmes `#set` en portée locale et le même `corps_une` sur une `Boite::rognee` — pas de
fond perdu : un ebook ne se coupe pas.

`dos_mm` vient du destinataire pointé, comme pour l'aperçu à l'écran. Il ne sert qu'au
cadrage panoramique ; absent, l'image se cadre sur la seule 1ère, ce que fait déjà
l'aperçu aujourd'hui. Aucun refus nouveau ici.

### La retouche de `interieur.rs`

Le corps actuel de `source` devient une fonction interne :

```rust
fn assemble(…, avant: Option<&str>) -> String
```

où `avant` est un fragment Typst inséré juste après le préambule, avant les
liminaires. `source` en reste un appel avec `None` — **aucun des huit appelants
existants ne bouge** — et une entrée nouvelle sert l'ebook :

```rust
/// L'intérieur du livre précédé de sa couverture, sans imposition.
pub fn source_ebook(
    livre: &Livre,
    int: &Interieur,
    pr: &Provider,
    chapitres: &[Chapitre],
    couverture: &str,
) -> String
```

Elle pose elle-même `Reglage { gouttiere: pr.exterieur, blanche: false }` : ce n'est
pas un réglage qu'on offre, c'est ce que veut dire « sans imposition ».

## 4. L'EPUB

### Deux modules

Ils sont séparés parce qu'ils ne dépendent pas des mêmes choses :

| Module | Rôle | Dépend de |
|---|---|---|
| `epub` | Chapitres + PNG de couverture + fichiers de police → une archive EPUB 3 | rien : ni Typst, ni prestataire, ni disque |
| `ebook` | Orchestre : source PDF, rendu PNG de la 1ère, choix des polices, écriture | Typst, `projet`, `epub` |

`ebook` est aux sorties locales ce que `package` est aux prestataires : il traverse la
chaîne, il ne compose rien lui-même.

`epub` ne touche pas au disque : il reçoit des octets et rend des octets. C'est ce qui
le rend entièrement éprouvable sans Typst, sans `fonts/` et sans répertoire temporaire.

### Contenu de l'archive

```
mimetype                    STORED, première entrée — la spec de l'EPUB l'exige
META-INF/container.xml
OEBPS/content.opf           métadonnées, manifeste, spine
OEBPS/nav.xhtml             table des matières EPUB 3
OEBPS/toc.ncx               la même, pour les liseuses antérieures à EPUB 3
OEBPS/style.css             @font-face, justification, alinéa, rupture de scène
OEBPS/couverture.xhtml
OEBPS/liminaires.xhtml      page de titre, copyright, dédicace
OEBPS/ch001.xhtml …         un fichier par chapitre
OEBPS/images/couverture.png
OEBPS/fonts/…               le romain et l'italique de la police de l'intérieur
```

Le faux-titre et les blanches ne passent pas : ils n'ont de sens que sur du papier
plié. La page de titre, le copyright et la dédicace, eux, sont du livre.

Les métadonnées portent le titre, l'auteur, `dc:language` à `fr`, un identifiant
`urn:ozalid:<titre-assaini>-<auteur-assaini>` — stable pour un livre donné, sans
dépendance `uuid` — et le `dcterms:modified` qu'EPUB 3 rend obligatoire, formaté
depuis `SystemTime`. C'est la seule chose du fichier qui change d'une génération à
l'autre.

### Du manuscrit au XHTML

| Source | EPUB |
|---|---|
| `Chapitre { numero, titre }` | `<h1>` avec le numéro, `<h2>` avec le titre quand il existe |
| `Bloc::Paragraphe` | `<p>` |
| `Bloc::Scene` | `<p class="scene">` portant trois astérisques séparées d'espaces insécables |
| `*emph*` | `<em>` |
| `**strong**` | `<strong>` |

La rupture de scène est **trois astérisques ordinaires espacées**, le même caractère
que sur le papier : `manuscrit::SCENE` l'a choisi parce qu'il est le seul présent dans
tous les fichiers de `fonts/`. Ce qu'on relit doit être ce qui s'imprime, et l'écran ne
fait pas exception. La constante elle-même n'est pas réutilisable — c'est du markup
Typst, `\*#h(0.8em)\*#h(0.8em)\*` — donc l'EPUB écrit les trois astérisques séparées
d'espaces insécables, et un test amarre les deux formes l'une à l'autre.

L'échappement XML est **propre à `epub`** : il protège `<`, `>`, `&`, `"` et `'`.
`manuscrit::echappe` protège le markup Typst, ce qui n'a rien à voir — les deux ne
doivent jamais être confondus, et un test le dit.

### Les polices

L'utilisateur a choisi de les embarquer : le livre garde son œil sur liseuse. Les sept
familles de `POLICES_TEXTE` sont OFL, donc redistribuables.

La règle de choix, sur les fichiers du répertoire de polices :

1. ne retenir que ceux dont `police::examine` rend la famille de `Interieur::police` ;
2. **écarter tout nom contenant « Bold »** — cela couvre `-Bold`, `-BoldItalic`,
   `-SemiBold` et `-SemiBoldItalic` d'un seul coup ;
3. le **romain** est celui qui reste et ne contient pas « Italic » ;
4. l'**italique** est celui qui reste et contient « Italic ».

L'exclusion de « Bold » n'est pas un raffinement : sans elle, la règle prendrait
`Cardo-Bold.ttf` pour romain de Cardo, dont le fichier ordinaire s'appelle
`Cardo-Regular.ttf`. À égalité improbable, le nom le plus court l'emporte.

Deux `@font-face` sont déclarés, avec `font-weight: 100 900` sur les fichiers
variables — l'axe sert alors le gras réel ; sur les fichiers statiques (Cardo,
Spectral), le lecteur le synthétise, ce qui est le comportement d'un EPUB ordinaire.

**Une police introuvable ne fait pas échouer la génération.** L'EPUB se fait en
`serif`, et le compte rendu le dit — comme `polices_introuvables` le fait déjà pour
Typst. Le livre reste juste ; seul son œil change, et cela se voit.

### La couverture

Rendue par `typst.apercu` depuis `couverture::source_une`, à **250 ppp** : sur une
hauteur de 170 mm cela donne environ 1670 px, au-dessus du seuil où Kindle et Kobo
cessent de recadrer. Le PNG entre dans l'archive en STORED — il est déjà compressé.

Elle est déclarée deux fois dans l'OPF : `properties="cover-image"` sur l'image pour
EPUB 3, et `<meta name="cover">` pour les liseuses anciennes. Les deux sont
nécessaires pour qu'une vignette s'affiche partout.

## 5. La commande et l'écran

Une commande, `ebook_generer`, qui rend :

```rust
#[derive(Serialize)]
pub struct Ebooks {
    pub pdf: String,
    pub epub: String,
    pub octets_pdf: u64,
    pub octets_epub: u64,
    /// Familles que Typst a composées par repli. Vide, tout va bien.
    pub polices_introuvables: Vec<String>,
    /// Renseigné quand la police de l'intérieur n'a pas été trouvée dans `fonts/` :
    /// l'EPUB est alors en écriture du lecteur.
    pub police_non_embarquee: Option<String>,
}
```

À l'écran, un bloc « Ebooks » à la fin de l'étape Livraison, bâti comme le bloc
« Épreuve » de l'étape Intérieur : un bouton, un témoin d'état, un compte rendu à côté
du bouton. Un travail long rend compte là où on a cliqué.

## 6. Les refus

Tous existent déjà ; aucun n'est inventé pour l'occasion.

| Cause | Message |
|---|---|
| Aucune maquette de couverture | celui de `package::assembler` |
| Projet non enregistré | celui des packages : pas d'endroit où écrire |
| Police d'intérieur hors liste | `Interieur::verifie` |
| Manuscrit non composable | `manuscrit::decoupe`, avec son numéro de ligne |

Et un seul avertissement, qui n'arrête rien : la police absente de `fonts/`.

## 7. Vérification

### Ce que les tests doivent tenir

Chacun **vu rouge d'abord** — TDD ou mutation ciblée. Un test qui n'a jamais échoué ne
protège rien.

- L'échappement XML protège `<`, `&` et `"` dans un titre de chapitre **et** dans un
  paragraphe. Un titre contenant `&` ne doit pas casser l'archive.
- L'emphase, le gras, la rupture de scène et le chapitre sans titre produisent le
  XHTML attendu.
- Le manifeste OPF liste exactement les entrées de l'archive : ni fichier manifesté
  absent, ni fichier présent non manifesté. C'est le défaut qui fait rejeter un EPUB
  par un lecteur strict sans qu'aucun autre test ne le voie.
- Le `spine` ne renvoie qu'à des `id` du manifeste, et dans l'ordre des chapitres.
- `mimetype` est la première entrée de l'archive et n'est pas compressée.
- La règle de choix des polices, appliquée aux **32 noms de fichiers relevés dans
  `fonts/`** : chacune des sept familles doit donner un romain et un italique, et
  aucune ne doit tomber sur `-BoldItalic`. Fonction pure sur une liste de noms, donc
  éprouvable alors que `fonts/` n'est pas versionné.
- `source_ebook` pose bien une gouttière égale à la marge extérieure et aucune blanche
  finale.

### Le témoin

`cargo run --example temoin` reste le garde-fou de la retouche de `interieur.rs` :
l'extraction de `assemble` ne doit pas déplacer d'une page la pagination de *Candide*.
C'est la vérification qui décide si le refactor est neutre.

### L'exercice sur livre réel

```
cd app/src-tauri
cargo run --example ebook -- <projet.ozalid> <sortie>
```

### À l'œil

Ce qu'aucun test ne peut faire, et qu'il faut refaire après toute modification :

- ouvrir l'`.epub` dans **Apple Livres**, **Calibre** et sur une **liseuse** ;
- la vignette de couverture s'affiche dans la bibliothèque ;
- la table des matières est navigable et mène au bon chapitre ;
- les italiques du manuscrit sont là ;
- le texte est dans la police du livre, pas dans celle du lecteur ;
- ouvrir le PDF sur une tablette : les marges sont symétriques, aucune page vide, la
  couverture est la page 1.

## 8. Les lots

1. **`epub`, sans rien autour.** Le module pur : chapitres et octets en entrée, archive
   en sortie. Tous les tests de la § 7 sauf ceux de `source_ebook`. Éprouvé par un test
   qui écrit une archive en mémoire et la relit.
2. **La couverture insérable.** `couverture::page_une`, et `interieur::assemble` /
   `source_ebook`. Le témoin passe : c'est ce qui clôt le lot.
3. **`ebook`, l'orchestration.** Rendu du PNG, choix des polices, écriture des deux
   fichiers, et l'exemple `ebook`. Premier ebook sur un livre réel, regardé.
4. **L'écran.** Commande `ebook_generer`, bloc à la Livraison, compte rendu.

## 9. Hors périmètre

- **Le mobi**, et toute conversion vers un format Amazon. Voir § 1.
- **Les signets PDF.** Les chapitres ne sont pas des `#heading` dans l'intérieur ; en
  faire des `#heading` avec une règle d'affichage identique demanderait de reprouver la
  pagination au témoin. Le gain — une table des matières dans le lecteur PDF — ne le
  vaut pas tant que l'EPUB en porte une.
- **`epubcheck`.** On ne dépose sur aucune plateforme. L'archive vise la conformité
  EPUB 3 et les tests en tiennent les points structurels, mais rien ne la valide
  automatiquement. Le jour où un dépôt serait visé, ce serait le premier ajout.
- **Un ebook par dédicataire.** L'envoi autographe est une affaire de tirage papier ;
  rien ici ne l'interdit plus tard, `interieur::source` prenant déjà sa `Trace`.
- **Un format d'ebook réglable.** Le gabarit vient du destinataire pointé. Un réglage
  de plus pour un fichier que le lecteur redimensionne de toute façon ne se justifie
  pas.
