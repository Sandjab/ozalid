//! Invocation du binaire Typst.
//!
//! Typst est embarqué en sidecar : un binaire statique, sans dépendance système, la
//! même version sur macOS et Windows. C'est ce qui rend la pagination reproductible
//! d'une machine à l'autre — la chaîne pandoc + WeasyPrint ne le garantissait pas.
//!
//! Deux opérations seulement, et elles sont distinctes à dessein : `pages` mesure
//! sans rien écrire, `compile` écrit le PDF. La boucle de convergence de l'intérieur
//! n'a besoin que de la première, donc elle ne produit plus de PDF jeté à chaque tour.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Étiquette du marqueur que la source doit porter pour que `pages` puisse la lire.
pub const MARQUEUR: &str = "#context [#metadata(counter(page).final().first()) <pages>]";

#[derive(Debug, Clone)]
pub struct Typst {
    binaire: PathBuf,
    /// Répertoire des polices embarquées. Sans lui, Typst n'a que les polices du
    /// système : une maquette rendrait différemment d'une machine à l'autre, ou
    /// serait substituée en silence.
    polices: Option<PathBuf>,
}

impl Typst {
    pub fn new(binaire: impl Into<PathBuf>) -> Self {
        Self {
            binaire: binaire.into(),
            polices: None,
        }
    }

    pub fn avec_polices(mut self, dossier: impl Into<PathBuf>) -> Self {
        self.polices = Some(dossier.into());
        self
    }

    /// Compte de pages final de la source, sans produire de PDF.
    /// La source doit se terminer par [`MARQUEUR`].
    pub fn pages(&self, source: &Path) -> Result<u32, String> {
        let sortie = self.lance(&[
            "eval",
            "query(<pages>).map(it => it.value)",
            "--in",
            &chemin(source)?,
        ])?;
        // `typst eval` rend un tableau Typst : « [272] ».
        sortie
            .trim()
            .trim_start_matches('[')
            .trim_end_matches(']')
            .trim()
            .parse()
            .map_err(|_| {
                format!("compte de pages illisible dans la réponse de Typst : « {sortie} »")
            })
    }

    pub fn compile(&self, source: &Path, sortie: &Path) -> Result<(), String> {
        self.lance(&["compile", &chemin(source)?, &chemin(sortie)?])
            .map(|_| ())
    }

    /// Rendu d'aperçu d'une page en PNG.
    pub fn apercu(&self, source: &Path, sortie: &Path, page: u32, ppi: u32) -> Result<(), String> {
        self.lance(&[
            "compile",
            "--format",
            "png",
            "--pages",
            &page.to_string(),
            "--ppi",
            &ppi.to_string(),
            &chemin(source)?,
            &chemin(sortie)?,
        ])
        .map(|_| ())
    }

    fn lance(&self, args: &[&str]) -> Result<String, String> {
        let mut cmd = Command::new(&self.binaire);
        cmd.args(args);
        if let Some(p) = &self.polices {
            // `--ignore-system-fonts` : sans lui, une police du poste pourrait se
            // substituer à une police embarquée et le rendu dépendrait de la machine.
            cmd.arg("--font-path").arg(p).arg("--ignore-system-fonts");
        }
        let r = cmd.output().map_err(|e| {
            format!(
                "Typst introuvable ou inexécutable ({}) : {e}",
                self.binaire.display()
            )
        })?;
        if !r.status.success() {
            // Le message de Typst est le seul indice exploitable : le remonter entier.
            return Err(String::from_utf8_lossy(&r.stderr).trim().to_string());
        }
        Ok(String::from_utf8_lossy(&r.stdout).to_string())
    }
}

fn chemin(p: &Path) -> Result<String, String> {
    p.to_str()
        .map(str::to_owned)
        .ok_or_else(|| format!("chemin non représentable : {}", p.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Un binaire absent doit produire un message qui nomme le chemin cherché : c'est
    /// la panne la plus probable au premier lancement (sidecar mal empaqueté).
    #[test]
    fn un_binaire_absent_est_signale_avec_son_chemin() {
        let t = Typst::new("/nexistepas/typst");
        let err = t.pages(Path::new("/tmp/x.typ")).unwrap_err();
        assert!(err.contains("/nexistepas/typst"), "{err}");
    }
}
