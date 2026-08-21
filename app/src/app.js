'use strict';

const { invoke } = window.__TAURI__.core;
const { open, save } = window.__TAURI__.dialog;
const { listen } = window.__TAURI__.event;
const { getCurrentWindow } = window.__TAURI__.window;

const $ = (id) => document.getElementById(id);

/** Construction de DOM sans innerHTML : le contenu vient de fichiers non fiables. */
function h(tag, texte, classe) {
  const el = document.createElement(tag);
  if (texte !== undefined) el.textContent = texte;
  if (classe) el.className = classe;
  return el;
}

let projet = null;
let providers = [];
let polices = [];
let face = 'une';
let attenteApercu = null;
/**
 * Dos de la dernière composition, en mm, avec le prestataire, le papier et la police
 * pour lesquels il vaut.
 *
 * Il n'est jamais saisi : il vient de la pagination mesurée par Typst. C'est ce qui
 * permet à l'aperçu de planche d'être juste, et ce qui le fait refuser de s'afficher
 * tant que l'intérieur n'a pas été composé.
 */
let dosCompose = null;

const nb = (v, d = 2) => v.toLocaleString('fr-FR', {
  minimumFractionDigits: d, maximumFractionDigits: d
});

/* ---------- coquille ---------- */

/**
 * Les quatre étapes, dans l'ordre où le livre se fait : leur clé — celle des entrées
 * `aller.*` du menu, au préfixe près — leur libellé d'onglet, et la section montrée.
 *
 * La table est la seule source : les onglets, le routage du menu et le masquage des
 * sections en sortent tous. Ajouter une étape, c'est une ligne ici, une section dans
 * `index.html` et une entrée dans `menu.rs` — jamais trois listes à tenir d'accord.
 */
const ETAPES = [
  ['livre', '1 · Livre', 'etapeLivre'],
  ['interieur', '2 · Intérieur', 'etapeInterieur'],
  ['couverture', '3 · Couverture', 'etapeCouverture'],
  ['livraison', '4 · Livraison', 'etapeLivraison'],
];

/** L'étape montrée. Sans projet, aucune ne l'est : l'accueil prend leur place. */
let etape = 'livre';

function construireEtapes() {
  for (const [cle, libelle] of ETAPES) {
    const b = h('button');
    b.type = 'button';
    b.id = `onglet-${cle}`;
    b.setAttribute('role', 'tab');
    b.append(h('span', libelle, 'nom'));
    // Le sous-libellé porte l'état de l'étape ; il est retrouvable par son identifiant
    // plutôt que par son rang, pour qu'ajouter un élément à l'onglet ne le déplace pas.
    const sous = h('span', '', 'sous');
    sous.id = `sous-${cle}`;
    b.append(sous);
    b.addEventListener('click', () => allerA(cle));
    $('etapes').append(b);
  }
  // Éteints dès leur naissance, sans attendre le premier projet : un démarrage qui
  // échoue n'affiche jamais rien et ne repasserait donc jamais par ici. Les onglets
  // resteraient d'apparence active sans mener nulle part, et le rang sans onglet
  // sélectionné — l'état que le HTML décrit n'est celui de personne.
  majEtapes();
}

/**
 * Montre une étape.
 *
 * Sans projet, le geste ne fait rien : les onglets sont inertes, mais le menu « Aller »,
 * lui, ne l'est pas. C'est ici que les deux chemins se rejoignent, et c'est le même
 * partage des rôles qu'« Enregistrer » — la protection vit du côté qu'ils ont en commun.
 */
function allerA(cle) {
  if (!projet) return;
  etape = cle;
  majEtapes();
}

/**
 * Ce que chaque onglet dit de son étape : un sous-libellé qui énonce où en est le
 * projet, et un témoin quand l'étape réclame attention.
 *
 * Trois témoins, et pas un de plus. Un manuscrit qui ne correspond plus au contrôle
 * d'intégrité ; une couverture sans maquette ; un dos qui ne vaut plus pour ce qui est
 * affiché, et qui s'allume à l'Intérieur parce que c'est là qu'on le répare. Un
 * manuscrit absent n'en est pas un : c'est l'état d'un projet neuf, pas une anomalie.
 *
 * La signature ment à moitié : l'état de l'Intérieur ne se déduit pas de `p` seul. Le
 * dos mesuré se compare au gabarit, au papier et à la police tels que les contrôles les
 * portent — `dosCourant()` les y lit. D'où deux contraintes sur les appelants : reposer
 * le panneau avant d'appeler, et rappeler après tout geste qui déplace un de ces trois
 * contrôles sans repasser par `afficherProjet`.
 */
