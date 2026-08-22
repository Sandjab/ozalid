'use strict';

/**
 * Schéma des réglages de couverture.
 *
 * Le panneau est construit à partir d'ici plutôt qu'écrit à la main : soixante
 * contrôles recopiés dans le HTML dérivent immanquablement du modèle Rust. Le chemin
 * est celui du champ dans l'objet `Couverture` sérialisé — s'il est faux, le contrôle
 * reste vide au chargement, ce qui se voit tout de suite.
 *
 * Rappel qui vaut pour tout ce fichier : les tailles et écarts sont en pourcentage de
 * la largeur de couverture, jamais en mm. C'est ce qui rend la maquette portable d'un
 * format à l'autre, donc le choix du prestataire repoussable à la fin.
 */

const CASSES = [['telle', 'Telle quelle'], ['capitales', 'Capitales']];

/** Les voiles de lisibilité, offerts partout où une image passe sous du texte. */
const VOILES = [
  ['aucun', 'Aucun'], ['haut', 'Haut'], ['bas', 'Bas'], ['deux', 'Haut et bas'],
  ['uni', 'Uni sombre'], ['clair', 'Uni clair'],
];

/** Les cinq réglages d'un cadrage d'image, sous un préfixe de chemin donné. */
function cadrage(prefixe) {
  const c = (suffixe, l, reste) => ({ chemin: `${prefixe}.${suffixe}`, libelle: l, ...reste });
  return [
    c('proportions', 'Proportions conservées', { type: 'case' }),
    c('x', 'Ancrage horizontal', { type: 'nombre', min: 0, max: 1, pas: 0.01 }),
    c('y', 'Ancrage vertical', { type: 'nombre', min: 0, max: 1, pas: 0.01 }),
    c('zoom', 'Zoom', { type: 'nombre', min: 0.2, max: 4, pas: 0.01 }),
    c('etirement', 'Déformation', { type: 'nombre', min: 0.5, max: 2, pas: 0.01 }),
  ];
}

/** Les six contrôles d'un style de texte, sous un préfixe de chemin donné. */
function style(prefixe, libelle, opts = {}) {
  const c = (suffixe, l, reste) => ({ chemin: `${prefixe}.${suffixe}`, libelle: l, ...reste });
  const champs = [
    c('police', 'Police', { type: 'polices' }),
    c('graisse', 'Graisse', { type: 'nombre', min: 100, max: 900, pas: 100 }),
    c('taille', 'Corps', { type: 'nombre', min: 0.5, max: 20, pas: 0.1, unite: '% larg.' }),
    c('couleur', 'Couleur', { type: 'couleur' }),
    c('tracking', 'Interlettrage', { type: 'nombre', min: -10, max: 30, pas: 0.5, unite: '/100 em' }),
  ];
  if (opts.casse !== false) champs.push(c('casse', 'Casse', { type: 'liste', options: CASSES }));
  if (opts.italique) champs.push(c('italique', 'Italique', { type: 'case' }));
  return { titre: libelle, champs };
}

/** Les trois places du dos, du début de la lecture — de bas en haut — vers la fin. */
const PLACES_DOS = [['pied', 'Pied'], ['centre', 'Centre'], ['tete', 'Tête']];

/**
 * Un élément du dos : où il se place, dans quel ordre, et son style entier.
 *
 * Les trois éléments ont exactement les mêmes réglages — le dos d'une collection met
 * le titre en tête et l'auteur au pied, celui d'une autre les groupe : rien ici ne
 * doit privilégier un usage.
 */
function elementDos(cle, libelle) {
  return {
    ...style(`dos.${cle}.style`, `Dos — ${libelle}`),
    face: 'dos',
    avant: [
      { chemin: `dos.${cle}.actif`, libelle: 'Afficher', type: 'case' },
      { chemin: `dos.${cle}.place`, libelle: 'Position', type: 'liste', options: PLACES_DOS },
      { chemin: `dos.${cle}.rang`, libelle: 'Ordre à cette position', type: 'nombre', min: 1, max: 9, pas: 1 },
    ],
  };
}

