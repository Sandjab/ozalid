# Le détourage des envois photographiés — plan d'implémentation

> **Pour un agent qui exécute :** SOUS-SKILL REQUIS — `superpowers:subagent-driven-development`
> (recommandé) ou `superpowers:executing-plans`, tâche par tâche. Les étapes sont en
> cases à cocher (`- [ ]`).

Spec : `docs/superpowers/specs/2026-08-25-envoi-detourage-design.md`

**But.** La photo d'un envoi écrit à la main perd son fond avant d'atteindre Typst, et
l'aperçu se juge sur la couleur du papier réel.

**Architecture.** Un module pur, `detourage.rs`, sépare l'encre du papier par deux
seuils de luminance. Il est appelé en **un seul endroit** — `package::trace`, par où
passent la composition d'un package *et* le rendu de l'objet que le canevas manipule —
si bien que l'écran ne peut pas diverger du tirage. L'archive garde la photo d'origine ;
le détourage se rejoue à chaque composition.

**Outils.** Rust (Tauri 2), crate `image` 0.25 restreinte à `jpeg`+`png`, Typst 0.15.1
en sidecar, front vanilla sans bundler, `node --test` avec `dom_shim`.

---

## Ce que l'écriture du plan a trouvé

**`Quoi::Image` emprunte son nom, et `Quoi` est `Copy`.** Le détourage produit un PNG,
donc un nom en `.png`, qui n'est plus celui de l'archive (`Léa.jpg`). Il faut un
`Cow<'a, str>` : l'emprunt subsiste quand il n'y a pas de détourage, et `Quoi`/`Trace`
perdent `Copy` en gardant `Clone`. Les six appelants de `trace` passent leur `Trace` par
valeur une seule fois — la compilation le confirmera à la tâche 6.

**Le détourage ne doit pas écraser un alpha existant.** Un PNG déjà détouré par l'auteur
entre avec sa couche alpha. L'alpha calculé la **multiplie** au lieu de la remplacer,
sans quoi un fond déjà transparent redeviendrait opaque là où il est clair.

---

## Structure des fichiers

| Fichier | Responsabilité |
|---|---|
| `app/src-tauri/src/detourage.rs` | **Créer.** Les deux seuils, leur estimation, leur application. Aucun disque, aucun état. |
| `app/src-tauri/src/envoi.rs` | Le champ `detourage` sur `Envoi`. |
| `app/src-tauri/src/projet.rs` | `poser_image_envoi` estime les seuils à la pose. |
| `app/src-tauri/src/interieur.rs` | `Quoi::Image` porte un `Cow`. |
| `app/src-tauri/src/package.rs` | `trace` détoure avant d'écrire. |
| `app/src-tauri/src/providers.rs` | La teinte de chaque `Papier`. |
| `app/src/index.html`, `envois.js`, `styles.css` | Les deux curseurs, la teinte du canevas. |

---

# Lot 1 — le module, seul et pur

Rien de visible. À la fin du lot, `detourage.rs` sait détourer, et rien ne l'appelle.

### Tâche 1 : la dépendance et la rampe d'alpha

**Fichiers :**
- Modifier : `app/src-tauri/Cargo.toml`
- Créer : `app/src-tauri/src/detourage.rs`
- Modifier : `app/src-tauri/src/lib.rs` (déclarer le module)

- [ ] **Étape 1 — poser la dépendance**

Dans `[dependencies]` de `app/src-tauri/Cargo.toml`, à la suite de `base64` :

```toml
# Décoder une photo d'envoi pour en séparer l'encre du papier. `image.rs` ne lit que
# des en-têtes ; un décodeur JPEG ne se réécrit pas. Restreinte aux deux formats que
# l'application accepte, comme `zip` l'est à `deflate`.
image = { version = "0.25", default-features = false, features = ["jpeg", "png"] }
```

- [ ] **Étape 2 — déclarer le module**

Dans `app/src-tauri/src/lib.rs`, à côté de `mod image;` (ordre alphabétique) :

```rust
mod detourage;
```

- [ ] **Étape 3 — écrire le test qui échoue**

Créer `app/src-tauri/src/detourage.rs` avec **le seul bloc de tests** pour l'instant :

