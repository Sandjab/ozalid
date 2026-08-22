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
/// Ce qu'un aperçu de face donne à voir : l'image, et où la couper s'il y a lieu.
#[derive(Serialize)]
pub struct Apercu {
    pub image: String,
    /// Absente sur les faces qui se composent sans fond perdu.
    pub coupe: Option<Coupe>,
}

/// La part du fond perdu sur chaque dimension de la planche, en fraction de celle-ci.
/// Les deux diffèrent : une planche est bien plus large que haute.
#[derive(Serialize)]
pub struct Coupe {
    pub x: f64,
    pub y: f64,
}
```

`coupe` est `None` pour la 1ère, la 4ème et le dos : ces trois faces se composent au
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

Le cadre reprend donc les contraintes qui dimensionnent l'image aujourd'hui
(`max-width`, `max-height`, et le `width: 100%` que la face Dos impose), sans les
rompre : c'est le point délicat de ce chantier, et il se vérifie à l'œil sur les quatre
faces.

### Un seul élément pour les deux effets

Un enfant du cadre, `.coupe`, posé sur toute l'image, en `pointer-events: none` :

- **le voile** est fait de quatre fonds — haut, bas, gauche, droite — dimensionnés en
  pourcentage depuis les deux fractions. Pas d'ombre étalée sous un `overflow: hidden`,
  qui aurait été plus courte à écrire : ce découpage emporterait l'ombre portée de
  l'aperçu, et la couverture paraîtrait posée à plat. Les bandes latérales s'arrêtent
  au-dessus et au-dessous des horizontales, sans quoi deux voiles se superposeraient
  aux quatre coins — et ces coins-là sont justement ce qu'on regarde.
- **la ligne de coupe** est un `::after` en `inset` sur le rectangle rogné, bordé d'un
  `1px dashed`. En pseudo-élément : elle n'a rien à dire au balisage.

Les deux variables sont posées par le JS depuis la `coupe` reçue. La visibilité de
`.coupe` tient à un `hidden` : pas de `coupe`, ou bascule éteinte, il disparaît.

### La bascule

Un bouton « Fond perdu » dans la barre `.outils`, à deux états `aria-pressed` — le
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

JS :

- Après un aperçu de la face Planche, `--coupe-x` et `--coupe-y` sont posées sur le
  cadre et `.coupe` est visible.
- La bascule éteinte, `.coupe` est masqué **sans** nouvel appel à `couverture_apercu` :
  c'est tout l'intérêt d'habiller plutôt que de recomposer.
- Sur la face 1ère, où la commande ne rend pas de `coupe`, `.coupe` est masqué.
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
  choses tombent. Autre chantier.
- **Le pli du dos.** Deux traits de plus, sur une planche qui montre déjà ses trois
  zones par leurs fonds. À revoir si le besoin se présente.
- **Les repères sur le PDF.** Jamais : les prestataires les refusent.
