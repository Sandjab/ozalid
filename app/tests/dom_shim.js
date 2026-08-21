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
    this.enfants = [];
    this.attrs = {};
    this.ecouteurs = {};
    this._texte = '';
    this.value = '';
    this.className = '';
    this.hidden = false;
    this.disabled = false;
    this._id = undefined;
    this._registre = null;
  }

  /**
   * Un élément créé à la volée devient retrouvable par son identifiant dès qu'il en
   * reçoit un : la liste des prestataires est construite ainsi, et l'application la
   * relit avec `getElementById`.
   */
  get id() {
    return this._id;
  }

  set id(v) {
    this._id = v;
    if (this._registre) this._registre.set(v, this);
  }

  /**
   * Indexée, mesurable et itérable — et rien de plus, comme une `HTMLCollection`.
   *
   * Un tableau donnerait `map` et `forEach` : le code passerait ici et casserait dans
   * l'application, où ces méthodes n'existent pas. Les tests qui veulent un tableau
   * l'étalent, comme l'application doit le faire.
   */
  get children() {
    const c = {
      length: this.enfants.length,
      item: (i) => this.enfants[i] ?? null,
      [Symbol.iterator]: () => this.enfants[Symbol.iterator](),
    };
    this.enfants.forEach((e, i) => { c[i] = e; });
    return c;
  }

  get textContent() {
    return this.enfants.length
      ? this.enfants.map((c) => c.textContent).join('')
      : this._texte;
  }

  set textContent(v) {
    this._texte = String(v);
    this.enfants = [];
  }

  append(...n) {
    for (const x of n) this.enfants.push(x);
    this.majSelection();
  }

  replaceChildren(...n) {
    this.enfants = [];
    // Le vrai DOM emporte aussi le texte. Le garder ferait passer une boîte vidée pour
    // une boîte encore pleine, et un test qui vérifie qu'on a bien oublié les sorties
    // du projet précédent passerait sans que rien n'ait été oublié.
    this._texte = '';
    if (this.tagName === 'SELECT') this.value = '';
    this.append(...n);
  }

  /** Un <select> vide qui reçoit des options sélectionne la première, comme le DOM. */
  majSelection() {
    if (this.tagName !== 'SELECT' || this.value !== '') return;
    const premiere = this.enfants.find((c) => c.tagName === 'OPTION');
    if (premiere) this.value = premiere.value;
  }

  addEventListener(type, fn) {
    (this.ecouteurs[type] ||= []).push(fn);
  }

  setAttribute(nom, valeur) {
    this.attrs[nom] = String(valeur);
  }

  getAttribute(nom) {
    return this.attrs[nom] ?? null;
  }

  removeAttribute(nom) {
    delete this.attrs[nom];
    if (nom === 'src') this.src = undefined;
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
      e.enfants.forEach(visite);
    };
    this.enfants.forEach(visite);
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
async function charge({
  ids,
  invoke,
  open = async () => null,
  save = async () => null,
  listen = async () => () => {},
  destroy = () => {},
}) {
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
    createElement: (tag) => Object.assign(new El(tag), { _registre: els }),
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
    // Le menu natif et la fermeture de fenêtre passent par des événements : sans
    // `event.listen` dans le faux contexte, `app.js` lèverait au chargement et aucun
    // test ne s'exécuterait.
    window: {
      __TAURI__: {
        core: { invoke },
        dialog: { open, save },
        event: { listen },
        window: { getCurrentWindow: () => ({ destroy }) },
      },
    },
    console,
    // L'aperçu est débounce : sans minuteur, rien ne se déclenche.
    setTimeout,
    clearTimeout,
    JSON,
    Number,
    String,
    module: undefined,
  };
  contexte.globalThis = contexte;
  vm.createContext(contexte);
  // Les deux scripts de l'application, dans l'ordre du HTML : les déclarations de
  // couverture.js sont visibles depuis app.js, comme dans un navigateur.
  for (const nom of ['couverture.js', 'app.js']) {
    const src = fs.readFileSync(path.join(__dirname, '..', 'src', nom), 'utf8');
    vm.runInContext(src, contexte, { filename: nom });
  }
  // chargerProviders() est asynchrone et lancé au chargement : lui laisser un tour.
  await new Promise((r) => setImmediate(r));
  return { els, contexte };
}

module.exports = { El, charge };
