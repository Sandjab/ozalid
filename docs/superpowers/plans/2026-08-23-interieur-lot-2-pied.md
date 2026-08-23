# Lot 2 — le pied prend la légende

Spec : `docs/superpowers/specs/2026-08-23-interieur-sans-onglet-design.md`
Précédent : `2026-08-23-interieur-lot-1-demenagement.md`

## Les deux arbitrages ouverts, tranchés

Les deux recommandations de la spec sont retenues.

1. **`polices_introuvables` entre dans `Mesure`** (§ 4). Une mesure décrit ce que la
   composition a produit, et un PDF composé dans une écriture de repli en fait partie.
   L'oublier à la réouverture ferait dire au pied que tout va bien devant un fichier qui
   ne suit pas la maquette.
2. **`tauri-plugin-opener` entre au projet** (§ 7), et le chemin de l'**épreuve** devient
   cliquable du même geste — le laisser en texte mort à côté d'un lien vivant serait une
   incohérence gratuite.

## Ce que ce lot fait

Le panneau de résultat meurt. Ce qu'il disait descend au pied, réduit à une légende, et
ce qu'il alertait se dédouble : un signe court au pied, le détail sous la police.

Après ce lot, l'étape Livre ne porte plus que des **réglages** — plus un seul compte
rendu. C'est ce qui résorbe le débordement à 900 × 640 relevé au lot 1, et que le
commentaire de `styles.css` annonce depuis deux chantiers.

**« Composer l'intérieur » survit encore.** Il ne meurt qu'au lot 3.

## Tâche 1 : le Rust porte ce que le pied doit lire

**Files:**
- Modify: `app/src-tauri/src/projet.rs`
- Modify: `app/src-tauri/src/commands.rs`

- [ ] **Step 1: `Mesure` retient le repli de police**

Ajouter à `Mesure` :

```rust
/// Familles que Typst n'a pas trouvées et a remplacées par une écriture de repli.
///
/// Retenu avec la mesure et non dans une variable de l'écran : le PDF composé dans une
/// écriture de repli ne redevient pas juste en rouvrant le livre. Un pied qui se
/// tairait à la réouverture dirait que tout va bien devant un fichier qui ne suit pas
/// la maquette.
#[serde(default, skip_serializing_if = "Vec::is_empty")]
pub polices_introuvables: Vec<String>,
```

**`Copy` doit partir** de son `derive` : un `Vec` n'est pas copiable. Trois points
d'appel dans les tests de `projet.rs` copient un `Option<Mesure>` derrière une référence
et réclameront un `.clone()`. `const MESURE` reste possible — `Vec::new()` est `const`.

`VERSION` ne bouge pas : le champ arrive avec son défaut, et une archive antérieure se
relit avec un vecteur vide, ce qui est exactement ce qu'elle voulait dire.

- [ ] **Step 2: `composer` la remplit**

Le `Mesure { … }` construit en fin de `composer` reçoit `polices_introuvables.clone()`
— le `Composition` renvoyé en garde une copie pour le compte rendu immédiat.

- [ ] **Step 3: `ProjetVue` sait où est le PDF de l'intérieur**

Ajouter à `ProjetVue` :

```rust
/// Le PDF de l'intérieur composé pour le destinataire visé, s'il est sur le disque.
///
/// Dérivé et non retenu : un `.ozalid` déplacé ou ouvert sur une autre machine porterait
/// un chemin absolu qui ne mène nulle part. Il est calculé à chaque vue, et l'existence
/// du fichier est vérifiée — un lien vers un PDF effacé à la main est pire que pas de
/// lien.
///
/// Absent tant que la mesure l'est : un PDF qui traîne d'une composition périmée n'est
/// pas celui du livre qu'on regarde.
pub interieur_pdf: Option<String>,
```

Calculé dans `vue(o)` — le seul entonnoir — depuis `livraison.courant()`, sa `compose`,
`sorties_dossier(o, cle)` et le nom `interieur-{cle}.pdf` qu'emploie `composer`. **Ce nom
est écrit à deux endroits désormais** : le dire en commentaire aux deux, ou le sortir
dans une fonction. Préférer la fonction.

## Tâche 2 : le plugin qui ouvre un fichier

**Files:**
- Modify: `app/src-tauri/Cargo.toml`
- Modify: `app/src-tauri/src/lib.rs`
- Modify: `app/src-tauri/capabilities/default.json`

