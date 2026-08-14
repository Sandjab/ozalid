# Inspecteur contextuel et manipulation directe — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal :** Réorganiser `index.html` (barre d'actions + inspecteur contextuel) et permettre le positionnement des éléments par drag sur la couverture, synchronisé avec les sliders existants.

**Architecture :** Fichier unique `index.html`, aucune dépendance nouvelle. Trois ajouts : (1) une barre d'actions en haut (presets + menus export/réglages), (2) un inspecteur dans le panneau droit (état repos « Général + navigateur », état sélection « réglages de l'élément »), (3) un calque `.overlay` dans `.holder` — donc hors de `.cover`, invisible pour `html2canvas` — portant liserés, poignées et gestionnaires de drag. Le drag convertit les px en % et **écrit dans les inputs `inXxx` existants** puis déclenche `input` → `render()` fait tout le reste. Aucun état nouveau hors `selected`.

**Tech stack :** HTML/CSS/JS vanilla inline, html2canvas (inchangé), Playwright MCP ou navigateur pour la vérification.

**Spec :** `docs/superpowers/specs/2026-08-14-inspecteur-contextuel-design.md`

---

## Notes préalables pour l'exécutant

- **Pas d'infrastructure de test** dans ce projet. La vérification suit CLAUDE.md : syntaxe JS via `node --check`, puis contrôles dans un navigateur servi en HTTP. Chaque tâche fournit les commandes exactes.
- **Extraction JS pour `node --check`** (le seul `<script>` sur ligne isolée est le bloc inline ; la balise CDN `<script src=…>` tient sur une ligne et n'est pas capturée) :

  ```bash
  cd /Users/jean-paulgavini/Documents/Dev/ozalid
  awk '/^<script>$/{f=1;next}/^<\/script>$/{f=0}f' index.html > /tmp/ozalid-check.js && node --check /tmp/ozalid-check.js && echo SYNTAXE-OK
  ```

- **Serveur de revue** (jamais `python3 -m http.server`) :

  ```bash
  /Users/jean-paulgavini/.claude/scripts/serve.sh /Users/jean-paulgavini/Documents/Dev/ozalid
  ```

  Il imprime l'URL à utiliser (la page marche aussi en `file://`, mais la revue passe par HTTP).
- **Ligne 160** du fichier actuel : une seule ligne de ~98 000 caractères (image base64 dans `#elImg`). Ne jamais la lire ni la réécrire ; toutes les éditions se font par `Edit` sur des ancres textuelles uniques, pas par réécriture du fichier.
- **Ne pas toucher** : `render()` hors des points précisés, la sérialisation PNG (`pngInsertText`/`pngReadText`), `freezeArtGeometry`, l'export.
- **Cas particulier connu** : en mode `band`, `render()` dérive la position du bloc titre de la hauteur du bandeau (`block.style.top = bandH*0.22`) — `inBlockY` n'y joue pas. Le drag vertical du bloc titre est donc désactivé en mode bandeau (l'élément reste sélectionnable pour sa typographie).
- Après chaque tâche : commit (message en français, se terminant par la ligne `Claude-Session` fournie par l'environnement).

---

### Tâche 1 : Barre d'actions et agencement vertical

**Files:**
- Modify: `index.html` (CSS ~l.98-145, HTML panneau ~l.177-365)

- [ ] **Étape 1.1 — CSS : agencement en colonne + barre**

Dans le `<style>`, remplacer la règle `.wrap{display:flex;min-height:100vh}` par :

```css
body{display:flex;flex-direction:column;height:100vh}
.wrap{display:flex;flex:1 1 auto;min-height:0}
.topbar{display:flex;flex-wrap:wrap;align-items:center;gap:14px;background:var(--ink);color:#eee;padding:8px 14px;position:relative;z-index:5}
.tb-brand{font-family:var(--mono);font-size:11px;letter-spacing:.14em;text-transform:uppercase;color:#999}
.tb-lab{font-family:var(--mono);font-size:10px;letter-spacing:.12em;text-transform:uppercase;color:#888;margin-right:2px}
.tb-group{display:flex;align-items:center;gap:6px}
.tb-group button{padding:6px 10px;border:1px solid #4a4a46;background:transparent;color:#eee;cursor:pointer;font-family:var(--ui);font-size:11px;font-weight:700;border-radius:2px}
.tb-group button:hover{background:#fff;color:var(--ink)}
.tb-right{margin-left:auto;display:flex;gap:8px}
.topbar .status{margin:0;min-height:0;color:#aaa}
.menu{position:relative}
.menu summary{list-style:none;cursor:pointer;padding:6px 10px;border:1px solid #4a4a46;border-radius:2px;font-size:11px;font-weight:700;user-select:none}
.menu summary::-webkit-details-marker{display:none}
.menu[open] summary{background:#fff;color:var(--ink)}
.menu-body{position:absolute;right:0;top:calc(100% + 8px);background:var(--panel);color:var(--ink);border:1px solid var(--rule);border-radius:3px;padding:14px;width:270px;box-shadow:0 10px 24px rgba(0,0,0,.25);z-index:10}
```

Puis supprimer les règles devenues orphelines : `.presets{…}`, `.presets button{…}`, `.presets button:hover{…}`.

Dans la media query `@media (max-width:820px)`, ne rien changer (le `.wrap` en colonne existant reste correct sous la barre).

- [ ] **Étape 1.2 — HTML : insérer la barre avant `.wrap`**

Juste après `<body>`, insérer :

```html
<header class="topbar">
  <span class="tb-brand">Atelier — couverture</span>
  <div class="tb-group" title="Chaque maquette recharge tous les réglages.">
    <span class="tb-lab">Maquettes</span>
    <button data-preset="folio">Folio</button>
    <button data-preset="blanche">Blanche</button>
    <button data-preset="overlay">Surimpression</button>
  </div>
  <div class="tb-right">
    <p class="status" id="status"></p>
    <details class="menu">
      <summary>Exporter PNG ▾</summary>
      <div class="menu-body">
        <div class="check"><input type="checkbox" id="inMarks" checked><span>Traits de coupe à l'écran</span></div>
        <div class="check"><input type="checkbox" id="inEmbedCfg" checked><span>Écrire les réglages dans le PNG</span></div>
        <div class="check"><input type="checkbox" id="inEmbedImg" checked><span>Y joindre la photo source</span></div>
        <button class="btn" id="btnPng">Exporter en PNG</button>
        <p class="note">Les réglages sont écrits dans un bloc <code>tEXt</code> du fichier. L'image reste lisible partout ; seul cet atelier sait relire ce bloc.</p>
      </div>
    </details>
    <details class="menu">
      <summary>Réglages ▾</summary>
      <div class="menu-body">
        <label><span class="lab">Depuis un PNG exporté</span>
          <input type="file" id="inLoadPng" accept="image/png"></label>
        <label><span class="lab">Depuis un fichier de réglages</span>
          <input type="file" id="inLoadJson" accept="application/json,.json"></label>
        <button class="btn ghost" id="btnJson">Enregistrer les réglages seuls (JSON)</button>
      </div>
    </details>
  </div>
</header>
```

- [ ] **Étape 1.3 — HTML : dégraisser le panneau**

Supprimer du panneau (`.panel-inner`) :
- `<p class="brand">Gabarit de première de couverture</p>` et `<h1>Atelier</h1>` ;
- le fieldset entier « Maquettes de départ » (boutons presets + note) ;
- le fieldset entier « Sortie » (ses contrôles sont désormais dans le menu Exporter) ;
- le fieldset entier « Reprendre des réglages » (dans le menu Réglages) — **y compris** `<p class="status" id="status"></p>` qui vit maintenant dans la barre.

Il ne doit rester **qu'une seule occurrence** de chaque id déplacé (`inMarks`, `inEmbedCfg`, `inEmbedImg`, `btnPng`, `inLoadPng`, `inLoadJson`, `btnJson`, `status`) :

```bash
for id in inMarks inEmbedCfg inEmbedImg btnPng inLoadPng inLoadJson btnJson status; do
  printf '%s : %s\n' "$id" "$(grep -c "id=\"$id\"" index.html)"
done
```

Attendu : `1` partout.

- [ ] **Étape 1.4 — JS : sélecteurs à adapter**

Trois retouches :

1. Le gestionnaire de presets — remplacer :
```js
document.querySelectorAll('.presets button').forEach(b =>
  b.addEventListener('click', () => applyPreset(b.dataset.preset)));
```
par :
```js
document.querySelectorAll('[data-preset]').forEach(b =>
  b.addEventListener('click', () => applyPreset(b.dataset.preset)));
```

2. `collectConfig()` — élargir le balayage à tout le document (des contrôles `inXxx` vivent désormais dans la barre) ; remplacer :
```js
  document.querySelectorAll('.panel input[id^="in"], .panel select[id^="in"]').forEach(el => {
```
par :
```js
  document.querySelectorAll('input[id^="in"], select[id^="in"]').forEach(el => {
```

3. Fermer les menus au clic ailleurs — ajouter avant la ligne `window.addEventListener('resize', …)` :
```js
/* les menus de la barre se referment au clic hors d'eux */
document.addEventListener('click', e => {
  document.querySelectorAll('details.menu[open]').forEach(d => {
    if (!d.contains(e.target)) d.removeAttribute('open');
  });
});
```

- [ ] **Étape 1.5 — JS : couverture plus grande**

Dans `fitCover()`, remplacer `Math.min(stage.clientWidth - 120, 400)` par `Math.min(stage.clientWidth - 120, 520)`.

- [ ] **Étape 1.6 — Vérifier**

```bash
awk '/^<script>$/{f=1;next}/^<\/script>$/{f=0}f' index.html > /tmp/ozalid-check.js && node --check /tmp/ozalid-check.js && echo SYNTAXE-OK
```

Attendu : `SYNTAXE-OK`. Puis servir et ouvrir l'URL de `serve.sh` ; vérifier visuellement (capture d'écran) :
- barre sombre en haut : presets à gauche, deux menus à droite qui s'ouvrent/se ferment ;
- les trois presets rechargent bien la maquette ;
- panneau droit sans les trois fieldsets déplacés ;
- console du navigateur sans erreur.

