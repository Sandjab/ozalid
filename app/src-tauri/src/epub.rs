//! Le livre en EPUB 3 : une archive, et rien d'autre.
//!
//! Ce module ne touche pas au disque, n'appelle pas Typst et ne connaît aucun
//! prestataire : il reçoit des chapitres et des octets, il rend des octets. C'est ce
//! qui le rend éprouvable en entier sans `fonts/`, sans sidecar et sans répertoire
//! temporaire.
//!
//! L'EPUB est **reflowable** : le lecteur choisit son corps, et la pagination n'y veut
//! plus rien dire. Rien ici ne cherche donc à reproduire la mise en page du papier —
//! seulement ce qui appartient au livre : son texte, sa coupure en chapitres, ses
//! ruptures de scène, son œil.

use crate::manuscrit::{self, Bloc, Chapitre, Morceau};
use std::time::SystemTime;

/// Texte brut → contenu XML.
///
/// Rien à voir avec `manuscrit::echappe`, qui protège le markup Typst : les deux
/// langages n'ont pas un caractère dangereux en commun, et les confondre laisserait
/// passer une esperluette ici ou un dièse là-bas.
///
/// L'apostrophe est échappée bien qu'elle ne soit dangereuse que dans un attribut :
/// le même échappement sert au texte et aux attributs, et une seule règle vaut mieux
/// que deux dont on choisirait mal.
fn echappe(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(c),
        }
    }
    out
}

/// La rupture de scène telle que l'EPUB l'écrit.
///
/// Le même caractère que sur le papier — `manuscrit::SCENE` l'a choisi parce qu'il est
/// le seul présent dans tous les fichiers de `fonts/` —, mais pas la même chaîne : la
/// constante du manuscrit est du markup Typst, `\*#h(0.8em)\*`, illisible ici.
/// L'espacement vient des espaces insécables et du CSS, non de `#h()`.
const SCENE_XHTML: &str = "*\u{a0}*\u{a0}*";

/// Texte d'un paragraphe → contenu XHTML.
///
/// La lecture des astérisques est celle de `manuscrit::morceaux`, partagée avec
/// l'intérieur : l'EPUB et le papier ne peuvent pas comprendre le même paragraphe
/// autrement. Seul le rendu diffère.
fn paragraphe(s: &str) -> String {
    manuscrit::morceaux(s)
        .into_iter()
        .map(|m| match m {
            Morceau::Brut(t) => echappe(&t),
            Morceau::Emph(t) => format!("<em>{}</em>", echappe(&t)),
            Morceau::Fort(t) => format!("<strong>{}</strong>", echappe(&t)),
        })
        .collect()
}

/// Le titre d'un chapitre tel qu'il paraît dans la table des matières.
fn intitule(ch: &Chapitre) -> String {
    if ch.titre.is_empty() {
        ch.numero.to_string()
    } else {
        format!("{} — {}", ch.numero, ch.titre)
    }
}

