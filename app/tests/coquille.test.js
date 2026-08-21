'use strict';

// La coquille : ce qui est montré, et quand. Une seule étape à la fois, aucune sans
// projet, et le même code derrière l'onglet et derrière le menu. La mise en page,
// elle, se vérifie dans l'application — pas ici.

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

/** Un destinataire neuf chez un prestataire, comme le Rust en fabrique un. */
const dest = (p) => ({
  provider: p.cle, papier: p.papiers[0].cle, dos_mm: null, fond_perdu_mm: null,
});

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
    livraison: { destinataires: [dest(LULU)], courant: LULU.cle },
    envois: { main: { mode: 'police', police: 'Caveat' }, liste: [] },
    ...sur,
  };
}

/**
 * Le Rust de façade. Il tient la liste des destinataires pour de vrai : depuis le lot 3,
 * le prestataire visé vit dans le projet, et un faux qui rendrait toujours le même
 * projet ne montrerait jamais les gestes qui le déplacent.
 */
function atelier({ recents = [], sur = {}, providers = [LULU], destinataires } = {}) {
  const appels = [];
  const liste = (destinataires ?? [dest(providers[0])]).map((d) => ({ ...d }));
  let livraison = { destinataires: liste, courant: liste[0].provider };
  const vue = () => projet({ livraison, ...sur });
  const invoke = async (cmd, args) => {
    appels.push([cmd, args]);
    switch (cmd) {
      case 'providers_liste': return providers;
      case 'polices_liste': return ['Bodoni Moda'];
      case 'polices_texte_liste': return ['EB Garamond'];
      case 'mains_liste': return ['Caveat', 'Dancing Script'];
      case 'maquettes_liste': return [];
      case 'recents_liste': return recents;
      case 'garde_modifications': return 'ignorer';
      case 'projet_fermer': return null;
      case 'interface_prete': return null;
      case 'couverture_apercu': throw new Error('pas de maquette');
      case 'destinataire_viser':
        livraison = { ...livraison, courant: args.providerCle };
        return vue();
      case 'destinataire_regler':
        livraison = {
          ...livraison,
          destinataires: livraison.destinataires.map((d) => (
            d.provider === args.destinataire.provider ? args.destinataire : d
          )),
        };
        return vue();
      default: return vue();
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

/** Une touche, comme le navigateur la donne : sa lettre et le refus qu'on lui oppose. */
function touche(key) {
  const ev = { key, defaut: true, preventDefault() { ev.defaut = false; } };
  return ev;
}

/**
 * Le `tablist` a deux moitiés, et seule la première se voyait : les rôles y étaient, le
 * clavier n'y était pas. Sans les flèches, atteindre le contenu d'une étape demande de
 * tabuler à travers les onglets qui la précèdent — le défaut est d'accès, pas de
 * confort, et il ne se voit pas à la souris.
 */
test('les flèches traversent les étapes et la sélection les suit', async () => {
  const a = atelier();
  const { els, contexte } = await charge({ invoke: a.invoke });
  await els.get('btNouveau').declenche('click');

  const ev = touche('ArrowRight');
  await els.get('etapes').declenche('keydown', ev);

  assert.deepEqual(montree(els), ['interieur']);
  assert.equal(els.get('onglet-interieur').getAttribute('aria-selected'), 'true');
  // Une flèche qui change d'onglet ne doit pas, en plus, faire défiler la bande sous
  // elle : le geste est pris, il n'est pas partagé.
  assert.equal(ev.defaut, false, 'la flèche a gardé son effet par défaut');
  assert.equal(contexte.document.activeElement, els.get('onglet-interieur'),
    'le focus est resté sur l\'onglet quitté');
});

/**
 * Un seul onglet dans l'ordre de tabulation : c'est ce qui distingue un `tablist` d'une
 * rangée de boutons, et c'est ce qui rend la bande traversable d'une tabulation.
 */
test('seul l\'onglet sélectionné est dans l\'ordre de tabulation', async () => {
  const a = atelier();
  const { els } = await charge({ invoke: a.invoke });
  await els.get('btNouveau').declenche('click');

  await els.get('onglet-couverture').declenche('click');

  const rang = (cle) => els.get(`onglet-${cle}`).getAttribute('tabindex');
  assert.equal(rang('couverture'), '0');
  assert.deepEqual(
    ETAPES.filter((c) => rang(c) === '0'), ['couverture'],
    'plusieurs onglets tabulables à la fois'
  );
});

/**
 * Les flèches bouclent, et `Home`/`End` sautent aux extrémités : c'est le pattern, et
 * c'est surtout ce qui évite de compter les pas pour revenir à la première étape.
 */
test('la flèche boucle et Home revient à la première étape', async () => {
  const a = atelier();
  const { els } = await charge({ invoke: a.invoke });
  await els.get('btNouveau').declenche('click');

  await els.get('etapes').declenche('keydown', touche('ArrowLeft'));
  assert.deepEqual(montree(els), ['livraison'], 'la flèche gauche n\'a pas bouclé');

  await els.get('etapes').declenche('keydown', touche('Home'));
  assert.deepEqual(montree(els), ['livre']);
});

/**
 * Une touche que le `tablist` ne connaît pas doit rester à qui de droit : `preventDefault`
 * sur tout ce qui passe volerait la tabulation elle-même.
 */
test('une touche étrangère au tablist n\'est pas confisquée', async () => {
  const a = atelier();
  const { els } = await charge({ invoke: a.invoke });
  await els.get('btNouveau').declenche('click');

  const ev = touche('Tab');
  await els.get('etapes').declenche('keydown', ev);

  assert.equal(ev.defaut, true, 'la tabulation a été confisquée');
  assert.deepEqual(montree(els), ['livre']);
});

/**
 * Sans projet, les onglets sont éteints — et une flèche ne doit pas faire par le clavier
 * ce que le clic ne fait pas. Le garde est dans `allerA`, partagé avec le menu ; ce test
 * vérifie que le clavier passe bien par lui.
 */
test('sans projet, les flèches ne mènent nulle part', async () => {
  const a = atelier();
  const { els, contexte } = await charge({ invoke: a.invoke });

  await els.get('etapes').declenche('keydown', touche('ArrowRight'));

  assert.deepEqual(montree(els), []);
  assert.equal(contexte.document.activeElement, null);
});

/**
 * Ce que l'onglet commande et le nom que la section en prend. Les deux sortent de la
 * table `ETAPES` et d'elle seule : le balisage n'en porte aucun, et une section renommée
 * sans son onglet donnerait un `aria-controls` qui pointe dans le vide.
 */
test('chaque onglet dit quelle section il commande, et réciproquement', async () => {
  const a = atelier();
  const { els } = await charge({ invoke: a.invoke });

  for (const cle of ETAPES) {
    const section = `etape${cle[0].toUpperCase()}${cle.slice(1)}`;
    assert.equal(els.get(`onglet-${cle}`).getAttribute('aria-controls'), section);
    assert.equal(els.get(section).getAttribute('aria-labelledby'), `onglet-${cle}`);
  }
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
 * Le menu « Aller » n'est jamais grisé : le Rust offre ses quatre entrées sans savoir
 * si un projet est ouvert. ⌘3 sur l'accueil est donc un geste que rien n'empêche, et
 * c'est à l'interface de n'en rien faire — le même partage des rôles qu'« Enregistrer ».
 *
 * Deux gardes s'y emploient, `allerA` et `majEtapes`, et l'une suffirait à faire passer
 * ce test. Ce qu'il vérifie est ce qui se voit : que l'accueil reste, et qu'aucune étape
 * ne paraît sous lui.
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
 * L'ardoise ne s'ouvre qu'après la garde, et c'est ce test qui l'y retient.
 *
 * Un geste que la garde refuse n'a rien à raconter, donc rien à effacer. Plus haut dans
 * la fonction, le même `alerter('')` emporterait un message qui dit encore vrai sur un
 * ⌘S resté sans effet — et l'entête est le seul endroit de l'écran où ce message-là
 * pouvait se lire.
 */
test('un enregistrement sans projet n\'efface pas ce qu\'il ne remplace pas', async () => {
  const a = atelier();
  const invoke = async (cmd, args) => {
    if (cmd === 'providers_liste') throw new Error('aucun gabarit lisible');
    return a.invoke(cmd, args);
  };
  const { els, menu } = await charge({ invoke });
  assert.match(els.get('alerte').textContent, /démarrage impossible/);

  await menu('fichier.enregistrer');
  await menu('fichier.enregistrer_sous');

  assert.match(els.get('alerte').textContent, /démarrage impossible/,
    'un geste inerte a emporté le message qui disait pourquoi');
});

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

/* ---------- le pied ---------- */

const COMPOSITION = {
  pages: 262, chapitres: 12, gouttiere: 25, blanche: true,
  dos: 16.513, pdf: '/livres/LHC/lulu/interieur-lulu.pdf',
};

/** Ce que le pied donne à lire : le destinataire choisi, puis l'état de son dos. */
const pied = (els) => `${els.get('inDestinataire').value} ${els.get('piedDos').textContent}`.trim();

test('le pied nomme le prestataire et dit le dos non composé', async () => {
  const a = atelier();
  const { els } = await charge({ invoke: a.invoke });
  await els.get('btNouveau').declenche('click');

  assert.equal(els.get('visee').hidden, false);
  assert.deepEqual(els.get('inDestinataire').textes('option'), ['Lulu — poche 108 × 175']);
  assert.equal(pied(els), 'lulu · dos non composé');
});

/**
 * Le dos affiché au pied vient de la pagination mesurée, jamais d'une saisie. C'est la
 * même règle que pour l'aperçu de planche, et pour la même raison : un dos inventé se
 * voit au massicot, jamais avant.
 */
test('une fois l\'intérieur composé, le pied porte le dos mesuré', async () => {
  const a = atelier();
  const invoke = async (cmd, args) => {
    if (cmd === 'composer') return COMPOSITION;
    return a.invoke(cmd, args);
  };
  const { els } = await charge({ invoke });
  await els.get('btNouveau').declenche('click');

  await els.get('btComposer').declenche('click');

  assert.equal(pied(els), 'lulu · dos 16,5 mm');
});

/**
 * Sans projet, il n'y a personne à viser : le choix disparaît plutôt que d'offrir une
 * liste vide, qui se lirait comme une table de gabarits illisible.
 */
test('sans projet, le pied ne prétend rien', async () => {
  const a = atelier();
  const { els } = await charge({ invoke: a.invoke });

  assert.equal(els.get('visee').hidden, true);
  assert.equal(els.get('piedDos').textContent, '');
});

/**
 * Le pied appartient au livre ouvert. Refermé, le nom du prestataire et le dos mesuré
 * resteraient affichés sous l'accueil, où plus rien ne dit de quel livre ils parlaient.
 */
test('fermer le projet efface le pied', async () => {
  const a = atelier();
  const { els, menu } = await charge({ invoke: a.invoke });
  await els.get('btNouveau').declenche('click');
  assert.equal(els.get('visee').hidden, false);

  await menu('fichier.fermer');

  assert.equal(els.get('visee').hidden, true);
  assert.equal(els.get('piedDos').textContent, '');
  assert.equal(els.get('inDestinataire').children.length, 0);
});

const KDP = {
  cle: 'kdp-6x9', libelle: 'Amazon KDP — 6 × 9 po',
  largeur: 152.4, hauteur: 228.6, fond_perdu: 3.175, dos_publie: true,
  papiers: [{ cle: 'creme', libelle: 'Crème' }, { cle: 'blanc', libelle: 'Blanc' }],
};

/** Un prestataire à gabarit : le dos ne s'y calcule pas, il se relève. */
const COOLLIBRI = {
  cle: 'coollibri-148x210', libelle: 'CoolLibri — A5',
  largeur: 148, hauteur: 210, fond_perdu: null, dos_publie: false,
  papiers: [{ cle: 'mesure', libelle: 'Dos relevé sur le gabarit' }],
};

/**
 * Un atelier qui compose, pour partir d'un pied qui porte un dos. Tous les prestataires
 * de la liste y sont destinataires : c'est ce qui rend le pointeur déplaçable.
 */
function atelierCompose(liste, composition = COMPOSITION) {
  const a = atelier({ providers: liste, destinataires: liste.map(dest) });
  return async (cmd, args) => {
    if (cmd === 'composer') return composition;
    return a.invoke(cmd, args);
  };
}

/**
 * Les deux causes que le pied porte lui-même : le destinataire qu'il vise, et le papier
 * qui périme le dos sans rien changer d'autre à l'écran. Un pied qui ne repart pas sur
 * ces gestes-là dit un dos qui vaut pour un autre livre que celui qu'on regarde.
 */
test('viser un autre destinataire renomme le pied et lui retire le dos', async () => {
  const { els } = await charge({ invoke: atelierCompose([LULU, KDP]) });
  await els.get('btNouveau').declenche('click');
  await els.get('btComposer').declenche('click');
  assert.equal(pied(els), 'lulu · dos 16,5 mm');

  els.get('inDestinataire').value = 'kdp-6x9';
  await els.get('inDestinataire').declenche('change');

  assert.equal(pied(els), 'kdp-6x9 · dos non composé');
});

test('changer de papier retire le dos du pied', async () => {
  const { els } = await charge({ invoke: atelierCompose([KDP]) });
  await els.get('btNouveau').declenche('click');
  await els.get('btComposer').declenche('click');
  assert.equal(pied(els), 'kdp-6x9 · dos 16,5 mm');

  els.get('dest-papier-kdp-6x9').value = 'blanc';
  await els.get('dest-papier-kdp-6x9').declenche('change');

  assert.equal(pied(els), 'kdp-6x9 · dos non composé');
});

/**
 * Chez un prestataire à gabarit, le dos ne se calcule pas : il se relève. La composition
 * a beau aboutir — 262 pages s'affichent au-dessus — elle ne rend aucun dos, et rien de
 * ce qu'on ferait ensuite n'en produirait un. « Non composé » enverrait recomposer en
 * boucle un livre dont la pagination est déjà juste.
 */
test('chez un prestataire à gabarit, le pied ne réclame pas une composition', async () => {
  const { els } = await charge({
    invoke: atelierCompose([COOLLIBRI], { ...COMPOSITION, dos: null }),
  });
  await els.get('btNouveau').declenche('click');

  await els.get('btComposer').declenche('click');

  assert.equal(pied(els), 'coollibri-148x210 · dos relevé sur le gabarit');
});

/**
 * Sans gabarit lisible, il n'y a pas de prestataire à nommer — mais les boutons de
 * l'accueil restent cliquables, et le pied est le premier à demander le prestataire quand
 * un projet s'ouvre. Muet, il laisse l'application dégradée ; sans garde, il lève depuis
 * `afficherProjet`, et l'exception traverse `tente()` en laissant l'écran à moitié dessiné.
 */
test('un démarrage en échec ne fait pas lever le pied au premier projet', async () => {
  const a = atelier();
  const invoke = async (cmd, args) => {
    if (cmd === 'providers_liste') throw new Error('aucun gabarit lisible');
    return a.invoke(cmd, args);
  };
  const { els } = await charge({ invoke });

  await els.get('btNouveau').declenche('click');

  // Le projet s'ouvre : c'est ce qui rend l'assertion suivante probante — le pied s'est
  // bien dessiné, il n'est pas resté muet faute d'avoir été appelé.
  assert.equal(els.get('titreLivre').textContent, 'Les Heures creuses');
  assert.equal(els.get('visee').hidden, true);
  assert.equal(els.get('piedDos').textContent, '');
});

/**
 * Ce test lit le balisage au lieu de passer par l'application, et ne prouve donc rien
 * de son comportement. Ce n'est pas un exemple à suivre : c'est le seul filet possible
 * pour cette propriété-là.
 *
 * Le faux DOM ne rapporte du HTML que la balise, `disabled`, `hidden` et `value` —
 * `aria-live` lui est invisible, et l'étendre pour un attribut ferait payer à soixante-
 * dix tests le prix d'un seul. Or l'entête est désormais le canal d'erreur unique, et
 * le refus d'une saisie laisse le focus dans le champ refusé : sans cet attribut, un
 * lecteur d'écran n'annonce jamais l'erreur. Une réécriture de l'entête l'emporterait
 * d'un coup, sans que rien ne change à l'écran ni dans la suite.
 */
test('l\'entête s\'annonce à qui ne la voit pas', () => {
  const html = fs.readFileSync(path.join(__dirname, '..', 'src', 'index.html'), 'utf8');

  assert.match(html, /id="alerte"[^>]*aria-live/,
    'l\'entête ne s\'annonce plus : le focus reste dans le champ refusé');
});

/* ---------- les témoins d'attention ---------- */

const sous = (els, cle) => els.get(`sous-${cle}`).textContent;
const alerte = (els, cle) => els.get(`onglet-${cle}`).className === 'alerte';

test('l\'onglet Livre dit l\'état du manuscrit sans crier', async () => {
  const a = atelier();
  const { els } = await charge({ invoke: a.invoke });
  await els.get('btNouveau').declenche('click');

  assert.equal(sous(els, 'livre'), '12 chapitres');
  assert.equal(alerte(els, 'livre'), false);
});

/**
 * L'écart avec le contrôle d'intégrité est le seul signe qu'un manuscrit périmé
 * laisse : le gabarit, la police et le papier, eux, n'ont pas bougé.
 */
test('un écart de contrôle d\'intégrité allume le témoin du Livre', async () => {
  const a = atelier({ sur: { chapitres_trouves: 2, livre: {
    titre: 'Les Heures creuses', titre_page: null, auteur: 'Ivan Pjig',
    genre: 'roman', copyright: '', chapitres: 64,
  } } });
  const { els } = await charge({ invoke: a.invoke });
  await els.get('btNouveau').declenche('click');

  assert.equal(sous(els, 'livre'), '2 chapitres, 64 attendus');
  assert.equal(alerte(els, 'livre'), true);
});

/** Un manuscrit absent est un état de projet neuf, pas une anomalie à signaler. */
test('un manuscrit absent se dit, sans allumer de témoin', async () => {
  const a = atelier({ sur: { manuscrit_absent: true, chapitres_trouves: 0, mots: 0 } });
  const { els } = await charge({ invoke: a.invoke });
  await els.get('btNouveau').declenche('click');

  assert.equal(sous(els, 'livre'), 'aucun manuscrit');
  assert.equal(alerte(els, 'livre'), false);
});

test('sans maquette, l\'onglet Couverture le dit et s\'allume', async () => {
  const a = atelier();
  const { els } = await charge({ invoke: a.invoke });
  await els.get('btNouveau').declenche('click');

  assert.equal(sous(els, 'couverture'), 'aucune maquette');
  assert.equal(alerte(els, 'couverture'), true);
});

/**
 * Le mode est nommé comme le panneau le nomme. Recopié ici, le libellé survivrait au
 * jour où le schéma renomme un mode, et l'onglet dirait un mot que plus rien n'offre.
 */
test('une maquette en place nomme son mode et éteint le témoin', async () => {
  const a = atelier({ sur: { couverture: { mode: 'bandeau' } } });
  const { els } = await charge({ invoke: a.invoke });
  await els.get('btNouveau').declenche('click');

  assert.equal(sous(els, 'couverture'), 'Bandeau');
  assert.equal(alerte(els, 'couverture'), false);
});

test('sans composition, l\'onglet Intérieur nomme la police et n\'alerte pas', async () => {
  const a = atelier();
  const { els } = await charge({ invoke: a.invoke });
  await els.get('btNouveau').declenche('click');

  assert.equal(sous(els, 'interieur'), 'EB Garamond');
  assert.equal(alerte(els, 'interieur'), false);
});

/**
 * Le sous-libellé s'ajoute au nom de l'étape, il ne le remplace pas. Les deux textes
 * vivent dans le même bouton, et un onglet qui ne dirait plus que « 12 chapitres »
 * aurait perdu le seul mot qui dit où il mène.
 */
test('l\'onglet garde le nom de son étape sous le sous-libellé', async () => {
  const a = atelier();
  const { els } = await charge({ invoke: a.invoke });
  await els.get('btNouveau').declenche('click');

  assert.match(els.get('onglet-livre').textContent, /^1 · Livre/);
  assert.match(els.get('onglet-livre').textContent, /12 chapitres$/);
});

/**
 * Changer de gabarit périme le dos : le même manuscrit ne fait pas le même nombre de
 * pages en poche et en grand format. Le témoin dit où le réparer — à l'Intérieur, la
 * seule étape qui recompose.
 */
test('un dos périmé par un changement de gabarit allume le témoin de l\'Intérieur', async () => {
  const { els } = await charge({ invoke: atelierCompose([LULU, KDP]) });
  await els.get('btNouveau').declenche('click');
  await els.get('btComposer').declenche('click');
  assert.equal(alerte(els, 'interieur'), false, 'un dos frais ne périme rien');

  els.get('inDestinataire').value = 'kdp-6x9';
  await els.get('inDestinataire').declenche('change');

  assert.equal(sous(els, 'interieur'), 'dos périmé');
  assert.equal(alerte(els, 'interieur'), true);
});

/**
 * Le témoin dit où réparer ; il doit donc s'éteindre quand on y répare. Recomposer est
 * le seul geste qui rend un dos juste, et il ne repasse pas par `afficherProjet` : un
 * témoin qui survivrait à sa réparation enverrait recomposer un livre déjà composé.
 */
test('recomposer éteint le témoin de l\'Intérieur', async () => {
  const { els } = await charge({ invoke: atelierCompose([LULU, KDP]) });
  await els.get('btNouveau').declenche('click');
  await els.get('btComposer').declenche('click');
  els.get('inDestinataire').value = 'kdp-6x9';
  await els.get('inDestinataire').declenche('change');
  assert.equal(alerte(els, 'interieur'), true, 'le dos devait être périmé avant');

  await els.get('btComposer').declenche('click');

  assert.equal(sous(els, 'interieur'), 'EB Garamond');
  assert.equal(alerte(els, 'interieur'), false);
});

/**
 * Une police refusée ne périme rien : le Rust n'a rien changé, et le dos vaut toujours
 * pour le livre tel qu'il est.
 *
 * Le lot 2 avait payé ce défaut, parce que `dosCourant()` lisait alors la police
 * *choisie à l'écran* : remis d'accord avant que le panneau ne soit reposé, le témoin
 * comparait le dos à une police que le refus venait d'annuler et envoyait recomposer un
 * livre déjà juste. Le lot 3 l'a fermé en faisant lire le projet plutôt que les
 * contrôles ; ce test dit ce qui doit se voir, et resterait vrai d'une autre solution.
 */
test('une police refusée n\'allume pas le témoin de l\'Intérieur', async () => {
  const base = atelierCompose([LULU]);
  const invoke = async (cmd, args) => {
    if (cmd === 'interieur_modifier') throw new Error('police d\'intérieur inconnue');
    return base(cmd, args);
  };
  const { els } = await charge({ invoke });
  await els.get('btNouveau').declenche('click');
  await els.get('btComposer').declenche('click');

  els.get('inPoliceInterieur').value = 'Alegreya';
  await els.get('inPoliceInterieur').declenche('change');

  assert.match(els.get('alerte').textContent, /police d'intérieur inconnue/);
  assert.equal(els.get('inPoliceInterieur').value, 'EB Garamond',
    'le panneau n\'est pas revenu au projet : le témoin ne prouverait rien');
  assert.equal(sous(els, 'interieur'), 'EB Garamond');
  assert.equal(alerte(els, 'interieur'), false);
});

/**
 * Un dos jamais composé ne réclame rien : c'est l'état d'un projet qu'on vient
 * d'ouvrir, et le pied le dit déjà. Seul un dos qui a existé et ne vaut plus allume.
 */
test('un dos jamais composé n\'allume pas le témoin de l\'Intérieur', async () => {
  const { els } = await charge({ invoke: atelierCompose([LULU, KDP]) });
  await els.get('btNouveau').declenche('click');

  els.get('inDestinataire').value = 'kdp-6x9';
  await els.get('inDestinataire').declenche('change');

  assert.equal(sous(els, 'interieur'), 'EB Garamond');
  assert.equal(alerte(els, 'interieur'), false);
});

/**
 * Le papier périme le dos sans rien changer d'autre à l'écran : c'est le geste où un
 * témoin qui ne repartirait pas serait le plus difficile à démentir.
 */
test('changer de papier allume aussi le témoin de l\'Intérieur', async () => {
  const { els } = await charge({ invoke: atelierCompose([KDP]) });
  await els.get('btNouveau').declenche('click');
  await els.get('btComposer').declenche('click');
  assert.equal(alerte(els, 'interieur'), false);

  els.get('dest-papier-kdp-6x9').value = 'blanc';
  await els.get('dest-papier-kdp-6x9').declenche('change');

  assert.equal(sous(els, 'interieur'), 'dos périmé');
  assert.equal(alerte(els, 'interieur'), true);
});

/**
 * L'étape Livraison n'a rien de vrai à dire avant qu'un package n'ait été généré, et le
 * pied porte déjà le dos. Un sous-libellé de remplissage se lirait comme un état.
 */
test('l\'onglet Livraison ne meuble pas', async () => {
  const a = atelier();
  const { els } = await charge({ invoke: a.invoke });
  await els.get('btNouveau').declenche('click');

  assert.equal(sous(els, 'livraison'), '');
  assert.equal(alerte(els, 'livraison'), false);
});

/**
 * Les sous-libellés appartiennent au livre ouvert. Refermé, « 12 chapitres » resterait
 * sous l'accueil, où plus rien ne dit de quel livre il parlait — et le témoin de la
 * Couverture y réclamerait une maquette pour un projet qui n'existe plus.
 */
test('fermer le projet efface les sous-libellés et éteint les témoins', async () => {
  const a = atelier();
  const { els, menu } = await charge({ invoke: a.invoke });
  await els.get('btNouveau').declenche('click');
  assert.equal(alerte(els, 'couverture'), true);

  await menu('fichier.fermer');

  for (const cle of ETAPES) {
    assert.equal(sous(els, cle), '', `sous-libellé ${cle} survit au projet fermé`);
    assert.equal(alerte(els, cle), false, `témoin ${cle} survit au projet fermé`);
  }
});

test('la dédicace saisie part avec le livre', async () => {
  const a = atelier();
  const { els } = await charge({ invoke: a.invoke });
  await els.get('btNouveau').declenche('click');

  els.get('inDedicace').value = 'À M., qui a tenu la lampe.';
  await els.get('inDedicace').declenche('change');

  const envoi = a.appels.findLast(([c]) => c === 'livre_modifier');
  assert.ok(envoi, 'aucun livre_modifier : le champ n\'a pas d\'écouteur');
  assert.equal(envoi[1].livre.dedicace, 'À M., qui a tenu la lampe.');
});

/**
 * `livre_modifier` remplace le livre entier, et le champ est facultatif côté Rust : un
 * livre envoyé sans sa dédicace ne lève rien, il l'efface. Modifier son titre suffirait
 * donc à la perdre, sans un message et sans que rien ne se voie avant le tirage.
 */
test('modifier un autre champ n\'efface pas la dédicace', async () => {
  const a = atelier({
    sur: {
      livre: {
        titre: 'Les Heures creuses', titre_page: null, auteur: 'Ivan Pjig',
        genre: 'roman', copyright: '', chapitres: null, dedicace: 'À M.',
      },
    },
  });
  const { els } = await charge({ invoke: a.invoke });
  await els.get('btNouveau').declenche('click');

  els.get('inTitre').value = 'Les Heures pleines';
  await els.get('inTitre').declenche('change');

  const envoi = a.appels.findLast(([c]) => c === 'livre_modifier');
  assert.equal(envoi[1].livre.dedicace, 'À M.', 'la dédicace a été effacée en douce');
});

/**
 * `envois_modifier` remplace l'objet entier : un envoi ajouté sans la main du livre
 * ramènerait la main au défaut, et tous les exemplaires changeraient d'écriture sans
 * qu'on l'ait demandé. Même piège que la dédicace, même garde.
 */
test('ajouter un envoi conserve la main du livre', async () => {
  const a = atelier({
    sur: { envois: { main: { mode: 'police', police: 'Dancing Script' }, liste: [] } },
  });
  const { els } = await charge({ invoke: a.invoke });
  await els.get('btNouveau').declenche('click');

  els.get('inDedicataire').value = 'Léa';
  await els.get('btAjouterEnvoi').declenche('click');

  const envoi = a.appels.findLast(([c]) => c === 'envois_modifier');
  assert.ok(envoi, 'aucun envois_modifier : le bouton n\'a pas d\'écouteur');
  assert.equal(envoi[1].envois.main.police, 'Dancing Script');
  assert.equal(envoi[1].envois.liste[0].dedicataire, 'Léa');
});

/**
 * Le bouton suit la liste. Vérifier qu'il est éteint sans envoi ne prouverait rien : il
 * l'est déjà dans le HTML, et le test passerait sans une ligne de JavaScript. C'est
 * l'allumage qui se garde.
 */
test('le bouton des envois s\'allume dès qu\'un mot est écrit', async () => {
  const avec = atelier({
    sur: {
      envois: {
        main: { mode: 'police', police: 'Caveat' },
        liste: [{ dedicataire: 'Léa', contenu: 'À Léa.' }],
      },
    },
  });
  const { els } = await charge({ invoke: avec.invoke });
  await els.get('btNouveau').declenche('click');
  assert.equal(els.get('btEnvoyer').disabled, false,
    'un envoi est écrit et le bouton reste éteint');

  const sans = atelier();
  const b = await charge({ invoke: sans.invoke });
  await b.els.get('btNouveau').declenche('click');
  assert.equal(b.els.get('btEnvoyer').disabled, true,
    'la liste est vide et le bouton reste allumé');
});
