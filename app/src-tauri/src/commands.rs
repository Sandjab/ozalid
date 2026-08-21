//! Commandes exposées à l'interface. Aucune logique métier ici : elles orchestrent
//! les modules, tiennent le projet ouvert et traduisent les erreurs en messages
//! affichables.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::couverture::{self, Couverture, Ressource};
use crate::epreuve;
use crate::import;
use crate::interieur::{self, Interieur, Reglage};
use crate::manuscrit;
use crate::maquettes;
use crate::package;
use crate::planche;
use crate::projet::{Livre, Projet};
use crate::providers::{self, Provider};
use crate::typst::Typst;

/// Le projet ouvert. Un seul à la fois : c'est un éditeur de document, pas une
/// bibliothèque. `chemin` est absent tant que le projet n'a pas été enregistré.
#[derive(Default)]
pub struct Atelier {
    ouvert: Mutex<Option<Ouvert>>,
}

struct Ouvert {
    chemin: Option<PathBuf>,
    projet: Projet,
    /// Vrai dès qu'une commande a touché au projet sans qu'il ait été réécrit.
    /// C'est lui, et lui seul, qui décide si fermer perd du travail.
    modifie: bool,
}

/// Vue d'un prestataire pour l'interface.
#[derive(Serialize)]
pub struct ProviderVue {
    cle: String,
    libelle: String,
    largeur: f64,
    hauteur: f64,
    fond_perdu: Option<f64>,
    /// Vrai quand le prestataire publie de quoi calculer le dos. Faux, l'interface
    /// réclame un relevé plutôt que de laisser croire à un chiffre.
    dos_publie: bool,
    papiers: Vec<PapierVue>,
}

#[derive(Serialize)]
pub struct PapierVue {
    cle: String,
    libelle: String,
}

impl From<&Provider> for ProviderVue {
    fn from(p: &Provider) -> Self {
        Self {
            cle: p.cle.into(),
            libelle: p.libelle.into(),
            largeur: p.format.0,
            hauteur: p.format.1,
            fond_perdu: p.fond_perdu,
            // Une pagination quelconque suffit à savoir si une formule existe.
            dos_publie: p.papier_defaut().dos.mm(100).is_some(),
            papiers: p
                .papiers
                .iter()
                .map(|pa| PapierVue {
                    cle: pa.cle.into(),
                    libelle: pa.libelle.into(),
                })
                .collect(),
        }
    }
}

/// Ce que l'interface affiche d'un projet ouvert.
#[derive(Serialize)]
pub struct ProjetVue {
    pub chemin: Option<String>,
    pub livre: Livre,
    pub manuscrit_source: Option<String>,
    /// Chapitres réellement trouvés dans le manuscrit embarqué.
    pub chapitres_trouves: u32,
    pub mots: u32,
    /// Vrai quand le projet ne porte aucun texte. Distinct de « zéro chapitre » :
    /// un manuscrit présent mais non composable en trouve zéro aussi, et ce n'est
    /// pas la même chose à corriger.
    pub manuscrit_absent: bool,
    /// Modifications non enregistrées.
    pub modifie: bool,
    /// Maquette de couverture du projet, si le projet en porte une.
    pub couverture: Option<Couverture>,
    pub couverture_importee: bool,
    pub images: Vec<String>,
    pub interieur: Interieur,
}

#[derive(Serialize)]
pub struct Composition {
    pub pages: u32,
    pub gouttiere: f64,
    pub blanche: bool,
    pub chapitres: u32,
    /// Épaisseur du dos en mm, ou `null` chez un prestataire à gabarit. C'est cette
    /// valeur qui alimentera la planche : elle n'est jamais ressaisie.
    pub dos: Option<f64>,
    pub pdf: String,
}

#[tauri::command]
pub fn providers_liste() -> Vec<ProviderVue> {
    providers::PROVIDERS.iter().map(ProviderVue::from).collect()
}

