use anyhow::{anyhow, ensure, Result};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use log::debug;
use walkdir::WalkDir;

struct Patch {
    version: usize,
    files: Vec<PathBuf>,
}

impl Patch {
    fn new(version: usize, files: Vec<PathBuf>) -> Patch {
        Patch { version, files }
    }

    fn find_file(&self, path: &Path) -> Option<&PathBuf> {
        self.files.iter().find(|patch_path| files_match(patch_path, path))
    }
}

pub struct PatchProvider {
    location: PathBuf,
    patches: Vec<Patch>,
}

impl PatchProvider {
    pub fn new<T: Into<PathBuf>>(location: T) -> Result<PatchProvider> {
        let path = location.into();
        ensure!(path.is_dir(), "Given location is not a directory.");
        Ok(PatchProvider {
            location: path,
            patches: Vec::new()
        })
    }

    pub fn load_patches(&mut self) -> Result<()> {
        let mut patches = Vec::new();
        for entry in self.location
            .read_dir()?
        {
            if let Ok(entry) = entry {
                let version = entry.file_name().to_str().map(|name| usize::from_str(name)).ok_or_else(|| anyhow!("Unable to read directory name"))??;
                let files = find_files_in_dir(&entry.path());
                debug!("Added patch {} with {} files.", version, files.len());
                patches.push(Patch::new(version, files))
            }
        }

        self.patches = patches;
        Ok(())
    }

    pub fn get_latest_version<T: AsRef<Path>>(&self, path: T) -> Option<PathBuf> {
        let path = path.as_ref();
        let mut available_versions: Vec<(usize, &PathBuf)> = self
            .patches
            .iter()
            .filter_map(|patch| patch.find_file(path).map(|file| (patch.version, file)))
            .collect();
        available_versions.sort_by(|(version, _), (version2, _)| version.cmp(version2));

        available_versions
            .last()
            .map(|(_, path)| PathBuf::from(path))
    }

    pub fn get_patch_count(&self) -> usize {
        self.patches.len()
    }
}

fn files_match(patch_file: &PathBuf, requested: &Path) -> bool {
    let relative_path: PathBuf = patch_file.components().skip(3).collect();
    relative_path.as_path() == requested
}

fn find_files_in_dir<T: AsRef<Path>>(path: T) -> Vec<PathBuf> {
    let path = path.as_ref();
    let mut files = Vec::new();
    for entry in WalkDir::new(path)
        .into_iter()
        .filter_map(|entry| entry.ok())
    {
        if entry.metadata().map(|meta| meta.is_file()).unwrap_or(false) {
            debug!("Added file: {:?}", entry.path());
            files.push(entry.path().to_path_buf())
        }
    }

    files
}
