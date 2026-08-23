# L'identité du livre — champs clés, champs dérivés, substitution

Date : 2026-08-23
Statut : validé (brainstorming)

## Objectif

L'identité d'un livre est aujourd'hui écrite à deux endroits qui s'ignorent. `Livre`
porte le titre, l'auteur, le genre, le copyright et la dédicace ; `Couverture` porte
l'éditeur, le monogramme, la collection, le prix et la mention. Rien ne dit pourquoi
la frontière passe là, et elle se paie deux fois : l'éditeur saisi dans le pied de la
1ère est relu par le dos (`planche.rs:155`), dont le commentaire avoue la dépendance
de travers ; la pastille de la maquette Folio porte `"folio"`, c'est-à-dire un nom de
collection saisi à un endroit où rien ne le nomme ainsi.

Cette spec range l'identité du livre dans `Livre`, en deux catégories :

- les **champs clés**, littéraux, qui nomment le livre et sa maison ;
- les **champs libres**, qui peuvent citer les clés par des jetons `%CLE%` et se
  résolvent à la composition.

Elle ne traite pas des maquettes en fichiers — le chantier suivant, qui en dépend :
ce qu'une maquette emporte ne peut se décider qu'une fois su ce qui l'a quittée.

## Décisions de cadrage (brainstorming du 23/08)

- **Deux chantiers, dans cet ordre.** L'identité d'abord, les maquettes ensuite. Une
  maquette enregistrée avant ce chantier porterait l'éditeur, la collection et le
  monogramme, et il faudrait migrer des fichiers qui viennent d'être créés.
- **Six clés** : Titre, Auteur, Genre, Éditeur, Collection, Monogramme.
- **Cinq libres** : Titre de la page de titre, Dédicace, Copyright, Prix, Mention.
- **La pastille perd son texte propre** et affiche la Collection : c'en était le
  doublon déguisé.
- **Le résumé de 4ème reste dans la maquette** — c'est une zone de mise en page, avec
  son style, son cadre et son voile — **mais il reconnaît les jetons**. C'est le seul
  endroit où la substitution sert les maquettes et non le seul livre courant.
- **Les valeurs par défaut sont de vraies valeurs**, reçues par le Rust et composées
  partout où la maquette les montre. La Dédicace fait exception et naît vide : elle
  n'a pas d'interrupteur, et coûterait deux pages à tout nouveau livre.
- **L'année du copyright est figée à la création**, pas un jeton. Un `%ANNEE%` résolu
  à chaque composition ferait dire 2028 au copyright d'un livre déposé en 2026, et le
  dépôt légal ne se rattrape pas.
- **`VERSION` passe à 3.** Ce n'est pas une section facultative de plus : des champs
  changent de place, et un binaire actuel ne saura pas lire un projet écrit après.
- **Les cinq `.ozalid` existants sont migrés à la lecture**, sans resaisie.

## 1. Les champs du livre

```rust
pub struct Livre {
    // Clés — littérales, jamais substituées.
    pub titre: String,       // « Titre »
    pub auteur: String,      // « Auteur »
    pub genre: String,       // « Genre »
    pub editeur: String,     // « Editeur »
    pub collection: String,  // « Collection »
    pub monogramme: String,  // « Monogramme »

    // Libres — substituées à la composition.
    pub titre_page: String,  // « %TITRE% »
    pub dedicace: String,    // vide ; « Dédicace » en indication grisée
    pub copyright: String,   // voir ci-dessous
    pub prix: String,        // « Prix »
    pub mention: String,     // « Mention »

    // Ni clé ni libre : contrôle d'intégrité du manuscrit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chapitres: Option<u32>,
}
```

Le copyright par défaut, sur trois lignes, avec l'année du jour de la création :

```
© %AUTEUR%, 2026.
Tous droits réservés.
Maquette de couverture : atelier Ozalid
```

`titre_page` cesse d'être un `Option`. Le repli « vide : le titre ci-dessus » devient
le jeton `%TITRE%`, qui dit la même chose en la montrant — et qui se retouche, ce que
le repli ne permettait pas. `Livre::titre_page()` ne fait donc plus de repli mais une
substitution.

`dedicace` cesse de l'être aussi : une chaîne vide suffit, et `dedicace()` filtrait
déjà le blanc. Cette méthode reste, avec son rognage — une dédicace réduite à une
espace ne doit toujours pas coûter deux pages.

Le défaut de `genre` passe de `"roman"` à `"Genre"`. Les projets existants gardent le
leur ; seuls les nouveaux naissent génériques.

**Pourquoi de vraies valeurs et non des indications grisées.** Un champ vide donne un
aperçu de couverture blanc et, au chantier suivant, une maquette enregistrée sans
aucun texte — l'inverse de la maquette lisible qu'on cherche. Une valeur générique ne
prétend rien : elle montre la maquette telle qu'elle est, et la maquette décide seule
de ce qui paraît. Dans Folio, `pied.actif` vaut `false` : l'Éditeur et le Monogramme
existent et ne se voient nulle part sur la 1ère. Dans Blanche, ils paraissent. Sans
rien resaisir.

