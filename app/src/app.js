'use strict';

const { invoke } = window.__TAURI__.core;
const { open, save } = window.__TAURI__.dialog;
const { openPath } = window.__TAURI__.opener;
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
/**
 * Les mains embarquées avec l'application.
 *
 * Retenues au démarrage plutôt qu'écrites une fois pour toutes dans le `select` : ce
 * dernier se refait à chaque projet, la police personnelle de l'auteur venant s'ajouter
 * aux mains de la maison.
 */
let mains = [];
/**
 * L'envoi dont une image générée attend d'être retenue, s'il y en a un.
 *
 * Le Rust tient l'image ; l'écran ne retient que la ligne à laquelle elle appartient,
 * pour n'allumer « Retenir » que là. Générer sur une autre ligne déplace l'attente :
 * il n'y a qu'une image en attente à la fois, comme il n'y a qu'un aperçu.
 */
let candidat = null;
let face = 'une';

/**
 * Les repères du dernier aperçu posé — coupe et plis —, et si la lunette est allumée.
 *
 * Les deux vivent ici et non dans le projet : ce qu'on regarde n'est pas ce qu'on
 * imprime. Rien n'en va dans le `.ozalid`, et le PDF remis ne porte aucun repère —
 * c'est ce que `planche.rs` promet en tête de fichier.
 */
let reperesCourants = null;
let reperesVisibles = true;
let attenteApercu = null;

/**
 * La manipulation directe de la couverture : ce qu'on tient, et de quoi le montrer.
 *
 * `geste` n'est pas qu'un drapeau — c'est lui qui dit aux deux autres mécanismes de se
 * taire pendant qu'on tire : le panneau ne réécrit pas les champs qu'on est en train de
 * déplacer, et l'aperçu vrai qui arrive ne retire pas le direct posé par-dessus. Sans
 * lui, la couverture reviendrait à son ancien cadrage à chaque composition rattrapée.
 *
 * `calques` est ce que le Rust a donné pour la face montrée : la photo, sa zone et
 * l'habillage. Absent, il n'y a rien à déplacer — et la prise de l'image ne s'offre
 * pas plutôt que d'offrir un geste qui mentirait.
 */
let geste = null;
let calques = null;
let boitesDos = [];
let attenteCommit = null;

/**
 * L'attente avant une recomposition automatique, et de quoi n'en lancer qu'une.
 *
 * `veilleSuspendue` couvre le seul rendu où la veille ne doit pas partir : celui d'un
 * projet qu'on vient d'ouvrir. Un livre enregistré dans un état périmé réclame bien une
 * composition, mais l'ouvrir n'est pas la demander — et une minute de Typst à
 * l'ouverture d'un fichier qu'on voulait seulement regarder serait pire que le clic
 * qu'on cherche à supprimer.
 */
let attenteComposition = null;
let compositionEnCours = false;
let compositionARefaire = false;
let veilleSuspendue = false;
/**
 * Le consentement du livre ouvert : un manuscrit y a été chargé.
 *
 * `deja_compose`, dans le projet, ne se lève qu'à une composition **réussie**. Il ne
 * suffit donc pas : si la toute première échoue — police invalide, compte de chapitres
 * faux —, la veille resterait muette et il n'y a plus de bouton pour reprendre. On
 * corrigerait la cause et il ne se passerait rien.
 *
 * Cette variable est ce qui manque : elle dit qu'on a demandé, non qu'on a obtenu. Elle
 * appartient au livre ouvert et retombe avec lui — un `.ozalid` rouvert ne consent pas,
 * c'est tout l'objet du dispositif.
 */
let consenti = false;
const DELAI_COMPOSITION = 400;

const nb = (v, d = 2) => v.toLocaleString('fr-FR', {
  minimumFractionDigits: d, maximumFractionDigits: d
});

/* ---------- coquille ---------- */

/**
 * Les quatre étapes, dans l'ordre où le livre se fait : leur clé — celle des entrées
 * `aller.*` du menu, au préfixe près — leur libellé d'onglet, et la section montrée.
 *
 * La table est la seule source de ce qu'elle porte : les onglets, le routage du menu et
 * le masquage des sections en sortent tous, et aucune de ces trois listes n'est à tenir
 * d'accord avec les autres.
 *
 * Ce qu'elle ne porte pas, en revanche, et qu'ajouter ou retirer une étape demande
 * encore : la section dans `index.html`, l'entrée dans `menu.rs` — écrite à la main,
 * elle, avec son accélérateur —, la clé dans `ETAPES` de `coquille.test.js`, et si
 * l'étape est un formulaire, les deux sélecteurs de `styles.css` qui énumèrent
 * `#etapeLivre, #etapeEnvois`. Les oublier ne casse rien de visible : l'étape hérite du
 * `height: 100%` des autres, ses blocs ne coulent pas en colonnes, et c'est la mise en
 * page qui paraît de travers sans qu'on sache pourquoi.
 *
 * Le README décrit l'écran, lui aussi. Et quatre fichiers de test nomment la structure
 * plutôt que les identifiants : `coquille` (la table, la navigation, les témoins),
 * `composition` (la liste des sections masquées), `contrats` (la règle CSS, lue par une
 * expression régulière littérale) et `epreuve` (la section qu'un projet ouvert montre).
 * Ceux qui pilotent un bouton par son `id` ne bougent pas — un élément qui déménage sans
 * être renommé leur est invisible. Onze fichiers, donc, pas trois.
 */
const ETAPES = [
  ['livre', '1 · Livre', 'etapeLivre'],
  ['couverture', '2 · Couverture', 'etapeCouverture'],
  ['livraison', '3 · Livraison', 'etapeLivraison'],
  ['envois', '4 · Envois', 'etapeEnvois'],
];

/** L'étape montrée. Sans projet, aucune ne l'est : l'accueil prend leur place. */
let etape = 'livre';

function construireEtapes() {
  for (const [cle, libelle, section] of ETAPES) {
    const b = h('button');
    b.type = 'button';
    b.id = `onglet-${cle}`;
    b.setAttribute('role', 'tab');
    // Les deux moitiés du lien entre l'onglet et sa section : ce que l'onglet commande,
    // et le nom que la section prend de son onglet. Elles ne sont écrites nulle part
    // dans le balisage — la table est la seule à savoir quelle section va avec quelle
    // clé, et c'est ici qu'elle le dit.
    b.setAttribute('aria-controls', section);
    $(section).setAttribute('aria-labelledby', b.id);
    b.append(h('span', libelle, 'nom'));
    // Le sous-libellé porte l'état de l'étape ; il est retrouvable par son identifiant
    // plutôt que par son rang, pour qu'ajouter un élément à l'onglet ne le déplace pas.
    const sous = h('span', '', 'sous');
    sous.id = `sous-${cle}`;
    b.append(sous);
    b.addEventListener('click', () => allerA(cle));
    $('etapes').append(b);
  }
  $('etapes').addEventListener('keydown', toucheEtapes);
  // Éteints dès leur naissance, sans attendre le premier projet : un démarrage qui
  // échoue n'affiche jamais rien et ne repasserait donc jamais par ici. Les onglets
  // resteraient d'apparence active sans mener nulle part, et le rang sans onglet
  // sélectionné — l'état que le HTML décrit n'est celui de personne.
  majEtapes();
}

