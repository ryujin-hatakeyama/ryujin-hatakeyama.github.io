use std::ffi::OsString;
use std::fs::{self, OpenOptions};
use std::io::{ErrorKind, Write};
use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result, bail, ensure};

use crate::artifact::ArtifactSet;
use crate::validate;

const WORKSPACE_MARKER: &str = "site-builder transaction workspace v1\n";

pub(crate) fn prepare_destination(root: &Path, output_relative: &Path) -> Result<PathBuf> {
    validate_output_path(output_relative)?;
    let root_metadata = fs::symlink_metadata(root)
        .with_context(|| format!("failed to inspect repository root {}", root.display()))?;
    ensure!(
        root_metadata.is_dir() && !root_metadata.file_type().is_symlink(),
        "repository root must be a real directory: {}",
        root.display()
    );

    let mut parent = root.to_owned();
    if let Some(relative_parent) = output_relative.parent() {
        for component in relative_parent.components() {
            let Component::Normal(name) = component else {
                unreachable!("output path was validated")
            };
            parent.push(name);
            match fs::symlink_metadata(&parent) {
                Ok(metadata) => ensure!(
                    metadata.is_dir() && !metadata.file_type().is_symlink(),
                    "output parent must be a real directory: {}",
                    parent.display()
                ),
                Err(error) if error.kind() == ErrorKind::NotFound => {
                    fs::create_dir(&parent)
                        .with_context(|| format!("failed to create {}", parent.display()))?;
                }
                Err(error) => {
                    return Err(error)
                        .with_context(|| format!("failed to inspect {}", parent.display()));
                }
            }
        }
    }

    let output = root.join(output_relative);
    if let Some(metadata) = metadata_if_exists(&output)? {
        ensure!(
            metadata.is_dir() && !metadata.file_type().is_symlink(),
            "output path must be a real directory when it exists: {}",
            output.display()
        );
        ensure_tree_has_no_symlinks(&output)?;
    }
    Ok(output)
}

fn validate_output_path(path: &Path) -> Result<()> {
    ensure!(
        !path.as_os_str().is_empty(),
        "output path must not be empty"
    );
    ensure!(
        !path.is_absolute(),
        "output path must be relative to the repository"
    );
    ensure!(
        path.components()
            .all(|component| matches!(component, Component::Normal(_))),
        "output path must not contain parent, root, or current-directory components"
    );
    ensure!(
        path == Path::new("dist") || path.starts_with("target/"),
        "output path must be dist or a directory below target"
    );
    Ok(())
}

fn ensure_tree_has_no_symlinks(directory: &Path) -> Result<()> {
    let mut entries = fs::read_dir(directory)
        .with_context(|| format!("failed to read {}", directory.display()))?
        .collect::<std::io::Result<Vec<_>>>()?;
    entries.sort_by_key(std::fs::DirEntry::file_name);
    for entry in entries {
        let path = entry.path();
        let file_type = entry
            .file_type()
            .with_context(|| format!("failed to inspect {}", path.display()))?;
        ensure!(
            !file_type.is_symlink(),
            "output tree must not contain symbolic links: {}",
            path.display()
        );
        if file_type.is_dir() {
            ensure_tree_has_no_symlinks(&path)?;
        } else {
            ensure!(
                file_type.is_file(),
                "output tree contains unsupported entry: {}",
                path.display()
            );
        }
    }
    Ok(())
}

pub(crate) struct StagedOutput {
    workspace: BuildWorkspace,
}

impl StagedOutput {
    pub(crate) fn generate(output: &Path, artifacts: ArtifactSet) -> Result<Self> {
        let workspace = BuildWorkspace::prepare(output)?;
        for (relative, contents) in artifacts.iter() {
            let path = workspace.stage.join(relative);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("failed to create {}", parent.display()))?;
            }
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)
                .with_context(|| format!("failed to create {}", path.display()))?;
            file.write_all(contents)
                .with_context(|| format!("failed to write {}", path.display()))?;
        }
        Ok(Self { workspace })
    }

    pub(crate) fn verify(self, source_root: &Path) -> Result<VerifiedOutput> {
        validate::dist(&self.workspace.stage, source_root)?;
        Ok(VerifiedOutput { staged: self })
    }
}

