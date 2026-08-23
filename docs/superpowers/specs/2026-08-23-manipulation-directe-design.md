# La couverture se règle sur elle-même

Date : 2026-08-23
Statut : appliqué (brainstorming du 23/08)

## Objectif

Soixante contrôles dans une colonne de 22 rem, et l'objet qu'ils règlent posé juste à
côté. Régler une marge latérale, c'est aujourd'hui chercher « Marge latérale » dans le
panneau, pousser un nombre, et regarder ailleurs pour voir ce qu'il a fait. L'atelier
HTML gelé, lui, se réglait à la souris **sur la couverture** — on tirait l'image, on
poussait la frontière du bandeau, on écartait les marges du bloc titre.

Cette spec rend ce geste à l'application, sur les deux faces qui portent une image et
sur le dos, et pose l'aperçu sur un fond où une couverture se juge.

Trois chantiers, du plus sûr au plus lourd :

1. **L'établi** — le fond de la fenêtre et de la scène.
2. **Les mesures** — ce que la planche mesure, écrit sous elle.
3. **La manipulation directe** — la 1ère, la 4ème, et le dos.

## Décisions de cadrage (brainstorming du 23/08)

- **La scène prend le carton gris 18 %, la coquille reste claire.** Reprendre le fond
  d'`index.html` entier aurait fait passer toute la fenêtre en gris sombre et obligé à
  refaire la table des gris. On prend le carton là où il sert — sous la couverture — et
  on neutralise la coquille à luminance égale.
- **La photo bouge en direct, pour de vrai.** Pas un cadre fantôme : la vraie photo, à
  la vraie place, sous le vrai titre. C'est le point qui a coûté le plus, et c'est celui
  qui fait la différence entre « je cadre » et « je pousse un curseur en aveugle ».
- **Le reste bouge en guide, et la vérité rattrape à la pause.** Le bandeau et le bloc
  titre sont du texte composé par Typst : le faire bouger en direct voudrait dire refaire
  la mise en page de Typst dans le navigateur.
- **Le dos se réorganise sur le dos**, pas dans un bandeau de jetons posé dessous. La
  conséquence — mesurer les textes — a été acceptée en connaissance de cause.
- **La 4ème suit la 1ère.** Ce sont les mêmes réglages sous un autre préfixe ; ne pas la
  faire coûterait une asymétrie inexplicable.

## 1. L'établi

`--fond` était `#eceae6` : un gris **chaud**, c'est-à-dire encore du crème. La feuille
de style raconte déjà pourquoi la coquille avait quitté le crème de la Blanche — sur du
crème, un blanc paraît bleu et un beige paraît neutre. Le gris chaud gardait le défaut
en plus discret.

Deux mouvements :

- la coquille passe au gris **neutre** `#ecece9`, celui des panneaux d'`index.html` ;
- la scène prend le carton gris 18 % (`#7a7b78` → `#6a6b68` en dégradé radial très
  court), valeurs reprises telles quelles : c'est une convention d'atelier, pas un goût.

Les quatre gris de la table sont neutralisés **à luminance égale**. C'est ce qui permet
de ne pas rejouer tout l'équilibre : l'invariant de la feuille — `--gris` est le plus
clair qui tienne 4,5:1 sur `--fond`, `--surface` et `--survol` — survit sans être
recalculé. Mesuré avant/après sur le pire des trois : 4,66 → 4,71.

L'aperçu gagne la double ombre de l'atelier, **et elle seule** : `.scene .apercu`, pas
`.apercu`. L'aperçu d'un envoi vit dans un bloc clair, où cette ombre-là serait une
tache.

## 2. Ce que la planche mesure

Les dimensions du fichier remis ne se lisaient nulle part dans l'étape Couverture. Elles
s'écrivent sous la planche, en chasse fixe :

```
Planche 238,98 × 181,35 mm — dos 16,63 mm — fond perdu 3,175 mm
```

Les quatre nombres viennent du Rust, dans un `Mesures` que `couverture_apercu` remplit
pour la seule face qui en a — la planche. La largeur d'une planche est deux couvertures,
un dos et deux fonds perdus : cette addition est déjà écrite dans `planche::Gabarit`, et
la refaire en JavaScript la ferait dériver le jour où un prestataire compterait
autrement.

`Mesures` est **séparé** de `Reperes`, avec qui il voyage pourtant toujours : les
repères sont des fractions posées *sur* l'image, ceux-ci des millimètres écrits *sous*
elle. Les confondre ferait porter à l'habillage une unité qui n'y survit pas.