/**
 * Les flèches traversent les étapes ; la tabulation les traverse d'un bloc.
 *
 * C'est le pattern `tablist` : un seul onglet dans l'ordre de tabulation — celui qui est
 * sélectionné, `majEtapes` s'en charge — et les flèches pour passer de l'un à l'autre.
 * Sans cela, atteindre au clavier le contenu de la Livraison demandait de traverser les
 * quatre onglets un par un ; avec, une tabulation suffit à sortir de la bande.
 *
 * La sélection suit la flèche, sans qu'il faille valider : cinq étapes qui montrent un
 * formulaire chacune, aucune n'est coûteuse à afficher, et l'activation manuelle du
 * pattern est faite pour les onglets qui chargent quelque chose.
 *
 * Le focus suit ce que `allerA` a bien voulu changer, et non ce qu'on lui a demandé :
 * sans projet il ne change rien, et il n'y a pas de second garde à écrire ici.
 *
 * Les cinq onglets sont en ligne : ce sont les flèches horizontales qui les
 * traversent. Un rail vertical demanderait les verticales et un `aria-orientation` — la
 * disposition, elle seule, dit lesquelles.
 */
function toucheEtapes(ev) {
  const cles = ETAPES.map(([cle]) => cle);
  const rang = cles.indexOf(etape);
  const vise = {
    ArrowRight: (rang + 1) % cles.length,
    ArrowLeft: (rang - 1 + cles.length) % cles.length,
    Home: 0,
    End: cles.length - 1,
  }[ev.key];
  if (vise === undefined) return;
  ev.preventDefault();
  allerA(cles[vise]);
  $(`onglet-${etape}`).focus();
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
  // Le rail et le canevas ne se rendent qu'en arrivant aux Envois, et pas à l'ouverture
  // du projet : ils coûtent une composition, et la payer à qui vient regarder une
  // couverture serait le prix de ce qu'il n'a pas demandé. C'est le même arbitrage que
  // la composition elle-même, qui attend son consentement.
  if (cle === 'envois') ouvrirCanevas();
}

/**
 * Un dos existe et ne vaut plus : ni « jamais composé », qui ne réclame rien, ni
 * « à jour ».
 *
 * `deja_compose` fait toute la différence : sans lui, un livre jamais composé et un
 * livre dont la mesure vient d'être périmée se ressembleraient trait pour trait, et le
 * premier serait signalé en alerte pour un travail qu'on ne lui a jamais demandé.
 *
 * Nommé plutôt que recopié : le pied le dit à qui regarde, `etatEtapes` s'en servait
 * pour son témoin, et deux copies d'une même condition finissent par diverger — c'est
 * ce qui a déjà fait mentir deux fois la liste des jetons recopiée dans le HTML.
 */
function dosPerime(p) {
  return p.livraison.deja_compose && !destinataireCourant()?.compose;
}

/**
 * Ce que chaque onglet dit de son étape : un sous-libellé qui énonce où en est le
 * projet, et un témoin quand l'étape réclame attention.
 *
 * Deux témoins, et pas un de plus. Un manuscrit qui ne correspond plus au contrôle
 * d'intégrité ; une couverture sans maquette. Un manuscrit absent n'en est pas un :
 * c'est l'état d'un projet neuf, pas une anomalie.
 *
 * Le troisième — un dos qui ne vaut plus — s'allumait à l'Intérieur, parce que c'était
 * là qu'on le réparait. Cette étape n'existe plus, et il est descendu au pied, qui
 * portait déjà le dos : il se lit désormais depuis n'importe quelle étape, ce qui vaut
 * mieux pour une mesure dont la Couverture est la première à souffrir.
 *
 * Tout se déduit de `p` : `dosCourant()` compare le dos mesuré au gabarit, au papier et
 * à la police que le *projet* porte, jamais à ce que les contrôles affichent. L'ordre
 * des appels est donc sans conséquence, et une saisie refusée ne peut plus allumer un
 * témoin sur un dos intact.
 */
