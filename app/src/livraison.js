'use strict';

/**
 * L'étape Livraison : les destinataires, les packages, et bientôt les envois.
 *
 * Même partage que `couverture.js` : ce fichier ne pose aucun écouteur et ne lit pas
 * le DOM au chargement. Il définit, `app.js` branche — c'est ce qui permet aux deux de
 * vivre dans le même contexte global sans dépendre de l'ordre de chargement.
 */

/**
 * La liste des destinataires du livre, et de quoi en ajouter un.
 *
 * Une ligne par destinataire : son papier, le format de son gabarit, et les relevés que
 * les prestataires à gabarit exigent — dos et fond perdu, qu'eux seuls ne publient pas.
 * Plus de cases à cocher : être dans la liste *est* le fait d'être destinataire, et le
 * prestataire n'est plus désigné deux fois.
 */
function afficherDestinataires() {
  const box = $('destinataires');
  box.replaceChildren();
  const declares = projet.livraison.destinataires;
  for (const d of declares) {
    const p = providers.find((pr) => pr.cle === d.provider);
    const ligne = h('div', undefined, 'destinataire');
    ligne.append(h('span', libelleProvider(d.provider), 'nom'));

    if (p) {
      const papier = h('select');
      papier.id = `dest-papier-${d.provider}`;
      for (const pa of p.papiers) papier.append(new Option(pa.libelle, pa.cle));
      papier.value = d.papier;
      papier.disabled = p.papiers.length < 2;
      papier.addEventListener('change', () => reglerDestinataire(d.provider));
      ligne.append(papier);

      if (!p.dos_publie || p.fond_perdu === null) {
        const releve = h('span', undefined, 'releve');
        const champ = (quoi, libelle, valeur) =>
          releve.append(champReleve(`dest-${quoi}-${d.provider}`, libelle, valeur, d.provider));
        if (!p.dos_publie) champ('dos', 'Dos relevé (mm)', d.dos_mm);
        if (p.fond_perdu === null) champ('fp', 'Fond perdu (mm)', d.fond_perdu_mm);
        ligne.append(releve);
      }
      ligne.append(h('span', noteFormat(p), 'note'));
    }

    const retirer = h('button', 'Retirer');
    retirer.type = 'button';
    retirer.id = `dest-retirer-${d.provider}`;
    // Le dernier ne se retire pas : le Rust refuse, mais un bouton qui ne peut
    // qu'échouer vaut mieux éteint que refusé.
    retirer.disabled = declares.length < 2;
    retirer.addEventListener('click', () => tente(async () =>
      afficherProjet(await invoke('destinataire_retirer', { providerCle: d.provider }))));
    ligne.append(retirer);
    box.append(ligne);
  }

  // Ne s'ajoute que ce qui n'est pas déjà là : la même clé deux fois n'aurait aucun
  // sens, et le Rust le refuserait.
  const sel = $('inAjoutDestinataire');
  const restants = providers.filter((p) => !declares.some((d) => d.provider === p.cle));
  sel.replaceChildren();
  for (const p of restants) sel.append(new Option(p.libelle, p.cle));
  sel.disabled = restants.length === 0;
  $('btAjouterDestinataire').disabled = restants.length === 0;
}

function noteFormat(p) {
  const fp = p.fond_perdu === null
    ? 'fond perdu à relever sur le gabarit'
    : `fond perdu ${nb(p.fond_perdu, 3)} mm`;
  return `${nb(p.largeur, 1)} × ${nb(p.hauteur, 1)} mm — ${fp}`;
}

/**
 * Un relevé fait sur le gabarit du prestataire.
 *
 * Vide au départ, jamais prérempli : un chiffre par défaut se lirait comme une mesure,
 * et une planche composée sur un dos inventé ne se voit qu'au massicot.
 */
function champReleve(id, libelle, valeur, providerCle) {
  const l = h('label', undefined, 'petit');
  const i = h('input');
  i.type = 'number';
  i.id = id;
  i.min = 0;
  i.step = 0.1;
  i.value = valeur === null || valeur === undefined ? '' : String(valeur);
  i.addEventListener('change', () => reglerDestinataire(providerCle));
  l.append(h('span', libelle), i);
  return l;
}

