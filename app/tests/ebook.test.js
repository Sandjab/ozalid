'use strict';

// Câblage des ebooks : le bouton de l'étape Livraison qui atteint le Rust, et le compte
// rendu qui reste là où l'on a cliqué. Ce que valent le PDF et l'EPUB produits se
// vérifie en les ouvrant, pas ici.

const test = require('node:test');
const assert = require('node:assert');
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
    genre: 'roman', copyright: '', chapitres: 64,
  },
  manuscrit_absent: false,
  modifie: false,
  manuscrit_source: '/x/WIP7.md',
  chapitres_trouves: 64,
  mots: 49344,
  couverture: null,
  couverture_importee: false,
  images: ['couverture.jpg'],
  interieur: { police: 'Alegreya' },
  envois: { main: { mode: 'police', police: 'Caveat' }, liste: [] },
  livraison: {
    destinataires: [{ provider: 'lulu', papier: 'standard', dos_mm: null, fond_perdu_mm: null }],
    courant: 'lulu',
  },
};

/** Fausse implémentation des commandes Rust. `sur` surcharge une commande. */
function faux(providers, sur = {}) {
  return async (cmd, args) => {
    if (cmd === 'providers_liste') return providers;
    if (cmd === 'polices_liste') return ['Bodoni Moda', 'Archivo', 'Spectral'];
    if (cmd === 'polices_texte_liste') return ['EB Garamond', 'Alegreya', 'Cardo'];
    if (cmd === 'jetons_liste') return ['%TITRE%', '%AUTEUR%', '%GENRE%', '%EDITEUR%', '%COLLECTION%', '%MONOGRAMME%'];
    if (cmd === 'mains_liste') return ['Caveat', 'Dancing Script'];
    if (cmd === 'maquettes_liste') return [{ cle: 'folio', libelle: 'Folio' }];
    if (cmd === 'couverture_apercu') return { image: 'data:image/png;base64,AAAA', reperes: null };
    if (cmd in sur) {
      const v = sur[cmd];
      return typeof v === 'function' ? v(args) : v;
    }
    // Le démarrage et la garde envoient ces trois commandes sans qu'aucun test ne les
    // demande : sans réponse ici, elles lèveraient avant que rien ne soit vérifié.
    if (cmd === 'recents_liste') return [];
    if (cmd === 'garde_modifications') return 'ignorer';
    if (cmd === 'interface_prete') return null;
    // L'accès au modèle de diffusion se lit au démarrage : il appartient à la
    // machine, et l'écran le montre avant qu'aucun projet ne soit ouvert.
    if (cmd === 'diffusion_lire') return { url: '', cle_posee: false };
    throw new Error(`commande inattendue : ${cmd}`);
  };
}

const EBOOKS = {
  pdf: '/livres/LHC/ebook/Les Heures creuses.pdf',
  epub: '/livres/LHC/ebook/Les Heures creuses.epub',
  octets_pdf: 2400000,
  octets_epub: 1100000,
  polices_introuvables: [],
  police_non_embarquee: null,
};

/**
 * Les deux fichiers sont la seule chose qu'on ait à emporter : c'est leur chemin qu'on
 * copie pour les ouvrir, et un compte rendu qui ne les nomme pas laisse chercher dans un
 * répertoire qu'on ne connaît pas.
 *
 * Leur poids est le seul chiffre du compte rendu, et c'est à lui qu'on juge si l'envoi
 * passera par courriel : une inversion Ko/Mo afficherait « 2400 Mo » pour un roman sans
 * que rien ne tombe. Les deux témoins sont au-dessus du mégaoctet, l'un de peu — 1,05 Mo
 * — pour que le seuil lui-même soit éprouvé.
 */