function etatEtapes(p) {
  const attendu = p.livre.chapitres;
  const ecart = attendu !== null && attendu !== undefined && attendu !== p.chapitres_trouves;
  return {
    livre: {
      sous: ecart
        ? `${p.chapitres_trouves} chapitres, ${attendu} attendus`
        : (p.manuscrit_absent ? 'aucun manuscrit' : `${p.chapitres_trouves} chapitres`),
      alerte: ecart,
    },
    couverture: {
      sous: p.couverture ? libelleMode(p.couverture.mode) : 'aucune maquette',
      alerte: !p.couverture,
    },
    // Rien de vrai à dire avant qu'un package n'ait été généré, et le pied porte déjà
    // le dos : mieux vaut se taire que meubler.
    livraison: { sous: '', alerte: false },
    // Le compte des envois est la seule chose vraie que le projet porte ici ; zéro
    // n'est pas une anomalie, donc pas un mot et jamais de témoin.
    envois: {
      sous: p.envois.liste.length
        ? `${p.envois.liste.length} envoi${p.envois.liste.length > 1 ? 's' : ''}`
        : '',
      alerte: false,
    },
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
    // Un seul onglet dans l'ordre de tabulation : voir `toucheEtapes`. Sans projet,
    // aucun — ils sont éteints, et un onglet éteint qui prendrait le focus laisserait la
    // tabulation dans une bande où il n'y a rien à faire.
    onglet.setAttribute('tabindex', !!projet && cle === etape ? '0' : '-1');
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
 * les packages ou les ebooks — reste dans `#etat`, `#etatEpreuve`, `#etatPackages`,
 * `#etatEbooks`, à côté du bouton qui l'a lancé : on attend là où l'on a cliqué, et un
 * compte rendu qui migre en haut de l'écran se lit comme une panne. Faire remonter le
 * reste ici par symétrie ferait perdre cette différence.
 *
 * Reste `#etatApercu`, qui n'entre dans aucun des deux : personne ne l'a demandé. La
 * composition part d'elle-même à chaque réglage, et son échec est un fait sur l'image
 * qu'on regarde, pas le compte rendu d'un geste — il se lit sous l'aperçu, comme une
 * légende, et le réglage suivant l'emporte sans que personne ait à l'effacer.
 */
function alerter(message) {
  $('alerte').textContent = message;
  $('alerte').className = message ? 'etat erreur' : 'etat';
}

/**
 * Le pied : pour qui l'on regarde, et ce que vaut le dos.
 *
 * Le destinataire visé s'y choisit, une fois pour toute la fenêtre — c'est le pointeur
 * de la spec, et il est ici plutôt qu'à l'étape Livraison parce qu'on en change en
 * réglant la couverture, sans avoir à quitter ce qu'on regarde.
 *
 * Le dos n'y paraît que s'il vaut pour ce qui est montré — c'est `dosCourant()` qui en
 * répond — parce qu'un dos périmé écrit en bas de l'écran est exactement ce qu'on ne
 * relirait pas.
 *
 * Quatre états, et un seul à la fois :
 *
 * - **périmé**, en rouge, prioritaire sur tout : c'est le témoin qui s'allumait sur
 *   l'onglet Intérieur, descendu ici avec l'étape qui a disparu. Il tenait sa place de
 *   ce qu'on allait y réparer ; il tient celle-ci de ce que le pied portait déjà le
 *   dos, et de ce qu'on ne quitte pas la Couverture pour aller lire un onglet.
 * - **relevé sur le gabarit** : chez un prestataire qui ne publie pas de formule, il n'y
 *   a jamais rien à composer, et « non composé » ferait recomposer en boucle un livre
 *   dont la pagination est déjà juste. Ce qui manque est un relevé, pas un calcul —
 *   c'est le vocabulaire que `noteFormat` emploie déjà pour le fond perdu.
 * - **non composé** : jamais composé, et rien à réclamer pour autant.
 * - **le chiffre**, quand il vaut.
 *
 * Périmé passe avant « non composé » et n'est pas son synonyme : sans lui, un livre
 * qu'on n'a jamais composé et un livre dont on vient de périmer la mesure se liraient
 * pareil, et le second passerait pour un projet neuf.
 */
function majPied() {
  // Le prestataire, pas seulement le projet : un démarrage qui n'a pas pu lire les
  // gabarits laisse la liste vide, et le premier projet ouvert ferait lever le pied
  // au lieu de dire ce qu'il sait — c'est-à-dire rien.
  const p = projet ? providerCourant() : null;
  const sel = $('inDestinataire');
  $('visee').hidden = !p;
  if (!p) {
    sel.replaceChildren();
    viderPied();
    return;
  }
  sel.replaceChildren();
  for (const d of projet.livraison.destinataires) {
    sel.append(new Option(libelleProvider(d.provider), d.provider));
  }
  sel.value = projet.livraison.courant;

  viderPied();
  // Une composition qui tourne passe avant tout le reste, et en gris : elle dure des
  // dizaines de secondes sur un vrai manuscrit, et personne ne l'a demandée. Laisser le
  // pied crier « dos périmé » en rouge tout ce temps ferait lire une panne là où il n'y
  // a qu'un travail en cours. C'est le mot que `#etat` disait à côté du bouton ; il n'a
  // pas disparu avec lui, il a déménagé où le compte rendu vit désormais.
  if (compositionEnCours) {
    $('piedDos').textContent = '· composition…';
    return;
  }
  const perime = dosPerime(projet);
  const dos = dosCourant();
  const etat = perime ? 'dos périmé'
    : !p.dos_publie ? 'dos relevé sur le gabarit'
      : dos === null ? 'dos non composé'
        : `dos ${nb(dos, 1)} mm`;
  $('piedDos').textContent = `· ${etat}`;
  $('piedDos').className = perime ? 'alerte' : '';

  // Les chiffres ne paraissent qu'avec une mesure, et un dos périmé n'en a pas : c'est
  // sa définition même — `dosPerime` est vrai quand le livre a été composé et que la
  // mesure du destinataire visé a disparu. Il n'y a donc pas de second garde à écrire
  // ici, et le pied ne peut pas donner à lire 264 pages sous un « dos périmé ».
  const mesure = destinataireCourant()?.compose;
  if (!mesure) return;

  $('piedMesure').textContent =
    `· ${mesure.pages} pages · ${projet.chapitres_trouves} chapitres`
    + ` · gouttière ${nb(mesure.gouttiere)} mm`;

  // Le lien ne paraît que si le Rust a trouvé le fichier : `interieur_pdf` est déjà
  // filtré par son existence, et l'écran n'a rien à revérifier.
  if (projet.interieur_pdf) {
    $('piedInterieur').append(h('span', '· '), lienFichier(projet.interieur_pdf, 'intérieur'));
  }

  // Absent du JSON quand il est vide — `skip_serializing_if`, comme la dédicace du
  // livre. C'est le cas normal, pas un cas dégradé : la plupart des compositions ne
  // substituent rien.
  if ((mesure.polices_introuvables ?? []).length) {
    $('piedRepli').textContent = '· ⚠ repli';
    $('piedRepli').className = 'alerte';
  }
}

/**
 * Le pied remis à zéro avant d'être rempli.
 *
 * Quatre éléments dont trois sont conditionnels : les effacer d'un bloc évite qu'un
 * lien d'une composition précédente survive au projet suivant, ou qu'un `⚠ repli`
 * reste allumé sur une mesure qui n'en porte plus.
 */
function viderPied() {
  for (const id of ['piedMesure', 'piedDos', 'piedInterieur', 'piedRepli']) {
    $(id).replaceChildren();
    $(id).className = '';
  }
}

/**
 * Un chemin de fichier rendu cliquable : le nom court se lit, le chemin entier se
 * survole, et le clic ouvre le fichier dans le lecteur du poste.
 *
 * Le `preventDefault` n'est pas une précaution de style : sans lui, l'ancre remplacerait
 * la fenêtre de l'application par le PDF, et il n'y a pas de bouton « Retour » dans une
 * fenêtre Tauri.
 */
function lienFichier(chemin, libelle) {
  const a = h('a', libelle);
  a.href = '#';
  a.title = chemin;
  a.addEventListener('click', (ev) => {
    ev.preventDefault();
    tente(() => openPath(chemin));
  });
  return a;
}

/* ---------- prestataires ---------- */

async function chargerProviders() {
  providers = await invoke('providers_liste');
  polices = await invoke('polices_liste');
  for (const p of await invoke('polices_texte_liste')) {
    $('inPoliceInterieur').append(new Option(p, p));
  }
  mains = await invoke('mains_liste');
  // La liste vient du Rust, seul à la connaître : `gabarit::JETONS` a grossi deux fois.
  $('aideJetons').textContent =
    `Ces champs peuvent citer les précédents : ${(await invoke('jetons_liste')).join(' ')}`;
  // L'accès au modèle appartient à la machine, pas au projet : il se lit une fois, au
  // démarrage, et il survit à tous les livres qu'on ouvrira ensuite.
  afficherDiffusion(await invoke('diffusion_lire'));
  await rafraichirMaquettes();
  $('inMaquette').addEventListener('change', choisirMaquette);
  $('btMaquettes').addEventListener('click', () => {
    $('etatMaquettes').textContent = '';
    $('dlgMaquettes').showModal();
  });
  $('btMaquettesFermer').addEventListener('click', () => $('dlgMaquettes').close());
  $('btMaquetteEnregistrer').addEventListener('click', enregistrerMaquette);
  construireReglages();
}

/**
 * Le gabarit du destinataire visé, tel que la table le décrit.
 *
 * Le projet ne porte que des clés ; le format, le fond perdu et les papiers viennent
 * de la table, jamais du document — c'est ce qui permet à un `.ozalid` de suivre un
 * prestataire qui change son guide.
 */
function providerCourant() {
  return providers.find((p) => p.cle === projet?.livraison.courant);
}

/** Le destinataire visé : son papier et ses relevés. */
function destinataireCourant() {
  return projet?.livraison.destinataires.find((d) => d.provider === projet.livraison.courant);
}

/** Le libellé d'un gabarit, ou sa clé si la table ne le connaît plus. */
function libelleProvider(cle) {
  return providers.find((p) => p.cle === cle)?.libelle ?? cle;
}

/* ---------- projet ---------- */

function afficherProjet(p) {
  projet = p;
  $('titreLivre').textContent = p.livre.titre || 'Sans titre';
  $('cheminProjet').textContent = p.chemin ?? 'projet non enregistré';
  // Le chemin se tronque dans la bande : entier, il n'est plus qu'à un survol.
  $('cheminProjet').title = p.chemin ?? '';
  $('etatEnregistrement').textContent = p.modifie
    ? 'modifié'
    : (p.chemin ? 'enregistré' : 'jamais enregistré');

  $('inTitre').value = p.livre.titre;
  $('inTitrePage').value = p.livre.titre_page;
  $('inAuteur').value = p.livre.auteur;
  $('inGenre').value = p.livre.genre;
  $('inEditeur').value = p.livre.editeur;
  $('inCollection').value = p.livre.collection;
  $('inMonogramme').value = p.livre.monogramme;
  $('inCopyright').value = p.livre.copyright;
  $('inPrix').value = p.livre.prix;
  $('inMention').value = p.livre.mention;
  // Le champ est absent du JSON quand le livre n'a pas de dédicace : `skip_serializing_if`.
  $('inDedicace').value = p.livre.dedicace ?? '';
  $('inChapitres').value = p.livre.chapitres ?? '';
  $('inPoliceInterieur').value = p.interieur.police;
  // Sans attendre : l'échantillon est une image de l'écriture, pas une donnée du projet,
  // et le reste de l'affichage n'a pas à tenir derrière une police de huit cent kilo-octets.
  montrerEchantillon(p.interieur.police);
  // Lu dans la mesure du projet, jamais dans le retour de `composer` : un PDF composé
  // dans une écriture de repli ne redevient pas juste en refermant le livre, et cette
  // phrase doit être là à la réouverture. Le pied n'en porte que le signe.
  const repli = destinataireCourant()?.compose?.polices_introuvables ?? [];
  $('repliPolices').textContent = repli.length
    ? `Police introuvable, composé dans une écriture de repli : ${repli.join(', ')}.`
      + ' Le PDF ne suit pas la maquette.'
    : '';
  $('repliPolices').hidden = !repli.length;

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

  // Dans la barre, à côté des deux boutons qui la changent : les noms suffisent, et
  // l'absence se dit en deux mots. La phrase qui expliquait ce qu'une couverture sans
  // photo compose — le papier seul — est partie avec le bloc : l'aperçu la montre.
  // Chaque nom porte le geste qui retire sa photo du projet ; voir `afficherPhotos`.
  afficherPhotos(p.images);

  // Le panneau, la face montrée et la disposition qu'elle demande sortent tous de
  // `poserDisposition` : c'est la couverture qui les commande, et sans elle il n'y a
  // rien à régler. L'invite à choisir une maquette, elle, s'écrit dans l'aperçu vide —
  // à l'endroit où le manque se voit.
  if (p.couverture) afficherCouverture(p.couverture);
  else poserDisposition(false);
  afficherDestinataires();
  afficherEnvois();
  demanderApercu();
  majPied();
  majEtapes();
  veiller();
}

/**
 * Recompose de soi-même quand la mesure du destinataire visé vient d'être périmée.
 *
 * Deux conditions, et il faut les deux.
 *
 * Le **consentement** : `consenti` — un manuscrit a été chargé dans ce livre — ou
 * `deja_compose` — il a déjà été composé au moins une fois, fût-ce dans une session
 * précédente. Avant l'un des deux, rien ne part tout seul : regarder une première de
 * couverture réclame un format, pas une composition, et faire payer une minute de Typst
 * à qui ouvre un `.ozalid` pour le regarder serait pire que tout ce qu'on lui épargne.
 * Les deux, et pas seulement le second : `deja_compose` ne se lève qu'à une réussite,
 * et un premier échec n'aurait plus aucune reprise depuis que le bouton a disparu.
 *
 * L'absence de mesure est le **besoin** : présente, il n'y a rien à recalculer.
 *
 * Débouncé, parce qu'un livre se modifie par rafales : changer le titre puis la
 * dédicace ne doit lancer qu'une composition, celle du dernier état.
 */
function veiller() {
  if (veilleSuspendue) {
    veilleSuspendue = false;
    return;
  }
  if (!(consenti || projet?.livraison.deja_compose) || destinataireCourant()?.compose) return;
  clearTimeout(attenteComposition);
  attenteComposition = setTimeout(() => recomposer(false), DELAI_COMPOSITION);
}

/**
 * Une composition à la fois, et la dernière gagne.
 *
 * Une composition dure des secondes ; un réglage changé pendant qu'elle tourne rendrait
 * son résultat faux à l'instant où il arrive. Plutôt que d'en lancer une seconde en
 * parallèle — le Rust les sérialiserait sur son verrou, et on paierait les deux —, on
 * note qu'il faudra recommencer, et on recommence une fois.
 */
async function recomposer(force) {
  if (compositionEnCours) {
    compositionARefaire = true;
    return;
  }
  // Le besoin a pu disparaître pendant l'attente : le bouton reste un recours, et
  // l'employer désarme la veille plutôt que de faire recalculer à l'identique ce que le
  // clic vient d'obtenir. `force` couvre le seul cas où la mesure présente ne vaut
  // rien — celui d'une reprise, expliqué plus bas.
  if (!force && destinataireCourant()?.compose) return;
  compositionEnCours = true;
  try {
    await composer();
  } finally {
    // Le pied est repeint **ici**, après que le drapeau est retombé, et pas seulement
    // dans le `finally` de `composer` : celui-là court encore pendant que la
    // composition est officiellement en cours, et le pied y resterait bloqué sur
    // « composition… » jusqu'au prochain geste.
    compositionEnCours = false;
    majPied();
  }
  // Reprogrammée sans repasser par `veiller` : la composition qui vient de finir a
  // déposé une mesure, et elle a l'air fraîche — mais elle a été lancée sur l'état
  // d'avant la modification qui nous a réveillés. `veiller` la croirait bonne et
  // laisserait le livre porter, jusqu'au prochain geste, le dos d'un texte périmé.
  if (compositionARefaire) {
    compositionARefaire = false;
    clearTimeout(attenteComposition);
    attenteComposition = setTimeout(() => recomposer(true), DELAI_COMPOSITION);
  }
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
 *
 * Rend vrai quand `fn` a abouti. La plupart des appelants n'en font rien — un geste de
 * réglage est fini quand il est fini — mais la garde des modifications, elle, doit
 * savoir si l'enregistrement a réellement écrit avant de laisser fermer.
 */
function tente(fn) {
  derniereTentative = essai(fn);
  return derniereTentative;
}

async function essai(fn) {
  try {
    alerter('');
    await fn();
    return true;
  } catch (e) {
    alerter(String(e));
    // `afficherProjet` ne touche pas à l'alerte : le message qu'on vient d'écrire y
    // survit au redessin.
    if (projet) afficherProjet(projet);
    return false;
  }
}

/**
 * Le dernier geste parti au Rust, pour pouvoir l'attendre.
 *
 * Tout ce qui modifie le projet passe par `tente` : le retenir ici est ce qui permet à
 * `terminerSaisie` de rendre la main une fois la frappe réellement arrivée, et non
 * seulement envoyée. Deux commandes lancées coup sur coup n'arrivent pas dans l'ordre
 * où elles sont parties — le Rust les sert sur son exécuteur, pas dans une file.
 */
let derniereTentative = Promise.resolve(true);

/**
 * Termine la saisie en cours, et attend qu'elle soit arrivée.
 *
 * Un champ que le clavier tient encore n'a rien envoyé : `change` n'arrive qu'à la
 * perte du focus, et l'accélérateur d'un menu natif ne la provoque pas — la fenêtre
 * garde son focus pendant que le Rust travaille. Sans cette ligne, ⌘S enregistrait
 * l'ancienne valeur, puis `afficherProjet` réécrivait le champ avec elle : la frappe
 * était perdue deux fois, et rien à l'écran ne l'annonçait.
 *
 * Le `blur` suffit à la faire partir — l'écouteur de `change` fait le reste, quel que
 * soit le champ et sans que rien ait à savoir lequel avait le focus.
 */
function terminerSaisie() {
  document.activeElement?.blur?.();
  return derniereTentative;
}

/**
 * Efface ce que la composition du texte précédent avait laissé à l'écran.
 *
 * Pagination, dos, chemins de fichiers : ces chiffres ne valent que pour le manuscrit
 * qui les a produits. Les laisser en place quand le texte est remplacé donnerait à lire
 * la pagination du mauvais livre — précisément l'erreur que l'application existe pour
 * supprimer. Les envois y sont : un exemplaire dédicacé porte son propre compte de pages
 * et son dos, sortis du même texte.
 *
 * Deux chemins y mènent, et ce sont les deux façons de périmer une composition : ouvrir
 * un autre projet, et remplacer le manuscrit de celui qui est ouvert.
 */
function oublierLaComposition() {
  // Le dos n'est plus effacé ici : il vit dans le projet, et c'est le Rust qui le périme
  // au moment du geste qui l'a rendu faux. Ce qui reste ici est ce qui n'appartient
  // qu'à l'écran — des chiffres affichés, des chemins de fichiers, des messages.
  for (const id of ['packages', 'ebooks', 'resultatEnvois']) {
    $(id).replaceChildren();
    $(id).hidden = true;
  }
  $('cheminEpreuve').textContent = '';
  // Les canaux de compte rendu qui vont avec, et pas seulement celui de la composition :
  // un message rouge appartient au texte qui l'a provoqué autant que le chiffre qu'il
  // commente. Effacer le chemin de l'épreuve en laissant l'erreur qui disait pourquoi
  // elle avait échoué donnerait à lire l'échec de l'ancien texte sous le nouveau.
  for (const id of ['etatEpreuve', 'etatPackages', 'etatEbooks', 'etatEnvois',
    'etatMaquettes']) {
    $(id).textContent = '';
    $(id).className = 'etat';
  }
}

/**
 * Efface ce que le projet précédent avait laissé à l'écran.
 *
 * Ses sorties de composition d'abord, puis ce qui n'appartient qu'à lui : l'étape où
 * l'on était, ses destinataires, ses envois, sa couverture. C'est ce partage qui sépare
 * les deux fonctions — remplacer le texte d'un livre n'est pas en ouvrir un autre.
 */
function oublierLesSorties() {
  oublierLaComposition();
  // Un autre livre s'ouvre : la composition en attente était celle de celui qu'on
  // quitte, et le premier rendu du nouveau ne doit rien déclencher. Le consentement
  // part avec elle — il appartenait au livre qu'on ferme, et ouvrir n'est pas demander.
  clearTimeout(attenteComposition);
  compositionARefaire = false;
  veilleSuspendue = true;
  consenti = false;
  // L'étape courante est une sortie comme une autre : elle appartenait au projet qu'on
  // regardait. Rester sur la Livraison en ouvrant un autre livre donnerait à lire ses
  // packages sous le titre du nouveau.
  etape = 'livre';
  // La liste des destinataires appartient au projet, pas à l'écran : sans projet, elle
  // n'a personne à nommer, et `afficherProjet` la refait entièrement pour le suivant.
  $('destinataires').replaceChildren();
  // Les envois de même : ce sont les mots écrits pour les lecteurs du livre A, et
  // l'aperçu de page de titre qui va avec. L'image proposée par le modèle s'en va avec
  // eux : le Rust l'a oubliée en posant l'autre projet, et laisser « Retenir » allumé
  // proposerait de figer, dans le livre B, une image demandée pour le livre A.
  candidat = null;
  choisi = 0;
  $('envois').replaceChildren();
  $('apercuEnvoi').removeAttribute('src');
  $('apercuEnvoi').hidden = true;
  // Le canevas et son rail sont ceux d'un livre qui n'est plus ouvert : les laisser
  // montrerait les pages du livre A pendant qu'on règle un envoi du livre B, et rien à
  // l'écran ne dirait laquelle des deux paginations on regarde.
  oublierPages();
  $('vignettes').replaceChildren();
  $('fondPage').removeAttribute('src');
  $('fondPage').hidden = true;
  $('objet').hidden = true;
  // L'entête par-dessus les canaux qu'`oublierLaComposition` vient d'éteindre : une
  // saisie refusée y est écrite au nom du livre qu'on quitte, et elle y resterait seule
  // à parler de lui.
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
  $('cheminProjet').title = '';
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
      // Le nom d'abord, le répertoire dessous, et le chemin entier au survol : écrits
      // d'un seul tenant, les chemins d'un poste réel poussaient la bande de contenu et
      // ouvraient une barre horizontale sur toute la fenêtre. Les tronquer par la fin
      // n'aurait rien réglé — cinq projets d'un même répertoire ont leurs cinquante
      // premiers caractères en commun, et se seraient lus pareil.
      const coupe = Math.max(c.lastIndexOf('/'), c.lastIndexOf('\\'));
      const b = h('button');
      b.type = 'button';
      b.title = c;
      b.append(h('span', c.slice(coupe + 1).replace(/\.ozalid$/, ''), 'nom'));
      b.append(h('span', c.slice(0, Math.max(coupe, 0)), 'chemin'));
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
  // La frappe en cours d'abord : le Rust ne sait pas encore qu'elle a eu lieu, et il
  // répondrait « rien à enregistrer » sur un projet qu'on vient de modifier.
  await terminerSaisie();
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
  // sans savoir si un projet est ouvert : c'est ici que la protection doit vivre. Avant
  // `tente()`, et non dedans : un geste inerte n'a rien à raconter, donc rien à effacer
  // dans l'entête — le message qu'un ⌘S en échec y a laissé dit encore vrai.
  if (!projet) return false;
  if (projet.chemin) {
    return tente(async () => afficherProjet(await invoke('projet_enregistrer')));
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
    // Un `livre.toml` importé apporte son manuscrit : c'est le même geste que d'en
    // choisir un, et il consent comme lui. Levé **après** `oublierLesSorties`, qui vient
    // de le faire retomber pour le livre qu'on quitte.
    consenti = true;
    afficherProjet(p);
    recomposer(true);
  });
}

/** « Enregistrer sous… » : demande où poser le projet. Rend vrai si écrit. */
async function enregistrerSous() {
  // Le menu y mène directement, sans passer par « Enregistrer » : la garde s'y répète.
  if (!projet) return false;
  const choix = await save({
    defaultPath: `${projet.livre.titre || 'projet'}.ozalid`,
    filters: [{ name: 'Projet Ozalid', extensions: ['ozalid'] }],
  });
  // Un sélecteur refermé sans choisir n'a pas plus écrit qu'un geste sans projet : rien
  // à dire, et rien à effacer de ce qui était dit.
  if (!choix) return false;
  return tente(async () =>
    afficherProjet(await invoke('projet_enregistrer_sous', { chemin: choix })));
}

/* ---------- livre et manuscrit ---------- */

/**
 * Le livre entier, à chaque modification d'un seul de ses champs : `livre_modifier`
 * remplace ce qu'il tient par ce qu'on lui envoie. Un champ oublié ici n'est pas une
 * erreur côté Rust, c'est une donnée effacée — la dédicace, facultative, se perdrait
 * ainsi au premier changement de titre.
 */
function livre() {
  const chap = $('inChapitres').value.trim();
  return {
    titre: $('inTitre').value.trim(),
    titre_page: $('inTitrePage').value.trim(),
    auteur: $('inAuteur').value.trim(),
    genre: $('inGenre').value.trim(),
    editeur: $('inEditeur').value.trim(),
    collection: $('inCollection').value.trim(),
    monogramme: $('inMonogramme').value.trim(),
    copyright: $('inCopyright').value,
    prix: $('inPrix').value.trim(),
    mention: $('inMention').value.trim(),
    // Non rognée : c'est le Rust qui rogne, en un seul endroit — et il substitue les
    // jetons avant de rogner, ce que le front ne saurait pas faire.
    dedicace: $('inDedicace').value,
    chapitres: chap === '' ? null : Number(chap),
  };
}

/**
 * Le livre vient d'être modifié : ses pages liminaires composent, donc paginent. Une
 * dédicace prend une belle page et sa blanche — deux pages —, un pavé de copyright plus
 * long peut refluer. Le dos suit, alors que le gabarit, le papier et la police n'ont pas
 * bougé : c'est exactement la cause qu'aucune estampille ne voit.
 *
 * Périmé sans regarder quel champ a changé ni s'il pagine réellement. La liste de ceux
 * qui composent vit dans `interieur::source` ; la tenir en double ici la ferait diverger
 * en silence, et se tromper de ce côté-là ne coûte qu'une composition.
 */
async function majLivre() {
  await tente(async () => {
    const p = await invoke('livre_modifier', { livre: livre() });
    oublierLaComposition();
    afficherProjet(p);
  });
}

/**
 * Le manuscrit vient d'être remplacé : le texte fait la pagination, donc le dos. Celui
 * de la dernière composition ne vaut plus rien, ni la pagination, ni les packages, ni
 * l'épreuve, ni les envois — et rien dans le panneau ne permettrait de s'en apercevoir,
 * le gabarit, le papier et la police, eux, n'ayant pas bougé.
 *
 * Les sorties, et elles seules : le projet est le même, avec ses destinataires, ses
 * envois à écrire et l'étape où l'on travaillait.
 *
 * Périmé sans regarder si le texte a réellement changé : réimporter un manuscrit
 * identique coûte une recomposition pour rien, comparer deux fois un roman entier à
 * chaque clic coûterait davantage, et se tromper de ce côté-là n'imprime rien de faux.
 */
/**
 * Un manuscrit vient d'arriver — réimporté, ou choisi ailleurs.
 *
 * C'est **le geste qui consent** : charger un manuscrit dit « ce livre m'intéresse »,
 * là où ouvrir un `.ozalid` ne dit que « je regarde ». La composition part donc d'ici,
 * et il n'y a plus de bouton pour la demander.
 *
 * `recomposer(true)` plutôt que `veiller()` : la mesure vient d'être effacée de toute
 * façon, et le rendu qui suit n'a pas à attendre son débounce pour un geste dont on
 * sait déjà qu'il périme tout.
 */
function manuscritRemplace(p) {
  oublierLaComposition();
  consenti = true;
  afficherProjet(p);
  recomposer(true);
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

/**
 * Les écritures déjà chargées, par famille. `afficherProjet` repasse ici à chaque
 * frappe dans l'onglet Livre, et chaque lecture côté Rust parcourt les dix mégaoctets
 * des polices embarquées : ce qui est chargé une fois l'est pour la session.
 *
 * On y range la promesse, jamais son résultat : deux affichages rapprochés tombent
 * alors sur la même lecture, là où deux promesses parallèles la feraient deux fois.
 */
const ecritures = new Map();

/**
 * Montre le texte d'exemple dans l'écriture d'intérieur choisie — et ne le montre pas
 * autrement.
 *
 * La police est chargée dans ses propres octets, ceux que Typst composera. Un
 * `font-family` posé sur le seul nom de la famille aurait pris celle du poste quand
 * elle s'y trouve : la face reçoit donc un nom qui n'existe sur aucun système.
 *
 * Un échec ne laisse rien à l'écran : le repli d'un navigateur est muet, comme celui de
 * Typst, et un échantillon rendu dans l'écriture de la fenêtre montrerait une police que
 * le livre n'aura pas. La raison, elle, s'affiche — c'est sous ce sélecteur qu'on vient
 * réparer, comme pour le repli de composition juste au-dessus.
 */
async function montrerEchantillon(famille) {
  if (!ecritures.has(famille)) ecritures.set(famille, chargerEcriture(famille));
  const { nom, erreur } = await ecritures.get(famille);
  // La lecture a pu durer plus longtemps que le choix : c'est le dernier qui compte.
  if (famille !== $('inPoliceInterieur').value) return;
  if (nom) $('echantillonPolice').style.setProperty('--police-echantillon', `"${nom}"`);
  else $('echantillonAbsent').textContent = `Pas d'échantillon de « ${famille} » : ${erreur}`;
  $('echantillonPolice').hidden = !nom;
  $('echantillonAbsent').hidden = !!nom;
}

/** Une famille d'intérieur, chargée dans la fenêtre sous un nom à elle. */
async function chargerEcriture(famille) {
  const nom = `echantillon-${famille}`;
  try {
    const donnee = await invoke('police_texte_donnee', { famille });
    const face = new FontFace(nom, `url(${donnee})`);
    await face.load();
    document.fonts.add(face);
    return { nom };
  } catch (e) {
    return { erreur: String(e) };
  }
}

/* ---------- composition ---------- */

/*
 * Aucun panneau de compte rendu ici, et c'est le fait de ce lot.
 *
 * Ce que la composition mesure entre dans le projet, chez le destinataire visé, et le
 * pied le relit de là — comme le dos le faisait déjà seul. `Composition` porte les mêmes
 * chiffres en copie de lecture ; l'écran ne s'en sert plus, et `afficherProjet` suffit.
 * C'est ce qui fait tenir la légende après une réouverture, là où un panneau rempli
 * depuis le retour de commande se serait tu.
 *
 * Ce qui se perd et qui ne manque pas : « Page blanche de fin ». Une parité qu'on
 * regarde une fois ne mérite pas une place dans une légende qui suit partout.
 */

/**
 * Compose l'intérieur pour le destinataire visé.
 *
 * Plus personne ne l'appelle depuis un bouton : elle part du chargement d'un manuscrit,
 * puis de la veille. Son compte rendu est donc une **légende** et non l'attente d'un
 * clic — le pied dit « composition… » pendant, et les chiffres après.
 *
 * L'échec monte à la bande d'alerte, la seule que toutes les étapes partagent : il n'y a
 * plus de bouton à côté duquel l'écrire, et une composition déclenchée depuis la
 * Couverture n'a aucune raison d'échouer dans un coin de l'étape Livre.
 *
 * Ce message s'efface au geste suivant — `essai()` remet l'alerte à zéro à chaque
 * tentative — et **ce n'est pas un trou à boucher** : tout geste qui l'efface relance
 * aussi la composition, la mesure étant toujours absente, et la réécrit si la cause
 * tient. Une garde qui le retiendrait ferait survivre l'échec à sa réparation.
 */
async function composer() {
  majPied();
  // Les vignettes du rail montrent l'intérieur qu'on s'apprête à recomposer : les
  // garder ferait placer un envoi sur les pages d'un livre qui n'existe plus, et rien
  // à l'écran ne dirait laquelle des deux paginations on regarde.
  oublierPages();
  try {
    const c = await invoke('composer');
    // Le dos sort de la pagination qu'on vient de mesurer, et c'est le projet qui le
    // retient désormais, chez le destinataire pour qui il vaut. L'interface n'en garde
    // aucune copie : elle le relit là où il est enregistré, comme tout le reste — et
    // depuis ce lot, les pages, les chapitres, la gouttière et le repli avec.
    afficherProjet(c.projet);
    if (face === 'planche') demanderApercu();
  } catch (e) {
    alerter(String(e));
  } finally {
    majPied();
    majEtapes();
  }
}

/* ---------- épreuve ---------- */

async function epreuve() {
  const bt = $('btEpreuve');
  bt.disabled = true;
  $('cheminEpreuve').replaceChildren();
  $('etatEpreuve').className = 'etat';
  $('etatEpreuve').textContent = 'composition…';
  try {
    const chemin = await invoke('epreuve_tirer', {
      corpsPt: Number($('inEpreuveCorps').value),
    });
    // Le même geste que le lien du pied : le laisser en texte mort à côté d'un lien
    // vivant serait une incohérence gratuite. `textContent` traverse l'ancre, donc ce
    // qui lisait le chemin ici le lit toujours.
    $('cheminEpreuve').append(lienFichier(chemin, chemin));
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
  // Le menu natif ne prend pas le focus de la page : c'est la seule porte de
  // l'application par laquelle un geste peut arriver sur une frappe non terminée.
  await terminerSaisie();
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
$('btReperes').addEventListener('click', basculerReperes);
// Le seul écouteur de l'application qui ne réponde pas à un geste : c'est l'image
// décodée qui donne au cadre sa taille, et elle ne l'est qu'après avoir été posée.
$('apercu').addEventListener('load', poserRatio);
$('fondPage').addEventListener('load', poserRatioPage);
$('btPackager').addEventListener('click', packager);
$('btEbooks').addEventListener('click', ebooks);
$('btEpreuve').addEventListener('click', epreuve);
$('inPoliceInterieur').addEventListener('change', majInterieur);
// Changer de destinataire déplace le format de l'aperçu et l'épaisseur du dos : c'est
// le projet qui les porte, et `afficherProjet` suffit à les remettre d'accord.
// Les vignettes du rail, elles, ne sont pas dans le projet : ce sont les pages d'une
// pagination, et deux destinataires n'ont pas les mêmes. Elles se périment donc ici, et
// non dans `composer` seul — revenir à un destinataire déjà mesuré ne recompose rien, et
// le rail garderait les pages du précédent.
$('inDestinataire').addEventListener('change', () => tente(async () => {
  afficherProjet(await invoke('destinataire_viser', {
    providerCle: $('inDestinataire').value,
  }));
  oublierPages();
}));
$('btAjouterDestinataire').addEventListener('click', () => tente(async () =>
  afficherProjet(await invoke('destinataire_ajouter', {
    providerCle: $('inAjoutDestinataire').value,
  }))));
$('btEnvoyer').addEventListener('click', envoyer);
// Un envoi neuf n'a pas encore de mot : c'est le nom qui l'ouvre, et le mot se saisit
// dans la ligne. Un dédicataire vide n'ajoute rien plutôt que d'ajouter un anonyme.
$('btAjouterEnvoi').addEventListener('click', () => {
  const qui = $('inDedicataire').value.trim();
  if (qui === '') return undefined;
  $('inDedicataire').value = '';
  // Le neuf s'ouvre aussitôt : il naît en fin de liste, et l'y laisser fermé obligerait
  // à le chercher parmi vingt pour lui écrire son mot.
  return tente(async () => {
    afficherProjet(await invoke('envoi_ajouter', { dedicataire: qui }));
    choisir(projet.envois.liste.length - 1);
  });
});
$('btRetirerEnvoi').addEventListener('click', () => tente(async () => {
  afficherProjet(await invoke('envoi_retirer', { index: choisi }));
  choisir(choisi);
}));
$('inMot').addEventListener('change', () => reglerEnvoi({ contenu: $('inMot').value })
  .then(majObjet));
// L'échelle et l'inclinaison se saisissent aussi au clavier : la poignée et le champ
// disent la même valeur, comme sur la couverture. `input` et non `change` — un curseur
// qu'on tire doit montrer ce qu'il fait.
$('inTaille').addEventListener('input', () => {
  $('vTaille').textContent = `${$('inTaille').value} %`;
  return reglerPlace({ taille: Number($('inTaille').value) / 100 }).then(majObjet);
});
$('inAngle').addEventListener('input', () => {
  $('vAngle').textContent = `${$('inAngle').value}°`;
  return reglerPlace({ angle: Number($('inAngle').value) });
});
$('btImageEnvoi').addEventListener('click', () => choisirImageEnvoi(choisi));
$('btGenerer').addEventListener('click', () => genererEnvoi(choisi));
$('btAccepter').addEventListener('click', () => accepterEnvoi(choisi));
$('btVoirPage').addEventListener('click', () => apercuEnvoi(choisi));
// La police de l'auteur est copiée dans le `.ozalid`, comme le manuscrit et les photos :
// le chemin d'où elle vient n'a plus à exister pour que les envois se composent.
$('btPolice').addEventListener('click', async () => {
  const chemin = await open({
    multiple: false,
    filters: [{ name: 'Police manuscrite', extensions: ['ttf', 'otf'] }],
  });
  if (!chemin) return;
  await tente(async () => afficherProjet(await invoke('police_choisir', { chemin })));
});
$('btPoliceRetirer').addEventListener('click', () => tente(async () =>
  afficherProjet(await invoke('police_retirer'))));
// Le gabarit appartient au livre, l'accès au modèle à la machine : deux commandes, et
// la clé ne redescend jamais — le champ reste vide, et « inchangée » le dit.
$('inGabarit').addEventListener('change', () => tente(async () =>
  afficherProjet(await invoke('envois_gabarit', { gabarit: $('inGabarit').value }))));
$('btDiffusionRegler').addEventListener('click', () => reglerDiffusion(
  $('inDiffusionCle').value === '' ? null : $('inDiffusionCle').value));
$('btDiffusionOublier').addEventListener('click', () => reglerDiffusion(''));
// La main appartient à l'exemplaire depuis la v4 : la changer ne touche que lui, et
// c'est tout l'objet du chantier — écrire à la main pour l'une, composer pour l'autre.
$('inMain').addEventListener('change', () => {
  const choix = $('inMain').value;
  // Une police emporte son nom ; les deux formes en image n'emportent rien — le gabarit
  // est au livre, et il a sa propre commande.
  const main = choix.startsWith('police:')
    ? { mode: 'police', police: choix.slice('police:'.length) }
    : { mode: choix };
  return reglerEnvoi({ main }).then(majObjet);
});
construireEtapes();
construireFaces();
cablerPrises();
cablerPlacement();
for (const id of ['inTitre', 'inTitrePage', 'inAuteur', 'inGenre', 'inEditeur',
  'inCollection', 'inMonogramme', 'inCopyright', 'inPrix', 'inMention',
  'inDedicace', 'inChapitres']) {
  $(id).addEventListener('change', majLivre);
}
/**
 * La taille de la fenêtre, écrite dans l'entête.
 *
 * Elle ne sert pas à faire le livre : elle sert à en parler. Une mise en page se juge à
 * une taille, et un défaut décrit sans elle ne se reproduit pas — le canevas tenait à
 * 1040 px et débordait à 1500, et rien à l'écran ne disait laquelle des deux on
 * regardait.
 */
function majTailleFenetre() {
  $('fenetreTaille').textContent = `${window.innerWidth} × ${window.innerHeight}`;
}
window.addEventListener('resize', majTailleFenetre);
majTailleFenetre();

chargerProviders()
  .then(afficherAucunProjet)
  .catch((e) => {
    // Sans les gabarits ni les polices, rien de ce que l'application propose n'a de
    // sens : mieux vaut le dire que d'offrir un écran d'accueil qui ne mène nulle part.
    alerter(`démarrage impossible : ${e}`);
  });
