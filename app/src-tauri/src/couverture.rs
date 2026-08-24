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

/// Hauteur d'encre d'une ligne, en fraction du corps, famille par famille.
///
/// Typst compose une ligne à la hauteur de sa capitale — ses réglages par défaut sont
/// `top-edge: "cap-height"`, `bottom-edge: "baseline"` —, mais l'encre déborde cette
/// boîte des deux côtés : un « j » sous la ligne de base, un accent au-dessus des
/// capitales. C'est l'encre qu'un dos doit contenir, pas la boîte de mise en page.
/// D'où l'extension que la fonte déclare elle-même, de l'ascendante à la descendante,
/// délibérément généreuse : une alerte qui manque un titre rogné ne sert à rien, une
/// alerte de trop se lève en regardant la vignette.
///
/// Relevé sur les fichiers de `app/src-tauri/fonts` avec le Typst épinglé (0.15.1) :
/// `measure(text(font: f, size: 10pt, top-edge: "ascender", bottom-edge: "descender")[…])`
/// divisé par le corps. Une famille ajoutée à [`POLICES`] s'ajoute ici — c'est ce que
/// tient le test `toute_police_declare_son_encre`.
const ENCRE: &[(&str, f64)] = &[
    ("Bodoni Moda", 1.525),
    ("Playfair Display", 1.333),
    ("Prata", 1.355),
    ("Spectral", 1.522),
    ("EB Garamond", 1.305),
    ("Libre Baskerville", 1.240),
    ("Archivo", 1.088),
    ("Libre Franklin", 1.212),
    ("Oswald", 1.482),
];

/// Une famille inconnue prend la plus haute encre de la table : une maquette venue
/// d'ailleurs doit lever une alerte de trop, jamais une de moins.
fn encre(police: &str) -> f64 {
    ENCRE.iter().find(|(f, _)| *f == police).map_or_else(
        || ENCRE.iter().map(|(_, h)| *h).fold(0.0, f64::max),
        |(_, h)| *h,
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Mode {
    /// Bande de titre en haut, image à fond perdu dessous (archétype Bandeau).
    Bandeau,
    /// Image sur toute la surface, texte par-dessus.
    Surimpression,
    /// Composition purement typographique : ni photo ni bandeau, le cadre pour seul ornement.
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
    /// Quart de tour d'affichage, en degrés depuis la lecture de bas en haut.
    ///
    /// Zéro est le sens du dos : le texte se lit le livre couché sur le dos, ou la tête
    /// penchée à gauche. 180 le retourne. Les deux quarts de tour le couchent **en
    /// travers** du dos — le sens d'une mention de collection qui se lit le livre
    /// debout, sans pencher la tête. Ce qu'il occupe alors le long du dos n'est plus la
    /// longueur de sa ligne mais sa hauteur d'encre, et réciproquement : c'est pourquoi
    /// [`crate::planche::source_mesures`] mesure le texte déjà tourné.
    ///
    /// Facultatif à la lecture : un projet écrit avant que le sens n'existe s'ouvre
    /// dans celui du dos.
    #[serde(default)]
    pub sens: u16,
    pub style: Style,
}

/// Le dos, tel qu'il paraît sur la planche.
///
/// Il ne porte aucun texte propre : l'auteur, le titre et l'éditeur viennent du livre. Sa **largeur** n'est pas réglable : elle vient de la pagination,
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
    #[serde(default = "dos_collection")]
    pub collection: ElementDos,
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
        sens: 0,
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

/// La collection, au pied avec la mention d'éditeur — et **éteinte**.
///
/// Allumée d'office, elle ajouterait un texte au dos de tous les livres qui portent une
/// collection, donc leur réclamerait de l'épaisseur, pour un réglage que personne n'a
/// demandé. Les projets écrits avant qu'elle n'existe s'ouvrent ainsi inchangés.
fn dos_collection() -> ElementDos {
    ElementDos {
        actif: false,
        ..element(PlaceDos::Pied, 2)
    }
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
        collection: dos_collection(),
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
    pub style: Style,
    pub fond: String,
    pub coin: Coin,
    pub verticale: bool,
    pub arrondie: bool,
    pub dx: f64,
    pub dy: f64,
}

/// Un filet de séparation : une ligne, sa couleur et son épaisseur.
///
/// Largeur et épaisseur en % de la largeur de couverture, comme tout le reste de la
/// maquette — c'est ce qui la rend portable d'un format à l'autre.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Filet {
    pub couleur: String,
    pub epaisseur: f64,
    pub largeur: f64,
}

impl Default for Filet {
    fn default() -> Self {
        Self {
            couleur: "#191917".into(),
            epaisseur: 0.3,
            largeur: 12.0,
        }
    }
}

/// La tête de la 4ème : l'auteur, le titre et un filet, dans cet ordre, au-dessus du
/// texte de présentation.
///
/// Trois interrupteurs et non un seul : une collection met l'auteur et le filet sans
/// répéter le titre, une autre le titre seul. Chacun porte son style entier — la police,
/// la graisse et la couleur y sont, comme partout ailleurs dans la maquette.
///
/// L'identité, elle, n'est pas ici : l'auteur et le titre composés sont **ceux du
/// livre**. Une maquette dit où et comment ça paraît, jamais ce qui est écrit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeteQuatre {
    /// L'alignement de la tête, distinct de celui du texte : une tête centrée au-dessus
    /// d'un résumé justifié est la mise en page la plus courante.
    #[serde(default = "align_centre")]
    pub align: Align,
    #[serde(default)]
    pub auteur_visible: bool,
    #[serde(default = "auteur_quatre_defaut")]
    pub auteur: Style,
    /// Écart auteur → titre, % de largeur.
    #[serde(default = "ecart_tete_defaut")]
    pub titre_ecart: f64,
    #[serde(default)]
    pub titre_visible: bool,
    #[serde(default = "titre_quatre_defaut")]
    pub titre: Style,
    /// Écart titre → filet, % de largeur.
    #[serde(default = "ecart_tete_defaut")]
    pub filet_ecart: f64,
    #[serde(default)]
    pub filet_visible: bool,
    #[serde(default)]
    pub filet: Filet,
    /// Écart tête → texte, % de largeur.
    #[serde(default = "ecart_texte_defaut")]
    pub ecart: f64,
}