La Dédicace échappe à cette règle parce qu'elle échappe aux interrupteurs :
`interieur.rs:411` compose une belle page et sa blanche dès que le texte n'est pas
vide. C'est le seul défaut qui déplacerait la pagination, donc le dos, sur tout
nouveau projet — et rien à l'écran n'attribuerait ces deux pages à un choix que
personne n'a fait.

## 2. Ce que la couverture perd

Quittent `Couverture` :

| Champ | Devient |
|---|---|
| `pied.editeur` | `livre.editeur` |
| `pied.monogramme` | `livre.monogramme` |
| `quatrieme.collection` | `livre.collection` |
| `quatrieme.prix` | `livre.prix` |
| `quatrieme.mention` | `livre.mention` |
| `pastille.texte` | `livre.collection` |

Les styles restent : `style_mono`, `style_editeur`, `style_pied` et le style de la
pastille sont de la mise en page, et c'est bien à la maquette d'en décider.

Restent aussi tous les interrupteurs — `pied.actif`, `pastille.actif`,
`genre_visible`, `quatrieme.pied_actif`, `dos.*.actif` — qui gouvernent ce qui paraît.
La séparation est celle-là, et elle se dit en une phrase : **le livre dit ce qui est
écrit, la maquette dit où et si ça se voit.**

Il reste **un seul texte** dans `Couverture` : `quatrieme.texte`, le résumé.

`planche.rs:155` lira `livre.editeur.trim()` au lieu de `cv.pied.editeur.trim()`. Le
commentaire de `Dos` — « il ne porte aucun texte propre : l'auteur et le titre viennent
du livre, l'éditeur du pied de la 1ère » — devient enfin vrai de bout en bout.

`couverture.rs:1117` compose les trois lignes du pied de 4ème depuis
`[&q.mention, &q.collection, &q.prix]` : elles viendront du livre, la collection
littérale, la mention et le prix substitués.

## 3. La substitution

Un module `gabarit.rs`, une fonction :

```rust
/// Remplace les jetons %CLE% par la valeur du champ clé correspondant.
pub fn substituer(texte: &str, livre: &Livre) -> String
```

Six jetons : `%TITRE%`, `%AUTEUR%`, `%GENRE%`, `%EDITEUR%`, `%COLLECTION%`,
`%MONOGRAMME%`.

**Une seule passe, sans récursion.** Les clés étant littérales par définition, rien
ne peut se substituer en cascade — et c'est ce qui rend la boucle infinie impossible
sans avoir à s'en garder.

**Un jeton inconnu est laissé tel quel.** `%TITER%` s'imprime avec ses pour-cent, se
voit dans l'aperçu, se voit sur l'épreuve. Aucun signalement n'est ajouté : le repli
de police est signalé parce qu'il est *muet*, une faute de frappe dans un jeton ne
l'est pas.

Elle s'applique aux cinq champs libres et au résumé de 4ème, par des accesseurs qui
suivent le motif déjà en place — le champ `titre_page` et la méthode `titre_page()`
coexistent aujourd'hui :

```rust
impl Livre {
    pub fn titre_page(&self) -> String
    pub fn copyright(&self) -> String
    pub fn dedicace(&self) -> Option<String>
    pub fn prix(&self) -> String
    pub fn mention(&self) -> String
}
```

Le résumé de 4ème est substitué au point où `couverture.rs` le compose, pas dans la
maquette : le `.ozalid` conserve le texte à jetons, qui doit rester tel quel pour
suivre le livre si le titre change.

## 4. L'interface

L'onglet Livre en deux groupes titrés :

```
Champs clés
  Titre        Auteur        Genre
  Éditeur      Collection    Monogramme

Champs libres — %TITRE% %AUTEUR% %GENRE% %EDITEUR% %COLLECTION% %MONOGRAMME%
  Titre de la page de titre
  Dédicace
  Copyright
  Prix
  Mention

Chapitres attendus     (contrôle d'intégrité, à part)
```

La liste des jetons est écrite **une seule fois**, dans l'aide du second groupe, et
tirée d'une commande plutôt que recopiée dans le HTML — sinon elle mentira le jour où
une clé s'ajoutera.

Le schéma de `couverture.js` perd six champs : `pied.monogramme`, `pied.editeur`,
`quatrieme.mention`, `quatrieme.collection`, `quatrieme.prix`, `pastille.texte`.

## 5. La migration en version 3

Dans `Projet::lire`, avant la désérialisation typée, une passe sur le `toml::Value`
quand `ozalid.version < 3` :

| Source v2 | Destination v3 |
|---|---|
| `couverture.maquette.pied.editeur` | `livre.editeur` |
| `couverture.maquette.pied.monogramme` | `livre.monogramme` |
| `couverture.maquette.quatrieme.collection`, à défaut `couverture.maquette.pastille.texte` | `livre.collection` |
| `couverture.maquette.quatrieme.prix` | `livre.prix` |
| `couverture.maquette.quatrieme.mention` | `livre.mention` |

