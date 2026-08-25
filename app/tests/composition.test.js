'use strict';

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

/** La livraison d'un livre qui n'a qu'un destinataire, comme un projet neuf en a un. */
const livraison = (p) => ({
  destinataires: [{
    provider: p.cle, papier: p.papiers[0].cle, dos_mm: null, fond_perdu_mm: null,
  }],
  courant: p.cle,
});

const PROJET = {
  chemin: '/livres/LHC.ozalid',
  livre: {
    titre: 'Les Heures creuses', titre_page: 'Les Heures\ncreuses',
    auteur: 'Ivan Pjig', genre: 'roman', copyright: '© Ivan Pjig, 2026.',
    chapitres: 64,
  },
  manuscrit_absent: false,
  modifie: false,
  manuscrit_source: '/dev/ozalid/build/in/texts/WIP7.md',
  chapitres_trouves: 64,
  mots: 49344,
  couverture_importee: true,
  images: ['couverture.jpg'],
  interieur: { police: 'Alegreya' },
  envois: { main: { mode: 'police', police: 'Caveat' }, liste: [] },
  livraison: livraison(LULU),
};

/**
 * Le geste qui compose, depuis que le bouton n'existe plus : charger un manuscrit.
 *
 * C'est le consentement du chantier « intérieur sans onglet » — ouvrir un `.ozalid` ne
 * compose pas, charger un manuscrit oui. Les tests qui ont besoin d'un livre composé
 * passent donc par là, comme l'utilisateur.
 */
const faireComposer = async (els) => {
  await els.get('btReimporter').declenche('click');
  // `manuscritRemplace` lance la composition sans l'attendre — l'utilisateur non plus.
  // Un tour de boucle pour qu'elle aboutisse avant qu'on regarde le résultat.
  await new Promise((r) => setImmediate(r));
};

/** Fausse implémentation des commandes Rust. `sur` surcharge une commande. */
function faux(providers, sur = {}) {
  // Le projet que ce faux sert, quel que soit le chemin par lequel on l'a ouvert : c'est
  // lui que `manuscrit_reimporter` doit rendre, et non un `PROJET` figé — un test qui
  // vise CoolLibri repasserait chez Lulu au premier rechargement.
  const servi = sur.projet_ouvrir ?? sur.projet_importer ?? PROJET;
  return async (cmd, args) => {
    if (cmd === 'providers_liste') return providers;
    // Recharger un manuscrit périme tout ce qui a été mesuré : c'est la règle du Rust,
    // et sans elle le front n'aurait rien à recomposer.
    if (cmd === 'manuscrit_reimporter' && !(cmd in sur)) {
      return {
        ...servi,
        livraison: {
          ...servi.livraison,
          destinataires: servi.livraison.destinataires.map(({ compose, ...d }) => d),
        },
      };
    }
    if (cmd === 'polices_liste') return ['Bodoni Moda', 'Archivo', 'Spectral'];
    if (cmd === 'polices_texte_liste') return ['EB Garamond', 'Alegreya', 'Cardo'];
    if (cmd === 'jetons_liste') return ['%TITRE%', '%AUTEUR%', '%GENRE%', '%EDITEUR%', '%COLLECTION%', '%MONOGRAMME%'];
    if (cmd === 'mains_liste') return ['Caveat', 'Dancing Script'];
    if (cmd === 'maquettes_liste') {
      return [{ cle: 'bandeau', libelle: 'Bandeau' }, { cle: 'filets', libelle: 'Filets' }];
    }
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
    if (cmd === 'diffusion_lire') return { url: '', modele: '', cle_posee: false };
    throw new Error(`commande inattendue : ${cmd}`);
  };
}

const PDF = '/livres/LHC/lulu/interieur-lulu.pdf';
const MESURE = { pages: 262, gouttiere: 25, blanche: true, dos: 16.513 };