Round-trip rapide dans la console du navigateur :
```js
(() => { const c = collectConfig(); return ['inMarks','inEmbedCfg'].every(k => k in c.fields); })()
```
Attendu : `true` (les contrôles de la barre sont bien sérialisés).

- [ ] **Étape 1.7 — Commit**

```bash
git add index.html && git commit -m "Barre d'actions : presets et menus export/réglages sortent du panneau"
```

---

### Tâche 2 : Inspecteur — navigateur d'éléments et états du panneau

**Files:**
- Modify: `index.html` (fieldsets du panneau, CSS, JS après `setMode`)

- [ ] **Étape 2.1 — HTML : identifier les fieldsets**

Donner un id aux fieldsets qui n'en ont pas (l'ancre est la balise `<legend>`) :

| Legend | Balise fieldset devient |
|---|---|
| `Mise en page` | `<fieldset id="fsGeneral" style="border-top:0;padding-top:0">` |
| `Cadre` | `<fieldset id="fsFrame" class="off">` |
| `Texte` | `<fieldset id="fsText" class="off">` |
| `Auteur` | `<fieldset id="fsAuthor" class="off">` |
| `Titre` | `<fieldset id="fsTitle" class="off">` |
| `Pied — éditeur` | `<fieldset id="fsImprint" class="off">` |

