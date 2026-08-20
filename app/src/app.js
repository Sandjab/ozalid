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

const nb = (v, d = 2) => v.toLocaleString('fr-FR', {
  minimumFractionDigits: d, maximumFractionDigits: d
});

/* ---------- prestataires ---------- */

async function chargerProviders() {
  providers = await invoke('providers_liste');
  for (const p of providers) $('inProvider').append(new Option(p.libelle, p.cle));
  majPapiers();
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
  for (const s of ['secLivre', 'secManuscrit', 'secComposer']) $(s).hidden = false;

  $('inTitre').value = p.livre.titre;
  $('inTitrePage').value = p.livre.titre_page ?? '';
  $('inAuteur').value = p.livre.auteur;
  $('inGenre').value = p.livre.genre;
  $('inCopyright').value = p.livre.copyright;
  $('inChapitres').value = p.livre.chapitres ?? '';

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

  const morceaux = [];
  if (p.couverture_importee) {
    morceaux.push('Réglages de couverture repris de l\'atelier — le moteur Typst les '
      + 'traduira au jalon 3.');
  }
  if (p.images.length) morceaux.push(`Photos source : ${p.images.join(', ')}.`);
  $('etatCouverture').textContent = morceaux.join(' ');
}

/** Enveloppe commune : affiche l'erreur au lieu de la laisser filer dans la console. */
async function tente(fn) {
  try {
    $('etat').textContent = '';
    $('etat').className = 'etat';
    await fn();
  } catch (e) {
    $('etat').textContent = String(e);
    $('etat').className = 'etat erreur';
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

async function reimporter() {
  await tente(async () => afficherProjet(await invoke('manuscrit_reimporter')));
}

async function choisirManuscrit() {
  const choix = await open({
    multiple: false,
    filters: [{ name: 'Manuscrit Markdown', extensions: ['md', 'markdown', 'txt'] }],
  });
  if (!choix) return;
  await tente(async () =>
    afficherProjet(await invoke('manuscrit_choisir', { chemin: choix })));
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
    afficher(await invoke('composer', {
      providerCle: $('inProvider').value,
      papierCle: $('inPapier').value,
    }));
    $('etat').textContent = '';
  } catch (e) {
    $('etat').textContent = String(e);
    $('etat').className = 'etat erreur';
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
$('inProvider').addEventListener('change', majPapiers);
for (const id of ['inTitre', 'inTitrePage', 'inAuteur', 'inGenre', 'inCopyright', 'inChapitres']) {
  $(id).addEventListener('change', majLivre);
}
chargerProviders();
