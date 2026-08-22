//! Moteur de couverture : réglages typés → source Typst.
//!
//! Il succède au moteur CSS d'`index.html`, gelé. Un seul moteur compose désormais
//! l'aperçu et le PDF, donc l'écart écran/export disparaît — et le rendu ne dépend
//! plus du moteur de la webview, WKWebView sur macOS et WebView2 sur Windows.
//!
//! Deux règles structurent tout ce fichier :
//!
//! - **Tout réglage est en pourcentage de la largeur de couverture**, jamais en mm
//!   absolus. C'est ce qui rend une maquette portable d'un format à l'autre, et ce qui
//!   permet de ne choisir le prestataire qu'à la fin. Seule exception héritée du CSS :
//!   les décalages verticaux posés en `top`/`bottom` sont des pourcentages de la
//!   **hauteur**, parce que c'est ainsi que le positionnement absolu les résout.
//! - **L'identité du livre vient du projet**, pas de la maquette : titre, auteur et
//!   genre sont lus dans `Livre`. La maquette ne porte que ce qui relève d'elle —
//!   l'éditeur, la collection, la mise en page.

use serde::{Deserialize, Serialize};

use crate::image::{self, Cadrage, Geometrie};
use crate::manuscrit::{echappe, echappe_chaine};
use crate::projet::Livre;

/// Familles embarquées avec l'application (`app/outils/polices.sh`).
///
/// Georgia et Helvetica, que proposait l'atelier HTML, n'y sont pas : elles
/// appartiennent au système, ne sont pas redistribuables, et Helvetica n'existe pas
/// sous Windows. Les accepter reviendrait à laisser une maquette rendre différemment
/// selon la machine — exactement ce que l'embarquement doit empêcher.
pub const POLICES: &[&str] = &[
    "Bodoni Moda",
    "Playfair Display",
    "Prata",
    "Spectral",
    "EB Garamond",
    "Libre Baskerville",
    "Archivo",
    "Libre Franklin",
    "Oswald",
];

