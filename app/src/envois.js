'use strict';

/**
 * L'étape Envois : la main du livre, l'accès au modèle, la liste des dédicataires
 * et leurs exemplaires.
 *
 * Même partage que `couverture.js` et `livraison.js` : ce fichier ne pose aucun
 * écouteur et ne lit pas le DOM au chargement. Il définit, `app.js` branche — c'est
 * ce qui permet à tous de vivre dans le même contexte global sans dépendre de
 * l'ordre de chargement.
 */

/** D'où vient l'écriture des envois de ce livre : `police`, `image` ou `diffusion`. */
function main() {
  return projet.envois.main.mode;
}

/**
 * Enregistre l'accès au modèle, et rend compte de ce qui est en place.
 *
 * `cle` vaut `null` pour laisser celle qui est enregistrée — le champ est vide à
 * l'écran puisqu'on ne la lui redonne jamais, et corriger l'adresse ne doit pas
 * l'effacer. La chaîne vide, elle, l'oublie pour de bon.
 */
async function reglerDiffusion(cle) {
  await tente(async () => {
    afficherDiffusion(await invoke('diffusion_regler', { url: $('inDiffusionUrl').value, cle }));
    $('inDiffusionCle').value = '';
  });
}

/** Ce que la machine sait du modèle : son adresse, et si une clé y est posée. */
function afficherDiffusion(acces) {
  $('inDiffusionUrl').value = acces.url;
  $('etatDiffusion').textContent = acces.cle_posee
    ? 'clé enregistrée sur cette machine'
    : 'aucune clé : la génération sera refusée';
}

/**
 * Choisit l'image écrite à la main d'un envoi.
 *
 * Elle est copiée dans le `.ozalid` sous `envois/`, à part de celles de la couverture :
 * là-bas, une image dont le nom ne commence pas par `quatrieme` **devient** la première
 * de couverture, et le mot manuscrit d'un lecteur remplacerait la couverture du livre.
 */
async function choisirImageEnvoi(index) {
  const chemin = await open({
    multiple: false,
    filters: [{ name: 'Mot écrit à la main', extensions: ['jpg', 'jpeg', 'png'] }],
  });
  if (!chemin) return;
  await tente(async () =>
    afficherProjet(await invoke('envoi_image_choisir', { index, chemin })));
}

/**
 * Le choix de la main : les trois écritures de la maison, et celle de l'auteur.
 *
 * Le `select` est refait à chaque projet plutôt que rempli une fois au démarrage : la
 * police personnelle appartient au livre ouvert, elle entre et sort avec lui. Sa valeur
 * est reposée depuis le projet — sans quoi le menu montrerait la première main pendant
 * que le livre en compose une autre, et le premier réglage de l'écran l'imposerait.
 */
function afficherMain() {
  const sel = $('inMain');
  const perso = projet.envois.personnelle;
  sel.replaceChildren();
  // Les écritures et les formes dans une seule liste, préfixées : la question posée est
  // « d'où vient l'écriture », et elle n'a qu'une réponse à la fois. Sans préfixe, une
  // police qui s'appellerait « image » désignerait l'autre forme.
  for (const m of mains) sel.append(new Option(m, `police:${m}`));
  if (perso) sel.append(new Option(`${perso} (votre police)`, `police:${perso}`));
  sel.append(new Option('Image écrite à la main', 'image'));
  sel.append(new Option('Image générée', 'diffusion'));
  sel.value = main() === 'police' ? `police:${projet.envois.main.police}` : main();

  // Le gabarit appartient au livre : il se relit du projet, comme la maquette.
  $('diffusion').hidden = main() !== 'diffusion';
  $('inGabarit').value = projet.envois.main.gabarit ?? '';
  $('etatPolice').textContent = perso
    ? `Police personnelle embarquée : ${perso}.`
    : 'Aucune police personnelle : les envois s\'écrivent dans une main de la maison.';
  $('btPoliceRetirer').disabled = !perso;
}

