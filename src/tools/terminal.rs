use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TerminalError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Path not found: {0}")]
    NotFound(String),
    #[error("Edit error: {0}")]
    Edit(String),
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

/// Returns (byte_start, byte_end_with_newline, line_str_without_newline) for each line
pub fn get_line_spans(content: &str) -> Vec<(usize, usize, &str)> {
    let mut spans = Vec::new();
    let mut line_start = 0;
    let bytes = content.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\n' {
            let text_end = if i > line_start && bytes[i - 1] == b'\r' {
                i - 1
            } else {
                i
            };
            spans.push((line_start, i + 1, &content[line_start..text_end]));
            line_start = i + 1;
        }
        i += 1;
    }
    if line_start < content.len() {
        spans.push((line_start, content.len(), &content[line_start..]));
    }
    spans
}

/// Apply a unified diff patch to file content
pub fn apply_patch(content: &str, patch: &str) -> Result<String, TerminalError> {
    let spans = get_line_spans(content);
    let orig_lines: Vec<&str> = spans.iter().map(|s| s.2).collect();

    let patch_lines: Vec<&str> = patch.lines().collect();
    let has_hunk_header = patch_lines.iter().any(|l| l.starts_with("@@"));

    if has_hunk_header {
        let mut result_lines = Vec::new();
        let mut orig_idx = 0; // 0-indexed cursor into orig_lines
        let mut i = 0;

        while i < patch_lines.len() {
            let line = patch_lines[i];
            if line.starts_with("@@") {
                if let Some(minus_pos) = line.find('-') {
                    let after_minus = &line[minus_pos + 1..];
                    let num_str: String = after_minus.chars().take_while(|c| c.is_ascii_digit()).collect();
                    if let Ok(start_line) = num_str.parse::<usize>() {
                        let target_idx = if start_line > 0 { start_line - 1 } else { 0 };
                        while orig_idx < target_idx && orig_idx < orig_lines.len() {
                            result_lines.push(orig_lines[orig_idx]);
                            orig_idx += 1;
                        }
                    }
                }
                i += 1;
                continue;
            }

            if line.starts_with('+') {
                result_lines.push(&line[1..]);
                i += 1;
            } else if line.starts_with('-') {
                if orig_idx < orig_lines.len() {
                    orig_idx += 1;
                }
                i += 1;
            } else if line.starts_with(' ') {
                if orig_idx < orig_lines.len() {
                    result_lines.push(orig_lines[orig_idx]);
                    orig_idx += 1;
                }
                i += 1;
            } else {
                i += 1;
            }
        }

        while orig_idx < orig_lines.len() {
            result_lines.push(orig_lines[orig_idx]);
            orig_idx += 1;
        }

        let newline = if content.contains("\r\n") { "\r\n" } else { "\n" };
        let mut res = result_lines.join(newline);
        if content.ends_with('\n') && !res.is_empty() && !res.ends_with('\n') {
            res.push_str(newline);
        }
        return Ok(res);
    }

    // Fallback: parse +/- blocks without @@ header
    let mut old_lines = Vec::new();
    let mut new_lines = Vec::new();
    for pl in &patch_lines {
        if let Some(rest) = pl.strip_prefix('-') {
            old_lines.push(rest);
        } else if let Some(rest) = pl.strip_prefix('+') {
            new_lines.push(rest);
        }
    }

    if !old_lines.is_empty() || !new_lines.is_empty() {
        let old_text = old_lines.join("\n");
        let new_text = new_lines.join("\n");
        if content.contains(&old_text) {
            let res = content.replacen(&old_text, &new_text, 1);
            return Ok(res);
        }
    }

    Err(TerminalError::Edit("Could not apply patch to file content".to_string()))
}