/**
 * Ce que `composer` rend.
 *
 * Les chiffres du dessus sont une **copie de lecture** ; ce qui compte est le `projet`,
 * où la mesure est rangée chez son destinataire. C'est de là que le pied la lit — et
 * c'est ce qui la fait survivre à la réouverture du livre, là où un panneau rempli
 * depuis le retour de commande se serait tu.
 *
 * `m` surcharge la mesure, `polices_introuvables` compris : c'est elle qui les porte
 * désormais, et non le seul retour de commande.
 */
const composition = (p = LULU, m = {}, pdf = PDF) => {
  const mesure = { ...MESURE, ...m };
  const l = livraison(p);
  return {
    ...mesure,
    chapitres: 64,
    pdf,
    polices_introuvables: mesure.polices_introuvables ?? [],
    projet: {
      ...PROJET,
      interieur_pdf: pdf,
      livraison: {
        ...l,
        deja_compose: true,
        destinataires: [{ ...l.destinataires[0], compose: mesure }],
      },
    },
  };
};

const COMPOSITION = composition();

/* ---------- destinataires ---------- */

/** Un projet ouvert, visé sur le prestataire donné. */
async function ouvre(p, sur = {}) {
  const projet = { ...PROJET, livraison: livraison(p) };
  const ctx = await charge({
    invoke: faux([p], { projet_ouvrir: projet, ...sur }),
    open: async () => '/livres/LHC.ozalid',
  });
  await ctx.els.get('btOuvrir').declenche('click');
  return ctx;
}

test('le choix du papier n\'est offert que quand il y en a plusieurs', async () => {
  const { els } = await ouvre(LULU);
  assert.strictEqual(els.get('dest-papier-lulu').disabled, true);
  assert.strictEqual(els.get('dest-papier-lulu').children.length, 1);

  const { els: chezKdp } = await ouvre(KDP);
  assert.strictEqual(chezKdp.get('dest-papier-kdp-6x9').disabled, false);
  assert.deepStrictEqual(
    [...chezKdp.get('dest-papier-kdp-6x9').children].map((o) => o.value),
    ['creme', 'blanc']
  );
});

test('un prestataire à gabarit annonce que le fond perdu se relève', async () => {
  const { els } = await ouvre(COOLLIBRI);
  const note = els.get('destinataires').textContent;
  assert.match(note, /148,0 × 210,0 mm/);
  assert.match(note, /relever sur le gabarit/);
  assert.doesNotMatch(note, /fond perdu \d/, 'aucun chiffre de fond perdu inventé');
});

/**
 * Le pied dit pour qui l'on regarde, et le dos n'y paraît qu'une fois composé : le pas
 * encore mesuré ne doit jamais s'y lire comme un chiffre.
 */
test('le pied nomme le destinataire visé et l\'état de son dos', async () => {
  const { els } = await ouvre(LULU);
  assert.strictEqual(els.get('inDestinataire').value, 'lulu');
  assert.deepStrictEqual(els.get('inDestinataire').textes('option'), ['Lulu — poche 108 × 175']);
  assert.match(els.get('piedDos').textContent, /dos non composé/);
});

/**
 * Chez un prestataire sans formule, il n'y a jamais rien à composer : « non composé »
 * ferait recomposer en boucle un livre dont la pagination est déjà juste.
 */
test('un prestataire à gabarit ne réclame pas une composition mais un relevé', async () => {
  const { els } = await ouvre(COOLLIBRI);
  assert.match(els.get('piedDos').textContent, /relevé sur le gabarit/);
});

/* ---------- projet ---------- */

test('rien n\'est proposé tant qu\'aucun projet n\'est ouvert', async () => {
  const { els } = await charge({ invoke: faux([LULU]) });
  assert.strictEqual(els.get('accueil').hidden, false);
  for (const s of ['etapeLivre', 'etapeCouverture', 'etapeLivraison', 'etapeEnvois']) {
    assert.strictEqual(els.get(s).hidden, true, `${s} visible sans projet`);
  }
});

