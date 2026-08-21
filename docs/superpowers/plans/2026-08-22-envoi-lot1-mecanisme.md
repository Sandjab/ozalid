# L'envoi autographe, lot 1 — le mécanisme

> **Pour les agents :** SOUS-SKILL REQUISE — `superpowers:subagent-driven-development`
> (recommandée) ou `superpowers:executing-plans` pour exécuter ce plan tâche par tâche.
> Les étapes sont des cases à cocher (`- [ ]`).

**But :** un livre porte une liste d'envois ; chacun dépose un mot manuscrit sur la
page de titre de **son** exemplaire, et donne son propre package, sans qu'aucune page
ne se déplace.

**Architecture :** un module `envoi.rs` pour le modèle et l'assainissement des noms ;
`interieur::source` reçoit une trace facultative qu'il pose en `#place` sur la page de
titre ; la génération converge une seule fois puis compile M intérieurs ; l'étape
Livraison quitte `app.js` pour un `livraison.js`, sur le patron de `couverture.js`.

**Pile :** Rust (Tauri 2, serde, Typst en sidecar), JavaScript sans build, tests
`cargo test` et `node --test`, `fontTools` pour éprouver les polices.

**Spec :** `docs/superpowers/specs/2026-08-22-envoi-autographe-design.md` (lot 1 du § 8)

---

## Fichiers touchés

| Fichier | Responsabilité dans ce lot |
|---|---|
| `app/src-tauri/src/envoi.rs` | **Créé.** `Main`, `Envoi`, `Envois`, l'assainissement des noms |
| `app/src-tauri/src/lib.rs` | Déclarer le module, enregistrer les commandes |
| `app/src-tauri/src/projet.rs` | La section `envois` dans `Metadonnees` |
| `app/src-tauri/src/interieur.rs` | `Trace`, le `#place` sur la page de titre |
| `app/src-tauri/src/package.rs` | `assembler_envois` : convergence unique, M intérieurs |
| `app/src-tauri/src/commands.rs` | Les commandes de l'étape Livraison |
| `app/src-tauri/fonts/`, `app/outils/polices.sh` | Les polices manuscrites |
| `app/src/livraison.js` | **Créé.** Le rendu de l'étape Livraison |
| `app/src/index.html`, `app/src/app.js` | Le balisage, les écouteurs |
| `app/tests/dom_shim.js`, `app/tests/coquille.test.js` | Charger le nouveau script, le tester |

Commandes `cargo` depuis `app/src-tauri`, commandes `node` depuis `app`.

---

## Tâche 1 : les polices manuscrites, éprouvées avant d'être retenues

La spec en fait un travail, pas une case à cocher : une police qui ne porte pas `À` ne
le dit pas, Typst compose par repli, et l'envoi part chez le dédicataire dans deux
écritures. On éprouve **avant** de retenir.

**Fichiers :**
- Modifier : `app/outils/polices.sh`
- Créer : `app/src-tauri/fonts/*.ttf` (produits par le script)

- [ ] **Étape 1 : éprouver les candidates**

Les trois candidates, toutes OFL et redistribuables comme les vingt-neuf autres :
`caveat/Caveat[wght].ttf`, `dancingscript/DancingScript[wght].ttf`,
`petitformalscript/PetitFormalScript-Regular.ttf`.

```bash
cd /tmp && for f in "caveat/Caveat[wght].ttf" "dancingscript/DancingScript[wght].ttf" \
  "petitformalscript/PetitFormalScript-Regular.ttf"; do
  curl -fsSL -o "$(basename "$f")" "https://raw.githubusercontent.com/google/fonts/main/ofl/$f"
done
python3 - Caveat*.ttf DancingScript*.ttf PetitFormalScript*.ttf <<'EOF'
from fontTools.ttLib import TTFont
import sys
# Ce qu'un envoi français réclame : accents, ligature, guillemets, apostrophe courbe.
REQUIS = "ÀÂÄÉÈÊËÇÙÛÜÔÖÎÏàâäéèêëçùûüôöîïœŒ«»’…"
for f in sys.argv[1:]:
    cmap = TTFont(f, fontNumber=0).getBestCmap()
    manque = "".join(c for c in REQUIS if ord(c) not in cmap)
    print(f"{f:40} {'OK' if not manque else 'MANQUE ' + manque}")
EOF
```

Attendu : au moins deux polices sans rien qui manque. **Toute police qui affiche
`MANQUE` est écartée** — pas corrigée, pas contournée : écartée. Si moins de deux
survivent, en éprouver d'autres du même dépôt (`indieflower`, `zeyada`, `sacramento`)
par la même commande, et retenir les deux ou trois premières qui passent.

- [ ] **Étape 2 : ajouter les retenues à `polices.sh`**

Dans le tableau `FICHIERS` de `app/outils/polices.sh`, à la suite des polices de
labeur, avec le commentaire qui dit **pourquoi** celles-là :

```bash
  # Mains manuscrites des envois autographes. Retenues sur relevé fontTools : chacune
  # porte les accents français, la ligature œ, les guillemets et l'apostrophe courbe.
  # Une police qui les ignore serait composée par repli, sans un mot, et l'envoi
  # partirait dans deux écritures.
  "caveat/Caveat[wght].ttf"
  "dancingscript/DancingScript[wght].ttf"
```