const SCHEMA = [
  {
    titre: 'Page',
    champs: [
      {
        chemin: 'mode', libelle: 'Mode', type: 'liste', options: [
          ['bandeau', 'Bandeau'], ['surimpression', 'Surimpression'], ['typo', 'Sans image'],
        ],
      },
      { chemin: 'papier', libelle: 'Papier', type: 'couleur' },
      {
        chemin: 'align', libelle: 'Alignement', type: 'liste', options: [
          ['gauche', 'Gauche'], ['centre', 'Centre'], ['droite', 'Droite'],
        ],
      },
      { chemin: 'pad_x', libelle: 'Marge latérale', type: 'nombre', min: 0, max: 40, pas: 0.5, unite: '% larg.' },
      { chemin: 'bandeau', libelle: 'Hauteur du bandeau', type: 'nombre', min: 5, max: 70, pas: 0.5, unite: '% haut.', modes: ['bandeau'] },
      { chemin: 'bandeau_retrait', libelle: 'Image en retrait', type: 'case', modes: ['bandeau'] },
      { chemin: 'bloc_y', libelle: 'Hauteur du bloc titre', type: 'nombre', min: 0, max: 80, pas: 0.5, unite: '% haut.', modes: ['surimpression', 'typo'] },
    ],
  },
  {
    titre: 'Cadre',
    champs: [
      { chemin: 'cadre.actif', libelle: 'Afficher le cadre', type: 'case' },
      { chemin: 'cadre.marge', libelle: 'Marge du cadre', type: 'nombre', min: 0, max: 30, pas: 0.5, unite: '%' },
      { chemin: 'cadre.filet1_couleur', libelle: 'Filet externe', type: 'couleur' },
      { chemin: 'cadre.filet1_epaisseur', libelle: 'Épaisseur externe', type: 'nombre', min: 0.05, max: 2, pas: 0.05, unite: '% larg.' },
      { chemin: 'cadre.decroche', libelle: 'Décroché', type: 'nombre', min: 0, max: 12, pas: 0.1, unite: '% larg.' },
      { chemin: 'cadre.filet2_couleur', libelle: 'Filets internes', type: 'couleur' },
      { chemin: 'cadre.filet2_epaisseur', libelle: 'Épaisseur interne', type: 'nombre', min: 0.05, max: 2, pas: 0.05, unite: '% larg.' },
      { chemin: 'cadre.ecart', libelle: 'Écart des deux filets', type: 'nombre', min: 0, max: 6, pas: 0.1, unite: '% larg.' },
    ],
  },
  style('auteur', 'Auteur'),
  {
    ...style('titre', 'Titre'),
    apres: [
      { chemin: 'titre_interligne', libelle: 'Interligne', type: 'nombre', min: 0.8, max: 2, pas: 0.01 },
      { chemin: 'titre_ecart', libelle: 'Écart auteur → titre', type: 'nombre', min: 0, max: 30, pas: 0.5, unite: '% larg.' },
    ],
  },
  {
    ...style('genre', 'Genre', { casse: false }),
    avant: [{ chemin: 'genre_visible', libelle: 'Afficher le genre', type: 'case' }],
    apres: [{ chemin: 'genre_ecart', libelle: 'Écart titre → genre', type: 'nombre', min: 0, max: 30, pas: 0.5, unite: '% larg.' }],
  },
  {
    titre: 'Pied éditeur',
    champs: [
      { chemin: 'pied.actif', libelle: 'Afficher le pied', type: 'case' },
      { chemin: 'pied.monogramme', libelle: 'Monogramme', type: 'texte' },
      { chemin: 'pied.editeur', libelle: 'Éditeur', type: 'texte' },
      { chemin: 'pied.y', libelle: 'Hauteur depuis le bas', type: 'nombre', min: 0, max: 50, pas: 0.5, unite: '% haut.' },
    ],
  },
  style('pied.style_mono', 'Monogramme', { casse: false, italique: true }),
  style('pied.style_editeur', 'Mention éditeur', { casse: false }),
  {
    titre: 'Pastille',
    champs: [
      { chemin: 'pastille.actif', libelle: 'Afficher la pastille', type: 'case' },
      { chemin: 'pastille.texte', libelle: 'Texte', type: 'texte' },
      { chemin: 'pastille.fond', libelle: 'Fond', type: 'couleur' },
      {
        chemin: 'pastille.coin', libelle: 'Coin', type: 'liste', options: [
          ['bas-droite', 'Bas droite'], ['bas-gauche', 'Bas gauche'],
          ['haut-droite', 'Haut droite'], ['haut-gauche', 'Haut gauche'],
        ],
      },
      { chemin: 'pastille.verticale', libelle: 'Verticale', type: 'case' },
      { chemin: 'pastille.arrondie', libelle: 'Coins arrondis', type: 'case' },
      { chemin: 'pastille.dx', libelle: 'Décalage horizontal', type: 'nombre', min: 0, max: 30, pas: 0.5, unite: '% larg.' },
      { chemin: 'pastille.dy', libelle: 'Décalage vertical', type: 'nombre', min: 0, max: 30, pas: 0.5, unite: '% larg.' },
    ],
  },
  style('pastille.style', 'Texte de pastille', { casse: false }),
  {
    titre: 'Image',
    modes: ['bandeau', 'surimpression'],
    champs: [
      ...cadrage('cadrage'),
      { chemin: 'voile', libelle: 'Voile de lisibilité', type: 'liste', options: VOILES },
      { chemin: 'voile_opacite', libelle: 'Opacité du voile', type: 'nombre', min: 0, max: 1, pas: 0.01 },
    ],
  },
  {
    titre: '4ème — fond et texte',
    face: 'quatre',
    champs: [
      {
        chemin: 'quatrieme.fond', libelle: 'Fond', type: 'liste', options: [
          ['herite', 'Papier de la 1ère'], ['couleur', 'Couleur distincte'],
          ['image', 'Image propre'], ['panorama', 'Prolongement de la 1ère'],
        ],
      },
      { chemin: 'quatrieme.couleur', libelle: 'Couleur du fond', type: 'couleur' },
      { chemin: 'quatrieme.texte', libelle: 'Texte de présentation', type: 'zone' },
      { chemin: 'quatrieme.interligne', libelle: 'Interligne', type: 'nombre', min: 1, max: 2.5, pas: 0.05 },
      {
        chemin: 'quatrieme.align', libelle: 'Alignement', type: 'liste', options: [
          ['gauche', 'Gauche'], ['centre', 'Centre'], ['droite', 'Droite'],
        ],
      },
      { chemin: 'quatrieme.pad_x', libelle: 'Marge latérale', type: 'nombre', min: 0, max: 40, pas: 0.5, unite: '% larg.' },
      { chemin: 'quatrieme.top', libelle: 'Hauteur du texte', type: 'nombre', min: 0, max: 60, pas: 0.5, unite: '% larg.' },
    ],
  },
  { ...style('quatrieme.style', '4ème — style du texte', { casse: false }), face: 'quatre' },
  {
    // La 4ème a son image, son cadrage et son voile, distincts de ceux de la 1ère :
    // les deux faces ne montrent pas la même chose et ne se recadrent pas ensemble.
    // Offerts sans condition, comme la zone ISBN : le fond qui les emploie se change
    // juste au-dessus, et un panneau qui apparaît et disparaît se cherche.
    titre: '4ème — image et voile',
    face: 'quatre',
    champs: [
      ...cadrage('quatrieme.cadrage'),
      { chemin: 'quatrieme.voile', libelle: 'Voile de lisibilité', type: 'liste', options: VOILES },
      { chemin: 'quatrieme.voile_opacite', libelle: 'Opacité du voile', type: 'nombre', min: 0, max: 1, pas: 0.01 },
    ],
  },
  {
    // Le dos n'a pas de contrôle de largeur : elle vient de la pagination. C'est le
    // seul réglage de la maquette que l'utilisateur ne peut pas toucher, et c'est
    // exactement ce que l'application apporte.
    titre: 'Dos — fond et espacements',
    face: 'dos',
    champs: [
      { chemin: 'dos.fond_propre', libelle: 'Fond distinct du papier', type: 'case' },
      { chemin: 'dos.fond', libelle: 'Couleur du fond', type: 'couleur' },
      { chemin: 'dos.marge', libelle: 'Retrait aux extrémités', type: 'nombre', min: 0, max: 20, pas: 0.5, unite: '% larg.' },
      { chemin: 'dos.ecart', libelle: 'Écart entre éléments', type: 'nombre', min: 0, max: 20, pas: 0.5, unite: '% larg.' },
    ],
  },
  elementDos('auteur', 'auteur'),
  elementDos('titre', 'titre'),
  elementDos('editeur', 'éditeur'),
  {
    titre: '4ème — pied et ISBN',
    face: 'quatre',
    champs: [
      { chemin: 'quatrieme.pied_actif', libelle: 'Afficher le pied', type: 'case' },
      { chemin: 'quatrieme.mention', libelle: 'Mention', type: 'texte' },
      { chemin: 'quatrieme.collection', libelle: 'Collection', type: 'texte' },
      { chemin: 'quatrieme.prix', libelle: 'Prix', type: 'texte' },
      { chemin: 'quatrieme.pied_y', libelle: 'Hauteur du pied', type: 'nombre', min: 0, max: 30, pas: 0.5, unite: '% larg.' },
      { chemin: 'quatrieme.isbn_actif', libelle: 'Réserver la zone ISBN', type: 'case' },
      { chemin: 'quatrieme.isbn_l', libelle: 'Largeur ISBN', type: 'nombre', min: 5, max: 60, pas: 0.5, unite: '% larg.' },
      { chemin: 'quatrieme.isbn_h', libelle: 'Hauteur ISBN', type: 'nombre', min: 5, max: 60, pas: 0.5, unite: '% larg.' },
      { chemin: 'quatrieme.isbn_dx', libelle: 'Décalage horizontal', type: 'nombre', min: 0, max: 30, pas: 0.5, unite: '% larg.' },
      { chemin: 'quatrieme.isbn_dy', libelle: 'Décalage vertical', type: 'nombre', min: 0, max: 30, pas: 0.5, unite: '% larg.' },
    ],
  },
  { ...style('quatrieme.style_pied', '4ème — style du pied', { casse: false }), face: 'quatre' },
];

