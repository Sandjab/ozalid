//! Les préférences de l'application : ce qui survit à la fermeture sans appartenir
//! à un livre.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Au-delà, la liste cesse d'être un raccourci pour devenir un historique — et le
/// sous-menu qui la porte devient illisible.
pub const MAX_RECENTS: usize = 10;

const FICHIER: &str = "preferences.toml";

/// Ce que porte `preferences.toml`.
///
/// `deny_unknown_fields` n'y figure pas volontairement : un champ écrit par une
/// version plus récente doit être ignoré, pas faire échouer la lecture. Un champ
/// perdu coûte un réglage ; une lecture refusée coûte la liste entière.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Preferences {
    #[serde(default)]
    pub recents: Vec<String>,
    /// L'adresse du modèle de diffusion et sa clé : elles appartiennent à la machine,
    /// pas au livre. Un `.ozalid` est fait pour être ouvert ailleurs — y écrire une clé
    /// la publierait au premier partage.
    ///
    /// La clé est en clair ici, avec les permissions du fichier. C'est un choix et non
    /// un oubli : le trousseau du système réclamerait une dépendance par plateforme. La
    /// contrepartie est tenue ailleurs — elle ne doit apparaître dans aucun message,
    /// aucune vue, aucune journalisation.
    #[serde(default)]
    pub diffusion: crate::diffusion::Acces,
    /// Le gabarit qu'un projet neuf reçoit : le style d'écriture qu'on ne veut pas
    /// retaper d'un livre à l'autre.
    ///
    /// Ici et non dans une maquette : une maquette dit la couverture, et un éditeur peut
    /// en avoir trois pour un seul style d'envoi — les coupler ferait recopier le même
    /// gabarit dans chacune. Ici et non dans le `.ozalid` : celui-là porte le gabarit du
    /// livre ouvert, qui compose. Celui-ci n'est qu'une valeur de départ, jamais
    /// consultée à la génération.
    ///
    /// Absent, il vaut celui de la maison. Un fichier écrit avant ce champ le reçoit
    /// donc, et c'est voulu.
    #[serde(default = "gabarit_defaut")]
    pub gabarit_defaut: String,
}

/// Le gabarit de la maison, quand l'utilisateur n'en a pas posé.
fn gabarit_defaut() -> String {
    crate::diffusion::GABARIT_DEFAUT.into()
}

impl Default for Preferences {
    /// `Default` à la main plutôt que dérivé : `#[serde(default = …)]` ne joue qu'à la
    /// désérialisation, et un `Preferences::default()` dérivé rendrait un gabarit vide
    /// là où un fichier absent rend celui de la maison. Deux chemins pour le même état
    /// initial doivent mener au même endroit.
    fn default() -> Self {
        Self {
            recents: Vec::new(),
            diffusion: crate::diffusion::Acces::default(),
            gabarit_defaut: gabarit_defaut(),
        }
    }
}

impl Preferences {
    /// Pose un projet en tête des récents, sans doublon ni débordement.
    pub fn ajouter_recent(&mut self, chemin: &Path) {
        let c = chemin.to_string_lossy().into_owned();
        self.recents.retain(|r| r != &c);
        self.recents.insert(0, c);
        self.recents.truncate(MAX_RECENTS);
    }

    /// Les récents dont le fichier existe encore.
    pub fn recents_existants(&self) -> Vec<String> {
        self.recents
            .iter()
            .filter(|r| Path::new(r).is_file())
            .cloned()
            .collect()
    }
}

pub fn fichier(config: &Path) -> PathBuf {
    config.join(FICHIER)
}