```rust
//! Séparer l'encre du papier dans la photo d'un envoi.
//!
//! Un envoi écrit à la main est presque toujours la photo d'un mot tracé sur une
//! feuille. Le papier photographié n'est pas du blanc pur — 230 à 245, teinté, avec le
//! dégradé de l'éclairage — et ce blanc-là s'encre. Sur un papier crème, il paraît.
//!
//! Aucun disque, aucun état : des octets entrent, des octets sortent. C'est la manière
//! d'`image.rs`, et c'est ce qui rend ce module vérifiable sur des images fabriquées en
//! mémoire.

#[cfg(test)]
mod tests {
    use super::*;

    /// Une image unie de la couleur donnée, en PNG.
    fn uni(r: u8, g: u8, b: u8) -> Vec<u8> {
        let img = image::RgbaImage::from_pixel(4, 4, image::Rgba([r, g, b, 255]));
        let mut out = Vec::new();
        image::DynamicImage::ImageRgba8(img)
            .write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png)
            .unwrap();
        out
    }

    /// L'alpha du premier pixel d'un PNG.
    fn alpha(octets: &[u8]) -> u8 {
        image::load_from_memory(octets).unwrap().to_rgba8().get_pixel(0, 0)[3]
    }

    const SEUILS: Detourage = Detourage { papier: 240.0, encre: 40.0 };

    /// Les deux bouts de la rampe. Sans eux, un détourage qui ne ferait rien passerait.
    #[test]
    fn le_papier_disparait_et_l_encre_reste() {
        assert_eq!(alpha(&applique(&uni(250, 250, 250), &SEUILS).unwrap()), 0);
        assert_eq!(alpha(&applique(&uni(10, 10, 10), &SEUILS).unwrap()), 255);
    }

    /// La rampe elle-même : un seuil binaire ferait tomber ce test, et c'est tout son
    /// objet — il hacherait le trait en escalier là où la photo l'a lissé.
    #[test]
    fn un_pixel_a_mi_chemin_sort_a_mi_alpha() {
        // Luminance 140, à mi-chemin de 240 et 40 : un gris neutre suffit, la luminance
        // d'un gris vaut sa composante.
        let a = alpha(&applique(&uni(140, 140, 140), &SEUILS).unwrap());
        assert!((a as i32 - 128).abs() <= 2, "alpha {a}, attendu 128 ± 2");
    }
}
```

- [ ] **Étape 4 — voir le test échouer**

Depuis `app/src-tauri/` : `cargo test detourage`
Attendu : **échec de compilation**, `cannot find function applique` / `cannot find type Detourage`.

- [ ] **Étape 5 — écrire le minimum**

En tête de `detourage.rs`, avant `mod tests` :

```rust
use serde::{Deserialize, Serialize};

/// Les deux seuils qui séparent l'encre du papier, en luminance 0-255.
///
/// Deux et non un : sans point d'encre, un trait bien noir ressort délavé — mesuré
/// (48, 51, 123) contre (28, 32, 105) pour un stylo bleu. Voir la spec, § « Ce qui est
/// vérifié ».
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Detourage {
    /// Au-dessus, c'est le papier : alpha 0.
    pub papier: f64,
    /// En dessous, c'est l'encre pleine : alpha 1.
    pub encre: f64,
}

/// La luminance perçue d'un pixel, 0-255.
fn luminance(r: u8, g: u8, b: u8) -> f64 {
    0.2126 * r as f64 + 0.7152 * g as f64 + 0.0722 * b as f64
}

/// La photo, son fond rendu transparent, en PNG.
///
/// **La couleur du pixel n'est pas touchée.** Démultiplier l'encre pour la « retrouver »
/// derrière le papier a été mesuré : le trait en sort moins fidèle qu'avec un point noir
/// bien posé, pour un calcul plus compliqué. Seul l'alpha se calcule.
///
/// L'alpha calculé **multiplie** celui d'entrée : un PNG déjà détouré par l'auteur ne
/// doit pas redevenir opaque là où son fond est clair.
pub fn applique(octets: &[u8], d: &Detourage) -> Result<Vec<u8>, String> {
    let mut img = image::load_from_memory(octets)
        .map_err(|e| format!("image illisible : {e}"))?
        .to_rgba8();
    let ecart = d.papier - d.encre;
    for p in img.pixels_mut() {
        let [r, g, b, a] = p.0;
        let f = ((d.papier - luminance(r, g, b)) / ecart).clamp(0.0, 1.0);
        p.0 = [r, g, b, (f * a as f64).round() as u8];
    }
    let mut out = Vec::new();
    image::DynamicImage::ImageRgba8(img)
        .write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png)
        .map_err(|e| format!("encodage PNG impossible : {e}"))?;
    Ok(out)
}
```

- [ ] **Étape 6 — voir les tests passer**

`cargo test detourage`
Attendu : `le_papier_disparait_et_l_encre_reste` et `un_pixel_a_mi_chemin_sort_a_mi_alpha` en `ok`.

- [ ] **Étape 7 — commit**

```bash
git add app/src-tauri/Cargo.toml app/src-tauri/Cargo.lock app/src-tauri/src/lib.rs app/src-tauri/src/detourage.rs
git commit -m "L'encre se sépare du papier, sur une rampe et non sur un seuil"
```

### Tâche 2 : le réglage impossible se refuse

**Fichiers :** Modifier `app/src-tauri/src/detourage.rs`

- [ ] **Étape 1 — écrire le test qui échoue**

Dans `mod tests` :

```rust
/// `papier <= encre` divise par zéro ou inverse la rampe : l'image sortirait
/// entièrement opaque sans qu'on sache pourquoi. On refuse en nommant les deux
/// valeurs — c'est un réglage que l'écran laisse atteindre.
#[test]
fn un_papier_plus_sombre_que_l_encre_se_refuse() {
    let d = Detourage { papier: 40.0, encre: 240.0 };
    let err = applique(&uni(200, 200, 200), &d).unwrap_err();
    assert!(err.contains("240"), "le message ne dit pas l'encre : {err}");
    assert!(err.contains("40"), "le message ne dit pas le papier : {err}");
}
```

- [ ] **Étape 2 — voir le test échouer**

`cargo test detourage::tests::un_papier_plus_sombre`
Attendu : **panique** sur `unwrap_err` — la fonction rend `Ok`.

