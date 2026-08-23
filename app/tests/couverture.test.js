'use strict';

const test = require('node:test');
const assert = require('node:assert');
const { charge } = require('./dom_shim');
const { groupes, lire, ecrire, placeImage } = require('../src/couverture.js');

const LULU = {
  cle: 'lulu', libelle: 'Lulu', largeur: 108, hauteur: 175, fond_perdu: 3.175, dos_publie: true,
  papiers: [{ cle: 'standard', libelle: 'Papier standard' }],
};

const style = (police, taille, couleur) => ({
  police, graisse: 400, italique: false, taille, couleur, tracking: 0, casse: 'telle',
});

const CADRAGE = { proportions: false, x: 0.5, y: 0.5, zoom: 1, etirement: 1 };

/** Un élément du dos, au format que sérialise le Rust. */
const elementDos = (place, rang, sur = {}) => ({
  actif: true, place, rang, sens: 0, style: style('Archivo', 2.6, '#191917'), ...sur,
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
      collection: elementDos('pied', 2, { actif: false }),
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
    envois: { main: { mode: 'police', police: 'Caveat' }, liste: [] },
    livraison: {
      destinataires: [{ provider: 'lulu', papier: 'standard', dos_mm: null, fond_perdu_mm: null }],
      courant: 'lulu',
    },
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
    if (cmd === 'jetons_liste') return ['%TITRE%', '%AUTEUR%', '%GENRE%', '%EDITEUR%', '%COLLECTION%', '%MONOGRAMME%'];
    if (cmd === 'mains_liste') return ['Caveat', 'Dancing Script'];
    if (cmd === 'maquettes_liste') {
      return [
        { cle: 'folio', libelle: 'Folio', fournie: true },
        { cle: 'blanche', libelle: 'Blanche', fournie: true },
        { cle: 'surimpression', libelle: 'Surimpression', fournie: true },
      ];
    }
    if (cmd === 'projet_ouvrir') return projet(couverture);
    // La planche est la seule face qui se compose avec du fond perdu : elle seule
    // rend une coupe. Les fractions sont celles d'une poche Lulu à 3,175 mm.
    if (cmd === 'couverture_apercu') {
      return {
        image: 'data:image/png;base64,QUJD',
        reperes: args.face === 'planche'
          ? { x: 0.0129, y: 0.0175, pli_quatre: 0.4724, pli_une: 0.5276 }
          : null,
        // Une poche Lulu de 108 × 175 à 3,175 mm de fond perdu, sur un dos de 16,6 :
        // 2 × 108 + 16,6 + 2 × 3,175 de large, 175 + 2 × 3,175 de haut.
        mesures: args.face === 'planche'
          ? { largeur: 238.95, hauteur: 181.35, dos: 16.6, fond_perdu: 3.175 }
          : null,
      };
    }
    // Les calques de la face manipulable, demandés après chaque aperçu. Une photo de
    // 1000 × 1000 dans le bandeau d'une poche Lulu : la zone commence sous la bande, à
    // 30 % de la hauteur, et prend toute la largeur.
    if (cmd === 'couverture_calques') {
      // Les mêmes conditions que le Rust : pas de photo à déplacer en composition
      // typographique, ni sur une 4ème qui ne porte pas sa propre image.
      const photo = args.face === 'une'
        ? couverture?.mode !== 'typo'
        : args.face === 'quatre' && couverture?.quatrieme.fond === 'image';
      return photo
        ? {
          habillage: 'data:image/png;base64,SEFC',
          photo: 'data:image/jpeg;base64,UEhP',
          naturel_l: 1000, naturel_h: 1000,
          zone: { x: 0, y: 0.3, l: 1, h: 0.7 },
          papier: '#ffffff',
        }
        : null;
    }
    // Les boîtes du dos, mesurées par Typst côté Rust. Celles de la maquette de ce
    // fichier : auteur et titre au pied, éditeur à la tête.
    if (cmd === 'couverture_dos_boites') {
      return [
        { cle: 'auteur', debut: 0.02, fin: 0.14 },
        { cle: 'titre', debut: 0.15, fin: 0.43 },
        { cle: 'editeur', debut: 0.92, fin: 0.98 },
      ];
    }
    // Viser un autre destinataire est un des gestes qui redemandent un aperçu : le
    // format de la page vient de lui. Le projet de ce fichier n'en déclare qu'un, et
    // c'est assez — ce qui est vérifié ici, c'est que l'aperçu reparte.
    if (cmd === 'destinataire_viser') return projet(couverture);
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
  const file = ['/livres/LHC.ozalid', ...dialogues];
  const ctx = await charge({ invoke, open: async () => file.shift() ?? null });
  await ctx.els.get('btOuvrir').declenche('click');
  return { ...ctx, appels };
}

/** Laisse passer le délai de grâce de l'aperçu. */
const attendreApercu = () => new Promise((r) => setTimeout(r, 300));

/**
 * Donne une taille à l'aperçu.
 *
 * Le faux DOM ne met rien en page : sans cette boîte, tout geste se refuse — comme il le
 * fait dans la fenêtre devant un aperçu qui n'est pas encore affiché. Deux pixels par
 * millimètre sur une poche Lulu de 108 × 175, ce qui rend les comptes lisibles.
 */
const poserBoite = (els) => {
  els.get('cadreApercu').rect = { left: 0, top: 0, width: 216, height: 350 };
};

/** Un événement de souris, réduit à ce que la manipulation directe en lit. */
const souris = (x, y) => ({
  button: 0, clientX: x, clientY: y, pointerId: 1,
  preventDefault() {}, stopPropagation() {},
});

/** Un geste complet sur une prise : presser, traîner, relâcher. */
async function glisser(els, prise, de, vers) {
  const el = els.get(prise);
  await el.declenche('pointerdown', souris(de[0], de[1]));
  await el.declenche('pointermove', souris(vers[0], vers[1]));
  await el.declenche('pointerup', souris(vers[0], vers[1]));
}

/** La dernière maquette envoyée au Rust. */
const derniereMaquette = (appels) =>
  [...appels].reverse().find(([c]) => c === 'couverture_modifier')?.[1].couverture;

/**
 * La face par son libellé, et non par son rang : l'application les retrouve par rang —
 * c'est ce que dit le commentaire de `FACES` — mais un test qui en fait autant se met à
 * viser sa voisine le jour où une face s'ajoute, comme l'a fait l'arrivée du Dos.
 */
const face = (els, libelle) =>
  [...els.get('faces').children].find((b) => b.textContent === libelle);

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
  // Une chaîne : un contrôle ne rend jamais le nombre qu'on lui a posé.
  assert.ok(valeurs.includes('6.4'), 'le corps de l\'auteur n\'est pas repris');
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

  await face(els, '4ème').declenche('click');
  assert.ok(titres().some((t) => t.startsWith('4ème')));
  assert.ok(!titres().includes('Cadre'));
});