(Remplacer par la liste réellement retenue à l'étape 1.)

- [ ] **Étape 3 : les récupérer et vérifier qu'elles sont là**

```bash
cd app && ./outils/polices.sh && ls src-tauri/fonts/ | grep -iE "caveat|dancing"
```

Attendu : les fichiers présents dans `src-tauri/fonts/`.

- [ ] **Étape 4 : commit**

```bash
git add app/outils/polices.sh app/src-tauri/fonts
git commit -m "Deux mains manuscrites, retenues sur ce qu'elles savent écrire"
```

---

## Tâche 2 : le modèle de l'envoi

**Fichiers :**
- Créer : `app/src-tauri/src/envoi.rs`
- Modifier : `app/src-tauri/src/lib.rs` (déclaration du module)
- Modifier : `app/src-tauri/src/projet.rs` (`Metadonnees`)

- [ ] **Étape 1 : écrire le module avec ses tests, tests d'abord**

Créer `app/src-tauri/src/envoi.rs` avec **uniquement** le module de tests et les
signatures vides, pour voir le rouge :

```rust
//! L'envoi autographe : le mot manuscrit adressé à une personne.
//!
//! À ne pas confondre avec la dédicace imprimée de `Livre::dedicace`, qui figure dans
//! tous les exemplaires. L'envoi est propre à un exemplaire, et il se pose **sur** une
//! page existante : il n'en ajoute aucune, donc il ne déplace ni la pagination, ni le
//! dos, ni la planche.

use serde::{Deserialize, Serialize};

/// Les polices manuscrites embarquées avec l'application.
///
/// Comme `POLICES_TEXTE`, la liste est fermée : Typst composerait une police inconnue
/// par repli sur son défaut **sans lever d'erreur**, et cela ne se verrait qu'après
/// tirage.
pub const MAINS: &[&str] = &["Caveat", "Dancing Script"];

#[cfg(test)]
mod tests {
    use super::*;

    /// Un dédicataire est un nom de personne, pas un chemin. « Marie D./Léa » ne doit
    /// créer aucun sous-répertoire, et « .. » ne doit pas sortir du dossier du projet
    /// — c'est la seule chaîne saisie par l'utilisateur qui devienne un chemin.
    #[test]
    fn un_dedicataire_ne_peut_pas_devenir_un_chemin() {
        assert_eq!(assaini("Marie D./Léa"), "Marie D-Léa");
        assert_eq!(assaini(".."), "envoi");
        assert_eq!(assaini("../../etc"), "etc");
        assert_eq!(assaini("  "), "envoi");
        assert_eq!(assaini("Léa"), "Léa");
    }

    /// Deux dédicataires qui se réduisent au même répertoire écraseraient l'un l'autre
    /// : le second exemplaire partirait avec le mot du premier.
    #[test]
    fn deux_noms_qui_se_confondent_recoivent_des_repertoires_distincts() {
        let noms = ["Marie/Léa", "Marie-Léa", "Marie:Léa"];
        let mut vus = Vec::new();
        for n in noms {
            vus.push(distinct(&assaini(n), &vus));
        }
        assert_eq!(vus, ["Marie-Léa", "Marie-Léa-2", "Marie-Léa-3"]);
    }

    /// Un livre neuf sait écrire sans qu'on lui règle quoi que ce soit, comme il sait
    /// déjà composer son intérieur en EB Garamond.
    #[test]
    fn un_livre_neuf_a_deja_une_main() {
        let Main::Police { police } = Envois::default().main;
        assert_eq!(police, MAINS[0]);
    }

    /// Une main hors liste est refusée, jamais substituée : même contrôle que
    /// `Interieur::verifie`, et pour la même raison.
    #[test]
    fn une_main_hors_liste_est_refusee() {
        let e = Envois {
            main: Main::Police {
                police: "Comic Sans".into(),
            },
            liste: vec![],
        };
        let err = e.verifie().unwrap_err();
        assert!(err.contains("Comic Sans"), "{err}");
    }
}
```

- [ ] **Étape 2 : lancer les tests et constater l'échec**

```bash
cd app/src-tauri && cargo test --lib envoi
```

Attendu : **échec de compilation** — `cannot find function 'assaini'`, `cannot find
type 'Envois'`. Ne pas passer à la suite avant de l'avoir vu. (Le module n'est pas
encore déclaré : ajouter `mod envoi;` à `lib.rs` d'abord si `cargo` ne le voit pas.)

- [ ] **Étape 3 : écrire le modèle**

Dans `envoi.rs`, au-dessus du module de tests :

```rust
/// D'où vient l'écriture des envois de ce livre.
///
/// Le livre fixe sa main, l'envoi apporte son contenu : tous les exemplaires d'un même
/// livre se ressemblent, comme dans la réalité.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "lowercase")]
pub enum Main {
    /// Police manuscrite embarquée avec l'application. Les lots suivants y ajouteront
    /// la police fournie par l'auteur, l'image écrite à la main et l'image générée.
    Police { police: String },
}

impl Default for Main {
    fn default() -> Self {
        Self::Police {
            police: MAINS[0].into(),
        }
    }
}

/// Un mot adressé à une personne, sur son exemplaire.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Envoi {
    pub dedicataire: String,
    /// Ce que la main réclame : ici, le texte à composer.
    pub contenu: String,
}

/// La main du livre et ses envois.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Envois {
    #[serde(default)]
    pub main: Main,
    #[serde(default)]
    pub liste: Vec<Envoi>,
}

impl Envois {
    /// Refuse une main hors liste.
    ///
    /// Sans ce contrôle, Typst composerait dans sa police par défaut **sans lever
    /// d'erreur** : `--ignore-system-fonts` empêche une substitution par le système,
    /// pas une substitution par le défaut du binaire. C'est le contrôle
    /// d'`Interieur::verifie`, pour la même raison.
    pub fn verifie(&self) -> Result<(), String> {
        let Main::Police { police } = &self.main;
        if MAINS.contains(&police.as_str()) {
            return Ok(());
        }
        Err(format!(
            "main inconnue : « {police} ». Attendu : {}.",
            MAINS.join(", ")
        ))
    }
}

/// Nom de répertoire tiré d'un dédicataire.
///
/// C'est la seule chaîne saisie par l'utilisateur qui devienne un chemin : tout ce qui
/// n'est ni lettre, ni chiffre, ni espace, ni tiret devient un tiret, et ce qui ne
/// laisse rien devient « envoi ». Un dédicataire nommé « .. » ne doit pas écrire hors
/// du dossier du projet.
pub fn assaini(dedicataire: &str) -> String {
    let brut: String = dedicataire
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == ' ' || c == '-' {
                c
            } else {
                '-'
            }
        })
        .collect();
    let net = brut.trim().trim_matches('-').trim();
    if net.is_empty() {
        "envoi".into()
    } else {
        net.into()
    }
}

/// Rend `nom` unique parmi `pris`, en le suffixant.
///
/// Deux dédicataires qui se réduisent au même répertoire écraseraient l'un l'autre :
/// le second exemplaire partirait avec le mot du premier.
pub fn distinct(nom: &str, pris: &[String]) -> String {
    if !pris.iter().any(|p| p == nom) {
        return nom.into();
    }
    (2..)
        .map(|n| format!("{nom}-{n}"))
        .find(|c| !pris.iter().any(|p| p == c))
        .expect("la suite des entiers ne s'épuise pas")
}
```

Dans `lib.rs`, à côté des autres modules, qui sont tous publics — les exemples de
`examples/` les atteignent par `ozalid_lib::` :

```rust
pub mod envoi;
```

- [ ] **Étape 4 : lancer les tests et constater le vert**

```bash
cd app/src-tauri && cargo test --lib envoi
```

Attendu : les quatre tests passent.

- [ ] **Étape 5 : brancher la section dans le projet**

Dans `projet.rs`, `Metadonnees`, après `livraison` :

```rust
    /// Facultative, comme `livraison` et la dédicace avant elle : un `.ozalid` écrit
    /// avant les envois s'ouvre sans un mot, avec une liste vide. `VERSION` ne bouge
    /// donc pas.
    #[serde(default)]
    pub envois: crate::envoi::Envois,
```

Et le compléter dans `Projet::nouveau` — chercher où `livraison` y est posé et poser
`envois: Envois::default()` de la même façon.

- [ ] **Étape 6 : le test de relecture et de round-trip**

Dans `projet.rs`, module `tests`, à la suite des tests de dédicace :

```rust
/// Un `.ozalid` écrit avant les envois s'ouvre sans un mot : troisième section
/// facultative après `[interieur]` et `[livraison]`, et `VERSION` n'a pas bougé.
#[test]
fn un_projet_sans_section_envois_se_relit() {
    let toml = r#"
[ozalid]
version = 2

[livre]
titre = "Les Heures creuses"
auteur = "Ivan Pjig"
"#;
    let m: Metadonnees = toml::from_str(toml).expect("projet sans [envois] refusé");
    assert!(m.envois.liste.is_empty());
    assert!(m.envois.verifie().is_ok(), "la main par défaut doit être valide");
}

/// Les envois sont du travail de l'utilisateur au même titre que la maquette : les
/// reperdre, c'est réécrire tous les mots à la main.
#[test]
fn les_envois_survivent_a_l_aller_retour() {
    let mut p = Projet::nouveau(livre(), "## 01\n\nA.\n".into());
    p.meta.envois.liste = vec![crate::envoi::Envoi {
        dedicataire: "Léa".into(),
        contenu: "À Léa, qui a lu la première version.".into(),
    }];

    let r = aller_retour(&p);
    assert_eq!(r.meta.envois.liste.len(), 1);
    assert_eq!(r.meta.envois.liste[0].dedicataire, "Léa");
    assert_eq!(
        r.meta.envois.liste[0].contenu,
        "À Léa, qui a lu la première version."
    );
}
```

- [ ] **Étape 7 : lancer toute la suite**

```bash
cd app/src-tauri && cargo test
```

Attendu : tout passe. Les tests existants ne changent pas de résultat — la section est
facultative et personne ne la lit encore.

- [ ] **Étape 8 : commit**

```bash
git add app/src-tauri/src/envoi.rs app/src-tauri/src/lib.rs app/src-tauri/src/projet.rs
git commit -m "Un dédicataire est un nom de personne, jamais un chemin"
```

---

## Tâche 3 : la surcharge de la page de titre

**Fichiers :**
- Modifier : `app/src-tauri/src/interieur.rs` (`Trace`, `liminaires`, `source`)
- Modifier : `app/src-tauri/src/package.rs:77`, `:93` et
  `app/src-tauri/src/commands.rs:517`, `:528` (les quatre appelants de `source`)

- [ ] **Étape 1 : écrire les tests**

À la fin du module `#[cfg(test)] mod tests` d'`interieur.rs` :

```rust
fn trace() -> Trace<'static> {
    Trace {
        police: "Caveat",
        texte: "À Léa, qui a lu la première version.",
    }
}

/// L'envoi se pose par `#place`, qui ne consomme pas le flux : il lui est impossible
/// de créer une page. Ce n'est pas une précaution, c'est la propriété sur laquelle
/// repose toute la promesse — même pagination, même dos, même planche pour tous les
/// envois. Si ce test tombe, tous les packages d'envoi sont faux.
#[test]
fn un_envoi_ne_cree_aucune_page() {
    let sans = liminaires(&livre(), None);
    let avec = liminaires(&livre(), Some(trace()));

    assert_eq!(
        avec.matches("#pagebreak()").count(),
        sans.matches("#pagebreak()").count(),
        "l'envoi a déplacé une page"
    );
    assert!(avec.contains("#place("), "l'envoi ne passe pas par #place : {avec}");
}

/// Hors de la page de titre, la source ne bouge pas d'un octet. Un envoi qui
/// modifierait le corps changerait la pagination sans qu'aucun compte ne le signale.
#[test]
fn un_envoi_ne_touche_que_la_page_de_titre() {
    let pr = provider("lulu").unwrap();
    let r = Reglage { gouttiere: 25.0, blanche: false };
    let sans = source(&livre(), &Interieur::default(), pr, &r, &chapitres(), None);
    let avec = source(&livre(), &Interieur::default(), pr, &r, &chapitres(), Some(trace()));

    let corps = |s: &str| s.split("#set page(footer: context").nth(1).unwrap().to_string();
    assert_eq!(corps(&sans), corps(&avec), "le corps a changé");
}

/// La main choisie doit être celle qui compose : sans le `font:`, Typst écrirait
/// l'envoi dans la police de labeur du livre, et le mot ne ressemblerait plus à un mot
/// écrit à la main.
#[test]
fn l_envoi_est_compose_dans_la_main_du_livre() {
    let s = liminaires(&livre(), Some(trace()));
    assert!(s.contains(r#"font: "Caveat""#), "main absente : {s}");
}

/// Même piège que le titre de page et que la dédicace : le markup Typst doit être
/// échappé, les sauts de ligne voulus doivent survivre.
#[test]
fn un_envoi_est_echappe_et_garde_ses_sauts_de_ligne() {
    let t = Trace { police: "Caveat", texte: "À #Léa,\navec mon amitié." };
    let s = liminaires(&livre(), Some(t));

    assert!(s.contains(r"À \#Léa,"), "envoi non échappé : {s}");
    assert!(s.contains(r"\ avec mon amitié."), "saut de ligne perdu : {s}");
}
```

- [ ] **Étape 2 : lancer les tests et constater l'échec**

```bash
cd app/src-tauri && cargo test --lib interieur
```

Attendu : **échec de compilation** — `cannot find type 'Trace'`, et `liminaires` prend
un argument de trop.

- [ ] **Étape 3 : poser la trace**

Dans `interieur.rs`, au-dessus de `source` :

```rust
/// Ce qu'un envoi dépose sur la page de titre.
///
/// `interieur` ne connaît pas la main du livre : il reçoit ce qu'elle a décidé. Les
/// lots suivants y ajouteront l'image ; la structure deviendra alors une énumération,
/// et ce module n'aura toujours pas à savoir d'où l'image vient.
#[derive(Debug, Clone, Copy)]
pub struct Trace<'a> {
    pub police: &'a str,
    pub texte: &'a str,
}
```

Changer la signature de `source` — le paramètre vient en dernier, après les chapitres :

```rust
pub fn source(
    livre: &Livre,
    int: &Interieur,
    pr: &Provider,
    r: &Reglage,
    chapitres: &[Chapitre],
    envoi: Option<Trace>,
) -> String {
```

et l'appel qu'elle fait aux liminaires :

```rust
    s.push_str(&liminaires(livre, envoi));
```

Dans `liminaires`, changer la signature et poser le `#place` **dans le bloc de la page
de titre**, c'est-à-dire juste avant le `#pagebreak()` qui la termine. Le plus simple
est de le pousser juste après le premier `push_str`, avant le bloc de copyright :

```rust
fn liminaires(livre: &Livre, envoi: Option<Trace>) -> String {
```

puis, immédiatement après le `s.push_str(&format!(…))` du faux-titre et de la page de
titre — attention, ce bloc se termine par `#pagebreak()`, donc le `#place` doit être
inséré **avant lui**. Découper le format existant en deux : garder tout jusqu'à
`#align(center, emph(text(size: 10pt)[{}]))`, puis :

```rust
    // L'envoi se pose sur la page de titre, dans le blanc que son contenu laisse au
    // bas. `#place` ne consomme pas le flux : il lui est impossible de créer une page,
    // et c'est là-dessus que repose la promesse — la pagination, le dos et la planche
    // sont les mêmes pour tous les envois du livre.
    if let Some(t) = envoi {
        s.push_str(&format!(
            r#"#place(bottom + center, dy: -28mm, block(width: 70%,
  text(font: "{}", size: 14pt)[{}]))
"#,
            // La main est validée en amont par `Envois::verifie` : pas d'échappement.
            t.police,
            echappe(t.texte).replace('\n', r" \ ")
        ));
    }
    s.push_str("#pagebreak()\n\n");
```

- [ ] **Étape 4 : rattraper les quatre appelants**

Ajouter `None` en dernier argument dans les quatre appels existants :

```
app/src-tauri/src/package.rs:77   (la mesure de convergence)
app/src-tauri/src/package.rs:93   (la composition finale)
app/src-tauri/src/commands.rs:517 (la mesure de convergence)
app/src-tauri/src/commands.rs:528 (la composition finale)
```

Les retrouver par `grep -rn "interieur::source" src` — un package ordinaire ne porte
pas d'envoi.

- [ ] **Étape 5 : lancer les tests et le témoin**

```bash
cd app/src-tauri && cargo test && cargo run --example temoin
```

Attendu : tout passe, et le témoin rend **98 pages, dos 7,21 mm** — inchangé. La
signature a bougé, pas le document.

- [ ] **Étape 6 : voir les tests échouer sur des mutations ciblées**

Appliquer, lancer `cargo test`, vérifier l'échec, **puis annuler**.

| Mutation | Échec attendu |
|---|---|
| Remplacer `#place(bottom + center, dy: -28mm,` par `#v(120mm)` | `un_envoi_ne_cree_aucune_page` |
| Retirer `font: "{}", ` du texte | `l_envoi_est_compose_dans_la_main_du_livre` |
| Retirer `echappe(` autour de `t.texte` | `un_envoi_est_echappe_et_garde_ses_sauts_de_ligne` |

- [ ] **Étape 7 : commit**

```bash
git add app/src-tauri/src
git commit -m "L'envoi se pose sur la page de titre, sans rien pousser devant lui"
```

---

## Tâche 4 : la génération, convergée une seule fois

**Fichiers :**
- Modifier : `app/src-tauri/src/package.rs` (`assembler_envois`)

- [ ] **Étape 1 : écrire le test**

À la fin du module `#[cfg(test)] mod tests` de `package.rs` :

```rust
/// Les répertoires d'envoi portent le nom du dédicataire, assaini et rendu unique.
/// Deux dédicataires qui se confondraient enverraient au second le mot du premier.
#[test]
fn les_repertoires_d_envoi_sont_distincts_et_sans_chemin() {
    let envois = [
        crate::envoi::Envoi { dedicataire: "Marie/Léa".into(), contenu: "A.".into() },
        crate::envoi::Envoi { dedicataire: "Marie-Léa".into(), contenu: "B.".into() },
        crate::envoi::Envoi { dedicataire: "..".into(), contenu: "C.".into() },
    ];
    assert_eq!(
        dossiers_d_envoi(&envois),
        vec!["Marie-Léa", "Marie-Léa-2", "envoi"]
    );
}
```

- [ ] **Étape 2 : lancer le test et constater l'échec**

```bash
cd app/src-tauri && cargo test --lib package
```

Attendu : `cannot find function 'dossiers_d_envoi'`.

- [ ] **Étape 3 : écrire la génération**

Dans `package.rs` :

```rust
/// Les noms de répertoire des envois, dans l'ordre de la liste.
///
/// Séparé d'`assembler_envois` pour être éprouvé sans toucher au disque ni à Typst :
/// c'est ici que se joue le fait qu'un exemplaire ne parte pas avec le mot d'un autre.
fn dossiers_d_envoi(envois: &[crate::envoi::Envoi]) -> Vec<String> {
    let mut pris: Vec<String> = Vec::with_capacity(envois.len());
    for e in envois {
        let d = crate::envoi::distinct(&crate::envoi::assaini(&e.dedicataire), &pris);
        pris.push(d);
    }
    pris
}

/// Compose un package par envoi, tous chez le même prestataire.
///
/// **La convergence n'a lieu qu'une fois.** L'envoi se pose par `#place`, qui ne peut
/// pas créer de page : la gouttière, la parité, le compte de pages, le dos et la
/// planche sont donc les mêmes pour tous. Converger M fois ne coûterait pas seulement
/// M fois le temps — cela laisserait croire que le résultat pourrait différer.
pub fn assembler_envois(
    projet: &Projet,
    pr: &Provider,
    papier: &Papier,
    releve: Releve,
    racine: &Path,
    typst: &Typst,
) -> Result<Vec<(String, Package)>, String> {
    let envois = &projet.meta.envois;
    envois.verifie()?;
    if envois.liste.is_empty() {
        return Err("aucun envoi : en écrire un avant de générer.".into());
    }

    // 1. Le package de référence, sans envoi : c'est lui qui converge, calcule le dos
    //    et compose la planche.
    let reference = racine.join(".reference");
    let base = assembler(projet, pr, papier, releve, &reference, typst)?;

    let int = &projet.meta.interieur;
    let livre = &projet.meta.livre;
    let chapitres = manuscrit::decoupe(&projet.texte, livre.chapitres)?;
    let reglage = Reglage {
        gouttiere: base.gouttiere,
        blanche: base.blanche,
    };
    let Main::Police { police } = &envois.main;

    let mut sorties = Vec::with_capacity(envois.liste.len());
    for (e, dossier_nom) in envois.liste.iter().zip(dossiers_d_envoi(&envois.liste)) {
        let dossier = racine.join(&dossier_nom);
        std::fs::create_dir_all(&dossier)
            .map_err(|e| format!("répertoire inutilisable ({}) : {e}", dossier.display()))?;

        let src = dossier.join(nom(pr, "interieur", "typ"));
        ecrire(
            &src,
            &interieur::source(
                livre,
                int,
                pr,
                &reglage,
                &chapitres,
                Some(interieur::Trace {
                    police,
                    texte: &e.contenu,
                }),
            ),
        )?;
        let pdf = dossier.join(nom(pr, "interieur", "pdf"));
        typst.compile(&src, &pdf)?;

        // La planche ne dépend pas de l'envoi : elle est recopiée, pas recomposée.
        let mut p = base.clone();
        p.chemins = vec![affiche(&pdf), copier_planche(&reference, &dossier, pr)?];
        p.vignette = copier(&reference, &dossier, &nom(pr, "couverture", "png"))?;
        sorties.push((dossier_nom, p));
    }
    Ok(sorties)
}

/// Recopie la planche de référence dans le répertoire d'un envoi, et rend son chemin.
fn copier_planche(reference: &Path, dossier: &Path, pr: &Provider) -> Result<String, String> {
    copier(reference, dossier, &nom(pr, "couverture", "pdf"))
}

fn copier(depuis: &Path, vers: &Path, fichier: &str) -> Result<String, String> {
    let cible = vers.join(fichier);
    std::fs::copy(depuis.join(fichier), &cible)
        .map_err(|e| format!("{fichier} : copie impossible : {e}"))?;
    Ok(affiche(&cible))
}
```

Ajouter en tête de `package.rs` les `use` qui manquent :

```rust
use crate::envoi::Main;
use crate::interieur::{self, Reglage};
```

(`interieur` et `Reglage` y sont peut-être déjà : vérifier avant d'ajouter en double.)

- [ ] **Étape 4 : lancer les tests**

```bash
cd app/src-tauri && cargo test && cargo clippy --all-targets -- -D warnings
```

Attendu : tout passe, clippy propre.

- [ ] **Étape 5 : voir le test échouer sur une mutation**

Remplacer `distinct(&assaini(…), &pris)` par `assaini(…)` seul : le test
`les_repertoires_d_envoi_sont_distincts_et_sans_chemin` doit tomber sur
`["Marie-Léa", "Marie-Léa", "envoi"]`. Annuler.

- [ ] **Étape 6 : commit**

```bash
git add app/src-tauri/src/package.rs
git commit -m "Un seul passage de convergence dit la pagination de tous les envois"
```

---

## Tâche 5 : les commandes

**Fichiers :**
- Modifier : `app/src-tauri/src/commands.rs`
- Modifier : `app/src-tauri/src/lib.rs` (enregistrement)

- [ ] **Étape 1 : écrire les commandes**

Dans `commands.rs`, à la suite des commandes de destinataires :

```rust
/// Remplace la liste des envois et la main du livre.
///
/// Comme `livre_modifier`, la commande reçoit **l'objet entier** : ce que le front
/// n'envoie pas est effacé. C'est le même piège que la dédicace, et il se garde du
/// même côté.
#[tauri::command]
pub fn envois_modifier(
    envois: crate::envoi::Envois,
    atelier: State<Atelier>,
) -> Result<ProjetVue, String> {
    envois.verifie()?;
    let mut garde = atelier.ouvert.lock().unwrap();
    let o = garde.as_mut().ok_or_else(aucun_projet)?;
    o.projet.meta.envois = envois;
    vue_modifiee(o)
}

/// Les mains offertes par l'application.
#[tauri::command]
pub fn mains_liste() -> Vec<&'static str> {
    crate::envoi::MAINS.to_vec()
}

