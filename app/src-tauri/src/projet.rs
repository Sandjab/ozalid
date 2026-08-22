//! Le projet : un livre entier dans un fichier `.ozalid`.
//!
//! L'archive est auto-portante — on l'ouvre, on la déplace, on la sauvegarde comme un
//! document, et elle reste complète sur une autre machine :
//!
//! ```text
//! projet.toml     identité du livre, réglages de couverture, chemin source du manuscrit
//! manuscrit.md
//! images/         photos source de la 1ère et de la 4ème
//! polices/        la police personnelle de l'auteur, quand il en fournit une
//! envois/         les images des envois, une par dédicataire
//! ```
//!
//! Les images d'envoi sont sous `envois/`, et **pas** avec celles de la couverture :
//! `package::ecrire_images` donne un rôle aux images du projet par leur seul nom, et
//! tout ce qui ne commence pas par `quatrieme` y devient la première de couverture. Une
//! image d'envoi versée dans ce tas remplacerait la couverture, en silence.
//!
//! `projet.toml` garde la forme et l'esprit du `livre.toml` historique : dézippée,
//! l'archive reste lisible et diffable.
//!
//! Les **sorties n'y sont pas**. Un `.ozalid` ne contient que des sources ; les
//! packages sont écrits à côté. L'archive reste légère, et aucune sortie périmée ne
//! survit à un déplacement du projet.

use std::collections::BTreeMap;
use std::fs::File;
use std::io::{Read, Seek, Write};
use std::path::Path;

use serde::{Deserialize, Serialize};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

/// Version 2 : la maquette de couverture y est typée, dans le vocabulaire du moteur
/// Typst, là où la 1 conservait le bloc de réglages brut de l'atelier HTML.
pub const VERSION: u32 = 2;
const PROJET_TOML: &str = "projet.toml";
const MANUSCRIT_MD: &str = "manuscrit.md";
const IMAGES: &str = "images/";
const POLICES: &str = "polices/";
const ENVOIS: &str = "envois/";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Livre {
    pub titre: String,
    /// Titre de la page de titre, avec ses sauts de ligne voulus. Absent, le titre sert.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub titre_page: Option<String>,
    pub auteur: String,
    #[serde(default = "genre_defaut")]
    pub genre: String,
    #[serde(default)]
    pub copyright: String,
    /// Dédicace imprimée, en belle page après le copyright. Absente ou vide, aucune
    /// page n'est composée : c'est `dedicace()` qui en juge, pas ses appelants.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dedicace: Option<String>,
    /// Contrôle d'intégrité facultatif : il n'a de sens qu'au gel du manuscrit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chapitres: Option<u32>,
}

fn genre_defaut() -> String {
    "roman".into()
}

impl Livre {
    /// Un livre à remplir : tous les champs vides, sauf le genre, dont le défaut
    /// vaut mieux qu'un blanc — et c'est le même défaut que celui d'un `projet.toml`
    /// qui ne le porte pas.
    pub fn vide() -> Self {
        Self {
            titre: String::new(),
            titre_page: None,
            auteur: String::new(),
            genre: genre_defaut(),
            copyright: String::new(),
            dedicace: None,
            chapitres: None,
        }
    }

    /// Titre tel qu'il doit paraître sur la page de titre, sauts de ligne compris.
    pub fn titre_page(&self) -> &str {
        self.titre_page.as_deref().unwrap_or(&self.titre)
    }

    /// La dédicace, si le livre en porte une qui ne soit pas que du blanc.
    ///
    /// Le rognage est ici et nulle part ailleurs : une dédicace réduite à une espace
    /// ajouterait sinon deux pages au livre, donc du dos, sans que rien ne se voie à
    /// l'écran.
    pub fn dedicace(&self) -> Option<&str> {
        self.dedicace
            .as_deref()
            .map(str::trim)
            .filter(|d| !d.is_empty())
    }
}

/// Le manuscrit est **embarqué** dans l'archive : c'est ce qui rend le `.ozalid`
/// auto-portant. Son chemin d'origine est mémorisé pour que « Réimporter le
/// manuscrit » soit un bouton, et non une navigation dans un sélecteur de fichiers.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Manuscrit {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

/// La maquette de couverture du projet, dans le vocabulaire du moteur Typst.
///
/// Absente tant qu'aucune maquette n'a été choisie ni importée : composer la
/// couverture n'a alors pas d'objet, et l'interface le dit plutôt que d'en inventer
/// une par défaut.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Couverture {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maquette: Option<crate::couverture::Couverture>,
}

/// Un destinataire du livre : le prestataire chez qui on livre, son papier, et — pour
/// ceux qui ne publient ni dos ni fond perdu — ce qui a été relevé sur leur gabarit.
///
/// Les relevés naissent absents, jamais préremplis : une valeur inventée qui ressemble
/// à une mesure est pire qu'un champ vide, et le refus de composer dit quoi faire.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Destinataire {
    pub provider: String,
    pub papier: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dos_mm: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fond_perdu_mm: Option<f64>,
}

impl Destinataire {
    /// Un destinataire neuf chez ce prestataire : son papier par défaut, aucun relevé.
    pub fn pour(pr: &crate::providers::Provider) -> Self {
        Self {
            provider: pr.cle.into(),
            papier: pr.papier_defaut().cle.into(),
            dos_mm: None,
            fond_perdu_mm: None,
        }
    }
}