/// Lit les préférences, ou rend celles par défaut.
///
/// Aucune erreur ne remonte : absent, illisible ou corrompu, le fichier doit laisser
/// l'application démarrer.
pub fn charger(config: &Path) -> Preferences {
    std::fs::read_to_string(fichier(config))
        .ok()
        .and_then(|s| toml::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn enregistrer(config: &Path, p: &Preferences) -> Result<(), String> {
    std::fs::create_dir_all(config).map_err(|e| {
        format!(
            "répertoire de configuration inutilisable ({}) : {e}",
            config.display()
        )
    })?;
    let s =
        toml::to_string_pretty(p).map_err(|e| format!("sérialisation des préférences : {e}"))?;
    std::fs::write(fichier(config), s).map_err(|e| {
        format!(
            "écriture des préférences ({}) : {e}",
            fichier(config).display()
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chemins(p: &Preferences) -> Vec<&str> {
        p.recents.iter().map(String::as_str).collect()
    }

    /// Le dernier projet ouvert passe en tête, et n'y figure qu'une fois : une liste
    /// de raccourcis qui répète le même projet n'en est plus une.
    #[test]
    fn un_recent_deja_present_remonte_sans_se_dupliquer() {
        let mut p = Preferences::default();
        p.ajouter_recent(Path::new("/a.ozalid"));
        p.ajouter_recent(Path::new("/b.ozalid"));
        p.ajouter_recent(Path::new("/a.ozalid"));
        assert_eq!(chemins(&p), ["/a.ozalid", "/b.ozalid"]);
    }

    /// Au-delà du plafond, la liste cesserait d'être un raccourci pour devenir un
    /// historique — et le sous-menu qui la porte, illisible.
    #[test]
    fn la_liste_des_recents_est_plafonnee() {
        let mut p = Preferences::default();
        for i in 0..MAX_RECENTS + 5 {
            p.ajouter_recent(Path::new(&format!("/{i}.ozalid")));
        }
        assert_eq!(p.recents.len(), MAX_RECENTS);
        assert_eq!(p.recents[0], format!("/{}.ozalid", MAX_RECENTS + 4));
    }

    /// Un projet effacé ne doit pas être proposé : le clic échouerait, et l'échec
    /// arriverait après le clic. L'élagage se fait à la lecture, pas à l'écriture —
    /// un projet sur un volume démonté revient de lui-même au remontage, alors
    /// qu'une purge l'aurait perdu pour de bon.
    #[test]
    fn seuls_les_recents_qui_existent_encore_sont_rendus() {
        let dir = tempfile::tempdir().unwrap();
        let vivant = dir.path().join("vivant.ozalid");
        std::fs::write(&vivant, b"zip").unwrap();
        let mort = dir.path().join("mort.ozalid");

        let mut p = Preferences::default();
        p.ajouter_recent(&mort);
        p.ajouter_recent(&vivant);

        assert_eq!(p.recents.len(), 2, "la liste garde tout");
        assert_eq!(
            p.recents_existants(),
            vec![vivant.to_string_lossy().into_owned()],
            "seul ce qui existe est proposé"
        );
    }

    #[test]
    fn les_preferences_font_l_aller_retour() {
        let dir = tempfile::tempdir().unwrap();
        let mut p = Preferences::default();
        p.ajouter_recent(Path::new("/livres/heures-creuses.ozalid"));
        p.diffusion = crate::diffusion::Acces {
            url: "https://exemple.test/images".into(),
            cle: "sk-tres-secrete".into(),
        };
        enregistrer(dir.path(), &p).unwrap();
        assert_eq!(charger(dir.path()), p);
    }

    /// Un `preferences.toml` écrit avant la diffusion doit s'ouvrir sans un mot, comme
    /// un `.ozalid` écrit avant les envois : ce qui se perd ici est un confort, et un
    /// fichier refusé coûterait la liste des projets récents.
    #[test]
    fn des_preferences_sans_section_diffusion_se_relisent() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(fichier(dir.path()), b"recents = [\"/a.ozalid\"]\n").unwrap();
        let p = charger(dir.path());
        assert_eq!(p.recents, ["/a.ozalid"]);
        assert!(!p.diffusion.pret());
    }

    /// Aucune de ces trois avaries ne doit empêcher l'application de démarrer : les
    /// préférences sont un confort, pas un document.
    #[test]
    fn des_preferences_absentes_ou_corrompues_valent_le_defaut() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(
            charger(dir.path()),
            Preferences::default(),
            "fichier absent"
        );

        std::fs::write(fichier(dir.path()), b"ceci n'est pas du TOML {{{").unwrap();
        assert_eq!(
            charger(dir.path()),
            Preferences::default(),
            "fichier illisible"
        );

        std::fs::write(fichier(dir.path()), b"autre_chose = 3\n").unwrap();
        assert_eq!(charger(dir.path()), Preferences::default(), "champ inconnu");
    }

    /// Des préférences qui n'ont jamais été écrites servent le gabarit de la maison, et
    /// non un champ vide : devant un champ vide, personne ne devine qu'il existe cinq
    /// marques ni ce qu'un modèle réclame pour rendre une écriture.
    #[test]
    fn des_preferences_vierges_servent_le_gabarit_de_la_maison() {
        let p = Preferences::default();
        assert_eq!(p.gabarit_defaut, crate::diffusion::GABARIT_DEFAUT);
    }

    /// Un défaut posé par l'utilisateur remplace celui de la maison, et se relit tel
    /// quel — sauts de ligne compris, ce qu'un gabarit de diffusion porte en nombre.
    #[test]
    fn un_gabarit_pose_se_relit_ligne_pour_ligne() {
        let d = tempfile::tempdir().unwrap();
        let p = Preferences {
            gabarit_defaut: "une aquarelle\n\npour {dedicataire}".into(),
            ..Default::default()
        };
        enregistrer(d.path(), &p).unwrap();
        assert_eq!(charger(d.path()).gabarit_defaut, p.gabarit_defaut);
    }
}
