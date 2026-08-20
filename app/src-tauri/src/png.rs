//! Lecture du bloc de réglages qu'`index.html` écrit dans les PNG de couverture.
//!
//! Format : un chunk `tEXt` de mot-clé `atelier-couverture`, dont la charge est le
//! base64 d'un JSON UTF-8. Le PNG reste un PNG standard — c'est la raison du choix
//! d'origine, et elle vaut toujours : les fichiers déjà publiés restent lisibles.
//!
//! On ne fait que lire. L'app n'écrit plus ce bloc : ses projets vivent dans un
//! `.ozalid`, et `index.html` est gelé.

use std::collections::BTreeMap;

use base64::Engine;
use serde::{Deserialize, Serialize};

pub const MOT_CLE: &str = "atelier-couverture";

/// Réglages relus depuis un PNG de l'atelier.
///
/// Les champs sont conservés tels quels : ils portent les identifiants de contrôles
/// d'`index.html` (`inTitre`, `inFrameM`…), et leur traduction vers le moteur Typst
/// appartient au jalon 3. Les versions successives de l'atelier n'ont pas les mêmes
/// champs — un PNG de 2026 en compte 54, la dernière version davantage —, donc rien
/// n'est exigé au-delà de l'enveloppe.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReglagesAtelier {
    pub app: String,
    #[serde(default)]
    pub v: u32,
    #[serde(default)]
    pub mode: String,
    #[serde(default)]
    pub format: Vec<f64>,
    #[serde(default)]
    pub fields: BTreeMap<String, serde_json::Value>,
    /// Photo source de la 1ère, en URL `data:` — absente si l'export ne l'a pas embarquée.
    #[serde(default)]
    pub image: Option<String>,
    /// Photo source de la 4ème.
    #[serde(default)]
    pub image4: Option<String>,
}

/// Charge d'un chunk `tEXt` portant ce mot-clé, ou `None` si le PNG n'en a pas.
pub fn texte(octets: &[u8], mot_cle: &str) -> Option<Vec<u8>> {
    if octets.len() < 8 || &octets[..8] != b"\x89PNG\r\n\x1a\n" {
        return None;
    }
    let mut p = 8usize;
    while p + 12 <= octets.len() {
        let taille = u32::from_be_bytes(octets[p..p + 4].try_into().ok()?) as usize;
        let genre = &octets[p + 4..p + 8];
        let fin = p.checked_add(8)?.checked_add(taille)?;
        if fin > octets.len() {
            // Fichier tronqué : on s'arrête au lieu de lire au-delà.
            return None;
        }
        if genre == b"tEXt" {
            let data = &octets[p + 8..fin];
            if let Some(z) = data.iter().position(|&b| b == 0) {
                if &data[..z] == mot_cle.as_bytes() {
                    return Some(data[z + 1..].to_vec());
                }
            }
        }
        if genre == b"IEND" {
            break;
        }
        p = fin + 4; // + CRC
    }
    None
}

/// Réglages de l'atelier contenus dans un PNG, s'il en porte.
pub fn reglages(octets: &[u8]) -> Result<Option<ReglagesAtelier>, String> {
    let Some(charge) = texte(octets, MOT_CLE) else {
        return Ok(None);
    };
    let json = base64::engine::general_purpose::STANDARD
        .decode(&charge)
        .map_err(|e| format!("bloc de réglages illisible (base64) : {e}"))?;
    let r: ReglagesAtelier = serde_json::from_slice(&json)
        .map_err(|e| format!("bloc de réglages illisible (JSON) : {e}"))?;
    if r.app != MOT_CLE {
        return Err(format!(
            "bloc « {} » : ce n'est pas un réglage de l'atelier.",
            r.app
        ));
    }
    Ok(Some(r))
}