- [ ] **Step 1: La dépendance**

```bash
cd app/src-tauri && cargo add tauri-plugin-opener
```

- [ ] **Step 2: L'initialisation et la permission**

`.plugin(tauri_plugin_opener::init())` à côté de `tauri_plugin_dialog::init()`, et
`"opener:allow-open-path"` dans les permissions de `default.json`.

Si la permission réclame une portée, la donner **la plus étroite possible** et écrire
pourquoi : ce plugin n'ouvre que des PDF que l'application vient d'écrire.

- [ ] **Step 3: Vérifier que ça compile et que le front voit le plugin**

`withGlobalTauri` est vrai, donc `window.__TAURI__.opener.openPath` doit exister. À
confirmer **dans la fenêtre**, pas seulement à la compilation.

## Tâche 3 : le pied prend la légende

**Files:**
- Modify: `app/src/index.html`
- Modify: `app/src/app.js`
- Modify: `app/src/styles.css`

- [ ] **Step 1: Quatre éléments, un rôle chacun**

Le pied porte aujourd'hui `#visee` et `#piedDos`. Il portera :

```
#visee        pour qui l'on regarde          (inchangé)
#piedMesure   · N pages · N chapitres · gouttière N mm
#piedDos      · dos N mm  /  périmé  /  non composé  /  relevé sur le gabarit
#piedInterieur  · intérieur                  (un lien)
#piedRepli    · ⚠ repli                      (en rouge)
```

**`#piedDos` garde son nom et son rôle**, tels que le lot 1 les a laissés : ses tests ne
bougent pas. Découper plutôt qu'entasser dans un seul élément a une raison précise —
le lien réclame un `<a>` enfant, et un élément qui mêle du texte et un enfant ne se lit
plus par `textContent` dans les tests.

- [ ] **Step 2: `majPied` remplit les quatre**

La mesure vient de `destinataireCourant()?.compose`, le compte de chapitres de
`projet.chapitres_trouves`, le lien de `projet.interieur_pdf`.

Règles :
- **Aucun projet** : tout est vide, comme aujourd'hui.
- **Périmé** (`dosPerime(projet)`) : `#piedDos` seul parle, en rouge. Les chiffres se
  taisent — ils décriraient une composition qui ne vaut plus.
- **Pas de mesure** : `#piedDos` dit « non composé » ou « relevé sur le gabarit », les
  chiffres se taisent.
- **Mesure présente** : les chiffres, le dos, le lien s'il y a un PDF, et `⚠ repli` si la
  mesure porte des polices introuvables.

**« Page blanche de fin » ne remonte pas.** Une parité qu'on regarde une fois ne mérite
pas une place dans une légende qui suit partout (spec § « Décisions de cadrage »).

- [ ] **Step 3: Le lien**

Un `<a href="#">` dont le `title` porte le chemin entier et le texte le seul mot
« intérieur ». Au clic : `openPath(chemin)`, et `preventDefault` — un `href` qui
naviguerait remplacerait la fenêtre de l'application par un PDF.

- [ ] **Step 4: La mise en forme**

Le pied doit tenir sur **une ligne à 900 px**. Cinq mentions plus un `<select>` de
destinataires, c'est le point de tension de ce lot. À mesurer à l'œil avec un libellé de
prestataire long, pas seulement à la lecture du CSS.

## Tâche 4 : le panneau de résultat meurt

**Files:**
- Modify: `app/src/index.html`
- Modify: `app/src/app.js`
- Modify: `app/src/styles.css`

- [ ] **Step 1: Le balisage**

`<div id="resultat" class="resultat" hidden></div>` disparaît de l'étape Livre. Un
`<p class="note alerte" id="repliPolices" hidden></p>` prend place **sous le sélecteur de
police**, avant la note qui explique la pagination.

- [ ] **Step 2: `afficher(c)` disparaît**

La fonction entière part. `composer()` ne l'appelle plus : la mesure entre dans le projet,
`afficherProjet` la rend au pied, et c'est tout. Le chemin du PDF n'a plus besoin d'être
lu dans le retour de `composer` — `vue()` le calcule.

- [ ] **Step 3: `oublierLaComposition` perd `resultat`**

Retirer l'identifiant de la liste. **Vérifier que rien d'autre ne le nomme** — le
commentaire de cette fonction explique pourquoi les canaux partent ensemble, et il vaut
toujours.

- [ ] **Step 4: Le détail du repli, sous la police**

