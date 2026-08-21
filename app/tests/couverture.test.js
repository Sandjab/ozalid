'use strict';

const test = require('node:test');
const assert = require('node:assert');
const { charge } = require('./dom_shim');
const { groupes, lire, ecrire } = require('../src/couverture.js');

const IDS = [
  'btNouveau', 'btOuvrir', 'btImporter', 'btEnregistrer', 'btEnregistrerSous',
  'cheminProjet', 'etatEnregistrement', 'recents',
  'secLivre', 'secManuscrit', 'secCouverture', 'secComposer',
  'inTitre', 'inTitrePage', 'inAuteur', 'inGenre', 'inCopyright', 'inChapitres',
  'etatManuscrit', 'sourceManuscrit', 'btReimporter', 'btChoisirManuscrit',
  'etatImages', 'btImageUne', 'btImageQuatre',
  'maquettes', 'etatCouverture', 'faces', 'apercu', 'etatApercu',
  'reglages',
  'inProvider', 'inPapier', 'noteFormat',
  'btComposer', 'etat', 'resultat',
  'secPackages', 'listePrestataires', 'btPackager', 'etatPackages', 'packages',
  'secInterieur', 'inPoliceInterieur',
  'secEpreuve', 'inEpreuveCorps', 'btEpreuve', 'etatEpreuve', 'cheminEpreuve',
];

const LULU = {
  cle: 'lulu', libelle: 'Lulu', largeur: 108, hauteur: 175, fond_perdu: 3.175, dos_publie: true,
  papiers: [{ cle: 'standard', libelle: 'Papier standard' }],
};

const style = (police, taille, couleur) => ({
  police, graisse: 400, italique: false, taille, couleur, tracking: 0, casse: 'telle',
});

const CADRAGE = { proportions: false, x: 0.5, y: 0.5, zoom: 1, etirement: 1 };

/** Un élément du dos, au format que sérialise le Rust. */
const elementDos = (place, rang) => ({
  actif: true, place, rang, style: style('Archivo', 2.6, '#191917'),
});

/** Maquette au format exact que sérialise le Rust. */
function maquette(mode = 'bandeau') {
  return {
    mode,
    papier: '#ffffff',
    align: 'gauche',
    pad_x: 7,
    bandeau: 30,
    bandeau_retrait: false,
    bloc_y: 13,
    cadre: {
      actif: false, marge: 9,
      filet1_couleur: '#000000', filet1_epaisseur: 0.3, decroche: 4,
      filet2_couleur: '#c00000', filet2_epaisseur: 0.25, ecart: 0.9,
    },
    auteur: style('Archivo', 6.4, '#c00000'),
    titre: style('Spectral', 8, '#191917'),
    titre_interligne: 1.1,
    titre_ecart: 3.5,
    genre_visible: false,
    genre: style('Spectral', 2.2, '#191917'),
    genre_ecart: 6,
    pied: {
      actif: false, monogramme: '', editeur: 'ÉDITEUR', y: 11,
      style_mono: { ...style('Spectral', 7, '#191917'), italique: true },
      style_editeur: style('Archivo', 3.2, '#191917'),
    },
    pastille: {
      actif: true, texte: 'folio', style: style('Archivo', 3.2, '#ffffff'),
      fond: '#111111', coin: 'bas-droite', verticale: false, arrondie: true,
      dx: 4.5, dy: 3.5,
    },
    cadrage: { ...CADRAGE },
    voile: 'aucun',
    voile_opacite: 0.55,
    quatrieme: {
      fond: 'herite', couleur: '#fcf0d8', texte: '', style: style('Spectral', 3, '#191917'),
      interligne: 1.45, align: 'gauche', pad_x: 10, top: 12,
      pied_actif: true, mention: '', collection: '', prix: '',
      style_pied: style('Archivo', 2.4, '#191917'), pied_y: 4,
      isbn_actif: false, isbn_l: 34, isbn_h: 21, isbn_dx: 7, isbn_dy: 7,
      cadrage: { ...CADRAGE }, voile: 'aucun', voile_opacite: 0.55,
    },
    dos: {
      auteur: elementDos('pied', 1),
      titre: elementDos('pied', 2),
      editeur: elementDos('tete', 1),
      ecart: 2,
      marge: 3,
      fond_propre: false,
      fond: '#fcf0d8',
    },
  };
}

