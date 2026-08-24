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
 * Un élément du dos : où il se place, dans quel sens, et son style entier.
 *
 * Les quatre éléments ont exactement les mêmes réglages — le dos d'une collection met
 * le titre en tête et l'auteur au pied, celui d'une autre les groupe : rien ici ne
 * doit privilégier un usage.
 *
 * La place, le rang et le sens sont `cache` : leurs contrôles existent, mais le panneau
 * ne les montre pas. Ils se règlent sur l'aperçu — on traîne le texte au tiers qu'on
 * veut, on le retourne par l'icône posée dans son coin — et trois contrôles de plus par
 * élément rediraient dans une colonne de 13,5 rem ce que le dos montre déjà. Ils
 * restent néanmoins des contrôles, et c'est délibéré : le geste y pose sa valeur, la
 * commande la relit avec toutes les autres, et rien n'a de chemin à lui.
 */
function elementDos(cle, libelle) {
  return {
    ...style(`dos.${cle}.style`, libelle),
    face: 'dos',
    avant: [
      { chemin: `dos.${cle}.actif`, libelle: 'Afficher', type: 'case' },
      { chemin: `dos.${cle}.place`, libelle: 'Position', type: 'liste', options: PLACES_DOS, cache: true },
      { chemin: `dos.${cle}.rang`, libelle: 'Ordre à cette position', type: 'nombre', min: 1, max: 9, pas: 1, cache: true },
      { chemin: `dos.${cle}.sens`, libelle: 'Sens', type: 'nombre', min: 0, max: 270, pas: 90, cache: true },
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
      { chemin: 'pied.y', libelle: 'Hauteur depuis le bas', type: 'nombre', min: 0, max: 50, pas: 0.5, unite: '% haut.' },
    ],
  },
  style('pied.style_mono', 'Monogramme', { casse: false, italique: true }),
  style('pied.style_editeur', 'Mention éditeur', { casse: false }),
  {
    titre: 'Pastille',
    champs: [
      { chemin: 'pastille.actif', libelle: 'Afficher la pastille', type: 'case' },
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
      // L'interligne sépare les lignes d'un passage, celui-ci les passages entre eux.
      // Une 4ème n'a ni alinéa ni blanc de série : à zéro, deux paragraphes s'y lisent
      // comme un seul.
      { chemin: 'quatrieme.paragraphe_ecart', libelle: 'Écart entre paragraphes', type: 'nombre', min: 0, max: 20, pas: 0.5, unite: '% larg.' },
      {
        chemin: 'quatrieme.align', libelle: 'Alignement', type: 'liste', options: [
          ['gauche', 'Gauche'], ['centre', 'Centre'], ['droite', 'Droite'],
        ],
      },
      { chemin: 'quatrieme.pad_x', libelle: 'Marge latérale', type: 'nombre', min: 0, max: 40, pas: 0.5, unite: '% larg.' },
      { chemin: 'quatrieme.top', libelle: 'Hauteur du texte', type: 'nombre', min: 0, max: 60, pas: 0.5, unite: '% larg.' },
    ],
  },
  {
    // L'italique est offert ici et nulle part ailleurs sur la 4ème : il couche tout le
    // bloc — un exergue, une citation. Pour un seul mot, le texte se marque comme le
    // manuscrit, `*mot*` et `**mot**`.
    ...style('quatrieme.style', '4ème — style du texte', { casse: false, italique: true }),
    face: 'quatre',
  },
  {
    // La tête de la 4ème : l'auteur, le titre et un filet, au-dessus du texte. Trois
    // interrupteurs et non un seul — une collection met l'auteur et le filet sans
    // répéter le titre, une autre le titre seul.
    //
    // Ce que ce groupe ne porte pas : le texte de l'auteur et du titre. Ils viennent du
    // livre, comme sur la 1ère, et une maquette ne dit jamais ce qui est écrit.
    titre: '4ème — tête',
    face: 'quatre',
    champs: [
      { chemin: 'quatrieme.tete.auteur_visible', libelle: 'Afficher l\'auteur', type: 'case' },
      { chemin: 'quatrieme.tete.titre_visible', libelle: 'Afficher le titre', type: 'case' },
      { chemin: 'quatrieme.tete.filet_visible', libelle: 'Afficher le filet', type: 'case' },
      {
        chemin: 'quatrieme.tete.align', libelle: 'Alignement de la tête', type: 'liste', options: [
          ['gauche', 'Gauche'], ['centre', 'Centre'], ['droite', 'Droite'],
        ],
      },
      { chemin: 'quatrieme.tete.titre_ecart', libelle: 'Écart auteur → titre', type: 'nombre', min: 0, max: 20, pas: 0.5, unite: '% larg.' },
      { chemin: 'quatrieme.tete.filet_ecart', libelle: 'Écart titre → filet', type: 'nombre', min: 0, max: 20, pas: 0.5, unite: '% larg.' },
      { chemin: 'quatrieme.tete.ecart', libelle: 'Écart tête → texte', type: 'nombre', min: 0, max: 30, pas: 0.5, unite: '% larg.' },
      { chemin: 'quatrieme.tete.filet.largeur', libelle: 'Largeur du filet', type: 'nombre', min: 1, max: 100, pas: 0.5, unite: '% larg.' },
      { chemin: 'quatrieme.tete.filet.epaisseur', libelle: 'Épaisseur du filet', type: 'nombre', min: 0.05, max: 3, pas: 0.05, unite: '% larg.' },
      { chemin: 'quatrieme.tete.filet.couleur', libelle: 'Couleur du filet', type: 'couleur' },
    ],
  },
  { ...style('quatrieme.tete.auteur', '4ème — auteur'), face: 'quatre' },
  { ...style('quatrieme.tete.titre', '4ème — titre'), face: 'quatre' },
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
    titre: 'Fond et espacements',
    face: 'dos',
    // Ce groupe ne règle pas un texte mais le dos lui-même : la feuille de style l'en
    // sépare, sans quoi il se lisait comme une cinquième colonne de texte.
    classe: 'groupe-dos-fond',
    champs: [
      { chemin: 'dos.fond_propre', libelle: 'Fond distinct du papier', type: 'case' },
      { chemin: 'dos.fond', libelle: 'Couleur du fond', type: 'couleur' },
      { chemin: 'dos.marge', libelle: 'Retrait aux extrémités', type: 'nombre', min: 0, max: 20, pas: 0.5, unite: '% larg.' },
      { chemin: 'dos.ecart', libelle: 'Écart entre éléments', type: 'nombre', min: 0, max: 20, pas: 0.5, unite: '% larg.' },
    ],
  },
  elementDos('auteur', 'Auteur'),
  elementDos('titre', 'Titre'),
  elementDos('editeur', 'Éditeur'),
  elementDos('collection', 'Collection'),
  {
    titre: '4ème — pied et ISBN',
    face: 'quatre',
    champs: [
      { chemin: 'quatrieme.pied_actif', libelle: 'Afficher le pied', type: 'case' },
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
 * (Re)construit le menu de la barre **et** la liste du dialogue, d'un seul appel.
 *
 * La liste vit dans le Rust, qui relit le répertoire de configuration à chaque appel ;
 * la tenir à jour ici la dédoublerait. Rappelée après chaque geste, sans quoi ce qu'on
 * vient de cloner manquerait à la liste d'où on l'a tiré.
 *
 * Les personnalisées suivent les fournies, derrière une option inerte qui fait le
 * séparateur — le Rust les rend déjà dans cet ordre, la fenêtre n'a qu'à repérer où
 * l'origine change.
 */
async function rafraichirMaquettes() {
  const maquettes = await invoke('maquettes_liste');

  const sel = $('inMaquette');
  sel.replaceChildren();
  sel.append(new Option('Repartir d\'une maquette…', ''));
  let separateur = false;
  for (const m of maquettes) {
    if (!m.fournie && !separateur) {
      const trait = new Option('──────────', '');
      trait.disabled = true;
      sel.append(trait);
      separateur = true;
    }
    sel.append(new Option(m.libelle, m.cle));
  }
  sel.value = '';

  const liste = $('listeMaquettes');
  liste.replaceChildren();
  for (const m of maquettes) liste.append(ligneMaquette(m));
}

/**
 * Fait un geste du dialogue, en rend compte, et refait la liste.
 *
 * Le compte rendu se lit **dans** le dialogue et non dans l'alerte de la fenêtre :
 * celle-ci est derrière lui, et un refus y passerait inaperçu — le geste paraîtrait
 * avoir marché.
 */
async function rendCompte(action, dit = '') {
  try {
    await action();
    $('etatMaquettes').textContent = dit;
    $('etatMaquettes').className = 'etat';
    await rafraichirMaquettes();
  } catch (e) {
    $('etatMaquettes').textContent = String(e);
    $('etatMaquettes').className = 'etat erreur';
  }
}

/**
 * Une ligne de la liste : le nom, ce qu'elle est, et ses gestes.
 *
 * Sans `innerHTML` — le nom d'une maquette vient d'un fichier qu'on n'a pas écrit. Une
 * fournie n'offre que Cloner : c'est une politesse, la garantie est dans le Rust, qui
 * refuse de renommer et d'effacer ce qui est livré avec lui.
 */
function ligneMaquette(m) {
  const ligne = document.createElement('div');
  ligne.className = 'ligne maquette';

  const nom = document.createElement('span');
  nom.className = 'nom';
  nom.textContent = m.libelle;
  ligne.append(nom);

  if (m.fournie) {
    const dit = document.createElement('span');
    dit.className = 'note';
    dit.textContent = 'fournie';
    ligne.append(dit);
  }

  ligne.append(boutonGeste('Cloner', () => invoke('maquette_cloner', { cle: m.cle })));
  if (!m.fournie) {
    ligne.append(gesteRenommer(m, ligne, nom));
    ligne.append(gesteEffacer(m));
  }
  return ligne;
}

/**
 * Un bouton de geste du dialogue : il agit, rend compte, et refait la liste.
 *
 * `boutonGeste` et non `geste` : `app.js` tient déjà un `geste` global — celui de la
 * manipulation directe en cours — et les deux scripts partagent leur portée.
 */
function boutonGeste(libelle, action) {
  const b = document.createElement('button');
  b.type = 'button';
  b.textContent = libelle;
  b.addEventListener('click', () => rendCompte(action));
  return b;
}

/**
 * Renommer, en place : le nom devient un champ, Entrée valide, perdre le focus annule.
 *
 * Échap n'est pas intercepté — dans un `<dialog>` il ferme la boîte, et le détourner
 * priverait l'utilisateur du geste qu'il connaît.
 */
function gesteRenommer(m, ligne, nom) {
  const b = document.createElement('button');
  b.type = 'button';
  b.textContent = 'Renommer';
  b.addEventListener('click', () => {
    const champ = document.createElement('input');
    champ.type = 'text';
    champ.className = 'nom';
    champ.value = m.libelle;
    champ.addEventListener('keydown', (e) => {
      if (e.key !== 'Enter') return;
      return rendCompte(() =>
        invoke('maquette_renommer', { cle: m.cle, nom: champ.value.trim() }));
    });
    champ.addEventListener('blur', () => rafraichirMaquettes());
    ligne.replaceChild(champ, nom);
    champ.focus();
  });
  return b;
}

/**
 * Effacer, en deux temps : ce qui se perd ici ne se retrouve pas, et le bouton est à
 * quelques pixels de « Renommer ». Le premier clic arme, le second efface.
 */
function gesteEffacer(m) {
  const b = document.createElement('button');
  b.type = 'button';
  b.textContent = 'Effacer';
  b.addEventListener('click', () => {
    if (b.textContent === 'Effacer') {
      b.textContent = 'Confirmer';
      b.className = 'danger';
      return undefined;
    }
    return rendCompte(() => invoke('maquette_effacer', { cle: m.cle }));
  });
  return b;
}

/**
 * Enregistre la couverture réglée comme maquette.
 *
 * Le champ ne se vide qu'une fois la commande passée : un refus doit laisser au nom sa
 * chance d'être corrigé.
 */
async function enregistrerMaquette() {
  const nom = $('inMaquetteNom').value.trim();
  await rendCompte(async () => {
    await invoke('maquette_enregistrer', { nom });
    $('inMaquetteNom').value = '';
  }, `« ${nom} » enregistrée.`);
}

/**
 * Repart d'une maquette, et rend l'invite à son menu.
 *
 * Le menu ne montre pas un état : le projet ne garde pas de quelle maquette il est
 * parti, et il n'aurait rien de vrai à dire une fois les réglages repris un par un. Il
 * ne porte donc qu'un geste, et revient sur son invite — y laisser « Bandeau » affiché
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

/**
 * Les photos du projet dans la barre, chacune avec le geste qui la retire.
 *
 * Le retrait est posé sur le nom plutôt que sur un bouton de plus : la barre en porte
 * déjà six et tronque ses libellés, et le nom est justement l'endroit où la présence de
 * la photo se lit.
 *
 * Il retire la photo du `.ozalid` — c'est le seul geste qui allège l'archive, régler le
 * fond de la 4ème sur le papier de la 1ère cesse seulement de la composer. La maquette
 * n'est pas touchée : un fond réglé sur « Image propre » compose alors son papier seul,
 * et l'aperçu le montre.
 */
function afficherPhotos(noms) {
  const box = $('etatImages');
  box.replaceChildren();
  if (!noms.length) {
    box.textContent = 'aucune photo';
    return;
  }
  for (const nom of noms) box.append(lignePhoto(nom));
}

/**
 * Un nom de photo et sa croix. Sans `innerHTML` : le nom vient d'un `.ozalid` qu'on n'a
 * pas forcément écrit.
 */
function lignePhoto(nom) {
  const ligne = h('span', undefined, 'photo');
  ligne.append(h('span', nom, 'nom'));
  const croix = h('button', '×', 'retirer');
  croix.type = 'button';
  croix.title = `Retirer ${nom} du projet`;
  croix.addEventListener('click', () =>
    tente(async () => afficherProjet(await invoke('image_retirer', { nom }))));
  ligne.append(croix);
  return ligne;
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
    const bloc = h('div', undefined, g.classe ? `groupe ${g.classe}` : 'groupe');
    bloc.append(h('h3', g.titre));
    for (const c of g.champs) {
      const ligne = h('label');
      const lib = c.unite ? `${c.libelle} (${c.unite})` : c.libelle;
      const el = controle(c);
      ligne.append(h('span', lib), el);
      // Un champ `cache` a bien son contrôle — le geste y pose sa valeur, la commande
      // l'y relit — mais sa ligne n'entre pas dans le panneau : elle n'est nulle part,
      // plutôt que posée et masquée, pour qu'aucun réglage de mode ne la ramène.
      if (!c.cache) bloc.append(ligne);
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
    // Un champ que la souris tient en ce moment ne se réécrit pas : la valeur qui
    // revient est celle de la composition rattrapée, et le geste, lui, a continué.
    // Sans cette ligne, la couverture reculerait d'un cran à chaque rattrapage.
    if (geste?.chemins.has(champ.chemin)) {
      ligne.hidden = !!champ.modes && !champ.modes.includes(cv.mode);
      continue;
    }
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
  poserPrises();
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
  // La lunette n'a d'objet que là où il y a du fond perdu à voir. Masquée plutôt que
  // grisée, comme les réglages sans objet.
  $('btReperes').hidden = face !== 'planche';
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
  // Zéro pendant un geste : la commande a déjà fait l'attente, et la répéter ajoutait
  // 180 ms à chaque rattrapage sans rien coalescer de plus.
  attenteApercu = setTimeout(rendreApercu, geste ? 0 : 180);
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
  // Le rapport d'aspect s'en va avec l'image : il dimensionne le cadre, qui garderait
  // sinon sa place, vide, entre la scène et le message qui dit qu'il n'y a rien à voir.
  if (!a) $('cadreApercu').style.removeProperty('--ratio');
  poserReperes(a?.reperes ?? null);
  poserMesures(a?.mesures ?? null);
  // Le direct ne s'efface qu'entre deux gestes : pendant, il montre plus juste que
  // l'image qui vient d'arriver, laquelle a déjà un cran de retard sur la souris.
  if (!geste) masquerDirect();
  poserPrises();
}

/**
 * Écrit sous la planche ce qu'elle mesure — ou retire la ligne.
 *
 * Les quatre nombres viennent du Rust et ne se recomposent pas ici : la largeur d'une
 * planche est deux couvertures, un dos et deux fonds perdus, et cette addition est déjà
 * faite dans `planche::Gabarit`. Trois décimales au fond perdu et deux ailleurs, comme
 * la Livraison : un fond perdu se relève au dixième de dixième sur les gabarits, un dos
 * jamais.
 */
function poserMesures(m) {
  const p = $('mesuresApercu');
  p.hidden = !m;
  if (!m) return;
  p.textContent = `Planche ${nb(m.largeur)} × ${nb(m.hauteur)} mm — dos ${nb(m.dos)} mm`
    + ` — fond perdu ${nb(m.fond_perdu, 3)} mm`;
}

/**
 * Donne au cadre le rapport d'aspect de l'image qu'il vient de recevoir.
 *
 * C'est ce qui lui donne une taille : sans lui, le cadre se dimensionnerait sur une
 * image elle-même bornée en pourcentage du cadre, et le navigateur tranche ce cycle à
 * zéro — mesuré, cadre et image à 0 × 0 dans une scène de 620 × 345. Le rapport ne se
 * connaît qu'une fois l'image décodée, d'où l'écoute du chargement.
 */
function poserRatio() {
  const img = $('apercu');
  if (!img.naturalHeight) return;
  $('cadreApercu').style.setProperty('--ratio', String(img.naturalWidth / img.naturalHeight));
}

/**
 * Les repères de la planche, mesurés par le Rust et posés sur l'image : la bande que le
 * massicot emporte, et les deux plis qui encadrent le dos.
 *
 * Des fractions, pas des millimètres : l'aperçu s'affiche à la taille que la fenêtre lui
 * laisse, et seules des proportions y survivent. Elles ne se recalculent pas ici — ce
 * serait redire la règle qui choisit entre le fond perdu publié par le prestataire et
 * celui relevé sur son gabarit, et refaire le calcul de dos que la pagination commande.
 */
function poserReperes(reperes) {
  reperesCourants = reperes;
  if (reperes) {
    const cadre = $('cadreApercu');
    cadre.style.setProperty('--coupe-x', String(reperes.x));
    cadre.style.setProperty('--coupe-y', String(reperes.y));
    cadre.style.setProperty('--pli-quatre', String(reperes.pli_quatre));
    cadre.style.setProperty('--pli-une', String(reperes.pli_une));
  }
  rendreReperes();
}

/** L'habillage suit deux choses : l'aperçu posé et la lunette. Les deux passent ici. */
function rendreReperes() {
  $('reperes').hidden = !reperesCourants || !reperesVisibles;
}

/**
 * Allume ou éteint la lunette.
 *
 * Rien à recomposer : l'habillage est posé **sur** l'image, pas dedans. C'est ce qui
 * rend la bascule instantanée — et ce qui garantit qu'aucun repère ne peut se glisser
 * dans le PDF remis au prestataire.
 */
function basculerReperes() {
  reperesVisibles = !reperesVisibles;
  $('btReperes').setAttribute('aria-pressed', String(reperesVisibles));
  rendreReperes();
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
    // Après l'aperçu, jamais pendant un geste : une composition de plus, dont le geste
    // en cours n'a pas besoin — l'habillage qu'il tient ne dépend pas du cadrage.
    if (!geste) demanderCalques();
  } catch (e) {
    poserApercu(null);
    $('etatApercu').textContent = String(e);
    $('etatApercu').className = 'note alerte';
  }
}

/* ---------- manipulation directe ---------- */

/**
 * Ce que chaque face manipulable règle, et sous quel nom.
 *
 * Les deux faces portent les mêmes trois choses — un cadrage, un bloc de texte et ses
 * marges — sous deux préfixes différents. La table est la seule à le savoir : les gestes
 * qui suivent ne connaissent que `cadrage`, `bloc` et `pad`.
 *
 * `bloc_sur` n'est pas un détail : la hauteur du bloc de la 1ère se compte en pourcentage
 * de la **hauteur** de la couverture, celle de la 4ème en pourcentage de sa **largeur**.
 * C'est le schéma qui le dit, chacun dans son unité, et un geste qui l'ignorerait
 * déplacerait le texte de la 4ème d'un tiers de trop.
 */
const REGLAGES = {
  une: { cadrage: 'cadrage', bloc: 'bloc_y', bloc_sur: 'hauteur', pad: 'pad_x' },
  quatre: {
    cadrage: 'quatrieme.cadrage', bloc: 'quatrieme.top', bloc_sur: 'largeur',
    pad: 'quatrieme.pad_x',
  },
};

/** Le format du destinataire visé, en millimètres — l'échelle de tout ce qui suit. */
function formatCourant() {
  const d = destinataireCourant();
  const p = providers.find((x) => x.cle === d?.provider);
  return p ? { largeur: p.largeur, hauteur: p.hauteur } : null;
}

/** Le contrôle qui porte un chemin du schéma, s'il existe. */
const controleDe = (chemin) => controles.find((c) => c.champ.chemin === chemin);

/**
 * La valeur que porte un contrôle — et non celle du projet.
 *
 * Pendant un geste, les deux diffèrent : le contrôle est déjà à la position de la
 * souris, le projet en est encore à la dernière composition. C'est le contrôle qui fait
 * foi, puisque c'est lui que la commande relira.
 */
function valeurSaisie(chemin) {
  const c = controleDe(chemin);
  if (!c) return undefined;
  if (c.champ.type === 'case') return c.el.checked;
  if (c.champ.type === 'nombre') return nombreSaisi(c.el, c.champ);
  return c.el.value;
}

/**
 * Pose une valeur dans son contrôle, au cran du schéma et dans ses bornes.
 *
 * Au cran, et c'est ce qui rend le geste et le panneau lisibles ensemble : la souris
 * produit des réels, et une hauteur de bandeau à 37,813567143516 % s'écrit telle quelle
 * dans le champ, part telle quelle au projet, et ne se retape pas à la main. Le pas est
 * celui que le schéma offre déjà aux flèches du champ — la souris n'a pas de raison
 * d'être plus fine que le clavier.
 */
function poserValeur(chemin, v) {
  const c = controleDe(chemin);
  if (!c) return;
  const { min, max, pas } = c.champ;
  if (min !== undefined) v = Math.min(Math.max(v, min), max);
  if (pas) {
    // `toFixed` après l'arrondi : sans lui, 0,1 × 3 s'écrirait 0,30000000000000004.
    const decimales = (String(pas).split('.')[1] ?? '').length;
    v = Number((Math.round(v / pas) * pas).toFixed(decimales));
  }
  c.el.value = v;
}

/** Les cinq réglages d'un cadrage, tels que les contrôles les portent. */
function cadrageSaisi(prefixe) {
  return {
    proportions: valeurSaisie(`${prefixe}.proportions`),
    x: valeurSaisie(`${prefixe}.x`),
    y: valeurSaisie(`${prefixe}.y`),
    zoom: valeurSaisie(`${prefixe}.zoom`),
    etirement: valeurSaisie(`${prefixe}.etirement`),
  };
}

/**
 * Où la photo se pose dans sa zone — le portage exact d'`image::place`.
 *
 * Traduit du Rust, et c'est le seul endroit de l'application où une règle de composition
 * existe en deux langues. Le prix est assumé : sans elle, la fenêtre ne peut pas montrer
 * la photo suivre la souris, et une composition Typst par pixel parcouru n'est pas
 * envisageable — 110 ms mesurés pour une 1ère. Les deux versions sont tenues d'accord
 * par une table de cas partagée, écrite deux fois : `tests/couverture.test.js` ici,
 * `image.rs` là-bas.
 */
function placeImage(zone, naturel, c) {
  if (zone.largeur <= 0 || zone.hauteur <= 0 || naturel.largeur <= 0 || naturel.hauteur <= 0) {
    return null;
  }
  const a = zone.largeur / naturel.largeur;
  const b = zone.hauteur / naturel.hauteur;
  const ajuste = c.proportions ? Math.min(a, b) : Math.max(a, b);
  // La déformation est repliée dans l'échelle horizontale, et n'a de sens qu'en cadrage
  // débordant : conserver les proportions la neutralise.
  const sx = c.zoom * (c.proportions ? 1 : c.etirement);
  const dl = naturel.largeur * ajuste;
  const dh = naturel.hauteur * ajuste;
  const gauche = (zone.largeur - dl) * c.x;
  const haut = (zone.hauteur - dh) * c.y;
  // Le zoom se prend autour du point d'ancrage, pas du coin de la zone.
  const ox = zone.largeur * c.x;
  const oy = zone.hauteur * c.y;
  return {
    gauche: ox - (ox - gauche) * sx,
    haut: oy - (oy - haut) * c.zoom,
    largeur: dl * sx,
    hauteur: dh * c.zoom,
  };
}

/** La zone de la photo en millimètres, pour la face montrée. */
function zoneMm() {
  const f = formatCourant();
  if (!f || !calques) return null;
  const z = calques.zone;
  return {
    gauche: z.x * f.largeur, haut: z.y * f.hauteur,
    largeur: z.l * f.largeur, hauteur: z.h * f.hauteur,
  };
}

/**
 * Le mou de la photo sur chaque axe : la course dont le cadrage dispose, **et son sens**.
 *
 * Nul quand l'image couvre sa zone pile — et le geste se refuse alors plutôt que de
 * déplacer un curseur qui ne déplace rien. C'est le parti de l'atelier HTML, repris
 * ici : un geste sans effet visible fait douter du réglage, pas du cadrage.
 *
 * Signé, et ce n'est pas un détail : la géométrie donne `gauche = ancrage × (zone −
 * affichée)`. Une photo qui déborde de sa zone a un facteur négatif — l'ancrage recule
 * ce que la souris avance — et une photo qui y tient un facteur positif, où l'ancrage
 * accompagne le geste. Pris en valeur absolue, comme il l'était, le mou perdait ce signe
 * et la photo d'une 4ème en proportions conservées partait à l'envers de la main.
 */
function mouPhoto() {
  const zone = zoneMm();
  if (!zone) return null;
  const g = placeImage(zone, { largeur: calques.naturel_l, hauteur: calques.naturel_h },
    cadrageSaisi(REGLAGES[face].cadrage));
  if (!g) return null;
  return { x: zone.largeur - g.largeur, y: zone.hauteur - g.hauteur, g, zone };
}

/* ---------- le direct ---------- */

/**
 * Empile le papier, la photo et l'habillage à la place que le cadrage courant leur donne.
 *
 * Tout est en pourcentage de sa boîte : l'aperçu s'affiche à la taille que la fenêtre lui
 * laisse, et des pixels calculés ici seraient faux au premier redimensionnement.
 */
function poserDirect() {
  const m = mouPhoto();
  if (!m) return;
  const { zone, g } = m;
  const z = calques.zone;
  $('directPapier').style.setProperty('--papier', calques.papier);
  poser($('directFenetre'), z.x, z.y, z.l, z.h);
  const ph = $('directPhoto');
  if (ph.src !== calques.photo) ph.src = calques.photo;
  // La photo se place en fraction de sa **zone**, pas de la face : c'est la fenêtre qui
  // la rogne, et c'est dans ses coordonnées à elle que Typst la compose.
  poser(ph, g.gauche / zone.largeur, g.haut / zone.hauteur,
    g.largeur / zone.largeur, g.hauteur / zone.hauteur);
  const hab = $('directHabillage');
  if (hab.src !== calques.habillage) hab.src = calques.habillage;
  $('direct').hidden = false;
}

/** Retire le direct : l'aperçu vrai a rattrapé, il n'a plus rien à montrer de plus. */
function masquerDirect() {
  $('direct').hidden = true;
}

/* ---------- les prises ---------- */

/**
 * Pose une boîte sur la face, en fractions de celle-ci.
 *
 * Par variables CSS et non par `style.left` : c'est déjà ce que fait la coupe, et pour
 * la même raison. Des fractions traversent un `calc()`, se relisent dans un test, et
 * survivent à un aperçu affiché à la taille que la fenêtre lui laisse ; des pixels
 * calculés ici seraient faux au premier redimensionnement.
 */
function poser(el, gauche, haut, largeur, hauteur) {
  el.style.setProperty('--gauche', String(gauche));
  el.style.setProperty('--haut', String(haut));
  if (largeur !== undefined) el.style.setProperty('--largeur', String(largeur));
  if (hauteur !== undefined) el.style.setProperty('--hauteur', String(hauteur));
}

/**
 * La hauteur du bloc de texte, en fraction de la face.
 *
 * En mode Bandeau, le bloc ne se règle pas : il se cale dans la bande, à 22 % de sa
 * hauteur — la règle est dans `bloc_texte`, côté Rust. La prise s'y dessine quand même,
 * mais seules ses poignées latérales répondent : c'est la frontière du bandeau qui la
 * déplace, et c'est exactement ce que fait l'atelier HTML.
 */
function hauteurBloc(cv) {
  const r = REGLAGES[face];
  if (face === 'une') {
    return (cv.mode === 'bandeau' ? valeurSaisie('bandeau') * 0.22 : valeurSaisie('bloc_y')) / 100;
  }
  const f = formatCourant();
  return valeurSaisie(r.bloc) / 100 * (f.largeur / f.hauteur);
}

/**
 * Les trois places du dos, dans l'ordre où l'aperçu couché les montre.
 *
 * De gauche à droite : pied, centre, tête. Ce n'est pas un choix d'affichage — c'est ce
 * que produit la double rotation de `source_dos`, et c'est aussi l'ordre de lecture du
 * dos, du début du livre vers sa fin.
 */
const PLACES_DOS_ORDRE = ['pied', 'centre', 'tete'];

/**
 * Les quatre éléments du dos : la clé du modèle, et le suffixe de leurs identifiants.
 *
 * Une seule liste pour les prises, leurs icônes de sens et le câblage des gestes : ces
 * trois-là recopiées à la main ont déjà divergé une fois — l'arrivée de la collection
 * l'a montré, une prise posée sans geste ne fait rien et ne dit rien.
 */
const ELEMENTS_DOS = [
  ['auteur', 'Auteur'], ['titre', 'Titre'],
  ['editeur', 'Editeur'], ['collection', 'Collection'],
];

/**
 * Les éléments qui se couchent **en travers** du dos, et non seulement dans son sens.
 *
 * Une mention d'éditeur ou de collection est courte : couchée en travers, elle se lit le
 * livre debout et tient dans l'épaisseur. Un titre ou un nom d'auteur, non — la longueur
 * de leur ligne y passerait, et l'épaisseur d'un dos vient de la pagination, pas d'un
 * réglage. Ils gardent donc les deux seuls sens qu'un dos leur offre.
 */
const QUARTS_DOS = ['editeur', 'collection'];

/** Toutes les prises du balisage, pour les éteindre d'un bloc avant de choisir. */
const PRISES = ['priseImage', 'priseBloc', 'priseBandeau',
  ...ELEMENTS_DOS.map(([, b]) => `priseDos${b}`)];

/** La place que vise un dépôt, par tiers du dos. */
const placeVisee = (u) => PLACES_DOS_ORDRE[Math.min(2, Math.max(0, Math.floor(u * 3)))];

/** Redessine les prises d'après ce que les contrôles portent. */
function poserPrises() {
  const cv = projet?.couverture;
  const r = REGLAGES[face];
  const boite = $('prises');
  boite.hidden = !cv || !formatCourant() || !(r || face === 'dos');
  // Tout s'éteint d'abord, et chaque face rallume ce qui la concerne : une prise laissée
  // d'une face à l'autre se poserait sur un aperçu qui ne la porte pas — la frontière
  // d'un bandeau en travers du dos, par exemple.
  for (const id of PRISES) $(id).hidden = true;
  $('zonesDos').hidden = true;
  if (boite.hidden) return;
  if (face === 'dos') {
    poserPrisesDos();
    return;
  }

  const pi = $('priseImage');
  pi.hidden = !calques;
  if (calques) {
    const z = calques.zone;
    poser(pi, z.x, z.y, z.l, z.h);
  }

  // Le bloc de la 4ème n'existe que s'il porte quelque chose : une prise posée sur du
  // vide déplacerait un réglage dont rien à l'écran ne montrerait l'effet. Sa tête
  // compte autant que son texte — une 4ème réglée sur son seul titre se déplace aussi.
  const pb = $('priseBloc');
  const teteVide = !['auteur_visible', 'titre_visible', 'filet_visible']
    .some((c) => valeurSaisie(`quatrieme.tete.${c}`));
  pb.hidden = face === 'quatre' && teteVide && !valeurSaisie('quatrieme.texte').trim();
  if (!pb.hidden) {
    const pad = valeurSaisie(r.pad) / 100;
    poser(pb, pad, hauteurBloc(cv), 1 - 2 * pad);
    // La barre du bloc se montre en mode Bandeau — le texte est bien là —, mais elle ne
    // se tire pas : sa hauteur découle de la bande. Seules ses poignées répondent.
    pb.setAttribute('data-figee', face === 'une' && cv.mode === 'bandeau' ? 'oui' : 'non');
  }

  const pbd = $('priseBandeau');
  pbd.hidden = !(face === 'une' && cv.mode === 'bandeau');
  if (!pbd.hidden) poser(pbd, 0, valeurSaisie('bandeau') / 100, 1);
}

/**
 * Les prises du dos : une par texte composé, à l'endroit exact où Typst le pose.
 *
 * Les boîtes viennent du Rust, qui les tient de Typst : la longueur d'une ligne dépend
 * de chaque glyphe, et l'estimer ici poserait la prise à côté du texte. Un dos nu, ou
 * une face montrée avant que la mesure ne soit revenue, n'offre simplement rien.
 */
function poserPrisesDos() {
  for (const [cle, b] of ELEMENTS_DOS) {
    const el = $(`priseDos${b}`);
    const boite = boitesDos.find((x) => x.cle === cle);
    el.hidden = !boite;
    if (!boite) continue;
    poserSens(cle, b);
    // La prise saisie suit la souris ; les deux autres restent où le texte est. Elle
    // seule porte son nom, et seulement le temps du geste : au repos, la prise est posée
    // sur son propre texte, qui se nomme mieux qu'aucun libellé — et un libellé par
    // dessus rendrait les deux illisibles.
    const saisie = geste?.cle === cle;
    el.setAttribute('data-saisi', saisie ? 'oui' : 'non');
    poser(el, boite.debut + (saisie ? geste.glisse.x : 0), 0, boite.fin - boite.debut, 1);
  }
  // Les places ne se montrent que pendant le geste, et celle qui recevrait s'allume.
  const zones = $('zonesDos');
  zones.hidden = !geste?.cle;
  if (geste?.cle) zones.setAttribute('data-cible', placeVisee(geste.u));
}

/**
 * Le sens de lecture de l'auteur et du titre, tel que la flèche le montre.
 *
 * La flèche dit le sens **courant**, et non ce que le clic ferait : c'est le seul
 * endroit de la fenêtre où se lise qu'un dos est montant ou descendant, et un aperçu
 * couché ne le montre pas — les deux s'y lisent de gauche à droite.
 */
function poserSens(cle, b) {
  const bt = $(`sensDos${b}`);
  if (!bt) return;
  const montant = valeurSaisie(`dos.${cle}.sens`) !== 180;
  bt.textContent = montant ? '↑' : '↓';
  bt.title = montant
    ? 'Texte montant, du pied vers la tête — cliquer pour le retourner'
    : 'Texte descendant, de la tête vers le pied — cliquer pour le retourner';
}

/**
 * Tourne un élément du dos d'un quart ou d'un demi-tour.
 *
 * Le modulo avant `poserValeur` n'est pas une précaution : le schéma borne le sens à
 * 270, et un quart de tour de plus s'y arrêterait au lieu de revenir à zéro — la
 * collection resterait couchée en travers du dos sans qu'aucun clic ne l'en sorte.
 */
function tournerDos(cle, quart) {
  const chemin = `dos.${cle}.sens`;
  poserValeur(chemin, (valeurSaisie(chemin) + quart + 360) % 360);
  poserPrises();
  // La promesse est rendue, et c'est ce qui rend le geste éprouvable : le tour n'est
  // fini que lorsque la maquette est revenue, et deux clics enchaînés plus vite que
  // l'aller-retour verraient sinon le premier réécrire le champ que le second a posé.
  return majCouverture();
}

/**
 * Range un élément du dos là où on vient de le déposer.
 *
 * La place se lit au tiers du dos où le curseur a lâché, le rang au nombre de voisins
 * déjà passés. Les trois rangs de chaque place sont ensuite renumérotés d'un bout à
 * l'autre : laisser des trous ferait dépendre l'ordre de nombres qui ne veulent plus
 * rien dire, et deux éléments finiraient par partager un rang — auquel cas c'est le
 * tri du Rust qui trancherait, sans que personne l'ait décidé.
 *
 * Les éléments que le dos ne compose pas — éteints, ou sans texte — ne sont pas touchés :
 * ils n'ont pas de boîte, donc pas de place dans ce que l'on voit, et leur rang ne gêne
 * personne puisqu'ils ne sont pas triés avec les autres.
 */
function deposerDos(cle, u) {
  const ordre = { pied: [], centre: [], tete: [] };
  for (const b of boitesDos) {
    if (b.cle !== cle) ordre[valeurSaisie(`dos.${b.cle}.place`)].push(b.cle);
  }
  const place = placeVisee(u);
  const avant = boitesDos.filter(
    (b) => b.cle !== cle && ordre[place].includes(b.cle) && (b.debut + b.fin) / 2 < u).length;
  ordre[place].splice(avant, 0, cle);
  for (const [p, cles] of Object.entries(ordre)) {
    cles.forEach((c, i) => {
      poserValeur(`dos.${c}.place`, p);
      poserValeur(`dos.${c}.rang`, i + 1);
    });
  }
}

/* ---------- les gestes ---------- */

/**
 * Recompose pendant le geste, à la première pause.
 *
 * Deux attentes se suivaient sur ce chemin — celle de la commande, puis celle de
 * l'aperçu — et une valeur posée mettait 400 ms à revenir en image. Pendant un geste,
 * la seconde tombe à zéro : la première a déjà fait l'attente, et la répéter ne
 * protège plus rien.
 */
function demanderCommit() {
  clearTimeout(attenteCommit);
  attenteCommit = setTimeout(majCouverture, 150);
}

/**
 * Un geste de souris sur une prise : ce qu'il tient, et ce que chaque pixel en fait.
 *
 * `deplace` reçoit le déplacement du curseur **en fraction de la face** — jamais en
 * pixels : l'aperçu s'affiche à la taille que la fenêtre lui laisse, et un geste calé
 * sur des pixels irait deux fois plus vite dans une petite fenêtre.
 */
function saisir(el, { chemins, cle = null, direct = false, prete, deplace, deposer }) {
  el.addEventListener('pointerdown', (e) => {
    if (e.button) return;
    const cadre = $('cadreApercu').getBoundingClientRect();
    if (!cadre.width || !cadre.height) return;
    if (prete && !prete()) return;
    e.preventDefault();
    e.stopPropagation();
    el.setPointerCapture(e.pointerId);

    const depart = Object.fromEntries(chemins.map((c) => [c, valeurSaisie(c)]));
    // Le geste porte ce qu'il traîne et où il en est : `poserPrises` en a besoin pour
    // faire suivre la prise saisie et allumer la place visée, sans que le geste ait à
    // dessiner lui-même.
    geste = { chemins: new Set(chemins), cle, glisse: { x: 0, y: 0 }, u: 0 };
    $('cadreApercu').setAttribute('data-geste', 'oui');
    if (direct) poserDirect();

    const bouger = (ev) => {
      geste.glisse = {
        x: (ev.clientX - e.clientX) / cadre.width,
        y: (ev.clientY - e.clientY) / cadre.height,
      };
      geste.u = (ev.clientX - cadre.left) / cadre.width;
      if (deplace) deplace(geste.glisse, depart);
      poserPrises();
      if (direct) poserDirect();
      // Un geste qui ne pose ses valeurs qu'au dépôt n'a rien à faire recomposer en
      // chemin : le commettre à chaque pause enverrait la maquette inchangée, et
      // marquerait le projet modifié pour un déplacement qui n'a pas encore eu lieu.
      if (deplace) demanderCommit();
    };
    const lacher = () => {
      el.removeEventListener('pointermove', bouger);
      el.removeEventListener('pointerup', lacher);
      el.removeEventListener('pointercancel', lacher);
      if (deposer) deposer(geste.u);
      geste = null;
      $('cadreApercu').removeAttribute('data-geste');
      poserPrises();
      clearTimeout(attenteCommit);
      // Un clic qui n'a rien déplacé ne se commet pas : il marquerait le projet
      // modifié, donc réveillerait la garde à la fermeture, pour avoir posé la souris
      // sur sa propre couverture. La comparaison porte sur les valeurs et non sur le
      // fait qu'un `pointermove` ait eu lieu — un pixel de tremblement, ramené dans les
      // bornes, ne change rien non plus.
      if (chemins.some((c) => valeurSaisie(c) !== depart[c])) majCouverture();
    };
    el.addEventListener('pointermove', bouger);
    el.addEventListener('pointerup', lacher);
    el.addEventListener('pointercancel', lacher);
  });
}

/**
 * Câble les quatre gestes de la couverture. Une fois, au démarrage : les prises sont
 * dans le balisage, seules leurs positions changent d'une face à l'autre.
 */
function cablerPrises() {
  // La photo suit la souris au 1:1 — la référence est le mou réel, pas la largeur de la
  // face. C'est ce qui fait qu'une image à peine plus grande que sa zone se déplace de
  // ce qu'elle peut, et pas d'un dixième de couverture par pixel.
  saisir($('priseImage'), {
    chemins: ['cadrage.x', 'cadrage.y', 'quatrieme.cadrage.x', 'quatrieme.cadrage.y'],
    direct: true,
    prete: () => {
      const m = mouPhoto();
      return !!m && (Math.abs(m.x) >= 0.5 || Math.abs(m.y) >= 0.5);
    },
    deplace: (d, depart) => {
      const r = REGLAGES[face];
      const f = formatCourant();
      const m = mouPhoto();
      if (!m) return;
      // Le mou porte le sens : négatif, la photo déborde de sa zone et tirer vers la
      // droite découvre sa gauche — l'ancrage décroît, comme dans l'atelier HTML ;
      // positif, elle y tient et l'ancrage accompagne le geste. La division fait le
      // reste, la course restant celle du mou et non de la face.
      if (Math.abs(m.x) >= 0.5) {
        poserValeur(`${r.cadrage}.x`, depart[`${r.cadrage}.x`] + d.x * f.largeur / m.x);
      }
      if (Math.abs(m.y) >= 0.5) {
        poserValeur(`${r.cadrage}.y`, depart[`${r.cadrage}.y`] + d.y * f.hauteur / m.y);
      }
    },
  });

  // La frontière du bandeau : tirer vers le bas agrandit la bande.
  saisir($('priseBandeau'), {
    chemins: ['bandeau'],
    deplace: (d, depart) => poserValeur('bandeau', depart.bandeau + d.y * 100),
  });

  // Le corps du bloc de texte : sa hauteur. Figé en mode Bandeau, où elle se déduit de
  // la bande — seules les poignées répondent alors.
  saisir($('priseBloc'), {
    chemins: ['bloc_y', 'quatrieme.top'],
    prete: () => !(face === 'une' && projet.couverture.mode === 'bandeau'),
    deplace: (d, depart) => {
      const r = REGLAGES[face];
      const f = formatCourant();
      // Le bloc de la 4ème se compte en pourcentage de la largeur, celui de la 1ère de
      // la hauteur : le même déplacement vertical n'y vaut pas le même nombre.
      const part = r.bloc_sur === 'hauteur' ? d.y : d.y * f.hauteur / f.largeur;
      poserValeur(r.bloc, depart[r.bloc] + part * 100);
    },
  });

  // Les deux poignées du bloc : la marge latérale, symétrique. Tirer la gauche vers la
  // droite l'élargit ; tirer la droite vers la gauche aussi.
  for (const [id, sens] of [['poigneeGauche', 1], ['poigneeDroite', -1]]) {
    saisir($(id), {
      chemins: ['pad_x', 'quatrieme.pad_x'],
      deplace: (d, depart) => {
        const r = REGLAGES[face];
        poserValeur(r.pad, depart[r.pad] + sens * d.x * 100);
      },
    });
  }

  // Les quatre textes du dos. Rien ne se commet en chemin : la place et le rang n'ont
  // de valeur qu'une fois le doigt levé, et une recomposition par tiers traversé
  // ferait clignoter le dos sous la souris.
  for (const [cle, b] of ELEMENTS_DOS) {
    saisir($(`priseDos${b}`), {
      chemins: ELEMENTS_DOS.flatMap(
        ([c]) => [`dos.${c}.place`, `dos.${c}.rang`]),
      cle,
      deposer: (u) => deposerDos(cle, u),
    });
    // Les icônes de sens vivent dans la prise : sans cet arrêt, presser l'une d'elles
    // commencerait aussi à traîner le texte qui la porte.
    for (const [id, quart] of QUARTS_DOS.includes(cle)
      ? [[`sensDos${b}Gauche`, -90], [`sensDos${b}Droite`, 90]]
      : [[`sensDos${b}`, 180]]) {
      $(id).addEventListener('pointerdown', (e) => e.stopPropagation());
      $(id).addEventListener('click', () => tournerDos(cle, quart));
    }
  }

  // La molette sur la photo : l'échelle. Pas de sélection à faire d'abord — la scène ne
  // défile pas, et la molette n'y a rien d'autre à commander.
  $('priseImage').addEventListener('wheel', (e) => {
    if (!calques) return;
    e.preventDefault();
    const r = REGLAGES[face];
    // Les trois unités de la molette : pixel, ligne, page. Sans cette table, un cran de
    // trackpad et un cran de souris ne feraient pas le même zoom.
    const px = { 0: 1, 1: 16, 2: 400 }[e.deltaMode] ?? 1;
    const d = -e.deltaY * px * 0.001;
    const z = valeurSaisie(`${r.cadrage}.zoom`);
    poserValeur(`${r.cadrage}.zoom`, z + (Math.abs(d) < 0.01 ? Math.sign(d) * 0.01 : d));
    poserPrises();
    poserDirect();
    demanderCommit();
  }, { passive: false });
}

/**
 * Demande au Rust les calques de la face montrée.
 *
 * Après l'aperçu et jamais pendant un geste : c'est une composition de plus, et
 * l'habillage ne dépend pas du cadrage — celui qu'on a vaut pour le geste entier.
 * Silencieux en cas d'échec : la prise de l'image ne s'offre alors pas, ce qui est la
 * bonne réponse et se voit à l'écran.
 */
async function demanderCalques() {
  const demandee = face;
  calques = null;
  boitesDos = [];
  try {
    if (REGLAGES[face]) {
      const c = await invoke('couverture_calques', { face, dosMm: dosCourant() });
      // La face a pu changer pendant la composition : poser ces calques-là sur une autre
      // face y collerait la photo de celle qu'on vient de quitter.
      if (demandee !== face) return;
      calques = c;
    } else if (face === 'dos') {
      const b = await invoke('couverture_dos_boites', { dosMm: dosCourant() });
      if (demandee !== face) return;
      boitesDos = b;
    }
  } catch {
    calques = null;
    boitesDos = [];
  }
  poserPrises();
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
  // Les calques appartiennent à la face qu'on quitte : gardés, la prise de l'image
  // resterait posée là où l'autre face composait la sienne.
  calques = null;
  boitesDos = [];
  [...$('faces').children].forEach((b, i) => {
    b.setAttribute('aria-pressed', String(FACES[i][0] === v));
  });
  if (projet?.couverture) afficherCouverture(projet.couverture);
  else poserDisposition(false);
  demanderApercu();
}

if (typeof module !== 'undefined') {
  module.exports = { SCHEMA, groupes, lire, ecrire, libelleMode, placeImage };
}
