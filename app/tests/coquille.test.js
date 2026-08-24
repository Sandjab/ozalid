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
    envois: { gabarit: '', liste: [] },
    ...sur,
  };
}

/**
 * Un événement de souris, réduit à ce que la manipulation directe en lit. Même forme
 * que dans `couverture.test.js`, dont les gestes du canevas reprennent l'idiome.
 */
const souris = (x, y) => ({
  button: 0, clientX: x, clientY: y, pointerId: 1,
  preventDefault() {}, stopPropagation() {},
});

/**
 * Le placement d'un envoi neuf, tel que `Place::default()` le pose côté Rust : la page
 * de titre, au bas. Le faux le reprend pour que l'écran montre les mêmes chiffres.
 */
const PLACE_DEFAUT = { page: 3, x: 0.5, y: 0.8, taille: 0.6, angle: 0 };

/**
 * Le Rust de façade. Il tient la liste des destinataires pour de vrai : depuis le lot 3,
 * le prestataire visé vit dans le projet, et un faux qui rendrait toujours le même
 * projet ne montrerait jamais les gestes qui le déplacent.
 */
function atelier({
  recents = [], sur = {}, providers = [LULU], destinataires,
  acces = { url: '', cle_posee: false }, composition,
} = {}) {
  const appels = [];
  const liste = (destinataires ?? [dest(providers[0])]).map((d) => ({ ...d }));
  let livraison = { destinataires: liste, courant: liste[0].provider, deja_compose: false };
  // Les règles du Rust, modélisées ici parce que le front les lit désormais dans le
  // projet au lieu de les tenir lui-même : une mesure entre chez le destinataire pour
  // qui elle a été faite, et tout ce qui pagine les efface toutes.
  const oublier = () => {
    livraison = {
      ...livraison,
      destinataires: livraison.destinataires.map(({ compose, ...d }) => d),
    };
  };
  // Les envois sont tenus pour de vrai, comme les destinataires : depuis que la main
  // appartient à l'exemplaire, un faux qui rendrait toujours la même liste ne montrerait
  // jamais qu'un envoi neuf hérite du précédent.
  let envois = sur.envois ?? { gabarit: '', liste: [] };
  const vue = () => projet({ livraison, ...sur, envois });
  const invoke = async (cmd, args) => {
    appels.push([cmd, args]);
    switch (cmd) {
      case 'providers_liste': return providers;
      case 'polices_liste': return ['Bodoni Moda'];
      case 'polices_texte_liste': return ['EB Garamond'];
      case 'jetons_liste': return ['%TITRE%', '%AUTEUR%', '%GENRE%', '%EDITEUR%', '%COLLECTION%', '%MONOGRAMME%'];
      case 'mains_liste': return ['Caveat', 'Dancing Script'];
      // L'accès au modèle appartient à la machine : le Rust ne rend jamais la clé, il
      // dit seulement qu'elle est posée.
      case 'diffusion_lire': return acces;
      case 'diffusion_regler': return { url: args.url, cle_posee: args.cle !== '' };
      case 'envoi_generer': return 'data:image/png;base64,QUJD';
      // Les trois rendus du canevas. Muets : ce qui compte dans un test de coquille,
      // c'est quelle page est demandée, jamais ce que Typst en fait.
      case 'envoi_vignettes': return ['data:image/png;base64,UDE', 'data:image/png;base64,UDI',
        'data:image/png;base64,UDM', 'data:image/png;base64,UDQ'];
      case 'envoi_page': return 'data:image/png;base64,R1JBTkQ=';
      case 'envoi_apercu': return 'data:image/png;base64,Q09ORklSTQ==';
      // Ce que la génération rend, un exemplaire par ligne. Le dos y est un nombre :
      // c'est le front qui l'écrit, et c'est ce qu'un test peut lui reprocher.
      case 'envoyer': return envois.liste.map((e) => ({
        dedicataire: e.dedicataire,
        dossier: e.dedicataire,
        package: { pages: 262, dos: 16.513 },
        vignette: null,
      }));
      case 'envoi_objet': return { image: 'data:image/png;base64,T0JK', ratio: 0.2 };
      case 'envoi_ajouter': {
        // La règle vit dans le Rust — `Envois::ajouter` : un envoi neuf naît comme le
        // précédent, main et placement compris, mais sans son mot ni son image.
        const modele = envois.liste[envois.liste.length - 1];
        envois = { ...envois, liste: [...envois.liste, {
          dedicataire: args.dedicataire,
          main: modele?.main ?? { mode: 'police', police: 'Caveat' },
          place: modele?.place ?? PLACE_DEFAUT,
          contenu: '',
          image: null,
        }] };
        return vue();
      }
      case 'envoi_regler':
        envois = { ...envois,
          liste: envois.liste.map((e, i) => (i === args.index ? args.envoi : e)) };
        return vue();
      case 'envoi_retirer':
        envois = { ...envois, liste: envois.liste.filter((_, i) => i !== args.index) };
        return vue();
      case 'envois_gabarit':
        envois = { ...envois, gabarit: args.gabarit };
        return vue();
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
        // Le papier et le relevé déplacent le dos : le Rust efface la mesure de ce
        // destinataire-là, et ne reprend jamais celle que l'interface lui enverrait.
        livraison = {
          ...livraison,
          destinataires: livraison.destinataires.map((d) => (
            d.provider === args.destinataire.provider
              ? { ...args.destinataire, compose: undefined }
              : d
          )),
        };
        return vue();
      case 'composer':
        livraison = {
          ...livraison,
          deja_compose: true,
          destinataires: livraison.destinataires.map((d) => (
            d.provider === livraison.courant
              ? {
                ...d,
                compose: {
                  pages: composition.pages,
                  gouttiere: composition.gouttiere,
                  blanche: composition.blanche,
                  dos: composition.dos,
                },
              }
              : d
          )),
        };
        return { ...composition, projet: vue() };
      case 'livre_modifier':
      case 'interieur_modifier':
      case 'manuscrit_reimporter':
      case 'manuscrit_choisir':
        oublier();
        return vue();
      default: return vue();
    }
  };
  return { appels, invoke, noms: () => appels.map(([c]) => c) };
}