/**
 * Les réglages du dos n'ont de sens que sur la face qui le montre. Les offrir sur la
 * 1ère donnerait à régler un élément absent de l'aperçu affiché.
 *
 * Les colonnes ne redisent plus « Dos » : la face est déjà nommée par l'onglet allumé,
 * et le préfixe mangeait la moitié de titres qui tiennent dans 13,5 rem.
 */
test('les quatre éléments du dos ne sont offerts que sur la face Dos', async () => {
  const { els } = await ouvre(maquette());
  const titres = () => [...els.get('reglages').children]
    .filter((g) => !g.hidden)
    .map((g) => g.children[0].textContent);

  assert.ok(!titres().includes('Collection'), 'dos offert sur la 1ère');
  assert.ok(!titres().includes('Fond et espacements'), 'dos offert sur la 1ère');

  await face(els, 'Dos').declenche('click');
  assert.deepStrictEqual(
    titres(), ['Fond et espacements', 'Auteur', 'Titre', 'Éditeur', 'Collection']);
});

/**
 * La place, le rang et le sens ne se règlent qu'à la souris, sur l'aperçu : trois
 * contrôles de plus par élément diraient dans le panneau ce que le dos montre déjà, et
 * « Ordre à cette position » ne se comprend qu'une fois le geste connu.
 *
 * Les contrôles existent toujours, hors du panneau : c'est eux que le geste écrit et
 * que la commande relit. Ce test tient les deux bouts — rien à l'écran, tout au geste.
 */
test('la place, le rang et le sens du dos ne sont plus offerts au panneau', async () => {
  const { els, contexte } = await ouvre(maquette());
  await face(els, 'Dos').declenche('click');

  const libelles = [...els.get('reglages').children]
    .filter((g) => !g.hidden)
    .flatMap((g) => [...g.children].slice(1).map((l) => l.children[0].textContent));
  for (const absent of ['Position', 'Ordre à cette position', 'Sens']) {
    assert.ok(!libelles.includes(absent), `« ${absent} » encore offert au panneau`);
  }
  assert.ok(libelles.includes('Afficher'), 'la colonne n\'offre plus rien du tout');
  assert.strictEqual(contexte.valeurSaisie('dos.titre.place'), 'pied');
  assert.strictEqual(contexte.valeurSaisie('dos.titre.rang'), 2);
  assert.strictEqual(contexte.valeurSaisie('dos.titre.sens'), 0);
});

/**
 * Le sens de lecture se retourne d'un clic sur le texte lui-même.
 *
 * Un dos se lit de bas en haut chez les uns, de haut en bas chez les autres, et c'est
 * une décision de maquette qui se prend en regardant le dos — pas dans une liste
 * déroulante. L'auteur et le titre n'ont que ces deux sens : couchés en travers, ils ne
 * tiendraient pas dans l'épaisseur.
 */
test('un clic retourne le texte du dos, et un second le remet', async () => {
  const { els, appels, contexte } = await ouvre(maquette(), {
    couverture_modifier: (a) => projet(a.couverture),
  });
  await face(els, 'Dos').declenche('click');
  await attendreApercu();

  await els.get('sensDosTitre').declenche('click');
  assert.strictEqual(contexte.valeurSaisie('dos.titre.sens'), 180);
  assert.strictEqual(derniereMaquette(appels).dos.titre.sens, 180, 'retournement non commis');
  assert.strictEqual(derniereMaquette(appels).dos.auteur.sens, 0, 'le voisin a tourné aussi');

  await els.get('sensDosTitre').declenche('click');
  assert.strictEqual(contexte.valeurSaisie('dos.titre.sens'), 0);
});

