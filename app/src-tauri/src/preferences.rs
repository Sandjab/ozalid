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
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Preferences {
    #[serde(default)]
    pub recents: Vec<String>,
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
        enregistrer(dir.path(), &p).unwrap();
        assert_eq!(charger(dir.path()), p);
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
}
