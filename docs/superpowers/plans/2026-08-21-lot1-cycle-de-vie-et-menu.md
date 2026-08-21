# Lot 1 — Cycle de vie du projet et menu natif

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Donner à Ozalid Studio un cycle de vie de document complet — créer, enregistrer, fermer, se souvenir — et un menu natif qui en fait foi, sans encore toucher à la mise en page de la page unique.

**Architecture:** Deux modules Rust neufs. `preferences.rs` porte un `preferences.toml` dans le répertoire de configuration de l'application, dont le seul contenu est la liste des projets récents. `menu.rs` construit le menu natif et n'agit jamais lui-même : chaque entrée émet un événement que l'interface traite avec le code de ses propres boutons. `commands.rs` gagne un drapeau `modifie` porté par le projet ouvert, les commandes de création, de fermeture et d'enregistrement en place, et une garde qui pose une boîte native à trois boutons avant tout ce qui perdrait du travail.

**Tech Stack:** Rust + Tauri 2.11.5, `tauri-plugin-dialog` 2.7.2, `serde` + `toml` (déjà présents, aucune dépendance nouvelle), front vanilla sans bundler, tests `cargo test --lib` et `node --test`.

---

## Contexte pour qui n'a jamais ouvert ce dépôt

L'application vit dans `app/`. Le Rust est dans `app/src-tauri/src/`, le front — trois fichiers, sans bundler — dans `app/src/`. Le front n'a **aucune logique métier** : il invoque des commandes et affiche ce qu'elles rendent.

`commands.rs` tient le projet ouvert dans un `Atelier`, structure `Default` gardée par Tauri en `State`. Un seul projet à la fois : c'est un éditeur de document, pas une bibliothèque. Chaque commande qui touche au projet rend une `ProjetVue`, et le front se redessine entièrement depuis elle (`afficherProjet` dans `app.js`).

Les tests Rust vivent dans un `mod tests` en bas de chaque module et ne visent que ce qui est testable sans fenêtre — jamais les `#[tauri::command]` eux-mêmes, qui exigent un `State`. Les tests du front exécutent le **vrai** `app.js` dans un faux DOM (`app/tests/dom_shim.js`) qui lit l'état initial du **vrai** `index.html`.

Le français est la langue de l'interface, des commentaires et des messages de commit. Les commits du dépôt sont des phrases qui disent ce que le code a appris, jamais `feat:` ni `fix:`.

Commandes de vérification, à connaître avant de commencer :

```
cd app/src-tauri && cargo test --lib && cargo clippy --all-targets && cargo fmt --check
cd app && node --test "tests/*.test.js"
```

---

## Structure des fichiers

**Créés**

| Fichier | Responsabilité |
|---|---|
| `app/src-tauri/src/preferences.rs` | Le `preferences.toml` : lecture au mieux, écriture, règles de la liste des récents |
| `app/src-tauri/src/menu.rs` | Construction du menu natif et émission de ses événements |
| `app/tests/cycle_de_vie.test.js` | Câblage front du cycle de vie : nouveau, enregistrer, garde, récents, menu |

**Modifiés**

| Fichier | Ce qui change |
|---|---|
| `app/src-tauri/src/projet.rs` | `Livre::vide()` |
| `app/src-tauri/src/commands.rs` | `modifie` sur `Ouvert`, `manuscrit_absent` sur `ProjetVue`, commandes du cycle de vie, récents, garde |
| `app/src-tauri/src/lib.rs` | Modules, menu, événements de menu, `CloseRequested`, nouvelles commandes |
| `app/src-tauri/capabilities/default.json` | `core:window:allow-destroy`, `core:event:allow-listen` |
| `app/src/index.html` | Boutons Nouveau et Enregistrer, état d'enregistrement, liste des récents |
| `app/src/app.js` | Câblage, garde, écoute des événements de menu |
| `app/src/styles.css` | Liste des récents et état d'enregistrement |
| `app/tests/dom_shim.js` | `window.__TAURI__.event` et `.window` dans le faux contexte |
| `app/tests/*.test.js` | `IDS` et `PROJET` gagnent les champs neufs |
| `app/README.md` | Table des modules, section cycle de vie |

---

## Écarts assumés en cours d'exécution

**Renommage de la paire d'enregistrement (décidé après la tâche 3).** La revue de qualité a
relevé un nommage à contre-sens : `projet_enregistrer(chemin)` était en réalité
« Enregistrer sous… », tandis que `projet_enregistrer_courant()` était le « Enregistrer »
de ⌘S — le nom court désignait l'action longue. Les tâches 7 et 8 câblent précisément ⌘S,
donc la confusion aurait été cimentée. Le renommage a été fait au début de la tâche 4, dans
son propre commit :

- `projet_enregistrer` → `projet_enregistrer_sous`
- `projet_enregistrer_courant` → `projet_enregistrer`

Les tâches 3 et 4 ci-dessous portent encore les anciens noms : c'est le texte tel qu'il a été
exécuté, laissé comme trace. **Les tâches 7 et 8 ont été corrigées** et portent les nouveaux.

---

## Task 1 : Le magasin de préférences

**Files:**
- Create: `app/src-tauri/src/preferences.rs`
- Modify: `app/src-tauri/src/lib.rs` (déclaration du module)

- [ ] **Step 1 : Écrire le module avec ses tests, en commençant par les tests**

Créer `app/src-tauri/src/preferences.rs` avec, pour l'instant, **seulement** le bloc de tests ci-dessous et les `use` :

```rust
//! Les préférences de l'application : ce qui survit à la fermeture sans appartenir
//! à un livre.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests {
    use super::*;

    fn chemins(p: &Preferences) -> Vec<&str> {
        p.recents.iter().map(String::as_str).collect()
    }

    /// Le dernier projet ouvert passe en tête, et n'y figure qu'une fois : une liste
    /// de raccourcis qui répète le même projet n'en est plus une.
    #[test]
    fn un_recent_deja_present_remonte_sans_se_dupliquer() {
        let mut p = Preferences::default();
        p.ajouter_recent(Path::new("/a.ozalid"));
        p.ajouter_recent(Path::new("/b.ozalid"));
        p.ajouter_recent(Path::new("/a.ozalid"));
        assert_eq!(chemins(&p), ["/a.ozalid", "/b.ozalid"]);
    }

    /// Au-delà du plafond, la liste cesserait d'être un raccourci pour devenir un
    /// historique — et le sous-menu qui la porte, illisible.
    #[test]
    fn la_liste_des_recents_est_plafonnee() {
        let mut p = Preferences::default();
        for i in 0..MAX_RECENTS + 5 {
            p.ajouter_recent(Path::new(&format!("/{i}.ozalid")));
        }
        assert_eq!(p.recents.len(), MAX_RECENTS);
        assert_eq!(p.recents[0], format!("/{}.ozalid", MAX_RECENTS + 4));
    }

    /// Un projet effacé ne doit pas être proposé : le clic échouerait, et l'échec
    /// arriverait après le clic. L'élagage se fait à la lecture, pas à l'écriture —
    /// un projet sur un volume démonté revient de lui-même au remontage, alors
    /// qu'une purge l'aurait perdu pour de bon.
    #[test]
    fn seuls_les_recents_qui_existent_encore_sont_rendus() {
        let dir = tempfile::tempdir().unwrap();
        let vivant = dir.path().join("vivant.ozalid");
        std::fs::write(&vivant, b"zip").unwrap();
        let mort = dir.path().join("mort.ozalid");

        let mut p = Preferences::default();
        p.ajouter_recent(&mort);
        p.ajouter_recent(&vivant);

        assert_eq!(p.recents.len(), 2, "la liste garde tout");
        assert_eq!(
            p.recents_existants(),
            vec![vivant.to_string_lossy().into_owned()],
            "seul ce qui existe est proposé"
        );
    }

    #[test]
    fn les_preferences_font_l_aller_retour() {
        let dir = tempfile::tempdir().unwrap();
        let mut p = Preferences::default();
        p.ajouter_recent(Path::new("/livres/heures-creuses.ozalid"));
        enregistrer(dir.path(), &p).unwrap();
        assert_eq!(charger(dir.path()), p);
    }

    /// Aucune de ces trois avaries ne doit empêcher l'application de démarrer : les
    /// préférences sont un confort, pas un document.
    #[test]
    fn des_preferences_absentes_ou_corrompues_valent_le_defaut() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(charger(dir.path()), Preferences::default(), "fichier absent");

        std::fs::write(fichier(dir.path()), b"ceci n'est pas du TOML {{{").unwrap();
        assert_eq!(charger(dir.path()), Preferences::default(), "fichier illisible");

        std::fs::write(fichier(dir.path()), b"autre_chose = 3\n").unwrap();
        assert_eq!(charger(dir.path()), Preferences::default(), "champ inconnu");
    }
}
```

- [ ] **Step 2 : Déclarer le module et lancer les tests pour les voir échouer**

Ajouter dans `app/src-tauri/src/lib.rs`, dans la liste des `pub mod` (ordre alphabétique, entre `png` et `projet`) :

