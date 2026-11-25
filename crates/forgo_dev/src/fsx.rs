//! File changes, planned first and then applied together.

use crate::error::{Error, Result};
use std::fs;
use std::path::{Path, PathBuf};

/// Read a file as UTF-8.
pub fn read(path: &Path) -> Result<String> {
    fs::read_to_string(path).map_err(|e| Error::Failed(format!("read {}: {e}", path.display())))
}

/// The changes a command makes, applied together by [`Plan::apply`].
pub struct Plan {
    root: PathBuf,
    /// Existing files and their new contents.
    edits: Vec<(PathBuf, String)>,
    /// New files and their contents.
    creates: Vec<(PathBuf, String)>,
    /// A directory to delete, last.
    remove: Option<PathBuf>,
}

impl Plan {
    /// An empty plan for the workspace at `root`.
    pub fn new(root: &Path) -> Plan {
        Plan {
            root: root.to_path_buf(),
            edits: Vec::new(),
            creates: Vec::new(),
            remove: None,
        }
    }

    /// Replace the contents of an existing file.
    pub fn edit(&mut self, path: PathBuf, contents: String) {
        self.edits.push((path, contents));
    }

    /// Create a new file (and its directories).
    pub fn create(&mut self, path: PathBuf, contents: String) {
        self.creates.push((path, contents));
    }

    /// Delete a directory and everything in it, after the file changes.
    pub fn remove_dir(&mut self, path: PathBuf) {
        self.remove = Some(path);
    }

    fn rel<'p>(&self, p: &'p Path) -> std::borrow::Cow<'p, str> {
        p.strip_prefix(&self.root).unwrap_or(p).to_string_lossy()
    }

    /// One line per change, with paths relative to the workspace root.
    pub fn describe(&self) -> String {
        let mut s = String::new();
        for (p, _) in &self.edits {
            s.push_str(&format!("  update {}\n", self.rel(p)));
        }
        for (p, _) in &self.creates {
            s.push_str(&format!("  create {}\n", self.rel(p)));
        }
        if let Some(d) = &self.remove {
            s.push_str(&format!(
                "  delete {}/ ({} files)\n",
                self.rel(d),
                count_files(d)
            ));
        }
        s
    }

    /// Apply the plan. Every file is first written next to its target,
    /// then renamed into place, keeping an existing file's permissions; the
    /// directory is deleted last. If a file can't be written, nothing is
    /// changed.
    pub fn apply(&self) -> Result<()> {
        let mut staged: Vec<(PathBuf, &Path)> = Vec::new();
        let mut made_dirs: Vec<PathBuf> = Vec::new();
        let result = self.stage(&mut staged, &mut made_dirs);
        if let Err(e) = result {
            for (tmp, _) in &staged {
                let _ = fs::remove_file(tmp);
            }
            for d in made_dirs.iter().rev() {
                let _ = fs::remove_dir(d);
            }
            return Err(e);
        }
        for (tmp, target) in &staged {
            fs::rename(tmp, target).map_err(|e| {
                Error::Failed(format!(
                    "replace {}: {e} (some files may already be updated)",
                    self.rel(target)
                ))
            })?;
        }
        if let Some(d) = &self.remove {
            fs::remove_dir_all(d)
                .map_err(|e| Error::Failed(format!("delete {}: {e}", self.rel(d))))?;
        }
        Ok(())
    }

    fn stage<'a>(
        &'a self,
        staged: &mut Vec<(PathBuf, &'a Path)>,
        made_dirs: &mut Vec<PathBuf>,
    ) -> Result<()> {
        for (target, _) in &self.creates {
            if let Some(dir) = target.parent() {
                let mut missing = Vec::new();
                let mut d = Some(dir);
                while let Some(x) = d.filter(|x| !x.exists()) {
                    missing.push(x.to_path_buf());
                    d = x.parent();
                }
                for x in missing.into_iter().rev() {
                    fs::create_dir(&x)
                        .map_err(|e| Error::Failed(format!("mkdir {}: {e}", self.rel(&x))))?;
                    made_dirs.push(x);
                }
            }
        }
        for (target, contents) in self.edits.iter().chain(&self.creates) {
            let name = target
                .file_name()
                .map_or_else(Default::default, |n| n.to_string_lossy());
            let tmp = target.with_file_name(format!(".{name}.forgo-dev.tmp"));
            fs::write(&tmp, contents)
                .map_err(|e| Error::Failed(format!("write {}: {e}", self.rel(target))))?;
            staged.push((tmp.clone(), target));
            if let Ok(meta) = fs::metadata(target) {
                fs::set_permissions(&tmp, meta.permissions())
                    .map_err(|e| Error::Failed(format!("chmod {}: {e}", self.rel(target))))?;
            }
        }
        Ok(())
    }
}

fn count_files(dir: &Path) -> usize {
    let Ok(entries) = fs::read_dir(dir) else {
        return 0;
    };
    entries
        .flatten()
        .map(|e| match e.file_type() {
            Ok(t) if t.is_dir() => count_files(&e.path()),
            _ => 1,
        })
        .sum()
}