test('le bouton Ebooks appelle le Rust et affiche les deux chemins', async () => {
  let appels = 0;
  const { els } = await charge({
    invoke: faux([LULU], {
      projet_importer: PROJET,
      ebook_generer: () => {
        appels += 1;
        return EBOOKS;
      },
    }),
    open: async () => '/dev/ozalid/build/LHC/livre.toml',
  });
  await els.get('btImporter').declenche('click');
  await els.get('btEbooks').declenche('click');

  assert.strictEqual(appels, 1);
  const rendu = els.get('ebooks').textContent;
  assert.match(rendu, /Les Heures creuses\.pdf/);
  assert.match(rendu, /Les Heures creuses\.epub/);
  assert.match(rendu, /2,3 Mo/, `${EBOOKS.octets_pdf} octets ne se lisent pas en Mo : ${rendu}`);
  assert.match(rendu, /1,0 Mo/, `${EBOOKS.octets_epub} octets ne se lisent pas en Mo : ${rendu}`);
  assert.strictEqual(els.get('ebooks').hidden, false);
  assert.strictEqual(els.get('etatEbooks').textContent, '');
});

/**
 * Une police absente des répertoires de Typst n'est pas une erreur : le livre reste
 * juste, seul son œil change. La dire en rouge à la place des chemins ferait croire à un
 * échec, et on chercherait des fichiers qui sont pourtant là.
 */
test('une police non embarquée est dite, sans que la génération échoue', async () => {
  const { els } = await charge({
    invoke: faux([LULU], {
      projet_importer: PROJET,
      ebook_generer: () => ({ ...EBOOKS, police_non_embarquee: 'Vollkorn' }),
    }),
    open: async () => '/dev/ozalid/build/LHC/livre.toml',
  });
  await els.get('btImporter').declenche('click');
  await els.get('btEbooks').declenche('click');

  const rendu = els.get('ebooks').textContent;
  assert.match(rendu, /Vollkorn/);
  assert.match(rendu, /Les Heures creuses\.epub/, 'les chemins ont cédé la place à l\'avertissement');
  assert.strictEqual(els.get('etatEbooks').className, 'etat');
});

/**
 * On attend là où l'on a cliqué : un refus qui migre dans l'entête se lit comme une panne
 * de l'application, à l'autre bout de l'écran que le bouton qui l'a provoqué. L'entête ne
 * porte que ce qui refuse une saisie, et générer n'en est pas une.
 */
test('un refus du Rust se lit à côté du bouton, pas en haut de l\'écran', async () => {
  const { els } = await charge({
    invoke: faux([LULU], {
      projet_importer: PROJET,
      ebook_generer: () => {
        throw 'aucune maquette de couverture : en choisir une avant de générer les ebooks.';
      },
    }),
    open: async () => '/dev/ozalid/build/LHC/livre.toml',
  });
  await els.get('btImporter').declenche('click');
  await els.get('btEbooks').declenche('click');

  assert.match(els.get('etatEbooks').textContent, /aucune maquette de couverture/);
  assert.strictEqual(els.get('etatEbooks').className, 'etat erreur');
  assert.strictEqual(els.get('alerte').textContent, '');
});

/**
 * Un compte rendu qui survivrait à l'ouverture d'un autre projet donnerait à lire les
 * chemins du livre A sous le titre du livre B — et ces fichiers-là existent, on irait
 * donc les ouvrir sans se douter de rien.
 */
test('ouvrir un autre projet efface le compte rendu', async () => {
  const { els } = await charge({
    invoke: faux([LULU], {
      projet_importer: PROJET,
      projet_nouveau: PROJET,
      ebook_generer: EBOOKS,
    }),
    open: async () => '/dev/ozalid/build/LHC/livre.toml',
  });
  await els.get('btImporter').declenche('click');
  await els.get('btEbooks').declenche('click');
  assert.strictEqual(els.get('ebooks').hidden, false);

  await els.get('btNouveau').declenche('click');

  assert.strictEqual(els.get('ebooks').textContent, '');
  assert.strictEqual(els.get('ebooks').hidden, true);
});