function projet(couverture) {
  return {
    chemin: '/livres/LHC.ozalid',
    livre: {
      titre: 'Les Heures creuses', titre_page: null, auteur: 'Ivan Pjig',
      genre: 'roman', copyright: '', chapitres: 64,
    },
    manuscrit_absent: false,
    modifie: false,
    manuscrit_source: '/x/WIP7.md',
    chapitres_trouves: 64,
    mots: 49344,
    couverture,
    couverture_importee: !!couverture,
    images: ['couverture.jpg'],
    interieur: { police: 'Alegreya' },
  };
}

/**
 * Contexte prêt : un projet ouvert, avec la maquette donnée.
 *
 * `dialogues` fournit ce que rendront les sélecteurs de fichier ouverts ensuite, dans
 * l'ordre ; une fois la liste épuisée, le sélecteur est réputé annulé.
 */
async function ouvre(couverture, sur = {}, dialogues = []) {
  const appels = [];
  const invoke = async (cmd, args) => {
    appels.push([cmd, args]);
    // Les surcharges passent avant les réponses par défaut, sinon un test ne
    // pourrait jamais remplacer le comportement d'une commande courante.
    if (cmd in sur) return sur[cmd](args);
    if (cmd === 'providers_liste') return [LULU];
    if (cmd === 'polices_liste') return ['Archivo', 'Spectral', 'Bodoni Moda'];
    if (cmd === 'polices_texte_liste') return ['EB Garamond', 'Alegreya', 'Cardo'];
    if (cmd === 'maquettes_liste') {
      return [
        { cle: 'folio', libelle: 'Folio' },
        { cle: 'blanche', libelle: 'Blanche' },
        { cle: 'surimpression', libelle: 'Surimpression' },
      ];
    }
    if (cmd === 'projet_ouvrir') return projet(couverture);
    if (cmd === 'couverture_apercu') return 'data:image/png;base64,QUJD';
    // Le démarrage et la garde envoient ces trois commandes sans qu'aucun test ne les
    // demande : sans réponse ici, elles lèveraient avant que rien ne soit vérifié.
    if (cmd === 'recents_liste') return [];
    if (cmd === 'garde_modifications') return 'ignorer';
    if (cmd === 'interface_prete') return null;
    throw new Error(`commande inattendue : ${cmd}`);
  };
  const file = ['/livres/LHC.ozalid', ...dialogues];
  const ctx = await charge({ ids: IDS, invoke, open: async () => file.shift() ?? null });
  await ctx.els.get('btOuvrir').declenche('click');
  return { ...ctx, appels };
}

/** Laisse passer le délai de grâce de l'aperçu. */
const attendreApercu = () => new Promise((r) => setTimeout(r, 300));

/* ---------- schéma ---------- */

/**
 * Le panneau est construit depuis le schéma : un chemin faux laisserait un contrôle
 * vide en silence, et le réglage correspondant deviendrait inatteignable.
 */
test('tous les chemins du schéma existent dans la maquette', () => {
  const m = maquette();
  for (const g of groupes()) {
    for (const c of g.champs) {
      assert.notStrictEqual(
        lire(m, c.chemin), undefined,
        `chemin absent de la maquette : ${c.chemin}`
      );
    }
  }
});

test('écrire puis relire un chemin imbriqué rend la valeur posée', () => {
  const m = maquette();
  ecrire(m, 'pied.style_mono.taille', 9.5);
  assert.strictEqual(m.pied.style_mono.taille, 9.5);
  assert.strictEqual(lire(m, 'pied.style_mono.taille'), 9.5);
});

/* ---------- panneau ---------- */

test('le panneau se remplit depuis la maquette du projet', async () => {
  const { els } = await ouvre(maquette());
  const lignes = [...els.get('reglages').children]
    .flatMap((g) => [...g.children].slice(1));
  const valeurs = lignes.map((l) => l.children[1].value);
  assert.ok(valeurs.includes('bandeau'), 'le mode n\'est pas repris');
  assert.ok(valeurs.includes('#ffffff'), 'le papier n\'est pas repris');
  assert.ok(valeurs.includes(6.4), 'le corps de l\'auteur n\'est pas repris');
});