/** Groupes du schéma, chaque `avant`/`apres` replié dans la liste des champs. */
function groupes() {
  return SCHEMA.map((g) => ({
    ...g,
    champs: [...(g.avant ?? []), ...g.champs, ...(g.apres ?? [])],
  }));
}

/**
 * Le libellé public d'un mode de page, lu dans le schéma.
 *
 * Recopié ailleurs, il dériverait du jour où un mode change de nom : le schéma est la
 * seule liste où ces trois mots sont écrits. Lu par `groupes()` et non dans `SCHEMA`,
 * parce que les champs d'un groupe peuvent aussi vivre dans son `avant` ou son `apres` —
 * ce n'est pas le cas de « Mode » aujourd'hui, et rien ne le garantit demain.
 */
function libelleMode(mode) {
  const champ = groupes()
    .flatMap((g) => g.champs)
    .find((c) => c.chemin === 'mode');
  return champ.options.find(([v]) => v === mode)?.[1] ?? mode;
}

const lire = (obj, chemin) => chemin.split('.').reduce((o, k) => (o ?? {})[k], obj);

function ecrire(obj, chemin, valeur) {
  const cles = chemin.split('.');
  const dernier = cles.pop();
  const cible = cles.reduce((o, k) => o[k], obj);
  cible[dernier] = valeur;
}

