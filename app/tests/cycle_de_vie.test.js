'use strict';

// Câblage du cycle de vie : ce que l'interface envoie au Rust selon la réponse de la
// garde, et ce que le menu déclenche. La boîte de dialogue elle-même est native :
// elle se vérifie dans l'application, pas ici.

const fs = require('node:fs');
const path = require('node:path');
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
    envois: { main: { mode: 'police', police: 'Caveat' }, liste: [] },
    livraison: {
      destinataires: [{ provider: 'lulu', papier: 'standard', dos_mm: null, fond_perdu_mm: null }],
      courant: 'lulu',
    },
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
      case 'jetons_liste': return ['%TITRE%', '%AUTEUR%', '%GENRE%', '%EDITEUR%', '%COLLECTION%', '%MONOGRAMME%'];
      case 'mains_liste': return ['Caveat', 'Dancing Script'];
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

  assert.equal(els.get('accueil').hidden, false);
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
  assert.equal(els.get('etapeLivre').hidden, false);
});

test('la garde refusée arrête tout : rien n\'est ouvert, rien n\'est perdu', async () => {
  const a = atelier({ garde: 'annuler' });
  const { els } = await charge({ invoke: a.invoke });

  await els.get('btNouveau').declenche('click');

  assert.ok(!a.noms().includes('projet_nouveau'),
    'un « Annuler » qui crée quand même le projet aurait perdu le travail');
  assert.equal(els.get('accueil').hidden, false);
});

test('la garde acceptée laisse passer', async () => {
  const a = atelier({ garde: 'ignorer' });
  const { els } = await charge({ invoke: a.invoke });

  await els.get('btNouveau').declenche('click');

  assert.ok(a.noms().includes('projet_nouveau'));
  assert.equal(els.get('etapeLivre').hidden, false);
});

test('« Enregistrer » réécrit en place, sans sélecteur de fichiers', async () => {
  const a = atelier();
  let demande = 0;
  const { els, menu } = await charge({
    invoke: a.invoke,
    save: async () => { demande += 1; return '/ailleurs.ozalid'; },
  });
  await els.get('btNouveau').declenche('click');   // ouvre un projet qui a un chemin
  await menu('fichier.enregistrer');

  assert.ok(a.noms().includes('projet_enregistrer'));
  assert.equal(demande, 0, 'un projet déjà posé ne redemande pas où');
});

