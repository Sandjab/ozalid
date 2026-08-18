# Lot 3 — Export PDF 300 dpi de la planche : plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal :** exporter la planche d'assemblage (4ème + dos + 1ère, fond perdu compris) en PDF aux dimensions exactes en mm, contenant un PNG 300 dpi rendu par `html2canvas`.

**Architecture :** `buildPlanche` devient paramétrable en échelle (`cwPl` optionnel) ; l'export la reconstruit à `cwExport` telle que `html2canvas` à `scale: 2` produise exactement 300 dpi (jamais `scale: 1` — piège de l'ombre, spec §5). Un `onclone` unique (`preparePlancheClone`) neutralise les guides écran (hachures, ombre, tirets), remplit le fond perdu par les fonds adjacents et fige la géométrie de l'image de la 1ère (même bug `object-fit` que l'export 1ère). Le PNG est encapsulé via **pdf-lib** (CDN, global `PDFLib`) dans une page aux dimensions mm exactes. `render()` reste l'unique écrivain de styles.

**Tech stack :** fichier unique `index.html` ; nouvelle dépendance **pdf-lib 1.17.1** via cdnjs — la « raison forte » prévue par le CLAUDE.md (Lulu attend un PDF).

**Spécification :** `docs/superpowers/specs/2026-08-18-packaging-couverture-design.md` (§4, risques §5).
**Inclus en plus de la spec** (notes des revues du lot 2) : `--dos-bg` en variable CSS au lieu de l'écriture directe `$('dos').style.backgroundColor` ; le clamp du nombre de pages réécrit le champ au `change`.
**Décision prise au plan (la spec est muette)** : à l'export, le fond perdu est rempli par les trois fonds adjacents (couleur 4ème | couleur dos | papier 1ère) en dégradé à arrêts durs — pratique d'imprimeur pour fonds unis. En mode image, l'image de la 1ère ne s'étend **pas** dans le fond perdu : c'est la couleur papier qui y figure. Compromis assumé, à signaler à l'utilisateur à la clôture.
**Hors périmètre :** image de fond propre à la 4ème (différée, spec §2) ; Amazon KDP ; traits de coupe vectoriels dans le PDF (Lulu n'en demande pas pour un PDF au format exact fond perdu compris).

**Vérification (pas de framework de test) :** `node --check` sur le JS extrait + sondes navigateur (outils Playwright MCP : `browser_navigate`, `browser_evaluate`/`browser_run_code_unsafe`, `browser_take_screenshot`) sur l'URL donnée par `/Users/jean-paulgavini/.claude/scripts/serve.sh` (jamais `python3 -m http.server`). Extraction :

```bash
node -e "
const html = require('fs').readFileSync('index.html','utf8');
const blocks = [...html.matchAll(/<script(?![^>]*src)[^>]*>([\s\S]*?)<\/script>/g)].map(m=>m[1]);
require('fs').writeFileSync('/tmp/ozalid-extrait.js', blocks.join('\n;\n'));
" && node --check /tmp/ozalid-extrait.js && echo "Syntaxe OK"
```

**Repères** (ancres textuelles, les numéros de ligne bougent ; **ne jamais lire `index.html` en entier** — ligne base64 géante ~90 Ko) :
- `buildPlanche(P, dosMm)` : après le commentaire `/* ---------- assemblage : la planche est reconstruite …`.
- `render()` : bloc `/* --- assemblage : dimensions de la planche --- */` (cible `sp = $('plancheFp').style`), clamp `pages`, écriture `$('dos').style.backgroundColor`.
- CSS planche : bloc `/* ---------- assemblage : planche 4ème + dos + 1ère ---------- */` (`.planche-fp`, `.planche`, `.dos`) ; le commentaire y annonce déjà la neutralisation à l'export.
- Export 1ère : `/* ================= export PNG ================= */`, `freezeArtGeometry`, `$('btnPng')`, `pickSaveFile`, `saveBlob`, `slug`.
- Panneau : `fieldset id="fsAsm"` (prestataire, pages) ; CDN : `<script src="…html2canvas…">` dans le `<head>`.
- Constantes utiles déjà en place : `format` (mm), `fr(v,d)`, `PROVIDERS`, `$()`.
- Sondes : les `const`/`function` top-level du script classique sont accessibles depuis `browser_evaluate` (portée lexicale globale) — les sondes des lots 1-2 s'en servaient déjà.
- **`box-sizing:border-box` est global** : les bordures 1px du `.dos` sont comprises dans sa largeur — les arrêts du dégradé de fond perdu tombent exactement sur les frontières des panneaux.

---

### Tâche 1 : `buildPlanche` paramétrable en échelle

Le calcul écran (`stage.clientWidth`) devient le défaut d'un paramètre `cwPl`, pour pouvoir reconstruire la planche à l'échelle d'export. Le clamp `Math.min(cwUne, …)` ne s'applique qu'au chemin écran : à l'export, `cwPl` dépasse `cwUne` et c'est voulu.

**Files :** Modify `index.html` (fonction `buildPlanche`)

- [x] **Step 1 : paramètre optionnel**

Remplacer le début de `buildPlanche` (jusqu'à la ligne `$('plancheFp').style.setProperty('--cw', cwPl + 'px');` incluse) par :

```js
function buildPlanche(P, dosMm, cwPl){
  const largeurMm = 2 * format[0] + dosMm + 2 * P.fondPerdu;
  const cwUne = parseFloat(getComputedStyle(cover).getPropertyValue('--cw')) || 340;
  if (cwPl === undefined) { /* échelle écran : la planche tient dans la scène */
    const stage = document.querySelector('.stage');
    const MARGE_SCENE = 160; // respiration de la scène autour de la planche, gouttières comprises
    cwPl = Math.max(120, Math.min(cwUne, (stage.clientWidth - MARGE_SCENE) * format[0] / largeurMm));
  }
  $('plancheFp').style.setProperty('--cw', cwPl + 'px');
```

Le reste de la fonction (clones, remise à l'échelle du cadre, `replaceChildren`) est inchangé — `const scale = cwPl / cwUne;` fonctionne tel quel avec le paramètre.

- [x] **Step 2 : syntaxe**

Run : la commande d'extraction + `node --check` de l'en-tête. Expected : `Syntaxe OK`.

- [x] **Step 3 : sonde de non-régression écran**

`serve.sh`, puis dans la page (onglet Assemblage ouvert au préalable via un clic sur le bouton `data-tab="assemblage"`) :

```js
/* browser_evaluate */
() => {
  const w0 = document.getElementById('planche').getBoundingClientRect().width;
  render(); /* repasse par buildPlanche sans argument */
  const w1 = document.getElementById('planche').getBoundingClientRect().width;
  return { w0, w1, egal: Math.abs(w0 - w1) < 0.5 };
}
```

Expected : `egal: true` (la planche écran est identique avant/après le refactor).

- [x] **Step 4 : commit**

```bash
git add index.html
git commit -m "buildPlanche : échelle en paramètre, calcul écran par défaut"
```

---

### Tâche 2 : sonde de faisabilité — `html2canvas` sur ~2810 px (spec §5, à valider en premier)

Aucune modification de `index.html`. Si cette sonde échoue (crash, canvas vide, dimensions fausses, durée prohibitive), **stop** : retour au planificateur avant d'écrire l'export.

**Files :** aucun (sonde navigateur ; captures dans le scratchpad de session)

- [x] **Step 1 : rendu à l'échelle d'export**

Onglet Assemblage ouvert, puis :

```js
/* browser_evaluate (async) */
async () => {
  const P = PROVIDERS[document.getElementById('inAsmProvider').value];
  const pages = Math.min(800, Math.max(32, +document.getElementById('inAsmPages').value || 244));
  const dosMm = P.dos(pages);
  const wMm = 2 * format[0] + dosMm + 2 * P.fondPerdu, hMm = format[1] + 2 * P.fondPerdu;
  const wPx = wMm / 25.4 * 300;               /* largeur cible 300 dpi */
  const cwExport = (wPx / 2) * format[0] / wMm; /* scale 2 => 300 dpi */
  buildPlanche(P, dosMm, cwExport);
  const t0 = performance.now();
  const canvas = await html2canvas(document.getElementById('plancheFp'),
    { scale: 2, backgroundColor: null, logging: false });
  const duree = Math.round(performance.now() - t0);
  const url = canvas.toDataURL('image/png');
  render(); /* planche ramenée à l'échelle écran */
  return { attendu: [Math.round(wPx), Math.round((hMm / 25.4) * 300)],
           obtenu: [canvas.width, canvas.height], duree, apercu: url.length };
}
```

Expected : `obtenu` à ±4 px de `attendu` (≈ 2810 × 2142 pour le poche Lulu 108×175 à 244 pages), `duree` de l'ordre de quelques secondes, pas d'exception.

- [x] **Step 2 : contrôle visuel**

Récupérer le dataURL (relancer la sonde en retournant `url` par morceaux si besoin, ou l'injecter dans un `<img>` ajouté au DOM puis `browser_take_screenshot`), enregistrer la capture dans le scratchpad et vérifier : les trois panneaux présents, textes nets, dos lisible, image de la 1ère présente (sa géométrie exacte sera corrigée en tâche 5 — un cadrage étiré est attendu et toléré ici). Noter le résultat (dimensions, durée) dans la section « Journal des sondes » en bas de ce plan.

---

### Tâche 3 : variables de fond sur `#plancheFp` et clamp des pages (notes de revue lot 2)

**Files :** Modify `index.html` (CSS `.dos`, `render()`, écouteurs)

- [x] **Step 1 : `.dos` peint par variable**

Dans le bloc CSS `.dos{…}`, ajouter la déclaration :

```css
background:var(--dos-bg,#fff);
```

- [x] **Step 2 : `render()` écrit les trois fonds sur `sp`**

Dans le bloc `/* --- assemblage : dimensions de la planche --- */`, **remplacer** la ligne :

```js
  $('dos').style.backgroundColor = $('inDosBgMode').value === 'couleur' ? $('inDosBg').value : $('inPaper').value;
```

par :

```js
  sp.setProperty('--dos-bg', $('inDosBgMode').value === 'couleur' ? $('inDosBg').value : $('inPaper').value);
  sp.setProperty('--q4-bg', $('inQ4BgMode').value === 'couleur' ? $('inQ4Bg').value : $('inPaper').value);
  sp.setProperty('--une-bg', $('inPaper').value);
```

(`--q4-bg` et `--une-bg` ne servent qu'au remplissage du fond perdu à l'export, tâche 4.)

Attention : l'ancienne écriture directe a pu laisser un `background-color` inline sur `#dos` dans des sessions sauvegardées — non : ce style n'est pas sérialisé (seuls les contrôles `inXxx` le sont), rien à migrer.

- [x] **Step 3 : clamp réécrit dans le champ au `change`**

Près des autres écouteurs spécifiques (ancre : l'écouteur de `inFormat`), ajouter :

```js
/* le clamp de render() borne le calcul ; au blur, refléter la borne dans le champ */
$('inAsmPages').addEventListener('change', e => {
  const v = Math.min(800, Math.max(32, +e.target.value || 244));
  if (+e.target.value !== v) e.target.value = v;
});
```

(Au `change` et pas à l'`input` : réécrire pendant la frappe transformerait « 3 » en « 32 » sous les doigts.)

- [x] **Step 4 : syntaxe + sonde**

`node --check` (extraction de l'en-tête), puis sonde :

```js
/* browser_evaluate */
() => {
  const dos = document.getElementById('dos');
  const bg = getComputedStyle(dos).backgroundColor;
  const p = document.getElementById('inAsmPages');
  p.value = 5000; p.dispatchEvent(new Event('change', { bubbles: true }));
  return { bg, clampe: p.value };
}
```

Expected : `bg` = la couleur papier de la maquette courante (non transparente), `clampe: "800"`. Vérifier aussi visuellement que le dos garde son fond dans l'onglet Assemblage, et que « couleur distincte » le change.

- [x] **Step 5 : commit**

```bash
git add index.html
git commit -m "Planche : fonds en variables CSS sur plancheFp ; clamp des pages reflété au champ"
```

---

### Tâche 4 : habillage d'export — guides neutralisés, fond perdu rempli

**Files :** Modify `index.html` (CSS planche, nouvelle fonction `preparePlancheClone` près de `freezeArtGeometry`)

- [x] **Step 1 : classe `.export` en CSS**

À la suite du bloc CSS de la planche (après `.dos-texte .esp{flex:1}` et les règles de casse), ajouter :

```css
/* export : la classe .export est posée par preparePlancheClone() sur le clone html2canvas —
   guides écran retirés ; border-color transparent (et pas border:none) pour ne pas changer la géométrie */
.planche-fp.export .planche{outline:none}
.planche-fp.export .dos{border-color:transparent}
```

- [x] **Step 2 : `preparePlancheClone`**

Avant le bloc `/* ================= export PNG ================= */` (ou juste après `freezeArtGeometry`), ajouter :

```js
/* clone html2canvas de la planche : guides écran neutralisés, fond perdu rempli par les
   fonds adjacents (arrêts durs en px résolus — html2canvas ne parse pas calc() dans un gradient) */
function preparePlancheClone(doc){
  const dst = doc.getElementById('plancheFp');
  if (!dst) return;
  dst.classList.add('export');
  const cs = getComputedStyle($('plancheFp'));
  const cw = parseFloat(cs.getPropertyValue('--cw'));
  const x1 = cw * (parseFloat(cs.getPropertyValue('--fp')) + 1);
  const x2 = x1 + cw * parseFloat(cs.getPropertyValue('--dos-larg'));
  const q4 = cs.getPropertyValue('--q4-bg').trim() || '#fff';
  const dosBg = cs.getPropertyValue('--dos-bg').trim() || '#fff';
  const une = cs.getPropertyValue('--une-bg').trim() || '#fff';
  dst.style.background = 'linear-gradient(90deg,' +
    q4 + ' 0,' + q4 + ' ' + x1 + 'px,' +
    dosBg + ' ' + x1 + 'px,' + dosBg + ' ' + x2 + 'px,' +
    une + ' ' + x2 + 'px,' + une + ' 100%)';
  dst.style.boxShadow = 'none';
}
```

- [x] **Step 3 : mettre à jour le commentaire CSS d'en-tête de bloc**

Le commentaire `/* guides et habillage écran seulement … à neutraliser à l'export de la planche (lot 3) … */` devient :

```css
/* guides et habillage écran seulement (hachures, ombre, tirets) : neutralisés à l'export par preparePlancheClone() + .export */
```

- [x] **Step 4 : syntaxe + sonde d'export habillé**

`node --check`, puis reprendre la sonde de la tâche 2 en passant `onclone: preparePlancheClone` à `html2canvas`. Contrôle visuel de la capture : plus de hachures ni de tirets ni d'ombre ; le tour de fond perdu montre la couleur 4ème à gauche, dos au centre, papier à droite, avec des frontières nettes alignées sur les panneaux. Tester aussi avec « couleur distincte » sur le fond du dos et de la 4ème pour voir trois couleurs différentes.

- [x] **Step 5 : commit**

```bash
git add index.html
git commit -m "Export planche : guides neutralisés, fond perdu rempli des fonds adjacents"
```

---

### Tâche 5 : géométrie de l'image de la 1ère dans le clone

`html2canvas` ignore `object-fit`/`object-position` : l'export 1ère passe par `freezeArtGeometry`, qui cible `#elImg` par id — or les clones de la planche n'ont plus d'id. On extrait le calcul en `artFreezeCss(zone)` et on fige l'image du clone de la 1ère depuis `preparePlancheClone`.

**Files :** Modify `index.html` (`freezeArtGeometry` → refactor, `preparePlancheClone`)

- [x] **Step 1 : extraire le calcul**

Remplacer intégralement `freezeArtGeometry` par :

```js
/* html2canvas ignore object-fit/object-position (il étire l'image sur sa boîte) :
   on fige dans le clone la géométrie que le CSS produit à l'écran — cadrage
   (cover/contain), position verticale et zoom — en px, sans transform. */
function artFreezeCss(zone){
  const src = $('elImg');
  if (!src || mode === 'typo') return null;
  const nw = src.naturalWidth, nh = src.naturalHeight;
  if (!nw || !nh || !zone.width || !zone.height) return null;
  const keep = $('inKeepRatio').checked;
  const fit = (keep ? Math.min : Math.max)(zone.width / nw, zone.height / nh);
  const artX = +$('inArtX').value / 100, artY = +$('inArtY').value / 100, zoom = +$('inZoom').value;
  const sx = zoom * (keep ? 1 : +$('inStretch').value); /* déformation repliée dans l'échelle horizontale */
  const dw = nw * fit, dh = nh * fit;
  const left = (zone.width - dw) * artX, top = (zone.height - dh) * artY;
  /* échelle repliée dans la géométrie, autour de l'origine (artX, artY) de la zone */
  const ox = zone.width * artX, oy = zone.height * artY;
  return 'position:absolute;' +
    'left:' + (ox - (ox - left) * sx) + 'px;top:' + (oy - (oy - top) * zoom) + 'px;' +
    'width:' + (dw * sx) + 'px;height:' + (dh * zoom) + 'px;transform:none;';
}
function freezeArtGeometry(doc){
  const dst = doc.getElementById('elImg');
  if (!dst) return;
  const css = artFreezeCss($('art').getBoundingClientRect());
  if (css) dst.style.cssText = css;
}
```

- [x] **Step 2 : figer l'image du clone de la planche**

Dans `preparePlancheClone`, avant la ligne `dst.style.boxShadow = 'none';`, ajouter :

```js
  /* la 1ère est le dernier enfant de #planche (4ème | dos | 1ère) ; ses ids ont été retirés
     par buildPlanche — on la retrouve par position, dans le document vivant comme dans le clone */
  const c1 = $('planche').lastElementChild;
  const img = doc.getElementById('planche') &&
              doc.getElementById('planche').lastElementChild.querySelector('.art img');
  if (c1 && img) {
    const zone = c1.querySelector('.art');
    const css = zone && artFreezeCss(zone.getBoundingClientRect());
    if (css) img.style.cssText = css;
  }
```

- [x] **Step 3 : syntaxe**

`node --check`. Expected : `Syntaxe OK`.

- [x] **Step 4 : sonde — l'export 1ère est inchangé**

La sonde compare l'export 1ère avant/après refactor sans dialogue de fichier : onglet 1ère, mode image (preset Surimpression), puis :

```js
/* browser_evaluate (async) */
async () => {
  const canvas = await html2canvas(cover, { scale: 3, backgroundColor: null, logging: false, onclone: freezeArtGeometry });
  return { w: canvas.width, h: canvas.height, apercu: canvas.toDataURL('image/png').slice(0, 64) };
}
```

Comparer visuellement la capture (dataURL injecté dans un `<img>` + `browser_take_screenshot`) avec l'écran : cadrage/zoom identiques à l'aperçu. Expected : rendu identique à l'export d'avant refactor.

- [x] **Step 5 : sonde — l'image dans la planche est bien cadrée**

Reprendre la sonde de la tâche 4 (export planche avec `onclone: preparePlancheClone`), preset Surimpression (mode image). Contrôle visuel : le cadrage de l'image dans le panneau 1ère de la planche correspond à l'aperçu écran (plus d'étirement).

- [x] **Step 6 : commit**

```bash
git add index.html
git commit -m "Export planche : géométrie d'image figée dans le clone (refactor artFreezeCss)"
```

---

### Tâche 6 : pdf-lib, bouton et flux d'export complet

**Files :** Modify `index.html` (`<head>` CDN, fieldset `fsAsm`, écouteur près de `btnPng`)

- [x] **Step 1 : CDN pdf-lib**

Dans le `<head>`, sous la ligne `<script src="…html2canvas…"></script>` :

```html
<script src="https://cdnjs.cloudflare.com/ajax/libs/pdf-lib/1.17.1/pdf-lib.min.js"></script>
```

Sonde immédiate (page rechargée) : `browser_evaluate` → `() => typeof PDFLib` — Expected : `"object"`.

- [x] **Step 2 : bouton dans le fieldset Assemblage**

Dans `fsAsm`, après le `<p class="note">Le dos est calculé…</p>` :

```html
        <button class="btn" id="btnPlanche">Exporter la planche (PDF 300 dpi)</button>
        <p class="note">PNG 300 dpi encapsulé dans un PDF aux dimensions exactes, fond perdu compris — le fichier attendu par le prestataire.</p>
```

- [x] **Step 3 : flux d'export**

Après l'écouteur de `btnPng` (avant `/* ================= rechargement ================= */`) :

```js
/* ================= export planche (PDF 300 dpi) ================= */
$('btnPlanche').addEventListener('click', async () => {
  if (tab !== 'assemblage') setTab('assemblage'); /* l'export rend la planche : elle doit être construite */
  /* dialogue avant le rendu : l'activation utilisateur expire pendant html2canvas */
  const handle = await pickSaveFile(slug() + '-planche.pdf', 'PDF de la planche', 'application/pdf', '.pdf');
  if (handle === 'aborted') return;
  const btn = $('btnPlanche'), old = btn.textContent;
  btn.textContent = 'Rendu en cours…'; btn.disabled = true;
  try {
    const P = PROVIDERS[$('inAsmProvider').value];
    const pages = Math.min(800, Math.max(32, +$('inAsmPages').value || 244));
    const dosMm = P.dos(pages);
    const wMm = 2 * format[0] + dosMm + 2 * P.fondPerdu, hMm = format[1] + 2 * P.fondPerdu;
    const wPx = wMm / 25.4 * 300;
    buildPlanche(P, dosMm, (wPx / 2) * format[0] / wMm); /* scale 2 => 300 dpi ; jamais scale 1 (ombre peinte) */
    const canvas = await html2canvas($('plancheFp'),
      { scale: 2, backgroundColor: null, logging: false, onclone: preparePlancheClone });
    const blob = await new Promise(r => canvas.toBlob(r, 'image/png'));
    const png = new Uint8Array(await blob.arrayBuffer());
    const MM2PT = 72 / 25.4;
    const pdf = await PDFLib.PDFDocument.create();
    const page = pdf.addPage([wMm * MM2PT, hMm * MM2PT]);
    const image = await pdf.embedPng(png);
    page.drawImage(image, { x: 0, y: 0, width: wMm * MM2PT, height: hMm * MM2PT });
    const bytes = await pdf.save();
    await saveBlob(handle, new Blob([bytes], { type: 'application/pdf' }), slug() + '-planche.pdf');
    status(`PDF ${fr(wMm, 2)} × ${fr(hMm, 2)} mm — ${canvas.width}×${canvas.height} px, ${(bytes.length/1048576).toFixed(2)} Mo`, 'ok');
  } catch (e) {
    status('Export impossible : ' + e.message, 'err');
  }
  btn.textContent = old; btn.disabled = false;
  render(); /* planche reconstruite à l'échelle écran */
});
```

(Le bouton `btnPlanche` n'est pas un contrôle `inXxx` : il ne doit pas être sérialisé, c'est voulu.)

- [x] **Step 4 : syntaxe**

`node --check`. Expected : `Syntaxe OK`.

- [x] **Step 5 : sonde du flux complet**

Dans un contexte Playwright, neutraliser le dialogue natif pour forcer le repli `download()` et intercepter le téléchargement :

```js
/* browser_evaluate, avant le clic */
() => { window.showSaveFilePicker = undefined; }
```

puis cliquer `#btnPlanche` (onglet Assemblage) et récupérer le fichier téléchargé. Vérifications sur le fichier :

```bash
grep -a MediaBox <fichier>.pdf
```

Expected : `MediaBox` = `[0 0 W H]` avec `W = wMm/25.4*72` et `H = hMm/25.4*72` calculés depuis l'état de la page (pour poche Lulu 108×175, 244 p : ≈ `674.17 × 514.03`). Ouvrir le PDF (Aperçu/`open`) : planche complète, nette, fond perdu rempli, dimensions du document en mm exactes (Lire les informations). Vérifier aussi le message de `status` dans la page.

- [x] **Step 6 : commit**

```bash
git add index.html
git commit -m "Assemblage : export PDF 300 dpi de la planche (pdf-lib)"
```

---

### Tâche 7 : vérifications de clôture (CLAUDE.md) et plan coché

**Files :** Modify `docs/superpowers/plans/2026-08-18-lot3-export-pdf.md` (cases cochées, journal des sondes)

- [x] **Step 1 : syntaxe** — extraction + `node --check` une dernière fois.
- [x] **Step 2 : trois presets × trois onglets** — pour Folio, Blanche, Surimpression : les onglets 1ère, 4ème, Assemblage s'affichent sans erreur console (`read_console_messages` / `browser_console_messages`).
- [x] **Step 3 : round-trip métadonnées** — exporter un PNG de la 1ère (réglages embarqués), le recharger, vérifier que les contrôles reviennent à l'identique (aucun contrôle nouveau dans ce lot : le round-trip doit être intact).
- [x] **Step 4 : export 1ère intact** — sonde `html2canvas(cover, {scale: 3, …, onclone: freezeArtGeometry})` sur un preset image : rendu identique à l'aperçu.
- [x] **Step 5 : export planche sur les trois presets** — trois PDF générés, contrôle visuel de chacun (guides absents, fond perdu rempli, dos lisible, image bien cadrée).
- [x] **Step 6 : cocher les cases du plan, remplir le journal des sondes, commit final**

```bash
git add docs/superpowers/plans/2026-08-18-lot3-export-pdf.md
git commit -m "Lot 3 : plan coché, sondes consignées"
```

---

## Journal des sondes

### Tâche 2 — faisabilité `html2canvas` à l'échelle d'export

Attendu 2809×2177 px, obtenu 2810×2178 px (écart 1 px), durée `html2canvas` 118 ms, dataURL ~3,7 Mo. Verdict : viable, risque spec §5 levé.

### Tâche 6 — flux complet (clic réel)

MediaBox `[0 0 674.1717 522.5669]` pt vs attendu 674,1717 × 522,5669 (écart < 0,001 pt ; format 108×178, 244 pages, dos 15,4828 mm), canvas 2810×2178 px, PDF 2,72 Mo, message `status` conforme. Sonde couleurs du fond perdu (rouge/vert/papier) validée sur pixels échantillonnés.

### Tâche 7 — vérifications de clôture

- **Syntaxe** : extraction des blocs `<script>` inline + `node --check` → `Syntaxe OK`.
- **Trois presets × trois onglets** : Folio, Blanche, Surimpression × 1ère, 4ème, Assemblage — 9 vues affichées, 0 erreur console JS (seul le 404 favicon préexistant, ignoré).
- **Round-trip métadonnées** (preset Folio, équivalent programmatique `collectConfig` → JSON → `applyConfig` → `collectConfig`) : 108 clés comparées à plat (hors horodatage `date`, régénéré par `collectConfig`), 0 divergence.
- **Export 1ère intact** (Surimpression, mode image) : canvas 1560×2553 px = 3× l'aperçu écran (520×850,9), durée 117 ms ; cadrage identique à l'aperçu (contrôle visuel côte à côte).
- **Export planche sur les trois presets** (Lulu, 244 pages, dos 15,4828 mm) :
  - Folio : MediaBox `[0 0 674.1717450764879 522.5669291338583]` (= calcul exact), canvas 2810×2178 px, PDF 2,72 Mo ;
  - Blanche : MediaBox `[0 0 855.5890679111337 599.1023622047245]`, canvas 3566×2498 px, PDF 0,10 Mo ;
  - Surimpression : MediaBox `[0 0 685.5103277536533 528.236220472441]`, canvas 2858×2202 px, PDF 3,39 Mo.
  - MediaBox extraits des object streams (zlib) : écart 0 pt sur les trois. Contrôle visuel des trois PNG convertis : guides absents, fond perdu rempli par les fonds adjacents, dos lisible, image non étirée. `render()` rappelé après chaque sonde (retour à l'échelle écran).
- `localStorage['atelier-couverture-session']` restauré à l'identique après les sondes (seule `date` réécrite par le debounce de `render()` au rechargement — comportement nominal).

### Écarts au plan pendant l'exécution

- Tâche 5 : deux retouches post-revue (commit 851501a) — commentaire « ordre porteur » côté `buildPlanche`, garde `p && p.lastElementChild && …` dans `preparePlancheClone` (écart au verbatim du plan, demandé par la revue qualité).

### Dettes notées par les revues (pour le prochain lot)

- Formules `wMm`/`hMm` et clamp pages en trois exemplaires (`render`/`buildPlanche`/`btnPlanche`) — factoriser à la prochaine modification.
- `s4.backgroundColor` reste une écriture directe hors variable CSS.
- Commentaire l.145 « (tâche 4) » ambigu depuis le lot 3.