## 3. Le direct

### Le mur des 110 ms

Mesuré sur une 1ère au format réel avec une vraie photo, 150 dpi : **110 ms** par
composition Typst, à quoi s'ajoutent un PNG de 680 Ko encodé en donnée `data:` et son
décodage — de l'ordre de 200 ms bout en bout. Cinq images par seconde. Trop lent pour
suivre la souris ; assez rapide pour rattraper une pause.

Recomposer à chaque `pointermove` était donc exclu, et un cadre fantôme sans image ne
valait pas le geste. D'où le calque.

### Les trois pièces

`couverture_calques(face)` publie trois choses qui, empilées dans cet ordre, refont la
face à l'identique :

```rust
pub struct Calques {
    /// La face composée sans son papier ni sa photo, en PNG à fond transparent.
    pub habillage: String,
    /// La photo telle que le projet la porte, en donnée `data:`.
    pub photo: String,
    pub naturel_l: u32,
    pub naturel_h: u32,
    /// La zone où la photo se compose, en fraction de la face.
    pub zone: Zone,
    /// Le papier de cette face-là : la 4ème peut avoir le sien.
    pub papier: String,
}
```

La fenêtre ne déplace que la pièce du milieu. Le titre reste donc posé **sur** la photo
pendant qu'on la cadre, comme dans l'atelier HTML où le DOM le permettait gratuitement.

Le point important est ce que l'habillage **n'est pas** : ce n'est pas une deuxième
source. C'est la même — `source_une`, `source_quatre` — composée sur un papier
transparent (`rgb("#00000000")`), sans photo, avec `#set page(fill: none)` en tête. Une
deuxième façon d'écrire une couverture finirait par montrer autre chose que ce qui
s'imprime, et c'est précisément le défaut que l'application existe pour supprimer.

La commande est demandée **après** l'aperçu et **jamais pendant un geste** : l'habillage
ne dépend pas du cadrage, celui qu'on tient vaut donc pour le geste entier.

### Le rattrapage

Pendant le geste, la valeur est posée dans le contrôle et une composition part à la
première pause — 150 ms. Deux attentes se suivaient sur ce chemin, celle de la commande
puis celle de l'aperçu ; la seconde tombe à zéro pendant un geste, la première ayant
déjà fait l'attente.

Deux gardes en découlent, et elles ne sont pas décoratives :

- **Le panneau ne réécrit pas le champ que la souris tient.** La maquette qui revient est
  celle du départ de *cette* composition, donc en retard sur la souris. Sans la garde, la
  couverture recule d'un cran à chaque rattrapage et le geste devient infinissable.
- **Le direct ne s'efface pas à l'arrivée d'un aperçu tant qu'un geste dure.** Il montre
  plus juste que l'image qui vient d'arriver. Il ne part qu'au premier aperçu posé après
  le relâchement — l'effacer au relâchement ferait revenir la couverture à son ancien
  cadrage le temps d'une composition.

### Ce qui n'est pas en direct

Le bandeau, le bloc titre et les marges déplacent du **texte composé**. Le montrer bouger
voudrait dire refaire la mise en page de Typst dans le navigateur. Ce qui suit la souris
est donc le guide — la frontière, le bord haut du bloc, les deux marges — et la vérité
arrive à la pause.

## 4. Les prises

Effacées tant que le curseur est ailleurs, révélées dès qu'on approche la main, et
maintenues tant qu'on tient (`data-geste` sur le cadre — sortir du cadre en tirant ne
doit pas les faire disparaître sous la souris qui les tient). Une couverture se juge sans
guides par-dessus : c'est tout l'objet de l'établi gris en dessous.

| Prise | Ce qu'elle déplace | Où elle se dessine |
| --- | --- | --- |
| Cadre de l'image | `cadrage.x` / `.y`, `.zoom` à la molette | la zone que le Rust publie |
| Frontière du bandeau | `bandeau` | `bandeau / 100` |
| Barre du bloc | `bloc_y`, ou `quatrieme.top` | voir ci-dessous |
| Deux poignées | `pad_x`, ou `quatrieme.pad_x` | aux bouts de la barre |

Trois choses que le geste ne peut pas ignorer :

- **La référence de l'image est son mou réel**, pas la largeur de la face. Une photo à
  peine plus grande que sa zone se déplace de ce qu'elle peut. Mou nul sur un axe : le
  geste s'y refuse plutôt que de bouger un curseur qui ne bouge rien — c'est le parti de
  l'atelier HTML, et un geste sans effet visible fait douter du réglage, pas du cadrage.
