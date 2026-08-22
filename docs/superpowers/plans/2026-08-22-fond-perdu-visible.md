# Le fond perdu visible sur la planche — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Faire apparaître, sur l'aperçu de la face Planche, la bande de fond perdu que le massicot emporte — voilée et bordée d'un pointillé — sans qu'un seul repère n'entre dans le PDF remis au prestataire.

**Architecture:** Le Rust mesure (`Gabarit::part_fond_perdu`) et le dit au front avec l'image (`Apercu { image, coupe }`) ; le front habille l'image d'un élément absolu dont les deux fractions pilotent le voile et le trait. Aucune source Typst n'est touchée : l'habillage vit dans une couche que le PDF ne traverse jamais, et la bascule ne relance donc jamais Typst.

**Tech Stack:** Rust + Tauri 2 (`app/src-tauri`), front vanilla sans bundler (`app/src`), tests `cargo test` et `node --test`, faux DOM maison (`app/tests/dom_shim.js`).

**Spec:** `docs/superpowers/specs/2026-08-22-fond-perdu-visible-design.md`

**Toutes les commandes de ce plan partent de `app/` ou de `app/src-tauri/`, jamais de la racine.**

---

## Structure des fichiers

| Fichier | Rôle dans ce chantier |
|---|---|
| `app/src-tauri/src/planche.rs` | Ajoute `Gabarit::part_fond_perdu` — le gabarit sait ce qu'il mesure. Aucune source Typst modifiée. |
| `app/src-tauri/src/commands.rs` | `couverture_apercu` rend `Apercu { image, coupe }` au lieu d'une chaîne nue. |
| `app/src/index.html` | Le cadre autour de l'aperçu, l'élément `#coupe`, le bouton `#btFondPerdu`. |
| `app/src/couverture.js` | `poserApercu` lit le nouveau contrat ; `poserCoupe`, `rendreCoupe`, `basculerFondPerdu`. |
| `app/src/app.js` | L'état de la lunette (`coupeCourante`, `fondPerduVisible`) et le branchement du bouton. |
| `app/src/styles.css` | Le cadre, le voile, le pointillé, le vêtement du bouton à deux états. |
| `app/tests/dom_shim.js` | Le faux DOM apprend à retenir une variable CSS. |
| `app/tests/*.test.js` | Les faux `invoke` suivent le nouveau contrat ; les tests neufs de l'habillage. |

---

## Task 1 : Le gabarit sait quelle part le fond perdu prend

**Files:**
- Modify: `app/src-tauri/src/planche.rs` (après `hauteur()`, vers la ligne 74)
- Test: `app/src-tauri/src/planche.rs` (module `tests`, à côté de `la_planche_mesure_le_gabarit_du_prestataire`)

- [ ] **Step 1: Écrire le test qui échoue**

À ajouter dans le module `tests` de `planche.rs`, juste après `la_planche_mesure_le_gabarit_du_prestataire` :

```rust
    /// L'aperçu marque la coupe en pourcentage de l'image qu'il habille : c'est une
    /// fraction, pas des millimètres, et il en faut **deux**. Une planche fait près de
    /// 250 mm de large pour 180 de haut ; la même fraction sur les deux dimensions
    /// marquerait la coupe à côté d'elle-même.
    #[test]
    fn la_part_du_fond_perdu_differe_en_largeur_et_en_hauteur() {
        let g = gabarit("tbe-110x170", 280);
        let (x, y) = g.part_fond_perdu();
        assert!((x - 5.0 / 246.8).abs() < 1e-6, "part en largeur : {x}");
        assert!((y - 5.0 / 180.0).abs() < 1e-6, "part en hauteur : {y}");
        assert!(x < y, "la planche est plus large que haute : {x} devrait être < {y}");
    }

    /// La face Dos compose sur un gabarit à fond perdu nul (voir `source_dos`). Rien à
    /// y marquer — et surtout pas un trait sur le bord même de l'image, qui se lirait
    /// comme une coupe à zéro millimètre du texte.
    #[test]
    fn un_gabarit_sans_fond_perdu_ne_donne_aucune_part() {
        let g = Gabarit {
            format: (108.0, 175.0),
            dos: 13.0,
            fond_perdu: 0.0,
        };
        assert_eq!(g.part_fond_perdu(), (0.0, 0.0));
    }
```

