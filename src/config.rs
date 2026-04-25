use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppConfig {
    pub task_dir: PathBuf,
    pub editor: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ConfigPaths {
    pub root: PathBuf,
    pub config_file: PathBuf,
}

impl ConfigPaths {
    pub fn from_root(root: PathBuf) -> Self {
        let config_file = root.join("config.txt");
        Self { root, config_file }
    }

    pub fn discover() -> io::Result<Self> {
        if let Some(config_home) = env::var_os("XDG_CONFIG_HOME") {
            return Ok(Self::from_root(PathBuf::from(config_home).join("ttd")));
        }

        if let Some(home) = env::var_os("HOME") {
            return Ok(Self::from_root(PathBuf::from(home).join(".config/ttd")));
        }

        Err(io::Error::new(
            io::ErrorKind::NotFound,
            "HOME or XDG_CONFIG_HOME must be set to resolve config paths",
        ))
    }
}

impl AppConfig {
    /// Persist the config. Writes the task directory on the first line, then
    /// any non-default settings as `key=value` pairs. Comments are not
    /// preserved — saving rewrites the file.
    pub fn save(&self, paths: &ConfigPaths) -> io::Result<()> {
        fs::create_dir_all(&paths.root)?;
        let mut content = self.task_dir.display().to_string();
        if let Some(editor) = &self.editor {
            content.push('\n');
            content.push_str("editor=");
            content.push_str(editor);
        }
        fs::write(&paths.config_file, content)
    }

    /// Parse the config file. The first non-empty, non-comment line is the
    /// task directory (legacy single-line form). Subsequent lines are
    /// `key=value` settings. Lines starting with `#` are comments. Empty
    /// lines are ignored.
    ///
    /// Recognized keys:
    ///
    /// - `editor` — command to launch when opening a smart list externally.
    ///   May include arguments (e.g. `editor=code -w`). Resolution falls
    ///   back to `$VISUAL`, then `$EDITOR`, then a platform default.
    pub fn load(paths: &ConfigPaths) -> io::Result<Self> {
        let raw = fs::read_to_string(&paths.config_file)?;

        let mut task_dir: Option<String> = None;
        let mut editor: Option<String> = None;

        for line in raw.lines() {
            let trimmed = line.trim_end_matches('\r');
            let trimmed = trimmed.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            if let Some((key, value)) = trimmed.split_once('=') {
                match key.trim() {
                    "editor" => {
                        let value = value.trim();
                        if !value.is_empty() {
                            editor = Some(value.to_string());
                        }
                    }
                    _ => {} // unknown keys silently ignored
                }
                continue;
            }
            // Legacy single-line form: the first bare line is the task
            // directory. A second bare line means the file has unrecognized
            // content and should be rejected so we don't silently load
            // half-corrupt config.
            if task_dir.is_some() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "config file contains an unrecognized non-key line",
                ));
            }
            task_dir = Some(trimmed.to_string());
        }

        let task_dir = task_dir.ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "config file is empty or contains no task directory",
            )
        })?;

        Ok(Self {
            task_dir: PathBuf::from(task_dir),
            editor,
        })
    }
}

pub fn validate_task_dir(path: &Path) -> io::Result<()> {
    if !path.exists() {
        fs::create_dir_all(path.join("done.txt.d"))?;
        return Ok(());
    }

    if !path.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "task dir is not a directory",
        ));
    }

    fs::create_dir_all(path.join("done.txt.d"))
}

/// Resolve the editor command: explicit config → `$VISUAL` → `$EDITOR` →
/// platform default. The returned string may include args (e.g. `code -w`);
/// callers should split on whitespace before spawning.
pub fn resolve_editor(config: Option<&AppConfig>) -> String {
    if let Some(cfg) = config
        && let Some(editor) = &cfg.editor
        && !editor.trim().is_empty()
    {
        return editor.trim().to_string();
    }
    for var in ["VISUAL", "EDITOR"] {
        if let Ok(value) = env::var(var)
            && !value.trim().is_empty()
        {
            return value.trim().to_string();
        }
    }
    if cfg!(windows) {
        "notepad".to_string()
    } else {
        "vi".to_string()
    }
}