- [ ] **Étape 3 — implémenter**

En tête de `applique`, avant le décodage :

```rust
    if d.papier <= d.encre {
        return Err(format!(
            "détourage impossible : le papier ({:.0}) doit être plus clair que \
             l'encre ({:.0}).",
            d.papier, d.encre
        ));
    }
```

- [ ] **Étape 4 — voir passer**

`cargo test detourage`  → tous en `ok`.

- [ ] **Étape 5 — commit**

```bash
git add app/src-tauri/src/detourage.rs
git commit -m "Un papier plus sombre que l'encre se refuse, en nommant les deux"
```

### Tâche 3 : l'estimation des seuils

**Fichiers :** Modifier `app/src-tauri/src/detourage.rs`

- [ ] **Étape 1 — écrire le test qui échoue**

Dans `mod tests` :

```rust
/// Une image faite d'une part d'encre sur du papier, en PNG.
fn encre_sur_papier(part: f64) -> Vec<u8> {
    let mut img = image::RgbaImage::from_pixel(20, 20, image::Rgba([242, 240, 235, 255]));
    let lignes = (20.0 * part).round() as u32;
    for y in 0..lignes {
        for x in 0..20 {
            img.put_pixel(x, y, image::Rgba([30, 36, 118, 255]));
        }
    }
    let mut out = Vec::new();
    image::DynamicImage::ImageRgba8(img)
        .write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png)
        .unwrap();
    out
}

/// L'estimation vise le papier haut et l'encre bas, et elle tient quand le mot est
/// court : la part encrée d'une image va de moins de 1 % pour une signature à plus de
/// 10 % pour un paragraphe. C'est ce qui a fait écarter un percentile unique — sur la
/// photo d'essai de la spec, le 5e percentile tombait déjà dans le papier.
#[test]
fn les_seuils_s_estiment_sur_une_signature_comme_sur_un_paragraphe() {
    for part in [0.02, 0.30] {
        let d = estime(&encre_sur_papier(part)).unwrap();
        assert!(d.papier > 200.0, "papier {} pour une part de {part}", d.papier);
        assert!(d.encre < 100.0, "encre {} pour une part de {part}", d.encre);
        assert!(d.papier > d.encre);
    }
}
```

- [ ] **Étape 2 — voir échouer**

`cargo test detourage::tests::les_seuils`
Attendu : **échec de compilation**, `cannot find function estime`.

- [ ] **Étape 3 — implémenter**

Après `applique` :

```rust
/// Les seuils que cette image-là appelle.
///
/// Le papier au 95e percentile de luminance, l'encre au 0,5e. Deux percentiles très
/// écartés parce qu'aucun n'est fiable seul : la part encrée d'une image varie du tout
/// au tout, et un percentile trop haut du côté de l'encre tombe dans le papier dès que
/// le mot est court. C'est une estimation de départ, que l'écran laisse reprendre.
pub fn estime(octets: &[u8]) -> Result<Detourage, String> {
    let img = image::load_from_memory(octets)
        .map_err(|e| format!("image illisible : {e}"))?
        .to_rgba8();
    let mut l: Vec<f64> = img.pixels().map(|p| luminance(p[0], p[1], p[2])).collect();
    if l.is_empty() {
        return Err("image vide : rien à détourer.".into());
    }
    l.sort_by(|a, b| a.total_cmp(b));
    let au = |q: f64| l[((q * (l.len() - 1) as f64).round() as usize).min(l.len() - 1)];
    Ok(Detourage { papier: au(0.95), encre: au(0.005) })
}
```

- [ ] **Étape 4 — voir passer**

`cargo test detourage` → tous en `ok`.

- [ ] **Étape 5 — vérifier la chaîne**

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
```

Attendu : rien. (Ne jamais mettre `clippy` dans un pipe : `| tail` masque l'échec.)

- [ ] **Étape 6 — commit**

```bash
git add app/src-tauri/src/detourage.rs
git commit -m "Les seuils s'estiment sur l'image, et tiennent d'une signature à un paragraphe"
```

---

# Lot 2 — le réglage entre dans le livre et s'applique

À la fin du lot, une photo posée se détoure au tirage comme à l'aperçu.

### Tâche 4 : le champ sur l'envoi

**Fichiers :** Modifier `app/src-tauri/src/envoi.rs`

- [ ] **Étape 1 — écrire le test qui échoue**

Dans le `mod tests` d'`envoi.rs` :

```rust
/// Un projet d'avant ce chantier n'a pas de détourage, et n'en reçoit pas d'office :
/// on ne change pas le tirage que quelqu'un a déjà relu. Le champ est un `Option`
/// pour cette seule raison, et `VERSION` ne bouge donc pas.
#[test]
fn un_envoi_ancien_n_a_pas_de_detourage() {
    let e: Envoi = toml::from_str("dedicataire = \"Léa\"\ncontenu = \"\"\n").unwrap();
    assert_eq!(e.detourage, None);
}
```

- [ ] **Étape 2 — voir échouer**

`cargo test envoi::tests::un_envoi_ancien`
Attendu : **échec de compilation**, `no field detourage`.

- [ ] **Étape 3 — implémenter**

Dans `struct Envoi`, après le champ `image` :

```rust
    /// Les seuils qui séparent l'encre du papier sur la photo de cet envoi.
    ///
    /// Sur l'envoi et non sur le livre : chaque photo a son éclairage. `None` — les
    /// projets d'avant ce chantier — vaut « aucun détourage », et l'image se compose
    /// telle quelle. Il survit à un passage en police : le perdre obligerait à régler à
    /// nouveau après un aller-retour, et ce n'est pas ce que changer de main veut dire.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detourage: Option<crate::detourage::Detourage>,