/**
 * La liste des envois : un dédicataire, son mot, et de quoi le voir ou le retirer.
 *
 * Le mot est un `textarea` : un envoi tient en deux ou trois lignes, et un `input` en
 * cacherait la fin — or c'est précisément ce qui sera imprimé.
 */
function afficherEnvois() {
  afficherMain();
  const box = $('envois');
  box.textContent = '';
  for (const [i, e] of projet.envois.liste.entries()) {
    const ligne = h('div', undefined, 'destinataire');

    const qui = document.createElement('input');
    qui.type = 'text';
    qui.value = e.dedicataire;
    qui.setAttribute('aria-label', `Dédicataire ${i + 1}`);
    qui.addEventListener('change', () => reglerEnvoi(i, { dedicataire: qui.value }));

    // Le mot change de nature avec la main : un texte à composer, une image à choisir,
    // ou ce qu'on demande au modèle. La ligne ne porte que ce que la main réclame — un
    // champ grisé sous une main en images donnerait à croire qu'on peut y écrire.
    const mot = main() === 'image' ? imageEnvoi(i, e) : motEnvoi(i, e);

    const voir = h('button', 'Voir la page');
    voir.type = 'button';
    voir.addEventListener('click', () => apercuEnvoi(i));

    const retirer = h('button', 'Retirer');
    retirer.type = 'button';
    retirer.addEventListener('click', () => envoisModifier(
      projet.envois.liste.filter((_, n) => n !== i)));

    ligne.append(qui, mot);
    // Deux gestes, et deux seulement : demander une image, et la retenir. C'est le
    // second qui la fait entrer dans le livre — avant lui, rien n'est figé, et l'on peut
    // regénérer autant qu'il faut. Un modèle de diffusion rend rarement une écriture
    // lisible du premier coup.
    if (main() === 'diffusion') ligne.append(...gestesDeDiffusion(i, e));
    ligne.append(voir, retirer);
    box.append(ligne);
  }
  $('btEnvoyer').disabled = projet.envois.liste.length === 0;
}

/**
 * Le mot d'un envoi, en toutes lettres.
 *
 * Un `textarea` : un envoi tient en deux ou trois lignes, et un `input` en cacherait la
 * fin — or c'est précisément ce qui sera imprimé.
 */
function motEnvoi(i, e) {
  const mot = document.createElement('textarea');
  mot.rows = 2;
  mot.value = e.contenu;
  mot.setAttribute('aria-label', `Mot pour ${e.dedicataire || 'ce dédicataire'}`);
  mot.addEventListener('change', () => reglerEnvoi(i, { contenu: mot.value }));
  return mot;
}

/**
 * L'image d'un envoi : le bouton qui la choisit, et le nom qu'elle porte dans l'archive.
 *
 * Ce nom-là et pas celui du fichier d'origine : c'est celui que la source Typst écrit,
 * et le seul qui dise laquelle des images est partie avec quel exemplaire.
 */
function imageEnvoi(i, e) {
  const bt = h('button', e.image ? `Image : ${e.image}` : 'Choisir une image…');
  bt.type = 'button';
  bt.id = `envoi-image-${i}`;
  bt.addEventListener('click', () => choisirImageEnvoi(i));
  return bt;
}

/**
 * Demander une image au modèle, puis la retenir.
 *
 * « Retenir » est éteint tant que rien n'a été généré dans cette ligne : c'est le geste
 * qui fige l'image dans le `.ozalid`, et il n'a pas d'objet avant qu'on ait regardé.
 */
function gestesDeDiffusion(i, e) {
  const generer = h('button', 'Générer');
  generer.type = 'button';
  generer.id = `envoi-generer-${i}`;
  generer.addEventListener('click', () => genererEnvoi(i));

  const accepter = h('button', e.image ? `Retenue : ${e.image}` : 'Retenir');
  accepter.type = 'button';
  accepter.id = `envoi-accepter-${i}`;
  accepter.disabled = candidat !== i;
  accepter.addEventListener('click', () => accepterEnvoi(i));
  return [generer, accepter];
}

