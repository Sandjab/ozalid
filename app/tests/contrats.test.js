'use strict';

// Les deux gardes de contrat versées par le lot 2 de la coquille : des chaînes et des
// nombres que le Rust, le JSON et le CSS se promettent sans qu'aucun compilateur ne
// les confronte. Chaque test lit les vrais fichiers, comme `dom_shim.js` lit le vrai
// `index.html` — c'est un couplage assumé : il casse quand le contrat casse.

const test = require('node:test');
const assert = require('node:assert');
const fs = require('node:fs');
const path = require('node:path');
const { charge } = require('./dom_shim');

const LULU = {
  cle: 'lulu', libelle: 'Lulu — poche 108 × 175',
  largeur: 108, hauteur: 175, fond_perdu: 3.175, dos_publie: true,
  papiers: [{ cle: 'standard', libelle: 'Papier standard' }],
};

/** Le strict nécessaire pour que l'application charge et route un menu. */
const invoke = async (cmd) => {
  if (cmd === 'providers_liste') return [LULU];
  if (cmd === 'polices_liste') return ['Archivo'];
  if (cmd === 'polices_texte_liste') return ['Alegreya'];
  if (cmd === 'mains_liste') return ['Caveat'];
  if (cmd === 'maquettes_liste') return [{ cle: 'folio', libelle: 'Folio' }];
  if (cmd === 'recents_liste') return [];
  if (cmd === 'garde_modifications') return 'ignorer';
  if (cmd === 'interface_prete') return null;
  if (cmd === 'diffusion_lire') return { url: '', cle_posee: false };
  // Une entrée peut déclencher une commande que ce faux ignore : l'action échoue,
  // mais elle échoue *nommée* — ce n'est pas « entrée de menu inconnue », et c'est
  // tout ce que la garde vérifie.
  throw new Error(`commande inattendue : ${cmd}`);
};

const source = (...morceaux) =>
  fs.readFileSync(path.join(__dirname, '..', ...morceaux), 'utf8');

/* ---------- menu.rs → MENU ---------- */

/**
 * Chaque identifiant que `menu.rs` déclare doit mener quelque part dans `app.js` :
 * une clé renommée d'un seul côté rendrait l'entrée et son accélérateur inertes.
 * Le routage est exercé pour de vrai — `routerMenu` via l'événement `menu` — plutôt
 * que comparé à une liste recopiée qui divergerait à son tour.
 */
test('chaque entrée de menu du Rust mène quelque part dans le front', async () => {
  const ids = [...new Set(
    [...source('src-tauri', 'src', 'menu.rs').matchAll(/with_id\("([^"]+)"/g)]
      .map((m) => m[1])
  )];
  // Onze au lot 2 : si le relevé s'effondre, c'est la moisson qui est cassée, pas le
  // menu — le dire distinctement.
  assert.ok(ids.length >= 11, `moisson suspecte : ${ids.length} identifiants (${ids})`);

  for (const id of ids) {
    const { els, menu } = await charge({ invoke, open: async () => null });
    // L'action derrière l'entrée peut échouer sur ce faux minimal — c'est son droit :
    // une entrée inconnue n'échoue pas, elle s'écrit dans l'alerte avant toute action,
    // et c'est elle seule que la garde regarde.
    await menu(id).catch(() => {});
    assert.doesNotMatch(
      els.get('alerte').textContent,
      /entrée de menu inconnue/,
      `« ${id} » déclaré dans menu.rs, inconnu de MENU`
    );
  }
});

/** Le contrôle de la garde : une clé de travers doit bien se faire nommer. */
test('une entrée de menu inconnue est nommée, pas avalée', async () => {
  const { els, menu } = await charge({ invoke, open: async () => null });
  await menu('aller.nulle_part');
  assert.match(els.get('alerte').textContent, /entrée de menu inconnue : aller\.nulle_part/);
});

/* ---------- styles.css → tauri.conf.json ---------- */

/**
 * L'addition des 848 px, refaite au lieu d'être promise en commentaire : deux
 * colonnes réclament 2 × `columns` + `column-gap` + les 2 × 2rem de `main`, et la
 * fenêtre minimale de `tauri.conf.json` doit les offrir. Qui touche à l'un de ces
 * nombres sans refaire le compte retombe à une colonne à la taille minimale — et
 * c'est l'ascenseur qui revient, sans qu'aucun autre test ne le voie.
 */
test('deux colonnes tiennent dans la fenêtre minimale', () => {
  const css = source('src', 'styles.css');

  const etapes = css.match(/#etapeLivre, #etapeInterieur \{[^}]*\}/s);
  assert.ok(etapes, 'la règle #etapeLivre, #etapeInterieur a changé de forme');
  const colonne = etapes[0].match(/columns: ([\d.]+)rem/);
  const gouttiere = etapes[0].match(/column-gap: ([\d.]+)rem/);
  assert.ok(colonne && gouttiere, `colonnes illisibles dans : ${etapes[0]}`);

  const main = css.match(/\nmain \{[^}]*\}/s);
  assert.ok(main, 'la règle main a changé de forme');
  const rembourrage = main[0].match(/padding: 0 ([\d.]+)rem/);
  assert.ok(rembourrage, `rembourrage illisible dans : ${main[0]}`);

  const requis = (2 * Number(colonne[1]) + Number(gouttiere[1])
    + 2 * Number(rembourrage[1])) * 16;

  const conf = JSON.parse(source('src-tauri', 'tauri.conf.json'));
  const { minWidth } = conf.app.windows[0];
  assert.ok(
    requis <= minWidth,
    `deux colonnes réclament ${requis} px, minWidth n'en donne que ${minWidth}`
  );
});