```rust
pub mod preferences;
```

Lancer :

```
cd app/src-tauri && cargo test --lib preferences
```

Attendu : échec de compilation — `cannot find type Preferences in this scope`, `cannot find function charger`, etc.

- [ ] **Step 3 : Écrire l'implémentation**

Insérer, dans `preferences.rs`, entre les `use` et le `mod tests` :

```rust
/// Au-delà, la liste cesse d'être un raccourci pour devenir un historique — et le
/// sous-menu qui la porte devient illisible.
pub const MAX_RECENTS: usize = 10;

const FICHIER: &str = "preferences.toml";

/// Ce que porte `preferences.toml`.
///
/// `deny_unknown_fields` n'y figure pas volontairement : un champ écrit par une
/// version plus récente doit être ignoré, pas faire échouer la lecture. Un champ
/// perdu coûte un réglage ; une lecture refusée coûte la liste entière.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Preferences {
    #[serde(default)]
    pub recents: Vec<String>,
}

impl Preferences {
    /// Pose un projet en tête des récents, sans doublon ni débordement.
    pub fn ajouter_recent(&mut self, chemin: &Path) {
        let c = chemin.to_string_lossy().into_owned();
        self.recents.retain(|r| r != &c);
        self.recents.insert(0, c);
        self.recents.truncate(MAX_RECENTS);
    }

    /// Les récents dont le fichier existe encore.
    pub fn recents_existants(&self) -> Vec<String> {
        self.recents
            .iter()
            .filter(|r| Path::new(r).is_file())
            .cloned()
            .collect()
    }
}

pub fn fichier(config: &Path) -> PathBuf {
    config.join(FICHIER)
}

/// Lit les préférences, ou rend celles par défaut.
///
/// Aucune erreur ne remonte : absent, illisible ou corrompu, le fichier doit laisser
/// l'application démarrer.
pub fn charger(config: &Path) -> Preferences {
    std::fs::read_to_string(fichier(config))
        .ok()
        .and_then(|s| toml::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn enregistrer(config: &Path, p: &Preferences) -> Result<(), String> {
    std::fs::create_dir_all(config).map_err(|e| {
        format!(
            "répertoire de configuration inutilisable ({}) : {e}",
            config.display()
        )
    })?;
    let s = toml::to_string_pretty(p).map_err(|e| format!("sérialisation des préférences : {e}"))?;
    std::fs::write(fichier(config), s)
        .map_err(|e| format!("écriture des préférences ({}) : {e}", fichier(config).display()))
}
```

- [ ] **Step 4 : Lancer les tests pour les voir passer**

```
cd app/src-tauri && cargo test --lib preferences
```

Attendu : `test result: ok. 5 passed`.

Note : `tempfile` est déjà en `dev-dependencies` (`Cargo.toml`), il n'y a rien à ajouter.

- [ ] **Step 5 : Formatage et clippy**

```
cd app/src-tauri && cargo fmt && cargo clippy --all-targets
```

Attendu : aucun avertissement.

- [ ] **Step 6 : Commit**

```bash
git add app/src-tauri/src/preferences.rs app/src-tauri/src/lib.rs
git commit -m "Les préférences existent, et leur perte ne coûte qu'un confort"
```

---

## Task 2 : Le projet ouvert sait s'il a changé

**Files:**
- Modify: `app/src-tauri/src/projet.rs` (ajout de `Livre::vide`)
- Modify: `app/src-tauri/src/commands.rs:26-33` (`Atelier`, `Ouvert`), `:74-88` (`ProjetVue`), `:577-604` (`poser`, `vue`)

- [ ] **Step 1 : Écrire les tests dans `commands.rs`**

