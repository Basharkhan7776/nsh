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

/// Clean interactive input by removing explicit execution prefixes (e.g. "live", "dev", "tui", "term", "interactive").
pub fn clean_interactive_input(input: &str) -> String {
    let trimmed = input.trim();
    let tokens = parse_command_line(trimmed);
    if tokens.is_empty() {
        return trimmed.to_string();
    }
    // Check if first token is an explicit prefix
    if matches!(
        tokens[0].as_str(),
        "live" | "dev" | "tui" | "term" | "interactive"
    ) {
        if let Some(rest) = trimmed.strip_prefix(&tokens[0]) {
            return rest.trim_start().to_string();
        }
    }
    // Check if sudo/doas is followed by an explicit prefix (e.g. "sudo live apt update" -> "sudo apt update")
    if (tokens[0] == "sudo" || tokens[0] == "doas") && tokens.len() > 1 {
        if matches!(
            tokens[1].as_str(),
            "live" | "dev" | "tui" | "term" | "interactive"
        ) {
            let mut rebuilt = vec![tokens[0].clone()];
            rebuilt.extend(tokens.iter().skip(2).cloned());
            return rebuilt.join(" ");
        }
    }
    trimmed.to_string()
}

/// Check if a command is interactive or requires direct terminal TTY access (editors, TUIs, pagers, REPLs, dev servers, live network/package commands).
pub fn is_interactive_command(input: &str) -> bool {
    let input = input.trim();
    if input.is_empty() {
        return false;
    }

    let tokens = parse_command_line(input);
    if tokens.is_empty() {
        return false;
    }

    // Explicit user prefixes: "live ...", "dev ...", "tui ...", "term ...", "interactive ..."
    if matches!(
        tokens[0].as_str(),
        "live" | "dev" | "tui" | "term" | "interactive"
    ) {
        return true;
    }

    // Inspect command, stripping leading sudo/doas and their flags
    let mut cmd_tokens = &tokens[..];
    if cmd_tokens[0] == "sudo" || cmd_tokens[0] == "doas" {
        let mut idx = 1;
        while idx < cmd_tokens.len() {
            let t = &cmd_tokens[idx];
            if t.starts_with('-') {
                if matches!(t.as_str(), "-u" | "-g" | "-C" | "-D" | "-R" | "-h" | "-p") {
                    idx += 2;
                } else if matches!(t.as_str(), "-i" | "-s") {
                    return true;
                } else {
                    idx += 1;
                }
            } else {
                break;
            }
        }
        if idx >= cmd_tokens.len() {
            return true;
        }
        cmd_tokens = &cmd_tokens[idx..];
    }

    if cmd_tokens.is_empty() {
        return false;
    }

    // Check if prefix was passed after sudo (e.g. "sudo live apt update")
    if matches!(
        cmd_tokens[0].as_str(),
        "live" | "dev" | "tui" | "term" | "interactive"
    ) {
        return true;
    }

    let raw_program = &cmd_tokens[0];
    let program = std::path::Path::new(raw_program)
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or(raw_program.as_str());
    let args: Vec<&str> = cmd_tokens.iter().skip(1).map(|s| s.as_str()).collect();

    // 1. Text editors
    if matches!(
        program,
        "nvim"
            | "vim"
            | "vi"
            | "nano"
            | "emacs"
            | "helix"
            | "hx"
            | "micro"
            | "kak"
            | "ed"
            | "pico"
            | "joe"
            | "vis"
            | "ne"
            | "view"
    ) {
        return true;
    }

    // 2. Interactive system / process monitors (TUIs)
    if matches!(
        program,
        "htop"
            | "top"
            | "btop"
            | "glances"
            | "nvtop"
            | "iotop"
            | "iftop"
            | "nethogs"
            | "powertop"
            | "tiptop"
            | "viddy"
            | "ctop"
            | "bashtop"
            | "bpytop"
            | "zenith"
    ) {
        return true;
    }

    // 3. Pagers / manual pages
    if matches!(program, "less" | "more" | "man" | "pager" | "info" | "most") {
        return true;
    }

    // 4. Terminal file managers
    if matches!(
        program,
        "ranger"
            | "yazi"
            | "nnn"
            | "lf"
            | "mc"
            | "midnight-commander"
            | "broot"
            | "vifm"
            | "clifm"
    ) {
        return true;
    }

    // 5. Git TUIs & interactive git workflows
    if matches!(program, "lazygit" | "tig" | "gitui") {
        return true;
    }
    if program == "git" {
        if let Some(sub) = args.first() {
            match *sub {
                // Network / transfer commands (require live uploading/downloading progress meters and credential prompts)
                "push" | "pull" | "clone" | "fetch" => {
                    return true;
                }
                "remote" => {
                    if args.iter().any(|a| matches!(*a, "update" | "prune")) {
                        return true;
                    }
                }
                // Pagers & diff inspections (open pager or colored diff viewer)
                "log" | "diff" | "show" | "blame" | "reflog" | "whatchanged" => {
                    return true;
                }
                // Commits: interactive editor unless message is provided via flags
                "commit" => {
                    let has_m = args.iter().any(|a| {
                        *a == "-m"
                            || a.starts_with("-m=")
                            || *a == "--message"
                            || a.starts_with("--message=")
                            || *a == "-F"
                            || *a == "-C"
                            || *a == "-c"
                    });
                    let is_patch = args.iter().any(|a| {
                        *a == "-p" || *a == "--patch" || *a == "-i" || *a == "--interactive"
                    });
                    if !has_m || is_patch {
                        return true;
                    }
                }
                // Rebase, merge, cherry-pick, revert, bisect
                "rebase" | "merge" | "cherry-pick" | "revert" | "bisect" => {
                    return true;
                }
                // Interactive patch or checkout/reset/stash/restore/switch
                "add" | "checkout" | "reset" | "restore" | "switch" => {
                    if args.iter().any(|a| {
                        *a == "-p" || *a == "--patch" || *a == "-i" || *a == "--interactive"
                    }) {
                        return true;
                    }
                }
                "stash" => {
                    if args.iter().any(|a| {
                        matches!(*a, "pop" | "apply" | "drop" | "show" | "branch")
                            || *a == "-p"
                            || *a == "--patch"
                    }) {
                        return true;
                    }
                }
                _ => {}
            }
        }
    }

    // 6. Dev servers, Bundlers, and Watchers (HMR, live servers, continuous processes)
    if matches!(
        program,
        "next"
            | "vite"
            | "nuxt"
            | "astro"
            | "remix"
            | "webpack"
            | "rollup"
            | "esbuild"
            | "parcel"
            | "turbo"
            | "turbopack"
            | "nodemon"
            | "live-server"
            | "http-server"
            | "serve"
            | "wrangler"
            | "flask"
            | "uvicorn"
            | "gunicorn"
            | "fastapi"
            | "rails"
            | "caddy"
            | "ngrok"
            | "cloudflared"
            | "localtunnel"
    ) {
        return true;
    }

    if program == "manage.py" && args.iter().any(|a| *a == "runserver") {
        return true;
    }

    // 7. Node / JS / TS Package Managers & Script Runners
    if matches!(program, "npm" | "pnpm" | "yarn" | "bun") {
        // Dev server / watch / start runs: e.g. "npm run dev", "pnpm dev", "yarn start", "bun dev"
        if args.iter().any(|a| matches!(*a, "dev" | "start" | "watch" | "serve" | "live" | "run-p")) {
            return true;
        }
        // Package installations / updates / interactive initializers
        if let Some(first_arg) = args.iter().find(|a| !a.starts_with('-')) {
            if matches!(
                *first_arg,
                "install"
                    | "i"
                    | "add"
                    | "update"
                    | "upgrade"
                    | "remove"
                    | "rm"
                    | "uninstall"
                    | "audit"
                    | "create"
                    | "init"
                    | "publish"
                    | "login"
            ) {
                return true;
            }
        }
        if args.iter().any(|a| *a == "-w" || *a == "--watch") {
            return true;
        }
    }

    if matches!(program, "npx" | "pnpx" | "bunx") {
        return true;
    }

    // 8. System Package Managers & Build/Run Tools (live progress bars, [Y/n] prompts)
    if matches!(
        program,
        "apt"
            | "apt-get"
            | "dpkg"
            | "aptitude"
            | "pacman"
            | "yay"
            | "paru"
            | "dnf"
            | "yum"
            | "rpm"
            | "zypper"
            | "apk"
            | "brew"
            | "snap"
            | "flatpak"
    ) {
        return true;
    }

    if matches!(program, "pip" | "pip3" | "conda" | "mamba") {
        if args.iter().any(|a| matches!(*a, "install" | "uninstall" | "download" | "upgrade")) {
            return true;
        }
    }

    if program == "cargo" && args.iter().any(|a| matches!(*a, "run" | "watch" | "install" | "binstall")) {
        return true;
    }

    if program == "go" && args.iter().any(|a| *a == "run") {
        return true;
    }

    if program == "dotnet" && args.iter().any(|a| *a == "run" || *a == "watch") {
        return true;
    }

    // 9. Continuous Streaming & Diagnostics
    if matches!(
        program,
        "ping"
            | "traceroute"
            | "tracepath"
            | "mtr"
            | "watch"
            | "tcpdump"
            | "tshark"
            | "wireshark"
    ) {
        return true;
    }

    if program == "tail" && args.iter().any(|a| *a == "-f" || *a == "-F" || *a == "--follow") {
        return true;
    }

    if program == "journalctl" && (args.is_empty() || args.iter().any(|a| *a == "-f" || *a == "--follow")) {
        return true;
    }

    if program == "dmesg" && args.iter().any(|a| *a == "-w" || *a == "--follow") {
        return true;
    }

    if matches!(program, "docker" | "podman") {
        if args.iter().any(|a| {
            matches!(*a, "attach" | "compose" | "stats")
                || *a == "-it"
                || *a == "-ti"
                || *a == "-f"
                || *a == "--follow"
        }) {
            return true;
        }
        if let Some(sub) = args.first() {
            if matches!(*sub, "compose") {
                return true;
            }
        }
    }

    if program == "kubectl" {
        if args.iter().any(|a| {
            matches!(*a, "exec" | "port-forward" | "attach")
                || *a == "-it"
                || *a == "-ti"
                || *a == "-f"
                || *a == "--follow"
        }) {
            return true;
        }
    }

    // 10. Multiplexers & remote shells
    if matches!(
        program,
        "tmux"
            | "screen"
            | "zellij"
            | "byobu"
            | "ssh"
            | "mosh"
            | "telnet"
            | "ftp"
            | "sftp"
            | "nmtui"
            | "ncmpcpp"
    ) {
        return true;
    }

    // 11. Interactive Debuggers & database / REPL shells
    if matches!(
        program,
        "gdb"
            | "lldb"
            | "irb"
            | "pry"
            | "ipython"
            | "bpython"
            | "sqlite3"
            | "psql"
            | "mysql"
            | "mongosh"
            | "redis-cli"
    ) {
        return true;
    }

    // 12. Shells & general REPLs when invoked interactively (without a script file or -c)
    if matches!(
        program,
        "bash" | "zsh" | "fish" | "sh" | "csh" | "tcsh" | "dash" | "ksh"
    ) {
        let has_script = args.iter().any(|a| *a == "-c" || !a.starts_with('-'));
        if !has_script {
            return true;
        }
    }

    if matches!(
        program,
        "python" | "python3" | "node" | "deno" | "bun" | "ruby" | "perl" | "php" | "lua"
    ) {
        let is_repl = args.is_empty() || args.iter().any(|a| *a == "-i" || *a == "-a");
        let has_script_file = args.iter().any(|a| !a.starts_with('-'));
        if is_repl && !has_script_file {
            return true;
        }
        if args.iter().any(|a| *a == "http.server" || *a == "runserver") {
            return true;
        }
    }

    // 13. Interactive authentication prompts
    if matches!(program, "passwd" | "su") {
        return true;
    }

    false
}