/// Importe un répertoire de travail de l'ancienne chaîne (son `livre.toml`).
/// Le projet devient le projet ouvert, sans être enregistré : l'utilisateur choisit
/// où poser le `.ozalid`.
#[tauri::command]
pub fn projet_importer(livre_toml: String, atelier: State<Atelier>) -> Result<ProjetVue, String> {
    let projet = import::depuis_livre_toml(Path::new(&livre_toml))?;
    poser(&atelier, None, projet, true)
}

#[tauri::command]
pub fn projet_ouvrir(chemin: String, atelier: State<Atelier>) -> Result<ProjetVue, String> {
    let c = PathBuf::from(&chemin);
    let projet = Projet::ouvrir(&c)?;
    poser(&atelier, Some(c), projet, false)
}

#[tauri::command]
pub fn projet_enregistrer(chemin: String, atelier: State<Atelier>) -> Result<ProjetVue, String> {
    let mut garde = atelier.ouvert.lock().unwrap();
    let o = garde.as_mut().ok_or_else(aucun_projet)?;
    let c = PathBuf::from(&chemin);
    o.projet.enregistrer(&c)?;
    o.chemin = Some(c);
    vue_enregistree(o)
}

/// Relit le manuscrit à sa source d'origine et remplace la copie embarquée.
///
/// Le `.ozalid` est auto-portant : le manuscrit y est copié, donc une correction faite
/// dans l'éditeur de texte n'y entre que par ce geste. Le chemin d'origine est
/// mémorisé pour que ce soit un bouton et non une navigation.
#[tauri::command]
pub fn manuscrit_reimporter(atelier: State<Atelier>) -> Result<ProjetVue, String> {
    let mut garde = atelier.ouvert.lock().unwrap();
    let o = garde.as_mut().ok_or_else(aucun_projet)?;
    let source = o.projet.meta.manuscrit.source.clone().ok_or_else(|| {
        "ce projet ne mémorise aucune source de manuscrit — en choisir une.".to_string()
    })?;
    o.projet.texte = std::fs::read_to_string(&source)
        .map_err(|e| format!("manuscrit introuvable ({source}) : {e}"))?;
    vue_modifiee(o)
}

/// Remplace le manuscrit par un fichier choisi, et mémorise son chemin.
#[tauri::command]
pub fn manuscrit_choisir(chemin: String, atelier: State<Atelier>) -> Result<ProjetVue, String> {
    let mut garde = atelier.ouvert.lock().unwrap();
    let o = garde.as_mut().ok_or_else(aucun_projet)?;
    o.projet.texte =
        std::fs::read_to_string(&chemin).map_err(|e| format!("manuscrit illisible : {e}"))?;
    o.projet.meta.manuscrit.source = Some(chemin);
    vue_modifiee(o)
}

#[tauri::command]
pub fn livre_modifier(livre: Livre, atelier: State<Atelier>) -> Result<ProjetVue, String> {
    let mut garde = atelier.ouvert.lock().unwrap();
    let o = garde.as_mut().ok_or_else(aucun_projet)?;
    o.projet.meta.livre = livre;
    vue_modifiee(o)
}

#[tauri::command]
pub fn polices_texte_liste() -> Vec<&'static str> {
    interieur::POLICES_TEXTE.to_vec()
}

#[tauri::command]
pub fn interieur_modifier(
    interieur: Interieur,
    atelier: State<Atelier>,
) -> Result<ProjetVue, String> {
    interieur.verifie()?;
    let mut garde = atelier.ouvert.lock().unwrap();
    let o = garde.as_mut().ok_or_else(aucun_projet)?;
    o.projet.meta.interieur = interieur;
    vue_modifiee(o)
}

