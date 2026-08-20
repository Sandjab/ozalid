# Release Windows par la CI — plan d'implémentation

> **Pour un agent exécutant :** SOUS-COMPÉTENCE REQUISE — utiliser
> `superpowers:subagent-driven-development` (recommandé) ou
> `superpowers:executing-plans` pour dérouler ce plan tâche par tâche. Les étapes
> sont cochables (`- [ ]`).

**Objectif :** établir par intégration continue que Windows compile, teste et
**pagine à l'identique** de macOS, puis produire sur tag un installeur NSIS
attaché à une release GitHub draft.

**Architecture :** un manuscrit-témoin du domaine public versionné dans le
dépôt, un exemple Rust (`temoin`) qui le compose et refuse tout écart avec une
pagination figée, et un workflow à deux jobs — `verifier` sur chaque push,
`publier` sur tag `v*`.

**Pile :** GitHub Actions (`windows-latest`), Rust + Tauri 2, Typst 0.15.1 en
sidecar, Git Bash pour réutiliser `app/outils/*.sh`.

**Spec :** `docs/superpowers/specs/2026-08-20-release-windows-ci-design.md`

---

## Fichiers touchés

| Fichier | Rôle |
|---|---|
| Créer `.gitattributes` | Fins de ligne LF forcées sur `*.md` : sans lui, un écart de pagination sous Windows serait indistinguable d'une conversion CRLF de git |
| Créer `app/src-tauri/temoin/manuscrit.md` | *Candide*, domaine public, mis au format du projet. Le seul livre que la CI puisse composer |
| Créer `app/src-tauri/examples/temoin.rs` | Compose le témoin au gabarit `bod` et compare la pagination à une constante figée |
| Modifier `app/src-tauri/src/manuscrit.rs` | Deux tests sur le témoin : il est composable, et il ne porte pas de `\r` |
| Modifier `app/outils/typst.sh` | Branche `*windows*` : repli sur `tar -xf` quand `unzip` manque |
| Créer `.github/workflows/windows.yml` | Jobs `verifier` et `publier` |
| Modifier `app/README.md` | État du jalon, installation sous Windows, le témoin dans les vérifications |

---

## Tâche 1 : les fins de ligne, avant tout le reste

**Fichiers :**
- Créer : `.gitattributes`

Cette tâche vient en premier parce que le manuscrit-témoin sera ajouté à la
tâche 2 : s'il entre dans l'index avant la règle, il pourra y entrer avec des
CRLF.

- [ ] **Étape 1 : écrire le fichier**

`.gitattributes` à la racine du dépôt :

```
# Le manuscrit-témoin est composé sur macOS et sur Windows, et les deux comptes de
# pages doivent coïncider. Sans fins de ligne normalisées, un checkout Windows
# pourrait livrer un texte différent octet pour octet, et l'on ne saurait plus dire
# si un écart de pagination vient de Typst ou de git.
*.md text eol=lf
*.rs text eol=lf
*.toml text eol=lf
*.sh text eol=lf
*.yml text eol=lf
```

- [ ] **Étape 2 : vérifier qu'aucun fichier suivi ne change**

```bash
cd /Users/jean-paulgavini/Documents/Dev/ozalid
git add --renormalize .
git status --short
```

Attendu : **aucune sortie**. Le dépôt est déjà en LF ; si des fichiers
apparaissent, les examiner avant de continuer — une normalisation massive n'est
pas l'objet de ce plan.

- [ ] **Étape 3 : commit**

```bash
git add .gitattributes
git commit -m "Le manuscrit ne doit pas changer d'octets en changeant de plateforme"
```

---

## Tâche 2 : le manuscrit-témoin

**Fichiers :**
- Créer : `app/src-tauri/temoin/manuscrit.md`

Le texte est *Candide* (Voltaire, 1759), domaine public. La conversion se fait
une fois, par un script jetable qui n'est **pas** versionné : seul son produit
l'est.

- [ ] **Étape 1 : récupérer le texte source**

Travailler dans le répertoire scratchpad de la session — rien de ce qui suit,
hormis le manuscrit final, n'entre dans le dépôt.

```bash
SCRATCH="$(mktemp -d)"; cd "$SCRATCH"; echo "$SCRATCH"
curl -fL https://www.gutenberg.org/cache/epub/4650/pg4650.txt -o candide-brut.txt
wc -l candide-brut.txt
```

