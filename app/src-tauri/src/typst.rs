//! Invocation du binaire Typst.
//!
//! Typst est embarqué en sidecar : un binaire statique, sans dépendance système, la
//! même version sur macOS et Windows. C'est ce qui rend la pagination reproductible
//! d'une machine à l'autre — la chaîne pandoc + WeasyPrint ne le garantissait pas.
//!
//! Les opérations qui n'écrivent rien et celle qui écrit sont distinctes à dessein :
//! `pages` et `mesures` évaluent le document sans produire de fichier, `compile` écrit
//! le PDF. La boucle de convergence de l'intérieur n'a besoin que de `pages`, donc elle
//! ne produit plus de PDF jeté à chaque tour ; l'aperçu du dos n'a besoin que de
//! `mesures`, et il les obtient pour un dixième du prix d'une composition.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Étiquette du marqueur que la source doit porter pour que `pages` puisse la lire.
pub const MARQUEUR: &str = "#context [#metadata(counter(page).final().first()) <pages>]";

#[derive(Debug, Clone)]
pub struct Typst {
    binaire: PathBuf,
    /// Répertoires de polices. Sans eux, Typst n'a que les polices du système : une
    /// maquette rendrait différemment d'une machine à l'autre, ou serait substituée en
    /// silence.
    ///
    /// Plusieurs, parce qu'un livre peut composer ses envois dans l'écriture de son
    /// auteur : celle-là vit dans le `.ozalid`, pas dans le binaire, et elle est
    /// dépliée à côté des sources au moment de composer.
    polices: Vec<PathBuf>,
}

impl Typst {
    pub fn new(binaire: impl Into<PathBuf>) -> Self {
        Self {
            binaire: binaire.into(),
            polices: Vec::new(),
        }
    }

    pub fn avec_polices(mut self, dossier: impl Into<PathBuf>) -> Self {
        self.polices.push(dossier.into());
        self
    }

    /// Les répertoires de polices, pour qui a besoin des fichiers eux-mêmes.
    ///
    /// L'EPUB embarque la police du livre : elle doit être **celle que Typst a
    /// employée**, donc venir des mêmes répertoires. En chercher d'autres embarquerait
    /// une écriture que la composition n'a pas vue.
    pub fn polices(&self) -> &[PathBuf] {
        &self.polices
    }

    /// Compte de pages final de la source, sans produire de PDF.
    /// La source doit se terminer par [`MARQUEUR`].
    pub fn pages(&self, source: &Path) -> Result<u32, String> {
        let (sortie, _) = self.lance(&[
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

    /// Les mesures qu'une source publie sous `<mesures>`, en JSON.
    ///
    /// Rien n'est composé : `typst eval` évalue le document et rend la métadonnée. Une
    /// longueur de texte ne se calcule pas autrement — elle dépend de chaque glyphe, et
    /// seul Typst tient les métriques des polices embarquées. Mesuré à une quinzaine de
    /// millisecondes, contre cent dix pour un aperçu.
    pub fn mesures(&self, source: &Path) -> Result<BTreeMap<String, f64>, String> {
        let (sortie, _) = self.lance(&[
            "eval",
            "query(<mesures>).map(it => it.value)",
            "--in",
            &chemin(source)?,
            "--format",
            "json",
        ])?;
        // `typst eval` rend le tableau des métadonnées trouvées : une seule ici.
        let tableau: Vec<BTreeMap<String, f64>> = serde_json::from_str(&sortie)
            .map_err(|e| format!("mesures illisibles dans la réponse de Typst : {e}"))?;
        Ok(tableau.into_iter().next().unwrap_or_default())
    }

    /// Compile la source en PDF, et rend les familles de police que Typst n'a pas
    /// trouvées — celles qu'il a remplacées par une écriture de repli, en sortant
    /// quand même en succès. Vide, tout va bien ; sinon, le PDF existe mais ne
    /// ressemble pas à la maquette, et le taire enverrait ce rendu-là à l'impression.
    pub fn compile(&self, source: &Path, sortie: &Path) -> Result<Vec<String>, String> {
        self.lance(&["compile", &chemin(source)?, &chemin(sortie)?])
            .map(|(_, stderr)| familles_introuvables(&stderr))
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

    /// Rend `(stdout, stderr)` d'une invocation réussie : Typst peut réussir en
    /// avertissant — la substitution de police, notamment — et une application
    /// graphique n'a pas de console où ce `stderr` se lirait tout seul.
    fn lance(&self, args: &[&str]) -> Result<(String, String), String> {
        let mut cmd = Command::new(&self.binaire);
        cmd.args(args);
        if !self.polices.is_empty() {
            for p in &self.polices {
                cmd.arg("--font-path").arg(p);
            }
            // `--ignore-system-fonts` : sans lui, une police du poste pourrait se
            // substituer à une police embarquée et le rendu dépendrait de la machine.
            cmd.arg("--ignore-system-fonts");
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
        Ok((
            String::from_utf8_lossy(&r.stdout).to_string(),
            String::from_utf8_lossy(&r.stderr).to_string(),
        ))
    }
}

/// Familles que Typst a remplacées sans échouer, relevées sur son `stderr` :
/// « warning: unknown font family: plume fantome », répété à chaque endroit de la
/// source qui demande la famille — d'où le dédoublonnage, dans l'ordre d'apparition.
fn familles_introuvables(stderr: &str) -> Vec<String> {
    let mut familles: Vec<String> = Vec::new();
    for ligne in stderr.lines() {
        if let Some(famille) = ligne.strip_prefix("warning: unknown font family: ") {
            let famille = famille.trim();
            if !familles.iter().any(|f| f == famille) {
                familles.push(famille.to_string());
            }
        }
    }
    familles
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

    /// Le `stderr` ci-dessous est celui du sidecar, relevé tel quel : quand une famille
    /// manque, Typst compose dans une écriture de repli, sort en code 0 et ne le dit
    /// que là. Chaque famille doit être relevée une fois — elle est répétée à chaque
    /// endroit de la source qui la demande — et le cadre du warning (`┌─`, `│`) ne
    /// doit pas passer pour une famille.
    #[test]
    fn les_familles_composees_par_repli_sont_relevees_sur_stderr() {
        let stderr = "\
warning: unknown font family: plume fantome
  ┌─ repli.typ:1:16
  │
1 │ #set text(font: \"Plume Fantome\")
  │                 ^^^^^^^^^^^^^^^

warning: unknown font family: autre absente
  ┌─ repli.typ:3:12

warning: unknown font family: plume fantome
  ┌─ repli.typ:5:16
";
        assert_eq!(
            familles_introuvables(stderr),
            vec!["plume fantome", "autre absente"]
        );
    }

    /// Un `stderr` vide — le cas de toutes les compositions saines — ne relève rien.
    #[test]
    fn un_stderr_sain_ne_releve_aucune_famille() {
        assert!(familles_introuvables("").is_empty());
    }

    /// `ebook` doit lire les fichiers de police pour en embarquer deux dans l'EPUB : les
    /// répertoires que Typst connaît sont les seuls qui fassent foi — s'en chercher
    /// d'autres embarquerait une police que la composition n'a pas employée.
    #[test]
    fn les_repertoires_de_polices_sont_lisibles_du_dehors() {
        let t = Typst::new("/x/typst")
            .avec_polices("/a/fonts")
            .avec_polices("/b/fonts");
        let p: Vec<_> = t
            .polices()
            .iter()
            .map(|p| p.display().to_string())
            .collect();
        assert_eq!(p, vec!["/a/fonts", "/b/fonts"]);
    }
}