function etatEtapes(p) {
  const attendu = p.livre.chapitres;
  const ecart = attendu !== null && attendu !== undefined && attendu !== p.chapitres_trouves;
  // Un dos existe et ne vaut plus : ni « jamais composé », qui ne réclame rien, ni
  // « à jour ».
  const dosPerime = dosCompose !== null && dosCourant() === null;
  return {
    livre: {
      sous: ecart
        ? `${p.chapitres_trouves} chapitres, ${attendu} attendus`
        : (p.manuscrit_absent ? 'aucun manuscrit' : `${p.chapitres_trouves} chapitres`),
      alerte: ecart,
    },
    interieur: {
      sous: dosPerime ? 'dos périmé' : p.interieur.police,
      alerte: dosPerime,
    },
    couverture: {
      sous: p.couverture ? libelleMode(p.couverture.mode) : 'aucune maquette',
      alerte: !p.couverture,
    },
    // Rien de vrai à dire avant qu'un package n'ait été généré, et le pied porte déjà
    // le dos : mieux vaut se taire que meubler.
    livraison: { sous: '', alerte: false },
  };
}

/**
 * Onglets, étapes et accueil remis d'accord avec ce qui est ouvert.
 *
 * Une seule étape est montrée à la fois, et aucune sans projet : l'accueil est un état
 * de l'application, pas un écran de plus posé devant les autres. Les sous-libellés et
 * les témoins s'en vont avec lui : ils parlaient d'un livre qui n'est plus ouvert.
 */
function majEtapes() {
  const etats = projet ? etatEtapes(projet) : null;
  $('accueil').hidden = !!projet;
  for (const [cle, , section] of ETAPES) {
    const onglet = $(`onglet-${cle}`);
    onglet.disabled = !projet;
    onglet.setAttribute('aria-selected', String(!!projet && cle === etape));
    $(section).hidden = !projet || cle !== etape;
    const e = etats?.[cle];
    onglet.className = e?.alerte ? 'alerte' : '';
    $(`sous-${cle}`).textContent = e ? e.sous : '';
  }
}

/**
 * L'erreur va dans l'entête, la seule bande que toutes les étapes partagent.
 *
 * Une erreur de la Livraison doit se lire depuis le Livre : elle ne peut donc pas vivre
 * dans une section que le changement d'étape emporte.
 *
 * Tout ne monte pas ici, et c'est voulu : ce qui refuse une saisie monte, parce que le
 * geste est fini avant qu'on ait bougé et que le message doit survivre au changement
 * d'étape. Ce qui rend compte d'un travail long — composer, tirer une épreuve, générer
 * les packages — reste dans `#etat`, `#etatEpreuve`, `#etatPackages`, à côté du bouton
 * qui l'a lancé : on attend là où l'on a cliqué, et un compte rendu qui migre en haut
 * de l'écran se lit comme une panne. Faire remonter le reste ici par symétrie ferait
 * perdre cette différence.
 */
function alerter(message) {
  $('alerte').textContent = message;
  $('alerte').className = message ? 'etat erreur' : 'etat';
}

/**
 * Le pied : pour qui l'on regarde, et ce que vaut le dos.
 *
 * Le prestataire y est nommé une fois pour toute la fenêtre. Le dos n'y paraît que s'il
 * vaut pour ce qui est montré — c'est `dosCourant()` qui en répond — parce qu'un dos
 * périmé écrit en bas de l'écran est exactement ce qu'on ne relirait pas.
 *
 * Trois états, pas deux : chez un prestataire qui ne publie pas de formule de dos, il
 * n'y a jamais rien à composer, et « non composé » ferait recomposer en boucle un livre
 * dont la pagination est déjà juste. Ce qui manque alors est un relevé sur le gabarit,
 * pas un calcul — c'est le vocabulaire que `noteFormat` emploie déjà pour le fond perdu.
 */
function majPied() {
  // Le prestataire, pas seulement le projet : un démarrage qui n'a pas pu lire les
  // gabarits laisse la liste vide, et le premier projet ouvert ferait lever le pied
  // au lieu de dire ce qu'il sait — c'est-à-dire rien.
  const p = projet ? providerCourant() : null;
  if (!p) {
    $('piedPrestataire').textContent = '';
    return;
  }
  const dos = dosCourant();
  const etat = !p.dos_publie ? 'dos relevé sur le gabarit'
    : dos === null ? 'dos non composé'
      : `dos ${nb(dos, 1)} mm`;
  $('piedPrestataire').textContent = `Vu pour : ${p.libelle} · ${etat}`;
}

