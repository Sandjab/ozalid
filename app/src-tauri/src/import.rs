//! Import d'un livre déjà fabriqué avec l'ancienne chaîne.
//!
//! Un `livre.toml` désigne son manuscrit et sa couverture ; la couverture, si elle
//! sort de l'atelier HTML, porte ses propres réglages dans un chunk PNG. Tout ce
//! qu'il faut pour reconstituer un projet est donc là, sans rien ressaisir — et les
//! livres déjà publiés deviennent du matériel de test réel.
//!
//! Convention de chemins reprise telle quelle : ceux du `livre.toml` partent du
//! **parent** du répertoire de travail (`build/`), un chemin absolu est pris tel quel.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::couverture::*;
use crate::image::Cadrage;
use crate::maquettes;
use crate::png::{self, ReglagesAtelier};
use crate::projet::{Livre, Projet};

#[derive(Debug, Deserialize)]
struct Fichier {
    livre: Section,
}

#[derive(Debug, Deserialize)]
struct Section {
    titre: String,
    #[serde(default)]
    titre_page: Option<String>,
    auteur: String,
    #[serde(default)]
    genre: Option<String>,
    #[serde(default)]
    copyright: String,
    #[serde(default)]
    chapitres: Option<u32>,
    #[serde(default)]
    manuscrit: Option<String>,
    #[serde(default)]
    couverture: Option<String>,
}

/// Ce qu'un `livre.toml` désigne, en plus de l'identité du livre.
#[derive(Debug, Clone, PartialEq)]
pub struct Designations {
    pub manuscrit: String,
    pub couverture: Option<String>,
}

/// Identité et désignations lues dans un `livre.toml`.
pub fn lire_livre_toml(contenu: &str) -> Result<(Livre, Designations), String> {
    let f: Fichier = toml::from_str(contenu).map_err(|e| format!("livre.toml illisible : {e}"))?;
    let s = f.livre;
    let v = Livre::vide();
    let livre = Livre {
        titre: s.titre,
        // Absent du `livre.toml`, il voulait déjà dire « le titre sert » : c'est ce
        // que le jeton dit désormais, en le montrant.
        titre_page: s
            .titre_page
            .unwrap_or_else(crate::projet::titre_page_defaut),
        auteur: s.auteur,
        // « roman » et non le générique d'un projet neuf : importer n'est pas créer.
        // Un `livre.toml` sans genre vient d'une chaîne qui en supposait un ; lui coller
        // « Genre » remplacerait une devinette plausible par un mot à remplir.
        genre: s.genre.unwrap_or_else(|| "roman".into()),
        // Un `livre.toml` de la chaîne Python ne porte aucune de ces cinq désignations :
        // elles prennent leurs génériques, comme la dédicace juste en dessous.
        editeur: v.editeur,
        collection: v.collection,
        monogramme: v.monogramme,
        prix: v.prix,
        mention: v.mention,
        copyright: s.copyright,
        // Un `livre.toml` de la chaîne Python ne porte pas de dédicace : le champ
        // n'existe pas de ce côté-là, et rien ne se perd à l'import.
        dedicace: String::new(),
        chapitres: s.chapitres,
    };
    let designations = Designations {
        // Même défaut que la chaîne Python, pour que les répertoires existants passent.
        manuscrit: s.manuscrit.unwrap_or_else(|| "text.md".into()),
        couverture: s.couverture,
    };
    Ok((livre, designations))
}

/// Assemble le projet à partir de ce qui a été lu. Sans accès disque : c'est ici que
/// vivent les décisions, l'orchestration est dans [`depuis_livre_toml`].
pub fn assemble(
    livre: Livre,
    texte: String,
    source_manuscrit: Option<String>,
    reglages: Option<ReglagesAtelier>,
) -> Result<Projet, String> {
    let mut p = Projet::nouveau(livre, texte);
    p.meta.manuscrit.source = source_manuscrit;

    if let Some(r) = reglages {
        let (maquette, textes) = traduit(&r)?;
        p.meta.couverture.maquette = Some(maquette);
        textes.applique(&mut p.meta.livre);
        for (url, nom) in [(&r.image, "couverture"), (&r.image4, "quatrieme")] {
            if let Some(url) = url {
                let (ext, octets) = png::data_url(url)?;
                p.images.insert(format!("{nom}.{ext}"), octets);
            }
        }
    }
    Ok(p)
}

