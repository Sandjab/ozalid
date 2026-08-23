# Ozalid Studio

Application de bureau macOS + Windows qui tient la chaîne entière : manuscrit →
intérieur composé → couverture → packages prestataires. Elle succède à la paire
`index.html` + `outils/`, désormais gelée (voir
`docs/superpowers/specs/2026-08-20-ozalid-studio-design.md`).

Ce qu'elle règle : le nombre de pages ne transite plus par un humain. L'intérieur
le produit, la couverture le consomme, et le dos suit le manuscrit sans ressaisie.

**État : jalon 5** — projet `.ozalid`, import d'un livre existant, composition de
l'intérieur, moteur de couverture, assemblage de la planche, packages
multi-prestataires, ebooks locaux en PDF et en EPUB, épreuve de relecture, cycle
de vie du document — créer, enregistrer, enregistrer sous, fermer, avec une garde
avant tout ce qui perdrait du travail —, menu natif et ses raccourcis, écran
d'accueil et projets récents, et vérification Windows par intégration continue :
chaque push et chaque pull request compilent, testent et paginent le témoin sur
`windows-latest`, et un tag `v*` produit l'installeur, l'installe en silencieux
pour vérifier son arborescence, et le dépose en release draft. Reste la
vérification manuelle du premier lancement sur une machine Windows — aucun runner
ne lance l'application avec sa fenêtre.

## Stack

- **Tauri 2 + Rust** pour le client, front vanilla sans bundler ni framework.
- **Typst** en sidecar : un binaire statique, sans dépendance système, la même
  version sur les deux plateformes. C'est ce qui rend la pagination reproductible
  d'une machine à l'autre — ni Python, ni pandoc, ni WeasyPrint.
- **`ureq`**, seule dépendance réseau, tirée par le seul envoi généré par diffusion.
  Rien d'autre dans l'application n'ouvre de connexion, et composer n'en ouvre
  jamais : une image acceptée est figée dans le `.ozalid`.

## Mise en route

```
app/outils/typst.sh --local     # ou sans --local pour télécharger la version épinglée
app/outils/polices.sh           # ~10 Mo de polices OFL
cd app/src-tauri && cargo tauri dev
```

`typst.sh` place le sidecar dans `src-tauri/binaries/`, `polices.sh` les polices
dans `src-tauri/fonts/` — deux répertoires non versionnés. La version de Typst est
**épinglée** : deux versions ne composent pas forcément le même nombre de pages,
donc pas le même dos. La relever est un changement délibéré, à revalider sur un
manuscrit réel.

Typst est lancé avec `--ignore-system-fonts` : seules les polices embarquées
comptent, sans quoi une police du poste pourrait s'y substituer et le rendu
dépendrait de la machine.

## Installer sous Windows

L'installeur (`.exe` NSIS) vient de la publication déclenchée par un tag `v*` : le job
`publier` le construit, l'installe en silencieux pour vérifier qu'il pose bien ses
fichiers, puis le dépose en **release draft** sur GitHub — c'est un humain qui publie,
après avoir lancé l'application au moins une fois.

