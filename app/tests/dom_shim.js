'use strict';
// Faux DOM minimal, juste assez pour exécuter le VRAI src/app.js.
//
// Réservé au câblage de l'interface : un champ reconstruit qui perd sa valeur, un
// prestataire sans formule dont on afficherait quand même un dos. Tout ce qui touche
// au rendu réel se vérifie dans l'application, pas ici.

const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');

class El {
  constructor(tag) {
    this.tagName = String(tag).toUpperCase();
    this.children = [];
    this.attrs = {};
    this.ecouteurs = {};
    this._texte = '';
    this.value = '';
    this.className = '';
    this.hidden = false;
    this.disabled = false;
  }

  get textContent() {
    return this.children.length
      ? this.children.map((c) => c.textContent).join('')
      : this._texte;
  }

  set textContent(v) {
    this._texte = String(v);
    this.children = [];
  }

  append(...n) {
    for (const x of n) this.children.push(x);
    this.majSelection();
  }

  replaceChildren(...n) {
    this.children = [];
    if (this.tagName === 'SELECT') this.value = '';
    this.append(...n);
  }

  /** Un <select> vide qui reçoit des options sélectionne la première, comme le DOM. */
  majSelection() {
    if (this.tagName !== 'SELECT' || this.value !== '') return;
    const premiere = this.children.find((c) => c.tagName === 'OPTION');
    if (premiere) this.value = premiere.value;
  }

  addEventListener(type, fn) {
    (this.ecouteurs[type] ||= []).push(fn);
  }

  /** Déclenche les écouteurs, comme le ferait un clic ou un change. */
  async declenche(type) {
    for (const fn of this.ecouteurs[type] || []) await fn();
  }

  /** Textes des descendants d'un type donné — pour lire un rendu. */
  textes(tag) {
    const out = [];
    const visite = (e) => {
      if (e.tagName === tag.toUpperCase()) out.push(e.textContent);
      e.children.forEach(visite);
    };
    this.children.forEach(visite);
    return out;
  }
}

/**
 * Type de balise et état initial d'un identifiant, lus dans le vrai index.html.
 * Le faux DOM part donc du même état que l'application : retirer un `disabled` ou
 * changer une balise dans le HTML se voit ici, au lieu de passer inaperçu.
 */
function depuisHtml(html, id) {
  const m = html.match(new RegExp(`<(\\w+)([^>]*\\bid="${id}"[^>]*)>`));
  if (!m) throw new Error(`identifiant absent d'index.html : ${id}`);
  const [, tag, attrs] = m;
  const valeur = attrs.match(/\bvalue="([^"]*)"/);
  return {
    tag,
    disabled: /\bdisabled\b/.test(attrs),
    hidden: /\bhidden\b/.test(attrs),
    value: valeur ? valeur[1] : '',
  };
}

/**
 * Charge src/app.js dans un contexte muni d'un faux DOM.
 * `ids` : identifiants à créer ; leur balise et leur état initial viennent d'index.html.
 * `invoke` : implémentation des commandes Rust.
 */
async function charge({ ids, invoke, open = async () => null }) {
  const html = fs.readFileSync(
    path.join(__dirname, '..', 'src', 'index.html'),
    'utf8'
  );
  const els = new Map(
    ids.map((id) => {
      const { tag, ...etat } = depuisHtml(html, id);
      return [id, Object.assign(new El(tag), { id }, etat)];
    })
  );
  const document = {
    getElementById: (id) => els.get(id) ?? null,
    createElement: (tag) => new El(tag),
  };
  const contexte = {
    document,
    Option: class extends El {
      constructor(texte, valeur) {
        super('option');
        this.textContent = texte;
        this.value = valeur;
      }
    },
    window: { __TAURI__: { core: { invoke }, dialog: { open } } },
    console,
  };
  contexte.globalThis = contexte;
  vm.createContext(contexte);
  const src = fs.readFileSync(
    path.join(__dirname, '..', 'src', 'app.js'),
    'utf8'
  );
  vm.runInContext(src, contexte);
  // chargerProviders() est asynchrone et lancé au chargement : lui laisser un tour.
  await new Promise((r) => setImmediate(r));
  return { els, contexte };
}

module.exports = { El, charge };
