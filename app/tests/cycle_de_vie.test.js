'use strict';

// Câblage du cycle de vie : ce que l'interface envoie au Rust selon la réponse de la
// garde, et ce que le menu déclenche. La boîte de dialogue elle-même est native :
// elle se vérifie dans l'application, pas ici.

const test = require('node:test');
const assert = require('node:assert');
const { charge } = require('./dom_shim');

const LULU = {
  cle: 'lulu', libelle: 'Lulu — poche 108 × 175',
  largeur: 108, hauteur: 175, fond_perdu: 3.175, dos_publie: true,
  papiers: [{ cle: 'standard', libelle: 'Papier standard' }],
};

function projet(sur = {}) {
  return {
    chemin: '/livres/LHC.ozalid',
    livre: {
      titre: 'Les Heures creuses', titre_page: null, auteur: 'Ivan Pjig',
      genre: 'roman', copyright: '', chapitres: null,
    },
    manuscrit_source: null,
    chapitres_trouves: 1,
    mots: 12,
    manuscrit_absent: false,
    modifie: false,
    couverture: null,
    couverture_importee: false,
    images: [],
    interieur: { police: 'EB Garamond' },
    ...sur,
  };
}

/** Un atelier de test : enregistre les commandes reçues, rend des vues plausibles. */
function atelier({ garde = 'ignorer', recents = [], sur = {} } = {}) {
  const appels = [];
  const invoke = async (cmd, args) => {
    appels.push([cmd, args]);
    switch (cmd) {
      case 'providers_liste': return [LULU];
      case 'polices_liste': return ['Bodoni Moda'];
      case 'polices_texte_liste': return ['EB Garamond'];
      case 'maquettes_liste': return [];
      case 'recents_liste': return recents;
      case 'garde_modifications': return garde;
      case 'projet_fermer': return null;
      case 'interface_prete': return null;
      case 'couverture_apercu': throw new Error('pas de maquette');
      default: return projet(sur);
    }
  };
  return { appels, invoke, noms: () => appels.map(([c]) => c) };
}

test('sans projet, aucune rubrique n\'est offerte et les récents s\'affichent', async () => {
  const a = atelier({ recents: ['/livres/A.ozalid', '/livres/B.ozalid'] });
  const { els } = await charge({ invoke: a.invoke });

  assert.equal(els.get('secLivre').hidden, true);
  assert.equal(els.get('btEnregistrer').disabled, true);
  assert.equal(els.get('cheminProjet').textContent, 'aucun projet ouvert');
  assert.deepEqual(els.get('recents').textes('BUTTON'),
    ['/livres/A.ozalid', '/livres/B.ozalid']);
});

test('cliquer un récent ouvre ce projet-là', async () => {
  const a = atelier({ recents: ['/livres/A.ozalid'] });
  const { els } = await charge({ invoke: a.invoke });

  await els.get('recents').enfants.find((e) => e.tagName === 'BUTTON').declenche('click');

  const ouvre = a.appels.find(([c]) => c === 'projet_ouvrir');
  assert.deepEqual(ouvre[1], { chemin: '/livres/A.ozalid' });
  assert.equal(els.get('secLivre').hidden, false);
});

test('la garde refusée arrête tout : rien n\'est ouvert, rien n\'est perdu', async () => {
  const a = atelier({ garde: 'annuler' });
  const { els } = await charge({ invoke: a.invoke });

  await els.get('btNouveau').declenche('click');

  assert.ok(!a.noms().includes('projet_nouveau'),
    'un « Annuler » qui crée quand même le projet aurait perdu le travail');
  assert.equal(els.get('secLivre').hidden, true);
});

test('la garde acceptée laisse passer', async () => {
  const a = atelier({ garde: 'ignorer' });
  const { els } = await charge({ invoke: a.invoke });

  await els.get('btNouveau').declenche('click');

  assert.ok(a.noms().includes('projet_nouveau'));
  assert.equal(els.get('secLivre').hidden, false);
});

test('« Enregistrer » réécrit en place, sans sélecteur de fichiers', async () => {
  const a = atelier();
  let demande = 0;
  const { els } = await charge({
    invoke: a.invoke,
    save: async () => { demande += 1; return '/ailleurs.ozalid'; },
  });
  await els.get('btNouveau').declenche('click');   // ouvre un projet qui a un chemin
  await els.get('btEnregistrer').declenche('click');

  assert.ok(a.noms().includes('projet_enregistrer'));
  assert.equal(demande, 0, 'un projet déjà posé ne redemande pas où');
});