Au premier lancement, Windows affiche « Windows a protégé votre PC » : SmartScreen ne
reconnaît pas l'éditeur tant que le binaire n'est pas signé. Il faut choisir
« Informations complémentaires », puis « Exécuter quand même ». L'installation elle-même
ne demande aucun droit administrateur : elle se fait par utilisateur, dans
`%LOCALAPPDATA%\Ozalid Studio`, où l'application trouve `typst.exe` à côté d'elle et ses
polices dans `fonts\`. Un certificat de signature de code lèverait l'avertissement ; il
n'a pas été pris tant que la diffusion reste confidentielle.

## L'écran

Quatre bandes, et la fenêtre elle-même ne défile plus : une **entête** qui nomme le
livre ouvert, son chemin et son état d'enregistrement ; une rangée de cinq
**onglets** — Livre, Intérieur, Couverture, Livraison, Envois — dans l'ordre où le
livre se fait ; l'**étape** courante, seule ; un **pied** où l'on choisit pour qui
l'on regarde, et qui dit ce que vaut le dos.

Les deux bandes du haut tiennent chacune sur **une ligne** : le chemin du `.ozalid` à
côté du titre, tronqué s'il le faut et entier au survol ; l'état d'une étape à côté de
son nom. Empilées, elles prenaient 176 px à toutes les étapes pour dire six choses
courtes — et c'était la Couverture qui les payait, en aperçu. Les Envois ont leur étape : la main du
livre, l'accès au modèle et la liste des dédicataires débordaient la Livraison de
quatre défilements — elle ne garde que les destinataires et leurs packages. Ce qui
ne tient pas se règle par la mise en page ; le panneau de réglages de la couverture
garde son propre ascenseur — sa longueur est irréductible. Une étape qui déborde
tombe, elle, dans le filet de la bande de contenu : c'est le cas de la Livraison dès
le deuxième compte rendu de génération, et la barre qui paraît alors est un défaut
de mise en page, pas un ascenseur qu'on offre.

Les cinq onglets se traversent aux flèches, et une seule tabulation suffit à sortir
de la bande : c'est le pattern `tablist`, tenu en entier.

La coquille est en **gris chauds**, blanc pour les surfaces de travail — les champs,
les comptes rendus, l'étape ouverte. Le rouge n'y sert qu'à l'alerte. L'écran est
d'un outil qui s'efface : sur un fond crème, un blanc paraît bleu et un beige paraît
neutre, et une couverture ne s'y juge pas. Elle est ici le seul objet coloré.

Chaque onglet porte un sous-libellé qui énonce où en est son étape — le nombre de
chapitres, la police, la maquette — et un témoin rouge quand elle réclame :
manuscrit qui ne correspond plus au contrôle d'intégrité (Livre), dos que la
dernière composition ne vaut plus (Intérieur, parce que c'est là qu'on le
recompose), couverture sans maquette (Couverture). Trois témoins et pas un de plus :
un manuscrit absent est un état, pas une anomalie.

Sans projet ouvert, les onglets sont inertes et un **accueil** prend la place de
l'étape : Nouveau projet, Ouvrir un `.ozalid`, Importer un `livre.toml`, et les
projets récents. L'accueil est un état de l'application, pas un écran de plus posé
devant les autres.

Ce qui **refuse une saisie** monte à l'entête, la seule bande que toutes les étapes
partagent : le geste est fini, et le message doit survivre au changement d'étape.
Ce qui rend compte d'un **travail long** — composer, tirer une épreuve, générer les
packages — reste à côté du bouton qui l'a lancé : on attend là où l'on a cliqué, et
un compte rendu qui migre en haut de l'écran se lit comme une panne. L'aperçu de
couverture n'est ni l'un ni l'autre : **personne ne l'a demandé**, il se recompose à
chaque réglage, et ce qu'il dit de lui-même se lit sous l'image comme une légende.

**Enregistrer n'est plus qu'un geste de menu** (⌘S, ⇧⌘S) : les deux boutons ont
quitté l'écran, comme dans tout éditeur de document macOS. Une entrée de menu termine
d'abord la saisie en cours : un champ que le clavier tient encore n'a rien envoyé — le
menu natif ne lui prend pas le focus — et ⌘S enregistrait sans lui, puis le remettait à
son ancienne valeur. Le sous-menu **« Aller »**
navigue entre les cinq étapes (⌘1 à ⌘5) ; sans projet ouvert, il ne mène nulle
part sans rien casser — la garde est du côté que les onglets et le menu ont en
commun.

## Le prestataire, choisi une seule fois

Un livre a des **destinataires** : les prestataires chez qui on le livre. Ils se
déclarent à l'étape Livraison — leur papier, et pour ceux qui ne publient ni dos ni
fond perdu, ce qu'on a relevé sur leur gabarit — et **nulle part ailleurs**. Le pied
de fenêtre porte le **pointeur** dessus : le destinataire visé, celui pour qui
l'étape 2 compose et à quel format l'étape 3 rend ses aperçus. L'étape 4, elle,
génère pour toute la liste.

Un prestataire courant est nécessaire même pour regarder une première de couverture,
qui ne réclame aucune composition mais réclame un format : un projet neuf naît donc
avec un destinataire, le premier de la table, et le dernier ne se retire pas.

Les relevés naissent **vides**, jamais préremplis. Un chiffre par défaut se lirait
comme une mesure ; à sa place, la génération refuse en disant quoi faire — « CoolLibri
ne publie pas de formule de dos : relever l'épaisseur sur son gabarit à 184 pages et
la saisir », le compte de pages compris, puisqu'il vient d'être mesuré.

Chaque package généré affiche sa **planche en vignette**, à côté de ses chiffres et de
ses chemins de fichiers. C'est là que « est-ce que ça tient » se vérifie : sur du vrai,
pour chaque prestataire, avec son dos mesuré — et non sur une approximation qu'on
espère fidèle. Le PNG est écrit à côté du PDF, depuis la même source Typst ; c'est le
PDF qui part à l'impression.

## Modules

L'interface n'a aucune logique métier : elle invoque des commandes et affiche des
résultats. Tout le reste est testable sans fenêtre.

| Module | Rôle |
|---|---|
| `providers` | Table **unique** des gabarits : format, marges, gouttières, fond perdu, formule de dos |
| `manuscrit` | Markdown → chapitres → contenu Typst, avec refus explicite du non composable |
| `projet` | Le `.ozalid` : lecture, écriture, identité du livre |
| `png` | Lecture du bloc de réglages qu'`index.html` écrit dans ses PNG |
| `import` | Un `livre.toml` et un PNG de l'atelier → un projet et sa maquette |
| `image` | Dimensions naturelles d'une image, et cadrage dans une zone |
| `couverture` | Maquette typée → source Typst des deux faces |
| `maquettes` | Folio, Blanche et Surimpression |
| `typst` | Invocation du sidecar : mesurer la pagination, compiler, rendre un aperçu |
| `interieur` | Source Typst de l'intérieur, police du livre, et convergence gouttière/parité |
| `envoi` | L'envoi autographe : la main du livre, ses envois, et les noms qu'ils prennent sur le disque |
| `police` | Ce qu'un fichier de police déclare : sa famille, et les caractères qu'il porte vraiment |
| `diffusion` | Demander une image à un modèle : le prompt, le contrat, et la clé qui ne remonte jamais |
| `epreuve` | Source Typst de l'épreuve de relecture : A4, numéros de ligne, marge d'annotation |
| `planche` | Assemblage 4ème \| dos \| 1ère au gabarit, et dos composé élément par élément |
| `package` | Un prestataire, un intérieur, une planche, dans son répertoire |
| `epub` | Chapitres, couverture et police → une archive EPUB 3 reflowable, sans disque ni Typst |
| `ebook` | Le PDF et l'EPUB du livre entier, à côté du projet : le pendant local de `package` |
| `preferences` | Le `preferences.toml` : projets récents, et ce qui ne tient pas dans un livre |
| `menu` | Le menu natif : il demande, il n'agit pas — l'interface exécute |
| `commands` | Frontière avec l'interface, et projet ouvert |

`providers` fusionne les deux tables historiques du projet — celle d'`index.html`
pour la couverture, celle de `gen_interieur.py` pour l'intérieur — qui décrivaient
les mêmes prestataires sans jamais se recouper.

## Le fichier .ozalid

Une archive, un document :

```
projet.toml     identité du livre, police de l'intérieur, réglages de couverture,
                destinataires, envois, chemin source du manuscrit