Ajouter dans le `mod tests` de `app/src-tauri/src/commands.rs` (à la fin, avant l'accolade fermante) :

```rust
    fn ouvert_neuf() -> Ouvert {
        Ouvert {
            chemin: None,
            projet: Projet::nouveau(Livre::vide(), String::new()),
            modifie: false,
        }
    }

    /// Le drapeau est ce qui décide si fermer l'application perd du travail. Il ne
    /// doit se lever que par une mutation, et retomber par une écriture — jamais
    /// par une simple relecture du projet.
    #[test]
    fn le_drapeau_de_modification_suit_les_mutations_et_les_ecritures() {
        let mut o = ouvert_neuf();
        assert!(!vue(&o).unwrap().modifie, "un projet neuf n'est pas modifié");
        assert!(!vue(&o).unwrap().modifie, "relire ne modifie pas");

        assert!(vue_modifiee(&mut o).unwrap().modifie);
        assert!(vue(&o).unwrap().modifie, "le drapeau reste levé");

        assert!(!vue_enregistree(&mut o).unwrap().modifie);
    }

    /// Un manuscrit absent et un manuscrit sans chapitre composable rendent tous
    /// deux zéro chapitre. L'interface doit pouvoir dire « aucun manuscrit » plutôt
    /// que « 0 chapitre » : ce n'est pas la même chose à corriger.
    #[test]
    fn un_manuscrit_vide_se_declare_absent_et_non_vide_de_chapitres() {
        let vide = ouvert_neuf();
        let v = vue(&vide).unwrap();
        assert!(v.manuscrit_absent);
        assert_eq!(v.chapitres_trouves, 0);

        let mut plein = ouvert_neuf();
        plein.projet.texte = "## 01 - Un\n\nTexte.\n".into();
        let v = vue(&plein).unwrap();
        assert!(!v.manuscrit_absent);
        assert_eq!(v.chapitres_trouves, 1);

        // Du texte qui ne porte aucun « ## » : présent, mais sans chapitre.
        let mut sans_chapitre = ouvert_neuf();
        sans_chapitre.projet.texte = "juste une phrase\n".into();
        let v = vue(&sans_chapitre).unwrap();
        assert!(!v.manuscrit_absent, "présent, même s'il ne compose pas");
        assert_eq!(v.chapitres_trouves, 0);
    }

    /// Le genre par défaut ne doit vivre qu'à un endroit : un projet neuf et un
    /// projet relu d'un TOML sans genre doivent porter le même.
    #[test]
    fn un_livre_vide_prend_le_genre_par_defaut() {
        let l = Livre::vide();
        assert_eq!(l.genre, "roman");
        assert!(l.titre.is_empty());
        assert!(l.auteur.is_empty());
        assert_eq!(l.chapitres, None);
        assert_eq!(l.titre_page, None);
    }
```

- [ ] **Step 2 : Lancer les tests pour les voir échouer**

```
cd app/src-tauri && cargo test --lib commands
```

Attendu : échec de compilation — `no function or associated item named vide found for struct Livre`, `struct Ouvert has no field named modifie`, `no field manuscrit_absent`.

- [ ] **Step 3 : Ajouter `Livre::vide()`**

Dans `app/src-tauri/src/projet.rs`, dans le bloc `impl Livre` (juste avant `pub fn titre_page`) :

```rust
    /// Un livre à remplir : tous les champs vides, sauf le genre, dont le défaut
    /// vaut mieux qu'un blanc — et c'est le même défaut que celui d'un `projet.toml`
    /// qui ne le porte pas.
    pub fn vide() -> Self {
        Self {
            titre: String::new(),
            titre_page: None,
            auteur: String::new(),
            genre: genre_defaut(),
            copyright: String::new(),
            chapitres: None,
        }
    }
```

- [ ] **Step 4 : Poser le drapeau et l'absence de manuscrit**

Dans `app/src-tauri/src/commands.rs`, remplacer la déclaration de `Ouvert` :

```rust
struct Ouvert {
    chemin: Option<PathBuf>,
    projet: Projet,
    /// Vrai dès qu'une commande a touché au projet sans qu'il ait été réécrit.
    /// C'est lui, et lui seul, qui décide si fermer perd du travail.
    modifie: bool,
}
```

Ajouter deux champs à `ProjetVue`, après `mots` :

```rust
    /// Vrai quand le projet ne porte aucun texte. Distinct de « zéro chapitre » :
    /// un manuscrit présent mais non composable en trouve zéro aussi, et ce n'est
    /// pas la même chose à corriger.
    pub manuscrit_absent: bool,
    /// Modifications non enregistrées.
    pub modifie: bool,
```

Dans `vue`, ajouter les deux champs à la construction de `ProjetVue` :

```rust
        manuscrit_absent: o.projet.texte.trim().is_empty(),
        modifie: o.modifie,
```

Ajouter, juste après la fonction `vue` :

```rust
/// La vue d'un projet qu'on vient de modifier.
///
/// Deux fonctions plutôt qu'un drapeau posé à la main dans chaque commande : le
/// point d'appel dit ce qu'il a fait, et oublier de le dire se voit à la lecture.
fn vue_modifiee(o: &mut Ouvert) -> Result<ProjetVue, String> {
    o.modifie = true;
    vue(o)
}

/// La vue d'un projet qu'on vient d'écrire sur le disque.
fn vue_enregistree(o: &mut Ouvert) -> Result<ProjetVue, String> {
    o.modifie = false;
    vue(o)
}
```

Modifier `poser` pour qu'il reçoive l'état d'enregistrement :

```rust
fn poser(
    atelier: &State<Atelier>,
    chemin: Option<PathBuf>,
    projet: Projet,
    modifie: bool,
) -> Result<ProjetVue, String> {
    let mut garde = atelier.ouvert.lock().unwrap();
    *garde = Some(Ouvert {
        chemin,
        projet,
        modifie,
    });
    vue(garde.as_ref().unwrap())
}
```

- [ ] **Step 5 : Reporter le drapeau sur tous les points d'appel**

Deux appels à `poser` existent. Les corriger :

- `projet_importer` → `poser(&atelier, None, projet, true)`. Un projet importé porte du travail réel qui n'est sur aucun disque : le fermer sans prévenir le perdrait.
- `projet_ouvrir` → `poser(&atelier, Some(c), projet, false)`.

Sept commandes mutent le projet et se terminent aujourd'hui par `vue(o)`. Remplacer par `vue_modifiee(o)` dans chacune : `manuscrit_reimporter`, `manuscrit_choisir`, `livre_modifier`, `interieur_modifier`, `maquette_choisir`, `couverture_modifier`, `image_choisir`.

Dans `projet_enregistrer`, remplacer le `vue(o)` final par `vue_enregistree(o)`.

Vérifier qu'aucun `vue(o)` ne subsiste dans une commande qui mute :

```
cd app/src-tauri && grep -n "vue(o)\|vue_modifiee(o)\|vue_enregistree(o)" src/commands.rs
```

Attendu : `vue(o)` n'apparaît plus qu'au bout de `couverture_apercu`… — c'est-à-dire nulle part si aucune commande de lecture ne rend de `ProjetVue`. Toute occurrence restante de `vue(o)` doit être justifiée : la commande ne touche pas au projet.

- [ ] **Step 6 : Lancer les tests pour les voir passer**

```
cd app/src-tauri && cargo test --lib && cargo clippy --all-targets && cargo fmt --check
```

Attendu : tout passe. Si `cargo fmt --check` échoue, lancer `cargo fmt` et relancer.

- [ ] **Step 7 : Commit**

```bash
git add app/src-tauri/src/commands.rs app/src-tauri/src/projet.rs
git commit -m "Le projet ouvert sait ce qu'il doit encore au disque"
```

---

## Task 3 : Créer, fermer, enregistrer en place

**Files:**
- Modify: `app/src-tauri/src/commands.rs`
- Modify: `app/src-tauri/src/lib.rs` (`invoke_handler`)

- [ ] **Step 1 : Écrire les trois commandes**

Ajouter dans `app/src-tauri/src/commands.rs`, juste après `projet_importer` :

```rust
/// Un projet vide, à remplir.
///
/// Ni assistant ni sélecteur de fichiers : c'est un document neuf, comme dans un
/// traitement de texte. Le manuscrit se choisit quand on veut, l'enregistrement se
/// fait quand on veut. Le projet n'est pas « modifié » : il n'y a encore rien à
/// perdre, et le premier champ saisi lèvera le drapeau.
#[tauri::command]
pub fn projet_nouveau(atelier: State<Atelier>) -> Result<ProjetVue, String> {
    poser(
        &atelier,
        None,
        Projet::nouveau(Livre::vide(), String::new()),
        false,
    )
}

/// Referme le projet sans rien écrire.
///
/// La garde des modifications appartient à l'appelant : cette commande ne demande
/// rien, elle exécute. Les séparer permet à l'interface de poser la même question
/// avant Nouveau, Ouvrir, Importer et la fermeture de la fenêtre.
#[tauri::command]
pub fn projet_fermer(atelier: State<Atelier>) {
    *atelier.ouvert.lock().unwrap() = None;
}

/// Réécrit le projet là où il a déjà été enregistré.
///
/// Sans chemin mémorisé, l'interface bascule sur « Enregistrer sous… » : elle seule
/// possède le sélecteur de fichiers.
#[tauri::command]
pub fn projet_enregistrer_courant(atelier: State<Atelier>) -> Result<ProjetVue, String> {
    let mut garde = atelier.ouvert.lock().unwrap();
    let o = garde.as_mut().ok_or_else(aucun_projet)?;
    let chemin = o
        .chemin
        .clone()
        .ok_or("projet jamais enregistré : choisir où le poser.")?;
    o.projet.enregistrer(&chemin)?;
    vue_enregistree(o)
}
```

- [ ] **Step 2 : Les déclarer**

Dans `app/src-tauri/src/lib.rs`, ajouter au `tauri::generate_handler![…]`, à la suite de `commands::projet_enregistrer` :

```rust
            commands::projet_nouveau,
            commands::projet_fermer,
            commands::projet_enregistrer_courant,
```

- [ ] **Step 3 : Compiler et vérifier**

```
cd app/src-tauri && cargo test --lib && cargo clippy --all-targets && cargo fmt --check
```

Attendu : tout passe. `projet_nouveau` est déjà couvert indirectement par `un_livre_vide_prend_le_genre_par_defaut` (Task 2) ; les commandes elles-mêmes ne sont pas testables sans `State`, et se vérifient à l'écran (Task 7).

- [ ] **Step 4 : Commit**

```bash
git add app/src-tauri/src/commands.rs app/src-tauri/src/lib.rs
git commit -m "Un projet peut naître de rien, se refermer, et se réécrire où il est"
```

---

## Task 4 : Les projets récents

**Files:**
- Modify: `app/src-tauri/src/commands.rs`
- Modify: `app/src-tauri/src/lib.rs` (`invoke_handler`)

- [ ] **Step 1 : Ajouter le pont vers les préférences**

Dans `app/src-tauri/src/commands.rs`, ajouter aux `use` du haut :

```rust
use tauri::Manager;

use crate::preferences;
```

Ajouter, juste avant la fonction `poser` (dans la zone des fonctions internes) :

```rust
/// Répertoire de configuration de l'application, s'il est atteignable.
fn config(app: &tauri::AppHandle) -> Option<PathBuf> {
    app.path().app_config_dir().ok()
}

/// Mémorise un projet dans les récents.
///
/// **Au mieux** : ne pas pouvoir écrire les préférences se signale sur la sortie
/// d'erreur et n'interrompt rien. Une liste de raccourcis perdue ne coûte rien ; un
/// enregistrement refusé parce qu'un confort a échoué coûterait un livre.
fn memoriser(app: &tauri::AppHandle, chemin: &Path) {
    let Some(dir) = config(app) else {
        eprintln!("préférences : répertoire de configuration introuvable, récents non mémorisés.");
        return;
    };
    let mut p = preferences::charger(&dir);
    p.ajouter_recent(chemin);
    if let Err(e) = preferences::enregistrer(&dir, &p) {
        eprintln!("préférences : {e}");
    }
}
```

Ajouter la commande, à la suite de `projet_enregistrer_courant` :

```rust
/// Les projets récents dont le fichier existe encore.
///
/// L'écran d'accueil et le sous-menu « Ouvrir un récent » lisent cette même liste :
/// il n'y a pas deux inventaires à tenir d'accord.
#[tauri::command]
pub fn recents_liste(app: tauri::AppHandle) -> Vec<String> {
    config(&app)
        .map(|d| preferences::charger(&d).recents_existants())
        .unwrap_or_default()
}
```

- [ ] **Step 2 : Mémoriser aux trois moments où un projet a un chemin**

Trois commandes connaissent un chemin de projet. Leur ajouter le paramètre `app: tauri::AppHandle` et l'appel à `memoriser` :

`projet_ouvrir` :

```rust
#[tauri::command]
pub fn projet_ouvrir(
    chemin: String,
    app: tauri::AppHandle,
    atelier: State<Atelier>,
) -> Result<ProjetVue, String> {
    let c = PathBuf::from(&chemin);
    let projet = Projet::ouvrir(&c)?;
    let vue = poser(&atelier, Some(c.clone()), projet, false)?;
    // Après l'ouverture réussie seulement : un projet qu'on n'a pas pu lire n'a
    // rien à faire dans une liste de raccourcis.
    memoriser(&app, &c);
    Ok(vue)
}
```

`projet_enregistrer` : même traitement, `memoriser(&app, &c)` après l'écriture réussie et avant le `vue_enregistree(o)` — attention, le verrou est encore tenu à cet endroit ; sortir l'appel du bloc verrouillé en capturant la vue d'abord :

```rust
#[tauri::command]
pub fn projet_enregistrer(
    chemin: String,
    app: tauri::AppHandle,
    atelier: State<Atelier>,
) -> Result<ProjetVue, String> {
    let c = PathBuf::from(&chemin);
    let vue = {
        let mut garde = atelier.ouvert.lock().unwrap();
        let o = garde.as_mut().ok_or_else(aucun_projet)?;
        o.projet.enregistrer(&c)?;
        o.chemin = Some(c.clone());
        vue_enregistree(o)?
    };
    memoriser(&app, &c);
    Ok(vue)
}
```

`projet_enregistrer_courant` : même forme.

```rust
#[tauri::command]
pub fn projet_enregistrer_courant(
    app: tauri::AppHandle,
    atelier: State<Atelier>,
) -> Result<ProjetVue, String> {
    let (vue, chemin) = {
        let mut garde = atelier.ouvert.lock().unwrap();
        let o = garde.as_mut().ok_or_else(aucun_projet)?;
        let chemin = o
            .chemin
            .clone()
            .ok_or("projet jamais enregistré : choisir où le poser.")?;
        o.projet.enregistrer(&chemin)?;
        (vue_enregistree(o)?, chemin)
    };
    memoriser(&app, &chemin);
    Ok(vue)
}
```

- [ ] **Step 3 : Déclarer `recents_liste`**

Dans `lib.rs`, ajouter au `generate_handler!` :

```rust
            commands::recents_liste,
```

- [ ] **Step 4 : Vérifier**

```
cd app/src-tauri && cargo test --lib && cargo clippy --all-targets && cargo fmt --check
```

Attendu : tout passe.

- [ ] **Step 5 : Commit**

```bash
git add app/src-tauri/src/commands.rs app/src-tauri/src/lib.rs
git commit -m "L'application se souvient des projets, et n'en propose aucun qui ait disparu"
```

---

## Task 5 : Le menu natif

**Files:**
- Create: `app/src-tauri/src/menu.rs`
- Modify: `app/src-tauri/src/lib.rs`

- [ ] **Step 1 : Écrire `menu.rs`**

Créer `app/src-tauri/src/menu.rs` :

```rust
//! Le menu natif, et le seul chemin par lequel ses entrées agissent.
//!
//! Aucune entrée n'exécute quoi que ce soit : chacune émet un événement que
//! l'interface traite avec le code de ses propres boutons. Il n'y a donc jamais
//! deux façons d'ouvrir un projet, seulement deux façons de demander la même — et
//! une garde des modifications à tenir à un seul endroit.
//!
//! Le menu Édition n'est pas décoratif : déclarer un menu sur mesure remplace celui
//! que Tauri pose par défaut, et ⌘C cesserait de fonctionner dans les champs de
//! saisie. Il en va de même du menu applicatif de macOS.

use tauri::menu::{MenuBuilder, MenuItemBuilder, SubmenuBuilder};
use tauri::{AppHandle, Manager, Runtime};

use crate::preferences;

/// Nom de l'événement porté à l'interface. Sa charge utile est l'identifiant de
/// l'entrée choisie.
pub const EVENEMENT: &str = "menu";

/// Préfixe des entrées « Ouvrir un récent ». Ce qui suit est le chemin du projet :
/// l'identifiant transporte la donnée, ce qui évite de tenir un index en parallèle
/// du menu.
pub const RECENT: &str = "fichier.recent:";

/// Construit le menu et le pose sur l'application.
///
/// Appelée au démarrage, puis à chaque fois que la liste des récents change : le
/// menu entier est reconstruit plutôt que retouché, parce que reconstruire est sûr
/// et que ce menu est petit.
pub fn poser<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let mut recents = SubmenuBuilder::new(app, "Ouvrir un récent");
    let liste = liste_recents(app);
    if liste.is_empty() {
        recents = recents.item(
            &MenuItemBuilder::new("Aucun projet récent")
                .enabled(false)
                .build(app)?,
        );
    } else {
        for c in &liste {
            recents = recents.text(format!("{RECENT}{c}"), c);
        }
    }
    let recents = recents.build()?;

    let fichier = SubmenuBuilder::new(app, "Fichier")
        .item(
            &MenuItemBuilder::with_id("fichier.nouveau", "Nouveau projet")
                .accelerator("CmdOrCtrl+N")
                .build(app)?,
        )
        .item(
            &MenuItemBuilder::with_id("fichier.ouvrir", "Ouvrir un projet…")
                .accelerator("CmdOrCtrl+O")
                .build(app)?,
        )
        .item(&recents)
        .separator()
        .item(&MenuItemBuilder::with_id("fichier.importer", "Importer un livre.toml…").build(app)?)
        .separator()
        .item(
            &MenuItemBuilder::with_id("fichier.enregistrer", "Enregistrer")
                .accelerator("CmdOrCtrl+S")
                .build(app)?,
        )
        .item(
            &MenuItemBuilder::with_id("fichier.enregistrer_sous", "Enregistrer sous…")
                .accelerator("CmdOrCtrl+Shift+S")
                .build(app)?,
        )
        .separator()
        // Pas de ⌘W : sous macOS il ferme la fenêtre, et l'application n'en a qu'une.
        .item(&MenuItemBuilder::with_id("fichier.fermer", "Fermer le projet").build(app)?)
        .build()?;

    let edition = SubmenuBuilder::new(app, "Édition")
        .undo()
        .redo()
        .separator()
        .cut()
        .copy()
        .paste()
        .select_all()
        .build()?;

    let aller = SubmenuBuilder::new(app, "Aller")
        .item(
            &MenuItemBuilder::with_id("aller.livre", "Livre")
                .accelerator("CmdOrCtrl+1")
                .build(app)?,
        )
        .item(
            &MenuItemBuilder::with_id("aller.interieur", "Intérieur")
                .accelerator("CmdOrCtrl+2")
                .build(app)?,
        )
        .item(
            &MenuItemBuilder::with_id("aller.couverture", "Couverture")
                .accelerator("CmdOrCtrl+3")
                .build(app)?,
        )
        .item(
            &MenuItemBuilder::with_id("aller.livraison", "Livraison")
                .accelerator("CmdOrCtrl+4")
                .build(app)?,
        )
        .build()?;

    let mut menu = MenuBuilder::new(app);
    // Sous macOS, le premier sous-menu devient le menu applicatif. Sans lui, ni
    // « À propos », ni « Masquer », ni ⌘Q.
    #[cfg(target_os = "macos")]
    {
        let appli = SubmenuBuilder::new(app, "Ozalid Studio")
            .about(None)
            .separator()
            .services()
            .separator()
            .hide()
            .hide_others()
            .show_all()
            .separator()
            .quit()
            .build()?;
        menu = menu.item(&appli);
    }
    let menu = menu.items(&[&fichier, &edition, &aller]).build()?;
    app.set_menu(menu)?;
    Ok(())
}

/// Les récents à porter au sous-menu. Même source que l'écran d'accueil, et même
/// élagage : un projet effacé n'y figure pas.
fn liste_recents<R: Runtime>(app: &AppHandle<R>) -> Vec<String> {
    app.path()
        .app_config_dir()
        .ok()
        .map(|d| preferences::charger(&d).recents_existants())
        .unwrap_or_default()
}
```

Si `SubmenuBuilder` n'expose pas l'un des raccourcis prédéfinis (`undo`, `redo`, `cut`, `copy`, `paste`, `select_all`, `about`, `services`, `hide`, `hide_others`, `show_all`, `quit`), le remplacer par l'item explicite : `PredefinedMenuItem::undo(app, None)?` posé via `.item(&…)`, avec `use tauri::menu::PredefinedMenuItem;`. Le compilateur le dira.

- [ ] **Step 2 : Brancher le menu et ses événements**

Dans `app/src-tauri/src/lib.rs`, déclarer le module (ordre alphabétique, après `maquettes`) :

```rust
pub mod menu;
```

Puis modifier la construction de l'application :

```rust
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(commands::Atelier::default())
        .setup(|app| {
            menu::poser(app.handle())?;
            Ok(())
        })
        // Le menu n'agit pas : il demande. L'interface exécute, avec le code de ses
        // propres boutons, et la garde des modifications n'a qu'un seul endroit où
        // vivre.
        .on_menu_event(|app, ev| {
            use tauri::Emitter;
            if let Err(e) = app.emit(menu::EVENEMENT, ev.id().as_ref()) {
                eprintln!("menu : événement non transmis à l'interface : {e}");
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::providers_liste,
            commands::projet_nouveau,
            commands::projet_importer,
            commands::projet_ouvrir,
            commands::projet_enregistrer,
            commands::projet_enregistrer_courant,
            commands::projet_fermer,
            commands::recents_liste,
            commands::manuscrit_choisir,
            commands::manuscrit_reimporter,
            commands::livre_modifier,
            commands::maquettes_liste,
            commands::polices_liste,
            commands::polices_texte_liste,
            commands::interieur_modifier,
            commands::epreuve_tirer,
            commands::maquette_choisir,
            commands::couverture_modifier,
            commands::image_choisir,
            commands::couverture_apercu,
            commands::composer,
            commands::packager
        ])
        .run(tauri::generate_context!())
        .expect("démarrage de l'application impossible");
}
```

`commands::garde_modifications` s'y ajoutera en Task 6.

- [ ] **Step 3 : Reconstruire le menu quand les récents changent**

Dans `app/src-tauri/src/commands.rs`, à la fin de `memoriser`, après l'enregistrement réussi :

```rust
    // Le sous-menu des récents vient d'être périmé par cette écriture : le
    // reconstruire ici évite d'avoir à s'en souvenir à chaque point d'appel.
    if let Err(e) = crate::menu::poser(app) {
        eprintln!("menu : reconstruction impossible : {e}");
    }
```

- [ ] **Step 4 : Vérifier la compilation**

```
cd app/src-tauri && cargo test --lib && cargo clippy --all-targets && cargo fmt --check
```

Attendu : tout passe.

- [ ] **Step 5 : Vérifier à l'écran**

```
cd app/src-tauri && cargo tauri dev
```

À vérifier, et à noter comme fait :
- les menus **Fichier**, **Édition**, **Aller** apparaissent, plus le menu applicatif sous macOS ;
- ⌘C et ⌘V fonctionnent dans le champ « Titre » — c'est le test qui prouve que le menu Édition n'a pas été perdu ;
- « Ouvrir un récent » affiche « Aucun projet récent » grisé au premier lancement ;
- cliquer sur une entrée ne fait encore rien de visible : l'interface ne les écoute pas avant la Task 8. Vérifier dans la console du webview (clic droit → Inspecter) qu'aucune erreur n'apparaît.

- [ ] **Step 6 : Commit**

```bash
git add app/src-tauri/src/menu.rs app/src-tauri/src/lib.rs app/src-tauri/src/commands.rs
git commit -m "Le menu demande, l'interface exécute, et Édition survit au menu sur mesure"
```

---

## Task 6 : La garde des modifications

**Files:**
- Modify: `app/src-tauri/src/commands.rs`
- Modify: `app/src-tauri/src/lib.rs`
- Modify: `app/src-tauri/capabilities/default.json`

- [ ] **Step 1 : Écrire la commande de garde**

Ajouter dans `app/src-tauri/src/commands.rs`, à la suite de `recents_liste` :

```rust
/// Libellés des trois boutons de la garde.
///
/// Ce sont eux qui font foi au retour : avec une variante personnalisée, le plugin
/// rend `MessageDialogResult::Custom(libellé)` et non un `Yes`/`No`. Les garder en
/// constantes évite que la comparaison et l'affichage divergent.
const ENREGISTRER: &str = "Enregistrer";
const IGNORER: &str = "Ne pas enregistrer";
const ANNULER: &str = "Annuler";

/// Demande quoi faire des modifications non enregistrées.
///
/// Rend `"enregistrer"`, `"ignorer"` ou `"annuler"`, et `"ignorer"` d'emblée quand
/// il n'y a rien à perdre. La commande **ne fait rien** de la réponse : c'est
/// l'interface qui agit, parce qu'elle seule possède le sélecteur de fichiers dont
/// « Enregistrer sous… » a besoin.
///
/// `async` par nécessité : `blocking_show_with_result` bloque son fil jusqu'au clic,
/// et le plugin interdit de l'appeler depuis le fil principal — ce qui serait le cas
/// d'une commande synchrone.
#[tauri::command]
pub async fn garde_modifications(
    app: tauri::AppHandle,
    atelier: State<'_, Atelier>,
) -> Result<String, String> {
    // Le verrou est relâché avant la boîte : la tenir pendant que l'utilisateur
    // réfléchit condamnerait toute autre commande.
    let modifie = {
        let garde = atelier.ouvert.lock().unwrap();
        garde.as_ref().is_some_and(|o| o.modifie)
    };
    if !modifie {
        return Ok("ignorer".into());
    }

    use tauri_plugin_dialog::{
        DialogExt, MessageDialogButtons, MessageDialogKind, MessageDialogResult,
    };
    let reponse = app
        .dialog()
        .message("Ce projet porte des modifications qui ne sont pas enregistrées.")
        .title("Enregistrer avant de continuer ?")
        .kind(MessageDialogKind::Warning)
        .buttons(MessageDialogButtons::YesNoCancelCustom(
            ENREGISTRER.into(),
            IGNORER.into(),
            ANNULER.into(),
        ))
        .blocking_show_with_result();

    Ok(match reponse {
        MessageDialogResult::Custom(s) if s == ENREGISTRER => "enregistrer",
        MessageDialogResult::Custom(s) if s == IGNORER => "ignorer",
        // Filet : si une plateforme rendait les valeurs canoniques plutôt que les
        // libellés, le sens resterait le même. Tout le reste — fermeture de la
        // boîte comprise — est un refus, parce que c'est le choix qui ne perd rien.
        MessageDialogResult::Yes => "enregistrer",
        MessageDialogResult::No => "ignorer",
        _ => "annuler",
    }
    .to_string())
}
```

- [ ] **Step 2 : Déclarer la commande et garder la fenêtre**

Dans `lib.rs`, ajouter `commands::garde_modifications,` au `generate_handler!`, puis ajouter le gestionnaire de fenêtre à la construction :

```rust
        // Fermer la fenêtre, c'est fermer l'application : la même garde doit s'y
        // appliquer. Elle ne peut pas être posée ici — la réponse « Enregistrer »
        // demande un sélecteur de fichiers, que seule l'interface possède — donc on
        // retient la fermeture et on lui passe la main.
        .on_window_event(|fenetre, ev| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = ev {
                use tauri::Emitter;
                api.prevent_close();
                if let Err(e) = fenetre.emit("fermeture-demandee", ()) {
                    // L'interface est injoignable : mieux vaut fermer que coincer
                    // l'utilisateur dans une fenêtre qui refuse de partir.
                    eprintln!("fermeture : interface injoignable ({e}), fermeture forcée.");
                    let _ = fenetre.destroy();
                }
            }
        })
```

- [ ] **Step 3 : Ouvrir les permissions dont le front a besoin**

Remplacer `app/src-tauri/capabilities/default.json` par :

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "description": "Permissions de la fenêtre principale.",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "core:event:allow-listen",
    "core:window:allow-destroy",
    "dialog:allow-open",
    "dialog:allow-save"
  ]
}
```

`core:window:allow-destroy` est ce qui permet à l'interface de refermer la fenêtre une fois la garde franchie ; `core:event:allow-listen` lui permet d'écouter les événements du menu et de la fermeture.

- [ ] **Step 4 : Vérifier**

```
cd app/src-tauri && cargo test --lib && cargo clippy --all-targets && cargo fmt --check
```

Attendu : tout passe. Le comportement se vérifiera à l'écran en Task 8, quand l'interface écoutera.

Attention : à ce stade, la fenêtre **refuse de se fermer** (la fermeture est retenue et personne ne l'écoute encore). Pour arrêter `cargo tauri dev`, utiliser ⌘Q ou interrompre le terminal. C'est temporaire et réglé en Task 8.

- [ ] **Step 5 : Commit**

```bash
git add app/src-tauri/src/commands.rs app/src-tauri/src/lib.rs app/src-tauri/capabilities/default.json
git commit -m "Rien ne se perd sans qu'on ait demandé, fermeture de la fenêtre comprise"
```

---

## Task 7 : L'interface — accueil, état, récents

**Files:**
- Modify: `app/src/index.html:16-24`
- Modify: `app/src/app.js`
- Modify: `app/src/styles.css`

- [ ] **Step 1 : Le balisage**

Dans `app/src/index.html`, remplacer la section « Projet » entière (lignes 16 à 24) par :

```html
  <section>
    <h2>Projet</h2>
    <div class="ligne">
      <button id="btNouveau" type="button">Nouveau projet</button>
      <button id="btOuvrir" type="button">Ouvrir un .ozalid…</button>
      <button id="btImporter" type="button">Importer un livre.toml…</button>
      <button id="btEnregistrer" type="button" disabled>Enregistrer</button>
      <button id="btEnregistrerSous" type="button" disabled>Enregistrer sous…</button>
    </div>
    <p class="chemin" id="cheminProjet">aucun projet ouvert</p>
    <p class="etat" id="etatEnregistrement"></p>
    <div id="recents" class="recents"></div>
  </section>