/// Compose l'intérieur du projet ouvert et rend le compte de pages avec le dos qui
/// en découle.
#[tauri::command]
pub fn composer(
    provider_cle: String,
    papier_cle: Option<String>,
    atelier: State<Atelier>,
) -> Result<Composition, String> {
    let garde = atelier.ouvert.lock().unwrap();
    let o = garde.as_ref().ok_or_else(aucun_projet)?;

    let pr = providers::provider(&provider_cle)
        .ok_or_else(|| format!("prestataire inconnu : {provider_cle}"))?;
    let papier = match papier_cle.as_deref() {
        Some(c) => pr
            .papier(c)
            .ok_or_else(|| format!("papier inconnu chez {} : {c}", pr.cle))?,
        None => pr.papier_defaut(),
    };

    let livre = &o.projet.meta.livre;
    let int = &o.projet.meta.interieur;
    // `interieur::source` interpole la police sans échappement : la validation est ici.
    int.verifie()?;
    let chapitres = manuscrit::decoupe(&o.projet.texte, livre.chapitres)?;

    let dossier = sorties_dossier(o, pr.cle)?;
    std::fs::create_dir_all(&dossier).map_err(|e| {
        format!(
            "répertoire de sortie inutilisable ({}) : {e}",
            dossier.display()
        )
    })?;

    let typst = typst()?;
    let src = dossier.join(format!("interieur-{}.typ", pr.cle));

    // La convergence ne mesure que le compte de pages : aucun PDF n'est produit tant
    // que le réglage n'est pas stable.
    let r = interieur::converge(pr, |reglage| {
        ecrire(
            &src,
            &interieur::source(livre, int, pr, reglage, &chapitres),
        )?;
        typst.pages(&src)
    })?;

    let reglage = Reglage {
        gouttiere: r.gouttiere,
        blanche: r.blanche,
    };
    ecrire(
        &src,
        &interieur::source(livre, int, pr, &reglage, &chapitres),
    )?;
    let pdf = dossier.join(format!("interieur-{}.pdf", pr.cle));
    typst.compile(&src, &pdf)?;

    Ok(Composition {
        pages: r.pages,
        gouttiere: r.gouttiere,
        blanche: r.blanche,
        chapitres: chapitres.len() as u32,
        dos: papier.dos.mm(r.pages),
        pdf: pdf.to_string_lossy().into_owned(),
    })
}

/// Tire l'épreuve de relecture à la racine des sorties : elle ne vise aucun éditeur,
/// elle ne descend donc pas dans un répertoire de prestataire.
#[tauri::command]
pub fn epreuve_tirer(corps_pt: f64, atelier: State<Atelier>) -> Result<String, String> {
    let garde = atelier.ouvert.lock().unwrap();
    let o = garde.as_ref().ok_or_else(aucun_projet)?;
    let livre = &o.projet.meta.livre;
    let int = &o.projet.meta.interieur;
    // `epreuve::source` interpole la police sans échappement : la validation est ici.
    int.verifie()?;
    let chapitres = manuscrit::decoupe(&o.projet.texte, livre.chapitres)?;

    let dossier = sorties_racine(o)?;
    std::fs::create_dir_all(&dossier).map_err(|e| {
        format!(
            "répertoire de sortie inutilisable ({}) : {e}",
            dossier.display()
        )
    })?;
    let src = dossier.join("epreuve.typ");
    ecrire(&src, &epreuve::source(livre, int, &chapitres, corps_pt))?;
    let pdf = dossier.join("epreuve.pdf");
    typst()?.compile(&src, &pdf)?;
    Ok(pdf.to_string_lossy().into_owned())
}

/* ---------- couverture ---------- */

#[derive(Serialize)]
pub struct MaquetteVue {
    cle: String,
    libelle: String,
}

#[tauri::command]
pub fn maquettes_liste() -> Vec<MaquetteVue> {
    maquettes::toutes()
        .into_iter()
        .map(|(cle, libelle, _)| MaquetteVue {
            cle: cle.into(),
            libelle: libelle.into(),
        })
        .collect()
}

#[tauri::command]
pub fn polices_liste() -> Vec<&'static str> {
    couverture::POLICES.to_vec()
}

/// Charge une maquette de départ. Elle remplace la mise en page, jamais l'identité du
/// livre : le titre et l'auteur imprimés restent ceux du projet.
#[tauri::command]
pub fn maquette_choisir(cle: String, atelier: State<Atelier>) -> Result<ProjetVue, String> {
    let m = maquettes::par_cle(&cle).ok_or_else(|| format!("maquette inconnue : {cle}"))?;
    let mut garde = atelier.ouvert.lock().unwrap();
    let o = garde.as_mut().ok_or_else(aucun_projet)?;
    o.projet.meta.couverture.maquette = Some(m);
    vue_modifiee(o)
}