/* ---------- prestataires ---------- */

async function chargerProviders() {
  providers = await invoke('providers_liste');
  for (const p of providers) $('inProvider').append(new Option(p.libelle, p.cle));
  majPapiers();
  polices = await invoke('polices_liste');
  for (const p of await invoke('polices_texte_liste')) {
    $('inPoliceInterieur').append(new Option(p, p));
  }
  for (const m of await invoke('maquettes_liste')) {
    const b = h('button', m.libelle);
    b.type = 'button';
    b.addEventListener('click', () => tente(async () =>
      afficherProjet(await invoke('maquette_choisir', { cle: m.cle }))));
    $('maquettes').append(b);
  }
  construireReglages();
  construirePrestataires();
}

function providerCourant() {
  return providers.find((p) => p.cle === $('inProvider').value);
}

function majPapiers() {
  const p = providerCourant();
  const sel = $('inPapier');
  sel.replaceChildren();
  for (const pa of p.papiers) sel.append(new Option(pa.libelle, pa.cle));
  sel.disabled = p.papiers.length < 2;

  const fp = p.fond_perdu === null
    ? 'fond perdu à relever sur le gabarit'
    : `fond perdu ${nb(p.fond_perdu, 3)} mm`;
  $('noteFormat').textContent = `${nb(p.largeur, 1)} × ${nb(p.hauteur, 1)} mm — ${fp}`;
}

/* ---------- projet ---------- */

function afficherProjet(p) {
  projet = p;
  $('titreLivre').textContent = p.livre.titre || 'Sans titre';
  $('cheminProjet').textContent = p.chemin ?? 'projet non enregistré';
  $('etatEnregistrement').textContent = p.modifie
    ? 'modifié'
    : (p.chemin ? 'enregistré' : 'jamais enregistré');

  $('inTitre').value = p.livre.titre;
  $('inTitrePage').value = p.livre.titre_page ?? '';
  $('inAuteur').value = p.livre.auteur;
  $('inGenre').value = p.livre.genre;
  $('inCopyright').value = p.livre.copyright;
  $('inChapitres').value = p.livre.chapitres ?? '';
  $('inPoliceInterieur').value = p.interieur.police;

  const attendu = p.livre.chapitres;
  const ecart = attendu !== null && attendu !== undefined && attendu !== p.chapitres_trouves;
  const em = $('etatManuscrit');
  // Un manuscrit absent et un manuscrit sans chapitre composable comptent tous deux
  // zéro : seul le Rust sait lequel des deux, et ce n'est pas la même chose à faire.
  if (p.manuscrit_absent) {
    em.textContent = 'Aucun manuscrit : en choisir un pour composer le livre.';
    em.className = 'note';
  } else {
    em.textContent = ecart
      ? `${p.chapitres_trouves} chapitres dans le manuscrit embarqué, ${attendu} attendus `
        + '— manuscrit périmé ou contrôle d\'intégrité à corriger.'
      : `${p.chapitres_trouves} chapitres, ${p.mots.toLocaleString('fr-FR')} mots.`;
    em.className = ecart ? 'note alerte' : 'note';
  }

  $('sourceManuscrit').textContent = p.manuscrit_source ?? 'aucune source mémorisée';
  $('btReimporter').disabled = !p.manuscrit_source;

  $('etatImages').textContent = p.images.length
    ? `Photos source : ${p.images.join(', ')}.`
    : 'Aucune photo source : les modes Bandeau et Surimpression composeront sur le papier seul.';

  $('etatCouverture').textContent = p.couverture
    ? ''
    : 'Aucune maquette : en choisir une pour composer la couverture.';
  $('reglages').hidden = !p.couverture;
  if (p.couverture) afficherCouverture(p.couverture);
  demanderApercu();
  majPied();
  // Après les contrôles, jamais avant : le témoin de l'Intérieur compare le dos mesuré
  // à la police *choisie à l'écran*, et un panneau pas encore reposé porte encore la
  // saisie qu'un refus vient d'annuler.
  majEtapes();
}

