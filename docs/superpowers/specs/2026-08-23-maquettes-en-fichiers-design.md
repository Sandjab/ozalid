# Les maquettes en fichiers — fournies, personnalisées, clonables

Date : 2026-08-23
Statut : validé (brainstorming)

## Objectif

Trois maquettes de couverture existent, écrites en dur dans `maquettes.rs` : Folio,
Blanche, Surimpression. Ce sont les seuls points de départ possibles. On ne peut ni en
ajouter, ni retoucher l'une d'elles et garder le résultat, ni repartir de la couverture
qu'on vient de régler pour le livre suivant. Régler une couverture, c'est du travail —
et ce travail meurt avec le `.ozalid` qui le porte.

Cette spec fait des maquettes des **fichiers** : trois **fournies**, livrées avec
l'exécutable et immuables ; autant de **personnalisées** qu'on veut, créées depuis le
projet ouvert, renommables et effaçables. Toute maquette, fournie ou non, se **clone**.

Elle vient après le chantier de l'identité du livre (spec du même jour) et en dépend :
ce qu'une maquette emporte ne pouvait se décider qu'une fois su ce qui l'a quittée.
Depuis, `Couverture` ne porte plus qu'un seul texte — le résumé de 4ème.

## Décisions de cadrage (brainstorming du 23/08)

- **Fournies embarquées par `include_bytes!`.** Des fichiers dans le dépôt,
  incorporés au binaire à la compilation : l'immuabilité est un fait, pas une règle
  applicative — il n'y a aucun fichier à protéger sur le poste. Aucune résolution de
  chemin, aucun mode dégradé, aucun écart entre développement et livraison. C'est
  précisément le piège connu de `fonts/`, où `target/debug` ne suit pas les sources.
- **Personnalisées dans `<config>/maquettes/`**, à côté de `preferences.toml` : ce qui
  appartient à la machine, non au livre. Un `.ozalid` reste auto-portant, sa couverture
  étant dans l'archive ; une maquette n'est qu'un point de départ.
- **Le nom est l'identité.** Un slug en dérive et nomme le fichier. Unicité imposée sur
  tout l'ensemble, fournies comprises : une personnalisée ne peut pas s'appeler
  « Folio », et cloner Folio propose « Folio (copie) ».
- **Édition depuis le projet ouvert.** Aucun éditeur nouveau : le panneau de réglages
  reste le seul endroit où l'on dessine une couverture, sur un vrai livre et un vrai
  format. Renommer, effacer et cloner sont des gestes sur des noms.
- **Un `<dialog>` « Maquettes… »**, ouvert par un bouton de la barre. Le `<select>`
  existant ne change pas de nature — un geste, pas un état — et gagne les
  personnalisées sous un séparateur.
- **Instantané fidèle : la maquette emporte tout**, cadrage et images compris. La
  discipline — préférer des images génériques, cohérentes avec la maquette, comme on
  préfère « Titre » à un vrai titre — appartient à l'utilisateur, pas au code.

## 1. Le format

Une maquette est une **archive**, comme le `.ozalid` : elle porte des images, elle ne
peut donc pas être un TOML seul.

```
maquette.toml   le nom affiché, et la couverture entière
images/         couverture.ext et quatrieme.ext, quand la maquette en porte
```

`maquette.toml` :

```toml
nom = "Ma collection"

[couverture]
mode = "typo"
papier = "#fcf0d8"
…
```

Extension `.maquette`. Dézippée, elle reste lisible et diffable — même promesse que le
`.ozalid`.

**Pas de champ `version`.** La convention du projet est déjà établie : `Couverture.dos`
a été ajouté avec `#[serde(default = "dos_defaut")]` pour ne pas refuser les projets
antérieurs, et tout futur champ suivra la même règle. Une maquette écrite par une
version antérieure se relira donc, et une maquette illisible est **ignorée** — élaguer
plutôt que refuser, comme les projets récents et les destinataires disparus.

Les fournies sont les mêmes archives, incorporées au binaire par `include_bytes!`.

## 2. Ce qu'une maquette emporte