- [ ] **Step 2: Vérifier qu'il échoue**

```bash
cd app/src-tauri && cargo test --lib planche::tests::la_part_du_fond_perdu
```

Attendu : ÉCHEC à la compilation — `no method named 'part_fond_perdu' found for struct 'Gabarit'`.

- [ ] **Step 3: Écrire l'implémentation minimale**

Dans `impl Gabarit`, juste après `hauteur()` :

```rust
    /// La part que le fond perdu prend sur la largeur et sur la hauteur de la planche,
    /// en fraction de celle-ci.
    ///
    /// C'est la mesure dont l'aperçu a besoin pour marquer la coupe sur une image qu'il
    /// affiche à une taille quelconque : les millimètres n'y survivent pas, les
    /// proportions oui. Deux fractions et non une : une planche est bien plus large que
    /// haute, et le même fond perdu n'y pèse pas pareil.
    pub fn part_fond_perdu(&self) -> (f64, f64) {
        (
            self.fond_perdu / self.largeur(),
            self.fond_perdu / self.hauteur(),
        )
    }
```

- [ ] **Step 4: Vérifier que les deux tests passent**

```bash
cd app/src-tauri && cargo test --lib planche::tests
```

Attendu : `test result: ok`, avec les deux tests neufs listés.

- [ ] **Step 5: Commit**

```bash
git add app/src-tauri/src/planche.rs
git commit -m "$(cat <<'EOF'
Le gabarit sait quelle part le fond perdu prend sur sa planche

Deux fractions et non une : la même bande de cinq millimètres ne pèse pas
pareil sur 247 mm de large et sur 180 de haut.

Claude-Session: https://claude.ai/code/session_01TztC3DS2P2stq3tgK1HctX
EOF
)"
```

---

## Task 2 : L'aperçu rend l'image **et** où la couper

Le contrat `couverture_apercu` passe de `String` à un objet. Les faux `invoke` des tests JS qui rendaient une chaîne nue doivent suivre dans le même commit : un contrat se change d'un bloc.

**Files:**
- Modify: `app/src-tauri/src/commands.rs` (la commande `couverture_apercu`, vers la ligne 714-787)
- Modify: `app/src/couverture.js` (`poserApercu`, vers la ligne 441)
- Modify: `app/tests/couverture.test.js` (trois faux `invoke`), `app/tests/composition.test.js`, `app/tests/ebook.test.js`

- [ ] **Step 1: Écrire le test qui échoue — le faux passe au nouveau contrat**

Dans `app/tests/couverture.test.js`, fonction `ouvre`, remplacer la ligne :

```js
    if (cmd === 'couverture_apercu') return 'data:image/png;base64,QUJD';
```

par :

```js
    // La planche est la seule face qui se compose avec du fond perdu : elle seule
    // rend une coupe. Les fractions sont celles d'une poche Lulu à 3,175 mm.
    if (cmd === 'couverture_apercu') {
      return {
        image: 'data:image/png;base64,QUJD',
        coupe: args.face === 'planche' ? { x: 0.0129, y: 0.0175 } : null,
      };
    }
```

- [ ] **Step 2: Vérifier que les tests d'aperçu échouent**

```bash
cd app && node --test tests/couverture.test.js
```

Attendu : ÉCHEC de « l'aperçu est demandé et affiché à l'ouverture du projet » — `els.get('apercu').src` vaut `[object Object]` au lieu de la data URL.

- [ ] **Step 3: Le front lit le nouveau contrat**

Dans `app/src/couverture.js`, remplacer `poserApercu` (le corps seul ; le commentaire au-dessus reste vrai) :

```js
function poserApercu(a) {
  const img = $('apercu');
  if (a) img.src = a.image;
  else img.removeAttribute('src');
  img.hidden = !a;
}
```

- [ ] **Step 4: Vérifier que les tests de ce fichier repassent**

```bash
cd app && node --test tests/couverture.test.js
```

Attendu : tous verts.

- [ ] **Step 5: Les deux autres faux suivent**

Dans `app/tests/composition.test.js` et `app/tests/ebook.test.js`, remplacer :

```js
    if (cmd === 'couverture_apercu') return 'data:image/png;base64,AAAA';
```

par :

```js
    if (cmd === 'couverture_apercu') return { image: 'data:image/png;base64,AAAA', coupe: null };
```