/* ---------- câblage de l'étape ---------- */

/* Le panneau, l'aperçu et les faces vivent ici, avec le schéma qu'ils servent. Rien
   ne s'exécute au chargement — le fichier reste requérable nu par les tests — et
   l'état partagé (`projet`, `face`…) est déclaré dans `app.js`, comme
   pour `livraison.js`. */

/**
 * Repart d'une maquette, et rend l'invite à son menu.
 *
 * Le menu ne montre pas un état : le projet ne garde pas de quelle maquette il est
 * parti, et il n'aurait rien de vrai à dire une fois les réglages repris un par un. Il
 * ne porte donc qu'un geste, et revient sur son invite — y laisser « Folio » affiché
 * ferait passer pour un état ce qui est un bouton, et le geste, refait par mégarde,
 * écrase tous les réglages.
 */
async function choisirMaquette() {
  const sel = $('inMaquette');
  const cle = sel.value;
  sel.value = '';
  if (!cle) return;
  await tente(async () => afficherProjet(await invoke('maquette_choisir', { cle })));
}

/**
 * Remplace la photo d'une face.
 *
 * Une seule par face, et c'est le projet qui la porte : la photo est copiée dans le
 * `.ozalid` comme le manuscrit, et le chemin d'où elle vient n'a plus à exister pour
 * que la couverture se compose.
 */
async function choisirImage(face) {
  const chemin = await open({
    multiple: false,
    filters: [{ name: 'Photo de couverture', extensions: ['jpg', 'jpeg', 'png'] }],
  });
  if (!chemin) return;
  await tente(async () =>
    afficherProjet(await invoke('image_choisir', { face, chemin })));
}