/**
 * Enveloppe commune : affiche l'erreur au lieu de la laisser filer dans la console, et
 * ramène le panneau à ce que le projet porte vraiment.
 *
 * Ce retour vaut pour tous les appelants, parce qu'ils font tous la même chose : ils
 * envoient une saisie au Rust et n'attendent qu'un projet en retour. Refusée, la saisie
 * n'est nulle part — la laisser à l'écran donnerait à lire un projet qui n'existe pas,
 * et tout ce qui se calcule depuis le panneau, à commencer par le dos de l'aperçu de
 * planche, vaudrait pour ce livre-là.
 */
async function tente(fn) {
  try {
    alerter('');
    await fn();
  } catch (e) {
    alerter(String(e));
    // `afficherProjet` ne touche pas à l'alerte : le message qu'on vient d'écrire y
    // survit au redessin.
    if (projet) afficherProjet(projet);
  }
}

/**
 * Efface ce que le projet précédent avait laissé à l'écran.
 *
 * Pagination, dos, chemins de fichiers : ces chiffres ne valent que pour le livre
 * qui les a produits. Les laisser en place pendant qu'on en ouvre un autre donnerait
 * à lire la pagination du mauvais livre — précisément l'erreur que l'application
 * existe pour supprimer.
 */
function oublierLesSorties() {
  dosCompose = null;
  // L'étape courante est une sortie comme une autre : elle appartenait au projet qu'on
  // regardait. Rester sur la Livraison en ouvrant un autre livre donnerait à lire ses
  // packages sous le titre du nouveau.
  etape = 'livre';
  for (const id of ['resultat', 'packages']) {
    $(id).replaceChildren();
    $(id).hidden = true;
  }
  $('cheminEpreuve').textContent = '';
  // Les quatre canaux de compte rendu, et pas seulement celui de la composition : un
  // message rouge appartient au livre qui l'a provoqué autant que le chiffre qu'il
  // commente. Effacer le chemin de l'épreuve en laissant l'erreur qui disait pourquoi
  // elle avait échoué donnerait à lire l'échec du livre A sous le titre du livre B.
  for (const id of ['etat', 'etatEpreuve', 'etatPackages']) {
    $(id).textContent = '';
    $(id).className = 'etat';
  }
  alerter('');
  // L'aperçu est une sortie comme les autres, et la seule qu'on lise sans la lire :
  // une couverture laissée en place est le genre d'erreur qui ne se remarque qu'une
  // fois la planche partie chez l'imprimeur.
  poserApercu(null);
}

/**
 * L'écran sans projet : les rubriques disparaissent, les récents s'offrent.
 *
 * Appelé au démarrage et après « Fermer le projet ». Il ne se contente pas de vider
 * l'affichage : il remet `projet` à null, faute de quoi l'aperçu continuerait de se
 * composer sur un livre qui n'est plus ouvert.
 */
async function afficherAucunProjet() {
  projet = null;
  oublierLesSorties();
  $('titreLivre').textContent = 'Ozalid Studio';
  $('cheminProjet').textContent = 'aucun projet ouvert';
  $('etatEnregistrement').textContent = '';
  majEtapes();
  await afficherRecents();
  majPied();
}

async function afficherRecents() {
  const box = $('recents');
  box.replaceChildren();
  const liste = await invoke('recents_liste');
  if (liste.length) {
    box.append(h('p', 'Projets récents', 'note'));
    for (const c of liste) {
      const b = h('button', c);
      b.type = 'button';
      b.addEventListener('click', () => ouvrirChemin(c));
      box.append(b);
    }
  }
}

/**
 * La garde : ce qui protège du travail non enregistré.
 *
 * Rend vrai quand l'appelant peut poursuivre. Le Rust pose la question et rend le
 * choix ; l'interface l'exécute, parce qu'elle seule possède le sélecteur de
 * fichiers dont « Enregistrer sous… » a besoin.
 */
async function garde() {
  const choix = await invoke('garde_modifications');
  if (choix === 'enregistrer') return enregistrerQuelquePart();
  if (choix === 'ignorer') return true;
  // « annuler », et tout ce qu'on n'aurait pas compris : le défaut penche du côté
  // qui ne perd rien, comme il le fait déjà côté Rust. Une divergence de
  // vocabulaire entre les deux devient ainsi inoffensive.
  return false;
}

