# Les pièces du manuscrit — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Le manuscrit reconnaît, en plus des chapitres numérotés, des pièces liminaires (`## Préface`), des pièces annexes (`## Postface`) et des pages de partie à titre libre (`## Partie III - Avant Clément`), et les quatre sorties les composent.

**Architecture:** `manuscrit::Chapitre` devient `manuscrit::Piece`, porteur d'une `Sorte` qui dit à la fois ce qu'est la pièce et où elle se compose. `decoupe` valide trois zones dans l'ordre — liminaires, corps, annexes — et refuse tout le reste avec son numéro de ligne, comme aujourd'hui. Les quatre sorties (intérieur Typst, épreuve, EPUB, ebook) matchent sur la `Sorte` plutôt que de supposer un numéro.

**Tech Stack:** Rust 2021, Tauri 2, Typst en sidecar. Tests : `cargo test` depuis `app/src-tauri/`, plus `cargo run --example temoin` comme témoin de non-régression du compte de pages.

**Spec :** `docs/superpowers/specs/2026-08-22-pieces-du-manuscrit-design.md`

---

## Avertissement au relecteur

Le format du manuscrit est **volontairement fermé** : ce plan l'ouvre de quatre crans
nommés, pas plus. À aucune étape on n'assouplit le refus général — `## Chapitre premier`
doit rester une erreur à la fin du chantier, et le test qui le prouve
(`manuscrit.rs:463`) ne doit pas avoir été modifié.

Le témoin de non-régression est le juge de la composition : `cargo run --example temoin`
doit rendre **le même compte de pages qu'avant le chantier**. Relever ce compte à la
tâche 0 et le comparer à la tâche 7.

## Structure des fichiers

| Fichier | Responsabilité | Tâches |
|---|---|---|
| `app/src-tauri/src/manuscrit.rs` | Modèle `Piece`/`Sorte`, romains, découpage, refus | 1 → 4 |
| `app/src-tauri/src/interieur.rs` | Composition Typst du livre imprimé | 5 |
| `app/src-tauri/src/epreuve.rs` | Bandeaux de l'épreuve de relecture | 6 |
| `app/src-tauri/src/epub.rs` | XHTML, table des matières, vérification XML | 6 |
| `app/src-tauri/src/commands.rs` | Compte de chapitres affiché par l'interface | 4 |
| `app/src-tauri/src/{ebook,package}.rs` | Appelants, suivent le changement de type | 2 |

---

## Tâche 0 : Relever le témoin d'avant

**Files:** aucun.

- [ ] **Step 1: Composer le témoin et noter son compte de pages**

```bash
cd app/src-tauri && cargo run --example temoin
```

Attendu : une composition qui aboutit, et un compte de pages affiché en fin de sortie.
**Noter ce nombre** — il est la valeur de référence de la tâche 7. S'il ne s'affiche pas,
le chantier ne peut pas être vérifié : arrêter et le signaler.

---

## Tâche 1 : Les romains de partie

Une page de partie est numérotée `I`, `II`, `III`. On refuse les formes non canoniques
(`IIII`) : le romain est réimprimé tel qu'il est écrit, un parseur laxiste composerait
donc une page fautive.

**Files:**
- Modify: `app/src-tauri/src/manuscrit.rs` (nouvelles fonctions, avant `refus`)

- [ ] **Step 1: Écrire le test qui échoue**

Dans le `mod tests` de `manuscrit.rs` :

```rust
    /// Les romains sont réimprimés tels qu'écrits sur la page de partie : une forme
    /// non canonique composerait un livre fautif. Seule la forme qu'on écrirait à la
    /// main est admise.
    #[test]
    fn seuls_les_romains_canoniques_sont_lus() {
        assert_eq!(romain("I"), Some(1));
        assert_eq!(romain("IV"), Some(4));
        assert_eq!(romain("XIV"), Some(14));
        assert_eq!(romain("L"), Some(50));
        assert_eq!(romain("IIII"), None, "forme non canonique");
        assert_eq!(romain("VX"), None);
        assert_eq!(romain(""), None);
        assert_eq!(romain("3"), None);
    }
```

- [ ] **Step 2: Vérifier qu'il échoue**

```bash
cd app/src-tauri && cargo test seuls_les_romains_canoniques
```

Attendu : ÉCHEC à la compilation — `cannot find function romain in this scope`.

- [ ] **Step 3: Implémenter**

Dans `manuscrit.rs`, après la fonction `refus` :

```rust
/// Un entier → son romain, forme canonique.
///
/// Les parties d'un roman se comptent sur les doigts : la table s'arrête à `L`, et
/// au-delà c'est une faute de frappe, pas une intention.
fn en_romain(mut n: u32) -> String {
    const TABLE: [(u32, &str); 7] = [
        (50, "L"),
        (40, "XL"),
        (10, "X"),
        (9, "IX"),
        (5, "V"),
        (4, "IV"),
        (1, "I"),
    ];
    let mut s = String::new();
    for (v, sym) in TABLE {
        while n >= v {
            s.push_str(sym);
            n -= v;
        }
    }
    s
}

/// Un romain de partie → sa valeur, à condition qu'il soit écrit sous sa forme
/// canonique. `IIII` vaudrait 4 pour un parseur laxiste et s'imprimerait tel quel :
/// on le refuse plutôt que de composer une page de partie fautive.
fn romain(s: &str) -> Option<u32> {
    (1..=50).find(|n| en_romain(*n) == s)
}
```

- [ ] **Step 4: Vérifier qu'il passe**