manuscrit.md
images/         photos source de la 1ère et de la 4ème
polices/        la police manuscrite de l'auteur, quand il en fournit une
envois/         les images des envois, une par dédicataire
```

Les images d'envoi sont rangées à part de celles de la couverture, et ce n'est pas
une préférence d'organisation : `package::ecrire_images` donne un rôle aux images du
projet **par leur seul nom**, et tout ce qui ne commence pas par `quatrieme` y devient
la première de couverture. La police, elle, est là pour que le `.ozalid` reste
auto-portant — un livre composé dans l'écriture de son auteur doit se recomposer à
l'identique sur une machine où elle n'est installée nulle part.

La police de l'intérieur est une section à part, `[interieur]`, qui vaut `EB Garamond`
quand elle manque — un projet écrit avant qu'elle existe s'ouvre donc sans rien dire.
Les destinataires en sont une autre, `[livraison]`, avec le même principe : un projet
qui ne la porte pas se voit doté du premier gabarit de la table. La version du format
ne bouge pas pour autant — ajouter une section facultative ne rend illisible aucun
fichier existant, et la monter interdirait aux binaires déjà distribués d'ouvrir les
projets écrits ensuite. Un prestataire ou un papier que la table ne porte plus est
**élagué à l'ouverture** plutôt que de faire refuser le projet : le manuscrit et la
maquette sont intacts, et la liste se refait en trois clics.

Chaque destinataire y porte en outre **ce que sa dernière composition a mesuré** —
pages, gouttière, blanche, dos. Une par destinataire, parce que le même manuscrit ne
fait pas le même nombre de pages en poche et en grand format, et dans le fichier, parce
que rouvrir un livre composé la veille ne doit pas redemander une composition entière
pour un chiffre qui n'a pas bougé. L'invariant qui tient tout le dispositif tient en une
phrase : **une mesure enregistrée vaut toujours.** Rien n'y est estampillé, rien n'est à
comparer avant de s'en servir — ce qui pourrait la périmer l'efface à la source, dans le
Rust, au moment du geste : le livre (`modifier_livre` — une dédicace prend une belle
page et sa blanche), la police (`modifier_interieur`), le texte (`remplacer_texte`), le
papier et le relevé (`destinataire_regler`). Grossièrement et sans rien comparer :
recomposer pour rien coûte une composition, en rater une imprime un mauvais dos.

`deja_compose`, à côté de la liste, dit que ce livre a été composé au moins une fois. Il
n'est jamais repris, parce que c'est de l'histoire et non un état : lui seul distingue un
dos qu'on n'a jamais demandé — rien à faire — d'un dos qu'une modification vient de
périmer. C'est aussi le consentement de la **recomposition automatique** : une fois ce
premier clic donné, périmer une mesure la refait toute seule, débouncée et une à la
fois. Avant lui, rien ne part — regarder une première de couverture réclame un format,
pas une composition.

Le manuscrit y est **copié**, ce qui rend le projet complet sur une autre machine.
Corriger le fichier d'origine ne met donc pas la copie à jour : « Réimporter le
manuscrit » le fait, en un bouton, grâce au chemin mémorisé. L'écart entre les
chapitres attendus et ceux du manuscrit embarqué est affiché — c'est le seul signe
qu'une copie est périmée.

Les **sorties ne sont pas dans l'archive** : elles vont à côté, dans
`<nom-du-projet>/<prestataire>/`. Un projet non enregistré ne peut donc pas
composer, faute d'endroit où écrire. Seule l'épreuve de relecture reste à la racine,
en `epreuve.pdf` : elle ne vise aucun prestataire, elle n'a rien à faire dans leurs
répertoires. Les ebooks n'en visent pas davantage, et ils ont pourtant leur
répertoire, `ebook/`, frère de ceux des prestataires : ils sont deux fichiers et non
un, et les poser à la racine mêlerait le livre du lecteur à l'épreuve du relecteur.

## Le cycle de vie d'un projet

Un `.ozalid` est un document : il se crée vide, se remplit, s'enregistre et se
ferme. « Nouveau projet » ne demande rien — ni assistant, ni manuscrit d'emblée :
le texte se choisit quand on veut.

L'atelier retient un drapeau **modifié**, levé par toute commande qui touche au
projet et abaissé à l'écriture. C'est lui, et lui seul, qui décide si fermer perd
du travail : Nouveau, Ouvrir, Importer, Fermer, la fermeture de la fenêtre et
**Quitter** posent alors une boîte à trois boutons — Enregistrer, Ne pas
enregistrer, Annuler.

Le Rust pose la question ; **l'interface exécute la réponse**. C'est elle qui
possède le sélecteur de fichiers dont « Enregistrer sous… » a besoin, et c'est la
raison pour laquelle la fermeture de la fenêtre est retenue côté Rust puis rendue
à l'interface plutôt que tranchée sur place.

Le menu natif suit la même règle : aucune entrée n'agit, chacune émet un événement
que l'interface traite avec le code de ses propres boutons. Les boutons de l'écran
d'accueil sont des raccourcis du menu, pas une seconde vérité.

**« Quitter » n'est pas l'entrée prédéfinie du système**, et c'est délibéré :
celle-ci envoie `terminate:`, qui ne consulte jamais les fenêtres et traverserait
donc la garde sans la voir — ⌘Q aurait perdu le travail par le geste le plus
courant de macOS. C'est une entrée ordinaire, qui demande comme les autres.

Comme l'interface devient dès lors nécessaire pour quitter, un témoin la protège :
tant qu'elle n'a pas appelé `interface_prete` — ce qu'elle fait une fois ses
écouteurs posés, et pas avant —, la fermeture n'est pas retenue et « Quitter »
sort directement. Une interface qui n'a jamais démarré n'a rien à perdre, et une
application qu'on ne pourrait plus quitter serait pire que la question qu'on
aurait manqué de poser. Ce filet ne tient que par un invariant : le seul chemin
vers le drapeau *modifié* passe par une commande de l'interface. Une restauration
automatique de projet au démarrage le briserait.

**Limite connue**, non comblée : le « Quitter » du menu contextuel du Dock et
l'extinction de la session macOS envoient `terminate:` directement. Les couvrir
exigerait un `applicationShouldTerminate:` que ni tao ni Tauri 2.11 n'exposent.

Les **projets récents** vivent dans un `preferences.toml` du répertoire de
configuration de l'application — jamais dans un `.ozalid`, qui porte le livre et
non les habitudes de celui qui l'ouvre. La liste est plafonnée à dix, et les
chemins dont le fichier a disparu sont élagués **à la lecture** : un projet sur un
volume démonté revient de lui-même au remontage, alors qu'une purge l'aurait
perdu. Son écriture est au mieux — un échec s'écrit sur la sortie d'erreur,
visible en développement, invisible pour qui lance le binaire empaqueté, et c'est
assumé : ce qui se perd est une liste de raccourcis, pas un livre.

## Vérifications

```
cd app/src-tauri && cargo test --lib && cargo clippy --all-targets && cargo fmt --check
cd app && node --test "tests/*.test.js"
```

Et les exercices sur livre réel, à rejouer après toute modification de la
composition — le compte de pages est ce qu'on compare :

```
cd app/src-tauri
cargo run --example importer -- <livre.toml> <projet.ozalid>
cargo run --example composer -- <projet.ozalid> lulu <sortie>
cargo run --example maquette -- <projet.ozalid> lulu <sortie>
cargo run --example packager -- <projet.ozalid> <sortie> lulu tbe-110x170 bookvault-127x203
cargo run --example epreuve -- <projet.ozalid> <epreuve.pdf>
cargo run --example ebook -- <projet.ozalid> <sortie>
```

`packager` traverse la chaîne entière sans interface : intérieur composé, pagination
mesurée, dos calculé, planche assemblée. C'est ce qui prouve que Typst compile
vraiment ce que le moteur émet.

`maquette` rend les maquettes en PNG : c'est la vérification qu'aucun test ne peut
faire — la position du cadre, l'assiette du bloc titre, le voile. À rejouer et à
regarder après toute modification du moteur de couverture.

`epreuve` tire l'épreuve de relecture sans interface. Elle se regarde de la même
façon : les numéros de ligne repartent-ils de 1 à chaque page, la marge d'annotation
est-elle libre, un chapitre commence-t-il bien en tête de page.

`ebook` écrit le PDF et l'EPUB sans interface, et l'un et l'autre se regardent. Le
PDF est le livre qu'on ne reliera pas : la couverture ouvre-t-elle le fichier, les
marges d'une page paire et d'une page impaire sont-elles symétriques — la gouttière
est revenue à l'extérieur —, aucune page vide ne traîne-t-elle à la fin, faute de
blanche de parité à combler. L'EPUB, lui, se juge dans une liseuse : la vignette
paraît-elle à l'ouverture, la table des matières mène-t-elle au bon chapitre, les
italiques sont-elles là, et le texte est-il dans la police du livre plutôt que dans
celle du lecteur.

`temoin` diffère des exercices ci-dessus : lui seul porte sa propre valeur attendue, et
il échoue au lieu d'afficher un résultat à interpréter.

```
cd app/src-tauri && cargo run --example temoin
```

Le manuscrit qu'il compose est *Candide* (Voltaire, 1759, domaine public), versionné
dans `temoin/manuscrit.md` parce que `build/` ne l'est pas et qu'un manuscrit personnel
n'a rien à faire sur un runner GitHub. Sa réussite sous Windows établit que Typst y
pagine comme sur macOS — donc qu'un dos calculé sur l'une des deux plateformes vaut
pour l'autre.

Les tests du front exécutent le vrai `src/app.js` dans un faux DOM qui lit l'état
initial dans le vrai `src/index.html`. Ils couvrent le câblage, jamais le rendu :
tout ce qui se voit se vérifie dans l'application.

## Points d'attention

- **`line-height` CSS ≠ `leading` Typst.** La boîte de ligne est ramenée à 1 em
  (`top-edge: 0.75em, bottom-edge: -0.25em`) pour que les deux grandeurs
  coïncident. Sans cela l'interligne dépend de la police.
- **L'aperçu par face montre la couverture rognée, la planche montre le fond perdu.**
  Un élément calé au bord touche le bord dans l'onglet 1ère et s'en trouve à quelques
  millimètres sur la planche : cette bande-là est celle que le massicot emporte, elle
  n'est pas un espace ajouté. Les deux vues sortent du même moteur ; c'est la boîte qui
  diffère (`Boite::rognee` contre `Boite::une`).
- **Une pastille réglée à 0 % déborde volontairement dans le fond perdu.** Le bord du
  livre fini est une ligne de coupe, pas une limite : le massicot y travaille à un ou
  deux millimètres près. Le fond de la pastille descend donc sous la coupe, et son
  placement suit d'autant — le texte ne bouge pas. Sans cela le tirage rendrait tantôt
  une pastille amputée, tantôt un liseré de couverture entre elle et le bord. Le débord
  se déduit de la boîte, jamais du prestataire : nul du côté du dos, nul sur un aperçu
  par face.
- **Le manuscrit n'admet qu'un sous-ensemble de Markdown.** Tout le reste est
  refusé avec son numéro de ligne — un aplatissement silencieux donnerait un
  livre faux, découvert après tirage.
- **Un saut de page de traitement de texte (U+000C) traverse la composition sans
  broncher.** Typst le compose sans une erreur ; le XML, lui, ne sait pas
  l'écrire, et la liseuse n'ouvre alors pas le chapitre. La génération de l'EPUB
  le refuse en le nommant, plutôt que de le retirer : un nettoyage silencieux
  donnerait un livre que personne n'a écrit, et laisserait le défaut dans le
  manuscrit pour la fois suivante.
- **La police de l'intérieur est un réglage du projet, et elle repagine.** Sept serifs
  de labeur sont admis, EB Garamond par défaut ; le compte de pages, donc le dos, en
  dépend. Une police hors liste est refusée au lieu d'être substituée : Typst, lui,
  composerait dans sa police par défaut sans lever la moindre erreur, et le livre
  sortirait faux en silence.
- **Georgia et Helvetica ne sont pas reprises.** Elles appartiennent au système, ne
  sont pas redistribuables, et Helvetica n'existe pas sous Windows. Une maquette
  importée qui les utilise est refusée avec la liste des familles embarquées.
- **Dans un EPUB, une police se nomme par une URL, et la plage de graisses qu'elle
  annonce est crue sur parole.** Les familles variables de Google Fonts portent leur
  bloc d'axes entre crochets — `EBGaramond[wght].ttf` —, interdits dans un segment
  d'URL : l'archive les renomme, sans quoi EPUBCheck la refuse en entier et une
  liseuse indulgente retombe sans un mot sur l'écriture du lecteur. Et une face
  annoncée sur toute la plage alors qu'elle ne la couvre pas ne fait pas
  synthétiser le gras : la liseuse sert le romain à sa place, et le livre perd son
  gras en silence.
- **Le prolongement panoramique dépend de la pagination.** L'image y est cadrée sur
  la planche entière — deux couvertures et le dos — et non sur la seule 1ère :
  le composer sans compte de pages est refusé, pas approximé. C'est un écart
  délibéré avec `index.html`, qui cadrait sur une couverture et laissait la 4ème en
  papier nu tant qu'on n'avait pas grossi l'image à la main.
- **La planche ne porte aucun trait de coupe.** Lulu, KDP et Bookvault les refusent
  explicitement ; le fond perdu suffit à dire où couper.
- **Le dos se règle élément par élément.** Auteur, titre, éditeur et collection y ont
  chacun leur style, leur place — pied, centre ou tête —, leur rang et leur sens de
  lecture, parce que les collections ne s'accordent pas là-dessus. La collection est
  éteinte par défaut : allumée d'office, elle ajouterait un texte au dos de tous les
  livres qui en portent une. Seule la **largeur** du dos échappe au réglage : elle
  vient de la pagination, et c'est tout l'objet de l'application.
- **La place, le rang et le sens ne se règlent qu'à la souris.** On traîne le texte au
  tiers du dos qu'on veut ; l'icône posée dans son coin le retourne — et, pour la
  collection seule, le couche en travers du dos, d'un quart de tour à gauche ou à
  droite. Le panneau ne les offre pas : il redirait ce que l'aperçu montre déjà.
  Une réserve à connaître : l'épaisseur qu'un élément couché en travers réclame est la
  longueur de sa ligne, que seul Typst connaît — `dos_requis` ne la mesure pas, et un
  tel élément trop long se voit rogné sur la face Dos sans autre signal.
- **Le dos a sa face, et elle est couchée.** L'étape Couverture en compte quatre —
  1ère, 4ème, Dos, Planche — et le Dos s'y compose seul, sans fond perdu, sur une page
  d'un quart de tour. À sa taille : treize millimètres restent treize millimètres, et
  c'est la page tournée qui les étale sur la largeur de la fenêtre. Debout, il n'aurait
  tenu à l'écran que par sa hauteur, et se serait réglé dans trente-neuf pixels — trois
  de plus que sur la planche. La **Planche**, elle, n'a plus aucun réglage : elle ne se
  règle pas, elle se vérifie, et son panneau disparu lui rend la fenêtre entière.
- **L'aperçu et le PDF sortent de la même source.** Il n'y a donc pas d'écart
  écran/export à surveiller — le piège que consignait le `CLAUDE.md` du projet
  n'existe plus ici.
- **Le panneau de réglages est construit depuis un schéma** (`src/couverture.js`),
  pas écrit à la main : un chemin faux y laisse un contrôle vide, ce qui se voit
  tout de suite, et un test vérifie que tous les chemins existent.
- **L'icône est provisoire.** Un placeholder, pas une identité visuelle.