```

- [ ] **Étape 4 — voir passer**

`cargo test envoi` → tous en `ok`. Les autres constructions d'`Envoi` du dépôt
utilisent `..Default::default()` ; si une construction littérale échoue à compiler,
lui ajouter `detourage: None`.

- [ ] **Étape 5 — commit**

```bash
git add app/src-tauri/src/envoi.rs
git commit -m "L'envoi porte les seuils de sa photo, et un projet ancien n'en reçoit pas"
```

### Tâche 5 : la pose estime les seuils

**Fichiers :** Modifier `app/src-tauri/src/projet.rs` (`poser_image_envoi`, ligne ~631)

- [ ] **Étape 1 — écrire le test qui échoue**

Dans le `mod tests` de `projet.rs`, à côté des tests de `poser_image_envoi` :

```rust
/// Une photo posée après ce chantier naît détourée : c'est le cas d'usage, et
/// demander un geste de plus pour l'obtenir reviendrait à livrer le défaut par
/// défaut.
#[test]
fn une_photo_posee_nait_detouree() {
    let mut p = avec_envois(&["Léa"]);
    p.poser_image_envoi(0, photo()).unwrap();
    let d = p.meta.envois.liste[0].detourage.expect("aucun détourage posé");
    assert!(d.papier > d.encre, "seuils incohérents : {d:?}");
}
```

- [ ] **Étape 2 — voir échouer**

`cargo test projet::tests::une_photo_posee_nait_detouree`
Attendu : **panique** sur `expect("aucun détourage posé")`.

**Attention : `png(300)`, l'aide de test existante de `projet.rs`, n'est qu'un en-tête**
— signature plus IHDR, sans IDAT ni IEND. La crate `image` ne la décode pas, `estime`
rendrait `Err`, et le test échouerait pour la mauvaise raison. Écrire une aide voisine
dans le même `mod tests` et l'employer ici :

```rust
    /// Une image décodable, contrairement à `png()` qui n'est qu'un en-tête : la
    /// crate `image` en lit les pixels, et l'estimation des seuils en a besoin.
    fn photo() -> Vec<u8> {
        let mut img = image::RgbaImage::from_pixel(16, 16, image::Rgba([243, 241, 236, 255]));
        for x in 0..16 {
            img.put_pixel(x, 8, image::Rgba([32, 38, 120, 255]));
        }
        let mut out = Vec::new();
        image::DynamicImage::ImageRgba8(img)
            .write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png)
            .unwrap();
        out
    }
```

Le test de l'étape 1 emploie donc `p.poser_image_envoi(0, photo())`.

- [ ] **Étape 3 — implémenter**

Dans `poser_image_envoi`, après `self.meta.envois.liste[index].image = Some(nom.clone());` :

```rust
        // Estimé sur l'image reçue, et non posé à des valeurs de maison : deux photos
        // n'ont ni le même papier ni le même éclairage. Une image que le décodeur ne
        // sait pas lire n'empêche pas de la poser — Typst la lira peut-être — et elle
        // se compose alors sans détourage.
        self.meta.envois.liste[index].detourage = crate::detourage::estime(&octets).ok();
```

- [ ] **Étape 4 — voir passer**

`cargo test projet` → tous en `ok`.

- [ ] **Étape 5 — commit**

```bash
git add app/src-tauri/src/projet.rs
git commit -m "Une photo posée naît détourée, sur des seuils relevés sur elle"
```

### Tâche 6 : `trace` détoure avant d'écrire

**Fichiers :**
- Modifier : `app/src-tauri/src/interieur.rs` (`enum Quoi`, ~ligne 135)
- Modifier : `app/src-tauri/src/package.rs` (`trace`, ~ligne 190)

- [ ] **Étape 1 — écrire le test qui échoue**

Dans le `mod tests` de `package.rs`, à côté des tests de `trace` existants :

```rust
/// Ce que `trace` écrit sur le disque est détouré, et porte un nom en `.png` : Typst
/// reconnaît le format d'une image **à son extension**, et un PNG rangé sous `.jpg`
/// ne se composerait pas — l'erreur tomberait sur l'exemplaire d'une personne.
#[test]
fn une_image_detouree_s_ecrit_en_png() {
    let dir = tempfile::tempdir().unwrap();
    let mut p = avec_envois(&["Léa"]);
    // Un JPEG uni clair : tout est papier, donc tout doit sortir transparent.
    let mut jpeg = Vec::new();
    image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(
        8, 8, image::Rgb([245, 243, 238]),
    ))
    .write_to(&mut std::io::Cursor::new(&mut jpeg), image::ImageFormat::Jpeg)
    .unwrap();
    p.poser_image_envoi(0, jpeg).unwrap();

    let t = trace(&p, &p.meta.envois.liste[0], dir.path()).unwrap();
    let interieur::Quoi::Image { fichier } = t.quoi else {
        panic!("la trace n'est pas une image");
    };
    assert!(fichier.ends_with(".png"), "écrit sous « {fichier} »");
    let ecrit = std::fs::read(dir.path().join(&*fichier)).unwrap();
    let px = image::load_from_memory(&ecrit).unwrap().to_rgba8();
    assert_eq!(px.get_pixel(0, 0)[3], 0, "le papier n'a pas été rendu transparent");
}
```

- [ ] **Étape 2 — voir échouer**

`cargo test package::tests::une_image_detouree`
Attendu : échec sur `ends_with(".png")` — le fichier s'appelle encore `Léa.jpg`.

- [ ] **Étape 3 — faire porter son nom à `Quoi::Image`**

Dans `interieur.rs`, remplacer la variante et retirer `Copy` des deux types :

```rust
#[derive(Debug, Clone)]
pub enum Quoi<'a> {
    /// Un texte, composé dans la main de cet envoi.
    Texte { police: &'a str, texte: &'a str },
    /// Une image, déjà écrite à côté de la source, désignée par son seul nom.
    ///
    /// `Cow` parce que le nom écrit n'est pas toujours celui de l'archive : une photo
    /// détourée sort en PNG et change d'extension. L'emprunt subsiste quand rien n'est
    /// détouré, et c'est le cas courant.
    Image { fichier: std::borrow::Cow<'a, str> },
}