pub(crate) struct VerifiedOutput {
    staged: StagedOutput,
}

impl VerifiedOutput {
    pub(crate) fn install(self) -> Result<InstallReport> {
        self.install_with(&FilesystemRenamer)
    }

    fn install_with(self, renamer: &impl Renamer) -> Result<InstallReport> {
        let mut workspace = self.staged.workspace;
        let output_metadata = metadata_if_exists(&workspace.output)?;
        if let Some(metadata) = &output_metadata {
            ensure!(
                metadata.is_dir() && !metadata.file_type().is_symlink(),
                "output changed during the build and is no longer a real directory: {}",
                workspace.output.display()
            );
        }
        let output_exists = output_metadata.is_some();

        if !output_exists {
            renamer
                .rename(&workspace.stage, &workspace.output)
                .with_context(|| {
                    format!(
                        "failed to install staged output {} at {}",
                        workspace.stage.display(),
                        workspace.output.display()
                    )
                })?;
            return Ok(workspace.finish_installation());
        }

        ensure!(
            metadata_if_exists(&workspace.backup)?.is_none(),
            "transaction backup already exists: {}",
            workspace.backup.display()
        );
        renamer
            .rename(&workspace.output, &workspace.backup)
            .with_context(|| {
                format!(
                    "failed to move previous output {} to recovery backup {}",
                    workspace.output.display(),
                    workspace.backup.display()
                )
            })?;

        if let Err(install_error) = renamer.rename(&workspace.stage, &workspace.output) {
            match renamer.rename(&workspace.backup, &workspace.output) {
                Ok(()) => {
                    return Err(install_error).with_context(|| {
                        format!(
                            "failed to install staged output; restored previous output at {}",
                            workspace.output.display()
                        )
                    });
                }
                Err(rollback_error) => {
                    workspace.cleanup_on_drop = false;
                    bail!(
                        "failed to install staged output ({install_error}); rollback also failed ({rollback_error}). Previous output remains at {} and staged output remains at {}",
                        workspace.backup.display(),
                        workspace.stage.display()
                    );
                }
            }
        }

        Ok(workspace.finish_installation())
    }
}

#[derive(Debug)]
pub(crate) struct InstallReport {
    pub(crate) warning: Option<String>,
}

trait Renamer {
    fn rename(&self, from: &Path, to: &Path) -> std::io::Result<()>;
}

struct FilesystemRenamer;

impl Renamer for FilesystemRenamer {
    fn rename(&self, from: &Path, to: &Path) -> std::io::Result<()> {
        fs::rename(from, to)
    }
}

struct BuildWorkspace {
    output: PathBuf,
    work: PathBuf,
    stage: PathBuf,
    backup: PathBuf,
    cleanup_on_drop: bool,
}

impl BuildWorkspace {
    fn prepare(output: &Path) -> Result<Self> {
        let parent = output
            .parent()
            .context("output must have a parent directory")?;
        let file_name = output.file_name().context("output must have a file name")?;
        let mut work_name = OsString::from(".");
        work_name.push(file_name);
        work_name.push(".site-builder-work");
        let work = parent.join(work_name);
        ensure_workspace_available(&work, output)?;

        fs::create_dir(&work).with_context(|| {
            format!("failed to create transaction workspace {}", work.display())
        })?;
        let stage = work.join("stage");
        let backup = work.join("backup");
        let initialize = || -> Result<()> {
            fs::write(work.join("owner"), WORKSPACE_MARKER).with_context(|| {
                format!("failed to mark transaction workspace {}", work.display())
            })?;
            fs::create_dir(&stage).with_context(|| {
                format!("failed to create staging directory {}", stage.display())
            })?;
            Ok(())
        };
        if let Err(error) = initialize() {
            let cleanup = fs::remove_dir_all(&work);
            return match cleanup {
                Ok(()) => Err(error),
                Err(cleanup_error) => Err(error.context(format!(
                    "workspace initialization also failed to clean {}: {cleanup_error}",
                    work.display()
                ))),
            };
        }

        Ok(Self {
            output: output.to_owned(),
            work,
            stage,
            backup,
            cleanup_on_drop: true,
        })
    }