/// Compose un package par envoi, chez le prestataire visé.
#[tauri::command]
pub fn envoyer(atelier: State<Atelier>) -> Result<Vec<ResultatEnvoi>, String> {
    let garde = atelier.ouvert.lock().unwrap();
    let o = garde.as_ref().ok_or_else(aucun_projet)?;
    let (pr, papier, d) = vise(o)?;
    let typst = typst()?;
    let racine = sorties_racine(o)?.join("envois");

    let sorties = package::assembler_envois(
        &o.projet,
        pr,
        papier,
        planche::Releve {
            dos: d.dos_mm,
            fond_perdu: d.fond_perdu_mm,
        },
        &racine,
        &typst,
    )?;

    Ok(sorties
        .into_iter()
        .zip(o.projet.meta.envois.liste.iter())
        .map(|((dossier, p), e)| ResultatEnvoi {
            dedicataire: e.dedicataire.clone(),
            dossier,
            // La vignette manquante ne perd pas le package : les PDF sont écrits.
            vignette: donnee_png(Path::new(&p.vignette)).ok(),
            package: p,
        })
        .collect())
}
```

L'aperçu, qui est ce qui permet de juger un envoi avant de le composer :

```rust
/// La page de titre d'un envoi, telle qu'elle sera imprimée.
///
/// La source est celle de l'intérieur **privée de ses chapitres** : la page de titre
/// ne dépend pas du corps, et composer trois cents pages pour en regarder une seule
/// ferait de l'aperçu quelque chose qu'on n'ouvre jamais. La gouttière prise est la
/// première tranche du gabarit — elle ne déplace que la marge intérieure, et cet
/// aperçu n'est pas ce qui part à l'imprimeur : le PDF l'est.
#[tauri::command]
pub fn envoi_apercu(index: usize, atelier: State<Atelier>) -> Result<String, String> {
    let garde = atelier.ouvert.lock().unwrap();
    let o = garde.as_ref().ok_or_else(aucun_projet)?;
    let (pr, _, _) = vise(o)?;
    let envois = &o.projet.meta.envois;
    envois.verifie()?;
    let e = envois
        .liste
        .get(index)
        .ok_or("envoi introuvable : la liste a changé.")?;
    let crate::envoi::Main::Police { police } = &envois.main;

    let int = &o.projet.meta.interieur;
    int.verifie()?;
    let dossier = sorties_racine(o)?.join("envois");
    std::fs::create_dir_all(&dossier)
        .map_err(|err| format!("répertoire inutilisable ({}) : {err}", dossier.display()))?;
    let src = dossier.join("apercu.typ");
    ecrire(
        &src,
        &interieur::source(
            &o.projet.meta.livre,
            int,
            pr,
            &Reglage {
                gouttiere: pr.gouttieres[0].2,
                blanche: false,
            },
            &[],
            Some(interieur::Trace {
                police,
                texte: &e.contenu,
            }),
        ),
    )?;
    let png = dossier.join("apercu.png");
    typst()?.apercu(&src, &png, 3, 110)?;
    donnee_png(&png)
}
```

Ajouter `commands::envoi_apercu` à la liste de `lib.rs`.

et la vue rendue au front, à côté de `Resultat` :

```rust
/// Ce qu'un envoi produit, du point de vue de l'interface.
#[derive(Debug, Clone, Serialize)]
pub struct ResultatEnvoi {
    pub dedicataire: String,
    pub dossier: String,
    pub package: package::Package,
    pub vignette: Option<String>,
}
```

`Package` doit dériver `Clone` — il le fait déjà (`#[derive(Debug, Clone, Serialize)]`).

