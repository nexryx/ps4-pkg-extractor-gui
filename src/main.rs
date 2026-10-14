mod gui;

use anyhow::{Context, Result, anyhow, bail};
use clap::Parser;
use orbis_pkg_util::{ConsoleProgress, PkgExtractor, open_pkg};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
#[command(
    name = "pkgextract",
    about = "Extract PlayStation 4 .pkg files. Double-click the program to open the GUI."
)]
struct Cli {
    #[arg(value_name = "PKG_FILE", required_unless_present = "gui")]
    pkg_file: Option<PathBuf>,

    #[arg(short, long, value_name = "DIR")]
    output: Option<PathBuf>,

    #[arg(short = 'f', long)]
    overwrite: bool,

    /// Open the graphical interface (with PKG_FILE preloaded, if given)
    #[arg(long)]
    gui: bool,
}

fn default_output_dir(pkg_file: &Path) -> Result<PathBuf> {
    let stem = pkg_file
        .file_stem()
        .and_then(|s| s.to_str())
        .context("could not derive output folder name from package filename")?;

    let parent = pkg_file.parent().unwrap_or_else(|| Path::new("."));
    Ok(parent.join(stem))
}

/// True when the program was started from the desktop (double-click, or a file
/// dropped on it) rather than from a terminal.
fn launched_from_desktop() -> bool {
    #[cfg(windows)]
    {
        use windows_sys::Win32::System::Console::GetConsoleProcessList;
        let mut processes = [0u32; 2];
        // SAFETY: the pointer and length describe the live `processes` array.
        // A console shared with a shell lists more than this one process.
        unsafe { GetConsoleProcessList(processes.as_mut_ptr(), processes.len() as u32) == 1 }
    }
    #[cfg(not(windows))]
    {
        use std::io::IsTerminal;
        std::env::args_os().len() == 1
            && !std::io::stdin().is_terminal()
            && !std::io::stdout().is_terminal()
    }
}

/// Closes the console window Windows opened for a double-click.
fn detach_console() {
    #[cfg(windows)]
    // SAFETY: no arguments; the process keeps running without a console.
    unsafe {
        windows_sys::Win32::System::Console::FreeConsole();
    }
}

fn open_gui(pkg_file: Option<PathBuf>) -> Result<()> {
    gui::run(pkg_file.filter(|path| path.is_file())).map_err(|error| anyhow!("{error}"))
}

fn main() -> Result<()> {
    if launched_from_desktop() {
        detach_console();
        return open_gui(std::env::args_os().nth(1).map(PathBuf::from));
    }

    let cli = Cli::parse();
    if cli.gui {
        return open_gui(cli.pkg_file);
    }
    let Some(pkg_file) = cli.pkg_file else {
        bail!("no PKG file given");
    };

    if !pkg_file.exists() {
        bail!("PKG file not found: {}", pkg_file.display());
    }

    let output_dir = match cli.output {
        Some(path) => path,
        None => default_output_dir(&pkg_file)?,
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

    let pkg = unsafe { open_pkg(&pkg_file) }.with_context(|| {
        format!("failed to open or parse PKG file: {}", pkg_file.display())
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