    fn finish_installation(&mut self) -> InstallReport {
        let mut warnings = Vec::new();
        match metadata_if_exists(&self.backup) {
            Ok(Some(_)) => {
                if let Err(error) = fs::remove_dir_all(&self.backup) {
                    warnings.push(format!(
                        "new output is installed, but the previous output could not be removed from {}: {error}. Keep it as a recovery copy until it can be removed manually",
                        self.backup.display()
                    ));
                    self.cleanup_on_drop = false;
                    return InstallReport {
                        warning: Some(warnings.join("; ")),
                    };
                }
            }
            Ok(None) => {}
            Err(error) => {
                warnings.push(format!(
                    "new output is installed, but the recovery backup at {} could not be inspected: {error}. Inspect the transaction workspace manually",
                    self.backup.display()
                ));
                self.cleanup_on_drop = false;
                return InstallReport {
                    warning: Some(warnings.join("; ")),
                };
            }
        }
        if let Err(error) = fs::remove_dir_all(&self.work) {
            warnings.push(format!(
                "new output is installed, but transaction workspace {} could not be removed: {error}",
                self.work.display()
            ));
            self.cleanup_on_drop = false;
        } else {
            self.cleanup_on_drop = false;
        }
        InstallReport {
            warning: (!warnings.is_empty()).then(|| warnings.join("; ")),
        }
    }
}

impl Drop for BuildWorkspace {
    fn drop(&mut self) {
        if self.cleanup_on_drop && metadata_if_exists(&self.backup).ok().flatten().is_none() {
            let _ = fs::remove_dir_all(&self.work);
        }
    }
}

fn ensure_workspace_available(work: &Path, output: &Path) -> Result<()> {
    let Some(metadata) = metadata_if_exists(work)? else {
        return Ok(());
    };
    ensure!(
        metadata.is_dir() && !metadata.file_type().is_symlink(),
        "transaction workspace is not a real directory: {}",
        work.display()
    );
    let owner = work.join("owner");
    let owner_metadata = fs::symlink_metadata(&owner)
        .with_context(|| format!("unrecognized transaction workspace {}", work.display()))?;
    ensure!(
        owner_metadata.is_file() && !owner_metadata.file_type().is_symlink(),
        "unrecognized transaction workspace {}",
        work.display()
    );
    ensure!(
        fs::read_to_string(&owner)? == WORKSPACE_MARKER,
        "refusing to remove unrecognized transaction workspace {}",
        work.display()
    );

    let backup = work.join("backup");
    if metadata_if_exists(&backup)?.is_some() {
        if metadata_if_exists(output)?.is_none() {
            bail!(
                "a build is active or was interrupted during installation. The previous output is at {} and can be restored by renaming it to {}; staged files and the workspace may then be removed",
                backup.display(),
                output.display()
            );
        } else {
            bail!(
                "a build is active or was interrupted after installation, leaving output {} and recovery backup {}. Validate the output before removing the backup and transaction workspace",
                output.display(),
                backup.display()
            );
        }
    }
    bail!(
        "transaction workspace {} already exists; another build may be active. If no build is running, it is safe to remove this marked workspace because installation has not started",
        work.display()
    )
}