#[tauri::command]
pub fn couverture_modifier(
    couverture: Couverture,
    atelier: State<Atelier>,
) -> Result<ProjetVue, String> {
    let mut garde = atelier.ouvert.lock().unwrap();
    let o = garde.as_mut().ok_or_else(aucun_projet)?;
    o.projet.meta.couverture.maquette = Some(couverture);
    vue_modifiee(o)
}

/// Nom sous lequel une image entre dans le projet, selon la face qu'elle sert.
///
/// Le nom porte le rôle — c'est ainsi que la composition le lit — et l'extension
/// vient du fichier choisi, parce que Typst distingue le PNG du JPEG.
fn nom_image(face: &str, ext: &str) -> Result<String, String> {
    match face {
        "une" => Ok(format!("couverture.{ext}")),
        "quatre" => Ok(format!("quatrieme.{ext}")),
        autre => Err(format!("face inconnue : {autre}")),
    }
}

/// Remplace l'image d'une face par un fichier choisi.
///
/// Le projet est auto-portant : l'image y est copiée, comme le manuscrit. Elle est
/// refusée ici plutôt qu'à la composition — une image dont Typst ne saura rien faire
/// n'a pas à entrer dans un `.ozalid` qui l'emporterait partout ensuite.
#[tauri::command]
pub fn image_choisir(
    face: String,
    chemin: String,
    atelier: State<Atelier>,
) -> Result<ProjetVue, String> {
    let source = Path::new(&chemin);
    let ext = source
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .filter(|e| matches!(e.as_str(), "jpg" | "jpeg" | "png"))
        .ok_or("image refusée : seuls le JPEG et le PNG se composent.")?;
    let nom = nom_image(&face, &ext)?;
    let octets = std::fs::read(source).map_err(|e| format!("image illisible : {e}"))?;
    Ressource::depuis(&nom, &octets)
        .ok_or_else(|| format!("{nom} : dimensions illisibles (ni PNG ni JPEG)."))?;

    let mut garde = atelier.ouvert.lock().unwrap();
    let o = garde.as_mut().ok_or_else(aucun_projet)?;
    poser_image(&mut o.projet.images, nom, octets);
    vue_modifiee(o)
}

/// Pose l'image d'une face et retire celle qui tenait déjà ce rôle.
///
/// Le remplacement se fait par rôle, pas par nom : une image importée s'appelle comme
/// elle veut, et deux images qui servent la même face laisseraient l'ordre alphabétique
/// décider laquelle se compose.
fn poser_image(images: &mut BTreeMap<String, Vec<u8>>, nom: String, octets: Vec<u8>) {
    let quatre = package::sert_la_quatrieme(&nom);
    images.retain(|n, _| package::sert_la_quatrieme(n) != quatre);
    images.insert(nom, octets);
}

