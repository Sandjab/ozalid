//! Demander une image à un modèle de diffusion.
//!
//! Le seul endroit du projet qui ouvre une connexion, et il ne s'ouvre qu'à la demande :
//! **composer ne rappelle jamais le réseau**. Une image acceptée est figée dans
//! l'archive, et un package se refait des mois plus tard, hors ligne, à l'identique.
//!
//! Le transport est injecté, comme la mesure de `converge` et comme Typst : une logique
//! qu'on ne pourrait éprouver qu'en ligne ne serait pas éprouvée. Les tests de ce module
//! tournent sans réseau, et c'est ce qui permet d'y écrire le cas d'une clé qui fuit.

use serde::{Deserialize, Serialize};

/// Ce qu'il faut pour demander une image. Les deux vivent dans les préférences, jamais
/// dans le `.ozalid` : un projet est fait pour être ouvert ailleurs, et y écrire une clé
/// la publierait au premier partage.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Acces {
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub cle: String,
    /// Le nom du modèle, quand le fournisseur l'attend dans le corps plutôt que dans
    /// son adresse — `gemini-3-pro-image` chez Google, où l'adresse est la même pour
    /// tous. Facultatif : là où l'adresse porte déjà le modèle, le nommer une seconde
    /// fois ferait refuser la demande.
    #[serde(default)]
    pub modele: String,
}

impl Acces {
    pub fn pret(&self) -> bool {
        !self.url.trim().is_empty() && !self.cle.trim().is_empty()
    }
}

/// Le transport, réduit à ce que ce module réclame.
///
/// Deux verbes parce que le format le plus répandu rend l'image de deux façons : encodée
/// dans la réponse, ou derrière une adresse qu'il faut aller chercher.
pub trait Transport {
    /// POST JSON authentifié ; rend le corps de la réponse.
    fn poste(&self, url: &str, cle: &str, corps: &str) -> Result<String, String>;
    /// GET ; rend les octets.
    fn prend(&self, url: &str) -> Result<Vec<u8>, String>;
}

/// Le gabarit livré avec l'application, celui qu'on trouve avant d'avoir rien réglé.
///
/// Un champ vide ne dit pas sa syntaxe : devant lui, personne ne devine qu'il existe
/// cinq marques ni ce qu'un modèle de diffusion demande pour rendre une écriture. Il est
/// en **anglais**, langue que les modèles à poids ouverts suivent le mieux ; le texte
/// écrit, lui, reste français puisqu'il vient de `{envoi}`. C'est aussi pourquoi la
/// couleur se saisit en anglais.
///
/// Il demande un fond blanc uni sans texture ni ombre, et ce n'est pas seulement une
/// question de goût : c'est ce qui donne au détourage la photo la plus facile à séparer.
pub const GABARIT_DEFAUT: &str = r#"A short handwritten dedication, isolated on a pure white background.
Flat scan, not a photograph. No paper texture, no shadow, no vignette, no lighting effect.

The dedication reads exactly:
À {dedicataire}, {envoi}

Handwriting: personal French cursive, {couleur} fountain pen ink, medium nib, written quickly, slightly untidy and irregular. Slight rightward slant. Uneven baseline — the lines drift a little.
Letter size varies naturally between words. Ink slightly heavier where the pen changes direction, thinner on upstrokes.

Written by an adult in one continuous gesture: unstudied, a little hurried, not calligraphic. Some letters joined, some lifted.

Render the accented characters exactly as written: é è à ç.

Below the dedication, offset slightly to the right, a personal signature mark: "{paraphe}", each word written as a single connected gesture, without the pen lifting. A wide loop encircles the letters before the pen exits to the right in a flat tapering stroke, thinner and slightly ragged at the end where the pen leaves the paper. The signature is slightly larger and looser than the dedication text above it, more gesture than writing. Written with the same fountain pen as the dedication, and it shows. Strong contrast between strokes: the downstrokes are broad, where the nib tines spread under pressure; the upstrokes are hairline-thin, where the nib barely touches. The transition between the two is gradual within a single stroke, never uniform. Ink pools into a darker, slightly bleeding dot where the pen starts and where it stops, and at the sharp turns. Along the broadest strokes the ink is unevenly saturated, a little darker at the edges than in the centre. 

