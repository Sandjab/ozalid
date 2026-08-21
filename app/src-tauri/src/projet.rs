//! Le projet : un livre entier dans un fichier `.ozalid`.
//!
//! L'archive est auto-portante — on l'ouvre, on la déplace, on la sauvegarde comme un
//! document, et elle reste complète sur une autre machine :
//!
//! ```text
//! projet.toml     identité du livre, réglages de couverture, chemin source du manuscrit
//! manuscrit.md
//! images/         photos source de la 1ère et de la 4ème
//! ```
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
}

/// Un projet ouvert : les métadonnées, le texte du manuscrit, les images.
#[derive(Debug, Clone)]
pub struct Projet {
    pub meta: Metadonnees,
    pub texte: String,
    /// Nom de fichier (sans `images/`) → contenu.
    pub images: BTreeMap<String, Vec<u8>>,
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
            },
            texte,
            images: BTreeMap::new(),
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
        let noms: Vec<String> = zip.file_names().map(str::to_owned).collect();
        for nom in noms {
            if let Some(court) = nom.strip_prefix(IMAGES) {
                if court.is_empty() {
                    continue;
                }
                if let Some(oct) = fichier(&mut zip, &nom)? {
                    images.insert(court.to_string(), oct);
                }
            }
        }

        Ok(Self {
            meta,
            texte,
            images,
        })
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
