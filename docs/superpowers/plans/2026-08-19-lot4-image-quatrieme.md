# Lot 4 — Dette lot 3 et image de fond de la 4ème : plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal :** purger la dette du lot 3 (formules dupliquées, course onclone, écriture directe, commentaire), puis donner à la 4ème un fond image en deux modes — `image` (upload propre, jeu complet de réglages) et `prolongement` (panorama depuis la 1ère, 1ère maître, tranches par panneau).

**Architecture :** un helper `plancheDims()` devient la source unique des dimensions physiques ; l'export capture ses valeurs AVANT `html2canvas` (`capturePlancheExport()` → fermeture `applyPlancheExport(doc, snap)`). Le calcul d'image est factorisé en `artGeom(zone, o)` (pur, nombres en px) ; la 1ère garde son mécanisme CSS + gel inchangé ; la 4ème et le dos sont positionnés **en px inline par `render()`/`buildPlanche`** dans tous les cas (un seul chemin écran/export ; les px des clones sont remis à l'échelle par la boucle existante de `buildPlanche`, le dos est placé par `buildPlanche` qui connaît `cwPl`). `render()` reste l'unique écrivain de styles ; `buildPlanche` garde son statut d'exception établie pour la planche.

**Tech stack :** fichier unique `index.html`, aucune dépendance nouvelle.

**Spécification :** `docs/superpowers/specs/2026-08-19-image-quatrieme-et-chaine-design.md` (volet app, §1-2).

**Vérification (pas de framework de test) :** `node --check` sur le JS extrait + sondes navigateur (Playwright MCP) sur l'URL de `/Users/jean-paulgavini/.claude/scripts/serve.sh` (jamais `python3 -m http.server`). Extraction :

```bash
node -e "
const html = require('fs').readFileSync('index.html','utf8');
const blocks = [...html.matchAll(/<script(?![^>]*src)[^>]*>([\s\S]*?)<\/script>/g)].map(m=>m[1]);
require('fs').writeFileSync('/tmp/ozalid-extrait.js', blocks.join('\n;\n'));
" && node --check /tmp/ozalid-extrait.js && echo "Syntaxe OK"
```

**Repères** (ancres textuelles ; **ne jamais lire `index.html` en entier** — ligne base64 géante ~90 Ko) :
- `buildPlanche` : commentaire `/* ---------- assemblage : la planche est reconstruite …` ; la boucle de remise à l'échelle du cadre cible `.frame,.frame-r1,.frame-r2` ; `replaceChildren(c4, $('dos'), c1)` porte le commentaire « ordre porteur ».
- `render()` : bloc `/* --- assemblage : dimensions de la planche --- */` (clamp pages, `sp`), bloc `/* --- image : position selon le mode --- */` (géométrie de `#art` : mode `band` → `top = bandH%`, insets `inInset ? padX% : 0` ; sinon 0), bloc `/* --- voile --- */` (objet `G` des dégradés), objet `R` des lectures.
- Export : `artFreezeCss` + `freezeArtGeometry`, `preparePlancheClone`, écouteur `$('btnPlanche')`, `pickSaveFile`/`saveBlob`/`slug`/`status`/`fr`.
- Images : `$('inFile')` (FileReader → `elImg.src`), `shrinkSource(maxDim = 1600)` (lit `$('elImg')` en dur — à paramétrer), `DEFAULT_IMG`, bloc localStorage (`cfg.image` conditionnel) dans la sauvegarde de session, `applyConfig` (`c.image`).
- Panneau : `fieldset id="fsQ4Fond"` (contient `inQ4BgMode` — 2 options aujourd'hui), `fsImage` (modèle à dupliquer), `applyInspector()` (liste `['fsQ4Fond','fsQ4Texte','fsQ4Pied','fsQ4Isbn']`).
- Persistance : `collectConfig` ignore les `type === 'file'` ; `DEFAULTS = collectConfig().fields` est un instantané au démarrage → tout nouveau contrôle `inXxx` présent dans le HTML est automatiquement couvert (défauts + round-trip).
- `#cover4` : enfants actuels `q4-texte`, `q4-pied`, `q4-isbn` ; `#dos` : enfant `dos-texte`.
- CSS : `%` de top/bottom se rapportent à la HAUTEUR du bloc conteneur, left/right à sa largeur — en tenir compte en répliquant la géométrie de `#art` en px.
- SESSION : chaque sonde relève `localStorage['atelier-couverture-session']` avant manipulation et le restaure à l'identique à la fin.

---

### Tâche 1 : `plancheDims()` — source unique des dimensions (dette a)

**Files :** Modify `index.html` (`buildPlanche`, `render()`, écouteur `btnPlanche`)

- [x] **Step 1 : le helper**

Juste avant `buildPlanche`, ajouter :

```js
/* dimensions physiques de la planche : source unique pour render() et l'export */
function plancheDims(){
  const P = PROVIDERS[$('inAsmProvider').value];
  const pages = Math.min(800, Math.max(32, +$('inAsmPages').value || 244));
  const dosMm = P.dos(pages);
  return { P, dosMm,
           wMm: 2 * format[0] + dosMm + 2 * P.fondPerdu,
           hMm: format[1] + 2 * P.fondPerdu };
}
```

- [x] **Step 2 : `buildPlanche(D, cwPl)`**

Changer la signature `function buildPlanche(P, dosMm, cwPl)` en `function buildPlanche(D, cwPl)` ; dans le corps, remplacer la ligne `const largeurMm = 2 * format[0] + dosMm + 2 * P.fondPerdu;` par `const largeurMm = D.wMm;`. Rien d'autre ne référence `P`/`dosMm` dans la fonction — vérifier par lecture.

- [x] **Step 3 : `render()` consomme `plancheDims()`**

Dans le bloc `/* --- assemblage : dimensions de la planche --- */`, remplacer les lignes qui calculent `P`, `pages`, `dosMm` par `const D = plancheDims();`, puis adapter les usages : `sp.setProperty('--dos-larg', D.dosMm / format[0]); … sp.setProperty('--fp', D.P.fondPerdu / format[0]);` ; le texte `plancheDims` devient :

```js
  $('plancheDims').textContent = 'Planche ' + fr(D.wMm, 2) + ' × ' + fr(D.hMm, 2) +
    ' mm — dos ' + fr(D.dosMm, 2) + ' mm — fond perdu ' + fr(D.P.fondPerdu, 3) + ' mm';
```

l'appel devient `if (tab === 'assemblage') buildPlanche(D);` et la lecture `vDosMm` dans `R` devient `[D.dosMm, 2, ' mm']`.

- [x] **Step 4 : `btnPlanche` consomme `plancheDims()`**

Dans l'écouteur, remplacer les cinq lignes `const P = …` à `const wPx = …` par :

```js
    const D = plancheDims();
    const wPx = D.wMm / 25.4 * 300;
    buildPlanche(D, (wPx / 2) * format[0] / D.wMm); /* scale 2 => 300 dpi ; jamais scale 1 (ombre peinte) */
```

et adapter les usages suivants (`wMm` → `D.wMm`, `hMm` → `D.hMm`) dans la création de page PDF et le message `status`.

- [x] **Step 5 : syntaxe + sonde de non-régression**

`node --check` (en-tête). Puis sonde : onglet Assemblage, relever `$('plancheDims').textContent` et la largeur de `#planche` — identiques à avant le refactor (valeurs de référence : les relever sur `git stash` ou simplement vérifier « Planche 237,83 × 184,35 mm — dos 15,48 mm » pour le folio 108×178 à 244 pages). Rejouer aussi un export planche par reproduction du flux (mêmes appels que `btnPlanche`) et vérifier le MediaBox inchangé.

- [x] **Step 6 : commit**

```bash
git add index.html
git commit -m "plancheDims : dimensions de la planche en un seul endroit"
```

---

### Tâche 2 : capture pré-rendu de l'export (dette b)

`preparePlancheClone` lit le DOM vivant depuis `onclone` (frontière macrotâche) : un `render()` intercalé fausserait l'export. On coupe en deux : capture synchrone juste après `buildPlanche`, application pure sur le clone.

**Files :** Modify `index.html` (`preparePlancheClone` → deux fonctions, `btnPlanche`, commentaires CSS)

- [x] **Step 1 : remplacer `preparePlancheClone` par capture + application**

Remplacer intégralement la fonction (commentaire compris) par :

```js
/* export de la planche : tout ce que le clone devra recevoir est capturé sur le
   document vivant JUSTE APRÈS buildPlanche (synchrone) — onclone n'applique que
   des valeurs figées, un render() intercalé pendant la création du clone ne peut
   plus fausser l'export */
function capturePlancheExport(){
  const cs = getComputedStyle($('plancheFp'));
  const cw = parseFloat(cs.getPropertyValue('--cw'));
  const x1 = cw * (parseFloat(cs.getPropertyValue('--fp')) + 1);
  const x2 = x1 + cw * parseFloat(cs.getPropertyValue('--dos-larg'));
  const q4 = cs.getPropertyValue('--q4-bg').trim() || '#fff';
  const dosBg = cs.getPropertyValue('--dos-bg').trim() || '#fff';
  const une = cs.getPropertyValue('--une-bg').trim() || '#fff';
  /* la 1ère est le dernier enfant de #planche (4ème | dos | 1ère) ; ses ids ont été retirés
     par buildPlanche — on la retrouve par position, dans le document vivant comme dans le clone */
  const c1 = $('planche').lastElementChild;
  const zone = c1 && c1.querySelector('.art');
  return {
    fond: 'linear-gradient(90deg,' + q4 + ' 0,' + q4 + ' ' + x1 + 'px,' +
      dosBg + ' ' + x1 + 'px,' + dosBg + ' ' + x2 + 'px,' +
      une + ' ' + x2 + 'px,' + une + ' 100%)',
    artCss: zone ? artFreezeCss(zone.getBoundingClientRect()) : null
  };
}
function applyPlancheExport(doc, snap){
  const dst = doc.getElementById('plancheFp');
  if (!dst) return;
  dst.classList.add('export');
  dst.style.background = snap.fond;
  dst.style.boxShadow = 'none';
  const p = doc.getElementById('planche');
  const img = p && p.lastElementChild && p.lastElementChild.querySelector('.art img');
  if (img && snap.artCss) img.style.cssText = snap.artCss;
}
```

- [x] **Step 2 : `btnPlanche` câble la fermeture**

Après l'appel `buildPlanche(D, …);`, ajouter `const snap = capturePlancheExport();` et remplacer `onclone: preparePlancheClone` par `onclone: doc => applyPlancheExport(doc, snap)`.

- [x] **Step 3 : commentaires**

Mettre à jour les deux commentaires CSS qui citent `preparePlancheClone` (en-tête du bloc planche et bloc `.planche-fp.export`) pour citer `capturePlancheExport()/applyPlancheExport()`. Vérifier par grep qu'aucune référence à `preparePlancheClone` ne subsiste.

- [x] **Step 4 : syntaxe + sonde**

`node --check`, puis export planche par reproduction du flux (avec la capture) : rendu identique à la tâche précédente (dimensions canvas, contrôle visuel rapide, fond perdu rempli).

- [x] **Step 5 : commit**

```bash
git add index.html
git commit -m "Export planche : valeurs capturées avant le rendu, onclone sans lecture du vivant"
```

---

### Tâche 3 : `--paper` sur `#cover4` et commentaire toiletté (dette c, d)

**Files :** Modify `index.html` (`render()`, commentaire CSS)

- [x] **Step 1 : fond de la 4ème par variable**

Dans `render()`, remplacer `s4.backgroundColor = $('inQ4BgMode').value === 'couleur' ? $('inQ4Bg').value : $('inPaper').value;` par :

```js
  s4.setProperty('--paper', $('inQ4BgMode').value === 'couleur' ? $('inQ4Bg').value : $('inPaper').value);
```

(`.cover{background:var(--paper)}` existe déjà : `#cover4` porte la classe `cover`, sa variable inline prime sur celle héritée — vérifier qu'aucun style inline `background-color` résiduel ne reste posé sur `#cover4` au chargement d'une session antérieure : l'ancienne écriture n'était pas sérialisée, rien à migrer, mais le confirmer par grep `backgroundColor`.)

- [x] **Step 2 : commentaire l.~145**

Le commentaire CSS `/* Les variables … --cw est posée sur #plancheFp par buildPlanche() (tâche 4) — d'ici là les fallbacks s'appliquent. */` perd sa référence de tâche : `… par buildPlanche() — d'ici là les fallbacks s'appliquent. */`

- [x] **Step 3 : syntaxe + sonde + commit**

`node --check` ; sonde : fond de la 4ème correct dans les deux modes (`herite`/`couleur`) à l'écran ET dans la planche (le clone copie la variable inline). Commit :

```bash
git add index.html
git commit -m "4ème : fond papier par variable CSS ; commentaire planche toiletté"
```

---

### Tâche 4 : DOM et contrôles du fond image (sans rendu)

**Files :** Modify `index.html` (HTML `#cover4`/`#dos`/panneau, CSS, `applyInspector`)

- [x] **Step 1 : couches image dans `#cover4`**

Dans `#cover4`, AVANT `<div class="q4-texte" …>`, insérer :

```html
        <div class="art hide" id="art4"><img id="elImg4" alt=""><img id="elImg4P" alt=""></div>
        <div class="scrim hide" id="scrim4"></div>
```

(`elImg4` = image propre uploadée ; `elImg4P` = tranche du prolongement, src synchronisé sur la 1ère — deux éléments pour ne jamais écraser l'upload propre. Un seul est visible à la fois, géré par `render()` à la tâche 5/6.)

- [x] **Step 2 : tranche du dos**

Dans `#dos`, AVANT `<div class="dos-texte" …>`, insérer :

```html
<div class="dos-art hide" id="dosArt"><img id="elImgDos" alt=""></div>
```

et ajouter au CSS, dans le bloc planche (après `.dos{…}`) :

```css
.dos-art{position:absolute;left:0;right:0;overflow:hidden}
.dos-art img{position:absolute}
```

(le conteneur `dosArt` réplique le rôle de zone clippante de `.art` — indispensable en mode bandeau pour que la tranche du dos s'arrête à la hauteur de la bande ; `top`/`height` posés en px par `buildPlanche`. Le `.dos-texte` déjà présent passe au-dessus par ordre de document + `position:absolute`.)

- [x] **Step 3 : sélecteur de fond étendu**

Dans `fsQ4Fond`, le `<select id="inQ4BgMode">` reçoit deux options après `couleur` :

```html
<option value="image">image propre</option><option value="prolongement">prolongement de la 1ère</option>
```

- [x] **Step 4 : fieldset `fsQ4Image`**

Après la fermeture `</fieldset>` de `fsQ4Fond`, insérer (miroir de `fsImage`, variantes Q4, mêmes bornes) :

```html
      <fieldset id="fsQ4Image" class="off">
        <legend>Image de la 4ème</legend>
        <label id="rowQ4File"><span class="lab">Choisir l'image</span><input type="file" id="inQ4File" accept="image/*"></label>
        <p class="note hide" id="noteQ4NoImg">Aucune image chargée : le fond reste papier en attendant.</p>
        <p class="note hide" id="noteQ4Pro">Cadrage piloté par la 1ère (prolongement) ; seule la 4ème règle son voile.</p>
        <p class="note hide" id="noteQ4Manque">La photo manque de matière à gauche : le papier apparaît sur la 4ème.</p>
        <div id="rowsQ4Cadrage">
          <label><span class="lab">Cadrage vertical <span class="val" id="vQ4ArtY">50 %</span></span>
            <input type="range" id="inQ4ArtY" min="0" max="100" step="1" value="50"></label>
          <label><span class="lab">Cadrage horizontal <span class="val" id="vQ4ArtX">50 %</span></span>
            <input type="range" id="inQ4ArtX" min="0" max="100" step="1" value="50"></label>
          <label><span class="lab">Échelle <span class="val" id="vQ4Zoom">1,00</span></span>
            <input type="range" id="inQ4Zoom" min="1" max="2.2" step="0.01" value="1"></label>
          <div class="check"><input type="checkbox" id="inQ4KeepRatio"><span>Conserver les proportions de l'image (sans recadrage)</span></div>
          <label id="rowQ4Stretch"><span class="lab">Déformation horizontale <span class="val" id="vQ4Stretch">1,00</span></span>
            <input type="range" id="inQ4Stretch" min="0.5" max="2" step="0.01" value="1"></label>
        </div>
        <label><span class="lab">Voile de lisibilité</span>
          <select id="inQ4Scrim">
            <option value="none" selected>Aucun</option>
            <option value="top">Assombrir le haut</option>
            <option value="bottom">Assombrir le bas</option>
            <option value="both">Assombrir haut et bas</option>
            <option value="flat">Voile uni sombre</option>
            <option value="light">Voile uni clair</option>
          </select></label>
        <label><span class="lab">Intensité du voile <span class="val" id="vQ4Scrim">55 %</span></span>
          <input type="range" id="inQ4ScrimOp" min="0" max="100" step="1" value="55"></label>
      </fieldset>
```

- [x] **Step 5 : visibilité**

Dans `applyInspector()`, la liste `['fsQ4Fond','fsQ4Texte','fsQ4Pied','fsQ4Isbn']` devient `['fsQ4Fond','fsQ4Image','fsQ4Texte','fsQ4Pied','fsQ4Isbn']`. (Le masquage fin — fieldset caché hors modes image/prolongement, lignes de cadrage cachées en prolongement — arrive à la tâche 5 dans `render()`, comme `fsImage`/`mode typo`.)

- [x] **Step 6 : upload et rechargement d'image**

À côté de l'écouteur `$('inFile')` existant, ajouter (même idiome) :

```js
$('inQ4File').addEventListener('change', e => {
  const f = e.target.files[0]; if (!f) return;
  const r = new FileReader();
  r.onload = () => { $('elImg4').src = r.result; };
  r.readAsDataURL(f);
});
for (const id of ['elImg4', 'elImg4P', 'elImgDos']) $(id).addEventListener('load', () => render());
```

(Vérifier comment `elImg` déclenche son re-rendu au chargement — s'il a déjà un écouteur `load`, suivre le même motif ; sinon celui-ci suffit pour les nouveaux éléments.)

- [x] **Step 7 : syntaxe + sonde + commit**

`node --check` ; sonde : onglet 4ème → le fieldset apparaît (encore inerte), aucune erreur console, aucune régression sur les trois presets × trois onglets (les nouveaux contrôles ont leurs valeurs par défaut, `DEFAULTS` les capture au démarrage — vérifier `DEFAULTS.inQ4ArtX === '50'` en console). Commit :

```bash
git add index.html
git commit -m "4ème : couches image, tranche du dos et contrôles (inertes)"
```

---

### Tâche 5 : rendu du mode « image propre »

**Files :** Modify `index.html` (`artFreezeCss` → `artGeom` + wrapper, bloc voile factorisé, `render()`, `buildPlanche`, `PRESETS`)

- [x] **Step 1 : extraire `artGeom` (calcul pur, formules STRICTEMENT identiques)**

Remplacer `artFreezeCss` par :

```js
/* géométrie d'une image posée dans une zone : mêmes formules pour la 1ère (gel
   d'export), l'image propre de la 4ème et les tranches du prolongement */
function artGeom(zone, o){
  if (!o.nw || !o.nh || !zone.w || !zone.h) return null;
  const fit = (o.keep ? Math.min : Math.max)(zone.w / o.nw, zone.h / o.nh);
  const sx = o.zoom * (o.keep ? 1 : o.stretch); /* déformation repliée dans l'échelle horizontale */
  const dw = o.nw * fit, dh = o.nh * fit;
  const left = (zone.w - dw) * o.artX, top = (zone.h - dh) * o.artY;
  /* échelle repliée dans la géométrie, autour de l'origine (artX, artY) de la zone */
  const ox = zone.w * o.artX, oy = zone.h * o.artY;
  return { left: ox - (ox - left) * sx, top: oy - (oy - top) * o.zoom,
           width: dw * sx, height: dh * o.zoom };
}
const artCss = g => g && ('position:absolute;left:' + g.left + 'px;top:' + g.top +
  'px;width:' + g.width + 'px;height:' + g.height + 'px;transform:none;');
/* réglages de la 1ère, sous la forme attendue par artGeom */
function artOptsUne(){
  const src = $('elImg');
  return { nw: src.naturalWidth, nh: src.naturalHeight, keep: $('inKeepRatio').checked,
           artX: +$('inArtX').value / 100, artY: +$('inArtY').value / 100,
           zoom: +$('inZoom').value, stretch: +$('inStretch').value };
}
function artFreezeCss(zone){
  if (mode === 'typo') return null;
  return artCss(artGeom({ w: zone.width, h: zone.height }, artOptsUne()));
}
```

(`freezeArtGeometry` et `capturePlancheExport` continuent d'appeler `artFreezeCss(rect)` sans changement. Contrainte forte : comparer terme à terme avec l'ancienne version — seul le découpage bouge, pas une formule.)

- [x] **Step 2 : factoriser le dégradé de voile**

Sortir l'objet `G` du bloc `/* --- voile --- */` en fonction :

```js
function scrimCss(kind, op){
  return {
    none:  'none',
    top:   `linear-gradient(to bottom, rgba(0,0,0,${op}) 0%, rgba(0,0,0,0) 55%)`,
    bottom:`linear-gradient(to top, rgba(0,0,0,${op}) 0%, rgba(0,0,0,0) 55%)`,
    both:  `linear-gradient(to bottom, rgba(0,0,0,${op}) 0%, rgba(0,0,0,0) 40%, rgba(0,0,0,0) 60%, rgba(0,0,0,${op}) 100%)`,
    flat:  `linear-gradient(rgba(0,0,0,${op}), rgba(0,0,0,${op}))`,
    light: `linear-gradient(rgba(255,255,255,${op}), rgba(255,255,255,${op}))`
  }[kind];
}
```

et faire consommer le bloc voile existant : `scrim.style.background = scrimCss(kind, op);`.

- [x] **Step 3 : bloc de rendu 4ème dans `render()`**

Après le bloc q4 existant (après l'écriture de `--paper`, tâche 3), ajouter :

```js
  /* --- 4ème : fond image (propre ou prolongement) --- */
  const q4m = $('inQ4BgMode').value;
  const art4 = $('art4'), img4 = $('elImg4'), img4P = $('elImg4P'), dosArt = $('dosArt');
  const cw4 = parseFloat(getComputedStyle(cover4).getPropertyValue('--cw')) || 340;
  const h4 = cw4 * format[1] / format[0];
  $('fsQ4Image').classList.toggle('hide', q4m !== 'image' && q4m !== 'prolongement');
  $('rowsQ4Cadrage').classList.toggle('hide', q4m !== 'image');
  $('rowQ4File').classList.toggle('hide', q4m !== 'image');
  $('noteQ4Pro').classList.toggle('hide', q4m !== 'prolongement');
  let g4 = null;
  if (q4m === 'image') {
    art4.style.top = art4.style.left = art4.style.right = art4.style.bottom = '0';
    g4 = artGeom({ w: cw4, h: h4 },
      { nw: img4.naturalWidth, nh: img4.naturalHeight, keep: $('inQ4KeepRatio').checked,
        artX: +$('inQ4ArtX').value / 100, artY: +$('inQ4ArtY').value / 100,
        zoom: +$('inQ4Zoom').value, stretch: +$('inQ4Stretch').value });
    if (g4) img4.style.cssText = artCss(g4);
  }
  $('noteQ4NoImg').classList.toggle('hide', q4m !== 'image' || !!g4);
  img4.classList.toggle('hide', q4m !== 'image' || !g4);
  const proOn = false; /* tranches du prolongement : tâche 6 */
  img4P.classList.add('hide'); dosArt.classList.add('hide');
  $('noteQ4Manque').classList.add('hide');
  art4.classList.toggle('hide', !(q4m === 'image' && g4) && !proOn);
  const op4 = +$('inQ4ScrimOp').value / 100, kind4 = $('inQ4Scrim').value;
  $('scrim4').style.background = scrimCss(kind4, op4) || 'none';
  $('scrim4').classList.toggle('hide', kind4 === 'none' || art4.classList.contains('hide'));
```

et compléter l'objet `R` : `vQ4ArtY:[$('inQ4ArtY').value,0,' %'], vQ4ArtX:[$('inQ4ArtX').value,0,' %'], vQ4Zoom:[$('inQ4Zoom').value,2,''], vQ4Stretch:[$('inQ4Stretch').value,2,''], vQ4Scrim:[$('inQ4ScrimOp').value,0,' %'],`

- [x] **Step 4 : remise à l'échelle dans la planche**

Dans `buildPlanche`, la boucle de remise à l'échelle des px inline (aujourd'hui `c1.querySelectorAll('.frame,.frame-r1,.frame-r2')`) s'applique aussi aux images en px du clone de la 4ème : ajouter après elle :

```js
  c4.querySelectorAll('.art img').forEach(el => {
    for (const prop of [...el.style]) {
      const v = el.style.getPropertyValue(prop);
      if (v.endsWith('px')) el.style.setProperty(prop, parseFloat(v) * scale + 'px');
    }
  });
```

- [x] **Step 5 : presets**

Dans chacune des trois entrées de `PRESETS`, à côté des clés `inQ4…` existantes, ajouter :

```js
    inQ4ArtX:50, inQ4ArtY:50, inQ4Zoom:1, inQ4KeepRatio:false, inQ4Stretch:1,
    inQ4Scrim:'none', inQ4ScrimOp:55,
```

- [x] **Step 6 : syntaxe + sondes**

`node --check`. Sondes (session sauvegardée/restaurée) : (1) onglet 4ème, mode `image`, injecter une image de test (`$('elImg4').src = $('elImg').src` en console de sonde pour éviter un upload), vérifier cadrage/zoom/voile réactifs et cohérents visuellement (capture) ; (2) l'export 1ère est inchangé (gel intact — capture scale 3 comparée à l'écran) ; (3) la planche écran montre l'image de la 4ème à la bonne échelle (capture) ; (4) sans image chargée : fond papier + note visible ; (5) trois presets sans changement d'aspect (mode `herite`).

- [x] **Step 7 : commit**

```bash
git add index.html
git commit -m "4ème : mode image propre (artGeom factorisé, voile, presets)"
```

---

### Tâche 6 : rendu du mode « prolongement »

**Files :** Modify `index.html` (`render()` — bloc 4ème, `buildPlanche` — tranche du dos)

- [x] **Step 1 : géométrie partagée du panorama**

Avant `render()` (près de `plancheDims`), ajouter :

```js
/* panorama « prolongement » : la zone image de la 1ère (position, hauteur, insets)
   répliquée à l'identique, et la géométrie de l'image dans cette zone, à l'échelle
   cw donnée. Retourne null si la 1ère n'a pas d'image (mode typo ou non chargée). */
function panoGeom(cw){
  if (mode === 'typo') return null;
  const h = cw * format[1] / format[0];
  const inset = $('inInset').checked ? +$('inPadX').value / 100 : 0;
  const top = mode === 'band' ? h * +$('inBand').value / 100 : 0;
  const left = mode === 'band' ? cw * inset : 0;
  const bottom = mode === 'band' ? h * inset : 0;
  const zone = { w: cw - 2 * left, h: h - top - bottom };
  const g = artGeom(zone, artOptsUne());
  if (!g) return null;
  const D = plancheDims();
  return { g, zoneTop: top, zoneLeft: left, zoneH: zone.h, zoneW: zone.w,
           dxDos: cw * (D.dosMm / format[0]),        /* décalage 1ère → dos */
           dx4: cw * (D.dosMm / format[0] + 1) };    /* décalage 1ère → 4ème */
}
```

- [x] **Step 2 : la tranche de la 4ème dans `render()`**

Dans le bloc 4ème (tâche 5), remplacer les lignes `const proOn = false; … $('noteQ4Manque').classList.add('hide');` par :

```js
  let proOn = false, pano = null;
  if (q4m === 'prolongement') {
    pano = panoGeom(cw4);
    if (pano) {
      proOn = true;
      const src1 = $('elImg').src;
      if (img4P.src !== src1) img4P.src = src1;
      if ($('elImgDos').src !== src1) $('elImgDos').src = src1;
      art4.style.top = pano.zoneTop + 'px'; art4.style.bottom = '';
      art4.style.left = pano.zoneLeft + 'px'; art4.style.right = '';
      art4.style.width = pano.zoneW + 'px'; art4.style.height = pano.zoneH + 'px';
      img4P.style.cssText = artCss({ ...pano.g, left: pano.g.left + pano.dx4 });
      /* tranche du dos : positionnée par buildPlanche, qui connaît l'échelle de la planche */
    }
  } else { art4.style.width = art4.style.height = ''; }
  img4P.classList.toggle('hide', !proOn);
  if (!proOn) dosArt.classList.add('hide'); /* en prolongement, c'est buildPlanche qui la montre */
  $('noteQ4Manque').classList.toggle('hide', !(proOn && pano.g.left + pano.dx4 > 0));
```

et adapter la ligne d'affichage de `art4` : `art4.classList.toggle('hide', !(q4m === 'image' && g4) && !proOn);` (déjà en place — vérifier). Le mode `image` de la tâche 5 remet `top/left/right/bottom = '0'` et doit maintenant aussi vider `width/height` (`art4.style.width = art4.style.height = '';`) — l'ajouter à la branche `image`.

- [x] **Step 3 : la tranche du dos dans `buildPlanche`**

Après la boucle de remise à l'échelle du clone `c4`, ajouter :

```js
  /* tranche du prolongement sur le dos : le dos est un élément vivant de la planche,
     sa zone et son image se calculent directement à l'échelle cwPl */
  const dosArt = $('dosArt');
  if ($('inQ4BgMode').value === 'prolongement') {
    const pano = panoGeom(cwPl);
    if (pano) {
      dosArt.style.top = pano.zoneTop + 'px';
      dosArt.style.height = pano.zoneH + 'px';
      $('elImgDos').style.cssText = artCss({ ...pano.g, left: pano.g.left + pano.dxDos });
      dosArt.classList.remove('hide');
    } else dosArt.classList.add('hide');
  } else dosArt.classList.add('hide');
```

Attention : le clone `c4` a été fabriqué avec les px écran (`cw4`) puis remis à l'échelle par la boucle — cohérent. Le dos, lui, est calculé directement à `cwPl` : pas de double échelle. La zone `dosArt` clippe la tranche à la hauteur de la bande en mode bandeau (même rôle que `.art` sur les couvertures) ; l'image y est positionnée relativement à la zone, comme sur la 4ème. En mode `image` de la tâche 5 et hors prolongement, `dosArt` reste cachée (le `render()` de la tâche 6 la cache déjà via `dosImg`/`dosArt` — harmoniser : c'est `dosArt` qu'on montre/cache, l'`img` n'a plus de classe `hide` propre ; adapter les lignes correspondantes des tâches 4-6 : l'écouteur `load` cible toujours `elImgDos`, mais les `classList` du bloc `render()` visent `dosArt`).

- [x] **Step 4 : syntaxe + sondes**

`node --check`. Sondes (preset Surimpression puis preset Folio — les deux géométries de zone, session restaurée ensuite) :
1. Onglet 4ème, `inQ4BgMode = 'prolongement'` : la 4ème montre la continuation gauche de la photo, à la même hauteur de bande (Folio) ou plein fond (Surimpression) — captures.
2. Onglet Assemblage : la planche écran montre le panorama continu 4ème → dos → 1ère, raccords sans couture aux frontières des panneaux (capture, zoom sur les jonctions).
3. `noteQ4Manque` apparaît quand la photo ne couvre pas (zoom 1, photo ~4:3 → matière insuffisante pour ~2,2 largeurs : cas normal) et disparaît en zoomant assez.
4. Mode typo sur la 1ère + prolongement : fond papier, pas d'erreur.
5. L'onglet 1ère est strictement inchangé (1ère maître).

- [x] **Step 5 : commit**

```bash
git add index.html
git commit -m "4ème : prolongement panoramique de la 1ère (tranches 4ème et dos)"
```

---

### Tâche 7 : sérialisation de l'image propre

**Files :** Modify `index.html` (`shrinkSource`, export PNG, sauvegarde locale, `applyConfig`)

- [x] **Step 1 : `shrinkSource` paramétré**

Signature `function shrinkSource(maxDim = 1600)` → `function shrinkSource(img = $('elImg'), maxDim = 1600)` ; le corps utilise le paramètre `img` au lieu de `const img = $('elImg')`. Vérifier par grep que les appels existants (`shrinkSource()`) restent valides.

- [x] **Step 2 : export PNG**

Dans l'écouteur `btnPng`, après la ligne `if ($('inEmbedImg').checked && mode !== 'typo') cfg.image = await shrinkSource();`, ajouter :

```js
      if ($('inEmbedImg').checked && $('inQ4BgMode').value === 'image' && $('elImg4').naturalWidth) {
        const d4 = await shrinkSource($('elImg4'));
        if (d4) cfg.image4 = d4;
      }
```

- [x] **Step 3 : sauvegarde locale**

Dans le bloc localStorage (ancre `DEFAULT_IMG`), après la ligne conditionnelle `cfg.image`, ajouter le pendant :

```js
    if ($('inQ4BgMode').value === 'image' && $('elImg4').naturalWidth) cfg.image4 = await shrinkSource($('elImg4'));
```

(vérifier le contexte : si le bloc supprime `cfg.image` dans un `else`, faire pareil pour `cfg.image4`).

- [x] **Step 4 : restauration**

Dans `applyConfig`, après `if (c.image) $('elImg').src = c.image;` :

```js
  if (c.image4) $('elImg4').src = c.image4; else $('elImg4').removeAttribute('src');
```

(déterminisme : un chargement sans `image4` efface l'image propre — cohérent avec le correctif `DEFAULTS` du lot 2 ; l'`img` sans `src` ne rend rien et `artGeom` renvoie null → fond papier + note).

- [x] **Step 5 : syntaxe + sonde round-trip**

`node --check`. Sonde : mode `image` avec image chargée → `collectConfig`+`cfg.image4` → `applyConfig` → tous les champs identiques ET l'image restaurée (`$('elImg4').naturalWidth > 0`) ; puis round-trip d'une config SANS image4 → `elImg4` vidé. Mesurer la taille de la session localStorage avec les deux images (`localStorage['atelier-couverture-session'].length`) : attendu ≪ 5 Mo (le consigner au journal). Session restaurée à l'identique après sonde.

- [x] **Step 6 : commit**

```bash
git add index.html
git commit -m "4ème : image propre sérialisée (PNG, session, applyConfig)"
```

---

### Tâche 8 : export planche des deux modes à 300 dpi

**Files :** aucun a priori (sondes) — Modify `index.html` seulement si un défaut est trouvé

- [x] **Step 1 : sonde export mode `image`**

Preset Folio, 4ème en mode `image` (image de test injectée), reproduction du flux `btnPlanche` (avec `capturePlancheExport`) → PDF au scratchpad, conversion PNG, contrôle visuel : image de la 4ème nette, bien cadrée, voile rendu, fond perdu correct. MediaBox inchangé.

- [x] **Step 2 : sonde export mode `prolongement`**

Preset Surimpression puis Folio, prolongement actif : PDF → PNG, contrôle des raccords aux jonctions 4ème/dos/1ère à 300 dpi (découper les jonctions au zoom), voile de la 4ème, texte du dos lisible par-dessus la tranche. Vérifier notamment le rendu html2canvas des `<img>` positionnées en px inline (aucun `object-fit` en jeu — le risque est faible, mais c'est le contrôle « rendu à l'export, pas seulement à l'écran » du CLAUDE.md).

- [x] **Step 3 : correctifs éventuels**

Tout défaut trouvé se corrige ici (commit dédié avec sonde re-jouée). Si le rendu est propre du premier coup, consigner les mesures au journal, pas de commit.

---

### Tâche 9 : clôture du lot 4

**Files :** Modify `docs/superpowers/plans/2026-08-19-lot4-image-quatrieme.md`

- [x] **Step 1 : syntaxe** — extraction + `node --check`.
- [x] **Step 2 : trois presets × trois onglets** — aucune erreur console, aucun changement d'aspect des maquettes (mode `herite` par défaut).
- [x] **Step 3 : round-trip complet** — avec et sans `image4` ; export 1ère intact (gel) ; export PNG 1ère relu → contrôles restaurés.
- [x] **Step 4 : exports planche** — les trois presets en `herite` (non-régression lot 3) + un export par nouveau mode.
- [x] **Step 5 : plan coché, journal rempli** (mesures des sondes, taille de session avec deux images, écarts au plan éventuels), commit :

```bash
git add docs/superpowers/plans/2026-08-19-lot4-image-quatrieme.md
git commit -m "Lot 4 : plan coché, sondes consignées"
```

---

## Journal des sondes

- **T1** : texte planche et largeur identiques après factorisation (« Planche 237,83 × 184,35 mm — dos 15,48 mm — fond perdu 3,175 mm » pour le folio 108×178 à 244 pages) ; export 2810×2178.
- **T2** : export bit à bit identique (SHA-256) avec un `render()` parasite intercalé — course éliminée ; preuve du correctif de la dette b.
- **T5** : mode image vérifié (cadrage/zoom/voile) ; remise à l'échelle clone prouvée (283,507 = 567,015/2) ; export 1ère aligné sur le motif capture-avant/application (écart au plan demandé en revue).
- **T6** : géométrie panorama prouvée formellement (revue) ; correctif critique `e872912` (la zone `.art` du clone 4ème n'était pas remise à l'échelle — invisible à l'écran car scale≈1, cassait l'export) ; alignements mesurés Δ ≤ 0.02 px.
- **T7** : round-trip 0 diff ; session avec deux images = 285 799 chars (0,273 Mo) ≪ quota.
- **T8** : défaut trouvé — couture 1 px aux jonctions du dos en prolongement (bordures-guides) ; correctif `d93565c` (overlay `dos-guides` réel, neutralisé à l'export ; deux impasses documentées : overflow padding-box, pseudos matérialisés avant onclone) ; après correctif : 0 colonne blanche/noire pure sur 120 testées, alignement Δ ≤ 0.009 px. Résidu connu : 1 colonne d'antialiasing sous-pixel (~6-16 %) par jonction, frontières flex fractionnaires, préexistant.
- **T9 (clôture)** : `node --check` OK sur le JS extrait. Trois presets × trois onglets : 0 erreur console (hors 404 favicon), aspect inchangé, planches « 237,83 × 184,35 », « 301,83 × 211,35 », « 241,83 × 186,35 mm — dos 15,48 mm ». Round-trip `collectConfig` → `applyConfig` → `collectConfig` : 111 champs, 0 divergence, AVEC image4 (141 687 chars, image restaurée) puis SANS (elImg4 vidé, `noteQ4NoImg` visible) ; round-trip via métadonnées PNG (`pngInsertText`/`pngReadText`) : 0 divergence, `image4` identique caractère à caractère. Export 1ère (scale 3, gel `artFreezeCss` pré-rendu) : 1560×2574 px, haut de la photo mesuré à 29,992 % de la hauteur pour 30 % attendu — cadrage identique à l'aperçu. Exports planche `herite` : folio 2810×2178, blanche 3566×2498, overlay 2858×2202 px (attendus 2809×2177 / 3565×2496 / 2856×2201 — Δ ≤ 2 px, non-régression lot 3). Export mode `image` : 2810×2178, image de la 4ème nette, voile `flat` rendu. Export mode `prolongement` (folio bandeau, zoom 2,2, cadrage horizontal 100 %, `noteQ4Manque` éteinte) : panorama continu 4ème → dos → 1ère, 0 colonne blanche/noire pure et 0 colonne quasi-pure (> 5 %) sur 122 colonnes testées autour des deux jonctions (fenêtre y 55-90 % de la planche). Session localStorage relevée avant sondes (2 499 chars) et restaurée à l'identique.

**Écarts au plan** : step 1bis (export 1ère) ; micro-retouches T6 (`art4On`, `scrimCss` garde interne, commentaires) ; `else` du plan omis en T6 (prouvé sans effet) ; correctifs `e872912` et `d93565c`.

**Limitations connues (pour mémoire)** : en bandeau + retrait latéral coché, la tranche du dos suit la formule du plan (décalage théorique = largeur du retrait, invisible car les marges papier séparent alors les panneaux) ; l'upload de la 4ème n'est sérialisé qu'en mode `image` (sémantique identique à la 1ère en typo) ; la note de l'export PNG ne mentionne pas la photo quand seule `image4` est embarquée.