/** Enregistre en place si le projet a un chemin, sinon demande où. Rend vrai si écrit. */
async function enregistrerQuelquePart() {
  // Enregistrer n'est plus qu'un geste de menu, et le menu offre toujours ses entrées
  // sans savoir si un projet est ouvert : c'est ici que la protection doit vivre.
  if (!projet) return false;
  // Ce geste-là n'entre pas dans `tente()` : à lui d'ouvrir sur une ardoise propre,
  // faute de quoi l'échec d'un premier ⌘S survivrait au second, qui a abouti.
  alerter('');
  if (projet.chemin) {
    try {
      afficherProjet(await invoke('projet_enregistrer'));
      return true;
    } catch (e) {
      alerter(String(e));
      return false;
    }
  }
  return enregistrerSous();
}

async function nouveau() {
  if (!await garde()) return;
  await tente(async () => {
    const p = await invoke('projet_nouveau');
    oublierLesSorties();
    afficherProjet(p);
  });
}

async function fermer() {
  if (!await garde()) return;
  await invoke('projet_fermer');
  await afficherAucunProjet();
}

async function ouvrir() {
  if (!await garde()) return;
  const choix = await open({
    multiple: false,
    filters: [{ name: 'Projet Ozalid', extensions: ['ozalid'] }],
  });
  if (!choix) return;
  await ouvrirChemin(choix);
}

async function ouvrirChemin(chemin) {
  await tente(async () => {
    const p = await invoke('projet_ouvrir', { chemin });
    // Après le succès, jamais avant : un projet qu'on n'a pas pu ouvrir laisse
    // intact celui qui l'est, et ses sorties avec lui.
    oublierLesSorties();
    afficherProjet(p);
  });
}

async function importer() {
  if (!await garde()) return;
  const choix = await open({
    multiple: false,
    filters: [{ name: 'Livre de l\'ancienne chaîne', extensions: ['toml'] }],
  });
  if (!choix) return;
  await tente(async () => {
    const p = await invoke('projet_importer', { livreToml: choix });
    oublierLesSorties();
    afficherProjet(p);
  });
}

/** « Enregistrer sous… » : demande où poser le projet. Rend vrai si écrit. */
async function enregistrerSous() {
  // Le menu y mène directement, sans passer par « Enregistrer » : la garde s'y répète,
  // et l'ardoise propre avec elle.
  if (!projet) return false;
  alerter('');
  const choix = await save({
    defaultPath: `${projet.livre.titre || 'projet'}.ozalid`,
    filters: [{ name: 'Projet Ozalid', extensions: ['ozalid'] }],
  });
  if (!choix) return false;
  try {
    afficherProjet(await invoke('projet_enregistrer_sous', { chemin: choix }));
    return true;
  } catch (e) {
    alerter(String(e));
    return false;
  }
}

/* ---------- livre et manuscrit ---------- */

function livre() {
  const chap = $('inChapitres').value.trim();
  const tp = $('inTitrePage').value.trim();
  return {
    titre: $('inTitre').value.trim(),
    titre_page: tp === '' ? null : tp,
    auteur: $('inAuteur').value.trim(),
    genre: $('inGenre').value.trim() || 'roman',
    copyright: $('inCopyright').value,
    chapitres: chap === '' ? null : Number(chap),
  };
}

async function majLivre() {
  await tente(async () =>
    afficherProjet(await invoke('livre_modifier', { livre: livre() })));
}

/**
 * Le manuscrit vient d'être remplacé : le texte fait la pagination, donc le dos. Celui
 * de la dernière composition ne vaut plus rien, et rien dans le panneau ne permettrait
 * de s'en apercevoir — le gabarit, le papier et la police, eux, n'ont pas bougé.
 *
 * Périmé sans regarder si le texte a réellement changé : réimporter un manuscrit
 * identique coûte une recomposition pour rien, comparer deux fois un roman entier à
 * chaque clic coûterait davantage, et se tromper de ce côté-là n'imprime rien de faux.
 */
function manuscritRemplace(p) {
  dosCompose = null;
  afficherProjet(p);
}

async function reimporter() {
  await tente(async () => manuscritRemplace(await invoke('manuscrit_reimporter')));
}

async function choisirManuscrit() {
  const choix = await open({
    multiple: false,
    filters: [{ name: 'Manuscrit Markdown', extensions: ['md', 'markdown', 'txt'] }],
  });
  if (!choix) return;
  await tente(async () =>
    manuscritRemplace(await invoke('manuscrit_choisir', { chemin: choix })));
}

/* ---------- intérieur ---------- */

async function majInterieur() {
  await tente(async () => afficherProjet(await invoke('interieur_modifier', {
    interieur: { police: $('inPoliceInterieur').value },
  })));
}

