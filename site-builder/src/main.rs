use std::env;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

fn main() {
    if let Err(error) = run() {
        eprintln!("site-builder failed: {error:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let root = repository_root()?;
    let mut arguments = env::args().skip(1);
    match arguments.next().as_deref() {
        Some("build") => {
            let output = parse_output(arguments.collect())?;
            site_builder::build::build(&root, &output)
        }
        Some("check-dist") => {
            let output = parse_output(arguments.collect())?;
            site_builder::validate::dist(&root.join(output), &root)?;
            println!("Built-site validation passed.");
            Ok(())
        }
        Some("compare") => {
            let left = arguments
                .next()
                .context("compare requires two directories")?;
            let right = arguments
                .next()
                .context("compare requires two directories")?;
            if arguments.next().is_some() {
                bail!("compare accepts exactly two directories");
            }
            site_builder::build::compare_directories(&root.join(left), &root.join(right))?;
            println!("Deterministic build comparison passed.");
            Ok(())
        }
        Some(command) => {
            bail!("unknown command {command:?}; expected build, check-dist, or compare")
        }
        None => bail!("usage: site-builder <build|check-dist|compare> [arguments]"),
    }
}

fn parse_output(arguments: Vec<String>) -> Result<PathBuf> {
    match arguments.as_slice() {
        [] => Ok(PathBuf::from("dist")),
        [flag, value] if flag == "--output" => Ok(PathBuf::from(value)),
        _ => bail!("expected no arguments or --output <relative-directory>"),
    }
}

fn repository_root() -> Result<PathBuf> {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .map(Path::to_owned)
        .context("site-builder must be inside the repository root")
}