/* ---------- traduction des réglages de l'atelier ---------- */

/// Lecteur tolérant du bloc de réglages.
///
/// Les versions successives de l'atelier n'ont pas les mêmes champs, et une même
/// valeur y est tantôt une chaîne (`"7"`), tantôt un nombre. Un champ absent laisse
/// la valeur de la maquette de départ : c'est ce qui permet à un PNG de 2026, qui
/// n'en compte que 54, de s'importer entièrement.
struct Champs<'a>(&'a BTreeMap<String, serde_json::Value>);

impl Champs<'_> {
    fn texte(&self, cle: &str) -> Option<&str> {
        self.0.get(cle)?.as_str()
    }

    fn nombre(&self, cle: &str) -> Option<f64> {
        match self.0.get(cle)? {
            serde_json::Value::Number(n) => n.as_f64(),
            serde_json::Value::String(s) => s.trim().parse().ok(),
            _ => None,
        }
    }

    fn oui(&self, cle: &str) -> Option<bool> {
        match self.0.get(cle)? {
            serde_json::Value::Bool(b) => Some(*b),
            serde_json::Value::String(s) => match s.as_str() {
                "true" | "on" => Some(true),
                "false" | "off" | "" => Some(false),
                _ => None,
            },
            _ => None,
        }
    }

    /// Couleur `#rrggbb`. Une valeur qui n'en est pas une est refusée : posée telle
    /// quelle dans la source Typst, elle ferait échouer la composition plus loin,
    /// avec un message incompréhensible.
    fn couleur(&self, cle: &str) -> Result<Option<String>, String> {
        let Some(v) = self.texte(cle) else {
            return Ok(None);
        };
        let ok =
            v.len() == 7 && v.starts_with('#') && v[1..].chars().all(|c| c.is_ascii_hexdigit());
        if ok {
            Ok(Some(v.to_string()))
        } else {
            Err(format!(
                "réglage « {cle} » : « {v} » n'est pas une couleur #rrggbb."
            ))
        }
    }
}

/// Pile CSS (`"Bodoni Moda", Didot, serif`) → famille embarquée.
fn famille(stack: &str) -> Result<String, String> {
    let premier = stack
        .split(',')
        .next()
        .unwrap_or("")
        .trim()
        .trim_matches('"');
    if police_connue(premier) {
        return Ok(premier.to_string());
    }
    Err(format!(
        "police « {premier} » non embarquée. Georgia et Helvetica appartiennent au \
         système et ne sont pas redistribuables ; choisir parmi : {}.",
        POLICES.join(", ")
    ))
}

fn style(ch: &Champs, prefixe: &str, base: &Style) -> Result<Style, String> {
    let mut s = base.clone();
    if let Some(f) = ch.texte(&format!("{prefixe}Face")) {
        s.police = famille(f)?;
    }
    if let Some(g) = ch.nombre(&format!("{prefixe}Weight")) {
        s.graisse = g as u16;
    }
    if let Some(t) = ch.nombre(&format!("{prefixe}Size")) {
        s.taille = t;
    }
    if let Some(c) = ch.couleur(&format!("{prefixe}Color"))? {
        s.couleur = c;
    }
    if let Some(t) = ch.nombre(&format!("{prefixe}Track")) {
        s.tracking = t;
    }
    if let Some(c) = ch.texte(&format!("{prefixe}Case")) {
        s.casse = match c {
            "none" => Casse::Telle,
            "uppercase" => Casse::Capitales,
            autre => {
                return Err(format!(
                    "réglage « {prefixe}Case » : casse « {autre} » non gérée."
                ))
            }
        };
    }
    Ok(s)
}