```bash
cd app/src-tauri && cargo test seuls_les_romains_canoniques
```

Attendu : `test result: ok. 1 passed`.

- [ ] **Step 5: Commit**

```bash
git add app/src-tauri/src/manuscrit.rs
git commit -m "Le manuscrit sait lire un romain de partie"
```

---

## Tâche 2 : `Chapitre` devient `Piece`

Étape **mécanique et sans changement de comportement** : le type change, tout le reste
tient. Les tests existants doivent rester verts sans qu'on touche à leurs assertions,
seulement à la façon dont ils lisent un champ.

**Files:**
- Modify: `app/src-tauri/src/manuscrit.rs`
- Modify: `app/src-tauri/src/interieur.rs`, `epreuve.rs`, `epub.rs`, `ebook.rs`, `package.rs`, `commands.rs`

- [ ] **Step 1: Remplacer le type dans `manuscrit.rs`**

Remplacer la struct `Chapitre` (`manuscrit.rs:43-48`) par :

```rust
/// Ce qu'une pièce est, et où elle se compose. La position découle de la sorte : aucun
/// appelant n'a à la déduire du titre.
#[derive(Debug, Clone, PartialEq)]
pub enum Sorte {
    /// Un chapitre et son numéro, tel que le manuscrit l'écrit.
    Chapitre(u32),
    /// Une pièce qui précède le corps : préface, avant-propos, prologue.
    Liminaire,
    /// Une pièce qui le suit : épilogue, postface, remerciements.
    Annexe,
    /// Une page de partie et son romain, réimprimé tel qu'écrit.
    Partie(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Piece {
    pub sorte: Sorte,
    pub titre: String,
    pub blocs: Vec<Bloc>,
}

impl Piece {
    /// Le compte d'intégrité du projet et celui qu'affiche l'interface ne comptent que
    /// les chapitres : une préface n'est pas un chapitre en moins ni en plus.
    pub fn est_chapitre(&self) -> bool {
        matches!(self.sorte, Sorte::Chapitre(_))
    }
}
```

- [ ] **Step 2: Adapter `decoupe`, `entete` et `elague_rupture_finale`**

`elague_rupture_finale(ch: Option<&mut Piece>)` — seul le type change.

`entete` rend une `Piece` :

```rust
fn entete(reste: &str, no: usize) -> Result<Piece, String> {
    let (num, titre) = match reste.split_once('-') {
        Some((n, t)) => (n.trim(), t.trim()),
        None => (reste, ""),
    };
    let numero: u32 = num.parse().map_err(|_| {
        format!("ligne {no} : titre de chapitre « {reste} » (attendu : « NN - Titre »).")
    })?;
    Ok(Piece {
        sorte: Sorte::Chapitre(numero),
        titre: titre.to_string(),
        blocs: Vec::new(),
    })
}
```

Dans `decoupe`, la signature devient `-> Result<Vec<Piece>, String>` ; renommer la
variable locale `chapitres` en `pieces`. Le contrôle d'intégrité compte pour l'instant
`pieces.len()` — il sera corrigé à la tâche 4.

- [ ] **Step 3: Faire suivre les appelants**

Partout où `&[Chapitre]` apparaît, écrire `&[Piece]` ; partout où `ch.numero` est lu,
écrire un `match` provisoire qui ne traite que le chapitre :

- `interieur.rs:149` (`assemble`), `:258` (`source`), `:276` (`source_ebook`) : le
  paramètre `chapitres: &[Chapitre]` devient `pieces: &[Piece]`. Dans la boucle
  (`interieur.rs:206`), remplacer `ch.numero` par :

```rust
        let Sorte::Chapitre(numero) = &p.sorte else {
            unreachable!("les autres sortes arrivent à la tâche 5")
        };
```

- `epreuve.rs:130` et `epub.rs:102`, `:115`, `:442`, `:464`, `:740` : même traitement.
- `ebook.rs:70`, `package.rs:73` et `:234`, `commands.rs:513`, `:568`, `:1239` : seule
  la variable change de nom, `decoupe` rend déjà le bon type.
- Dans les `mod tests`, les littéraux `Chapitre { numero: 12, … }` deviennent
  `Piece { sorte: Sorte::Chapitre(12), … }`.

- [ ] **Step 4: Vérifier que rien n'a bougé**

```bash
cd app/src-tauri && cargo test && cargo clippy --all-targets -- -D warnings
```

Attendu : tous les tests passent, aucun avertissement. Aucune assertion n'a été
modifiée — seulement la façon de construire ou de lire une pièce.

- [ ] **Step 5: Commit**

```bash
git add app/src-tauri/src
git commit -m "Un chapitre n'est plus qu'une sorte de pièce"
```

---

## Tâche 3 : Les pièces liminaires et annexes

**Files:**
- Modify: `app/src-tauri/src/manuscrit.rs`

- [ ] **Step 1: Écrire les tests qui échouent**

