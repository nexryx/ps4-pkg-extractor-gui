use anyhow::{Context, Result, bail};
use clap::Parser;
use orbis_pkg_util::{ConsoleProgress, PkgExtractor, open_pkg};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
#[command(name = "pkgextract", about = "Extract PlayStation 4 .pkg files")]
struct Cli {
    #[arg(value_name = "PKG_FILE")]
    pkg_file: PathBuf,

    #[arg(short, long, value_name = "DIR")]
    output: Option<PathBuf>,

    #[arg(short = 'f', long)]
    overwrite: bool,
}

fn default_output_dir(pkg_file: &Path) -> Result<PathBuf> {
    let stem = pkg_file
        .file_stem()
        .and_then(|s| s.to_str())
        .context("could not derive output folder name from package filename")?;

    let parent = pkg_file.parent().unwrap_or_else(|| Path::new("."));
    Ok(parent.join(stem))
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    if !cli.pkg_file.exists() {
        bail!("PKG file not found: {}", cli.pkg_file.display());
    }

    let output_dir = match cli.output {
        Some(path) => path,
        None => default_output_dir(&cli.pkg_file)?,
    };

    if output_dir.exists() && !cli.overwrite {
        bail!(
            "output directory already exists: {}. Use --overwrite to replace it.",
            output_dir.display()
        );
    }

    if output_dir.exists() && cli.overwrite {
        fs::remove_dir_all(&output_dir).with_context(|| {
            format!(
                "failed to remove existing output directory: {}",
                output_dir.display()
            )
        })?;
    }
    fs::create_dir_all(&output_dir).with_context(|| {
        format!(
            "failed to create output directory: {}",
            output_dir.display()
        )
    })?;

    let pkg = unsafe { open_pkg(&cli.pkg_file) }.with_context(|| {
        format!(
            "failed to open or parse PKG file: {}",
            cli.pkg_file.display()
        )
    })?;

    let extractor = PkgExtractor::new(&pkg, ConsoleProgress::new(), cli.overwrite);
    extractor.extract(&output_dir).with_context(|| {
        format!(
            "failed while extracting package to {}",
            output_dir.display()
        )
    })?;

    println!("Extraction finished: {}", output_dir.display());
    Ok(())
}
