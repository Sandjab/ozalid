# Le fond perdu visible sur la planche

Date : 2026-08-22
Statut : validé (brainstorming)

## Objectif

La face Planche est la vue de contrôle : 4ème | dos | 1ère, au gabarit du prestataire
visé, fond perdu compris. Mais rien à l'écran ne dit **où passe la coupe**. Une image
qui déborde volontairement à fond perdu et une pastille qui descend sous le trait de
coupe se ressemblent : les deux mordent sur la même bande de quelques millimètres, et
l'une est voulue quand l'autre est une faute qui ne se verra qu'au massicot.

Cette spec fait apparaître cette bande à l'aperçu — voilée, bordée d'un pointillé — sans
qu'un seul trait n'entre dans le fichier qui part à l'imprimeur.

## Décisions de cadrage (brainstorming du 22/08)

- **Voile *et* pointillé**, pas l'un ou l'autre : le voile dit « ceci sera rogné », le
  pointillé dit « la coupe est ici ». Sur une couverture claire le voile ne se voit
  presque pas et le trait reste ; sur une image sombre à fond perdu le trait se perd
  dans le motif et le voile reste.
- **Une bascule, allumée par défaut.** On éteint pour juger la couverture telle qu'elle
  sera en main.
- **L'habillage est posé par le front, à partir d'une mesure que le Rust donne.** Le PDF
  remis reste intact par construction, et la bascule ne relance pas Typst.
- **Ce qu'on montre est la zone, pas l'élément fautif.** « La pastille dépasse de 2 mm »
  serait une mesure à calculer en Rust — hors périmètre, voir § 6.

## 1. Ce qui existe et qu'on ne touche pas

`planche::source` ne porte **aucun trait de coupe ni repère de pli** : Lulu, KDP et
Bookvault les refusent explicitement. Cette source sert deux usages sans se dédoubler —
le package du prestataire et l'aperçu de la face Planche. Elle continue.

C'est la raison du choix : le repère n'existe qu'à l'affichage, dans une couche que le
PDF ne traverse jamais. Le mode « épreuve » qui hante `planche::source` sous un drapeau
aurait été un trait à un `if` près du fichier d'impression.

## 2. Le contrat Rust → front

`couverture_apercu` renvoie aujourd'hui une `String` — la data URL du PNG. Il renverra :

```rust
/// Ce qu'un aperçu de face donne à voir : l'image, et où la planche se coupe et se plie
/// s'il y a lieu.
#[derive(Serialize)]
pub struct Apercu {
    pub image: String,
    /// Absents sur les faces qui se composent sans fond perdu.
    pub reperes: Option<Reperes>,
}

/// Où la planche se coupe et où elle se plie, en fraction de ses propres dimensions.
/// `x` et `y` diffèrent : une planche est bien plus large que haute.
#[derive(Serialize)]
pub struct Reperes {
    pub x: f64,
    pub y: f64,
    pub pli_quatre: f64,
    pub pli_une: f64,
}
```

Les deux plis sont venus après coup, le 23/08 : une maquette dont le dos porte le papier
des deux faces paraît d'un seul tenant, et c'est précisément celle où le dos se rate. Ils
voyagent avec la coupe parce qu'ils s'affichent avec elle, sous la même lunette — d'où
son nom, « Repères », et non « Fond perdu ».

`reperes` est `None` pour la 1ère, la 4ème et le dos : ces trois faces se composent au
format rogné, sans fond perdu (`source_dos` le dit déjà en toutes lettres). Il n'y a
donc rien à y marquer — et c'est le Rust qui l'affirme, plutôt que le front qui le
déduise d'un nom de face.

Le calcul vit dans `planche::Gabarit`, à côté de `largeur()` et `hauteur()` :

```rust
/// La part du fond perdu sur la largeur et sur la hauteur de la planche.
pub fn part_fond_perdu(&self) -> (f64, f64) {
    (self.fond_perdu / self.largeur(), self.fond_perdu / self.hauteur())
}
```

Il n'est pas dans `commands.rs` : la commande assemble déjà le gabarit à partir du
prestataire visé et du relevé, et c'est au gabarit de savoir ce qu'il mesure.

## 3. L'habillage, côté front

### Le cadre épouse l'image

`#apercu` est enveloppé dans un `<div class="cadre">`. L'habillage se cale sur **l'image
rendue**, jamais sur `.scene` : la scène occupe la largeur de la colonne, une couverture
y est centrée et plus étroite, et un habillage calé sur la scène marquerait la coupe à
côté de la couverture.