```

`btEnregistrer` change de rôle : il enregistre en place. « Enregistrer sous… » prend son ancien comportement.

- [ ] **Step 2 : Le style**

Ajouter dans `app/src/styles.css`, avant la section « masquage » finale :

```css
/* ---------- projets récents ---------- */

.recents { margin-top: .8rem; display: grid; gap: .2rem; justify-items: start; }

.recents button {
  background: transparent;
  border-color: transparent;
  color: var(--encre);
  padding: .15rem 0;
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  font-size: .78rem;
  text-align: left;
}

.recents button:hover { text-decoration: underline; }
```

- [ ] **Step 3 : Le câblage**

Dans `app/src/app.js`, remplacer le bloc `/* ---------- projet ---------- */`, c'est-à-dire les fonctions `afficherProjet`, `ouvrir`, `importer` et `enregistrer`, par ce qui suit.

**Attention :** la fonction `tente` se trouve entre `afficherProjet` et `ouvrir` dans le fichier actuel, avec son long commentaire. Elle **ne fait pas partie du remplacement** et doit rester telle quelle — tout le reste du fichier s'en sert. Le plus sûr est de remplacer les fonctions une par une plutôt que le bloc d'un seul tenant.

```js
/* ---------- projet ---------- */

function afficherProjet(p) {
  projet = p;
  $('cheminProjet').textContent = p.chemin ?? 'projet non enregistré';
  $('btEnregistrer').disabled = !p.chemin;
  $('btEnregistrerSous').disabled = false;
  $('etatEnregistrement').textContent = p.modifie
    ? 'modifié'
    : (p.chemin ? 'enregistré' : 'jamais enregistré');
  $('recents').hidden = true;
  for (const s of ['secLivre', 'secManuscrit', 'secInterieur', 'secCouverture',
                   'secComposer', 'secPackages', 'secEpreuve']) {
    $(s).hidden = false;
  }

  $('inTitre').value = p.livre.titre;
  $('inTitrePage').value = p.livre.titre_page ?? '';
  $('inAuteur').value = p.livre.auteur;
  $('inGenre').value = p.livre.genre;
  $('inCopyright').value = p.livre.copyright;
  $('inChapitres').value = p.livre.chapitres ?? '';
  $('inPoliceInterieur').value = p.interieur.police;

  const attendu = p.livre.chapitres;
  const ecart = attendu !== null && attendu !== undefined && attendu !== p.chapitres_trouves;
  const em = $('etatManuscrit');
  // Un manuscrit absent et un manuscrit sans chapitre composable comptent tous deux
  // zéro : seul le Rust sait lequel des deux, et ce n'est pas la même chose à faire.
  if (p.manuscrit_absent) {
    em.textContent = 'Aucun manuscrit : en choisir un pour composer le livre.';
    em.className = 'note';
  } else {
    em.textContent = ecart
      ? `${p.chapitres_trouves} chapitres dans le manuscrit embarqué, ${attendu} attendus `
        + '— manuscrit périmé ou contrôle d\'intégrité à corriger.'
      : `${p.chapitres_trouves} chapitres, ${p.mots.toLocaleString('fr-FR')} mots.`;
    em.className = ecart ? 'note alerte' : 'note';
  }

  $('sourceManuscrit').textContent = p.manuscrit_source ?? 'aucune source mémorisée';
  $('btReimporter').disabled = !p.manuscrit_source;

  $('etatImages').textContent = p.images.length
    ? `Photos source : ${p.images.join(', ')}.`
    : 'Aucune photo source : les modes Bandeau et Surimpression composeront sur le papier seul.';

  $('etatCouverture').textContent = p.couverture
    ? ''
    : 'Aucune maquette : en choisir une pour composer la couverture.';
  $('reglages').hidden = !p.couverture;
  if (p.couverture) afficherCouverture(p.couverture);
  demanderApercu();
}

