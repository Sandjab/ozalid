'use strict';

// Câblage de l'étape « Packages » : ce que l'interface envoie au Rust, et ce qu'elle
// en montre. Le rendu des planches se vérifie dans l'application, pas ici.

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
    titre: 'Les Heures creuses', titre_page: null, auteur: 'Ivan Pjig',
    genre: 'roman', copyright: '', chapitres: 64,
  },
  manuscrit_source: '/x/WIP7.md',
  chapitres_trouves: 64,
  mots: 49344,
  couverture: null,
  couverture_importee: false,
  images: ['couverture.jpg'],
  interieur: { police: 'Alegreya' },
};

const COMPOSITION = {
  pages: 278, chapitres: 64, gouttiere: 25, blanche: true,
  dos: 17.427, pdf: '/livres/LHC/lulu/interieur-lulu.pdf',
};

function paquet(sur = {}) {
  return {
    provider: 'lulu',
    libelle: 'Lulu — poche 108 × 175',
    papier: 'Papier standard',
    pages: 278,
    gouttiere: 25,
    blanche: true,
    dos: 17.427,
    fond_perdu: 3.175,
    planche: [239.779, 181.35],
    chemins: ['/livres/LHC/lulu/interieur-lulu.pdf', '/livres/LHC/lulu/couverture-lulu.pdf'],
    ...sur,
  };
}

/** Un projet ouvert, prêt pour l'étape packages. */
async function ouvre(providers, sur = {}) {
  const appels = [];
  const invoke = async (cmd, args) => {
    appels.push([cmd, args]);
    if (cmd in sur) {
      const v = sur[cmd];
      return typeof v === 'function' ? v(args) : v;
    }
    if (cmd === 'providers_liste') return providers;
    if (cmd === 'polices_liste') return ['Archivo', 'Spectral'];
    if (cmd === 'polices_texte_liste') return ['EB Garamond', 'Alegreya', 'Cardo'];
    if (cmd === 'maquettes_liste') return [{ cle: 'folio', libelle: 'Folio' }];
    if (cmd === 'projet_ouvrir') return PROJET;
    if (cmd === 'couverture_apercu') return 'data:image/png;base64,QUJD';
    throw new Error(`commande inattendue : ${cmd}`);
  };
  const ctx = await charge({ ids: IDS, invoke, open: async () => '/livres/LHC.ozalid' });
  await ctx.els.get('btOuvrir').declenche('click');
  return { ...ctx, appels };
}

const attendreApercu = () => new Promise((r) => setTimeout(r, 300));

/* ---------- relevés ---------- */

/**
 * Un prestataire qui publie sa formule n'a rien à faire saisir : offrir un champ de
 * dos donnerait à croire qu'il compte, alors que la formule prime toujours.
 */
test('seul un prestataire à gabarit demande un relevé', async () => {
  const { els } = await ouvre([LULU, COOLLIBRI]);
  assert.ok(!els.get('pkg-dos-lulu'), 'dos saisissable chez Lulu');
  assert.ok(!els.get('pkg-fp-lulu'), 'fond perdu saisissable chez Lulu');
  assert.ok(els.get('pkg-dos-coollibri-148x210'), 'dos non demandé chez CoolLibri');
  assert.ok(els.get('pkg-fp-coollibri-148x210'), 'fond perdu non demandé chez CoolLibri');
});

/* ---------- génération ---------- */

test('les prestataires cochés sont envoyés avec leur papier et leurs relevés', async () => {
  let recu = null;
  const { els } = await ouvre([LULU, KDP, COOLLIBRI], {
    packager: ({ choix }) => {
      recu = choix;
      return choix.map((c) => ({
        provider: c.providerCle, libelle: c.providerCle, package: paquet(), erreur: null,
      }));
    },
  });

  els.get('pkg-lulu').checked = true;
  els.get('pkg-coollibri-148x210').checked = true;
  els.get('pkg-dos-coollibri-148x210').value = '18.4';
  els.get('pkg-fp-coollibri-148x210').value = '4';
  await els.get('btPackager').declenche('click');

  assert.strictEqual(recu.length, 2, 'KDP non coché a été envoyé');
  assert.deepStrictEqual({ ...recu[0] }, {
    providerCle: 'lulu', papierCle: 'standard', dosMm: null, fondPerduMm: null,
  });
  assert.deepStrictEqual({ ...recu[1] }, {
    providerCle: 'coollibri-148x210', papierCle: 'mesure', dosMm: 18.4, fondPerduMm: 4,
  });
});

test('sans prestataire coché, rien n\'est envoyé et l\'interface le dit', async () => {
  const { els, appels } = await ouvre([LULU]);
  await els.get('btPackager').declenche('click');
  assert.ok(!appels.some(([c]) => c === 'packager'), 'commande lancée à vide');
  assert.match(els.get('etatPackages').textContent, /au moins un prestataire/);
  assert.strictEqual(els.get('etatPackages').className, 'etat erreur');
});

/**
 * Un prestataire en échec ne doit pas emporter les autres : ce qui a été produit est
 * livrable, et l'échec doit être lisible plutôt que noyé dans un message global.
 */
