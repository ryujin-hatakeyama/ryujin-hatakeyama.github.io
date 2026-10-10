use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use chrono::{Days, NaiveDate};

use anyhow::{Context, Result, bail, ensure};

use crate::artifact::ArtifactSet;
use crate::content::{self, ValidatedSiteContent};
use crate::publish::{self, StagedOutput};
use crate::render::{self, GeneratedPage};

pub fn build(root: &Path, output_relative: &Path) -> Result<()> {
    let output = publish::prepare_destination(root, output_relative)?;
    let total_started = Instant::now();

    let started = Instant::now();
    let content = content::load(root)?;
    let content_time = started.elapsed();
    // A planned event that has begun would otherwise be published under
    // "Upcoming". Stop before staging, so the previous output is kept, and
    // leave the editorial decision to the author.
    if let Some(today) = utc_today() {
        let stale: Vec<_> = content::planned_updates_needing_review(content.updates(), today)
            .map(|update| format!("{:?} (event began {})", update.id, update.date))
            .collect();
        ensure!(
            stale.is_empty(),
            "planned updates need editorial review because their events have begun: {}. Set each to draft, or record what actually happened with event_status \"completed\"",
            stale.join(", ")
        );
    }

    let started = Instant::now();
    let pages = render::pages(&content);
    let render_time = started.elapsed();
    let route_count = pages.len();

    let started = Instant::now();
    let plan = BuildPlan::new(root, &content, pages)?;
    let plan_time = started.elapsed();

    let started = Instant::now();
    let staged = StagedOutput::generate(&output, plan.artifacts)?;
    let stage_time = started.elapsed();

    let started = Instant::now();
    let verified = staged.verify(root)?;
    let validation_time = started.elapsed();

    let started = Instant::now();
    let report = verified.install()?;
    let install_time = started.elapsed();

    println!(
        "Built and validated {route_count} public HTML routes in {}.",
        output.display()
    );
    if let Some(warning) = report.warning {
        eprintln!("site-builder warning: {warning}");
    }
    if std::env::var_os("SITE_BUILDER_TIMINGS").is_some() {
        print_timings(
            content_time,
            render_time,
            plan_time,
            stage_time,
            validation_time,
            install_time,
            total_started.elapsed(),
        );
    }
    Ok(())
}

/// Today's UTC date, used only to stop the build for stale planned updates so
/// that generated output never depends on the build date.
fn utc_today() -> Option<NaiveDate> {
    let seconds = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs();
    NaiveDate::from_ymd_opt(1970, 1, 1)?.checked_add_days(Days::new(seconds / 86_400))
}

fn print_timings(
    content: Duration,
    render: Duration,
    plan: Duration,
    stage: Duration,
    validation: Duration,
    install: Duration,
    total: Duration,
) {
    eprintln!(
        "site-builder timings: content={:.3}ms render={:.3}ms plan={:.3}ms stage={:.3}ms validation={:.3}ms install={:.3}ms total={:.3}ms",
        milliseconds(content),
        milliseconds(render),
        milliseconds(plan),
        milliseconds(stage),
        milliseconds(validation),
        milliseconds(install),
        milliseconds(total),
    );
}

fn milliseconds(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1_000.0
}

struct BuildPlan {
    artifacts: ArtifactSet,
}

impl BuildPlan {
    fn new(root: &Path, content: &ValidatedSiteContent, pages: Vec<GeneratedPage>) -> Result<Self> {
        let page_artifacts = page_artifacts(&pages)?;
        let public_assets = read_tree(&root.join("assets/public"), Path::new(""))?;
        let font_assets = read_tree(&root.join("assets/fonts"), Path::new("assets/fonts"))?;
        let generated = generated_site_files(root, content, &pages)?;

        let artifacts = page_artifacts
            .compose(public_assets)?
            .compose(font_assets)?
            .compose(generated)?;
        Ok(Self { artifacts })
    }
}

