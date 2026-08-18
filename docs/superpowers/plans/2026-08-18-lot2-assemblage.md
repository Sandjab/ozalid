# Lot 2 — Assemblage de la planche : plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal :** ajouter l'onglet « Assemblage » qui affiche la planche complète (4ème + dos + 1ère) aux dimensions du prestataire, avec calcul du dos depuis le nombre de pages, dos composé verticalement, et guides de fond perdu.

**Architecture :** un objet `PROVIDERS` (Lulu seul) porte fond perdu et formule de dos. La planche est un flex `4ème | dos | 1ère` où les deux couvertures sont des **clones** (`cloneNode(true)`) de `#cover4`/`#cover` — tout leur style vient de variables CSS inline et de classes, un clone rend donc à l'identique ; seuls les ids sont retirés et `--cw` remplacé par l'échelle de la planche. Le dos est un élément persistant `#dos`, texte pivoté à 90° anti-horaire (lecture de bas en haut, comme la pastille). `render()` reste l'unique écrivain de styles (cibles : `s`, `s4`, et `sp = plancheFp.style`).

**Tech stack :** fichier unique `index.html`, aucune dépendance nouvelle.

**Spécification :** `docs/superpowers/specs/2026-08-18-packaging-couverture-design.md` (§3).
**Inclus en plus de la spec :** correctif « applyConfig déterministe » consigné à la revue finale du lot 1 (le chargement d'un PNG antérieur laissait les réglages absents en l'état).
**Hors périmètre :** export PDF de la planche (lot 3 — les guides écran devront alors être neutralisables, c'est noté dans les tâches) ; réordonnancement du texte du dos (ordre fixe auteur → titre → éditeur, de bas en haut).

**Vérification (pas de framework de test) :** `node --check` sur le JS extrait + sondes navigateur via `/Users/jean-paulgavini/.claude/scripts/serve.sh` (jamais `python3 -m http.server`). Extraction :

```bash
node -e "
const html = require('fs').readFileSync('index.html','utf8');
const blocks = [...html.matchAll(/<script(?![^>]*src)[^>]*>([\s\S]*?)<\/script>/g)].map(m=>m[1]);
require('fs').writeFileSync('/tmp/ozalid-extrait.js', blocks.join('\n;\n'));
" && node --check /tmp/ozalid-extrait.js && echo "Syntaxe OK"
```

**Repères** (ancres textuelles, les numéros de ligne bougent ; ne jamais lire index.html en entier — ligne base64 géante ~90 Ko) :
- Onglets : `#segTab` dans la topbar ; `setTab` / `let tab` près de `let mode = 'band';`.
- Holders : `#holderUne`, `#holderQuatre` dans `.stage`.
- `applyInspector()` : visibilité des fieldsets (variantes `q4`).
- `render()` : cibles `s = cover.style` et `s4 = cover4.style` ; objet `R` des lectures ; `fr(v,d)` formate en français.
- `PRESETS` (3 entrées) ; `collectConfig`/`applyConfig` ; init en bas de script (`loadLocal()`).
- CSS panneau : règles nues `input[type=text],select,textarea{…}` et la règle d'outline accent `…:focus`.

---

### Tâche 1 : onglet Assemblage, holder et squelette

**Files :** Modify `index.html` (topbar, stage, CSS, `setTab`, `applyInspector`, panneau)

- [x] **Step 1 : troisième bouton d'onglet**

Dans `#segTab`, après le bouton `data-tab="quatre"` :

```html
    <button data-tab="assemblage" aria-pressed="false">Assemblage</button>
```

- [x] **Step 2 : troisième holder dans `.stage`**

Juste après la fermeture de `#holderQuatre` :

```html
    <div class="holder hide" id="holderAsm">
      <div class="planche-fp" id="plancheFp">
        <div class="planche" id="planche">
          <div class="dos" id="dos">
            <div class="dos-texte" id="dosTexte">
              <span id="elDosAuthor"></span><span id="elDosTitle"></span><i class="esp"></i><span id="elDosEditor"></span>
            </div>
          </div>
        </div>
      </div>
      <p class="planche-dims" id="plancheDims"></p>
    </div>
```

- [x] **Step 3 : CSS de la planche**

Après le bloc `.q4-isbn{…}` :

```css
/* ---------- assemblage : planche 4ème + dos + 1ère ---------- */
/* Les variables --fp/--dos-l/--dos-hauteur sont écrites par render() (tâche 2) ;
   les fallbacks rendent le squelette visible d'ici là. */
.planche-fp{
  padding:calc(var(--cw,340px)*var(--fp,.03));
  background:repeating-linear-gradient(45deg, rgba(255,255,255,.08) 0 6px, rgba(255,255,255,0) 6px 12px);
  box-shadow:0 24px 48px -12px rgba(0,0,0,.45), 0 2px 6px rgba(0,0,0,.3);
}
/* guides écran seulement : à neutraliser à l'export de la planche (lot 3) */
.planche{display:flex;align-items:stretch;outline:1px dashed rgba(255,255,255,.55)}
.planche .cover{box-shadow:none}
.dos{
  position:relative;overflow:hidden;
  width:calc(var(--cw,340px)*var(--dos-l,.14));
  border-left:1px dashed rgba(255,255,255,.55);border-right:1px dashed rgba(255,255,255,.55);
}
.dos-texte{
  position:absolute;left:50%;top:50%;
  width:calc(var(--cw,340px)*var(--dos-hauteur,1.65));height:calc(var(--cw,340px)*var(--dos-l,.14));
  transform:translate(-50%,-50%) rotate(-90deg);
  display:flex;align-items:center;gap:calc(var(--cw,340px)*.02);
  padding:0 calc(var(--cw,340px)*.03);
  font-family:var(--dos-face);font-weight:var(--dos-weight);
  font-size:calc(var(--cw,340px)*var(--dos-size,.026));color:var(--dos-color,#191917);
  line-height:1;white-space:nowrap;
}
.dos-texte .esp{flex:1}
.planche-dims{color:rgba(255,255,255,.7);font-family:var(--mono);font-size:11px;text-align:center;margin-top:10px}
```

(Vérifier que `--mono` existe : `grep -n "\-\-mono" index.html` — il est utilisé par `input[type=file]`.)

- [x] **Step 4 : deux fieldsets vides dans le panneau**

Après la fermeture `</fieldset>` de `fsQ4Isbn` :

```html
      <fieldset id="fsAsm" class="off"><legend>Assemblage</legend></fieldset>
      <fieldset id="fsDos" class="off"><legend>Dos</legend></fieldset>
```

- [x] **Step 5 : `setTab` à trois états**

Remplacer le corps de `setTab` par :

```js
function setTab(v){
  tab = v;
  if (v !== 'une') selected = null;
  [...$('segTab').children].forEach(b => {
    if (b.tagName === 'BUTTON') b.setAttribute('aria-pressed', b.dataset.tab === v);
  });
  $('holderUne').classList.toggle('hide', v !== 'une');
  $('holderQuatre').classList.toggle('hide', v !== 'quatre');
  $('holderAsm').classList.toggle('hide', v !== 'assemblage');
  applyInspector(); render();
}
```

(La ligne `if (v === 'quatre') selected = null;` existante est généralisée en `v !== 'une'`.)

- [x] **Step 6 : `applyInspector` à trois états**

Remplacer le corps par :

```js
function applyInspector(){
  const q4 = tab === 'quatre', asm = tab === 'assemblage';
  const detail = !!selected && tab === 'une';
  $('fsGeneral').classList.toggle('off', detail || q4 || asm);
  $('fsNav').classList.toggle('off', detail || q4 || asm);
  $('btnBack').classList.toggle('off', !detail);
  for (const [name, ids] of Object.entries(ELEMENTS))
    ids.forEach(id => $(id).classList.toggle('off', q4 || asm || selected !== name));
  for (const id of ['fsQ4Fond','fsQ4Texte','fsQ4Pied','fsQ4Isbn'])
    $(id).classList.toggle('off', !q4);
  for (const id of ['fsAsm','fsDos'])
    $(id).classList.toggle('off', !asm);
}
```

- [x] **Step 7 : syntaxe** — extraction + `node --check`. Attendu : `Syntaxe OK`.

- [x] **Step 8 : vérification navigateur**

```js
setTab('assemblage');
JSON.stringify({
  asmVisible: !$('holderAsm').classList.contains('hide'),
  autresCaches: $('holderUne').classList.contains('hide') && $('holderQuatre').classList.contains('hide'),
  fieldsets: !$('fsAsm').classList.contains('off') && !$('fsDos').classList.contains('off') && $('fsGeneral').classList.contains('off'),
  dosLarge: $('dos').getBoundingClientRect().width > 10
});
```
Attendu : tout `true` (le dos s'affiche avec sa largeur de fallback). Retours `setTab('quatre')` et `setTab('une')` : états corrects, sélection d'élément intacte sur la 1ère. Terminer par `applyPreset('folio'); setTab('une'); render();`, serveur arrêté, onglet fermé.

- [x] **Step 9 : commit** — `git add index.html` ; message : « Onglet Assemblage : squelette de la planche et du dos ».

---

### Tâche 2 : PROVIDERS, contrôles d'assemblage et calculs

**Files :** Modify `index.html` (JS près de `PRESETS`, fieldset fsAsm, `render()`, objet `R`, CSS panneau)

- [x] **Step 1 : objet `PROVIDERS`**

Juste au-dessus de `const PRESETS = {` :

```js
/* ---------- prestataires d'impression ---------- */
const PROVIDERS = {
  lulu: {
    nom: 'Lulu (poche)',
    fondPerdu: 3.175,                     /* mm — 0,125 po */
    dos: pages => pages / 17.48 + 1.524   /* mm — formule Lulu, vérifiée sur 244 p → 15,48 mm */
  }
};
```

- [x] **Step 2 : contrôles du fieldset**

Remplacer `<fieldset id="fsAsm" class="off"><legend>Assemblage</legend></fieldset>` par :

```html
      <fieldset id="fsAsm" class="off">
        <legend>Assemblage</legend>
        <label><span class="lab">Prestataire</span>
          <select id="inAsmProvider"><option value="lulu" selected>Lulu (poche)</option></select></label>
        <label><span class="lab">Pages de l'intérieur — dos : <span class="val" id="vDosMm">15,48 mm</span></span>
          <input type="number" id="inAsmPages" min="32" max="800" step="2" value="244"></label>
        <p class="note">Le dos est calculé du nombre de pages. Si l'intérieur regénéré change de compte de pages, reporte le nouveau nombre ici avant d'exporter la planche.</p>
      </fieldset>
```

- [x] **Step 3 : CSS du champ nombre**

Ajouter `input[type=number]` à la règle partagée `input[type=text],select,textarea{…}` ET à la règle d'outline accent (`…:focus`).

- [x] **Step 4 : calculs dans `render()`**

Après le bloc des écritures `s4` (fin des réglages 4ème), ajouter :

```js
  /* --- assemblage : dimensions de la planche --- */
  const P = PROVIDERS[$('inAsmProvider').value];
  const dosMm = P.dos(+$('inAsmPages').value);
  const sp = $('plancheFp').style;
  sp.setProperty('--dos-l', dosMm / format[0]);
  sp.setProperty('--dos-hauteur', format[1] / format[0]);
  sp.setProperty('--fp', P.fondPerdu / format[0]);
  $('plancheDims').textContent = 'Planche ' + fr(2*format[0] + dosMm + 2*P.fondPerdu, 2) +
    ' × ' + fr(format[1] + 2*P.fondPerdu, 2) + ' mm — dos ' + fr(dosMm, 2) +
    ' mm — fond perdu ' + fr(P.fondPerdu, 2) + ' mm';
```

(`fr` est la fonction de formatage français déjà utilisée par les lectures `R` — vérifier sa signature avant usage.)

- [x] **Step 5 : lecture `R`** — ajouter dans l'objet `R` :

```js
    vDosMm:[dosMm, 2, ' mm'],
```

(`dosMm` est dans la portée de `render()` grâce au Step 4 — le bloc du Step 4 doit donc être AVANT l'objet `R` dans la fonction.)

- [x] **Step 6 : syntaxe** — extraction + `node --check`. Attendu : `Syntaxe OK`.

- [x] **Step 7 : vérification navigateur**

```js
setTab('assemblage');
$('inAsmPages').value = 244; render();
const a = $('vDosMm').textContent;            // « 15,48 mm »
$('inAsmPages').value = 400; render();
const b = $('vDosMm').textContent;            // « 24,41 mm » (400/17,48 + 1,524)
JSON.stringify({dos244: a, dos400: b, caption: $('plancheDims').textContent,
  dosPlusLarge: $('dos').getBoundingClientRect().width});
```
Attendu : `dos244:"15,48 mm"`, `dos400:"24,41 mm"`, caption « Planche 237,83 × 181,35 mm — dos 15,48 mm — fond perdu 3,18 mm » pour 244 p (au format 108×178 : 2×108+15,48+2×3,175 = 237,83), et la largeur écran du dos qui grandit entre 244 et 400 pages. Vérifier aussi que la frappe dans `inAsmPages` re-rend en direct (listener global input/select/textarea : le type number est un `input`, donc déjà couvert). Terminer par `applyPreset('folio'); setTab('une'); render();` (remettre 244 avant), serveur arrêté.

- [x] **Step 8 : commit** — « Assemblage : prestataire, pages et calcul du dos ».

---

### Tâche 3 : dos composé

**Files :** Modify `index.html` (fieldset fsDos, `render()`, boucle des polices, objet `R`)

- [x] **Step 1 : contrôles**

Remplacer `<fieldset id="fsDos" class="off"><legend>Dos</legend></fieldset>` par :

```html
      <fieldset id="fsDos" class="off">
        <legend>Dos</legend>
        <label><span class="lab">Police</span><select id="inDosFace"></select></label>
        <div class="row">
          <label><span class="lab">Graisse</span>
            <select id="inDosWeight"><option>300</option><option>400</option><option>500</option><option selected>600</option><option>700</option></select></label>
          <label><span class="lab">Corps <span class="val" id="vDosSize">2,6 %</span></span>
            <input type="range" id="inDosSize" min="1.5" max="5" step="0.1" value="2.6"></label>
        </div>
        <div class="row">
          <label><span class="lab">Couleur</span><input type="color" id="inDosColor" value="#191917"></label>
          <label><span class="lab">Fond</span>
            <select id="inDosBgMode"><option value="herite" selected>papier de la 1ère</option><option value="couleur">couleur distincte</option></select></label>
        </div>
        <label><span class="lab">Couleur du fond</span><input type="color" id="inDosBg" value="#fcf0d8"></label>
        <p class="note">Le dos reprend l'auteur, le titre et l'éditeur de la 1ère, en lecture de bas en haut.</p>
      </fieldset>
```

- [x] **Step 2 : `render()`**

À la suite du bloc « assemblage : dimensions » (tâche 2) :

```js
  sp.setProperty('--dos-face', $('inDosFace').value);
  sp.setProperty('--dos-weight', $('inDosWeight').value);
  sp.setProperty('--dos-size', +$('inDosSize').value / 100);
  sp.setProperty('--dos-color', $('inDosColor').value);
  $('dos').style.backgroundColor = $('inDosBgMode').value === 'couleur' ? $('inDosBg').value : $('inPaper').value;
  $('elDosAuthor').textContent = $('inAuthor').value.trim();
  $('elDosTitle').textContent = $('inTitle').value.trim();
  $('elDosEditor').textContent = $('inEditor').value.trim();
```

- [x] **Step 3 : polices et lecture** — ajouter `'inDosFace'` à la boucle de population des polices, et dans `R` :

```js
    vDosSize:[$('inDosSize').value,1,' %'],
```

- [x] **Step 4 : syntaxe** — extraction + `node --check`. Attendu : `Syntaxe OK`.

- [x] **Step 5 : vérification navigateur**

```js
applyPreset('folio'); setTab('assemblage');
$('inAuthor').value = 'Marguerite Duras'; $('inTitle').value = 'Le Ravissement'; render();
const t = $('dosTexte').getBoundingClientRect(), d = $('dos').getBoundingClientRect();
JSON.stringify({
  auteur: $('elDosAuthor').textContent, titre: $('elDosTitle').textContent,
  pivote: t.width < t.height,          /* la boîte pivotée est plus haute que large à l'écran */
  dansLeDos: Math.abs((t.left + t.width/2) - (d.left + d.width/2)) < 2,
  fondHerite: getComputedStyle($('dos')).backgroundColor === getComputedStyle(cover).backgroundColor
});
```
Attendu : textes corrects, `pivote:true`, `dansLeDos:true`, `fondHerite:true`. Basculer `inDosBgMode` sur couleur → fond du dos = `inDosBg`. Screenshot du dos : texte lisible de bas en haut (auteur en bas, éditeur en haut). Terminer proprement (preset folio, onglet 1ère), serveur arrêté.

- [x] **Step 6 : commit** — « Assemblage : dos composé (auteur, titre, éditeur) ».

---

### Tâche 4 : construction de la planche (clones et échelle)

**Files :** Modify `index.html` (fonction `buildPlanche`, appel dans `render()`)

- [x] **Step 1 : fonction `buildPlanche`**

Juste avant `render()` (ou juste après, au même niveau) :

```js
/* ---------- assemblage : la planche est reconstruite à chaque render() de l'onglet.
   Clones : tout le style des couvertures vient de classes et de variables inline,
   un cloneNode(true) rend donc à l'identique ; on retire seulement les ids
   (uniques dans le document) et on impose l'échelle de la planche. ---------- */
function buildPlanche(){
  const P = PROVIDERS[$('inAsmProvider').value];
  const dosMm = P.dos(+$('inAsmPages').value);
  const largeurMm = 2 * format[0] + dosMm + 2 * P.fondPerdu;
  const stage = document.querySelector('.stage');
  const cwUne = parseFloat(getComputedStyle(cover).getPropertyValue('--cw')) || 340;
  const cwPl = Math.max(120, Math.min(cwUne, (stage.clientWidth - 160) * format[0] / largeurMm));
  $('plancheFp').style.setProperty('--cw', cwPl + 'px');
  const c4 = cover4.cloneNode(true), c1 = cover.cloneNode(true);
  for (const c of [c4, c1]) {
    c.removeAttribute('id');
    c.querySelectorAll('[id]').forEach(e => e.removeAttribute('id'));
    c.style.setProperty('--cw', cwPl + 'px');
  }
  $('planche').replaceChildren(c4, $('dos'), c1);
}
```

- [x] **Step 2 : appel dans `render()`**

À la fin du bloc assemblage (après les écritures du dos, avant l'objet `R`) :

```js
  if (tab === 'assemblage') buildPlanche();
```

- [x] **Step 3 : redimensionnement**

Vérifier (grep `addEventListener('resize'`) que le handler de resize enchaîne `fitCover()` puis `render()` — si oui, rien à faire (la planche se reconstruit via render). Si le handler n'appelle pas `render()`, l'y ajouter.

- [x] **Step 4 : syntaxe** — extraction + `node --check`. Attendu : `Syntaxe OK`.

- [x] **Step 5 : vérification navigateur**

```js
applyPreset('folio'); render(); setTab('assemblage');
const panneaux = [...$('planche').children];
const r = panneaux.map(e => Math.round(e.getBoundingClientRect().width));
const ids = $('planche').querySelectorAll('.cover [id], .cover[id]').length;
const stage = document.querySelector('.stage');
JSON.stringify({
  troisPanneaux: panneaux.length === 3,
  ordre: panneaux[1] === $('dos'),
  largeurs: r,                                   /* [cw4, dos, cw1] : cw4 === cw1 */
  couvEgales: r[0] === r[2],
  pasDIdsDupliques: ids === 0,
  tientDansLaScene: $('plancheFp').getBoundingClientRect().width <= stage.clientWidth
});
```
Attendu : `troisPanneaux:true`, `ordre:true` (4ème à gauche, dos au centre, 1ère à droite), `couvEgales:true`, `pasDIdsDupliques:0 → true`, `tientDansLaScene:true`. Vérifier aussi la réactivité : modifier `inTitle` pendant l'onglet assemblage → le clone de la 1ère affiche le nouveau titre au render suivant (le listener global re-rend à la frappe). Screenshot de la planche complète. Vérifier que `document.getElementById('cover')` retourne toujours l'original (dans `#holderUne`). Terminer proprement, serveur arrêté.

- [x] **Step 6 : commit** — « Assemblage : planche complète par clones des deux couvertures ».

---

### Tâche 5 : applyConfig déterministe (correctif consigné au lot 1)

**Files :** Modify `index.html` (`applyConfig`, init)

- [x] **Step 1 : capture des défauts**

Dans le bloc d'init en bas de script, JUSTE AVANT l'appel à `loadLocal()` (ou l'expression `loadLocal() || …`) :

```js
/* état neuf du panneau, capturé avant tout chargement : sert de base aux chargements partiels */
const DEFAULTS = collectConfig().fields;
```

- [x] **Step 2 : remise aux défauts avant application**

Dans `applyConfig`, juste avant la boucle `for (const [k, v] of Object.entries(c.fields))` :

```js
  for (const [k, v] of Object.entries(DEFAULTS)) {
    const el = $(k); if (!el || el.type === 'file') continue;
    if (el.type === 'checkbox') el.checked = !!v; else el.value = v;
  }
```

(Un PNG/JSON d'avant le lot 1, sans clés `inQ4*`/`inDos*`/`inAsm*`, ramène désormais ces contrôles aux défauts au lieu de laisser l'état courant. `DEFAULTS` est un `const` top-level initialisé avant le premier appel réel d'`applyConfig` — `loadLocal()` est appelé après sa déclaration.)

- [x] **Step 3 : syntaxe** — extraction + `node --check`. Attendu : `Syntaxe OK`.

- [x] **Step 4 : vérification navigateur**

```js
$('inQ4Text').value = 'résidu'; $('inAsmPages').value = 500; render();
const vieux = collectConfig();
delete vieux.fields.inQ4Text; delete vieux.fields.inAsmPages;   /* simule un PNG d'avant les lots */
applyConfig(vieux); render();
JSON.stringify({q4Reset: $('inQ4Text').value === '', pagesReset: $('inAsmPages').value === '244',
  titreConserve: $('inTitle').value === vieux.fields.inTitle});
```
Attendu : les trois `true` (les clés absentes reviennent aux défauts, les présentes s'appliquent). Vérifier aussi qu'un rechargement de page restaure toujours la session normale. Terminer proprement, serveur arrêté.

- [x] **Step 5 : commit** — « applyConfig : chargements partiels ramenés aux défauts ».

---

### Tâche 6 : persistance, presets et revue du lot

**Files :** Modify `index.html` (`PRESETS`), `docs/superpowers/plans/2026-08-18-lot2-assemblage.md`

- [x] **Step 1 : valeurs dans les trois `PRESETS`**

Ajouter à CHACUNE des trois entrées, après les clés `inQ4*` :

```js
    inAsmProvider:'lulu', inAsmPages:244,
    inDosFace:F['Archivo'], inDosWeight:'600', inDosSize:2.6, inDosColor:'#191917',
    inDosBgMode:'herite', inDosBg:'#fcf0d8',
```

**Exception** : pour le preset `overlay` (papier `#000000` hérité par le dos),
`inDosColor:'#f4efe4'` — texte crème sur dos noir, cohérent avec sa couverture ;
`#191917` y serait illisible (relevé en revue qualité T3).

(Ponctuation locale : virgule finale selon la position. Invariant à vérifier : chaque contrôle `inAsm*`/`inDos*` du panneau a sa clé — grep croisé `id="inAsm` / `id="inDos` vs les clés.)

- [x] **Step 2 : syntaxe** — extraction + `node --check`. Attendu : `Syntaxe OK`.

- [x] **Step 3 : round-trip et non-régression**

En navigateur :
a) Round-trip : régler `inAsmPages:320`, `inDosBgMode:'couleur'`, `inDosBg:'#101010'`, `inDosSize:'3.4'` ; `cfg = collectConfig()` ; `applyPreset('folio')` ; `applyConfig(cfg)` → les quatre valeurs reviennent.
b) `applyPreset('blanche')` réinitialise l'assemblage (`inAsmPages === '244'`, `inDosBgMode === 'herite'`).
c) Les trois presets × trois modes × trois onglets : aucune erreur console nouvelle (404 favicon tolérée).
d) Depuis l'onglet assemblage, cliquer « Exporter PNG » n'est pas testé jusqu'au dialogue, mais vérifier que la garde du handler ramène bien sur la 1ère (`setTab('assemblage')` puis simuler la première ligne du handler : `if (tab !== 'une') setTab('une');` → `tab === 'une'`).
e) Passe visuelle : screenshot de la planche au preset folio avec le pied et la zone ISBN de la 4ème activés — composition d'ensemble plausible, dos lisible, caption des dimensions exacte.

- [x] **Step 4 : cocher le plan** — passer toutes les cases de ce fichier à `- [x]`.

- [x] **Step 5 : commit** — « Assemblage : persistance et presets ; lot 2 vérifié » (index.html + ce plan).