`#repliPolices` est rempli depuis `afficherProjet`, comme le reste du panneau : il lit
`destinataireCourant()?.compose?.polices_introuvables`. Il ne vit pas d'un retour de
commande — sinon il disparaîtrait à la réouverture, ce que la tâche 1 vient précisément
d'empêcher.

- [ ] **Step 5: Le CSS**

La règle `.resultat` et ce qui la décore ne servent peut-être plus qu'aux packages, aux
ebooks et aux envois — **vérifier avant de supprimer quoi que ce soit**. Le commentaire
des colonnes annonce que l'ascenseur de l'étape s'en irait avec ce bloc : le relire et le
mettre d'accord avec ce qui est vrai après ce lot.

## Tâche 5 : l'épreuve devient cliquable

**Files:**
- Modify: `app/src/app.js`

- [ ] **Step 1**

`#cheminEpreuve` reçoit un `<a>` au lieu d'un texte quand `epreuve_tirer` a rendu un
chemin. Même geste que le lien du pied — en extraire une fonction plutôt que l'écrire
deux fois.

Les tests d'`epreuve.test.js` lisent `cheminEpreuve.textContent` : un `<a>` enfant le
laisse lisible, `textContent` traversant les enfants. **À vérifier avec le faux DOM**,
qui n'est pas un navigateur.

## Tâche 6 : les tests

- [ ] **Step 1: Voir rougir d'abord**

```bash
node --test tests/*.test.js
```

Lire les échecs avant d'y toucher : ils disent ce que le panneau protégeait.
`composition.test.js` et `coquille.test.js` en portent le plus.

- [ ] **Step 2: Ce qui doit être neuf, et vu échouer**

- Le pied porte les chiffres de la mesure, et se tait quand il n'y en a pas.
- Le pied se tait aussi quand la mesure est **périmée** — un dos périmé qui laisserait
  les pages à l'écran donnerait à lire un livre qui n'existe plus.
- Le lien n'est là que si `interieur_pdf` est présent.
- `⚠ repli` paraît au pied et le détail sous la police, tous deux depuis le **projet** —
  donc **après une réouverture**, ce qui est tout l'objet de la tâche 1. Ce test-là est
  celui qui compte.
- Côté Rust : `polices_introuvables` survit à l'aller-retour d'un `.ozalid`, et une
  archive écrite sans le champ se relit.

- [ ] **Step 3: Mutations ciblées**

Là où le rouge ne viendrait pas tout seul : retirer le `skip_serializing_if`, faire lire
les polices au retour de commande plutôt qu'au projet, laisser les chiffres à l'écran
quand le dos est périmé.

## Tâche 7 : vérifications, œil, commit

- [ ] **Step 1: Les commandes**

Depuis `app/src-tauri/` et `app/`, **jamais dans un pipe** : `cargo fmt --check`,
`cargo clippy --all-targets -- -D warnings`, `cargo test`, `node --test tests/*.test.js`,
puis `cargo run --example temoin` — attendu **98 pages, dos 7,21 mm**.

- [ ] **Step 2: À l'œil** (`touch src/lib.rs && cargo build` d'abord)

1. Un projet composé : le pied porte pages, chapitres, gouttière, dos et le lien.
2. Cliquer « intérieur » : le PDF s'ouvre.
3. Effacer le PDF à la main, rouvrir le projet : le lien n'est plus là, les chiffres si.
4. Périmer le dos : les chiffres se taisent, « dos périmé » en rouge reste seul.
5. **La fenêtre à 900 px** : le pied tient sur une ligne, et l'étape Livre ne défile plus.
6. Une police absente : `⚠ repli` au pied, le détail sous la police — **et après avoir
   refermé puis rouvert le projet**.

- [ ] **Step 3: Le README**

« L'écran » : ce que le pied dit désormais. Et la phrase sur le compte rendu d'un travail
long — « reste à côté du bouton qui l'a lancé » — gagne son troisième cas, celui que la
spec § 3 identifie : ce que personne n'a demandé se lit en légende, comme l'aperçu de
couverture.

- [ ] **Step 4: Commiter**

## Ce que ce lot laisse au lot 3

« Composer l'intérieur » vit toujours dans l'étape Livre. Il ne meurt qu'avec le
déclenchement automatique au chargement du manuscrit, et l'échec qui monte à
`alerter()` — le seul lot qui change vraiment le comportement, et le seul qui se révoque
d'un `revert`.
