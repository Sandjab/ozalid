'use strict';

// La coquille : ce qui est montré, et quand. Une seule étape à la fois, aucune sans
// projet, et le même code derrière l'onglet et derrière le menu. La mise en page,
// elle, se vérifie dans l'application — pas ici.

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
    chapitres_trouves: 12,
    mots: 42000,
    manuscrit_absent: false,
    modifie: false,
    couverture: null,
    couverture_importee: false,
    images: [],
    interieur: { police: 'EB Garamond' },
    ...sur,
  };
}

function atelier({ recents = [], sur = {} } = {}) {
  const appels = [];
  const invoke = async (cmd, args) => {
    appels.push([cmd, args]);
    switch (cmd) {
      case 'providers_liste': return [LULU];
      case 'polices_liste': return ['Bodoni Moda'];
      case 'polices_texte_liste': return ['EB Garamond'];
      case 'maquettes_liste': return [];
      case 'recents_liste': return recents;
      case 'garde_modifications': return 'ignorer';
      case 'projet_fermer': return null;
      case 'interface_prete': return null;
      case 'couverture_apercu': throw new Error('pas de maquette');
      default: return projet(sur);
    }
  };
  return { appels, invoke, noms: () => appels.map(([c]) => c) };
}

const ETAPES = ['livre', 'interieur', 'couverture', 'livraison'];
const montree = (els) =>
  ETAPES.filter((c) => els.get(`etape${c[0].toUpperCase()}${c.slice(1)}`).hidden === false);

test('sans projet, l\'accueil s\'offre et les onglets sont inertes', async () => {
  const a = atelier();
  const { els } = await charge({ invoke: a.invoke });

  assert.equal(els.get('accueil').hidden, false);
  assert.deepEqual(montree(els), [], 'une étape est montrée sans projet');
  for (const cle of ETAPES) {
    assert.equal(els.get(`onglet-${cle}`).disabled, true, `onglet ${cle} actif sans projet`);
  }
});

test('ouvrir un projet retire l\'accueil et montre la première étape', async () => {
  const a = atelier();
  const { els } = await charge({ invoke: a.invoke });

  await els.get('btNouveau').declenche('click');

  assert.equal(els.get('accueil').hidden, true);
  assert.deepEqual(montree(els), ['livre']);
  assert.equal(els.get('onglet-livre').getAttribute('aria-selected'), 'true');
  assert.equal(els.get('titreLivre').textContent, 'Les Heures creuses');
});

test('une seule étape est montrée à la fois', async () => {
  const a = atelier();
  const { els } = await charge({ invoke: a.invoke });
  await els.get('btNouveau').declenche('click');

  await els.get('onglet-couverture').declenche('click');

  assert.deepEqual(montree(els), ['couverture']);
  assert.equal(els.get('onglet-livre').getAttribute('aria-selected'), 'false');
  assert.equal(els.get('onglet-couverture').getAttribute('aria-selected'), 'true');
});

/**
 * Le menu et l'onglet doivent appeler la même fonction. Deux implémentations
 * dériveraient, et c'est la leçon que le lot 1 a déjà payée sur « Enregistrer ».
 */
test('le menu « Aller » montre la même étape que l\'onglet', async () => {
  const a = atelier();
  const { els, menu } = await charge({ invoke: a.invoke });
  await els.get('btNouveau').declenche('click');

  await menu('aller.livraison');

  assert.deepEqual(montree(els), ['livraison']);
  assert.equal(els.get('onglet-livraison').getAttribute('aria-selected'), 'true');
});

/**
 * Les onglets sont grisés sans projet ; le menu, lui, offre toujours ses entrées.
 * Sans garde ici, ⌘3 sur l'accueil montrerait une étape vide — et une exception
 * remonterait dans le rappel de `listen`, que personne n'attrape.
 */
test('sans projet, « Aller » ne montre rien et ne lève rien', async () => {
  const a = atelier();
  const { els, menu } = await charge({ invoke: a.invoke });

  await menu('aller.couverture');

  assert.equal(els.get('accueil').hidden, false);
  assert.deepEqual(montree(els), []);
});

/**
 * L'étape courante appartient au projet qu'on regardait. Rester sur la Livraison en
 * ouvrant un autre livre donnerait à lire ses packages sous le titre du nouveau.
 */
test('ouvrir un autre projet ramène à la première étape', async () => {
  const a = atelier();
  const { els, menu } = await charge({
    invoke: a.invoke,
    open: async () => '/livres/B.ozalid',
  });
  await els.get('btNouveau').declenche('click');
  await els.get('onglet-livraison').declenche('click');

  await menu('fichier.ouvrir');

  assert.deepEqual(montree(els), ['livre']);
});

