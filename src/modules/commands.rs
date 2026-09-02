// Command execution and utilities

// Convert absolute path to relative path using home directory
pub fn shorten_cwd(cwd: &str) -> String {
    if let Ok(home) = std::env::var("HOME") {
        if cwd.starts_with(&home) {
            let remainder = &cwd[home.len()..];
            if remainder.is_empty() {
                return "~".to_string();
            }
            return format!("~{}", remainder);
        }
    }
    cwd.to_string()
}

/// Split process output into display lines.
/// - Normalizes `\r\n` / `\r`
/// - Expands tab-separated multi-column rows (e.g. `ls -C`) into one cell per line
/// - Preserves blank lines
pub fn normalize_output_lines(text: &str) -> Vec<String> {
    let text = text.replace("\r\n", "\n").replace('\r', "\n");
    let mut lines = Vec::new();

    for raw in text.split('\n') {
        // Don't drop a final trailing empty from split — handle below.
        if raw.contains('\t') {
            // Columnar layout using tabs → one entry per line (list-friendly).
            let cells: Vec<String> = raw
                .split('\t')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
                .collect();
            if cells.len() > 1 {
                lines.extend(cells);
                continue;
            }
        }
        lines.push(raw.to_string());
    }

    // If the source ended without a trailing newline, `split` still yields the last
    // chunk; if it ended WITH a newline we get a trailing empty string — keep one
    // trailing blank only if the original text had content after a mid blank.
    // Strip a single trailing empty that comes from a final `\n` (usual case).
    if lines.len() > 1 && lines.last().is_some_and(|l| l.is_empty()) && text.ends_with('\n') {
        lines.pop();
    }

    lines
}

/// Whether this looks like a bare listing tool where one-entry-per-line is preferred.
fn is_list_command(program: &str) -> bool {
    matches!(
        program,
        "ls" | "dir" | "vdir" | "lsd" | "exa" | "eza" | "tree"
    )
}

/// If user ran `ls` without an explicit format flag, force one name per line (`-1`).
fn adjust_list_args(program: &str, args: &[&str]) -> Vec<String> {
    if !is_list_command(program) {
        return args.iter().map(|s| s.to_string()).collect();
    }

    // Respect user format choices (long list, columns, commas, etc.).
    let has_format = args.iter().any(|a| {
        if matches!(*a, "-1" | "-l" | "-C" | "-m" | "-x") {
            return true;
        }
        if a.starts_with("--format") {
            return true;
        }
        // Combined short flags like -la, -lh already imply long listing.
        a.starts_with('-')
            && !a.starts_with("--")
            && a.chars()
                .skip(1)
                .any(|c| matches!(c, 'l' | 'C' | 'm' | 'x' | '1'))
    });

    if has_format {
        return args.iter().map(|s| s.to_string()).collect();
    }

    // Default: one name per line (shell-friendly list layout).
    let mut out = vec!["-1".to_string()];
    out.extend(args.iter().map(|s| s.to_string()));
    out
}

/// Parse a command string into words, respecting single quotes, double quotes, and backslash escapes.
pub fn parse_command_line(input: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut escaped = false;
    let mut in_token = false;

    for c in input.chars() {
        if escaped {
            current.push(c);
            in_token = true;
            escaped = false;
        } else if c == '\\' && !in_single_quote {
            escaped = true;
            in_token = true;
        } else if c == '\'' && !in_double_quote {
            in_single_quote = !in_single_quote;
            in_token = true;
        } else if c == '"' && !in_single_quote {
            in_double_quote = !in_double_quote;
            in_token = true;
        } else if c.is_whitespace() && !in_single_quote && !in_double_quote {
            if in_token {
                tokens.push(std::mem::take(&mut current));
                in_token = false;
            }
        } else {
            current.push(c);
            in_token = true;
        }
    }

    if in_token {
        tokens.push(current);
    }

    tokens
}

/// Check if input has unquoted shell metacharacters like pipes, redirections, or chains.
pub fn has_unquoted_shell_metachars(input: &str) -> bool {
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut escaped = false;

    for c in input.chars() {
        if escaped {
            escaped = false;
        } else if c == '\\' && !in_single_quote {
            escaped = true;
        } else if c == '\'' && !in_double_quote {
            in_single_quote = !in_single_quote;
        } else if c == '"' && !in_single_quote {
            in_double_quote = !in_double_quote;
        } else if !in_single_quote && !in_double_quote {
            if matches!(c, '|' | '>' | '<' | '&' | ';') {
                return true;
            }
        }
    }
    false
}

