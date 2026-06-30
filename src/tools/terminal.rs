use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TerminalError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Path not found: {0}")]
    NotFound(String),
}

pub fn cat(path: &str) -> Result<String, TerminalError> {
    let path = Path::new(path);
    if !path.exists() {
        return Err(TerminalError::NotFound(path.display().to_string()));
    }
    let content = std::fs::read_to_string(path)?;
    Ok(content)
}

pub fn ls(path: Option<&str>) -> Result<Vec<FileEntry>, TerminalError> {
    let target = path.map(Path::new).unwrap_or(Path::new("."));

    if !target.exists() {
        return Err(TerminalError::NotFound(target.display().to_string()));
    }

    let entries = std::fs::read_dir(target)?;
    let mut files = Vec::new();

    for entry in entries {
        let entry = entry?;
        let metadata = entry.metadata()?;
        files.push(FileEntry {
            name: entry.file_name().to_string_lossy().to_string(),
            is_dir: metadata.is_dir(),
            size: metadata.len(),
        });
    }

    files.sort_by(|a, b| match (a.is_dir, b.is_dir) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.cmp(&b.name),
    });

    Ok(files)
}

pub fn grep(pattern: &str, path: Option<&str>) -> Result<String, TerminalError> {
    let target = path.unwrap_or(".");
    let path = Path::new(target);

    if !path.exists() {
        return Err(TerminalError::NotFound(target.to_string()));
    }

    let regex = regex::Regex::new(pattern)
        .map_err(|e| TerminalError::Io(std::io::Error::new(std::io::ErrorKind::InvalidInput, e)))?;
    let mut results = Vec::new();

    if path.is_file() {
        let content = std::fs::read_to_string(path)?;
        for (line_num, line) in content.lines().enumerate() {
            if regex.is_match(line) {
                results.push(format!("{}: {}", line_num + 1, line));
            }
        }
    } else if path.is_dir() {
        grep_dir(path, &regex, &mut results, 0, 3)?;
    }

    Ok(results.join("\n"))
}

fn grep_dir(
    dir: &Path,
    regex: &regex::Regex,
    results: &mut Vec<String>,
    depth: usize,
    max_depth: usize,
) -> Result<(), TerminalError> {
    if depth > max_depth {
        return Ok(());
    }

    let entries = std::fs::read_dir(dir)?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            if let Some(ext) = path.extension() {
                let ext = ext.to_string_lossy().to_lowercase();
                if !["rs", "toml", "md", "txt", "json", "yaml", "yml"].contains(&ext.as_str()) {
                    continue;
                }
            }

            let content = std::fs::read_to_string(&path)?;
            for (line_num, line) in content.lines().enumerate() {
                if regex.is_match(line) {
                    results.push(format!("{}:{}: {}", path.display(), line_num + 1, line));
                }
            }
        } else if path.is_dir() {
            grep_dir(&path, regex, results, depth + 1, max_depth)?;
        }
    }

    Ok(())
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct FileEntry {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
}

impl std::fmt::Display for FileEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_dir {
            write!(f, "{}/", self.name)
        } else {
            write!(f, "{} ({})", self.name, self.size)
        }
    }
}

// --- Write / mutate tools for agentic do + build ---

pub fn write_file(path: &str, content: &str) -> Result<(), TerminalError> {
    let p = Path::new(path);
    if let Some(parent) = p.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    std::fs::write(p, content)?;
    Ok(())
}

pub fn delete_path(path: &str) -> Result<(), TerminalError> {
    let p = Path::new(path);
    if !p.exists() {
        return Err(TerminalError::NotFound(path.to_string()));
    }
    if p.is_dir() {
        std::fs::remove_dir_all(p)?;
    } else {
        std::fs::remove_file(p)?;
    }
    Ok(())
}

pub fn copy_path(src: &str, dst: &str) -> Result<(), TerminalError> {
    let s = Path::new(src);
    let d = Path::new(dst);
    if !s.exists() {
        return Err(TerminalError::NotFound(src.to_string()));
    }
    if let Some(parent) = d.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    if s.is_dir() {
        // simple recursive copy for dirs (std has no built-in until recent)
        copy_dir_all(s, d)?;
    } else {
        std::fs::copy(s, d)?;
    }
    Ok(())
}

pub fn move_path(src: &str, dst: &str) -> Result<(), TerminalError> {
    let s = Path::new(src);
    let d = Path::new(dst);
    if !s.exists() {
        return Err(TerminalError::NotFound(src.to_string()));
    }
    if let Some(parent) = d.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    std::fs::rename(s, d)?;
    Ok(())
}

pub fn mkdir(path: &str) -> Result<(), TerminalError> {
    std::fs::create_dir_all(path)?;
    Ok(())
}

fn copy_dir_all(src: &Path, dst: &Path) -> Result<(), TerminalError> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let target = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &target)?;
        } else {
            std::fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}