```rust
    /// Le mot fait la pièce, et la pièce fait sa place : l'auteur n'a rien à déclarer.
    #[test]
    fn une_preface_est_une_piece_liminaire_et_une_postface_une_annexe() {
        let p = decoupe("## Préface\n\nA.\n\n## 01 - Un\n\nB.\n\n## Postface\n\nC.\n", None)
            .unwrap();
        assert_eq!(p[0].sorte, Sorte::Liminaire);
        assert_eq!(p[0].titre, "Préface");
        assert_eq!(p[1].sorte, Sorte::Chapitre(1));
        assert_eq!(p[2].sorte, Sorte::Annexe);
        assert_eq!(p[2].titre, "Postface");
    }

    /// La casse tapée ne doit pas ressortir à l'impression : le titre composé est celui
    /// de la liste. Les accents, eux, sont exigés — le projet est en français accentué,
    /// et un mot désaccentué est plus probablement une faute qu'une intention.
    #[test]
    fn le_mot_cle_est_insensible_a_la_casse_mais_pas_aux_accents() {
        let p = decoupe("## préface\n\nA.\n\n## 01 - Un\n\nB.\n", None).unwrap();
        assert_eq!(p[0].titre, "Préface", "le titre composé suit la liste");

        let err = decoupe("## Preface\n\nA.\n\n## 01 - Un\n\nB.\n", None).unwrap_err();
        assert!(err.contains("NN - Titre"), "{err}");
    }

    /// « Avant-propos » porte un tiret : reconnu après le découpage « NN - Titre », il
    /// deviendrait un chapitre de numéro « Avant ». La liste blanche passe donc avant.
    #[test]
    fn un_mot_cle_a_trait_d_union_n_est_pas_lu_comme_un_chapitre() {
        let p = decoupe("## Avant-propos\n\nA.\n\n## 01 - Un\n\nB.\n", None).unwrap();
        assert_eq!(p[0].sorte, Sorte::Liminaire);
        assert_eq!(p[0].titre, "Avant-propos");
    }

    /// La position découle du mot **et** doit être tenue : pas de réordonnancement
    /// silencieux d'un manuscrit dont l'auteur a mis la préface au milieu.
    #[test]
    fn une_piece_hors_de_sa_zone_est_refusee_avec_sa_ligne() {
        let err = decoupe("## 01 - Un\n\nA.\n\n## Préface\n\nB.\n", None).unwrap_err();
        assert!(err.contains("ligne 5"), "{err}");
        assert!(err.contains("Préface"), "{err}");

        let err = decoupe("## Postface\n\nA.\n\n## 01 - Un\n\nB.\n", None).unwrap_err();
        assert!(err.contains("ligne 5"), "{err}");
    }
```

- [ ] **Step 2: Vérifier qu'ils échouent**

```bash
cd app/src-tauri && cargo test manuscrit::tests
```

Attendu : les quatre nouveaux tests échouent — les trois premiers sur
`titre de chapitre « Préface »`, le quatrième parce qu'aucune erreur n'est levée.

- [ ] **Step 3: Implémenter**

Dans `manuscrit.rs`, après `romain` :

```rust
/// Les pièces qui précèdent le corps, et celles qui le suivent.
///
/// Liste **fermée** : c'est elle qui permet d'admettre un titre non numéroté sans
/// rouvrir le format. `## Chapitre premier` doit rester une erreur.
const LIMINAIRES: [&str; 3] = ["Préface", "Avant-propos", "Prologue"];
const ANNEXES: [&str; 3] = ["Épilogue", "Postface", "Remerciements"];

/// Un titre → la pièce qu'il nomme, s'il en nomme une.
///
/// Insensible à la casse, pas aux accents : le titre rendu est celui de la liste, pour
/// que ce qui s'imprime ne dépende pas de ce qui a été tapé.
fn mot_cle(titre: &str) -> Option<(Sorte, &'static str)> {
    let bas = titre.to_lowercase();
    if let Some(m) = LIMINAIRES.iter().find(|m| m.to_lowercase() == bas) {
        return Some((Sorte::Liminaire, m));
    }
    ANNEXES
        .iter()
        .find(|m| m.to_lowercase() == bas)
        .map(|m| (Sorte::Annexe, *m))
}
```

En tête de `entete`, avant le découpage `NN - Titre` :

```rust
    if let Some((sorte, mot)) = mot_cle(reste) {
        return Ok(Piece {
            sorte,
            titre: mot.to_string(),
            blocs: Vec::new(),
        });
    }
```

Dans `decoupe`, tenir l'état des zones. Déclarer avant la boucle :

```rust
    // Le manuscrit est trois zones dans cet ordre : liminaires, corps, annexes.
    let mut vu_corps = false;
    let mut vu_annexe = false;
```

et, juste après `let piece = entete(reste.trim(), no)?;` (remplaçant le `push` direct) :

```rust
        match piece.sorte {
            Sorte::Liminaire if vu_corps || vu_annexe => {
                return Err(format!(
                    "ligne {no} : « {} » est une pièce liminaire, elle ne peut pas suivre \
                     un chapitre.",
                    piece.titre
                ));
            }
            // Une pièce liminaire précède le corps sans l'ouvrir : deux liminaires se
            // suivent, et c'est le premier chapitre qui ferme la zone. Sans ce bras, un
            // liminaire légitime tomberait dans le `_` final et poserait `vu_corps`.
            Sorte::Liminaire => {}
            Sorte::Annexe => vu_annexe = true,
            _ if vu_annexe => {
                return Err(format!(
                    "ligne {no} : « {} » vient après une pièce annexe, qui ferme le livre.",
                    piece.titre
                ));
            }
            _ => vu_corps = true,
        }
        pieces.push(piece);
```

Le test qui protège ce bras :

```rust
    /// Une pièce liminaire n'ouvre pas le corps : elle le précède. Sans quoi une
    /// préface suivie d'un prologue — un manuscrit parfaitement ordinaire — serait
    /// refusée au motif que le prologue « suit un chapitre » qui n'existe pas.
    #[test]
    fn deux_pieces_liminaires_se_suivent() {
        let p = decoupe(
            "## Préface\n\nA.\n\n## Prologue\n\nB.\n\n## 01 - Un\n\nC.\n",
            None,
        )
        .unwrap();
        assert_eq!(p[0].sorte, Sorte::Liminaire);
        assert_eq!(p[1].sorte, Sorte::Liminaire);
        assert_eq!(p[2].sorte, Sorte::Chapitre(1));
    }