Les faux de `contrats.test.js`, `coquille.test.js` et `cycle_de_vie.test.js` lèvent une erreur pour cette commande : ils n'ont rien à changer.

- [ ] **Step 6: Vérifier toute la suite JS**

```bash
cd app && node --test tests/*.test.js
```

Attendu : tous verts.

- [ ] **Step 7: Le Rust rend le nouveau contrat**

Dans `app/src-tauri/src/commands.rs`, juste au-dessus de `#[tauri::command] pub fn couverture_apercu` :

```rust
/// Ce qu'un aperçu de face donne à voir : l'image, et où la couper s'il y a lieu.
#[derive(Serialize)]
pub struct Apercu {
    pub image: String,
    /// Absente sur les faces qui se composent au format rogné, sans fond perdu — la
    /// 1ère, la 4ème et le dos. C'est le Rust qui l'affirme plutôt que la fenêtre qui
    /// le déduise d'un nom de face : le jour où une face gagne du fond perdu, elle
    /// gagne sa coupe sans qu'on y pense.
    pub coupe: Option<Coupe>,
}

/// La part du fond perdu sur chaque dimension de la planche, en fraction de celle-ci.
#[derive(Serialize)]
pub struct Coupe {
    pub x: f64,
    pub y: f64,
}
```

Changer la signature de la commande :

```rust
pub fn couverture_apercu(
    face: String,
    dos_mm: Option<f64>,
    atelier: State<Atelier>,
) -> Result<Apercu, String> {
```

Dans le corps, la branche `"planche"` retient le gabarit pour le mesurer ensuite. Remplacer la branche par :

```rust
        "planche" => {
            let dos = dos_mm.ok_or(
                "planche : composer l'intérieur d'abord, c'est la pagination qui donne le dos.",
            )?;
            let fp = pr.fond_perdu.or(fond_perdu_mm).ok_or_else(|| {
                format!(
                    "{} ne publie pas de fond perdu : le relever sur son gabarit et le saisir.",
                    pr.libelle
                )
            })?;
            let g = planche::Gabarit {
                format: pr.format,
                dos,
                fond_perdu: fp,
            };
            let (x, y) = g.part_fond_perdu();
            coupe = Some(Coupe { x, y });
            planche::source(&o.projet.meta.livre, cv, &g, une.as_ref(), quatre.as_ref())?
        }
```

Déclarer `coupe` avant le `match`, juste au-dessus de `let src = match face.as_str() {` :

```rust
    // Seule la planche se compose avec du fond perdu : les trois autres faces n'ont
    // rien à faire marquer.
    let mut coupe = None;
```

Et remplacer la dernière ligne de la fonction (`donnee_png(&png)`) par :

```rust
    Ok(Apercu {
        image: donnee_png(&png)?,
        coupe,
    })
```

- [ ] **Step 8: Vérifier que le Rust compile et que ses tests passent**

```bash
cd app/src-tauri && cargo test
```

Attendu : `test result: ok` sur toute la bibliothèque.

- [ ] **Step 9: Commit**

```bash
git add app/src-tauri/src/commands.rs app/src/couverture.js app/tests/
git commit -m "$(cat <<'EOF'
L'aperçu rend l'image et, sur la planche, où la couper

Le contrat passe d'une chaîne à un objet : la fenêtre reçoit avec le PNG la
part que le fond perdu prend sur chaque dimension. Les faux invoke des tests
suivent — un contrat se change d'un bloc.

Claude-Session: https://claude.ai/code/session_01TztC3DS2P2stq3tgK1HctX
EOF
)"
```

---

## Task 3 : Le faux DOM retient une variable CSS

Le front ne pose aujourd'hui aucun style inline — il travaille par attributs `data-*` et par `hidden`. Deux fractions ne se transportent pas ainsi : le CSS ne sait pas lire un nombre dans un attribut. Il faut donc `style.setProperty`, que le faux DOM ne connaît pas.

**Files:**
- Modify: `app/tests/dom_shim.js` (classe `El`)
- Test: `app/tests/dom_shim.test.js`

- [ ] **Step 1: Écrire le test qui échoue**

À la fin de `app/tests/dom_shim.test.js` :

