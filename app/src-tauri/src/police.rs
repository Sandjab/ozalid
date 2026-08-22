//! Lire une police dans son fichier : le nom que Typst lui donnera, et ce qui lui manque.
//!
//! Les trois mains embarquées ont été choisies, éprouvées, et leur nom écrit dans
//! `MAINS`. La police personnelle de l'auteur, elle, arrive telle quelle : son nom de
//! famille et sa couverture de caractères ne se relèvent **que sur le fichier**. Ni le
//! nom du fichier, ni la fiche du fondeur ne font foi — Typst désigne une police par sa
//! famille, et compose par repli, en silence, tout ce qu'elle ne porte pas.
//!
//! Même parti que `image::dimensions` : on lit les tables qui portent la réponse, sans
//! décoder la police ni tirer une bibliothèque pour le faire.

/// Ce qu'un envoi français réclame : accents, ligature, guillemets, apostrophe courbe.
///
/// La même liste que celle qui a éliminé les candidates du lot 1. Une main qui ignore
/// `À` ne le dit pas : Typst compose la lettre manquante dans une autre écriture, et
/// l'envoi part chez le dédicataire en deux mains.
pub const REQUIS: &str = "ÀÂÄÉÈÊËÇÙÛÜÔÖÎÏàâäéèêëçùûüôöîïœŒ«»’…";

/// Ce qu'on retient d'un fichier de police.
#[derive(Debug, Clone, PartialEq)]
pub struct Police {
    /// La famille telle que Typst la désignera.
    pub famille: String,
}

/// Relève la famille d'une police, et refuse ce qui ne composerait pas un envoi.
///
/// Trois refus, et chacun correspond à une composition muette : un fichier qui n'est pas
/// une police, une police que Typst ne saurait pas nommer, une police qui ne porte pas
/// le français. Aucun des trois ne se verrait avant le tirage.
pub fn examine(octets: &[u8]) -> Result<Police, String> {
    let table = |tag| table(octets, tag);
    let noms = table(b"name").ok_or(
        "ce fichier n'est pas une police TrueType ou OpenType, ou sa table de noms est \
         illisible.",
    )?;
    let famille = famille(noms).ok_or(
        "police sans nom de famille : Typst n'aurait aucun moyen de la désigner, et \
         composerait les envois dans une autre écriture.",
    )?;
    // La famille est interpolée telle quelle dans `#set text(font: "…")`, comme celle
    // des mains embarquées : un guillemet la refermerait, et la source ne compilerait
    // plus — plusieurs centaines de pages plus loin.
    if famille.contains(['"', '\\']) {
        return Err(format!(
            "nom de famille inutilisable : « {famille} » porte un guillemet ou une barre \
             oblique inverse."
        ));
    }
    let cmap = table(b"cmap")
        .ok_or("police sans table de caractères : rien ne dit ce qu'elle sait écrire.")?;
    let manque: String = REQUIS.chars().filter(|c| !couvre(cmap, *c)).collect();
    if !manque.is_empty() {
        return Err(format!(
            "« {famille} » ne porte pas {manque} : Typst composerait ces caractères-là \
             dans une autre écriture, sans le dire, et l'envoi partirait en deux mains."
        ));
    }
    Ok(Police { famille })
}

/// Le contenu d'une table sfnt, si le fichier en est une et qu'elle s'y trouve.
fn table<'a>(o: &'a [u8], tag: &[u8; 4]) -> Option<&'a [u8]> {
    let version = u32a(o, 0)?;
    // TrueType (0x00010000), OpenType (« OTTO »), et le vieux TrueType Mac (« true »).
    // Une collection (« ttcf ») porte plusieurs polices : rien ne dirait laquelle
    // l'auteur veut, donc elle n'est pas une police au sens de ce module.
    if !matches!(version, 0x0001_0000 | 0x4F54_544F | 0x7472_7565) {
        return None;
    }
    let n = u16a(o, 4)? as usize;
    (0..n).find_map(|i| {
        let r = 12 + 16 * i;
        (o.get(r..r + 4)? == tag).then_some(())?;
        let debut = u32a(o, r + 8)? as usize;
        let long = u32a(o, r + 12)? as usize;
        o.get(debut..debut.checked_add(long)?)
    })
}