```

- [ ] **Step 4: Vérifier qu'ils passent**

```bash
cd app/src-tauri && cargo test manuscrit::tests
```

Attendu : tous verts, y compris `un_entete_mal_forme_est_refuse` (`manuscrit.rs:463`),
qui n'a pas été touché.

- [ ] **Step 5: Commit**

```bash
git add app/src-tauri/src/manuscrit.rs
git commit -m "Une préface entre dans le manuscrit sans ouvrir le format"
```

---

## Tâche 4 : Les pages de partie, et le compte qui ne les compte pas

**Files:**
- Modify: `app/src-tauri/src/manuscrit.rs`
- Modify: `app/src-tauri/src/commands.rs:1239`
- Modify: `app/src-tauri/src/epreuve.rs:122`

- [ ] **Step 1: Écrire les tests qui échouent**

```rust
    /// Un titre libre est indiscernable d'un chapitre mal formé : la page de partie se
    /// marque donc explicitement, et son romain se vérifie comme le reste.
    #[test]
    fn une_page_de_partie_porte_un_romain_et_un_titre_libre() {
        let p = decoupe(
            "## Partie I - Avant Clément\n\n## 01 - Un\n\nA.\n\n\
             ## Partie II - Après Clément\n\n## 02 - Deux\n\nB.\n",
            None,
        )
        .unwrap();
        assert_eq!(p[0].sorte, Sorte::Partie("I".into()));
        assert_eq!(p[0].titre, "Avant Clément");
        assert_eq!(p[2].sorte, Sorte::Partie("II".into()));
    }

    /// Comme `## 7`, une partie peut n'avoir que son numéro.
    #[test]
    fn une_page_de_partie_sans_titre_est_admise() {
        let p = decoupe("## Partie I\n\n## 01 - Un\n\nA.\n", None).unwrap();
        assert_eq!(p[0].sorte, Sorte::Partie("I".into()));
        assert_eq!(p[0].titre, "");
    }

    /// Une partie sautée est une partie perdue en route, et elle ne se verrait qu'au
    /// tirage.
    #[test]
    fn un_romain_de_partie_qui_ne_suit_pas_est_refuse() {
        let md = "## Partie I - Un\n\n## 01 - Un\n\nA.\n\n## Partie IV - Quatre\n\n\
                  ## 02 - Deux\n\nB.\n";
        let err = decoupe(md, None).unwrap_err();
        assert!(err.contains("ligne 7"), "{err}");
        assert!(err.contains("II"), "l'erreur doit dire ce qui était attendu : {err}");

        let err = decoupe("## Partie X - Dix\n\n## 01 - Un\n\nA.\n", None).unwrap_err();
        assert!(err.contains("ligne 1"), "{err}");
    }

    /// Une page de partie ne porte que son titre : un paragraphe écrit là serait
    /// silencieusement perdu à la composition, ce que le format refuse partout ailleurs.
    #[test]
    fn du_texte_sous_une_page_de_partie_est_refuse() {
        let err = decoupe("## Partie I - Un\n\nDu texte.\n\n## 01 - Un\n\nA.\n", None)
            .unwrap_err();
        assert!(err.contains("ligne 3"), "{err}");
    }

    /// Le contrôle d'intégrité du projet dit un nombre de **chapitres** : une préface
    /// ajoutée au manuscrit ne doit pas faire croire à un chapitre de plus.
    #[test]
    fn le_controle_d_integrite_ne_compte_que_les_chapitres() {
        let md = "## Préface\n\nA.\n\n## Partie I - Un\n\n## 01 - Un\n\nB.\n\n\
                  ## 02 - Deux\n\nC.\n\n## Postface\n\nD.\n";
        let p = decoupe(md, Some(2)).expect("deux chapitres, pièces en plus");
        assert_eq!(p.len(), 5);
        assert_eq!(p.iter().filter(|p| p.est_chapitre()).count(), 2);
        assert!(decoupe(md, Some(5)).is_err(), "les pièces ne sont pas des chapitres");
    }
```

- [ ] **Step 2: Vérifier qu'ils échouent**

```bash
cd app/src-tauri && cargo test manuscrit::tests
```

Attendu : les cinq nouveaux échouent — les quatre premiers sur
`titre de chapitre « Partie I - … »`, le dernier sur `5 chapitres attendus`.

- [ ] **Step 3: Implémenter**

Dans `entete`, après le bloc `mot_cle` et avant le découpage `NN - Titre` :

```rust
    if let Some(apres) = reste.strip_prefix("Partie ") {
        let (num, titre) = match apres.split_once('-') {
            Some((n, t)) => (n.trim(), t.trim()),
            None => (apres.trim(), ""),
        };
        if romain(num).is_none() {
            return Err(format!(
                "ligne {no} : « {num} » n'est pas un numéro de partie (attendu : I, II, \
                 III…)."
            ));
        }
        return Ok(Piece {
            sorte: Sorte::Partie(num.to_string()),
            titre: titre.to_string(),
            blocs: Vec::new(),
        });
    }
```

Dans `decoupe`, la consécutivité. Déclarer avant la boucle :

```rust
    // Les parties se suivent depuis I : une partie sautée ne se verrait qu'au tirage.
    let mut derniere_partie = 0;