fn cadrage(ch: &Champs, prefixe: &str, base: Cadrage) -> Cadrage {
    let mut c = base;
    if let Some(v) = ch.nombre(&format!("{prefixe}ArtX")) {
        c.x = v / 100.0;
    }
    if let Some(v) = ch.nombre(&format!("{prefixe}ArtY")) {
        c.y = v / 100.0;
    }
    if let Some(v) = ch.nombre(&format!("{prefixe}Zoom")) {
        c.zoom = v;
    }
    if let Some(v) = ch.oui(&format!("{prefixe}KeepRatio")) {
        c.proportions = v;
    }
    if let Some(v) = ch.nombre(&format!("{prefixe}Stretch")) {
        c.etirement = v;
    }
    c
}

fn voile(ch: &Champs, cle: &str, base: Voile) -> Result<Voile, String> {
    let Some(v) = ch.texte(cle) else {
        return Ok(base);
    };
    Ok(match v {
        "none" => Voile::Aucun,
        "top" => Voile::Haut,
        "bottom" => Voile::Bas,
        "both" => Voile::Deux,
        "flat" => Voile::Uni,
        "light" => Voile::Clair,
        autre => return Err(format!("réglage « {cle} » : voile « {autre} » inconnu.")),
    })
}

fn align(ch: &Champs, cle: &str, base: Align) -> Result<Align, String> {
    let Some(v) = ch.texte(cle) else {
        return Ok(base);
    };
    Ok(match v {
        "left" => Align::Gauche,
        "center" => Align::Centre,
        "right" => Align::Droite,
        autre => {
            return Err(format!(
                "réglage « {cle} » : alignement « {autre} » inconnu."
            ))
        }
    })
}

/// Ce qu'un bloc de réglages de l'atelier portait et qui appartient au livre, non à la
/// maquette : l'atelier ne connaissait pas cette frontière, et rangeait l'éditeur dans
/// le pied de la 1ère, la collection sous le nom de « pastille ».
///
/// Chacun est facultatif : un bloc qui ne le porte pas ne doit pas écraser par du vide
/// la valeur générique du livre.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct TextesAtelier {
    pub monogramme: Option<String>,
    pub editeur: Option<String>,
    /// La collection vient de deux champs de l'atelier : celui de la 4ème, explicite, et
    /// la pastille, qui la portait sous un autre nom. L'explicite gagne — c'est
    /// l'arbitrage de la migration des `.ozalid`, tenu ici aussi.
    pub collection: Option<String>,
    pub prix: Option<String>,
    pub mention: Option<String>,
}

impl TextesAtelier {
    /// Pose sur le livre ce que le bloc portait, et lui seul.
    pub fn applique(self, livre: &mut Livre) {
        if let Some(v) = self.monogramme {
            livre.monogramme = v;
        }
        if let Some(v) = self.editeur {
            livre.editeur = v;
        }
        if let Some(v) = self.collection {
            livre.collection = v;
        }
        if let Some(v) = self.prix {
            livre.prix = v;
        }
        if let Some(v) = self.mention {
            livre.mention = v;
        }
    }
}