/// La famille déclarée par la table `name`.
///
/// L'ordre — nom typographique (16) d'abord, nom de famille (1) ensuite — est celui de
/// Typst. Le suivre est ce qui garantit que le nom écrit dans la source est celui que le
/// moteur cherchera : en prendre un autre reviendrait à désigner une police absente,
/// c'est-à-dire à composer par repli.
fn famille(name: &[u8]) -> Option<String> {
    let count = u16a(name, 2)? as usize;
    let magasin = u16a(name, 4)? as usize;
    let lire = |voulu: u16| {
        (0..count)
            .filter_map(|i| {
                let r = 6 + 12 * i;
                let plateforme = u16a(name, r)?;
                if u16a(name, r + 6)? != voulu {
                    return None;
                }
                let long = u16a(name, r + 8)? as usize;
                let debut = magasin + u16a(name, r + 10)? as usize;
                let brut = name.get(debut..debut.checked_add(long)?)?;
                let nom = decode(plateforme, brut)?;
                (!nom.trim().is_empty()).then(|| (plateforme, nom.trim().to_string()))
            })
            // Windows (3) et Unicode (0) portent l'UTF-16 : à égalité de nom, ils
            // disent mieux qu'un Mac Roman réduit à l'ASCII.
            .min_by_key(|(plateforme, _)| match plateforme {
                3 => 0,
                0 => 1,
                _ => 2,
            })
            .map(|(_, nom)| nom)
    };
    lire(16).or_else(|| lire(1))
}

fn decode(plateforme: u16, brut: &[u8]) -> Option<String> {
    match plateforme {
        0 | 3 => {
            if !brut.len().is_multiple_of(2) {
                return None;
            }
            let unites = brut
                .chunks_exact(2)
                .map(|c| u16::from_be_bytes([c[0], c[1]]));
            char::decode_utf16(unites)
                .collect::<Result<String, _>>()
                .ok()
        }
        // Mac Roman : identique à l'ASCII sur ses 128 premiers points, et au-delà rien
        // ne dit lequel des jeux Mac est en vigueur. Un nom de famille non ASCII sur
        // cette plateforme-là est donc laissé aux autres enregistrements.
        1 => brut
            .is_ascii()
            .then(|| String::from_utf8_lossy(brut).into_owned()),
        _ => None,
    }
}

/// La police porte-t-elle ce caractère ?
///
/// L'union de ses sous-tables : une police en porte plusieurs, qui se recouvrent. Les
/// formats 4 (le format universel) et 12 (celui des polices au-delà du plan de base)
/// suffisent — un caractère absent des deux n'est porté par aucune police moderne.
fn couvre(cmap: &[u8], c: char) -> bool {
    let Some(n) = u16a(cmap, 2).map(usize::from) else {
        return false;
    };
    (0..n).any(|i| {
        let r = 4 + 8 * i;
        let Some(debut) = u32a(cmap, r + 4).map(|d| d as usize) else {
            return false;
        };
        let Some(sous) = cmap.get(debut..) else {
            return false;
        };
        match u16a(sous, 0) {
            Some(4) => format4(sous, c),
            Some(12) => format12(sous, c),
            _ => false,
        }
    })
}