test('fermer le projet rend l\'accueil, éteint les onglets et efface l\'alerte', async () => {
  const a = atelier();
  const invoke = async (cmd, args) => {
    if (cmd === 'livre_modifier') throw new Error('titre vide');
    return a.invoke(cmd, args);
  };
  const { els, menu } = await charge({ invoke });
  await els.get('btNouveau').declenche('click');
  // Une erreur laissée en attente : « Fermer » ne passe pas par `tente()`, et c'est
  // `oublierLesSorties` qui doit la ramasser. Le message appartenait au livre qu'on
  // vient de fermer ; l'accueil le donnerait à lire comme le sien.
  await els.get('inTitre').declenche('change');
  assert.match(els.get('alerte').textContent, /titre vide/);

  await menu('fichier.fermer');

  assert.equal(els.get('accueil').hidden, false);
  assert.deepEqual(montree(els), []);
  assert.equal(els.get('onglet-livre').disabled, true);
  assert.equal(els.get('titreLivre').textContent, 'Ozalid Studio');
  assert.equal(els.get('alerte').textContent, '');
});

/**
 * Une erreur survenue à l'étape 4 doit se lire depuis l'étape 1 : l'entête est la
 * seule bande que toutes les étapes partagent, et c'est pour cela qu'elle la porte.
 */
test('une erreur s\'affiche dans l\'entête, visible depuis n\'importe quelle étape', async () => {
  const a = atelier();
  const invoke = async (cmd, args) => {
    if (cmd === 'livre_modifier') throw new Error('titre vide');
    return a.invoke(cmd, args);
  };
  const { els } = await charge({ invoke });
  await els.get('btNouveau').declenche('click');

  await els.get('inTitre').declenche('change');

  assert.match(els.get('alerte').textContent, /titre vide/);
  assert.equal(els.get('alerte').className, 'etat erreur');
});

/**
 * L'entête ne disparaît jamais : une erreur qu'on n'y efface pas y reste pour toute la
 * session, et se lirait comme le compte rendu du geste suivant, qui a réussi.
 */
test('un geste réussi efface l\'erreur du précédent', async () => {
  const a = atelier();
  let refuse = true;
  const invoke = async (cmd, args) => {
    if (cmd === 'livre_modifier' && refuse) throw new Error('titre vide');
    return a.invoke(cmd, args);
  };
  const { els } = await charge({ invoke });
  await els.get('btNouveau').declenche('click');
  await els.get('inTitre').declenche('change');
  assert.match(els.get('alerte').textContent, /titre vide/);

  refuse = false;
  await els.get('inTitre').declenche('change');

  assert.equal(els.get('alerte').textContent, '');
  assert.equal(els.get('alerte').className, 'etat');
});

/**
 * Les deux gestes d'enregistrement écrivent dans l'entête sans passer par `tente()` :
 * à eux d'effacer ce qu'ils y ont mis. Un « disque plein » laissé en place après le
 * ⌘S qui a fini par aboutir dit le contraire de ce qui vient de se passer.
 *
 * Les deux, et non le seul premier : « Enregistrer sous… » a son entrée de menu propre
 * et ne passe pas toujours par « Enregistrer ». Une ardoise qu'un seul des deux nettoie
 * est une ardoise sale un jour sur deux.
 */
for (const [libelle, entree, commande] of [
  ['Enregistrer', 'fichier.enregistrer', 'projet_enregistrer'],
  ['Enregistrer sous…', 'fichier.enregistrer_sous', 'projet_enregistrer_sous'],
]) {
  test(`« ${libelle} » qui aboutit efface l'échec du précédent`, async () => {
    const a = atelier();
    let refuse = true;
    const invoke = async (cmd, args) => {
      if (cmd === commande && refuse) throw new Error('disque plein');
      return a.invoke(cmd, args);
    };
    const { els, menu } = await charge({
      invoke,
      save: async () => '/livres/LHC.ozalid',
    });
    await els.get('btNouveau').declenche('click');   // un projet qui a déjà un chemin

    await menu(entree);
    assert.match(els.get('alerte').textContent, /disque plein/);

    refuse = false;
    await menu(entree);

    assert.equal(els.get('alerte').textContent, '');
    assert.equal(els.get('alerte').className, 'etat');
  });
}

/**
 * Un démarrage qui échoue n'affiche jamais de projet, donc ne repasse jamais par ce qui
 * remet les onglets d'accord avec la table. Nés dans l'état du balisage, ils resteraient
 * d'apparence active sans mener nulle part, et le `tablist` sans onglet sélectionné :
 * une commande sans effet ressemble à une panne, grisée elle annonce un chantier.
 */
test('un démarrage en échec laisse les onglets éteints, jamais indéterminés', async () => {
  const a = atelier();
  const invoke = async (cmd, args) => {
    if (cmd === 'providers_liste') throw new Error('aucun gabarit lisible');
    return a.invoke(cmd, args);
  };
  const { els } = await charge({ invoke });

  assert.match(els.get('alerte').textContent, /démarrage impossible/);
  for (const cle of ETAPES) {
    const onglet = els.get(`onglet-${cle}`);
    assert.equal(onglet.disabled, true, `onglet ${cle} actif après un démarrage en échec`);
    assert.equal(onglet.getAttribute('aria-selected'), 'false',
      `onglet ${cle} sans état annoncé`);
  }
});
