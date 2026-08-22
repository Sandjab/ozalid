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

const PROJET = {
  chemin: '/livres/LHC.ozalid',
  livre: {
    titre: 'Les Heures creuses', titre_page: null, auteur: 'Ivan Pjig',
    genre: 'roman', copyright: '', chapitres: null,
  },
  manuscrit_source: null,
  chapitres_trouves: 12,
  mots: 42000,
  manuscrit_absent: false,
  modifie: true,
  couverture: null,
  couverture_importee: false,
  images: [],
  interieur: { police: 'Alegreya' },
  livraison: {
    destinataires: [{ provider: 'lulu', papier: 'standard', dos_mm: null, fond_perdu_mm: null }],
    courant: 'lulu',
  },
  envois: { main: { mode: 'police', police: 'Caveat' }, liste: [] },
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

/**
 * Le même, mais qui répond aux gestes du cycle de vie et retient ce qu'on lui demande.
 *
 * La première garde répond toujours « ignorer » : il faut un projet ouvert pour que la
 * question ait un sens, et le seul chemin qui en ouvre un passe déjà par elle.
 */
function atelier(reponseDeGarde) {
  const appels = [];
  let premiere = true;
  return {
    appels,
    invoke: async (cmd, args) => {
      appels.push(cmd);
      if (cmd === 'garde_modifications') {
        if (!premiere) return reponseDeGarde;
        premiere = false;
        return 'ignorer';
      }
      if (cmd === 'couverture_apercu') throw new Error('pas de maquette');
      if (cmd === 'projet_fermer') return null;
      try {
        return await invoke(cmd, args);
      } catch {
        // Tout ce qui rend un projet : `projet_nouveau`, `projet_ouvrir`,
        // `projet_enregistrer`. Les distinguer n'apprendrait rien de plus ici.
        return PROJET;
      }
    },
  };
}

/** Le corps d'une fonction Rust, de son accolade ouvrante à celle qui la ferme seule. */
function corps(rust, signature) {
  const debut = rust.indexOf(signature);
  assert.notEqual(debut, -1, `${signature} introuvable`);
  const fin = rust.indexOf('\n}', debut);
  assert.notEqual(fin, -1, `${signature} sans fin`);
  return rust.slice(debut, fin);
}

/** La valeur d'une constante Rust `pub const NOM: &str = "…";`. */
function constante(rust, nom) {
  const m = rust.match(new RegExp(`const ${nom}: &str = "([^"]+)"`));
  assert.ok(m, `constante ${nom} introuvable`);
  return m[1];
}

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

/* ---------- commands.rs → garde() ---------- */

/**
 * Les trois réponses de la boîte de garde sont écrites en toutes lettres des deux
 * côtés, et le front penche du côté qui ne perd rien : ce qu'il ne reconnaît pas vaut
 * « annuler ». Ce défaut rend une divergence inoffensive, et c'est précisément ce qui
 * la rend invisible — un « ignorer » renommé d'un seul côté ne casse rien, il refuse
 * en silence tout ce qu'on lui demande, et l'application paraît sourde.
 *
 * La garde ne recopie pas les trois mots : elle les relève dans `reponse_garde` et
 * regarde ce que chacun *fait*. Une seule réponse doit arrêter le geste, une seule
 * doit enregistrer avant de le laisser passer. Deux refus, c'est un mot de travers.
 */
test('chaque réponse de la garde du Rust est comprise du front', async () => {
  const rust = corps(source('src-tauri', 'src', 'commands.rs'), 'fn reponse_garde');
  const reponses = [...new Set([...rust.matchAll(/=> "([a-zé]+)"/g)].map((m) => m[1]))];
  assert.equal(reponses.length, 3, `moisson suspecte : ${reponses.join(', ')}`);

  const effets = [];
  for (const r of reponses) {
    const a = atelier(r);
    const { els } = await charge({ invoke: a.invoke, open: async () => null });
    // Le premier ouvre un projet — modifié, comme le veut `PROJET` — et c'est le
    // second qui pose la question dont on regarde la réponse.
    await els.get('btNouveau').declenche('click');
    await els.get('btNouveau').declenche('click');
    const combien = (cmd) => a.appels.filter((c) => c === cmd).length;
    effets.push({
      reponse: r,
      passe: combien('projet_nouveau') === 2,
      enregistre: combien('projet_enregistrer') === 1,
    });
  }

  const refus = effets.filter((e) => !e.passe);
  assert.equal(refus.length, 1,
    `${refus.length} réponses arrêtent le geste : ${refus.map((e) => e.reponse).join(', ')}`);
  const ecrit = effets.filter((e) => e.enregistre);
  assert.equal(ecrit.length, 1,
    `${ecrit.length} réponses enregistrent : ${ecrit.map((e) => e.reponse).join(', ')}`);
});

/* ---------- lib.rs → listen() ---------- */

/**
 * Les deux événements que le Rust envoie à l'interface. Un nom qui diverge ne lève
 * rien nulle part : l'émission part dans le vide, le menu et la fermeture deviennent
 * inertes, et le Rust n'a même pas de quoi s'en apercevoir.
 *
 * Les noms sont relevés sur les `emit` de `lib.rs`, constantes résolues, et confrontés
 * à ce que l'application écoute réellement au chargement — pas à une liste tenue à
 * côté, qui divergerait à son tour.
 */
test('chaque événement émis par le Rust est écouté par le front', async () => {
  const lib = source('src-tauri', 'src', 'lib.rs');
  const menuRs = source('src-tauri', 'src', 'menu.rs');
  const emis = [...new Set(
    [...lib.matchAll(/\.emit\(\s*(?:\w+::)?([A-Z_]+|"[^"]+")/g)]
      .map((m) => (m[1].startsWith('"') ? m[1].slice(1, -1) : constante(menuRs, m[1])))
  )];
  assert.equal(emis.length, 2, `moisson suspecte : ${emis.join(', ')}`);

  const ecoutes = [];
  await charge({
    invoke,
    listen: async (nom) => {
      ecoutes.push(nom);
      return () => {};
    },
  });

  for (const nom of emis) {
    assert.ok(ecoutes.includes(nom),
      `« ${nom} » émis par le Rust, écouté par personne (posés : ${ecoutes.join(', ')})`);
  }
});

/* ---------- menu.rs → RECENT ---------- */

/**
 * Le préfixe qui distingue « ouvrir un récent » du reste du menu, et dont ce qui suit
 * est un chemin. Écrit des deux côtés ; divergent, l'entrée ne serait plus reconnue
 * comme un récent et irait chercher une action de ce nom — « entrée de menu inconnue »
 * sur un clic parfaitement légitime.
 *
 * Le préfixe du Rust est exercé, non comparé : c'est le routage qui doit en faire un
 * chemin, jusqu'à `projet_ouvrir`.
 */
test('un récent du Rust est reconnu comme récent par le front', async () => {
  const prefixe = constante(source('src-tauri', 'src', 'menu.rs'), 'RECENT');
  const a = atelier('ignorer');
  const { els, menu } = await charge({ invoke: a.invoke, open: async () => null });

  await menu(`${prefixe}/livres/A.ozalid`);

  assert.ok(a.appels.includes('projet_ouvrir'),
    `« ${prefixe} » n'a pas été reconnu comme préfixe de récent`);
  assert.equal(els.get('alerte').textContent, '');
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

  const etapes = css.match(/#etapeLivre, #etapeInterieur, #etapeEnvois \{[^}]*\}/s);
  assert.ok(etapes, 'la règle des étapes en colonnes a changé de forme');
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
