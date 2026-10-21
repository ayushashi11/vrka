use crate::syntax::cst::Module;
use crate::syntax::parse;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct ModuleLoader {
    search_paths: Vec<PathBuf>,
    pub visited: HashSet<PathBuf>,
}

impl ModuleLoader {
    pub fn new(current_file_dir: Option<PathBuf>) -> Self {
        Self::with_mesh_path(current_file_dir, std::env::var("MESH_PATH").ok().as_deref())
    }

    /// Construct a ModuleLoader with an explicit MESH_PATH override (e.g. for testing or CLI flags)
    pub fn with_mesh_path(current_file_dir: Option<PathBuf>, mesh_path_env: Option<&str>) -> Self {
        let mut search_paths = Vec::new();
        if let Some(dir) = current_file_dir {
            search_paths.push(dir);
        }
        if let Some(mesh_path) = mesh_path_env {
            for p in mesh_path.split(':') {
                if !p.is_empty() {
                    search_paths.push(PathBuf::from(p));
                }
            }
        }
        if let Ok(cwd) = std::env::current_dir() {
            search_paths.push(cwd);
        }

        Self {
            search_paths,
            visited: HashSet::new(),
        }
    }

    /// Resolve module identifier or path string to a canonical file path
    pub fn resolve_path(&self, target: &str) -> Option<PathBuf> {
        let p = Path::new(target);
        if p.is_absolute() && p.exists() {
            return fs::canonicalize(p).ok();
        }

        for base in &self.search_paths {
            // 1. Direct join: base / target
            let candidate = base.join(target);
            if candidate.is_file() {
                return fs::canonicalize(&candidate).ok();
            }

            // 2. Try candidate.mesh
            let candidate_mesh = base.join(format!("{}.mesh", target));
            if candidate_mesh.is_file() {
                return fs::canonicalize(&candidate_mesh).ok();
            }

            // 3. Try candidate.ion
            let candidate_ion = base.join(format!("{}.ion", target));
            if candidate_ion.is_file() {
                return fs::canonicalize(&candidate_ion).ok();
            }

            // 4. Try candidate/mod.mesh
            let mod_mesh = candidate.join("mod.mesh");
            if mod_mesh.is_file() {
                return fs::canonicalize(&mod_mesh).ok();
            }

            // 5. Try candidate/mod.ion
            let mod_ion = candidate.join("mod.ion");
            if mod_ion.is_file() {
                return fs::canonicalize(&mod_ion).ok();
            }
        }

        None
    }

    /// Load and parse a module from resolved path
    pub fn load_file(&mut self, path: &Path) -> Result<(Module, PathBuf), String> {
        let canonical = fs::canonicalize(path).map_err(|e| format!("Cannot find path '{}': {}", path.display(), e))?;
        if self.visited.contains(&canonical) {
            return Err(format!("Circular import detected for '{}'", canonical.display()));
        }
        self.visited.insert(canonical.clone());

        let source = fs::read_to_string(&canonical).map_err(|e| format!("Failed to read '{}': {}", canonical.display(), e))?;
        let module = parse(&source).map_err(|e| format!("Parse error in '{}': {}", canonical.display(), e))?;

        Ok((module, canonical))
    }
}
