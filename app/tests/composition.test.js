'use strict';

const test = require('node:test');
const assert = require('node:assert');
const { charge } = require('./dom_shim');

const IDS = [
  'btOuvrir', 'btImporter', 'btEnregistrer', 'cheminProjet',
  'secLivre', 'secManuscrit', 'secCouverture', 'secComposer',
  'inTitre', 'inTitrePage', 'inAuteur', 'inGenre', 'inCopyright', 'inChapitres',
  'etatManuscrit', 'sourceManuscrit', 'btReimporter', 'btChoisirManuscrit',
  'etatImages', 'maquettes', 'etatCouverture', 'faces', 'apercu', 'etatApercu',
  'reglages',
  'inProvider', 'inPapier', 'noteFormat',
  'btComposer', 'etat', 'resultat',
  'secPackages', 'listePrestataires', 'btPackager', 'etatPackages', 'packages',
  'secInterieur', 'inPoliceInterieur',
  'secEpreuve', 'inEpreuveCorps', 'btEpreuve', 'etatEpreuve', 'cheminEpreuve',
];

const LULU = {
  cle: 'lulu', libelle: 'Lulu — poche 108 × 175',
  largeur: 108, hauteur: 175, fond_perdu: 3.175, dos_publie: true,
  papiers: [{ cle: 'standard', libelle: 'Papier standard' }],
};
const KDP = {
  cle: 'kdp-6x9', libelle: 'Amazon KDP — 6 × 9 po',
  largeur: 152.4, hauteur: 228.6, fond_perdu: 3.175, dos_publie: true,
  papiers: [{ cle: 'creme', libelle: 'Crème' }, { cle: 'blanc', libelle: 'Blanc' }],
};
const COOLLIBRI = {
  cle: 'coollibri-148x210', libelle: 'CoolLibri — A5',
  largeur: 148, hauteur: 210, fond_perdu: null, dos_publie: false,
  papiers: [{ cle: 'mesure', libelle: 'Dos relevé sur le gabarit' }],
};

const PROJET = {
  chemin: '/livres/LHC.ozalid',
  livre: {
    titre: 'Les Heures creuses', titre_page: 'Les Heures\ncreuses',
    auteur: 'Ivan Pjig', genre: 'roman', copyright: '© Ivan Pjig, 2026.',
    chapitres: 64,
  },
  manuscrit_source: '/dev/ozalid/build/in/texts/WIP7.md',
  chapitres_trouves: 64,
  mots: 49344,
  couverture_importee: true,
  images: ['couverture.jpg'],
  interieur: { police: 'Alegreya' },
};

/** Fausse implémentation des commandes Rust. `sur` surcharge une commande. */
function faux(providers, sur = {}) {
  return async (cmd, args) => {
    if (cmd === 'providers_liste') return providers;
    if (cmd === 'polices_liste') return ['Bodoni Moda', 'Archivo', 'Spectral'];
    if (cmd === 'polices_texte_liste') return ['EB Garamond', 'Alegreya', 'Cardo'];
    if (cmd === 'maquettes_liste') {
      return [{ cle: 'folio', libelle: 'Folio' }, { cle: 'blanche', libelle: 'Blanche' }];
    }
    if (cmd === 'couverture_apercu') return 'data:image/png;base64,AAAA';
    if (cmd in sur) {
      const v = sur[cmd];
      return typeof v === 'function' ? v(args) : v;
    }
    throw new Error(`commande inattendue : ${cmd}`);
  };
}

const COMPOSITION = {
  pages: 278, chapitres: 64, gouttiere: 25, blanche: true,
  dos: 17.427, pdf: '/livres/LHC/lulu/interieur-lulu.pdf',
};

/* ---------- prestataires ---------- */

test('le choix du papier n\'est offert que quand il y en a plusieurs', async () => {
  const { els } = await charge({ ids: IDS, invoke: faux([LULU, KDP]) });
  assert.strictEqual(els.get('inPapier').disabled, true);
  assert.strictEqual(els.get('inPapier').children.length, 1);

  els.get('inProvider').value = 'kdp-6x9';
  await els.get('inProvider').declenche('change');
  assert.strictEqual(els.get('inPapier').disabled, false);
  assert.deepStrictEqual(
    els.get('inPapier').children.map((o) => o.value),
    ['creme', 'blanc']
  );
});