```js
/**
 * Une variable CSS est le seul moyen de faire passer un nombre du Rust à la feuille de
 * style : un attribut `data-` ne se lit pas dans un `calc()`. Le faux DOM doit donc
 * savoir en retenir une, sans quoi l'habillage de la coupe ne s'exécute nulle part.
 *
 * Sur `couv`, et non sur le cadre de l'aperçu : ce qui est vérifié ici est le faux DOM
 * lui-même, pas ce que l'application en fait — n'importe quel élément fait l'affaire.
 */
test('une variable CSS posée sur un élément se relit', async () => {
  const { els } = await charge({ invoke: invokeMuet });
  const el = els.get('couv');
  el.style.setProperty('--coupe-x', '0.0129');
  assert.strictEqual(el.style.getPropertyValue('--coupe-x'), '0.0129');
  assert.strictEqual(el.style.getPropertyValue('--coupe-y'), '',
    'une variable jamais posée doit se lire vide, comme dans le navigateur');
});
```

- [ ] **Step 2: Vérifier qu'il échoue**

```bash
cd app && node --test tests/dom_shim.test.js
```

Attendu : ÉCHEC — `Cannot read properties of undefined (reading 'setProperty')`.

- [ ] **Step 3: Écrire l'implémentation minimale**

Dans `app/tests/dom_shim.js`, ajouter dans le constructeur de `El`, après `this.hidden = false;` :

```js
    // Le style inline, réduit aux variables CSS : c'est tout ce que l'application y
    // pose — les deux fractions de la coupe, que ni un attribut `data-` ni une classe
    // ne peuvent transporter jusqu'à un `calc()`.
    const proprietes = new Map();
    this.style = {
      setProperty: (nom, valeur) => proprietes.set(nom, String(valeur)),
      getPropertyValue: (nom) => proprietes.get(nom) ?? '',
    };
```

- [ ] **Step 4: Vérifier que le test passe**

```bash
cd app && node --test tests/dom_shim.test.js
```

Attendu : vert.

- [ ] **Step 5: Commit**

```bash
git add app/tests/dom_shim.js app/tests/dom_shim.test.js
git commit -m "$(cat <<'EOF'
Le faux DOM retient les variables CSS que l'application pose

Réduit à ce que l'application en fait : deux fractions qu'un attribut data ne
peut pas porter jusqu'à un calc().

Claude-Session: https://claude.ai/code/session_01TztC3DS2P2stq3tgK1HctX
EOF
)"
```

---

## Task 4 : Le cadre, l'habillage, et les fractions posées dessus

