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
    // Le style inline, réduit aux variables CSS : c'est tout ce que l'application y
    // pose — les deux fractions de la coupe, que ni un attribut `data-` ni une classe
    // ne peuvent transporter jusqu'à un `calc()`.
    const proprietes = new Map();
    this.style = {
      setProperty: (nom, valeur) => proprietes.set(nom, String(valeur)),
      getPropertyValue: (nom) => proprietes.get(nom) ?? '',
      removeProperty: (nom) => proprietes.delete(nom),
    };
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

  /**
   * Le vrai DOM convertit en chaîne ce qu'on affecte à `value` : `input.value = 64`
   * donne « 64 », et l'application appelle `.trim()` dessus sans y penser. Un faux qui
   * garderait le nombre ferait échouer ici du code qui marche dans la fenêtre — c'est
   * ce qui a caché `livre()` à tous les tests jusqu'au premier qui l'a exercé.
   *
   * `null` devient la chaîne vide, comme le `[LegacyNullToEmptyString]` que la spec
   * pose sur `input` et `textarea`.
   */
  get value() {
    return this._value;
  }

  set value(v) {
    this._value = v == null ? '' : String(v);
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

  /**
   * Retire un écouteur, et le retire vraiment.
   *
   * Un geste de souris pose trois écouteurs à la pression et les reprend au
   * relâchement : sans ce retrait, le deuxième geste rejouerait le premier par-dessus,
   * et le troisième les deux — c'est exactement le genre de fuite qu'un faux DOM
   * complaisant laisserait passer jusqu'à la fenêtre.
   */
  removeEventListener(type, fn) {
    const l = this.ecouteurs[type];
    if (!l) return;
    const i = l.indexOf(fn);
    if (i >= 0) l.splice(i, 1);
  }

  /**
   * La boîte de l'élément à l'écran, que les tests posent eux-mêmes.
   *
   * Le faux DOM ne met rien en page : il n'a pas de boîte à mesurer. Mais la
   * manipulation directe convertit des pixels de souris en pourcentages de couverture,
   * et cette division-là est exactement ce qu'un test doit pouvoir vérifier — sans
   * elle, un geste calé sur des pixels passerait inaperçu jusqu'à la première fenêtre
   * d'une autre taille. Nulle par défaut : l'application refuse alors le geste, comme
   * elle le fait devant un aperçu qui n'est pas encore affiché.
   */
  getBoundingClientRect() {
    return this.rect ?? { left: 0, top: 0, width: 0, height: 0 };
  }

  /** La capture du pointeur, sans objet ici : il n'y a qu'un geste à la fois. */
  setPointerCapture() {}

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

  /**
   * Déclenche les écouteurs, comme le ferait un clic ou un change.
   *
   * L'événement est facultatif parce que la plupart des écouteurs n'en lisent rien : un
   * clic sur un bouton est tout entier dans le fait qu'il a eu lieu. Une touche, elle,
   * n'est que son événement — les tests en passent un pour dire laquelle.
   */
  /**
   * Le `<dialog>` du dialogue des maquettes. `open` est l'attribut que le vrai DOM
   * pose : les tests s'en servent pour dire si la boîte est ouverte, sans singer la
   * pile de modales ni le piège à focus, dont l'application ne dépend pas.
   */
  showModal() {
    this.open = true;
  }

  close() {
    this.open = false;
  }

  async declenche(type, evenement) {
    for (const fn of this.ecouteurs[type] || []) await fn(evenement);
  }

  /**
   * Le focus, réduit à ce que le pattern `tablist` en demande : savoir où il est allé.
   * Un onglet éteint le refuse, comme dans le navigateur — sans quoi une navigation au
   * clavier qui n'aurait pas dû aboutir paraîtrait avoir abouti.
   */
  focus() {
    if (this._doc && !this.disabled) this._doc.activeElement = this;
    this._valeurAuFocus = this.value;
  }

  /**
   * Le focus s'en va — et la valeur saisie part avec lui, comme dans le navigateur.
   *
   * `change` ne se déclenche qu'à la perte du focus, et seulement si la valeur a bougé
   * depuis qu'il a été pris : c'est cette règle-là que l'application exploite pour
   * qu'un ⌘S n'enregistre pas l'ancienne valeur d'un champ encore en cours de frappe.
   * Un faux DOM qui blurerait sans rien signaler ferait passer le correctif pour bon.
   *
   * Sans `await` sur `declenche`, comme le navigateur : l'écouteur est asynchrone, et
   * le rendre attendu ici donnerait à l'application une garantie qu'elle n'a pas.
   */
  blur() {
    if (!this._doc || this._doc.activeElement !== this) return;
    this._doc.activeElement = null;
    if (this.value !== this._valeurAuFocus) this.declenche('change');
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
 * Tous les identifiants posés dans le vrai index.html.
 *
 * Les énumérer dans chaque fichier de test revenait à tenir à la main une copie du
 * balisage : une section renommée s'y voyait en `null` sans message, cinq fois de
 * suite. Le faux DOM lit déjà le HTML pour connaître l'état initial de chaque élément ;
 * qu'il en lise aussi la liste ne fait qu'aller au bout de la même idée.
 */
function idsDuHtml(html) {
  return [...html.matchAll(/\bid="([^"]+)"/g)].map((m) => m[1]);
}

/**
 * Charge src/app.js dans un contexte muni d'un faux DOM.
 * `ids` : identifiants à créer ; par défaut, tous ceux du vrai index.html, avec leur
 * balise et leur état initial. `invoke` : implémentation des commandes Rust.
 */
async function charge({
  ids,
  invoke,
  open = async () => null,
  save = async () => null,
  listen,
  destroy = () => {},
}) {
  const html = fs.readFileSync(
    path.join(__dirname, '..', 'src', 'index.html'),
    'utf8'
  );
  const document = {
    activeElement: null,
    getElementById: (id) => els.get(id) ?? null,
    createElement: (tag) => Object.assign(new El(tag), { _registre: els, _doc: document }),
  };
  const els = new Map(
    (ids ?? idsDuHtml(html)).map((id) => {
      const { tag, ...etat } = depuisHtml(html, id);
      return [id, Object.assign(new El(tag), { id, _doc: document }, etat)];
    })
  );
  // Les écouteurs que l'application pose, retenus pour que les tests puissent les
  // actionner : le menu natif et la fermeture de fenêtre n'ont pas d'autre porte.
  const ecouteurs = {};
  const listenUtilise = listen ?? (async (nom, fn) => {
    ecouteurs[nom] = fn;
    return () => {};
  });
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
        event: { listen: listenUtilise },
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
  for (const nom of ['couverture.js', 'livraison.js', 'envois.js', 'app.js']) {
    const src = fs.readFileSync(path.join(__dirname, '..', 'src', nom), 'utf8');
    vm.runInContext(src, contexte, { filename: nom });
  }
  // chargerProviders() est asynchrone et lancé au chargement : lui laisser un tour.
  await new Promise((r) => setImmediate(r));

  const declencheEvenement = async (nom, charge) => {
    const fn = ecouteurs[nom];
    if (!fn) {
      throw new Error(
        `aucun écouteur « ${nom} » : un listen sur mesure a-t-il remplacé celui du faux DOM ?`
      );
    }
    await fn(charge);
  };

  return {
    els,
    contexte,
    /** Ce que fait une entrée de menu, désignée par son identifiant côté Rust. */
    menu: (id) => declencheEvenement('menu', { payload: id }),
    /** La fenêtre demande à se fermer. */
    fermeture: () => declencheEvenement('fermeture-demandee', {}),
  };
}

module.exports = { El, charge, idsDuHtml };