/// Un chapitre, dans son propre fichier.
///
/// Un seul `<h1>`, qui porte le numéro et le titre : c'est lui que la table des
/// matières vise, et deux titres de rang 1 par fichier dérouteraient les liseuses qui
/// bâtissent leur sommaire sur la structure plutôt que sur le `nav`.
fn chapitre_xhtml(ch: &Chapitre) -> String {
    let mut corps = String::from("<h1>");
    corps.push_str(&format!(r#"<span class="numero">{}</span>"#, ch.numero));
    if !ch.titre.is_empty() {
        corps.push_str(&format!(
            r#"<span class="titre">{}</span>"#,
            echappe(&ch.titre)
        ));
    }
    corps.push_str("</h1>\n");
    for b in &ch.blocs {
        match b {
            Bloc::Paragraphe(p) => corps.push_str(&format!("<p>{}</p>\n", paragraphe(p))),
            Bloc::Scene => corps.push_str(&format!("<p class=\"scene\">{SCENE_XHTML}</p>\n")),
        }
    }
    page(&intitule(ch), &corps)
}

/// L'enveloppe XHTML commune à toutes les pages de l'archive.
fn page(titre: &str, corps: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops" lang="fr" xml:lang="fr">
<head>
<meta charset="utf-8"/>
<title>{}</title>
<link rel="stylesheet" type="text/css" href="style.css"/>
</head>
<body>
{corps}</body>
</html>
"#,
        echappe(titre)
    )
}

/// Les deux fichiers d'une famille que l'EPUB déclare.
#[derive(Debug, Clone, PartialEq)]
pub struct Faces {
    pub romain: String,
    pub italique: Option<String>,
}

/// Le romain et l'italique parmi les fichiers d'une **même** famille.
///
/// Tout nom portant « Bold » est écarté d'abord : cela couvre `-Bold`, `-BoldItalic`,
/// `-SemiBold` et `-SemiBoldItalic` d'un seul coup. Sans cette exclusion, Cardo
/// donnerait « Cardo-Bold.ttf » pour romain — son fichier ordinaire s'appelle
/// « Cardo-Regular.ttf », plus long — et le livre entier sortirait en gras.
///
/// Le gras n'est pas embarqué : sur un fichier variable l'axe `wght` le rend, sur un
/// fichier statique la liseuse le synthétise. C'est le comportement d'un EPUB
/// ordinaire, et `**mot**` reste rare dans un roman.
pub fn faces(noms: &[String]) -> Option<Faces> {
    let choisir = |italique: bool| -> Option<String> {
        noms.iter()
            .filter(|n| !n.contains("Bold"))
            .filter(|n| n.contains("Italic") == italique)
            .min_by_key(|n| n.len())
            .cloned()
    };
    Some(Faces {
        romain: choisir(false)?,
        italique: choisir(true),
    })
}

/// `SystemTime` → date ISO 8601 en UTC, à la seconde, telle qu'EPUB 3 l'exige pour
/// `dcterms:modified`.
///
/// Écrit à la main plutôt que tiré d'une crate : c'est le seul endroit du projet qui
/// ait besoin d'une date, et l'algorithme tient en dix lignes. Une horloge d'avant
/// 1970 rendrait l'époque — un EPUB daté de 1970 se voit, un `unwrap` sur une machine
/// mal réglée ferait perdre un livre.
pub fn horodatage(t: SystemTime) -> String {
    let s = t
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0) as i64;
    let (a, m, j) = civil(s.div_euclid(86_400));
    let reste = s.rem_euclid(86_400);
    format!(
        "{a:04}-{m:02}-{j:02}T{:02}:{:02}:{:02}Z",
        reste / 3600,
        (reste % 3600) / 60,
        reste % 60
    )
}

/// Jours depuis 1970-01-01 → (année, mois, jour), par l'algorithme de Howard Hinnant.
/// Il place mars en tête d'année, ce qui range le 29 février en fin de cycle et évite
/// tout cas particulier de bissextile.
fn civil(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let ere = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let an = yoe + ere * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let jour = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let mois = (if mp < 10 { mp + 3 } else { mp - 9 }) as u32;
    (if mois <= 2 { an + 1 } else { an }, mois, jour)
}

/// Ce qu'un EPUB porte du livre. Les emprunts évitent de recopier le projet pour le
/// traverser ; ce module ne garde rien.
#[derive(Debug, Clone)]
pub struct Livre<'a> {
    pub titre: &'a str,
    pub auteur: &'a str,
    pub genre: &'a str,
    pub copyright: &'a str,
    pub dedicace: Option<&'a str>,
}

/// Un fichier de police, prêt à entrer dans l'archive.
#[derive(Debug, Clone, PartialEq)]
pub struct Face {
    /// Nom du fichier, sans répertoire : il devient `OEBPS/fonts/<nom>`.
    pub nom: String,
    pub octets: Vec<u8>,
}

/// L'écriture du livre, telle que l'EPUB l'embarque.
#[derive(Debug, Clone, PartialEq)]
pub struct Polices {
    /// La famille, telle que le CSS la nommera.
    pub famille: String,
    pub romain: Face,
    pub italique: Option<Face>,
}