/// Execute an interactive command with direct terminal I/O (inherited stdin, stdout, stderr).
pub fn execute_interactive_command(input: &str) -> Result<std::process::ExitStatus, String> {
    let clean_str = clean_interactive_input(input);
    let clean_input = clean_str.trim();
    if clean_input.is_empty() {
        return Err("Empty command".to_string());
    }

    let tokens = parse_command_line(clean_input);
    if tokens.is_empty() {
        return Err("Empty command".to_string());
    }

    use std::process::{Command, Stdio};

    let mut cmd = if has_unquoted_shell_metachars(clean_input) {
        let mut c = Command::new("sh");
        c.arg("-c").arg(clean_input);
        c
    } else {
        let program = &tokens[0];
        let raw_args: Vec<&str> = tokens.iter().skip(1).map(|s| s.as_str()).collect();
        let mut c = Command::new(program);
        c.args(&raw_args);
        c
    };

    match cmd
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
    {
        Ok(status) => Ok(status),
        Err(e) => Err(format!("{}: {}", tokens[0], e)),
    }
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

            let mut stdout = child.stdout.take();
            let mut stderr = child.stderr.take();

            let stdout_reader = std::thread::spawn(move || {
                let mut buf = Vec::new();
                if let Some(mut r) = stdout.take() {
                    let _ = std::io::Read::read_to_end(&mut r, &mut buf);
                }
                buf
            });

            let stderr_reader = std::thread::spawn(move || {
                let mut buf = Vec::new();
                if let Some(mut r) = stderr.take() {
                    let _ = std::io::Read::read_to_end(&mut r, &mut buf);
                }
                buf
            });

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
                let _ = stdout_reader.join();
                let _ = stderr_reader.join();
                return vec!["^C [Process stopped by Esc]".to_string()];
            }

            let wait_res = child.wait();
            let stdout_bytes = stdout_reader.join().unwrap_or_default();
            let stderr_bytes = stderr_reader.join().unwrap_or_default();

            match wait_res {
                Ok(status) => {
                    let mut lines = Vec::new();
                    let stdout = String::from_utf8_lossy(&stdout_bytes);
                    let stderr = String::from_utf8_lossy(&stderr_bytes);

                    lines.extend(normalize_output_lines(&stdout));
                    let err_lines = normalize_output_lines(&stderr);
                    if !err_lines.is_empty() {
                        lines.extend(err_lines);
                    }

                    if lines.is_empty() && !status.success() {
                        if let Some(code) = status.code() {
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

    #[test]
    fn test_is_interactive_command() {
        // Editors
        assert!(is_interactive_command("nvim"));
        assert!(is_interactive_command("nvim src/main.rs"));
        assert!(is_interactive_command("nano /tmp/test.txt"));
        assert!(is_interactive_command("vim file.rs"));
        assert!(is_interactive_command("vi file.rs"));
        assert!(is_interactive_command("helix"));
        assert!(is_interactive_command("hx"));
        assert!(is_interactive_command("micro test.txt"));

        // Sudo with editors
        assert!(is_interactive_command("sudo nvim /etc/hosts"));
        assert!(is_interactive_command("sudo nano /etc/hosts"));
        assert!(is_interactive_command("sudo -E nvim"));
        assert!(is_interactive_command("sudo -i"));

        // TUIs & Monitors
        assert!(is_interactive_command("htop"));
        assert!(is_interactive_command("top"));
        assert!(is_interactive_command("btop"));
        assert!(is_interactive_command("lazygit"));
        assert!(is_interactive_command("ranger"));
        assert!(is_interactive_command("yazi"));
        assert!(is_interactive_command("tmux"));

        // Pagers
        assert!(is_interactive_command("less README.md"));
        assert!(is_interactive_command("man ls"));

        // Explicit TUI prefix for custom binaries
        assert!(is_interactive_command("tui ./my_custom_app"));
        assert!(is_interactive_command("term cargo run"));

        // Git network & interactive commands
        assert!(is_interactive_command("git push"));
        assert!(is_interactive_command("git push origin main"));
        assert!(is_interactive_command("git pull"));
        assert!(is_interactive_command("git pull --rebase"));
        assert!(is_interactive_command("git fetch"));
        assert!(is_interactive_command("git clone https://github.com/user/repo"));
        assert!(is_interactive_command("git log"));
        assert!(is_interactive_command("git diff"));
        assert!(is_interactive_command("git commit"));
        assert!(!is_interactive_command("git commit -m \"initial commit\""));
        assert!(is_interactive_command("git rebase -i HEAD~3"));
        assert!(is_interactive_command("git add -p"));
        assert!(is_interactive_command("git stash pop"));

        // Dev servers & watchers
        assert!(is_interactive_command("vite"));
        assert!(is_interactive_command("vite dev"));
        assert!(is_interactive_command("next dev"));
        assert!(is_interactive_command("next start"));
        assert!(is_interactive_command("nuxt dev"));
        assert!(is_interactive_command("astro dev"));
        assert!(is_interactive_command("remix dev"));
        assert!(is_interactive_command("nodemon index.js"));
        assert!(is_interactive_command("python -m http.server 8000"));
        assert!(is_interactive_command("python manage.py runserver"));

        // JS/TS package managers & runners
        assert!(is_interactive_command("npm run dev"));
        assert!(is_interactive_command("npm start"));
        assert!(is_interactive_command("npm run watch"));
        assert!(is_interactive_command("npm install"));
        assert!(is_interactive_command("npm i lodash"));
        assert!(is_interactive_command("pnpm dev"));
        assert!(is_interactive_command("pnpm add react"));
        assert!(is_interactive_command("yarn dev"));
        assert!(is_interactive_command("bun dev"));
        assert!(is_interactive_command("npx create-next-app"));

        // System package managers & runners
        assert!(is_interactive_command("apt install ripgrep"));
        assert!(is_interactive_command("sudo apt install ripgrep"));
        assert!(is_interactive_command("sudo apt-get update"));
        assert!(is_interactive_command("pacman -Syu"));
        assert!(is_interactive_command("cargo run"));
        assert!(is_interactive_command("cargo watch -x check"));
        assert!(is_interactive_command("pip install requests"));

        // Continuous streaming & diagnostics
        assert!(is_interactive_command("ping 8.8.8.8"));
        assert!(is_interactive_command("tail -f app.log"));
        assert!(is_interactive_command("journalctl -f"));

        // Explicit prefixes
        assert!(is_interactive_command("live curl -sSL https://example.com"));
        assert!(is_interactive_command("dev ./server"));
        assert!(is_interactive_command("sudo live apt update"));

        // Non-interactive commands (remain captured in TUI output history)
        assert!(!is_interactive_command("ls -la"));
        assert!(!is_interactive_command("cat Cargo.toml"));
        assert!(!is_interactive_command("git status"));
        assert!(!is_interactive_command("cargo test"));
        assert!(!is_interactive_command("cargo check"));
        assert!(!is_interactive_command("echo hello world"));
    }

    #[test]
    fn test_clean_interactive_input() {
        assert_eq!(clean_interactive_input("live git push"), "git push");
        assert_eq!(clean_interactive_input("dev vite"), "vite");
        assert_eq!(clean_interactive_input("tui nvim"), "nvim");
        assert_eq!(clean_interactive_input("term ./app"), "./app");
        assert_eq!(clean_interactive_input("sudo live apt update"), "sudo apt update");
        assert_eq!(clean_interactive_input("git push origin main"), "git push origin main");
    }

    #[test]
    fn test_execute_command_large_output_no_deadlock() {
        // Output > 64KB (e.g. 100KB) tests that pipe buffers do not deadlock when drained asynchronously
        let output = execute_command("python3 -c \"print('X' * 100000)\"");
        assert!(!output.is_empty());
        let total_len: usize = output.iter().map(|s| s.len()).sum();
        assert!(total_len >= 100000);
    }
}