fn page_artifacts(pages: &[GeneratedPage]) -> Result<ArtifactSet> {
    let mut artifacts = ArtifactSet::new();
    for page in pages {
        artifacts.insert(&page.output_path, format!("{}\n", page.html).into_bytes())?;
    }
    Ok(artifacts)
}

fn generated_site_files(
    root: &Path,
    content: &ValidatedSiteContent,
    pages: &[GeneratedPage],
) -> Result<ArtifactSet> {
    let fonts = read_required(&root.join("assets/styles/fonts.css"))?;
    let site = read_required(&root.join("assets/styles/site.css"))?;
    let katex = read_required(&root.join("assets/styles/katex.min.css"))?;
    let client = read_required(&root.join("target/client/site.js"))?;
    let (sitemap, sitemap_index) = render::sitemap(pages);

    let mut artifacts = ArtifactSet::new();
    artifacts.insert(
        "assets/site.css",
        format!("{fonts}\n{site}\n{katex}\n").into_bytes(),
    )?;
    artifacts.insert("assets/site.js", client.into_bytes())?;
    artifacts.insert("rss.xml", render::rss(content.updates()).into_bytes())?;
    artifacts.insert("sitemap-0.xml", sitemap.into_bytes())?;
    artifacts.insert("sitemap-index.xml", sitemap_index.into_bytes())?;
    artifacts.insert(
        "robots.txt",
        b"User-agent: *\nAllow: /\n\nSitemap: https://ryujin-hatakeyama.github.io/sitemap-index.xml\n".to_vec(),
    )?;
    artifacts.insert(".nojekyll", Vec::new())?;
    Ok(artifacts)
}

fn read_required(path: &Path) -> Result<String> {
    let metadata = fs::symlink_metadata(path)
        .with_context(|| format!("missing or unreadable asset {}", path.display()))?;
    ensure!(
        metadata.is_file() && !metadata.file_type().is_symlink(),
        "asset must be a real file: {}",
        path.display()
    );
    fs::read_to_string(path)
        .with_context(|| format!("missing or unreadable asset {}", path.display()))
}

fn read_tree(source: &Path, destination: &Path) -> Result<ArtifactSet> {
    let metadata = fs::symlink_metadata(source)
        .with_context(|| format!("missing asset directory {}", source.display()))?;
    ensure!(
        metadata.is_dir() && !metadata.file_type().is_symlink(),
        "asset source must be a real directory: {}",
        source.display()
    );
    let mut artifacts = ArtifactSet::new();
    collect_tree(source, source, destination, &mut artifacts)?;
    Ok(artifacts)
}

fn collect_tree(
    root: &Path,
    directory: &Path,
    destination: &Path,
    artifacts: &mut ArtifactSet,
) -> Result<()> {
    let mut entries = fs::read_dir(directory)
        .with_context(|| format!("failed to read {}", directory.display()))?
        .collect::<std::io::Result<Vec<_>>>()?;
    entries.sort_by_key(std::fs::DirEntry::file_name);
    for entry in entries {
        let source_path = entry.path();
        let file_type = entry
            .file_type()
            .with_context(|| format!("failed to inspect {}", source_path.display()))?;
        ensure!(
            !file_type.is_symlink(),
            "asset tree must not contain symbolic links: {}",
            source_path.display()
        );
        if file_type.is_dir() {
            collect_tree(root, &source_path, destination, artifacts)?;
        } else if file_type.is_file() {
            let relative = source_path.strip_prefix(root)?;
            let output_path = destination.join(relative);
            let contents = fs::read(&source_path)
                .with_context(|| format!("failed to read {}", source_path.display()))?;
            artifacts.insert(output_path, contents)?;
        } else {
            bail!("unsupported asset type: {}", source_path.display());
        }
    }
    Ok(())
}

pub fn compare_directories(left: &Path, right: &Path) -> Result<()> {
    let left_files = relative_files(left)?;
    let right_files = relative_files(right)?;
    ensure!(
        left_files == right_files,
        "determinism check produced different file sets"
    );
    for relative in left_files {
        let left_bytes = fs::read(left.join(&relative))?;
        let right_bytes = fs::read(right.join(&relative))?;
        ensure!(
            left_bytes == right_bytes,
            "determinism check differs at {}",
            relative.display()
        );
    }
    Ok(())
}