/// Une entrée de l'archive, sous `OEBPS/`.
///
/// C'est la **seule** liste des fichiers du livre : le manifeste de l'OPF en découle,
/// le contenu du ZIP aussi. Un fichier qu'on ajouterait à l'un sans l'autre ferait
/// rejeter l'archive par une liseuse stricte, et rien d'autre ne le dirait.
#[derive(Debug, Clone, PartialEq)]
pub struct Entree {
    pub nom: String,
    pub octets: Vec<u8>,
    pub media: &'static str,
    /// `properties` de l'OPF : « nav » pour la table des matières, « cover-image »
    /// pour la couverture. Ce sont les deux seules dont EPUB 3 ait besoin.
    pub proprietes: Option<&'static str>,
    /// Vrai si l'entrée est une page du fil de lecture.
    pub spine: bool,
    /// Faux pour ce qui est déjà compressé — le PNG, les polices.
    pub compresse: bool,
}

impl Entree {
    fn xhtml(nom: &str, corps: String, spine: bool, proprietes: Option<&'static str>) -> Self {
        Self {
            nom: nom.into(),
            octets: corps.into_bytes(),
            media: "application/xhtml+xml",
            proprietes,
            spine,
            compresse: true,
        }
    }
}

/// Nom de fichier d'un chapitre. Trois chiffres : un roman dépasse rarement 999
/// chapitres, et l'ordre alphabétique des noms reste celui de la lecture.
fn nom_chapitre(rang: usize) -> String {
    format!("ch{:03}.xhtml", rang + 1)
}

/// Tout ce que l'archive porte sous `OEBPS/`, sauf `content.opf` — qui décrit cette
/// liste et ne peut donc pas s'y décrire lui-même.
fn contenu(
    livre: &Livre,
    chapitres: &[Chapitre],
    couverture_png: &[u8],
    polices: Option<&Polices>,
) -> Vec<Entree> {
    let mut e = vec![
        Entree::xhtml("couverture.xhtml", couverture_xhtml(), true, None),
        Entree::xhtml("liminaires.xhtml", liminaires_xhtml(livre), true, None),
    ];
    for (i, ch) in chapitres.iter().enumerate() {
        e.push(Entree::xhtml(
            &nom_chapitre(i),
            chapitre_xhtml(ch),
            true,
            None,
        ));
    }
    e.push(Entree::xhtml(
        "nav.xhtml",
        nav_xhtml(chapitres),
        false,
        Some("nav"),
    ));
    e.push(Entree {
        nom: "toc.ncx".into(),
        octets: ncx(livre, chapitres).into_bytes(),
        media: "application/x-dtbncx+xml",
        proprietes: None,
        spine: false,
        compresse: true,
    });
    e.push(Entree {
        nom: "style.css".into(),
        octets: css(polices).into_bytes(),
        media: "text/css",
        proprietes: None,
        spine: false,
        compresse: true,
    });
    e.push(Entree {
        nom: "images/couverture.png".into(),
        octets: couverture_png.to_vec(),
        media: "image/png",
        proprietes: Some("cover-image"),
        spine: false,
        // Un PNG est déjà compressé : le repasser en deflate ne gagne rien.
        compresse: false,
    });
    if let Some(p) = polices {
        for f in std::iter::once(&p.romain).chain(p.italique.iter()) {
            e.push(Entree {
                nom: format!("fonts/{}", f.nom),
                octets: f.octets.clone(),
                media: "font/ttf",
                proprietes: None,
                spine: false,
                compresse: false,
            });
        }
    }
    e
}

fn couverture_xhtml() -> String {
    page(
        "Couverture",
        "<div class=\"couverture\"><img src=\"images/couverture.png\" alt=\"Couverture\"/></div>\n",
    )
}