// Execute shell command and return output lines (one display row each).
pub fn execute_command(input: &str) -> Vec<String> {
    let input = input.trim();
    if input.is_empty() {
        return vec![];
    }

    let tokens = parse_command_line(input);
    if tokens.is_empty() {
        return vec![];
    }

    let program = tokens[0].as_str();
    let raw_args: Vec<&str> = tokens.iter().skip(1).map(|s| s.as_str()).collect();

    // Built-in commands
    match program {
        "exit" | "quit" => vec![],

        "cd" => {
            let target = if raw_args.is_empty() {
                std::env::var("HOME").unwrap_or_else(|_| String::from("/"))
            } else {
                let p = raw_args[0].trim();
                if p == "~" {
                    std::env::var("HOME").unwrap_or_else(|_| String::from("/"))
                } else if let Some(rest) = p.strip_prefix("~/") {
                    if let Ok(home) = std::env::var("HOME") {
                        format!("{}/{}", home, rest)
                    } else {
                        p.to_string()
                    }
                } else {
                    p.to_string()
                }
            };
            if let Err(e) = std::env::set_current_dir(&target) {
                return vec![format!("cd: {}: {}", target, e)];
            }
            vec![]
        }

        "clear" => return vec!["__CLEAR__".to_string()],

        "/help" | "help" => {
            return vec![
                "Available commands:".to_string(),
                "  ask <q> | /ask   - Ask AI anything (uses current model + RAG)".to_string(),
                "  do <task> | /do  - Small agentic terminal tasks (cp/mv/rm/write)".to_string(),
                "  plan <goal> | /plan - Generate a plan".to_string(),
                "  build <spec> | /build - Agentic build: explore, code, create files".to_string(),
                "  /index <path>    - Index files into RAG".to_string(),
                "  /settings        - Open AI settings".to_string(),
                "  cd <dir>         - Change directory".to_string(),
                "  clear            - Clear screen".to_string(),
                "  exit / quit      - Exit shell".to_string(),
            ];
        }

        "/settings" | "settings" => {
            return vec!["__SETTINGS__".to_string()];
        }

        "/ask" | "ask" | "/do" | "do" | "/plan" | "plan" | "/build" | "build" | "/index"
        | "index" => {
            // These are handled in main.rs with full AI + tool loop (fresh config on every use).
            return vec![format!(
                "{}: AI command routing active (pass full input to AI handler).",
                program
            )];
        }

        // External commands
        _ => {
            use std::process::{Command, Stdio};

            let spawn_res = if has_unquoted_shell_metachars(input) {
                Command::new("sh")
                    .arg("-c")
                    .arg(input)
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .spawn()
            } else {
                let args = adjust_list_args(program, &raw_args);
                Command::new(program)
                    .args(&args)
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .spawn()
            };

            let mut child = match spawn_res {
                Ok(c) => c,
                Err(_e) => {
                    return vec![format!("{}: command not found", program)];
                }
            };

            let mut user_cancelled = false;
            loop {
                match child.try_wait() {
                    Ok(Some(_status)) => break,
                    Ok(None) => {
                        if crossterm::event::poll(std::time::Duration::from_millis(25)).unwrap_or(false) {
                            if let Ok(crossterm::event::Event::Key(key)) = crossterm::event::read() {
                                if key.code == crossterm::event::KeyCode::Esc
                                    || (key.code == crossterm::event::KeyCode::Char('c')
                                        && key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL))
                                {
                                    let _ = child.kill();
                                    user_cancelled = true;
                                    break;
                                }
                            }
                        }
                    }
                    Err(_) => break,
                }
            }

            if user_cancelled {
                let _ = child.wait();
                return vec!["^C [Process stopped by Esc]".to_string()];
            }

            match child.wait_with_output() {
                Ok(output) => {
                    let mut lines = Vec::new();
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    let stderr = String::from_utf8_lossy(&output.stderr);

                    lines.extend(normalize_output_lines(&stdout));
                    let err_lines = normalize_output_lines(&stderr);
                    if !err_lines.is_empty() {
                        lines.extend(err_lines);
                    }

                    if lines.is_empty() && !output.status.success() {
                        if let Some(code) = output.status.code() {
                            lines.push(format!("{}: exited with status {}", program, code));
                        }
                    }

                    lines
                }
                Err(e) => vec![format!("{}: execution failed ({})", program, e)],
            }
        }
    }
}