/**
 * L'écran sans projet : les rubriques disparaissent, les récents s'offrent.
 *
 * Appelé au démarrage et après « Fermer le projet ». Il ne se contente pas de vider
 * l'affichage : il remet `projet` à null, faute de quoi l'aperçu continuerait de se
 * composer sur un livre qui n'est plus ouvert.
 */
async function afficherAucunProjet() {
  projet = null;
  dosCompose = null;
  $('cheminProjet').textContent = 'aucun projet ouvert';
  $('etatEnregistrement').textContent = '';
  $('btEnregistrer').disabled = true;
  $('btEnregistrerSous').disabled = true;
  for (const s of ['secLivre', 'secManuscrit', 'secInterieur', 'secCouverture',
                   'secComposer', 'secPackages', 'secEpreuve']) {
    $(s).hidden = true;
  }
  await afficherRecents();
}

async function afficherRecents() {
  const box = $('recents');
  box.replaceChildren();
  const liste = await invoke('recents_liste');
  if (liste.length) {
    box.append(h('p', 'Projets récents', 'note'));
    for (const c of liste) {
      const b = h('button', c);
      b.type = 'button';
      b.addEventListener('click', () => ouvrirChemin(c));
      box.append(b);
    }
  }
  box.hidden = !liste.length;
}