Et ajouter `class="off"` aux trois fieldsets déjà nommés : `<fieldset id="fsBand" class="off">`, `<fieldset id="fsBlockY" class="off">`, `<fieldset id="fsImage" class="off">`. (`off` = masqué par l'inspecteur ; indépendant du `.hide` que `render()` pose selon le mode. Le fieldset « Mise en page » perd son ancien `style="border-top:0;padding-top:0"` au profit de la version ci-dessus — il devient le premier du panneau.)

- [ ] **Étape 2.2 — HTML : bouton retour et navigateur**

En tête de `.panel-inner` (première ligne), insérer :

```html
<button class="back off" id="btnBack">‹ Général</button>
```

Juste après le fieldset `#fsGeneral` (fermeture `</fieldset>`), insérer :

```html
<fieldset id="fsNav">
  <legend>Éléments</legend>
  <div class="nav">
    <button data-el="image">Image <span>›</span></button>
    <button data-el="band">Bandeau <span>›</span></button>
    <button data-el="frame">Cadre <span>›</span></button>
    <button data-el="block">Bloc titre <span>›</span></button>
    <button data-el="imprint">Pied éditeur <span>›</span></button>
  </div>
  <p class="note">Clique un élément ici ou directement sur la couverture.</p>
</fieldset>
```

- [ ] **Étape 2.3 — CSS**

Ajouter au `<style>` (près des règles du panneau) :

```css
.off{display:none!important}
.nav button{display:flex;justify-content:space-between;width:100%;padding:8px 10px;border:1px solid var(--rule);background:#fff;cursor:pointer;font-family:var(--ui);font-size:12px;font-weight:600;color:var(--ink);border-radius:2px;margin-bottom:6px}
.nav button:hover{border-color:var(--ink)}
.nav button.dim{color:var(--muted);background:transparent}
.nav button span{color:var(--muted)}
.back{display:block;width:100%;text-align:left;padding:8px 0;border:0;background:none;cursor:pointer;font-family:var(--ui);font-size:12px;font-weight:700;color:var(--accent)}
```

- [ ] **Étape 2.4 — JS : état de sélection**

Après la fonction `setMode(v){…}`, insérer :

```js
/* ---------- inspecteur : sélection d'élément ---------- */
const ELEMENTS = {
  image:  ['fsImage'],
  band:   ['fsBand'],
  frame:  ['fsFrame'],
  block:  ['fsBlockY','fsText','fsAuthor','fsTitle'],
  imprint:['fsImprint']
};
let selected = null;

function applyInspector(){
  const detail = !!selected;
  $('fsGeneral').classList.toggle('off', detail);
  $('fsNav').classList.toggle('off', detail);
  $('btnBack').classList.toggle('off', !detail);
  for (const [name, ids] of Object.entries(ELEMENTS))
    ids.forEach(id => $(id).classList.toggle('off', selected !== name));
}

function selectEl(name){
  selected = ELEMENTS[name] ? name : null;
  applyInspector();
  render();
}

$('btnBack').addEventListener('click', () => selectEl(null));
document.querySelectorAll('#fsNav .nav button').forEach(b =>
  b.addEventListener('click', () => selectEl(b.dataset.el)));
```

- [ ] **Étape 2.5 — JS : retours au Général et navigateur grisé**

1. Dans `applyPreset(name)`, remplacer la dernière ligne `fitCover(); render();` par :
```js
  fitCover(); selectEl(null);
```
(`selectEl` appelle `render()`.)

2. Dans `applyConfig(c)`, remplacer `fitCover(); render();` par :
```js
  fitCover(); selectEl(null);
```

3. À la fin de `render()`, juste après la ligne `$('marks').style.setProperty('--marks-on', …)`, ajouter :
```js
  /* navigateur : griser les éléments sans objet dans l'état courant */
  const navDim = {
    image: mode === 'typo', band: mode !== 'band',
    frame: !$('inFrameOn').checked, block: false,
    imprint: !$('inImprintOn').checked
  };
  document.querySelectorAll('#fsNav .nav button').forEach(b =>
    b.classList.toggle('dim', navDim[b.dataset.el]));
```

**Piège d'ordre d'exécution :** `applyPreset('folio')` en fin de script appelle désormais `selectEl` → `applyInspector` → les ids `fsGeneral`, `fsNav`, `btnBack` doivent exister dans le DOM (fait en 2.1/2.2), et `ELEMENTS`/`selectEl` doivent être définis avant cet appel (le bloc de 2.4 est bien plus haut dans le script). Rien d'autre à faire, juste ne pas déplacer l'appel final.

- [ ] **Étape 2.6 — Vérifier**

`node --check` (commande de la tâche 1, attendu `SYNTAXE-OK`), puis dans le navigateur :
- au chargement (preset folio) : panneau = Général + Éléments, rien d'autre ; « Cadre » et « Pied éditeur » grisés (désactivés dans ce preset) ;
- clic « Bloc titre » → le panneau n'affiche que « ‹ Général » + Texte/Auteur/Titre (fsBlockY reste masqué par `render()` en mode bandeau : normal) ;
- « ‹ Général » → retour à l'état repos ;
- clic preset « Blanche » pendant une sélection → retour au Général ;
- console sans erreur.

Console du navigateur — la sérialisation ne doit pas voir les fieldsets masqués :
```js
(() => { selectEl('image'); const n = Object.keys(collectConfig().fields).length; selectEl(null); return n; })()
```
Attendu : le même nombre qu'au repos (~55) — le masquage CSS ne change pas le balayage DOM.

- [ ] **Étape 2.7 — Commit**

```bash
git add index.html && git commit -m "Inspecteur : navigateur d'éléments et panneau contextuel"
```

---

### Tâche 3 : Calque de sélection sur la couverture

**Files:**
- Modify: `index.html` (HTML `.holder`, CSS, JS `render()` + nouveau bloc)

- [ ] **Étape 3.1 — HTML : le calque**

Dans `.holder`, juste après la fermeture `</div>` de `.cover` (celle qui précède `</div>` de `.holder`), insérer :

```html
<div class="overlay" id="overlay">
  <div class="ovl" data-el="image"></div>
  <div class="ovl ovl-line" data-el="band"></div>
  <div class="ovl" data-el="frame"><i class="handle h-corner"></i></div>
  <div class="ovl" data-el="block"><i class="handle h-left"></i><i class="handle h-right"></i></div>
  <div class="ovl" data-el="imprint"><i class="handle h-left"></i><i class="handle h-right"></i></div>
</div>
```

(L'ordre DOM fait l'empilement : l'image dessous, les blocs dessus — un clic sur le bloc titre par-dessus l'image sélectionne bien le bloc.)

- [ ] **Étape 3.2 — CSS**

```css
/* ---------- calque de sélection (hors .cover : jamais exporté) ---------- */
.overlay{position:absolute;inset:0;pointer-events:none}
.ovl{position:absolute;pointer-events:auto;cursor:pointer;border:1px solid transparent}
.ovl:hover{border-color:rgba(26,110,224,.5)}
.ovl.sel{border:1.5px solid #1a6ee0;cursor:grab}
.ovl.sel:active{cursor:grabbing}
.ovl-line{cursor:ns-resize}
.handle{position:absolute;width:11px;height:11px;background:#fff;border:1.5px solid #1a6ee0;border-radius:50%;display:none;pointer-events:auto}
.ovl.sel .handle{display:block}
.h-left{left:-6px;top:50%;margin-top:-6px;cursor:ew-resize}
.h-right{right:-6px;top:50%;margin-top:-6px;cursor:ew-resize}
.h-corner{left:-6px;top:-6px;cursor:nwse-resize}
```

- [ ] **Étape 3.3 — JS : synchroniser le calque**

Après le bloc inspecteur de la tâche 2 (fonction `selectEl` et ses listeners), insérer :

```js
/* ---------- calque : positionné d'après la géométrie réelle ---------- */
function syncOverlay(){
  const hb = $('overlay').parentElement.getBoundingClientRect();
  const place = (name, el, visible) => {
    const o = document.querySelector(`.ovl[data-el="${name}"]`);
    o.classList.toggle('off', !visible);
    o.classList.toggle('sel', selected === name);
    if (!visible) return;
    const r = el.getBoundingClientRect();
    o.style.left = (r.left - hb.left) + 'px';
    o.style.top = (r.top - hb.top) + 'px';
    o.style.width = r.width + 'px';
    o.style.height = r.height + 'px';
  };
  place('image', $('art'), mode !== 'typo');
  place('frame', $('frame'), $('inFrameOn').checked);
  place('block', $('block'), true);
  place('imprint', $('imprint'), $('inImprintOn').checked);
  /* frontière du bandeau : fine bande horizontale sur le bord haut de l'image */
  const ob = document.querySelector('.ovl[data-el="band"]');
  const bandOn = mode === 'band';
  ob.classList.toggle('off', !bandOn);
  ob.classList.toggle('sel', selected === 'band');
  if (bandOn) {
    const cr = cover.getBoundingClientRect();
    ob.style.left = (cr.left - hb.left) + 'px';
    ob.style.top = (cr.top - hb.top + cr.height * (+$('inBand').value / 100) - 5) + 'px';
    ob.style.width = cr.width + 'px';
    ob.style.height = '10px';
  }
}

/* clic sur le calque = sélection ; clic sur le fond de la scène = désélection */
document.querySelectorAll('.ovl').forEach(o =>
  o.addEventListener('pointerdown', () => { if (selected !== o.dataset.el) selectEl(o.dataset.el); }));
document.querySelector('.stage').addEventListener('pointerdown', e => {
  if (!e.target.closest('.holder')) selectEl(null);
});
document.addEventListener('keydown', e => { if (e.key === 'Escape') selectEl(null); });
```

- [ ] **Étape 3.4 — JS : appeler la synchronisation depuis `render()`**

Tout à la fin de `render()`, après la boucle `for (const [k,[v,d,u]] of Object.entries(R)) …`, ajouter :

```js
  syncOverlay();
}
```

(`render()` est appelée pour la première fois par `applyPreset('folio')` en fin de script — `syncOverlay` est alors déjà définie. Le redimensionnement de fenêtre passe déjà par `fitCover(); render();`.)

- [ ] **Étape 3.5 — Vérifier**

`node --check` (attendu `SYNTAXE-OK`), puis dans le navigateur :
- au survol des zones (image, bloc titre, pastille du pied si activé) : liseré bleu léger épousant l'élément ;
- clic sur l'image → liseré appuyé + le panneau bascule sur les réglages Image ;
- Échap ou clic sur le fond gris → désélection, panneau au repos ;
- bouger le slider « Hauteur » du bandeau → la ligne bleue de la frontière suit ;
- **export PNG** : exporter, ouvrir le fichier → aucun liseré ni poignée dans l'image (le calque est hors `.cover`).

- [ ] **Étape 3.6 — Commit**

```bash
git add index.html && git commit -m "Calque de sélection sur la couverture, synchronisé avec le rendu"
```

---

### Tâche 4 : Drag vertical des blocs et poignées de marge

**Files:**
- Modify: `index.html` (JS après `syncOverlay` et ses listeners)

- [ ] **Étape 4.1 — JS : primitives de drag**

Après le bloc « clic sur le calque » de la tâche 3, insérer :

```js
/* ---------- manipulation directe : px souris → % → input inXxx → render ---------- */
function setParam(id, v){
  const el = $(id);
  v = Math.min(+el.max, Math.max(+el.min, v));
  el.value = v;
  el.dispatchEvent(new Event('input', { bubbles: true }));
}

/* drag du corps d'un élément du calque. opts : { input, axis:'x'|'y', invert, enabled } */
function dragParam(name, opts){
  const o = document.querySelector(`.ovl[data-el="${name}"]`);
  o.addEventListener('pointerdown', e => {
    if (opts.enabled && !opts.enabled()) return;
    e.preventDefault();
    const r = cover.getBoundingClientRect();
    const ref = opts.axis === 'x' ? r.width : r.height;
    const start = opts.axis === 'x' ? e.clientX : e.clientY;
    const v0 = +$(opts.input).value;
    const move = ev => {
      const d = ((opts.axis === 'x' ? ev.clientX : ev.clientY) - start) / ref * 100;
      setParam(opts.input, v0 + (opts.invert ? -d : d));
    };
    const up = () => {
      window.removeEventListener('pointermove', move);
      window.removeEventListener('pointerup', up);
    };
    window.addEventListener('pointermove', move);
    window.addEventListener('pointerup', up);
  });
}

/* drag d'une poignée. opts : { input, axis, dir(handle) → +1|-1 } */
function dragHandle(sel, opts){
  document.querySelectorAll(sel).forEach(h => {
    h.addEventListener('pointerdown', e => {
      e.stopPropagation(); e.preventDefault();
      const r = cover.getBoundingClientRect();
      const ref = opts.axis === 'x' ? r.width : r.height;
      const start = opts.axis === 'x' ? e.clientX : e.clientY;
      const v0 = +$(opts.input).value;
      const dir = opts.dir ? opts.dir(h) : 1;
      const move = ev => {
        const d = ((opts.axis === 'x' ? ev.clientX : ev.clientY) - start) / ref * 100;
        setParam(opts.input, v0 + d * dir);
      };
      const up = () => {
        window.removeEventListener('pointermove', move);
        window.removeEventListener('pointerup', up);
      };
      window.addEventListener('pointermove', move);
      window.addEventListener('pointerup', up);
    });
  });
}
```

(Le `pointerdown` de sélection posé en tâche 3 sur chaque `.ovl` coexiste : premier clic = sélection **et** début de drag dans le même geste. `setParam` borne toujours aux min/max du slider.)

- [ ] **Étape 4.2 — JS : brancher les blocs**

Juste après les primitives :

```js
/* bloc titre : vertical (sauf mode bandeau, où la position dérive du bandeau) */
dragParam('block',   { input: 'inBlockY',   axis: 'y', enabled: () => mode !== 'band' });
/* pied éditeur : inImprintY est une distance depuis le bas → inversé */
dragParam('imprint', { input: 'inImprintY', axis: 'y', invert: true });
/* poignées latérales des deux blocs : marge symétrique */
dragHandle('.ovl[data-el="block"] .handle, .ovl[data-el="imprint"] .handle', {
  input: 'inPadX', axis: 'x',
  dir: h => h.classList.contains('h-left') ? 1 : -1
});
```

- [ ] **Étape 4.3 — Vérifier**

`node --check` (attendu `SYNTAXE-OK`). Dans le navigateur, preset **Blanche** (mode sans image, `inBlockY` actif) :
- glisser le bloc titre vers le bas → il suit la souris, et le slider « Position verticale » (visible en re-sélectionnant, ou sa valeur via console) suit ;
- glisser le pied éditeur vers le bas → il descend (valeur `inImprintY` qui diminue) ;
- poignée gauche du bloc sélectionné vers la droite → la marge augmente des deux côtés ;
- inversement, bouger le slider → l'élément et son liseré suivent.

Contrôle scriptable (console du navigateur, preset Blanche chargé) :
```js
(() => {
  const o = document.querySelector('.ovl[data-el="block"]');
  const r = o.getBoundingClientRect(), x = r.left + r.width/2, y = r.top + 4;
  const v0 = +document.getElementById('inBlockY').value;
  o.dispatchEvent(new PointerEvent('pointerdown', {bubbles:true, clientX:x, clientY:y}));
  window.dispatchEvent(new PointerEvent('pointermove', {clientX:x, clientY:y+40}));
  window.dispatchEvent(new PointerEvent('pointerup', {}));
  return [v0, +document.getElementById('inBlockY').value];
})()
```
Attendu : la seconde valeur ≈ première + 40/hauteurCouverture×100, bornée à [0,85].

En preset **Folio** (mode bandeau) : le drag vertical du bloc titre ne fait rien (position dérivée), la sélection et les poignées de marge marchent.

- [ ] **Étape 4.4 — Commit**

```bash
git add index.html && git commit -m "Drag vertical des blocs et poignées de marge, synchronisés avec les sliders"
```

---

### Tâche 5 : Image, bandeau et cadre à la souris

**Files:**
- Modify: `index.html` (JS, à la suite des branchements de la tâche 4)

- [ ] **Étape 5.1 — JS : brancher image, bandeau, cadre**

```js
/* image : le contenu suit la souris (artY diminue quand on tire vers le bas) */
dragParam('image', { input: 'inArtY', axis: 'y', invert: true });
/* frontière du bandeau : tirer vers le bas agrandit le bandeau */
dragParam('band',  { input: 'inBand', axis: 'y' });
/* coin du cadre : tirer vers l'intérieur (droite) élargit la marge */
dragHandle('.ovl[data-el="frame"] .handle', { input: 'inFrameM', axis: 'x', dir: () => 1 });

/* molette / trackpad sur l'image sélectionnée : échelle */
document.querySelector('.ovl[data-el="image"]').addEventListener('wheel', e => {
  if (selected !== 'image') return;
  e.preventDefault();
  setParam('inZoom', +$('inZoom').value - e.deltaY * 0.002);
}, { passive: false });
```

- [ ] **Étape 5.2 — Vérifier**

`node --check` (attendu `SYNTAXE-OK`). Dans le navigateur :
- preset **Folio** : glisser la ligne de frontière du bandeau vers le bas → le bandeau grandit, le slider « Hauteur » suit ; glisser l'image verticalement → le cadrage suit (`inArtY`) ; image sélectionnée + molette → zoom borné [1, 2.2] ;
- preset **Blanche** : sélectionner le cadre, tirer la poignée de coin vers la droite → la marge du cadre augmente, slider « Marge du cadre » synchrone ;
- preset **Surimpression** : drag de l'image OK en pleine page ;
- après chaque drag, le liseré bleu reste collé à l'élément (le `syncOverlay` de `render()` suit).

- [ ] **Étape 5.3 — Commit**

```bash
git add index.html && git commit -m "Cadrage image, bandeau et cadre à la souris ; zoom à la molette"
```

---

### Tâche 6 : Ajustement fin au clavier

**Files:**
- Modify: `index.html` (JS : remplacer le listener Échap de la tâche 3)

- [ ] **Étape 6.1 — JS : flèches sur l'élément sélectionné**

Remplacer la ligne posée en tâche 3 :

```js
document.addEventListener('keydown', e => { if (e.key === 'Escape') selectEl(null); });
```

par :

```js
/* clavier : Échap désélectionne ; flèches = ± un pas du paramètre de position principal.
   KEY_DIR : signe appliqué à ArrowDown pour que « bas » déplace visuellement vers le bas. */
const PRIMARY = { image:'inArtY', band:'inBand', frame:'inFrameM', block:'inBlockY', imprint:'inImprintY' };
const KEY_DIR = { image:-1, band:1, frame:1, block:1, imprint:-1 };
document.addEventListener('keydown', e => {
  if (e.key === 'Escape') { selectEl(null); return; }
  if (!selected || (e.key !== 'ArrowUp' && e.key !== 'ArrowDown')) return;
  const t = e.target;
  if (t && ['INPUT','SELECT','TEXTAREA','BUTTON'].includes(t.tagName)) return;
  if (selected === 'block' && mode === 'band') return;
  const el = $(PRIMARY[selected]);
  const step = +el.step || 1;
  setParam(el.id, +el.value + (e.key === 'ArrowDown' ? 1 : -1) * KEY_DIR[selected] * step);
  e.preventDefault();
});
```

- [ ] **Étape 6.2 — Vérifier**

`node --check` (attendu `SYNTAXE-OK`). Navigateur, preset Blanche : sélectionner le bloc titre en cliquant la couverture, presser ↓ cinq fois → le bloc descend de 5 pas et le slider suit ; ↑ le remonte ; les flèches dans un champ texte du panneau continuent d'y déplacer le curseur (pas de vol d'événement) ; Échap désélectionne.