Un champ vide côté v2 prend la valeur générique. Les clés migrées sont ensuite
retirées de la maquette, pour qu'aucun `.ozalid` réécrit ne conserve deux vérités.

`titre_page` n'a pas besoin de cette passe : il passe d'`Option<String>` à `String`
avec `#[serde(default = "titre_page_defaut")]` valant `"%TITRE%"`. Un v2 qui le
porte le garde tel quel, un v2 qui ne le porte pas reçoit le jeton — c'est-à-dire
exactement l'ancien repli, rendu visible et retouchable. C'est aussi ce qui permet de
livrer la substitution avant que la version ne monte.

**Le repli par la pastille est délibéré et borné** : la collection explicite gagne
toujours, la pastille ne sert que si elle est vide. Dans Folio, elle porte `"folio"`,
et le laisser tomber ferait perdre la seule chose que ce champ disait.

C'est la première vraie migration du format. La v1 n'en avait pas eu besoin :
`[couverture]` étant `#[serde(default)]`, un projet v1 s'ouvrait par défaut, et la
version 2 a été posée avant toute distribution.

## 6. Risques

**L'accès direct au champ brut.** Le motif `copyright` / `copyright()` est discret :
rien n'empêche un futur appelant de lire le champ et d'envoyer `%AUTEUR%` à
l'impression, sans que rien ne proteste. La parade n'est pas dans le type mais dans
les tests, posés **aux points de sortie** plutôt que sur `substituer` : ils cassent le
jour où quelqu'un branche une nouvelle sortie en oubliant la substitution.

**Le compte de pages.** Rien dans ce chantier ne doit le déplacer — c'est justement
pourquoi la Dédicace naît vide. Le témoin est la garde.

**La v3 ferme la porte derrière elle.** Un projet écrit après ce chantier ne s'ouvrira
plus avec un binaire antérieur. C'est assumé et c'est la raison pour laquelle la
version monte : la laisser à 2 ferait échouer la lecture d'un champ manquant au lieu
de la refuser proprement.

## 7. Vérification

### Le témoin

`cargo run --example temoin` : le compte de pages est relevé **avant** le premier lot
et doit être identique après chacun. Un écart d'une seule page signifie qu'un défaut
générique a atteint l'intérieur.

### Ce que les tests doivent tenir

- Chaque jeton se substitue, dans chacun des six champs qui les reconnaissent.
- Un jeton inconnu traverse intact.
- Aucune cascade : un `%TITRE%` tapé dans le champ Titre ressort littéral.
- **Un test par point de sortie** — intérieur, couverture, planche, épreuve, ebook —
  vérifiant qu'aucun jeton connu ne survit à la composition.
- La dédicace vide ou blanche ne compose rien (test existant, conservé).
- Un nouveau livre porte un copyright daté de l'année en cours.
- Migration : un v2 complet remonte les cinq textes ; un v2 sans
  `quatrieme.collection` prend la pastille ; un v2 aux champs vides prend les
  génériques ; les clés ne restent pas dans la maquette réécrite.
- Aller-retour v3 complet.
- `contrats.test.js` : les six contrôles retirés de l'onglet Couverture, les onze
  champs présents dans l'onglet Livre, la liste des jetons servie par le Rust.

Chaque test doit être vu échouer — TDD, ou mutation ciblée sur un test rétroactif.

### À l'œil

Un projet neuf ouvert sur chacune des trois maquettes, pour vérifier que les valeurs
génériques paraissent là où la maquette les montre et nulle part ailleurs. Puis
« Les Heures creuses.ozalid » ouvert et comparé à son état actuel, champ par champ.

## 8. Les lots

Chaque lot laisse l'application juste — aucun ne coupe l'interface en attendant le
suivant, et un seul touche au format.

**Lot 1 — La substitution.** `gabarit.rs` et les trois jetons des clés qui existent
déjà : `%TITRE%`, `%AUTEUR%`, `%GENRE%`. Les accesseurs de `titre_page`, `dedicace` et
`copyright`. `titre_page` passe en `String` avec le défaut `"%TITRE%"`, ce qui préserve
l'ancien repli en le rendant visible. Les tests aux points de sortie sont posés ici,
avant qu'il n'y ait cinq champs de plus à oublier. **`VERSION` ne bouge pas** : aucun
champ n'a changé de place.

**Lot 2 — Les clés montent.** `editeur`, `monogramme`, `collection`, `prix` et
`mention` quittent `Couverture` pour `Livre` ; la pastille affiche la Collection ;
`planche.rs` lit le livre ; les trois jetons manquants s'ajoutent ; les contrôles
changent d'onglet. La migration v2→v3 et **la seule montée de version** sont ici.

**Lot 3 — L'onglet Livre en deux groupes**, l'aide sur les jetons servie par une
commande, et les valeurs par défaut génériques de `Livre::vide()` — Dédicace exceptée,
qui naît vide.

Le chantier suivant — les maquettes en fichiers, fournies contre personnalisées, avec
images et cadrage — aura sa propre spec.