/**
 * L'éditeur et la collection, eux, peuvent se coucher en travers du dos — c'est le sens
 * d'une mention qui se lit le livre debout, et ces deux textes-là sont assez courts pour
 * tenir dans l'épaisseur. Leurs deux boutons tournent chacun d'un quart de tour, et le
 * tour complet ramène à zéro : sans le modulo, le sens sortirait des bornes du schéma et
 * s'y arrêterait, le texte restant coincé en travers.
 */
for (const [cle, b] of [['editeur', 'Editeur'], ['collection', 'Collection']]) {
  test(`${cle} : le quart de tour va dans les deux sens`, async () => {
    const { els, contexte } = await ouvre(maquette(), {
      couverture_modifier: (a) => projet(a.couverture),
    });
    await face(els, 'Dos').declenche('click');
    await attendreApercu();

    const sens = () => contexte.valeurSaisie(`dos.${cle}.sens`);
    const droite = () => els.get(`sensDos${b}Droite`).declenche('click');
    await droite();
    assert.strictEqual(sens(), 90);
    await droite();
    await droite();
    assert.strictEqual(sens(), 270);
    await droite();
    assert.strictEqual(sens(), 0, 'le tour complet ne revient pas à zéro');

    await els.get(`sensDos${b}Gauche`).declenche('click');
    assert.strictEqual(sens(), 270, 'la gauche ne tourne pas à l\'envers de la droite');
  });
}

/**
 * La planche ne se règle pas, elle se vérifie : c'est ce qui lui vaut la fenêtre
 * entière. Un seul groupe qui y resterait rouvrirait la colonne de 22 rem, et l'aperçu
 * qu'on est venu regarder perdrait le tiers de sa largeur pour un panneau presque vide.
 */
test('la planche n\'offre aucun réglage et rend sa colonne à l\'aperçu', async () => {
  const { els } = await ouvre(maquette());
  await face(els, 'Planche').declenche('click');

  const offerts = [...els.get('reglages').children].filter((g) => !g.hidden);
  assert.deepStrictEqual(offerts.map((g) => g.children[0].textContent), []);
  assert.strictEqual(els.get('reglages').hidden, true);
  assert.strictEqual(els.get('couv').getAttribute('data-panneau'), 'non');
});

/**
 * Le dos couché a sa disposition à lui, et la feuille de style ne peut pas la deviner :
 * seule la face montrée dit si l'aperçu est un bandeau ou une page.
 */
test('la face montrée est écrite sur la couverture pour la mise en page', async () => {
  const { els } = await ouvre(maquette());
  assert.strictEqual(els.get('couv').getAttribute('data-face'), 'une');

  await face(els, 'Dos').declenche('click');
  assert.strictEqual(els.get('couv').getAttribute('data-face'), 'dos');
  assert.strictEqual(els.get('couv').getAttribute('data-panneau'), 'oui');
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
 * Le format vient du destinataire visé : en changer change l'aperçu, même si aucun
 * réglage de maquette n'a bougé.
 */
test('viser un autre destinataire redemande un aperçu', async () => {
  const { els, appels } = await ouvre(maquette());
  await attendreApercu();
  const avant = appels.filter(([c]) => c === 'couverture_apercu').length;
  await els.get('inDestinataire').declenche('change');
  await attendreApercu();
  const apres = appels.filter(([c]) => c === 'couverture_apercu').length;
  assert.ok(apres > avant, 'aperçu non redemandé');
});

/**
 * L'invite ne s'écrit qu'à un seul endroit, et c'est celui où le manque se voit : dans
 * l'aperçu vide. Elle s'écrivait aussi en haut de l'étape, mot pour mot — deux fois la
 * même phrase, dont l'une occupait une ligne à demeure sur un écran qui en manque.
 */
test('sans maquette, l\'aperçu le dit au lieu de rester vide', async () => {
  const { els } = await ouvre(null);
  await attendreApercu();
  assert.match(els.get('etatApercu').textContent, /Choisir une maquette/);
  assert.strictEqual(els.get('apercu').hidden, true, 'cadre d\'image sans image');
  // Rien à régler, donc pas de panneau — et la colonne qu'il occupait rendue à la scène
  // qui porte l'invite.
  assert.strictEqual(els.get('reglages').hidden, true);
  assert.strictEqual(els.get('couv').getAttribute('data-panneau'), 'non');
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
      return { image: 'data:image/png;base64,QUJD', reperes: null };
    },
  });
  await attendreApercu();
  assert.ok(els.get('apercu').src);
  assert.strictEqual(els.get('apercu').hidden, false, 'aperçu réussi mais masqué');

  casse = true;
  await els.get('inDestinataire').declenche('change');
  await attendreApercu();
  assert.strictEqual(els.get('apercu').src, undefined, 'aperçu périmé laissé à l\'écran');
  assert.strictEqual(els.get('apercu').hidden, true, 'cadre d\'image sans image');
  assert.match(els.get('etatApercu').textContent, /largeur du dos/);
  assert.strictEqual(els.get('etatApercu').className, 'note alerte');
});

/**
 * La classe qui colore un message lui survit si personne ne la reprend. Après un aperçu
 * en échec, l'invitation à choisir une maquette s'écrirait en rouge — et une invitation
 * en rouge se lit comme un refus, alors qu'elle ne demande qu'un choix.
 */
