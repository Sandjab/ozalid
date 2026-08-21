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
    face: 'planche',
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
    face: 'planche',
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

if (typeof module !== 'undefined') {
  module.exports = { SCHEMA, groupes, lire, ecrire, libelleMode };
}
