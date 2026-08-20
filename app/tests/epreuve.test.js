'use strict';

// Câblage de l'intérieur et de l'épreuve : la police du projet qui paraît au panneau,
// le réglage qui redescend jusqu'au Rust, et l'épreuve qui se tire sans rien composer
// au préalable. Le rendu de l'épreuve se vérifie en la composant, pas ici.

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

/** Fausse implémentation des commandes Rust. `sur` surcharge une commande. */
function faux(providers, sur = {}) {
  return async (cmd, args) => {
    if (cmd === 'providers_liste') return providers;
    if (cmd === 'polices_liste') return ['Bodoni Moda', 'Archivo', 'Spectral'];
    if (cmd === 'polices_texte_liste') return ['EB Garamond', 'Alegreya', 'Cardo'];
    if (cmd === 'maquettes_liste') return [{ cle: 'folio', libelle: 'Folio' }];
    if (cmd === 'couverture_apercu') return 'data:image/png;base64,AAAA';
    if (cmd in sur) {
      const v = sur[cmd];
      return typeof v === 'function' ? v(args) : v;
    }
    throw new Error(`commande inattendue : ${cmd}`);
  };
}

/* ---------- intérieur ---------- */

test('la police d\'intérieur du projet est celle qui paraît au panneau', async () => {
  const { els } = await charge({
    ids: IDS,
    invoke: faux([LULU], { projet_importer: PROJET }),
    open: async () => '/dev/ozalid/build/LHC/livre.toml',
  });
  await els.get('btImporter').declenche('click');
  assert.strictEqual(els.get('secInterieur').hidden, false);
  assert.strictEqual(els.get('inPoliceInterieur').value, 'Alegreya');
});

/**
 * Le réglage doit atteindre le Rust : un sélecteur qui change d'apparence sans rien
 * enregistrer laisserait composer dans une autre police que celle qu'on voit.
 */
test('changer la police enregistre le réglage dans le projet', async () => {
  let recu = null;
  const { els } = await charge({
    ids: IDS,
    invoke: faux([LULU], {
      projet_importer: PROJET,
      interieur_modifier: (args) => {
        recu = args.interieur;
        return { ...PROJET, interieur: args.interieur };
      },
    }),
    open: async () => '/dev/ozalid/build/LHC/livre.toml',
  });
  await els.get('btImporter').declenche('click');
  els.get('inPoliceInterieur').value = 'Cardo';
  await els.get('inPoliceInterieur').declenche('change');
  // Recopié : l'objet vient du contexte où s'exécute app.js, donc d'un autre realm.
  assert.deepStrictEqual({ ...recu }, { police: 'Cardo' });
  assert.strictEqual(els.get('inPoliceInterieur').value, 'Cardo');
});

/**
 * Une police refusée par le Rust doit rester lisible à l'écran, et le panneau doit
 * revenir à ce que le projet porte vraiment : laisser le sélecteur sur un choix non
 * enregistré ferait croire qu'on compose dans une police qui n'est pas celle-là.
 */
test('une police refusée est dite, et le panneau revient au projet', async () => {
  const { els } = await charge({
    ids: IDS,
    invoke: faux([LULU], {
      projet_importer: PROJET,
      interieur_modifier: () => {
        throw 'police d\'intérieur inconnue : « Oswald ».';
      },
    }),
    open: async () => '/dev/ozalid/build/LHC/livre.toml',
  });
  await els.get('btImporter').declenche('click');
  els.get('inPoliceInterieur').value = 'Cardo';
  await els.get('inPoliceInterieur').declenche('change');
  assert.match(els.get('etat').textContent, /police d'intérieur inconnue/);
  assert.strictEqual(els.get('etat').className, 'etat erreur');
  assert.strictEqual(
    els.get('inPoliceInterieur').value,
    'Alegreya',
    'le panneau reste sur une police que le projet ne porte pas'
  );
});

/* ---------- épreuve ---------- */

/**
 * L'épreuve ne dépend d'aucune pagination ni d'aucun prestataire : elle doit pouvoir
 * être tirée dès qu'un manuscrit est là, sans intérieur composé au préalable.
 */
test('l\'épreuve se tire sans intérieur composé', async () => {
  let corps = null;
  const { els } = await charge({
    ids: IDS,
    invoke: faux([LULU], {
      projet_importer: PROJET,
      epreuve_tirer: (args) => {
        corps = args.corpsPt;
        return '/livres/LHC/epreuve.pdf';
      },
    }),
    open: async () => '/dev/ozalid/build/LHC/livre.toml',
  });
  await els.get('btImporter').declenche('click');
  await els.get('btEpreuve').declenche('click');
  assert.strictEqual(corps, 12);
  assert.strictEqual(els.get('cheminEpreuve').textContent, '/livres/LHC/epreuve.pdf');
  assert.strictEqual(els.get('etatEpreuve').textContent, '');
});

/**
 * Un échec doit être dit et ne pas laisser en place le chemin d'une épreuve précédente :
 * on irait relire un PDF périmé en croyant relire celui qu'on vient de demander.
 */
test('une épreuve en échec est signalée et efface le chemin précédent', async () => {
  let echoue = false;
  const { els } = await charge({
    ids: IDS,
    invoke: faux([LULU], {
      projet_importer: PROJET,
      epreuve_tirer: () => {
        if (echoue) throw 'enregistrer le projet avant de composer.';
        return '/livres/LHC/epreuve.pdf';
      },
    }),
    open: async () => '/dev/ozalid/build/LHC/livre.toml',
  });
  await els.get('btImporter').declenche('click');
  await els.get('btEpreuve').declenche('click');
  assert.strictEqual(els.get('cheminEpreuve').textContent, '/livres/LHC/epreuve.pdf');

  echoue = true;
  await els.get('btEpreuve').declenche('click');
  assert.match(els.get('etatEpreuve').textContent, /enregistrer le projet/);
  assert.strictEqual(els.get('etatEpreuve').className, 'etat erreur');
  assert.strictEqual(els.get('cheminEpreuve').textContent, '', 'chemin périmé laissé à l\'écran');
  assert.strictEqual(els.get('btEpreuve').disabled, false, 'bouton laissé bloqué');
});