test('un projet importé remplit les champs et ouvre la première étape', async () => {
  const { els } = await charge({
    invoke: faux([LULU], { projet_importer: PROJET }),
    open: async () => '/dev/ozalid/build/LHC/livre.toml',
  });
  await els.get('btImporter').declenche('click');

  assert.strictEqual(els.get('inTitre').value, 'Les Heures creuses');
  assert.strictEqual(els.get('inTitrePage').value, 'Les Heures\ncreuses');
  // Une chaîne, comme dans la fenêtre : `input.value` n'est jamais un nombre.
  assert.strictEqual(els.get('inChapitres').value, '64');
  assert.strictEqual(els.get('etapeLivre').hidden, false);
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
    invoke: faux([LULU], { projet_ouvrir: sansSource }),
    open: async () => '/livres/X.ozalid',
  });
  await els.get('btOuvrir').declenche('click');
  assert.strictEqual(els.get('btReimporter').disabled, true);
  assert.match(els.get('sourceManuscrit').textContent, /aucune source/);
});

/**
 * Sans chemin, il n'y a rien à réécrire : c'est l'entête qui porte cet état, et le
 * geste d'enregistrement qui bascule sur « Enregistrer sous… » — vérifié pour sa
 * part dans `cycle_de_vie.test.js`.
 */
test('un projet non enregistré le dit dans l\'entête', async () => {
  const neuf = { ...PROJET, chemin: null };
  const { els } = await charge({
    invoke: faux([LULU], { projet_importer: neuf }),
    open: async () => '/x/livre.toml',
  });
  await els.get('btImporter').declenche('click');
  assert.match(els.get('cheminProjet').textContent, /non enregistré/);
  assert.strictEqual(els.get('etatEnregistrement').textContent, 'jamais enregistré');
});

/* ---------- composition ---------- */

/**
 * Le pied porte la légende de la dernière composition : les pages, les chapitres, la
 * gouttière et le dos. Le chiffre que l'application existe pour ne pas faire ressaisir
 * doit se lire depuis n'importe quelle étape, sans qu'on ait à revenir le chercher.
 *
 * Il ne dit plus la page blanche de parité : on la regarde une fois, et une légende qui
 * suit partout n'a pas de place pour ce qui ne se relit jamais.
 */
test('le pied porte les chiffres de la composition', async () => {
  const { els } = await charge({
    invoke: faux([LULU], { projet_ouvrir: PROJET, composer: COMPOSITION }),
    open: async () => '/livres/LHC.ozalid',
  });
  await els.get('btOuvrir').declenche('click');
  await faireComposer(els);

  assert.strictEqual(
    els.get('piedMesure').textContent,
    '· 262 pages · 64 chapitres · gouttière 25,00 mm',
  );
  assert.strictEqual(els.get('piedDos').textContent, '· dos 16,5 mm');
  assert.strictEqual(els.get('piedInterieur').textContent, '· intérieur');
});

/**
 * Le consentement appartient au **livre ouvert**, pas à la fenêtre.
 *
 * Sans cela, avoir chargé un manuscrit dans un premier livre ferait composer le
 * suivant au premier geste — alors même qu'on vient seulement de l'ouvrir. C'est le
 * pari du chantier retourné contre lui-même : ouvrir n'est pas demander, et ça vaut
 * aussi pour le deuxième livre de la session.
 */
test('ouvrir un autre livre retire le consentement du précédent', async () => {
  const vus = [];
  const base = faux([LULU], {
    projet_ouvrir: PROJET,
    composer: COMPOSITION,
    livre_modifier: PROJET,
  });
  const invoke = async (cmd, args) => {
    vus.push(cmd);
    return base(cmd, args);
  };
  const { els } = await charge({ invoke, open: async () => '/livres/LHC.ozalid' });

  await els.get('btOuvrir').declenche('click');
  await faireComposer(els);
  const apres = vus.length;

  // Un autre livre s'ouvre. `PROJET` ne porte aucune mesure et n'a jamais été composé :
  // si le consentement l'avait suivi, le premier geste ferait tourner Typst.
  await els.get('btOuvrir').declenche('click');
  els.get('inDedicace').value = 'À M.';
  await els.get('inDedicace').declenche('change');
  await new Promise((r) => setTimeout(r, 700));

  assert.ok(
    !vus.slice(apres).includes('composer'),
    'le consentement du livre précédent a suivi dans le suivant',
  );
});