/// Format 4 : segments à delta, avec un tableau de glyphes pour les segments irréguliers.
///
/// Le glyphe est calculé pour de bon, et non déduit de l'appartenance à un segment : un
/// caractère peut tomber dans un segment déclaré et y être associé au glyphe 0, qui est
/// précisément le glyphe « absent ». C'est la différence entre « la police prétend » et
/// « la police porte ».
fn format4(t: &[u8], c: char) -> bool {
    let point = c as u32;
    if point > 0xFFFF {
        return false;
    }
    let point = point as u16;
    let Some(segs) = u16a(t, 6).map(|s| s as usize / 2) else {
        return false;
    };
    let fin = 14;
    let debut = fin + segs * 2 + 2;
    let delta = debut + segs * 2;
    let ecart = delta + segs * 2;
    for i in 0..segs {
        let (Some(f), Some(d)) = (u16a(t, fin + i * 2), u16a(t, debut + i * 2)) else {
            return false;
        };
        if f < point {
            continue;
        }
        if d > point {
            return false;
        }
        let (Some(dl), Some(ec)) = (u16a(t, delta + i * 2), u16a(t, ecart + i * 2)) else {
            return false;
        };
        let glyphe = if ec == 0 {
            point.wrapping_add(dl)
        } else {
            // L'écart est compté depuis sa propre case, en octets : c'est ce qui rend
            // ce format illisible, et c'est aussi ce qui le rend compact.
            let ou = ecart + i * 2 + ec as usize + (point - d) as usize * 2;
            match u16a(t, ou) {
                Some(0) | None => return false,
                Some(g) => g.wrapping_add(dl),
            }
        };
        return glyphe != 0;
    }
    false
}

/// Format 12 : groupes d'intervalles, sans repli ni delta.
fn format12(t: &[u8], c: char) -> bool {
    let point = c as u32;
    let Some(n) = u32a(t, 12).map(|n| n as usize) else {
        return false;
    };
    (0..n).any(|i| {
        let r = 16 + 12 * i;
        let (Some(d), Some(f), Some(g)) = (u32a(t, r), u32a(t, r + 4), u32a(t, r + 8)) else {
            return false;
        };
        (d..=f).contains(&point) && g + (point - d) != 0
    })
}

fn u16a(o: &[u8], p: usize) -> Option<u16> {
    Some(u16::from_be_bytes(o.get(p..p + 2)?.try_into().ok()?))
}

fn u32a(o: &[u8], p: usize) -> Option<u32> {
    Some(u32::from_be_bytes(o.get(p..p + 4)?.try_into().ok()?))
}

#[cfg(test)]
mod tests {
    use super::*;

    const CAVEAT: &[u8] = include_bytes!("../fonts/Caveat[wght].ttf");
    const GARAMOND: &[u8] = include_bytes!("../fonts/EBGaramond[wght].ttf");

    /// Le relevé se fait sur de vrais fichiers, pas sur des maquettes : c'est là-dessus
    /// que porte la promesse. Caveat n'a pas de nom typographique (16), seulement un nom
    /// de famille (1) ; EB Garamond porte en plus une sous-table de format 12. Les deux
    /// chemins de lecture sont donc réellement empruntés.
    #[test]
    fn les_polices_de_la_maison_donnent_le_nom_que_typst_cherche() {
        assert_eq!(examine(CAVEAT).unwrap().famille, "Caveat");
        assert_eq!(examine(GARAMOND).unwrap().famille, "EB Garamond");
    }

    /// Un manuscrit, une image, un fichier tronqué : rien de tout cela n'est une police,
    /// et le dire au moment du choix vaut mieux que de composer une écriture de repli.
    #[test]
    fn ce_qui_n_est_pas_une_police_est_refuse() {
        for creux in [
            b"## 01 - Un\n\nTexte.\n".to_vec(),
            b"\x89PNG\r\n\x1a\n".to_vec(),
            CAVEAT[..40].to_vec(),
            Vec::new(),
        ] {
            let e = examine(&creux).unwrap_err();
            // La phrase exacte, et non le mot « police » : tous les refus de ce module
            // le portent, si bien qu'un contrôle plus lâche serait vrai même si le
            // fichier avait été pris pour une police sans nom.
            assert!(e.contains("n'est pas une police"), "{e}");
        }
    }

