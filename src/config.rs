use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppConfig {
    pub task_dir: PathBuf,
    pub editor: Option<String>,
    pub sidebar_width: u8,
    pub sidebar_min_width: u8,
    pub sidebar_max_width: u8,
    pub show_help_bar: bool,
    pub auto_update_on_edit: bool,
    pub editor_highlighting: bool,
}

#[derive(Debug, Clone)]
pub struct ConfigPaths {
    pub root: PathBuf,
    pub config_file: PathBuf,
}

impl ConfigPaths {
    pub fn config_file(&self) -> &Path {
        &self.config_file
    }

    pub fn from_root(root: PathBuf) -> Self {
        let config_file = root.join("config.conf");
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
    pub fn new(task_dir: PathBuf) -> Self {
        Self {
            task_dir,
            editor: None,
            sidebar_width: 20,
            sidebar_min_width: 0,
            sidebar_max_width: 50,
            show_help_bar: true,
            auto_update_on_edit: false,
            editor_highlighting: true,
        }
    }

    /// Persist the config. Writes the task directory on the first line, then
    /// any non-default settings as `key=value` pairs. Comments are not
    /// preserved — saving rewrites the file.
    pub fn save(&self, paths: &ConfigPaths) -> io::Result<()> {
        fs::create_dir_all(&paths.root)?;
        let mut content = "task_dir=".to_string();
        content.push_str(&self.task_dir.display().to_string());
        if let Some(editor) = &self.editor {
            content.push('\n');
            content.push_str("editor=");
            content.push_str(editor);
        }
        content.push('\n');
        content.push_str("sidebar_width=");
        content.push_str(&self.sidebar_width.to_string());
        content.push('\n');
        content.push_str("sidebar_min_width=");
        content.push_str(&self.sidebar_min_width.to_string());
        content.push('\n');
        content.push_str("sidebar_max_width=");
        content.push_str(&self.sidebar_max_width.to_string());
        content.push('\n');
        content.push_str("show_help_bar=");
        content.push_str(if self.show_help_bar { "true" } else { "false" });
        content.push('\n');
        content.push_str("auto_update_on_edit=");
        content.push_str(if self.auto_update_on_edit {
            "true"
        } else {
            "false"
        });
        content.push('\n');
        content.push_str("editor_highlighting=");
        content.push_str(if self.editor_highlighting {
            "true"
        } else {
            "false"
        });
        fs::write(&paths.config_file, content)
    }

    /// Parse the config file. Lines starting with `#` are comments; empty
    /// lines are ignored.
    ///
    /// Recognized keys:
    ///
    /// - `task_dir` — path to the todo.txt.d directory. Also accepted as a
    ///   bare (legacy) first line for backwards compatibility.
    /// - `editor` — command to launch when opening a smart list externally.
    ///   May include arguments (e.g. `editor=code -w`). Resolution falls
    ///   back to `$VISUAL`, then `$EDITOR`, then a platform default.
    /// - `show_help_bar` — whether the bottom hint panel is visible on
    ///   startup. Accepts `true` or `false`. Defaults to `true`.
    pub fn load(paths: &ConfigPaths) -> io::Result<Self> {
        let raw = match fs::read_to_string(&paths.config_file) {
            Ok(raw) => raw,
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                let legacy = paths.root.join("config.txt");
                if legacy.exists() {
                    fs::read_to_string(&legacy)?
                } else {
                    return Err(e);
                }
            }
            Err(e) => return Err(e),
        };

        let mut task_dir: Option<String> = None;
        let mut editor: Option<String> = None;
        let mut sidebar_width: u8 = 20;
        let mut sidebar_min_width: u8 = 0;
        let mut sidebar_max_width: u8 = 50;
        let mut show_help_bar = true;
        let mut auto_update_on_edit = false;
        let mut editor_highlighting = true;

        for line in raw.lines() {
            let trimmed = line.trim_end_matches('\r');
            let trimmed = trimmed.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            if let Some((key, value)) = trimmed.split_once('=') {
                match key.trim() {
                    "task_dir" => {
                        let value = value.trim();
                        if !value.is_empty() {
                            task_dir = Some(value.to_string());
                        }
                    }
                    "editor" => {
                        let value = value.trim();
                        if !value.is_empty() {
                            editor = Some(value.to_string());
                        }
                    }
                    "sidebar_width" => {
                        if let Ok(v) = value.trim().parse::<u8>() {
                            sidebar_width = v.min(100);
                        }
                    }
                    "sidebar_min_width" => {
                        if let Ok(v) = value.trim().parse::<u8>() {
                            sidebar_min_width = v.min(100);
                        }
                    }
                    "sidebar_max_width" => {
                        if let Ok(v) = value.trim().parse::<u8>() {
                            sidebar_max_width = v.min(100);
                        }
                    }
                    "show_help_bar" => {
                        show_help_bar = value.trim() == "true";
                    }
                    "auto_update_on_edit" => {
                        auto_update_on_edit = value.trim() == "true";
                    }
                    "editor_highlighting" => {
                        editor_highlighting = value.trim() == "true";
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
            sidebar_width,
            sidebar_min_width,
            sidebar_max_width,
            show_help_bar,
            auto_update_on_edit,
            editor_highlighting,
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