test('un prestataire en échec est signalé sans masquer ceux qui ont abouti', async () => {
  const { els } = await ouvre([LULU, KDP], {
    packager: () => [
      { provider: 'lulu', libelle: 'Lulu', package: paquet(), erreur: null },
      {
        provider: 'kdp-6x9',
        libelle: 'Amazon KDP',
        package: null,
        erreur: '1200 pages : tranche de gouttière absente du gabarit kdp-6x9',
      },
    ],
  });
  els.get('pkg-lulu').checked = true;
  els.get('pkg-kdp-6x9').checked = true;
  await els.get('btPackager').declenche('click');

  const box = els.get('packages');
  assert.strictEqual(box.hidden, false);
  assert.deepStrictEqual(box.textes('h3'), ['Lulu', 'Amazon KDP']);
  assert.match(box.textContent, /17,43 mm/, 'dos du package abouti absent');
  assert.match(box.textContent, /tranche de gouttière absente/);
});

test('un package affiche le dos, la planche et les fichiers produits', async () => {
  const { els } = await ouvre([LULU], {
    packager: () => [{ provider: 'lulu', libelle: 'Lulu', package: paquet(), erreur: null }],
  });
  els.get('pkg-lulu').checked = true;
  await els.get('btPackager').declenche('click');

  const dd = els.get('packages').textes('dd');
  assert.deepStrictEqual(dd, [
    '278 (blanche de parité)',
    'Papier standard',
    '25,0 mm',
    '17,43 mm',
    '239,78 × 181,35 mm, fond perdu 3,175 mm',
  ]);
  assert.match(els.get('packages').textContent, /couverture-lulu\.pdf/);
});

/* ---------- aperçu de la planche ---------- */

/**
 * Le cœur du projet, vu de l'interface : le dos de l'aperçu vient de la composition,
 * jamais d'une saisie. Tant que l'intérieur n'a pas été composé, il n'y a pas de dos
 * à passer — et la planche refusera de s'afficher plutôt que d'en inventer un.
 */
test('l\'aperçu de planche n\'a pas de dos tant que l\'intérieur n\'est pas composé', async () => {
  const { els, appels } = await ouvre([LULU], { projet_ouvrir: { ...PROJET, couverture: {} } });
  await els.get('faces').children[2].declenche('click');
  await attendreApercu();

  const dernier = appels.filter(([c]) => c === 'couverture_apercu').pop();
  assert.strictEqual(dernier[1].face, 'planche');
  assert.strictEqual(dernier[1].dosMm, null, 'un dos est passé sans composition');
});

test('une fois l\'intérieur composé, l\'aperçu de planche reçoit ce dos-là', async () => {
  const { els, appels } = await ouvre([LULU], {
    projet_ouvrir: { ...PROJET, couverture: {} },
    composer: COMPOSITION,
  });
  await els.get('btComposer').declenche('click');
  await els.get('faces').children[2].declenche('click');
  await attendreApercu();

  const dernier = appels.filter(([c]) => c === 'couverture_apercu').pop();
  assert.strictEqual(dernier[1].dosMm, 17.427);
});

/**
 * Le dos vaut pour un gabarit et un seul : le même manuscrit ne fait pas le même
 * nombre de pages en poche et en grand format. Le traîner d'un prestataire à l'autre
 * donnerait à voir une planche fausse, et c'est exactement le défaut que l'atelier
 * HTML avait.
 */
test('changer de prestataire périme le dos de l\'aperçu', async () => {
  const { els, appels } = await ouvre([LULU, KDP], {
    projet_ouvrir: { ...PROJET, couverture: {} },
    composer: COMPOSITION,
  });
  await els.get('btComposer').declenche('click');
  await els.get('faces').children[2].declenche('click');
  await attendreApercu();
  assert.strictEqual(appels.filter(([c]) => c === 'couverture_apercu').pop()[1].dosMm, 17.427);

  els.get('inProvider').value = 'kdp-6x9';
  await els.get('inProvider').declenche('change');
  await attendreApercu();
  assert.strictEqual(
    appels.filter(([c]) => c === 'couverture_apercu').pop()[1].dosMm,
    null,
    'dos de Lulu réutilisé pour KDP'
  );
});

/**
 * Même raison, autre cause : la police repagine le livre. Un dos calculé en Alegreya
 * n'est plus le dos du livre dès qu'on le compose en Cardo, et le laisser sur la
 * planche donnerait un chiffre faux — ce qui vaut moins que pas de chiffre.
 */
test('un dos calculé pour une autre police ne vaut plus rien', async () => {
  const { els, appels } = await ouvre([LULU], {
    projet_ouvrir: { ...PROJET, couverture: {} },
    composer: COMPOSITION,
    interieur_modifier: (args) => ({ ...PROJET, couverture: {}, interieur: args.interieur }),
  });
  await els.get('btComposer').declenche('click');
  await els.get('faces').children[2].declenche('click');
  await attendreApercu();
  assert.strictEqual(appels.filter(([c]) => c === 'couverture_apercu').pop()[1].dosMm, 17.427);

  els.get('inPoliceInterieur').value = 'Cardo';
  await els.get('inPoliceInterieur').declenche('change');
  await attendreApercu();
  assert.strictEqual(
    appels.filter(([c]) => c === 'couverture_apercu').pop()[1].dosMm,
    null,
    'dos d\'Alegreya réutilisé pour Cardo'
  );
});
