'use strict';

const { invoke } = window.__TAURI__.core;
const { open, save } = window.__TAURI__.dialog;

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
  $('cheminProjet').textContent = p.chemin ?? 'projet non enregistré';
  $('btEnregistrer').disabled = false;
  for (const s of ['secLivre', 'secManuscrit', 'secInterieur', 'secCouverture',
                   'secComposer', 'secPackages', 'secEpreuve']) {
    $(s).hidden = false;
  }

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
  em.textContent = ecart
    ? `${p.chapitres_trouves} chapitres dans le manuscrit embarqué, ${attendu} attendus `
      + '— manuscrit périmé ou contrôle d\'intégrité à corriger.'
    : `${p.chapitres_trouves} chapitres, ${p.mots.toLocaleString('fr-FR')} mots.`;
  em.className = ecart ? 'note alerte' : 'note';

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
    $('etat').textContent = '';
    $('etat').className = 'etat';
    await fn();
  } catch (e) {
    $('etat').textContent = String(e);
    $('etat').className = 'etat erreur';
    // `afficherProjet` ne touche pas `#etat` : le message qu'on vient d'écrire survit.
    if (projet) afficherProjet(projet);
  }
}

async function ouvrir() {
  const choix = await open({
    multiple: false,
    filters: [{ name: 'Projet Ozalid', extensions: ['ozalid'] }],
  });
  if (!choix) return;
  await tente(async () => afficherProjet(await invoke('projet_ouvrir', { chemin: choix })));
}

async function importer() {
  const choix = await open({
    multiple: false,
    filters: [{ name: 'Livre de l\'ancienne chaîne', extensions: ['toml'] }],
  });
  if (!choix) return;
  await tente(async () =>
    afficherProjet(await invoke('projet_importer', { livreToml: choix })));
}

async function enregistrer() {
  const choix = await save({
    defaultPath: `${projet.livre.titre || 'projet'}.ozalid`,
    filters: [{ name: 'Projet Ozalid', extensions: ['ozalid'] }],
  });
  if (!choix) return;
  await tente(async () =>
    afficherProjet(await invoke('projet_enregistrer', { chemin: choix })));
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

$('btOuvrir').addEventListener('click', ouvrir);
$('btImporter').addEventListener('click', importer);
$('btEnregistrer').addEventListener('click', enregistrer);
$('btReimporter').addEventListener('click', reimporter);
$('btChoisirManuscrit').addEventListener('click', choisirManuscrit);
$('btComposer').addEventListener('click', composer);
$('btPackager').addEventListener('click', packager);
$('btEpreuve').addEventListener('click', epreuve);
$('inPoliceInterieur').addEventListener('change', majInterieur);
$('inProvider').addEventListener('change', () => {
  majPapiers();
  // Le format vient du prestataire : l'aperçu change avec lui, même si aucun réglage
  // de maquette n'a bougé.
  demanderApercu();
});
// Le papier ne change ni le format ni la maquette : il ne touche que le dos, et c'est
// pour cela seul que l'aperçu doit repartir.
$('inPapier').addEventListener('change', demanderApercu);
construireFaces();
for (const id of ['inTitre', 'inTitrePage', 'inAuteur', 'inGenre', 'inCopyright', 'inChapitres']) {
  $(id).addEventListener('change', majLivre);
}
chargerProviders();