test('un projet jamais enregistré bascule sur « Enregistrer sous… »', async () => {
  const a = atelier({ sur: { chemin: null, modifie: false } });
  let demande = 0;
  const { els, menu } = await charge({
    invoke: a.invoke,
    save: async () => { demande += 1; return '/livres/LHC.ozalid'; },
  });
  await els.get('btNouveau').declenche('click');

  assert.equal(els.get('etatEnregistrement').textContent, 'jamais enregistré');

  await menu('fichier.enregistrer');

  assert.equal(demande, 1, 'sans chemin, il faut bien demander où poser le projet');
  assert.ok(a.noms().includes('projet_enregistrer_sous'));
  assert.ok(!a.noms().includes('projet_enregistrer'),
    'réécrire en place un projet qui n\'est nulle part');
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
  assert.equal(els.get('etapeLivre').hidden, false);

  await menu('fichier.fermer');
  assert.ok(a.noms().includes('projet_fermer'));
  assert.equal(els.get('accueil').hidden, false);
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
  assert.equal(els.get('accueil').hidden, false);
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

/**
 * Le Rust et le front se donnent rendez-vous sur des chaînes que ni l'un ni l'autre ne
 * vérifie. Une clé renommée dans `ETAPES`, une faute de frappe dans `menu.rs`, et
 * l'entrée de menu comme son accélérateur cessent d'agir — sans message, sans test
 * rouge, dans les deux langages. Muet, le geste se lit comme une panne de
 * l'application ; nommé, il se lit comme la faute de frappe qu'il est.
 */
test('une entrée de menu que le front ne connaît pas se donne à voir', async () => {
  const a = atelier();
  const { els, menu } = await charge({ invoke: a.invoke });

  await menu('aller.quatrieme_de_couverture');

  assert.match(els.get('alerte').textContent, /aller\.quatrieme_de_couverture/,
    'une entrée inconnue n\'a rien dit : le menu paraît en panne');
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
/**
 * Les canaux de compte rendu, lus dans le balisage au lieu d'être recopiés de
 * `oublierLesSorties`.
 *
 * Un test qui énumère ce que la fonction efface ne peut que confirmer ce qu'elle fait :
 * il resterait vert le jour où l'on ajoute un `#etatMachin` sans l'effacer, c'est-à-dire
 * précisément le jour où il devrait parler. Celui-ci part de l'écran — tout ce qui porte
 * `etat` ou `resultat` rend compte d'un geste, donc appartient au livre qui l'a produit.
 *
 * Deux échappent, et pour la raison inverse : `etatEnregistrement` décrit le projet
 * qu'on vient d'ouvrir, pas celui qu'on quitte, et `etatDiffusion` ne décrit aucun
 * livre — l'adresse du modèle et sa clé appartiennent à la machine, et survivent à tous
 * les livres qu'on y ouvrira.
 */
const DECRIT_LE_NOUVEAU = new Set(['etatEnregistrement', 'etatDiffusion']);

function canauxDeCompteRendu() {
  const html = fs.readFileSync(
    path.join(__dirname, '..', 'src', 'index.html'),
    'utf8'
  );
  const canaux = [];
  for (const [, attrs] of html.matchAll(/<\w+([^>]*)>/g)) {
    const id = attrs.match(/\bid="([^"]+)"/);
    const classe = attrs.match(/\bclass="([^"]*)"/);
    if (!id || !classe) continue;
    const classes = classe[1].split(/\s+/);
    if (!classes.includes('etat') && !classes.includes('resultat')) continue;
    if (!DECRIT_LE_NOUVEAU.has(id[1])) canaux.push(id[1]);
  }
  return canaux;
}

test('ouvrir un autre projet oublie les sorties du précédent', async () => {
  const a = atelier();
  const { els, menu } = await charge({
    invoke: a.invoke,
    open: async () => '/livres/B.ozalid',
  });
  await els.get('btNouveau').declenche('click');

  const canaux = canauxDeCompteRendu();
  assert.ok(canaux.length >= 5, `inventaire suspect : ${canaux.join(', ')}`);
  // Ce que des gestes du projet A auraient laissé à l'écran — tous, sans en choisir.
  for (const id of canaux) {
    els.get(id).textContent = `compte rendu du livre A (${id})`;
    els.get(id).hidden = false;
  }
  els.get('cheminEpreuve').textContent = '/livres/A/epreuve.pdf';
  els.get('apercu').src = 'data:image/png;base64,AAAA';

  // Par le menu : un projet ouvert masque l'accueil, et « Ouvrir » avec lui.
  await menu('fichier.ouvrir');

  for (const id of canaux) {
    assert.equal(els.get(id).textContent, '',
      `« ${id} » raconte encore le livre qu'on vient de quitter`);
  }
  assert.equal(els.get('packages').hidden, true);
  assert.equal(els.get('cheminEpreuve').textContent, '');
  assert.equal(els.get('apercu').src, undefined,
    'la couverture du livre précédent reste affichée');
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
  const { els, menu } = await charge({
    invoke,
    open: async () => '/livres/casse.ozalid',
  });
  await els.get('btNouveau').declenche('click');

  els.get('packages').textContent = 'package Lulu écrit';
  els.get('packages').hidden = false;
  els.get('cheminEpreuve').textContent = '/livres/A/epreuve.pdf';

  await menu('fichier.ouvrir');

  assert.equal(els.get('etapeLivre').hidden, false, 'le projet ouvert le reste');
  assert.equal(els.get('packages').hidden, false, 'ses sorties aussi');
  assert.equal(els.get('packages').textContent, 'package Lulu écrit');
  assert.equal(els.get('cheminEpreuve').textContent, '/livres/A/epreuve.pdf');
  assert.match(els.get('alerte').textContent, /illisible/);
});

/**
 * Un champ que le clavier tient encore n'a rien envoyé : `change` n'arrive qu'à la
 * perte du focus, et l'accélérateur du menu natif ne la provoque pas — la page garde
 * son focus pendant que le Rust enregistre.
 *
 * Sans la validation de la saisie, ⌘S écrivait le fichier avec l'ancienne valeur, puis
 * `afficherProjet` réécrivait le champ avec elle : la frappe était perdue deux fois, et
 * l'écran ne montrait rien qui l'annonce.
 */
test('⌘S enregistre ce que le champ porte encore, sans l\'avoir quitté', async () => {
  const a = atelier();
  let livre = null;
  const invoke = async (cmd, args) => {
    // Le Rust rend le livre qu'on lui a envoyé : un faux qui rendrait toujours le même
    // ne montrerait jamais la valeur revenir dans le champ.
    if (cmd === 'livre_modifier') livre = args.livre;
    const p = await a.invoke(cmd, args);
    return livre && p ? { ...p, livre: { ...p.livre, ...livre } } : p;
  };
  const { els, contexte, menu } = await charge({ invoke });
  await els.get('btNouveau').declenche('click');

  const champ = els.get('inTitre');
  champ.focus();
  champ.value = 'Les Heures pleines';
  await menu('fichier.enregistrer');

  const noms = a.noms();
  assert.ok(noms.includes('livre_modifier'), 'la frappe en cours n\'est jamais partie');
  assert.ok(noms.indexOf('livre_modifier') < noms.lastIndexOf('projet_enregistrer'),
    'le fichier a été écrit avant que la frappe n\'arrive');
  assert.equal(livre.titre, 'Les Heures pleines');
  assert.equal(champ.value, 'Les Heures pleines', 'le champ est revenu en arrière');
  assert.equal(contexte.document.activeElement, null);
});
