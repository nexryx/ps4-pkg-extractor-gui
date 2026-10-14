use anyhow::{Context as _, Result, bail};
use orbis_pkg_util::{ExtractProgress, PkgExtractor, open_pkg};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Default)]
pub struct Shared {
    entries_total: AtomicUsize,
    entries_done: AtomicUsize,
    pfs_total: AtomicUsize,
    pfs_done: AtomicUsize,
    bytes: AtomicU64,
    skipped: AtomicUsize,
    current: Mutex<String>,
}

pub enum Phase {
    Reading,
    Files { done: usize, total: usize },
}

impl Shared {
    fn set_current(&self, path: &Path) {
        if let Ok(mut cur) = self.current.lock() {
            *cur = path.display().to_string();
        }
    }

    pub fn current(&self) -> String {
        self.current.lock().map(|c| c.clone()).unwrap_or_default()
    }

    pub fn phase(&self) -> Phase {
        let total = self.pfs_total.load(Ordering::Relaxed);
        if total > 0 {
            Phase::Files {
                done: self.pfs_done.load(Ordering::Relaxed),
                total,
            }
        } else {
            Phase::Reading
        }
    }

    pub fn fraction(&self) -> f32 {
        let total = self.pfs_total.load(Ordering::Relaxed);
        if total > 0 {
            return (self.pfs_done.load(Ordering::Relaxed) as f32 / total as f32).min(1.0);
        }
        let total = self.entries_total.load(Ordering::Relaxed);
        if total > 0 {
            return (self.entries_done.load(Ordering::Relaxed) as f32 / total as f32).min(1.0);
        }
        0.0
    }

    pub fn bytes(&self) -> u64 {
        self.bytes.load(Ordering::Relaxed)
    }

    pub fn skipped(&self) -> usize {
        self.skipped.load(Ordering::Relaxed)
    }
}

struct GuiProgress(Arc<Shared>);

impl ExtractProgress for GuiProgress {
    fn entry_start(&self, path: &Path, current: usize, total: usize) {
        self.0.entries_total.store(total, Ordering::Relaxed);
        self.0.entries_done.store(current, Ordering::Relaxed);
        self.0.set_current(path);
    }

    fn entry_skipped(&self, _path: &Path, _reason: &str) {
        self.0.skipped.fetch_add(1, Ordering::Relaxed);
    }

    fn pfs_start(&self, total_items: usize) {
        self.0.pfs_done.store(0, Ordering::Relaxed);
        self.0.pfs_total.store(total_items.max(1), Ordering::Relaxed);
    }

    fn pfs_file(&self, path: &Path, _size: u64) {
        self.0.set_current(path);
    }

    fn pfs_file_completed(&self, written: u64) {
        self.0.bytes.fetch_add(written, Ordering::Relaxed);
        self.0.pfs_done.fetch_add(1, Ordering::Relaxed);
    }
}

pub fn default_output_dir(pkg_file: &Path) -> Option<PathBuf> {
    let stem = pkg_file.file_stem()?;
    let parent = pkg_file.parent().unwrap_or_else(|| Path::new("."));
    Some(parent.join(stem))
}

fn ensure_safe_to_replace(output: &Path, pkg: &Path) -> Result<()> {
    let output = output.canonicalize().unwrap_or_else(|_| output.to_path_buf());
    if output.parent().is_none() {
        bail!("refusing to replace a drive root: {}", output.display());
    }
    if let Ok(pkg) = pkg.canonicalize()
        && pkg.starts_with(&output)
    {
        bail!(
            "refusing to replace {} because the PKG file is inside it",
            output.display()
        );
    }
    if let Some(dirs) = directories::BaseDirs::new() {
        let home = dirs.home_dir();
        let home = home.canonicalize().unwrap_or_else(|_| home.to_path_buf());
        if home == output {
            bail!("refusing to replace your home folder: {}", output.display());
        }
    }
    Ok(())
}

pub fn run_extraction(
    pkg_file: PathBuf,
    output_dir: PathBuf,
    overwrite: bool,
    shared: Arc<Shared>,
) -> Result<PathBuf> {
    if !pkg_file.is_file() {
        bail!("PKG file not found: {}", pkg_file.display());
    }

    if output_dir.exists() {
        if !overwrite {
            bail!(
                "output directory already exists: {}",
                output_dir.display()
            );
        }
        ensure_safe_to_replace(&output_dir, &pkg_file)?;
        fs::remove_dir_all(&output_dir).with_context(|| {
            format!(
                "failed to remove existing output directory: {}",
                output_dir.display()
            )
        })?;
    }
    fs::create_dir_all(&output_dir)
        .with_context(|| format!("failed to create output directory: {}", output_dir.display()))?;

    let pkg = unsafe { open_pkg(&pkg_file) }
        .with_context(|| format!("failed to open or parse PKG file: {}", pkg_file.display()))?;

    PkgExtractor::new(&pkg, GuiProgress(shared), overwrite)
        .extract(&output_dir)
        .with_context(|| format!("failed while extracting package to {}", output_dir.display()))?;

    Ok(output_dir)
}