/// Réglages de l'atelier HTML → maquette du moteur Typst, et les textes du livre.
///
/// La traduction part de la maquette du même mode : ce qui manque au bloc importé —
/// et il en manque, les versions de l'atelier ayant grossi — reste donc cohérent
/// plutôt que nul.
pub fn traduit(r: &ReglagesAtelier) -> Result<(Couverture, TextesAtelier), String> {
    let ch = Champs(&r.fields);
    let mut cv = match r.mode.as_str() {
        "band" => maquettes::folio(),
        "overlay" => maquettes::surimpression(),
        "typo" => maquettes::blanche(),
        "" => maquettes::folio(), // bloc sans mode : le bandeau est le défaut de l'atelier
        autre => return Err(format!("mode de couverture inconnu : « {autre} ».")),
    };

    if let Some(c) = ch.couleur("inPaper")? {
        cv.papier = c;
    }
    cv.align = align(&ch, "inAlign", cv.align)?;
    if let Some(v) = ch.nombre("inPadX") {
        cv.pad_x = v;
    }
    if let Some(v) = ch.nombre("inBand") {
        cv.bandeau = v;
    }
    if let Some(v) = ch.oui("inInset") {
        cv.bandeau_retrait = v;
    }
    if let Some(v) = ch.nombre("inBlockY") {
        cv.bloc_y = v;
    }

    if let Some(v) = ch.oui("inFrameOn") {
        cv.cadre.actif = v;
    }
    if let Some(v) = ch.nombre("inFrameM") {
        cv.cadre.marge = v;
    }
    if let Some(c) = ch.couleur("inRule1Color")? {
        cv.cadre.filet1_couleur = c;
    }
    if let Some(v) = ch.nombre("inRule1W") {
        cv.cadre.filet1_epaisseur = v;
    }
    if let Some(v) = ch.nombre("inRule2Off") {
        cv.cadre.decroche = v;
    }
    if let Some(c) = ch.couleur("inRule2Color")? {
        cv.cadre.filet2_couleur = c;
    }
    if let Some(v) = ch.nombre("inRule2W") {
        cv.cadre.filet2_epaisseur = v;
    }
    if let Some(v) = ch.nombre("inRule2Gap") {
        cv.cadre.ecart = v;
    }

    cv.auteur = style(&ch, "inAuthor", &cv.auteur)?;
    cv.titre = style(&ch, "inTitle", &cv.titre)?;
    if let Some(v) = ch.nombre("inLeading") {
        cv.titre_interligne = v;
    }
    if let Some(v) = ch.nombre("inGap") {
        cv.titre_ecart = v;
    }
    if let Some(v) = ch.oui("inGenreOn") {
        cv.genre_visible = v;
    }
    cv.genre = style(&ch, "inGenre", &cv.genre)?;
    if let Some(v) = ch.nombre("inGenreGap") {
        cv.genre_ecart = v;
    }

    if let Some(v) = ch.oui("inImprintOn") {
        cv.pied.actif = v;
    }
    if let Some(v) = ch.nombre("inImprintY") {
        cv.pied.y = v;
    }
    cv.pied.style_mono = Style {
        italique: true,
        ..style(&ch, "inMono", &cv.pied.style_mono)?
    };
    cv.pied.style_editeur = style(&ch, "inEditor", &cv.pied.style_editeur)?;

    if let Some(v) = ch.oui("inPastilleOn") {
        cv.pastille.actif = v;
    }
    let mut textes = TextesAtelier {
        monogramme: ch.texte("inMono").map(str::to_string),
        editeur: ch.texte("inEditor").map(str::to_string),
        // La pastille de l'atelier était un nom de collection sous un autre nom. Elle ne
        // vaut que si la 4ème n'en donne pas d'explicite — voir plus bas.
        collection: ch.texte("inPastille").map(str::to_string),
        prix: None,
        mention: None,
    };
    cv.pastille.style = style(&ch, "inPastille", &cv.pastille.style)?;
    if let Some(c) = ch.couleur("inPastilleBg")? {
        cv.pastille.fond = c;
    }
    if let Some(v) = ch.texte("inPastilleAnchor") {
        cv.pastille.coin = match v {
            "bd" => Coin::BasDroite,
            "bg" => Coin::BasGauche,
            "hd" => Coin::HautDroite,
            "hg" => Coin::HautGauche,
            autre => return Err(format!("ancrage de pastille inconnu : « {autre} ».")),
        };
    }
    if let Some(v) = ch.texte("inPastilleOrient") {
        cv.pastille.verticale = v == "v";
    }
    if let Some(v) = ch.oui("inPastilleRound") {
        cv.pastille.arrondie = v;
    }
    if let Some(v) = ch.nombre("inPastilleDx") {
        cv.pastille.dx = v;
    }
    if let Some(v) = ch.nombre("inPastilleDy") {
        cv.pastille.dy = v;
    }

    cv.cadrage = cadrage(&ch, "in", cv.cadrage);
    cv.voile = voile(&ch, "inScrim", cv.voile)?;
    if let Some(v) = ch.nombre("inScrimOp") {
        cv.voile_opacite = v / 100.0;
    }

    let q = &mut cv.quatrieme;
    if let Some(v) = ch.texte("inQ4BgMode") {
        q.fond = match v {
            "herite" => FondQuatre::Herite,
            "couleur" => FondQuatre::Couleur,
            "image" => FondQuatre::Image,
            "prolongement" => FondQuatre::Panorama,
            autre => return Err(format!("fond de 4ème inconnu : « {autre} ».")),
        };
    }
    if let Some(c) = ch.couleur("inQ4Bg")? {
        q.couleur = c;
    }
    if let Some(v) = ch.texte("inQ4Text") {
        q.texte = v.to_string();
    }
    q.style = style(&ch, "inQ4Text", &q.style)?;
    if let Some(v) = ch.nombre("inQ4Leading") {
        q.interligne = v;
    }
    q.align = align(&ch, "inQ4Align", q.align)?;
    if let Some(v) = ch.nombre("inQ4PadX") {
        q.pad_x = v;
    }
    if let Some(v) = ch.nombre("inQ4Top") {
        q.top = v;
    }
    if let Some(v) = ch.oui("inQ4PiedOn") {
        q.pied_actif = v;
    }
    textes.mention = ch.texte("inQ4Mention").map(str::to_string);
    textes.prix = ch.texte("inQ4Prix").map(str::to_string);
    // La collection explicite bat la pastille, jamais l'inverse.
    if let Some(v) = ch.texte("inQ4Coll").filter(|v| !v.is_empty()) {
        textes.collection = Some(v.to_string());
    }
    q.style_pied = style(&ch, "inQ4Pied", &q.style_pied)?;
    if let Some(v) = ch.nombre("inQ4PiedY") {
        q.pied_y = v;
    }
    if let Some(v) = ch.oui("inQ4IsbnOn") {
        q.isbn_actif = v;
    }
    for (cle, cible) in [
        ("inQ4IsbnW", &mut q.isbn_l),
        ("inQ4IsbnH", &mut q.isbn_h),
        ("inQ4IsbnDx", &mut q.isbn_dx),
        ("inQ4IsbnDy", &mut q.isbn_dy),
    ] {
        if let Some(v) = ch.nombre(cle) {
            *cible = v;
        }
    }
    q.cadrage = cadrage(&ch, "inQ4", q.cadrage);
    q.voile = voile(&ch, "inQ4Scrim", q.voile)?;
    if let Some(v) = ch.nombre("inQ4ScrimOp") {
        q.voile_opacite = v / 100.0;
    }
    Ok((cv, textes))
}

