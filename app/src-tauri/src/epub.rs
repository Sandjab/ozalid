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

use crate::manuscrit::{self, Bloc, Morceau, Piece, Sorte};
use std::io::{Cursor, Write};
use std::time::SystemTime;

use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

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

/// Le premier caractère qu'XML 1.0 n'admet pas, s'il y en a un.
///
/// L'échappement ne peut rien pour ceux-là : XML n'a de représentation ni pour le
/// caractère nu, ni pour son entité numérique. La règle est écrite en positif — `#x9`,
/// `#xA`, `#xD`, puis tout ce qui va au-delà de `#x20` sauf les non-caractères `#xFFFE`
/// et `#xFFFF` — parce qu'énumérer les interdits en oublie, et qu'un oubli ne se
/// verrait qu'à l'ouverture du livre. Les demi-codets, l'autre trou de la production
/// `Char`, ne peuvent pas exister dans un `char` de Rust.
fn caractere_interdit(s: &str) -> Option<char> {
    s.chars().find(|&c| {
        !(c == '\t' || c == '\n' || c == '\r' || (c >= ' ' && c != '\u{fffe}' && c != '\u{ffff}'))
    })
}

/// Refuse un texte que l'archive ne saurait pas porter, en disant d'où il vient.
///
/// Un refus, jamais un nettoyage : retirer le caractère donnerait un livre que personne
/// n'a écrit, et le manuscrit garderait le défaut pour la génération suivante. Le
/// chemin d'impression, lui, compose ce caractère sans broncher — c'est l'EPUB qui ne
/// sait pas le représenter, c'est donc lui qui refuse, et `manuscrit` ne bouge pas.
///
/// `ou` doit permettre d'aller le corriger : un numéro de chapitre, un nom de champ.
fn verifie_xml(s: &str, ou: &str) -> Result<(), String> {
    match caractere_interdit(s) {
        Some(c) => Err(format!(
            "{ou} : le caractère U+{:04X} ne s'écrit pas en XML, et l'EPUB en est fait. \
             À retirer du manuscrit — un traitement de texte en pose sans rien montrer.",
            c as u32
        )),
        None => Ok(()),
    }
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

/// Le titre d'une pièce tel qu'il paraît dans la table des matières. Le mot « Partie »
/// n'y figure pas : le romain suffit à distinguer une ouverture de partie d'un chapitre.
fn intitule(p: &Piece) -> String {
    match &p.sorte {
        Sorte::Chapitre(n) if p.titre.is_empty() => n.to_string(),
        Sorte::Chapitre(n) => format!("{n} — {}", p.titre),
        Sorte::Partie(r) if p.titre.is_empty() => r.clone(),
        Sorte::Partie(r) => format!("{r} — {}", p.titre),
        Sorte::Liminaire | Sorte::Annexe => p.titre.clone(),
    }
}

/// Un chapitre, dans son propre fichier.
///
/// Un seul `<h1>`, qui porte le numéro et le titre : c'est lui que la table des
/// matières vise, et deux titres de rang 1 par fichier dérouteraient les liseuses qui
/// bâtissent leur sommaire sur la structure plutôt que sur le `nav`.
fn piece_xhtml(p: &Piece) -> String {
    let mut corps = String::from("<h1>");
    match &p.sorte {
        Sorte::Chapitre(n) => corps.push_str(&format!(r#"<span class="numero">{n}</span>"#)),
        Sorte::Partie(r) => corps.push_str(&format!(r#"<span class="numero">{r}</span>"#)),
        Sorte::Liminaire | Sorte::Annexe => {}
    }
    if !p.titre.is_empty() {
        corps.push_str(&format!(
            r#"<span class="titre">{}</span>"#,
            echappe(&p.titre)
        ));
    }
    corps.push_str("</h1>\n");
    corps.push_str(&blocs_xhtml(&p.blocs));
    page(&intitule(p), &corps)
}

/// Les blocs d'une pièce, en XHTML. Une page de partie n'en a aucun : elle ne rend
/// alors que son `<h1>`.
fn blocs_xhtml(blocs: &[Bloc]) -> String {
    let mut s = String::new();
    for b in blocs {
        match b {
            Bloc::Paragraphe(p) => s.push_str(&format!("<p>{}</p>\n", paragraphe(p))),
            Bloc::Scene => s.push_str(&format!("<p class=\"scene\">{SCENE_XHTML}</p>\n")),
        }
    }
    s
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
/// fichier statique la liseuse le synthétise — à condition que le CSS ne lui annonce pas
/// une plage de graisses que le fichier ne couvre pas, ce dont [`variable`] décide.
/// C'est le comportement d'un EPUB ordinaire, et `**mot**` reste rare dans un roman.
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
    /// Le titre tel qu'il paraît sur la page de titre, avec les sauts de ligne que
    /// l'auteur a écrits. Distinct de `titre`, qui est la métadonnée : une liseuse range
    /// le livre sous celui-là, et un saut de ligne n'a rien à y faire.
    pub titre_page: &'a str,
    pub auteur: &'a str,
    pub genre: &'a str,
    pub copyright: &'a str,
    pub dedicace: Option<&'a str>,
}

/// Un fichier de police, prêt à entrer dans l'archive.
#[derive(Debug, Clone, PartialEq)]
pub struct Face {
    /// Nom du fichier sur le disque, sans répertoire. Ce n'est pas celui de l'archive :
    /// [`nom_dans_l_archive`] en dérive un que le `href` du manifeste et l'`url()` du
    /// CSS puissent porter.
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

/// Nom de fichier du disque → nom sous lequel l'archive le porte.
///
/// Le `href` du manifeste et l'`url()` du CSS sont des URL, pas des chemins : les
/// crochets du bloc d'axes de Google Fonts — `EBGaramond[wght].ttf`, que cinq des sept
/// familles de labeur portent — sont des *gen-delims* de la RFC 3986, interdits dans un
/// segment. EPUBCheck refuse l'archive entière, et une liseuse indulgente ne résout pas
/// la police : le livre retombe sans un mot sur l'écriture du lecteur.
///
/// Le nom dans l'archive n'a aucune raison d'être celui du disque, c'est donc lui qui
/// cède. Seul le radical est repris en main, l'extension porte le type et ne se
/// réécrit pas. Un nom déjà sobre — `Cardo-Regular.ttf` — en ressort intact.
fn nom_dans_l_archive(nom: &str) -> String {
    let (radical, ext) = match nom.rsplit_once('.') {
        Some((r, e)) => (r, format!(".{e}")),
        None => (nom, String::new()),
    };
    let brut: String = radical
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect();
    let propre: Vec<&str> = brut.split('-').filter(|s| !s.is_empty()).collect();
    format!("{}{ext}", propre.join("-"))
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
    pieces: &[Piece],
    couverture_png: &[u8],
    polices: Option<&Polices>,
) -> Vec<Entree> {
    let mut e = vec![
        Entree::xhtml("couverture.xhtml", couverture_xhtml(), true, None),
        Entree::xhtml("liminaires.xhtml", liminaires_xhtml(livre), true, None),
    ];
    for (i, p) in pieces.iter().enumerate() {
        e.push(Entree::xhtml(&nom_chapitre(i), piece_xhtml(p), true, None));
    }
    e.push(Entree::xhtml(
        "nav.xhtml",
        nav_xhtml(pieces),
        false,
        Some("nav"),
    ));
    e.push(Entree {
        nom: "toc.ncx".into(),
        octets: ncx(livre, pieces).into_bytes(),
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
                nom: format!("fonts/{}", nom_dans_l_archive(&f.nom)),
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
        titre_coupe(livre.titre_page),
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

/// Le titre de la page de titre, ses coupures rendues.
///
/// XHTML replie tout blanc : sans `<br/>`, le titre que l'auteur a coupé se recollerait,
/// là où le papier honore la coupure par le `\` de Typst. Les lignes vides ne sont pas
/// écartées comme dans [`lignes`] : dans un titre, une ligne sautée est un espacement
/// voulu, non un reste de pavé.
fn titre_coupe(s: &str) -> String {
    s.lines().map(echappe).collect::<Vec<_>>().join("<br/>")
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

fn nav_xhtml(pieces: &[Piece]) -> String {
    let mut l = String::new();
    for (i, p) in pieces.iter().enumerate() {
        l.push_str(&format!(
            "<li><a href=\"{}\">{}</a></li>\n",
            nom_chapitre(i),
            echappe(&intitule(p))
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
fn ncx(livre: &Livre, pieces: &[Piece]) -> String {
    let mut points = String::new();
    for (i, p) in pieces.iter().enumerate() {
        points.push_str(&format!(
            "<navPoint id=\"nav{n}\" playOrder=\"{n}\">\
             <navLabel><text>{}</text></navLabel>\
             <content src=\"{}\"/></navPoint>\n",
            echappe(&intitule(p)),
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

/// Ce fichier porte-t-il un axe variable, au vu de son nom **sur le disque** ?
///
/// C'est la convention de nommage de Google Fonts, que suit `app/outils/polices.sh` :
/// le bloc d'axes entre crochets, `EBGaramond[wght].ttf`. Ce n'est pas une lecture de
/// la table `fvar` — il faudrait ouvrir la police, et ce module ne lit que des octets
/// qu'on lui tend. Le nom du disque, non celui de l'archive : [`nom_dans_l_archive`] a
/// justement pour tâche de faire disparaître ces crochets.
///
/// Se tromper coûte peu dans les deux sens : croire statique une variable fait perdre
/// un vrai gras au profit d'un gras synthétique, croire variable une statique ne coûte
/// rien tant que le fichier couvre la plage. Les deux dégradations sont douces, là où
/// annoncer `100 900` sur une statique supprime le gras purement et simplement — la
/// liseuse prend la face telle quelle et ne synthétise plus rien.
fn variable(nom: &str) -> bool {
    nom.contains('[')
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
                     font-weight: {};\n  src: url(\"fonts/{}\");\n}}\n",
                    p.famille,
                    if variable(&f.nom) {
                        "100 900"
                    } else {
                        "normal"
                    },
                    nom_dans_l_archive(&f.nom)
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
///
/// Les espaces d'`assaini` deviennent des tirets : elle nomme des fichiers, où
/// l'espace est légitime, mais une URN ne peut pas en porter un sans l'encoder — et un
/// identifiant encodé se relirait mal d'une génération à l'autre. Un titre à tiret peut
/// donc produire un `--` dans le résultat ; ça ne se répare pas, l'identifiant est
/// opaque et seule sa stabilité compte.
fn identifiant(livre: &Livre) -> String {
    format!(
        "urn:ozalid:{}-{}",
        crate::envoi::assaini(livre.titre).replace(' ', "-"),
        crate::envoi::assaini(livre.auteur).replace(' ', "-")
    )
}

/// Nom de fichier → `id` XML.
///
/// Un `id` ne peut ni commencer par un chiffre ni porter de barre oblique ou de point ;
/// un nom de fichier peut les trois. Le préfixe règle le chiffre, la substitution règle
/// le reste.
///
/// La fonction n'est **pas** injective : elle écrase toute ponctuation sur un même
/// tiret, et `Foo_1.ttf` comme `Foo-1.ttf` donnent `f-Foo-1-ttf`. Ce qui garantit
/// l'unicité des `id` du manifeste n'est donc pas elle, c'est l'inventaire : ses noms
/// sont tous écrits ici, en nombre fixe — les pages, le CSS, le PNG — ou numérotés
/// — les chapitres —, et les deux seules faces qui viennent du disque diffèrent par
/// « Italic », des lettres qu'aucune substitution ne touche.
///
/// Ce qui la romprait : une entrée dont le nom viendrait du disque sans cette
/// contrainte, et qui ne différerait d'une autre que par un caractère non alphanumérique.
/// `deux_entrees_ne_partagent_jamais_un_id` monte cette garde.
fn id_de(nom: &str) -> String {
    let corps: String = nom
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    format!("f-{corps}")
}

/// Le manifeste et le fil de lecture, dérivés de l'inventaire.
///
/// `content.opf` ne se manifeste pas lui-même : c'est lui qui décrit les autres, et la
/// spec de l'EPUB le désigne par `META-INF/container.xml`.
fn opf(livre: &Livre, entrees: &[Entree], modifie: &str) -> String {
    let mut manifeste = String::new();
    for e in entrees {
        let props = match e.proprietes {
            Some(p) => format!(" properties=\"{p}\""),
            None => String::new(),
        };
        manifeste.push_str(&format!(
            "<item id=\"{}\" href=\"{}\" media-type=\"{}\"{props}/>\n",
            id_de(&e.nom),
            echappe(&e.nom),
            e.media
        ));
    }
    let mut fil = String::new();
    for e in entrees.iter().filter(|e| e.spine) {
        fil.push_str(&format!("<itemref idref=\"{}\"/>\n", id_de(&e.nom)));
    }
    format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<package xmlns="http://www.idpf.org/2007/opf" version="3.0" unique-identifier="pub-id" xml:lang="fr">
<metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
<dc:identifier id="pub-id">{ident}</dc:identifier>
<dc:title>{titre}</dc:title>
<dc:creator>{auteur}</dc:creator>
<dc:language>fr</dc:language>
<dc:rights>{droits}</dc:rights>
<meta property="dcterms:modified">{modifie}</meta>
<meta name="cover" content="{cover}"/>
</metadata>
<manifest>
{manifeste}</manifest>
<spine toc="{ncx}">
{fil}</spine>
</package>
"#,
        ident = echappe(&identifiant(livre)),
        titre = echappe(livre.titre),
        auteur = echappe(livre.auteur),
        droits = echappe(&livre.copyright.replace('\n', " ")),
        cover = id_de("images/couverture.png"),
        ncx = id_de("toc.ncx"),
    )
}

/// Le type MIME de l'EPUB, en clair et non compressé, en tête d'archive.
const MIMETYPE: &str = "application/epub+zip";

/// Le seul chemin fixe de la spec : c'est là que toute liseuse entre, et c'est lui qui
/// désigne l'OPF.
const CONTAINER: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
<rootfiles>
<rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/>
</rootfiles>
</container>
"#;

/// Le livre en EPUB 3, en mémoire.
///
/// `modifie` est l'horodatage qu'EPUB 3 exige — voir [`horodatage`]. Il est passé plutôt
/// que lu ici : ce module ne consulte pas d'horloge, sans quoi ses tests dépendraient
/// du jour où on les lance.
pub fn archive(
    livre: &Livre,
    pieces: &[Piece],
    couverture_png: &[u8],
    polices: Option<&Polices>,
    modifie: &str,
) -> Result<Vec<u8>, String> {
    verifie(livre, pieces)?;
    let entrees = contenu(livre, pieces, couverture_png, polices);
    let opf = opf(livre, &entrees, modifie);

    let mut buf = Vec::new();
    {
        let mut zip = ZipWriter::new(Cursor::new(&mut buf));
        let stocke = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
        let deflate = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        // L'ordre des trois premières entrées n'est pas un style : la spec veut
        // `mimetype` en tête, non compressé, et `META-INF/container.xml` est le seul
        // chemin qu'une liseuse cherche sans qu'on le lui dise.
        pose(&mut zip, "mimetype", MIMETYPE.as_bytes(), stocke)?;
        pose(
            &mut zip,
            "META-INF/container.xml",
            CONTAINER.as_bytes(),
            deflate,
        )?;
        pose(&mut zip, "OEBPS/content.opf", opf.as_bytes(), deflate)?;
        for e in &entrees {
            let opts = if e.compresse { deflate } else { stocke };
            pose(&mut zip, &format!("OEBPS/{}", e.nom), &e.octets, opts)?;
        }
        zip.finish()
            .map_err(|e| format!("clôture de l'EPUB : {e}"))?;
    }
    Ok(buf)
}

/// Tout ce que l'archive refuse, et qui ne dépend que du projet.
///
/// Séparée d'[`archive`] parce que ces trois refus sont connus **avant** la première
/// écriture sur le disque : les laisser tomber à la fin faisait payer à l'auteur la
/// composition entière — vingt secondes et un PDF neuf — pour un caractère qu'un
/// traitement de texte avait posé sans rien montrer. `ebook::generer` les pose donc en
/// tête, et `archive` continue de les poser aussi : un module qui ne peut produire une
/// archive invalide que si son appelant a oublié de vérifier n'est pas une garde.
/// Où une faute a été trouvée, dit comme l'auteur nomme la pièce dans son manuscrit :
/// c'est ce nom-là qu'il ira chercher pour corriger.
fn ou_dans_le_livre(p: &Piece) -> String {
    match &p.sorte {
        Sorte::Chapitre(n) => format!("chapitre {n}"),
        Sorte::Partie(r) => format!("partie {r}"),
        Sorte::Liminaire | Sorte::Annexe => p.titre.to_lowercase(),
    }
}

pub fn verifie(livre: &Livre, pieces: &[Piece]) -> Result<(), String> {
    if pieces.is_empty() {
        return Err("aucun chapitre : il n'y a pas de livre à mettre en EPUB.".into());
    }
    // `dc:title` doit porter au moins un caractère, sans quoi l'archive est rejetée à
    // l'ingestion. Des blancs la passeraient, mais laisseraient une ligne muette dans la
    // bibliothèque du lecteur : ils ne font pas un titre.
    if livre.titre.trim().is_empty() {
        return Err(
            "aucun titre : une liseuse range le livre sous ce nom, et une archive \
                    qui n'en porte pas est refusée à l'ingestion."
                .into(),
        );
    }
    verifie_xml(livre.titre, "le titre du livre")?;
    verifie_xml(livre.titre_page, "le titre de la page de titre")?;
    verifie_xml(livre.auteur, "l'auteur")?;
    verifie_xml(livre.genre, "le genre")?;
    verifie_xml(livre.copyright, "le copyright")?;
    if let Some(d) = livre.dedicace {
        verifie_xml(d, "la dédicace")?;
    }
    for p in pieces {
        let ou = ou_dans_le_livre(p);
        verifie_xml(&p.titre, &ou)?;
        for b in &p.blocs {
            if let Bloc::Paragraphe(t) = b {
                verifie_xml(t, &ou)?;
            }
        }
    }
    Ok(())
}

fn pose<W: Write + std::io::Seek>(
    zip: &mut ZipWriter<W>,
    nom: &str,
    contenu: &[u8],
    opts: SimpleFileOptions,
) -> Result<(), String> {
    zip.start_file(nom, opts)
        .map_err(|e| format!("{nom} : {e}"))?;
    zip.write_all(contenu).map_err(|e| format!("{nom} : {e}"))
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
        let ch = Piece {
            sorte: Sorte::Chapitre(12),
            titre: "Le seuil".into(),
            blocs: vec![
                Bloc::Paragraphe("Premier.".into()),
                Bloc::Scene,
                Bloc::Paragraphe("Second.".into()),
            ],
        };
        let x = piece_xhtml(&ch);
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
        let ch = Piece {
            sorte: Sorte::Chapitre(1),
            titre: String::new(),
            blocs: vec![],
        };
        let x = piece_xhtml(&ch);
        assert!(!x.contains(r#"class="titre""#), "{x}");
        assert!(x.contains(r#"<span class="numero">1</span>"#), "{x}");
    }

    /// Un titre de chapitre contenant une esperluette casserait l'archive s'il n'était pas
    /// échappé — et le manuscrit en admet une, c'est du texte ordinaire.
    #[test]
    fn un_titre_de_chapitre_est_echappe() {
        let ch = Piece {
            sorte: Sorte::Chapitre(3),
            titre: "Pile & face".into(),
            blocs: vec![],
        };
        assert!(piece_xhtml(&ch).contains("Pile &amp; face"));
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

    /// Les noms qu'`app/outils/polices.sh` va réellement chercher pour une famille.
    ///
    /// Le script est versionné, `fonts/` ne l'est pas : c'est donc lui qui fait foi, et
    /// il se lit sans le répertoire — donc en intégration continue. Ses chemins sont
    /// écrits en clair, un par ligne, entre guillemets, sous le répertoire OFL de Google
    /// Fonts — qui est le nom de la famille en minuscules et sans espace, pour les sept.
    fn poses_par_le_script(famille: &str) -> Vec<String> {
        let script = include_str!("../../outils/polices.sh");
        let repertoire = format!("{}/", famille.to_lowercase().replace(' ', ""));
        script
            .lines()
            .filter_map(|l| l.trim().strip_prefix('"'))
            .filter_map(|l| l.strip_suffix('"'))
            .filter_map(|l| l.strip_prefix(&repertoire))
            .map(str::to_string)
            .collect()
    }

    /// [`fichiers`] est une **copie** de la réalité, prise à un instant. Google Fonts
    /// renomme — le jour où `polices.sh` livrera `EBGaramond-Regular[wght].ttf`, `faces`
    /// choisirait mal ou ne choisirait rien, tous les EPUB perdraient leur police sans un
    /// mot, le compte rendu dirait « famille introuvable » — le message d'une autre cause
    /// — et la suite de tests resterait verte de bout en bout. Ce test est ce qui ferme
    /// le trou.
    #[test]
    fn la_liste_des_fichiers_suit_le_script_qui_les_pose() {
        for famille in crate::interieur::POLICES_TEXTE {
            let mut poses = poses_par_le_script(famille);
            let mut copie = fichiers(famille);
            poses.sort();
            copie.sort();
            assert!(
                !poses.is_empty(),
                "{famille} : `app/outils/polices.sh` ne pose plus aucun fichier sous ce \
                 répertoire OFL. Le tableau de `fichiers` doit suivre le script."
            );
            assert_eq!(
                copie, poses,
                "{famille} : le tableau de `fichiers` ne dit plus ce que \
                 `app/outils/polices.sh` pose dans `fonts/`. C'est au tableau de suivre \
                 le script, jamais l'inverse."
            );
        }
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

    /// Les crochets du bloc d'axes sont des *gen-delims* de la RFC 3986 : un segment de
    /// chemin ne peut pas les porter, et EPUBCheck rejette l'archive entière pour ce seul
    /// motif. Cinq des sept familles de labeur les portent sur le disque, donc le nom du
    /// disque ne peut pas être celui de l'archive. Les deux autres, elles, ne doivent pas
    /// bouger : un assainissement qui renommerait tout ferait perdre le nom d'origine sans
    /// rien gagner.
    #[test]
    fn le_nom_d_une_police_perd_ce_qu_une_url_interdit() {
        assert_eq!(
            nom_dans_l_archive("EBGaramond[wght].ttf"),
            "EBGaramond-wght.ttf"
        );
        assert_eq!(nom_dans_l_archive("Cardo-Regular.ttf"), "Cardo-Regular.ttf");
        assert_eq!(
            nom_dans_l_archive("Libre Baskerville & Cie.ttf"),
            "Libre-Baskerville-Cie.ttf"
        );
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
            titre_page: "Les Heures creuses",
            auteur: "Ivan Pjig",
            genre: "roman",
            copyright: "© 2026 Ivan Pjig\nTous droits réservés",
            dedicace: Some("À R."),
        }
    }

    fn chapitres_temoins() -> Vec<Piece> {
        vec![
            Piece {
                sorte: Sorte::Chapitre(1),
                titre: "Le seuil".into(),
                blocs: vec![Bloc::Paragraphe("Premier.".into())],
            },
            Piece {
                sorte: Sorte::Chapitre(2),
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

    /// L'écriture que cinq familles sur sept portent réellement sur le disque : un nom à
    /// bloc d'axes. C'est le jeu d'essai qui manquait — les deux témoins précédents,
    /// Cardo et Spectral, sont justement les deux seules familles épargnées.
    fn polices_temoins() -> Polices {
        Polices {
            famille: "EB Garamond".into(),
            romain: Face {
                nom: "EBGaramond[wght].ttf".into(),
                octets: b"R".to_vec(),
            },
            italique: Some(Face {
                nom: "EBGaramond-Italic[wght].ttf".into(),
                octets: b"I".to_vec(),
            }),
        }
    }

    /// Le nom assaini sert aux trois endroits qui doivent s'accorder : le chemin dans
    /// l'archive, le `href` du manifeste — qui en découle — et l'`url()` du CSS. Un seul
    /// des trois resté sur le nom du disque, et la liseuse ne résout plus la police : elle
    /// retombe sans un mot sur l'écriture du lecteur.
    #[test]
    fn le_css_et_l_archive_visent_le_meme_nom_assaini() {
        let ch = chapitres_temoins();
        let p = polices_temoins();
        let e = contenu(&livre_temoin(), &ch, b"\x89PNG", Some(&p));
        let noms: Vec<&str> = e.iter().map(|x| x.nom.as_str()).collect();
        assert!(noms.contains(&"fonts/EBGaramond-wght.ttf"), "{noms:?}");
        assert!(
            noms.contains(&"fonts/EBGaramond-Italic-wght.ttf"),
            "{noms:?}"
        );
        let css = e.iter().find(|x| x.nom == "style.css").unwrap();
        let css = String::from_utf8(css.octets.clone()).unwrap();
        assert!(css.contains(r#"url("fonts/EBGaramond-wght.ttf")"#), "{css}");
        assert!(!css.contains('['), "{css}");
    }

    /// `font-weight: 100 900` sur un fichier statique dit à la liseuse que cette face
    /// couvre déjà 700 : elle la prend telle quelle et ne synthétise rien. Mesuré dans
    /// Chromium sur `Cardo-Regular.ttf`, la même chaîne à 64 px rend 2449 pixels opaques
    /// en gras comme en normal — `**mot**` sort identique au texte courant. Avec
    /// `font-weight: normal`, 2832 : le gras revient. Les deux familles statiques des
    /// sept, Cardo et Spectral, en dépendent.
    #[test]
    fn seule_une_police_variable_annonce_la_plage_des_graisses() {
        let statique = Polices {
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
        let s = css(Some(&statique));
        assert_eq!(graisses_des_faces(&s), ["normal", "normal"], "{s}");

        let v = css(Some(&polices_temoins()));
        assert_eq!(graisses_des_faces(&v), ["100 900", "100 900"], "{v}");
    }

    /// La graisse déclarée par chaque `@font-face`, et elle seule : le CSS du livre en
    /// porte d'autres pour ses titres, et les compter avec fausserait la mesure.
    fn graisses_des_faces(css: &str) -> Vec<String> {
        css.split("@font-face")
            .skip(1)
            .map(|bloc| {
                bloc.split("font-weight:")
                    .nth(1)
                    .and_then(|g| g.split(';').next())
                    .expect("un @font-face sans graisse")
                    .trim()
                    .to_string()
            })
            .collect()
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

    /// Le blanc que l'auteur a écrit dans son titre atteint le papier — `interieur` le
    /// rend par le `\` de Typst. Ici, XHTML replie tout blanc : sans `<br/>`, le titre
    /// que l'auteur avait coupé se recolle, et les deux sorties du même livre ne montrent
    /// plus la même page de titre.
    ///
    /// `dc:title` ne suit pas : c'est une métadonnée, la liseuse range le livre sous ce
    /// nom dans sa bibliothèque, et un saut de ligne n'y a rien à faire.
    #[test]
    fn le_titre_de_page_garde_ses_sauts_de_ligne_la_metadonnee_non() {
        let l = Livre {
            titre_page: "Les Heures\ncreuses",
            ..livre_temoin()
        };
        let x = liminaires_xhtml(&l);
        assert!(x.contains("Les Heures<br/>creuses"), "{x}");

        let e = contenu(&l, &chapitres_temoins(), b"\x89PNG", None);
        let o = opf(&l, &e, "2026-08-22T10:00:00Z");
        assert!(o.contains("<dc:title>Les Heures creuses</dc:title>"), "{o}");
        assert!(!o.contains("<br/>"), "{o}");
    }

    /// Le manifeste porte exactement les entrées de l'inventaire, et le fil de lecture ne
    /// renvoie qu'à des `id` du manifeste. Un `idref` orphelin fait rejeter l'archive.
    #[test]
    fn le_manifeste_porte_l_inventaire_et_le_fil_n_y_renvoie_que_des_id() {
        let ch = chapitres_temoins();
        let e = contenu(&livre_temoin(), &ch, b"\x89PNG", None);
        let x = opf(&livre_temoin(), &e, "2026-08-22T10:00:00Z");
        for entree in &e {
            assert!(
                x.contains(&format!("href=\"{}\"", entree.nom)),
                "{} n'est pas manifesté : {x}",
                entree.nom
            );
        }
        for entree in e.iter().filter(|x| x.spine) {
            assert!(
                x.contains(&format!("idref=\"{}\"", id_de(&entree.nom))),
                "{} n'est pas dans le fil : {x}",
                entree.nom
            );
        }
        // Le fil ne porte que des pages : ni le CSS, ni le PNG, ni le `nav`.
        assert!(
            !x.contains(&format!("idref=\"{}\"", id_de("style.css"))),
            "{x}"
        );
        assert!(
            !x.contains(&format!("idref=\"{}\"", id_de("nav.xhtml"))),
            "{x}"
        );
    }

    /// Les métadonnées qu'EPUB 3 exige, plus celle que les liseuses anciennes lisent pour
    /// afficher une vignette.
    #[test]
    fn les_metadonnees_disent_le_livre_et_sa_couverture() {
        let ch = chapitres_temoins();
        let e = contenu(&livre_temoin(), &ch, b"\x89PNG", None);
        let x = opf(&livre_temoin(), &e, "2026-08-22T10:00:00Z");
        assert!(x.contains("<dc:title>Les Heures creuses</dc:title>"), "{x}");
        assert!(x.contains("<dc:creator>Ivan Pjig</dc:creator>"), "{x}");
        assert!(x.contains("<dc:language>fr</dc:language>"), "{x}");
        assert!(x.contains("urn:ozalid:Les-Heures-creuses-Ivan-Pjig"), "{x}");
        assert!(
            x.contains(r#"<meta property="dcterms:modified">2026-08-22T10:00:00Z</meta>"#),
            "{x}"
        );
        // La vignette, deux fois : `properties` pour EPUB 3, `meta name` pour le reste.
        assert!(x.contains(r#"properties="cover-image""#), "{x}");
        assert!(
            x.contains(&format!(
                r#"<meta name="cover" content="{}"/>"#,
                id_de("images/couverture.png")
            )),
            "{x}"
        );
    }

    /// Un `id` XML ne peut ni commencer par un chiffre ni porter de barre oblique ou de
    /// point. Un nom de fichier, si.
    #[test]
    fn un_nom_de_fichier_devient_un_id_xml_valide() {
        assert_eq!(id_de("images/couverture.png"), "f-images-couverture-png");
        assert_eq!(id_de("ch001.xhtml"), "f-ch001-xhtml");
        assert_eq!(
            id_de("fonts/Cardo-Regular.ttf"),
            "f-fonts-Cardo-Regular-ttf"
        );
    }

    /// Deux entrées qui partageraient un `id` donneraient un manifeste où le fil de
    /// lecture viserait la mauvaise page, sans qu'aucune liseuse ne le dise. Ce n'est pas
    /// `id_de` qui l'empêche — elle écrase toute ponctuation sur un tiret, et deux noms
    /// qui n'en diffèrent que là se confondent : c'est l'inventaire, dont les noms sont
    /// tous écrits ici. La garde est donc sur l'inventaire, pas sur la fonction.
    #[test]
    fn deux_entrees_ne_partagent_jamais_un_id() {
        let e = contenu(
            &livre_temoin(),
            &chapitres_temoins(),
            b"\x89PNG",
            Some(&polices_temoins()),
        );
        let ids: std::collections::BTreeSet<String> = e.iter().map(|x| id_de(&x.nom)).collect();
        assert_eq!(ids.len(), e.len(), "{ids:?}");
        assert_eq!(id_de("Foo_1.ttf"), id_de("Foo-1.ttf"));
    }

    use std::io::Read;

    fn relire(octets: &[u8]) -> zip::ZipArchive<Cursor<Vec<u8>>> {
        zip::ZipArchive::new(Cursor::new(octets.to_vec())).expect("archive illisible")
    }

    /// `mimetype` doit être la **première** entrée et n'être pas compressée : c'est la
    /// seule dont la spec de l'EPUB fixe la place et la méthode, et une liseuse stricte
    /// refuse l'archive sinon. Le défaut est invisible tant qu'on n'ouvre le fichier que
    /// dans Calibre, indulgent.
    #[test]
    fn le_mimetype_ouvre_l_archive_et_n_est_pas_compresse() {
        let ch = chapitres_temoins();
        let a = archive(
            &livre_temoin(),
            &ch,
            b"\x89PNG",
            None,
            "2026-08-22T10:00:00Z",
        )
        .unwrap();
        let mut z = relire(&a);
        let e = z.by_index(0).unwrap();
        assert_eq!(e.name(), "mimetype");
        assert_eq!(e.compression(), zip::CompressionMethod::Stored);
        drop(e);
        let mut s = String::new();
        z.by_name("mimetype")
            .unwrap()
            .read_to_string(&mut s)
            .unwrap();
        assert_eq!(s, "application/epub+zip");
    }

    /// Ce que l'archive porte sous `OEBPS/` et ce que le manifeste déclare doivent se
    /// recouvrir **exactement**, `content.opf` excepté. C'est le défaut qui fait rejeter
    /// un EPUB par une liseuse stricte sans qu'aucun autre test ne le voie.
    ///
    /// Le témoin porte un nom à bloc d'axes, celui de cinq familles sur sept : recouvrir
    /// ne suffit pas, encore faut-il que le nom commun aux deux soit une URL légale.
    #[test]
    fn l_archive_et_le_manifeste_se_recouvrent_exactement() {
        let ch = chapitres_temoins();
        let p = polices_temoins();
        let a = archive(
            &livre_temoin(),
            &ch,
            b"\x89PNG",
            Some(&p),
            "2026-08-22T10:00:00Z",
        )
        .unwrap();
        let mut z = relire(&a);

        let dans_l_archive: std::collections::BTreeSet<String> = (0..z.len())
            .map(|i| z.by_index(i).unwrap().name().to_string())
            .filter(|n| n.starts_with("OEBPS/") && n != "OEBPS/content.opf")
            .map(|n| n["OEBPS/".len()..].to_string())
            .collect();

        let mut opf = String::new();
        z.by_name("OEBPS/content.opf")
            .unwrap()
            .read_to_string(&mut opf)
            .unwrap();
        let manifestes: std::collections::BTreeSet<String> = opf
            .lines()
            .filter_map(|l| l.split("href=\"").nth(1))
            .filter_map(|l| l.split('"').next())
            .map(str::to_string)
            .collect();

        assert_eq!(dans_l_archive, manifestes);
        assert!(dans_l_archive.contains("fonts/EBGaramond-Italic-wght.ttf"));
        // Aucun nom d'entrée ne peut porter un caractère qu'un segment d'URL interdit :
        // le `href` du manifeste et l'`url()` du CSS sont des URL, pas des chemins.
        for n in &dans_l_archive {
            assert!(!n.contains('[') && !n.contains(']'), "{n}");
        }
    }

    /// `META-INF/container.xml` désigne l'OPF : c'est par lui que toute liseuse entre dans
    /// l'archive, et un chemin faux la rend illisible sans autre message.
    #[test]
    fn le_container_designe_l_opf() {
        let ch = chapitres_temoins();
        let a = archive(
            &livre_temoin(),
            &ch,
            b"\x89PNG",
            None,
            "2026-08-22T10:00:00Z",
        )
        .unwrap();
        let mut z = relire(&a);
        let mut s = String::new();
        z.by_name("META-INF/container.xml")
            .unwrap()
            .read_to_string(&mut s)
            .unwrap();
        assert!(s.contains(r#"full-path="OEBPS/content.opf""#), "{s}");
    }

    /// `dc:title` doit porter au moins un caractère : EPUBCheck le dit `ERROR` et
    /// l'ingestion s'arrête là. Or un projet neuf s'ouvre sur un titre vide, et rien ne
    /// gardait ce champ nulle part. Le refus va où est déjà celui du livre sans chapitre —
    /// et des blancs ne font pas un titre : ils passeraient la validation en laissant une
    /// ligne muette dans la bibliothèque du lecteur.
    #[test]
    fn un_livre_sans_titre_est_refuse() {
        for t in ["", "   ", "\n\t"] {
            let l = Livre {
                titre: t,
                ..livre_temoin()
            };
            let err = archive(
                &l,
                &chapitres_temoins(),
                b"\x89PNG",
                None,
                "2026-08-22T10:00:00Z",
            )
            .unwrap_err();
            assert!(err.contains("titre"), "{t:?} : {err}");
        }
    }

    /// XML 1.0 n'a aucune représentation pour un caractère de contrôle : ni le caractère
    /// nu, ni une entité numérique. Un manuscrit collé depuis Word en porte — un saut de
    /// page manuel y devient U+000C —, et la liseuse n'ouvre alors pas le chapitre du
    /// tout : EPUBCheck le dit `FATAL`. Le message doit nommer le chapitre et le
    /// caractère, sans quoi il n'y a rien à aller corriger dans le manuscrit.
    #[test]
    fn un_caractere_interdit_en_xml_fait_refuser_l_archive() {
        let mut ch = chapitres_temoins();
        ch[1].blocs = vec![Bloc::Paragraphe("Un saut\u{c} de page".into())];
        let err = archive(
            &livre_temoin(),
            &ch,
            b"\x89PNG",
            None,
            "2026-08-22T10:00:00Z",
        )
        .unwrap_err();
        assert!(err.contains("U+000C"), "{err}");
        assert!(err.contains("chapitre 2"), "{err}");
    }

    /// Le titre d'un chapitre entre dans l'archive au même titre que ses blocs — dans le
    /// `<h1>`, dans le `nav` et dans le NCX, trois fois plutôt qu'une.
    #[test]
    fn un_titre_de_chapitre_de_controle_est_refuse() {
        let mut ch = chapitres_temoins();
        ch[0].titre = "Le seuil\u{1}".into();
        let err = archive(
            &livre_temoin(),
            &ch,
            b"\x89PNG",
            None,
            "2026-08-22T10:00:00Z",
        )
        .unwrap_err();
        assert!(err.contains("U+0001"), "{err}");
        assert!(err.contains("chapitre 1"), "{err}");
    }

    /// Le refus balaie **tout** ce qui entre dans l'archive, pas seulement les chapitres :
    /// le titre, l'auteur, le genre, le copyright et la dédicace paraissent aux
    /// liminaires, dans l'OPF ou dans le NCX. Un contrôle dans l'un d'eux est tout aussi
    /// fatal, et le message doit dire lequel.
    #[test]
    fn aucun_champ_du_livre_n_echappe_au_refus() {
        let sale = "gêne\u{b}ici";
        let cas: Vec<(&str, Livre)> = vec![
            (
                "titre",
                Livre {
                    titre: sale,
                    ..livre_temoin()
                },
            ),
            (
                "page de titre",
                Livre {
                    titre_page: sale,
                    ..livre_temoin()
                },
            ),
            (
                "auteur",
                Livre {
                    auteur: sale,
                    ..livre_temoin()
                },
            ),
            (
                "genre",
                Livre {
                    genre: sale,
                    ..livre_temoin()
                },
            ),
            (
                "copyright",
                Livre {
                    copyright: sale,
                    ..livre_temoin()
                },
            ),
            (
                "dédicace",
                Livre {
                    dedicace: Some(sale),
                    ..livre_temoin()
                },
            ),
        ];
        for (champ, l) in cas {
            let err = archive(
                &l,
                &chapitres_temoins(),
                b"\x89PNG",
                None,
                "2026-08-22T10:00:00Z",
            )
            .unwrap_err();
            assert!(err.contains("U+000B"), "{champ} : {err}");
            assert!(err.contains(champ), "{champ} : {err}");
        }
    }

    /// La tabulation, le saut de ligne et le retour chariot sont légaux en XML 1.0, et le
    /// copyright en porte : un refus qui les prendrait pour des contrôles rendrait tout
    /// livre inexportable. La règle est écrite en positif pour cette raison.
    #[test]
    fn les_blancs_legaux_en_xml_ne_sont_pas_refuses() {
        let l = Livre {
            copyright: "© 2026\tIvan Pjig\r\nTous droits réservés",
            ..livre_temoin()
        };
        assert!(archive(
            &l,
            &chapitres_temoins(),
            b"\x89PNG",
            None,
            "2026-08-22T10:00:00Z"
        )
        .is_ok());
    }

    /// Le refus s'obtient sans bâtir l'archive : c'est ce qui permet à `ebook::generer`
    /// de le rendre avant vingt secondes de composition, sur le seul projet. Le message
    /// doit être le même que par `archive` — c'est le même code, et le lecteur du message
    /// n'a pas à savoir par où il est passé.
    #[test]
    fn la_verification_seule_refuse_ce_que_l_archive_refuserait() {
        let mut ch = chapitres_temoins();
        ch[1].blocs = vec![Bloc::Paragraphe("Un saut\u{c} de page".into())];
        let err = verifie(&livre_temoin(), &ch).unwrap_err();
        assert!(err.contains("U+000C"), "{err}");
        assert!(err.contains("chapitre 2"), "{err}");

        let par_l_archive = archive(
            &livre_temoin(),
            &ch,
            b"\x89PNG",
            None,
            "2026-08-22T10:00:00Z",
        )
        .unwrap_err();
        assert_eq!(err, par_l_archive);
    }

    /// Un livre sans chapitre ne produit pas d'archive : ce serait une couverture et deux
    /// pages liminaires, et le refus vaut mieux que le fichier qu'on découvrirait vide.
    #[test]
    fn un_livre_sans_chapitre_est_refuse() {
        let err = archive(
            &livre_temoin(),
            &[],
            b"\x89PNG",
            None,
            "2026-08-22T10:00:00Z",
        )
        .unwrap_err();
        assert!(err.contains("chapitre"), "{err}");
    }

    /// Le `<span class="numero">` dit le rang d'un chapitre : une préface n'en a pas,
    /// et une liseuse afficherait un numéro inventé.
    #[test]
    fn une_piece_liminaire_n_emet_pas_de_numero() {
        let p = Piece {
            sorte: Sorte::Liminaire,
            titre: "Préface".into(),
            blocs: vec![Bloc::Paragraphe("Entrez.".into())],
        };
        let x = piece_xhtml(&p);
        assert!(!x.contains(r#"class="numero""#), "{x}");
        assert!(x.contains(r#"<span class="titre">Préface</span>"#), "{x}");
        assert!(x.contains("<p>Entrez.</p>"), "{x}");
    }

    /// Toutes les pièces sont dans la table des matières : un lecteur doit pouvoir
    /// sauter à la préface comme à un chapitre.
    #[test]
    fn toutes_les_pieces_figurent_a_la_table_des_matieres() {
        let pieces = vec![
            Piece {
                sorte: Sorte::Liminaire,
                titre: "Préface".into(),
                blocs: vec![Bloc::Paragraphe("A.".into())],
            },
            Piece {
                sorte: Sorte::Partie("I".into()),
                titre: "Avant Clément".into(),
                blocs: Vec::new(),
            },
            Piece {
                sorte: Sorte::Chapitre(1),
                titre: "Un".into(),
                blocs: vec![Bloc::Paragraphe("B.".into())],
            },
        ];
        let nav = nav_xhtml(&pieces);
        assert!(nav.contains("Préface"), "{nav}");
        assert!(nav.contains("I — Avant Clément"), "{nav}");
        assert!(nav.contains("1 — Un"), "{nav}");
    }

    /// Une faute dans une pièce non numérotée doit se situer aussi bien qu'ailleurs :
    /// « chapitre 0 » enverrait l'auteur chercher au mauvais endroit.
    #[test]
    fn une_faute_dans_une_piece_est_situee_par_son_nom() {
        let pieces = vec![Piece {
            sorte: Sorte::Liminaire,
            titre: "Préface".into(),
            blocs: vec![Bloc::Paragraphe("un \u{1} de contrôle".into())],
        }];
        let err = verifie(&livre_temoin(), &pieces).unwrap_err();
        assert!(err.contains("préface"), "{err}");
    }
}