pub fn police_connue(f: &str) -> bool {
    POLICES.contains(&f)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Mode {
    /// Bande de titre en haut, image à fond perdu dessous (archétype Folio).
    Bandeau,
    /// Image sur toute la surface, texte par-dessus.
    Surimpression,
    /// Composition purement typographique (archétype Blanche).
    Typo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Align {
    Gauche,
    Centre,
    Droite,
}

impl Align {
    fn typst(self) -> &'static str {
        match self {
            Align::Gauche => "left",
            Align::Centre => "center",
            Align::Droite => "right",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Casse {
    Telle,
    Capitales,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Voile {
    Aucun,
    Haut,
    Bas,
    Deux,
    Uni,
    Clair,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FondQuatre {
    /// Le papier de la 1ère.
    Herite,
    /// Une couleur distincte.
    Couleur,
    /// Une image propre à la 4ème.
    Image,
    /// L'image de la 1ère prolongée sur toute la planche.
    Panorama,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Coin {
    BasDroite,
    BasGauche,
    HautDroite,
    HautGauche,
}

/// Un style de texte. `taille` et `tracking` sont en % de la largeur de couverture,
/// sauf `tracking` qui est en centièmes d'em (comme le contrôle d'origine).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Style {
    pub police: String,
    pub graisse: u16,
    #[serde(default)]
    pub italique: bool,
    /// % de la largeur de couverture.
    pub taille: f64,
    pub couleur: String,
    #[serde(default)]
    pub tracking: f64,
    #[serde(default = "casse_telle")]
    pub casse: Casse,
}

fn casse_telle() -> Casse {
    Casse::Telle
}

/// Cadre à filets. Trois niveaux imbriqués, comme le triple filet Gallimard :
/// un filet externe, puis deux filets rapprochés à l'intérieur.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Cadre {
    pub actif: bool,
    /// Marge du cadre. Horizontalement en % de largeur, verticalement en % de hauteur —
    /// le positionnement absolu du CSS résout ainsi, et le dessin en dépend.
    pub marge: f64,
    pub filet1_couleur: String,
    pub filet1_epaisseur: f64,
    /// Décroché entre le filet externe et le premier filet interne.
    pub decroche: f64,
    pub filet2_couleur: String,
    pub filet2_epaisseur: f64,
    /// Écart entre les deux filets internes.
    pub ecart: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Pied {
    pub actif: bool,
    pub monogramme: String,
    pub editeur: String,
    /// % de la hauteur, depuis le bas.
    pub y: f64,
    pub style_mono: Style,
    pub style_editeur: Style,
}

/// Où un élément se cale sur le dos, en lecture de bas en haut.
///
/// Le vocabulaire est celui de la reliure : la **tête** est le haut du livre posé
/// debout, le **pied** son bas. Sur un dos qui se lit de bas en haut, le pied est donc
/// le début de la lecture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PlaceDos {
    Pied,
    Centre,
    Tete,
}

/// Un élément du dos : l'auteur, le titre ou l'éditeur.
///
/// Chacun porte son propre style et sa propre place, parce que les usages divergent :
/// une collection met le titre en tête et son logo en pied, une autre groupe auteur et
/// titre au pied. `rang` départage ceux qui partagent une place.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ElementDos {
    pub actif: bool,
    pub place: PlaceDos,
    /// Ordre au sein d'une même place, du début de la lecture vers la fin.
    pub rang: u8,
    pub style: Style,
}

/// Le dos, tel qu'il paraît sur la planche.
///
/// Il ne porte aucun texte propre — l'auteur et le titre viennent du livre, l'éditeur
/// du pied de la 1ère. Sa **largeur** n'est pas réglable : elle vient de la pagination,
/// et c'est tout l'objet de l'application.
///
/// Les champs sont tous facultatifs à la lecture : un projet écrit avant que le dos
/// ne devienne réglable élément par élément s'ouvre avec les valeurs par défaut plutôt
/// que d'être refusé.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Dos {
    #[serde(default = "dos_auteur")]
    pub auteur: ElementDos,
    #[serde(default = "dos_titre")]
    pub titre: ElementDos,
    #[serde(default = "dos_editeur")]
    pub editeur: ElementDos,
    /// Écart entre deux éléments d'une même place, % de la largeur de couverture.
    #[serde(default = "dos_ecart")]
    pub ecart: f64,
    /// Retrait aux deux extrémités, % de la largeur de couverture.
    #[serde(default = "dos_marge")]
    pub marge: f64,
    /// Fond distinct du papier de la 1ère.
    #[serde(default)]
    pub fond_propre: bool,
    #[serde(default = "dos_fond")]
    pub fond: String,
}

/// Style commun aux trois éléments : c'est celui que portait le dos d'`index.html`.
pub fn dos_style() -> Style {
    Style {
        police: "Archivo".into(),
        graisse: 600,
        italique: false,
        taille: 2.6,
        couleur: "#191917".into(),
        tracking: 0.0,
        casse: Casse::Telle,
    }
}

fn element(place: PlaceDos, rang: u8) -> ElementDos {
    ElementDos {
        actif: true,
        place,
        rang,
        style: dos_style(),
    }
}

fn dos_auteur() -> ElementDos {
    element(PlaceDos::Tete, 1)
}

fn dos_titre() -> ElementDos {
    element(PlaceDos::Tete, 2)
}

fn dos_editeur() -> ElementDos {
    element(PlaceDos::Pied, 1)
}

// Les deux écarts que le CSS d'origine fixait en dur, devenus réglables.
fn dos_ecart() -> f64 {
    2.0
}

fn dos_marge() -> f64 {
    3.0
}

fn dos_fond() -> String {
    "#fcf0d8".into()
}

fn dos_defaut() -> Dos {
    Dos {
        auteur: dos_auteur(),
        titre: dos_titre(),
        editeur: dos_editeur(),
        ecart: dos_ecart(),
        marge: dos_marge(),
        fond_propre: false,
        fond: dos_fond(),
    }
}

impl Dos {
    /// Le dos d'un poche courant : auteur puis titre en tête, mention d'éditeur au
    /// pied. L'atelier HTML faisait l'inverse — c'était une fidélité à son CSS, pas à
    /// un livre. Les maquettes partent de là et n'en changent que ce qui leur est
    /// propre.
    pub fn defaut() -> Self {
        dos_defaut()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Pastille {
    pub actif: bool,
    pub texte: String,
    pub style: Style,
    pub fond: String,
    pub coin: Coin,
    pub verticale: bool,
    pub arrondie: bool,
    pub dx: f64,
    pub dy: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Quatrieme {
    pub fond: FondQuatre,
    pub couleur: String,
    pub texte: String,
    pub style: Style,
    pub interligne: f64,
    pub align: Align,
    /// % de largeur.
    pub pad_x: f64,
    /// % de largeur (le contrôle d'origine l'exprime ainsi, malgré la verticalité).
    pub top: f64,
    pub pied_actif: bool,
    pub mention: String,
    pub collection: String,
    pub prix: String,
    pub style_pied: Style,
    pub pied_y: f64,
    pub isbn_actif: bool,
    pub isbn_l: f64,
    pub isbn_h: f64,
    pub isbn_dx: f64,
    pub isbn_dy: f64,
    pub cadrage: Cadrage,
    pub voile: Voile,
    pub voile_opacite: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Couverture {
    pub mode: Mode,
    pub papier: String,
    pub align: Align,
    /// Marge latérale du bloc texte, % de largeur.
    pub pad_x: f64,
    /// Hauteur du bandeau, % de hauteur. Mode Bandeau seulement.
    pub bandeau: f64,
    /// Image en retrait des bords plutôt qu'à fond perdu. Mode Bandeau seulement.
    pub bandeau_retrait: bool,
    /// Hauteur du bloc texte, % de hauteur. Hors mode Bandeau.
    pub bloc_y: f64,
    pub cadre: Cadre,
    pub auteur: Style,
    pub titre: Style,
    pub titre_interligne: f64,
    /// Écart auteur → titre, % de largeur.
    pub titre_ecart: f64,
    pub genre_visible: bool,
    pub genre: Style,
    /// Écart titre → genre, % de largeur.
    pub genre_ecart: f64,
    pub pied: Pied,
    pub pastille: Pastille,
    pub cadrage: Cadrage,
    pub voile: Voile,
    pub voile_opacite: f64,
    pub quatrieme: Quatrieme,
    /// Absent des projets écrits avant l'assemblage : ils reprennent le dos par défaut
    /// plutôt que d'être refusés à l'ouverture.
    #[serde(default = "dos_defaut")]
    pub dos: Dos,
}

/// Une image disponible pour la composition : son nom de fichier, tel qu'il sera écrit
/// à côté de la source Typst, et ses dimensions naturelles.
#[derive(Debug, Clone, PartialEq)]
pub struct Ressource {
    pub fichier: String,
    pub largeur: u32,
    pub hauteur: u32,
}

impl Ressource {
    pub fn depuis(fichier: &str, octets: &[u8]) -> Option<Self> {
        let (largeur, hauteur) = image::dimensions(octets)?;
        Some(Self {
            fichier: fichier.to_string(),
            largeur,
            hauteur,
        })
    }
}

/* ---------- émission Typst ---------- */

fn mm(v: f64) -> String {
    format!("{v:.4}mm")
}

fn couleur(hex: &str) -> String {
    format!("rgb(\"{}\")", hex.replace('"', ""))
}

/// Une couleur avec alpha, pour les voiles.
fn couleur_alpha(hex: &str, alpha: f64) -> String {
    format!(
        "rgb(\"{}\").transparentize({:.1}%)",
        hex,
        100.0 - alpha * 100.0
    )
}

impl Style {
    /// Corps en mm à partir du % de largeur.
    fn corps(&self, largeur: f64) -> f64 {
        self.taille / 100.0 * largeur
    }

    fn typst_text(&self, largeur: f64) -> String {
        format!(
            "font: \"{}\", weight: {}, style: \"{}\", size: {}, fill: {}, tracking: {}em",
            self.police,
            self.graisse,
            if self.italique { "italic" } else { "normal" },
            mm(self.corps(largeur)),
            couleur(&self.couleur),
            self.tracking / 100.0,
        )
    }

    pub fn applique(&self, largeur: f64, texte: &str) -> String {
        let t = echappe(texte);
        let t = match self.casse {
            Casse::Capitales => format!("#upper[{t}]"),
            Casse::Telle => t,
        };
        format!("text({})[{t}]", self.typst_text(largeur))
    }
}

/// Voile de lisibilité, en fond d'un rectangle.
fn voile_fond(v: Voile, opacite: f64) -> Option<String> {
    let noir = |a: f64| couleur_alpha("#000000", a);
    Some(match v {
        Voile::Aucun => return None,
        Voile::Uni => noir(opacite),
        Voile::Clair => couleur_alpha("#ffffff", opacite),
        // 90deg : de haut en bas dans le repère de Typst.
        Voile::Haut => format!(
            "gradient.linear(angle: 90deg, ({}, 0%), ({}, 55%))",
            noir(opacite),
            noir(0.0)
        ),
        Voile::Bas => format!(
            "gradient.linear(angle: 90deg, ({}, 45%), ({}, 100%))",
            noir(0.0),
            noir(opacite)
        ),
        Voile::Deux => format!(
            "gradient.linear(angle: 90deg, ({}, 0%), ({}, 40%), ({}, 60%), ({}, 100%))",
            noir(opacite),
            noir(0.0),
            noir(0.0),
            noir(opacite)
        ),
    })
}

/// La boîte qu'occupe une face, fond perdu compris, et où s'y trouve la couverture
/// rognée.
///
/// Le fond perdu déborde vers l'**extérieur de la planche** : à droite pour la 1ère,
/// à gauche pour la 4ème, en haut et en bas pour les deux. Jamais vers le dos, où les
/// deux faces se rejoignent — y déborder n'aurait rien à déborder.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Boite {
    pub largeur: f64,
    pub hauteur: f64,
    /// Coin de la couverture rognée dans la boîte.
    pub x0: f64,
    pub y0: f64,
}

impl Boite {
    /// Face isolée : pas de fond perdu, la boîte est la couverture rognée. C'est ce que
    /// montre l'aperçu par face, où il n'y a pas de planche autour.
    pub fn rognee((fw, fh): (f64, f64)) -> Self {
        Self {
            largeur: fw,
            hauteur: fh,
            x0: 0.0,
            y0: 0.0,
        }
    }

    pub fn une((fw, fh): (f64, f64), fp: f64) -> Self {
        Self {
            largeur: fw + fp,
            hauteur: fh + 2.0 * fp,
            x0: 0.0,
            y0: fp,
        }
    }

    pub fn quatre((fw, fh): (f64, f64), fp: f64) -> Self {
        Self {
            largeur: fw + fp,
            hauteur: fh + 2.0 * fp,
            x0: fp,
            y0: fp,
        }
    }

    /// La même boîte, élargie vers la droite. La couverture rognée n'y bouge pas :
    /// seuls le fond, le voile et la zone d'image s'étendent.
    pub fn elargie(self, de: f64) -> Self {
        Self {
            largeur: self.largeur + de,
            ..self
        }
    }
}

/// Zone occupée par l'image de la 1ère dans la boîte, en mm : (x, y, largeur, hauteur).
/// `None` en composition purement typographique.
///
/// Une image à fond perdu s'étend jusqu'aux bords de la **boîte**, fond perdu compris :
/// sans quoi le rognage découvrirait une bande de papier là où la photo devait courir.
/// Une image en retrait, elle, ne touche aucun bord et reste où la maquette la met.
fn zone_image(cv: &Couverture, (fw, fh): (f64, f64), b: Boite) -> Option<(f64, f64, f64, f64)> {
    match cv.mode {
        Mode::Typo => None,
        Mode::Surimpression => Some((0.0, 0.0, b.largeur, b.hauteur)),
        Mode::Bandeau => {
            // Le retrait est le même pourcentage lu sur la largeur horizontalement et
            // sur la hauteur verticalement : c'est ainsi que le CSS le résolvait.
            let (rx, ry) = if cv.bandeau_retrait {
                (cv.pad_x / 100.0 * fw, cv.pad_x / 100.0 * fh)
            } else {
                (0.0, 0.0)
            };
            // Le haut de l'image est toujours sous le bandeau, qui couvre le fond perdu
            // supérieur avec le papier.
            let haut = b.y0 + cv.bandeau / 100.0 * fh;
            let gauche = if rx > 0.0 { b.x0 + rx } else { 0.0 };
            let droite = if rx > 0.0 { b.x0 + fw - rx } else { b.largeur };
            let bas = if ry > 0.0 { b.y0 + fh - ry } else { b.hauteur };
            Some((gauche, haut, droite - gauche, bas - haut))
        }
    }
}

/// Rectangle de fond couvrant toute la boîte. Un `fill` de page ne suffirait pas :
/// dans la planche, les trois zones ont chacune le leur.
fn bloc_fond(b: Boite, couleur_hex: &str) -> String {
    format!(
        "#place(top + left, rect(width: {}, height: {}, fill: {}))\n",
        mm(b.largeur),
        mm(b.hauteur),
        couleur(couleur_hex)
    )
}

/// Contenu positionné par rapport à la couverture **rognée**, dans un bloc à sa taille.
/// Cadre, textes, pied et pastille se placent ainsi sans jamais connaître le fond perdu.
fn cale(b: Boite, (fw, fh): (f64, f64), contenu: &str) -> String {
    if contenu.is_empty() {
        return String::new();
    }
    format!(
        "#place(top + left, dx: {}, dy: {}, block(width: {}, height: {})[\n{contenu}])\n",
        mm(b.x0),
        mm(b.y0),
        mm(fw),
        mm(fh),
    )
}

/// Image posée dans une zone, découpée à ses bords.
///
/// Le nom du fichier est cité, donc échappé : `image_choisir` le fabrique, mais
/// l'ouverture d'un `.ozalid` prend celui que l'archive porte, et un guillemet droit y
/// refermerait la chaîne. C'est le seul chemin d'image dans ce cas — celui d'un envoi
/// est assaini par `envoi::nom_image`.
pub fn bloc_image(zone: (f64, f64, f64, f64), g: &Geometrie, fichier: &str) -> String {
    let (x, y, w, h) = zone;
    let fichier = echappe_chaine(fichier);
    format!(
        "#place(top + left, dx: {}, dy: {}, box(width: {}, height: {}, clip: true,\n  \
         place(top + left, dx: {}, dy: {}, image(\"{}\", width: {}, height: {}, fit: \"stretch\"))))\n",
        mm(x),
        mm(y),
        mm(w),
        mm(h),
        mm(g.gauche),
        mm(g.haut),
        fichier,
        mm(g.largeur),
        mm(g.hauteur),
    )
}

/// Les trois filets du cadre, chacun tracé depuis la boîte intérieure au précédent.
fn bloc_cadre(c: &Cadre, (fw, fh): (f64, f64)) -> String {
    if !c.actif {
        return String::new();
    }
    let (mut x, mut y) = (c.marge / 100.0 * fw, c.marge / 100.0 * fh);
    let (mut w, mut h) = (fw - 2.0 * x, fh - 2.0 * y);
    let e1 = c.filet1_epaisseur / 100.0 * fw;
    let e2 = c.filet2_epaisseur / 100.0 * fw;
    let niveaux = [
        (e1, c.filet1_couleur.as_str(), 0.0),
        (e2, c.filet2_couleur.as_str(), c.decroche / 100.0 * fw),
        (e2, c.filet2_couleur.as_str(), c.ecart / 100.0 * fw),
    ];
    let mut precedent = 0.0;
    let mut out = String::new();
    for (ep, col, decroche) in niveaux {
        if decroche > 0.0 {
            let d = precedent + decroche;
            x += d;
            y += d;
            w -= 2.0 * d;
            h -= 2.0 * d;
        }
        if w <= 0.0 || h <= 0.0 {
            break;
        }
        out.push_str(&format!(
            "#place(top + left, dx: {}, dy: {}, rect(width: {}, height: {}, stroke: {} + {}))\n",
            mm(x),
            mm(y),
            mm(w),
            mm(h),
            mm(ep),
            couleur(col)
        ));
        precedent = ep;
    }
    out
}

/// Bloc auteur / titre / genre.
fn bloc_texte(livre: &Livre, cv: &Couverture, (fw, fh): (f64, f64)) -> String {
    // Mode Bandeau : le bloc se cale dans la bande, à 22 % de sa hauteur.
    let y = match cv.mode {
        Mode::Bandeau => cv.bandeau * 0.22,
        _ => cv.bloc_y,
    } / 100.0
        * fh;
    let pad = cv.pad_x / 100.0 * fw;

    let mut corps = format!("#set align({})\n", cv.align.typst());
    corps.push_str(&format!(
        "#set par(leading: 0.08em, spacing: 0em)\n#{}\n",
        cv.auteur.applique(fw, &livre.auteur)
    ));
    corps.push_str(&format!(
        "#v({})\n#set par(leading: {}em)\n#{}\n",
        mm(cv.titre_ecart / 100.0 * fw),
        cv.titre_interligne - 1.0,
        cv.titre.applique(fw, &livre.titre)
    ));
    if cv.genre_visible {
        corps.push_str(&format!(
            "#v({})\n#set par(leading: 0em)\n#{}\n",
            mm(cv.genre_ecart / 100.0 * fw),
            cv.genre.applique(fw, &livre.genre)
        ));
    }
    format!(
        "#place(top + left, dx: {}, dy: {}, block(width: {})[\n{corps}])\n",
        mm(pad),
        mm(y),
        mm(fw - 2.0 * pad)
    )
}

fn bloc_pied(p: &Pied, cv: &Couverture, (fw, fh): (f64, f64)) -> String {
    if !p.actif {
        return String::new();
    }
    let pad = cv.pad_x / 100.0 * fw;
    // L'écart monogramme → éditeur est de 6 % de la largeur, fixé par le CSS d'origine.
    format!(
        "#place(bottom + left, dx: {}, dy: -{}, block(width: {})[\n\
         #set align({})\n#set par(leading: 0em, spacing: 0em)\n\
         #{}\n#v({})\n#{}\n])\n",
        mm(pad),
        mm(p.y / 100.0 * fh),
        mm(fw - 2.0 * pad),
        cv.align.typst(),
        p.style_mono.applique(fw, &p.monogramme),
        mm(0.06 * fw),
        p.style_editeur.applique(fw, &p.editeur),
    )
}

fn bloc_pastille(p: &Pastille, fw: f64) -> String {
    if !p.actif || p.texte.trim().is_empty() {
        return String::new();
    }
    let coin = match p.coin {
        Coin::BasDroite => "bottom + right",
        Coin::BasGauche => "bottom + left",
        Coin::HautDroite => "top + right",
        Coin::HautGauche => "top + left",
    };
    let (sx, sy) = match p.coin {
        Coin::BasDroite => (-1.0, -1.0),
        Coin::BasGauche => (1.0, -1.0),
        Coin::HautDroite => (-1.0, 1.0),
        Coin::HautGauche => (1.0, 1.0),
    };
    // L'interlettrage de la pastille est fixé par la maquette d'origine, pas réglable :
    // il ne suit donc pas le tracking du style.
    let style = Style {
        tracking: 2.0,
        ..p.style.clone()
    };
    let badge = format!(
        "box(fill: {}, inset: (x: {}, y: {}), radius: {}, text({})[{}])",
        couleur(&p.fond),
        mm(0.028 * fw),
        mm(0.012 * fw),
        mm(if p.arrondie { 0.02 * fw } else { 0.0 }),
        style.typst_text(fw),
        echappe(&p.texte),
    );
    // Quart de tour anti-horaire : la pastille se lit de bas en haut, calée dans son
    // coin. `reflow: true` donne à la boîte ses dimensions tournées, sans quoi le
    // placement dans le coin porterait sur la boîte d'avant rotation.
    let contenu = if p.verticale {
        format!("rotate(-90deg, reflow: true, {badge})")
    } else {
        badge
    };
    format!(
        "#place({coin}, dx: {}, dy: {}, {contenu})\n",
        mm(sx * p.dx / 100.0 * fw),
        mm(sy * p.dy / 100.0 * fw),
    )
}

/// Préambule d'une page d'une seule face. La planche a le sien.
fn preambule(largeur: f64, hauteur: f64) -> String {
    format!(
        "#set page(width: {}, height: {}, margin: 0mm)\n\
         // Boîte de ligne ramenée à 1em : « leading » Typst et « line-height » CSS\n\
         // deviennent alors la même grandeur.\n\
         #set text(lang: \"fr\", top-edge: 0.75em, bottom-edge: -0.25em)\n\
         #set par(leading: 0em, spacing: 0em, justify: false)\n\n",
        mm(largeur),
        mm(hauteur),
    )
}

/// Où se cadre l'image quand la 4ème prolonge la 1ère : sur la planche entière, et non
/// sur la seule 1ère.
///
/// C'est le point où l'on s'écarte d'`index.html`, sciemment. L'atelier cadrait l'image
/// sur une couverture puis la décalait pour la 4ème : elle n'y arrivait donc jamais, et
/// il fallait la grossir à la main. Or le zoom se prend autour du point d'ancrage — le
/// centre de la 1ère — si bien qu'aucun zoom raisonnable n'atteignait le bord gauche de
/// la planche. L'atelier affichait « il manque de l'image » et laissait l'utilisateur
/// avec le problème. Une image panoramique se cadre sur ce qu'elle doit couvrir.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Panorama {
    /// Largeur de la planche entière, fond perdu compris.
    pub largeur: f64,
    /// Abscisse du bord gauche de la planche dans la boîte courante — donc négative
    /// pour la 1ère, qui est à droite du dos.
    pub x_zone: f64,
}

/// Zone de l'image de la 1ère dans une boîte, et la géométrie de l'image dedans.
///
/// Toutes les zones qui portent l'image en prolongement — 4ème, dos, 1ère — passent
/// par ici avec le même `pano` : elles obtiennent donc la même géométrie et ne peuvent
/// pas se décaler l'une de l'autre. Un cheveu d'écart, au pli, se voit.
pub fn image_une(
    cv: &Couverture,
    format: (f64, f64),
    image: Option<&Ressource>,
    b: Boite,
    pano: Option<Panorama>,
) -> Option<((f64, f64, f64, f64), Geometrie)> {
    let mut zone = zone_image(cv, format, b)?;
    if let Some(p) = pano {
        if cv.quatrieme.fond == FondQuatre::Panorama {
            // Même bande verticale, étendue à toute la planche.
            zone = (p.x_zone, zone.1, p.largeur, zone.3);
        }
    }
    let r = image?;
    let g = image::place((zone.2, zone.3), (r.largeur, r.hauteur), &cv.cadrage)?;
    Some((zone, g))
}

/// Corps de la 1ère de couverture, dans la boîte donnée.
///
/// Rien n'y fixe la page : c'est ce qui permet de composer la même face seule, pour
/// l'aperçu, et posée dans la planche, sans que les deux puissent diverger.
pub fn corps_une(
    livre: &Livre,
    cv: &Couverture,
    format: (f64, f64),
    image: Option<&Ressource>,
    b: Boite,
    pano: Option<Panorama>,
) -> String {
    let (fw, _) = format;
    let mut s = bloc_fond(b, &cv.papier);

    if let (Some((zone, g)), Some(r)) = (image_une(cv, format, image, b, pano), image) {
        s.push_str(&bloc_image(zone, &g, &r.fichier));
    }
    // Le voile couvre toute la face, pas seulement la zone image : c'est ce que faisait
    // le CSS, et le bandeau en dépend visuellement.
    if cv.mode != Mode::Typo {
        if let Some(f) = voile_fond(cv.voile, cv.voile_opacite) {
            s.push_str(&format!(
                "#place(top + left, rect(width: {}, height: {}, fill: {f}, stroke: none))\n",
                mm(b.largeur),
                mm(b.hauteur)
            ));
        }
    }

    let mut cadre = bloc_cadre(&cv.cadre, format);
    cadre.push_str(&bloc_texte(livre, cv, format));
    cadre.push_str(&bloc_pied(&cv.pied, cv, format));
    cadre.push_str(&bloc_pastille(&cv.pastille, fw));
    s.push_str(&cale(b, format, &cadre));
    s
}

/// Le prolongement panoramique tel que le voit une face **isolée**, sans fond perdu :
/// la planche y est réduite à deux couvertures et un dos.
///
/// L'aperçu par face doit montrer le même cadrage que la planche, sans quoi on règle
/// la couverture sur une image qui ne sera pas celle imprimée.
pub fn panorama_face(format: (f64, f64), dos_mm: Option<f64>, face_une: bool) -> Option<Panorama> {
    let dos = dos_mm?;
    let fw = format.0;
    Some(Panorama {
        largeur: 2.0 * fw + dos,
        // La 4ème est le bord gauche de la planche ; la 1ère en est à une couverture et
        // un dos, d'où une abscisse négative.
        x_zone: if face_une { -(fw + dos) } else { 0.0 },
    })
}

/// Source Typst de la 1ère de couverture, seule sur sa page.
pub fn source_une(
    livre: &Livre,
    cv: &Couverture,
    format: (f64, f64),
    image: Option<&Ressource>,
    dos_mm: Option<f64>,
) -> String {
    let b = Boite::rognee(format);
    let pano = panorama_face(format, dos_mm, true);
    preambule(b.largeur, b.hauteur) + &corps_une(livre, cv, format, image, b, pano)
}

/// Corps de la 4ème de couverture, dans la boîte donnée.
///
/// `pano` n'est requis que pour le prolongement : il porte la largeur de la planche,
/// donc celle du dos, donc la pagination. C'est le couplage que l'application existe
/// pour tenir — ici, il est explicite au lieu d'être recopié à la main, et la 4ème
/// refuse de se composer sans lui plutôt que de se composer de travers.
pub fn corps_quatre(
    cv: &Couverture,
    format: (f64, f64),
    image_quatre: Option<&Ressource>,
    photo_une: Option<&Ressource>,
    pano: Option<Panorama>,
    b: Boite,
) -> Result<String, String> {
    let (fw, _) = format;
    let q = &cv.quatrieme;
    let fond = match q.fond {
        FondQuatre::Couleur => &q.couleur,
        _ => &cv.papier,
    };
    let mut s = bloc_fond(b, fond);

    match q.fond {
        FondQuatre::Image => {
            if let Some(r) = image_quatre {
                let zone = (0.0, 0.0, b.largeur, b.hauteur);
                if let Some(g) = image::place((zone.2, zone.3), (r.largeur, r.hauteur), &q.cadrage)
                {
                    s.push_str(&bloc_image(zone, &g, &r.fichier));
                }
            }
        }
        FondQuatre::Panorama => {
            let p = pano.ok_or_else(|| {
                "prolongement panoramique : la largeur du dos est inconnue — composer \
                 l'intérieur d'abord, la pagination la détermine."
                    .to_string()
            })?;
            let r = photo_une.ok_or_else(|| {
                "prolongement panoramique : la 1ère n'a pas d'image à prolonger.".to_string()
            })?;
            // Rigoureusement le même appel que la 1ère et que le dos : la zone est celle
            // de la planche entière, chaque face n'en montre que sa part. Le raccord au
            // pli est donc exact par construction, et non par un décalage à recalculer.
            let (zone, g) = image_une(cv, format, Some(r), b, Some(p)).ok_or_else(|| {
                "prolongement panoramique : sans objet en composition typographique.".to_string()
            })?;
            s.push_str(&bloc_image(zone, &g, &r.fichier));
        }
        _ => {}
    }

    let avec_image = matches!(q.fond, FondQuatre::Image | FondQuatre::Panorama);
    if avec_image {
        if let Some(f) = voile_fond(q.voile, q.voile_opacite) {
            s.push_str(&format!(
                "#place(top + left, rect(width: {}, height: {}, fill: {f}, stroke: none))\n",
                mm(b.largeur),
                mm(b.hauteur)
            ));
        }
    }

    let mut c = String::new();
    let pad = q.pad_x / 100.0 * fw;
    if !q.texte.trim().is_empty() {
        c.push_str(&format!(
            "#place(top + left, dx: {}, dy: {}, block(width: {})[\n\
             #set align({})\n#set par(leading: {}em, spacing: {}em, justify: false)\n\
             #{}\n])\n",
            mm(pad),
            mm(q.top / 100.0 * fw),
            mm(fw - 2.0 * pad),
            q.align.typst(),
            q.interligne - 1.0,
            q.interligne - 1.0,
            q.style.applique(fw, &q.texte),
        ));
    }

    if q.pied_actif {
        let lignes: Vec<String> = [&q.mention, &q.collection, &q.prix]
            .iter()
            .map(|v| v.trim())
            .filter(|v| !v.is_empty())
            .map(|v| format!("#{}", q.style_pied.applique(fw, v)))
            .collect();
        if !lignes.is_empty() {
            c.push_str(&format!(
                "#place(bottom + left, dx: {}, dy: -{}, block(width: {})[\n\
                 #set align(center)\n#set par(leading: 0.5em, spacing: 0.5em)\n{}\n])\n",
                mm(pad),
                mm(q.pied_y / 100.0 * fw),
                mm(fw - 2.0 * pad),
                lignes.join("\n\n"),
            ));
        }
    }

    if q.isbn_actif {
        c.push_str(&format!(
            "#place(bottom + right, dx: -{}, dy: -{}, \
             rect(width: {}, height: {}, fill: rgb(\"#ffffff\"), stroke: none))\n",
            mm(q.isbn_dx / 100.0 * fw),
            mm(q.isbn_dy / 100.0 * fw),
            mm(q.isbn_l / 100.0 * fw),
            mm(q.isbn_h / 100.0 * fw),
        ));
    }
    s.push_str(&cale(b, format, &c));
    Ok(s)
}

/// Source Typst de la 4ème de couverture, seule sur sa page.
pub fn source_quatre(
    cv: &Couverture,
    format: (f64, f64),
    image_quatre: Option<&Ressource>,
    image_une: Option<&Ressource>,
    dos_mm: Option<f64>,
) -> Result<String, String> {
    let b = Boite::rognee(format);
    let pano = panorama_face(format, dos_mm, false);
    let corps = corps_quatre(cv, format, image_quatre, image_une, pano, b)?;
    Ok(preambule(b.largeur, b.hauteur) + &corps)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::maquettes;

    fn livre() -> Livre {
        Livre {
            titre: "Les Heures creuses".into(),
            titre_page: None,
            auteur: "Ivan Pjig".into(),
            genre: "roman".into(),
            copyright: String::new(),
            dedicace: None,
            chapitres: None,
        }
    }

    const FORMAT: (f64, f64) = (110.0, 180.0);

    fn photo() -> Ressource {
        Ressource {
            fichier: "couverture.jpg".into(),
            largeur: 1200,
            hauteur: 1980,
        }
    }

    /// Le nom de l'image entre dans une chaîne Typst, et il ne vient pas toujours de
    /// l'application : `image_choisir` le fabrique — `couverture.jpg` — mais l'ouverture
    /// d'un `.ozalid` prend celui que l'archive porte, quel qu'il soit. Un guillemet
    /// droit y referme la chaîne et la couverture ne compose plus, sur un fichier que
    /// l'utilisateur n'a pas écrit lui-même.
    #[test]
    fn un_nom_d_image_a_guillemets_ne_referme_pas_la_chaine() {
        let g = Geometrie {
            gauche: 0.0,
            haut: 0.0,
            largeur: 100.0,
            hauteur: 150.0,
        };
        let s = bloc_image((0.0, 0.0, 100.0, 150.0), &g, r#"ma "photo".jpg"#);
        assert!(
            s.contains(r#"image("ma \"photo\".jpg""#),
            "nom d'image non échappé : {s}"
        );
    }

    /// Une maquette est portable d'un format à l'autre : c'est la promesse des
    /// réglages en pourcentage. Doubler la largeur doit doubler le corps du titre,
    /// jamais laisser une valeur figée derrière.
    #[test]
    fn une_maquette_suit_le_format_sans_valeur_figee() {
        let cv = maquettes::folio();
        let petit = source_une(&livre(), &cv, (100.0, 160.0), None, None);
        let grand = source_une(&livre(), &cv, (200.0, 320.0), None, None);
        let corps = |s: &str| {
            let i = s.find("size: ").unwrap() + 6;
            s[i..].split("mm").next().unwrap().parse::<f64>().unwrap()
        };
        let (a, b) = (corps(&petit), corps(&grand));
        assert!((b - 2.0 * a).abs() < 0.001, "{a} puis {b}");
    }

    /// En composition typographique il n'y a ni image ni voile : émettre le voile
    /// poserait un rectangle noir sur une couverture qui n'a pas d'image dessous.
    #[test]
    fn la_composition_typographique_n_emet_ni_image_ni_voile() {
        let mut cv = maquettes::blanche();
        cv.voile = Voile::Uni;
        cv.voile_opacite = 0.5;
        let s = source_une(&livre(), &cv, FORMAT, Some(&photo()), None);
        assert!(!s.contains("image("), "image émise en mode typo");
        assert!(!s.contains("gradient"), "voile émis en mode typo");
    }

    /// Le cadre est le dessin le plus scruté de la maquette : trois filets, dans
    /// l'ordre, avec les bonnes couleurs.
    #[test]
    fn le_cadre_emet_trois_filets_dans_l_ordre() {
        let cv = maquettes::blanche();
        assert!(cv.cadre.actif);
        let s = source_une(&livre(), &cv, FORMAT, None, None);
        // Le fond de la face est un rectangle lui aussi : seuls les filets portent un
        // contour, c'est ce qui les distingue.
        let filets = s.matches("stroke: ").count();
        assert_eq!(filets, 3, "trois filets attendus");
        // Externe noir, puis les deux internes rouges.
        let pos_noir = s.find("#000000").unwrap();
        let pos_rouge = s.find("#c00000").unwrap();
        assert!(pos_noir < pos_rouge, "filet externe après les internes");
    }

    #[test]
    fn un_cadre_inactif_n_emet_rien() {
        let cv = maquettes::folio();
        assert!(!cv.cadre.actif);
        assert!(!source_une(&livre(), &cv, FORMAT, None, None).contains("stroke:"));
    }

    /// L'identité du livre vient du projet : la maquette ne doit pas pouvoir la
    /// contredire. Changer de maquette ne change jamais le titre imprimé.
    #[test]
    fn le_titre_et_l_auteur_viennent_du_projet() {
        for cv in [
            maquettes::folio(),
            maquettes::blanche(),
            maquettes::surimpression(),
        ] {
            let s = source_une(&livre(), &cv, FORMAT, None, None);
            assert!(s.contains("Les Heures creuses"), "{:?}", cv.mode);
            assert!(s.contains("Ivan Pjig"), "{:?}", cv.mode);
        }
    }

    /// Le bandeau réserve le haut de la couverture : l'image commence dessous.
    #[test]
    fn le_bandeau_pousse_l_image_sous_la_bande() {
        let cv = maquettes::folio();
        let s = source_une(&livre(), &cv, FORMAT, Some(&photo()), None);
        let dy = s
            .split("dy: ")
            .nth(1)
            .and_then(|x| x.split("mm").next())
            .unwrap()
            .parse::<f64>()
            .unwrap();
        assert!(
            (dy - cv.bandeau / 100.0 * FORMAT.1).abs() < 0.01,
            "image à {dy} mm"
        );
    }

    #[test]
    fn la_surimpression_couvre_toute_la_couverture() {
        let cv = maquettes::surimpression();
        let s = source_une(&livre(), &cv, FORMAT, Some(&photo()), None);
        assert!(s.contains(&format!("width: {}", mm(FORMAT.0))));
        assert!(s.contains("gradient.linear"), "voile attendu");
    }

    /// Le prolongement panoramique dépend du dos, donc de la pagination. Le composer
    /// sans elle produirait une 4ème décalée — et personne ne le verrait avant tirage.
    #[test]
    fn le_prolongement_refuse_de_composer_sans_le_dos() {
        let mut cv = maquettes::folio();
        cv.quatrieme.fond = FondQuatre::Panorama;
        let err = source_quatre(&cv, FORMAT, None, Some(&photo()), None).unwrap_err();
        assert!(err.contains("dos"), "{err}");
        assert!(err.contains("pagination"), "{err}");
    }

    /// En prolongement, l'image se cadre sur la **planche entière** — deux couvertures
    /// et le dos — et non sur la seule face qui la porte. C'est ce qui la fait couvrir
    /// la 4ème sans réglage supplémentaire, et ce qui fait qu'un dos plus large élargit
    /// la zone de cadrage. L'atelier HTML cadrait sur une couverture et laissait la
    /// 4ème en papier nu : la divergence est ici, et elle est voulue.
    #[test]
    fn le_prolongement_cadre_l_image_sur_la_planche_entiere() {
        let mut cv = maquettes::folio();
        cv.quatrieme.fond = FondQuatre::Panorama;
        let largeur_zone = |dos: f64| {
            let s = source_quatre(&cv, FORMAT, None, Some(&photo()), Some(dos)).unwrap();
            let i = s.find("image(\"").unwrap();
            s[..i]
                .rsplit("box(width: ")
                .next()
                .unwrap()
                .split("mm")
                .next()
                .unwrap()
                .parse::<f64>()
                .unwrap()
        };
        assert!((largeur_zone(0.0) - 2.0 * FORMAT.0).abs() < 0.01);
        assert!((largeur_zone(17.43) - (2.0 * FORMAT.0 + 17.43)).abs() < 0.01);
    }

    /// La zone ISBN est laissée vide et blanche : le code-barres est posé par le
    /// prestataire. En imprimer un serait le pire des services.
    #[test]
    fn la_zone_isbn_est_un_rectangle_blanc_vide() {
        let mut cv = maquettes::folio();
        cv.quatrieme.isbn_actif = true;
        let s = source_quatre(&cv, FORMAT, None, None, None).unwrap();
        assert!(s.contains("fill: rgb(\"#ffffff\")"));
        assert!(!s.contains("isbn"), "aucun contenu dans la zone");
    }

    /// Les trois maquettes doivent rester composables : c'est ce que promet le bouton
    /// qui les charge.
    #[test]
    fn les_trois_maquettes_composent_les_deux_faces() {
        for cv in [
            maquettes::folio(),
            maquettes::blanche(),
            maquettes::surimpression(),
        ] {
            assert!(!source_une(&livre(), &cv, FORMAT, Some(&photo()), None).is_empty());
            source_quatre(&cv, FORMAT, None, Some(&photo()), Some(15.0)).unwrap();
        }
    }

    /// Toutes les polices des maquettes doivent être embarquées, sans quoi Typst
    /// substituerait en silence et le rendu changerait d'une machine à l'autre.
    #[test]
    fn les_maquettes_n_utilisent_que_des_polices_embarquees() {
        for cv in [
            maquettes::folio(),
            maquettes::blanche(),
            maquettes::surimpression(),
        ] {
            for st in [
                &cv.auteur,
                &cv.titre,
                &cv.genre,
                &cv.pied.style_mono,
                &cv.pied.style_editeur,
                &cv.pastille.style,
                &cv.quatrieme.style,
                &cv.quatrieme.style_pied,
            ] {
                assert!(police_connue(&st.police), "police absente : {}", st.police);
            }
        }
    }

    /// Rien de ce qui vient du projet ou de la maquette ne doit pouvoir ouvrir une
    /// expression Typst : un titre contenant `#` casserait la composition.
    #[test]
    fn le_texte_saisi_ne_peut_pas_injecter_de_syntaxe_typst() {
        let mut l = livre();
        l.titre = "Le #Titre".into();
        let mut cv = maquettes::folio();
        cv.pastille.texte = "col#lection".into();
        let s = source_une(&l, &cv, FORMAT, None, None);
        assert!(s.contains(r"Le \#Titre"));
        assert!(s.contains(r"col\#lection"));
    }
}