/**
 * Un réglage sans objet dans le mode courant est masqué : le panneau est long, et un
 * contrôle qui ne produirait aucun effet y serait un piège.
 */
test('les réglages sans objet dans le mode courant sont masqués', async () => {
  const visibles = (els) => {
    const out = new Map();
    for (const g of els.get('reglages').children) {
      for (const l of [...g.children].slice(1)) {
        out.set(l.children[0].textContent, !g.hidden && !l.hidden);
      }
    }
    return out;
  };

  const bandeau = visibles((await ouvre(maquette('bandeau'))).els);
  assert.strictEqual(bandeau.get('Hauteur du bandeau (% haut.)'), true);
  assert.strictEqual(bandeau.get('Hauteur du bloc titre (% haut.)'), false);

  const typo = visibles((await ouvre(maquette('typo'))).els);
  assert.strictEqual(typo.get('Hauteur du bandeau (% haut.)'), false);
  assert.strictEqual(typo.get('Hauteur du bloc titre (% haut.)'), true);
  assert.strictEqual(typo.get('Zoom'), false, 'cadrage image offert sans image');
});

test('basculer sur la 4ème change les groupes offerts', async () => {
  const { els } = await ouvre(maquette());
  const titres = () => [...els.get('reglages').children]
    .filter((g) => !g.hidden)
    .map((g) => g.children[0].textContent);

  assert.ok(titres().includes('Cadre'));
  assert.ok(!titres().some((t) => t.startsWith('4ème')));

  await els.get('faces').children[1].declenche('click');
  assert.ok(titres().some((t) => t.startsWith('4ème')));
  assert.ok(!titres().includes('Cadre'));
});

/**
 * Les réglages du dos n'ont de sens que sur la planche : c'est là qu'il se voit. Les
 * offrir sur la 1ère donnerait à régler un élément absent de l'aperçu affiché.
 */
test('les trois éléments du dos ne sont offerts que sur la planche', async () => {
  const { els } = await ouvre(maquette());
  const titres = () => [...els.get('reglages').children]
    .filter((g) => !g.hidden)
    .map((g) => g.children[0].textContent);

  assert.ok(!titres().some((t) => t.startsWith('Dos')), 'dos offert sur la 1ère');

  await els.get('faces').children[2].declenche('click');
  const t = titres();
  assert.ok(t.includes('Dos — auteur'));
  assert.ok(t.includes('Dos — titre'));
  assert.ok(t.includes('Dos — éditeur'));
  assert.ok(t.includes('Dos — fond et espacements'));
  assert.ok(!t.includes('Cadre'), 'réglages de 1ère laissés sur la planche');
});

/**
 * Modifier un contrôle renvoie la maquette **entière**, pas le seul champ touché :
 * un envoi partiel écraserait tous les autres réglages par leurs valeurs par défaut.
 */
test('modifier un réglage renvoie la maquette entière', async () => {
  let recue = null;
  const { els } = await ouvre(maquette(), {
    couverture_modifier: ({ couverture }) => {
      recue = couverture;
      return projet(couverture);
    },
  });
  const papier = els.get('reglages').children[0].children[2].children[1];
  papier.value = '#fcf0d8';
  await papier.declenche('change');

  assert.strictEqual(recue.papier, '#fcf0d8');
  assert.strictEqual(recue.pastille.texte, 'folio', 'pastille perdue');
  assert.strictEqual(recue.cadre.filet2_couleur, '#c00000', 'cadre perdu');
  assert.strictEqual(recue.quatrieme.interligne, 1.45, '4ème perdue');
});

/**
 * Le schéma borne chaque réglage, mais seules les flèches du champ s'y tiennent : au
 * clavier, rien n'empêche une marge de 500 % de largeur. Elle composerait une
 * couverture où le titre n'a plus de place, sans que rien ne dise d'où vient
 * l'absurdité.
 */