test('l\'invitation à choisir une maquette n\'hérite pas du rouge de l\'échec', async () => {
  let couverture = maquette();
  const { els } = await ouvre(
    couverture,
    {
      couverture_apercu: () => {
        throw 'prolongement panoramique : la largeur du dos est inconnue';
      },
      projet_ouvrir: () => projet(couverture),
    },
    ['/livres/B.ozalid']
  );
  await attendreApercu();
  assert.strictEqual(els.get('etatApercu').className, 'note alerte');

  // Le même écran, mais un projet sans maquette : c'est l'invitation qui s'affiche.
  couverture = null;
  await els.get('btOuvrir').declenche('click');
  await attendreApercu();

  assert.match(els.get('etatApercu').textContent, /Choisir une maquette/);
  assert.strictEqual(els.get('etatApercu').className, 'note',
    'une invitation à choisir écrite en rouge se lit comme un refus');
});

/**
 * La face Planche est la vue de contrôle : c'est là, et là seulement, qu'une image à
 * fond perdu voulue et une pastille tombée sous la coupe cessent de se ressembler, et
 * qu'un dos se distingue des faces quand il en porte le papier.
 * Les quatre fractions viennent du Rust — les recalculer ici redirait la règle qui
 * choisit entre le fond perdu publié et le relevé, et referait le calcul de dos que la
 * pagination commande.
 */
test('la planche marque la coupe et les deux plis que le Rust donne', async () => {
  const { els } = await ouvre(maquette());
  await face(els, 'Planche').declenche('click');
  await attendreApercu();
  const cadre = els.get('cadreApercu');
  assert.strictEqual(cadre.style.getPropertyValue('--coupe-x'), '0.0129');
  assert.strictEqual(cadre.style.getPropertyValue('--coupe-y'), '0.0175');
  assert.strictEqual(cadre.style.getPropertyValue('--pli-quatre'), '0.4724');
  assert.strictEqual(cadre.style.getPropertyValue('--pli-une'), '0.5276');
  assert.strictEqual(els.get('reperes').hidden, false, 'repères non marqués sur la planche');
});

/**
 * Les quatre nombres écrits sous la planche sont ceux du fichier remis au prestataire :
 * c'est en les comparant à son gabarit qu'on vérifie qu'on lui envoie la bonne planche.
 * Ils viennent du Rust en millimètres et ne se recomposent pas ici — refaire l'addition
 * dans la fenêtre, c'est la voir dériver le jour où un prestataire compte autrement.
 * Le fond perdu porte trois décimales et le reste deux, comme la Livraison : un fond
 * perdu se relève au millième de millimètre sur les gabarits, un dos jamais.
 */
test('la planche écrit sous elle ce qu\'elle mesure', async () => {
  const { els } = await ouvre(maquette());
  await face(els, 'Planche').declenche('click');
  await attendreApercu();
  const p = els.get('mesuresApercu');
  assert.strictEqual(p.hidden, false, 'mesures absentes sous la planche');
  assert.strictEqual(
    p.textContent,
    'Planche 238,95 × 181,35 mm — dos 16,60 mm — fond perdu 3,175 mm',
  );
});

/**
 * Une face rognée n'a ni dos ni fond perdu à annoncer, et sa largeur est celle du
 * format — que le pied de la fenêtre donne déjà. Comme pour la coupe, le détour par la
 * planche est ce qui fait le test : `#mesuresApercu` naît masqué, et sans lui on
 * vérifierait qu'une ligne jamais écrite reste absente.
 */
test('une face sans fond perdu n\'écrit aucune mesure', async () => {
  const { els } = await ouvre(maquette());
  await face(els, 'Planche').declenche('click');
  await attendreApercu();
  assert.strictEqual(els.get('mesuresApercu').hidden, false);

  await face(els, '1ère').declenche('click');
  await attendreApercu();
  assert.strictEqual(
    els.get('mesuresApercu').hidden, true, 'mesures de la planche restées sous la 1ère');
});

/**
 * La 1ère se compose au format rogné : il n'y a pas de bande à couper, et un trait sur
 * le bord même de l'image se lirait comme une coupe à zéro millimètre du texte.
 *
 * Le détour par la planche n'est pas une précaution de style : `#reperes` naît masqué
 * dans le HTML, et sans lui ce test serait vrai d'avance — vrai même si tout le
 * mécanisme avait disparu. Ce qu'il vérifie, c'est qu'une face rognée *éteint* un
 * habillage allumé, pas qu'elle le laisse éteint.
 */
test('une face sans fond perdu ne montre aucune coupe', async () => {
  const { els } = await ouvre(maquette());
  await attendreApercu();
  assert.strictEqual(els.get('reperes').hidden, true, 'coupe marquée sur la 1ère');

  await face(els, 'Planche').declenche('click');
  await attendreApercu();
  assert.strictEqual(els.get('reperes').hidden, false, 'coupe non marquée sur la planche');

  await face(els, '1ère').declenche('click');
  await attendreApercu();
  assert.strictEqual(els.get('reperes').hidden, true, 'habillage resté d\'une face à l\'autre');
});

/**
 * Un aperçu qui échoue retire l'image ; l'habillage doit partir avec elle. Seul sur la
 * scène, il marquerait la coupe d'une couverture qui n'est plus affichée.
 */
