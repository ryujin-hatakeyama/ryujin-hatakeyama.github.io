use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};

use anyhow::{Result, bail, ensure};

/// A relative output path which cannot escape the build root.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct ArtifactPath(PathBuf);

impl ArtifactPath {
    pub(crate) fn new(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        ensure!(
            !path.as_os_str().is_empty(),
            "artifact path must not be empty"
        );
        ensure!(!path.is_absolute(), "artifact path must be relative");
        ensure!(
            path.components()
                .all(|component| matches!(component, Component::Normal(_))),
            "artifact path must contain only normal path components: {}",
            path.display()
        );
        Ok(Self(path.to_owned()))
    }

    pub(crate) fn as_path(&self) -> &Path {
        &self.0
    }
}

/// A finite map of validated output paths to bytes.
///
/// Composition is partial: it succeeds exactly when no path is equal to, an
/// ancestor of, or a descendant of a path in the other set.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct ArtifactSet {
    entries: BTreeMap<ArtifactPath, Vec<u8>>,
}

impl ArtifactSet {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn insert(
        &mut self,
        path: impl AsRef<Path>,
        contents: impl Into<Vec<u8>>,
    ) -> Result<()> {
        let path = ArtifactPath::new(path)?;
        if let Some(existing) = self
            .entries
            .keys()
            .find(|existing| paths_conflict(existing.as_path(), path.as_path()))
        {
            bail!(
                "artifact path conflict between {} and {}",
                existing.as_path().display(),
                path.as_path().display()
            );
        }
        self.entries.insert(path, contents.into());
        Ok(())
    }

    pub(crate) fn compose(mut self, other: Self) -> Result<Self> {
        for (path, contents) in other.entries {
            self.insert(path.as_path(), contents)?;
        }
        Ok(self)
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = (&Path, &[u8])> {
        self.entries
            .iter()
            .map(|(path, contents)| (path.as_path(), contents.as_slice()))
    }
}

fn paths_conflict(left: &Path, right: &Path) -> bool {
    left == right || left.starts_with(right) || right.starts_with(left)
}

#[cfg(test)]
mod tests {
    use super::ArtifactSet;

    fn set(entries: &[(&str, &str)]) -> ArtifactSet {
        let mut set = ArtifactSet::new();
        for (path, contents) in entries {
            set.insert(path, contents.as_bytes().to_vec()).unwrap();
        }
        set
    }

    #[test]
    fn empty_set_is_a_two_sided_identity() {
        let artifacts = set(&[("a.txt", "a"), ("nested/b.txt", "b")]);
        assert_eq!(
            ArtifactSet::new().compose(artifacts.clone()).unwrap(),
            artifacts
        );
        assert_eq!(
            artifacts.clone().compose(ArtifactSet::new()).unwrap(),
            artifacts
        );
    }

    #[test]
    fn composition_is_associative_for_compatible_sets() {
        let a = set(&[("a.txt", "a")]);
        let b = set(&[("b.txt", "b")]);
        let c = set(&[("nested/c.txt", "c")]);

        let left = a
            .clone()
            .compose(b.clone())
            .unwrap()
            .compose(c.clone())
            .unwrap();
        let right = a.compose(b.compose(c).unwrap()).unwrap();
        assert_eq!(left, right);
    }

    #[test]
    fn composition_is_commutative_for_compatible_sets() {
        let a = set(&[("a.txt", "a")]);
        let b = set(&[("nested/b.txt", "b")]);
        assert_eq!(a.clone().compose(b.clone()).unwrap(), b.compose(a).unwrap());
    }

    #[test]
    fn composition_rejects_equal_and_ancestor_paths() {
        assert!(
            set(&[("same.txt", "a")])
                .compose(set(&[("same.txt", "b")]))
                .is_err()
        );
        assert!(
            set(&[("assets", "a")])
                .compose(set(&[("assets/site.css", "b")]))
                .is_err()
        );
    }

    #[test]
    fn artifact_paths_cannot_escape_the_output_root() {
        let mut artifacts = ArtifactSet::new();
        assert!(artifacts.insert("../outside", Vec::new()).is_err());
        assert!(artifacts.insert("/absolute", Vec::new()).is_err());
        assert!(artifacts.insert("", Vec::new()).is_err());
    }
}
