use std::fs;
use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result, ensure};

use crate::content;
use crate::render::{self, GeneratedPage};
use crate::validate;

pub fn build(root: &Path, output_relative: &Path) -> Result<()> {
    validate_output_path(output_relative)?;
    let output = root.join(output_relative);
    clean_output(&output)?;
    fs::create_dir_all(&output)
        .with_context(|| format!("failed to create {}", output.display()))?;

    let content = content::load(root)?;
    let pages = render::pages(&content);
    write_pages(&output, &pages)?;
    write_site_files(root, &output, &content.updates, &pages)?;
    validate::dist(&output, root)?;

    println!(
        "Built and validated {} public HTML routes in {}.",
        pages.len(),
        output.display()
    );
    Ok(())
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

fn clean_output(output: &Path) -> Result<()> {
    if output.exists() {
        ensure!(
            output.is_dir(),
            "output path {} is not a directory",
            output.display()
        );
        fs::remove_dir_all(output)
            .with_context(|| format!("failed to clean {}", output.display()))?;
    }
    Ok(())
}

fn write_pages(output: &Path, pages: &[GeneratedPage]) -> Result<()> {
    for page in pages {
        write(output, &page.output_path, &format!("{}\n", page.html))?;
    }
    Ok(())
}

fn write_site_files(
    root: &Path,
    output: &Path,
    updates: &[crate::model::Update],
    pages: &[GeneratedPage],
) -> Result<()> {
    copy_tree(&root.join("assets/public"), output)?;
    copy_tree(&root.join("assets/fonts"), &output.join("assets/fonts"))?;

    let fonts = read_required(&root.join("assets/styles/fonts.css"))?;
    let site = read_required(&root.join("assets/styles/site.css"))?;
    let katex = read_required(&root.join("assets/styles/katex.min.css"))?;
    write(
        output,
        "assets/site.css",
        &format!("{fonts}\n{site}\n{katex}\n"),
    )?;

    let client = read_required(&root.join("target/client/site.js"))?;
    write(output, "assets/site.js", &client)?;

    write(output, "rss.xml", &render::rss(updates))?;
    let (sitemap, sitemap_index) = render::sitemap(pages);
    write(output, "sitemap-0.xml", &sitemap)?;
    write(output, "sitemap-index.xml", &sitemap_index)?;
    write(
        output,
        "robots.txt",
        "User-agent: *\nAllow: /\n\nSitemap: https://ryujin-hatakeyama.github.io/sitemap-index.xml\n",
    )?;
    write(output, ".nojekyll", "")?;
    Ok(())
}

fn read_required(path: &Path) -> Result<String> {
    fs::read_to_string(path)
        .with_context(|| format!("missing or unreadable asset {}", path.display()))
}

fn write(output: &Path, relative: impl AsRef<Path>, contents: &str) -> Result<()> {
    let path = output.join(relative.as_ref());
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    fs::write(&path, contents).with_context(|| format!("failed to write {}", path.display()))
}

fn copy_tree(source: &Path, destination: &Path) -> Result<()> {
    ensure!(
        source.is_dir(),
        "missing asset directory {}",
        source.display()
    );
    fs::create_dir_all(destination)
        .with_context(|| format!("failed to create {}", destination.display()))?;
    let mut entries = fs::read_dir(source)
        .with_context(|| format!("failed to read {}", source.display()))?
        .collect::<std::io::Result<Vec<_>>>()?;
    entries.sort_by_key(std::fs::DirEntry::file_name);
    for entry in entries {
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        let metadata = entry
            .metadata()
            .with_context(|| format!("failed to inspect {}", source_path.display()))?;
        if metadata.is_dir() {
            copy_tree(&source_path, &destination_path)?;
        } else if metadata.is_file() {
            fs::copy(&source_path, &destination_path).with_context(|| {
                format!(
                    "failed to copy {} to {}",
                    source_path.display(),
                    destination_path.display()
                )
            })?;
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
    fn collect(root: &Path, directory: &Path, output: &mut Vec<PathBuf>) -> Result<()> {
        let mut entries = fs::read_dir(directory)?.collect::<std::io::Result<Vec<_>>>()?;
        entries.sort_by_key(std::fs::DirEntry::file_name);
        for entry in entries {
            let path = entry.path();
            if entry.file_type()?.is_dir() {
                collect(root, &path, output)?;
            } else {
                output.push(path.strip_prefix(root)?.to_owned());
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
    use super::validate_output_path;
    use std::path::Path;

    #[test]
    fn limits_cleanable_output_paths() {
        assert!(validate_output_path(Path::new("dist")).is_ok());
        assert!(validate_output_path(Path::new("target/determinism-dist")).is_ok());
        assert!(validate_output_path(Path::new(".")).is_err());
        assert!(validate_output_path(Path::new("../outside")).is_err());
        assert!(validate_output_path(Path::new("content")).is_err());
    }
}
