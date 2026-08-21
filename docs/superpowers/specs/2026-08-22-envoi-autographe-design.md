# L'envoi autographe

Date : 2026-08-22
Statut : validé (brainstorming)

## Objectif

Un auteur auto-édité n'a pas ses livres sous la main : le prestataire imprime et
expédie. Il ne peut donc pas faire ce que fait n'importe quel auteur en séance —
écrire un mot à la personne qui reçoit l'exemplaire.

Cette spec fait entrer l'envoi autographe dans la chaîne : un mot adressé à une
personne, d'une écriture manuscrite, imprimé dans **son** exemplaire. Chaque envoi
donne son propre package, prêt à commander à l'unité.

La **spec 2a**, livrée le 21/08, traite l'autre chose que l'usage courant appelle
« dédicace » : la page liminaire imprimée dans tous les exemplaires. Les deux
coexistent sans se connaître — un livre peut porter les deux, l'une en page 5, l'autre
en page 3.

## Décisions de cadrage (brainstorming du 22/08)

- **Un package complet par envoi**, planche comprise. La planche est identique dans
  chaque répertoire : c'est le prix assumé pour qu'aucun fichier ne parte au mauvais
  dédicataire.
- **L'envoi surcharge la page de titre**, il n'ajoute aucune page.
- **Le livre fixe sa main, l'envoi apporte son contenu.** Quatre sources, trois formes.
- **Les envois se composent chez le prestataire visé**, par un geste distinct de
  « Générer les packages ».
- **Convergence unique** : la pagination ne peut pas bouger d'un envoi à l'autre, donc
  on ne la cherche qu'une fois.
- **La clé d'API vit dans les préférences**, jamais dans le `.ozalid`.
- **Une image générée est figée dès qu'elle est acceptée.** Composer ne rappelle
  jamais le réseau.

## 1. Le vocabulaire

« Destinataire » est **pris** : depuis le lot 3 du chantier précédent, il désigne le
prestataire chez qui l'on livre. La personne à qui l'on dédicace est le
**dédicataire** — le mot de l'édition, et il ne collisionne avec rien.

Un **envoi** est ce qu'on écrit à un dédicataire. Une **main** est ce qui l'écrit.

## 2. Le modèle

### La main du livre

```rust
/// D'où vient l'écriture des envois de ce livre.
pub enum Main {
    /// Police manuscrite : embarquée avec l'application, ou fournie par l'auteur et
    /// embarquée dans le `.ozalid`. Une seule variante pour ces deux sources : seule
    /// la provenance du fichier diffère, la composition est la même.
    Police { police: String },
    /// Une image écrite à la main, une par envoi.
    Image,
    /// Une image par envoi, produite par un modèle de diffusion.
    Diffusion { gabarit: String },
}
```

`gabarit` est le prompt du livre, dans lequel le contenu de chaque envoi s'insère.

### Un envoi

```rust
pub struct Envoi {
    pub dedicataire: String,
    /// Ce que la main réclame : un texte à composer, ou un prompt. Vide en mode Image.
    pub contenu: String,
    /// Image figée dans l'archive, sous `envois/` — fournie, ou générée puis acceptée.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
}
```

Deux champs pour trois formes plutôt qu'un champ par forme : la ligne d'envoi change de
nature avec la main, elle ne s'encombre pas de ce que la main ne réclame pas.

### Dans le projet

`Metadonnees` reçoit une section facultative :

```rust
#[derive(Default)]
pub struct Envois {
    pub main: Main,
    pub liste: Vec<Envoi>,
}
```

`Main::default()` est `Police { police: <la première manuscrite embarquée> }` : un
livre neuf sait écrire sans qu'on lui règle quoi que ce soit, comme il sait déjà
composer son intérieur en EB Garamond.

`#[serde(default)]`, donc **`VERSION` ne bouge pas** : un `.ozalid` écrit avant cette
spec s'ouvre sans un mot, avec une liste vide — le même arbitrage que `[livraison]` et
que la dédicace imprimée.

### Où vivent les images

Dans l'archive, sous `envois/`, **et pas dans `projet.images`**.

Ce n'est pas une préférence d'organisation. `package::ecrire_images` classe les images
du projet par leur nom, et sa règle est binaire : tout ce qui ne commence pas par
`quatrieme` **devient la première de couverture**. Une image d'envoi versée dans ce
tas remplacerait donc la couverture, en silence. La règle de nom est déjà fragile à
deux rôles ; l'étendre à un troisième la rendrait dangereuse.