/// Aperçu d'une face de couverture ou de la planche entière, en PNG encodé dans une
/// URL `data:`.
///
/// L'aperçu sort du **même** moteur et de la même source que le PDF final : il n'y a
/// donc pas d'écart écran/export à surveiller, contrairement à l'atelier HTML.
///
/// `dos_mm` vient de la dernière composition de l'intérieur ; il n'est jamais saisi.
/// Sans lui, la planche ne s'aperçoit pas — c'est voulu : une planche dont le dos
/// serait deviné donnerait à voir un livre qui n'existe pas.
#[tauri::command]
pub fn couverture_apercu(
    face: String,
    provider_cle: String,
    dos_mm: Option<f64>,
    fond_perdu_mm: Option<f64>,
    atelier: State<Atelier>,
) -> Result<String, String> {
    let garde = atelier.ouvert.lock().unwrap();
    let o = garde.as_ref().ok_or_else(aucun_projet)?;
    let cv = o
        .projet
        .meta
        .couverture
        .maquette
        .as_ref()
        .ok_or("aucune maquette : en choisir une.")?;
    let pr = providers::provider(&provider_cle)
        .ok_or_else(|| format!("prestataire inconnu : {provider_cle}"))?;

    // Répertoire de travail de l'aperçu : temporaire, jamais à côté du projet. Un
    // aperçu n'est pas une sortie, et il est réécrit à chaque réglage.
    let dossier = std::env::temp_dir().join("ozalid-apercu");
    std::fs::create_dir_all(&dossier).map_err(|e| format!("aperçu impossible : {e}"))?;

    let (une, quatre) = ecrire_images(&o.projet, &dossier)?;
    let src = match face.as_str() {
        "une" => couverture::source_une(&o.projet.meta.livre, cv, pr.format, une.as_ref(), dos_mm),
        "quatre" => {
            couverture::source_quatre(cv, pr.format, quatre.as_ref(), une.as_ref(), dos_mm)?
        }
        "planche" => {
            let dos = dos_mm.ok_or(
                "planche : composer l'intérieur d'abord, c'est la pagination qui donne le dos.",
            )?;
            let fp = pr.fond_perdu.or(fond_perdu_mm).ok_or_else(|| {
                format!(
                    "{} ne publie pas de fond perdu : le relever sur son gabarit et le saisir.",
                    pr.libelle
                )
            })?;
            let g = planche::Gabarit {
                format: pr.format,
                dos,
                fond_perdu: fp,
            };
            planche::source(&o.projet.meta.livre, cv, &g, une.as_ref(), quatre.as_ref())?
        }
        autre => return Err(format!("face inconnue : {autre}")),
    };

    let typ = dossier.join(format!("apercu-{face}.typ"));
    let png = dossier.join(format!("apercu-{face}.png"));
    ecrire(&typ, &src)?;
    typst()?.apercu(&typ, &png, 1, 150)?;

    let octets = std::fs::read(&png).map_err(|e| format!("aperçu illisible : {e}"))?;
    Ok(format!(
        "data:image/png;base64,{}",
        base64::Engine::encode(&base64::engine::general_purpose::STANDARD, octets)
    ))
}

/* ---------- packages ---------- */

/// Un prestataire coché, avec son papier et, s'il ne publie rien, ce que
/// l'utilisateur a relevé sur son gabarit.
///
/// Tauri met les *arguments* de commande en snake_case, jamais les champs d'une
/// struct : `Choix` voyage dans un tableau, il porte donc les noms que l'interface
/// écrit, comme tout ce qu'elle envoie.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Choix {
    pub provider_cle: String,
    pub papier_cle: Option<String>,
    pub dos_mm: Option<f64>,
    pub fond_perdu_mm: Option<f64>,
}

/// Ce que rend la génération pour un prestataire : le package, ou l'erreur qui l'a
/// empêché. Un prestataire en échec n'interrompt pas les autres — mais il est dit.
#[derive(Serialize)]
pub struct Resultat {
    pub provider: String,
    pub libelle: String,
    pub package: Option<package::Package>,
    pub erreur: Option<String>,
}

/// Génère les packages des prestataires cochés, chacun dans son répertoire.
///
/// Une seule maquette, N prestataires, aucun réglage retouché entre eux : chaque
/// prestataire compose son propre intérieur, donc sa propre pagination, donc son
/// propre dos. C'est la promesse de l'étape « Prestataires ».
#[tauri::command]
pub fn packager(choix: Vec<Choix>, atelier: State<Atelier>) -> Result<Vec<Resultat>, String> {
    let garde = atelier.ouvert.lock().unwrap();
    let o = garde.as_ref().ok_or_else(aucun_projet)?;
    if choix.is_empty() {
        return Err("aucun prestataire coché.".into());
    }
    let typst = typst()?;

    let mut sorties = Vec::with_capacity(choix.len());
    for c in &choix {
        let Some(pr) = providers::provider(&c.provider_cle) else {
            sorties.push(Resultat {
                provider: c.provider_cle.clone(),
                libelle: c.provider_cle.clone(),
                package: None,
                erreur: Some(format!("prestataire inconnu : {}", c.provider_cle)),
            });
            continue;
        };
        let r = papier(pr, c.papier_cle.as_deref()).and_then(|pa| {
            let dossier = sorties_dossier(o, pr.cle)?;
            package::assembler(
                &o.projet,
                pr,
                pa,
                planche::Releve {
                    dos: c.dos_mm,
                    fond_perdu: c.fond_perdu_mm,
                },
                &dossier,
                &typst,
            )
        });
        sorties.push(match r {
            Ok(p) => Resultat {
                provider: pr.cle.into(),
                libelle: pr.libelle.into(),
                package: Some(p),
                erreur: None,
            },
            Err(e) => Resultat {
                provider: pr.cle.into(),
                libelle: pr.libelle.into(),
                package: None,
                erreur: Some(e),
            },
        });
    }
    Ok(sorties)
}