#[derive(Debug, Clone)]
pub struct Trace<'a> {
    pub quoi: Quoi<'a>,
    pub place: &'a crate::envoi::Place,
}
```

Puis corriger les constructions existantes : `Quoi::Image { fichier }` devient
`Quoi::Image { fichier: fichier.into() }`, et les lectures `format!("…{fichier}…")`
fonctionnent telles quelles (`Cow` implémente `Display`). Un site est dans les tests
d'`interieur.rs` — `Quoi::Image { fichier: "mot.png" }`, dans
`un_envoi_ne_cree_aucune_page_ou_qu_il_se_pose` — et il doit devenir
`fichier: "mot.png".into()`. Le compilateur nommera les autres.

- [ ] **Étape 4 — détourer dans `trace`**

Dans `package.rs`, remplacer le corps de la branche image :

```rust
        crate::envoi::Main::Image | crate::envoi::Main::Diffusion => {
            let fichier = e
                .image
                .as_deref()
                .ok_or_else(|| format!("{qui} n'a pas d'image : en choisir une."))?;
            let octets = projet.images_envois.get(fichier).ok_or_else(|| {
                format!("{qui} : l'image « {fichier} » ne figure pas dans le projet.")
            })?;
            // Détouré ici et nulle part ailleurs : `trace` est le seul chemin par où
            // passent la composition d'un package et le rendu de l'objet du canevas.
            // L'écran ne peut donc pas montrer autre chose que ce qui s'imprime.
            let (nom, octets) = match &e.detourage {
                Some(d) => {
                    let png = crate::detourage::applique(octets, d)
                        .map_err(|err| format!("{qui} : {err}"))?;
                    let tige = fichier.rsplit_once('.').map_or(fichier, |(t, _)| t);
                    (format!("{tige}.png"), std::borrow::Cow::Owned(png))
                }
                None => (fichier.to_string(), std::borrow::Cow::Borrowed(octets.as_slice())),
            };
            std::fs::write(dossier.join(&nom), &*octets)
                .map_err(|err| format!("{nom} : écriture impossible : {err}"))?;
            interieur::Quoi::Image { fichier: nom.into() }
        }