```

et, dans le `match` des zones ajouté à la tâche 3, avant `_ => vu_corps = true` :

```rust
            Sorte::Partie(ref r) if vu_annexe => {
                return Err(format!(
                    "ligne {no} : « Partie {r} » vient après une pièce annexe, qui ferme \
                     le livre."
                ));
            }
            Sorte::Partie(ref r) => {
                // `entete` a déjà refusé ce qui n'est pas un romain canonique.
                let n = romain(r).expect("romain validé par entete");
                if n != derniere_partie + 1 {
                    return Err(format!(
                        "ligne {no} : partie {r} après la partie {}, attendu {}.",
                        en_romain(derniere_partie),
                        en_romain(derniere_partie + 1)
                    ));
                }
                derniere_partie = n;
                vu_corps = true;
            }
```

Le refus du texte sous une partie, dans la branche paragraphe de `decoupe`
(`manuscrit.rs:240`) :

```rust
        } else if let Some(courant) = pieces.last_mut() {
            if let Sorte::Partie(r) = &courant.sorte {
                return Err(format!(
                    "ligne {no} : du texte sous « Partie {r} » — une page de partie ne \
                     porte que son titre."
                ));
            }
            courant.blocs.push(Bloc::Paragraphe(t.to_string()));
```

Le contrôle d'intégrité, en fin de `decoupe` :

```rust
    if let Some(n) = attendu {
        let trouves = pieces.iter().filter(|p| p.est_chapitre()).count() as u32;
        if trouves != n {
            return Err(format!(
                "{n} chapitres attendus (projet), {trouves} trouvés."
            ));
        }
    }
```

- [ ] **Step 4: Corriger les deux comptes affichés**

`commands.rs:1239` — l'interface signale un manuscrit périmé sur l'écart entre ce compte
et `livre.chapitres` : une préface le ferait mentir.

```rust
    let chapitres_trouves = manuscrit::decoupe(&o.projet.texte, None)
        .map(|p| p.iter().filter(|p| p.est_chapitre()).count() as u32)
        .unwrap_or(0);
```

`epreuve.rs:122` — `nb_chapitres = chapitres.len()` devient :

```rust
        nb_chapitres = pieces.iter().filter(|p| p.est_chapitre()).count(),
```

- [ ] **Step 5: Vérifier**

```bash
cd app/src-tauri && cargo test && cargo clippy --all-targets -- -D warnings
```

Attendu : tout vert, aucun avertissement.

- [ ] **Step 6: Commit**

```bash
git add app/src-tauri/src
git commit -m "Une page de partie s'ouvre sur un romain qui se suit"
```

---

## Tâche 5 : La composition de l'intérieur

**Files:**
- Modify: `app/src-tauri/src/interieur.rs` (`assemble` `:144-250`, `liminaires` `:291`)

- [ ] **Step 1: Écrire les tests qui échouent**

Dans le `mod tests` de `interieur.rs`, à la suite des tests de dédicace :

```rust
    /// La préface est une pièce liminaire : elle se compose avant le rétablissement du
    /// folio, donc ses pages n'en portent pas — la règle validée au cadrage.
    #[test]
    fn une_preface_se_compose_avant_le_folio() {
        let mut pieces = vec![Piece {
            sorte: Sorte::Liminaire,
            titre: "Préface".into(),
            blocs: vec![Bloc::Paragraphe("Entrez.".into())],
        }];
        pieces.extend(chapitres());
        let s = source(&livre(), &Interieur::default(), provider("lulu").unwrap(),
                       &Reglage { gouttiere: 25.0, blanche: false }, &pieces, None);
        let preface = s.find("Préface").expect("la préface doit être composée");
        let folio = s.find("#set page(footer: context").expect("le folio du corps");
        assert!(preface < folio, "la préface passe après le rétablissement du folio");
        assert!(s.contains("Entrez."), "le texte de la préface est perdu");
    }

    /// Une page de partie prend une belle page au verso blanc, sans folio : deux
    /// `#page(footer: none)`. Et comme `#page` rompt le flux de lui-même, le chapitre
    /// qui suit ne doit pas ajouter un `#pagebreak()` — il laisserait une page blanche
    /// de plus, invisible à la lecture du code et payée au tirage.
    #[test]
    fn une_page_de_partie_prend_une_belle_page_sans_folio_et_sans_saut_en_trop() {
        let mut pieces = chapitres();
        pieces.insert(1, Piece {
            sorte: Sorte::Partie("I".into()),
            titre: "Avant Clément".into(),
            blocs: Vec::new(),
        });
        let pr = provider("lulu").unwrap();
        let r = Reglage { gouttiere: 25.0, blanche: false };
        let avec = source(&livre(), &Interieur::default(), pr, &r, &pieces, None);
        let sans = source(&livre(), &Interieur::default(), pr, &r, &chapitres(), None);
        assert_eq!(
            avec.matches("#page(footer: none)").count(),
            sans.matches("#page(footer: none)").count() + 2,
            "la partie doit ajouter exactement deux pages sans folio"
        );
        assert_eq!(
            avec.matches("#pagebreak()").count(),
            sans.matches("#pagebreak()").count(),
            "le chapitre qui suit la partie ne doit pas ajouter de saut"
        );
        assert!(avec.contains("AVANT CLÉMENT") || avec.contains("Avant Clément"));
    }

    /// Le folio appartient au corps : une postface n'en porte pas, comme la préface.
    #[test]
    fn une_annexe_se_compose_sans_folio() {
        let mut pieces = chapitres();
        pieces.push(Piece {
            sorte: Sorte::Annexe,
            titre: "Postface".into(),
            blocs: vec![Bloc::Paragraphe("Après coup.".into())],
        });
        let s = source(&livre(), &Interieur::default(), provider("lulu").unwrap(),
                       &Reglage { gouttiere: 25.0, blanche: false }, &pieces, None);
        let coupe = s.find("#set page(footer: none)").expect("le folio doit être coupé");
        let postface = s.find("Postface").expect("la postface doit être composée");
        assert!(coupe < postface, "la postface se compose avant la coupure du folio");
        assert!(s.contains("Après coup."));
    }
```

Notes :
- `chapitres()` est la fabrique de test existante (`interieur.rs:408`) ; elle rend
  désormais un `Vec<Piece>` d'un seul chapitre.
- `provider("lulu").unwrap()` et `gouttiere: 25.0` sont les valeurs des tests voisins
  (`interieur.rs:751`) — ne pas en inventer d'autres.
- Les trois tests de dédicace appellent `liminaires(&livre(), None)` : leur **appel**
  gagne un troisième argument `&[]`, leurs assertions ne changent pas.

- [ ] **Step 2: Vérifier qu'ils échouent**

```bash
cd app/src-tauri && cargo test interieur::tests
```

Attendu : les trois échouent — la préface et la postface ne sont composées nulle part
(le `unreachable!()` de la tâche 2 panique, ou le texte est absent).

- [ ] **Step 3: Factoriser le gabarit d'une pièce à texte**

Dans `interieur.rs`, près de `majuscules` :

```rust
/// L'ouverture d'une pièce à texte — préface, postface.
///
/// Le mot occupe la ligne du numéro, mais composé comme un **titre** de chapitre : ce
/// sont la casse et l'espacement qui font le titre, les 13 pt du gabarit étant la
/// taille d'un chiffre isolé. Le blanc de 14,5 mm est la somme des deux blancs du
/// gabarit (3,5 + 11) : le texte s'ouvre à la même hauteur que celui d'un chapitre.
fn ouverture_piece(titre: &str) -> String {
    format!(
        "#v(22mm)\n#align(center, text(size: 10pt, tracking: 0.14em)[{}])\n#v(14.5mm)\n",
        majuscules(titre)
    )
}

/// Les blocs d'une pièce, composés. Partagé par les chapitres et les pièces à texte :
/// une préface se lit dans la même page qu'un chapitre.
fn blocs_typst(blocs: &[Bloc]) -> String {
    let mut s = String::new();
    for b in blocs {
        match b {
            Bloc::Paragraphe(p) => {
                s.push_str(&inline(p));
                s.push_str("\n\n");
            }
            Bloc::Scene => {
                s.push_str(&format!("#v(1em)\n#align(center)[{SCENE}]\n#v(1em)\n\n"))
            }
        }
    }
    s
}
```

Remplacer la boucle `for b in &ch.blocs` de `assemble` (`interieur.rs:223-241`) par un
appel à `blocs_typst`. Les commentaires qui expliquent le blanc en em restent sur
`blocs_typst`.

- [ ] **Step 4: Découper les trois zones dans `assemble`**

Au début du corps de `assemble`, après la ligne des réglages :

```rust
    // Les zones sont déjà validées par `decoupe` : le découpage n'a qu'à les suivre.
    let lim = pieces
        .iter()
        .take_while(|p| matches!(p.sorte, Sorte::Liminaire))
        .count();
    let (liminaires_manuscrit, reste) = pieces.split_at(lim);
    let corps = reste
        .iter()
        .take_while(|p| !matches!(p.sorte, Sorte::Annexe))
        .count();
    let (corps, annexes) = reste.split_at(corps);
```

`liminaires(livre, envoi)` devient `liminaires(livre, envoi, liminaires_manuscrit)`, et
compose, après la dédicace :

```rust
    // Les pièces liminaires du manuscrit ferment la série : `footer: none` court encore,
    // le folio ne sera rétabli qu'au premier chapitre.
    for p in pieces {
        s.push_str(&ouverture_piece(&p.titre));
        s.push_str(&blocs_typst(&p.blocs));
        s.push_str("#pagebreak()\n\n");
    }
```

- [ ] **Step 5: Composer le corps et les annexes**

Remplacer la boucle `for (i, ch) in chapitres.iter().enumerate()` par :

```rust
    // `#page(…)[…]` rompt le flux de lui-même, avant et après : après une page de
    // partie, le `#pagebreak()` d'ouverture du chapitre suivant ferait une page blanche
    // de plus. Le compte de pages est le seul juge de ce détail.
    let mut apres_page = false;
    for (i, p) in corps.iter().enumerate() {
        match &p.sorte {
            Sorte::Partie(r) => {
                s.push_str(&format!(
                    "#page(footer: none)[\n#v(22mm)\n\
                     #align(center, text(size: 13pt)[{r}])\n"
                ));
                if !p.titre.is_empty() {
                    s.push_str(&format!(
                        "#v(3.5mm)\n\
                         #align(center, text(size: 10pt, tracking: 0.14em)[{}])\n",
                        majuscules(&p.titre)
                    ));
                }
                s.push_str("]\n#page(footer: none)[]\n");
                apres_page = true;
            }
            Sorte::Chapitre(numero) => {
                if i > 0 && !apres_page {
                    s.push_str("#pagebreak()\n");
                }
                s.push_str(&format!(
                    "#v(22mm)\n#align(center, text(size: 13pt)[{numero}])\n"
                ));
                if !p.titre.is_empty() {
                    s.push_str(&format!(
                        "#v(3.5mm)\n\
                         #align(center, text(size: 10pt, tracking: 0.14em)[{}])\n",
                        majuscules(&p.titre)
                    ));
                }
                s.push_str("#v(11mm)\n");
                s.push_str(&blocs_typst(&p.blocs));
                apres_page = false;
            }
            // `decoupe` garantit les zones : ni liminaire ni annexe n'entre dans le corps.
            Sorte::Liminaire | Sorte::Annexe => unreachable!("zone validée par decoupe"),
        }
    }

    // Les annexes rejoignent les liminaires hors du folio : il appartient au corps.
    if !annexes.is_empty() {
        if !apres_page {
            s.push_str("#pagebreak()\n");
        }
        s.push_str("#set page(footer: none)\n");
        for (i, p) in annexes.iter().enumerate() {
            if i > 0 {
                s.push_str("#pagebreak()\n");
            }
            s.push_str(&ouverture_piece(&p.titre));
            s.push_str(&blocs_typst(&p.blocs));
        }
    }
```

- [ ] **Step 6: Vérifier**

```bash
cd app/src-tauri && cargo test interieur:: && cargo clippy --all-targets -- -D warnings
```

Attendu : les trois nouveaux tests passent, **et** les tests existants — en particulier
`le_premier_chapitre_n_ajoute_pas_de_saut_de_page` (`:749`), `la_blanche_de_fin_est_sans_folio`
(`:567`) et les trois tests de dédicace (`:787`, `:808`, `:825`).

- [ ] **Step 7: Vérifier le témoin — le compte de pages ne doit pas bouger**

```bash
cd app/src-tauri && cargo run --example temoin
```

Attendu : **exactement** le compte relevé à la tâche 0. Le manuscrit témoin ne porte
aucune pièce : un écart d'une seule page signifie que le chemin nominal a bougé.

- [ ] **Step 8: Commit**

```bash
git add app/src-tauri/src/interieur.rs
git commit -m "Le livre composé accueille la préface et les pages de partie"
```

---

## Tâche 6 : L'épreuve et l'EPUB

**Files:**
- Modify: `app/src-tauri/src/epreuve.rs:125-135`
- Modify: `app/src-tauri/src/epub.rs:102`, `:115`, `:740`

- [ ] **Step 1: Écrire les tests qui échouent**

Dans `epreuve.rs` :

```rust
    /// Une pièce se relit comme le reste — mais son bandeau ne peut pas porter un
    /// numéro de chapitre qu'elle n'a pas.
    #[test]
    fn le_bandeau_d_une_piece_ne_porte_pas_de_numero() {
        let pieces = vec![
            Piece {
                sorte: Sorte::Liminaire,
                titre: "Préface".into(),
                blocs: vec![Bloc::Paragraphe("Entrez.".into())],
            },
            Piece {
                sorte: Sorte::Partie("III".into()),
                titre: "Avant Clément".into(),
                blocs: Vec::new(),
            },
        ];
        let s = source(&livre(), &Interieur::default(), &pieces, 12.0);
        assert!(s.contains("= Préface"), "{s}");
        assert!(s.contains("= Partie III — Avant Clément"), "{s}");
    }
```

Dans `epub.rs` :

```rust
    /// Le `<span class="numero">` dit le rang d'un chapitre : une préface n'en a pas,
    /// et une liseuse afficherait un numéro inventé.
    #[test]
    fn une_piece_liminaire_n_emet_pas_de_numero() {
        let p = Piece {
            sorte: Sorte::Liminaire,
            titre: "Préface".into(),
            blocs: vec![Bloc::Paragraphe("Entrez.".into())],
        };
        let x = piece_xhtml(&p);
        assert!(!x.contains(r#"class="numero""#), "{x}");
        assert!(x.contains(r#"<span class="titre">Préface</span>"#), "{x}");
        assert!(x.contains("<p>Entrez.</p>"), "{x}");
    }

    /// Toutes les pièces sont dans la table des matières : un lecteur doit pouvoir
    /// sauter à la préface comme à un chapitre.
    #[test]
    fn toutes_les_pieces_figurent_a_la_table_des_matieres() {
        let pieces = vec![
            Piece {
                sorte: Sorte::Liminaire,
                titre: "Préface".into(),
                blocs: vec![Bloc::Paragraphe("A.".into())],
            },
            Piece {
                sorte: Sorte::Partie("I".into()),
                titre: "Avant Clément".into(),
                blocs: Vec::new(),
            },
            Piece {
                sorte: Sorte::Chapitre(1),
                titre: "Un".into(),
                blocs: vec![Bloc::Paragraphe("B.".into())],
            },
        ];
        let nav = nav_xhtml(&pieces);
        assert!(nav.contains("Préface"), "{nav}");
        assert!(nav.contains("I — Avant Clément"), "{nav}");
        assert!(nav.contains("1 — Un"), "{nav}");
    }
```

- [ ] **Step 2: Vérifier qu'ils échouent**

```bash
cd app/src-tauri && cargo test epreuve:: epub::
```

Attendu : les trois échouent (fonction `piece_xhtml` inconnue, bandeaux sans le bon
libellé).

- [ ] **Step 3: Implémenter l'épreuve**

Remplacer le calcul de `titre_ch` (`epreuve.rs:130-134`) par :

```rust
        let titre_ch = match &p.sorte {
            Sorte::Chapitre(n) if p.titre.is_empty() => n.to_string(),
            Sorte::Chapitre(n) => format!("{n} — {}", echappe(&p.titre)),
            Sorte::Partie(r) if p.titre.is_empty() => format!("Partie {r}"),
            Sorte::Partie(r) => format!("Partie {r} — {}", echappe(&p.titre)),
            // Le titre d'un liminaire ou d'une annexe est son mot-clé : il vient de la
            // liste, pas du manuscrit, mais rien n'oblige à le croire sur parole.
            Sorte::Liminaire | Sorte::Annexe => echappe(&p.titre),
        };
```

- [ ] **Step 4: Implémenter l'EPUB**

`intitule` (`epub.rs:102`) :

```rust
/// Le titre d'une pièce tel qu'il paraît dans la table des matières. Le mot « Partie »
/// n'y figure pas : le romain suffit à distinguer une ouverture de partie d'un chapitre.
fn intitule(p: &Piece) -> String {
    match &p.sorte {
        Sorte::Chapitre(n) if p.titre.is_empty() => n.to_string(),
        Sorte::Chapitre(n) => format!("{n} — {}", p.titre),
        Sorte::Partie(r) if p.titre.is_empty() => r.clone(),
        Sorte::Partie(r) => format!("{r} — {}", p.titre),
        Sorte::Liminaire | Sorte::Annexe => p.titre.clone(),
    }
}
```

`chapitre_xhtml` devient `piece_xhtml` (`epub.rs:115`) :

```rust
fn piece_xhtml(p: &Piece) -> String {
    let mut corps = String::from("<h1>");
    match &p.sorte {
        Sorte::Chapitre(n) => corps.push_str(&format!(r#"<span class="numero">{n}</span>"#)),
        Sorte::Partie(r) => corps.push_str(&format!(r#"<span class="numero">{r}</span>"#)),
        Sorte::Liminaire | Sorte::Annexe => {}
    }
    if !p.titre.is_empty() {
        corps.push_str(&format!(
            r#"<span class="titre">{}</span>"#,
            echappe(&p.titre)
        ));
    }
    corps.push_str("</h1>\n");
    corps.push_str(&blocs_xhtml(&p.blocs));
    page(&intitule(p), &corps)
}
```

où `blocs_xhtml` est la boucle `for b in &ch.blocs` de `chapitre_xhtml`, extraite telle
quelle :

```rust
/// Les blocs d'une pièce, en XHTML. Une page de partie n'en a aucun : elle ne rend
/// alors que son `<h1>`.
fn blocs_xhtml(blocs: &[Bloc]) -> String {
    let mut s = String::new();
    for b in blocs {
        match b {
            Bloc::Paragraphe(p) => s.push_str(&format!("<p>{}</p>\n", paragraphe(p))),
            Bloc::Scene => s.push_str(&format!("<p class=\"scene\">{SCENE_XHTML}</p>\n")),
        }
    }
    s
}
```

`nav_xhtml` (`:442`), `ncx` (`:464`) et `contenu` (`:326`) prennent `&[Piece]` : seuls
les types et le nom de la variable changent, `intitule` fait déjà le reste.

La vérification XML (`epub.rs:739-746`) désigne la pièce par son intitulé :

```rust
    for p in pieces {
        let ou = format!("la pièce « {} »", intitule(p));
        verifie_xml(&p.titre, &ou)?;
        for b in &p.blocs {
            if let Bloc::Paragraphe(t) = b {
                verifie_xml(t, &ou)?;
            }
        }
    }
```

- [ ] **Step 5: Vérifier**

```bash
cd app/src-tauri && cargo test && cargo clippy --all-targets -- -D warnings
```

Attendu : tout vert, aucun avertissement.

- [ ] **Step 6: Commit**

```bash
git add app/src-tauri/src
git commit -m "L'épreuve et l'ebook nomment les pièces au lieu de les numéroter"
```

---

## Tâche 7 : Vérification de bout en bout

**Files:** aucun — sauf correctif si un écart apparaît.

- [ ] **Step 1: La suite complète**

```bash
cd app/src-tauri && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
cd app && node --test tests/*.test.js
```

Attendu : tout vert. Si `cargo fmt --check` échoue, lancer `cargo fmt` et reprendre.

- [ ] **Step 2: Le témoin, comparé à la tâche 0**

```bash
cd app/src-tauri && cargo run --example temoin
```

Attendu : **le même compte de pages qu'à la tâche 0**, au chiffre près. Un écart est un
échec du chantier, pas un détail : le manuscrit témoin ne porte aucune pièce, donc rien
ne devait bouger pour lui.

- [ ] **Step 3: L'essai qui a motivé le chantier**

Lancer l'application, importer `build/in/texts/WIP8.md`, et vérifier :

1. l'import ne renvoie plus `ligne 7 : titre de chapitre « Préface »` ;
2. l'onglet Livre affiche **64 chapitres trouvés**, pas 65 — la préface n'est pas un
   chapitre ;
3. l'intérieur composé porte la préface avant le chapitre 1, sans folio sur ses pages ;
4. le dos a changé par rapport à l'ancien compte : c'est attendu, la préface ajoute des
   pages et le dos découle de la pagination mesurée.

Le point 2 est le plus facile à rater et le plus silencieux : s'il affiche 65, c'est que
`commands.rs:1239` (tâche 4, étape 4) n'a pas été corrigé.

- [ ] **Step 4: Commit final s'il reste quelque chose**

```bash
git status
```

Attendu : arbre propre. Sinon, committer le correctif avec un message qui dit ce qui
avait été manqué.