Attendu : quelques milliers de lignes. Si l'identifiant 4650 ne rend pas
*Candide* en français, chercher l'édition française sur
`https://www.gutenberg.org/ebooks/search/?query=candide+voltaire&l=fr` et
reprendre avec le bon numéro.

- [ ] **Étape 2 : relever la forme réelle du fichier**

Le script de conversion dépend de la façon dont cette édition marque ses
chapitres et ses limites. Les relever plutôt que les supposer :

```bash
grep -n "\*\*\* START\|\*\*\* END" candide-brut.txt
grep -n "^CHAPITRE" candide-brut.txt | head -35
grep -c "^CHAPITRE" candide-brut.txt
```

Attendu : deux marqueurs Gutenberg, et **30 en-têtes de chapitre**. Noter les
numéros de ligne des marqueurs et la forme exacte des en-têtes (« CHAPITRE
PREMIER », « CHAPITRE SECOND », « CHAPITRE III », etc.) : l'étape suivante s'y
adosse.

- [ ] **Étape 3 : écrire le script de conversion**

Dans le scratchpad, `convertir.py`. Adapter les expressions régulières à ce que
l'étape 2 a montré — le squelette ci-dessous suppose des en-têtes en
`^CHAPITRE …` suivis, sur une ligne voisine, du titre du chapitre.

```python
#!/usr/bin/env python3
"""Candide (Gutenberg) → manuscrit au format Ozalid.

Format admis, et lui seul (voir app/src-tauri/src/manuscrit.rs) :
titre en « # », chapitres en « ## NN - Titre », paragraphes séparés par une ligne
vide. Refusés par la chaîne : listes, citations « > », tableaux « | », accents
graves, images. Le script les élimine ou échoue.
"""
import re
import sys
import unicodedata

brut = open("candide-brut.txt", encoding="utf-8").read()

# 1. Retirer l'en-tête et la licence Gutenberg : ils ne couvrent pas l'œuvre,
#    qui est du domaine public, mais ils ne sont pas du texte de Voltaire.
debut = brut.index("*** START")
fin = brut.index("*** END")
corps = brut[brut.index("\n", debut) + 1 : fin]

# 2. Découper aux en-têtes de chapitre.
morceaux = re.split(r"\n(?=CHAPITRE\b)", corps)
liminaires, chapitres = morceaux[0], morceaux[1:]
if len(chapitres) != 30:
    sys.exit(f"{len(chapitres)} chapitres trouvés, 30 attendus")

def paragraphes(bloc):
    """Lignes repliées par Gutenberg → un paragraphe par ligne."""
    out = []
    for p in re.split(r"\n\s*\n", bloc):
        p = " ".join(l.strip() for l in p.splitlines() if l.strip())
        if p:
            out.append(p)
    return out

def nettoie(p):
    # Apostrophe typographique : c'est celle qu'un livre imprime.
    p = p.replace("'", "’")
    # Le texte ne doit porter aucune construction que `refus()` rejette.
    p = p.replace("`", "’").replace("|", "—")
    return unicodedata.normalize("NFC", p)

sortie = ["# Candide", ""]
for i, ch in enumerate(chapitres, start=1):
    ps = paragraphes(ch)
    entete = ps[0]                      # « CHAPITRE PREMIER » + titre
    titre = re.sub(r"^CHAPITRE\s+\S+\.?\s*", "", entete).strip(" .")
    titre = nettoie(titre) or f"Chapitre {i}"
    sortie.append(f"## {i:02d} - {titre}")
    sortie.append("")
    for p in ps[1:]:
        p = nettoie(p)
        if p.startswith(("- ", "+ ", "* ", "> ", "|", "![", "###")):
            sys.exit(f"chapitre {i} : paragraphe non composable — « {p[:60]} »")
        sortie.append(p)
        sortie.append("")

open("manuscrit.md", "w", encoding="utf-8", newline="\n").write("\n".join(sortie))
print(f"{len(chapitres)} chapitres, {sum(len(l.split()) for l in sortie)} mots")
```

- [ ] **Étape 4 : convertir et contrôler à l'œil**

```bash
python3 convertir.py
head -20 manuscrit.md
grep -c "^## " manuscrit.md
grep -n "	\|  $" manuscrit.md | head
```

Attendu : « 30 chapitres, ~35000 mots », trente `## `, et aucune tabulation ni
espace en fin de ligne. Lire les vingt premières lignes : le titre, le premier
chapitre, un paragraphe entier sur une seule ligne.

