'use strict';

/**
 * La géométrie du placement d'un envoi : où il est, quelle taille il fait, comment il
 * penche.
 *
 * Aucun DOM ici, et c'est ce qui compte : ce fichier reçoit des nombres et en rend, si
 * bien qu'il se vérifie sans fenêtre. Les écouteurs et le dessin sont dans `envois.js`.
 *
 * **Tout est en fraction de la page**, jamais en pixels : le canevas s'affiche à la
 * taille que la fenêtre lui laisse, et un geste calé sur des pixels irait deux fois plus
 * vite dans une petite fenêtre. C'est la règle de `saisir()` dans `couverture.js`, et
 * c'est aussi la forme que le Rust attend — le même `Place` voyage jusqu'à Typst.
 */

/**
 * La plus petite taille qu'on puisse encore attraper à la souris.
 *
 * Une taille nulle ferait disparaître l'objet, prises comprises : on ne pourrait plus
 * le rattraper, et le seul recours serait le champ numérique.
 */
const TAILLE_MIN = 0.05;

/** Ramène un nombre entre deux bornes. */
const entre = (v, min, max) => Math.min(max, Math.max(min, v));

/**
 * Un placement ramené dans ce qu'une page peut porter.
 *
 * Le centre reste sur le papier : un envoi glissé au-delà ne s'imprimerait pas, et rien
 * à l'écran ne dirait où il est parti. L'objet, lui, peut déborder — c'est son centre
 * qui est tenu, pas ses bords, et un envoi qui mord la marge est un choix légitime.
 *
 * L'angle est ramené dans un tour : 370° et 10° sont le même envoi, et le champ qui
 * l'affiche doit le dire.
 */
function borne(p) {
  return {
    ...p,
    x: entre(p.x, 0, 1),
    y: entre(p.y, 0, 1),
    taille: entre(p.taille, TAILLE_MIN, 1),
    angle: p.angle % 360,
  };
}

/** L'objet suit la souris : le déplacement du curseur, rapporté au canevas. */
function deplace(p, { dx, dy }, canevas) {
  return borne({
    ...p,
    x: p.x + dx / canevas.largeur,
    y: p.y + dy / canevas.hauteur,
  });
}

/**
 * La prise de coin règle la taille, sur la **largeur** et non sur la diagonale.
 *
 * C'est la largeur que Typst reçoit — `box(width: …%)` —, et une prise qui suivrait la
 * diagonale ferait diverger l'écran du rendu.
 *
 * Le facteur 2 vient du centre : l'objet est centré sur `x`, son bord droit est donc à
 * `x + taille / 2`. Tirer ce bord d'un pixel écarte les deux bords d'un pixel chacun,
 * et la largeur croît du double. Sans lui, l'objet fuirait sous la souris.
 */
function redimensionne(p, { dx }, canevas) {
  return borne({ ...p, taille: p.taille + (2 * dx) / canevas.largeur });
}

/**
 * L'inclinaison, mesurée autour du **centre** de l'objet.
 *
 * Le centre parce que c'est le pivot de `rotate` en Typst comme de
 * `transform-origin: center` en CSS : un autre pivot ferait diverger l'écran du rendu.
 *
 * L'origine des angles est le **haut** — une prise à l'aplomb du centre vaut 0°, et la
 * tirer vers la droite fait croître l'angle, comme le veut `Place::angle`, positif dans
 * le sens horaire. D'où l'`atan2(dx, -dy)` plutôt que l'`atan2(dy, dx)` habituel, qui
 * partirait de la droite et tournerait à l'envers.
 *
 * `prise` est la position du curseur en fraction du canevas ; les dimensions du canevas
 * rétablissent les proportions — une page de livre n'est pas carrée, et mesurer l'angle
 * sur des fractions le déformerait.
 */
function incline(p, prise, canevas) {
  const dx = (prise.x - p.x) * canevas.largeur;
  const dy = (prise.y - p.y) * canevas.hauteur;
  return borne({ ...p, angle: (Math.atan2(dx, -dy) * 180) / Math.PI });
}