/** Relit la ligne d'un destinataire et la renvoie au projet. */
async function reglerDestinataire(cle) {
  // Un champ vide est une absence de relevé, pas un zéro : composer sur un dos nul
  // produirait une planche fausse au lieu d'un refus.
  const lu = (id) => {
    const v = $(id)?.value.trim();
    return v ? Number(v) : null;
  };
  await tente(async () => afficherProjet(await invoke('destinataire_regler', {
    destinataire: {
      provider: cle,
      papier: $(`dest-papier-${cle}`).value,
      dos_mm: lu(`dest-dos-${cle}`),
      fond_perdu_mm: lu(`dest-fp-${cle}`),
    },
  })));
}

/**
 * Les fichiers d'un package : leur répertoire une fois, leurs noms ensuite.
 *
 * Un package écrit tous ses fichiers au même endroit, et redire soixante-dix caractères
 * de chemin identiques à chaque ligne coûtait deux lignes de plus par destinataire —
 * l'ascenseur de la Livraison se payait en redites. Coupés ainsi, les noms tiennent sur
 * une ligne au lieu de se replier au milieu d'un mot.
 *
 * Si les fichiers ne partagent pas leur répertoire, chacun reprend son chemin entier :
 * un chemin long se lit, un chemin faux se suit jusqu'à un fichier qui n'y est pas. Les
 * deux séparateurs sont reconnus — l'application est aussi empaquetée pour Windows, et
 * un `\` pris pour une lettre rendrait le groupement muet là-bas.
 */
function cheminsGroupes(chemins) {
  const dossier = (c) => c.slice(0, Math.max(c.lastIndexOf('/'), c.lastIndexOf('\\')) + 1);
  const commun = chemins.length ? dossier(chemins[0]) : '';
  if (!commun || !chemins.every((c) => dossier(c) === commun)) return chemins;
  return [commun, chemins.map((c) => c.slice(commun.length)).join('   ')];
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
      // Les chiffres et les chemins d'un côté, la vignette de l'autre : ce qui
      // s'empilait tient désormais côte à côte, et la hauteur d'un compte rendu est
      // celle de sa planche au lieu d'en être la somme.
      const infos = h('div', undefined, 'infos');
      infos.append(dl);
      for (const c of cheminsGroupes(p.chemins)) infos.append(h('p', c, 'chemin'));
      bloc.append(infos);
      // La planche telle qu'elle part à l'impression, avec le dos mesuré de ce
      // prestataire-là : c'est ici que « est-ce que ça tient » se vérifie, sur du vrai
      // et non sur une approximation qu'on espère fidèle.
      if (r.vignette) {
        const img = h('img', undefined, 'vignette');
        img.src = r.vignette;
        img.alt = `Planche composée pour ${r.libelle}`;
        bloc.append(img);
      }
    }
    box.append(bloc);
  }
  box.hidden = false;
}

async function packager() {
  const combien = projet.livraison.destinataires.length;
  const bt = $('btPackager');
  bt.disabled = true;
  $('packages').hidden = true;
  $('etatPackages').className = 'etat';
  $('etatPackages').textContent = `composition de ${combien} package(s)…`;
  try {
    afficherPackages(await invoke('packager'));
    $('etatPackages').textContent = '';
  } catch (e) {
    $('etatPackages').textContent = String(e);
    $('etatPackages').className = 'etat erreur';
  } finally {
    bt.disabled = false;
  }
}

/* ---------- envois ---------- */

/** D'où vient l'écriture des envois de ce livre : `police` ou `image`. */
function main() {
  return projet.envois.main.mode;
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
  sel.value = main() === 'image' ? 'image' : `police:${projet.envois.main.police}`;
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

    // Le mot change de nature avec la main : un texte à composer, ou une image à
    // choisir. La ligne ne porte que ce que la main réclame — un champ de texte grisé
    // sous une main en images donnerait à croire qu'on peut encore y écrire.
    const mot = main() === 'image' ? imageEnvoi(i, e) : motEnvoi(i, e);

    const voir = h('button', 'Voir la page');
    voir.type = 'button';
    voir.addEventListener('click', () => apercuEnvoi(i));

    const retirer = h('button', 'Retirer');
    retirer.type = 'button';
    retirer.addEventListener('click', () => envoisModifier(
      projet.envois.liste.filter((_, n) => n !== i)));

    ligne.append(qui, mot, voir, retirer);
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