/** Un contrôle du schéma. Son id porte le chemin, ce qui suffit à le relire. */
function controle(c) {
  let el;
  if (c.type === 'liste' || c.type === 'polices') {
    el = h('select');
    const options = c.type === 'polices' ? polices.map((p) => [p, p]) : c.options;
    for (const [v, l] of options) el.append(new Option(l, v));
  } else if (c.type === 'zone') {
    el = h('textarea');
    el.rows = 4;
  } else {
    el = h('input');
    el.type = c.type === 'couleur' ? 'color' : c.type === 'case' ? 'checkbox' : c.type === 'nombre' ? 'number' : 'text';
    if (c.type === 'nombre') {
      el.min = c.min; el.max = c.max; el.step = c.pas;
    }
  }
  el.addEventListener('change', majCouverture);
  return el;
}

/** Contrôles construits, avec leur champ de schéma et leur ligne. */
let controles = [];
/** Blocs du panneau, avec la face et les modes qui les concernent. */
let blocs = [];

function construireReglages() {
  const box = $('reglages');
  box.replaceChildren();
  controles = [];
  blocs = [];
  for (const g of groupes()) {
    const bloc = h('div', undefined, 'groupe');
    bloc.append(h('h3', g.titre));
    for (const c of g.champs) {
      const ligne = h('label');
      const lib = c.unite ? `${c.libelle} (${c.unite})` : c.libelle;
      const el = controle(c);
      ligne.append(h('span', lib), el);
      bloc.append(ligne);
      controles.push({ champ: c, el, ligne });
    }
    blocs.push({ el: bloc, face: g.face ?? 'une', modes: g.modes ?? null });
    box.append(bloc);
  }
}

/** Remplit les contrôles depuis la maquette, et masque ce qui est sans objet. */
function afficherCouverture(cv) {
  for (const { champ, el, ligne } of controles) {
    const v = lire(cv, champ.chemin);
    if (champ.type === 'case') el.checked = !!v;
    else el.value = v ?? '';
    // Un réglage sans objet dans l'état courant est masqué plutôt que grisé : le
    // panneau est long, et un contrôle inopérant y serait un piège.
    ligne.hidden = !!champ.modes && !champ.modes.includes(cv.mode);
  }
  for (const b of blocs) {
    b.el.hidden = b.face !== face || (!!b.modes && !b.modes.includes(cv.mode));
  }
  poserDisposition(blocs.some((b) => !b.el.hidden));
}

/**
 * Dit à la feuille de style ce que la face demande de la fenêtre.
 *
 * Deux choses qu'un sélecteur ne peut pas déduire du balisage : quelle face est montrée
 * — le dos couché prend la largeur en bandeau, ses réglages coulent en colonnes dessous —
 * et si le panneau a quelque chose à montrer. La planche n'a plus de réglage à elle : la
 * colonne qui l'attendait rendrait à l'aperçu une fenêtre amputée du tiers.
 */
function poserDisposition(panneau) {
  const couv = $('couv');
  couv.setAttribute('data-face', face);
  couv.setAttribute('data-panneau', panneau ? 'oui' : 'non');
  $('reglages').hidden = !panneau;
}

/**
 * Un nombre ramené dans les bornes du schéma.
 *
 * Les flèches du champ les respectent, la frappe au clavier non : rien n'empêche de
 * taper 500 dans une marge qui va jusqu'à 40. Ramenée ici, la valeur revient corrigée
 * dans le panneau au rafraîchissement, et la maquette reste composable.
 */
function nombreSaisi(el, champ) {
  const v = Number(el.value);
  if (champ.min === undefined || champ.max === undefined) return v;
  return Math.min(Math.max(v, champ.min), champ.max);
}

/** Relit les contrôles et renvoie la maquette modifiée. */
function couvertureSaisie() {
  const cv = JSON.parse(JSON.stringify(projet.couverture));
  for (const { champ, el } of controles) {
    let v;
    if (champ.type === 'case') v = el.checked;
    else if (champ.type === 'nombre') v = nombreSaisi(el, champ);
    else v = el.value;
    ecrire(cv, champ.chemin, v);
  }
  return cv;
}

async function majCouverture() {
  await tente(async () =>
    afficherProjet(await invoke('couverture_modifier', { couverture: couvertureSaisie() })));
}