Dans `lib.rs`, à côté des autres :

```rust
            commands::envois_modifier,
            commands::mains_liste,
            commands::envoyer,
```

- [ ] **Étape 2 : vérifier que tout compile et que rien n'a bougé**

```bash
cd app/src-tauri && cargo test && cargo clippy --all-targets -- -D warnings
```

Attendu : tout passe. Ces commandes ne sont pas encore appelées.

- [ ] **Étape 3 : commit**

```bash
git add app/src-tauri/src
git commit -m "Les envois s'écrivent, se règlent et se composent depuis l'atelier"
```

---

## Tâche 6 : l'étape Livraison quitte `app.js`

Le déplacement se fait **avant** d'ajouter quoi que ce soit : sinon on ne saura pas si
une régression vient du déplacement ou de l'ajout.

**Fichiers :**
- Créer : `app/src/livraison.js`
- Modifier : `app/src/app.js`, `app/src/index.html`, `app/tests/dom_shim.js`

- [ ] **Étape 1 : déplacer, sans rien changer**

Couper de `app.js` et coller dans un nouveau `app/src/livraison.js`, dans cet ordre :
`afficherDestinataires`, `reglerDestinataire`, `cheminsGroupes`, `afficherPackages`,
`packager`. En tête du fichier :

```javascript
'use strict';

/**
 * L'étape Livraison : les destinataires, les packages, et bientôt les envois.
 *
 * Même partage que `couverture.js` : ce fichier ne pose aucun écouteur et ne lit pas
 * le DOM au chargement. Il définit, `app.js` branche — c'est ce qui permet aux deux
 * de vivre dans le même contexte global sans dépendre de l'ordre de chargement.
 */
```

