'use strict';

// Câblage de l'étape « Livraison » : ce que l'interface envoie au Rust, et ce qu'elle
// en montre. Le rendu des planches se vérifie dans l'application, pas ici.

const test = require('node:test');
const assert = require('node:assert');
const { charge } = require('./dom_shim');

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

/**
 * La face par son libellé, et non par son rang : ces boutons se retrouvent par rang
 * dans l'application — c'est ce que dit le commentaire de `FACES` — mais un test qui
 * en fait autant se met à viser sa voisine le jour où une face s'ajoute. C'est
 * exactement ce qu'a fait l'arrivée du Dos entre la 4ème et la Planche.
 */
const face = (els, libelle) =>
  [...els.get('faces').children].find((b) => b.textContent === libelle);

/** Un destinataire neuf chez un prestataire, comme le Rust en fabrique un. */
const chez = (p) => ({
  provider: p.cle, papier: p.papiers[0].cle, dos_mm: null, fond_perdu_mm: null,
});

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
};

const COMPOSITION = {
  pages: 262, chapitres: 64, gouttiere: 25, blanche: true,
  dos: 16.513, pdf: '/livres/LHC/lulu/interieur-lulu.pdf',
  polices_introuvables: [],
};

function paquet(sur = {}) {
  return {
    provider: 'lulu',
    libelle: 'Lulu — poche 108 × 175',
    papier: 'Papier standard',
    pages: 262,
    gouttiere: 25,
    blanche: true,
    dos: 16.513,
    dos_requis: null,
    fond_perdu: 3.175,
    planche: [238.863, 181.35],
    chemins: ['/livres/LHC/lulu/interieur-lulu.pdf', '/livres/LHC/lulu/couverture-lulu.pdf'],
    vignette: '/livres/LHC/lulu/couverture-lulu.png',
    polices_introuvables: [],
    ...sur,
  };
}

/**
 * Un projet ouvert, avec un Rust de façade qui **tient réellement** la liste des
 * destinataires.
 *
 * Depuis le lot 3, le prestataire vit dans le projet et non dans un contrôle : le front
 * relit la liste à chaque retour de commande. Un faux qui rendrait toujours le même
 * projet ne prouverait donc plus rien — il masquerait justement le câblage qu'on vérifie.
 */