/**
 * Aperçu, avec un délai de grâce : chaque réglage relance une composition Typst, et
 * enchaîner les crans d'un curseur en lancerait une par cran.
 */
function demanderApercu() {
  clearTimeout(attenteApercu);
  attenteApercu = setTimeout(rendreApercu, 180);
}

/**
 * Dos à passer à l'aperçu : celui que le destinataire visé porte, et rien d'autre.
 *
 * Il n'y a plus rien à comparer ici, et c'est tout l'objet du dispositif : une mesure
 * enregistrée vaut toujours. Les quatre causes qui la déplaçaient — le gabarit, le
 * papier, la police, le texte — l'effacent maintenant à la source, dans le Rust, au
 * moment même du geste. L'estampille qu'on tenait ici ne voyait que trois d'entre
 * elles ; le livre, cinquième cause, lui échappait entièrement.
 */
function dosCourant() {
  return destinataireCourant()?.compose?.dos ?? null;
}

/**
 * Pose l'aperçu, ou le retire faute d'image.
 *
 * Retiré pour de bon : une image sans source garde sa place et son fond blanc, et ce
 * rectangle-là ne se distingue pas d'une couverture vide — il donne à voir un livre
 * là où le message dit qu'il n'y en a pas.
 */
function poserApercu(a) {
  const img = $('apercu');
  if (a) img.src = a.image;
  else img.removeAttribute('src');
  img.hidden = !a;
}

async function rendreApercu() {
  if (!projet?.couverture) {
    poserApercu(null);
    $('etatApercu').textContent = 'Choisir une maquette de départ.';
    // Sans cette ligne, une invitation à choisir s'écrirait en rouge dès que l'aperçu
    // précédent avait échoué : la classe survivrait au message qu'elle qualifiait.
    $('etatApercu').className = 'note';
    return;
  }
  $('etatApercu').textContent = 'composition de l\'aperçu…';
  try {
    // Ni gabarit ni fond perdu à passer : ils viennent du destinataire visé, que le
    // Rust lit dans le projet.
    poserApercu(await invoke('couverture_apercu', { face, dosMm: dosCourant() }));
    $('etatApercu').textContent = '';
    $('etatApercu').className = 'note';
  } catch (e) {
    poserApercu(null);
    $('etatApercu').textContent = String(e);
    $('etatApercu').className = 'note alerte';
  }
}

/**
 * Les quatre faces de la couverture.
 *
 * Le dos a la sienne depuis que la planche a cessé de la lui prêter : trois textes et
 * leurs places se réglaient en regardant une bande de seize pixels. Séparés, chacun
 * montre ce qu'il règle — et la planche, qui ne règle plus rien, devient ce qu'elle est :
 * la vue de contrôle, sans panneau, sur la fenêtre entière.
 *
 * Rien à voir avec les onglets d'étape, malgré l'air de famille et le mot « onglets »
 * qu'ils partagent en CSS : ceux-là sont des `tab` d'un `tablist`, dont un seul est
 * sélectionné et qui commandent chacun une section ; ceux-ci sont des boutons à deux
 * états (`aria-pressed`) qui changent ce qu'un même aperçu montre. Deux patterns ARIA,
 * et deux façons de retrouver le bouton : par identifiant là-bas, **par rang** ici —
 * `choisirFace` relit `FACES[i][0]` en parcourant les enfants de `#faces`.
 *
 * Les unifier serait un vrai travail, pas un nettoyage : il faudrait leur trouver un
 * pattern commun qu'aucun des deux n'a. Les croire déjà unifiés coûterait plus cher.
 */
const FACES = [['une', '1ère'], ['quatre', '4ème'], ['dos', 'Dos'], ['planche', 'Planche']];

function construireFaces() {
  for (const [cle, libelle] of FACES) {
    const b = h('button', libelle);
    b.type = 'button';
    b.setAttribute('aria-pressed', String(cle === face));
    b.addEventListener('click', () => choisirFace(cle));
    $('faces').append(b);
  }
}

function choisirFace(v) {
  face = v;
  [...$('faces').children].forEach((b, i) => {
    b.setAttribute('aria-pressed', String(FACES[i][0] === v));
  });
  if (projet?.couverture) afficherCouverture(projet.couverture);
  else poserDisposition(false);
  demanderApercu();
}

if (typeof module !== 'undefined') {
  module.exports = { SCHEMA, groupes, lire, ecrire, libelleMode };
}