/**
 * L'import d'un `livre.toml` apporte un manuscrit : c'est le même geste que d'en choisir
 * un, et il consent comme lui. Il ne passe pas par le même entonnoir du front — c'est
 * exactement le déclencheur qu'on rate, et sans lui un livre importé resterait sans dos
 * jusqu'au premier geste.
 */
test('importer un livre.toml compose, comme charger un manuscrit', async () => {
  const vus = [];
  const base = faux([LULU], { projet_importer: PROJET, composer: COMPOSITION });
  const invoke = async (cmd, args) => {
    vus.push(cmd);
    return base(cmd, args);
  };
  const { els } = await charge({
    invoke,
    open: async () => '/dev/ozalid/build/LHC/livre.toml',
  });

  await els.get('btImporter').declenche('click');
  await new Promise((r) => setImmediate(r));

  assert.ok(vus.includes('composer'), 'un livre.toml importé n\'a rien composé');
});

/**
 * Un dos périmé fait taire les chiffres, et ce n'est pas un raffinement : laisser lire
 * « 262 pages » sous un « dos périmé » donnerait à croire une pagination que la ligne
 * d'à côté vient de déclarer fausse.
 */
test('un dos périmé fait taire la légende', async () => {
  const c = composition();
  const perime = {
    ...c.projet,
    livraison: {
      ...c.projet.livraison,
      destinataires: c.projet.livraison.destinataires.map(({ compose, ...d }) => d),
    },
  };
  const { els } = await charge({
    invoke: faux([LULU], { projet_ouvrir: perime }),
    open: async () => '/livres/LHC.ozalid',
  });

  await els.get('btOuvrir').declenche('click');

  assert.strictEqual(els.get('piedDos').textContent, '· dos périmé');
  assert.strictEqual(els.get('piedMesure').textContent, '');
  assert.strictEqual(els.get('piedInterieur').textContent, '');
});

/**
 * Le lien du pied ne vaut que si le Rust a trouvé le fichier. `interieur_pdf` est déjà
 * filtré par son existence là-bas : un PDF effacé à la main entre deux ouvertures rend
 * un lien qui ne mène nulle part, et un lien mort est pire que pas de lien.
 */
test('sans PDF sur le disque, le pied ne propose pas de lien', async () => {
  const c = composition();
  const { els } = await charge({
    invoke: faux([LULU], {
      projet_ouvrir: PROJET,
      composer: { ...c, projet: { ...c.projet, interieur_pdf: null } },
    }),
    open: async () => '/livres/LHC.ozalid',
  });
  await els.get('btOuvrir').declenche('click');
  await faireComposer(els);

  assert.strictEqual(els.get('piedInterieur').textContent, '');
  assert.match(els.get('piedMesure').textContent, /262 pages/,
    'les chiffres partent avec le lien');
});

/**
 * **Le test qui compte de ce lot.** La légende se lit dans le projet, pas dans le retour
 * de `composer` : rouvrir un livre composé la veille doit la retrouver entière, sans
 * recomposer. C'est tout ce qui distingue une mesure retenue d'un compte rendu d'écran.
 */
test('la légende du pied survit à une réouverture, sans composer', async () => {
  const { els } = await charge({
    invoke: faux([LULU], { projet_ouvrir: composition().projet }),
    open: async () => '/livres/LHC.ozalid',
  });

  await els.get('btOuvrir').declenche('click');

  assert.match(els.get('piedMesure').textContent, /262 pages · 64 chapitres/);
  assert.strictEqual(els.get('piedDos').textContent, '· dos 16,5 mm');
});