test('un nombre tapé hors des bornes du schéma y est ramené', async () => {
  let recue = null;
  const { els } = await ouvre(maquette(), {
    couverture_modifier: ({ couverture }) => {
      recue = couverture;
      return projet(couverture);
    },
  });
  const marge = els.get('reglages').children[0].children[4].children[1];

  marge.value = '500';
  await marge.declenche('change');
  assert.strictEqual(recue.pad_x, 40, 'maximum du schéma dépassé');

  marge.value = '-8';
  await marge.declenche('change');
  assert.strictEqual(recue.pad_x, 0, 'minimum du schéma franchi');
});

/* ---------- photos ---------- */

/**
 * La photo entre dans le projet par la face qu'elle sert, et non par le nom du fichier
 * choisi : c'est ce rôle que la composition relira, et lui seul.
 */
test('choisir une photo la pose sur la face demandée', async () => {
  let recu = null;
  const { els } = await ouvre(maquette(), {
    image_choisir: (args) => {
      recu = args;
      return projet(maquette());
    },
  }, ['/photos/fumee.jpg']);

  await els.get('btImageQuatre').declenche('click');
  assert.deepStrictEqual({ ...recu }, { face: 'quatre', chemin: '/photos/fumee.jpg' });
});

test('un sélecteur de photo annulé ne touche pas au projet', async () => {
  const { els, appels } = await ouvre(maquette());
  await els.get('btImageUne').declenche('click');
  assert.ok(!appels.some(([c]) => c === 'image_choisir'), 'photo posée sans fichier');
});

/* ---------- aperçu ---------- */

test('l\'aperçu est demandé et affiché à l\'ouverture du projet', async () => {
  const { els, appels } = await ouvre(maquette());
  await attendreApercu();
  const demandes = appels.filter(([c]) => c === 'couverture_apercu');
  assert.ok(demandes.length >= 1, 'aucun aperçu demandé');
  assert.strictEqual(demandes[0][1].face, 'une');
  assert.strictEqual(els.get('apercu').src, 'data:image/png;base64,QUJD');
});

/**
 * Le format vient du prestataire : changer de prestataire change l'aperçu, même si
 * aucun réglage de maquette n'a bougé.
 */
test('changer de prestataire redemande un aperçu', async () => {
  const { els, appels } = await ouvre(maquette());
  await attendreApercu();
  const avant = appels.filter(([c]) => c === 'couverture_apercu').length;
  await els.get('inProvider').declenche('change');
  await attendreApercu();
  const apres = appels.filter(([c]) => c === 'couverture_apercu').length;
  assert.ok(apres > avant, 'aperçu non redemandé');
});

test('sans maquette, l\'aperçu le dit au lieu de rester vide', async () => {
  const { els } = await ouvre(null);
  await attendreApercu();
  assert.match(els.get('etatCouverture').textContent, /Aucune maquette/);
  assert.strictEqual(els.get('reglages').hidden, true);
  assert.match(els.get('etatApercu').textContent, /Choisir une maquette/);
  assert.strictEqual(els.get('apercu').hidden, true, 'cadre d\'image sans image');
});

/**
 * Une composition qui échoue — le prolongement panoramique sans pagination, par
 * exemple — doit le dire et retirer l'image périmée, pas laisser un aperçu qui ne
 * correspond plus aux réglages affichés.
 */
test('un aperçu qui échoue efface l\'image et affiche la cause', async () => {
  let casse = false;
  const { els } = await ouvre(maquette(), {
    couverture_apercu: () => {
      if (casse) throw 'prolongement panoramique : la largeur du dos est inconnue';
      return 'data:image/png;base64,QUJD';
    },
  });
  await attendreApercu();
  assert.ok(els.get('apercu').src);
  assert.strictEqual(els.get('apercu').hidden, false, 'aperçu réussi mais masqué');

  casse = true;
  await els.get('inProvider').declenche('change');
  await attendreApercu();
  assert.strictEqual(els.get('apercu').src, undefined, 'aperçu périmé laissé à l\'écran');
  assert.strictEqual(els.get('apercu').hidden, true, 'cadre d\'image sans image');
  assert.match(els.get('etatApercu').textContent, /largeur du dos/);
  assert.strictEqual(els.get('etatApercu').className, 'note alerte');
});