`Couverture` entière, telle qu'elle est à l'écran : les modes, le cadre, les styles, la
pastille, le dos, le voile, **le cadrage** et **le résumé de 4ème**. Plus les images de
1ère et de 4ème, copiées dans l'archive.

Rien n'est retiré, rien n'est filtré. Ce qui a été décidé au chantier précédent tient :
l'éditeur, la collection, le monogramme, le prix et la mention sont au **livre** et ne
peuvent donc pas entrer dans une maquette. Le résumé de 4ème, lui, y reste — et il
reconnaît les jetons, ce qui permet à une maquette de porter une 4ème générique du genre
`%TITRE%, un %GENRE% de %AUTEUR%.` qui se résout pour chaque livre où on la charge.

**La discipline est à l'utilisateur.** Une maquette enregistrée depuis un livre réel
emportera son résumé, ses photos et le cadrage réglé sur elles. C'est voulu : filtrer
demanderait au code de deviner ce qui est générique, et il devinerait mal. La bonne
pratique — des images neutres, un résumé en jetons — se documente, elle ne se contraint
pas.

## 3. Charger une maquette

`maquette_choisir` remplace la maquette **et les images** du projet. C'est la
conséquence directe de l'instantané fidèle : une maquette qui porte une photo la pose,
une maquette qui n'en porte pas laisse celles du projet en place — sans quoi charger une
maquette purement typographique effacerait la photo du livre.

L'identité du livre n'est jamais touchée : c'est déjà la règle, et le chantier précédent
l'a rendue vraie de bout en bout.

## 4. Le module `maquettes`

`toutes()` rend aujourd'hui des triplets `(&'static str, &'static str, Couverture)`.
Elle devient :

```rust
pub struct Maquette {
    pub cle: String,        // le slug
    pub nom: String,        // ce qui s'affiche
    pub fournie: bool,      // ni renommable, ni effaçable
    pub couverture: Couverture,
    pub images: BTreeMap<String, Vec<u8>>,
}

pub fn toutes(config: Option<&Path>) -> Vec<Maquette>
pub fn par_cle(config: Option<&Path>, cle: &str) -> Option<Maquette>
pub fn ecrire(config: &Path, nom: &str, m: &Maquette) -> Result<(), String>
pub fn renommer(config: &Path, cle: &str, nom: &str) -> Result<(), String>
pub fn effacer(config: &Path, cle: &str) -> Result<(), String>
```

Le `Option<&Path>` porte le cas « répertoire de configuration inatteignable » : les
fournies restent disponibles, les personnalisées sont absentes. Même arbitrage que les
projets récents.

`folio()`, `blanche()` et `surimpression()` cessent d'être des constructeurs publics ;
les tests passent par `par_cle(None, "folio")`.

**La lecture est au mieux, l'écriture échoue fort.** Une maquette illisible est ignorée
avec un mot sur la sortie d'erreur — ce qui se perd est un point de départ. Mais un
« Enregistrer » qui échoue perd du travail : il remonte à l'interface, comme toutes les
commandes.

## 5. Commandes et interface

| Commande | Effet |
|---|---|
| `maquettes_liste` | gagne `fournie` dans `MaquetteVue`, et l'`AppHandle` |
| `maquette_choisir(cle)` | inchangée en surface, cherche dans les deux origines, pose aussi les images |
| `maquette_enregistrer(nom)` | écrit la couverture et les images du projet ouvert |
| `maquette_cloner(cle, nom)` | depuis une fournie comme depuis une personnalisée |
| `maquette_renommer(cle, nom)` | refuse sur une fournie |
| `maquette_effacer(cle)` | refuse sur une fournie |

Le refus côté Rust n'est pas une redondance de l'interface qui masque les boutons : c'est
la seule garantie réelle de l'immuabilité, l'interface n'étant qu'une politesse.

Le `<dialog>` — un premier dans ce front, mais c'est la primitive standard, qui gère
seule le focus et Échap :