Nothing else on the image. No border, no ornament, no signature line, no date, no extra text.

High resolution, sharp edges, pure {couleur} ink on pure white background."#;

/// La marque, dans le gabarit du livre, où le mot de chaque envoi s'insère.
pub const MARQUE: &str = "{envoi}";

/// Ce qu'un gabarit peut appeler par son nom.
///
/// Une structure plutôt que trois `&str` de rang : les trois ont le même type, et un
/// appel qui les inverserait composerait un prompt plausible que rien ne rattraperait.
#[derive(Debug, Clone, Copy, Default)]
pub struct Mots<'a> {
    /// La dédicace de cet envoi — le seul mot qui distingue une image de la suivante.
    pub envoi: &'a str,
    /// À qui l'exemplaire est adressé.
    pub dedicataire: &'a str,
    /// Le titre du livre : commun à tout le tirage, il ne distingue rien.
    pub titre: &'a str,
    /// La couleur de l'encre, telle que le gabarit la nomme au modèle. Commune au
    /// tirage : elle ne distingue rien.
    pub couleur: &'a str,
    /// Le paraphe de l'auteur, sa signature. Commun au tirage lui aussi — c'est la même
    /// main qui signe les vingt exemplaires.
    pub paraphe: &'a str,
}

/// Une marque et le mot qu'elle nomme.
type Nommee = (&'static str, for<'a> fn(&Mots<'a>) -> &'a str);

/// Les marques reconnues, dans l'ordre où l'aide les présente.
const MARQUES: [Nommee; 5] = [
    (MARQUE, |m| m.envoi),
    ("{dedicataire}", |m| m.dedicataire),
    ("{titre}", |m| m.titre),
    ("{couleur}", |m| m.couleur),
    ("{paraphe}", |m| m.paraphe),
];

/// Celles qui peuvent distinguer un envoi du suivant. Ni le titre, ni la couleur de
/// l'encre, ni le paraphe n'en sont : ils sont les mêmes pour tout le tirage, et un
/// gabarit qui ne nommerait qu'eux rendrait M fois la même image.
const DISTINGUENT: [Nommee; 2] = [MARQUES[0], MARQUES[1]];

/// Le prompt d'un envoi : le gabarit du livre, où ses mots viennent se poser.
///
/// Sans marque qui distingue un envoi du suivant, la dédicace est ajoutée à la suite
/// plutôt qu'ignorée : un gabarit écrit sans connaître la syntaxe produirait sinon M
/// images identiques, ce qui ne se verrait qu'à l'aperçu — et seulement si on les
/// regardait toutes.
pub fn prompt(gabarit: &str, mots: &Mots) -> String {
    let g = gabarit.trim();
    let sortie = substituer(g, mots);
    let c = mots.envoi.trim();
    // Une marque vide ne distingue rien : elle est écrite dans le gabarit, mais deux
    // exemplaires en tirent la même phrase. C'est la valeur qui décide, pas la présence.
    let distingue = DISTINGUENT
        .iter()
        .any(|(marque, mot)| g.contains(marque) && !mot(mots).trim().is_empty());
    if c.is_empty() || distingue {
        return sortie;
    }
    // `trim_end` et non `sortie` telle quelle : une marque vide en fin de gabarit y a
    // laissé un blanc, et la dédicace se poserait derrière deux espaces.
    format!("{} {c}", sortie.trim_end())
}

/// Remplace les marques connues par le mot qu'elles nomment.
///
/// **Une seule passe**, comme `gabarit::substituer` et pour la même raison : le texte
/// est parcouru une fois de gauche à droite, et ce qu'une marque produit est poussé dans
/// la sortie sans jamais être réexaminé. Un `replace` par marque en boucle aurait l'air
/// équivalent et ne le serait pas — il traiterait le mot précédent comme du gabarit, et
/// une dédicace citant `{titre}` suffit à le montrer.
///
/// Une marque inconnue est recopiée telle quelle : ce qui part au modèle est du texte,
/// pas une syntaxe, et une faute de frappe doit se voir plutôt que vider la phrase.
fn substituer(texte: &str, mots: &Mots) -> String {
    let mut sortie = String::with_capacity(texte.len());
    let mut reste = texte;
    while let Some(i) = reste.find('{') {
        sortie.push_str(&reste[..i]);
        let a_partir_de_l_accolade = &reste[i..];
        match MARQUES
            .iter()
            .find(|(marque, _)| a_partir_de_l_accolade.starts_with(marque))
        {
            Some((marque, mot)) => {
                sortie.push_str(mot(mots).trim());
                reste = &a_partir_de_l_accolade[marque.len()..];
            }
            None => {
                sortie.push('{');
                reste = &a_partir_de_l_accolade[1..];
            }
        }
    }
    sortie.push_str(reste);
    sortie
}

/// Demande l'image, et rend ses octets — ou dit pourquoi elle n'est pas venue.
///
/// **Aucun message qui remonte d'ici ne porte la clé.** Le transport, lui, la voit ; ce
/// qu'il en dit passe par `expurge` avant d'atteindre l'interface, la journalisation ou
/// une vue. C'est la contrepartie de la garder en clair dans `preferences.toml`.
pub fn genere(acces: &Acces, prompt: &str, transport: &dyn Transport) -> Result<Vec<u8>, String> {
    if !acces.pret() {
        return Err("aucun accès au modèle : renseigner son adresse et sa clé.".into());
    }
    if prompt.trim().is_empty() {
        return Err("rien à demander : le gabarit et le mot de l'envoi sont vides.".into());
    }
    let mut corps = serde_json::json!({ "prompt": prompt });
    // Nommé seulement s'il l'est : voir `Acces::modele`.
    let modele = acces.modele.trim();
    if !modele.is_empty() {
        corps["model"] = modele.into();
    }
    let corps = corps.to_string();
    let reponse = transport
        .poste(acces.url.trim(), acces.cle.trim(), &corps)
        .map_err(|e| expurge(&e, &acces.cle))?;
    let octets = octets(&reponse, transport).map_err(|e| expurge(&e, &acces.cle))?;
    // Ce que le modèle a rendu doit être une image, et se relever comme telle : un
    // message d'erreur rendu en 200 se retrouverait sinon dans un exemplaire imprimé.
    crate::image::dimensions(&octets)
        .ok_or("le modèle n'a pas rendu une image (ni PNG ni JPEG).")?;
    Ok(octets)
}

/// Les octets de l'image, encodés dans la réponse ou derrière l'adresse qu'elle donne.
fn octets(reponse: &str, transport: &dyn Transport) -> Result<Vec<u8>, String> {
    let json: serde_json::Value =
        serde_json::from_str(reponse).map_err(|e| format!("réponse du modèle illisible : {e}"))?;
    let premiere = json
        .get("data")
        .and_then(|d| d.get(0))
        .ok_or("réponse du modèle sans image : « data » est absent ou vide.")?;
    if let Some(b64) = premiere.get("b64_json").and_then(|v| v.as_str()) {
        use base64::Engine;
        return base64::engine::general_purpose::STANDARD
            .decode(b64)
            .map_err(|e| format!("image du modèle illisible (base64) : {e}"));
    }
    let url = premiere
        .get("url")
        .and_then(|v| v.as_str())
        .ok_or("réponse du modèle sans image : ni « b64_json » ni « url ».")?;
    transport.prend(url)
}

/// Ce qu'une réponse dit, ou pourquoi elle ne dit rien d'utilisable.
///
/// **Le statut seul ne diagnostique pas.** « 404 » sur une adresse juste veut dire que
/// le modèle demandé n'existe pas pour cette clé, « 400 » que le corps ne porte pas ce
/// qu'on attend — et c'est la réponse, jamais le statut, qui nomme lequel. Le corps est
/// donc joint au message ; `genere` en retire la clé avant qu'il n'atteigne l'écran.
///
/// Tronqué, parce qu'une adresse qui n'est pas celle d'un modèle rend volontiers une
/// page entière, et que celle-ci finirait dans une bulle d'interface.
fn reponse(statut: u16, corps: String) -> Result<String, String> {
    if (200..300).contains(&statut) {
        return Ok(corps);
    }
    let dit = corps.trim();
    if dit.is_empty() {
        return Err(format!("le modèle a répondu {statut}, sans un mot."));
    }
    // Sur une frontière de caractère : le corps peut être en UTF-8, et une coupe à
    // l'octet paniquerait au lieu de rapporter l'erreur qu'on est en train de rapporter.
    let court = match dit.char_indices().nth(500) {
        Some((i, _)) => format!("{}…", &dit[..i]),
        None => dit.to_string(),
    };
    Err(format!("le modèle a répondu {statut} : {court}"))
}

/// Efface la clé d'un message avant qu'il ne remonte.
///
/// Le message d'un client HTTP porte volontiers ce qu'on lui a donné — l'adresse
/// appelée, parfois l'en-tête. La clé est en clair dans `preferences.toml`, avec les
/// permissions du fichier ; elle ne doit pas se retrouver en plus dans une bulle
/// d'erreur, sur une sortie d'erreur ou dans une capture d'écran.
pub fn expurge(message: &str, cle: &str) -> String {
    let c = cle.trim();
    if c.is_empty() {
        return message.to_string();
    }
    message.replace(c, "[clé masquée]")
}

/// Le transport réel.
pub struct Reseau;

impl Transport for Reseau {
    fn poste(&self, url: &str, cle: &str, corps: &str) -> Result<String, String> {
        // Le statut n'est pas traité comme une erreur, pour que le corps soit lu : c'est
        // lui qui dit *pourquoi* la demande est refusée, et `ureq` le jetterait avec la
        // réponse. Le tri revient à `reponse`.
        let mut r = ureq::post(url)
            .config()
            .http_status_as_error(false)
            .build()
            .header("Authorization", &format!("Bearer {cle}"))
            .header("Content-Type", "application/json")
            .send(corps)
            .map_err(|e| e.to_string())?;
        let statut = r.status().as_u16();
        let lu = r.body_mut().read_to_string().map_err(|e| e.to_string())?;
        reponse(statut, lu)
    }

    fn prend(&self, url: &str) -> Result<Vec<u8>, String> {
        let mut r = ureq::get(url).call().map_err(|e| e.to_string())?;
        // Une image de couverture pèse quelques mégaoctets ; la borne est là pour qu'une
        // adresse qui rend autre chose ne remplisse pas la mémoire.
        r.body_mut()
            .with_config()
            .limit(32 * 1024 * 1024)
            .read_to_vec()
            .map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    const CLE: &str = "sk-tres-secrete";

    /// Un modèle de façade : ce qu'il a reçu, et ce qu'il rend.
    struct Faux {
        reponse: Result<String, String>,
        image: Result<Vec<u8>, String>,
        vu: RefCell<Vec<String>>,
    }

    impl Faux {
        fn rend(corps: &str) -> Self {
            Self {
                reponse: Ok(corps.into()),
                image: Ok(png()),
                vu: RefCell::new(Vec::new()),
            }
        }
    }

    impl Transport for Faux {
        fn poste(&self, url: &str, cle: &str, corps: &str) -> Result<String, String> {
            self.vu
                .borrow_mut()
                .push(format!("POST {url} | {cle} | {corps}"));
            self.reponse.clone()
        }
        fn prend(&self, url: &str) -> Result<Vec<u8>, String> {
            self.vu.borrow_mut().push(format!("GET {url}"));
            self.image.clone()
        }
    }

    fn png() -> Vec<u8> {
        let mut p = b"\x89PNG\r\n\x1a\n".to_vec();
        p.extend(13u32.to_be_bytes());
        p.extend(b"IHDR");
        p.extend(1024u32.to_be_bytes());
        p.extend(1024u32.to_be_bytes());
        p.extend([8, 6, 0, 0, 0]);
        p
    }

    /// Les mots d'un envoi, dont seule la dédicace varie d'un test à l'autre.
    fn mots(envoi: &str) -> Mots<'_> {
        Mots {
            envoi,
            dedicataire: "Léa",
            titre: "Le Chemin",
            ..Mots::default()
        }
    }

    fn acces() -> Acces {
        Acces {
            url: "https://exemple.test/images".into(),
            cle: CLE.into(),
            modele: String::new(),
        }
    }

    fn encodee() -> String {
        use base64::Engine;
        let b64 = base64::engine::general_purpose::STANDARD.encode(png());
        format!(r#"{{"data":[{{"b64_json":"{b64}"}}]}}"#)
    }

    /// Le gabarit appartient au livre, le mot à l'envoi : chaque image demandée doit
    /// donc différer par le seul mot. Un gabarit qui ne saurait pas où poser le mot
    /// produirait M images identiques, et cela ne se verrait qu'en les regardant toutes.
    #[test]
    fn le_mot_de_l_envoi_entre_dans_le_gabarit_du_livre() {
        let g = "une aquarelle, mention manuscrite « {envoi} », papier grené";
        assert_eq!(
            prompt(g, &mots("À Léa")),
            "une aquarelle, mention manuscrite « À Léa », papier grené"
        );
        assert_ne!(prompt(g, &mots("À Léa")), prompt(g, &mots("À Marie")));
        assert_eq!(
            prompt("une aquarelle", &mots("À Léa")),
            "une aquarelle À Léa",
            "sans la marque, le mot doit être ajouté et non perdu"
        );
    }

    /// Le dédicataire et le titre s'appellent par leur nom, comme la dédicace.
    ///
    /// Le titre vient du livre et le dédicataire de l'exemplaire : sans eux, un gabarit
    /// qui veut « pour Léa, d'après Le Chemin » obligerait à réécrire les deux dans
    /// chacune des M dédicaces, où ils se désaccorderaient du livre au premier renommage.
    #[test]
    fn le_dedicataire_et_le_titre_entrent_aussi_dans_le_gabarit() {
        assert_eq!(
            prompt(
                "une aquarelle pour {dedicataire}, d'après « {titre} » : {envoi}",
                &mots("À Léa")
            ),
            "une aquarelle pour Léa, d'après « Le Chemin » : À Léa"
        );
    }

    /// **Une seule passe**, comme `gabarit::substituer` et pour la même raison : ce
    /// qu'une marque produit est écrit et jamais réexaminé.
    ///
    /// Une dédicace est un texte libre, écrit par quelqu'un qui vient de lire l'aide des
    /// marques. Qu'elle en cite une ne doit pas la faire remplacer — trois `replace` en
    /// chaîne auraient l'air équivalents et ne le sont pas.
    #[test]
    fn une_marque_citee_dans_la_dedicace_reste_du_texte() {
        assert_eq!(
            prompt(
                "une aquarelle : {envoi}",
                &Mots {
                    envoi: "pour toi qui écris {titre}",
                    dedicataire: "Léa",
                    titre: "Le Chemin",
                    ..Mots::default()
                }
            ),
            "une aquarelle : pour toi qui écris {titre}",
            "la sortie d'une marque n'est pas relue"
        );
    }

    /// Une marque inconnue est recopiée telle quelle : le gabarit part au modèle, qui
    /// lit du texte et non une syntaxe. Une faute de frappe doit se voir dans l'image
    /// plutôt que vider la phrase de son sujet.
    ///
    /// L'exemple était `{couleur}` avant qu'elle ne devienne une marque : une faute de
    /// frappe sur une marque connue est un meilleur témoin, parce que c'est le cas que
    /// ce comportement existe pour rattraper — et il ne risque pas d'être adopté un
    /// jour, contrairement à un nom qu'on pourrait vouloir reconnaître.
    #[test]
    fn une_marque_inconnue_est_recopiee() {
        assert_eq!(
            prompt("une aquarelle {envooi} pour {dedicataire}", &mots("")),
            "une aquarelle {envooi} pour Léa"
        );
    }

    /// Le repli garde son objet — éviter M images identiques — et rien de plus.
    ///
    /// Un gabarit qui nomme le dédicataire distingue déjà chaque envoi : y ajouter la
    /// dédicace en queue doublerait un mot que l'auteur a placé lui-même. Le titre, lui,
    /// ne distingue rien : il est le même pour tout le tirage.
    #[test]
    fn le_repli_ne_joue_que_sans_marque_qui_distingue_les_envois() {
        assert_eq!(
            prompt("une aquarelle pour {dedicataire}", &mots("À Léa")),
            "une aquarelle pour Léa",
            "le dédicataire distingue déjà : rien à ajouter en queue"
        );
        assert_eq!(
            prompt("une aquarelle, « {titre} »", &mots("À Léa")),
            "une aquarelle, « Le Chemin » À Léa",
            "le titre ne distingue rien : la dédicace doit rester"
        );
        // Une liste se remplit avant d'être nommée : un envoi sans dédicataire est un
        // état de travail, pas une avarie — la marque est là, mais elle ne distingue
        // rien, et sans la dédicace en queue tous les exemplaires partagent une image.
        assert_eq!(
            prompt(
                "une aquarelle pour {dedicataire}",
                &Mots {
                    envoi: "À Léa",
                    dedicataire: "  ",
                    titre: "Le Chemin",
                    ..Mots::default()
                }
            ),
            "une aquarelle pour À Léa",
            "un dédicataire vide ne distingue pas : la dédicace doit rester"
        );
    }

    /// **Un statut seul ne diagnostique rien.** « 404 » sur une adresse juste veut dire
    /// que le modèle demandé n'existe pas pour cette clé, et seul le corps de la réponse
    /// le nomme. Sans lui, il n'y a rien à chercher : ni le champ fautif, ni le modèle
    /// refusé, ni l'expiration de la clé.
    #[test]
    fn une_reponse_en_erreur_remonte_avec_ce_qu_elle_dit() {
        let err = reponse(
            404,
            r#"{"error":{"message":"models/gemini-3-pro-image is not found"}}"#.into(),
        )
        .unwrap_err();
        assert!(err.contains("404"), "{err}");
        assert!(
            err.contains("gemini-3-pro-image"),
            "le corps est perdu : {err}"
        );
    }

    /// Une adresse qui n'est pas celle d'un modèle rend volontiers une page entière.
    /// Elle remonte jusqu'à une bulle d'interface : ce qui la nomme tient au début.
    #[test]
    fn un_corps_d_erreur_demesure_est_tronque() {
        let err = reponse(500, "é".repeat(4000)).unwrap_err();
        assert!(
            err.chars().count() < 700,
            "{} caractères",
            err.chars().count()
        );
        assert!(err.ends_with('…'), "la troncature ne se voit pas : {err}");
    }

    /// Un statut muet ne doit pas laisser un message qui promet un détail absent.
    #[test]
    fn une_erreur_sans_un_mot_le_dit() {
        let err = reponse(502, "  \n ".into()).unwrap_err();
        assert!(err.contains("502"), "{err}");
        assert!(err.contains("sans un mot"), "{err}");
    }

    #[test]
    fn une_reponse_normale_passe_telle_quelle() {
        assert_eq!(reponse(200, "{}".into()).unwrap(), "{}");
    }

    /// Le modèle se nomme dans le corps, quand il a un nom à donner.
    ///
    /// Un fournisseur porte son modèle dans l'adresse, un autre l'attend dans le corps —
    /// c'est le cas de la couche compatible de Google, qui refuse la demande sans lui.
    /// Le champ est donc envoyé quand il est réglé, et **omis** quand il ne l'est pas :
    /// un `"model": ""` ferait refuser les adresses qui n'en attendent aucun, et le
    /// réglage d'un fournisseur casserait l'autre.
    #[test]
    fn le_modele_regle_est_nomme_dans_le_corps() {
        let f = Faux::rend(&encodee());
        let mut a = acces();
        a.modele = "gemini-3-pro-image".into();
        genere(&a, "une aquarelle", &f).unwrap();
        assert!(
            f.vu.borrow()[0].contains(r#""model":"gemini-3-pro-image""#),
            "modèle absent du corps : {:?}",
            f.vu.borrow()[0]
        );
    }

    #[test]
    fn sans_modele_regle_le_corps_n_en_nomme_aucun() {
        let f = Faux::rend(&encodee());
        genere(&acces(), "une aquarelle", &f).unwrap();
        assert!(
            !f.vu.borrow()[0].contains("model"),
            "un modèle vide est envoyé quand même : {:?}",
            f.vu.borrow()[0]
        );
    }

    /// Le format le plus répandu encode l'image dans sa réponse. C'est le chemin normal,
    /// et il ne doit demander qu'un seul aller-retour.
    #[test]
    fn une_image_encodee_dans_la_reponse_est_decodee() {
        let f = Faux::rend(&encodee());
        assert_eq!(genere(&acces(), "une aquarelle", &f).unwrap(), png());
        assert_eq!(f.vu.borrow().len(), 1, "un aller-retour de trop");
    }

    /// À défaut, l'image est derrière une adresse : il faut aller la chercher. Sans ce
    /// second appel, le projet embarquerait une adresse à la place d'une image, et
    /// l'exemplaire partirait avec une page de titre vide.
    #[test]
    fn une_image_derriere_une_adresse_est_allee_chercher() {
        let f = Faux::rend(r#"{"data":[{"url":"https://exemple.test/i/1.png"}]}"#);
        assert_eq!(genere(&acces(), "une aquarelle", &f).unwrap(), png());
        assert_eq!(f.vu.borrow()[1], "GET https://exemple.test/i/1.png");
    }

    /// **La garde du lot.** La clé est en clair dans `preferences.toml`, avec les
    /// permissions du fichier : c'est un choix. Il ne tient que si elle ne se retrouve
    /// nulle part ailleurs — or le message d'un client HTTP porte volontiers ce qu'on
    /// lui a donné, et ce message-là remonte jusqu'à l'écran.
    #[test]
    fn la_cle_ne_remonte_dans_aucune_erreur() {
        let f = Faux {
            reponse: Err(format!(
                "401 sur https://exemple.test/images (Authorization: Bearer {CLE})"
            )),
            image: Ok(png()),
            vu: RefCell::new(Vec::new()),
        };
        let err = genere(&acces(), "une aquarelle", &f).unwrap_err();
        assert!(!err.contains(CLE), "la clé a fui : {err}");
        assert!(err.contains("401"), "l'erreur ne dit plus rien : {err}");
    }

    /// Ce que le modèle rend n'est pas toujours une image : une erreur en 200, une page
    /// HTML, un JSON d'excuses. Le relever ici évite qu'elle atterrisse, telle quelle,
    /// sur la page de titre d'un exemplaire imprimé.
    #[test]
    fn ce_qui_n_est_pas_une_image_est_refuse() {
        let f = Faux {
            reponse: Ok(r#"{"data":[{"url":"https://exemple.test/i/1.png"}]}"#.into()),
            image: Ok(b"<html>desole</html>".to_vec()),
            vu: RefCell::new(Vec::new()),
        };
        let err = genere(&acces(), "une aquarelle", &f).unwrap_err();
        assert!(err.contains("image"), "{err}");
    }

    /// Trois réponses qui ne portent pas d'image, et le message doit dire laquelle des
    /// trois : sans cela, « échec de la génération » laisse chercher dans le vide.
    #[test]
    fn une_reponse_sans_image_est_signalee_pour_ce_qu_elle_est() {
        for (reponse, attendu) in [
            ("pas du json", "illisible"),
            (r#"{"data":[]}"#, "data"),
            (r#"{"data":[{"revised_prompt":"…"}]}"#, "b64_json"),
        ] {
            let f = Faux::rend(reponse);
            let err = genere(&acces(), "une aquarelle", &f).unwrap_err();
            assert!(err.contains(attendu), "{reponse} → {err}");
        }
    }

    /// Sans adresse ni clé, rien n'est tenté : l'erreur doit dire quoi renseigner, et
    /// non rapporter un échec de connexion vers une adresse vide.
    #[test]
    fn sans_acces_rien_n_est_demande() {
        let f = Faux::rend(&encodee());
        let err = genere(&Acces::default(), "une aquarelle", &f).unwrap_err();
        assert!(err.contains("accès"), "{err}");
        assert!(
            f.vu.borrow().is_empty(),
            "le réseau a été appelé quand même"
        );
    }

    /// Le prompt part tel quel dans un corps JSON valide : un gabarit qui porte un
    /// guillemet ou un saut de ligne ne doit pas produire une requête malformée.
    #[test]
    fn le_prompt_voyage_dans_un_corps_json_valide() {
        let f = Faux::rend(&encodee());
        genere(&acces(), "une aquarelle « À Léa »\nsur papier", &f).unwrap();
        let corps = f.vu.borrow()[0].split(" | ").nth(2).unwrap().to_string();
        let json: serde_json::Value = serde_json::from_str(&corps).expect("corps illisible");
        assert_eq!(json["prompt"], "une aquarelle « À Léa »\nsur papier");
    }

    /// La couleur de l'encre et le paraphe entrent dans le gabarit comme le reste.
    ///
    /// Ils appartiennent au **livre** et non à l'exemplaire : un auteur signe ses vingt
    /// exemplaires du même stylo, et le gabarit est déjà le style d'écriture du livre.
    #[test]
    fn la_couleur_et_le_paraphe_entrent_aussi_dans_le_gabarit() {
        let m = Mots {
            envoi: "ces heures creuses",
            dedicataire: "Léa",
            titre: "Les Heures creuses",
            couleur: "blue-black",
            paraphe: "Ivan Pjig",
        };
        let p = prompt("encre {couleur}, signé « {paraphe} » : {envoi}", &m);
        assert_eq!(
            p,
            "encre blue-black, signé « Ivan Pjig » : ces heures creuses"
        );
    }

    /// **Ni la couleur ni le paraphe ne distinguent un envoi du suivant** : ils sont les
    /// mêmes pour tout le tirage, comme le titre. Un gabarit qui ne citerait qu'eux
    /// rendrait M images identiques, et c'est justement ce que le repli existe pour
    /// éviter — il doit continuer de s'en apercevoir.
    #[test]
    fn la_couleur_et_le_paraphe_ne_distinguent_rien() {
        let m = Mots {
            envoi: "ces heures creuses",
            dedicataire: "Léa",
            titre: "Les Heures creuses",
            couleur: "black",
            paraphe: "Ivan Pjig",
        };
        let p = prompt("encre {couleur}, paraphe {paraphe}", &m);
        assert!(
            p.ends_with("ces heures creuses"),
            "le repli n'a pas joué : {p}"
        );
    }
}