async function ouvre(
  providers,
  sur = {},
  { couverture = null, destinataires, dejaCompose = false } = {}
) {
  const appels = [];
  const liste = (destinataires ?? [chez(providers[0])]).map((d) => ({ ...d }));
  let projet = {
    ...PROJET,
    couverture,
    livraison: { destinataires: liste, courant: liste[0].provider, deja_compose: dejaCompose },
  };
  const maj = (livraison) => {
    projet = { ...projet, livraison: { ...projet.livraison, ...livraison } };
    return projet;
  };
  // Les règles du Rust, modélisées ici : la mesure d'une composition entre chez le
  // destinataire pour qui elle a été faite, et tout ce qui pagine les efface toutes.
  // Sans ce modèle, le front n'aurait plus rien à lire — il ne tient plus de dos.
  const oublier = () => maj({
    destinataires: projet.livraison.destinataires.map(({ compose, ...d }) => d),
  });
  const retenir = (c) => maj({
    deja_compose: true,
    destinataires: projet.livraison.destinataires.map((d) => (
      d.provider === projet.livraison.courant
        ? {
          ...d,
          compose: {
            pages: c.pages, gouttiere: c.gouttiere, blanche: c.blanche, dos: c.dos,
          },
        }
        : d
    )),
  });
  const invoke = async (cmd, args) => {
    appels.push([cmd, args]);
    if (cmd in sur) {
      const v = sur[cmd];
      const r = typeof v === 'function' ? await v(args) : v;
      // Une composition surchargée par un test reste soumise aux règles : c'est le
      // projet qui porte la mesure, et le front la relit là.
      return cmd === 'composer' ? { ...r, projet: retenir(r) } : r;
    }
    if (cmd === 'providers_liste') return providers;
    if (cmd === 'polices_liste') return ['Archivo', 'Spectral'];
    if (cmd === 'polices_texte_liste') return ['EB Garamond', 'Alegreya', 'Cardo'];
    if (cmd === 'mains_liste') return ['Caveat', 'Dancing Script'];
    if (cmd === 'maquettes_liste') return [{ cle: 'folio', libelle: 'Folio' }];
    if (cmd === 'projet_ouvrir') return projet;
    if (cmd === 'couverture_apercu') return { image: 'data:image/png;base64,QUJD', reperes: null };
    if (cmd === 'destinataire_viser') return maj({ courant: args.providerCle });
    if (cmd === 'destinataire_regler') {
      return maj({
        destinataires: projet.livraison.destinataires.map((d) => (
          d.provider === args.destinataire.provider
            ? { ...args.destinataire, compose: undefined }
            : d
        )),
      });
    }
    if (cmd === 'destinataire_ajouter') {
      return maj({
        destinataires: [
          ...projet.livraison.destinataires,
          chez(providers.find((p) => p.cle === args.providerCle)),
        ],
      });
    }
    if (cmd === 'destinataire_retirer') {
      return maj({
        destinataires: projet.livraison.destinataires.filter(
          (d) => d.provider !== args.providerCle
        ),
      });
    }
    if (cmd === 'interieur_modifier') {
      projet = { ...projet, interieur: args.interieur };
      return oublier();
    }
    if (cmd === 'livre_modifier') {
      projet = { ...projet, livre: args.livre };
      return oublier();
    }
    if (cmd === 'manuscrit_reimporter' || cmd === 'manuscrit_choisir') return oublier();
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
  const ctx = await charge({ invoke, open: async () => '/livres/LHC.ozalid' });
  await ctx.els.get('btOuvrir').declenche('click');
  return { ...ctx, appels };
}

const attendreApercu = () => new Promise((r) => setTimeout(r, 300));
/** Plus long que le débounce de la recomposition automatique (400 ms). */
const attendreComposition = () => new Promise((r) => setTimeout(r, 700));
const combien = (appels, cmd) => appels.filter(([c]) => c === cmd).length;
const dernier = (appels, cmd) => appels.filter(([c]) => c === cmd).pop();

/* ---------- la liste des destinataires ---------- */

/**
 * Un prestataire qui publie sa formule n'a rien à faire saisir : offrir un champ de
 * dos donnerait à croire qu'il compte, alors que la formule prime toujours.
 */
test('seul un prestataire à gabarit demande un relevé', async () => {
  const { els } = await ouvre([LULU, COOLLIBRI], {}, {
    destinataires: [chez(LULU), chez(COOLLIBRI)],
  });
  assert.ok(!els.get('dest-dos-lulu'), 'dos saisissable chez Lulu');
  assert.ok(!els.get('dest-fp-lulu'), 'fond perdu saisissable chez Lulu');
  assert.ok(els.get('dest-dos-coollibri-148x210'), 'dos non demandé chez CoolLibri');
  assert.ok(els.get('dest-fp-coollibri-148x210'), 'fond perdu non demandé chez CoolLibri');
});

/**
 * La liste ne montre que les destinataires du livre — c'est tout l'objet du lot : un
 * prestataire n'est plus désigné deux fois, et la table entière n'a plus à s'afficher.
 */
test('la liste ne porte que les destinataires déclarés', async () => {
  const { els } = await ouvre([LULU, KDP, COOLLIBRI], {}, { destinataires: [chez(LULU)] });
  assert.deepStrictEqual(els.get('destinataires').textes('span').filter((t) => t.includes('—')), [
    'Lulu — poche 108 × 175',
    '108,0 × 175,0 mm — fond perdu 3,175 mm',
  ]);
  assert.ok(!els.get('dest-papier-kdp-6x9'), 'un prestataire non destinataire est offert');
});

test('on ne peut ajouter que ce qui n\'est pas déjà destinataire', async () => {
  const { els } = await ouvre([LULU, KDP, COOLLIBRI], {}, {
    destinataires: [chez(LULU), chez(KDP)],
  });
  assert.deepStrictEqual(
    els.get('inAjoutDestinataire').textes('option'),
    ['CoolLibri — A5']
  );

  els.get('inAjoutDestinataire').value = 'coollibri-148x210';
  await els.get('btAjouterDestinataire').declenche('click');
  assert.ok(els.get('dest-papier-coollibri-148x210'), 'ajout sans effet à l\'écran');
  assert.strictEqual(
    els.get('btAjouterDestinataire').disabled,
    true,
    'ajouter reste offert alors que la table est épuisée'
  );
});

/**
 * Le dernier destinataire ne se retire pas : c'est lui qui donne son format à l'aperçu,
 * et une liste vide rendrait la Couverture inutilisable. Le Rust refuse ; le bouton
 * s'éteint plutôt que de mener à ce refus.
 */
test('le dernier destinataire ne peut pas être retiré', async () => {
  const { els, appels } = await ouvre([LULU, KDP], {}, {
    destinataires: [chez(LULU), chez(KDP)],
  });
  assert.strictEqual(els.get('dest-retirer-lulu').disabled, false);

  await els.get('dest-retirer-kdp-6x9').declenche('click');
  assert.strictEqual(dernier(appels, 'destinataire_retirer')[1].providerCle, 'kdp-6x9');
  assert.strictEqual(
    els.get('dest-retirer-lulu').disabled,
    true,
    'le dernier destinataire reste retirable'
  );
});

/* ---------- les relevés ---------- */

test('un relevé saisi part au projet, avec le papier de la ligne', async () => {
  const { els, appels } = await ouvre([COOLLIBRI], {}, { destinataires: [chez(COOLLIBRI)] });

  els.get('dest-dos-coollibri-148x210').value = '18.4';
  els.get('dest-fp-coollibri-148x210').value = '4';
  await els.get('dest-dos-coollibri-148x210').declenche('change');

  // Étalé : l'objet vient du contexte `vm`, et `deepStrictEqual` compare les prototypes.
  assert.deepStrictEqual({ ...dernier(appels, 'destinataire_regler')[1].destinataire }, {
    provider: 'coollibri-148x210',
    papier: 'mesure',
    dos_mm: 18.4,
    fond_perdu_mm: 4,
  });
});

/**
 * Un champ vidé est une absence de relevé, pas un zéro. La différence n'est pas
 * cosmétique : un dos de zéro millimètre compose une planche que rien ne refuse, et
 * qui ne se voit qu'au massicot. Un relevé absent, lui, fait refuser la composition.
 */
test('un relevé effacé redevient une absence, jamais un zéro', async () => {
  const { els, appels } = await ouvre([COOLLIBRI], {}, {
    destinataires: [{ ...chez(COOLLIBRI), dos_mm: 18.4, fond_perdu_mm: 4 }],
  });
  assert.strictEqual(els.get('dest-dos-coollibri-148x210').value, '18.4');

  els.get('dest-dos-coollibri-148x210').value = '';
  await els.get('dest-dos-coollibri-148x210').declenche('change');

  assert.strictEqual(dernier(appels, 'destinataire_regler')[1].destinataire.dos_mm, null);
});

/* ---------- génération ---------- */

/**
 * La génération n'envoie plus rien : la liste est dans le projet. Lui repasser des
 * cases cochées rétablirait la double désignation que ce lot supprime.
 */
test('générer ne transmet aucune liste : elle est dans le projet', async () => {
  const { els, appels } = await ouvre([LULU, KDP], {
    packager: () => [{ provider: 'lulu', libelle: 'Lulu', package: paquet(), erreur: null }],
  }, { destinataires: [chez(LULU), chez(KDP)] });

  await els.get('btPackager').declenche('click');
  assert.deepStrictEqual(dernier(appels, 'packager')[1], undefined);
  assert.match(els.get('packages').textContent, /16,51 mm/);
});

/**
 * Un prestataire en échec ne doit pas emporter les autres : ce qui a été produit est
 * livrable, et l'échec doit être lisible plutôt que noyé dans un message global.
 */
test('un prestataire en échec est signalé sans masquer ceux qui ont abouti', async () => {
  const { els } = await ouvre([LULU, KDP], {
    packager: () => [
      { provider: 'lulu', libelle: 'Lulu', package: paquet(), vignette: null, erreur: null },
      {
        provider: 'kdp-6x9',
        libelle: 'Amazon KDP',
        package: null,
        vignette: null,
        erreur: '1200 pages : tranche de gouttière absente du gabarit kdp-6x9',
      },
    ],
  }, { destinataires: [chez(LULU), chez(KDP)] });
  await els.get('btPackager').declenche('click');

  const box = els.get('packages');
  assert.strictEqual(box.hidden, false);
  assert.deepStrictEqual(box.textes('h3'), ['Lulu', 'Amazon KDP']);
  assert.match(box.textContent, /16,51 mm/, 'dos du package abouti absent');
  assert.match(box.textContent, /tranche de gouttière absente/);
});

/**
 * Même promesse que pour la composition : une police que Typst a remplacée sans
 * échouer doit se lire sur le package qu'elle a traversé — c'est ce PDF-là qui part
 * chez l'imprimeur.
 */
test('un package composé par repli porte l\'alerte de police', async () => {
  const { els } = await ouvre([LULU], {
    packager: () => [{
      provider: 'lulu', libelle: 'Lulu', erreur: null,
      package: paquet({ polices_introuvables: ['plume ivan'] }),
    }],
  });
  await els.get('btPackager').declenche('click');

  const t = els.get('packages').textContent;
  assert.match(t, /plume ivan/);
  assert.match(t, /repli/);
});

test('un package sans substitution n\'affiche aucune alerte de police', async () => {
  const { els } = await ouvre([LULU], {
    packager: () => [{ provider: 'lulu', libelle: 'Lulu', package: paquet(), erreur: null }],
  });
  await els.get('btPackager').declenche('click');
  assert.doesNotMatch(els.get('packages').textContent, /repli/);
});

/**
 * Le seul endroit où une maquette unique pour N formats produit un fichier **faux** et
 * non un fichier différent : le corps du dos suit la largeur de couverture, son
 * épaisseur suit la pagination, et la zone qui compose le dos rogne ce qui dépasse sans
 * rien dire. Le compte rendu du package est le dernier écran avant l'imprimeur.
 */
test('un dos trop mince pour son texte porte l\'alerte sur son package', async () => {
  const { els } = await ouvre([LULU], {
    packager: () => [{
      provider: 'lulu', libelle: 'Lulu', erreur: null,
      package: paquet({ dos: 4.2, dos_requis: 6.31 }),
    }],
  });
  await els.get('btPackager').declenche('click');

  const t = els.get('packages').textContent;
  assert.match(t, /4,20 mm/, 'le dos réel doit se lire');
  assert.match(t, /6,31 mm/, 'le dos réclamé doit se lire');
  assert.match(t, /rogné/);
});

test('un dos qui tient n\'affiche aucune alerte', async () => {
  const { els } = await ouvre([LULU], {
    packager: () => [{ provider: 'lulu', libelle: 'Lulu', package: paquet(), erreur: null }],
  });
  await els.get('btPackager').declenche('click');
  assert.doesNotMatch(els.get('packages').textContent, /rogné/);
});

test('un package affiche le dos, la planche et les fichiers produits', async () => {
  const { els } = await ouvre([LULU], {
    packager: () => [{
      provider: 'lulu', libelle: 'Lulu', package: paquet(), vignette: null, erreur: null,
    }],
  });
  await els.get('btPackager').declenche('click');

  const dd = els.get('packages').textes('dd');
  assert.deepStrictEqual(dd, [
    '262 (blanche de parité)',
    'Papier standard',
    '25,0 mm',
    '16,51 mm',
    '238,86 × 181,35 mm, fond perdu 3,175 mm',
  ]);
  assert.match(els.get('packages').textContent, /couverture-lulu\.pdf/);
});

/**
 * Le répertoire une fois, les noms ensuite. Ce n'est pas de la cosmétique : le compte
 * rendu de deux destinataires ne tient dans la fenêtre que si le chemin du projet n'y
 * est pas écrit quatre fois. Ce que le test protège, c'est que les noms de fichiers
 * restent lisibles — pas la mise en page qui les range.
 */
test('les fichiers d\'un package nomment leur répertoire une seule fois', async () => {
  const { els } = await ouvre([LULU], {
    packager: () => [{
      provider: 'lulu', libelle: 'Lulu', package: paquet(), vignette: null, erreur: null,
    }],
  });
  await els.get('btPackager').declenche('click');

  const lignes = els.get('packages').textes('p');
  assert.deepStrictEqual(lignes, [
    '/livres/LHC/lulu/',
    'interieur-lulu.pdf   couverture-lulu.pdf',
  ]);
});

/**
 * Deux fichiers dans deux répertoires n'ont pas de répertoire commun à mettre en
 * facteur : chacun reprend le sien, en entier. Un chemin long se lit ; un chemin
 * raccourci de travers se suit jusqu'à un fichier qui n'existe pas.
 */
test('des fichiers dispersés gardent chacun leur chemin entier', async () => {
  const disperses = { ...paquet(), chemins: ['/a/interieur.pdf', '/b/couverture.pdf'] };
  const { els } = await ouvre([LULU], {
    packager: () => [{
      provider: 'lulu', libelle: 'Lulu', package: disperses, vignette: null, erreur: null,
    }],
  });
  await els.get('btPackager').declenche('click');

  assert.deepStrictEqual(els.get('packages').textes('p'),
    ['/a/interieur.pdf', '/b/couverture.pdf']);
});

/**
 * La vignette est le seul endroit où « est-ce que ça tient » se vérifie sur du vrai,
 * pour chaque prestataire, avec son dos mesuré. Le package qui a échoué n'en a pas —
 * et l'absence ne doit pas poser une image vide, qui se lirait comme une planche.
 */
test('chaque package abouti montre sa planche en vignette', async () => {
  const { els } = await ouvre([LULU, KDP], {
    packager: () => [
      {
        provider: 'lulu',
        libelle: 'Lulu',
        package: paquet(),
        vignette: 'data:image/png;base64,QUJD',
        erreur: null,
      },
      {
        provider: 'kdp-6x9', libelle: 'KDP', package: null, vignette: null, erreur: 'raté',
      },
    ],
  }, { destinataires: [chez(LULU), chez(KDP)] });
  await els.get('btPackager').declenche('click');

  const images = [];
  const visite = (e) => {
    if (e.tagName === 'IMG') images.push(e);
    e.enfants.forEach(visite);
  };
  els.get('packages').enfants.forEach(visite);
  assert.strictEqual(images.length, 1, 'une vignette pour un package en échec');
  assert.strictEqual(images[0].src, 'data:image/png;base64,QUJD');
});

/* ---------- aperçu de la planche ---------- */

/**
 * Le cœur du projet, vu de l'interface : le dos de l'aperçu vient de la composition,
 * jamais d'une saisie. Tant que l'intérieur n'a pas été composé, il n'y a pas de dos
 * à passer — et la planche refusera de s'afficher plutôt que d'en inventer un.
 */
test('l\'aperçu de planche n\'a pas de dos tant que l\'intérieur n\'est pas composé', async () => {
  const { els, appels } = await ouvre([LULU], {}, { couverture: {} });
  await face(els, 'Planche').declenche('click');
  await attendreApercu();

  const [, args] = dernier(appels, 'couverture_apercu');
  assert.strictEqual(args.face, 'planche');
  assert.strictEqual(args.dosMm, null, 'un dos est passé sans composition');
});

/**
 * Le gabarit ne voyage plus avec l'aperçu : le Rust le lit dans le projet. Le repasser
 * ici rouvrirait la porte à deux vérités sur le prestataire courant.
 */
test('l\'aperçu ne transporte plus de gabarit', async () => {
  const { els, appels } = await ouvre([LULU], {}, { couverture: {} });
  await face(els, '1ère').declenche('click');
  await attendreApercu();

  assert.deepStrictEqual(
    Object.keys(dernier(appels, 'couverture_apercu')[1]).sort(),
    ['dosMm', 'face']
  );
});

test('une fois l\'intérieur composé, l\'aperçu de planche reçoit ce dos-là', async () => {
  const { els, appels } = await ouvre([LULU], { composer: COMPOSITION }, { couverture: {} });
  await els.get('btComposer').declenche('click');
  await face(els, 'Planche').declenche('click');
  await attendreApercu();

  assert.strictEqual(dernier(appels, 'couverture_apercu')[1].dosMm, 16.513);
});

/** Composer, c'est composer pour le destinataire visé : plus rien à lui désigner. */
test('composer ne transmet plus de prestataire', async () => {
  const { els, appels } = await ouvre([LULU], { composer: COMPOSITION });
  await els.get('btComposer').declenche('click');
  assert.deepStrictEqual(dernier(appels, 'composer')[1], undefined);
});

/**
 * Le dos vaut pour un gabarit et un seul : le même manuscrit ne fait pas le même
 * nombre de pages en poche et en grand format. Le traîner d'un destinataire à l'autre
 * donnerait à voir une planche fausse, et c'est exactement le défaut que l'atelier
 * HTML avait.
 */
test('viser un autre destinataire périme le dos de l\'aperçu', async () => {
  const { els, appels } = await ouvre([LULU, KDP], { composer: COMPOSITION }, {
    couverture: {}, destinataires: [chez(LULU), chez(KDP)],
  });
  await els.get('btComposer').declenche('click');
  await face(els, 'Planche').declenche('click');
  await attendreApercu();
  assert.strictEqual(dernier(appels, 'couverture_apercu')[1].dosMm, 16.513);

  els.get('inDestinataire').value = 'kdp-6x9';
  await els.get('inDestinataire').declenche('change');
  await attendreApercu();
  assert.strictEqual(
    dernier(appels, 'couverture_apercu')[1].dosMm,
    null,
    'dos de Lulu réutilisé pour KDP'
  );
});

/**
 * Le papier est la cause la plus chère des trois, parce qu'il déplace le dos **sans
 * passer par la pagination** : chez KDP, 0,0635 mm par page en crème contre 0,0572 en
 * blanc, soit 1,65 mm d'écart sur 262 pages — l'épaisseur d'une couverture entière.
 */
test('un dos calculé sur un autre papier ne vaut plus rien', async () => {
  const { els, appels } = await ouvre([KDP], { composer: COMPOSITION }, { couverture: {} });
  await els.get('btComposer').declenche('click');
  await face(els, 'Planche').declenche('click');
  await attendreApercu();
  assert.strictEqual(dernier(appels, 'couverture_apercu')[1].dosMm, 16.513);

  els.get('dest-papier-kdp-6x9').value = 'blanc';
  await els.get('dest-papier-kdp-6x9').declenche('change');
  await attendreApercu();
  assert.strictEqual(
    dernier(appels, 'couverture_apercu')[1].dosMm,
    null,
    'dos du papier crème réutilisé pour le blanc'
  );
});

/**
 * Même raison, autre cause : la police repagine le livre. Un dos calculé en Alegreya
 * n'est plus le dos du livre dès qu'on le compose en Cardo, et le laisser sur la
 * planche donnerait un chiffre faux — ce qui vaut moins que pas de chiffre.
 */
test('un dos calculé pour une autre police ne vaut plus rien', async () => {
  const { els, appels } = await ouvre([LULU], { composer: COMPOSITION }, { couverture: {} });
  await els.get('btComposer').declenche('click');
  await face(els, 'Planche').declenche('click');
  await attendreApercu();
  assert.strictEqual(dernier(appels, 'couverture_apercu')[1].dosMm, 16.513);

  els.get('inPoliceInterieur').value = 'Cardo';
  await els.get('inPoliceInterieur').declenche('change');
  await attendreApercu();
  assert.strictEqual(
    dernier(appels, 'couverture_apercu')[1].dosMm,
    null,
    'dos d\'Alegreya réutilisé pour Cardo'
  );
});

/**
 * **Le test qui porte le lot.** Le même livre a autant de paginations que de gabarits,
 * et chacune coûte une composition entière. Les retenir une par destinataire, dans le
 * projet, fait de la lunette ce qu'elle prétend être : revenir sur un prestataire déjà
 * composé retrouve son dos, sans rien recalculer et sans emprunter celui du voisin.
 *
 * Le compte des `composer` est la moitié du test : sans lui, une implémentation qui
 * recomposerait en douce à chaque aller-retour passerait pour juste.
 */
test('revenir à un destinataire déjà composé retrouve son dos sans recomposer', async () => {
  const dos = [16.513, 21.4];
  let n = 0;
  const { els, appels } = await ouvre([LULU, KDP], {
    composer: () => ({ ...COMPOSITION, dos: dos[n++] }),
  }, { couverture: {}, destinataires: [chez(LULU), chez(KDP)] });
  const vise = async (cle) => {
    els.get('inDestinataire').value = cle;
    await els.get('inDestinataire').declenche('change');
    await attendreComposition();
  };
  await els.get('btComposer').declenche('click');
  await face(els, 'Planche').declenche('click');
  await attendreApercu();
  assert.strictEqual(dernier(appels, 'couverture_apercu')[1].dosMm, 16.513);

  // KDP n'a jamais été composé : la veille s'en charge, et lui donne son dos à lui.
  await vise('kdp-6x9');
  assert.strictEqual(dernier(appels, 'couverture_apercu')[1].dosMm, 21.4);

  const avant = combien(appels, 'composer');
  await vise('lulu');
  assert.strictEqual(dernier(appels, 'couverture_apercu')[1].dosMm, 16.513,
    'le dos de Lulu n\'a pas été retrouvé');
  assert.strictEqual(combien(appels, 'composer'), avant,
    'revenir sur un destinataire déjà composé a recomposé');
});

/**
 * Ce que le lot rend : une mesure périmée se refait toute seule. Le geste qui l'a
 * périmée — ici la police — suffit, et le bouton n'est plus qu'un recours.
 */
test('une modification recompose d\'elle-même, une fois le livre composé', async () => {
  const { els, appels } = await ouvre([LULU], { composer: COMPOSITION }, { couverture: {} });
  await els.get('btComposer').declenche('click');
  assert.strictEqual(combien(appels, 'composer'), 1);

  els.get('inPoliceInterieur').value = 'Cardo';
  await els.get('inPoliceInterieur').declenche('change');
  await attendreComposition();

  assert.strictEqual(combien(appels, 'composer'), 2, 'la police n\'a rien relancé');
});

/**
 * L'autre moitié de la règle, et la plus importante : **rien ne part avant le premier
 * clic**. Une composition dure des secondes et écrit des fichiers ; la déclencher chez
 * quelqu'un qui n'a jamais rien composé — qui règle une couverture, par exemple —
 * coûterait plus cher que le clic qu'on lui épargne.
 */
test('rien ne se compose tout seul tant qu\'on ne l\'a pas demandé une fois', async () => {
  const { els, appels } = await ouvre([LULU], { composer: COMPOSITION }, { couverture: {} });

  els.get('inPoliceInterieur').value = 'Cardo';
  await els.get('inPoliceInterieur').declenche('change');
  els.get('inDedicace').value = 'À M.';
  await els.get('inDedicace').declenche('change');
  await attendreComposition();

  assert.strictEqual(combien(appels, 'composer'), 0, 'composé sans qu\'on le demande');
});

/**
 * Une composition dure des secondes ; ce qu'on modifie pendant qu'elle tourne rend son
 * résultat faux à l'instant où il arrive. Deux exigences, et la seconde est celle qui
 * fait mal : n'en lancer qu'une à la fois — deux en parallèle se sérialiseraient sur le
 * verrou du Rust et on paierait les deux —, et **recommencer** quand quelque chose a
 * bougé entre-temps, alors même que la composition qui vient de finir a déposé une
 * mesure d'apparence fraîche.
 */
test('une modification pendant la composition la fait recommencer, une fois', async () => {
  let enCours = 0;
  let parallele = 0;
  const { els, appels } = await ouvre([LULU], {
    composer: async () => {
      enCours += 1;
      parallele = Math.max(parallele, enCours);
      await new Promise((r) => setTimeout(r, 1000));
      enCours -= 1;
      return COMPOSITION;
    },
  }, { couverture: {} });
  await els.get('btComposer').declenche('click');

  els.get('inPoliceInterieur').value = 'Cardo';
  await els.get('inPoliceInterieur').declenche('change');
  await new Promise((r) => setTimeout(r, 600));
  // La recomposition tourne depuis 200 ms : la dédicace arrive en plein milieu.
  els.get('inDedicace').value = 'À M.';
  await els.get('inDedicace').declenche('change');
  await new Promise((r) => setTimeout(r, 3000));

  assert.strictEqual(parallele, 1, 'deux compositions en parallèle');
  assert.strictEqual(combien(appels, 'composer'), 3,
    'la modification arrivée en cours de route n\'a pas fait recommencer');
});

/**
 * Le bouton reste un recours, et l'employer doit désarmer la veille : sans quoi une
 * impatience — modifier puis cliquer aussitôt — se paierait d'une seconde composition,
 * qui recalculerait à l'identique ce que le clic venait d'obtenir.
 */
test('composer à la main pendant l\'attente annule la recomposition', async () => {
  const { els, appels } = await ouvre([LULU], { composer: COMPOSITION }, { couverture: {} });
  await els.get('btComposer').declenche('click');

  els.get('inPoliceInterieur').value = 'Cardo';
  await els.get('inPoliceInterieur').declenche('change');
  await els.get('btComposer').declenche('click');
  await attendreComposition();

  assert.strictEqual(combien(appels, 'composer'), 2, 'la veille a recomposé par-dessus');
});

/**
 * Un livre enregistré dans un état périmé réclame bien une composition, mais l'ouvrir
 * n'est pas la demander : on ouvre aussi un `.ozalid` pour regarder sa couverture, et
 * une minute de Typst au premier double-clic serait exactement le genre de zèle qu'on
 * reproche à une application.
 */
test('ouvrir un livre dont la mesure est périmée ne compose rien', async () => {
  const { appels } = await ouvre([LULU], { composer: COMPOSITION }, {
    couverture: {}, dejaCompose: true,
  });
  await attendreComposition();

  assert.strictEqual(combien(appels, 'composer'), 0, 'composé à la seule ouverture');
});

/**
 * La cause qu'aucune estampille ne voyait : le livre lui-même compose des pages
 * liminaires. Une dédicace prend une belle page et son verso blanc — deux pages de plus,
 * et le corps s'ouvre en page 7 au lieu de 5 (`interieur.rs`, test
 * `une_dedicace_ajoute_une_belle_page_et_sa_blanche`). Le gabarit, le papier et la
 * police n'ont pas bougé d'un pouce, et le dos n'est pourtant plus le même.
 *
 * La péremption est volontairement grossière — n'importe quelle modification du livre,
 * sans regarder si elle pagine — pour la même raison que le manuscrit : la liste des
 * champs qui composent vit dans `interieur::source`, et une liste tenue en double ici
 * finirait par diverger sans que rien ne le dise.
 */
test('un dos calculé avant la dédicace ne vaut plus rien', async () => {
  const { els, appels } = await ouvre([LULU], { composer: COMPOSITION }, { couverture: {} });
  await els.get('btComposer').declenche('click');
  await face(els, 'Planche').declenche('click');
  await attendreApercu();
  assert.strictEqual(dernier(appels, 'couverture_apercu')[1].dosMm, 16.513);

  els.get('inDedicace').value = 'À M., qui a tenu la lampe.';
  await els.get('inDedicace').declenche('change');
  await attendreApercu();
  assert.strictEqual(
    dernier(appels, 'couverture_apercu')[1].dosMm,
    null,
    'dos d\'avant la dédicace réutilisé'
  );
});

/**
 * La dernière cause, et la seule qui ne se lise nulle part : le texte fait la
 * pagination. Un dos calculé sur le manuscrit d'avant ne vaut rien même si le gabarit,
 * le papier et la police n'ont pas bougé — c'est précisément ce qui la rend facile à
 * oublier. Les deux portes par lesquelles le texte est remplacé sont exercées ici.
 */
test('un dos calculé sur un autre manuscrit ne vaut plus rien', async () => {
  const { els, appels } = await ouvre([LULU], { composer: COMPOSITION }, { couverture: {} });
  const dernierDos = () => dernier(appels, 'couverture_apercu')[1].dosMm;

  await els.get('btComposer').declenche('click');
  await face(els, 'Planche').declenche('click');
  await attendreApercu();
  assert.strictEqual(dernierDos(), 16.513);

  await els.get('btReimporter').declenche('click');
  await attendreApercu();
  assert.strictEqual(dernierDos(), null, 'dos gardé après une réimportation du manuscrit');

  await els.get('btComposer').declenche('click');
  await attendreApercu();
  assert.strictEqual(dernierDos(), 16.513);

  await els.get('btChoisirManuscrit').declenche('click');
  await attendreApercu();
  assert.strictEqual(dernierDos(), null, 'dos gardé après un changement de manuscrit');
});

/**
 * Le dos n'est pas seul à sortir du texte, et il est le seul qui ne se lise nulle part :
 * la pagination, les chemins des fichiers composés et les envois déjà écrits en parlent
 * aussi, sous les yeux et en chiffres. Une application dont l'objet est que le nombre de
 * pages soit vrai ne peut pas afficher celui d'un manuscrit qu'on vient de remplacer.
 */
test('réimporter le manuscrit efface ce que l\'ancien texte avait fait afficher', async () => {
  const { els } = await ouvre([LULU], {
    composer: COMPOSITION,
    packager: [paquet()],
    epreuve_tirer: '/livres/LHC/epreuve.pdf',
  }, { couverture: {} });

  await els.get('btComposer').declenche('click');
  await els.get('btPackager').declenche('click');
  await els.get('btEpreuve').declenche('click');
  // Un envoi porte lui aussi un compte de pages et un dos ; le composer demanderait une
  // liste de dédicataires que ce projet-là n'a pas, et c'est ce qu'il laisse qui compte.
  els.get('resultatEnvois').textContent = 'Rex — envois/rex/ — 262 pages, dos 16,51 mm';
  els.get('resultatEnvois').hidden = false;
  assert.strictEqual(els.get('resultat').hidden, false, 'rien à effacer, test sans objet');

  await els.get('btReimporter').declenche('click');

  assert.strictEqual(els.get('resultat').hidden, true,
    'la pagination de l\'ancien texte reste à lire');
  assert.strictEqual(els.get('packages').hidden, true,
    'les packages de l\'ancien texte restent à lire');
  assert.strictEqual(els.get('resultatEnvois').hidden, true,
    'les envois de l\'ancien texte restent à lire');
  assert.strictEqual(els.get('cheminEpreuve').textContent, '',
    'l\'épreuve de l\'ancien texte reste désignée');
});

/**
 * Remplacer le texte n'est pas changer de livre : le projet, ses destinataires et
 * l'étape où l'on travaille sont les mêmes avant et après. Oublier les sorties du
 * précédent — celles qui renvoient à l'accueil et vident la liste — renverrait au Livre
 * quelqu'un qui venait de réimporter depuis la Livraison.
 */
test('réimporter le manuscrit ne quitte pas l\'étape où l\'on travaille', async () => {
  const { els } = await ouvre([LULU], { composer: COMPOSITION }, { couverture: {} });
  await els.get('onglet-livraison').declenche('click');
  assert.strictEqual(els.get('etapeLivraison').hidden, false);

  await els.get('btReimporter').declenche('click');

  assert.strictEqual(els.get('etapeLivraison').hidden, false,
    'un réimport a renvoyé au Livre');
  assert.ok(els.get('destinataires').textes('span').includes('Lulu — poche 108 × 175'),
    'un réimport a vidé la liste des destinataires');
});