```

- [ ] **Étape 5 — voir passer, et la chaîne entière**

```bash
cargo test
cargo fmt --check
cargo clippy --all-targets -- -D warnings
```

Attendu : tout vert. Si un appelant de `trace` réclame `Copy`, c'est ici qu'il se
signale — le corriger en clonant explicitement plutôt qu'en rétablissant `Copy`.

- [ ] **Étape 6 — commit**

```bash
git add app/src-tauri/src/interieur.rs app/src-tauri/src/package.rs
git commit -m "L'image d'un envoi se détoure sur le chemin de Typst, et y va en PNG"
```

### Tâche 7 : les trois tests que la spec réclame, et le témoin

Ces trois-là ne peuvent pas être écrits avant leur implémentation — elle est faite aux
tâches 1 à 6. C'est le second chemin que le dépôt admet : **mutation ciblée**. Chaque
étape donne la mutation qui doit faire tomber le test, et le test n'est acquis que
vu rouge une fois.

**Fichiers :**
- Modifier : `app/src-tauri/src/detourage.rs` (tests)
- Modifier : `app/src-tauri/src/package.rs` (tests)

- [ ] **Étape 1 — la couleur du pixel est conservée**

Dans le `mod tests` de `detourage.rs` :

```rust
/// La couleur ne se retouche pas : seul l'alpha se calcule. C'est la décision du
/// § 2 de la spec — la démultiplication a été mesurée moins fidèle qu'un point noir
/// bien posé — et rien d'autre ne la protège.
#[test]
fn la_couleur_de_l_encre_ne_bouge_pas() {
    let px = image::load_from_memory(&applique(&uni(30, 36, 118), &SEUILS).unwrap())
        .unwrap()
        .to_rgba8();
    let [r, g, b, a] = px.get_pixel(0, 0).0;
    assert_eq!((r, g, b), (30, 36, 118), "la couleur a été retouchée");
    assert!(a > 200, "un bleu franc devrait être quasi opaque, alpha {a}");
}
```

**Mutation qui doit le faire tomber :** dans `applique`, remplacer
`p.0 = [r, g, b, …]` par `p.0 = [0, 0, 0, …]`. Lancer `cargo test detourage`, voir
`la_couleur_de_l_encre_ne_bouge_pas` échouer, **rétablir**.

- [ ] **Étape 2 — le JPEG entre comme le PNG**

```rust
/// Les photos d'appareil sont des JPEG : les refuser viderait le chantier de son
/// objet. Le format se relève sur le contenu, comme partout ailleurs dans
/// l'application.
#[test]
fn un_jpeg_se_detoure_comme_un_png() {
    let mut jpeg = Vec::new();
    image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(4, 4, image::Rgb([248, 246, 241])))
        .write_to(&mut std::io::Cursor::new(&mut jpeg), image::ImageFormat::Jpeg)
        .unwrap();
    assert_eq!(alpha(&applique(&jpeg, &SEUILS).unwrap()), 0);
}
```

**Mutation :** en tête d'`applique`, ajouter
`if crate::image::extension(octets) != Some("png") { return Err("non".into()); }`.
Voir le test échouer, **rétablir**.

- [ ] **Étape 3 — sans détourage, l'original passe tel quel**

Dans le `mod tests` de `package.rs`, à côté du test de la tâche 6 :

```rust
/// Un projet d'avant ce chantier compose exactement ce qu'il composait : mêmes
/// octets, même nom. C'est l'autre moitié de la décision « un projet ancien garde
/// son rendu » — la première moitié est dans `envoi.rs`, et elle ne dit que le
/// modèle.
#[test]
fn sans_detourage_l_image_part_telle_quelle() {
    let dir = tempfile::tempdir().unwrap();
    let mut p = avec_envois(&["Léa"]);
    let mut jpeg = Vec::new();
    image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(8, 8, image::Rgb([245, 243, 238])))
        .write_to(&mut std::io::Cursor::new(&mut jpeg), image::ImageFormat::Jpeg)
        .unwrap();
    p.poser_image_envoi(0, jpeg.clone()).unwrap();
    // Le projet ancien : la photo est là, les seuils n'y sont pas.
    p.meta.envois.liste[0].detourage = None;

    let t = trace(&p, &p.meta.envois.liste[0], dir.path()).unwrap();
    let interieur::Quoi::Image { fichier } = t.quoi else {
        panic!("la trace n'est pas une image");
    };
    assert!(fichier.ends_with(".jpg"), "le nom a changé : « {fichier} »");
    assert_eq!(std::fs::read(dir.path().join(&*fichier)).unwrap(), jpeg,
        "les octets ont été retouchés sans qu'on l'ait demandé");
}
```

**Mutation :** dans `trace`, remplacer la branche `None` par un détourage à des seuils
de maison (`Detourage { papier: 240.0, encre: 40.0 }`). Voir le test échouer,
**rétablir**.

- [ ] **Étape 4 — voir les trois passer**

```bash
cargo test
```

Attendu : tout vert, y compris les tests marqués `#[ignore]` laissés de côté.

- [ ] **Étape 5 — l'invariant qui tient la chaîne**

`interieur.rs` porte déjà `un_envoi_ne_cree_aucune_page_ou_qu_il_se_pose`, qui compose
pour de vrai sur quatre pages, variante image comprise. Il est marqué `#[ignore]` :

```bash
cargo test -- --ignored
```

Attendu : `ok`. C'est lui qui dit que le dos d'un livre dédicacé est celui du même
livre nu. S'il tombe, le nom du fichier écrit à la tâche 6 ne correspond plus à celui
que la source nomme.

- [ ] **Étape 6 — le témoin**

```bash
cargo run --example temoin
```

Un fichier de `src-tauri/` a changé : la règle du dépôt demande ce relevé. Comparer le
compte de pages affiché au précédent sur le même manuscrit — il doit être identique.

- [ ] **Étape 7 — la chaîne, puis commit**

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
git add app/src-tauri/src/detourage.rs app/src-tauri/src/package.rs
git commit -m "La couleur ne bouge pas, le JPEG entre, et sans seuils rien ne change"
```

---

# Lot 3 — l'écran montre le papier et laisse régler

### Tâche 8 : la teinte du papier

**Fichiers :** Modifier `app/src-tauri/src/providers.rs`

- [ ] **Étape 1 — écrire le test qui échoue**

Dans le `mod tests` de `providers.rs` :

```rust
/// Chaque papier dit sa couleur, en notation CSS : c'est le front qui la peint, et une
/// conversion en chemin serait une occasion de se tromper. La valeur est une
/// convention d'Ozalid, pas une mesure — aucun prestataire ne publie la teinte de son
/// crème.
#[test]
fn chaque_papier_annonce_sa_teinte() {
    for p in PROVIDERS {
        for pa in p.papiers {
            assert!(
                pa.teinte.len() == 7 && pa.teinte.starts_with('#'),
                "{} / {} : teinte « {} » illisible en CSS",
                p.cle, pa.cle, pa.teinte
            );
        }
    }
}
```

- [ ] **Étape 2 — voir échouer**

`cargo test providers::tests::chaque_papier`
Attendu : **échec de compilation**, `no field teinte`.

- [ ] **Étape 3 — implémenter**

Dans `struct Papier` :

```rust
    /// La couleur du papier, en notation CSS, telle que le canevas la peint.
    ///
    /// Convention d'Ozalid et non mesure : aucun prestataire ne publie la teinte de son
    /// crème. Elle ne sert qu'à l'écran — le PDF n'a pas de fond, et lui en donner un
    /// ferait imprimer un aplat sur toutes les pages.
    pub teinte: &'static str,