test('un prestataire à gabarit annonce que le fond perdu se relève', async () => {
  const { els } = await charge({ ids: IDS, invoke: faux([COOLLIBRI]) });
  const note = els.get('noteFormat').textContent;
  assert.match(note, /148,0 × 210,0 mm/);
  assert.match(note, /relever sur le gabarit/);
  assert.doesNotMatch(note, /fond perdu \d/, 'aucun chiffre de fond perdu inventé');
});

/* ---------- projet ---------- */

test('rien n\'est proposé tant qu\'aucun projet n\'est ouvert', async () => {
  const { els } = await charge({ ids: IDS, invoke: faux([LULU]) });
  for (const s of ['secLivre', 'secManuscrit', 'secComposer']) {
    assert.strictEqual(els.get(s).hidden, true, `${s} visible sans projet`);
  }
  assert.strictEqual(els.get('btEnregistrer').disabled, true);
});

test('un projet importé remplit les champs et ouvre les sections', async () => {
  const { els } = await charge({
    ids: IDS,
    invoke: faux([LULU], { projet_importer: PROJET }),
    open: async () => '/dev/ozalid/build/LHC/livre.toml',
  });
  await els.get('btImporter').declenche('click');

  assert.strictEqual(els.get('inTitre').value, 'Les Heures creuses');
  assert.strictEqual(els.get('inTitrePage').value, 'Les Heures\ncreuses');
  assert.strictEqual(els.get('inChapitres').value, 64);
  assert.strictEqual(els.get('secComposer').hidden, false);
  assert.strictEqual(els.get('btEnregistrer').disabled, false);
  assert.match(els.get('etatImages').textContent, /couverture\.jpg/);
});

/**
 * L'écart entre les chapitres attendus et ceux du manuscrit embarqué est le seul
 * signe qu'un manuscrit est périmé : le `.ozalid` en porte une copie, corriger le
 * fichier d'origine ne la met pas à jour. Le taire livrerait un livre amputé.
 */
test('un manuscrit périmé est signalé, pas passé sous silence', async () => {
  const perime = { ...PROJET, chapitres_trouves: 61 };
  const { els } = await charge({
    ids: IDS,
    invoke: faux([LULU], { projet_ouvrir: perime }),
    open: async () => '/livres/LHC.ozalid',
  });
  await els.get('btOuvrir').declenche('click');

  const t = els.get('etatManuscrit');
  assert.match(t.textContent, /61 chapitres/);
  assert.match(t.textContent, /64 attendus/);
  assert.match(t.textContent, /périmé/);
  assert.strictEqual(t.className, 'note alerte');
});

test('un manuscrit conforme n\'affiche aucune alerte', async () => {
  const { els } = await charge({
    ids: IDS,
    invoke: faux([LULU], { projet_ouvrir: PROJET }),
    open: async () => '/livres/LHC.ozalid',
  });
  await els.get('btOuvrir').declenche('click');
  assert.strictEqual(els.get('etatManuscrit').className, 'note');
  // \s : toLocaleString('fr-FR') sépare les milliers par une espace fine insécable.
  assert.match(els.get('etatManuscrit').textContent, /49\s344 mots/);
});

/**
 * « Réimporter » relit la source d'origine. Sans source mémorisée le bouton n'a rien
 * à relire : le laisser actif promettrait une action qui échouerait.
 */
test('réimporter n\'est offert que si une source est mémorisée', async () => {
  const sansSource = { ...PROJET, manuscrit_source: null };
  const { els } = await charge({
    ids: IDS,
    invoke: faux([LULU], { projet_ouvrir: sansSource }),
    open: async () => '/livres/X.ozalid',
  });
  await els.get('btOuvrir').declenche('click');
  assert.strictEqual(els.get('btReimporter').disabled, true);
  assert.match(els.get('sourceManuscrit').textContent, /aucune source/);
});