/// À qui le livre est destiné, et pour lequel de ces destinataires on regarde.
///
/// Une seule liste et un pointeur dessus : l'intérieur se compose pour le courant, la
/// couverture s'aperçoit à son format, la génération sert toute la liste. Le prestataire
/// n'est donc désigné qu'une fois, là où il l'a toujours été deux.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Livraison {
    pub destinataires: Vec<Destinataire>,
    /// Clé du prestataire visé — toujours l'un des destinataires ci-dessus.
    pub courant: String,
}

/// Un livre naît avec un destinataire, le premier de la table : le pointeur ne doit
/// jamais être vide, ne serait-ce que pour regarder une première de couverture, qui
/// réclame un format sans réclamer aucune composition.
impl Default for Livraison {
    fn default() -> Self {
        let pr = &crate::providers::PROVIDERS[0];
        Self {
            destinataires: vec![Destinataire::pour(pr)],
            courant: pr.cle.into(),
        }
    }
}

impl Livraison {
    /// Le destinataire visé, s'il y en a un.
    pub fn courant(&self) -> Option<&Destinataire> {
        self.destinataires
            .iter()
            .find(|d| d.provider == self.courant)
    }

    /// Remet la liste d'accord avec la table des gabarits.
    ///
    /// Un `.ozalid` peut nommer un prestataire ou un papier que la table ne porte plus,
    /// ou le même prestataire deux fois. Élaguer vaut mieux que refuser d'ouvrir : le
    /// reste du projet — le manuscrit, la maquette — est intact, et la liste des
    /// destinataires se refait en trois clics. C'est le même arbitrage que les projets
    /// récents dont le fichier a disparu.
    fn normalise(&mut self) {
        let mut vus = std::collections::BTreeSet::new();
        self.destinataires.retain_mut(|d| {
            let Some(pr) = crate::providers::provider(&d.provider) else {
                return false;
            };
            if pr.papier(&d.papier).is_none() {
                d.papier = pr.papier_defaut().cle.into();
            }
            vus.insert(d.provider.clone())
        });
        if self.destinataires.is_empty() {
            *self = Self::default();
        } else if self.courant().is_none() {
            self.courant = self.destinataires[0].provider.clone();
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entete {
    pub version: u32,
}

/// Ce que porte `projet.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metadonnees {
    pub ozalid: Entete,
    pub livre: Livre,
    #[serde(default)]
    pub manuscrit: Manuscrit,
    #[serde(default)]
    pub couverture: Couverture,
    #[serde(default)]
    pub interieur: crate::interieur::Interieur,
    /// Facultative : un `.ozalid` écrit avant elle s'ouvre sans rien dire et se voit
    /// doté du premier gabarit de la table — ce que faisait déjà le `select` en se
    /// posant sur sa première option. `VERSION` ne bouge donc pas.
    #[serde(default)]
    pub livraison: Livraison,
    /// Facultative, comme `livraison` et la dédicace avant elle : un `.ozalid` écrit
    /// avant les envois s'ouvre sans un mot, avec une liste vide. `VERSION` ne bouge
    /// donc pas.
    #[serde(default)]
    pub envois: crate::envoi::Envois,
}

/// Un projet ouvert : les métadonnées, le texte du manuscrit, les images, les polices.
#[derive(Debug, Clone)]
pub struct Projet {
    pub meta: Metadonnees,
    pub texte: String,
    /// Nom de fichier (sans `images/`) → contenu.
    pub images: BTreeMap<String, Vec<u8>>,
    /// Nom de fichier (sans `polices/`) → contenu. Une seule police y vit à la fois :
    /// c'est celle de l'auteur, et un livre n'a qu'une main.
    pub polices: BTreeMap<String, Vec<u8>>,
    /// Nom de fichier (sans `envois/`) → contenu. Une image par envoi, désignée par
    /// `Envoi::image` : c'est ce lien-là, et non l'ordre de la liste, qui garantit
    /// qu'un exemplaire ne part pas avec le mot d'un autre.
    pub images_envois: BTreeMap<String, Vec<u8>>,
}

impl Projet {
    pub fn nouveau(livre: Livre, texte: String) -> Self {
        Self {
            meta: Metadonnees {
                ozalid: Entete { version: VERSION },
                livre,
                manuscrit: Manuscrit::default(),
                couverture: Couverture::default(),
                interieur: crate::interieur::Interieur::default(),
                livraison: Livraison::default(),
                envois: crate::envoi::Envois::default(),
            },
            texte,
            images: BTreeMap::new(),
            polices: BTreeMap::new(),
            images_envois: BTreeMap::new(),
        }
    }

    /// Reprend la liste des envois saisie par l'interface, et jette ce qu'elle abandonne.
    ///
    /// Un envoi retiré emporte son image : sans cet élagage, l'archive garderait le mot
    /// manuscrit d'une personne à qui l'on n'envoie plus rien, et le `.ozalid` grossirait
    /// d'un fichier que plus rien ne nomme.
    pub fn regler_envois(&mut self, saisie: crate::envoi::Envois) -> Result<(), String> {
        self.meta.envois = self.meta.envois.reprend(saisie)?;
        self.elaguer_images_envois();
        Ok(())
    }

    /// Ne garde sous `envois/` que les images qu'un envoi nomme encore.
    fn elaguer_images_envois(&mut self) {
        let vives: Vec<String> = self
            .meta
            .envois
            .liste
            .iter()
            .filter_map(|e| e.image.clone())
            .collect();
        self.images_envois.retain(|n, _| vives.contains(n));
    }

    /// Embarque l'image d'un envoi, sous le nom que l'archive lui donnera.
    ///
    /// Le nom vient du dédicataire, pas du fichier choisi : deux photos venues du même
    /// appareil s'appellent souvent pareil, et l'une écraserait l'autre — le second
    /// exemplaire partirait avec le mot du premier.
    pub fn poser_image_envoi(
        &mut self,
        index: usize,
        ext: &str,
        octets: Vec<u8>,
    ) -> Result<(), String> {
        crate::image::dimensions(&octets)
            .ok_or("image refusée : seuls le JPEG et le PNG se composent.")?;
        let e = self
            .meta
            .envois
            .liste
            .get(index)
            .ok_or("envoi introuvable : la liste a changé.")?;
        let pris: Vec<String> = self.images_envois.keys().cloned().collect();
        let nom = crate::envoi::nom_image(&e.dedicataire, ext, &pris);
        self.meta.envois.liste[index].image = Some(nom.clone());
        self.images_envois.insert(nom, octets);
        // L'image que cet envoi portait avant n'est plus nommée par personne.
        self.elaguer_images_envois();
        Ok(())
    }

    /// La famille que déclare la police embarquée, relevée dans le fichier.
    ///
    /// Le `.ozalid` peut annoncer ce qu'il veut dans son TOML : c'est le fichier qui
    /// compose. Relever la famille à chaque ouverture est ce qui empêche un nom recopié
    /// à la main de désigner une police que Typst ne trouverait pas.
    pub fn police_personnelle(&self) -> Option<String> {
        self.polices
            .values()
            .next()
            .and_then(|o| crate::police::examine(o).ok())
            .map(|p| p.famille)
    }

    /// Embarque la police de l'auteur, et en fait la main du livre.
    ///
    /// Une seule à la fois : la précédente s'en va, comme la photo d'une face s'en va
    /// quand on en choisit une autre. Faire du livre sa main dans le même geste est ce
    /// qu'on vient de demander — importer une écriture pour ne pas s'en servir n'a pas
    /// d'usage, et l'oublier laisserait le livre dans son écriture d'avant.
    pub fn poser_police(&mut self, nom: &str, octets: Vec<u8>) -> Result<(), String> {
        let famille = crate::police::examine(&octets)?.famille;
        self.polices.clear();
        self.polices.insert(nom.to_string(), octets);
        self.meta.envois.personnelle = Some(famille.clone());
        self.meta.envois.main = crate::envoi::Main::Police { police: famille };
        Ok(())
    }

    /// Retire la police de l'auteur, et rend au livre une main qu'il sait composer.
    ///
    /// Laisser la main sur une police qui vient de partir ferait refuser la composition :
    /// exact, mais inutilement — c'est le geste de l'utilisateur qui l'a retirée, et il
    /// n'a pas demandé un livre qui ne compose plus.
    pub fn retirer_police(&mut self) {
        self.polices.clear();
        self.meta.envois.personnelle = None;
        if self.meta.envois.verifie().is_err() {
            self.meta.envois.main = crate::envoi::Main::default();
        }
    }

    pub fn ouvrir(chemin: &Path) -> Result<Self, String> {
        let f = File::open(chemin).map_err(|e| format!("{} : {e}", chemin.display()))?;
        Self::lire(f).map_err(|e| format!("{} : {e}", chemin.display()))
    }

    pub fn enregistrer(&self, chemin: &Path) -> Result<(), String> {
        let f = File::create(chemin).map_err(|e| format!("{} : {e}", chemin.display()))?;
        self.ecrire(f)
            .map_err(|e| format!("{} : {e}", chemin.display()))
    }

    fn lire<R: Read + Seek>(source: R) -> Result<Self, String> {
        let mut zip = ZipArchive::new(source).map_err(|e| format!("archive illisible : {e}"))?;

        let toml_brut = fichier(&mut zip, PROJET_TOML)?.ok_or_else(|| {
            format!("archive sans {PROJET_TOML} : ce n'est pas un projet Ozalid.")
        })?;
        let toml_brut = String::from_utf8(toml_brut)
            .map_err(|_| format!("{PROJET_TOML} n'est pas de l'UTF-8."))?;
        let mut meta: Metadonnees =
            toml::from_str(&toml_brut).map_err(|e| format!("{PROJET_TOML} : {e}"))?;
        if meta.ozalid.version > VERSION {
            return Err(format!(
                "projet en version {}, cette application lit jusqu'à la {VERSION}.",
                meta.ozalid.version
            ));
        }
        meta.livraison.normalise();

        let texte = fichier(&mut zip, MANUSCRIT_MD)?
            .ok_or_else(|| format!("archive sans {MANUSCRIT_MD}."))?;
        let texte = String::from_utf8(texte)
            .map_err(|_| format!("{MANUSCRIT_MD} n'est pas de l'UTF-8."))?;

        let mut images = BTreeMap::new();
        let mut polices = BTreeMap::new();
        let mut images_envois = BTreeMap::new();
        let noms: Vec<String> = zip.file_names().map(str::to_owned).collect();
        for nom in noms {
            for (prefixe, cible) in [
                (IMAGES, &mut images),
                (POLICES, &mut polices),
                (ENVOIS, &mut images_envois),
            ] {
                let Some(court) = nom.strip_prefix(prefixe) else {
                    continue;
                };
                if court.is_empty() {
                    continue;
                }
                if let Some(oct) = fichier(&mut zip, &nom)? {
                    cible.insert(court.to_string(), oct);
                }
            }
        }

        let mut p = Self {
            meta,
            texte,
            images,
            polices,
            images_envois,
        };
        // La famille de la police personnelle est relevée dans le fichier embarqué, et
        // non lue dans le TOML : un `.ozalid` dont on aurait retiré la police, ou dont
        // le TOML nommerait une famille que le fichier ne déclare pas, composerait
        // sinon par repli — en silence, dans une écriture que personne n'a choisie. Le
        // nom devient alors introuvable et `Envois::verifie` refuse de composer.
        p.meta.envois.personnelle = p.police_personnelle();
        Ok(p)
    }

    fn ecrire<W: Write + Seek>(&self, sortie: W) -> Result<(), String> {
        let mut zip = ZipWriter::new(sortie);
        let texte_opts =
            SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        // Les images sont déjà compressées (PNG, JPEG) : les dégonfler coûte du temps
        // pour un gain nul, parfois négatif.
        let brut_opts = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);

        let toml_brut = toml::to_string_pretty(&self.meta)
            .map_err(|e| format!("sérialisation de {PROJET_TOML} : {e}"))?;
        ajoute(&mut zip, PROJET_TOML, toml_brut.as_bytes(), texte_opts)?;
        ajoute(&mut zip, MANUSCRIT_MD, self.texte.as_bytes(), texte_opts)?;
        for (nom, oct) in &self.images {
            ajoute(&mut zip, &format!("{IMAGES}{nom}"), oct, brut_opts)?;
        }
        for (nom, oct) in &self.images_envois {
            ajoute(&mut zip, &format!("{ENVOIS}{nom}"), oct, brut_opts)?;
        }
        // Une police, elle, se dégonfle de moitié : elle est faite de tables et de
        // courbes, pas d'un flux déjà compressé.
        for (nom, oct) in &self.polices {
            ajoute(&mut zip, &format!("{POLICES}{nom}"), oct, texte_opts)?;
        }
        zip.finish().map_err(|e| format!("clôture : {e}"))?;
        Ok(())
    }
}

fn ajoute<W: Write + Seek>(
    zip: &mut ZipWriter<W>,
    nom: &str,
    contenu: &[u8],
    opts: SimpleFileOptions,
) -> Result<(), String> {
    zip.start_file(nom, opts)
        .map_err(|e| format!("{nom} : {e}"))?;
    zip.write_all(contenu).map_err(|e| format!("{nom} : {e}"))
}

fn fichier<R: Read + Seek>(zip: &mut ZipArchive<R>, nom: &str) -> Result<Option<Vec<u8>>, String> {
    match zip.by_name(nom) {
        Ok(mut f) => {
            let mut buf = Vec::with_capacity(f.size() as usize);
            f.read_to_end(&mut buf)
                .map_err(|e| format!("{nom} : {e}"))?;
            Ok(Some(buf))
        }
        Err(zip::result::ZipError::FileNotFound) => Ok(None),
        Err(e) => Err(format!("{nom} : {e}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn livre() -> Livre {
        Livre {
            titre: "Les Heures creuses".into(),
            titre_page: Some("Les Heures\ncreuses".into()),
            auteur: "Ivan Pjig".into(),
            genre: "roman".into(),
            copyright: "© Ivan Pjig, 2026.\nTous droits réservés.".into(),
            dedicace: None,
            chapitres: Some(64),
        }
    }

    fn aller_retour(p: &Projet) -> Projet {
        let mut buf = Vec::new();
        p.ecrire(Cursor::new(&mut buf)).unwrap();
        Projet::lire(Cursor::new(buf)).unwrap()
    }

    /// Le `.ozalid` est le document de l'utilisateur : ce qui y entre doit en ressortir
    /// identique, sauts de ligne du titre et réglages de couverture compris. Une perte
    /// silencieuse ici, c'est une maquette à refaire.
    ///
    /// La police posée est volontairement `Cardo`, pas `EB Garamond` : avec le défaut,
    /// le test passerait même si la police n'était jamais écrite dans l'archive, la
    /// relecture la reconstruisant de toute façon par `#[serde(default)]`.
    #[test]
    fn un_projet_complet_survit_a_l_aller_retour() {
        let mut p = Projet::nouveau(livre(), "## 01 - Un\n\nTexte.\n".into());
        p.meta.manuscrit.source = Some("/travail/roman.md".into());
        p.meta.interieur.police = "Cardo".into();
        p.meta.livre.dedicace = Some("À M., qui a tenu la lampe.".into());
        let mut maquette = crate::maquettes::blanche();
        maquette.pad_x = 16.5;
        maquette.titre.taille = 9.25;
        maquette.pastille.texte = "collection « Ozalid »".into();
        p.meta.couverture.maquette = Some(maquette);
        p.images
            .insert("couverture.jpg".into(), vec![0xFF, 0xD8, 0xFF]);

        let r = aller_retour(&p);
        assert_eq!(r.meta.livre.titre_page(), "Les Heures\ncreuses");
        assert_eq!(r.meta.livre.chapitres, Some(64));
        assert_eq!(r.meta.livre.copyright, p.meta.livre.copyright);
        assert_eq!(
            r.meta.manuscrit.source.as_deref(),
            Some("/travail/roman.md")
        );
        assert_eq!(r.meta.interieur.police, "Cardo");
        assert_eq!(r.meta.livre.dedicace(), Some("À M., qui a tenu la lampe."));
        assert_eq!(r.texte, p.texte);
        assert_eq!(r.images["couverture.jpg"], vec![0xFF, 0xD8, 0xFF]);

        // La maquette entière survit, y compris les valeurs affinées à la main : la
        // reperdre obligerait à refaire le réglage fin de la couverture.
        let m = r.meta.couverture.maquette.unwrap();
        assert_eq!(m.mode, crate::couverture::Mode::Typo);
        assert_eq!(m.pad_x, 16.5);
        assert_eq!(m.titre.taille, 9.25);
        assert_eq!(m.titre.casse, crate::couverture::Casse::Capitales);
        assert!(m.cadre.actif);
        assert_eq!(m.cadre.filet2_couleur, "#c00000");
        assert_eq!(m.pastille.texte, "collection « Ozalid »");
    }

    /// Un `.ozalid` écrit avant que la police ne soit réglable doit s'ouvrir, pas être
    /// refusé — même principe que le dos rendu réglable élément par élément.
    #[test]
    fn un_projet_sans_section_interieur_prend_la_police_par_defaut() {
        let toml = r#"
[ozalid]
version = 1

[livre]
titre = "Les Heures creuses"
auteur = "Ivan Pjig"
"#;
        let m: Metadonnees = toml::from_str(toml).expect("projet sans [interieur] refusé");
        assert_eq!(m.interieur.police, "EB Garamond");
    }

    /// Le lot 3 ajoute `[livraison]` sans monter `VERSION` : les `.ozalid` déjà écrits
    /// doivent s'ouvrir et se retrouver visés sur le premier gabarit de la table, comme
    /// le `select` s'y posait.
    #[test]
    fn un_projet_sans_section_livraison_prend_le_premier_gabarit() {
        let toml = r#"
[ozalid]
version = 2

[livre]
titre = "Les Heures creuses"
auteur = "Ivan Pjig"
"#;
        let mut m: Metadonnees = toml::from_str(toml).expect("projet sans [livraison] refusé");
        m.livraison.normalise();
        let attendu = crate::providers::PROVIDERS[0].cle;
        assert_eq!(m.livraison.courant, attendu);
        assert_eq!(m.livraison.destinataires.len(), 1);
        assert_eq!(m.livraison.destinataires[0].provider, attendu);
    }

    /// La liste des destinataires est du travail de l'utilisateur au même titre que la
    /// maquette : la reperdre, c'est refaire ses relevés de gabarit à la main.
    #[test]
    fn la_liste_des_destinataires_survit_a_l_aller_retour() {
        let mut p = Projet::nouveau(livre(), "## 01\n\nA.\n".into());
        p.meta.livraison = Livraison {
            destinataires: vec![
                Destinataire::pour(crate::providers::provider("lulu").unwrap()),
                Destinataire {
                    provider: "coollibri-148x210".into(),
                    papier: "mesure".into(),
                    dos_mm: Some(18.4),
                    fond_perdu_mm: Some(3.0),
                },
            ],
            courant: "coollibri-148x210".into(),
        };

        let r = aller_retour(&p);
        assert_eq!(r.meta.livraison.courant, "coollibri-148x210");
        let d = r.meta.livraison.courant().expect("courant perdu");
        assert_eq!(d.dos_mm, Some(18.4));
        assert_eq!(d.fond_perdu_mm, Some(3.0));
        assert_eq!(r.meta.livraison.destinataires[0].provider, "lulu");
    }

    /// Un prestataire retiré de la table, un papier renommé, le même prestataire deux
    /// fois : le projet s'ouvre quand même. Refuser ferait perdre le manuscrit et la
    /// maquette pour une liste qui se refait en trois clics.
    #[test]
    fn une_livraison_incoherente_est_elaguee_plutot_que_refusee() {
        let mut l = Livraison {
            destinataires: vec![
                Destinataire {
                    provider: "prestataire-disparu".into(),
                    papier: "standard".into(),
                    dos_mm: None,
                    fond_perdu_mm: None,
                },
                Destinataire {
                    provider: "lulu".into(),
                    papier: "papier-renomme".into(),
                    dos_mm: None,
                    fond_perdu_mm: None,
                },
                Destinataire::pour(crate::providers::provider("lulu").unwrap()),
            ],
            courant: "prestataire-disparu".into(),
        };
        l.normalise();

        assert_eq!(l.destinataires.len(), 1, "doublon ou inconnu conservé");
        assert_eq!(l.destinataires[0].provider, "lulu");
        assert_eq!(l.destinataires[0].papier, "standard");
        assert_eq!(l.courant, "lulu", "le pointeur désigne un absent");
    }

    /// Le pointeur ne peut pas être vide : sans lui, même regarder une première de
    /// couverture est impossible, faute de format.
    #[test]
    fn une_livraison_videe_reprend_le_premier_gabarit() {
        let mut l = Livraison {
            destinataires: vec![],
            courant: String::new(),
        };
        l.normalise();
        assert_eq!(l.destinataires.len(), 1);
        assert!(l.courant().is_some());
    }

    #[test]
    fn un_projet_sans_couverture_ni_images_reste_valide() {
        let p = Projet::nouveau(livre(), "## 01\n\nA.\n".into());
        let r = aller_retour(&p);
        assert!(r.meta.couverture.maquette.is_none());
        assert!(r.images.is_empty());
    }

    /// Ouvrir autre chose qu'un projet doit le dire clairement, plutôt que d'échouer
    /// plus loin sur un manuscrit vide.
    #[test]
    fn une_archive_etrangere_est_refusee_explicitement() {
        let mut buf = Vec::new();
        {
            let mut zip = ZipWriter::new(Cursor::new(&mut buf));
            zip.start_file("autre.txt", SimpleFileOptions::default())
                .unwrap();
            zip.write_all(b"rien a voir").unwrap();
            zip.finish().unwrap();
        }
        let err = Projet::lire(Cursor::new(buf)).unwrap_err();
        assert!(err.contains("projet Ozalid"), "{err}");
    }

    #[test]
    fn un_fichier_qui_n_est_pas_une_archive_est_refuse() {
        let err = Projet::lire(Cursor::new(b"pas un zip".to_vec())).unwrap_err();
        assert!(err.contains("archive illisible"), "{err}");
    }

    /// Un projet écrit par une version future doit être refusé, pas lu de travers :
    /// un champ ignoré silencieusement se traduirait par une planche fausse.
    #[test]
    fn un_projet_plus_recent_que_l_application_est_refuse() {
        let mut p = Projet::nouveau(livre(), "## 01\n\nA.\n".into());
        p.meta.ozalid.version = VERSION + 1;
        let mut buf = Vec::new();
        p.ecrire(Cursor::new(&mut buf)).unwrap();
        let err = Projet::lire(Cursor::new(buf)).unwrap_err();
        assert!(err.contains("version"), "{err}");
    }

    /// L'archive ne doit contenir que des sources : y glisser des sorties la ferait
    /// gonfler et transporter des PDF périmés.
    #[test]
    fn l_archive_ne_contient_que_des_sources() {
        let mut p = Projet::nouveau(livre(), "## 01\n\nA.\n".into());
        p.images.insert("quatrieme.png".into(), vec![1, 2, 3]);
        let mut buf = Vec::new();
        p.ecrire(Cursor::new(&mut buf)).unwrap();

        let zip = ZipArchive::new(Cursor::new(buf)).unwrap();
        let mut noms: Vec<&str> = zip.file_names().collect();
        noms.sort_unstable();
        assert_eq!(
            noms,
            vec!["images/quatrieme.png", "manuscrit.md", "projet.toml"]
        );
    }

    /// Un `.ozalid` écrit avant la dédicace s'ouvre sans un mot : le champ est
    /// facultatif, `VERSION` n'a donc pas bougé. Même principe que la police et que
    /// `[livraison]` avant elle.
    #[test]
    fn un_projet_sans_champ_dedicace_se_relit() {
        let toml = r#"
[ozalid]
version = 2

[livre]
titre = "Les Heures creuses"
auteur = "Ivan Pjig"
"#;
        let m: Metadonnees = toml::from_str(toml).expect("projet sans dédicace refusé");
        assert_eq!(m.livre.dedicace, None);
    }

    /// Un `.ozalid` écrit avant les envois s'ouvre sans un mot : troisième section
    /// facultative après `[interieur]` et `[livraison]`, et `VERSION` n'a pas bougé.
    #[test]
    fn un_projet_sans_section_envois_se_relit() {
        let toml = r#"
[ozalid]
version = 2

[livre]
titre = "Les Heures creuses"
auteur = "Ivan Pjig"
"#;
        let m: Metadonnees = toml::from_str(toml).expect("projet sans [envois] refusé");
        assert!(m.envois.liste.is_empty());
        assert!(
            m.envois.verifie().is_ok(),
            "la main par défaut doit être valide"
        );
    }

    /// Les envois sont du travail de l'utilisateur au même titre que la maquette : les
    /// reperdre, c'est réécrire tous les mots à la main.
    #[test]
    fn les_envois_survivent_a_l_aller_retour() {
        let mut p = Projet::nouveau(livre(), "## 01\n\nA.\n".into());
        p.meta.envois.liste = vec![crate::envoi::Envoi {
            dedicataire: "Léa".into(),
            contenu: "À Léa, qui a lu la première version.".into(),
            image: None,
        }];

        let r = aller_retour(&p);
        assert_eq!(r.meta.envois.liste.len(), 1);
        assert_eq!(r.meta.envois.liste[0].dedicataire, "Léa");
        assert_eq!(
            r.meta.envois.liste[0].contenu,
            "À Léa, qui a lu la première version."
        );
    }

    /// Une police de la maison qui n'est **pas** une main : sa famille ne figure pas
    /// dans `MAINS`. C'est ce qui compte ici — avec Caveat, dont la famille est déjà la
    /// main par défaut, aucun de ces tests ne pourrait échouer.
    const FOURNIE: &[u8] = include_bytes!("../fonts/EBGaramond[wght].ttf");
    const FAMILLE: &str = "EB Garamond";

    /// Le `.ozalid` doit être auto-portant : un projet composé avec l'écriture de son
    /// auteur doit se recomposer à l'identique sur une autre machine, où cette police
    /// n'est installée nulle part. Les octets voyagent donc dans l'archive, et la main
    /// avec.
    #[test]
    fn la_police_personnelle_voyage_dans_l_archive() {
        let mut p = Projet::nouveau(livre(), "## 01\n\nA.\n".into());
        p.poser_police("main.ttf", FOURNIE.to_vec()).unwrap();
        assert_eq!(
            p.meta.envois.main,
            crate::envoi::Main::Police {
                police: FAMILLE.into()
            }
        );

        let r = aller_retour(&p);
        assert_eq!(r.polices["main.ttf"], FOURNIE);
        assert_eq!(r.meta.envois.personnelle.as_deref(), Some(FAMILLE));
        assert!(r.meta.envois.verifie().is_ok(), "main perdue à l'ouverture");
    }

    /// Ce n'est pas le TOML qui compose, c'est le fichier. Un `.ozalid` qui annonce une
    /// famille que son archive ne porte pas — police retirée, TOML recopié à la main —
    /// doit se retrouver sans police personnelle, et sa main être refusée : composer par
    /// repli enverrait au dédicataire un mot dans une écriture que personne n'a choisie.
    #[test]
    fn une_police_annoncee_mais_absente_ne_compose_pas() {
        let mut p = Projet::nouveau(livre(), "## 01\n\nA.\n".into());
        p.meta.envois.personnelle = Some("Ma Main".into());
        p.meta.envois.main = crate::envoi::Main::Police {
            police: "Ma Main".into(),
        };

        let r = aller_retour(&p);
        assert_eq!(r.meta.envois.personnelle, None);
        let err = r.meta.envois.verifie().unwrap_err();
        assert!(err.contains("Ma Main"), "{err}");
    }

    /// Une seule police à la fois : la précédente s'en va, comme la photo d'une face
    /// quand on en choisit une autre. Deux polices dans l'archive laisseraient l'ordre
    /// alphabétique décider laquelle écrit les envois.
    #[test]
    fn une_police_choisie_remplace_la_precedente() {
        let mut p = Projet::nouveau(livre(), "## 01\n\nA.\n".into());
        p.poser_police("premiere.ttf", FOURNIE.to_vec()).unwrap();
        p.poser_police("seconde.ttf", FOURNIE.to_vec()).unwrap();
        assert_eq!(p.polices.keys().collect::<Vec<_>>(), ["seconde.ttf"]);
    }

    /// Retirer la police rend au livre une main qu'il sait composer : la laisser sur une
    /// écriture qui vient de partir ferait refuser la composition, exactement, mais pour
    /// rien — l'utilisateur n'a pas demandé un livre qui ne compose plus.
    #[test]
    fn retirer_la_police_rend_au_livre_une_main_composable() {
        let mut p = Projet::nouveau(livre(), "## 01\n\nA.\n".into());
        p.poser_police("main.ttf", FOURNIE.to_vec()).unwrap();
        p.retirer_police();
        assert!(p.polices.is_empty());
        assert_eq!(p.meta.envois.personnelle, None);
        assert!(p.meta.envois.verifie().is_ok(), "le livre ne compose plus");
    }

    /// La main embarquée que l'auteur a choisie ne doit pas être emportée par le retrait
    /// de sa police personnelle : elle ne dépendait pas d'elle.
    #[test]
    fn retirer_la_police_ne_touche_pas_a_une_main_embarquee() {
        let mut p = Projet::nouveau(livre(), "## 01\n\nA.\n".into());
        p.poser_police("main.ttf", FOURNIE.to_vec()).unwrap();
        let choisie = crate::envoi::Main::Police {
            police: crate::envoi::MAINS[1].into(),
        };
        p.meta.envois.main = choisie.clone();
        p.retirer_police();
        assert_eq!(p.meta.envois.main, choisie);
    }

    /// Un manuscrit renommé en `.ttf` n'est pas une écriture : le refus est au moment du
    /// choix, seul endroit où il peut encore être corrigé, et l'archive reste intacte.
    #[test]
    fn un_fichier_qui_n_est_pas_une_police_ne_devient_pas_la_main() {
        let mut p = Projet::nouveau(livre(), "## 01\n\nA.\n".into());
        let avant = p.meta.envois.main.clone();
        let err = p.poser_police("faux.ttf", b"## 01 - Un\n\nTexte.".to_vec());
        assert!(err.is_err());
        assert!(p.polices.is_empty(), "l'archive a gardé le faux fichier");
        assert_eq!(p.meta.envois.main, avant);
    }

    /// Un PNG réduit à ce que `image::dimensions` sait lire : sa signature et son IHDR.
    fn png(largeur: u32) -> Vec<u8> {
        let mut p = b"\x89PNG\r\n\x1a\n".to_vec();
        p.extend(13u32.to_be_bytes());
        p.extend(b"IHDR");
        p.extend(largeur.to_be_bytes());
        p.extend(400u32.to_be_bytes());
        p.extend([8, 6, 0, 0, 0]);
        p
    }

    fn avec_envois(qui: &[&str]) -> Projet {
        let mut p = Projet::nouveau(livre(), "## 01\n\nA.\n".into());
        p.meta.envois.main = crate::envoi::Main::Image;
        p.meta.envois.liste = qui
            .iter()
            .map(|d| crate::envoi::Envoi {
                dedicataire: (*d).into(),
                contenu: String::new(),
                image: None,
            })
            .collect();
        p
    }

    /// L'image écrite à la main est une source au même titre que le manuscrit : elle
    /// voyage dans l'archive, et le nom qu'elle y prend est celui que la source Typst
    /// écrira. Le perdre, c'est un envoi qui ne compose plus.
    #[test]
    fn une_image_d_envoi_voyage_dans_l_archive() {
        let mut p = avec_envois(&["Léa"]);
        p.poser_image_envoi(0, "png", png(300)).unwrap();

        let r = aller_retour(&p);
        assert_eq!(r.meta.envois.liste[0].image.as_deref(), Some("Léa.png"));
        assert_eq!(r.images_envois["Léa.png"], png(300));
    }

    /// Le nom vient du dédicataire, jamais du fichier choisi : deux photos du même
    /// appareil s'appellent pareil, et la seconde écraserait la première — le second
    /// exemplaire partirait avec le mot du premier.
    #[test]
    fn deux_envois_ne_partagent_jamais_leur_image() {
        // Deux personnes du même prénom : le cas où le nom seul ne suffit pas.
        let mut p = avec_envois(&["Léa", "Léa"]);
        p.poser_image_envoi(0, "png", png(300)).unwrap();
        p.poser_image_envoi(1, "png", png(500)).unwrap();

        assert_eq!(p.meta.envois.liste[0].image.as_deref(), Some("Léa.png"));
        assert_eq!(p.meta.envois.liste[1].image.as_deref(), Some("Léa-2.png"));
        assert_eq!(p.images_envois["Léa.png"], png(300), "images échangées");
        assert_eq!(p.images_envois["Léa-2.png"], png(500));
    }

    /// Une image remplacée ne laisse rien derrière elle : l'archive garderait sinon des
    /// mots que plus aucun envoi ne nomme, et grossirait d'une photo par essai.
    #[test]
    fn une_image_remplacee_ne_laisse_rien_dans_l_archive() {
        let mut p = avec_envois(&["Léa"]);
        p.poser_image_envoi(0, "png", png(300)).unwrap();
        p.poser_image_envoi(0, "jpg", png(500)).unwrap();

        assert_eq!(p.images_envois.len(), 1, "l'ancienne image est restée");
        assert_eq!(p.meta.envois.liste[0].image.as_deref(), Some("Léa.jpg"));
    }

    /// Un envoi retiré emporte son image : c'est le mot écrit pour quelqu'un à qui l'on
    /// n'envoie plus rien.
    #[test]
    fn un_envoi_retire_emporte_son_image() {
        let mut p = avec_envois(&["Léa", "Marie"]);
        p.poser_image_envoi(0, "png", png(300)).unwrap();
        p.poser_image_envoi(1, "png", png(500)).unwrap();

        let restants = crate::envoi::Envois {
            liste: vec![p.meta.envois.liste[1].clone()],
            ..p.meta.envois.clone()
        };
        p.regler_envois(restants).unwrap();

        assert_eq!(p.images_envois.keys().collect::<Vec<_>>(), ["Marie.png"]);
    }

    /// Ni PNG ni JPEG, Typst ne saurait pas la poser : le refus est au moment du choix,
    /// seul endroit où il peut encore être corrigé.
    #[test]
    fn une_image_d_envoi_illisible_est_refusee() {
        let mut p = avec_envois(&["Léa"]);
        assert!(p
            .poser_image_envoi(0, "png", b"pas une image".to_vec())
            .is_err());
        assert!(p.images_envois.is_empty());
        assert_eq!(p.meta.envois.liste[0].image, None);
    }

    /// Les images d'envoi ne rejoignent pas celles de la couverture : là-bas, tout ce
    /// qui ne s'appelle pas `quatrieme…` **devient** la première de couverture. Un mot
    /// manuscrit versé dans ce tas remplacerait la couverture du livre, en silence.
    #[test]
    fn les_images_d_envoi_ne_se_melent_pas_a_celles_de_la_couverture() {
        let mut p = avec_envois(&["Léa"]);
        p.images
            .insert("couverture.jpg".into(), vec![0xFF, 0xD8, 0xFF]);
        p.poser_image_envoi(0, "png", png(300)).unwrap();

        let mut buf = Vec::new();
        p.ecrire(Cursor::new(&mut buf)).unwrap();
        let zip = ZipArchive::new(Cursor::new(buf)).unwrap();
        let mut noms: Vec<&str> = zip.file_names().collect();
        noms.sort_unstable();
        assert_eq!(
            noms,
            vec![
                "envois/Léa.png",
                "images/couverture.jpg",
                "manuscrit.md",
                "projet.toml"
            ]
        );
    }

    /// Une dédicace faite d'espaces ne doit pas coûter deux pages et du dos : c'est
    /// l'accesseur qui tranche, une seule fois, pour tous ses appelants.
    #[test]
    fn une_dedicace_de_blanc_equivaut_a_pas_de_dedicace() {
        let mut l = livre();
        assert_eq!(l.dedicace(), None);
        l.dedicace = Some("   \n  ".into());
        assert_eq!(l.dedicace(), None, "du blanc a été pris pour une dédicace");
        l.dedicace = Some("  À M.  ".into());
        assert_eq!(l.dedicace(), Some("À M."), "les bords doivent être rognés");
    }
}