/**
 * Demande l'image, et la montre sans la garder.
 *
 * Le Rust la tient de côté jusqu'à ce qu'on la retienne : l'archive n'a pas à conserver
 * la suite des essais, et un livre fermé entre-temps les laisse là où ils étaient.
 */
async function genererEnvoi(i) {
  const img = $('apercuEnvoi');
  $('etatEnvois').className = 'etat';
  $('etatEnvois').textContent = 'le modèle compose…';
  try {
    img.src = await invoke('envoi_generer', { index: i });
    img.alt = `Image proposée pour l'exemplaire de ${projet.envois.liste[i].dedicataire}`;
    img.hidden = false;
    candidat = i;
    $('etatEnvois').textContent = '';
    afficherEnvois();
  } catch (e) {
    $('etatEnvois').textContent = String(e);
    $('etatEnvois').className = 'etat erreur';
  }
}

/** Fige l'image proposée : elle entre dans le livre, et n'en bouge plus. */
async function accepterEnvoi(i) {
  await tente(async () => {
    const vue = await invoke('envoi_accepter', { index: i });
    candidat = null;
    afficherProjet(vue);
  });
}

/** Remplace un envoi par lui-même modifié. */
function reglerEnvoi(i, sur) {
  return envoisModifier(
    projet.envois.liste.map((e, n) => (n === i ? { ...e, ...sur } : e)));
}

/**
 * Envoie la liste **et la main** : la commande remplace l'objet entier, et une main
 * omise reviendrait au défaut — tous les exemplaires changeraient d'écriture sans que
 * personne ne l'ait demandé.
 */
async function envoisModifier(liste) {
  await tente(async () => afficherProjet(await invoke('envois_modifier', {
    envois: { main: projet.envois.main, liste },
  })));
}

/**
 * La page de titre de cet envoi, telle qu'elle sera imprimée.
 *
 * C'est la seule façon de voir qu'un mot déborde : le compte de pages, lui, ne bougera
 * pas — c'est tout l'objet du `#place`, et c'est aussi ce qui rend un débordement
 * silencieux.
 */
async function apercuEnvoi(i) {
  const img = $('apercuEnvoi');
  await tente(async () => {
    img.src = await invoke('envoi_apercu', { index: i });
    img.alt = `Page de titre de l'exemplaire de ${projet.envois.liste[i].dedicataire}`;
    img.hidden = false;
  });
}

async function envoyer() {
  const bt = $('btEnvoyer');
  bt.disabled = true;
  $('resultatEnvois').hidden = true;
  $('etatEnvois').className = 'etat';
  $('etatEnvois').textContent = `composition de ${projet.envois.liste.length} envoi(s)…`;
  try {
    afficherResultatEnvois(await invoke('envoyer'));
    $('etatEnvois').textContent = '';
  } catch (e) {
    $('etatEnvois').textContent = String(e);
    $('etatEnvois').className = 'etat erreur';
  } finally {
    bt.disabled = false;
  }
}

/**
 * Ce qui a été écrit, pour qui, et où le trouver.
 *
 * Le répertoire montré est celui qui a réellement été écrit, assaini : c'est celui-là
 * qu'il faut ouvrir, et il ne porte pas toujours le nom saisi.
 */
function afficherResultatEnvois(resultats) {
  const box = $('resultatEnvois');
  box.textContent = '';
  for (const r of resultats) {
    const bloc = h('div', undefined, 'package');
    bloc.append(h('h3', r.dedicataire || 'sans nom'));
    bloc.append(h('p', `envois/${r.dossier}/ — ${r.package.pages} pages, dos `
      + `${r.package.dos.toFixed(2)} mm`, 'chemin'));
    if (r.vignette) {
      const img = h('img', undefined, 'vignette');
      img.src = r.vignette;
      img.alt = `Planche de l'exemplaire de ${r.dedicataire}`;
      bloc.append(img);
    }
    box.append(bloc);
  }
  box.hidden = false;
}
