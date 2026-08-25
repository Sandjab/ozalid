'use strict';

/**
 * L'étape Envois : la liste des dédicataires, et l'exemplaire qu'on règle.
 *
 * Quatre bandes qui ne parlent que d'une personne à la fois — la liste dit qui, le
 * rail dit quelle page, le canevas montre, la colonne de droite règle. C'est ce qui
 * distingue cette étape de la liste-formulaire qu'elle remplace : vingt lignes portant
 * chacune leur `textarea` ne se lisaient plus.
 *
 * Même partage que `couverture.js` et `livraison.js` : ce fichier ne pose aucun
 * écouteur et ne lit pas le DOM au chargement. Il définit, `app.js` branche — c'est
 * ce qui permet à tous de vivre dans le même contexte global sans dépendre de
 * l'ordre de chargement.
 */

/**
 * Le rang du dédicataire dont on règle l'exemplaire.
 *
 * Le rang plutôt que l'objet : la liste se refait à chaque retour du projet, et un
 * objet retenu serait celui d'avant — on réglerait un exemplaire en croyant en régler
 * un autre.
 */
let choisi = 0;

/** L'envoi qu'on règle, ou `null` quand la liste est vide. */
function envoi() {
  return projet.envois.liste[choisi] ?? null;
}

/** D'où vient l'écriture de l'exemplaire ouvert : `police`, `image` ou `diffusion`. */
function main() {
  return envoi()?.main.mode ?? 'police';
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

/* ---------- la liste ---------- */

/**
 * La liste des dédicataires : un nom par ligne, celui qu'on règle marqué.
 *
 * Un envoi sans nom porte son rang plutôt que rien : trois lignes vides se
 * confondraient, et l'on réglerait l'exemplaire d'un autre.
 */
function afficherEnvois() {
  const liste = projet.envois.liste;
  // Un retrait peut avoir laissé le rang au-delà de la liste : le ramener plutôt que
  // d'ouvrir un exemplaire qui n'existe plus.
  choisi = Math.min(choisi, Math.max(0, liste.length - 1));

  const box = $('envois');
  box.textContent = '';
  for (const [i, e] of liste.entries()) {
    const li = h('li', e.dedicataire || `envoi ${i + 1}`);
    li.setAttribute('role', 'option');
    li.setAttribute('aria-selected', String(i === choisi));
    li.addEventListener('click', () => choisir(i));
    box.append(li);
  }
  afficherReglages();
  $('btEnvoyer').disabled = liste.length === 0;
}

/** Ouvre l'exemplaire d'un autre dédicataire : les trois autres bandes le suivent. */
function choisir(i) {
  choisi = i;
  afficherEnvois();
  ouvrirCanevas();
}

/**
 * Rend ce que les trois bandes de droite montrent : le rail, la page, l'objet.
 *
 * Appelé en arrivant à l'étape et à chaque changement de dédicataire, jamais à
 * l'ouverture du projet : rendre les pages coûte une composition, et la payer à qui
 * vient regarder une couverture serait le prix de ce qu'il n'a pas demandé.
 */
function ouvrirCanevas() {
  majVignettes();
  majPage();
  majObjet();
}

/* ---------- les réglages de l'exemplaire ouvert ---------- */

/**
 * Ce que la main réclame, et rien d'autre.
 *
 * Un champ grisé sous une main en images donnerait à croire qu'on peut y écrire ; c'est
 * pourquoi les trois formes ne se montrent pas ensemble.
 */
function afficherReglages() {
  const e = envoi();
  for (const id of ['inMain', 'inMot', 'inTaille', 'inAngle', 'btVoirPage',
    'btRetirerEnvoi']) {
    $(id).disabled = !e;
  }
  $('champImage').hidden = !e || main() !== 'image';
  // Les seuils suivent l'image, et non la seule main : un envoi d'avant ce chantier
  // n'en porte pas, et deux curseurs sans valeur à régler ne diraient rien.
  $('champDetourage').hidden = !e || main() !== 'image' || !e.detourage;
  $('champDiffusion').hidden = !e || main() !== 'diffusion';
  // La main générée garde son mot : le gabarit dit le style du livre, le mot dit ce que
  // cette image-ci doit porter, et c'est lui que `{envoi}` va chercher. Seule la forme
  // en images n'a pas de texte à composer.
  $('champMot').hidden = !!e && main() === 'image';
  // Ce qui appartient au livre se pose **avant** la sortie : un livre sans dédicataire a
  // quand même sa police personnelle, et la laisser dans l'état du livre précédent
  // offrirait de retirer une écriture que celui-ci ne porte pas.
  afficherLivre();
  if (!e) return;

  afficherMain();
  $('inMot').value = e.contenu;
  $('btImageEnvoi').textContent = e.image ? `Image : ${e.image}` : 'Choisir une image…';
  $('btAccepter').textContent = e.image ? `Retenue : ${e.image}` : 'Retenir';
  // « Retenir » est éteint tant que rien n'a été généré pour cette ligne : c'est le
  // geste qui fige l'image dans le `.ozalid`, et il n'a pas d'objet avant qu'on ait
  // regardé. Un modèle de diffusion rend rarement une écriture lisible du premier coup.
  $('btAccepter').disabled = candidat !== choisi;

  if (e.detourage) {
    for (const [id, val, v] of [['inPapier', e.detourage.papier, 'vPapier'],
      ['inEncre', e.detourage.encre, 'vEncre']]) {
      const n = Math.round(val);
      $(id).value = n;
      $(v).textContent = String(n);
    }
  }

  const taille = Math.round(e.place.taille * 100);
  $('inTaille').value = taille;
  $('vTaille').textContent = `${taille} %`;
  const angle = Math.round(e.place.angle);
  $('inAngle').value = angle;
  $('vAngle').textContent = `${angle}°`;
}

/**
 * Le choix de la main : les trois écritures de la maison, et celle de l'auteur.
 *
 * Le `select` est refait à chaque projet plutôt que rempli une fois au démarrage : la
 * police personnelle appartient au livre ouvert, elle entre et sort avec lui. Sa valeur
 * est reposée depuis l'envoi — sans quoi le menu montrerait la première main pendant
 * que l'exemplaire en compose une autre, et le premier réglage de l'écran l'imposerait.
 */
function afficherMain() {
  const sel = $('inMain');
  const perso = projet.envois.personnelle;
  const e = envoi();
  sel.replaceChildren();
  // Les écritures et les formes dans une seule liste, préfixées : la question posée est
  // « d'où vient l'écriture », et elle n'a qu'une réponse à la fois. Sans préfixe, une
  // police qui s'appellerait « image » désignerait l'autre forme.
  for (const m of mains) sel.append(new Option(m, `police:${m}`));
  if (perso) sel.append(new Option(`${perso} (votre police)`, `police:${perso}`));
  sel.append(new Option('Image écrite à la main', 'image'));
  sel.append(new Option('Image générée', 'diffusion'));
  sel.value = e.main.mode === 'police' ? `police:${e.main.police}` : e.main.mode;
}

/**
 * Ce qui appartient au livre et non à l'exemplaire : l'écriture de l'auteur, et le
 * gabarit partagé des envois générés.
 *
 * À part du reste des réglages, parce qu'un livre sans dédicataire les porte quand
 * même : les poser dans `afficherMain`, qui n'a de sens qu'avec un envoi ouvert, les
 * laissait dans l'état du livre précédent.
 */
function afficherLivre() {
  const perso = projet.envois.personnelle;
  $('inGabarit').value = projet.envois.gabarit ?? '';
  $('etatPolice').textContent = perso
    ? `Police personnelle embarquée : ${perso}.`
    : 'Aucune police personnelle : les envois s\'écrivent dans une main de la maison.';
  $('btPoliceRetirer').disabled = !perso;
}

/** Remplace l'envoi ouvert par lui-même modifié. */
function reglerEnvoi(sur) {
  const e = envoi();
  if (!e) return Promise.resolve();
  return tente(async () => afficherProjet(
    await invoke('envoi_regler', { index: choisi, envoi: { ...e, ...sur } })));
}

/** Déplace l'envoi ouvert, sans repasser par le reste de ses réglages. */
function reglerPlace(sur) {
  const e = envoi();
  if (!e) return Promise.resolve();
  return reglerEnvoi({ place: { ...e.place, ...sur } });
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
  await tente(async () => afficherProjet(await invoke('envoi_image_choisir', { index, chemin })));
  await majObjet();
}

/**
 * Demande l'image au modèle, et la montre sans la garder.
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
  await majObjet();
}

/* ---------- le canevas ---------- */

/**
 * Les vignettes de toutes les pages, gardées en mémoire.
 *
 * Rendues en une invocation : 190 pages coûtent six dixièmes de seconde, et la page de
 * fond ne dépend d'aucun envoi — les redemander à chaque changement de dédicataire
 * ferait payer ce prix vingt fois pour des images identiques.
 */
let pages = null;

/**
 * Les vignettes de l'intérieur sont périmées : la prochaine ouverture les refera.
 *
 * Et tout de suite si l'on regarde déjà le rail : vider le cache ne décroche pas les
 * images accrochées, et l'on continuerait de viser les pages d'une pagination qui
 * n'est plus celle du pied — page 264 d'un intérieur qui n'en fait plus que 190. Le
 * refus à la génération le dirait, mais une fois le mot écrit.
 */
function oublierPages() {
  pages = null;
  if (etape === 'envois') ouvrirCanevas();
}

/**
 * Le rail : toutes les pages, la page visée marquée.
 *
 * Cliquer une vignette déplace l'envoi sur cette page — c'est le seul moyen d'en
 * changer, et c'est pourquoi il n'y a pas de champ « page ».
 */
async function majVignettes() {
  const ol = $('vignettes');
  if (!envoi()) {
    ol.textContent = '';
    return;
  }
  if (!pages) {
    $('etatEnvois').className = 'etat';
    $('etatEnvois').textContent = 'rendu des pages…';
    try {
      pages = await invoke('envoi_vignettes');
      $('etatEnvois').textContent = '';
    } catch (e) {
      $('etatEnvois').textContent = String(e);
      $('etatEnvois').className = 'etat erreur';
      return;
    }
  }
  const visee = envoi()?.place.page ?? 0;
  ol.textContent = '';
  for (const [i, src] of pages.entries()) {
    const n = i + 1;
    const li = h('li');
    li.setAttribute('aria-current', String(n === visee));
    const img = h('img');
    img.src = src;
    img.alt = `Page ${n}`;
    li.addEventListener('click', async () => {
      await reglerPlace({ page: n });
      await majPage();
      afficherEnvois();
      marquerVignette(n);
    });
    li.append(img);
    ol.append(li);
  }
}

/** Déplace le liseré sans refaire le rail : deux cents images ne se reposent pas. */
function marquerVignette(n) {
  // `children` et non la propriété du faux DOM : c'est l'API du navigateur qui compte,
  // et `couverture.js` la parcourt déjà ainsi pour ses onglets de face.
  [...$('vignettes').children].forEach((li, i) => {
    li.setAttribute('aria-current', String(i + 1 === n));
  });
}

/**
 * La couleur du papier que le destinataire visé imprimera.
 *
 * Le premier papier du prestataire à défaut du sien : c'est la règle du Rust, dont
 * `papier_defaut` rend le premier de la liste. Blanc quand rien ne se retrouve — mieux
 * vaut un canevas honnêtement blanc qu'un crème inventé.
 */
function teintePapier() {
  const l = projet?.livraison;
  const d = l?.destinataires.find((x) => x.provider === l.courant);
  const pr = providers.find((p) => p.cle === d?.provider);
  const pa = pr?.papiers.find((x) => x.cle === d?.papier) ?? pr?.papiers[0];
  return pa?.teinte ?? '#ffffff';
}

/**
 * La page de fond du canevas : celle que l'envoi vise, rendue **sans envoi**.
 *
 * Sans envoi parce qu'un `foreground` ne réordonne rien : la page ne dépend d'aucun
 * dédicataire, et la même image sert à tous. C'est aussi ce qui permet de glisser
 * l'objet sans rappeler Typst — le fond ne bouge pas.
 */
async function majPage() {
  const e = envoi();
  const img = $('fondPage');
  // La page change : la confirmation, elle, est figée sur celle d'avant. La laisser
  // reviendrait à confirmer une page qu'on vient de quitter, le canevas caché derrière.
  revenirAuCanevas();
  if (!e) {
    img.hidden = true;
    // Le rapport s'en va avec la page : un canevas qui le garderait garderait sa
    // place, l'établi seul, un rectangle sombre là où il n'y a rien à montrer.
    $('canevas').style.removeProperty('--ratio');
    $('canevas').style.removeProperty('--papier');
    return;
  }

  // La teinte du papier que le destinataire visé imprimera. C'est un fait d'écran : le
  // PDF n'a pas de fond, et lui en donner un ferait imprimer un aplat sur toutes les
  // pages. Sans elle, un fond mal détouré resterait invisible — blanc de photo sur
  // blanc d'écran — jusqu'au tirage.
  $('canevas').style.setProperty('--papier', teintePapier());
  await tente(async () => {
    img.src = await invoke('envoi_page', { page: e.place.page });
    img.alt = `Page ${e.place.page} de l'intérieur`;
    img.hidden = false;
  });
}

/**
 * Le canevas prend le rapport de la page décodée.
 *
 * C'est lui qui lui donne sa taille, comme au cadre de l'aperçu de couverture et pour
 * la même raison : borné sur sa seule largeur, le canevas suit la bande élastique, et
 * une fenêtre large lui vaut une page plus haute que l'étape. L'étape ne défilant pas,
 * le bas de la page passe sous le bord — et l'envoi s'y pose.
 *
 * Le rapport ne se connaît qu'une fois l'image décodée : d'où l'écoute du chargement.
 */
function poserRatioPage() {
  const img = $('fondPage');
  if (!img.naturalHeight) return;
  $('canevas').style.setProperty('--ratio', String(img.naturalWidth / img.naturalHeight));
}

/**
 * L'objet manipulé : l'envoi rendu par Typst, sur fond transparent.
 *
 * Rendu par Typst et non imité en CSS : ce qu'on déplace **est** ce qui s'imprimera —
 * même police, même corps, mêmes coupures de lignes. Typst n'est rappelé qu'ici, quand
 * le mot ou la main changent ; glisser, redimensionner et incliner ne sont ensuite que
 * des `transform`.
 */
async function majObjet() {
  const e = envoi();
  const bloc = $('objet');
  if (!e) {
    bloc.hidden = true;
    return;
  }
  await tente(async () => {
    const o = await invoke('envoi_objet', { index: choisi });
    $('objetImage').src = o.image;
    $('objetImage').alt = `Envoi pour ${e.dedicataire || 'ce dédicataire'}`;
    bloc.hidden = false;
    poserObjet();
  });
}

/**
 * Pose l'objet à sa place sur le canevas.
 *
 * En pourcentages du canevas et non en pixels : c'est la forme que le Rust reçoit, et
 * c'est ce qui fait qu'un canevas plus petit montre le même placement.
 */
function poserObjet() {
  const e = envoi();
  if (!e) return;
  const s = $('objet').style;
  s.setProperty('left', `${e.place.x * 100}%`);
  s.setProperty('top', `${e.place.y * 100}%`);
  s.setProperty('width', `${e.place.taille * 100}%`);
  s.setProperty('--angle', `${e.place.angle}deg`);
}

/**
 * La page composée avec son envoi, par le chemin complet.
 *
 * C'est la confirmation, non l'aperçu : le canevas montre déjà le rendu, puisque le
 * fond et l'objet viennent tous deux de Typst. Ce bouton les fait passer par la même
 * composition, celle qui part à l'impression.
 */
async function apercuEnvoi(i) {
  const img = $('apercuEnvoi');
  if (!img.hidden) return revenirAuCanevas();
  return tente(async () => {
    img.src = await invoke('envoi_apercu', { index: i });
    img.alt = `Page ${projet.envois.liste[i].place.page} de l'exemplaire de `
      + `${projet.envois.liste[i].dedicataire}`;
    img.hidden = false;
    // À la place du canevas, et non par-dessus : la bande n'a la hauteur que d'une
    // page. C'est aussi ce qui rend la confirmation lisible — on va et vient d'une
    // image à l'autre, et l'objet ne doit pas bouger d'un pouce.
    $('canevas').hidden = true;
    $('btVoirPage').textContent = 'Revenir au canevas';
  });
}

/** Referme la confirmation : le canevas reprend sa place, et le bouton son mot. */
function revenirAuCanevas() {
  $('apercuEnvoi').hidden = true;
  $('canevas').hidden = false;
  $('btVoirPage').textContent = 'Voir la page';
}

/* ---------- la génération ---------- */

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
    // `nb` et non `toFixed` : le pied écrit « 16,51 mm » deux centimètres plus bas, et
    // deux écritures d'un même millimètre dans une même fenêtre se lisent comme deux
    // mesures.
    bloc.append(h('p', `envois/${r.dossier}/ — ${r.package.pages} pages, dos `
      + `${nb(r.package.dos)} mm`, 'chemin'));
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

/* ---------- les gestes ---------- */

/**
 * Un geste sur le canevas : ce qu'il tient, et ce que chaque pixel en fait.
 *
 * Le modèle est `saisir()` dans `couverture.js`, dont c'est l'idiome : le déplacement
 * se mesure en **fraction du canevas** et jamais en pixels — le canevas s'affiche à la
 * taille que la fenêtre lui laisse —, et un clic qui n'a rien déplacé n'est pas commis,
 * pour ne pas marquer le projet modifié en ayant seulement posé la souris sur sa propre
 * page.
 *
 * Le code n'est pas partagé avec lui, et c'est délibéré : `saisir()` est soudé à
 * `#cadreApercu` et aux chemins de contrôles de la couverture. L'extraire dans un module
 * commun est le bon geste, et c'est un remaniement du code le plus délicat de
 * l'application, sans rapport avec ce que cette étape livre.
 *
 * Pendant le geste, **seul l'écran suit** : le projet n'est touché qu'au dépôt. C'est ce
 * qui rend le glisser instantané — l'objet est déjà rendu, et le fond ne bouge pas.
 */
function saisirPlacement(el, calcule) {
  el.addEventListener('pointerdown', (ev) => {
    if (ev.button) return;
    const e = envoi();
    if (!e) return;
    const cadre = $('canevas').getBoundingClientRect();
    if (!cadre.width || !cadre.height) return;
    ev.preventDefault();
    ev.stopPropagation();
    el.setPointerCapture(ev.pointerId);

    const depart = e.place;
    const canevas = { largeur: cadre.width, hauteur: cadre.height };
    let dernier = depart;
    $('canevas').setAttribute('data-geste', 'oui');

    const bouger = (m) => {
      dernier = calcule(depart, {
        dx: m.clientX - ev.clientX,
        dy: m.clientY - ev.clientY,
        x: (m.clientX - cadre.left) / cadre.width,
        y: (m.clientY - cadre.top) / cadre.height,
      }, canevas);
      projet.envois.liste[choisi].place = dernier;
      poserObjet();
    };
    const lacher = () => {
      el.removeEventListener('pointermove', bouger);
      el.removeEventListener('pointerup', lacher);
      el.removeEventListener('pointercancel', lacher);
      $('canevas').removeAttribute('data-geste');
      // Un clic qui n'a rien déplacé ne se commet pas : il marquerait le projet
      // modifié, donc réveillerait la garde à la fermeture, pour avoir posé la souris
      // sur sa propre page. La comparaison porte sur les valeurs et non sur le fait
      // qu'un `pointermove` ait eu lieu — un pixel de tremblement, ramené dans les
      // bornes, ne change rien non plus.
      if (memePlace(dernier, depart)) return;
      // La taille change les coupures de lignes de l'objet, donc son rendu ; la
      // position et l'angle ne changent que sa pose, et le PNG en main suffit.
      const refaire = dernier.taille !== depart.taille;
      reglerPlace(dernier).then(() => (refaire ? majObjet() : afficherEnvois()));
    };
    el.addEventListener('pointermove', bouger);
    el.addEventListener('pointerup', lacher);
    el.addEventListener('pointercancel', lacher);
  });
}

/** Deux placements que rien ne distingue : le geste n'a rien fait. */
function memePlace(a, b) {
  return a.page === b.page && a.x === b.x && a.y === b.y
    && a.taille === b.taille && a.angle === b.angle;
}

/**
 * Câble les trois gestes du canevas. Une fois, au démarrage : les poignées sont dans le
 * balisage, seul l'objet qu'elles tiennent change d'un dédicataire à l'autre.
 *
 * Le déplacement se saisit sur `#objet` et non sur l'image qu'il contient : celle-ci
 * porte `pointer-events: none` — sans quoi WebKit y verrait une image à traîner —, si
 * bien que le hit-test la traverse et qu'un écouteur posé dessus n'est jamais appelé.
 * C'est aussi le conteneur que le CSS annonce par son `cursor: grab`. Les poignées,
 * elles, sont ses enfants et arrêtent la propagation : le geste ne se prend pas deux
 * fois.
 */
function cablerPlacement() {
  saisirPlacement($('objet'), (p, d, c) => deplace(p, d, c));
  saisirPlacement($('poigneeTaille'), (p, d, c) => redimensionne(p, d, c));
  saisirPlacement($('poigneeAngle'), (p, d, c) => incline(p, { x: d.x, y: d.y }, c));
}
