# Lot 1 — Onglets et 4ème de couverture : plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal :** ajouter à `index.html` une navigation à deux onglets (1ère / 4ème de couverture) et l'éditeur complet de la 4ème (fond, texte de présentation, pied, zone ISBN), persistant automatiquement.

**Architecture :** un état `tab` en mémoire bascule deux `.holder` dans la scène ; `#cover4` est un second élément `.cover` dont tous les styles sont écrits par `render()` via des variables CSS `--q4-*`, toutes en fraction de `--cw`. Les contrôles `inQ4*` héritent de la persistance automatique (`collectConfig` étendu aux `textarea`).

**Tech stack :** fichier unique `index.html` (CSS/JS inline), aucune dépendance nouvelle.

**Spécification :** `docs/superpowers/specs/2026-08-18-packaging-couverture-design.md`.
**Hors périmètre du lot 1 :** image de fond propre à la 4ème (différée, voir spec) ; onglet Assemblage (lot 2) ; export planche (lot 3). Le bouton « Exporter PNG » continue d'exporter la 1ère seule.

**Vérification (pas de framework de test) :** chaque tâche se vérifie par `node --check` sur le JS extrait et par des sondes JS exécutées dans la page servie par `/Users/jean-paulgavini/.claude/scripts/serve.sh` (jamais `python3 -m http.server`). Commande d'extraction du JS, à réutiliser telle quelle :

```bash
node -e "
const html = require('fs').readFileSync('index.html','utf8');
const blocks = [...html.matchAll(/<script(?![^>]*src)[^>]*>([\s\S]*?)<\/script>/g)].map(m=>m[1]);
require('fs').writeFileSync('/tmp/ozalid-extrait.js', blocks.join('\n;\n'));
" && node --check /tmp/ozalid-extrait.js && echo "Syntaxe OK"
```

**Repères dans `index.html`** (les numéros de ligne bougent, les ancres textuelles font foi) :
- Barre haute : `<header class="topbar">`, groupe maquettes `tb-group`.
- Scène : `<div class="stage">` contient `<div class="holder">` (marks, dims, `#cover`, `#overlay`).
- CSS `.cover` : bloc `--cw:340px; aspect-ratio:var(--fw)/var(--fh)`.
- `render()` : commence par `const s = cover.style;`.
- `fitCover()` : se termine par `cover.style.setProperty('--cw', w + 'px');`.
- `applyInspector()` / `ELEMENTS` : visibilité des fieldsets du panneau.
- `collectConfig()` : sélecteur `'input[id^="in"], select[id^="in"]'`.
- Population des polices : boucle `['inAuthorFace',…,'inPastilleFace'].forEach(id => …)`.
- `PRESETS` : trois entrées `folio`, `blanche`, `overlay`.

---

### Tâche 1 : barre d'onglets et second holder

**Files :**
- Modify : `index.html` (topbar, stage, CSS, JS près de `setMode`)

- [x] **Step 1 : vérifier l'existence de la classe `.hide`**

```bash
grep -n "\.hide{" index.html
```
Attendu : une règle du type `.hide{display:none !important}` (ou équivalent). Si elle n'existe pas, STOP : relire le CSS et adapter (le plan suppose `display:none`).

- [x] **Step 2 : ajouter la barre d'onglets dans la topbar**

Juste après `<h1 class="tb-brand">Atelier — couverture</h1>` :

```html
  <div class="tb-group" id="segTab">
    <span class="tb-lab">Face</span>
    <button data-tab="une" aria-pressed="true">1ère</button>
    <button data-tab="quatre" aria-pressed="false">4ème</button>
  </div>
```

- [x] **Step 3 : CSS de l'état enfoncé des onglets**

À côté des règles `.topbar` existantes :

```css
#segTab button[aria-pressed="true"]{background:var(--accent);color:#fff;border-color:var(--accent)}
```

(Vérifier que `--accent` existe : `grep -n "\-\-accent" index.html`. Sinon utiliser la couleur des boutons `aria-pressed` de `segMode`.)