/**
 * Typst peut réussir en remplaçant une police introuvable par une écriture de repli :
 * le PDF existe, les chiffres sont justes, mais le rendu n'est pas celui de la
 * maquette. Le warning part sur un stderr qu'aucune fenêtre ne montre.
 *
 * Deux endroits, et il les faut tous les deux : un signe au pied, qui suit dans toutes
 * les étapes et se voit depuis la Couverture où l'on regarde le résultat ; le détail —
 * quelles familles — sous le sélecteur de police, là où l'on va réparer.
 */
test('une police composée par repli se signale au pied et sous la police', async () => {
  const { els } = await charge({
    invoke: faux([LULU], {
      projet_ouvrir: PROJET,
      composer: composition(LULU, { polices_introuvables: ['bodoni moda'] }),
    }),
    open: async () => '/livres/LHC.ozalid',
  });
  await els.get('btOuvrir').declenche('click');
  await faireComposer(els);

  assert.match(els.get('piedRepli').textContent, /repli/);
  assert.strictEqual(els.get('piedRepli').className, 'alerte');
  assert.strictEqual(els.get('repliPolices').hidden, false);
  assert.match(els.get('repliPolices').textContent, /bodoni moda/);
});

/**
 * Et il doit être là **après une réouverture** : un PDF composé dans une écriture de
 * repli ne redevient pas juste en refermant le livre. C'est pour cela que la mesure le
 * retient, et non l'écran.
 */
test('le repli de police survit à une réouverture', async () => {
  const c = composition(LULU, { polices_introuvables: ['bodoni moda'] });
  const { els } = await charge({
    invoke: faux([LULU], { projet_ouvrir: c.projet }),
    open: async () => '/livres/LHC.ozalid',
  });

  await els.get('btOuvrir').declenche('click');

  assert.match(els.get('piedRepli').textContent, /repli/);
  assert.match(els.get('repliPolices').textContent, /bodoni moda/);
});

test('une composition sans substitution n\'alerte nulle part', async () => {
  const { els } = await charge({
    invoke: faux([LULU], { projet_ouvrir: PROJET, composer: COMPOSITION }),
    open: async () => '/livres/LHC.ozalid',
  });
  await els.get('btOuvrir').declenche('click');
  await faireComposer(els);

  assert.strictEqual(els.get('piedRepli').textContent, '');
  assert.strictEqual(els.get('repliPolices').hidden, true);
});

/* ---------- échantillon d'écriture ---------- */

/** Le romain de la police d'intérieur, tel que le Rust le rend : des octets, en `data:`. */
const DONNEE_POLICE = 'data:font/ttf;base64,AAEAAAA=';

/**
 * L'échantillon montre l'écriture **choisie**, dans ses propres octets — ceux que Typst
 * composera. Un `font-family` posé sur le seul nom de la famille aurait pris la police
 * du poste quand elle s'y trouve, et rien n'aurait distingué les deux à l'écran.
 */
test('l\'échantillon est rendu dans la police d\'intérieur du projet', async () => {
  const demandes = [];
  const { els, faces } = await charge({
    invoke: faux([LULU], {
      projet_ouvrir: PROJET,
      police_texte_donnee: (args) => {
        demandes.push(args.famille);
        return DONNEE_POLICE;
      },
    }),
    open: async () => '/livres/LHC.ozalid',
  });
  await els.get('btOuvrir').declenche('click');
  await new Promise((r) => setImmediate(r));

  assert.deepStrictEqual(demandes, ['Alegreya']);
  assert.strictEqual(faces.length, 1, 'aucune police chargée dans la fenêtre');
  assert.match(faces[0].source, /AAEAAAA=/, 'la face ne porte pas les octets du Rust');
  const echantillon = els.get('echantillonPolice');
  assert.strictEqual(echantillon.hidden, false);
  // La face porte un nom qui n'existe sur aucun système : sans cela, une « Alegreya »
  // installée sur le poste passerait devant celle du livre.
  assert.strictEqual(
    echantillon.style.getPropertyValue('--police-echantillon'),
    `"${faces[0].family}"`
  );
  assert.notStrictEqual(faces[0].family, 'Alegreya');
});