/// Check if a command requires sudo authentication and credentials are not cached.
pub fn command_needs_sudo_password(input: &str) -> bool {
    let tokens = parse_command_line(input);
    if tokens.is_empty() {
        return false;
    }

    // Check if sudo is invoked as the command or in a chained command
    let has_sudo = tokens.iter().any(|t| t == "sudo");
    if !has_sudo {
        return false;
    }

    // Check if sudo credentials are already cached non-interactively.
    // `sudo -n true` exits 0 if cached ticket is valid, non-zero if password needed.
    let check = std::process::Command::new("sudo")
        .arg("-n")
        .arg("true")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();

    match check {
        Ok(status) => !status.success(),
        Err(_) => false,
    }
}

/// Validate password via `sudo -S -v -p ""` to cache sudo credentials.
pub fn validate_and_cache_sudo_password(password: &str) -> Result<(), String> {
    use std::io::Write;
    use std::process::{Command, Stdio};

    let mut child = Command::new("sudo")
        .arg("-S")
        .arg("-v")
        .arg("-p")
        .arg("")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn sudo: {}", e))?;

    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(password.as_bytes());
        let _ = stdin.write_all(b"\n");
        let _ = stdin.flush();
    }

    let output = child
        .wait_with_output()
        .map_err(|e| format!("sudo error: {}", e))?;

    if output.status.success() {
        Ok(())
    } else {
        let err = String::from_utf8_lossy(&output.stderr);
        let msg = if err.trim().is_empty() {
            "Authentication failed. Please try again.".to_string()
        } else {
            let first_line = err
                .lines()
                .find(|l| !l.trim().is_empty())
                .unwrap_or("Authentication failed. Please try again.");
            first_line.trim().to_string()
        };
        Err(msg)
    }
}

/// Try prompting for password using Linux desktop dialog (e.g. zenity).
pub fn prompt_gui_password(title: &str) -> Option<String> {
    use std::process::Command;

    let has_display =
        std::env::var("DISPLAY").is_ok() || std::env::var("WAYLAND_DISPLAY").is_ok();
    if !has_display {
        return None;
    }

    let output = Command::new("zenity")
        .arg("--password")
        .arg(format!("--title={}", title))
        .output();

    if let Ok(out) = output {
        if out.status.success() {
            let pass = String::from_utf8_lossy(&out.stdout)
                .trim_end_matches(['\r', '\n'])
                .to_string();
            return Some(pass);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_command_line_double_quotes() {
        let input = r#"git commit -m "feat: improve settings ui and add more suggestion params""#;
        let tokens = parse_command_line(input);
        assert_eq!(
            tokens,
            vec![
                "git",
                "commit",
                "-m",
                "feat: improve settings ui and add more suggestion params"
            ]
        );
    }

    #[test]
    fn test_parse_command_line_single_quotes() {
        let input = "echo 'hello world' 'another test'";
        let tokens = parse_command_line(input);
        assert_eq!(tokens, vec!["echo", "hello world", "another test"]);
    }

    #[test]
    fn test_parse_command_line_escaped_and_nested() {
        let input = r#"echo "escaped \"quotes\"" hello\ world 'nested "quote"'"#;
        let tokens = parse_command_line(input);
        assert_eq!(
            tokens,
            vec![
                "echo",
                r#"escaped "quotes""#,
                "hello world",
                r#"nested "quote""#
            ]
        );
    }

    #[test]
    fn test_parse_command_line_empty_strings() {
        let input = r#"git commit -m """#;
        let tokens = parse_command_line(input);
        assert_eq!(tokens, vec!["git", "commit", "-m", ""]);
    }

    #[test]
    fn test_has_unquoted_shell_metachars() {
        assert!(!has_unquoted_shell_metachars(
            r#"git commit -m "feat: improve settings ui and add more suggestion params""#
        ));
        assert!(!has_unquoted_shell_metachars(r#"echo "hello | world""#));
        assert!(has_unquoted_shell_metachars("cat file.txt | grep foo"));
        assert!(has_unquoted_shell_metachars("echo foo > bar.txt"));
        assert!(has_unquoted_shell_metachars("cargo check && cargo test"));
    }

    #[test]
    fn test_command_needs_sudo_password_non_sudo() {
        assert!(!command_needs_sudo_password("ls -la"));
        assert!(!command_needs_sudo_password("git status"));
        assert!(!command_needs_sudo_password("cargo check"));
        assert!(!command_needs_sudo_password(""));
    }
}