- [x] **Step 4 : nommer le holder existant et ajouter le second**

Sur le holder existant : `<div class="holder">` → `<div class="holder" id="holderUne">`.
Juste après la fermeture `</div>` de ce holder (celle qui suit `#overlay`), toujours dans `.stage` :

```html
    <div class="holder hide" id="holderQuatre">
      <div class="cover" id="cover4">
        <div class="q4-texte" id="elQ4Texte"></div>
        <div class="q4-pied" id="q4Pied">
          <div id="elQ4Mention"></div>
          <div id="elQ4Coll"></div>
          <div id="elQ4Prix"></div>
        </div>
        <div class="q4-isbn" id="elQ4Isbn"></div>
      </div>
    </div>
```

- [x] **Step 5 : état `tab`, `setTab`, écouteurs**

Près de `let mode = 'band';` :

```js
let tab = 'une';
function setTab(v){
  tab = v;
  if (v === 'quatre') selectEl(null); /* applyInspector + render */
  [...$('segTab').children].forEach(b => {
    if (b.tagName === 'BUTTON') b.setAttribute('aria-pressed', b.dataset.tab === v);
  });
  $('holderUne').classList.toggle('hide', v !== 'une');
  $('holderQuatre').classList.toggle('hide', v !== 'quatre');
  applyInspector(); render();
}
```

Près des écouteurs existants (`data-preset`) :

```js
document.querySelectorAll('#segTab button').forEach(b =>
  b.addEventListener('click', () => setTab(b.dataset.tab)));
```

- [x] **Step 6 : `fitCover()` dimensionne aussi `#cover4`**

Après `cover.style.setProperty('--cw', w + 'px');` :

```js
  cover4.style.setProperty('--cw', w + 'px');
```

- [x] **Step 7 : `render()` écrit format et fond de base sur `#cover4`**

Au début de `render()`, après `const s = cover.style;` :

```js
  const s4 = cover4.style;
```

Après les trois lignes `s.setProperty('--fw'/'--fh'/'--paper', …)` :

```js
  s4.setProperty('--fw', format[0]);
  s4.setProperty('--fh', format[1]);
  s4.setProperty('--paper', $('inPaper').value);
```

- [x] **Step 8 : syntaxe**

Lancer l'extraction + `node --check` (commande en tête de plan). Attendu : `Syntaxe OK`.

- [x] **Step 9 : vérification navigateur**

Servir via `serve.sh`, ouvrir la page, exécuter :

```js
setTab('quatre');
JSON.stringify({
  uneCachee: $('holderUne').classList.contains('hide'),
  quatreVisible: !$('holderQuatre').classList.contains('hide'),
  memeLargeur: getComputedStyle(cover4).width === getComputedStyle(cover).width,
  memeRatio: getComputedStyle(cover4).aspectRatio
});
```
Attendu : `uneCachee:true`, `quatreVisible:true`, `memeLargeur:true`, ratio = `108 / 178` (préréglage courant). Puis `setTab('une')` doit restaurer la 1ère. Vérifier aussi qu'un clic sur chaque onglet fonctionne à la souris.

- [x] **Step 10 : commit**

```bash
git add index.html
git commit -m "Onglets 1ère/4ème et second support de couverture"
```

---

### Tâche 2 : fond de la 4ème + fieldsets du panneau (visibilité par onglet)

**Files :**
- Modify : `index.html` (panneau, `applyInspector`, `render()`, CSS)

- [x] **Step 1 : quatre fieldsets vides dans le panneau**

Après la fermeture `</fieldset>` de `fsPastille` :

```html
      <fieldset id="fsQ4Fond" class="off">
        <legend>Fond de la 4ème</legend>
        <div class="row">
          <label><span class="lab">Fond</span>
            <select id="inQ4BgMode"><option value="herite" selected>papier de la 1ère</option><option value="couleur">couleur distincte</option></select></label>
          <label><span class="lab">Couleur</span><input type="color" id="inQ4Bg" value="#fcf0d8"></label>
        </div>
      </fieldset>
      <fieldset id="fsQ4Texte" class="off"><legend>Texte de présentation</legend></fieldset>
      <fieldset id="fsQ4Pied" class="off"><legend>Pied de 4ème</legend></fieldset>
      <fieldset id="fsQ4Isbn" class="off"><legend>Zone code-barres</legend></fieldset>
```