- [ ] **Étape 5 : installer le manuscrit dans le dépôt**

```bash
cd /Users/jean-paulgavini/Documents/Dev/ozalid
mkdir -p app/src-tauri/temoin
cp "$SCRATCH/manuscrit.md" app/src-tauri/temoin/manuscrit.md
file app/src-tauri/temoin/manuscrit.md
```

Attendu : `UTF-8 Unicode text`, **sans** mention `CRLF`.

- [ ] **Étape 6 : commit**

```bash
git add app/src-tauri/temoin/manuscrit.md
git commit -m "Un livre que la CI peut composer, puisque les nôtres ne sont pas versionnés"
```

---

## Tâche 3 : le témoin est composable, et il est en LF

**Fichiers :**
- Modifier : `app/src-tauri/src/manuscrit.rs` (dans `mod tests`, en fin de fichier)

Deux tests unitaires, avant l'exemple : ils tournent en une seconde, sans Typst,
et ce sont eux qui tomberont en premier si un checkout Windows dénature le
texte.

- [ ] **Étape 1 : écrire les tests qui échouent**

À ajouter à la fin du `mod tests` de `app/src-tauri/src/manuscrit.rs` :

```rust
    /// Le manuscrit-témoin de la CI, embarqué à la compilation des tests.
    const TEMOIN: &str = include_str!("../temoin/manuscrit.md");

    /// Ce que la CI compose doit d'abord passer la porte du format. Ce test échoue en
    /// une seconde là où l'exemple `temoin` coûte une composition entière.
    #[test]
    fn le_manuscrit_temoin_est_composable() {
        let chapitres = decoupe(TEMOIN, Some(30)).expect("le témoin doit être composable");
        assert_eq!(chapitres.len(), 30);
        assert_eq!(chapitres[0].numero, 1);
        assert!(
            !chapitres[0].titre.is_empty(),
            "un chapitre sans titre : la conversion a mangé l'en-tête"
        );
    }

    /// Un checkout Windows peut convertir les fins de ligne malgré `.gitattributes` si
    /// celui-ci venait à disparaître. Les `\r` ne se verraient pas dans le découpage —
    /// `str::lines` les retire — mais ils entreraient dans les paragraphes, donc dans la
    /// source Typst, et déplaceraient peut-être la pagination sans rien dire.
    #[test]
    fn le_manuscrit_temoin_est_en_fins_de_ligne_unix() {
        assert!(
            !TEMOIN.contains('\r'),
            "le témoin porte des retours chariot : .gitattributes n'a pas joué"
        );
    }
```

- [ ] **Étape 2 : lancer les tests**

```bash
cd app/src-tauri
cargo test --lib manuscrit::tests::le_manuscrit_temoin -- --nocapture
```

Attendu : **PASS** pour les deux. Ils ne sont pas écrits pour échouer d'abord —
ils vérifient un fichier déjà en place. S'ils échouent, c'est la conversion de
la tâche 2 qui est en cause : lire le message (numéro de ligne du refus, ou
compte de chapitres) et reprendre le script.

- [ ] **Étape 3 : commit**

```bash
git add src/manuscrit.rs
git commit -m "Le témoin passe la porte du format avant qu'on l'imprime"
```

---

## Tâche 4 : l'exemple `temoin`, sans encore juger

**Fichiers :**
- Créer : `app/src-tauri/examples/temoin.rs`

L'exemple est d'abord écrit **sans** valeur attendue : on ne peut pas figer une
pagination qu'on n'a pas relevée.

- [ ] **Étape 1 : écrire l'exemple**

`app/src-tauri/examples/temoin.rs` :