**Ne pas** déplacer les `addEventListener` : ils restent dans `app.js`, comme pour
`couverture.js`.

- [ ] **Étape 2 : charger le fichier aux deux endroits**

Dans `app/src/index.html`, avant `app.js` :

```html
<script src="couverture.js"></script>
<script src="livraison.js"></script>
<script src="app.js"></script>
```

Dans `app/tests/dom_shim.js`, la liste des scripts chargés :

```javascript
  for (const nom of ['couverture.js', 'livraison.js', 'app.js']) {
```

- [ ] **Étape 3 : vérifier que rien n'a bougé**

```bash
cd app && node --test "tests/*.test.js" && node --check src/livraison.js && node --check src/app.js
```

Attendu : **120 tests passent**, exactement comme avant le déplacement. Un seul échec
signifie que le découpage a coupé une dépendance : la corriger, ne pas l'ignorer.

- [ ] **Étape 4 : commit**

```bash
git add app/src/livraison.js app/src/app.js app/src/index.html app/tests/dom_shim.js
git commit -m "L'étape Livraison a son fichier, avant qu'on l'agrandisse"
```

---

## Tâche 7 : les envois dans l'atelier

**Fichiers :**
- Modifier : `app/src/index.html` (étape Livraison), `app/src/livraison.js`,
  `app/src/app.js`, `app/tests/coquille.test.js`