test('un aperçu qui échoue emporte l\'habillage avec l\'image', async () => {
  let casse = false;
  const { els } = await ouvre(maquette(), {
    couverture_apercu: (args) => {
      if (casse) throw 'prolongement panoramique : la largeur du dos est inconnue';
      return {
        image: 'data:image/png;base64,QUJD',
        reperes: args.face === 'planche'
          ? { x: 0.0129, y: 0.0175, pli_quatre: 0.4724, pli_une: 0.5276 }
          : null,
      };
    },
  });
  await face(els, 'Planche').declenche('click');
  await attendreApercu();
  assert.strictEqual(els.get('reperes').hidden, false);

  casse = true;
  await els.get('inDestinataire').declenche('change');
  await attendreApercu();
  assert.strictEqual(els.get('reperes').hidden, true, 'habillage laissé seul à l\'écran');
});

/**
 * Le rapport d'aspect est ce qui donne au cadre sa taille : sans lui, il se
 * dimensionnerait sur une image elle-même bornée en pourcentage de ce cadre, et le
 * navigateur tranche ce cycle à zéro — mesuré, cadre et image à 0 × 0 dans une scène de
 * 620 × 345. Le retirer ne casse aucun autre test : l'aperçu disparaîtrait sans un mot.
 */
test('le cadre prend le rapport d\'aspect de l\'image décodée', async () => {
  const { els } = await ouvre(maquette());
  await attendreApercu();
  const img = els.get('apercu');
  // Une planche Lulu : 235,35 mm de large pour 181,35 de haut, à 150 ppi.
  img.naturalWidth = 1390;
  img.naturalHeight = 1071;
  await img.declenche('load');
  assert.strictEqual(
    els.get('cadreApercu').style.getPropertyValue('--ratio'), String(1390 / 1071)
  );
});

/**
 * Un cadre qui garderait son rapport d'aspect sans image garderait sa place, vide, et
 * pousserait plus bas le message qui dit justement qu'il n'y a rien à voir.
 */
test('l\'aperçu retiré emporte le rapport d\'aspect du cadre', async () => {
  const { els } = await ouvre(maquette(), {
    couverture_apercu: () => {
      throw 'prolongement panoramique : la largeur du dos est inconnue';
    },
  });
  const cadre = els.get('cadreApercu');
  cadre.style.setProperty('--ratio', '1.29');
  await attendreApercu();
  assert.strictEqual(cadre.style.getPropertyValue('--ratio'), '');
});

/**
 * Éteindre la lunette montre la couverture telle qu'elle sera en main. Sans nouvelle
 * composition : c'est tout l'intérêt d'habiller l'image plutôt que de la refaire —
 * Typst met une seconde là où le CSS ne met rien.
 */
test('éteindre le fond perdu retire l\'habillage sans recomposer', async () => {
  const { els, appels } = await ouvre(maquette());
  await face(els, 'Planche').declenche('click');
  await attendreApercu();
  const avant = appels.filter(([c]) => c === 'couverture_apercu').length;

  await els.get('btReperes').declenche('click');
  assert.strictEqual(els.get('reperes').hidden, true, 'habillage resté allumé');
  assert.strictEqual(els.get('btReperes').getAttribute('aria-pressed'), 'false');
  assert.strictEqual(
    appels.filter(([c]) => c === 'couverture_apercu').length, avant,
    'la bascule a relancé une composition'
  );

  await els.get('btReperes').declenche('click');
  assert.strictEqual(els.get('reperes').hidden, false, 'habillage non rallumé');
});

/**
 * Un bouton qui ne peut rien faire est un piège : les trois autres faces n'ont pas de
 * fond perdu à montrer. Même raison que les réglages sans objet, masqués plutôt que
 * grisés.
 */
test('la bascule ne s\'offre que sur la planche', async () => {
  const { els } = await ouvre(maquette());
  await attendreApercu();
  assert.strictEqual(els.get('btReperes').hidden, true, 'bascule offerte sur la 1ère');

  await face(els, 'Planche').declenche('click');
  await attendreApercu();
  assert.strictEqual(els.get('btReperes').hidden, false, 'bascule absente de la planche');
});

/* ---------- manipulation directe ---------- */

/**
 * Le portage de `image::place` en JavaScript dit la même chose que le Rust.
 *
 * C'est la seule règle de composition qui existe en deux langues dans l'application, et
 * cette table est ce qui les tient d'accord : les cinq cas sont exactement ceux des
 * tests de `image.rs`, valeurs comprises. Les recalculer autrement ici ne prouverait
 * rien — ce qu'on vérifie, c'est que les deux versions se rejoignent, pas que celle-ci
 * est cohérente avec elle-même.
 *
 * Si l'une des deux bouge sans l'autre, la photo suivra la souris ailleurs que là où
 * Typst la composera, et le geste mentira sans jamais lever d'erreur.
 */
test('le portage de place dit la même chose que le Rust', () => {
  const zone = { largeur: 100, hauteur: 50 };
  const nat = { largeur: 100, hauteur: 100 };
  const base = { proportions: false, x: 0.5, y: 0.5, zoom: 1, etirement: 1 };

  const debordante = placeImage(zone, nat, base);
  assert.deepStrictEqual(debordante, { gauche: 0, haut: -25, largeur: 100, hauteur: 100 });

  const gardees = placeImage(zone, nat, { ...base, proportions: true });
  assert.deepStrictEqual(gardees, { gauche: 25, haut: 0, largeur: 50, hauteur: 50 });

  for (const [y, attendu] of [[0, 0], [0.5, -25], [1, -50]]) {
    assert.strictEqual(placeImage(zone, nat, { ...base, y }).haut, attendu, `ancrage ${y}`);
  }

  const zoome = placeImage(zone, nat, { ...base, x: 0, y: 0, zoom: 2 });
  assert.deepStrictEqual(zoome, { gauche: 0, haut: 0, largeur: 200, hauteur: 200 });

  const etire = placeImage(zone, nat, { ...base, etirement: 1.5 });
  assert.strictEqual(etire.largeur, 150);
  assert.strictEqual(etire.hauteur, 100, 'la hauteur ne suit pas l\'étirement');
  const neutralise = placeImage(zone, nat, { ...base, proportions: true, etirement: 1.5 });
  assert.strictEqual(neutralise.largeur, 50, 'étirement non neutralisé par les proportions');
});