/**
 * La garde : ce qui protège du travail non enregistré.
 *
 * Rend vrai quand l'appelant peut poursuivre. Le Rust pose la question et rend le
 * choix ; l'interface l'exécute, parce qu'elle seule possède le sélecteur de
 * fichiers dont « Enregistrer sous… » a besoin.
 */
async function garde() {
  const choix = await invoke('garde_modifications');
  if (choix === 'annuler') return false;
  if (choix === 'enregistrer') return enregistrerQuelquePart();
  return true;
}

/** Enregistre en place si le projet a un chemin, sinon demande où. Rend vrai si écrit. */
async function enregistrerQuelquePart() {
  if (projet?.chemin) {
    try {
      afficherProjet(await invoke('projet_enregistrer'));
      return true;
    } catch (e) {
      $('etat').textContent = String(e);
      $('etat').className = 'etat erreur';
      return false;
    }
  }
  return enregistrerSous();
}

async function nouveau() {
  if (!await garde()) return;
  await tente(async () => afficherProjet(await invoke('projet_nouveau')));
}

async function fermer() {
  if (!await garde()) return;
  await invoke('projet_fermer');
  await afficherAucunProjet();
}

async function ouvrir() {
  if (!await garde()) return;
  const choix = await open({
    multiple: false,
    filters: [{ name: 'Projet Ozalid', extensions: ['ozalid'] }],
  });
  if (!choix) return;
  await ouvrirChemin(choix);
}

async function ouvrirChemin(chemin) {
  await tente(async () => afficherProjet(await invoke('projet_ouvrir', { chemin })));
}

async function importer() {
  if (!await garde()) return;
  const choix = await open({
    multiple: false,
    filters: [{ name: 'Livre de l\'ancienne chaîne', extensions: ['toml'] }],
  });
  if (!choix) return;
  await tente(async () =>
    afficherProjet(await invoke('projet_importer', { livreToml: choix })));
}

