'use strict';

// Le faux DOM est l'outil des autres tests : ce qu'il promet doit être vérifié une
// fois ici, plutôt que supposé quarante fois ailleurs.

const test = require('node:test');
const assert = require('node:assert');
const { charge } = require('./dom_shim');

/** Le strict minimum pour que app.js démarre et ouvre un projet sans lever. */
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
  modifie: false,
  couverture: null,
  couverture_importee: false,
  images: [],
  interieur: { police: 'EB Garamond' },
  livraison: {
    destinataires: [{ provider: 'lulu', papier: 'standard', dos_mm: null, fond_perdu_mm: null }],
    courant: 'lulu',
  },
  envois: { main: { mode: 'police', police: 'Caveat' }, liste: [] },
};

const invokeMuet = async (cmd) => {
  switch (cmd) {
    case 'providers_liste': return [{
      cle: 'lulu', libelle: 'Lulu — poche 108 × 175',
      largeur: 108, hauteur: 175, fond_perdu: 3.175, dos_publie: true,
      papiers: [{ cle: 'standard', libelle: 'Papier standard' }],
    }];
    case 'polices_liste': return ['Bodoni Moda'];
    case 'polices_texte_liste': return ['EB Garamond'];
    case 'mains_liste': return ['Caveat', 'Dancing Script'];
    case 'maquettes_liste': return [];
    case 'recents_liste': return [];
    case 'interface_prete': return null;
    // Sans elle, la garde refuserait de laisser passer et « Nouveau projet » ne
    // ferait rien : le test du helper de menu se vérifierait lui-même.
    case 'garde_modifications': return 'ignorer';
    case 'couverture_apercu': throw new Error('pas de maquette');
    default: return PROJET;
  }
};

test('sans liste d\'identifiants, tous ceux du vrai HTML sont là', async () => {
  const { els } = await charge({ invoke: invokeMuet });

  assert.ok(els.get('inTitre'), 'un identifiant du HTML manque au faux DOM');
  assert.ok(els.get('btNouveau'));
  assert.equal(els.get('btReimporter').disabled, true,
    'l\'état initial doit toujours venir du HTML');
});

test('le helper de menu actionne le routeur de l\'application', async () => {
  const appels = [];
  const { menu } = await charge({
    invoke: async (cmd, args) => { appels.push(cmd); return invokeMuet(cmd, args); },
  });

  await menu('fichier.nouveau');

  assert.ok(appels.includes('projet_nouveau'));
});

/**
 * Un test qui pose son propre `listen` remplace celui du faux DOM. Le helper n'a alors
 * plus rien à actionner : mieux vaut le dire que rendre un geste silencieusement inerte.
 */
test('un listen sur mesure fait dire au helper pourquoi il ne peut rien', async () => {
  const { menu } = await charge({
    invoke: invokeMuet,
    listen: async () => () => {},
  });

  await assert.rejects(() => menu('fichier.nouveau'), /listen/);
});

/**
 * Une variable CSS est le seul moyen de faire passer un nombre du Rust à la feuille de
 * style : un attribut `data-` ne se lit pas dans un `calc()`. Le faux DOM doit donc
 * savoir en retenir une, sans quoi l'habillage de la coupe ne s'exécute nulle part.
 *
 * Sur `couv`, et non sur le cadre de l'aperçu : ce qui est vérifié ici est le faux DOM
 * lui-même, pas ce que l'application en fait — n'importe quel élément fait l'affaire.
 */
test('une variable CSS posée sur un élément se relit', async () => {
  const { els } = await charge({ invoke: invokeMuet });
  const el = els.get('couv');
  el.style.setProperty('--coupe-x', '0.0129');
  assert.strictEqual(el.style.getPropertyValue('--coupe-x'), '0.0129');
  assert.strictEqual(el.style.getPropertyValue('--coupe-y'), '',
    'une variable jamais posée doit se lire vide, comme dans le navigateur');
});