Le cadre ne peut pas se dimensionner sur l'image : celle-ci se borne en pourcentage de
son cadre, le cadre attend sa taille de l'image, et le navigateur tranche ce cycle à
zéro — mesuré, cadre et image à 0 × 0 dans une scène de 620 × 345. Il tient donc sa
taille de son **rapport d'aspect**, borné par `max-width` et `max-height`, et l'image le
remplit. Le rapport est posé par le front à partir de l'image décodée (`naturalWidth /
naturalHeight`), et retiré avec elle : un cadre sans image garderait sinon sa place,
vide, et pousserait plus bas le message qui dit qu'il n'y a rien à voir.

C'était le point délicat du chantier, et il ne se voyait qu'à l'écran : aucun test du
faux DOM ne mesure une boîte. La garde est donc double — un test qui vérifie que le
rapport est bien posé et bien retiré, et une vérification au navigateur.

### Un seul élément pour les trois effets

Un enfant du cadre, `.reperes`, posé sur toute l'image, en `pointer-events: none` :

- **le voile** est fait de quatre fonds — haut, bas, gauche, droite — dimensionnés en
  pourcentage depuis les deux fractions. Pas d'ombre étalée sous un `overflow: hidden`,
  qui aurait été plus courte à écrire : ce découpage emporterait l'ombre portée de
  l'aperçu, et la couverture paraîtrait posée à plat. Les bandes latérales s'arrêtent
  au-dessus et au-dessous des horizontales, sans quoi deux voiles se superposeraient
  aux quatre coins — et ces coins-là sont justement ce qu'on regarde.
- **la ligne de coupe** est un `::after` en `inset` sur le rectangle rogné, bordé d'un
  `1px dashed`. En pseudo-élément : elle n'a rien à dire au balisage.
- **les deux plis** sont un `::before` portant deux dégradés horizontaux, chacun deux
  filets accolés — un clair, un sombre — sur toute la hauteur. Leurs positions sont
  redites en entier à chaque arrêt : un arrêt de dégradé qui recule est ramené au
  précédent, et deux `0` de suite donneraient une bande de largeur nulle, donc un pli
  absent sans que rien ne le signale.

Les deux variables sont posées par le JS depuis les `reperes` reçus. La visibilité de
`.reperes` tient à un `hidden` : pas de `reperes`, ou bascule éteinte, il disparaît.

### La bascule

Un bouton « Repères » dans la barre `.outils`, à deux états `aria-pressed` — le
pattern des boutons de face, pas celui des onglets d'étape. Allumé par défaut, masqué
hors face Planche, comme les blocs de réglages sans objet.

Son état vit en mémoire à côté de `face`, dans `app.js`. Rien n'en va dans le
`.ozalid` : c'est une lunette, pas un réglage du livre.

## 4. Ce que ça coûte ailleurs

Les faux `invoke` des tests qui renvoient une chaîne nue pour `couverture_apercu`
suivent le nouveau contrat : `tests/composition.test.js`, `tests/couverture.test.js`
(trois occurrences), `tests/ebook.test.js`. Ceux qui lèvent une erreur pour cette
commande — `contrats`, `coquille`, `cycle_de_vie` — n'ont rien à changer.

C'est le prix direct du changement de contrat, pas un ménage adjacent : aucun autre
test n'est touché.

## 5. Vérification

### Ce que les tests doivent tenir

Rust :

- `part_fond_perdu` sur un gabarit connu rend bien deux fractions distinctes, celle de
  la largeur plus petite que celle de la hauteur.
- Un gabarit à fond perdu nul rend `(0.0, 0.0)` — la face n'a alors rien à marquer et
  l'habillage ne trace pas un trait sur le bord même de l'image.
- `plis` rend deux fractions dont l'écart, ramené en millimètres, **vaut le dos** : c'est
  ce qui fait de ces deux traits la seule chose qui montre où le livre se plie.

JS :

- Après un aperçu de la face Planche, les quatre variables — `--coupe-x`, `--coupe-y`,
  `--pli-quatre`, `--pli-une` — sont posées sur le cadre et `.reperes` est visible.
- La bascule éteinte, `.reperes` est masqué **sans** nouvel appel à `couverture_apercu` :
  c'est tout l'intérêt d'habiller plutôt que de recomposer.
- Sur la face 1ère, où la commande ne rend pas de `reperes`, `.reperes` est masqué.
- Un aperçu en échec ne laisse pas l'habillage seul à l'écran : `poserApercu(null)`
  retire l'image *et* la coupe.

Chaque test neuf doit avoir été vu rouge — TDD ou mutation ciblée.

### À l'œil

Sur un manuscrit réel, chez un prestataire à fond perdu publié :

- les quatre faces gardent leur mise en page, la face Dos comprise, avec son bandeau ;
- une couverture claire et une couverture à photo sombre montrent toutes deux la coupe ;
- la fenêtre à 900 px de large — la plus étroite qu'on autorise — garde sa barre
  d'outils sur une ligne, bouton compris.

## 6. Hors périmètre

- **Mesurer le débord d'un élément.** Dire de combien la pastille passe sous la coupe
  demanderait au Rust de connaître la boîte de chaque élément composé ; la couverture
  est aujourd'hui décrite en pourcentages et composée par Typst, qui seul sait où les
  choses tombent. Autre chantier. *Le 23/08, la manipulation directe entrouvre la
  porte : `typst eval` mesure un texte en quinze millisecondes, et les trois textes du
  dos ont désormais leur boîte. La marche restante est entière — connaître la boîte
  d'un élément **quelconque** sur une page composée n'est pas la même chose que mesurer
  trois textes dont on connaît d'avance la mise en page. Voir
  `2026-08-23-manipulation-directe-design.md`.*
- ~~**Le pli du dos.**~~ Écarté le 22/08 au motif que la planche montre ses trois zones
  par leurs fonds — faux dès qu'elles portent le même papier, et c'est le cas courant.
  **Fait le 23/08** : deux filets accolés, l'un clair et l'autre sombre, sur toute la
  hauteur. Le pli traverse la zone imprimée, qu'aucun voile n'éclaircit : un seul filet
  disparaîtrait sur un dos noir ou sur un papier blanc.
- **Les repères sur le PDF.** Jamais : les prestataires les refusent.