fn relative_files(root: &Path) -> Result<Vec<PathBuf>> {
    let metadata = fs::symlink_metadata(root)
        .with_context(|| format!("failed to inspect {}", root.display()))?;
    ensure!(
        metadata.is_dir() && !metadata.file_type().is_symlink(),
        "comparison root must be a real directory: {}",
        root.display()
    );

    fn collect(root: &Path, directory: &Path, output: &mut Vec<PathBuf>) -> Result<()> {
        let mut entries = fs::read_dir(directory)?.collect::<std::io::Result<Vec<_>>>()?;
        entries.sort_by_key(std::fs::DirEntry::file_name);
        for entry in entries {
            let path = entry.path();
            let file_type = entry.file_type()?;
            ensure!(
                !file_type.is_symlink(),
                "comparison tree contains symbolic link {}",
                path.display()
            );
            if file_type.is_dir() {
                collect(root, &path, output)?;
            } else if file_type.is_file() {
                output.push(path.strip_prefix(root)?.to_owned());
            } else {
                bail!(
                    "comparison tree contains unsupported entry {}",
                    path.display()
                );
            }
        }
        Ok(())
    }

    let mut files = Vec::new();
    collect(root, root, &mut files)?;
    files.sort();
    Ok(files)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::build;

    static NEXT_TEST_DIRECTORY: AtomicU64 = AtomicU64::new(0);

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new(label: &str) -> Self {
            let sequence = NEXT_TEST_DIRECTORY.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "site-builder-{label}-{}-{sequence}",
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

    fn minimal_content(root: &Path) {
        fs::create_dir_all(root.join("content/publications")).unwrap();
        fs::create_dir_all(root.join("content/writings")).unwrap();
        fs::create_dir_all(root.join("content/projects")).unwrap();
        fs::create_dir_all(root.join("content/miscellany/notes")).unwrap();
        fs::create_dir_all(root.join("content/miscellany/diary")).unwrap();
        write(&root.join("content/miscellany/reading.yaml"), "items: []\n");
        write(&root.join("content/home.json"), r#"{"bio":["Biography."]}"#);
        write(&root.join("target/generated/updates.json"), "[]");
    }

    fn previous_output(root: &Path) {
        write(&root.join("dist/previous.txt"), "previous valid output");
    }

    #[test]
    fn invalid_content_preserves_previous_output() {
        let root = TestDirectory::new("invalid-content");
        minimal_content(root.path());
        write(
            &root.path().join("content/publications/invalid.json"),
            "{not JSON}",
        );
        previous_output(root.path());

        assert!(build(root.path(), Path::new("dist")).is_err());
        assert_eq!(
            fs::read_to_string(root.path().join("dist/previous.txt")).unwrap(),
            "previous valid output"
        );
    }

    #[test]
    fn missing_assets_preserve_previous_output() {
        let root = TestDirectory::new("missing-assets");
        minimal_content(root.path());
        previous_output(root.path());

        let error = build(root.path(), Path::new("dist")).unwrap_err();
        assert!(format!("{error:#}").contains("missing asset directory"));
        assert_eq!(
            fs::read_to_string(root.path().join("dist/previous.txt")).unwrap(),
            "previous valid output"
        );
    }

    #[cfg(unix)]
    #[test]
    fn asset_symlinks_are_rejected_without_reading_the_target() {
        use std::os::unix::fs::symlink;

        let root = TestDirectory::new("asset-symlink");
        let assets = root.path().join("assets");
        fs::create_dir(&assets).unwrap();
        write(&root.path().join("private.txt"), "not a public asset");
        symlink(root.path().join("private.txt"), assets.join("leak.txt")).unwrap();

        let error = super::read_tree(&assets, Path::new("")).unwrap_err();
        assert!(format!("{error:#}").contains("must not contain symbolic links"));
    }
}
