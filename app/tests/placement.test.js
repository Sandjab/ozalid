'use strict';

// La géométrie du placement d'un envoi, sans DOM : ce module reçoit des nombres et en
// rend. Le canevas lui-même — les prises, les écouteurs — se vérifie dans
// l'application, comme le rendu de la couverture.

const test = require('node:test');
const assert = require('node:assert');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');

/**
 * Le vrai `placement.js`, exécuté dans un contexte nu.
 *
 * Nu, et non le faux DOM de `dom_shim` : ce module ne touche à rien, et lui monter une
 * fenêtre entière ferait croire qu'il en a besoin.
 */
const contexte = { Math };
vm.createContext(contexte);
vm.runInContext(
  fs.readFileSync(path.join(__dirname, '..', 'src', 'placement.js'), 'utf8'),
  contexte,
  { filename: 'placement.js' }
);
const { borne, deplace, redimensionne, incline } = contexte;

const PLACE = { page: 3, x: 0.5, y: 0.8, taille: 0.6, angle: 0 };

/**
 * Deux fractions égales à un cheveu près.
 *
 * Les fractions de page s'additionnent en virgule flottante : 0,8 + 0,05 donne
 * 0,8500000000000001. L'écart n'a aucun sens à l'échelle d'une page — il vaut un
 * milliardième de millimètre — mais il ferait échouer une comparaison exacte, et le
 * test parlerait alors de l'arithmétique de JavaScript plutôt que du placement.
 */
const proche = (a, b, message) =>
  assert.ok(Math.abs(a - b) < 1e-9, message ?? `${a} ≠ ${b}`);

/**
 * Un geste se mesure en **fraction du canevas**, jamais en pixels : le canevas
 * s'affiche à la taille que la fenêtre lui laisse, et un geste calé sur des pixels
 * irait deux fois plus vite dans une petite fenêtre. C'est la règle de `saisir()`.
 */
test('un glisser se mesure en fraction du canevas', () => {
  const petit = deplace(PLACE, { dx: 30, dy: 20 }, { largeur: 300, hauteur: 400 });
  const grand = deplace(PLACE, { dx: 60, dy: 40 }, { largeur: 600, hauteur: 800 });
  proche(petit.x, grand.x, 'le geste dépend de la taille du canevas');
  proche(petit.y, grand.y, 'le geste dépend de la taille du canevas');
  proche(petit.x, 0.6);
  proche(petit.y, 0.85);
});

/** Le reste du placement ne bouge pas : glisser ne redimensionne ni n'incline. */
test('un glisser ne touche qu-à la position', () => {
  const p = deplace(PLACE, { dx: 30, dy: 20 }, { largeur: 300, hauteur: 400 });
  assert.equal(p.taille, PLACE.taille);
  assert.equal(p.angle, PLACE.angle);
  assert.equal(p.page, PLACE.page);
});

/**
 * Le centre reste sur le papier : un envoi glissé au-delà ne s'imprimerait pas, et rien
 * à l'écran ne dirait où il est parti.
 */
test('le centre du placement reste dans la page', () => {
  const loin = deplace(PLACE, { dx: 9000, dy: -9000 }, { largeur: 300, hauteur: 400 });
  assert.equal(loin.x, 1);
  assert.equal(loin.y, 0);
});

/**
 * La taille se prend sur la largeur, pas sur la diagonale : c'est la largeur que Typst
 * reçoit, et une prise qui suivrait la diagonale ferait diverger l'écran du rendu.
 *
 * Le facteur 2 vient du centre : le bord droit est à `x + taille / 2`, le tirer d'un
 * pixel écarte les deux bords d'un pixel chacun. Sans lui, l'objet fuirait sous la
 * souris — le bord n'irait qu'à mi-vitesse du curseur.
 */
test('la prise de coin suit le bord, donc double la course', () => {
  const p = redimensionne(PLACE, { dx: 30, dy: 0 }, { largeur: 300, hauteur: 400 });
  proche(p.taille, 0.8);
  assert.equal(p.x, PLACE.x, 'le centre a bougé');
  assert.equal(p.y, PLACE.y, 'le centre a bougé');
});

/**
 * Une taille nulle ferait disparaître l'objet, prises comprises : on ne pourrait plus
 * le rattraper à la souris.
 */
test('la taille garde une borne basse attrapable', () => {
  const p = redimensionne(PLACE, { dx: -9000 }, { largeur: 300, hauteur: 400 });
  assert.ok(p.taille >= 0.05, `taille inattrapable : ${p.taille}`);
});

/**
 * L'origine des angles est le haut, et le sens est horaire — c'est ce que veut
 * `Place::angle` côté Rust, donc le `rotate` de Typst. Une origine à droite, ou un sens
 * inverse, ferait pencher l'envoi imprimé à l'opposé de ce que montre le canevas.
 */
test("l'inclinaison part du haut et tourne dans le sens horaire", () => {
  const canevas = { largeur: 400, hauteur: 400 };
  // À l'aplomb du centre, au-dessus de lui.
  assert.equal(Math.round(incline(PLACE, { x: 0.5, y: 0.5 }, canevas).angle), 0);
  // À droite, à la hauteur du centre : un quart de tour horaire.
  assert.equal(Math.round(incline(PLACE, { x: 0.9, y: 0.8 }, canevas).angle), 90);
  // À gauche, à la hauteur du centre : un quart de tour dans l'autre sens.
  assert.equal(Math.round(incline(PLACE, { x: 0.1, y: 0.8 }, canevas).angle), -90);
});

/**
 * La rotation tourne **autour du centre** : c'est le pivot de `rotate` en Typst comme de
 * `transform-origin: center` en CSS. Un pivot qui déplacerait l'objet ferait fuir
 * l'envoi sous la souris pendant qu'on l'incline.
 */
test("l'inclinaison ne déplace ni ne redimensionne", () => {
  const p = incline(PLACE, { x: 0.9, y: 0.8 }, { largeur: 400, hauteur: 400 });
  assert.equal(p.x, PLACE.x);
  assert.equal(p.y, PLACE.y);
  assert.equal(p.taille, PLACE.taille);
});

/**
 * Le canevas rétablit les proportions : une page de livre n'est pas carrée, et mesurer
 * l'angle sur des fractions le déformerait — 45° à l'écran s'imprimeraient autrement.
 */
test("l'inclinaison tient compte du format de la page", () => {
  // Une prise en diagonale exacte du canevas : sur un canevas deux fois plus haut que
  // large, le même écart en fractions ne fait pas le même angle.
  const carre = incline(PLACE, { x: 0.7, y: 0.6 }, { largeur: 400, hauteur: 400 });
  const haut = incline(PLACE, { x: 0.7, y: 0.6 }, { largeur: 400, hauteur: 800 });
  assert.notEqual(
    Math.round(carre.angle),
    Math.round(haut.angle),
    'le format de la page est ignoré'
  );
});

/** 370° et 10° sont le même envoi, et le champ qui l'affiche doit le dire. */
test("l'angle est ramené dans un tour", () => {
  assert.equal(borne({ ...PLACE, angle: 370 }).angle, 10);
  assert.equal(borne({ ...PLACE, angle: -370 }).angle, -10);
});