La police personnelle suit la même logique, sous `polices/` : le `.ozalid` est
auto-portant, un projet qui compose avec la main de son auteur doit pouvoir le faire
sur une autre machine.

## 3. La composition

### Ce qui a été constaté

La page de titre porte son contenu dans le tiers supérieur — auteur, titre, genre —
et laisse les deux tiers du bas vides. C'est là qu'un auteur écrit, et c'est la seule
page liminaire dont le blanc est à la fois large et attendu.

### Ce qui est décidé

`interieur::source` prend l'envoi en paramètre, et `liminaires` pose sur la page de
titre un `#place` — le dispositif du pavé de copyright, repris tel quel :

```typst
#place(bottom + center, dy: -28mm, block(width: 70%)[…])
```

En mode `Police`, le bloc porte le texte composé dans la police de la main. En mode
`Image` et `Diffusion`, il porte `image("envois/…", width: 70%)`.

**`#place` ne consomme pas le flux.** Il lui est donc *impossible* de créer une page :
ce n'est pas une précaution qu'on prend, c'est une propriété du mécanisme. C'est ce qui
garantit que la pagination, le dos et la planche sont les mêmes pour tous les envois,
sans qu'on ait à les recalculer ni à croiser les doigts.

Le seul risque résiduel est un envoi trop long qui déborde de la page. Il se voit à
l'œil et ne se voit pas dans le compte de pages — d'où l'aperçu du § 5.

Le texte passe par `echappe()`, comme partout ailleurs.

## 4. La génération

Un bouton **« Générer les envois »**, distinct de « Générer les packages ». Deux
gestes séparés parce qu'ils ne servent pas le même moment : l'un prépare le tirage,
l'autre prépare des cadeaux.

Sorties : `envois/<dédicataire assaini>/`, à côté du `.ozalid`, avec l'intérieur, la
planche et sa vignette — les mêmes noms de fichiers que les packages, portant la clé du
prestataire.

**L'assainissement du nom de répertoire n'est pas cosmétique** : un dédicataire nommé
« Marie D./Léa » ou « .. » ne doit ni créer deux niveaux, ni sortir du dossier du
projet. Tout ce qui n'est ni lettre, ni chiffre, ni espace, ni tiret devient un tiret ;
deux dédicataires qui se réduisent au même nom sont suffixés.

### La convergence unique

L'ordre est :

1. Converger **une fois**, sans envoi, pour trouver gouttière et parité.
2. Calculer le dos, **une fois**.
3. Composer la planche, **une fois**.
4. Pour chaque envoi : compiler l'intérieur avec le réglage figé, copier la planche et
   sa vignette dans le répertoire de l'envoi.

Sur un livre de deux cent soixante pages, c'est la différence entre une trentaine de
secondes par envoi et une trentaine de secondes en tout. Mais l'économie n'est pas la
raison principale : converger une fois **exprime** la promesse de la surcharge, là où
converger M fois laisserait croire que le résultat pourrait différer.

## 5. L'aperçu

L'étape Livraison montre la page de titre de l'envoi sélectionné, rendue en PNG par
Typst — le mécanisme de la vignette de planche, à l'identique.

C'est la seule façon de voir qu'un envoi déborde, qu'une image générée est illisible,
ou qu'une police manuscrite rend mal une apostrophe. **Ce qu'on regarde est ce qui part
à l'impression**, pas une approximation qu'on espère fidèle : c'est la règle que la
vignette de planche a posée, et elle vaut ici pour la même raison.

## 6. La diffusion

### Ce qui vit où

L'**URL** et la **clé** vont dans `preferences.toml` : elles appartiennent à la
machine, pas au livre. Un `.ozalid` est fait pour être ouvert ailleurs — y écrire une
clé la publierait au premier partage. Le livre ne porte que le gabarit de prompt.

La clé est en clair dans `preferences.toml`, avec les permissions du fichier. C'est un
choix, pas un oubli : le trousseau du système réclamerait une dépendance par
plateforme. En conséquence, **la clé ne doit apparaître dans aucun message d'erreur,
aucun log, aucune vue rendue au front** — la remontée d'erreur de `typst.rs` remonte le
message entier du processus, ce qui serait ici exactement ce qu'il ne faut pas faire.

### Le contrat

`POST <url>`, corps JSON `{"prompt": …}`, en-tête `Authorization: Bearer <clé>`.
Réponse lue dans `data[0].b64_json`, à défaut `data[0].url` — le format le plus
répandu. Une dépendance HTTP est nécessaire : `ureq`, synchrone et léger. C'est la
première dépendance réseau du projet, et elle n'est tirée que par ce lot.