```rust
//! Compose le manuscrit-témoin et vérifie que la pagination n'a pas bougé.
//!
//! Le témoin est *Candide* (Voltaire, 1759), du domaine public, récupéré depuis Project
//! Gutenberg et mis au format du projet. Il n'est pas là pour se lire : `build/` n'étant
//! pas versionné, c'est le seul livre que l'intégration continue puisse composer.
//!
//! Ce qu'il prouve, et qu'aucun test unitaire ne peut prouver : Typst compose le même
//! nombre de pages sur macOS et sur Windows. Un écart invaliderait la promesse centrale
//! du projet — un dos calculé sur une plateforme ne vaudrait que pour elle.
//!
//! Le gabarit est `bod`, et non `lulu` : la table Lulu ne porte pas de tranche de
//! gouttière sous 151 pages, et la compléter pour les besoins d'un test reviendrait à
//! laisser le test dicter la production.
//!
//! Usage : cargo run --example temoin [répertoire de sortie]

use std::path::{Path, PathBuf};

use ozalid_lib::maquettes;
use ozalid_lib::package;
use ozalid_lib::planche::Releve;
use ozalid_lib::projet::{Livre, Projet};
use ozalid_lib::providers;
use ozalid_lib::typst::Typst;

const PROVIDER: &str = "bod";

fn main() -> Result<(), String> {
    let sortie = std::env::args()
        .nth(1)
        .map_or_else(|| std::env::temp_dir().join("ozalid-temoin"), PathBuf::from);

    let livre = Livre {
        titre: "Candide".into(),
        titre_page: None,
        auteur: "Voltaire".into(),
        genre: "conte philosophique".into(),
        copyright: "Texte du domaine public.".into(),
        chapitres: Some(30),
    };
    let mut projet = Projet::nouveau(livre, include_str!("../temoin/manuscrit.md").to_string());
    // La Blanche est purement typographique : le témoin traverse la planche entière sans
    // qu'une seule image ait à être versionnée.
    projet.meta.couverture.maquette = Some(maquettes::blanche());

    let pr = providers::provider(PROVIDER).ok_or("prestataire inconnu : bod")?;
    let typst =
        Typst::new("typst").avec_polices(Path::new(env!("CARGO_MANIFEST_DIR")).join("fonts"));
    let p = package::assembler(
        &projet,
        pr,
        pr.papier_defaut(),
        // BoD publie son dos et son fond perdu : le relevé est ignoré.
        Releve::default(),
        &sortie,
        &typst,
    )?;

    println!(
        "{} — {} pages, gouttière {:.1} mm, dos {:.2} mm, planche {:.2} × {:.2} mm{}",
        p.libelle,
        p.pages,
        p.gouttiere,
        p.dos,
        p.planche.0,
        p.planche.1,
        if p.blanche { ", blanche de parité" } else { "" }
    );
    Ok(())
}
```

- [ ] **Étape 2 : composer et relever la pagination**

```bash
cd app/src-tauri
cargo run --quiet --example temoin -- /tmp/ozalid-temoin
```

Attendu : une ligne « BoD — 13,5 × 21,5 cm — N pages … », avec N de l'ordre de
la centaine. **Noter N** : c'est la valeur qui sera figée à la tâche 5.

Si la commande échoue sur « tranche de gouttière absente » ou sur les bornes du
prestataire, c'est que le témoin est trop court ou trop long : le dire plutôt
que de changer la table `providers`, qui décrit des gabarits réels.

- [ ] **Étape 3 : regarder le PDF**

```bash
open /tmp/ozalid-temoin/interieur-bod.pdf
open /tmp/ozalid-temoin/couverture-bod.pdf
```

Vérifier à l'œil, une fois : les accents et les apostrophes sont composés, les
titres de chapitre longs se replient sans déborder, la couverture Blanche porte
son cadre. C'est la vérification qu'aucun test ne remplace.

- [ ] **Étape 4 : commit**

```bash
git add examples/temoin.rs
git commit -m "Le témoin se compose, avant qu'on lui demande un chiffre"
```

---

## Tâche 5 : figer la pagination

**Fichiers :**
- Modifier : `app/src-tauri/examples/temoin.rs`

- [ ] **Étape 1 : ajouter la constante et le verdict**

Insérer après `const PROVIDER: &str = "bod";` — remplacer `000` par le N relevé
à la tâche 4 :

```rust
/// Pagination attendue du témoin.
///
/// Relevée sur macOS avec Typst 0.15.1 et EB Garamond, au corps et à l'interligne que
/// `providers` fixe pour BoD. Elle dépend de chacun de ces éléments : la déplacer est un
/// acte délibéré, à revalider sur un livre réel — jamais un ajustement pour faire passer
/// l'intégration continue.
const PAGES_ATTENDUES: u32 = 000;
```

Et remplacer la fin de `main`, après le `println!`, par :

```rust
    if p.pages != PAGES_ATTENDUES {
        return Err(format!(
            "pagination déplacée : {} pages, {PAGES_ATTENDUES} attendues.\n\
             Si le changement est voulu — police, gabarit, version de Typst —, relever la \
             nouvelle valeur et la figer dans PAGES_ATTENDUES. Sinon, cette plateforme ne \
             compose pas comme l'autre, et aucun dos calculé ici ne vaut ailleurs.",
            p.pages
        ));
    }
    Ok(())
}
```