test('un projet jamais enregistré n\'offre que « Enregistrer sous… »', async () => {
  const a = atelier({ sur: { chemin: null, modifie: false } });
  const { els } = await charge({ invoke: a.invoke });
  await els.get('btNouveau').declenche('click');

  assert.equal(els.get('btEnregistrer').disabled, true);
  assert.equal(els.get('btEnregistrerSous').disabled, false);
  assert.equal(els.get('etatEnregistrement').textContent, 'jamais enregistré');
});

test('l\'état d\'enregistrement suit le drapeau du Rust', async () => {
  const a = atelier({ sur: { modifie: true } });
  const { els } = await charge({ invoke: a.invoke });
  await els.get('btNouveau').declenche('click');

  assert.equal(els.get('etatEnregistrement').textContent, 'modifié');
});

test('un manuscrit absent se dit absent, et non vide de chapitres', async () => {
  const a = atelier({ sur: { manuscrit_absent: true, chapitres_trouves: 0, mots: 0 } });
  const { els } = await charge({ invoke: a.invoke });
  await els.get('btNouveau').declenche('click');

  assert.match(els.get('etatManuscrit').textContent, /Aucun manuscrit/);
  assert.doesNotMatch(els.get('etatManuscrit').textContent, /0 chapitres/);
});

test('le menu passe par le même code que les boutons', async () => {
  const a = atelier();
  const { els, menu } = await charge({ invoke: a.invoke });

  await menu('fichier.nouveau');
  assert.ok(a.noms().includes('projet_nouveau'));
  assert.equal(els.get('secLivre').hidden, false);

  await menu('fichier.fermer');
  assert.ok(a.noms().includes('projet_fermer'));
  assert.equal(els.get('secLivre').hidden, true);
});

/**
 * Les boutons sont grisés sans projet ; le menu, lui, offre toujours ses entrées.
 * Une exception y remonterait dans le rappel de `listen`, que personne n'attrape :
 * le geste ne ferait rien, sans un mot.
 */
test('enregistrer depuis le menu sans projet ne lève rien', async () => {
  const a = atelier();
  let demande = 0;
  const { menu } = await charge({
    invoke: a.invoke,
    save: async () => { demande += 1; return null; },
  });

  await menu('fichier.enregistrer');
  await menu('fichier.enregistrer_sous');

  assert.equal(demande, 0, 'aucun sélecteur de fichiers ne doit s\'ouvrir');
  assert.ok(!a.noms().includes('projet_enregistrer'));
  assert.ok(!a.noms().includes('projet_enregistrer_sous'));
});

/**
 * Le Rust ne rend que trois réponses connues. Si un jour il en rendait une autre,
 * poursuivre perdrait le travail : le défaut doit pencher du côté qui ne perd rien.
 */
test('une réponse de garde inattendue arrête au lieu de poursuivre', async () => {
  const a = atelier({ garde: 'un mot que personne n\'attend' });
  const { els } = await charge({ invoke: a.invoke });

  await els.get('btNouveau').declenche('click');

  assert.ok(!a.noms().includes('projet_nouveau'),
    'une réponse incomprise doit annuler, jamais laisser passer');
  assert.equal(els.get('secLivre').hidden, true);
});

test('un récent du menu porte son chemin dans son identifiant', async () => {
  const a = atelier();
  const { menu } = await charge({ invoke: a.invoke });

  await menu('fichier.recent:/livres/Z.ozalid');

  const ouvre = a.appels.find(([c]) => c === 'projet_ouvrir');
  assert.deepEqual(ouvre[1], { chemin: '/livres/Z.ozalid' });
});

/**
 * Un chemin peut contenir un deux-points, et le préfixe des récents en contient un.
 * Découper sur « : » au lieu de retirer le préfixe casserait sur ces chemins-là,
 * rarement et en silence — c'est le pire mode de panne, et ce test l'interdit.
 */
test('un chemin qui contient un deux-points survit au préfixe', async () => {
  const a = atelier();
  const { menu } = await charge({ invoke: a.invoke });

  await menu('fichier.recent:/Users/x/Mon:livre.ozalid');

  const ouvre = a.appels.find(([c]) => c === 'projet_ouvrir');
  assert.deepEqual(ouvre[1], { chemin: '/Users/x/Mon:livre.ozalid' });
});