- [ ] **Étape 1 : écrire les tests**

À la fin de `app/tests/coquille.test.js` :

```js
/**
 * `envois_modifier` remplace l'objet entier : un envoi ajouté sans la main du livre
 * ramènerait la main au défaut, et tous les exemplaires changeraient d'écriture sans
 * qu'on l'ait demandé. Même piège que la dédicace, même garde.
 */
test('ajouter un envoi conserve la main du livre', async () => {
  const a = atelier({ sur: { envois: { main: { mode: 'police', police: 'Dancing Script' }, liste: [] } } });
  const { els } = await charge({ invoke: a.invoke });
  await els.get('btNouveau').declenche('click');

  els.get('inDedicataire').value = 'Léa';
  await els.get('btAjouterEnvoi').declenche('click');

  const envoi = a.appels.findLast(([c]) => c === 'envois_modifier');
  assert.ok(envoi, 'aucun envois_modifier : le bouton n\'a pas d\'écouteur');
  assert.equal(envoi[1].envois.main.police, 'Dancing Script');
  assert.equal(envoi[1].envois.liste[0].dedicataire, 'Léa');
});

/** Sans envoi, le bouton de génération n'a rien à composer : il reste éteint. */
test('le bouton des envois est éteint tant que la liste est vide', async () => {
  const a = atelier();
  const { els } = await charge({ invoke: a.invoke });
  await els.get('btNouveau').declenche('click');

  assert.equal(els.get('btEnvoyer').disabled, true);
});
```