/**
 * Le texte d'exemple porte ce sur quoi une écriture française se choisit : accents,
 * ligature œ, guillemets et apostrophe courbe. Un « Lorem ipsum » ne montrerait aucun
 * des caractères qui font préférer une police à une autre — et c'est précisément sur
 * ceux-là qu'une police embarquée peut manquer.
 */
test('le texte d\'exemple montre ce qui distingue une écriture française', () => {
  const html = fs.readFileSync(path.join(__dirname, '..', 'src', 'index.html'), 'utf8');
  const texte = html.match(/id="echantillonPolice"[^>]*>([^<]*)</)[1];
  for (const c of ['œ', '’', '«', 'î']) {
    assert.ok(texte.includes(c), `« ${c} » absent du texte d'exemple : ${texte}`);
  }
});

/**
 * Une police que la fenêtre ne peut pas charger ne doit **rien** montrer : le repli d'un
 * navigateur est muet, comme celui de Typst, et un échantillon rendu dans l'écriture de
 * l'interface donnerait à voir une police que le livre n'aura pas.
 */
test('une police illisible ne se montre pas dans une autre écriture', async () => {
  const { els } = await charge({
    invoke: faux([LULU], {
      projet_ouvrir: PROJET,
      police_texte_donnee: () => {
        throw 'police d\'intérieur « Alegreya » introuvable dans les polices embarquées';
      },
    }),
    open: async () => '/livres/LHC.ozalid',
  });
  await els.get('btOuvrir').declenche('click');
  await new Promise((r) => setImmediate(r));

  assert.strictEqual(els.get('echantillonPolice').hidden, true);
  assert.strictEqual(els.get('echantillonAbsent').hidden, false);
  assert.match(els.get('echantillonAbsent').textContent, /Alegreya/);
});

/** Une face qui refuse de se charger est le même cas qu'un Rust qui refuse de la lire. */
test('une police que la fenêtre refuse de charger ne se montre pas non plus', async () => {
  const { els } = await charge({
    invoke: faux([LULU], { projet_ouvrir: PROJET, police_texte_donnee: DONNEE_POLICE }),
    open: async () => '/livres/LHC.ozalid',
    FontFace: class {
      constructor(famille) {
        this.family = famille;
      }

      async load() {
        throw new Error('OTS parsing error');
      }
    },
  });
  await els.get('btOuvrir').declenche('click');
  await new Promise((r) => setImmediate(r));

  assert.strictEqual(els.get('echantillonPolice').hidden, true);
  assert.strictEqual(els.get('echantillonAbsent').hidden, false);
});

/**
 * `afficherProjet` repasse à chaque frappe dans l'onglet Livre, et chaque lecture côté
 * Rust parcourt les dix mégaoctets des polices embarquées. Une famille déjà chargée ne
 * se redemande donc pas — mais en changer en redemande une, sans quoi l'échantillon
 * mentirait sur le champ.
 */
test('une écriture déjà chargée ne se redemande pas, une autre si', async () => {
  const demandes = [];
  const { els } = await charge({
    invoke: faux([LULU], {
      projet_ouvrir: PROJET,
      police_texte_donnee: (args) => {
        demandes.push(args.famille);
        return DONNEE_POLICE;
      },
      livre_modifier: PROJET,
      interieur_modifier: { ...PROJET, interieur: { police: 'Cardo' } },
    }),
    open: async () => '/livres/LHC.ozalid',
  });
  await els.get('btOuvrir').declenche('click');
  await new Promise((r) => setImmediate(r));

  // Une frappe dans un champ du livre : le projet est redessiné, la police n'a pas bougé.
  els.get('inTitre').value = 'Les Heures creuses !';
  await els.get('inTitre').declenche('input');
  await new Promise((r) => setImmediate(r));
  assert.deepStrictEqual(demandes, ['Alegreya']);

  els.get('inPoliceInterieur').value = 'Cardo';
  await els.get('inPoliceInterieur').declenche('change');
  await new Promise((r) => setImmediate(r));
  assert.deepStrictEqual(demandes, ['Alegreya', 'Cardo']);
});