/**
 * La photo suit la souris au 1:1 — et la référence est son mou réel, pas la largeur de
 * la couverture.
 *
 * Une photo de 1000 × 1000 dans une zone de 108 × 122,5 mm se compose en 122,5 × 122,5 :
 * elle déborde de 14,5 mm en largeur et de rien du tout en hauteur. Les 10 px du geste
 * valent 5 mm sur cet aperçu-là, soit un tiers du mou : l'ancrage passe de 0,5 à 0,16.
 *
 * Deux choses se vérifient ici et nulle part ailleurs. Le **sens** : tirer vers la
 * droite découvre la gauche de la photo, donc l'ancrage décroît — l'inverse ferait
 * fuir l'image sous le curseur. Et le **refus** de l'axe sans mou : la hauteur ne bouge
 * pas d'un cheveu, parce qu'il n'y a rien à y découvrir. Un geste calé sur la largeur de
 * la face aurait donné 0,45 au lieu de 0,16, et le réglage aurait paru mou.
 */
test('tirer la photo déplace l\'ancrage de son mou réel, dans le sens du geste', async () => {
  const cv = maquette();
  const { els, appels, contexte } = await ouvre(cv, { couverture_modifier: (a) => projet(a.couverture) });
  await attendreApercu();
  poserBoite(els);

  await glisser(els, 'priseImage', [100, 200], [110, 200]);
  assert.strictEqual(contexte.valeurSaisie('cadrage.x'), 0.16);
  assert.strictEqual(contexte.valeurSaisie('cadrage.y'), 0.5, 'axe sans mou déplacé');
  assert.strictEqual(derniereMaquette(appels).cadrage.x, 0.16, 'valeur non commise au Rust');
});

/**
 * La hauteur du bloc de la 4ème se compte en pourcentage de la **largeur** de la
 * couverture, celle de la 1ère en pourcentage de sa **hauteur** — c'est le schéma qui le
 * dit, chacun dans son unité.
 *
 * Le même geste vertical n'y vaut donc pas le même nombre : 17,5 px sur un aperçu haut de
 * 350 font un vingtième de la hauteur, soit 8,75 mm, soit 8,1 % de la largeur — et non 5.
 * Un geste qui ignorerait l'unité poserait 17 au lieu de 20,5, et le texte de la 4ème
 * partirait d'un tiers de trop à chaque tirée.
 */
test('la hauteur du bloc de la 4ème se compte sur la largeur', async () => {
  const cv = maquette();
  cv.quatrieme.texte = 'Un texte de présentation.';
  const { els, contexte } = await ouvre(cv, { couverture_modifier: (a) => projet(a.couverture) });
  await face(els, '4ème').declenche('click');
  await attendreApercu();
  poserBoite(els);

  await glisser(els, 'priseBloc', [100, 100], [100, 117.5]);
  assert.strictEqual(contexte.valeurSaisie('quatrieme.top'), 20);
});

/**
 * Le panneau ne réécrit pas le champ que la souris tient.
 *
 * Pendant un geste lent, une composition part à chaque pause et revient avec la maquette
 * telle qu'elle était **au départ de cette composition** — donc en retard sur la souris,
 * qui a continué. Sans garde, la couverture reculerait d'un cran à chaque rattrapage, et
 * le geste deviendrait impossible à finir.
 */
test('le panneau ne réécrit pas le réglage que la souris tient', async () => {
  // Une commande qu'on retient : c'est ce qui fait le test. Elle part avec le bandeau à
  // 40, la souris continue jusqu'à 50, et sa réponse — périmée de dix points — n'arrive
  // qu'ensuite. Une commande instantanée ne prouverait rien : elle répondrait toujours
  // ce que le champ porte encore.
  let repondre;
  const retenue = new Promise((r) => { repondre = r; });
  const cv = maquette();
  const { els, contexte } = await ouvre(cv, {
    couverture_modifier: async (a) => { await retenue; return projet(a.couverture); },
  });
  await attendreApercu();
  poserBoite(els);

  const prise = els.get('priseBandeau');
  await prise.declenche('pointerdown', souris(100, 100));
  await prise.declenche('pointermove', souris(100, 135));
  assert.strictEqual(contexte.valeurSaisie('bandeau'), 40, '10 % de la hauteur non ajoutés');

  // Le rattrapage part…
  await new Promise((r) => setTimeout(r, 200));
  // …la souris continue…
  await prise.declenche('pointermove', souris(100, 170));
  assert.strictEqual(contexte.valeurSaisie('bandeau'), 50);
  // …et la réponse périmée arrive.
  repondre();
  await new Promise((r) => setTimeout(r, 50));
  assert.strictEqual(contexte.valeurSaisie('bandeau'), 50, 'champ repris par le panneau');
  await prise.declenche('pointerup', souris(100, 170));
});