- [ ] **Étape 2 : vérifier que le témoin passe**

```bash
cargo run --quiet --example temoin -- /tmp/ozalid-temoin
echo "code de sortie : $?"
```

Attendu : la ligne de résultat, et `code de sortie : 0`.

- [ ] **Étape 3 : vérifier qu'il sait échouer**

Un juge qui ne condamne jamais ne prouve rien. Changer temporairement la
constante pour `PAGES_ATTENDUES + 1`, relancer, constater le message et le code
non nul, puis **remettre la bonne valeur**.

```bash
cargo run --quiet --example temoin -- /tmp/ozalid-temoin
echo "code de sortie : $?"
```

Attendu, pendant l'essai : « pagination déplacée : … » et `code de sortie : 1`.

- [ ] **Étape 4 : commit**

```bash
git add examples/temoin.rs
git commit -m "La pagination du témoin est un chiffre, pas une impression"
```

---

## Tâche 6 : `typst.sh` sans `unzip`

**Fichiers :**
- Modifier : `app/outils/typst.sh` (branche `*windows*` du bloc de téléchargement)

Git Bash, sur les runners Windows, ne garantit pas `unzip`. `tar` y est présent
et sait ouvrir un zip depuis Windows 10.

- [ ] **Étape 1 : modifier la branche windows**

Remplacer, dans le `case "$TRIPLE"` du bloc de téléchargement :

```bash
    *windows*)
      curl -fL "$BASE/typst-${TRIPLE}.zip" -o "$TMP/t.zip"
      unzip -q "$TMP/t.zip" -d "$TMP"
      ;;
```

par :

```bash
    *windows*)
      curl -fL "$BASE/typst-${TRIPLE}.zip" -o "$TMP/t.zip"
      # `unzip` n'est pas garanti dans Git Bash, où ce script tourne sur les runners
      # Windows ; le `tar` de Windows 10 et au-delà ouvre un zip.
      if command -v unzip >/dev/null 2>&1; then
        unzip -q "$TMP/t.zip" -d "$TMP"
      else
        tar -xf "$TMP/t.zip" -C "$TMP"
      fi
      ;;
```

- [ ] **Étape 2 : vérifier que le script reste correct**

```bash
cd /Users/jean-paulgavini/Documents/Dev/ozalid
bash -n app/outils/typst.sh && echo "syntaxe correcte"
app/outils/typst.sh x86_64-pc-windows-msvc
ls -l app/src-tauri/binaries/
```

Attendu : « syntaxe correcte », puis une ligne
`…/binaries/typst-x86_64-pc-windows-msvc.exe — non exécutable sur cet hôte` —
le binaire Windows a bien été téléchargé et déposé, macOS ne peut pas le lancer,
c'est le comportement prévu par le script.

- [ ] **Étape 3 : commit**

```bash
git add app/outils/typst.sh
git commit -m "Le sidecar Windows se dépaquette là où unzip n'existe pas"
```

---

## Tâche 7 : le job `verifier`

**Fichiers :**
- Créer : `.github/workflows/windows.yml`

- [ ] **Étape 1 : écrire le workflow**