/**
 * Le cœur du projet : le dos ne doit jamais apparaître comme un nombre quand le
 * prestataire n'en publie pas de formule. Un « 0,00 mm » affiché ici enverrait une
 * planche fausse à l'impression sans que rien ne l'ait signalé.
 */
test('un prestataire sans formule n\'affiche jamais de dos chiffré', async () => {
  const c = composition(COOLLIBRI, { pages: 190, dos: null });
  const { els } = await charge({
    invoke: faux([COOLLIBRI], {
      projet_ouvrir: { ...PROJET, livraison: livraison(COOLLIBRI) },
      composer: c,
    }),
    open: async () => '/livres/LHC.ozalid',
  });
  await els.get('btOuvrir').declenche('click');
  await faireComposer(els);

  const dos = els.get('piedDos').textContent;
  assert.match(dos, /relevé sur le gabarit/);
  assert.doesNotMatch(dos, /\d/, `dos chiffré affiché : « ${dos} »`);
  // Les pages, elles, sont mesurées : composé ne veut pas dire chiffré, mais le
  // manuscrit fait bien 190 pages chez ce prestataire-là.
  assert.match(els.get('piedMesure').textContent, /190 pages/);
});

/**
 * Une erreur de la chaîne doit rester lisible — et depuis qu'aucun bouton ne la
 * provoque, elle monte à l'entête, la seule bande que toutes les étapes partagent. Une
 * composition déclenchée depuis la Couverture n'a aucune raison d'échouer dans un coin
 * de l'étape Livre, que personne ne regardera.
 *
 * Ce que ce test **ne demande pas** : que la légende du pied s'efface. Elle ne vient
 * pas du geste qui échoue mais du projet, et le projet porte toujours la mesure de la
 * dernière composition réussie — qui n'a pas cessé d'être vraie parce qu'une autre a
 * échoué. Ce qui la périmerait, c'est le Rust qui l'efface à la source.
 */
test('une erreur de composition monte à l\'entête', async () => {
  let echoue = false;
  const base = faux([LULU], { projet_ouvrir: PROJET });
  const invoke = async (cmd, args) => {
    if (cmd === 'composer') {
      if (echoue) throw '64 chapitres attendus (projet), 61 trouvés.';
      return COMPOSITION;
    }
    return base(cmd, args);
  };
  const { els } = await charge({ invoke, open: async () => '/livres/LHC.ozalid' });
  await els.get('btOuvrir').declenche('click');
  await faireComposer(els);
  assert.match(els.get('piedMesure').textContent, /262 pages/);

  echoue = true;
  await faireComposer(els);
  assert.match(els.get('alerte').textContent, /64 chapitres attendus/);
  assert.strictEqual(els.get('alerte').className, 'etat erreur');
});

/** Une erreur d'ouverture doit s'afficher, pas disparaître dans la console. */
test('un fichier qui n\'est pas un projet est signalé à l\'écran', async () => {
  const { els } = await charge({
    invoke: faux([LULU], {
      projet_ouvrir: () => {
        throw 'archive sans projet.toml : ce n\'est pas un projet Ozalid.';
      },
    }),
    open: async () => '/x/photos.zip',
  });
  await els.get('btOuvrir').declenche('click');
  // L'entête est la seule bande que l'accueil et les étapes partagent : le message s'y
  // lit quel que soit l'écran d'où l'ouverture est partie.
  assert.match(els.get('alerte').textContent, /pas un projet Ozalid/);
  assert.strictEqual(els.get('alerte').className, 'etat erreur');
  assert.strictEqual(els.get('accueil').hidden, false, 'étapes ouvertes sur un échec');
});