/**
 * Poser la souris sur sa propre couverture ne modifie pas le document.
 *
 * Un clic sans déplacement ne commet rien : sinon regarder une couverture suffirait à
 * réveiller la garde des modifications à la fermeture, et à faire proposer d'enregistrer
 * un travail que personne n'a touché.
 */
test('un clic qui ne déplace rien ne modifie pas le projet', async () => {
  const cv = maquette();
  const { els, appels } = await ouvre(cv, { couverture_modifier: (a) => projet(a.couverture) });
  await attendreApercu();
  poserBoite(els);

  const prise = els.get('priseImage');
  await prise.declenche('pointerdown', souris(100, 200));
  await prise.declenche('pointerup', souris(100, 200));
  assert.strictEqual(
    appels.some(([c]) => c === 'couverture_modifier'), false, 'clic commis au Rust');
});

/**
 * Une prise ne s'offre que là où il y a quelque chose à saisir.
 *
 * La frontière du bandeau n'existe qu'en mode Bandeau ; la planche ne règle rien, donc
 * n'offre aucune prise. Le détour par la 1ère est ce qui fait le test : les prises
 * naissent masquées dans le HTML, et sans lui on vérifierait qu'une boîte jamais montrée
 * reste absente.
 */
test('les prises ne s\'offrent que là où il y a quelque chose à saisir', async () => {
  const { els } = await ouvre(maquette());
  await attendreApercu();
  assert.strictEqual(els.get('prises').hidden, false, 'aucune prise sur la 1ère');
  assert.strictEqual(els.get('priseBandeau').hidden, false, 'frontière du bandeau absente');
  assert.strictEqual(els.get('priseImage').hidden, false, 'photo non saisissable');

  await face(els, 'Planche').declenche('click');
  await attendreApercu();
  assert.strictEqual(els.get('prises').hidden, true, 'prises laissées sur la planche');

  const { els: sansImage } = await ouvre(maquette('typo'));
  await attendreApercu();
  assert.strictEqual(
    sansImage.get('priseBandeau').hidden, true, 'frontière de bandeau hors mode Bandeau');
  assert.strictEqual(sansImage.get('priseImage').hidden, true, 'photo saisissable sans photo');
});

/**
 * Les prises du dos tombent sur les boîtes que le Rust donne, et pas ailleurs.
 *
 * Ces boîtes-là ne se devinent pas : la longueur d'un texte dépend de chaque glyphe, et
 * seul Typst tient les métriques des polices embarquées. La fenêtre n'a donc rien à
 * calculer ici — ce qui se vérifie, c'est qu'elle pose sans rien réinterpréter.
 */
test('les prises du dos se posent sur les boîtes mesurées', async () => {
  const { els } = await ouvre(maquette());
  await face(els, 'Dos').declenche('click');
  await attendreApercu();

  const prise = els.get('priseDosTitre');
  assert.strictEqual(prise.hidden, false, 'le titre du dos ne se saisit pas');
  assert.strictEqual(prise.style.getPropertyValue('--gauche'), '0.15');
  assert.strictEqual(prise.style.getPropertyValue('--largeur'), '0.28');
  // L'éditeur est éteint dans la maquette de ce fichier ? Non : il est à la tête.
  assert.strictEqual(els.get('priseDosEditeur').style.getPropertyValue('--gauche'), '0.92');
  // Les prises des autres faces n'ont rien à faire en travers du dos.
  assert.strictEqual(els.get('priseBandeau').hidden, true, 'bandeau posé sur le dos');
  assert.strictEqual(els.get('priseImage').hidden, true, 'cadre d\'image posé sur le dos');
});

/**
 * Déposer un texte du dos dans un autre tiers le range, et renumérote la place entière.
 *
 * L'éditeur part de la tête et tombe à 20 % du dos, donc au pied, entre l'auteur — dont
 * le texte s'arrête avant — et le titre, qui commence après. Les trois rangs du pied
 * sont alors réécrits d'un bout à l'autre : laisser un trou ferait dépendre l'ordre de
 * nombres qui ne veulent plus rien dire, et deux éléments finiraient par partager un
 * rang, auquel cas c'est le tri du Rust qui trancherait sans que personne l'ait décidé.
 *
 * Rien n'est commis en chemin : la place et le rang n'ont de valeur qu'une fois le doigt
 * levé, et une composition par tiers traversé ferait clignoter le dos sous la souris.
 */
test('déposer un texte du dos le range et renumérote sa place', async () => {
  const cv = maquette();
  const { els, appels, contexte } = await ouvre(cv, {
    couverture_modifier: (a) => projet(a.couverture),
  });
  await face(els, 'Dos').declenche('click');
  await attendreApercu();
  poserBoite(els);

  const prise = els.get('priseDosEditeur');
  await prise.declenche('pointerdown', souris(200, 40));
  await prise.declenche('pointermove', souris(43.2, 40));

  // La prise suit la souris pendant qu'on la traîne : sans ce déplacement, on lâcherait
  // à l'aveugle un rectangle resté à sa place de départ.
  assert.ok(
    Math.abs(Number(prise.style.getPropertyValue('--gauche')) - (0.92 - 156.8 / 216)) < 1e-9,
    'la prise saisie ne suit pas la souris');

  // Au-delà du délai de grâce : rien ne part tant que le doigt n'est pas levé.
  await new Promise((r) => setTimeout(r, 250));
  assert.strictEqual(
    appels.some(([c]) => c === 'couverture_modifier'), false, 'commis avant le dépôt');

  await prise.declenche('pointerup', souris(43.2, 40));
  assert.strictEqual(contexte.valeurSaisie('dos.editeur.place'), 'pied');
  assert.strictEqual(contexte.valeurSaisie('dos.auteur.rang'), 1);
  assert.strictEqual(contexte.valeurSaisie('dos.editeur.rang'), 2, 'inséré au mauvais rang');
  assert.strictEqual(contexte.valeurSaisie('dos.titre.rang'), 3, 'le voisin n\'a pas reculé');
  assert.strictEqual(derniereMaquette(appels).dos.editeur.place, 'pied', 'dépôt non commis');
});