```

Puis renseigner chaque `Papier` du fichier : `teinte: "#f7f0e0"` pour les crème
(`creme-90` de BoD, `creme` de KDP), `teinte: "#ffffff"` pour les blancs et pour
`standard` de Lulu et `mesure`.

- [ ] **Étape 4 — voir passer**

`cargo test providers` → `ok`.

- [ ] **Étape 5 — commit**

```bash
git add app/src-tauri/src/providers.rs
git commit -m "Chaque papier dit sa couleur, pour l'écran et non pour le fichier"
```

### Tâche 9 : les deux curseurs

**Fichiers :**
- Modifier : `app/src/index.html` (~ligne 398, le bloc `champImage`)
- Modifier : `app/src/envois.js`
- Modifier : `app/tests/coquille.test.js`

- [ ] **Étape 1 — écrire le test qui échoue**

Dans `app/tests/coquille.test.js`, à la suite des tests de l'étape Envois :

```js
/**
 * Les deux seuils ne paraissent que sous une main en image : c'est la règle de la
 * bande de réglages — ce que la main ne réclame pas ne paraît pas. Un curseur grisé
 * sous une main en police donnerait à croire qu'on peut y toucher.
 */
test('les seuils de détourage ne paraissent que sous une image', async () => {
  const a = atelier({
    sur: {
      envois: {
        gabarit: '',
        liste: [{ dedicataire: 'Léa', main: { mode: 'image' }, place: PLACE_DEFAUT,
          contenu: '', image: 'Léa.jpg', detourage: { papier: 240, encre: 40 } }],
      },
    },
  });
  const { els } = await charge({ invoke: a.invoke });
  await els.get('btNouveau').declenche('click');
  await allerAuxEnvois(els);

  assert.equal(els.get('champDetourage').hidden, false,
    'les seuils sont cachés sous une main en image');

  els.get('inMain').value = 'police';
  await els.get('inMain').declenche('change');

  assert.equal(els.get('champDetourage').hidden, true,
    'les seuils restent sous une main en police');
});
```

- [ ] **Étape 2 — voir échouer**

Depuis `app/` : `node --test tests/coquille.test.js`
Attendu : échec — `els.get('champDetourage')` est indéfini.

- [ ] **Étape 3 — le balisage**

Dans `app/src/index.html`, juste après le bloc `<div id="champImage" …>` :

```html
      <!-- Les deux seuils, sous une main en image seulement. Le papier est le plus
           utile des deux : c'est lui qui décide de ce qui reste du fond. -->
      <div id="champDetourage" hidden>
        <label><span>Papier</span>
          <input type="range" id="inPapier" min="0" max="255" step="1">
          <span class="val" id="vPapier"></span></label>
        <label><span>Encre</span>
          <input type="range" id="inEncre" min="0" max="255" step="1">
          <span class="val" id="vEncre"></span></label>
      </div>
```

- [ ] **Étape 4 — le câblage**

Dans `envois.js`, dans `afficherEnvois()`, à côté de la ligne qui pose
`$('champImage').hidden` :

```js
  $('champDetourage').hidden = !e || main() !== 'image' || !e.detourage;
  if (e?.detourage) {
    const p = Math.round(e.detourage.papier);
    $('inPapier').value = p;
    $('vPapier').textContent = String(p);
    const n = Math.round(e.detourage.encre);
    $('inEncre').value = n;
    $('vEncre').textContent = String(n);
  }
```

Et au câblage du démarrage, à côté de celui d'`inTaille` :

```js
  // Au relâchement et non à chaque pixel parcouru : un `input` par pixel rappellerait
  // Typst pour chaque valeur traversée. C'est la règle des gestes du canevas, qui ne
  // commettent leur placement qu'au dépôt.
  for (const [id, champ] of [['inPapier', 'papier'], ['inEncre', 'encre']]) {
    $(id).addEventListener('input', () => {
      $(id === 'inPapier' ? 'vPapier' : 'vEncre').textContent = $(id).value;
    });
    $(id).addEventListener('change', async () => {
      const e = envoi();
      if (!e?.detourage) return;
      await reglerEnvoi({ detourage: { ...e.detourage, [champ]: Number($(id).value) } });
      await majObjet();
    });
  }