fn align_centre() -> Align {
    Align::Centre
}

fn ecart_tete_defaut() -> f64 {
    2.5
}

fn ecart_texte_defaut() -> f64 {
    6.0
}

/// Les deux styles de la tête reprennent l'exemple qui l'a demandée : l'auteur en
/// linéale grasse, le titre en capitales espacées. Ils ne composent rien tant que leur
/// interrupteur est éteint — ce ne sont que des valeurs de départ à retoucher.
fn auteur_quatre_defaut() -> Style {
    Style {
        police: "Archivo".into(),
        graisse: 700,
        italique: false,
        taille: 2.6,
        couleur: "#191917".into(),
        tracking: 6.0,
        casse: Casse::Capitales,
    }
}

fn titre_quatre_defaut() -> Style {
    Style {
        police: "Spectral".into(),
        graisse: 400,
        italique: false,
        taille: 3.4,
        couleur: "#191917".into(),
        tracking: 14.0,
        casse: Casse::Capitales,
    }
}

impl Default for TeteQuatre {
    fn default() -> Self {
        Self {
            align: align_centre(),
            auteur_visible: false,
            auteur: auteur_quatre_defaut(),
            titre_ecart: ecart_tete_defaut(),
            titre_visible: false,
            titre: titre_quatre_defaut(),
            filet_ecart: ecart_tete_defaut(),
            filet_visible: false,
            filet: Filet::default(),
            ecart: ecart_texte_defaut(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Quatrieme {
    pub fond: FondQuatre,
    pub couleur: String,
    /// La tête de la 4ème — auteur, titre, filet —, au-dessus du texte.
    ///
    /// Absente des maquettes et des projets écrits avant elle : ils reprennent une tête
    /// éteinte plutôt que d'être refusés, et leur 4ème ne change pas.
    #[serde(default)]
    pub tete: TeteQuatre,
    pub texte: String,
    pub style: Style,
    pub interligne: f64,
    /// Écart entre deux paragraphes du texte de présentation, % de largeur, **en plus**
    /// de l'espacement ordinaire.
    ///
    /// L'interligne sépare les lignes d'un même passage ; celui-ci sépare les passages.
    /// Une 4ème n'a ni alinéa ni blanc de série : sans lui, deux paragraphes s'y lisent
    /// comme un seul. Nul dans les maquettes d'avant, qui composent donc comme avant.
    #[serde(default)]
    pub paragraphe_ecart: f64,
    pub align: Align,
    /// % de largeur.
    pub pad_x: f64,
    /// % de largeur (le contrôle d'origine l'exprime ainsi, malgré la verticalité).
    pub top: f64,
    pub pied_actif: bool,
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

    /// Hauteur d'encre d'une ligne composée dans ce style, en mm.
    ///
    /// Elle suit la largeur de couverture, comme le corps : c'est ce qui la rend
    /// comparable à une épaisseur de dos, qui n'en dépend pas du tout.
    pub fn encre_mm(&self, largeur: f64) -> f64 {
        self.corps(largeur) * encre(&self.police)
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
        self.habille(largeur, echappe(texte))
    }

    /// Comme [`Style::applique`], mais le texte est lu comme celui du manuscrit :
    /// `*mot*` passe en italique, `**mot**` en gras.
    ///
    /// C'est la **même** lecture que celle du livre — `manuscrit::inline` —, et c'est
    /// tout son intérêt : une marque veut dire la même chose des deux côtés. Réservé au
    /// texte de présentation de la 4ème : ailleurs, une astérisque dans un titre ou un
    /// nom d'auteur doit s'imprimer plutôt que d'ouvrir une emphase.
    ///
    /// Le piège en est un vrai : une famille sans italique embarqué — Prata, Oswald —
    /// compose `#emph` **sans une inclinaison et sans un mot**, relevé sur PDF. Typst
    /// n'avertit pas : la face manque, la famille non.
    pub fn applique_emphase(&self, largeur: f64, texte: &str) -> String {
        self.habille(largeur, crate::manuscrit::inline(texte))
    }

    /// Le texte déjà mis en markup, habillé de sa casse et de son style.
    fn habille(&self, largeur: f64, t: String) -> String {
        let t = match self.casse {
            Casse::Capitales => format!("#upper[{t}]"),
            Casse::Telle => t,
        };
        format!("text({})[{t}]", self.typst_text(largeur))
    }
}

/// Le voile de lisibilité posé sur une boîte entière, ou rien s'il n'y en a pas.
///
/// Extraite pour la même raison que [`photo_quatre`] : deux appelants en ont besoin et
/// un seul compose. L'autre est l'habillage de la manipulation directe, qui compose la
/// 4ème **sans** sa photo pour la laisser bouger dessous — et qui doit néanmoins porter
/// le voile, puisque c'est par-dessus la photo qu'il se pose. Le recopier là-bas, c'est
/// accepter qu'un jour le direct montre une photo nue et l'aperçu une photo voilée.
pub fn bloc_voile(b: Boite, v: Voile, opacite: f64) -> String {
    voile_fond(v, opacite).map_or_else(String::new, |f| {
        format!(
            "#place(top + left, rect(width: {}, height: {}, fill: {f}, stroke: none))\n",
            mm(b.largeur),
            mm(b.hauteur)
        )
    })
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

fn bloc_pied(livre: &Livre, p: &Pied, cv: &Couverture, (fw, fh): (f64, f64)) -> String {
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
        p.style_mono.applique(fw, &livre.monogramme),
        mm(0.06 * fw),
        p.style_editeur.applique(fw, &livre.editeur),
    )
}

/// Ce dont un élément dispose pour déborder, bord par bord : la bande de fond perdu que
/// le massicot emportera.
///
/// Elle se déduit de la boîte, jamais du prestataire — et c'est ce qui la rend juste
/// partout. Nulle du côté du dos, où la face voisine commence et où rien ne serait
/// coupé ; nulle sur les quatre bords d'un aperçu par face, qui montre le livre déjà
/// rogné.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Debords {
    haut: f64,
    bas: f64,
    gauche: f64,
    droite: f64,
}

impl Debords {
    fn de(b: Boite, (fw, fh): (f64, f64)) -> Self {
        Self {
            haut: b.y0,
            bas: b.hauteur - b.y0 - fh,
            gauche: b.x0,
            droite: b.largeur - b.x0 - fw,
        }
    }
}

fn bloc_pastille(p: &Pastille, collection: &str, fw: f64, d: Debords) -> String {
    if !p.actif || collection.trim().is_empty() {
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
    // Un réglage à zéro veut dire « au bord », et le bord du livre fini est une ligne de
    // coupe, pas une limite : le massicot y travaille à un ou deux millimètres près. Le
    // fond de la pastille descend donc dans le fond perdu, que la coupe emportera. Sans
    // cela le tirage rendrait tantôt une pastille amputée, tantôt un liseré de
    // couverture entre elle et le bord, et cela varierait d'un exemplaire à l'autre.
    //
    // Un décalage non nul, lui, est une distance voulue : rien ne déborde.
    let (vy, vx) = (
        if p.dy.abs() < f64::EPSILON {
            if sy < 0.0 {
                d.bas
            } else {
                d.haut
            }
        } else {
            0.0
        },
        if p.dx.abs() < f64::EPSILON {
            if sx < 0.0 {
                d.droite
            } else {
                d.gauche
            }
        } else {
            0.0
        },
    );
    // Ce que la pastille a à gagner sur chaque bord de la **page**.
    let (ph, pb) = if sy > 0.0 { (vy, 0.0) } else { (0.0, vy) };
    let (pg, pd) = if sx > 0.0 { (vx, 0.0) } else { (0.0, vx) };
    // Le badge est composé à plat, puis tourné d'un quart de tour anti-horaire quand la
    // pastille est verticale : son côté gauche devient alors le bas de la page, et son
    // bas la droite. C'est dans ce repère-là qu'il faut allonger, d'où la permutation.
    let (ix, iy) = (0.028 * fw, 0.012 * fw);
    let (haut, bas, gauche, droite) = if p.verticale {
        (iy + pg, iy + pd, ix + pb, ix + ph)
    } else {
        (iy + ph, iy + pb, ix + pg, ix + pd)
    };
    let badge = format!(
        "box(fill: {}, inset: (top: {}, bottom: {}, left: {}, right: {}), radius: {}, text({})[{}])",
        couleur(&p.fond),
        mm(haut),
        mm(bas),
        mm(gauche),
        mm(droite),
        mm(if p.arrondie { 0.02 * fw } else { 0.0 }),
        style.typst_text(fw),
        echappe(collection),
    );
    // Quart de tour anti-horaire : la pastille se lit de bas en haut, calée dans son
    // coin. `reflow: true` donne à la boîte ses dimensions tournées, sans quoi le
    // placement dans le coin porterait sur la boîte d'avant rotation.
    let contenu = if p.verticale {
        format!("rotate(-90deg, reflow: true, {badge})")
    } else {
        badge
    };
    // Le placement suit le débord d'autant que l'encart s'est allongé : la boîte grandit
    // vers l'extérieur, la voici repoussée d'autant, et le texte ne bouge pas d'un
    // dixième. C'est ce qui fait que l'aperçu rogné dit vrai sur le livre en main.
    format!(
        "#place({coin}, dx: {}, dy: {}, {contenu})\n",
        mm(sx * p.dx / 100.0 * fw - sx * vx),
        mm(sy * p.dy / 100.0 * fw - sy * vy),
    )
}

/// Préambule d'une page d'une seule face. La planche a le sien.
/// Publique pour l'habillage de la manipulation directe : il glisse le voile entre le
/// préambule et le corps, et ne peut donc pas prendre la source d'une face toute faite.
pub fn preambule(largeur: f64, hauteur: f64) -> String {
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
    cadre.push_str(&bloc_pied(livre, &cv.pied, cv, format));
    cadre.push_str(&bloc_pastille(
        &cv.pastille,
        &livre.collection,
        fw,
        Debords::de(b, format),
    ));
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

/// La 1ère, sur une page insérée dans un autre document.
///
/// Même corps que [`source_une`], mais les réglages de texte et de paragraphe sont
/// portés par le **bloc de la page** au lieu du document : les `#set` de [`preambule`]
/// valent jusqu'à la fin de la source, et l'intérieur qui suivrait perdrait son
/// interligne et sa justification sur des centaines de pages.
///
/// La boîte est rognée, sans fond perdu : un ebook ne se coupe pas.
pub fn page_une(
    livre: &Livre,
    cv: &Couverture,
    format: (f64, f64),
    image: Option<&Ressource>,
    dos_mm: Option<f64>,
) -> String {
    let b = Boite::rognee(format);
    let pano = panorama_face(format, dos_mm, true);
    format!(
        "#page(width: {}, height: {}, margin: 0mm, footer: none)[\n  \
         #set text(lang: \"fr\", top-edge: 0.75em, bottom-edge: -0.25em)\n  \
         #set par(leading: 0em, spacing: 0em, justify: false)\n{}]\n",
        mm(b.largeur),
        mm(b.hauteur),
        corps_une(livre, cv, format, image, b, pano)
    )
}

/// Le papier de la 4ème : le sien quand elle en déclare un, celui de la 1ère sinon.
///
/// Une image ou un prolongement se composent **sur** ce papier, et non à sa place : il
/// reste ce qui se voit là où l'image ne porte pas.
pub fn papier_quatre(cv: &Couverture) -> &str {
    match cv.quatrieme.fond {
        FondQuatre::Couleur => &cv.quatrieme.couleur,
        _ => &cv.papier,
    }
}

/// La photo de la 4ème : où elle se compose, comment elle s'y place, et laquelle.
///
/// Extraite de [`corps_quatre`] parce que deux appelants en ont besoin et qu'un seul
/// compose : l'autre est l'aperçu manipulable, qui doit poser la photo sur la même zone
/// que la composition pour que le geste dise vrai. Recopier ce `match` là-bas, c'est
/// accepter qu'un jour la souris cadre une zone et Typst une autre.
///
/// `Ok(None)` couvre les fonds qui n'ont pas de photo — le papier hérité, la couleur
/// distincte — et l'image annoncée mais absente ou illisible : rien à composer n'est pas
/// une erreur. Le prolongement, lui, refuse : il a été demandé, et le composer de
/// travers ne se verrait qu'au pli.
#[allow(clippy::type_complexity)]
pub fn photo_quatre<'a>(
    cv: &Couverture,
    format: (f64, f64),
    image_quatre: Option<&'a Ressource>,
    photo_une: Option<&'a Ressource>,
    pano: Option<Panorama>,
    b: Boite,
) -> Result<Option<((f64, f64, f64, f64), Geometrie, &'a Ressource)>, String> {
    match cv.quatrieme.fond {
        FondQuatre::Image => {
            let Some(r) = image_quatre else {
                return Ok(None);
            };
            let zone = (0.0, 0.0, b.largeur, b.hauteur);
            let g = image::place(
                (zone.2, zone.3),
                (r.largeur, r.hauteur),
                &cv.quatrieme.cadrage,
            );
            Ok(g.map(|g| (zone, g, r)))
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
            Ok(Some((zone, g, r)))
        }
        _ => Ok(None),
    }
}

/// La tête de la 4ème : auteur, titre, filet, dans cet ordre et chacun s'il est allumé.
///
/// Rend une chaîne vide quand les trois sont éteints — c'est ce qui laisse la 4ème
/// d'une maquette d'avant composer exactement comme avant.
///
/// L'auteur et le titre viennent du livre. Aucune substitution de jetons : ce sont les
/// clés elles-mêmes, comme sur la 1ère, et un `%TITRE%` n'aurait rien à y résoudre.
fn bloc_tete_quatre(livre: &Livre, t: &TeteQuatre, fw: f64) -> String {
    let mut s = String::new();
    let ecart = |v: f64| format!("#v({})\n", mm(v / 100.0 * fw));
    if t.auteur_visible {
        s.push_str(&format!("#{}\n", t.auteur.applique(fw, &livre.auteur)));
    }
    if t.titre_visible {
        if !s.is_empty() {
            s.push_str(&ecart(t.titre_ecart));
        }
        s.push_str(&format!("#{}\n", t.titre.applique(fw, &livre.titre)));
    }
    if t.filet_visible {
        if !s.is_empty() {
            s.push_str(&ecart(t.filet_ecart));
        }
        // Le filet est centré dans le bloc quel que soit l'alignement de la tête : une
        // ligne de 12 % de large collée au fer d'un titre centré se lirait comme un
        // défaut. C'est le seul élément de la tête qui ne suive pas l'alignement.
        s.push_str(&format!(
            "#align(center, line(length: {}, stroke: {} + {}))\n",
            mm(t.filet.largeur / 100.0 * fw),
            mm(t.filet.epaisseur / 100.0 * fw),
            couleur(&t.filet.couleur),
        ));
    }
    if s.is_empty() {
        return s;
    }
    // L'alignement et l'interligne de la tête sont posés une fois, en ouverture du
    // bloc : le texte qui suit remet les siens.
    format!(
        "#set align({})\n#set par(leading: 0.2em, spacing: 0em)\n{s}",
        t.align.typst()
    )
}

/// Corps de la 4ème de couverture, dans la boîte donnée.
///
/// `pano` n'est requis que pour le prolongement : il porte la largeur de la planche,
/// donc celle du dos, donc la pagination. C'est le couplage que l'application existe
/// pour tenir — ici, il est explicite au lieu d'être recopié à la main, et la 4ème
/// refuse de se composer sans lui plutôt que de se composer de travers.
pub fn corps_quatre(
    livre: &Livre,
    cv: &Couverture,
    format: (f64, f64),
    image_quatre: Option<&Ressource>,
    photo_une: Option<&Ressource>,
    pano: Option<Panorama>,
    b: Boite,
) -> Result<String, String> {
    let (fw, _) = format;
    let q = &cv.quatrieme;
    let mut s = bloc_fond(b, papier_quatre(cv));

    // Le voile suit la photo réellement composée, et non le mode : un fond réglé sur
    // l'image se règle avant que la photo n'arrive, et le reste quand on la retire. Sur
    // le mode, le voile devenait alors un rectangle sombre sur du papier nu.
    if let Some((zone, g, r)) = photo_quatre(cv, format, image_quatre, photo_une, pano, b)? {
        s.push_str(&bloc_image(zone, &g, &r.fichier));
        s.push_str(&bloc_voile(b, q.voile, q.voile_opacite));
    }

    let mut c = String::new();
    let pad = q.pad_x / 100.0 * fw;
    // Le seul texte que la maquette porte encore, et le seul endroit où la substitution
    // la sert : une 4ème générique se résout pour chaque livre où on la charge.
    let resume = crate::gabarit::substituer(&q.texte, livre);
    let tete = bloc_tete_quatre(livre, &q.tete, fw);
    // Un seul bloc pour la tête et le texte : ils se suivent sur la page, et deux
    // placements séparés auraient demandé deux hauteurs à tenir d'accord à la main.
    // C'est aussi ce qui fait que `top` reste le seul point d'ancrage — celui que la
    // prise de l'aperçu déplace.
    if !tete.is_empty() || !resume.trim().is_empty() {
        let mut corps = tete;
        if !resume.trim().is_empty() {
            if !corps.is_empty() {
                corps.push_str(&format!("#v({})\n", mm(q.tete.ecart / 100.0 * fw)));
            }
            corps.push_str(&format!(
                "#set align({})\n\
                 #set par(leading: {}em, spacing: {}em + {}, justify: false)\n#{}\n",
                q.align.typst(),
                q.interligne - 1.0,
                q.interligne - 1.0,
                mm(q.paragraphe_ecart / 100.0 * fw),
                q.style.applique_emphase(fw, &resume),
            ));
        }
        c.push_str(&format!(
            "#place(top + left, dx: {}, dy: {}, block(width: {})[\n{corps}])\n",
            mm(pad),
            mm(q.top / 100.0 * fw),
            mm(fw - 2.0 * pad),
        ));
    }

    if q.pied_actif {
        // La collection est une clé, littérale ; la mention et le prix sont des champs
        // libres, donc substitués.
        let lignes: Vec<String> = [livre.mention(), livre.collection.clone(), livre.prix()]
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
    livre: &Livre,
    cv: &Couverture,
    format: (f64, f64),
    image_quatre: Option<&Ressource>,
    image_une: Option<&Ressource>,
    dos_mm: Option<f64>,
) -> Result<String, String> {
    let b = Boite::rognee(format);
    let pano = panorama_face(format, dos_mm, false);
    let corps = corps_quatre(livre, cv, format, image_quatre, image_une, pano, b)?;
    Ok(preambule(b.largeur, b.hauteur) + &corps)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::maquettes;

    fn pastille_au_bord(coin: Coin, verticale: bool) -> Pastille {
        Pastille {
            actif: true,
            style: dos_style(),
            fond: "#111111".into(),
            coin,
            verticale,
            arrondie: true,
            dx: 0.0,
            dy: 0.0,
        }
    }

    /// Le bord du livre fini est une ligne de coupe, pas une limite : le massicot y
    /// travaille à un ou deux millimètres près. Une pastille calée dessus sortirait du
    /// tirage tantôt amputée, tantôt bordée d'un liseré de couverture — et cela
    /// varierait d'un exemplaire à l'autre. Son fond descend donc dans le fond perdu.
    #[test]
    fn une_pastille_au_bord_deborde_dans_le_fond_perdu() {
        let d = Debords {
            haut: 5.0,
            bas: 5.0,
            gauche: 0.0,
            droite: 5.0,
        };
        let s = bloc_pastille(
            &pastille_au_bord(Coin::BasDroite, false),
            "bandeau",
            135.0,
            d,
        );
        assert!(s.contains("bottom: 6.6200mm"), "{s}"); // 0.012 × 135 + 5
        assert!(s.contains("right: 8.7800mm"), "{s}"); // 0.028 × 135 + 5
                                                       // Le fond s'allonge et le placement suit d'autant : le texte, lui, ne bouge pas.
        assert!(s.contains("dx: 5.0000mm"), "{s}");
        assert!(s.contains("dy: 5.0000mm"), "{s}");
    }

    /// Le côté du dos n'a pas de fond perdu : la 1ère y touche la 4ème, pas le vide.
    /// Une pastille qui y déborderait s'imprimerait sur le dos, et rien ne la couperait.
    #[test]
    fn une_pastille_ne_deborde_pas_du_cote_du_dos() {
        let d = Debords {
            haut: 5.0,
            bas: 5.0,
            gauche: 0.0,
            droite: 5.0,
        };
        let s = bloc_pastille(
            &pastille_au_bord(Coin::BasGauche, false),
            "bandeau",
            135.0,
            d,
        );
        assert!(s.contains("bottom: 6.6200mm"), "{s}");
        assert!(s.contains("left: 3.7800mm"), "{s}"); // 0.028 × 135, sans débord
        assert!(s.contains("dx: 0.0000mm"), "{s}");
    }

    /// L'aperçu par face montre la couverture **rognée**, c'est-à-dire le livre tel
    /// qu'il sort du massicot : aucun débord à y composer, sans quoi il montrerait une
    /// pastille plus grande que celle qu'on aura en main.
    #[test]
    fn sans_fond_perdu_la_pastille_ne_deborde_pas() {
        let d = Debords {
            haut: 0.0,
            bas: 0.0,
            gauche: 0.0,
            droite: 0.0,
        };
        let s = bloc_pastille(
            &pastille_au_bord(Coin::BasDroite, false),
            "bandeau",
            135.0,
            d,
        );
        assert!(
            s.contains("inset: (top: 1.6200mm, bottom: 1.6200mm, left: 3.7800mm, right: 3.7800mm)"),
            "{s}"
        );
        assert!(s.contains("dx: 0.0000mm"), "{s}");
        assert!(s.contains("dy: 0.0000mm"), "{s}");
    }

    /// Un décalage non nul est une distance voulue au bord : la pastille est alors dans
    /// la page, et la faire déborder la déplacerait sans que personne l'ait demandé.
    ///
    /// Les deux axes décident chacun pour soi : une pastille contre le bord droit mais
    /// remontée de 3,5 % déborde à droite et pas en bas. Sans quoi il faudrait renoncer
    /// au débord dès qu'on écarte la pastille d'un seul côté.
    #[test]
    fn chaque_axe_decide_seul_de_son_debord() {
        let d = Debords {
            haut: 5.0,
            bas: 5.0,
            gauche: 0.0,
            droite: 5.0,
        };
        let mut p = pastille_au_bord(Coin::BasDroite, false);
        p.dy = 3.5;
        let s = bloc_pastille(&p, "bandeau", 135.0, d);
        assert!(s.contains("bottom: 1.6200mm"), "{s}");
        assert!(s.contains("right: 8.7800mm"), "{s}");
        assert!(s.contains("dx: 5.0000mm"), "{s}");
        // 3,5 % de 135 mm vers le haut, et rien de plus.
        assert!(s.contains("dy: -4.7250mm"), "{s}");
    }

    /// Une famille admise sans encre déclarée tomberait sur la valeur de repli, la plus
    /// haute de la table : le dos d'une maquette composée dans cette famille-là serait
    /// jugé trop juste, sans que rien ne dise pourquoi. La table suit `POLICES`.
    #[test]
    fn toute_police_declare_son_encre() {
        for f in POLICES {
            assert!(
                ENCRE.iter().any(|(nom, _)| nom == f),
                "{f} est admise mais n'a pas d'encre relevée"
            );
        }
    }

    fn livre() -> Livre {
        Livre {
            titre: "Les Heures creuses".into(),
            titre_page: crate::projet::titre_page_defaut(),
            auteur: "Ivan Pjig".into(),
            genre: "roman".into(),
            editeur: "Editeur".into(),
            collection: "Collection".into(),
            monogramme: "Monogramme".into(),
            copyright: String::new(),
            prix: "Prix".into(),
            mention: "Mention".into(),
            dedicace: String::new(),
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

    /// La 1ère, posée dans un autre document, ne doit pas emporter ses `#set` : ceux de
    /// `source_une` valent pour le document entier — `par(leading: 0em, justify: false)`,
    /// notamment — et écraseraient ceux de l'intérieur pour toutes les pages qui suivent.
    /// Le livre sortirait sans interligne et au fer à gauche, plusieurs centaines de pages
    /// durant.
    #[test]
    fn la_couverture_inseree_ne_pose_aucun_reglage_de_document() {
        let p = page_une(&livre(), &maquettes::fournie("bandeau"), FORMAT, None, None);
        // Tout est enveloppé dans un seul bloc de page : un `#set` posé dedans ne vaut
        // que pour elle, quelle que soit la colonne où il tombe. Ce qui vaudrait pour
        // le document, c'est ce que `preambule` écrit *avant* la page — `#set page`, et
        // les deux réglages de tête.
        assert!(p.starts_with("#page("), "{p}");
        assert!(
            p.trim_end().ends_with(']'),
            "bloc de page non refermé : {p}"
        );
        assert!(!p.contains("#set page("), "{p}");
        assert!(p.contains("margin: 0mm"), "{p}");
    }

    /// Les deux formes de la 1ère ne diffèrent que par la portée de leurs réglages, et
    /// c'est toute leur raison d'être. `source_une` est une source **autonome** : elle
    /// ouvre par un `#set page` de document, ce qu'il faut pour l'aperçu par face.
    /// `page_une` est **insérable** : rien chez elle n'atteint le document qui l'accueille.
    ///
    /// Ce test tient les deux bouts. Rendre `source_une` scopée casserait l'aperçu ;
    /// écrire `page_une` comme `preambule(…) + corps_une(…)` ferait perdre son interligne
    /// et sa justification à l'intérieur qui suit, sur des centaines de pages et sans
    /// qu'aucune erreur ne soit levée.
    #[test]
    fn seule_la_forme_autonome_de_la_1ere_regle_le_document() {
        let (l, cv) = (livre(), maquettes::fournie("bandeau"));
        assert!(source_une(&l, &cv, FORMAT, None, None).contains("#set page("));
        assert!(!page_une(&l, &cv, FORMAT, None, None).contains("#set page("));
    }

    /// Une maquette est portable d'un format à l'autre : c'est la promesse des
    /// réglages en pourcentage. Doubler la largeur doit doubler le corps du titre,
    /// jamais laisser une valeur figée derrière.
    #[test]
    fn une_maquette_suit_le_format_sans_valeur_figee() {
        let cv = maquettes::fournie("bandeau");
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
        let mut cv = maquettes::fournie("filets");
        cv.voile = Voile::Uni;
        cv.voile_opacite = 0.5;
        let s = source_une(&livre(), &cv, FORMAT, Some(&photo()), None);
        assert!(!s.contains("image("), "image émise en mode typo");
        assert!(!s.contains("gradient"), "voile émis en mode typo");
    }

    /// Une 4ème qui réclame sa propre image sans en avoir compose son papier, et rien
    /// d'autre : pas de voile.
    ///
    /// Même raison qu'en composition typographique — un voile sans photo dessous n'est
    /// qu'un rectangle sombre sur du papier — mais l'état s'atteint autrement : le fond
    /// se règle avant que la photo n'arrive, et il reste réglé quand on la retire.
    /// Conditionner le voile au **mode** le posait alors sur une 4ème qui n'a rien à
    /// assombrir.
    #[test]
    fn une_quatrieme_sans_photo_ne_compose_pas_son_voile() {
        let mut cv = maquettes::fournie("bandeau");
        cv.quatrieme.fond = FondQuatre::Image;
        cv.quatrieme.voile = Voile::Uni;
        cv.quatrieme.voile_opacite = 0.5;

        let sans = source_quatre(&livre(), &cv, FORMAT, None, None, None).unwrap();
        assert!(!sans.contains("image("), "image émise sans photo");
        assert!(
            !sans.contains("transparentize"),
            "voile émis sans photo :\n{sans}"
        );

        // La photo revenue, le voile revient avec elle : c'est bien la photo qu'il suit.
        let avec = source_quatre(&livre(), &cv, FORMAT, Some(&photo()), None, None).unwrap();
        assert!(
            avec.contains("transparentize"),
            "voile perdu avec la photo :\n{avec}"
        );
    }

    /// Le cadre est le dessin le plus scruté de la maquette : trois filets, dans
    /// l'ordre, avec les bonnes couleurs.
    #[test]
    fn le_cadre_emet_trois_filets_dans_l_ordre() {
        let cv = maquettes::fournie("filets");
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
        let cv = maquettes::fournie("bandeau");
        assert!(!cv.cadre.actif);
        assert!(!source_une(&livre(), &cv, FORMAT, None, None).contains("stroke:"));
    }

    /// L'identité du livre vient du projet : la maquette ne doit pas pouvoir la
    /// contredire. Changer de maquette ne change jamais le titre imprimé.
    #[test]
    fn le_titre_et_l_auteur_viennent_du_projet() {
        for cv in [
            maquettes::fournie("bandeau"),
            maquettes::fournie("filets"),
            maquettes::fournie("surimpression"),
        ] {
            let s = source_une(&livre(), &cv, FORMAT, None, None);
            assert!(s.contains("Les Heures creuses"), "{:?}", cv.mode);
            assert!(s.contains("Ivan Pjig"), "{:?}", cv.mode);
        }
    }

    /// Le bandeau réserve le haut de la couverture : l'image commence dessous.
    #[test]
    fn le_bandeau_pousse_l_image_sous_la_bande() {
        let cv = maquettes::fournie("bandeau");
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
        let cv = maquettes::fournie("surimpression");
        let s = source_une(&livre(), &cv, FORMAT, Some(&photo()), None);
        assert!(s.contains(&format!("width: {}", mm(FORMAT.0))));
        assert!(s.contains("gradient.linear"), "voile attendu");
    }

    /// Le prolongement panoramique dépend du dos, donc de la pagination. Le composer
    /// sans elle produirait une 4ème décalée — et personne ne le verrait avant tirage.
    #[test]
    fn le_prolongement_refuse_de_composer_sans_le_dos() {
        let mut cv = maquettes::fournie("bandeau");
        cv.quatrieme.fond = FondQuatre::Panorama;
        let err = source_quatre(&livre(), &cv, FORMAT, None, Some(&photo()), None).unwrap_err();
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
        let mut cv = maquettes::fournie("bandeau");
        cv.quatrieme.fond = FondQuatre::Panorama;
        let largeur_zone = |dos: f64| {
            let s = source_quatre(&livre(), &cv, FORMAT, None, Some(&photo()), Some(dos)).unwrap();
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
        let mut cv = maquettes::fournie("bandeau");
        cv.quatrieme.isbn_actif = true;
        let s = source_quatre(&livre(), &cv, FORMAT, None, None, None).unwrap();
        assert!(s.contains("fill: rgb(\"#ffffff\")"));
        assert!(!s.contains("isbn"), "aucun contenu dans la zone");
    }

    /// Les trois maquettes doivent rester composables : c'est ce que promet le bouton
    /// qui les charge.
    #[test]
    fn les_trois_maquettes_composent_les_deux_faces() {
        for cv in [
            maquettes::fournie("bandeau"),
            maquettes::fournie("filets"),
            maquettes::fournie("surimpression"),
        ] {
            assert!(!source_une(&livre(), &cv, FORMAT, Some(&photo()), None).is_empty());
            source_quatre(&livre(), &cv, FORMAT, None, Some(&photo()), Some(15.0)).unwrap();
        }
    }

    /// Toutes les polices des maquettes doivent être embarquées, sans quoi Typst
    /// substituerait en silence et le rendu changerait d'une machine à l'autre.
    #[test]
    fn les_maquettes_n_utilisent_que_des_polices_embarquees() {
        for cv in [
            maquettes::fournie("bandeau"),
            maquettes::fournie("filets"),
            maquettes::fournie("surimpression"),
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
        l.collection = "col#lection".into();
        let cv = maquettes::fournie("bandeau");
        let s = source_une(&l, &cv, FORMAT, None, None);
        assert!(s.contains(r"Le \#Titre"));
        assert!(s.contains(r"col\#lection"));
    }

    /// **Point de sortie : la couverture.** Le pied de 1ère, la pastille et le pied de
    /// 4ème composent désormais des textes du livre. Aucun jeton ne doit y survivre, et
    /// la maquette n'a plus rien à dire de ce qui est écrit.
    #[test]
    fn la_couverture_compose_les_textes_du_livre() {
        let mut l = livre();
        l.editeur = "Ozalid".into();
        l.monogramme = "O".into();
        l.collection = "Les Heures".into();
        l.prix = "18 € — %COLLECTION%".into();
        l.mention = "%EDITEUR%".into();

        let mut cv = maquettes::fournie("filets");
        cv.pied.actif = true;
        cv.pastille.actif = true;
        cv.quatrieme.pied_actif = true;

        let une = page_une(&l, &cv, FORMAT, None, None);
        assert!(
            une.contains("Ozalid"),
            "l'éditeur du livre n'est pas au pied"
        );
        assert!(
            une.contains("Les Heures"),
            "la collection n'est pas en pastille"
        );

        let quatre = source_quatre(&l, &cv, FORMAT, None, None, None).unwrap();
        assert!(
            quatre.contains("18 € — Les Heures"),
            "le prix n'est pas substitué"
        );
        assert!(quatre.contains("Ozalid"), "la mention n'est pas substituée");
        for jeton in ["%EDITEUR%", "%COLLECTION%", "%TITRE%"] {
            assert!(!une.contains(jeton), "{jeton} a traversé la 1ère");
            assert!(!quatre.contains(jeton), "{jeton} a traversé la 4ème");
        }
    }

    /// Le texte de présentation se lit comme le manuscrit : `*mot*` en italique,
    /// `**mot**` en gras. C'est la **même** lecture — `manuscrit::morceaux` —, et c'est
    /// tout l'intérêt : une marque veut dire la même chose sur la couverture et dans le
    /// livre, sans qu'on ait à se rappeler laquelle des deux on est en train d'écrire.
    ///
    /// Le reste de la maquette n'en veut pas : un titre ou un nom d'auteur portant une
    /// astérisque doit l'imprimer, pas ouvrir une emphase.
    #[test]
    fn le_texte_de_presentation_lit_l_emphase_du_manuscrit() {
        let mut cv = maquettes::fournie("filets");
        cv.quatrieme.texte = "Un *mot* et un **autre**.".into();
        let s = source_quatre(&livre(), &cv, FORMAT, None, None, None).unwrap();
        assert!(s.contains("#emph[mot]"), "{s}");
        assert!(s.contains("#strong[autre]"), "{s}");

        // Le titre, lui, garde ses astérisques : la tête n'est pas du markup.
        let mut l = livre();
        l.titre = "Un *titre*".into();
        let mut cv = maquettes::fournie("filets");
        cv.quatrieme.tete.titre_visible = true;
        let s = source_quatre(&l, &cv, FORMAT, None, None, None).unwrap();
        assert!(s.contains(r"\*titre\*"), "{s}");
        assert!(!s.contains("#emph["), "{s}");
    }

    /// Le texte de présentation s'aère par un écart entre paragraphes, distinct de
    /// l'interligne : celle-ci sépare les lignes d'un même passage, celui-là les
    /// passages entre eux. Sans lui, une 4ème n'a ni blanc ni alinéa — deux paragraphes
    /// s'y lisent comme un seul, et c'est le défaut relevé sur la première 4ème composée
    /// avec sa tête.
    ///
    /// À zéro — la valeur que reprend toute maquette écrite avant ce réglage — la
    /// composition ne bouge pas d'un point.
    #[test]
    fn l_ecart_entre_paragraphes_de_la_quatrieme_s_ajoute_a_l_interligne() {
        let compose = |ecart| {
            let mut cv = maquettes::fournie("filets");
            cv.quatrieme.texte = "Premier passage.\n\nSecond passage.".into();
            cv.quatrieme.interligne = 1.45;
            cv.quatrieme.paragraphe_ecart = ecart;
            source_quatre(&livre(), &cv, FORMAT, None, None, None).unwrap()
        };

        // 5 % de 110 mm : l'écart s'ajoute à l'espacement ordinaire, il ne le remplace
        // pas — un espacement plus petit que l'interligne resserrerait les passages.
        let large = compose(5.0);
        assert!(
            large.contains("em + 5.5000mm"),
            "l'écart n'est pas composé : {large}"
        );

        let nul = compose(0.0);
        assert!(
            nul.contains("em + 0.0000mm"),
            "l'écart nul doit laisser l'espacement ordinaire : {nul}"
        );
    }

    /// **Non-régression de la tête.** Une maquette écrite avant elle compose sa 4ème
    /// comme avant : ni auteur, ni titre, ni filet. Les trois naissent éteints, sans
    /// quoi tout projet existant verrait son identité paraître sur sa 4ème sans que
    /// personne l'ait demandé — et une couverture qui change toute seule se découvre
    /// au tirage.
    #[test]
    fn une_tete_de_quatrieme_eteinte_ne_compose_rien() {
        let cv = maquettes::fournie("filets");
        let s = source_quatre(&livre(), &cv, FORMAT, None, None, None).unwrap();
        assert!(
            !s.contains("Ivan Pjig"),
            "auteur composé sans être allumé : {s}"
        );
        assert!(
            !s.contains("Les Heures creuses"),
            "titre composé sans être allumé : {s}"
        );
        assert!(
            !s.contains("line(length:"),
            "filet composé sans être allumé : {s}"
        );
    }

    /// Chacun s'allume seul : une collection met l'auteur et le filet sans le titre, une
    /// autre le titre seul. Rien ici ne doit privilégier une mise en page.
    #[test]
    fn chaque_element_de_la_tete_s_allume_seul() {
        let compose = |auteur, titre, filet| {
            let mut cv = maquettes::fournie("filets");
            cv.quatrieme.tete.auteur_visible = auteur;
            cv.quatrieme.tete.titre_visible = titre;
            cv.quatrieme.tete.filet_visible = filet;
            source_quatre(&livre(), &cv, FORMAT, None, None, None).unwrap()
        };

        let a = compose(true, false, false);
        assert!(a.contains("Ivan Pjig"), "{a}");
        assert!(!a.contains("Les Heures creuses"), "{a}");
        assert!(!a.contains("line(length:"), "{a}");

        let t = compose(false, true, false);
        assert!(!t.contains("Ivan Pjig"), "{t}");
        assert!(t.contains("Les Heures creuses"), "{t}");

        let f = compose(false, false, true);
        assert!(!f.contains("Ivan Pjig"), "{f}");
        assert!(f.contains("line(length:"), "{f}");
    }

    /// L'auteur et le titre de la tête viennent du **livre**, jamais de la maquette :
    /// charger une maquette ne change pas ce qui s'imprime comme identité. C'est la même
    /// règle que sur la 1ère, et c'est celle qui fait tenir tout le reste.
    #[test]
    fn la_tete_de_quatrieme_prend_l_identite_du_livre() {
        let mut l = livre();
        l.auteur = "Ivan Pjig".into();
        l.titre = "Les Heures creuses".into();
        let mut cv = maquettes::fournie("filets");
        cv.quatrieme.tete.auteur_visible = true;
        cv.quatrieme.tete.titre_visible = true;
        cv.quatrieme.tete.auteur.couleur = "#c00000".into();

        let s = source_quatre(&l, &cv, FORMAT, None, None, None).unwrap();
        assert!(s.contains("Ivan Pjig"), "{s}");
        assert!(s.contains("Les Heures creuses"), "{s}");
        // La couleur demandée est bien celle qui compose l'auteur.
        assert!(s.contains("#c00000"), "{s}");
    }

    /// L'ordre est celui de la page : auteur, titre, filet, puis le texte. Il est tenu
    /// par la composition et non par l'ordre où les réglages sont écrits.
    #[test]
    fn la_tete_se_compose_avant_le_texte() {
        let mut cv = maquettes::fournie("filets");
        cv.quatrieme.tete.auteur_visible = true;
        cv.quatrieme.tete.titre_visible = true;
        cv.quatrieme.tete.filet_visible = true;
        cv.quatrieme.texte = "Le texte de présentation.".into();

        let s = source_quatre(&livre(), &cv, FORMAT, None, None, None).unwrap();
        let ou = |quoi: &str| {
            s.find(quoi)
                .unwrap_or_else(|| panic!("{quoi} absent : {s}"))
        };
        assert!(ou("Ivan Pjig") < ou("Les Heures creuses"), "{s}");
        assert!(ou("Les Heures creuses") < ou("line(length:"), "{s}");
        assert!(ou("line(length:") < ou("Le texte de présentation."), "{s}");
    }

    /// Le texte de présentation n'est plus la condition d'existence du bloc : une 4ème
    /// qui ne porte qu'un titre et un filet se compose. Sans cela, une couverture réglée
    /// sur sa seule tête resterait vide sans rien dire.
    #[test]
    fn une_tete_sans_texte_se_compose_quand_meme() {
        let mut cv = maquettes::fournie("filets");
        cv.quatrieme.texte = String::new();
        cv.quatrieme.tete.titre_visible = true;

        let s = source_quatre(&livre(), &cv, FORMAT, None, None, None).unwrap();
        assert!(s.contains("Les Heures creuses"), "{s}");
    }

    /// Le résumé de 4ème est le seul texte que la maquette porte encore, et c'est le
    /// seul endroit où la substitution la sert : une maquette peut ainsi porter une
    /// 4ème générique qui se résout pour chaque livre où on la charge.
    #[test]
    fn le_resume_de_quatrieme_cite_les_cles() {
        let mut l = livre();
        l.genre = "roman".into();
        let mut cv = maquettes::fournie("filets");
        cv.quatrieme.texte = "%TITRE%, un %GENRE% de %AUTEUR%.".into();

        let quatre = source_quatre(&l, &cv, FORMAT, None, None, None).unwrap();
        assert!(
            quatre.contains("Les Heures creuses, un roman de Ivan Pjig."),
            "{quatre}"
        );
        assert!(!quatre.contains('%'), "un jeton a traversé le résumé");
    }
}
