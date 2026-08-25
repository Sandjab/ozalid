# Le fond de la photo ne s'imprime plus sur le papier

Date : 2026-08-25
Statut : validé (brainstorming du 25/08)

## Objectif

Un envoi en « image écrite à la main » est presque toujours la photo d'un mot
tracé sur une feuille. Cette photo entre dans le `.ozalid` telle quelle et se
compose telle quelle : `image("Léa.jpg", width: 42%)`, sans le moindre
traitement. Son fond — le papier photographié — est donc **imprimé**.

Trois faits se tiennent, et c'est leur conjonction qui fait le défaut :

- **Le papier n'est pas dans le fichier.** `Papier` « ne change que l'épaisseur
  du dos, jamais la composition » ; l'intérieur ne pose aucun `fill` de page. Le
  crème de BoD ou de KDP est le papier physique du tirage. Rien dans le PDF ne
  le connaît, donc rien ne peut s'y accorder.
- **Le blanc d'une photo n'est pas du blanc.** Un blanc pur ne serait pas encré
  et laisserait voir le crème. Mais une feuille photographiée rend du 230-245
  teinté, avec un dégradé d'éclairage et du bruit — et cela, ça s'encre.
- **Rien ne le montre.** Le canevas et « Voir la page » rendent sur fond blanc :
  un rectangle blanc-photo sur fond blanc écran est invisible. Le défaut
  n'apparaît qu'au tirage, sur un exemplaire dédicacé, payé.

Cette spec rend le fond transparent avant que Typst ne voie l'image, et teinte
l'aperçu au papier réel pour que le réglage se juge à l'écran.

## Décisions de cadrage (brainstorming du 25/08)

- **L'original ne se perd jamais.** L'archive garde la photo telle qu'elle est
  entrée ; le détourage s'applique à la volée, au moment d'écrire l'image pour
  Typst. Un réglage se reprend six mois plus tard, et un `.ozalid` ancien
  profitera d'un algorithme amélioré sans qu'on redemande la photo à personne.
- **Le réglage vit sur l'envoi**, pas sur le livre : chaque photo a son
  éclairage. C'est la règle que la main a déjà suivie en descendant du livre
  vers l'exemplaire.
- **Un projet ancien garde son rendu.** `detourage` est un `Option`, absent des
  projets d'avant cette spec, et absent vaut « aucun détourage ». On ne change
  pas sous les pieds de quelqu'un le tirage qu'il a relu. Une photo posée après
  ce chantier, elle, naît détourée : c'est le cas d'usage.
- **Le papier se voit à l'écran.** Sans un fond teinté, le réglage serait
  aveugle — un fond résiduel gris pâle ne se distingue pas du blanc. La teinte
  est un fait d'écran : rien de la composition ne bouge.
- **`VERSION` ne bouge pas.** Un champ neuf qui reçoit un défaut ne casse pas la
  relecture d'un fichier ancien ; c'est la convention posée par les chantiers
  précédents.

## Ce qui est vérifié, et non supposé

Quatre hypothèses portaient la conception. Elles ont été éprouvées avant
d'écrire cette spec, avec le sidecar épinglé (Typst 0.15.1).

**Typst transporte l'alpha d'une image d'entrée jusqu'au PDF.** C'est
l'hypothèse dont tout dépend : si l'alpha était aplati, il n'y aurait pas de
chantier. Un PNG RGBA — moitié haute à alpha 0, moitié basse noire opaque —
composé sur une page `fill: rgb("#F7F0E0")` rend un PNG où le pixel du haut vaut
**exactement (247, 240, 224)**, la couleur de la page, et celui du bas (0, 0, 0).
Le PDF porte un **`/SMask` en `DeviceGray`** : la couche alpha y voyage comme
masque doux. Ni aplatissement, ni composition sur blanc.

**Une feuille photographiée s'encre.** Sur une photo d'essai portant un dégradé
d'éclairage, du bruit de capteur et une compression JPEG q92, les quatre coins
valent 230 à 244 — soit **4 à 13 % d'encre** là où l'auteur croit poser du
blanc. C'est ce rectangle-là qui s'imprime sur le crème.

**La démultiplication de la couleur est inutile ; le point noir est
nécessaire.** Quatre variantes mesurées sur cette photo, trait de stylo bleu
(28, 34, 120), fond résiduel relevé au coin le plus sombre :

| variante | fond résiduel | trait vu sur crème |
|---|---|---|
| un seuil, couleur démultipliée | 20/255 | (30, 29, 107) |
| un seuil, couleur conservée | 20/255 | (48, 51, 123) — délavé |
| deux seuils, couleur conservée | 24/255 | (28, 32, 105) — fidèle |
| deux seuils, papier abaissé à 228 | **4/255** | (28, 32, 105) |

La démultiplication complique le calcul pour un trait *moins* fidèle qu'un
point noir bien posé. Elle est écartée : la couleur du pixel est conservée
telle quelle, et seul l'alpha est calculé.