**Files:**
- Modify: `app/src/index.html` (la scène, vers la ligne 138)
- Modify: `app/src/couverture.js` (`poserApercu`, `poserCoupe`, `rendreCoupe`)
- Modify: `app/src/app.js` (l'état, vers la ligne 37)
- Test: `app/tests/couverture.test.js`

- [ ] **Step 1: Écrire les tests qui échouent**

Dans `app/tests/couverture.test.js`, à la fin de la section `/* ---------- aperçu ---------- */` :

```js
/**
 * La face Planche est la vue de contrôle : c'est là, et là seulement, qu'une image à
 * fond perdu voulue et une pastille tombée sous la coupe cessent de se ressembler.
 * Les fractions viennent du Rust — les recalculer ici redirait la règle qui choisit
 * entre le fond perdu publié et le relevé.
 */
test('la planche marque la coupe avec les fractions que le Rust donne', async () => {
  const { els } = await ouvre(maquette());
  await face(els, 'Planche').declenche('click');
  await attendreApercu();
  const cadre = els.get('cadreApercu');
  assert.strictEqual(cadre.style.getPropertyValue('--coupe-x'), '0.0129');
  assert.strictEqual(cadre.style.getPropertyValue('--coupe-y'), '0.0175');
  assert.strictEqual(els.get('coupe').hidden, false, 'coupe non marquée sur la planche');
});

/**
 * La 1ère se compose au format rogné : il n'y a pas de bande à couper, et un trait sur
 * le bord même de l'image se lirait comme une coupe à zéro millimètre du texte.
 */
test('une face sans fond perdu ne montre aucune coupe', async () => {
  const { els } = await ouvre(maquette());
  await attendreApercu();
  assert.strictEqual(els.get('coupe').hidden, true, 'coupe marquée sur la 1ère');
});

/**
 * Un aperçu qui échoue retire l'image ; l'habillage doit partir avec elle. Seul sur la
 * scène, il marquerait la coupe d'une couverture qui n'est plus affichée.
 */
test('un aperçu qui échoue emporte l\'habillage avec l\'image', async () => {
  let casse = false;
  const { els } = await ouvre(maquette(), {
    couverture_apercu: (args) => {
      if (casse) throw 'prolongement panoramique : la largeur du dos est inconnue';
      return {
        image: 'data:image/png;base64,QUJD',
        coupe: args.face === 'planche' ? { x: 0.0129, y: 0.0175 } : null,
      };
    },
  });
  await face(els, 'Planche').declenche('click');
  await attendreApercu();
  assert.strictEqual(els.get('coupe').hidden, false);

  casse = true;
  await els.get('inDestinataire').declenche('change');
  await attendreApercu();
  assert.strictEqual(els.get('coupe').hidden, true, 'habillage laissé seul à l\'écran');
});
```

- [ ] **Step 2: Vérifier qu'ils échouent**

```bash
cd app && node --test tests/couverture.test.js
```

Attendu : ÉCHEC — `identifiant absent d'index.html : cadreApercu`.

- [ ] **Step 3: Poser le cadre et l'habillage dans le HTML**

Dans `app/src/index.html`, remplacer la ligne 138 :

```html
        <div class="scene"><img id="apercu" class="apercu" alt="Aperçu de la couverture"></div>
```

par :

```html
        <!-- L'habillage se cale sur l'image, jamais sur la scène : la scène occupe la
             colonne entière, une couverture y est centrée et plus étroite, et un
             habillage calé sur elle marquerait la coupe à côté du livre. D'où ce cadre,
             qui n'existe que pour épouser l'image. -->
        <div class="scene">
          <div class="cadre" id="cadreApercu">
            <img id="apercu" class="apercu" alt="Aperçu de la couverture">
            <div class="coupe" id="coupe" hidden></div>
          </div>
        </div>
```

- [ ] **Step 4: Poser l'état dans `app.js`**

Dans `app/src/app.js`, après `let face = 'une';` (ligne 37) :

```js
/**
 * La coupe du dernier aperçu posé, s'il en avait une, et si la lunette est allumée.
 *
 * Les deux vivent ici et non dans le projet : ce qu'on regarde n'est pas ce qu'on
 * imprime. Rien n'en va dans le `.ozalid`, et le PDF remis ne porte aucun repère —
 * c'est ce que `planche.rs` promet en tête de fichier.
 */
let coupeCourante = null;
let fondPerduVisible = true;
```

- [ ] **Step 5: Poser les fractions dans `couverture.js`**

Remplacer `poserApercu` (issu de la tâche 2) par :

```js
function poserApercu(a) {
  const img = $('apercu');
  if (a) img.src = a.image;
  else img.removeAttribute('src');
  img.hidden = !a;
  poserCoupe(a?.coupe ?? null);
}

/**
 * La bande que le massicot emporte, mesurée par le Rust et posée sur l'image.
 *
 * Deux fractions, pas des millimètres : l'aperçu s'affiche à la taille que la fenêtre
 * lui laisse, et seules des proportions y survivent. Elles ne se recalculent pas ici —
 * ce serait redire la règle qui choisit entre le fond perdu publié par le prestataire
 * et celui relevé sur son gabarit.
 */
function poserCoupe(coupe) {
  coupeCourante = coupe;
  if (coupe) {
    const cadre = $('cadreApercu');
    cadre.style.setProperty('--coupe-x', String(coupe.x));
    cadre.style.setProperty('--coupe-y', String(coupe.y));
  }
  rendreCoupe();
}

/** L'habillage suit deux choses : l'aperçu posé et la lunette. Les deux passent ici. */
function rendreCoupe() {
  $('coupe').hidden = !coupeCourante || !fondPerduVisible;
}
```

- [ ] **Step 6: Vérifier que tout passe**

```bash
cd app && node --test tests/*.test.js
```

Attendu : tous verts, les trois tests neufs compris.

- [ ] **Step 7: Commit**

```bash
git add app/src/index.html app/src/app.js app/src/couverture.js app/tests/
git commit -m "$(cat <<'EOF'
L'aperçu porte la coupe que le Rust a mesurée

Un cadre qui épouse l'image — la scène est plus large que le livre — et deux
fractions posées dessus. Ce qu'on regarde n'est pas ce qu'on imprime : rien
n'en va dans le .ozalid.

Claude-Session: https://claude.ai/code/session_01TztC3DS2P2stq3tgK1HctX
EOF
)"
```

---

## Task 5 : Le voile et le pointillé

Aucun test automatique ici : le faux DOM ne rend rien, et un test qui vérifierait la présence d'une déclaration CSS ne protégerait que sa propre orthographe. La garde est visuelle, et l'étape 3 la décrit précisément.

**Files:**
- Modify: `app/src/styles.css` (après la règle `.apercu`, vers la ligne 558)

- [ ] **Step 1: Écrire le CSS**

Après la règle `.apercu { … }` :

```css
/* Le cadre de l'aperçu : il n'a pas d'apparence, il n'existe que pour donner à
   l'habillage exactement la boîte de l'image. Il reprend donc les contraintes qui
   dimensionnent l'image — sans quoi une planche haute pousserait la fenêtre, ce que
   `.scene` existe pour empêcher. */
.cadre {
  position: relative;
  display: flex;
  max-width: 100%;
  max-height: 100%;
  min-height: 0;
}

/* Le dos couché prend toute la largeur ; son cadre aussi, sans quoi l'image y serait
   à nouveau dimensionnée par sa hauteur. */
.couv[data-face="dos"] .cadre { width: 100%; }

/* La bande que le massicot emporte, et la ligne de coupe.
   Les deux ensemble, et pas l'un ou l'autre : sur une couverture claire le voile ne se
   voit presque pas et le trait reste ; sur une photo sombre à fond perdu le trait se
   perd dans le motif et le voile reste.
   Le voile est fait de quatre fonds et non d'une ombre étalée sous un `overflow`
   caché : l'aperçu porte une ombre portée que ce découpage emporterait, et la
   couverture paraîtrait posée à plat. Les bandes latérales s'arrêtent au-dessus et
   au-dessous des horizontales — deux voiles superposés assombriraient les quatre coins,
   et ces coins-là sont justement ce qu'on regarde.
   Les deux fractions viennent du Rust, posées par `poserCoupe`. */
.coupe {
  position: absolute;
  inset: 0;
  pointer-events: none;
  --voile: rgba(255, 255, 255, .5);
  background:
    linear-gradient(var(--voile) 0 0) 0 0 / 100% calc(var(--coupe-y) * 100%),
    linear-gradient(var(--voile) 0 0) 0 100% / 100% calc(var(--coupe-y) * 100%),
    linear-gradient(var(--voile) 0 0) 0 50%
      / calc(var(--coupe-x) * 100%) calc(100% - var(--coupe-y) * 200%),
    linear-gradient(var(--voile) 0 0) 100% 50%
      / calc(var(--coupe-x) * 100%) calc(100% - var(--coupe-y) * 200%);
  background-repeat: no-repeat;
}

/* La ligne de coupe elle-même, sur le rectangle rogné. En pseudo-élément : elle n'a
   rien à dire au balisage, et l'habillage reste un seul nœud. */
.coupe::after {
  content: '';
  position: absolute;
  inset: calc(var(--coupe-y) * 100%) calc(var(--coupe-x) * 100%);
  border: 1px dashed rgba(0, 0, 0, .6);
}
```

- [ ] **Step 2: Reconstruire l'application**

Le front est embarqué à la compilation : un fichier de `src/` modifié seul ne repart pas dans le binaire.

```bash
cd app/src-tauri && touch src/lib.rs && cargo tauri dev
```

- [ ] **Step 3: Vérifier à l'œil, et noter ce qui a été vu**

Ouvrir un projet réel, viser un prestataire qui publie son fond perdu (Lulu), composer l'intérieur pour avoir un dos, puis :

1. **Face Planche** : la bande de fond perdu est éclaircie sur les quatre côtés, un pointillé la borde, et les quatre coins ont la même clarté que les côtés (pas de coin plus sombre).
2. **Une couverture à photo sombre à fond perdu** : le pointillé se lit encore.
3. **Une couverture claire sans image** : la bande se distingue encore, ne serait-ce que par le trait.
4. **Les trois autres faces** : aucun habillage, et l'ombre portée sous l'image est toujours là — c'est ce que le voile en quatre fonds protège.
5. **Face Dos** : le bandeau prend toujours la largeur de la fenêtre.
6. **Fenêtre réduite à 900 px de large** : la barre d'outils tient sur une ligne.

- [ ] **Step 4: Commit**

```bash
git add app/src/styles.css
git commit -m "$(cat <<'EOF'
La planche montre en clair la bande que le massicot emporte

Un voile en quatre fonds plutôt qu'une ombre étalée sous un overflow caché :
celle-ci emporterait l'ombre portée de l'aperçu. Les bandes latérales
s'arrêtent aux horizontales, sans quoi les quatre coins doubleraient de voile.

Claude-Session: https://claude.ai/code/session_01TztC3DS2P2stq3tgK1HctX
EOF
)"
```

---

## Task 6 : La bascule

**Files:**
- Modify: `app/src/index.html` (la barre `.outils`, après `#faces`)
- Modify: `app/src/couverture.js` (`basculerFondPerdu`, `poserDisposition`)
- Modify: `app/src/app.js` (le branchement, vers la ligne 995)
- Modify: `app/src/styles.css` (le vêtement du bouton)
- Test: `app/tests/couverture.test.js`

- [ ] **Step 1: Écrire les tests qui échouent**

À la suite des tests de la tâche 4 :

```js
/**
 * Éteindre la lunette montre la couverture telle qu'elle sera en main. Sans nouvelle
 * composition : c'est tout l'intérêt d'habiller l'image plutôt que de la refaire —
 * Typst met une seconde là où le CSS ne met rien.
 */
test('éteindre le fond perdu retire l\'habillage sans recomposer', async () => {
  const { els, appels } = await ouvre(maquette());
  await face(els, 'Planche').declenche('click');
  await attendreApercu();
  const avant = appels.filter(([c]) => c === 'couverture_apercu').length;

  await els.get('btFondPerdu').declenche('click');
  assert.strictEqual(els.get('coupe').hidden, true, 'habillage resté allumé');
  assert.strictEqual(els.get('btFondPerdu').getAttribute('aria-pressed'), 'false');
  assert.strictEqual(
    appels.filter(([c]) => c === 'couverture_apercu').length, avant,
    'la bascule a relancé une composition'
  );

  await els.get('btFondPerdu').declenche('click');
  assert.strictEqual(els.get('coupe').hidden, false, 'habillage non rallumé');
});

/**
 * Un bouton qui ne peut rien faire est un piège : les trois autres faces n'ont pas de
 * fond perdu à montrer. Même raison que les réglages sans objet, masqués plutôt que
 * grisés.
 */
test('la bascule ne s\'offre que sur la planche', async () => {
  const { els } = await ouvre(maquette());
  await attendreApercu();
  assert.strictEqual(els.get('btFondPerdu').hidden, true, 'bascule offerte sur la 1ère');

  await face(els, 'Planche').declenche('click');
  await attendreApercu();
  assert.strictEqual(els.get('btFondPerdu').hidden, false, 'bascule absente de la planche');
});
```

- [ ] **Step 2: Vérifier qu'ils échouent**

```bash
cd app && node --test tests/couverture.test.js
```

Attendu : ÉCHEC — `identifiant absent d'index.html : btFondPerdu`.

- [ ] **Step 3: Poser le bouton dans le HTML**

Dans `app/src/index.html`, juste après `<div class="ligne onglets" id="faces"></div>` :

```html
      <!-- La lunette de la planche. Hors de `#faces` : cette boîte est reconstruite par
           `construireFaces`, et `choisirFace` y compte les enfants par rang — un bouton
           étranger dedans ferait viser la face voisine. Elle naît allumée et masquée :
           la planche n'est pas la face de départ. -->
      <button id="btFondPerdu" class="lunette" type="button" aria-pressed="true" hidden>
        Fond perdu
      </button>
```

- [ ] **Step 4: Brancher la bascule**

Dans `app/src/couverture.js`, après `rendreCoupe` :

```js
/**
 * Allume ou éteint la lunette.
 *
 * Rien à recomposer : l'habillage est posé **sur** l'image, pas dedans. C'est ce qui
 * rend la bascule instantanée — et ce qui garantit qu'aucun repère ne peut se glisser
 * dans le PDF remis au prestataire.
 */
function basculerFondPerdu() {
  fondPerduVisible = !fondPerduVisible;
  $('btFondPerdu').setAttribute('aria-pressed', String(fondPerduVisible));
  rendreCoupe();
}
```

Dans la même fonction `poserDisposition`, après la ligne `$('reglages').hidden = !panneau;` :

```js
  // La lunette n'a d'objet que là où il y a du fond perdu à voir. Masquée plutôt que
  // grisée, comme les réglages sans objet.
  $('btFondPerdu').hidden = face !== 'planche';
```

Dans `app/src/app.js`, à côté des autres branchements de la fin du fichier, juste avant `construireEtapes();` :

```js
$('btFondPerdu').addEventListener('click', basculerFondPerdu);
```

- [ ] **Step 5: Habiller le bouton**

Dans `app/src/styles.css`, après la règle `.onglets button[aria-pressed="true"] { … }` :

```css
/* La lunette de la planche : deux états, comme une face, mais elle n'est pas une face —
   `.onglets button` ne peut pas l'habiller, elle vit hors de `#faces` pour la raison
   que dit le balisage. Elle porte donc le même vêtement, redit ici. */
.lunette { background: transparent; color: var(--encre); border-color: var(--trait); }

.lunette[aria-pressed="true"] {
  background: var(--encre);
  color: var(--surface);
  border-color: var(--encre);
}

.lunette:hover:not([aria-pressed="true"]) { background: var(--survol); }
```

- [ ] **Step 6: Vérifier que tout passe**

```bash
cd app && node --test tests/*.test.js
```

Attendu : tous verts.

- [ ] **Step 7: Vérifier à l'œil**

```bash
cd app/src-tauri && touch src/lib.rs && cargo tauri dev
```

Sur la face Planche : le bouton « Fond perdu » est plein (allumé) ; un clic le creuse et l'habillage disparaît ; un second clic le ramène. Sur les trois autres faces, le bouton n'est pas là et la barre reste sur une ligne à 900 px.

- [ ] **Step 8: Commit**

```bash
git add app/src/index.html app/src/app.js app/src/couverture.js app/src/styles.css app/tests/couverture.test.js
git commit -m "$(cat <<'EOF'
La lunette du fond perdu s'éteint pour montrer le livre en main

Une bascule, pas un réglage : elle n'entre pas dans le .ozalid et ne recompose
rien — l'habillage est posé sur l'image, pas dedans.

Claude-Session: https://claude.ai/code/session_01TztC3DS2P2stq3tgK1HctX
EOF
)"
```

---

## Task 7 : Les vérifications avant commit du projet

**Files:** aucun, sauf correction.

- [ ] **Step 1: Le format et les avertissements**

```bash
cd app/src-tauri && cargo fmt --check && cargo clippy --all-targets -- -D warnings
```

Attendu : aucune sortie, code 0.

- [ ] **Step 2: Les deux suites**

```bash
cd app/src-tauri && cargo test
cd app && node --test tests/*.test.js
```

Attendu : `test result: ok` des deux côtés.

- [ ] **Step 3: Le témoin de non-régression**

Des fichiers de `app/src-tauri/` ont changé : le compte de pages affiché doit être identique au précédent sur le même manuscrit.

```bash
cd app/src-tauri && cargo run --example temoin
```

Attendu : le même nombre de pages qu'avant le chantier. Ce chantier ne touche aucune source Typst — un écart ici est un bug, pas un effet de bord acceptable.

- [ ] **Step 4: Commit, seulement si une des trois étapes a demandé une correction**

Si les trois étapes sont passées du premier coup, il n'y a rien à commiter : ne pas
fabriquer un commit vide. Sinon, commiter la correction sous son propre motif — une
règle de `clippy`, un `cargo fmt`, un test rendu vert — en disant ce qui a été corrigé
et pourquoi, jamais « corrections diverses ».

```bash
git add -A app
git commit
```

---

## Ce que ce plan ne fait pas

- **Mesurer le débord d'un élément** (« la pastille dépasse de 2 mm »). Il faudrait que le Rust connaisse la boîte de chaque élément composé ; c'est Typst qui la sait. Autre chantier.
- **Marquer le pli du dos.** La planche montre déjà ses trois zones par leurs fonds.
- **Toucher `planche::source`.** Le fichier remis au prestataire ne porte aucun repère, et rien dans ce plan ne l'approche.