    /// Une police sans accents composerait l'envoi en deux écritures : celle de l'auteur
    /// pour ce qu'elle porte, celle de repli pour le reste. Cela ne se voit pas dans un
    /// compte de pages, et l'erreur doit nommer ce qui manque.
    #[test]
    fn une_police_sans_accents_est_refusee_en_nommant_ce_qui_manque() {
        let sans = fonte("Ma Main", "abcABC’«»…œŒçàâäéèêëùûüôöîï");
        let e = examine(&sans).unwrap_err();
        assert!(e.contains("Ma Main"), "la police n'est pas nommée : {e}");
        assert!(e.contains('À'), "ce qui manque n'est pas dit : {e}");

        let avec = fonte("Ma Main", REQUIS);
        assert_eq!(examine(&avec).unwrap().famille, "Ma Main");
    }

    /// Le nom de famille est interpolé sans échappement dans la source, comme celui des
    /// mains embarquées : un guillemet refermerait la chaîne de `#set text(font: …)` et
    /// ferait échouer la compilation du livre entier.
    #[test]
    fn un_nom_de_famille_qui_casserait_la_source_est_refuse() {
        let e = examine(&fonte("Ma \"Main\"", REQUIS)).unwrap_err();
        assert!(e.contains("guillemet"), "{e}");
    }

    /// Un fichier de police minimal : une table `name` (nom de famille en UTF-16BE) et
    /// une table `cmap` de format 12, un groupe par caractère. C'est le seul moyen
    /// d'éprouver le refus : aucune des trente-deux polices de la maison n'échoue, elles
    /// ont toutes été relevées avant d'entrer.
    fn fonte(famille: &str, caracteres: &str) -> Vec<u8> {
        let mut nom: Vec<u8> = Vec::new();
        let utf16: Vec<u8> = famille.encode_utf16().flat_map(u16::to_be_bytes).collect();
        nom.extend(0u16.to_be_bytes()); // format
        nom.extend(1u16.to_be_bytes()); // count
        nom.extend(18u16.to_be_bytes()); // début du magasin de chaînes
        nom.extend(3u16.to_be_bytes()); // plateforme Windows
        nom.extend(1u16.to_be_bytes()); // encodage
        nom.extend(0x0409u16.to_be_bytes()); // langue
        nom.extend(1u16.to_be_bytes()); // nameID : famille
        nom.extend((utf16.len() as u16).to_be_bytes());
        nom.extend(0u16.to_be_bytes()); // décalage dans le magasin
        nom.extend(&utf16);

        let points: Vec<u32> = caracteres.chars().map(|c| c as u32).collect();
        let mut sous: Vec<u8> = Vec::new();
        sous.extend(12u16.to_be_bytes()); // format
        sous.extend(0u16.to_be_bytes()); // réservé
        sous.extend(0u32.to_be_bytes()); // longueur, ignorée à la lecture
        sous.extend(0u32.to_be_bytes()); // langue
        sous.extend((points.len() as u32).to_be_bytes());
        for (i, p) in points.iter().enumerate() {
            sous.extend(p.to_be_bytes());
            sous.extend(p.to_be_bytes());
            sous.extend((i as u32 + 1).to_be_bytes());
        }
        let mut cmap: Vec<u8> = Vec::new();
        cmap.extend(0u16.to_be_bytes()); // version
        cmap.extend(1u16.to_be_bytes()); // une sous-table
        cmap.extend(3u16.to_be_bytes()); // plateforme Windows
        cmap.extend(10u16.to_be_bytes()); // encodage UCS-4
        cmap.extend(12u32.to_be_bytes()); // son décalage
        cmap.extend(&sous);

        let mut f: Vec<u8> = Vec::new();
        f.extend(0x0001_0000u32.to_be_bytes());
        f.extend(2u16.to_be_bytes()); // deux tables
        f.extend([0u8; 6]); // searchRange, entrySelector, rangeShift
        let debut = 12 + 32;
        for (tag, contenu) in [(b"cmap", &cmap), (b"name", &nom)] {
            f.extend(tag);
            f.extend(0u32.to_be_bytes()); // somme de contrôle, ignorée à la lecture
            f.extend(
                (debut as u32 + if tag == b"name" { cmap.len() as u32 } else { 0 }).to_be_bytes(),
            );
            f.extend((contenu.len() as u32).to_be_bytes());
        }
        f.extend(&cmap);
        f.extend(&nom);
        f
    }
}