```yaml
# Établit que Windows compile, teste et pagine comme macOS, puis produit sur tag
# l'installeur NSIS. L'app n'est testable sous Windows que par ici : c'est le second
# volet du jalon 5 (docs/superpowers/specs/2026-08-20-release-windows-ci-design.md).
name: Windows

on:
  push:
    branches: [main]
    tags: ["v*"]
  pull_request:
  workflow_dispatch:

permissions:
  contents: write

concurrency:
  group: windows-${{ github.ref }}
  cancel-in-progress: true

jobs:
  verifier:
    runs-on: windows-latest
    defaults:
      run:
        shell: bash
    steps:
      - uses: actions/checkout@v4

      - uses: actions/setup-node@v4
        with:
          node-version: "22"

      # La compilation Rust d'un projet Tauri dépasse largement le reste du job ; sans
      # cache, chaque push coûterait une dizaine de minutes.
      - uses: Swatinem/rust-cache@v2
        with:
          workspaces: app/src-tauri

      # Le sidecar et les polices ne sont pas versionnés. La clé porte les scripts
      # eux-mêmes : relever la version épinglée de Typst ou changer la liste des polices
      # invalide le cache, ce qui est le comportement voulu.
      - name: Cache du sidecar et des polices
        uses: actions/cache@v4
        with:
          path: |
            app/src-tauri/binaries
            app/src-tauri/fonts
          key: typst-polices-${{ hashFiles('app/outils/typst.sh', 'app/outils/polices.sh') }}

      - name: Sidecar Typst et polices embarquées
        run: |
          app/outils/typst.sh x86_64-pc-windows-msvc
          app/outils/polices.sh

      # `temoin` et `packager` prennent le Typst du PATH, comme en local : la CI ne se
      # donne pas une procédure à elle.
      - name: Typst dans le PATH
        run: |
          mkdir -p "$RUNNER_TEMP/bin"
          cp app/src-tauri/binaries/typst-x86_64-pc-windows-msvc.exe "$RUNNER_TEMP/bin/typst.exe"
          echo "$RUNNER_TEMP/bin" >> "$GITHUB_PATH"

      - name: Version de Typst
        run: typst --version

      - name: Format
        working-directory: app/src-tauri
        run: cargo fmt --check

      # `-D warnings` en plus de la commande du README : en local un avertissement se
      # lit, ici personne ne lit le journal d'un job vert.
      - name: Clippy
        working-directory: app/src-tauri
        run: cargo clippy --all-targets -- -D warnings

      - name: Tests Rust
        working-directory: app/src-tauri
        run: cargo test --lib

      - name: Tests du front
        working-directory: app
        run: node --test "tests/*.test.js"

      # L'assertion centrale du volet : la pagination ne dépend pas de la plateforme.
      - name: Témoin de pagination
        working-directory: app/src-tauri
        run: cargo run --example temoin -- "$RUNNER_TEMP/temoin"
```

- [ ] **Étape 2 : commit et pousser**

```bash
git add .github/workflows/windows.yml
git commit -m "Windows doit d'abord compter les mêmes pages"
git push
```

- [ ] **Étape 3 : suivre le run et relever le verdict**

```bash
gh run watch --exit-status
```

Attendu : job au vert, et dans le journal du témoin **le même nombre de pages
que celui relevé sur macOS** à la tâche 4.

Si le compte diffère, **ne pas ajuster la constante** : c'est la découverte que
le jalon devait faire. Consigner les deux valeurs, puis chercher la cause dans
cet ordre — fins de ligne du manuscrit (le test de la tâche 3 aurait dû tomber),
version de Typst effectivement téléchargée (`typst --version` dans le journal),
polices présentes (`ls app/src-tauri/fonts`).

Si l'échec porte sur `clippy -D warnings` alors que le code passe sur macOS,
c'est un avertissement propre à la cible Windows : le corriger dans le code,
sans relâcher le `-D warnings`.

---

## Tâche 8 : le job `publier`

**Fichiers :**
- Modifier : `.github/workflows/windows.yml`

- [ ] **Étape 1 : ajouter le job**

À la suite de `verifier`, dans le même fichier :

```yaml
  publier:
    needs: verifier
    if: startsWith(github.ref, 'refs/tags/v')
    runs-on: windows-latest
    defaults:
      run:
        shell: bash
    steps:
      - uses: actions/checkout@v4

      # Une release dont le numéro ment sur son contenu est pire que pas de release.
      - name: Le tag et la version de l'application doivent coïncider
        run: |
          tag="${GITHUB_REF_NAME#v}"
          conf=$(node -p "require('./app/src-tauri/tauri.conf.json').version")
          if [ "$tag" != "$conf" ]; then
            echo "tag $GITHUB_REF_NAME contre version $conf de tauri.conf.json" >&2
            exit 1
          fi
          echo "version $conf"

      - uses: actions/setup-node@v4
        with:
          node-version: "22"

      - uses: Swatinem/rust-cache@v2
        with:
          workspaces: app/src-tauri

      - name: Cache du sidecar et des polices
        uses: actions/cache@v4
        with:
          path: |
            app/src-tauri/binaries
            app/src-tauri/fonts
          key: typst-polices-${{ hashFiles('app/outils/typst.sh', 'app/outils/polices.sh') }}

      - name: Sidecar Typst et polices embarquées
        run: |
          app/outils/typst.sh x86_64-pc-windows-msvc
          app/outils/polices.sh

      # Un binaire préconstruit, quelques secondes, là où `cargo install tauri-cli`
      # recompile pendant plusieurs minutes.
      - name: CLI Tauri
        run: npm install -g @tauri-apps/cli@^2

      # Le format est choisi ici et non dans tauri.conf.json, qui reste à
      # `targets: "all"` pour que la construction macOS locale produise son .dmg.
      - name: Construire l'installeur
        working-directory: app/src-tauri
        run: tauri build --bundles nsis

      - name: Release draft
        env:
          GH_TOKEN: ${{ github.token }}
        run: |
          exe=$(find app/src-tauri/target/release/bundle/nsis -name "*-setup.exe" | head -1)
          [ -n "$exe" ] || { echo "aucun installeur produit" >&2; exit 1; }
          gh release create "$GITHUB_REF_NAME" "$exe" \
            --draft \
            --title "Ozalid Studio $GITHUB_REF_NAME" \
            --notes "$(cat <<'FIN'
          Application de bureau Windows. L'installeur n'est **pas signé** : au premier
          lancement, Windows affiche « Windows a protégé votre PC ». Choisir
          « Informations complémentaires », puis « Exécuter quand même ».

          L'installation ne demande pas de droits administrateur.
          FIN
          )"
```

