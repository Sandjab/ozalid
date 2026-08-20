'use strict';

const test = require('node:test');
const assert = require('node:assert');
const { charge } = require('./dom_shim');

const IDS = [
  'btChoisir', 'btComposer', 'cheminManuscrit',
  'inTitre', 'inTitrePage', 'inAuteur', 'inGenre', 'inCopyright', 'inChapitres',
  'inProvider', 'inPapier', 'noteFormat',
  'etat', 'resultat',
];

const LULU = {
  cle: 'lulu', libelle: 'Lulu — poche 108 × 175',
  largeur: 108, hauteur: 175, fond_perdu: 3.175,
  papiers: [{ cle: 'standard', libelle: 'Papier standard' }],
};
const KDP = {
  cle: 'kdp-6x9', libelle: 'Amazon KDP — 6 × 9 po',
  largeur: 152.4, hauteur: 228.6, fond_perdu: 3.175,
  papiers: [{ cle: 'creme', libelle: 'Crème' }, { cle: 'blanc', libelle: 'Blanc' }],
};
const COOLLIBRI = {
  cle: 'coollibri-148x210', libelle: 'CoolLibri — A5',
  largeur: 148, hauteur: 210, fond_perdu: null,
  papiers: [{ cle: 'mesure', libelle: 'Dos relevé sur le gabarit' }],
};

function faux(providers, composition) {
  return async (cmd) => {
    if (cmd === 'providers_liste') return providers;
    if (cmd === 'composer') return composition;
    throw new Error(`commande inattendue : ${cmd}`);
  };
}

test('le choix du papier n\'est offert que quand il y en a plusieurs', async () => {
  const { els } = await charge({ ids: IDS, invoke: faux([LULU, KDP]) });
  // Lulu : un seul papier — le sélecteur ne doit pas laisser croire à un choix.
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

test('le dos calculé est affiché avec le compte de pages qui le produit', async () => {
  const { els } = await charge({
    ids: IDS,
    invoke: faux([LULU], {
      pages: 278, chapitres: 64, gouttiere: 25, blanche: true,
      dos: 17.427, pdf: '/x/interieur-lulu.pdf',
    }),
    open: async () => '/x/roman.md',
  });
  await els.get('btChoisir').declenche('click');
  await els.get('btComposer').declenche('click');

  const res = els.get('resultat');
  assert.strictEqual(res.hidden, false);
  const valeurs = res.textes('dd');
  assert.deepStrictEqual(valeurs, [
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
      pages: 190, chapitres: 64, gouttiere: 20, blanche: false,
      dos: null, pdf: '/x/interieur.pdf',
    }),
    open: async () => '/x/roman.md',
  });
  await els.get('btChoisir').declenche('click');
  await els.get('btComposer').declenche('click');

  const dos = els.get('resultat').textes('dd').at(-1);
  assert.match(dos, /relever sur le gabarit/);
  assert.doesNotMatch(dos, /\d/, `dos chiffré affiché : « ${dos} »`);
});

test('le bouton Composer reste inerte tant qu\'aucun manuscrit n\'est choisi', async () => {
  const { els } = await charge({ ids: IDS, invoke: faux([LULU]) });
  assert.strictEqual(els.get('btComposer').disabled, true);
  await els.get('btChoisir').declenche('click'); // open rend null : annulation
  assert.strictEqual(els.get('btComposer').disabled, true);
  assert.strictEqual(els.get('cheminManuscrit').textContent, '');
});

/**
 * Une erreur de la chaîne (chapitre manquant, construction non composable) doit
 * rester lisible à l'écran, et surtout ne pas laisser croire à une composition
 * réussie en gardant un résultat précédent affiché.
 */
test('une erreur de composition efface le résultat précédent', async () => {
  let echoue = false;
  const invoke = async (cmd) => {
    if (cmd === 'providers_liste') return [LULU];
    if (echoue) throw '3 chapitres attendus (projet), 64 trouvés.';
    return {
      pages: 278, chapitres: 64, gouttiere: 25, blanche: true,
      dos: 17.43, pdf: '/x/a.pdf',
    };
  };
  const { els } = await charge({ ids: IDS, invoke, open: async () => '/x/roman.md' });
  await els.get('btChoisir').declenche('click');
  await els.get('btComposer').declenche('click');
  assert.strictEqual(els.get('resultat').hidden, false);

  echoue = true;
  await els.get('btComposer').declenche('click');
  assert.strictEqual(els.get('resultat').hidden, true, 'résultat périmé laissé à l\'écran');
  assert.match(els.get('etat').textContent, /3 chapitres attendus/);
  assert.strictEqual(els.get('etat').className, 'etat erreur');
  assert.strictEqual(els.get('btComposer').disabled, false, 'bouton laissé bloqué');
});