/// Importe le répertoire de travail qui contient ce `livre.toml`.
pub fn depuis_livre_toml(chemin: &Path) -> Result<Projet, String> {
    let contenu =
        std::fs::read_to_string(chemin).map_err(|e| format!("{} : {e}", chemin.display()))?;
    let (livre, d) = lire_livre_toml(&contenu)?;

    let repertoire = chemin
        .parent()
        .ok_or_else(|| format!("{} : pas de répertoire parent", chemin.display()))?;
    let racine = repertoire.parent().unwrap_or(repertoire);

    // Chemin absolu : il est mémorisé dans le projet pour « Réimporter le manuscrit »,
    // et le `.ozalid` se déplace. Un chemin relatif au répertoire d'exécution de
    // l'import ne voudrait plus rien dire dès la première réouverture.
    let manuscrit = absolu(&resout(racine, &d.manuscrit), "manuscrit")?;
    let texte = std::fs::read_to_string(&manuscrit)
        .map_err(|e| format!("manuscrit illisible ({}) : {e}", manuscrit.display()))?;

    let reglages = match &d.couverture {
        Some(c) => {
            let png_path = absolu(&resout(racine, c), "couverture")?;
            let octets = std::fs::read(&png_path)
                .map_err(|e| format!("couverture illisible ({}) : {e}", png_path.display()))?;
            png::reglages(&octets)?
        }
        None => None,
    };

    assemble(
        livre,
        texte,
        Some(manuscrit.to_string_lossy().into_owned()),
        reglages,
    )
}

