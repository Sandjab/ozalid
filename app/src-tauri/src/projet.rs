//! Identité du livre.
//!
//! Elle fait foi pour la composition : le titre et l'auteur embarqués dans une image
//! de couverture sont un rendu, jamais une source. Au jalon 2, cette structure sera
//! sérialisée dans le `projet.toml` du `.ozalid` ; elle vit ici dès maintenant parce
//! que la composition de l'intérieur en dépend.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Livre {
    pub titre: String,
    /// Titre de la page de titre, avec ses sauts de ligne voulus. Absent, le titre sert.
    #[serde(default)]
    pub titre_page: Option<String>,
    pub auteur: String,
    #[serde(default = "genre_defaut")]
    pub genre: String,
    #[serde(default)]
    pub copyright: String,
    /// Contrôle d'intégrité facultatif : il n'a de sens qu'au gel du manuscrit.
    #[serde(default)]
    pub chapitres: Option<u32>,
}

fn genre_defaut() -> String {
    "roman".into()
}

impl Livre {
    /// Titre tel qu'il doit paraître sur la page de titre, sauts de ligne compris.
    pub fn titre_page(&self) -> &str {
        self.titre_page.as_deref().unwrap_or(&self.titre)
    }
}