fn papier(pr: &'static Provider, cle: Option<&str>) -> Result<&'static providers::Papier, String> {
    match cle {
        Some(c) => pr
            .papier(c)
            .ok_or_else(|| format!("papier inconnu chez {} : {c}", pr.cle)),
        None => Ok(pr.papier_defaut()),
    }
}

/// Écrit les images du projet à côté de la source, et rend leurs descriptions.
fn ecrire_images(
    projet: &Projet,
    dossier: &Path,
) -> Result<(Option<Ressource>, Option<Ressource>), String> {
    package::ecrire_images(projet, dossier)
}

/// Racine des sorties : un répertoire du nom du projet, à côté du `.ozalid`, jamais
/// dedans. Un projet non enregistré n'a donc pas d'endroit où écrire — c'est voulu,
/// sinon les sorties atterriraient dans un répertoire temporaire que personne ne
/// retrouve. L'épreuve s'y range directement : elle ne vise aucun éditeur.
fn sorties_racine(o: &Ouvert) -> Result<PathBuf, String> {
    let chemin = o.chemin.as_ref().ok_or_else(|| {
        "enregistrer le projet avant de composer : les sorties se rangent à côté du \
         fichier .ozalid."
            .to_string()
    })?;
    let parent = chemin.parent().unwrap_or(Path::new("."));
    let nom = chemin
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "projet".into());
    Ok(parent.join(nom))
}

/// Sorties d'un prestataire : un répertoire par prestataire, sous la racine.
fn sorties_dossier(o: &Ouvert, provider: &str) -> Result<PathBuf, String> {
    Ok(sorties_racine(o)?.join(provider))
}

fn poser(
    atelier: &State<Atelier>,
    chemin: Option<PathBuf>,
    projet: Projet,
    modifie: bool,
) -> Result<ProjetVue, String> {
    let mut garde = atelier.ouvert.lock().unwrap();
    *garde = Some(Ouvert {
        chemin,
        projet,
        modifie,
    });
    vue(garde.as_ref().unwrap())
}

fn vue(o: &Ouvert) -> Result<ProjetVue, String> {
    // Le compte de chapitres affiché est celui du manuscrit embarqué, pas celui que le
    // projet déclare : c'est l'écart entre les deux qui signale un manuscrit périmé.
    let chapitres_trouves = manuscrit::decoupe(&o.projet.texte, None)
        .map(|c| c.len() as u32)
        .unwrap_or(0);
    Ok(ProjetVue {
        chemin: o.chemin.as_ref().map(|c| c.to_string_lossy().into_owned()),
        livre: o.projet.meta.livre.clone(),
        manuscrit_source: o.projet.meta.manuscrit.source.clone(),
        chapitres_trouves,
        mots: o.projet.texte.split_whitespace().count() as u32,
        manuscrit_absent: o.projet.texte.trim().is_empty(),
        modifie: o.modifie,
        couverture: o.projet.meta.couverture.maquette.clone(),
        couverture_importee: o.projet.meta.couverture.maquette.is_some(),
        images: o.projet.images.keys().cloned().collect(),
        interieur: o.projet.meta.interieur.clone(),
    })
}

/// La vue d'un projet qu'on vient de modifier.
///
/// Deux fonctions plutôt qu'un drapeau posé à la main dans chaque commande : le
/// point d'appel dit ce qu'il a fait, et oublier de le dire se voit à la lecture.
fn vue_modifiee(o: &mut Ouvert) -> Result<ProjetVue, String> {
    o.modifie = true;
    vue(o)
}