fn resout(racine: &Path, chemin: &str) -> PathBuf {
    // `join` ignore la racine quand le chemin est absolu : exactement la règle voulue.
    racine.join(chemin)
}

fn absolu(chemin: &Path, quoi: &str) -> Result<PathBuf, String> {
    std::fs::canonicalize(chemin)
        .map_err(|e| format!("{quoi} introuvable ({}) : {e}", chemin.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Le répertoire courant est global au processus, et les tests s'exécutent en
    /// parallèle : tout test qui le déplace doit prendre ce verrou, sinon il fait
    /// dérailler les autres de façon intermittente.
    static CWD: std::sync::Mutex<()> = std::sync::Mutex::new(());

    const TOML: &str = r#"
[livre]
titre = "Les Heures creuses"
titre_page = "Les Heures\ncreuses"
auteur = "Ivan Pjig"
genre = "roman"
copyright = """© Ivan Pjig, 2026.
Tous droits réservés."""
chapitres = 64
manuscrit = "in/texts/WIP7.md"
couverture = "in/covers/LHC-Photo.png"
"#;

    #[test]
    fn un_livre_toml_complet_donne_identite_et_designations() {
        let (l, d) = lire_livre_toml(TOML).unwrap();
        assert_eq!(l.titre, "Les Heures creuses");
        assert_eq!(l.titre_page(), "Les Heures\ncreuses");
        assert_eq!(l.chapitres, Some(64));
        assert!(l.copyright.contains("Tous droits réservés"));
        assert_eq!(d.manuscrit, "in/texts/WIP7.md");
        assert_eq!(d.couverture.as_deref(), Some("in/covers/LHC-Photo.png"));
    }

    /// Le genre et le manuscrit ont des défauts hérités de la chaîne Python : les
    /// répertoires de travail existants doivent s'importer sans être retouchés.
    #[test]
    fn les_defauts_de_la_chaine_existante_sont_respectes() {
        let (l, d) = lire_livre_toml("[livre]\ntitre = \"T\"\nauteur = \"A\"\n").unwrap();
        assert_eq!(l.genre, "roman");
        assert_eq!(d.manuscrit, "text.md");
        assert_eq!(d.couverture, None);
    }

    #[test]
    fn un_livre_toml_sans_auteur_est_refuse() {
        let err = lire_livre_toml("[livre]\ntitre = \"T\"\n").unwrap_err();
        assert!(err.contains("livre.toml illisible"), "{err}");
    }

    /// Les chemins partent du parent du répertoire de travail — « in/texts/x.md »
    /// désigne une ressource partagée, pas un fichier du répertoire lui-même.
    #[test]
    fn un_chemin_relatif_part_du_parent_du_repertoire_de_travail() {
        let racine = Path::new("/dev/ozalid/build");
        assert_eq!(
            resout(racine, "in/texts/WIP7.md"),
            PathBuf::from("/dev/ozalid/build/in/texts/WIP7.md")
        );
    }

    #[test]
    fn un_chemin_absolu_est_pris_tel_quel() {
        let racine = Path::new("/dev/ozalid/build");
        assert_eq!(
            resout(racine, "/ailleurs/roman.md"),
            PathBuf::from("/ailleurs/roman.md")
        );
    }

    /// Un PNG de l'atelier apporte les réglages ET les photos source : c'est ce qui
    /// évite de tout ressaisir. Les types des réglages sont préservés au passage.
    #[test]
    fn les_reglages_et_la_photo_du_png_entrent_dans_le_projet() {
        let b64 = {
            use base64::Engine;
            base64::engine::general_purpose::STANDARD.encode([0xFF, 0xD8, 0xFF])
        };
        let r = ReglagesAtelier {
            app: png::MOT_CLE.into(),
            v: 1,
            mode: "band".into(),
            format: vec![108.0, 178.0],
            fields: BTreeMap::from([
                ("inFormat".into(), serde_json::json!("108,178")),
                ("inFrameOn".into(), serde_json::json!(false)),
                ("inPadX".into(), serde_json::json!(7)),
            ]),
            image: Some(format!("data:image/jpeg;base64,{b64}")),
            image4: None,
        };
        let p = assemble(
            Livre {
                titre: "T".into(),
                titre_page: crate::projet::titre_page_defaut(),
                auteur: "A".into(),
                genre: "roman".into(),
                editeur: "Editeur".into(),
                collection: "Collection".into(),
                monogramme: "Monogramme".into(),
                copyright: String::new(),
                prix: "Prix".into(),
                mention: "Mention".into(),
                dedicace: String::new(),
                chapitres: None,
            },
            "## 01\n\nA.\n".into(),
            Some("/travail/roman.md".into()),
            Some(r),
        )
        .unwrap();

        let m = p.meta.couverture.maquette.unwrap();
        assert_eq!(m.mode, Mode::Bandeau);
        assert!(!m.cadre.actif, "case décochée lue comme vraie");
        assert_eq!(m.pad_x, 7.0, "réglage numérique perdu");
        // La photo suit son type déclaré, pas l'extension du PNG hôte.
        assert_eq!(p.images["couverture.jpg"], vec![0xFF, 0xD8, 0xFF]);
        assert!(!p.images.contains_key("quatrieme.png"));
        assert_eq!(
            p.meta.manuscrit.source.as_deref(),
            Some("/travail/roman.md")
        );
    }

    fn reglages(mode: &str, champs: &[(&str, serde_json::Value)]) -> ReglagesAtelier {
        ReglagesAtelier {
            app: png::MOT_CLE.into(),
            v: 1,
            mode: mode.into(),
            format: vec![108.0, 178.0],
            fields: champs
                .iter()
                .map(|(k, v)| (k.to_string(), v.clone()))
                .collect(),
            image: None,
            image4: None,
        }
    }

    /// L'atelier écrit ses valeurs tantôt en chaîne, tantôt en nombre, selon le
    /// contrôle et la version. Les deux doivent donner le même réglage — sinon un
    /// même PNG s'importerait différemment d'une version à l'autre.
    #[test]
    fn une_valeur_ecrite_en_chaine_vaut_la_meme_ecrite_en_nombre() {
        let a = traduit(&reglages("band", &[("inPadX", serde_json::json!("7.5"))]))
            .unwrap()
            .0;
        let b = traduit(&reglages("band", &[("inPadX", serde_json::json!(7.5))]))
            .unwrap()
            .0;
        assert_eq!(a.pad_x, 7.5);
        assert_eq!(a.pad_x, b.pad_x);
    }

    /// Le mode détermine la maquette de départ : ce que le bloc importé ne dit pas
    /// reste cohérent avec l'archétype, au lieu d'être nul ou emprunté à un autre.
    #[test]
    fn un_bloc_incomplet_part_de_la_maquette_de_son_mode() {
        let m = traduit(&reglages("typo", &[])).unwrap().0;
        assert_eq!(m.mode, Mode::Typo);
        assert!(m.cadre.actif, "le triple filet de la Blanche est perdu");
        assert_eq!(m.titre.police, "Bodoni Moda");
    }

    /// Une police du système ne peut pas être embarquée : l'accepter laisserait le
    /// rendu dépendre de la machine, ce que Typst existe ici pour empêcher.
    #[test]
    fn une_police_non_embarquee_interrompt_l_import() {
        let err = traduit(&reglages(
            "band",
            &[("inTitleFace", serde_json::json!("Georgia, serif"))],
        ))
        .unwrap_err();
        assert!(err.contains("Georgia"), "{err}");
        assert!(err.contains("non embarquée"), "{err}");
    }

    /// Une couleur illisible doit être refusée à l'import : posée telle quelle dans la
    /// source Typst, elle ferait échouer la composition avec un message obscur.
    #[test]
    fn une_couleur_illisible_est_refusee_a_l_import() {
        let err = traduit(&reglages(
            "band",
            &[("inPaper", serde_json::json!("blanc cassé"))],
        ))
        .unwrap_err();
        assert!(err.contains("inPaper"), "{err}");
        assert!(err.contains("#rrggbb"), "{err}");
    }

    #[test]
    fn un_mode_de_couverture_inconnu_est_refuse() {
        let err = traduit(&reglages("diorama", &[])).unwrap_err();
        assert!(err.contains("diorama"), "{err}");
    }

    /// Les pourcentages d'opacité et de cadrage changent d'échelle au passage : 62 %
    /// devient 0,62. Se tromper ici donnerait un voile opaque ou invisible.
    #[test]
    fn les_pourcentages_sont_ramenes_a_leur_echelle() {
        let m = traduit(&reglages(
            "overlay",
            &[
                ("inScrimOp", serde_json::json!(62)),
                ("inArtY", serde_json::json!(62)),
            ],
        ))
        .unwrap()
        .0;
        assert_eq!(m.voile_opacite, 0.62);
        assert_eq!(m.cadrage.y, 0.62);
    }

    /// Le chemin du manuscrit est mémorisé pour « Réimporter », et le `.ozalid` se
    /// déplace : un chemin relatif au répertoire d'où l'import a été lancé ne
    /// désignerait plus rien à la réouverture.
    #[test]
    fn la_source_memorisee_du_manuscrit_est_absolue() {
        let tmp = tempfile::tempdir().unwrap();
        let racine = tmp.path();
        std::fs::create_dir_all(racine.join("in/texts")).unwrap();
        std::fs::create_dir_all(racine.join("travail")).unwrap();
        std::fs::write(racine.join("in/texts/roman.md"), "## 01 - Un\n\nTexte.\n").unwrap();
        let toml = racine.join("travail/livre.toml");
        std::fs::write(
            &toml,
            "[livre]\ntitre = \"T\"\nauteur = \"A\"\nmanuscrit = \"in/texts/roman.md\"\n",
        )
        .unwrap();

        // Chemin relatif volontairement passé à l'import, comme depuis une ligne de
        // commande lancée ailleurs.
        let _verrou = CWD.lock().unwrap_or_else(|e| e.into_inner());
        let precedent = std::env::current_dir().unwrap();
        std::env::set_current_dir(racine).unwrap();
        let p = depuis_livre_toml(Path::new("travail/livre.toml"));
        std::env::set_current_dir(precedent).unwrap();

        let source = p.unwrap().meta.manuscrit.source.unwrap();
        assert!(
            Path::new(&source).is_absolute(),
            "source relative : {source}"
        );
        // Le séparateur appartient à la plateforme : Windows canonise en
        // « \\?\C:\…\in\texts\roman.md ». Ce que le test vérifie est le chemin désigné,
        // pas la forme qu'il prend.
        let normalise = source.replace('\\', "/");
        assert!(normalise.ends_with("in/texts/roman.md"), "{source}");
    }

    /// Un manuscrit désigné mais absent doit être signalé avec son chemin : c'est la
    /// panne courante quand un répertoire de travail a été déplacé.
    #[test]
    fn un_manuscrit_absent_est_signale_avec_son_chemin() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("travail")).unwrap();
        let toml = tmp.path().join("travail/livre.toml");
        std::fs::write(
            &toml,
            "[livre]\ntitre = \"T\"\nauteur = \"A\"\nmanuscrit = \"in/texts/absent.md\"\n",
        )
        .unwrap();
        let err = depuis_livre_toml(&toml).unwrap_err();
        assert!(err.contains("manuscrit introuvable"), "{err}");
        assert!(err.contains("absent.md"), "{err}");
    }

    #[test]
    fn un_livre_sans_png_de_l_atelier_s_importe_sans_couverture() {
        let p = assemble(
            Livre {
                titre: "T".into(),
                titre_page: crate::projet::titre_page_defaut(),
                auteur: "A".into(),
                genre: "roman".into(),
                editeur: "Editeur".into(),
                collection: "Collection".into(),
                monogramme: "Monogramme".into(),
                copyright: String::new(),
                prix: "Prix".into(),
                mention: "Mention".into(),
                dedicace: String::new(),
                chapitres: None,
            },
            "## 01\n\nA.\n".into(),
            None,
            None,
        )
        .unwrap();
        assert!(p.meta.couverture.maquette.is_none());
        assert!(p.images.is_empty());
    }
}
