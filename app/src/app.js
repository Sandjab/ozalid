'use strict';

const { invoke } = window.__TAURI__.core;
const { open } = window.__TAURI__.dialog;

const $ = (id) => document.getElementById(id);

/** Construction de DOM sans innerHTML : le contenu vient de fichiers non fiables. */
function h(tag, texte, classe) {
  const el = document.createElement(tag);
  if (texte !== undefined) el.textContent = texte;
  if (classe) el.className = classe;
  return el;
}

let manuscrit = null;
let providers = [];

const nb = (v, d = 2) => v.toLocaleString('fr-FR', {
  minimumFractionDigits: d, maximumFractionDigits: d
});

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

async function choisirManuscrit() {
  const choix = await open({
    multiple: false,
    filters: [{ name: 'Manuscrit Markdown', extensions: ['md', 'markdown', 'txt'] }],
  });
  if (!choix) return;
  manuscrit = choix;
  $('cheminManuscrit').textContent = choix;
  $('btComposer').disabled = false;
}

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
  for (const [k, v] of lignes) {
    dl.append(h('dt', k), h('dd', v));
  }
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
    const sortie = manuscrit.replace(/[^/\\]+$/, '') + 'out';
    const c = await invoke('composer', {
      manuscritPath: manuscrit,
      livre: livre(),
      providerCle: $('inProvider').value,
      papierCle: $('inPapier').value,
      sortie,
    });
    $('etat').textContent = '';
    afficher(c);
  } catch (e) {
    $('etat').textContent = String(e);
    $('etat').className = 'etat erreur';
  } finally {
    bt.disabled = false;
  }
}

$('btChoisir').addEventListener('click', choisirManuscrit);
$('btComposer').addEventListener('click', composer);
$('inProvider').addEventListener('change', majPapiers);
chargerProviders();