```
Barre :  [Repartir d'une maquette… ▾] [Maquettes…]

┌─ Maquettes ──────────────────────────┐
│ Folio          fournie   [Cloner]    │
│ Blanche        fournie   [Cloner]    │
│ Surimpression  fournie   [Cloner]    │
│ ──────────────────────────────────── │
│ Ma collection  [Cloner][Renommer][✗] │
│                                      │
│ Enregistrer la couverture actuelle : │
│ [………………………………] [Enregistrer]         │
│                           [Fermer]   │
└──────────────────────────────────────┘
```

Le remplissage du `<select>` sort de l'initialisation d'`app.js` pour devenir une
fonction rappelée après chaque geste du dialogue.

**Conséquence assumée** : le bouton ne vit que dans l'étape Couverture, donc gérer ses
maquettes suppose un projet ouvert. C'est ce qu'implique le choix « depuis le projet
ouvert ».

## 6. Risques

**Le poids.** Une maquette avec deux photos pèse ce que pèsent les photos — quelques
mégaoctets. Le répertoire de configuration n'est pas fait pour de gros volumes, et rien
ne borne le nombre de maquettes. Aucun quota n'est posé : l'utilisateur voit ses
fichiers et les efface. Le dire dans l'aide vaut mieux qu'un plafond arbitraire.

**Les trois fournies changent de nature.** Les tests de propriété qui les tiennent
aujourd'hui — modes distincts, pied qui ne traverse pas le cadre, voile réservé à
l'image pleine page — porteront sur des archives lues, non sur du code. Un TOML mal
formé ne casserait plus la compilation mais le démarrage. La parade est un test qui les
parse toutes les trois, et `cargo test` est exigé avant commit.

**La génération des trois archives.** Elles ne seront pas écrites à la main : un test
transitoire les produit depuis les constructeurs actuels, ce qui les rend identiques par
construction. Un second, transitoire lui aussi, compare l'archive relue au constructeur ;
une fois vu passer, les constructeurs partent et les tests avec.

## 7. Vérification

### Le témoin

`cargo run --example temoin` : **98 pages, dos 7,21 mm**. Le témoin compose en Blanche —
si la maquette lue depuis une archive ne rend pas ce que le constructeur rendait, c'est
là que ça se verra.

### Ce que les tests doivent tenir

- Les trois archives fournies se lisent, et rendent exactement ce que les constructeurs
  rendaient (test transitoire, vu passer puis retiré).
- Le slug : accents décapés, espaces en tirets, casse ignorée ; deux noms qui donnent le
  même slug sont le même nom, et le refus le dit.
- Aller-retour d'une personnalisée, images comprises.
- Une maquette illisible n'empêche pas les autres de se lister.
- Renommer et effacer sont refusés sur une fournie, **par le Rust**, même si l'interface
  n'offre pas les boutons.
- Cloner marche depuis une fournie comme depuis une personnalisée.
- Charger une maquette sans images laisse celles du projet ; avec images, les remplace.
- Les trois tests de propriété existants, sur les maquettes lues.
- `contrats.test.js` : `fournie` dans `MaquetteVue`, et le dialogue qui n'offre pas
  Effacer sur une fournie.

Chaque test doit être vu échouer — TDD, ou mutation ciblée.

### À l'œil

Enregistrer la couverture d'un livre réel comme maquette, ouvrir un autre projet, la
charger : la mise en page doit être identique, les images posées, et l'identité du
nouveau livre intacte. Puis cloner Folio, renommer le clone, l'effacer.

## 8. Les lots

**Lot 1 — Le format et les trois fournies.** L'archive, `Maquette`, `toutes(None)` et
`par_cle(None, …)` servies depuis `include_bytes!`, les constructeurs retirés. Rien ne
change pour l'utilisateur : c'est la bascule qui doit être invisible, et le témoin le
prouve.

**Lot 2 — Les personnalisées.** Le répertoire de configuration, le slug, l'écriture, la
lecture au mieux, les commandes, et le `<select>` qui les liste sous un séparateur.
« Enregistrer la couverture actuelle » est le seul geste de ce lot.

**Lot 3 — Le dialogue.** Cloner, renommer, effacer, et le `<dialog>` qui les porte.