/**
 * Reposer un texte du dos là où il était ne modifie pas le document, comme un clic sur
 * la couverture. Le dépôt écrit bien les six réglages, mais il les écrit à l'identique.
 */
test('reposer un texte du dos au même endroit ne modifie pas le projet', async () => {
  const cv = maquette();
  const { els, appels } = await ouvre(cv, { couverture_modifier: (a) => projet(a.couverture) });
  await face(els, 'Dos').declenche('click');
  await attendreApercu();
  poserBoite(els);

  // L'éditeur est à la tête, et 200 px sur 216 tombent dans le dernier tiers.
  const prise = els.get('priseDosEditeur');
  await prise.declenche('pointerdown', souris(200, 40));
  await prise.declenche('pointermove', souris(198, 40));
  await prise.declenche('pointerup', souris(198, 40));
  assert.strictEqual(
    appels.some(([c]) => c === 'couverture_modifier'), false, 'dépôt sans effet commis');
});

/* ---------- le menu des maquettes et le dialogue ---------- */

/** Les trois fournies, plus une personnalisée : de quoi exercer le séparateur. */
const AVEC_PERSONNALISEE = () => [
  { cle: 'folio', libelle: 'Folio', fournie: true },
  { cle: 'blanche', libelle: 'Blanche', fournie: true },
  { cle: 'ma-collection', libelle: 'Ma collection', fournie: false },
];

/**
 * Le menu est un geste, pas un état : les personnalisées s'y rangent après les
 * fournies, derrière un séparateur qu'on ne peut pas choisir. Une option désactivée
 * plutôt qu'un `optgroup` — le faux DOM sélectionne la première option *enfant* d'un
 * select, et des options rangées dans un groupe ne le seraient plus.
 */
test('le menu des maquettes range les personnalisées sous un séparateur', async () => {
  const { els } = await ouvre(maquette(), { maquettes_liste: AVEC_PERSONNALISEE });
  const options = [...els.get('inMaquette').children].map((o) => ({
    texte: o.textContent, valeur: o.value, inerte: !!o.disabled,
  }));
  assert.deepEqual(options, [
    { texte: 'Repartir d\'une maquette…', valeur: '', inerte: false },
    { texte: 'Folio', valeur: 'folio', inerte: false },
    { texte: 'Blanche', valeur: 'blanche', inerte: false },
    { texte: '──────────', valeur: '', inerte: true },
    { texte: 'Ma collection', valeur: 'ma-collection', inerte: false },
  ]);
});

/**
 * Le geste du lot : le nom saisi part au Rust, et la liste se refait derrière — sans
 * quoi la maquette qu'on vient d'enregistrer manquerait au menu jusqu'au prochain
 * démarrage.
 */
test('enregistrer une maquette la fait paraître au menu', async () => {
  const enregistrees = [];
  const { els } = await ouvre(maquette(), {
    maquette_enregistrer: ({ nom }) => { enregistrees.push(nom); return null; },
    maquettes_liste: () => [
      { cle: 'folio', libelle: 'Folio', fournie: true },
      ...enregistrees.map((n) => ({ cle: 'x', libelle: n, fournie: false })),
    ],
  });

  await els.get('btMaquettes').declenche('click');
  els.get('inMaquetteNom').value = 'Ma collection';
  await els.get('btMaquetteEnregistrer').declenche('click');

  assert.deepEqual(enregistrees, ['Ma collection']);
  assert.ok(
    [...els.get('inMaquette').children].some((o) => o.textContent === 'Ma collection'),
    'la maquette enregistrée doit paraître au menu'
  );
  assert.strictEqual(els.get('inMaquetteNom').value, '', 'le champ se vide après le geste');
  assert.ok(els.get('dlgMaquettes').open, 'le dialogue reste ouvert : on en enregistre souvent deux');
});

/**
 * Un refus du Rust — nom déjà pris, nom sans slug — doit se lire *dans* le dialogue :
 * l'alerte de la fenêtre est derrière lui, et le geste paraîtrait avoir marché.
 */
test('un refus d\'enregistrement se lit dans le dialogue', async () => {
  const { els } = await ouvre(maquette(), {
    maquette_enregistrer: () => { throw new Error('« Folio » porte déjà ce nom.'); },
    maquettes_liste: AVEC_PERSONNALISEE,
  });
  await els.get('btMaquettes').declenche('click');
  els.get('inMaquetteNom').value = 'Folio';
  await els.get('btMaquetteEnregistrer').declenche('click');
  assert.match(els.get('etatMaquettes').textContent, /porte déjà ce nom/);
});