test('un projet non enregistré le dit, et reste enregistrable', async () => {
  const neuf = { ...PROJET, chemin: null };
  const { els } = await charge({
    ids: IDS,
    invoke: faux([LULU], { projet_importer: neuf }),
    open: async () => '/x/livre.toml',
  });
  await els.get('btImporter').declenche('click');
  assert.match(els.get('cheminProjet').textContent, /non enregistré/);
  assert.strictEqual(els.get('btEnregistrer').disabled, false);
});

/* ---------- composition ---------- */

test('le dos calculé est affiché avec le compte de pages qui le produit', async () => {
  const { els } = await charge({
    ids: IDS,
    invoke: faux([LULU], { projet_ouvrir: PROJET, composer: COMPOSITION }),
    open: async () => '/livres/LHC.ozalid',
  });
  await els.get('btOuvrir').declenche('click');
  await els.get('btComposer').declenche('click');

  const res = els.get('resultat');
  assert.strictEqual(res.hidden, false);
  assert.deepStrictEqual(res.textes('dd'), [
    '278', '64', '25,00 mm', 'ajoutée (parité)', '17,43 mm',
  ]);
});

/**
 * Le cœur du projet : le dos ne doit jamais apparaître comme un nombre quand le
 * prestataire n'en publie pas de formule. Un « 0,00 mm » affiché ici enverrait une
 * planche fausse à l'impression sans que rien ne l'ait signalé.
 */
test('un prestataire sans formule n\'affiche jamais de dos chiffré', async () => {
  const { els } = await charge({
    ids: IDS,
    invoke: faux([COOLLIBRI], {
      projet_ouvrir: PROJET,
      composer: { ...COMPOSITION, pages: 190, dos: null },
    }),
    open: async () => '/livres/LHC.ozalid',
  });
  await els.get('btOuvrir').declenche('click');
  await els.get('btComposer').declenche('click');

  const dos = els.get('resultat').textes('dd').at(-1);
  assert.match(dos, /relever sur le gabarit/);
  assert.doesNotMatch(dos, /\d/, `dos chiffré affiché : « ${dos} »`);
});

/**
 * Une erreur de la chaîne doit rester lisible, et surtout ne pas laisser croire à une
 * composition réussie en gardant un résultat précédent affiché.
 */
test('une erreur de composition efface le résultat précédent', async () => {
  let echoue = false;
  const base = faux([LULU], { projet_ouvrir: PROJET });
  const invoke = async (cmd, args) => {
    if (cmd === 'composer') {
      if (echoue) throw '64 chapitres attendus (projet), 61 trouvés.';
      return COMPOSITION;
    }
    return base(cmd, args);
  };
  const { els } = await charge({
    ids: IDS, invoke, open: async () => '/livres/LHC.ozalid',
  });
  await els.get('btOuvrir').declenche('click');
  await els.get('btComposer').declenche('click');
  assert.strictEqual(els.get('resultat').hidden, false);

  echoue = true;
  await els.get('btComposer').declenche('click');
  assert.strictEqual(els.get('resultat').hidden, true, 'résultat périmé laissé à l\'écran');
  assert.match(els.get('etat').textContent, /64 chapitres attendus/);
  assert.strictEqual(els.get('etat').className, 'etat erreur');
  assert.strictEqual(els.get('btComposer').disabled, false, 'bouton laissé bloqué');
});

/** Une erreur d'ouverture doit s'afficher, pas disparaître dans la console. */
test('un fichier qui n\'est pas un projet est signalé à l\'écran', async () => {
  const { els } = await charge({
    ids: IDS,
    invoke: faux([LULU], {
      projet_ouvrir: () => {
        throw 'archive sans projet.toml : ce n\'est pas un projet Ozalid.';
      },
    }),
    open: async () => '/x/photos.zip',
  });
  await els.get('btOuvrir').declenche('click');
  assert.match(els.get('etat').textContent, /pas un projet Ozalid/);
  assert.strictEqual(els.get('etat').className, 'etat erreur');
  assert.strictEqual(els.get('secLivre').hidden, true, 'sections ouvertes sur un échec');
});