/// La vue d'un projet qu'on vient d'écrire sur le disque.
fn vue_enregistree(o: &mut Ouvert) -> Result<ProjetVue, String> {
    o.modifie = false;
    vue(o)
}

fn aucun_projet() -> String {
    "aucun projet ouvert.".to_string()
}

fn ecrire(chemin: &Path, contenu: &str) -> Result<(), String> {
    std::fs::write(chemin, contenu)
        .map_err(|e| format!("écriture impossible ({}) : {e}", chemin.display()))
}

/// Binaire Typst à utiliser.
///
/// En release, seul le sidecar embarqué fait foi : se rabattre sur un Typst du système
/// rendrait la pagination dépendante de la machine, exactement ce que l'embarquement
/// doit empêcher. En développement, le Typst du PATH est accepté pour ne pas imposer
/// de vendorisation à chaque itération.
fn binaire_typst() -> Result<PathBuf, String> {
    let sidecar = std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(|d| d.join(nom_sidecar())))
        .filter(|p| p.is_file());
    match sidecar {
        Some(p) => Ok(p),
        None if cfg!(debug_assertions) => Ok(PathBuf::from("typst")),
        None => Err("Typst embarqué introuvable : l'application est mal empaquetée.".into()),
    }
}

/// Typst prêt à composer, polices embarquées comprises.
fn typst() -> Result<Typst, String> {
    let b = binaire_typst()?;
    let voisin = b.parent().map(Path::to_path_buf).unwrap_or_default();
    let candidats = [
        voisin.join("fonts"),
        // Empaquetage macOS : les ressources sont dans Contents/Resources, pas à côté
        // du binaire. Le chemin réel en release se vérifie au jalon 5.
        voisin.join("../Resources/fonts"),
        // Développement : les polices vivent dans les sources.
        Path::new(env!("CARGO_MANIFEST_DIR")).join("fonts"),
    ];
    let dossier = candidats
        .into_iter()
        .find(|p| p.is_dir())
        .ok_or("polices embarquées introuvables : lancer app/outils/polices.sh.")?;
    Ok(Typst::new(b).avec_polices(dossier))
}

