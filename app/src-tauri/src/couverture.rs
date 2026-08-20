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
use crate::manuscrit::echappe;
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

    fn applique(&self, largeur: f64, texte: &str) -> String {
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

/// Zone occupée par l'image de la 1ère, en mm : (x, y, largeur, hauteur).
/// `None` en composition purement typographique.
fn zone_image(cv: &Couverture, (fw, fh): (f64, f64)) -> Option<(f64, f64, f64, f64)> {
    match cv.mode {
        Mode::Typo => None,
        Mode::Surimpression => Some((0.0, 0.0, fw, fh)),
        Mode::Bandeau => {
            // Le retrait est le même pourcentage lu sur la largeur horizontalement et
            // sur la hauteur verticalement : c'est ainsi que le CSS le résolvait.
            let (rx, ry) = if cv.bandeau_retrait {
                (cv.pad_x / 100.0 * fw, cv.pad_x / 100.0 * fh)
            } else {
                (0.0, 0.0)
            };
            let haut = cv.bandeau / 100.0 * fh;
            Some((rx, haut, fw - 2.0 * rx, fh - haut - ry))
        }
    }
}

/// Image posée dans une zone, découpée à ses bords.
fn bloc_image(zone: (f64, f64, f64, f64), g: &Geometrie, fichier: &str) -> String {
    let (x, y, w, h) = zone;
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

/// Préambule commun aux deux faces.
fn preambule((fw, fh): (f64, f64), fond: &str) -> String {
    format!(
        "#set page(width: {}, height: {}, margin: 0mm, fill: {})\n\
         // Boîte de ligne ramenée à 1em : « leading » Typst et « line-height » CSS\n\
         // deviennent alors la même grandeur.\n\
         #set text(lang: \"fr\", top-edge: 0.75em, bottom-edge: -0.25em)\n\
         #set par(leading: 0em, spacing: 0em, justify: false)\n\n",
        mm(fw),
        mm(fh),
        couleur(fond)
    )
}

/// Source Typst de la 1ère de couverture.
pub fn source_une(
    livre: &Livre,
    cv: &Couverture,
    format: (f64, f64),
    image: Option<&Ressource>,
) -> String {
    let (fw, fh) = format;
    let mut s = preambule(format, &cv.papier);

    if let (Some(zone), Some(r)) = (zone_image(cv, format), image) {
        if let Some(g) = image::place((zone.2, zone.3), (r.largeur, r.hauteur), &cv.cadrage) {
            s.push_str(&bloc_image(zone, &g, &r.fichier));
        }
    }
    // Le voile couvre toute la couverture, pas seulement la zone image : c'est ce que
    // faisait le CSS, et le bandeau en dépend visuellement.
    if cv.mode != Mode::Typo {
        if let Some(f) = voile_fond(cv.voile, cv.voile_opacite) {
            s.push_str(&format!(
                "#place(top + left, rect(width: {}, height: {}, fill: {f}, stroke: none))\n",
                mm(fw),
                mm(fh)
            ));
        }
    }
    s.push_str(&bloc_cadre(&cv.cadre, format));
    s.push_str(&bloc_texte(livre, cv, format));
    s.push_str(&bloc_pied(&cv.pied, cv, format));
    s.push_str(&bloc_pastille(&cv.pastille, fw));
    s
}

/// Source Typst de la 4ème de couverture.
///
/// `dos_mm` n'est requis que pour le prolongement panoramique : la 4ème y montre la
/// partie de l'image de la 1ère située au-delà du dos, donc la largeur du dos — donc
/// la pagination — entre dans le calcul. C'est le couplage que l'application existe
/// pour tenir : ici, il est explicite au lieu d'être recopié à la main.
pub fn source_quatre(
    cv: &Couverture,
    format: (f64, f64),
    image_quatre: Option<&Ressource>,
    image_une: Option<&Ressource>,
    dos_mm: Option<f64>,
) -> Result<String, String> {
    let (fw, fh) = format;
    let q = &cv.quatrieme;
    let fond = match q.fond {
        FondQuatre::Couleur => &q.couleur,
        _ => &cv.papier,
    };
    let mut s = preambule(format, fond);

    match q.fond {
        FondQuatre::Image => {
            if let Some(r) = image_quatre {
                if let Some(g) = image::place((fw, fh), (r.largeur, r.hauteur), &q.cadrage) {
                    s.push_str(&bloc_image((0.0, 0.0, fw, fh), &g, &r.fichier));
                }
            }
        }
        FondQuatre::Panorama => {
            let dos = dos_mm.ok_or_else(|| {
                "prolongement panoramique : la largeur du dos est inconnue — composer \
                 l'intérieur d'abord, la pagination la détermine."
                    .to_string()
            })?;
            let r = image_une.ok_or_else(|| {
                "prolongement panoramique : la 1ère n'a pas d'image à prolonger.".to_string()
            })?;
            let zone = zone_image(cv, format).ok_or_else(|| {
                "prolongement panoramique : sans objet en composition typographique.".to_string()
            })?;
            if let Some(g) = image::place((zone.2, zone.3), (r.largeur, r.hauteur), &cv.cadrage) {
                // La 4ème est à gauche de la planche : l'image y est décalée de la
                // largeur d'une couverture plus celle du dos.
                let decale = Geometrie {
                    gauche: g.gauche + fw + dos,
                    ..g
                };
                s.push_str(&bloc_image(zone, &decale, &r.fichier));
            }
        }
        _ => {}
    }

    let avec_image = matches!(q.fond, FondQuatre::Image | FondQuatre::Panorama);
    if avec_image {
        if let Some(f) = voile_fond(q.voile, q.voile_opacite) {
            s.push_str(&format!(
                "#place(top + left, rect(width: {}, height: {}, fill: {f}, stroke: none))\n",
                mm(fw),
                mm(fh)
            ));
        }
    }

    let pad = q.pad_x / 100.0 * fw;
    if !q.texte.trim().is_empty() {
        s.push_str(&format!(
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
            s.push_str(&format!(
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
        s.push_str(&format!(
            "#place(bottom + right, dx: -{}, dy: -{}, \
             rect(width: {}, height: {}, fill: rgb(\"#ffffff\"), stroke: none))\n",
            mm(q.isbn_dx / 100.0 * fw),
            mm(q.isbn_dy / 100.0 * fw),
            mm(q.isbn_l / 100.0 * fw),
            mm(q.isbn_h / 100.0 * fw),
        ));
    }
    Ok(s)
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

    /// Une maquette est portable d'un format à l'autre : c'est la promesse des
    /// réglages en pourcentage. Doubler la largeur doit doubler le corps du titre,
    /// jamais laisser une valeur figée derrière.
    #[test]
    fn une_maquette_suit_le_format_sans_valeur_figee() {
        let cv = maquettes::folio();
        let petit = source_une(&livre(), &cv, (100.0, 160.0), None);
        let grand = source_une(&livre(), &cv, (200.0, 320.0), None);
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
        let s = source_une(&livre(), &cv, FORMAT, Some(&photo()));
        assert!(!s.contains("image("), "image émise en mode typo");
        assert!(!s.contains("gradient"), "voile émis en mode typo");
    }

    /// Le cadre est le dessin le plus scruté de la maquette : trois filets, dans
    /// l'ordre, avec les bonnes couleurs.
    #[test]
    fn le_cadre_emet_trois_filets_dans_l_ordre() {
        let cv = maquettes::blanche();
        assert!(cv.cadre.actif);
        let s = source_une(&livre(), &cv, FORMAT, None);
        let filets: Vec<&str> = s.match_indices("rect(").map(|(_, m)| m).collect();
        assert_eq!(filets.len(), 3, "trois filets attendus");
        // Externe noir, puis les deux internes rouges.
        let pos_noir = s.find("#000000").unwrap();
        let pos_rouge = s.find("#c00000").unwrap();
        assert!(pos_noir < pos_rouge, "filet externe après les internes");
    }

    #[test]
    fn un_cadre_inactif_n_emet_rien() {
        let cv = maquettes::folio();
        assert!(!cv.cadre.actif);
        assert!(!source_une(&livre(), &cv, FORMAT, None).contains("stroke:"));
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
            let s = source_une(&livre(), &cv, FORMAT, None);
            assert!(s.contains("Les Heures creuses"), "{:?}", cv.mode);
            assert!(s.contains("Ivan Pjig"), "{:?}", cv.mode);
        }
    }

    /// Le bandeau réserve le haut de la couverture : l'image commence dessous.
    #[test]
    fn le_bandeau_pousse_l_image_sous_la_bande() {
        let cv = maquettes::folio();
        let s = source_une(&livre(), &cv, FORMAT, Some(&photo()));
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
        let s = source_une(&livre(), &cv, FORMAT, Some(&photo()));
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

    /// Avec le dos, l'image de la 4ème est décalée d'une couverture plus le dos :
    /// c'est ce décalage qui fait que la photo se prolonge sans rupture au pli.
    #[test]
    fn le_prolongement_decale_l_image_d_une_couverture_plus_le_dos() {
        let mut cv = maquettes::folio();
        cv.quatrieme.fond = FondQuatre::Panorama;
        let sans = source_quatre(&cv, FORMAT, None, Some(&photo()), Some(0.0)).unwrap();
        let avec = source_quatre(&cv, FORMAT, None, Some(&photo()), Some(17.43)).unwrap();
        let dx = |s: &str| {
            s.split("image(\"")
                .next()
                .unwrap()
                .rsplit("dx: ")
                .next()
                .unwrap()
                .split("mm")
                .next()
                .unwrap()
                .parse::<f64>()
                .unwrap()
        };
        assert!((dx(&avec) - dx(&sans) - 17.43).abs() < 0.01);
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
            assert!(!source_une(&livre(), &cv, FORMAT, Some(&photo())).is_empty());
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
        let s = source_une(&l, &cv, FORMAT, None);
        assert!(s.contains(r"Le \#Titre"));
        assert!(s.contains(r"col\#lection"));
    }
}