```

- [ ] **Étape 5 — voir passer**

`node --test tests/coquille.test.js` → tous en `ok`.

- [ ] **Étape 6 — commit**

```bash
git add app/src/index.html app/src/envois.js app/tests/coquille.test.js
git commit -m "Les deux seuils se règlent, et ne paraissent que sous une image"
```

### Tâche 10 : le canevas prend la couleur du papier

**Fichiers :**
- Modifier : `app/src/envois.js`, `app/src/styles.css`
- Modifier : `app/tests/coquille.test.js`

- [ ] **Étape 1 — écrire le test qui échoue**

```js
/**
 * Sans fond teinté, le réglage se ferait à l'aveugle : un fond résiduel gris pâle ne
 * se distingue pas du blanc de l'écran, et c'est précisément ce qu'on cherche à voir.
 * La teinte suit le papier du destinataire visé — c'est lui qu'on tire.
 */
test('le canevas prend la couleur du papier visé', async () => {
  const a = atelier({
    sur: {
      envois: {
        gabarit: '',
        liste: [{ dedicataire: 'Léa', main: { mode: 'image' }, place: PLACE_DEFAUT,
          contenu: '', image: 'Léa.jpg', detourage: { papier: 240, encre: 40 } }],
      },
    },
  });
  const { els } = await charge({ invoke: a.invoke });
  await els.get('btNouveau').declenche('click');
  await allerAuxEnvois(els);

  assert.match(els.get('canevas').style.getPropertyValue('--papier'), /^#[0-9a-f]{6}$/i,
    'le canevas ne porte pas la teinte du papier');
});
```

Le faux Rust de `coquille.test.js` doit servir des papiers portant une `teinte` : ajouter
le champ aux papiers de son `providers_liste`, sans quoi le test vérifierait un contrat
que le faux ne tient pas.

- [ ] **Étape 2 — voir échouer**

`node --test tests/coquille.test.js`
Attendu : échec — la variable `--papier` est vide.

- [ ] **Étape 3 — implémenter le JS**

Dans `envois.js`, dans `majPage()` — la fonction qui pose la page de fond — juste
avant la demande de la page à `envoi_page` :

```js
  // La teinte du papier que le destinataire visé imprimera. C'est un fait d'écran :
  // le PDF n'a pas de fond, et lui en donner un ferait imprimer un aplat.
  const d = projet.livraison.destinataires.find((x) => x.provider === projet.livraison.courant);
  const pr = providers.find((p) => p.cle === d?.provider);
  const pa = pr?.papiers.find((x) => x.cle === d?.papier) ?? pr?.papiers[0];
  $('canevas').style.setProperty('--papier', pa?.teinte ?? '#ffffff');
```

- [ ] **Étape 4 — implémenter le CSS**

Dans `styles.css`, au bloc `.envois .canevas` :

```css
/* Le papier sous la page : c'est lui qui rend visible un fond mal détouré. La page et
   l'objet se posent dessus en `multiply` — l'encre multiplie le papier, c'est la
   physique de l'impression, et c'est ce qui fait qu'un blanc pur disparaît quand un
   gris de photo se voit. */
.envois .canevas { background: var(--papier, #ffffff); }
.envois .canevas .fond,
.envois .objet img { mix-blend-mode: multiply; }
```

Attention : la règle `.envois .objet img` existe déjà (`display: block; width: 100%;
pointer-events: none;`). **Ne pas la dupliquer** — y ajouter `mix-blend-mode` plutôt
que d'en écrire une seconde, et surtout ne pas toucher au `pointer-events: none`, que
`contrats.test.js` garde.

- [ ] **Étape 5 — voir passer**

`node --test tests/*.test.js` → tous en `ok`, la garde des contrats comprise.

- [ ] **Étape 6 — vérifier dans la fenêtre**

Le front est embarqué à la compilation :

```bash
touch app/src-tauri/src/lib.rs
```

puis lancer l'application. Poser une photo de mot sur un livre dont le destinataire
visé est en papier crème, et **regarder** : le canevas doit être crème, et le fond de
la photo ne doit pas y faire un rectangle. Bouger les deux curseurs et voir le fond
apparaître et disparaître. C'est la seule vérification que les tests ne font pas.

- [ ] **Étape 7 — commit**

```bash
git add app/src/envois.js app/src/styles.css app/tests/coquille.test.js
git commit -m "Le canevas prend la couleur du papier, et l'encre s'y multiplie"
```

---

## Tâche 11 : le relevé sur une vraie photo

La spec porte une réserve : la photo d'essai est synthétique. C'est le dernier pas, et
il peut déplacer les valeurs par défaut de la tâche 3.

- [ ] Photographier un mot manuscrit avec un téléphone, en lumière ordinaire.
- [ ] Le poser comme envoi, relever les seuils estimés, composer, ouvrir le PDF.
- [ ] Vérifier qu'aucun rectangle ne paraît sur le crème, et que le trait n'est pas mangé.
- [ ] Si les percentiles de la tâche 3 tombent à côté, les corriger — et **mettre à jour
      la spec** en disant sur quoi le relevé a été fait.
- [ ] Consigner le relevé dans `app/README.md`, à la section des envois.

---

## Ce que ce plan ne fait pas

- **Aucune correction de l'éclairage inégal.** Les seuils sont globaux ; 1,6 % de fond
  subsistait au meilleur réglage de l'essai. La normalisation locale attend un défaut
  vu sur une photo réelle.
- **Aucun détourage des images de couverture.** Une illustration n'est pas de l'encre
  sur du papier.
- **Aucune retouche** : ni contraste, ni recadrage, ni rotation, ni poussière.