/**
 * Le geste qui compose, depuis que le bouton n'existe plus : charger un manuscrit.
 *
 * C'est le consentement du chantier « intérieur sans onglet » — ouvrir un `.ozalid` ne
 * compose pas, charger un manuscrit oui. Les tests qui ont besoin d'un livre composé
 * passent donc par là, comme l'utilisateur.
 *
 * `manuscritRemplace` lance la composition sans l'attendre — l'utilisateur non plus.
 * Un tour de boucle pour qu'elle aboutisse avant qu'on regarde le résultat.
 */
const faireComposer = async (els) => {
  await els.get('btReimporter').declenche('click');
  await new Promise((r) => setImmediate(r));
};

/**
 * Aller à l'étape Envois, comme on y va : par son onglet.
 *
 * C'est l'arrivée qui rend le rail, la page et l'objet — pas l'ouverture du projet :
 * ils coûtent une composition, et la payer à qui vient regarder une couverture serait
 * le prix de ce qu'il n'a pas demandé. Les rendus partent sans être attendus ; un tour
 * de boucle pour qu'ils aboutissent, comme pour la composition.
 */
const allerAuxEnvois = async (els) => {
  await els.get('onglet-envois').declenche('click');
  await new Promise((r) => setImmediate(r));
};

const ETAPES = ['livre', 'couverture', 'livraison', 'envois'];
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

  assert.deepEqual(montree(els), ['couverture']);
  assert.equal(els.get('onglet-couverture').getAttribute('aria-selected'), 'true');
  // Une flèche qui change d'onglet ne doit pas, en plus, faire défiler la bande sous
  // elle : le geste est pris, il n'est pas partagé.
  assert.equal(ev.defaut, false, 'la flèche a gardé son effet par défaut');
  assert.equal(contexte.document.activeElement, els.get('onglet-couverture'),
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
  assert.deepEqual(montree(els), ['envois'], 'la flèche gauche n\'a pas bouclé');

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
 * Les deux gestes d'enregistrement écrivent dans l'entête : à eux d'effacer ce qu'ils y
 * ont mis. Un « disque plein » laissé en place après le ⌘S qui a fini par aboutir dit
 * le contraire de ce qui vient de se passer.
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
 * Le même principe une porte plus loin : un sélecteur de fichiers refermé sans choisir
 * n'a rien fait non plus, et l'échec qu'il laisse derrière lui dit toujours vrai — le
 * projet est encore là où le disque plein l'avait laissé.
 *
 * C'est ce test qui retient l'ardoise du côté de l'écriture : ouvert avant le sélecteur,
 * `alerter('')` effacerait le message pour un geste abandonné.
 */
test('« Enregistrer sous… » abandonné garde l\'échec qui dit encore vrai', async () => {
  const a = atelier();
  const invoke = async (cmd, args) => {
    if (cmd === 'projet_enregistrer') throw new Error('disque plein');
    return a.invoke(cmd, args);
  };
  const { els, menu } = await charge({ invoke, save: async () => null });
  await els.get('btNouveau').declenche('click');

  await menu('fichier.enregistrer');
  assert.match(els.get('alerte').textContent, /disque plein/);

  await menu('fichier.enregistrer_sous');

  assert.match(els.get('alerte').textContent, /disque plein/,
    'un sélecteur refermé sans choisir a emporté le message');
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

/* ---------- la taille de la fenêtre ---------- */

/**
 * La seule mention de l'écran qui ne parle pas du livre.
 *
 * Elle est là pour qu'on puisse **dire** la fenêtre : une mise en page se juge à une
 * taille, et « c'est coupé chez moi » sans le chiffre ne se reproduit pas. Elle ne
 * dépend d'aucun projet — un écran d'accueil la porte comme une étape.
 */
test('l\'entête porte la taille de la fenêtre, et la suit', async () => {
  const a = atelier();
  const { els, redimensionner } = await charge({ invoke: a.invoke });

  assert.equal(els.get('fenetreTaille').textContent, '1040 × 780',
    'la taille de départ n\'est pas écrite : rien ne la dit avant le premier geste');

  await redimensionner(1500, 950);

  assert.equal(els.get('fenetreTaille').textContent, '1500 × 950',
    'la mention ne suit pas la fenêtre : elle donnerait un chiffre faux, pire que rien');
});

/* ---------- le pied ---------- */

const COMPOSITION = {
  pages: 262, chapitres: 12, gouttiere: 25, blanche: true,
  dos: 16.513, pdf: '/livres/LHC/lulu/interieur-lulu.pdf',
  polices_introuvables: [],
};

/** Ce que le pied donne à lire : le destinataire choisi, puis l'état de son dos. */
const pied = (els) => `${els.get('inDestinataire').value} ${els.get('piedDos').textContent}`.trim();
// Le témoin du dos périmé : il a quitté l'onglet Intérieur pour le pied, qui portait
// déjà le dos. Le texte et le rouge sont deux affirmations distinctes — l'un dit ce
// qu'on lit, l'autre qu'on le remarque sans le chercher.
const piedAlerte = (els) => els.get('piedDos').className === 'alerte';

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
  const { els } = await charge({ invoke: atelierCompose([LULU]) });
  await els.get('btNouveau').declenche('click');

  await faireComposer(els);

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
  return atelier({ providers: liste, destinataires: liste.map(dest), composition }).invoke;
}

/**
 * Les deux causes que le pied porte lui-même : le destinataire qu'il vise, et le papier
 * qui périme le dos sans rien changer d'autre à l'écran. Un pied qui ne repart pas sur
 * ces gestes-là dit un dos qui vaut pour un autre livre que celui qu'on regarde.
 *
 * « Périmé » et non « non composé » : les deux se ressemblaient tant que le pied n'avait
 * que trois états, et c'est précisément ce que le quatrième sépare — un livre qu'on n'a
 * jamais composé et un livre dont la mesure vient d'être périmée ne réclament pas la
 * même chose.
 */
test('viser un autre destinataire renomme le pied et lui retire le dos', async () => {
  const { els } = await charge({ invoke: atelierCompose([LULU, KDP]) });
  await els.get('btNouveau').declenche('click');
  await faireComposer(els);
  assert.equal(pied(els), 'lulu · dos 16,5 mm');

  els.get('inDestinataire').value = 'kdp-6x9';
  await els.get('inDestinataire').declenche('change');

  assert.equal(pied(els), 'kdp-6x9 · dos périmé');
});

test('changer de papier retire le dos du pied', async () => {
  const { els } = await charge({ invoke: atelierCompose([KDP]) });
  await els.get('btNouveau').declenche('click');
  await faireComposer(els);
  assert.equal(pied(els), 'kdp-6x9 · dos 16,5 mm');

  els.get('dest-papier-kdp-6x9').value = 'blanc';
  await els.get('dest-papier-kdp-6x9').declenche('change');

  assert.equal(pied(els), 'kdp-6x9 · dos périmé');
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

  await faireComposer(els);

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
 * pages en poche et en grand format. Le témoin le signale au pied, qui portait déjà le
 * dos — l'étape Intérieur, qui le portait avant, n'existe plus, et la Couverture est la
 * première à souffrir d'une mesure périmée sans qu'on ait à la quitter pour le lire.
 *
 * Le voisin de ce test lit le *texte* du pied ; celui-ci lit le *rouge*. Ce sont deux
 * affirmations : un état qu'on ne peut pas nommer et un état qu'on ne remarque pas
 * échouent différemment.
 */
test('un dos périmé par un changement de gabarit allume le témoin du pied', async () => {
  const { els } = await charge({ invoke: atelierCompose([LULU, KDP]) });
  await els.get('btNouveau').declenche('click');
  await faireComposer(els);
  assert.equal(piedAlerte(els), false, 'un dos frais ne périme rien');

  els.get('inDestinataire').value = 'kdp-6x9';
  await els.get('inDestinataire').declenche('change');

  assert.equal(piedAlerte(els), true);
});

/**
 * Le témoin dit qu'il faut réparer ; il doit donc s'éteindre quand on a réparé.
 * Recomposer est le seul geste qui rend un dos juste : un témoin qui survivrait à sa
 * réparation enverrait recomposer un livre déjà composé.
 */
test('recomposer éteint le témoin du pied', async () => {
  const { els } = await charge({ invoke: atelierCompose([LULU, KDP]) });
  await els.get('btNouveau').declenche('click');
  await faireComposer(els);
  els.get('inDestinataire').value = 'kdp-6x9';
  await els.get('inDestinataire').declenche('change');
  assert.equal(piedAlerte(els), true, 'le dos devait être périmé avant');

  await faireComposer(els);

  assert.equal(pied(els), 'kdp-6x9 · dos 16,5 mm');
  assert.equal(piedAlerte(els), false);
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
test('une police refusée n\'allume pas le témoin du pied', async () => {
  const base = atelierCompose([LULU]);
  const invoke = async (cmd, args) => {
    if (cmd === 'interieur_modifier') throw new Error('police d\'intérieur inconnue');
    return base(cmd, args);
  };
  const { els } = await charge({ invoke });
  await els.get('btNouveau').declenche('click');
  await faireComposer(els);

  els.get('inPoliceInterieur').value = 'Alegreya';
  await els.get('inPoliceInterieur').declenche('change');

  assert.match(els.get('alerte').textContent, /police d'intérieur inconnue/);
  assert.equal(els.get('inPoliceInterieur').value, 'EB Garamond',
    'le panneau n\'est pas revenu au projet : le témoin ne prouverait rien');
  assert.equal(piedAlerte(els), false);
});

/**
 * Un dos jamais composé ne réclame rien : c'est l'état d'un projet qu'on vient
 * d'ouvrir. Seul un dos qui a existé et ne vaut plus allume — et le pied, qui dit les
 * deux, doit les dire différemment.
 */
test('un dos jamais composé n\'allume pas le témoin du pied', async () => {
  const { els } = await charge({ invoke: atelierCompose([LULU, KDP]) });
  await els.get('btNouveau').declenche('click');

  els.get('inDestinataire').value = 'kdp-6x9';
  await els.get('inDestinataire').declenche('change');

  assert.equal(pied(els), 'kdp-6x9 · dos non composé');
  assert.equal(piedAlerte(els), false);
});

/**
 * Le papier périme le dos sans rien changer d'autre à l'écran : c'est le geste où un
 * témoin qui ne repartirait pas serait le plus difficile à démentir.
 */
test('changer de papier allume aussi le témoin du pied', async () => {
  const { els } = await charge({ invoke: atelierCompose([KDP]) });
  await els.get('btNouveau').declenche('click');
  await faireComposer(els);
  assert.equal(piedAlerte(els), false);

  els.get('dest-papier-kdp-6x9').value = 'blanc';
  await els.get('dest-papier-kdp-6x9').declenche('change');

  assert.equal(piedAlerte(els), true);
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
 * Ajouter n'envoie plus que le nom : la liste entière ne voyage plus, et la règle du
 * neuf — il naît comme le précédent — vit dans le Rust, où elle se teste.
 *
 * Ce que l'écran doit faire, lui, c'est **ouvrir** le neuf : il naît en fin de liste, et
 * l'y laisser fermé obligerait à le chercher parmi vingt pour lui écrire son mot.
 */
test('un envoi ajouté s\'ouvre aussitôt', async () => {
  const a = atelier({
    sur: {
      envois: {
        gabarit: '',
        liste: [{ dedicataire: 'Léa', main: { mode: 'image' }, place: PLACE_DEFAUT,
          contenu: '', image: 'Léa.png' }],
      },
    },
  });
  const { els } = await charge({ invoke: a.invoke });
  await els.get('btNouveau').declenche('click');

  els.get('inDedicataire').value = 'Marc';
  await els.get('btAjouterEnvoi').declenche('click');

  const envoi = a.appels.findLast(([c]) => c === 'envoi_ajouter');
  assert.ok(envoi, 'aucun envoi_ajouter : le bouton n\'a pas d\'écouteur');
  assert.deepEqual(envoi[1], { dedicataire: 'Marc' });

  const lignes = [...els.get('envois').children];
  assert.deepEqual(lignes.map((l) => l.textContent), ['Léa', 'Marc']);
  assert.equal(lignes[1].attrs['aria-selected'], 'true',
    'le neuf n\'est pas ouvert : il faudrait aller le chercher dans la liste');
  // Il a hérité la main de Léa : le menu montre donc l'image, pas une police.
  assert.equal(els.get('inMain').value, 'image');
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
        gabarit: '',
        liste: [{ dedicataire: 'Léa', main: { mode: 'police', police: 'Caveat' },
          place: PLACE_DEFAUT, contenu: 'À Léa.', image: null }],
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

/**
 * Le menu des mains montre celle de l'exemplaire ouvert, et change avec lui.
 *
 * Rempli une fois au démarrage, il se posait sur la première écriture de la liste
 * pendant que le livre en composait une autre — et le premier réglage de l'écran
 * l'aurait imposée. Depuis que la main appartient à l'exemplaire, le piège est pire :
 * changer de dédicataire sans que le menu suive ferait réécrire la main de Marc avec
 * celle de Léa au premier passage sur le menu.
 */
test('le menu des mains suit l\'exemplaire ouvert', async () => {
  const a = atelier({
    sur: {
      envois: {
        gabarit: '',
        liste: [
          { dedicataire: 'Léa', main: { mode: 'police', police: 'Dancing Script' },
            place: PLACE_DEFAUT, contenu: 'À Léa.', image: null },
          { dedicataire: 'Marc', main: { mode: 'image' },
            place: PLACE_DEFAUT, contenu: '', image: 'Marc.png' },
        ],
      },
    },
  });
  const { els } = await charge({ invoke: a.invoke });
  await els.get('btNouveau').declenche('click');
  assert.equal(els.get('inMain').value, 'police:Dancing Script');

  await [...els.get('envois').children][1].declenche('click');
  assert.equal(els.get('inMain').value, 'image',
    'le menu montre encore la main du dédicataire précédent');
});

/**
 * La police de l'auteur appartient au livre ouvert, pas à l'application : elle s'ajoute
 * aux mains de la maison quand le projet en porte une, et disparaît avec lui. Sans elle
 * dans le menu, la main du livre ne serait désignable par rien.
 */
test('la police personnelle s\'ajoute aux mains de la maison', async () => {
  const a = atelier({
    sur: {
      envois: {
        gabarit: '',
        personnelle: 'Ma Main',
        liste: [{ dedicataire: 'Léa', main: { mode: 'police', police: 'Ma Main' },
          place: PLACE_DEFAUT, contenu: 'À Léa.', image: null }],
      },
    },
  });
  const { els } = await charge({ invoke: a.invoke });
  await els.get('btNouveau').declenche('click');

  const offertes = [...els.get('inMain').children].map((o) => o.value);
  assert.deepEqual(offertes,
    ['police:Caveat', 'police:Dancing Script', 'police:Ma Main', 'image', 'diffusion']);
  assert.equal(els.get('inMain').value, 'police:Ma Main');
  assert.equal(els.get('btPoliceRetirer').disabled, false,
    'une police est embarquée et rien ne la retire');
});

/**
 * Sans police personnelle, il n'y a rien à retirer. Le bouton l'est déjà dans le HTML :
 * ce que ce test garde, c'est qu'il le redevienne en ouvrant un livre qui n'en porte
 * pas, après un livre qui en portait une.
 */
test('le retrait de police s\'éteint avec le livre qui portait la police', async () => {
  const avec = {
    gabarit: '',
    personnelle: 'Ma Main',
    liste: [{ dedicataire: 'Léa', main: { mode: 'police', police: 'Ma Main' },
      place: PLACE_DEFAUT, contenu: 'À Léa.', image: null }],
  };
  const a = atelier({ sur: { envois: avec } });
  const { els, contexte } = await charge({ invoke: a.invoke });
  await els.get('btNouveau').declenche('click');
  assert.equal(els.get('btPoliceRetirer').disabled, false);

  contexte.afficherProjet(projet());
  assert.equal(els.get('btPoliceRetirer').disabled, true,
    'le livre suivant n\'a pas de police et le bouton reste allumé');
});

/**
 * Le fichier est copié dans le `.ozalid` : c'est le Rust qui le lit, à partir du chemin
 * choisi. Un dialogue annulé ne doit rien envoyer — sans quoi la commande partirait avec
 * un chemin vide et l'erreur remonterait à l'écran pour un geste que personne n'a fait.
 */
test('choisir une police envoie son chemin, et l\'annuler n\'envoie rien', async () => {
  const a = atelier();
  let repond = '/polices/ma-main.ttf';
  const { els } = await charge({ invoke: a.invoke, open: async () => repond });
  await els.get('btNouveau').declenche('click');

  await els.get('btPolice').declenche('click');
  const choix = a.appels.findLast(([c]) => c === 'police_choisir');
  assert.ok(choix, 'aucun police_choisir : le bouton n\'a pas d\'écouteur');
  assert.equal(choix[1].chemin, '/polices/ma-main.ttf');

  repond = null;
  await els.get('btPolice').declenche('click');
  assert.equal(a.appels.filter(([c]) => c === 'police_choisir').length, 1,
    'un dialogue annulé a quand même envoyé un chemin');
});

/**
 * La main est une forme autant qu'une écriture : le menu porte les deux, et le mode
 * choisi doit repartir tel quel. Envoyer une police là où l'on a choisi une image ferait
 * composer un texte vide sur la page de l'exemplaire.
 *
 * Et il ne part que sur **cet** exemplaire : c'est tout l'objet du chantier — écrire à
 * la main pour l'une et faire composer pour l'autre. Le voisin ne doit pas bouger.
 */
test('changer la main ne touche que l\'exemplaire ouvert', async () => {
  const a = atelier({
    sur: {
      envois: {
        gabarit: '',
        liste: [
          { dedicataire: 'Léa', main: { mode: 'police', police: 'Caveat' },
            place: PLACE_DEFAUT, contenu: 'À Léa.', image: null },
          { dedicataire: 'Marc', main: { mode: 'police', police: 'Caveat' },
            place: PLACE_DEFAUT, contenu: 'À Marc.', image: null },
        ],
      },
    },
  });
  const { els } = await charge({ invoke: a.invoke });
  await els.get('btNouveau').declenche('click');

  els.get('inMain').value = 'image';
  await els.get('inMain').declenche('change');

  const envoi = a.appels.findLast(([c]) => c === 'envoi_regler');
  assert.ok(envoi, 'aucun envoi_regler : le menu n\'a pas d\'écouteur');
  assert.equal(envoi[1].index, 0, 'la main est partie sur le mauvais exemplaire');
  assert.deepEqual(envoi[1].envoi.main, { mode: 'image' });
  // Le mot de Léa la suit : c'est l'envoi entier qui repart, seule sa main a changé.
  assert.equal(envoi[1].envoi.contenu, 'À Léa.');

  await [...els.get('envois').children][1].declenche('click');
  assert.equal(els.get('inMain').value, 'police:Caveat',
    'Marc a changé de main alors qu\'on réglait Léa');
});

/**
 * Une image générée n'emporte plus son gabarit : celui-ci est au livre, partagé, et il a
 * sa propre commande. L'envoyer dans la main ferait réécrire le prompt du tirage entier
 * à chaque exemplaire qu'on passe en images.
 */
test('choisir l\'image générée n\'emporte pas le gabarit', async () => {
  const a = atelier({
    sur: {
      envois: {
        gabarit: 'une aquarelle, mention « {envoi} »',
        liste: [{ dedicataire: 'Léa', main: { mode: 'police', police: 'Caveat' },
          place: PLACE_DEFAUT, contenu: 'À Léa.', image: null }],
      },
    },
  });
  const { els } = await charge({ invoke: a.invoke });
  await els.get('btNouveau').declenche('click');

  els.get('inMain').value = 'diffusion';
  await els.get('inMain').declenche('change');

  const envoi = a.appels.findLast(([c]) => c === 'envoi_regler');
  assert.deepEqual(envoi[1].envoi.main, { mode: 'diffusion' });
  assert.equal(els.get('inGabarit').value, 'une aquarelle, mention « {envoi} »',
    'le gabarit du livre a été perdu en changeant de main');
});

/**
 * Les réglages suivent la main de l'exemplaire ouvert : sous une main en images, il n'y
 * a pas de mot à écrire mais une image à choisir. Laisser le champ de texte donnerait à
 * croire qu'on peut encore y écrire, alors que rien de ce qu'on y taperait ne serait
 * imprimé.
 */
test('les réglages suivent la main de l\'exemplaire ouvert', async () => {
  const en_images = {
    gabarit: '',
    liste: [{ dedicataire: 'Léa', main: { mode: 'image' }, place: PLACE_DEFAUT,
      contenu: '', image: 'Léa.png' }],
  };
  const a = atelier({ sur: { envois: en_images } });
  const { els } = await charge({ invoke: a.invoke, open: async () => '/photos/mot.png' });
  await els.get('btNouveau').declenche('click');

  assert.equal(els.get('champMot').hidden, true, 'un champ de texte sous une main en images');
  assert.equal(els.get('champImage').hidden, false, 'rien pour choisir l\'image');
  assert.equal(els.get('champDiffusion').hidden, true);
  assert.equal(els.get('btImageEnvoi').textContent, 'Image : Léa.png',
    'le nom de l\'image dans l\'archive n\'est pas montré');

  await els.get('btImageEnvoi').declenche('click');
  const choix = a.appels.findLast(([c]) => c === 'envoi_image_choisir');
  assert.ok(choix, 'aucun envoi_image_choisir : le bouton n\'a pas d\'écouteur');
  assert.deepEqual(choix[1], { index: 0, chemin: '/photos/mot.png' });
});

/**
 * Cliquer une vignette déplace l'envoi sur cette page.
 *
 * C'est le **seul** moyen d'en changer — il n'y a pas de champ « page » —, et c'est
 * pourquoi il se garde : sans écouteur, le rail serait une frise décorative et l'envoi
 * resterait à jamais sur sa page de titre.
 */
test('cliquer une vignette déplace l\'envoi sur cette page', async () => {
  const a = atelier({
    sur: {
      envois: {
        gabarit: '',
        liste: [{ dedicataire: 'Léa', main: { mode: 'police', police: 'Caveat' },
          place: PLACE_DEFAUT, contenu: 'À Léa.', image: null }],
      },
    },
  });
  const { els } = await charge({ invoke: a.invoke });
  await els.get('btNouveau').declenche('click');
  await allerAuxEnvois(els);

  const vignettes = [...els.get('vignettes').children];
  assert.equal(vignettes.length, 4, 'le rail ne montre pas les pages rendues');
  assert.equal(vignettes[2].attrs['aria-current'], 'true',
    'la page visée n\'est pas marquée : on ne saurait pas où est l\'envoi');

  await vignettes[0].declenche('click');

  const regle = a.appels.findLast(([c]) => c === 'envoi_regler');
  assert.equal(regle[1].envoi.place.page, 1, 'la vignette n\'a pas déplacé l\'envoi');
  // Le reste du placement ne bouge pas : changer de page n'est pas repartir de zéro.
  assert.equal(regle[1].envoi.place.taille, PLACE_DEFAUT.taille);
  assert.equal(regle[1].envoi.place.angle, PLACE_DEFAUT.angle);
  const page = a.appels.findLast(([c]) => c === 'envoi_page');
  assert.deepEqual(page[1], { page: 1 }, 'le canevas montre encore l\'ancienne page');
});

/**
 * Une recomposition change les pages : les vignettes d'avant montreraient un livre qui
 * n'existe plus, et l'on placerait un envoi page 40 d'une pagination périmée.
 */
test('recomposer refait le rail', async () => {
  const a = atelier({
    composition: { pages: 100, gouttiere: 20, blanche: false, dos: 7 },
    sur: {
      envois: {
        gabarit: '',
        liste: [{ dedicataire: 'Léa', main: { mode: 'police', police: 'Caveat' },
          place: PLACE_DEFAUT, contenu: 'À Léa.', image: null }],
      },
    },
  });
  const { els } = await charge({ invoke: a.invoke });
  await els.get('btNouveau').declenche('click');
  await allerAuxEnvois(els);
  const avant = a.appels.filter(([c]) => c === 'envoi_vignettes').length;
  assert.ok(avant > 0, 'le rail ne s\'est jamais rendu');

  // Recomposer depuis le Livre, puis revenir : c'est le chemin réel, et c'est en
  // revenant que le rail doit se refaire.
  await faireComposer(els);
  await allerAuxEnvois(els);

  assert.ok(a.appels.filter(([c]) => c === 'envoi_vignettes').length > avant,
    'le rail garde les vignettes d\'avant la recomposition');
});

/**
 * Le pied vit sous l'étape : on change de destinataire **sans la quitter**, et c'est
 * le seul chemin qui change la pagination pendant qu'on la regarde.
 *
 * Oublier les vignettes ne suffit pas alors : le cache se vide, mais celles d'avant
 * restent à l'écran. On viserait les pages d'un tirage qui n'est plus celui du pied —
 * page 264 d'un intérieur qui n'en fait plus que 190 —, et seul le refus à la
 * génération le dirait, une fois le mot écrit.
 */
test('changer de destinataire au pied refait le rail sans quitter l\'étape', async () => {
  const a = atelier({
    providers: [LULU, KDP],
    destinataires: [LULU, KDP].map(dest),
    composition: { pages: 100, gouttiere: 20, blanche: false, dos: 7 },
    sur: {
      envois: {
        gabarit: '',
        liste: [{ dedicataire: 'Léa', main: { mode: 'police', police: 'Caveat' },
          place: PLACE_DEFAUT, contenu: 'À Léa.', image: null }],
      },
    },
  });
  const { els } = await charge({ invoke: a.invoke });
  await els.get('btNouveau').declenche('click');
  await allerAuxEnvois(els);
  const avant = a.appels.filter(([c]) => c === 'envoi_vignettes').length;
  assert.ok(avant > 0, 'le rail ne s\'est jamais rendu');

  els.get('inDestinataire').value = 'kdp-6x9';
  await els.get('inDestinataire').declenche('change');
  await new Promise((r) => setImmediate(r));

  assert.ok(a.appels.filter(([c]) => c === 'envoi_vignettes').length > avant,
    'le rail montre encore les pages du destinataire d\'avant');
});

/**
 * Revenir à un destinataire **déjà composé** ne recompose rien : sa mesure est là. Le
 * rail n'a donc aucune recomposition à laquelle s'accrocher, et pourtant il montre les
 * pages de l'autre — deux paginations n'ont ni le même nombre de pages ni la même
 * gouttière. C'est le changement de visée qui périme les vignettes, pas la composition.
 */
test('revenir à un destinataire déjà composé refait aussi le rail', async () => {
  const a = atelier({
    providers: [LULU, KDP],
    destinataires: [LULU, KDP].map(dest),
    composition: { pages: 100, gouttiere: 20, blanche: false, dos: 7 },
    sur: {
      envois: {
        gabarit: '',
        liste: [{ dedicataire: 'Léa', main: { mode: 'police', police: 'Caveat' },
          place: PLACE_DEFAUT, contenu: 'À Léa.', image: null }],
      },
    },
  });
  const { els } = await charge({ invoke: a.invoke });
  await els.get('btNouveau').declenche('click');
  await faireComposer(els);
  els.get('inDestinataire').value = 'kdp-6x9';
  await els.get('inDestinataire').declenche('change');
  await faireComposer(els);
  await allerAuxEnvois(els);
  const avant = a.appels.filter(([c]) => c === 'envoi_vignettes').length;
  assert.ok(avant > 0, 'le rail ne s\'est jamais rendu');

  // Lulu est composé : rien ne repagine, et c'est tout le sujet.
  els.get('inDestinataire').value = 'lulu';
  await els.get('inDestinataire').declenche('change');
  await new Promise((r) => setImmediate(r));

  assert.equal(a.appels.filter(([c]) => c === 'composer').length, 2,
    'le décor a bougé : ce retour a recomposé, il ne prouve plus rien');
  assert.ok(a.appels.filter(([c]) => c === 'envoi_vignettes').length > avant,
    'le rail garde les pages de l\'autre destinataire');
});

/**
 * Le canevas se dimensionne par le rapport de la page, comme le cadre de l'aperçu de
 * couverture et pour la même raison : sans lui, il ne tient sa taille que de sa
 * largeur, et une fenêtre large lui donne une page plus haute que la bande. L'étape
 * ne défilant pas, le bas de la page — donc l'envoi, qui s'y pose — passe sous le
 * bord et devient inatteignable.
 */
test('le canevas prend le rapport de la page qu\'il montre', async () => {
  const a = atelier({
    sur: {
      envois: {
        gabarit: '',
        liste: [{ dedicataire: 'Léa', main: { mode: 'police', police: 'Caveat' },
          place: PLACE_DEFAUT, contenu: 'À Léa.', image: null }],
      },
    },
  });
  const { els } = await charge({ invoke: a.invoke });
  await els.get('btNouveau').declenche('click');
  await allerAuxEnvois(els);

  const fond = els.get('fondPage');
  // Une page Lulu poche : 108 × 175 mm à 150 ppi.
  fond.naturalWidth = 638;
  fond.naturalHeight = 1033;
  await fond.declenche('load');

  assert.strictEqual(
    els.get('canevas').style.getPropertyValue('--ratio'), String(638 / 1033)
  );
});

/**
 * Un canevas qui garderait son rapport sans page garderait sa place : l'établi seul,
 * un rectangle sombre et vide au milieu de l'étape, là où il n'y a rien à montrer.
 * Même règle que le cadre de l'aperçu de couverture, et pour la même raison.
 */
test('le dernier envoi retiré emporte le rapport du canevas', async () => {
  const a = atelier({
    sur: {
      envois: {
        gabarit: '',
        liste: [{ dedicataire: 'Léa', main: { mode: 'police', police: 'Caveat' },
          place: PLACE_DEFAUT, contenu: 'À Léa.', image: null }],
      },
    },
  });
  const { els } = await charge({ invoke: a.invoke });
  await els.get('btNouveau').declenche('click');
  await allerAuxEnvois(els);
  const fond = els.get('fondPage');
  fond.naturalWidth = 638;
  fond.naturalHeight = 1033;
  await fond.declenche('load');

  await els.get('btRetirerEnvoi').declenche('click');
  await new Promise((r) => setImmediate(r));

  assert.strictEqual(els.get('canevas').style.getPropertyValue('--ratio'), '');
});

/**
 * « Voir la page » est la confirmation, pas un second aperçu : la page composée prend
 * la place du canevas, et le bouton ramène. C'est ce va-et-vient qui la rend utile —
 * l'objet ne doit pas bouger d'un pouce entre les deux images, et c'est la seule
 * manière de le voir à l'œil.
 *
 * Les montrer ensemble ne tient pas : la bande n'a la hauteur que d'une page, et la
 * confirmation posée par-dessus recouvrait le canevas et son bouton, sans rien pour
 * la refermer.
 */
test('la page composée prend la place du canevas, et le bouton ramène', async () => {
  const a = atelier({
    sur: {
      envois: {
        gabarit: '',
        liste: [{ dedicataire: 'Léa', main: { mode: 'police', police: 'Caveat' },
          place: PLACE_DEFAUT, contenu: 'À Léa.', image: null }],
      },
    },
  });
  const { els } = await charge({ invoke: a.invoke });
  await els.get('btNouveau').declenche('click');
  await allerAuxEnvois(els);

  await els.get('btVoirPage').declenche('click');
  assert.equal(els.get('apercuEnvoi').hidden, false, 'la page composée ne paraît pas');
  assert.equal(els.get('canevas').hidden, true,
    'les deux pages se superposent : la confirmation recouvre le canevas');

  await els.get('btVoirPage').declenche('click');
  assert.equal(els.get('apercuEnvoi').hidden, true, 'rien ne referme la confirmation');
  assert.equal(els.get('canevas').hidden, false, 'le canevas ne revient pas');
});

/**
 * Le dos du compte rendu s'écrit comme celui du pied.
 *
 * Deux écritures d'un même millimètre dans une même fenêtre — « 16,51 » au pied et
 * « 16.51 » deux centimètres à droite — donnent à croire à deux mesures. C'est la
 * langue de l'interface qui tranche, et `nb` la porte depuis le début.
 */
test('le compte rendu d\'un envoi écrit son dos à la française', async () => {
  const a = atelier({
    sur: {
      envois: {
        gabarit: '',
        liste: [{ dedicataire: 'Léa', main: { mode: 'police', police: 'Caveat' },
          place: PLACE_DEFAUT, contenu: 'À Léa.', image: null }],
      },
    },
  });
  const { els } = await charge({ invoke: a.invoke });
  await els.get('btNouveau').declenche('click');
  await allerAuxEnvois(els);

  await els.get('btEnvoyer').declenche('click');
  await new Promise((r) => setImmediate(r));

  const rendu = els.get('resultatEnvois').textContent;
  assert.ok(rendu.includes('dos 16,51 mm'), `dos écrit à l'anglaise : ${rendu}`);
});

/**
 * La confirmation est une image figée : elle vaut pour la page et l'exemplaire d'où
 * elle sort. Déplacer l'envoi pendant qu'elle est à l'écran la laisserait confirmer une
 * page qu'on vient de quitter — et c'est le canevas, désormais caché derrière elle, qui
 * dirait la vérité.
 */
test('déplacer l\'envoi referme la confirmation', async () => {
  const a = atelier({
    sur: {
      envois: {
        gabarit: '',
        liste: [{ dedicataire: 'Léa', main: { mode: 'police', police: 'Caveat' },
          place: PLACE_DEFAUT, contenu: 'À Léa.', image: null }],
      },
    },
  });
  const { els } = await charge({ invoke: a.invoke });
  await els.get('btNouveau').declenche('click');
  await allerAuxEnvois(els);
  await els.get('btVoirPage').declenche('click');
  assert.equal(els.get('apercuEnvoi').hidden, false, 'la confirmation devait être là');

  await [...els.get('vignettes').children][0].declenche('click');
  await new Promise((r) => setImmediate(r));

  assert.equal(els.get('apercuEnvoi').hidden, true,
    'la confirmation montre encore la page d\'avant le déplacement');
  assert.equal(els.get('canevas').hidden, false, 'le canevas reste caché derrière elle');
});

/* ---------- l'image générée ---------- */

const EN_DIFFUSION = {
  gabarit: 'une aquarelle, mention « {envoi} »',
  liste: [{ dedicataire: 'Léa', main: { mode: 'diffusion' }, place: PLACE_DEFAUT,
    contenu: 'À Léa', image: null }],
};

/**
 * Le gabarit appartient au livre et a sa propre commande : c'est le style d'écriture du
 * tirage, dans lequel le mot de chacun s'insère, et le faire voyager avec la main d'un
 * exemplaire le ferait réécrire à chaque personne.
 */
test('le gabarit a sa commande, et ne passe pas par un envoi', async () => {
  const a = atelier({ sur: { envois: EN_DIFFUSION } });
  const { els } = await charge({ invoke: a.invoke });
  await els.get('btNouveau').declenche('click');

  assert.equal(els.get('inGabarit').value, 'une aquarelle, mention « {envoi} »');
  els.get('inGabarit').value = 'une gravure, mention « {envoi} »';
  await els.get('inGabarit').declenche('change');

  const envoi = a.appels.findLast(([c]) => c === 'envois_gabarit');
  assert.ok(envoi, 'aucun envois_gabarit : le champ n\'a pas d\'écouteur');
  assert.deepEqual(envoi[1], { gabarit: 'une gravure, mention « {envoi} »' });
  assert.ok(!a.noms().includes('envoi_regler'),
    'le gabarit est passé par un envoi : il serait réécrit pour chaque personne');
});

/**
 * **La clé ne redescend jamais.** Elle est en clair dans `preferences.toml`, avec les
 * permissions du fichier ; la poser dans un champ la ferait entrer dans une capture
 * d'écran. L'écran ne sait donc que deux choses : l'adresse, et qu'une clé est là.
 */
test('la clé du modèle n\'est jamais rendue à l\'écran', async () => {
  const a = atelier({ acces: { url: 'https://exemple.test/images', cle_posee: true } });
  const { els } = await charge({ invoke: a.invoke });

  assert.equal(els.get('inDiffusionUrl').value, 'https://exemple.test/images');
  assert.equal(els.get('inDiffusionCle').value, '', 'une clé est arrivée à l\'écran');
  assert.match(els.get('etatDiffusion').textContent, /clé enregistrée/);
});

/**
 * Corriger l'adresse ne doit pas effacer la clé : le champ est vide à l'écran puisqu'on
 * ne la redonne jamais, et l'envoyer telle quelle l'effacerait à chaque correction.
 * L'oubli, lui, se demande — et il envoie une clé vide, pas une absence.
 */
test('une clé non ressaisie est laissée en place, et l\'oubli se demande', async () => {
  const a = atelier({ acces: { url: 'https://exemple.test/images', cle_posee: true } });
  const { els } = await charge({ invoke: a.invoke });

  els.get('inDiffusionUrl').value = 'https://autre.test/images';
  await els.get('btDiffusionRegler').declenche('click');
  const regle = a.appels.findLast(([c]) => c === 'diffusion_regler');
  assert.deepEqual(regle[1], { url: 'https://autre.test/images', cle: null });

  await els.get('btDiffusionOublier').declenche('click');
  const oubli = a.appels.findLast(([c]) => c === 'diffusion_regler');
  assert.equal(oubli[1].cle, '', 'oublier la clé ne l\'efface pas');
});

/**
 * Retenir est le geste qui fige l'image dans le livre : il n'a pas d'objet tant qu'on
 * n'a rien vu. Vérifier qu'il est éteint au départ ne prouverait rien — c'est
 * l'allumage, après génération, qui se garde.
 */
test('retenir une image s\'allume quand le modèle a répondu', async () => {
  const a = atelier({ sur: { envois: EN_DIFFUSION } });
  const { els } = await charge({ invoke: a.invoke });
  await els.get('btNouveau').declenche('click');
  assert.equal(els.get('btAccepter').disabled, true);

  await els.get('btGenerer').declenche('click');

  assert.equal(els.get('apercuEnvoi').src, 'data:image/png;base64,QUJD',
    'l\'image proposée n\'est pas montrée');
  assert.equal(els.get('btAccepter').disabled, false,
    'le modèle a répondu et rien ne permet de retenir son image');

  await els.get('btAccepter').declenche('click');
  const accepte = a.appels.findLast(([c]) => c === 'envoi_accepter');
  assert.deepEqual(accepte[1], { index: 0 });
});

/**
 * Une image proposée appartient au livre pour lequel on l'a demandée. Le Rust l'oublie
 * en posant le suivant ; si l'écran ne l'oubliait pas aussi, « Retenir » proposerait de
 * figer dans le livre B une image demandée pour le livre A.
 */
test('une image proposée ne survit pas au livre suivant', async () => {
  const a = atelier({ sur: { envois: EN_DIFFUSION } });
  const { els } = await charge({ invoke: a.invoke });
  await els.get('btNouveau').declenche('click');
  await els.get('btGenerer').declenche('click');

  await els.get('btNouveau').declenche('click');

  assert.equal(els.get('btAccepter').disabled, true,
    'le livre suivant hérite de l\'image proposée pour le précédent');
});

/**
 * Glisser l'objet déplace l'envoi.
 *
 * C'est le seul des trois gestes du canevas qui ne se rattrape nulle part : l'échelle
 * et l'inclinaison ont chacune leur champ dans les réglages, la position n'en a aucun.
 * Un déplacement qui ne prend pas laisse donc l'envoi au bas de la page de titre, sans
 * recours — et l'étape paraît entière, puisque les deux poignées répondent.
 *
 * Le geste se saisit sur `#objet`, non sur l'image qu'il contient : voir la garde
 * `styles.css → envois.js` de `contrats.test.js`, qui dit pourquoi.
 */
test('glisser l\'objet déplace l\'envoi', async () => {
  const a = atelier({
    sur: {
      envois: {
        gabarit: '',
        liste: [{ dedicataire: 'Léa', main: { mode: 'police', police: 'Caveat' },
          place: PLACE_DEFAUT, contenu: 'À Léa.', image: null }],
      },
    },
  });
  const { els } = await charge({ invoke: a.invoke });
  await els.get('btNouveau').declenche('click');
  await allerAuxEnvois(els);

  // Le faux DOM ne met rien en page : sans boîte, le geste se refuse — comme il le fait
  // dans la fenêtre devant un canevas qui n'est pas encore affiché. Deux pixels par
  // millimètre sur une poche Lulu de 108 × 175, ce qui rend les comptes lisibles.
  els.get('canevas').rect = { left: 0, top: 0, width: 216, height: 350 };

  const objet = els.get('objet');
  await objet.declenche('pointerdown', souris(100, 100));
  await objet.declenche('pointermove', souris(154, 100));
  await objet.declenche('pointerup', souris(154, 100));
  await new Promise((r) => setImmediate(r));

  const regle = a.appels.findLast(([c]) => c === 'envoi_regler');
  assert.ok(regle, 'le geste n\'est jamais parti : l\'objet ne se saisit pas');
  // 54 px sur 216 : un quart de la largeur de page. La fraction et non les pixels —
  // c'est ce qui fait qu'un canevas plus petit montre le même placement.
  assert.equal(regle[1].envoi.place.x, PLACE_DEFAUT.x + 0.25,
    'le déplacement ne se compte pas en fraction du canevas');
  assert.equal(regle[1].envoi.place.y, PLACE_DEFAUT.y,
    'l\'envoi a dérivé en hauteur alors que le geste était horizontal');
});