/// Décode une URL `data:` et rend (extension, octets).
pub fn data_url(url: &str) -> Result<(&'static str, Vec<u8>), String> {
    let reste = url
        .strip_prefix("data:")
        .ok_or_else(|| "photo embarquée : ce n'est pas une URL data:".to_string())?;
    let (entete, b64) = reste
        .split_once(",")
        .ok_or_else(|| "photo embarquée : URL data: tronquée".to_string())?;
    // L'atelier rééchantillonne avant d'embarquer : le type peut être JPEG même dans
    // un PNG de couverture. Suivre le type déclaré, pas l'extension du fichier hôte.
    let ext = match entete.split(';').next().unwrap_or("") {
        "image/png" => "png",
        "image/jpeg" | "image/jpg" => "jpg",
        "image/webp" => "webp",
        autre => return Err(format!("photo embarquée de type non géré : {autre}")),
    };
    let octets = base64::engine::general_purpose::STANDARD
        .decode(b64)
        .map_err(|e| format!("photo embarquée illisible : {e}"))?;
    Ok((ext, octets))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Construit un PNG minimal portant un chunk tEXt — assez pour exercer le parcours
    /// des chunks sans embarquer un fichier binaire dans les tests.
    fn png_avec(mot_cle: &str, charge: &[u8]) -> Vec<u8> {
        let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
        let mut ajoute = |genre: &[u8], data: &[u8]| {
            out.extend((data.len() as u32).to_be_bytes());
            out.extend(genre);
            out.extend(data);
            out.extend([0u8; 4]); // CRC non vérifié à la lecture
        };
        ajoute(b"IHDR", &[0u8; 13]);
        let mut t = mot_cle.as_bytes().to_vec();
        t.push(0);
        t.extend(charge);
        ajoute(b"tEXt", &t);
        ajoute(b"IEND", &[]);
        out
    }

    #[test]
    fn le_bloc_de_l_atelier_est_retrouve_parmi_les_chunks() {
        let png = png_avec(MOT_CLE, b"charge utile");
        assert_eq!(texte(&png, MOT_CLE).unwrap(), b"charge utile");
    }

    #[test]
    fn un_chunk_d_un_autre_mot_cle_n_est_pas_confondu() {
        let png = png_avec("Software", b"autre chose");
        assert_eq!(texte(&png, MOT_CLE), None);
    }

    /// Un fichier qui n'est pas un PNG doit être écarté, et un bloc amputé ne doit pas
    /// rendre des octets tronqués qu'on prendrait pour des réglages.
    #[test]
    fn un_fichier_qui_n_est_pas_un_png_est_ecarte() {
        assert_eq!(texte(b"", MOT_CLE), None);
        assert_eq!(texte(b"pas un png du tout", MOT_CLE), None);
        // Coupé avant la fin du chunk tEXt : sa charge est incomplète, donc inutilisable.
        let png = png_avec(MOT_CLE, b"charge utile");
        for coupe in [9, 20, 50] {
            assert_eq!(texte(&png[..coupe], MOT_CLE), None, "coupé à {coupe}");
        }
    }

    /// L'utilisateur désigne ses fichiers à la main : aucune troncature, à aucun octet,
    /// ne doit faire paniquer la lecture — un plantage vaudrait perte du travail en cours.
    #[test]
    fn aucune_troncature_ne_fait_paniquer_la_lecture() {
        let png = png_avec(MOT_CLE, b"charge utile");
        for coupe in 0..=png.len() {
            let _ = texte(&png[..coupe], MOT_CLE);
        }
    }

    #[test]
    fn les_reglages_sont_decodes_depuis_le_base64() {
        let json = r#"{"app":"atelier-couverture","v":1,"mode":"band",
                       "format":[108,178],"fields":{"inFormat":"108,178"}}"#;
        let b64 = base64::engine::general_purpose::STANDARD.encode(json);
        let png = png_avec(MOT_CLE, b64.as_bytes());
        let r = reglages(&png).unwrap().unwrap();
        assert_eq!(r.mode, "band");
        assert_eq!(r.format, vec![108.0, 178.0]);
        assert_eq!(r.fields["inFormat"], "108,178");
        assert!(r.image.is_none());
    }

    /// Les versions de l'atelier n'ont pas les mêmes champs : un PNG ancien doit
    /// s'importer, pas être rejeté parce qu'il lui manque des réglages récents.
    #[test]
    fn un_bloc_d_une_version_anterieure_reste_lisible() {
        let json = r#"{"app":"atelier-couverture","fields":{"inPadX":"7"}}"#;
        let b64 = base64::engine::general_purpose::STANDARD.encode(json);
        let r = reglages(&png_avec(MOT_CLE, b64.as_bytes()))
            .unwrap()
            .unwrap();
        assert_eq!(r.fields.len(), 1);
        assert_eq!(r.v, 0);
        assert!(r.format.is_empty());
    }

    #[test]
    fn un_png_sans_bloc_n_est_pas_une_erreur() {
        let png = png_avec("Comment", b"rien a voir");
        assert!(reglages(&png).unwrap().is_none());
    }

    #[test]
    fn un_bloc_corrompu_est_signale_et_non_ignore() {
        let png = png_avec(MOT_CLE, b"ceci n'est pas du base64 !!!");
        assert!(reglages(&png).unwrap_err().contains("base64"));
    }

    /// La photo embarquée suit le type qu'elle déclare : l'atelier rééchantillonne en
    /// JPEG même quand la couverture est un PNG.
    #[test]
    fn la_photo_embarquee_suit_son_type_declare() {
        let b64 = base64::engine::general_purpose::STANDARD.encode([1u8, 2, 3]);
        let (ext, oct) = data_url(&format!("data:image/jpeg;base64,{b64}")).unwrap();
        assert_eq!(ext, "jpg");
        assert_eq!(oct, vec![1, 2, 3]);
        let (ext, _) = data_url(&format!("data:image/png;base64,{b64}")).unwrap();
        assert_eq!(ext, "png");
        assert!(data_url("pas une url").is_err());
    }
}