### Le client est injecté

Comme `converge` reçoit sa mesure et comme Typst est injecté : la génération d'image
prend une closure. Les tests tournent **sans réseau**, pour la raison même qui a fait
injecter le reste — une logique qu'on ne peut éprouver qu'en ligne n'est pas éprouvée.

### Un mot sur ce que rend un modèle de diffusion

Les modèles de diffusion rendent mal le texte écrit ; seuls les plus récents s'en
sortent. Un envoi généré a donc toutes les chances d'être illisible au premier essai.
C'est la raison de l'aperçu et du figeage : on regarde, on regénère si besoin, et **on
accepte** — après quoi l'image ne bouge plus.

## 7. Vérification

### Le témoin

`cargo run --example temoin` reste à **98 pages**, avec envoi comme sans. C'est la
garde centrale du chantier : si le compte bouge d'une seule page, la promesse est
fausse et tous les packages d'envoi sont faux avec elle.

### Ce que les tests doivent tenir

- Deux envois différents sur le même livre : **même compte de pages, même dos**.
- La source **hors page de titre** est identique à l'octet près, avec ou sans envoi.
- Une image acceptée, puis une composition **réseau coupé** : le package se refait.
- Le nom de répertoire est assaini — « Marie D./Léa », « .. », un nom vide, deux
  dédicataires homonymes.
- La clé d'API n'apparaît dans aucune erreur remontée.
- Un `.ozalid` sans section `envois` s'ouvre, et son round-trip conserve la liste.

Chaque test doit être vu échouer sur une mutation ciblée.

### À l'œil

Un envoi composé dans chacune des trois formes, la page de titre regardée à l'aperçu
et dans le PDF, et un envoi volontairement trop long — pour voir ce que déborder veut
dire, et juger si la largeur de 70 % et le `dy` de 28 mm tiennent.

## 8. Les lots

1. **Le mécanisme.** Envois, main `Police`, polices manuscrites embarquées, surcharge
   de la page de titre, génération, aperçu, assainissement des noms. Utilisable seul.

   **Le choix des polices manuscrites est un travail du lot, pas un détail.** Deux ou
   trois suffisent, sous licence OFL et redistribuables comme les vingt-neuf autres.
   Le critère qui les élimine est le même que celui qui a fait retenir l'astérisque
   pour la marque de rupture de scène : **chaque police doit porter les accents
   français** — `À`, `É`, `ç`, l'apostrophe courbe — et cela se relève sur le fichier,
   pas sur la fiche du fondeur. Une police manuscrite anglo-saxonne qui ignore `À` ne
   le dirait pas : Typst composerait par repli, sans un mot, et l'envoi partirait chez
   le dédicataire dans deux écritures différentes.
2. **La police personnelle**, embarquée dans le `.ozalid` sous `polices/`.
3. **L'image fournie** : `Main::Image`, une image par envoi, stockée sous `envois/`.
4. **L'image générée** : préférences, `ureq`, client injecté, aperçu, figeage.

Le témoin se rejoue à la fin de chaque lot, pas seulement à la fin du chantier.

## 9. Une dette à traiter en chemin

`app.js` fait **1210 lignes**, au-dessus de la bande de vigilance 1000-1100, et
l'étape Livraison va recevoir une seconde liste.

Le lot 1 sort cette étape dans un `app/src/livraison.js`. Il y a un précédent dans la
maison — `couverture.js` — et `index.html` charge des `<script>` classiques, donc ni
bundler ni module à introduire. La coupe bute toujours sur les trois `let` partagés,
mais l'étape Livraison n'en touche qu'un, `projet`.

Ce n'est pas un refactor d'occasion : c'est le fichier qu'on s'apprête à faire grossir.

## 10. Hors périmètre

- **La dédicace imprimée** : spec 2a, livrée. Les deux se composent sans se connaître.
- **L'épreuve de relecture** ne porte pas d'envoi : elle ne porte aucun liminaire.
- **`outils/`** : archive, acté au chantier précédent.
- **La commande chez le prestataire** : l'application produit des fichiers, elle ne
  passe aucune commande et n'ouvre aucun compte.
- **Une main par envoi** : écartée au cadrage. Tous les exemplaires d'un livre
  partagent la même écriture, comme dans la réalité.
- **Les dettes de `NOTES.md`** : sans rapport.