Le faux projet doit porter la section : ajouter à `projet()`, dans
`app/tests/coquille.test.js`, à côté de `livraison` :

```js
    envois: { main: { mode: 'police', police: 'Caveat' }, liste: [] },
```

et à `invokeMuet` de `dom_shim.test.js` ainsi qu'au `PROJET` du même fichier, la même
ligne — sans quoi le faux DOM rend un projet que le vrai `app.js` ne sait plus lire.

Enfin, `atelier()` doit répondre à la nouvelle commande de liste : dans son `switch`,

```js
      case 'mains_liste': return ['Caveat', 'Dancing Script'];
```

- [ ] **Étape 2 : lancer les tests et constater l'échec**

```bash
cd app && node --test "tests/coquille.test.js"
```

Attendu : les deux échouent — `inDedicataire` et `btEnvoyer` n'existent pas dans le
faux DOM, qui lit les identifiants du vrai HTML.

- [ ] **Étape 3 : le balisage**

Dans `app/src/index.html`, à l'intérieur de `#etapeLivraison`, après le bloc
« Destinataires », un second bloc :

```html
    <div class="bloc">
      <h2>Envois</h2>
      <p class="note">Un mot manuscrit sur la page de titre, et un exemplaire par
        personne. L'envoi ne déplace aucune page : le dos et la planche restent ceux du
        tirage. Les fichiers sont écrits dans <code>envois/</code>, à côté du
        <code>.ozalid</code>.</p>
      <label><span>Main</span><select id="inMain"></select></label>
      <div class="ligne">
        <input type="text" id="inDedicataire" placeholder="à qui ?"
               aria-label="Dédicataire à ajouter">
        <button id="btAjouterEnvoi" type="button">Ajouter</button>
      </div>
      <div id="envois" class="destinataires"></div>
      <img id="apercuEnvoi" alt="" hidden>
      <div class="ligne">
        <button id="btEnvoyer" type="button" disabled>Générer les envois</button>
        <span id="etatEnvois" class="etat"></span>
      </div>
      <div id="resultatEnvois" class="resultat" hidden></div>
    </div>
```

- [ ] **Étape 4 : le rendu, dans `livraison.js`**

```javascript
/**
 * La liste des envois : un dédicataire, son mot, et de quoi le retirer.
 *
 * Le mot est un `textarea` : un envoi tient en deux ou trois lignes, et un `input`
 * cacherait la fin de ce qu'on écrit — or c'est précisément ce qui sera imprimé.
 */
function afficherEnvois() {
  const box = $('envois');
  box.textContent = '';
  for (const [i, e] of projet.envois.liste.entries()) {
    const ligne = document.createElement('div');
    ligne.className = 'destinataire';

    const qui = document.createElement('input');
    qui.type = 'text';
    qui.value = e.dedicataire;
    qui.setAttribute('aria-label', `Dédicataire ${i + 1}`);
    qui.addEventListener('change', () => reglerEnvoi(i, { dedicataire: qui.value }));

    const mot = document.createElement('textarea');
    mot.rows = 2;
    mot.value = e.contenu;
    mot.setAttribute('aria-label', `Mot pour ${e.dedicataire || 'ce dédicataire'}`);
    mot.addEventListener('change', () => reglerEnvoi(i, { contenu: mot.value }));

    const voir = document.createElement('button');
    voir.type = 'button';
    voir.textContent = 'Voir la page';
    voir.addEventListener('click', () => apercuEnvoi(i));

    const retirer = document.createElement('button');
    retirer.type = 'button';
    retirer.textContent = 'Retirer';
    retirer.addEventListener('click', () => envoisModifier(
      projet.envois.liste.filter((_, n) => n !== i)));

    ligne.append(qui, mot, voir, retirer);
    box.append(ligne);
  }
  $('btEnvoyer').disabled = projet.envois.liste.length === 0;
}

/**
 * La page de titre de cet envoi, telle qu'elle sera imprimée.
 *
 * C'est la seule façon de voir qu'un mot déborde : le compte de pages, lui, ne bougera
 * pas — c'est tout l'objet du `#place`, et c'est aussi ce qui rend un débordement
 * silencieux.
 */
async function apercuEnvoi(i) {
  const img = $('apercuEnvoi');
  await tente(async () => {
    img.src = `data:image/png;base64,${await invoke('envoi_apercu', { index: i })}`;
    img.alt = `Page de titre de l'exemplaire de ${projet.envois.liste[i].dedicataire}`;
    img.hidden = false;
  });
}

/** Remplace un envoi par lui-même modifié. */
function reglerEnvoi(i, sur) {
  envoisModifier(projet.envois.liste.map((e, n) => (n === i ? { ...e, ...sur } : e)));
}

/**
 * Envoie la liste **et la main** : la commande remplace l'objet entier, et une main
 * omise reviendrait au défaut — tous les exemplaires changeraient d'écriture sans que
 * personne ne l'ait demandé.
 */
async function envoisModifier(liste) {
  await tente(async () => afficherProjet(await invoke('envois_modifier', {
    envois: { main: projet.envois.main, liste },
  })));
}