fn metadata_if_exists(path: &Path) -> Result<Option<fs::Metadata>> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => Ok(Some(metadata)),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error).with_context(|| format!("failed to inspect {}", path.display())),
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::{
        BuildWorkspace, Renamer, StagedOutput, VerifiedOutput, prepare_destination,
        validate_output_path,
    };

    static NEXT_TEST_DIRECTORY: AtomicU64 = AtomicU64::new(0);

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new(label: &str) -> Self {
            let sequence = NEXT_TEST_DIRECTORY.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "site-builder-publish-{label}-{}-{sequence}",
                std::process::id()
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn write(path: &Path, contents: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }

    #[test]
    fn limits_replaceable_output_paths() {
        assert!(validate_output_path(Path::new("dist")).is_ok());
        assert!(validate_output_path(Path::new("target/determinism-dist")).is_ok());
        assert!(validate_output_path(Path::new(".")).is_err());
        assert!(validate_output_path(Path::new("../outside")).is_err());
        assert!(validate_output_path(Path::new("content")).is_err());
    }

    struct FailSecondRename {
        calls: Cell<usize>,
    }

    impl Renamer for FailSecondRename {
        fn rename(&self, from: &Path, to: &Path) -> std::io::Result<()> {
            let call = self.calls.get() + 1;
            self.calls.set(call);
            if call == 2 {
                Err(std::io::Error::other("induced installation failure"))
            } else {
                fs::rename(from, to)
            }
        }
    }

    #[test]
    fn installation_failure_rolls_back_previous_output() {
        let root = TestDirectory::new("install-rollback");
        let output = root.path().join("dist");
        write(&output.join("previous.txt"), "previous valid output");
        let workspace = BuildWorkspace::prepare(&output).unwrap();
        write(&workspace.stage.join("new.txt"), "new output");
        let verified = VerifiedOutput {
            staged: StagedOutput { workspace },
        };
        let renamer = FailSecondRename {
            calls: Cell::new(0),
        };

        let error = verified.install_with(&renamer).unwrap_err();
        assert!(format!("{error:#}").contains("restored previous output"));
        assert_eq!(
            fs::read_to_string(output.join("previous.txt")).unwrap(),
            "previous valid output"
        );
        assert!(!output.join("new.txt").exists());
    }

    struct FailInstallAndRollback {
        calls: Cell<usize>,
    }

    impl Renamer for FailInstallAndRollback {
        fn rename(&self, from: &Path, to: &Path) -> std::io::Result<()> {
            let call = self.calls.get() + 1;
            self.calls.set(call);
            if call == 1 {
                fs::rename(from, to)
            } else {
                Err(std::io::Error::other("induced rename failure"))
            }
        }
    }

    #[test]
    fn failed_rollback_preserves_both_recovery_directories() {
        let root = TestDirectory::new("failed-rollback");
        let output = root.path().join("dist");
        write(&output.join("previous.txt"), "previous valid output");
        let workspace = BuildWorkspace::prepare(&output).unwrap();
        let stage = workspace.stage.clone();
        let backup = workspace.backup.clone();
        write(&stage.join("new.txt"), "new output");
        let verified = VerifiedOutput {
            staged: StagedOutput { workspace },
        };
        let renamer = FailInstallAndRollback {
            calls: Cell::new(0),
        };

        let error = verified.install_with(&renamer).unwrap_err();
        let message = format!("{error:#}");
        assert!(message.contains(&backup.display().to_string()));
        assert!(message.contains(&stage.display().to_string()));
        assert_eq!(
            fs::read_to_string(backup.join("previous.txt")).unwrap(),
            "previous valid output"
        );
        assert_eq!(
            fs::read_to_string(stage.join("new.txt")).unwrap(),
            "new output"
        );
        assert!(!output.exists());
    }

    #[test]
    fn an_existing_workspace_is_not_removed_as_stale() {
        let root = TestDirectory::new("workspace-lock");
        let output = root.path().join("dist");
        let first = BuildWorkspace::prepare(&output).unwrap();

        let error = BuildWorkspace::prepare(&output).err().unwrap();
        assert!(format!("{error:#}").contains("another build may be active"));
        assert!(first.stage.is_dir());
    }

    #[cfg(unix)]
    #[test]
    fn output_symlink_is_rejected_without_touching_its_target() {
        use std::os::unix::fs::symlink;

        let root = TestDirectory::new("output-symlink");
        let target = root.path().join("unrelated");
        write(&target.join("sentinel.txt"), "do not touch");
        symlink(&target, root.path().join("dist")).unwrap();

        let error = prepare_destination(root.path(), Path::new("dist")).unwrap_err();
        assert!(format!("{error:#}").contains("real directory"));
        assert_eq!(
            fs::read_to_string(target.join("sentinel.txt")).unwrap(),
            "do not touch"
        );
    }
}
