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

// Execute shell command and return output lines (one display row each).
pub fn execute_command(input: &str) -> Vec<String> {
    let input = input.trim();
    if input.is_empty() {
        return vec![];
    }

    let mut parts = input.split_whitespace();
    let program = match parts.next() {
        Some(p) => p,
        None => return vec![],
    };
    let raw_args: Vec<&str> = parts.collect();
    let args = adjust_list_args(program, &raw_args);

    // Built-in commands
    match program {
        "exit" | "quit" => vec![],

        "cd" => {
            let target = if raw_args.is_empty() {
                std::env::var("HOME").unwrap_or_else(|_| String::from("/"))
            } else {
                raw_args[0].to_string()
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
            use std::process::Command;

            let result = Command::new(program).args(&args).output();

            match result {
                Ok(output) => {
                    let mut lines = Vec::new();
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    let stderr = String::from_utf8_lossy(&output.stderr);

                    lines.extend(normalize_output_lines(&stdout));
                    // Keep stderr after stdout, still line-oriented.
                    let err_lines = normalize_output_lines(&stderr);
                    if !err_lines.is_empty() {
                        if !lines.is_empty() {
                            // Visual separation between stdout and stderr blocks.
                            // (only if stdout had content)
                        }
                        lines.extend(err_lines);
                    }

                    if lines.is_empty() && !output.status.success() {
                        if let Some(code) = output.status.code() {
                            lines.push(format!("{}: exited with status {}", program, code));
                        }
                    }

                    lines
                }
                Err(_e) => {
                    vec![format!("{}: command not found", program)]
                }
            }
        }
    }
}