/// Modify an existing file by line numbers, search target, or diff patch without rewriting entire file
pub fn edit_file(
    path: &str,
    start_line: Option<usize>,
    end_line: Option<usize>,
    replacement: &str,
    target: Option<&str>,
    patch: Option<&str>,
) -> Result<String, TerminalError> {
    let p = Path::new(path);
    if !p.exists() {
        return Err(TerminalError::NotFound(path.to_string()));
    }
    if p.is_dir() {
        return Err(TerminalError::Edit(format!("'{}' is a directory, not a file", path)));
    }

    let content = std::fs::read_to_string(p)?;
    let newline = if content.contains("\r\n") { "\r\n" } else { "\n" };

    // 1. Diff patch mode
    let maybe_patch = patch.or_else(|| {
        let t = replacement.trim_start();
        if t.starts_with("@@ ") || (t.starts_with("--- ") && t.contains("\n@@ ")) {
            Some(replacement)
        } else {
            None
        }
    });

    if let Some(patch_str) = maybe_patch {
        let updated = apply_patch(&content, patch_str)?;
        std::fs::write(p, &updated)?;
        return Ok(format!("Applied patch to '{}' ({} bytes)", path, updated.len()));
    }

    let spans = get_line_spans(&content);
    let total_lines = spans.len();

    // 2. Line range mode
    if start_line.is_some() || end_line.is_some() {
        let start = start_line.unwrap_or_else(|| end_line.unwrap_or(1));
        let end = end_line.unwrap_or(start);

        let start = if start == 0 { 1 } else { start };
        let end = if end == 0 { 1 } else { end };

        if start > end {
            return Err(TerminalError::Edit(format!(
                "start_line ({}) cannot be greater than end_line ({})",
                start, end
            )));
        }

        if total_lines == 0 {
            let mut final_content = replacement.to_string();
            if !final_content.is_empty() && !final_content.ends_with('\n') {
                final_content.push_str(newline);
            }
            std::fs::write(p, &final_content)?;
            return Ok(format!("Wrote replacement to empty file '{}'", path));
        }

        if start > total_lines + 1 {
            return Err(TerminalError::Edit(format!(
                "start_line {} exceeds total lines {} in file '{}'",
                start, total_lines, path
            )));
        }

        let mut actual_start = start;
        let mut actual_end = end.min(total_lines);

        // Self-healing / target verification if target is provided
        if let Some(tgt) = target {
            let tgt_trimmed = tgt.trim();
            if !tgt_trimmed.is_empty() {
                let start_idx = actual_start - 1;
                let end_idx = actual_end - 1;
                let slice_text = spans[start_idx..=end_idx]
                    .iter()
                    .map(|s| s.2)
                    .collect::<Vec<_>>()
                    .join("\n");

                if !slice_text.contains(tgt_trimmed) {
                    let mut found_range = None;
                    let mut matches = 0;
                    for i in 0..total_lines {
                        for j in i..total_lines {
                            let candidate = spans[i..=j]
                                .iter()
                                .map(|s| s.2)
                                .collect::<Vec<_>>()
                                .join("\n");
                            if candidate.trim() == tgt_trimmed {
                                matches += 1;
                                found_range = Some((i + 1, j + 1));
                            }
                        }
                    }
                    if matches == 1 {
                        let (ns, ne) = found_range.unwrap();
                        actual_start = ns;
                        actual_end = ne;
                    } else if matches > 1 {
                        return Err(TerminalError::Edit(format!(
                            "Target text did not match lines {}-{} and matches {} other locations in '{}'",
                            start, end, matches, path
                        )));
                    } else {
                        return Err(TerminalError::Edit(format!(
                            "Target text did not match lines {}-{} in '{}'",
                            start, end, path
                        )));
                    }
                }
            }
        }

        let new_content = if actual_start == total_lines + 1 {
            // Append at EOF
            let mut s = content.clone();
            if !s.ends_with('\n') {
                s.push_str(newline);
            }
            s.push_str(replacement);
            if !s.ends_with('\n') {
                s.push_str(newline);
            }
            s
        } else {
            let start_idx = actual_start - 1;
            let end_idx = (actual_end - 1).min(total_lines - 1);
            let byte_start = spans[start_idx].0;
            let byte_end = spans[end_idx].1;

            let mut out = String::with_capacity(content.len() + replacement.len());
            out.push_str(&content[..byte_start]);

            if !replacement.is_empty() {
                out.push_str(replacement);
                if byte_end < content.len() && !replacement.ends_with('\n') {
                    out.push_str(newline);
                } else if byte_end == content.len() && content.ends_with('\n') && !replacement.ends_with('\n') {
                    out.push_str(newline);
                }
            }
            out.push_str(&content[byte_end..]);
            out
        };

        std::fs::write(p, &new_content)?;
        let lines_removed = actual_end - actual_start + 1;
        let lines_added = if replacement.is_empty() { 0 } else { replacement.lines().count().max(1) };
        return Ok(format!(
            "Replaced lines {}-{} in '{}' ({} lines removed, {} lines added)",
            actual_start, actual_end, path, lines_removed, lines_added
        ));
    }

    // 3. Search & replace mode
    if let Some(tgt) = target {
        let occurrences = content.matches(tgt).count();
        if occurrences == 0 {
            return Err(TerminalError::Edit(format!(
                "Target string not found in '{}'",
                path
            )));
        }
        if occurrences > 1 {
            return Err(TerminalError::Edit(format!(
                "Target string found {} times in '{}'. Please specify start_line and end_line.",
                occurrences, path
            )));
        }

        let new_content = content.replacen(tgt, replacement, 1);
        std::fs::write(p, &new_content)?;
        return Ok(format!("Replaced target string in '{}'", path));
    }

    Err(TerminalError::Edit(
        "Either start_line/end_line, target, or patch must be specified".to_string()
    ))
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

pub fn cd(path: Option<&str>) -> Result<String, TerminalError> {
    let target = match path {
        Some(p) if !p.trim().is_empty() && p.trim() != "~" => {
            let p_trimmed = p.trim();
            if let Some(rest) = p_trimmed.strip_prefix("~/") {
                if let Ok(home) = std::env::var("HOME") {
                    format!("{}/{}", home, rest)
                } else {
                    p_trimmed.to_string()
                }
            } else {
                p_trimmed.to_string()
            }
        }
        _ => std::env::var("HOME").unwrap_or_else(|_| String::from("/")),
    };

    let p = Path::new(&target);
    if !p.exists() {
        return Err(TerminalError::NotFound(target));
    }
    std::env::set_current_dir(p)?;
    let new_cwd = std::env::current_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or(target);
    Ok(new_cwd)
}

pub fn touch(path: &str) -> Result<(), TerminalError> {
    let p = Path::new(path);
    if let Some(parent) = p.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    if !p.exists() {
        std::fs::File::create(p)?;
    } else {
        let file = std::fs::OpenOptions::new().write(true).open(p)?;
        let _ = file.set_modified(std::time::SystemTime::now());
    }
    Ok(())
}

pub fn exec_cmd(cmd: &str) -> Result<String, TerminalError> {
    let output = std::process::Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let combined = if stderr.is_empty() {
        stdout
    } else if stdout.is_empty() {
        stderr
    } else {
        format!("{}\n{}", stdout.trim_end(), stderr.trim_end())
    };
    Ok(combined)
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

#[cfg(test)]
pub static TEST_CWD_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_touch_creates_file() {
        let _guard = TEST_CWD_LOCK.lock().unwrap();
        let test_file = "test_touch_file.tmp";
        if Path::new(test_file).exists() {
            let _ = std::fs::remove_file(test_file);
        }

        assert!(touch(test_file).is_ok());
        assert!(Path::new(test_file).exists());

        // Second touch updates timestamp without error
        assert!(touch(test_file).is_ok());

        let _ = std::fs::remove_file(test_file);
    }

    #[test]
    fn test_cd_and_ls() {
        let _guard = TEST_CWD_LOCK.lock().unwrap();
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        std::env::set_current_dir(&manifest_dir).unwrap();
        let orig_cwd = std::env::current_dir().unwrap();

        // cd into src
        let new_cwd = cd(Some("src")).expect("cd src should succeed");
        assert!(new_cwd.ends_with("src"));

        // ls should list files in src
        let files = ls(None).expect("ls in src should succeed");
        assert!(files.iter().any(|f| f.name == "main.rs" || f.name == "lib.rs"));

        // Restore original cwd
        std::env::set_current_dir(orig_cwd).unwrap();
    }

    #[test]
    fn test_exec_cmd() {
        let out = exec_cmd("echo 'hello nsh'").expect("exec_cmd should succeed");
        assert_eq!(out.trim(), "hello nsh");
    }

    #[test]
    fn test_edit_file_line_range_replace() {
        let _guard = TEST_CWD_LOCK.lock().unwrap();
        let test_file = "test_edit_file_range.tmp";
        let initial = "line 1\nline 2\nline 3\nline 4\nline 5\n";
        std::fs::write(test_file, initial).unwrap();

        let res = edit_file(
            test_file,
            Some(2),
            Some(3),
            "new line 2\nnew line 3",
            None,
            None,
        );
        assert!(res.is_ok(), "edit_file should succeed: {:?}", res);

        let content = std::fs::read_to_string(test_file).unwrap();
        assert_eq!(
            content,
            "line 1\nnew line 2\nnew line 3\nline 4\nline 5\n"
        );

        let _ = std::fs::remove_file(test_file);
    }

    #[test]
    fn test_edit_file_line_deletion() {
        let _guard = TEST_CWD_LOCK.lock().unwrap();
        let test_file = "test_edit_file_delete.tmp";
        let initial = "line 1\nline 2\nline 3\n";
        std::fs::write(test_file, initial).unwrap();

        let res = edit_file(test_file, Some(2), Some(2), "", None, None);
        assert!(res.is_ok());

        let content = std::fs::read_to_string(test_file).unwrap();
        assert_eq!(content, "line 1\nline 3\n");

        let _ = std::fs::remove_file(test_file);
    }

    #[test]
    fn test_edit_file_target_search_replace() {
        let _guard = TEST_CWD_LOCK.lock().unwrap();
        let test_file = "test_edit_file_target.tmp";
        let initial = "fn old_func() {\n    return 42;\n}\n";
        std::fs::write(test_file, initial).unwrap();

        let res = edit_file(
            test_file,
            None,
            None,
            "fn new_func() {\n    return 100;\n}",
            Some("fn old_func() {\n    return 42;\n}"),
            None,
        );
        assert!(res.is_ok());

        let content = std::fs::read_to_string(test_file).unwrap();
        assert_eq!(content, "fn new_func() {\n    return 100;\n}\n");

        let _ = std::fs::remove_file(test_file);
    }

    #[test]
    fn test_edit_file_diff_patch() {
        let _guard = TEST_CWD_LOCK.lock().unwrap();
        let test_file = "test_edit_file_patch.tmp";
        let initial = "alpha\nbeta\ngamma\ndelta\n";
        std::fs::write(test_file, initial).unwrap();

        let patch = "@@ -2,2 +2,2 @@\n-beta\n-gamma\n+BETA\n+GAMMA\n";
        let res = edit_file(test_file, None, None, "", None, Some(patch));
        assert!(res.is_ok(), "apply patch should succeed: {:?}", res);

        let content = std::fs::read_to_string(test_file).unwrap();
        assert_eq!(content, "alpha\nBETA\nGAMMA\ndelta\n");

        let _ = std::fs::remove_file(test_file);
    }
}