/* ---------- couverture ---------- */

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
 * Dos à passer à l'aperçu : celui de la composition, et seulement s'il vaut pour ce
 * qui est affiché. Trois choses le déplacent, et il faut les trois : le gabarit, parce
 * que le même manuscrit ne fait pas le même nombre de pages en poche et en grand
 * format ; la police, qui repagine ; le papier, qui change l'épaisseur d'une page sans
 * même toucher à la pagination. La quatrième cause, le texte lui-même, n'a rien à
 * comparer ici : elle périme `dosCompose` au moment du remplacement.
 */
function dosCourant() {
  return dosCompose?.provider === $('inProvider').value
    && dosCompose?.papier === $('inPapier').value
    && dosCompose?.police === $('inPoliceInterieur').value
    ? dosCompose.mm
    : null;
}

/**
 * Pose l'aperçu, ou le retire faute d'image.
 *
 * Retiré pour de bon : une image sans source garde sa place et son fond blanc, et ce
 * rectangle-là ne se distingue pas d'une couverture vide — il donne à voir un livre
 * là où le message dit qu'il n'y en a pas.
 */
function poserApercu(data) {
  const img = $('apercu');
  if (data) img.src = data;
  else img.removeAttribute('src');
  img.hidden = !data;
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
    poserApercu(await invoke('couverture_apercu', {
      face,
      providerCle: $('inProvider').value,
      dosMm: dosCourant(),
      fondPerduMm: null,
    }));
    $('etatApercu').textContent = '';
    $('etatApercu').className = 'note';
  } catch (e) {
    poserApercu(null);
    $('etatApercu').textContent = String(e);
    $('etatApercu').className = 'note alerte';
  }
}

const FACES = [['une', '1ère'], ['quatre', '4ème'], ['planche', 'Planche']];

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
  demanderApercu();
}

/* ---------- composition ---------- */

function afficher(c) {
  const box = $('resultat');
  box.replaceChildren();

  const lignes = [
    ['Pages', String(c.pages)],
    ['Chapitres', String(c.chapitres)],
    ['Gouttière', `${nb(c.gouttiere)} mm`],
    ['Page blanche de fin', c.blanche ? 'ajoutée (parité)' : 'aucune'],
    ['Dos', c.dos === null
      ? 'à relever sur le gabarit du prestataire'
      : `${nb(c.dos)} mm`],
  ];
  const dl = h('dl');
  for (const [k, v] of lignes) dl.append(h('dt', k), h('dd', v));
  box.append(dl);
  box.append(h('p', c.pdf, 'chemin'));
  box.hidden = false;
}

async function composer() {
  const bt = $('btComposer');
  bt.disabled = true;
  $('etat').textContent = 'composition…';
  $('etat').className = 'etat';
  $('resultat').hidden = true;
  try {
    const c = await invoke('composer', {
      providerCle: $('inProvider').value,
      papierCle: $('inPapier').value,
    });
    afficher(c);
    // Le dos sort de la pagination qu'on vient de mesurer : l'aperçu de planche s'en
    // sert tel quel, sans que personne ne le retape.
    dosCompose = c.dos === null ? null : {
      provider: $('inProvider').value,
      papier: $('inPapier').value,
      police: $('inPoliceInterieur').value,
      mm: c.dos,
    };
    if (face === 'planche') demanderApercu();
    $('etat').textContent = '';
  } catch (e) {
    $('etat').textContent = String(e);
    $('etat').className = 'etat erreur';
  } finally {
    bt.disabled = false;
    majPied();
    majEtapes();
  }
}

/* ---------- packages prestataires ---------- */

/** Une ligne par prestataire : la case à cocher, le papier, et les relevés que les
 * prestataires à gabarit exigent — dos et fond perdu, qu'eux seuls ne publient pas. */
function construirePrestataires() {
  const box = $('listePrestataires');
  box.replaceChildren();
  for (const p of providers) {
    const ligne = h('div', undefined, 'prestataire');
    const case_ = h('input');
    case_.type = 'checkbox';
    case_.id = `pkg-${p.cle}`;
    const nom = h('label', p.libelle);
    nom.htmlFor = case_.id;
    ligne.append(case_, nom);

    const papier = h('select');
    papier.id = `pkg-papier-${p.cle}`;
    for (const pa of p.papiers) papier.append(new Option(pa.libelle, pa.cle));
    papier.disabled = p.papiers.length < 2;
    ligne.append(papier);

    if (!p.dos_publie || p.fond_perdu === null) {
      const releve = h('span', undefined, 'releve');
      if (!p.dos_publie) releve.append(champReleve(`pkg-dos-${p.cle}`, 'Dos relevé (mm)', 12));
      if (p.fond_perdu === null) {
        releve.append(champReleve(`pkg-fp-${p.cle}`, 'Fond perdu (mm)', 3));
      }
      ligne.append(releve);
    }
    box.append(ligne);
  }
}