- **Les unités ne sont pas les mêmes.** `bloc_y` se compte en pourcentage de la
  **hauteur** de couverture, `quatrieme.top` en pourcentage de sa **largeur**. Le même
  déplacement vertical n'y vaut pas le même nombre — d'un tiers d'écart en poche.
- **En mode Bandeau, la barre du bloc se montre mais ne se tire pas** : sa hauteur
  découle de la bande (22 % de celle-ci, règle tenue par `bloc_texte`). Seules ses
  poignées répondent, exactement comme dans l'atelier HTML.

La souris pose ses valeurs **au cran du schéma**, celui qu'offrent déjà les flèches du
champ : elle produit des réels, et une hauteur de bandeau à 37,813567143516 % s'écrit
telle quelle dans le champ, part telle quelle au projet, et ne se retape pas à la main.

Un geste qui ne déplace rien ne commet rien. La comparaison porte sur les valeurs, pas
sur le fait qu'un `pointermove` ait eu lieu : un pixel de tremblement, ramené au cran, ne
change rien non plus. Sans cela, poser la souris sur sa propre couverture réveillerait la
garde des modifications à la fermeture.

Le portage de `image::place` en JavaScript est **la seule règle de composition qui existe
en deux langues** dans l'application. Le prix est assumé : sans elle, la fenêtre ne peut
pas montrer la photo suivre la souris. Une table de cas partagée — les cinq cas des tests
d'`image.rs`, recopiés dans ceux de la fenêtre — est ce qui les tient d'accord. Si l'une
bouge sans l'autre, la photo suit la souris ailleurs que là où Typst la composera, et le
geste ment sans jamais lever d'erreur.

## 5. Le dos

### Les boîtes ne se devinent pas

La hauteur d'encre d'une ligne ne dépend que de la famille — d'où la table `ENCRE` et
`dos_requis`. Sa **longueur** dépend de chaque glyphe, et seul Typst tient les métriques
des polices embarquées.

`typst eval` les donne sans rien composer :

```
typst eval 'query(<mesures>).map(it => it.value)' --in mesures-dos.typ --format json
```

Une source qui ne rend aucune page, un `#metadata` qui porte trois `measure(...).width`,
et une réponse JSON en **quinze millisecondes** — contre cent dix pour un aperçu.
`Typst::mesures` la porte, à côté de `pages` qui emploie déjà `eval` pour la même raison.

### La grille à cinq colonnes, relue depuis l'autre bout

`bloc_dos` compose `(auto, 1fr, auto, 1fr, auto)` — pied, ressort, centre, ressort, tête
— dans un bloc long comme le dos, retiré de sa marge aux deux extrémités.
`boites_dos` relit cette même mise en page pour en déduire où chaque texte tombe.

Le piège, trouvé à l'écran et pas en test : **les deux ressorts se partagent ce que le
pied et la tête laissent**. Le centre n'est donc centré sur le dos que lorsque les deux
extrémités pèsent pareil. Le croire centré le décale de la moitié de leur différence —
sept millimètres sur une poche, et la prise se pose à côté du titre.

L'aperçu du dos est couché, et la double rotation de `source_dos` place le **pied à
gauche** et la **tête à droite**. Ce n'est pas un choix d'affichage : c'est aussi l'ordre
de lecture du dos, du début du livre vers sa fin.

### Le dépôt

La place se lit au tiers du dos où le curseur a lâché ; le rang, au nombre de voisins
déjà passés. Les rangs de chaque place sont ensuite renumérotés d'un bout à l'autre :
laisser des trous ferait dépendre l'ordre de nombres qui ne veulent plus rien dire, et
deux éléments finiraient par partager un rang — auquel cas c'est le tri du Rust qui
trancherait, sans que personne l'ait décidé.

Rien n'est commis en chemin : la place et le rang n'ont de valeur qu'une fois le doigt
levé, et une composition par tiers traversé ferait clignoter le dos sous la souris. Les
trois places n'apparaissent que pendant le geste — hors geste, elles diraient une
structure qu'on ne cherche pas, sur une bande de soixante pixels.

## 6. Ce que ça coûte ailleurs

- `composes` rend désormais la clé de chaque élément avec lui : deux appelants la
  demandent, et la déduire de l'ordre du tableau était un piège.