**Aucune estimation automatique ne tient seule.** Abaisser le seuil du papier de
244 à 228 fait passer le fond résiduel de 8 % à 1,6 % : le curseur sert. Et le
point noir ne s'estime pas de façon fiable, parce que la part encrée d'une image
varie du tout au tout — 4,7 % sur la photo d'essai, moins de 1 % pour une
signature, davantage pour un paragraphe. Sur cette image, le 1er percentile de
luminance vaut 38,8 (dans le trait, bon) et le 5e vaut 185,4 (déjà dans le
papier, faux). Un percentile fixe tombe dans le papier dès que le mot est court.
D'où deux réglages estimés à la pose **et** repris à la main.

**Le portage Rust rend les mêmes chiffres** (relevé le 25/08, après implémentation, sur
la même photo d'essai) : papier estimé 243,9 et encre 37,0, contre 243,9 et 37,1 au
prototype ; fond résiduel 22/255 au seuil estimé et 3/255 à 228, contre 24 et 4. Les
écarts tiennent aux pixels échantillonnés et aux arrondis. Le trait sort opaque, à
(28, 31, 110) — sa couleur photographiée, non retouchée. **Ce sont ces valeurs-là qui
font foi**, et c'est celles que le README cite.

**Réserve assumée.** La photo d'essai est synthétique : dégradé, bruit et JPEG
imités, pas une vraie photo de téléphone. Les ordres de grandeur sont crédibles,
les chiffres exacts ne valent que pour elle. Le premier relevé sur une photo
réelle est le premier pas du plan, et il peut déplacer les valeurs par défaut.

## 1. Le modèle

`Envoi` gagne un champ :

```rust
#[serde(default, skip_serializing_if = "Option::is_none")]
pub detourage: Option<Detourage>,
```

```rust
/// Les deux seuils qui séparent l'encre du papier, en luminance 0-255.
pub struct Detourage {
    /// Au-dessus, c'est le papier : alpha 0.
    pub papier: f64,
    /// En dessous, c'est l'encre pleine : alpha 1.
    pub encre: f64,
}
```

`None` — les projets d'avant cette spec — vaut « aucun détourage » : l'image se
compose comme aujourd'hui. `poser_image_envoi` pose `Some(Detourage)` estimé sur
l'image reçue : le papier au 95e percentile de luminance, l'encre au 0,5e.

`Detourage` n'a de sens que sous une main en image. Un envoi qui repasse en
police le garde sans l'employer : le perdre obligerait à le régler à nouveau
après un aller-retour, et ce n'est pas ce que « changer de main » veut dire.

**`VERSION` reste à 4.** Le champ est absent des fichiers anciens et reçoit son
défaut ; aucune migration n'est écrite.

## 2. Le détourage

Un module neuf, `detourage.rs`, à côté d'`image.rs` dont il partage la manière :
des octets en entrée, des octets en sortie, aucun accès disque, aucun état. Il
se teste sur des images fabriquées en mémoire.

```rust
pub fn applique(octets: &[u8], d: &Detourage) -> Result<Vec<u8>, String>
```

JPEG ou PNG en entrée, PNG RGBA en sortie. Pour chaque pixel :

```
L     = 0.2126 R + 0.7152 G + 0.0722 B
alpha = clamp((papier - L) / (papier - encre), 0, 1)
couleur = celle du pixel, inchangée
```

Le seuil binaire est écarté : il hache le trait en escalier. L'alpha continu
garde l'anti-aliasing de la photo, et c'est ce qui distingue une signature d'un
tampon. La couleur n'est pas retouchée — la mesure ci-dessus dit que la
démultiplication n'y gagne rien.

`papier <= encre` est refusé plutôt que borné : c'est un réglage impossible, et
le taire donnerait une image entièrement opaque sans qu'on sache pourquoi.

## 3. Où il s'applique

**Un seul point.** `package::trace` écrit l'image pour Typst
(`std::fs::write(dossier.join(fichier), octets)`), et c'est le même `trace`
qu'appellent la composition d'un package **et** `envoi_objet`, le rendu de
l'objet que le canevas manipule. Le détourage entre là, et l'aperçu ne peut pas
diverger du tirage — la promesse de l'étape, « ce qu'on déplace est ce qui
s'imprimera », est tenue par construction.

Les envois par diffusion sont couverts sans effort : `trace` traite
`Main::Image` et `Main::Diffusion` dans la même branche, et une écriture
générée sort elle aussi sur un fond.

Le nom du fichier écrit passe en `.png` quand le détourage s'applique : Typst
reconnaît le format d'une image **à son extension**, et un PNG rangé sous
`.jpg` ne se composerait pas. Le nom dans l'archive, lui, ne bouge pas —
`Envoi::image` continue de nommer l'original.

**Effet de bord à assumer.** `Quoi::Image { fichier: &'a str }` emprunte
aujourd'hui son nom à `Envoi::image`, et `Quoi` comme `Trace` sont `Copy`. Un
nom qui n'est plus celui de l'archive doit être possédé : `Cow<'a, str>` — qui
garde l'emprunt quand il n'y a pas de détourage — ou `String`. Dans les deux
cas `Quoi` et `Trace` perdent `Copy` et gardent `Clone`. Aucun appelant ne
paraît en dépendre, mais c'est à vérifier au premier pas du plan plutôt qu'à
découvrir en route. L'alternative — un nom fixe, un dossier par envoi — n'est
pas retenue sans avoir relevé que `trace` n'écrit bien qu'une image par
dossier.

## 4. L'écran

**Le papier se voit.** `Papier` gagne une teinte — `&'static str`, la notation
`#rrggbb` que le CSS attend, aucune conversion en chemin —, et le canevas la
peint : la
page de fond et l'objet se posent dessus en `mix-blend-mode: multiply`. L'encre
multiplie le papier — c'est la physique de l'impression —, si bien qu'un fond
résiduel paraît comme un rectangle terne sur le crème, et qu'un détourage franc
laisse le papier intact. Aucune source Typst n'est touchée : la teinte est du
CSS.

La teinte est une **convention d'Ozalid**, pas une mesure : aucun prestataire ne
publie la valeur de son crème. Elle est écrite comme telle dans `providers.rs`,
à côté des formules de dos qui, elles, sont relevées.

**Les deux réglages** prennent place dans la bande des réglages, sous le choix
de l'image, et ne paraissent que sous une main en image — la règle de l'étape :
ce que la main ne réclame pas ne paraît pas. Deux curseurs en luminance 0-255,
« Papier » et « Encre », avec leur valeur à côté.

Un mouvement de curseur ne rappelle pas Typst à chaque pixel parcouru : le
réglage est commis au relâchement, comme les gestes du canevas commettent leur
placement au dépôt.

## 5. La dépendance

```toml
image = { version = "0.25", default-features = false, features = ["jpeg", "png"] }
```

`image.rs` ne lit aujourd'hui que des en-têtes — signature PNG, segments JPEG —
et décoder un JPEG entier à la main n'est pas raisonnable. Les features sont
restreintes aux deux formats que l'application accepte, comme `zip` l'est à
`deflate` : c'est la neuvième dépendance du projet, et elle se justifie par le
seul fait qu'on ne réécrit pas un décodeur JPEG.

## 6. Ce qui se vérifie

Aucun test n'est écrit qui n'ait été **vu échouer** — TDD ou mutation ciblée.

**Rust.**

- Une image toute blanche détourée sort entièrement transparente ; une image
  toute noire sort entièrement opaque. Les deux bouts de la rampe.
- Un pixel à mi-chemin entre les deux seuils sort à alpha 128 ± 1 : c'est la
  rampe elle-même, et un seuil binaire ferait tomber ce test.
- La couleur d'un pixel encré est conservée à l'identique. C'est la décision du
  § 2, et rien d'autre ne la protège.
- `papier <= encre` est refusé, et le message dit lequel des deux corriger.
- Un JPEG et un PNG de même contenu donnent le même résultat : l'entrée est
  relevée sur le contenu, jamais sur le nom.
- `poser_image_envoi` pose un détourage estimé ; un envoi qui n'en avait pas
  après ouverture d'un projet ancien n'en reçoit pas d'office.
- `trace` écrit une image détourée sous un nom en `.png` quand le détourage
  s'applique, et l'original tel quel quand il vaut `None`.
- Le compte de pages ne change pas, détourage ou non. Typst lancé pour de vrai :
  c'est l'invariant qui tient le dos et la planche.

**Front** (`node --test`, via `dom_shim`).

- Les deux curseurs ne paraissent que sous une main en image.
- Bouger un curseur envoie `envoi_regler` une fois, au relâchement.
- Le canevas porte la teinte du papier du destinataire visé, et en change quand
  on vise un autre prestataire.

**Chaîne.** `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
`cargo test`, `node --test tests/*.test.js`, et `cargo run --example temoin` —
le compte de pages affiché, comparé au précédent sur le même manuscrit.

**Sur pièce.** Un relevé sur une vraie photo de téléphone, avant d'arrêter les
valeurs par défaut : c'est la réserve du § « Ce qui est vérifié ».

## Ce que cette spec ne fait pas

- **Aucune correction de l'éclairage inégal.** Les seuils sont globaux. Sur une
  photo dont un coin est nettement plus sombre, il reste du fond, ou le trait
  pâle s'efface — c'est le compromis que les deux curseurs donnent à arbitrer,
  et 1,6 % de fond subsistait au meilleur réglage de l'essai. La normalisation
  locale attendra qu'on ait vu le défaut sur une photo réelle.
- **Aucun détourage des images de couverture.** La photo d'une 1ère de
  couverture est une illustration, pas de l'encre sur du papier ; elle n'a
  aucune raison de perdre son fond.
- **Aucune retouche.** Ni contraste, ni rotation, ni recadrage, ni suppression
  de poussière. L'application sépare l'encre du papier, elle ne devient pas un
  éditeur d'images.
- **Aucune teinte de papier dans le PDF.** Le crème reste le papier physique :
  poser un fond crème dans le fichier ferait imprimer un aplat sur toutes les
  pages, ce qui est exactement l'erreur qu'on corrige ici.