function champReleve(id, libelle, defaut) {
  const l = h('label', undefined, 'petit');
  const i = h('input');
  i.type = 'number';
  i.id = id;
  i.min = 0;
  i.step = 0.1;
  i.value = String(defaut);
  l.append(h('span', libelle), i);
  return l;
}

/** Ce que l'utilisateur a coché, prêt pour la commande. */
function choixPrestataires() {
  const lu = (id) => ($(id) ? Number($(id).value) : null);
  return providers
    .filter((p) => $(`pkg-${p.cle}`)?.checked)
    .map((p) => ({
      providerCle: p.cle,
      papierCle: $(`pkg-papier-${p.cle}`).value,
      dosMm: lu(`pkg-dos-${p.cle}`),
      fondPerduMm: lu(`pkg-fp-${p.cle}`),
    }));
}

function afficherPackages(resultats) {
  const box = $('packages');
  box.replaceChildren();
  for (const r of resultats) {
    const bloc = h('div', undefined, 'package');
    bloc.append(h('h3', r.libelle));
    if (r.erreur) {
      bloc.append(h('p', r.erreur, 'note alerte'));
    } else {
      const p = r.package;
      const dl = h('dl');
      for (const [k, v] of [
        ['Pages', `${p.pages}${p.blanche ? ' (blanche de parité)' : ''}`],
        ['Papier', p.papier],
        ['Gouttière', `${nb(p.gouttiere, 1)} mm`],
        ['Dos', `${nb(p.dos)} mm`],
        ['Planche', `${nb(p.planche[0])} × ${nb(p.planche[1])} mm, `
          + `fond perdu ${nb(p.fond_perdu, 3)} mm`],
      ]) dl.append(h('dt', k), h('dd', v));
      bloc.append(dl);
      for (const c of p.chemins) bloc.append(h('p', c, 'chemin'));
    }
    box.append(bloc);
  }
  box.hidden = false;
}

async function packager() {
  const choix = choixPrestataires();
  if (!choix.length) {
    $('etatPackages').textContent = 'Cocher au moins un prestataire.';
    $('etatPackages').className = 'etat erreur';
    return;
  }
  const bt = $('btPackager');
  bt.disabled = true;
  $('packages').hidden = true;
  $('etatPackages').className = 'etat';
  $('etatPackages').textContent = `composition de ${choix.length} package(s)…`;
  try {
    afficherPackages(await invoke('packager', { choix }));
    $('etatPackages').textContent = '';
  } catch (e) {
    $('etatPackages').textContent = String(e);
    $('etatPackages').className = 'etat erreur';
  } finally {
    bt.disabled = false;
  }
}

/* ---------- épreuve ---------- */

async function epreuve() {
  const bt = $('btEpreuve');
  bt.disabled = true;
  $('cheminEpreuve').textContent = '';
  $('etatEpreuve').className = 'etat';
  $('etatEpreuve').textContent = 'composition…';
  try {
    $('cheminEpreuve').textContent = await invoke('epreuve_tirer', {
      corpsPt: Number($('inEpreuveCorps').value),
    });
    $('etatEpreuve').textContent = '';
  } catch (e) {
    $('etatEpreuve').textContent = String(e);
    $('etatEpreuve').className = 'etat erreur';
  } finally {
    bt.disabled = false;
  }
}

/* ---------- menu natif ---------- */

/**
 * Ce que chaque entrée du menu déclenche.
 *
 * Les valeurs sont les fonctions des boutons, pas des copies : le menu et la souris
 * font la même chose, et la garde des modifications n'a qu'un endroit où vivre.
 */
const MENU = {
  'fichier.nouveau': nouveau,
  'fichier.ouvrir': ouvrir,
  'fichier.importer': importer,
  'fichier.enregistrer': enregistrerQuelquePart,
  'fichier.enregistrer_sous': enregistrerSous,
  'fichier.fermer': fermer,
  'fichier.quitter': quitter,
  // Les quatre étapes viennent de la table : le menu et les onglets appellent la même
  // fonction, et les identifiants du Rust s'en déduisent au lieu d'être recopiés.
  ...Object.fromEntries(ETAPES.map(([cle]) => [`aller.${cle}`, () => allerA(cle)])),
};