- `photo_quatre` est extraite de `corps_quatre` — deux appelants en ont besoin et un seul
  compose. Recopier ce `match` ailleurs, c'est accepter qu'un jour la souris cadre une
  zone et Typst une autre.
- `papier_quatre` sort du corps de `corps_quatre` pour la même raison.
- Le faux DOM des tests gagne `getBoundingClientRect` et `removeEventListener`. Le
  premier parce qu'un geste convertit des pixels de souris en pourcentages de couverture,
  et que cette division-là est exactement ce qu'un test doit vérifier. Le second parce
  qu'un geste pose trois écouteurs à la pression et les reprend au relâchement : sans
  retrait réel, le deuxième geste rejouerait le premier par-dessus.
- Les positions passent par des **variables CSS** et non par `style.left` : c'est déjà ce
  que fait la coupe, ça traverse un `calc()`, ça se relit dans un test, et ça survit à un
  aperçu affiché à la taille que la fenêtre lui laisse.

## 7. Vérification

### Ce que les tests doivent tenir

Rust :

- `boites_dos` pose le pied contre la marge, sépare deux voisins de l'écart, et cale la
  tête contre la marge de l'autre bout.
- Le centre, sur un dos dont la tête est vide, **n'est pas** au milieu du dos : c'est le
  cas qui distingue la vraie règle de la fausse.

JS :

- Le portage de `place` rend, sur les cinq cas d'`image.rs`, exactement ce que le Rust
  rend.
- Tirer la photo déplace l'ancrage **de son mou réel**, dans le sens du geste, et
  **ne touche pas** l'axe sans mou.
- La hauteur du bloc de la 4ème se compte sur la largeur : un geste qui ignore l'unité
  pose un nombre visiblement différent.
- Le panneau ne réécrit pas le champ que la souris tient. La commande doit être
  **retenue** dans le test : une commande instantanée répondrait toujours ce que le champ
  porte encore, et le test serait vrai d'avance.
- Un clic qui ne déplace rien ne commet rien, sur la couverture comme sur le dos.
- Les prises ne s'offrent que là où il y a quelque chose à saisir, et une prise d'une
  face ne reste pas posée en travers d'une autre.
- Déposer un texte du dos le range et renumérote sa place ; la prise saisie suit la
  souris ; rien ne part avant que le doigt ne se lève, **délai de grâce dépassé**.
- Les mesures s'écrivent sous la planche et sous elle seule.

Chaque test neuf doit avoir été vu rouge — TDD ou mutation ciblée. Deux d'entre eux ne
protégeaient rien au premier jet et ont été refaits : celui du panneau (bouchon trop
rapide) et celui du dépôt (assertion posée avant le délai de grâce, et rien qui vérifie
que la prise suit la souris).

### À l'œil

Sur un manuscrit réel, chez un prestataire à fond perdu publié :

- la photo suit la souris au 1:1 et l'image composée qui arrive ensuite **tombe au même
  endroit** — c'est la seule preuve que le portage de `place` dit vrai ;
- le titre reste visible sur la photo pendant tout le geste ;
- les prises du dos tombent **sur** leurs textes, à toutes les répartitions des trois
  places, et notamment quand une extrémité est vide ;
- la 4ème sans texte ni image propre n'offre aucune prise, et c'est correct ;
- la fenêtre à 900 px garde sa barre d'outils sur une ligne.

## 8. Hors périmètre

- **Le direct sur le bandeau et le bloc titre.** Il faudrait la mise en page de Typst
  dans le navigateur. Le guide plus le rattrapage à la pause est le bon compromis.
- **La 4ème en prolongement panoramique ne se cadre pas depuis la 4ème.** Son cadrage est
  celui de la 1ère — le panneau le dit déjà —, et offrir ici une poignée qui déplacerait
  la photo de l'autre face serait un piège.
- **Mesurer le débord d'un élément sous la coupe.** La spec du fond perdu visible
  l'écartait au motif que le Rust ne connaît pas la boîte des éléments composés. Ce
  chantier ouvre la porte — `typst eval` sait mesurer — mais ne la franchit pas : ce qui
  est mesuré ici, ce sont trois textes de dos dont on connaît d'avance la mise en page,
  pas la boîte d'un élément quelconque sur une page composée. Autre chantier, toujours.
- **Une sélection à la manière de l'atelier HTML.** Les prises répondent au survol et au
  geste, sans état sélectionné : la scène ne défile pas, la molette n'y a rien d'autre à
  commander, et un état de plus se serait vu dans les tests avant de se voir à l'écran.