- [ ] **Étape 2 : commit et pousser**

```bash
git add .github/workflows/windows.yml
git commit -m "Un tag produit un installeur, et un numéro qui ne ment pas"
git push
```

- [ ] **Étape 3 : essayer sur un tag**

```bash
git tag v0.1.0
git push origin v0.1.0
gh run watch --exit-status
gh release view v0.1.0
```

Attendu : `verifier` au vert, `publier` à sa suite, et une release **draft**
`v0.1.0` portant un fichier `…-setup.exe` de quelques dizaines de Mo.

Le garde-fou de version doit passer, `tauri.conf.json` portant déjà `0.1.0`.
Pour vérifier qu'il sait mordre, pousser un tag `v9.9.9` : le job doit échouer
sur « tag v9.9.9 contre version 0.1.0 » avant toute compilation. Supprimer
ensuite ce tag :

```bash
git push --delete origin v9.9.9 && git tag -d v9.9.9
```

---

## Tâche 9 : l'installeur pose-t-il ses fichiers là où l'app les cherche

**Fichiers :**
- Modifier : `.github/workflows/windows.yml` (job `publier`, avant l'étape « Release draft »)

`commands.rs::typst()` cherche les polices dans trois répertoires candidats et le
sidecar à côté de l'exécutable. Un écart signifie une application qui s'ouvre
puis refuse de composer — la panne la plus probable au premier lancement, et
celle que le commentaire de `commands.rs:583` laissait ouverte.

- [ ] **Étape 1 : ajouter l'étape d'inspection**

```yaml
      # L'installation silencieuse permet d'inspecter l'arborescence réelle sans lancer
      # de fenêtre. Ce que l'app cherchera : `typst.exe` à côté de l'exécutable, et
      # `fonts/*.ttf` dans l'un des candidats de `commands.rs::typst()`.
      - name: L'installeur pose ses fichiers là où l'application les cherche
        run: |
          exe=$(find app/src-tauri/target/release/bundle/nsis -name "*-setup.exe" | head -1)
          "$exe" /S
          sleep 20
          racine="$LOCALAPPDATA/Ozalid Studio"
          echo "— arborescence installée —"
          find "$racine" -maxdepth 2 | head -40
          test -f "$racine/typst.exe" \
            || { echo "sidecar absent de l'installation" >&2; exit 1; }
          ls "$racine/fonts/"*.ttf >/dev/null 2>&1 \
            || { echo "polices absentes de $racine/fonts" >&2; exit 1; }
          echo "sidecar et polices en place"
```

- [ ] **Étape 2 : pousser et lire l'arborescence**

Le tag `v0.1.0` a déjà servi à la tâche 8 et sa release draft existe : la
reprendre demande de défaire les deux, sans quoi `gh release create` refusera de
créer une release qui existe déjà.

```bash
git add .github/workflows/windows.yml
git commit -m "Un empaquetage qui s'installe n'est pas un empaquetage qui compose"
git push

gh release delete v0.1.0 --yes
git push --delete origin v0.1.0 && git tag -d v0.1.0
git tag v0.1.0 && git push origin v0.1.0
gh run watch --exit-status
```

- [ ] **Étape 3 : traiter ce que l'arborescence révèle**

Trois issues possibles, à traiter différemment :

1. **Tout est en place** — l'étape passe. Retirer la ligne
   `find "$racine" -maxdepth 2 | head -40` si le journal est trop bavard, ou la
   garder : elle documente l'empaquetage.
2. **La racine d'installation n'est pas `$LOCALAPPDATA/Ozalid Studio`** —
   le `find` échoue. Relever le chemin réel dans le journal de l'installeur
   (`ls "$LOCALAPPDATA" "$PROGRAMFILES"`), corriger la variable `racine`.
3. **Les polices ne sont pas dans `<racine>/fonts`** — c'est une vraie
   découverte, celle que le jalon devait faire. Ne pas ajuster le test : ajouter
   le chemin réel aux candidats de `commands.rs::typst()`, avec un commentaire
   disant d'où il vient, puis relancer. C'est la réponse attendue au commentaire
   « le chemin réel en release se vérifie au jalon 5 ».

Dans les cas 2 et 3, commiter le correctif séparément :

```bash
git add -A && git commit -m "Le chemin réel des ressources en release Windows"
```

---

## Tâche 10 : la documentation

**Fichiers :**
- Modifier : `app/README.md`

- [ ] **Étape 1 : l'état du jalon**

Remplacer, dans la section d'ouverture :

```
multi-prestataires et épreuve de relecture. Reste la release Windows.
```

par :

```
multi-prestataires, épreuve de relecture et release Windows par intégration
continue. Le jalon 5 est clos.
```

- [ ] **Étape 2 : la section d'installation**

Ajouter après la section « Mise en route » :

```markdown
## Installer sous Windows

Chaque tag `v*` produit un installeur NSIS, attaché en release. Il **n'est pas
signé** : au premier lancement, Windows affiche « Windows a protégé votre PC ».
Choisir « Informations complémentaires », puis « Exécuter quand même ».
L'installation ne demande pas de droits administrateur.

Un certificat de signature de code lèverait cet avertissement ; il n'a pas été
pris tant que la diffusion reste confidentielle.
```

- [ ] **Étape 3 : le témoin dans les vérifications**

Ajouter à la fin de la section « Vérifications », après la liste des exercices
sur livre réel :

````markdown
Et le témoin de la CI, seul exercice à porter sa propre valeur attendue :

```
cd app/src-tauri && cargo run --example temoin
```

Il compose *Candide* — texte du domaine public versionné dans `temoin/`, le seul
livre que l'intégration continue puisse composer, `build/` n'étant pas suivi — et
**échoue si la pagination s'écarte** de la constante figée dans l'exemple. C'est
ce qui établit que Windows et macOS composent le même livre : un dos calculé sur
l'une vaut pour l'autre. Le workflow `.github/workflows/windows.yml` le lance à
chaque push.
````

- [ ] **Étape 4 : commit**

```bash
git add app/README.md
git commit -m "La documentation dit comment s'installe Windows, et ce que le témoin prouve"
```

---

## Tâche 11 : ce que la CI ne peut pas prouver

**Fichiers :** aucun. C'est une vérification manuelle, et elle conditionne la
publication de la release draft.

Aucun runner ne lance l'application avec sa WebView2. La CI établit la
compilation, les tests, la pagination et l'emplacement des fichiers installés —
pas qu'une fenêtre s'ouvre.

- [ ] **Étape 1 : installer sur une machine ou une VM Windows**

Télécharger le `…-setup.exe` de la release draft, l'installer, passer
l'avertissement SmartScreen.

- [ ] **Étape 2 : dérouler la chaîne à la main**

- La fenêtre s'ouvre.
- Un projet `.ozalid` produit sur macOS s'ouvre sans erreur.
- L'aperçu de couverture s'affiche.
- « Composer » produit un intérieur, et le compte de pages affiché est **celui
  qu'affiche macOS sur le même projet**.
- Un package est écrit dans le répertoire choisi.
- L'épreuve de relecture se tire.

- [ ] **Étape 3 : publier ou consigner**

Si tout passe : publier la release depuis GitHub.

Sinon : **ne pas publier**. Consigner le défaut dans `NOTES.md` § 4, avec ce qui
a été fait et ce qui a échoué, et le traiter avant de reprendre. Une release qui
s'installe mais ne compose pas est plus coûteuse que pas de release.

---

## Ce qui reste hors de ce plan

- macOS en intégration continue.
- La signature de code, sur l'une ou l'autre plateforme.
- Linux.
- La mise à jour automatique de l'application.
- La correction des ruptures de scène de l'intérieur (dette `NOTES.md` § 4).
- Le portage des scripts d'approvisionnement en PowerShell.