fn nom_sidecar() -> &'static str {
    if cfg!(windows) {
        "typst.exe"
    } else {
        "typst"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// L'interface envoie les prestataires cochés dans un tableau, et Tauri ne
    /// renomme que les arguments d'une commande : si `Choix` cessait de lire les
    /// noms écrits par `choixPrestataires()`, la génération échouerait avant même
    /// d'atteindre le premier prestataire.
    #[test]
    fn les_choix_de_l_interface_se_lisent() {
        let json = r#"[{
            "providerCle": "lulu",
            "papierCle": "standard",
            "dosMm": null,
            "fondPerduMm": null
        }, {
            "providerCle": "coollibri-148x210",
            "papierCle": "mesure",
            "dosMm": 18.4,
            "fondPerduMm": 4
        }]"#;
        let choix: Vec<Choix> = serde_json::from_str(json).unwrap();
        assert_eq!(choix[0].provider_cle, "lulu");
        assert_eq!(choix[0].papier_cle.as_deref(), Some("standard"));
        assert_eq!(choix[0].dos_mm, None);
        assert_eq!(choix[1].provider_cle, "coollibri-148x210");
        assert_eq!(choix[1].dos_mm, Some(18.4));
        assert_eq!(choix[1].fond_perdu_mm, Some(4.0));
    }

    /// Choisir l'image d'une face remplace celle qui s'y composait, quel que soit le
    /// nom qu'elle portait — un projet importé nomme ses photos comme il l'entend — et
    /// laisse l'autre face intacte.
    #[test]
    fn une_face_ne_garde_qu_une_image() {
        let mut images = BTreeMap::from([
            ("photo.jpg".to_string(), vec![1]),
            ("quatrieme.jpg".to_string(), vec![2]),
        ]);

        poser_image(&mut images, "couverture.png".into(), vec![3]);
        assert_eq!(
            images.keys().collect::<Vec<_>>(),
            ["couverture.png", "quatrieme.jpg"],
            "l'image de 1ère n'a pas été remplacée, ou la 4ème a été emportée"
        );

        poser_image(&mut images, "quatrieme.png".into(), vec![4]);
        assert_eq!(
            images.keys().collect::<Vec<_>>(),
            ["couverture.png", "quatrieme.png"]
        );
    }

    /// Le nom porte le rôle : c'est tout ce que la composition lit pour savoir quelle
    /// face une image sert.
    #[test]
    fn le_nom_d_une_image_dit_la_face_qu_elle_sert() {
        assert_eq!(nom_image("une", "jpg").unwrap(), "couverture.jpg");
        assert_eq!(nom_image("quatre", "png").unwrap(), "quatrieme.png");
        assert!(package::sert_la_quatrieme(
            &nom_image("quatre", "png").unwrap()
        ));
        assert!(!package::sert_la_quatrieme(
            &nom_image("une", "png").unwrap()
        ));
        assert!(nom_image("planche", "png").is_err());
    }

    fn ouvert_neuf() -> Ouvert {
        Ouvert {
            chemin: None,
            projet: Projet::nouveau(Livre::vide(), String::new()),
            modifie: false,
        }
    }

    /// Le drapeau est ce qui décide si fermer l'application perd du travail. Il ne
    /// doit se lever que par une mutation, et retomber par une écriture — jamais
    /// par une simple relecture du projet.
    #[test]
    fn le_drapeau_de_modification_suit_les_mutations_et_les_ecritures() {
        let mut o = ouvert_neuf();
        assert!(
            !vue(&o).unwrap().modifie,
            "un projet neuf n'est pas modifié"
        );
        assert!(!vue(&o).unwrap().modifie, "relire ne modifie pas");

        assert!(vue_modifiee(&mut o).unwrap().modifie);
        assert!(vue(&o).unwrap().modifie, "le drapeau reste levé");

        assert!(!vue_enregistree(&mut o).unwrap().modifie);
    }

    /// Un manuscrit absent et un manuscrit sans chapitre composable rendent tous
    /// deux zéro chapitre. L'interface doit pouvoir dire « aucun manuscrit » plutôt
    /// que « 0 chapitre » : ce n'est pas la même chose à corriger.
    #[test]
    fn un_manuscrit_vide_se_declare_absent_et_non_vide_de_chapitres() {
        let vide = ouvert_neuf();
        let v = vue(&vide).unwrap();
        assert!(v.manuscrit_absent);
        assert_eq!(v.chapitres_trouves, 0);

        let mut plein = ouvert_neuf();
        plein.projet.texte = "## 01 - Un\n\nTexte.\n".into();
        let v = vue(&plein).unwrap();
        assert!(!v.manuscrit_absent);
        assert_eq!(v.chapitres_trouves, 1);

        // Du texte qui ne porte aucun « ## » : présent, mais sans chapitre.
        let mut sans_chapitre = ouvert_neuf();
        sans_chapitre.projet.texte = "juste une phrase\n".into();
        let v = vue(&sans_chapitre).unwrap();
        assert!(!v.manuscrit_absent, "présent, même s'il ne compose pas");
        assert_eq!(v.chapitres_trouves, 0);

        // Des espaces et des sauts de ligne ne sont pas un manuscrit : c'est ce que
        // `trim` établit, et rien ne le dirait si on le retirait.
        let mut blancs = ouvert_neuf();
        blancs.projet.texte = "  \n\n\t \n".into();
        assert!(vue(&blancs).unwrap().manuscrit_absent);
    }

    /// Le genre par défaut ne doit vivre qu'à un endroit : un projet neuf et un
    /// projet relu d'un TOML sans genre doivent porter le même.
    #[test]
    fn un_livre_vide_prend_le_genre_par_defaut() {
        let l = Livre::vide();
        assert_eq!(l.genre, "roman");
        assert!(l.titre.is_empty());
        assert!(l.auteur.is_empty());
        assert_eq!(l.chapitres, None);
        assert_eq!(l.titre_page, None);
    }
}