- [ ] **Étape 6.3 — Commit**

```bash
git add index.html && git commit -m "Ajustement fin au clavier de l'élément sélectionné"
```

---

### Tâche 7 : Vérifications finales de la spec

**Files:** aucun (contrôles), retouches éventuelles dans `index.html`

- [ ] **Étape 7.1 — Syntaxe et grille complète**

`node --check` (attendu `SYNTAXE-OK`), puis dérouler dans le navigateur la grille de CLAUDE.md et de la spec :

1. **3 presets × 3 modes** : pour chaque preset, passer par les trois boutons du segment Mode — pas d'erreur console, panneau cohérent (fieldsets liés au mode masqués), calque cohérent (frontière bandeau seulement en mode bandeau, image absente en mode typo).
2. **Round-trip PNG** : exporter avec « réglages dans le PNG » coché, recharger via Réglages ▾ → chaque contrôle revient à l'identique (vérifier notamment un contrôle de la barre, un du Général, un d'un élément). Contrôle console avant/après :
   ```js
   JSON.stringify(collectConfig().fields)
   ```
   Les deux chaînes doivent être égales.
3. **Round-trip JSON** : enregistrer les réglages seuls, modifier deux sliders, recharger le JSON → valeurs restaurées.
4. **Export propre** : PNG exporté sans liseré, poignées ni traits de coupe ; cadrage image identique à l'écran (le `freezeArtGeometry` n'a pas bougé).
5. **Synchronisation croisée** : pour chacun des cinq éléments — drag → slider à jour ; slider → couverture et liseré à jour.
6. **Viewport étroit** (< 820 px, réduire la fenêtre) : barre utilisable (retour à la ligne), panneau sous la scène, sélection et drag encore fonctionnels.

- [ ] **Étape 7.2 — Corriger ce que la grille révèle**

Toute anomalie se corrige ici, puis re-dérouler le point concerné. Ne pas élargir le périmètre.

- [ ] **Étape 7.3 — Commit final**

```bash
git add index.html && git commit -m "Finitions de la revue d'ergonomie après vérification complète"
```

(Si l'étape 7.2 n'a rien changé, pas de commit — la grille aura simplement validé l'existant.)