/// La page de titre, le copyright et — quand le livre en porte une — la dédicace.
///
/// Le faux-titre et les blanches du papier ne passent pas : ils n'ont de sens que sur
/// une feuille pliée. Le reste est du livre.
fn liminaires_xhtml(livre: &Livre) -> String {
    let mut c = format!(
        "<div class=\"titre-page\">\n\
         <p class=\"auteur\">{}</p>\n\
         <h1 class=\"grand-titre\">{}</h1>\n\
         <p class=\"genre\">{}</p>\n\
         </div>\n",
        echappe(livre.auteur),
        echappe(livre.titre),
        echappe(livre.genre),
    );
    c.push_str(&format!(
        "<div class=\"copyright\">{}</div>\n",
        lignes(livre.copyright)
    ));
    if let Some(d) = livre.dedicace {
        c.push_str(&format!("<div class=\"dedicace\">{}</div>\n", lignes(d)));
    }
    page("Titre", &c)
}

/// Texte à sauts de ligne → paragraphes XHTML. Les lignes vides sont écartées : elles
/// espaçaient un pavé Typst, le CSS s'en charge ici.
fn lignes(s: &str) -> String {
    s.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(|l| format!("<p>{}</p>", echappe(l)))
        .collect()
}

fn nav_xhtml(chapitres: &[Chapitre]) -> String {
    let mut l = String::new();
    for (i, ch) in chapitres.iter().enumerate() {
        l.push_str(&format!(
            "<li><a href=\"{}\">{}</a></li>\n",
            nom_chapitre(i),
            echappe(&intitule(ch))
        ));
    }
    page(
        "Table des matières",
        &format!(
            "<nav epub:type=\"toc\" id=\"toc\">\n\
             <h1>Table des matières</h1>\n\
             <ol>\n{l}</ol>\n\
             </nav>\n"
        ),
    )
}

/// La même table, au format des liseuses antérieures à EPUB 3. Elle ne coûte que
/// quelques centaines d'octets et évite un sommaire vide sur les appareils anciens.
fn ncx(livre: &Livre, chapitres: &[Chapitre]) -> String {
    let mut points = String::new();
    for (i, ch) in chapitres.iter().enumerate() {
        points.push_str(&format!(
            "<navPoint id=\"nav{n}\" playOrder=\"{n}\">\
             <navLabel><text>{}</text></navLabel>\
             <content src=\"{}\"/></navPoint>\n",
            echappe(&intitule(ch)),
            nom_chapitre(i),
            n = i + 1
        ));
    }
    format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<ncx xmlns="http://www.daisy.org/z3986/2005/ncx/" version="2005-1">
<head><meta name="dtb:uid" content="{}"/></head>
<docTitle><text>{}</text></docTitle>
<navMap>
{points}</navMap>
</ncx>
"#,
        echappe(&identifiant(livre)),
        echappe(livre.titre)
    )
}

/// Le CSS du livre. Court à dessein : ce qui n'est pas dit reste au réglage du lecteur,
/// et c'est ce qu'on attend d'un EPUB.
fn css(polices: Option<&Polices>) -> String {
    let mut s = String::new();
    let famille = match polices {
        Some(p) => {
            for (f, style) in std::iter::once((&p.romain, "normal"))
                .chain(p.italique.iter().map(|i| (i, "italic")))
            {
                s.push_str(&format!(
                    "@font-face {{\n  font-family: \"{}\";\n  font-style: {style};\n  \
                     font-weight: 100 900;\n  src: url(\"fonts/{}\");\n}}\n",
                    p.famille, f.nom
                ));
            }
            format!("\"{}\", serif", p.famille)
        }
        None => "serif".into(),
    };
    s.push_str(&format!(
        r#"
body {{ font-family: {famille}; margin: 0 5%; line-height: 1.45;
       text-align: justify; hyphens: auto; -webkit-hyphens: auto; }}
p {{ margin: 0; text-indent: 1.2em; }}
/* Le premier paragraphe d'un chapitre n'a pas d'alinéa — comme sur le papier, où
   Typst n'indente pas le paragraphe qui ouvre un bloc. Après une rupture de scène,
   en revanche, l'alinéa revient : c'est ce qui a été relevé sur la page composée. */
h1 + p {{ text-indent: 0; }}
h1 {{ margin: 2.5em 0 2em; text-align: center; font-weight: normal; }}
h1 .numero {{ display: block; font-size: 1.2em; }}
h1 .titre {{ display: block; margin-top: 0.6em; font-size: 0.85em;
             letter-spacing: 0.14em; text-transform: uppercase; }}
p.scene {{ text-align: center; text-indent: 0; margin: 1em 0; word-spacing: 0.5em; }}
.couverture {{ margin: 0; text-align: center; }}
.couverture img {{ max-width: 100%; }}
.titre-page {{ margin-top: 25%; text-align: center; }}
.titre-page p, .titre-page h1 {{ text-indent: 0; }}
.grand-titre {{ font-size: 1.6em; font-weight: normal; letter-spacing: 0.06em; }}
.genre {{ font-style: italic; }}
.copyright {{ margin-top: 40%; font-size: 0.8em; text-align: center; }}
.copyright p {{ text-indent: 0; }}
.dedicace {{ margin-top: 25%; font-style: italic; text-align: center; }}
.dedicace p {{ text-indent: 0; }}
"#
    ));
    s
}