/** Préfixe des entrées « Ouvrir un récent » ; ce qui suit est le chemin du projet. */
const RECENT = 'fichier.recent:';

/**
 * Quitter, c'est fermer la fenêtre : l'application n'en a qu'une.
 *
 * `destroy` et surtout pas `close` — `close` repasserait par la fermeture que le Rust
 * retient pour nous poser cette question même, et la fenêtre tournerait en rond.
 */
async function quitter() {
  if (await garde()) getCurrentWindow().destroy();
}

async function routerMenu(id) {
  // Retirer le préfixe, jamais découper sur « : » — un chemin peut en contenir un.
  if (id.startsWith(RECENT)) {
    if (!await garde()) return;
    await ouvrirChemin(id.slice(RECENT.length));
    return;
  }
  const fait = MENU[id];
  if (!fait) {
    // Le Rust et le front se donnent rendez-vous sur des chaînes que ni le compilateur
    // ni le navigateur ne confronte. Avalée, une clé qui ne correspond plus rendrait
    // l'entrée de menu et son accélérateur inertes sans un mot, et c'est l'application
    // entière qui paraîtrait en panne pour une lettre de travers.
    alerter(`entrée de menu inconnue : ${id}`);
    return;
  }
  await fait();
}

/**
 * La fenêtre a demandé à se fermer, et le Rust a retenu la fermeture.
 *
 * C'est ici qu'elle se conclut : la garde d'abord, la destruction ensuite. Le Rust
 * ne peut pas s'en charger — répondre « Enregistrer » demande un sélecteur de
 * fichiers, que seule l'interface possède.
 *
 * `await` et non un simple ordre d'écriture : `listen` rend une promesse, et
 * l'écouteur n'existe côté Rust qu'à sa résolution. Annoncer qu'on écoute avant
 * d'écouter vraiment rouvrirait la fenêtre de temps que ce témoin existe pour fermer.
 */
Promise.all([
  listen('menu', (ev) => routerMenu(ev.payload)),
  listen('fermeture-demandee', quitter),
])
  .then(() => invoke('interface_prete'))
  .catch((e) => {
    // Sans écouteurs, le menu et la fermeture ne mènent nulle part. Le Rust s'en
    // tire — faute de témoin, il ne retient rien et l'application reste quittable —
    // mais l'utilisateur mérite de savoir pourquoi la moitié des gestes est inerte.
    alerter(`menu inopérant : ${e}`);
  });

$('btNouveau').addEventListener('click', nouveau);
$('btOuvrir').addEventListener('click', ouvrir);
$('btImporter').addEventListener('click', importer);
$('btReimporter').addEventListener('click', reimporter);
$('btChoisirManuscrit').addEventListener('click', choisirManuscrit);
$('btImageUne').addEventListener('click', () => choisirImage('une'));
$('btImageQuatre').addEventListener('click', () => choisirImage('quatre'));
$('btComposer').addEventListener('click', composer);
$('btPackager').addEventListener('click', packager);
$('btEpreuve').addEventListener('click', epreuve);
$('inPoliceInterieur').addEventListener('change', majInterieur);
$('inProvider').addEventListener('change', () => {
  majPapiers();
  majPied();
  // Le témoin du dos ne se déduit pas du projet : il tient au gabarit choisi, que ce
  // geste déplace sans repasser par `afficherProjet`.
  majEtapes();
  // Le format vient du prestataire : l'aperçu change avec lui, même si aucun réglage
  // de maquette n'a bougé.
  demanderApercu();
});
// Le papier ne change ni le format ni la maquette : il ne touche que le dos, et c'est
// pour cela seul que l'aperçu, le pied et le témoin de l'Intérieur doivent repartir.
$('inPapier').addEventListener('change', () => {
  majPied();
  majEtapes();
  demanderApercu();
});
construireEtapes();
construireFaces();
for (const id of ['inTitre', 'inTitrePage', 'inAuteur', 'inGenre', 'inCopyright', 'inChapitres']) {
  $(id).addEventListener('change', majLivre);
}
chargerProviders()
  .then(afficherAucunProjet)
  .catch((e) => {
    // Sans les gabarits ni les polices, rien de ce que l'application propose n'a de
    // sens : mieux vaut le dire que d'offrir un écran d'accueil qui ne mène nulle part.
    alerter(`démarrage impossible : ${e}`);
  });