async function envoyer() {
  const bt = $('btEnvoyer');
  bt.disabled = true;
  $('resultatEnvois').hidden = true;
  $('etatEnvois').className = 'etat';
  $('etatEnvois').textContent = `composition de ${projet.envois.liste.length} envoi(s)…`;
  try {
    afficherResultatEnvois(await invoke('envoyer'));
    $('etatEnvois').textContent = '';
  } catch (e) {
    $('etatEnvois').textContent = String(e);
    $('etatEnvois').className = 'etat erreur';
  } finally {
    bt.disabled = false;
  }
}

/** Ce qui a été écrit, pour qui, et la page de titre telle qu'elle partira. */
function afficherResultatEnvois(resultats) {
  const box = $('resultatEnvois');
  box.textContent = '';
  for (const r of resultats) {
    const bloc = document.createElement('div');
    bloc.className = 'package';
    const titre = document.createElement('h3');
    titre.textContent = r.dedicataire || 'sans nom';
    const chemin = document.createElement('p');
    chemin.className = 'chemin';
    chemin.textContent = `envois/${r.dossier}/ — ${r.package.pages} pages, dos ${
      r.package.dos.toFixed(2)} mm`;
    bloc.append(titre, chemin);
    if (r.vignette) {
      const img = document.createElement('img');
      img.src = `data:image/png;base64,${r.vignette}`;
      img.alt = `Planche de l'exemplaire de ${r.dedicataire}`;
      bloc.append(img);
    }
    box.append(bloc);
  }
  box.hidden = false;
}
```

- [ ] **Étape 5 : les branchements, dans `app.js`**

Dans la fonction qui affiche le projet, à côté de l'appel à `afficherDestinataires()` :

```js
  afficherEnvois();
```

Le sélecteur de main se remplit dans `chargerProviders`, sur le patron exact de la
police d'intérieur, juste après la boucle qui remplit `inPoliceInterieur` :

```js
  for (const m of await invoke('mains_liste')) {
    $('inMain').append(new Option(m, m));
  }
```

Et les écouteurs, à côté de `btPackager` :

```js
$('btEnvoyer').addEventListener('click', envoyer);
$('btAjouterEnvoi').addEventListener('click', () => {
  const qui = $('inDedicataire').value.trim();
  if (qui === '') return;
  $('inDedicataire').value = '';
  return envoisModifier([...projet.envois.liste, { dedicataire: qui, contenu: '' }]);
});
$('inMain').addEventListener('change', () => tente(async () =>
  afficherProjet(await invoke('envois_modifier', {
    envois: { main: { mode: 'police', police: $('inMain').value }, liste: projet.envois.liste },
  }))));
```

- [ ] **Étape 6 : lancer les tests et constater le vert**

```bash
cd app && node --test "tests/*.test.js" && node --check src/livraison.js && node --check src/app.js
```

Attendu : tout passe, 122 tests.

- [ ] **Étape 7 : voir les tests échouer sur des mutations ciblées**

| Mutation | Échec attendu |
|---|---|
| Dans `envoisModifier`, remplacer `main: projet.envois.main` par `main: undefined` | `ajouter un envoi conserve la main du livre` |
| Retirer la ligne `$('btEnvoyer').disabled = …` | `le bouton des envois est éteint tant que la liste est vide` |

Annuler chaque mutation après l'avoir vue échouer.

- [ ] **Étape 8 : commit**

```bash
git add app/src app/tests
git commit -m "L'atelier écrit les envois, et n'oublie pas la main du livre"
```

---

## Tâche 8 : vérification d'ensemble

- [ ] **Étape 1 : la chaîne complète**

```bash
cd app/src-tauri && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
cd app && node --test "tests/*.test.js" && node --check src/app.js && node --check src/livraison.js && node --check src/couverture.js
cd app/src-tauri && cargo run --example temoin
```

Attendu : propre partout, et **98 pages, dos 7,21 mm**.

- [ ] **Étape 2 : le témoin porte un envoi**

Poser temporairement dans `examples/temoin.rs`, après la construction du projet :

```rust
    projet.meta.envois.liste = vec![ozalid_lib::envoi::Envoi {
        dedicataire: "Léa".into(),
        contenu: "À Léa, qui a lu la première version.".into(),
    }];
```

Cela ne changera **rien** au compte affiché, puisque `assembler` ne pose pas d'envoi —
c'est justement le point : le témoin doit rendre **98 pages**. Puis annuler.

La vraie mesure se fait à l'écran, à l'étape suivante.

- [ ] **Étape 3 : à l'écran**

```bash
caffeinate -u -t 1 && killall ScreenSaverEngine 2>/dev/null; cd app && cargo tauri dev
```

Rappel d'outillage : pour saisir du texte accentué dans la fenêtre, passer par le
presse-papiers (`osascript -e 'set the clipboard to "…"'` puis `Cmd+V`) — `keystroke`
déforme les accents.

À vérifier :

1. L'étape Livraison montre le bloc « Envois » sous les destinataires, main comprise.
2. Ajouter deux dédicataires, écrire deux mots différents, dont un avec un `#` et un
   accent.
3. Générer les envois : **le compte de pages et le dos sont identiques à ceux du
   package ordinaire**, pour les deux envois. C'est la mesure qui compte.
4. Ouvrir les deux PDF : chacun porte **son** mot en page 3, sous le titre, dans la
   main choisie — et rien d'autre n'a bougé.
   Vérifier aussi que « Voir la page » montre **la même chose** que le PDF : un aperçu
   qui divergerait de ce qui part à l'imprimeur serait pire que pas d'aperçu.
5. Changer la main, régénérer : les deux mots changent d'écriture.
6. Un dédicataire nommé `../essai` : le répertoire créé s'appelle `essai`, dans
   `envois/`, et nulle part ailleurs.
7. Un envoi volontairement long (dix lignes) : regarder ce que déborder veut dire, et
   juger si `width: 70%` et `dy: -28mm` tiennent. **C'est le moment de corriger ces
   deux valeurs** — la spec les donne comme point de départ.

Travailler sur une **copie** du `.ozalid`, jamais sur `build/projects/` : relever son
SHA-256 avant et après.

- [ ] **Étape 4 : compte rendu**

Écrire les valeurs relevées : compte de pages et dos du package ordinaire, puis des
deux envois ; les mutations vues échouer ; les valeurs typographiques finalement
retenues.