/** « Enregistrer sous… » : demande où poser le projet. Rend vrai si écrit. */
async function enregistrerSous() {
  const choix = await save({
    defaultPath: `${projet.livre.titre || 'projet'}.ozalid`,
    filters: [{ name: 'Projet Ozalid', extensions: ['ozalid'] }],
  });
  if (!choix) return false;
  try {
    afficherProjet(await invoke('projet_enregistrer_sous', { chemin: choix }));
    return true;
  } catch (e) {
    $('etat').textContent = String(e);
    $('etat').className = 'etat erreur';
    return false;
  }
}
```

Attention à `ouvrirChemin` : `tente` avale l'erreur et réaffiche le projet courant. C'est le comportement voulu — un récent illisible affiche son erreur sans fermer ce qui est ouvert.

- [ ] **Step 4 : Les écouteurs**

Remplacer, en bas d'`app.js`, les trois lignes d'écouteurs du projet :

```js
$('btNouveau').addEventListener('click', nouveau);
$('btOuvrir').addEventListener('click', ouvrir);
$('btImporter').addEventListener('click', importer);
$('btEnregistrer').addEventListener('click', enregistrerQuelquePart);
$('btEnregistrerSous').addEventListener('click', enregistrerSous);
```

Et remplacer l'appel final `chargerProviders();` par :

```js
chargerProviders().then(afficherAucunProjet);
```

`chargerProviders` construit les contrôles dont `afficherProjet` a besoin : afficher l'accueil avant qu'elle ait fini laisserait un panneau à moitié bâti.

- [ ] **Step 5 : Vérifier à l'écran**

```
cd app/src-tauri && cargo tauri dev
```

À vérifier :
- au démarrage, aucune rubrique n'est visible et « aucun projet ouvert » s'affiche ;
- « Nouveau projet » fait apparaître les rubriques, l'état dit « jamais enregistré », « Enregistrer » reste grisé, « Enregistrer sous… » est actif ;
- l'étape Manuscrit dit « Aucun manuscrit », pas « 0 chapitres » ;
- taper un titre puis quitter le champ fait passer l'état à « modifié » ;
- « Enregistrer sous… » écrit le fichier, l'état passe à « enregistré », « Enregistrer » s'active ;
- « Nouveau projet » sur un projet modifié pose la boîte à trois boutons ; « Annuler » ne change rien, « Ne pas enregistrer » repart à vide, « Enregistrer » écrit puis repart à vide ;
- après avoir enregistré au moins un projet, l'écran d'accueil (atteint en fermant puis rouvrant l'application) affiche le projet en récent, et cliquer dessus l'ouvre.

- [ ] **Step 6 : Commit**

```bash
git add app/src/index.html app/src/app.js app/src/styles.css
git commit -m "L'accueil offre ce qu'on peut faire, et l'entête dit ce qu'on doit au disque"
```

---

## Task 8 : L'interface écoute le menu et la fermeture

**Files:**
- Modify: `app/src/app.js`
- Modify: `app/tests/dom_shim.js`

- [ ] **Step 1 : Ouvrir le faux DOM aux événements**

Dans `app/tests/dom_shim.js`, remplacer la signature et le contexte de `charge` :

```js
async function charge({
  ids,
  invoke,
  open = async () => null,
  save = async () => null,
  listen = async () => () => {},
  destroy = () => {},
}) {
```

et, dans l'objet `contexte`, remplacer la ligne `window:` par :

```js
    window: {
      __TAURI__: {
        core: { invoke },
        dialog: { open, save },
        event: { listen },
        window: { getCurrentWindow: () => ({ destroy }) },
      },
    },
```

Ajouter ce commentaire au-dessus, dans le même esprit que les autres :

```js
  // Le menu natif et la fermeture de fenêtre passent par des événements : sans
  // `event.listen` dans le faux contexte, `app.js` lèverait au chargement et aucun
  // test ne s'exécuterait.
```

- [ ] **Step 2 : Écrire le test du routage, dans un fichier neuf**

Créer `app/tests/cycle_de_vie.test.js` :

```js
'use strict';

// Câblage du cycle de vie : ce que l'interface envoie au Rust selon la réponse de la
// garde, et ce que le menu déclenche. La boîte de dialogue elle-même est native :
// elle se vérifie dans l'application, pas ici.

const test = require('node:test');
const assert = require('node:assert');
const { charge } = require('./dom_shim');

const IDS = [
  'btNouveau', 'btOuvrir', 'btImporter', 'btEnregistrer', 'btEnregistrerSous',
  'cheminProjet', 'etatEnregistrement', 'recents',
  'secLivre', 'secManuscrit', 'secCouverture', 'secComposer',
  'inTitre', 'inTitrePage', 'inAuteur', 'inGenre', 'inCopyright', 'inChapitres',
  'etatManuscrit', 'sourceManuscrit', 'btReimporter', 'btChoisirManuscrit',
  'etatImages', 'btImageUne', 'btImageQuatre',
  'maquettes', 'etatCouverture', 'faces', 'apercu', 'etatApercu', 'reglages',
  'inProvider', 'inPapier', 'noteFormat',
  'btComposer', 'etat', 'resultat',
  'secPackages', 'listePrestataires', 'btPackager', 'etatPackages', 'packages',
  'secInterieur', 'inPoliceInterieur',
  'secEpreuve', 'inEpreuveCorps', 'btEpreuve', 'etatEpreuve', 'cheminEpreuve',
];

const LULU = {
  cle: 'lulu', libelle: 'Lulu — poche 108 × 175',
  largeur: 108, hauteur: 175, fond_perdu: 3.175, dos_publie: true,
  papiers: [{ cle: 'standard', libelle: 'Papier standard' }],
};

function projet(sur = {}) {
  return {
    chemin: '/livres/LHC.ozalid',
    livre: {
      titre: 'Les Heures creuses', titre_page: null, auteur: 'Ivan Pjig',
      genre: 'roman', copyright: '', chapitres: null,
    },
    manuscrit_source: null,
    chapitres_trouves: 1,
    mots: 12,
    manuscrit_absent: false,
    modifie: false,
    couverture: null,
    couverture_importee: false,
    images: [],
    interieur: { police: 'EB Garamond' },
    ...sur,
  };
}

/** Un atelier de test : enregistre les commandes reçues, rend des vues plausibles. */
function atelier({ garde = 'ignorer', recents = [], sur = {} } = {}) {
  const appels = [];
  const invoke = async (cmd, args) => {
    appels.push([cmd, args]);
    switch (cmd) {
      case 'providers_liste': return [LULU];
      case 'polices_liste': return ['Bodoni Moda'];
      case 'polices_texte_liste': return ['EB Garamond'];
      case 'maquettes_liste': return [];
      case 'recents_liste': return recents;
      case 'garde_modifications': return garde;
      case 'projet_fermer': return null;
      case 'couverture_apercu': throw new Error('pas de maquette');
      default: return projet(sur);
    }
  };
  return { appels, invoke, noms: () => appels.map(([c]) => c) };
}

test('sans projet, aucune rubrique n\'est offerte et les récents s\'affichent', async () => {
  const a = atelier({ recents: ['/livres/A.ozalid', '/livres/B.ozalid'] });
  const { els } = await charge({ ids: IDS, invoke: a.invoke });

  assert.equal(els.get('secLivre').hidden, true);
  assert.equal(els.get('btEnregistrer').disabled, true);
  assert.equal(els.get('cheminProjet').textContent, 'aucun projet ouvert');
  assert.deepEqual(els.get('recents').textes('BUTTON'),
    ['/livres/A.ozalid', '/livres/B.ozalid']);
});

test('cliquer un récent ouvre ce projet-là', async () => {
  const a = atelier({ recents: ['/livres/A.ozalid'] });
  const { els } = await charge({ ids: IDS, invoke: a.invoke });

  await els.get('recents').enfants.find((e) => e.tagName === 'BUTTON').declenche('click');

  const ouvre = a.appels.find(([c]) => c === 'projet_ouvrir');
  assert.deepEqual(ouvre[1], { chemin: '/livres/A.ozalid' });
  assert.equal(els.get('secLivre').hidden, false);
});

test('la garde refusée arrête tout : rien n\'est ouvert, rien n\'est perdu', async () => {
  const a = atelier({ garde: 'annuler' });
  const { els } = await charge({ ids: IDS, invoke: a.invoke });

  await els.get('btNouveau').declenche('click');

  assert.ok(!a.noms().includes('projet_nouveau'),
    'un « Annuler » qui crée quand même le projet aurait perdu le travail');
  assert.equal(els.get('secLivre').hidden, true);
});

test('la garde acceptée laisse passer', async () => {
  const a = atelier({ garde: 'ignorer' });
  const { els } = await charge({ ids: IDS, invoke: a.invoke });

  await els.get('btNouveau').declenche('click');

  assert.ok(a.noms().includes('projet_nouveau'));
  assert.equal(els.get('secLivre').hidden, false);
});

test('« Enregistrer » réécrit en place, sans sélecteur de fichiers', async () => {
  const a = atelier();
  let demande = 0;
  const { els } = await charge({
    ids: IDS,
    invoke: a.invoke,
    save: async () => { demande += 1; return '/ailleurs.ozalid'; },
  });
  await els.get('btNouveau').declenche('click');   // ouvre un projet qui a un chemin
  await els.get('btEnregistrer').declenche('click');

  assert.ok(a.noms().includes('projet_enregistrer'));
  assert.equal(demande, 0, 'un projet déjà posé ne redemande pas où');
});

test('un projet jamais enregistré n\'offre que « Enregistrer sous… »', async () => {
  const a = atelier({ sur: { chemin: null, modifie: false } });
  const { els } = await charge({ ids: IDS, invoke: a.invoke });
  await els.get('btNouveau').declenche('click');

  assert.equal(els.get('btEnregistrer').disabled, true);
  assert.equal(els.get('btEnregistrerSous').disabled, false);
  assert.equal(els.get('etatEnregistrement').textContent, 'jamais enregistré');
});

test('l\'état d\'enregistrement suit le drapeau du Rust', async () => {
  const a = atelier({ sur: { modifie: true } });
  const { els } = await charge({ ids: IDS, invoke: a.invoke });
  await els.get('btNouveau').declenche('click');

  assert.equal(els.get('etatEnregistrement').textContent, 'modifié');
});

test('un manuscrit absent se dit absent, et non vide de chapitres', async () => {
  const a = atelier({ sur: { manuscrit_absent: true, chapitres_trouves: 0, mots: 0 } });
  const { els } = await charge({ ids: IDS, invoke: a.invoke });
  await els.get('btNouveau').declenche('click');

  assert.match(els.get('etatManuscrit').textContent, /Aucun manuscrit/);
  assert.doesNotMatch(els.get('etatManuscrit').textContent, /0 chapitres/);
});

test('le menu passe par le même code que les boutons', async () => {
  const a = atelier();
  let router;
  const { els } = await charge({
    ids: IDS,
    invoke: a.invoke,
    listen: async (nom, fn) => { if (nom === 'menu') router = fn; return () => {}; },
  });

  await router({ payload: 'fichier.nouveau' });
  assert.ok(a.noms().includes('projet_nouveau'));
  assert.equal(els.get('secLivre').hidden, false);

  await router({ payload: 'fichier.fermer' });
  assert.ok(a.noms().includes('projet_fermer'));
  assert.equal(els.get('secLivre').hidden, true);
});

test('un récent du menu porte son chemin dans son identifiant', async () => {
  const a = atelier();
  let router;
  await charge({
    ids: IDS,
    invoke: a.invoke,
    listen: async (nom, fn) => { if (nom === 'menu') router = fn; return () => {}; },
  });

  await router({ payload: 'fichier.recent:/livres/Z.ozalid' });

  const ouvre = a.appels.find(([c]) => c === 'projet_ouvrir');
  assert.deepEqual(ouvre[1], { chemin: '/livres/Z.ozalid' });
});

test('la fenêtre ne se ferme que si la garde le permet', async () => {
  const refuse = atelier({ garde: 'annuler' });
  let fermetures = 0;
  let surFermeture;
  await charge({
    ids: IDS,
    invoke: refuse.invoke,
    listen: async (nom, fn) => { if (nom === 'fermeture-demandee') surFermeture = fn; return () => {}; },
    destroy: () => { fermetures += 1; },
  });

  await surFermeture({});
  assert.equal(fermetures, 0, 'un « Annuler » qui ferme quand même perdrait tout');

  const accepte = atelier({ garde: 'ignorer' });
  let fermee = 0;
  let surFermeture2;
  await charge({
    ids: IDS,
    invoke: accepte.invoke,
    listen: async (nom, fn) => { if (nom === 'fermeture-demandee') surFermeture2 = fn; return () => {}; },
    destroy: () => { fermee += 1; },
  });

  await surFermeture2({});
  assert.equal(fermee, 1);
});
```

- [ ] **Step 3 : Lancer les tests pour les voir échouer**

```
cd app && node --test "tests/cycle_de_vie.test.js"
```

Attendu : échecs — `router is not a function`, `surFermeture is not a function`, et les tests de garde qui passent déjà par accident (Task 7 les a câblés). Seuls ceux du menu et de la fermeture doivent échouer.

- [ ] **Step 4 : Écrire le routage dans `app.js`**

Ajouter en haut d'`app.js`, sous les deux `const` existants :

```js
const { listen } = window.__TAURI__.event;
const { getCurrentWindow } = window.__TAURI__.window;
```

Ajouter, avant le bloc des écouteurs de fin de fichier :

```js
/* ---------- menu natif ---------- */

/**
 * Ce que chaque entrée du menu déclenche.
 *
 * Les valeurs sont les fonctions des boutons, pas des copies : le menu et la souris
 * font la même chose, et la garde des modifications n'a qu'un endroit où vivre.
 */
const MENU = {
  'fichier.nouveau': nouveau,
  'fichier.ouvrir': ouvrir,
  'fichier.importer': importer,
  'fichier.enregistrer': enregistrerQuelquePart,
  'fichier.enregistrer_sous': enregistrerSous,
  'fichier.fermer': fermer,
};

/** Préfixe des entrées « Ouvrir un récent » ; ce qui suit est le chemin du projet. */
const RECENT = 'fichier.recent:';

async function routerMenu(id) {
  if (id.startsWith(RECENT)) {
    if (!await garde()) return;
    await ouvrirChemin(id.slice(RECENT.length));
    return;
  }
  await MENU[id]?.();
}

listen('menu', (ev) => routerMenu(ev.payload));

/**
 * La fenêtre a demandé à se fermer, et le Rust a retenu la fermeture.
 *
 * C'est ici qu'elle se conclut : la garde d'abord, la destruction ensuite. Le Rust
 * ne peut pas s'en charger — répondre « Enregistrer » demande un sélecteur de
 * fichiers, que seule l'interface possède.
 */
listen('fermeture-demandee', async () => {
  if (await garde()) getCurrentWindow().destroy();
});
```

Le menu « Aller » n'a pas d'entrée dans `MENU` : les quatre étapes n'existent pas encore. `MENU[id]?.()` les ignore sans erreur, et le lot 2 les branchera.

- [ ] **Step 5 : Lancer les tests pour les voir passer**

```
cd app && node --test "tests/cycle_de_vie.test.js"
```

Attendu : `pass 11`, `fail 0`.

- [ ] **Step 6 : Réparer les tests existants**

Les quatre fichiers de test existants construisent une `ProjetVue` sans les champs neufs et une liste `IDS` sans les identifiants neufs. Dans `composition.test.js`, `couverture.test.js`, `epreuve.test.js` et `packages.test.js` :

1. ajouter à chaque liste `IDS` : `'btNouveau'`, `'btEnregistrerSous'`, `'etatEnregistrement'`, `'recents'` ;
2. ajouter à chaque objet `PROJET` (ou équivalent) : `manuscrit_absent: false`, `modifie: false` ;
3. faire répondre chaque faux `invoke` à deux commandes neuves : `recents_liste` doit rendre `[]` et `garde_modifications` doit rendre `'ignorer'`. Sans quoi ces faux `invoke` rendraient une `ProjetVue` là où le code attend un tableau ou une chaîne, et les tests tomberaient sur un symptôme qui ne dit rien de leur objet.

Lancer :

```
cd app && node --test "tests/*.test.js"
```

Attendu : tout passe. **Aucun test ne doit être ignoré ni commenté** — un test mis de côté ici, c'est la seule garde automatique du front qui disparaît.

- [ ] **Step 7 : Vérifier à l'écran**

```
cd app/src-tauri && cargo tauri dev
```

À vérifier :
- ⌘N, ⌘O, ⌘S, ⇧⌘S font ce que les boutons font ;
- « Ouvrir un récent » liste les projets enregistrés, et l'un d'eux s'ouvre ;
- fermer la fenêtre avec un projet modifié pose la boîte ; « Annuler » laisse la fenêtre ouverte, « Ne pas enregistrer » la ferme, « Enregistrer » écrit puis ferme ;
- fermer la fenêtre sans modification ferme sans rien demander ;
- ⌘C et ⌘V fonctionnent toujours dans le champ « Titre ».

- [ ] **Step 8 : Commit**

```bash
git add app/src/app.js app/tests/
git commit -m "Le menu et la souris demandent la même chose, et la fenêtre attend la réponse"
```

---

## Task 9 : La documentation et le témoin

**Files:**
- Modify: `app/README.md`

- [ ] **Step 1 : Compléter la table des modules**

Dans `app/README.md`, dans la table « Modules », ajouter deux lignes, avant la ligne `commands` :

```markdown
| `preferences` | Le `preferences.toml` : projets récents, et ce qui ne tient pas dans un livre |
| `menu` | Le menu natif : il demande, il n'agit pas — l'interface exécute |
```

- [ ] **Step 2 : Écrire la section du cycle de vie**

Ajouter dans `app/README.md`, après la section « Le fichier .ozalid » :

```markdown
## Le cycle de vie d'un projet

Un `.ozalid` est un document : il se crée vide, se remplit, s'enregistre et se
ferme. « Nouveau projet » ne demande rien — ni assistant, ni manuscrit d'emblée :
le texte se choisit quand on veut.

L'atelier retient un drapeau **modifié**, levé par toute commande qui touche au
projet et abaissé à l'écriture. C'est lui, et lui seul, qui décide si fermer perd
du travail : Nouveau, Ouvrir, Importer, Fermer et la fermeture de la fenêtre posent
alors une boîte à trois boutons — Enregistrer, Ne pas enregistrer, Annuler.

Le Rust pose la question ; **l'interface exécute la réponse**. C'est elle qui
possède le sélecteur de fichiers dont « Enregistrer sous… » a besoin, et c'est la
raison pour laquelle la fermeture de la fenêtre est retenue côté Rust puis rendue
à l'interface plutôt que tranchée sur place.

Le menu natif suit la même règle : aucune entrée n'agit, chacune émet un événement
que l'interface traite avec le code de ses propres boutons. Les boutons de l'écran
d'accueil sont des raccourcis du menu, pas une seconde vérité.

Les **projets récents** vivent dans un `preferences.toml` du répertoire de
configuration de l'application — jamais dans un `.ozalid`, qui porte le livre et
non les habitudes de celui qui l'ouvre. La liste est plafonnée à dix, et les
chemins dont le fichier a disparu sont élagués **à la lecture** : un projet sur un
volume démonté revient de lui-même au remontage, alors qu'une purge l'aurait perdu.
Son écriture est au mieux : ne pas pouvoir l'enregistrer se signale et n'interrompt
rien.
```

- [ ] **Step 3 : Rejouer le témoin de non-régression**

Ce lot ne touche à aucun moteur de composition. Le compte de pages doit donc être identique.

```
cd app/src-tauri && cargo run --example temoin
```

Attendu : le témoin passe. Il porte sa propre valeur attendue et échoue au lieu d'afficher un résultat à interpréter — s'il tombe, quelque chose a bougé qui n'aurait pas dû.

- [ ] **Step 4 : Vérification d'ensemble**

```
cd app/src-tauri && cargo test --lib && cargo clippy --all-targets && cargo fmt --check
cd app && node --test "tests/*.test.js"
```

Attendu : tout passe, aucun test ignoré.

- [ ] **Step 5 : Commit**

```bash
git add app/README.md
git commit -m "Le README dit comment un projet naît, se garde et se souvient"
```

---

## Ce que ce lot ne fait pas

- La page reste unique : les huit sections sont toujours empilées, le double ascenseur est intact. C'est le lot 2.
- Le menu **Aller** existe mais ne va nulle part : ses quatre entrées sont ignorées jusqu'à ce que les étapes existent.
- Aucune préférence autre que les récents n'est livrée.
- Le prestataire est toujours choisi à deux endroits. C'est le lot 3.
- La palette ne change pas. C'est le lot 4.