/// L'identifiant unique du livre.
///
/// Tiré du titre et de l'auteur, non d'un tirage au sort : deux générations du même
/// livre doivent porter le même identifiant, sans quoi une liseuse y verrait deux
/// ouvrages et garderait les deux. `envoi::assaini` est déjà la fonction du projet qui
/// décide ce qu'un titre devient quand il sert de nom.
fn identifiant(livre: &Livre) -> String {
    format!(
        "urn:ozalid:{}-{}",
        crate::envoi::assaini(livre.titre),
        crate::envoi::assaini(livre.auteur)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Les deux langages de sortie du projet n'ont pas un caractère dangereux en
    /// commun : `manuscrit::echappe` protège le markup Typst (`#`, `$`, `*`…), celui-ci
    /// protège le XML. Les confondre laisserait passer une esperluette dans une
    /// archive, qu'aucune liseuse n'ouvrirait.
    #[test]
    fn l_echappement_xml_protege_ce_que_le_xml_craint() {
        assert_eq!(
            echappe(r#"Rémi & <Léa> dit "oui" d'un trait"#),
            "Rémi &amp; &lt;Léa&gt; dit &quot;oui&quot; d&apos;un trait"
        );
    }

    /// Le dièse ouvre une expression Typst, pas une entité XML : l'échappement XML ne
    /// doit pas y toucher, sans quoi les deux modules finiraient par se recopier.
    #[test]
    fn l_echappement_xml_laisse_passer_ce_qui_ne_regarde_que_typst() {
        assert_eq!(echappe("#1 *gras* $x$"), "#1 *gras* $x$");
    }

    /// L'emphase du manuscrit devient une balise, et l'échappement s'applique **au
    /// contenu**, pas au marqueur : un « & » sous emphase doit sortir échappé dans son
    /// `<em>`.
    #[test]
    fn l_emphase_devient_une_balise_et_le_contenu_reste_echappe() {
        assert_eq!(
            paragraphe("il dit *oui & non* puis **rien**"),
            "il dit <em>oui &amp; non</em> puis <strong>rien</strong>"
        );
    }

    /// La rupture de scène est le même caractère que sur le papier. `manuscrit::SCENE`
    /// n'est pas réutilisable — c'est du markup Typst — mais l'astérisque qu'il porte,
    /// si : ce test amarre les deux formes l'une à l'autre, pour que changer la marque du
    /// livre sans changer celle de l'EPUB se voie.
    #[test]
    fn la_rupture_de_scene_porte_la_meme_asterisque_que_le_papier() {
        assert!(crate::manuscrit::SCENE.contains(r"\*"));
        assert_eq!(SCENE_XHTML.matches('*').count(), 3);
        assert!(!SCENE_XHTML.contains('#'));
    }

    /// Un chapitre rend un titre unique — numéro et titre dans le même `<h1>` — puis ses
    /// blocs dans l'ordre. Deux `<h1>` par fichier dérouteraient la table des matières.
    #[test]
    fn un_chapitre_rend_un_titre_unique_puis_ses_blocs() {
        let ch = Chapitre {
            numero: 12,
            titre: "Le seuil".into(),
            blocs: vec![
                Bloc::Paragraphe("Premier.".into()),
                Bloc::Scene,
                Bloc::Paragraphe("Second.".into()),
            ],
        };
        let x = chapitre_xhtml(&ch);
        assert_eq!(x.matches("<h1").count(), 1);
        assert!(x.contains(r#"<span class="numero">12</span>"#), "{x}");
        assert!(x.contains(r#"<span class="titre">Le seuil</span>"#), "{x}");
        assert!(x.contains("<p>Premier.</p>"), "{x}");
        assert!(x.contains(r#"<p class="scene">"#), "{x}");
        assert!(x.contains("<p>Second.</p>"), "{x}");
        // L'ordre du manuscrit est l'ordre du fichier.
        assert!(x.find("Premier.") < x.find("Second."));
    }

    /// Un chapitre sans titre n'écrit pas de `<span class="titre">` vide : une liseuse
    /// afficherait une ligne blanche dans sa table des matières.
    #[test]
    fn un_chapitre_sans_titre_n_ecrit_pas_de_titre_vide() {
        let ch = Chapitre {
            numero: 1,
            titre: String::new(),
            blocs: vec![],
        };
        let x = chapitre_xhtml(&ch);
        assert!(!x.contains(r#"class="titre""#), "{x}");
        assert!(x.contains(r#"<span class="numero">1</span>"#), "{x}");
    }

    /// Un titre de chapitre contenant une esperluette casserait l'archive s'il n'était pas
    /// échappé — et le manuscrit en admet une, c'est du texte ordinaire.
    #[test]
    fn un_titre_de_chapitre_est_echappe() {
        let ch = Chapitre {
            numero: 3,
            titre: "Pile & face".into(),
            blocs: vec![],
        };
        assert!(chapitre_xhtml(&ch).contains("Pile &amp; face"));
    }

    /// Les noms de fichiers réellement posés par `app/outils/polices.sh`, groupés par
    /// famille. C'est le seul endroit du projet où ils soient écrits : `fonts/` n'est pas
    /// versionné, la règle de choix doit donc s'éprouver sur une liste, pas sur un
    /// répertoire.
    fn fichiers(famille: &str) -> Vec<String> {
        let l: &[&str] = match famille {
            "EB Garamond" => &["EBGaramond[wght].ttf", "EBGaramond-Italic[wght].ttf"],
            "Crimson Pro" => &["CrimsonPro[wght].ttf", "CrimsonPro-Italic[wght].ttf"],
            "Alegreya" => &["Alegreya[wght].ttf", "Alegreya-Italic[wght].ttf"],
            "Cardo" => &["Cardo-Regular.ttf", "Cardo-Bold.ttf", "Cardo-Italic.ttf"],
            "Vollkorn" => &["Vollkorn[wght].ttf", "Vollkorn-Italic[wght].ttf"],
            "Spectral" => &[
                "Spectral-Regular.ttf",
                "Spectral-Italic.ttf",
                "Spectral-Bold.ttf",
                "Spectral-BoldItalic.ttf",
                "Spectral-SemiBold.ttf",
                "Spectral-SemiBoldItalic.ttf",
            ],
            "Libre Baskerville" => &[
                "LibreBaskerville[wght].ttf",
                "LibreBaskerville-Italic[wght].ttf",
            ],
            _ => &[],
        };
        l.iter().map(|s| s.to_string()).collect()
    }

    /// Chacune des sept polices de labeur doit donner un romain et un italique. Une
    /// famille qui n'en donnerait pas composerait l'EPUB dans l'écriture du lecteur sans
    /// que rien d'autre ne le dise.
    #[test]
    fn les_sept_polices_de_labeur_donnent_un_romain_et_un_italique() {
        for famille in crate::interieur::POLICES_TEXTE {
            let f = faces(&fichiers(famille)).unwrap_or_else(|| panic!("{famille} : aucune face"));
            assert!(!f.romain.contains("Italic"), "{famille} : {}", f.romain);
            assert!(f.italique.is_some(), "{famille} : pas d'italique");
        }
    }

    /// Le piège de la règle : Cardo livre son romain sous « Cardo-Regular.ttf », plus long
    /// que « Cardo-Bold.ttf ». Choisir le nom le plus court sans écarter le gras
    /// composerait tout le livre en gras — et cela ne se verrait qu'à la lecture.
    #[test]
    fn le_gras_n_est_jamais_pris_pour_le_romain() {
        let f = faces(&fichiers("Cardo")).unwrap();
        assert_eq!(f.romain, "Cardo-Regular.ttf");
        assert_eq!(f.italique.as_deref(), Some("Cardo-Italic.ttf"));
    }

    /// Même piège du côté de l'italique : Spectral porte quatre fichiers en « Italic »,
    /// dont deux gras.
    #[test]
    fn l_italique_gras_n_est_jamais_pris_pour_l_italique() {
        let f = faces(&fichiers("Spectral")).unwrap();
        assert_eq!(f.romain, "Spectral-Regular.ttf");
        assert_eq!(f.italique.as_deref(), Some("Spectral-Italic.ttf"));
    }

    /// Aucun fichier, aucune face : c'est le cas « police introuvable dans `fonts/` », qui
    /// n'est pas une erreur — l'EPUB se fait alors dans l'écriture du lecteur.
    #[test]
    fn sans_fichier_il_n_y_a_pas_de_face() {
        assert!(faces(&[]).is_none());
    }

    use std::time::Duration;

    /// EPUB 3 exige un `dcterms:modified` en ISO 8601 UTC à la seconde. Les trois valeurs
    /// ci-dessous ont été relevées avec `date -u -r <secondes>` : l'époque, une date
    /// quelconque, et le 29 février d'une année bissextile — le seul cas où un calcul de
    /// calendrier écrit à la main se trompe sans qu'on s'en aperçoive.
    #[test]
    fn l_horodatage_suit_le_calendrier_annees_bissextiles_comprises() {
        let t = |s: u64| SystemTime::UNIX_EPOCH + Duration::from_secs(s);
        assert_eq!(horodatage(t(0)), "1970-01-01T00:00:00Z");
        assert_eq!(horodatage(t(1_755_000_000)), "2025-08-12T12:00:00Z");
        assert_eq!(horodatage(t(1_709_164_800)), "2024-02-29T00:00:00Z");
    }

    fn livre_temoin() -> Livre<'static> {
        Livre {
            titre: "Les Heures creuses",
            auteur: "Ivan Pjig",
            genre: "roman",
            copyright: "© 2026 Ivan Pjig\nTous droits réservés",
            dedicace: Some("À R."),
        }
    }

    fn chapitres_temoins() -> Vec<Chapitre> {
        vec![
            Chapitre {
                numero: 1,
                titre: "Le seuil".into(),
                blocs: vec![Bloc::Paragraphe("Premier.".into())],
            },
            Chapitre {
                numero: 2,
                titre: String::new(),
                blocs: vec![Bloc::Paragraphe("Second.".into())],
            },
        ]
    }

    /// L'inventaire porte, dans l'ordre, la couverture, les liminaires puis un fichier par
    /// chapitre. C'est cet ordre qui devient celui de la lecture.
    #[test]
    fn l_inventaire_ouvre_sur_la_couverture_et_suit_les_chapitres() {
        let ch = chapitres_temoins();
        let e = contenu(&livre_temoin(), &ch, b"\x89PNG", None);
        let lisibles: Vec<&str> = e
            .iter()
            .filter(|x| x.spine)
            .map(|x| x.nom.as_str())
            .collect();
        assert_eq!(
            lisibles,
            vec![
                "couverture.xhtml",
                "liminaires.xhtml",
                "ch001.xhtml",
                "ch002.xhtml"
            ]
        );
    }

    /// Le `nav` est un document XHTML, mais il n'est pas une page du livre : le laisser
    /// dans le fil de lecture ferait tourner une table des matières entre la couverture et
    /// le premier chapitre.
    #[test]
    fn la_table_des_matieres_n_est_pas_une_page_du_livre() {
        let ch = chapitres_temoins();
        let e = contenu(&livre_temoin(), &ch, b"\x89PNG", None);
        let nav = e.iter().find(|x| x.nom == "nav.xhtml").expect("pas de nav");
        assert!(!nav.spine);
        assert_eq!(nav.proprietes, Some("nav"));
    }

    /// Le PNG de couverture est **stocké** tel quel : il est déjà compressé, et le
    /// repasser en deflate ne gagne rien pour un livre qui pèse déjà quelques mégaoctets.
    #[test]
    fn la_couverture_entre_dans_l_archive_sans_etre_recompressee() {
        let ch = chapitres_temoins();
        let e = contenu(&livre_temoin(), &ch, b"\x89PNG", None);
        let img = e
            .iter()
            .find(|x| x.nom == "images/couverture.png")
            .expect("pas d'image");
        assert!(!img.compresse);
        assert_eq!(img.media, "image/png");
        assert_eq!(img.proprietes, Some("cover-image"));
        assert_eq!(img.octets, b"\x89PNG");
    }

    /// Sans police embarquée, l'inventaire n'en porte aucune et le CSS retombe sur
    /// `serif`. Ce n'est pas une erreur : le livre reste juste, seul son œil change.
    #[test]
    fn sans_police_l_inventaire_n_en_porte_aucune() {
        let ch = chapitres_temoins();
        let e = contenu(&livre_temoin(), &ch, b"\x89PNG", None);
        assert!(!e.iter().any(|x| x.nom.starts_with("fonts/")));
        let css = e.iter().find(|x| x.nom == "style.css").unwrap();
        let css = String::from_utf8(css.octets.clone()).unwrap();
        assert!(!css.contains("@font-face"), "{css}");
        assert!(css.contains("serif"), "{css}");
    }

    /// Avec une police, les deux faces entrent dans l'archive et le CSS les déclare.
    #[test]
    fn les_deux_faces_entrent_dans_l_archive_et_le_css_les_declare() {
        let ch = chapitres_temoins();
        let p = Polices {
            famille: "Cardo".into(),
            romain: Face {
                nom: "Cardo-Regular.ttf".into(),
                octets: b"R".to_vec(),
            },
            italique: Some(Face {
                nom: "Cardo-Italic.ttf".into(),
                octets: b"I".to_vec(),
            }),
        };
        let e = contenu(&livre_temoin(), &ch, b"\x89PNG", Some(&p));
        assert!(e.iter().any(|x| x.nom == "fonts/Cardo-Regular.ttf"));
        assert!(e.iter().any(|x| x.nom == "fonts/Cardo-Italic.ttf"));
        let css = e.iter().find(|x| x.nom == "style.css").unwrap();
        let css = String::from_utf8(css.octets.clone()).unwrap();
        assert_eq!(css.matches("@font-face").count(), 2, "{css}");
        assert!(css.contains("font-style: italic"), "{css}");
        assert!(css.contains(r#"url("fonts/Cardo-Regular.ttf")"#), "{css}");
    }

    /// La dédicace ne paraît que si le livre en porte une : une page vide se verrait.
    #[test]
    fn la_dedicace_ne_parait_que_si_le_livre_en_porte_une() {
        let ch = chapitres_temoins();
        let avec = contenu(&livre_temoin(), &ch, b"\x89PNG", None);
        let lim = |e: &[Entree]| {
            let x = e.iter().find(|x| x.nom == "liminaires.xhtml").unwrap();
            String::from_utf8(x.octets.clone()).unwrap()
        };
        assert!(lim(&avec).contains("À R."));

        let mut l = livre_temoin();
        l.dedicace = None;
        let sans = contenu(&l, &ch, b"\x89PNG", None);
        assert!(!lim(&sans).contains("dedicace"), "{}", lim(&sans));
    }
}