- [x] **Step 2 : `applyInspector()` tient compte de l'onglet**

Remplacer le corps de `applyInspector()` par :

```js
function applyInspector(){
  const q4 = tab === 'quatre';
  const detail = !!selected && !q4;
  $('fsGeneral').classList.toggle('off', detail || q4);
  $('fsNav').classList.toggle('off', detail || q4);
  $('btnBack').classList.toggle('off', !detail);
  for (const [name, ids] of Object.entries(ELEMENTS))
    ids.forEach(id => $(id).classList.toggle('off', q4 || selected !== name));
  for (const id of ['fsQ4Fond','fsQ4Texte','fsQ4Pied','fsQ4Isbn'])
    $(id).classList.toggle('off', !q4);
}
```

Attention : `applyInspector` est défini après `setTab` dans le fichier mais appelé par lui à l'exécution — pas de problème (hoisting de `function`).

- [x] **Step 3 : `render()` applique le fond**

Avec les autres écritures `s4` :

```js
  s4.background = $('inQ4BgMode').value === 'couleur' ? $('inQ4Bg').value : $('inPaper').value;
```

(`.cover` a `background:var(--paper)` ; l'écriture inline prime.)

- [x] **Step 4 : syntaxe** — extraction + `node --check`. Attendu : `Syntaxe OK`.

- [x] **Step 5 : vérification navigateur**

```js
setTab('quatre');
$('inQ4BgMode').value = 'couleur'; $('inQ4Bg').value = '#123456'; render();
const a = getComputedStyle(cover4).backgroundColor;         // rgb(18, 52, 86)
$('inQ4BgMode').value = 'herite'; render();
const b = getComputedStyle(cover4).backgroundColor === getComputedStyle(cover).backgroundColor;
JSON.stringify({couleur: a, herite: b,
  fieldsetsQ4: !$('fsQ4Fond').classList.contains('off'),
  generalCache: $('fsGeneral').classList.contains('off')});
```
Attendu : `couleur:"rgb(18, 52, 86)"`, `herite:true`, `fieldsetsQ4:true`, `generalCache:true`. Retour `setTab('une')` : panneau normal restauré, sélection d'élément (clic sur « Bloc titre ») fonctionne encore.

- [x] **Step 6 : commit**

```bash
git add index.html
git commit -m "Onglet 4ème : fond hérité ou distinct, panneau par onglet"
```

---

### Tâche 3 : texte de présentation

**Files :**
- Modify : `index.html` (fieldset fsQ4Texte, CSS `.q4-texte`, `render()`, population des polices, objet `R`)

- [x] **Step 1 : contrôles du fieldset**

Remplacer `<fieldset id="fsQ4Texte" class="off"><legend>Texte de présentation</legend></fieldset>` par :

```html
      <fieldset id="fsQ4Texte" class="off">
        <legend>Texte de présentation</legend>
        <label><span class="lab">Texte</span>
          <textarea id="inQ4Text" rows="8" placeholder="Extrait ou argumentaire…"></textarea></label>
        <label><span class="lab">Police</span><select id="inQ4TextFace"></select></label>
        <div class="row">
          <label><span class="lab">Graisse</span>
            <select id="inQ4TextWeight"><option>300</option><option selected>400</option><option>500</option><option>600</option><option>700</option></select></label>
          <label><span class="lab">Corps <span class="val" id="vQ4TextSize">3,0 %</span></span>
            <input type="range" id="inQ4TextSize" min="1.5" max="6" step="0.1" value="3"></label>
        </div>
        <div class="row">
          <label><span class="lab">Interlignage <span class="val" id="vQ4Leading">1,45</span></span>
            <input type="range" id="inQ4Leading" min="1" max="2" step="0.05" value="1.45"></label>
          <label><span class="lab">Alignement</span>
            <select id="inQ4Align"><option value="left" selected>Gauche</option><option value="justify">Justifié</option><option value="center">Centre</option></select></label>
        </div>
        <div class="row">
          <label><span class="lab">Marge latérale <span class="val" id="vQ4PadX">10,0 %</span></span>
            <input type="range" id="inQ4PadX" min="4" max="24" step="0.5" value="10"></label>
          <label><span class="lab">Position haute <span class="val" id="vQ4Top">12,0 %</span></span>
            <input type="range" id="inQ4Top" min="4" max="60" step="0.5" value="12"></label>
        </div>
        <label><span class="lab">Couleur</span><input type="color" id="inQ4TextColor" value="#191917"></label>
      </fieldset>
```

Si le CSS du panneau ne couvre pas `textarea`, ajouter près des styles des `input` du panneau :

```css
.panel textarea{width:100%;font:inherit;font-size:12px;background:var(--field,#fff);border:1px solid var(--rule);border-radius:4px;padding:6px;resize:vertical}
```
(Adapter aux variables réellement présentes — copier le style des `input[type=text]` du panneau.)

- [x] **Step 2 : CSS de l'élément**

Après le bloc `.pastille…` :

```css
/* ---------- 4ème de couverture ---------- */
.q4-texte{
  position:absolute;
  left:calc(var(--cw)*var(--q4-pad));right:calc(var(--cw)*var(--q4-pad));
  top:calc(var(--cw)*var(--q4-top));
  font-family:var(--q4-face);font-weight:var(--q4-weight);
  font-size:calc(var(--cw)*var(--q4-size));line-height:var(--q4-leading);
  color:var(--q4-color);text-align:var(--q4-align);
  white-space:pre-wrap;
}
```

(Tout en fraction de `--cw`, y compris la position verticale — règle du projet : jamais de % de hauteur.)

- [x] **Step 3 : `render()`**

Avec les écritures `s4` :

```js
  s4.setProperty('--q4-face', $('inQ4TextFace').value);
  s4.setProperty('--q4-weight', $('inQ4TextWeight').value);
  s4.setProperty('--q4-size', +$('inQ4TextSize').value / 100);
  s4.setProperty('--q4-leading', $('inQ4Leading').value);
  s4.setProperty('--q4-align', $('inQ4Align').value);
  s4.setProperty('--q4-pad', +$('inQ4PadX').value / 100);
  s4.setProperty('--q4-top', +$('inQ4Top').value / 100);
  s4.setProperty('--q4-color', $('inQ4TextColor').value);
  $('elQ4Texte').textContent = $('inQ4Text').value;
```

- [x] **Step 4 : population des polices**

Dans la boucle `['inAuthorFace','inTitleFace','inGenreFace','inMonoFace','inEditorFace','inPastilleFace'].forEach(…)`, ajouter `'inQ4TextFace'` à la liste.

- [x] **Step 5 : lectures affichées**

Dans l'objet `R` de `render()` :

```js
    vQ4TextSize:[$('inQ4TextSize').value,1,' %'], vQ4Leading:[$('inQ4Leading').value,2,''],
    vQ4PadX:[$('inQ4PadX').value,1,' %'], vQ4Top:[$('inQ4Top').value,1,' %'],
```

- [x] **Step 6 : syntaxe** — extraction + `node --check`. Attendu : `Syntaxe OK`.

- [x] **Step 7 : vérification navigateur**

```js
setTab('quatre');
$('inQ4Text').value = 'Ligne un.\nLigne deux.'; $('inQ4TextSize').value = 4; render();
const cs = getComputedStyle($('elQ4Texte'));
JSON.stringify({
  texte: $('elQ4Texte').textContent,
  corps: Math.abs(parseFloat(cs.fontSize) - parseFloat(getComputedStyle(cover4).width) * 0.04) < 0.5,
  lecture: $('vQ4TextSize').textContent
});
```
Attendu : `texte:"Ligne un.\nLigne deux."` (retour à la ligne visible à l'écran), `corps:true`, `lecture:"4,0 %"`. Vérifier visuellement le rendu (capture) : texte posé sur le fond, marges symétriques.

- [x] **Step 8 : commit**

```bash
git add index.html
git commit -m "4ème : texte de présentation réglable"
```

---

### Tâche 4 : pied de 4ème

**Files :**
- Modify : `index.html` (fieldset fsQ4Pied, CSS `.q4-pied`, `render()`, polices, objet `R`)

- [x] **Step 1 : contrôles**

Remplacer le fieldset vide `fsQ4Pied` par :

```html
      <fieldset id="fsQ4Pied" class="off">
        <legend>Pied de 4ème</legend>
        <div class="check"><input type="checkbox" id="inQ4PiedOn" checked><span>Pied de 4ème</span></div>
        <label><span class="lab">Mention</span><input type="text" id="inQ4Mention" value=""></label>
        <div class="row">
          <label><span class="lab">Numéro</span><input type="text" id="inQ4Coll" value=""></label>
          <label><span class="lab">Prix</span><input type="text" id="inQ4Prix" value=""></label>
        </div>
        <label><span class="lab">Police</span><select id="inQ4PiedFace"></select></label>
        <div class="row">
          <label><span class="lab">Corps <span class="val" id="vQ4PiedSize">2,4 %</span></span>
            <input type="range" id="inQ4PiedSize" min="1.5" max="5" step="0.1" value="2.4"></label>
          <label><span class="lab">Hauteur <span class="val" id="vQ4PiedY">4,0 %</span></span>
            <input type="range" id="inQ4PiedY" min="1" max="20" step="0.5" value="4"></label>
        </div>
        <label><span class="lab">Couleur</span><input type="color" id="inQ4PiedColor" value="#191917"></label>
      </fieldset>
```

- [x] **Step 2 : CSS**

Sous `.q4-texte{…}` :

```css
.q4-pied{
  position:absolute;left:calc(var(--cw)*var(--q4-pad));right:calc(var(--cw)*var(--q4-pad));
  bottom:calc(var(--cw)*var(--q4-pied-y));
  font-family:var(--q4-pied-face);font-size:calc(var(--cw)*var(--q4-pied-size));
  color:var(--q4-pied-color);text-align:center;line-height:1.5;
}
.q4-pied div:empty{display:none}
```

- [x] **Step 3 : `render()`**

```js
  s4.setProperty('--q4-pied-face', $('inQ4PiedFace').value);
  s4.setProperty('--q4-pied-size', +$('inQ4PiedSize').value / 100);
  s4.setProperty('--q4-pied-y', +$('inQ4PiedY').value / 100);
  s4.setProperty('--q4-pied-color', $('inQ4PiedColor').value);
  $('elQ4Mention').textContent = $('inQ4Mention').value;
  $('elQ4Coll').textContent = $('inQ4Coll').value;
  $('elQ4Prix').textContent = $('inQ4Prix').value;
  $('q4Pied').classList.toggle('hide', !$('inQ4PiedOn').checked);
```

- [x] **Step 4 : polices et lectures**

Ajouter `'inQ4PiedFace'` à la boucle de population des polices, et dans `R` :

```js
    vQ4PiedSize:[$('inQ4PiedSize').value,1,' %'], vQ4PiedY:[$('inQ4PiedY').value,1,' %'],
```

- [x] **Step 5 : syntaxe** — extraction + `node --check`. Attendu : `Syntaxe OK`.

- [x] **Step 6 : vérification navigateur**

```js
setTab('quatre');
$('inQ4Mention').value = 'Au Petit Remords'; $('inQ4Prix').value = '9 €'; render();
const lignes = [...$('q4Pied').children].filter(d => getComputedStyle(d).display !== 'none').length;
$('inQ4PiedOn').checked = false; render();
JSON.stringify({lignesVisibles: lignes, cacheApresDecoche: $('q4Pied').classList.contains('hide')});
```
Attendu : `lignesVisibles:2` (mention + prix, le numéro vide est masqué), `cacheApresDecoche:true`. Recocher et vérifier visuellement.

- [x] **Step 7 : commit**

```bash
git add index.html
git commit -m "4ème : pied (mention, numéro, prix)"
```

---

### Tâche 5 : zone code-barres / ISBN

**Files :**
- Modify : `index.html` (fieldset fsQ4Isbn, CSS `.q4-isbn`, `render()`, objet `R`)

- [x] **Step 1 : contrôles**

Remplacer le fieldset vide `fsQ4Isbn` par :

```html
      <fieldset id="fsQ4Isbn" class="off">
        <legend>Zone code-barres</legend>
        <div class="check"><input type="checkbox" id="inQ4IsbnOn"><span>Réserver la zone (cadre blanc, bas droite)</span></div>
        <div class="row">
          <label><span class="lab">Largeur <span class="val" id="vQ4IsbnW">34,0 %</span></span>
            <input type="range" id="inQ4IsbnW" min="15" max="50" step="0.5" value="34"></label>
          <label><span class="lab">Hauteur <span class="val" id="vQ4IsbnH">21,0 %</span></span>
            <input type="range" id="inQ4IsbnH" min="8" max="35" step="0.5" value="21"></label>
        </div>
        <div class="row">
          <label><span class="lab">Retrait latéral <span class="val" id="vQ4IsbnDx">7,0 %</span></span>
            <input type="range" id="inQ4IsbnDx" min="0" max="30" step="0.5" value="7"></label>
          <label><span class="lab">Retrait vertical <span class="val" id="vQ4IsbnDy">7,0 %</span></span>
            <input type="range" id="inQ4IsbnDy" min="0" max="30" step="0.5" value="7"></label>
        </div>
        <p class="note">Zone vide pour l'usage privé ; le code-barres est ajouté par le prestataire quand un ISBN existe.</p>
      </fieldset>
```

- [x] **Step 2 : CSS**

Sous `.q4-pied…` :

```css
.q4-isbn{
  position:absolute;
  right:calc(var(--cw)*var(--q4-isbn-dx));bottom:calc(var(--cw)*var(--q4-isbn-dy));
  width:calc(var(--cw)*var(--q4-isbn-w));height:calc(var(--cw)*var(--q4-isbn-h));
  background:#fff;
}
```

- [x] **Step 3 : `render()`**

```js
  s4.setProperty('--q4-isbn-w', +$('inQ4IsbnW').value / 100);
  s4.setProperty('--q4-isbn-h', +$('inQ4IsbnH').value / 100);
  s4.setProperty('--q4-isbn-dx', +$('inQ4IsbnDx').value / 100);
  s4.setProperty('--q4-isbn-dy', +$('inQ4IsbnDy').value / 100);
  $('elQ4Isbn').classList.toggle('hide', !$('inQ4IsbnOn').checked);
```

- [x] **Step 4 : lectures** — dans `R` :

```js
    vQ4IsbnW:[$('inQ4IsbnW').value,1,' %'], vQ4IsbnH:[$('inQ4IsbnH').value,1,' %'],
    vQ4IsbnDx:[$('inQ4IsbnDx').value,1,' %'], vQ4IsbnDy:[$('inQ4IsbnDy').value,1,' %'],
```

- [x] **Step 5 : syntaxe** — extraction + `node --check`. Attendu : `Syntaxe OK`.

- [x] **Step 6 : vérification navigateur**

```js
setTab('quatre');
$('inQ4IsbnOn').checked = true; render();
const r = $('elQ4Isbn').getBoundingClientRect(), c = cover4.getBoundingClientRect();
JSON.stringify({
  largeurOk: Math.abs(r.width - c.width * 0.34) < 1,
  retraitDroit: Math.abs((c.right - r.right) - c.width * 0.07) < 1,
  retraitBas: Math.abs((c.bottom - r.bottom) - c.width * 0.07) < 1
});
```
Attendu : les trois `true`. Décocher → zone masquée.

- [x] **Step 7 : commit**

```bash
git add index.html
git commit -m "4ème : zone code-barres réservée"
```

---

### Tâche 6 : persistance (textarea, presets, round-trip)

**Files :**
- Modify : `index.html` (`collectConfig`, `PRESETS`)

- [x] **Step 1 : étendre `collectConfig` aux `textarea`**

```js
  document.querySelectorAll('input[id^="in"], select[id^="in"], textarea[id^="in"]').forEach(el => {
```
(`applyConfig` gère déjà `el.value` génériquement — rien d'autre à changer.)

- [x] **Step 2 : mettre à jour la doc du projet**

Dans `CLAUDE.md`, section Sérialisation PNG, remplacer la mention `input[id^="in"]` par « `input`, `select` et `textarea` dont l'id commence par `in` ».

- [x] **Step 3 : valeurs dans les trois `PRESETS`**

Ajouter à **chacune** des trois entrées (`folio`, `blanche`, `overlay`) :

```js
    inQ4BgMode:'herite', inQ4Bg:'#fcf0d8', inQ4Text:'', inQ4TextFace:F['Spectral'],
    inQ4TextWeight:'400', inQ4TextSize:3, inQ4Leading:1.45, inQ4Align:'left',
    inQ4PadX:10, inQ4Top:12, inQ4TextColor:'#191917',
    inQ4PiedOn:true, inQ4Mention:'', inQ4Coll:'', inQ4Prix:'', inQ4PiedFace:F['Archivo'],
    inQ4PiedSize:2.4, inQ4PiedY:4, inQ4PiedColor:'#191917',
    inQ4IsbnOn:false, inQ4IsbnW:34, inQ4IsbnH:21, inQ4IsbnDx:7, inQ4IsbnDy:7,
```
(Vérifier dans le fichier que `F['Spectral']` et `F['Archivo']` existent — c'est le cas dans les presets actuels.)

- [x] **Step 4 : syntaxe** — extraction + `node --check`. Attendu : `Syntaxe OK`.

- [x] **Step 5 : round-trip complet dans le navigateur**

```js
setTab('quatre');
const vals = {inQ4Text:'Un extrait.\nSur deux lignes.', inQ4BgMode:'couleur', inQ4Bg:'#222222',
  inQ4Mention:'Mention', inQ4IsbnOn:true, inQ4IsbnW:'40'};
for (const [k,v] of Object.entries(vals)) {
  const el = $(k); if (el.type === 'checkbox') el.checked = v; else el.value = v;
}
render();
const cfg = collectConfig();
applyPreset('folio'); /* écrase tout */
applyConfig(cfg); render();
JSON.stringify(Object.entries(vals).map(([k,v]) => {
  const el = $(k);
  return [k, (el.type === 'checkbox') ? el.checked === v : el.value === String(v)];
}));
```
Attendu : toutes les paires à `true` — y compris le `textarea` multiligne.

- [x] **Step 6 : non-régression 1ère**

Appliquer les trois presets et les trois modes (`applyPreset('folio'|'blanche'|'overlay')`, `setMode('band'|'typo'|'overlay')` + `render()` à chaque fois) : aucune erreur console, rendu de la 1ère inchangé. Exporter un PNG par le bouton réel et le recharger par « Depuis un PNG exporté » : tous les contrôles (dont `inQ4*`) reviennent à l'identique.

- [x] **Step 7 : commit**

```bash
git add index.html CLAUDE.md
git commit -m "4ème : persistance complète (textarea, presets, round-trip)"
```

---

### Tâche 7 : revue finale du lot

- [x] **Step 1 : passe visuelle** — capture de la 4ème remplie (fond distinct, texte, pied, zone ISBN) et de la 1ère (préréglage folio) ; comparer avec l'état attendu de la spec.
- [x] **Step 2 : session locale** — recharger la page : l'état (y compris l'onglet 1ère par défaut et les réglages 4ème) revient de `localStorage`. « Réinitialiser l'atelier » remet tout à zéro sans erreur.
- [x] **Step 3 : cocher les cases de ce plan**, noter tout écart dans le message de commit final s'il y en a un.