test('la fenêtre ne se ferme que si la garde le permet', async () => {
  const refuse = atelier({ garde: 'annuler' });
  let fermetures = 0;
  const { fermeture } = await charge({
    invoke: refuse.invoke,
    destroy: () => { fermetures += 1; },
  });

  await fermeture();
  assert.equal(fermetures, 0, 'un « Annuler » qui ferme quand même perdrait tout');

  const accepte = atelier({ garde: 'ignorer' });
  let fermee = 0;
  const { fermeture: fermeture2 } = await charge({
    invoke: accepte.invoke,
    destroy: () => { fermee += 1; },
  });

  await fermeture2();
  assert.equal(fermee, 1);
});

/** ⌘Q ne peut pas être une porte de sortie qui traverse la garde sans la voir. */
test('« Quitter » demande comme le reste, et ferme par destroy', async () => {
  const a = atelier({ garde: 'ignorer' });
  let fermee = 0;
  const { menu } = await charge({
    invoke: a.invoke,
    destroy: () => { fermee += 1; },
  });

  await menu('fichier.quitter');

  assert.ok(a.noms().includes('garde_modifications'));
  assert.equal(fermee, 1);
});

/**
 * Le Rust ne retient la fermeture que s'il sait que quelqu'un écoute. Annoncer qu'on
 * écoute avant de l'avoir fait rouvrirait la fenêtre de temps que ce témoin ferme.
 */
test('l\'interface ne s\'annonce qu\'une fois ses écouteurs posés', async () => {
  const a = atelier();
  const poses = [];
  let temoin = null;
  await charge({
    invoke: async (cmd, args) => {
      // Photographier les écouteurs déjà posés à l'instant de l'annonce : c'est le
      // seul moment où l'ordre est observable.
      if (cmd === 'interface_prete') temoin = poses.slice();
      return a.invoke(cmd, args);
    },
    listen: async (nom) => {
      // Un tour de boucle avant de résoudre : une annonce prématurée passerait devant.
      await new Promise((r) => setImmediate(r));
      poses.push(nom);
      return () => {};
    },
  });
  await new Promise((r) => setImmediate(r));

  assert.deepEqual(poses.sort(), ['fermeture-demandee', 'menu']);
  assert.ok(temoin !== null, 'l\'interface doit s\'annoncer');
  assert.deepEqual(temoin.sort(), ['fermeture-demandee', 'menu'],
    'annoncée avant que les deux écouteurs ne soient posés');
});

/**
 * Les chiffres d'une composition ne valent que pour le livre qui les a produits.
 * Les laisser à l'écran pendant qu'on en ouvre un autre donnerait à lire la
 * pagination du mauvais livre — l'erreur même que l'application existe pour éviter.
 */
test('ouvrir un autre projet oublie les sorties du précédent', async () => {
  const a = atelier();
  const { els } = await charge({
    invoke: a.invoke,
    open: async () => '/livres/B.ozalid',
  });
  await els.get('btNouveau').declenche('click');

  // Ce qu'une composition aurait laissé à l'écran.
  els.get('resultat').textContent = '262 pages, dos 16,5 mm';
  els.get('resultat').hidden = false;
  els.get('cheminEpreuve').textContent = '/livres/A/epreuve.pdf';

  await els.get('btOuvrir').declenche('click');

  assert.equal(els.get('resultat').hidden, true);
  assert.equal(els.get('resultat').textContent, '');
  assert.equal(els.get('cheminEpreuve').textContent, '');
});

/**
 * Un projet qu'on n'a pas pu ouvrir ne doit rien coûter à celui qui l'est déjà.
 * Ses fichiers composés existent toujours sur le disque : effacer ce qui les
 * désigne donnerait à croire qu'ils ont disparu.
 */
test('un projet illisible ne détruit pas les sorties de celui qui est ouvert', async () => {
  const a = atelier();
  const invoke = async (cmd, args) => {
    if (cmd === 'projet_ouvrir') throw new Error('archive illisible');
    return a.invoke(cmd, args);
  };
  const { els } = await charge({
    invoke,
    open: async () => '/livres/casse.ozalid',
  });
  await els.get('btNouveau').declenche('click');

  els.get('resultat').textContent = '262 pages, dos 16,5 mm';
  els.get('resultat').hidden = false;
  els.get('cheminEpreuve').textContent = '/livres/A/epreuve.pdf';

  await els.get('btOuvrir').declenche('click');

  assert.equal(els.get('secLivre').hidden, false, 'le projet ouvert le reste');
  assert.equal(els.get('resultat').hidden, false, 'ses sorties aussi');
  assert.equal(els.get('resultat').textContent, '262 pages, dos 16,5 mm');
  assert.equal(els.get('cheminEpreuve').textContent, '/livres/A/epreuve.pdf');
  assert.match(els.get('etat').textContent, /illisible/);
});
