// Command completion and suggestion logic for NSH Shell
// Supports command completion, flag completion, and file/directory completion

use super::state::App;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

// Lazily loaded list of executable commands from PATH
pub static PATH_COMMANDS: LazyLock<Vec<String>> = LazyLock::new(|| {
    std::env::var("PATH")
        .unwrap_or_default()
        .split(':')
        .filter_map(|dir| {
            std::fs::read_dir(dir).ok().map(|d| {
                d.filter_map(|e| e.ok())
                    .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
                    .filter_map(|e| e.file_name().into_string().ok())
                    .collect::<Vec<_>>()
            })
        })
        .flatten()
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect()
});

// Built-in commands supported by nsh shell
pub const BUILTIN_COMMANDS: &[&str] = &[
    "ask", "/ask",
    "do", "/do",
    "plan", "/plan",
    "build", "/build",
    "index", "/index",
    "/settings", "settings",
    "/help", "help",
    "cd", "clear", "exit", "quit",
];

// Common flags for popular CLI tools
const RM_FLAGS: &[(&str, &str)] = &[
    ("-rf", "-rf  (recursive & force)"),
    ("-r", "-r  (remove directories recursively)"),
    ("-f", "-f  (ignore nonexistent files, never prompt)"),
    ("-i", "-i  (prompt before every removal)"),
    ("-I", "-I  (prompt once before removing >3 files)"),
    ("-d", "-d  (remove empty directories)"),
    ("-v", "-v  (verbose: explain what is being done)"),
    ("--recursive", "--recursive  (remove directories and contents)"),
    ("--force", "--force  (never prompt)"),
    ("--verbose", "--verbose"),
    ("--help", "--help"),
    ("--version", "--version"),
];

const CAT_FLAGS: &[(&str, &str)] = &[
    ("-n", "-n  (number all output lines)"),
    ("-b", "-b  (number nonempty output lines)"),
    ("-s", "-s  (squeeze multiple adjacent empty lines)"),
    ("-E", "-E  (display $ at end of each line)"),
    ("-T", "-T  (display TAB characters as ^I)"),
    ("-A", "-A  (show all / equivalent to -vET)"),
    ("-v", "-v  (use ^ and M- notation)"),
    ("--number", "--number  (number all output lines)"),
    ("--number-nonblank", "--number-nonblank"),
    ("--squeeze-blank", "--squeeze-blank"),
    ("--help", "--help"),
    ("--version", "--version"),
];

const LS_FLAGS: &[(&str, &str)] = &[
    ("-l", "-l  (use long listing format)"),
    ("-a", "-a  (all, do not ignore entries starting with .)"),
    ("-la", "-la  (long listing, all entries)"),
    ("-lh", "-lh  (long listing, human-readable sizes)"),
    ("-lah", "-lah  (long listing, all entries, human-readable)"),
    ("-h", "-h  (human-readable sizes: 1K, 234M, 2G)"),
    ("-r", "-r  (reverse order while sorting)"),
    ("-t", "-t  (sort by modification time, newest first)"),
    ("-R", "-R  (list subdirectories recursively)"),
    ("-1", "-1  (list one file per line)"),
    ("-S", "-S  (sort by file size, largest first)"),
    ("-d", "-d  (list directories themselves, not contents)"),
    ("--all", "--all"),
    ("--human-readable", "--human-readable"),
    ("--help", "--help"),
    ("--version", "--version"),
];

const CP_FLAGS: &[(&str, &str)] = &[
    ("-r", "-r  (copy directories recursively)"),
    ("-R", "-R  (copy directories recursively)"),
    ("-a", "-a  (archive: same as -dR --preserve=all)"),
    ("-f", "-f  (force: overwrite existing destinations)"),
    ("-i", "-i  (interactive: prompt before overwrite)"),
    ("-p", "-p  (preserve file attributes)"),
    ("-u", "-u  (update: copy only when source is newer)"),
    ("-v", "-v  (verbose)"),
    ("--recursive", "--recursive"),
    ("--force", "--force"),
    ("--help", "--help"),
];

const MV_FLAGS: &[(&str, &str)] = &[
    ("-f", "-f  (force: do not prompt before overwriting)"),
    ("-i", "-i  (interactive: prompt before overwrite)"),
    ("-n", "-n  (no-clobber: do not overwrite existing file)"),
    ("-u", "-u  (update: move only when source is newer)"),
    ("-v", "-v  (verbose)"),
    ("--force", "--force"),
    ("--help", "--help"),
];

const MKDIR_FLAGS: &[(&str, &str)] = &[
    ("-p", "-p  (parents: make parent directories as needed)"),
    ("-v", "-v  (verbose: print message for each directory)"),
    ("-m", "-m  (mode: set file permissions)"),
    ("--parents", "--parents"),
    ("--help", "--help"),
];

const GREP_FLAGS: &[(&str, &str)] = &[
    ("-i", "-i  (ignore case distinctions)"),
    ("-r", "-r  (recursively search subdirectories)"),
    ("-rn", "-rn  (recursive with line numbers)"),
    ("-rni", "-rni  (recursive, line numbers, ignore case)"),
    ("-n", "-n  (print line number with output lines)"),
    ("-v", "-v  (invert match: select non-matching lines)"),
    ("-l", "-l  (print only names of matching files)"),
    ("-w", "-w  (match only whole words)"),
    ("-E", "-E  (extended regular expressions / egrep)"),
    ("-F", "-F  (fixed strings / fgrep)"),
    ("-c", "-c  (print count of matching lines)"),
    ("-o", "-o  (show only matching parts of line)"),
    ("--ignore-case", "--ignore-case"),
    ("--recursive", "--recursive"),
    ("--line-number", "--line-number"),
    ("--help", "--help"),
];

const CURL_FLAGS: &[(&str, &str)] = &[
    ("-X", "-X <method>  (HTTP method: GET, POST, PUT, DELETE)"),
    ("-H", "-H <header>  (pass custom HTTP header)"),
    ("-d", "-d <data>  (HTTP POST data)"),
    ("-s", "-s  (silent mode)"),
    ("-S", "-S  (show error when used with -s)"),
    ("-o", "-o <file>  (write output to file)"),
    ("-O", "-O  (write output to file named like remote)"),
    ("-i", "-i  (include HTTP response headers)"),
    ("-I", "-I  (fetch headers only / HEAD request)"),
    ("-L", "-L  (follow HTTP redirects)"),
    ("-k", "-k  (insecure: allow unverified SSL)"),
    ("-u", "-u <user:pass>  (server user and password)"),
    ("-v", "-v  (verbose: show full request/response)"),
    ("--request", "--request <method>"),
    ("--header", "--header <header>"),
    ("--data", "--data <data>"),
    ("--silent", "--silent"),
    ("--location", "--location"),
    ("--help", "--help"),
];

const TAR_FLAGS: &[(&str, &str)] = &[
    ("-xvf", "-xvf  (extract, verbose, from file)"),
    ("-cvf", "-cvf  (create, verbose, to file)"),
    ("-xzvf", "-xzvf  (extract gzip, verbose, from file)"),
    ("-czvf", "-czvf  (create gzip, verbose, to file)"),
    ("-tvf", "-tvf  (list contents, verbose, from file)"),
    ("-x", "-x  (extract files from archive)"),
    ("-c", "-c  (create a new archive)"),
    ("-t", "-t  (list archive contents)"),
    ("-z", "-z  (filter through gzip)"),
    ("-j", "-j  (filter through bzip2)"),
    ("-J", "-J  (filter through xz)"),
    ("-v", "-v  (verbosely list files processed)"),
    ("-f", "-f  (use archive file)"),
    ("-C", "-C <dir>  (change to directory)"),
    ("--extract", "--extract"),
    ("--create", "--create"),
    ("--help", "--help"),
];

const CHMOD_FLAGS: &[(&str, &str)] = &[
    ("-R", "-R  (change files and directories recursively)"),
    ("-v", "-v  (verbose diagnostic for each file)"),
    ("-c", "-c  (report only when change is made)"),
    ("+x", "+x  (make executable)"),
    ("-x", "-x  (remove executable permission)"),
    ("755", "755  (rwxr-xr-x)"),
    ("644", "644  (rw-r--r--)"),
    ("700", "700  (rwx------)"),
    ("600", "600  (rw-------)"),
    ("--recursive", "--recursive"),
    ("--help", "--help"),
];

const CHOWN_FLAGS: &[(&str, &str)] = &[
    ("-R", "-R  (operate recursively)"),
    ("-v", "-v  (verbose)"),
    ("-c", "-c  (report changes only)"),
    ("--recursive", "--recursive"),
    ("--help", "--help"),
];

const FIND_FLAGS: &[(&str, &str)] = &[
    ("-type", "-type [f|d|l]  (filter by file type)"),
    ("-name", "-name <pattern>  (match name case-sensitive)"),
    ("-iname", "-iname <pattern>  (match name case-insensitive)"),
    ("-maxdepth", "-maxdepth <N>  (descend at most N levels)"),
    ("-mindepth", "-mindepth <N>  (descend at least N levels)"),
    ("-size", "-size <size>  (filter by file size)"),
    ("-mtime", "-mtime <days>  (modified days ago)"),
    ("-exec", "-exec <cmd> {} +  (execute command on matches)"),
    ("-delete", "-delete  (delete matching files)"),
    ("-empty", "-empty  (empty file or directory)"),
    ("--help", "--help"),
];

const GIT_COMMANDS: &[(&str, &str)] = &[
    ("status", "status  (show working tree status)"),
    ("add", "add  (add file contents to index)"),
    ("commit", "commit  (record changes to repository)"),
    ("push", "push  (update remote refs)"),
    ("pull", "pull  (fetch and integrate with local repo)"),
    ("checkout", "checkout  (switch branches or restore paths)"),
    ("branch", "branch  (list, create, or delete branches)"),
    ("diff", "diff  (show changes between commits/work tree)"),
    ("log", "log  (show commit logs)"),
    ("clone", "clone  (clone a repository)"),
    ("fetch", "fetch  (download objects and refs from another repo)"),
    ("merge", "merge  (join two or more development histories)"),
    ("rebase", "rebase  (reapply commits on top of another base)"),
    ("reset", "reset  (reset current HEAD to specified state)"),
    ("stash", "stash  (stash changes in dirty working directory)"),
    ("switch", "switch  (switch branches)"),
    ("restore", "restore  (restore working tree files)"),
    ("tag", "tag  (create, list, delete or verify tags)"),
    ("remote", "remote  (manage set of tracked repositories)"),
    ("-C", "-C <path>  (run as if git was started in path)"),
    ("--version", "--version"),
    ("--help", "--help"),
];

const GIT_CHECKOUT_FLAGS: &[(&str, &str)] = &[
    ("-b", "-b <branch>  (create and checkout new branch)"),
    ("-B", "-B <branch>  (create or reset and checkout branch)"),
    ("--orphan", "--orphan <branch>  (create orphan branch)"),
    ("-f", "-f  (force checkout / discard local changes)"),
];

const GIT_COMMIT_FLAGS: &[(&str, &str)] = &[
    ("-m", "-m <msg>  (commit message)"),
    ("-am", "-am <msg>  (stage all modified and commit with message)"),
    ("-a", "-a  (automatically stage modified/deleted files)"),
    ("--amend", "--amend  (amend previous commit)"),
    ("--no-verify", "--no-verify  (bypass pre-commit/commit-msg hooks)"),
];

const GIT_STATUS_FLAGS: &[(&str, &str)] = &[
    ("-s", "-s  (short format)"),
    ("-b", "-b  (show branch info in short format)"),
    ("--short", "--short"),
    ("--branch", "--branch"),
];

const GIT_PUSH_FLAGS: &[(&str, &str)] = &[
    ("-u", "-u  (set upstream tracking)"),
    ("-f", "-f  (force push)"),
    ("--force", "--force"),
    ("--tags", "--tags"),
    ("--set-upstream", "--set-upstream"),
];

const GIT_PULL_FLAGS: &[(&str, &str)] = &[
    ("-r", "-r  (rebase instead of merge)"),
    ("--rebase", "--rebase"),
    ("--ff-only", "--ff-only"),
];

const GIT_ADD_FLAGS: &[(&str, &str)] = &[
    ("-A", "-A  (add all tracked and untracked changes)"),
    ("-u", "-u  (update tracked files only)"),
    ("-p", "-p  (interactively choose hunks to add)"),
    (".", ".  (add current directory)"),
    ("--all", "--all"),
];

const GIT_LOG_FLAGS: &[(&str, &str)] = &[
    ("--oneline", "--oneline  (compact one-line format)"),
    ("-n", "-n <number>  (limit number of commits)"),
    ("-p", "-p  (show patch / diff for each commit)"),
    ("--graph", "--graph  (text-based graphical representation)"),
    ("--stat", "--stat  (generate diffstat for each commit)"),
];

const DOCKER_COMMANDS: &[(&str, &str)] = &[
    ("ps", "ps  (list containers)"),
    ("run", "run  (create and run a new container)"),
    ("exec", "exec  (execute a command in a running container)"),
    ("build", "build  (build an image from a Dockerfile)"),
    ("images", "images  (list images)"),
    ("stop", "stop  (stop running container(s))"),
    ("start", "start  (start stopped container(s))"),
    ("restart", "restart  (restart container(s))"),
    ("rm", "rm  (remove container(s))"),
    ("rmi", "rmi  (remove image(s))"),
    ("logs", "logs  (fetch logs of a container)"),
    ("compose", "compose  (Docker Compose commands)"),
    ("network", "network  (manage networks)"),
    ("volume", "volume  (manage volumes)"),
    ("pull", "pull  (download an image)"),
    ("push", "push  (upload an image)"),
    ("-d", "-d  (run container in background / detached)"),
    ("-it", "-it  (interactive with pseudo-TTY)"),
    ("-p", "-p  (publish port: host:container)"),
    ("-v", "-v  (bind mount a volume)"),
    ("-e", "-e  (set environment variables)"),
    ("--name", "--name  (assign a name to container)"),
    ("--rm", "--rm  (remove container when it exits)"),
    ("--help", "--help"),
];

const CARGO_COMMANDS: &[(&str, &str)] = &[
    ("build", "build  (compile current package)"),
    ("check", "check  (analyze package for errors without compiling)"),
    ("run", "run  (run binary or example)"),
    ("test", "test  (execute all unit and integration tests)"),
    ("clean", "clean  (remove generated artifacts)"),
    ("update", "update  (update dependencies in Cargo.lock)"),
    ("add", "add  (add dependencies to Cargo.toml)"),
    ("remove", "remove  (remove dependencies from Cargo.toml)"),
    ("new", "new  (create a new cargo package at <path>)"),
    ("init", "init  (create a new cargo package in cwd)"),
    ("clippy", "clippy  (catch common mistakes)"),
    ("fmt", "fmt  (format rust source files)"),
    ("doc", "doc  (build documentation)"),
    ("tree", "tree  (display dependency tree)"),
    ("--release", "--release  (build artifacts in release mode)"),
    ("--features", "--features <features>"),
    ("--all-features", "--all-features"),
    ("--verbose", "--verbose"),
    ("-v", "-v  (verbose)"),
    ("-q", "-q  (quiet)"),
    ("--workspace", "--workspace"),
    ("--help", "--help"),
];

const KILL_FLAGS: &[(&str, &str)] = &[
    ("-9", "-9  (SIGKILL: force kill immediately)"),
    ("-15", "-15  (SIGTERM: graceful termination)"),
    ("-2", "-2  (SIGINT: interrupt / Ctrl+C)"),
    ("-l", "-l  (list signal names)"),
];

const PS_FLAGS: &[(&str, &str)] = &[
    ("-aux", "-aux  (list all running processes in detail)"),
    ("-ef", "-ef  (list all processes standard format)"),
    ("-u", "-u <user>  (filter by user)"),
    ("-p", "-p <pid>  (filter by process ID)"),
];

const HEAD_TAIL_FLAGS: &[(&str, &str)] = &[
    ("-n", "-n <lines>  (number of lines to show)"),
    ("-f", "-f  (follow: output appended data as file grows)"),
    ("-F", "-F  (follow by name with retry)"),
    ("-c", "-c <bytes>  (output first/last bytes)"),
    ("-q", "-q  (never print headers with file names)"),
    ("-v", "-v  (always print headers with file names)"),
];

const TOUCH_FLAGS: &[(&str, &str)] = &[
    ("-a", "-a  (change only access time)"),
    ("-m", "-m  (change only modification time)"),
    ("-c", "-c  (do not create any files)"),
    ("-d", "-d <date>  (parse date string)"),
];

const DF_DU_FLAGS: &[(&str, &str)] = &[
    ("-h", "-h  (human readable sizes: 1K, 234M, 2G)"),
    ("-s", "-s  (display only a total for each argument)"),
    ("-sh", "-sh  (summary and human readable)"),
    ("-a", "-a  (all files, not just directories)"),
    ("-c", "-c  (produce a grand total)"),
    ("-d", "-d <depth>  (maximum depth)"),
];

const ECHO_FLAGS: &[(&str, &str)] = &[
    ("-n", "-n  (do not output trailing newline)"),
    ("-e", "-e  (enable interpretation of backslash escapes)"),
    ("-E", "-E  (disable interpretation of backslash escapes)"),
];

const GENERIC_FLAGS: &[(&str, &str)] = &[
    ("--help", "--help  (display help and exit)"),
    ("--version", "--version  (output version info and exit)"),
    ("-h", "-h  (display help)"),
    ("-v", "-v  (verbose / version)"),
];

/// Returns flag and subcommand suggestions for a given command.
pub fn get_command_flags<'a>(
    cmd: &'a str,
    subcmd: Option<&'a str>,
) -> &'static [(&'static str, &'static str)] {
    let clean_cmd = cmd.trim_start_matches('/');
    match clean_cmd {
        "rm" => RM_FLAGS,
        "cat" => CAT_FLAGS,
        "ls" | "dir" | "vdir" => LS_FLAGS,
        "cp" => CP_FLAGS,
        "mv" => MV_FLAGS,
        "mkdir" => MKDIR_FLAGS,
        "grep" | "rg" => GREP_FLAGS,
        "curl" => CURL_FLAGS,
        "tar" => TAR_FLAGS,
        "chmod" => CHMOD_FLAGS,
        "chown" => CHOWN_FLAGS,
        "find" => FIND_FLAGS,
        "kill" => KILL_FLAGS,
        "ps" => PS_FLAGS,
        "head" | "tail" => HEAD_TAIL_FLAGS,
        "touch" => TOUCH_FLAGS,
        "df" | "du" => DF_DU_FLAGS,
        "echo" => ECHO_FLAGS,
        "docker" => DOCKER_COMMANDS,
        "cargo" => CARGO_COMMANDS,
        "git" => match subcmd {
            Some("checkout" | "switch") => GIT_CHECKOUT_FLAGS,
            Some("commit") => GIT_COMMIT_FLAGS,
            Some("status") => GIT_STATUS_FLAGS,
            Some("push") => GIT_PUSH_FLAGS,
            Some("pull") => GIT_PULL_FLAGS,
            Some("add") => GIT_ADD_FLAGS,
            Some("log") => GIT_LOG_FLAGS,
            _ => GIT_COMMANDS,
        },
        _ => GENERIC_FLAGS,
    }
}

/// Expand `~` prefix to `$HOME`.
fn expand_tilde(path: &str) -> String {
    if path == "~" {
        std::env::var("HOME").unwrap_or_else(|_| String::from("~"))
    } else if let Some(rest) = path.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            format!("{}/{}", home, rest)
        } else {
            path.to_string()
        }
    } else {
        path.to_string()
    }
}

/// Parse a path string into `(base_part, prefix)`.
/// `base_part`: directory portion as typed (e.g. `""`, `"src/"`, `"./"`, `"~/"`, `"/usr/"`).
/// `prefix`: entry prefix being typed within `base_part`.
pub fn parse_dir_path(path_token: &str) -> (String, String) {
    if path_token.is_empty() {
        return (String::new(), String::new());
    }

    if let Some(last_slash_idx) = path_token.rfind('/') {
        let base = &path_token[..last_slash_idx + 1];
        let prefix = &path_token[last_slash_idx + 1..];
        (base.to_string(), prefix.to_string())
    } else {
        (String::new(), path_token.to_string())
    }
}

/// Find matching files and directories in `base_part` matching `prefix`.
/// Returns `Vec<(completed_arg, display_name)>`.
/// - Directories end with `/` and do not have a trailing space.
/// - Files have a trailing space added to `completed_arg`.
pub fn complete_paths(
    base_part: &str,
    prefix: &str,
    directories_only: bool,
) -> Vec<(String, String)> {
    let mut results: Vec<(String, String)> = Vec::new();

    // Determine the filesystem directory to read
    let fs_dir: PathBuf = if base_part.is_empty() {
        PathBuf::from(".")
    } else if base_part == "~" || base_part == "~/" {
        PathBuf::from(expand_tilde("~"))
    } else if base_part.starts_with("~/") {
        PathBuf::from(expand_tilde(base_part))
    } else {
        PathBuf::from(base_part)
    };

    // Special handling for `..` and `.`
    if base_part.is_empty() && prefix == ".." {
        results.push((String::from("../"), String::from("../")));
        return results;
    }
    if base_part.is_empty() && prefix == "." {
        results.push((String::from("./"), String::from("./")));
        results.push((String::from("../"), String::from("../")));
    }

    let entries = match std::fs::read_dir(&fs_dir) {
        Ok(e) => e,
        Err(_) => return results,
    };

    let unescaped_prefix = prefix.replace("\\ ", " ");
    let prefix_lower = unescaped_prefix.to_lowercase();
    let show_hidden = unescaped_prefix.starts_with('.');

    for entry in entries.flatten() {
        let name = match entry.file_name().into_string() {
            Ok(n) => n,
            Err(_) => continue,
        };

        // Skip hidden entries unless user explicitly typed a leading dot
        if !show_hidden && name.starts_with('.') {
            continue;
        }

        // Check if directory (following symlinks)
        let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false)
            || std::fs::metadata(entry.path()).map(|m| m.is_dir()).unwrap_or(false);

        if directories_only && !is_dir {
            continue;
        }

        let name_lower = name.to_lowercase();
        if !prefix.is_empty() && !name_lower.starts_with(&prefix_lower) {
            continue;
        }

        // Escape spaces for command-line insertion
        let escaped_name = if name.contains(' ') {
            name.replace(' ', "\\ ")
        } else {
            name.clone()
        };

        if is_dir {
            let completed = format!("{}{}/", base_part, escaped_name);
            let display = format!("{}/", name);
            results.push((completed, display));
        } else {
            let completed = format!("{}{}{} ", base_part, escaped_name, "");
            let display = name;
            results.push((completed, display));
        }
    }

    // Sort: directories first, then files; exact prefix case matches prioritized
    results.sort_by(|a, b| {
        let a_is_dir = a.1.ends_with('/');
        let b_is_dir = b.1.ends_with('/');
        if a_is_dir != b_is_dir {
            return b_is_dir.cmp(&a_is_dir);
        }
        let a_exact = a.1.starts_with(&unescaped_prefix);
        let b_exact = b.1.starts_with(&unescaped_prefix);
        if a_exact != b_exact {
            return b_exact.cmp(&a_exact);
        }
        a.1.to_lowercase().cmp(&b.1.to_lowercase())
    });

    results
}

/// Find local git branches if inside a git repository.
pub fn get_git_branches(prefix: &str) -> Vec<(String, String)> {
    let mut branches = Vec::new();
    let prefix_lower = prefix.to_lowercase();

    // Check .git/refs/heads
    let heads_dir = Path::new(".git/refs/heads");
    if let Ok(entries) = std::fs::read_dir(heads_dir) {
        for entry in entries.flatten() {
            if let Ok(name) = entry.file_name().into_string() {
                if prefix.is_empty() || name.to_lowercase().starts_with(&prefix_lower) {
                    branches.push((format!("{} ", name), format!("{}  (branch)", name)));
                }
            }
        }
    }

    branches.sort_by(|a, b| a.0.cmp(&b.0));
    branches
}

/// Representation of parsed input line up to the cursor position.
#[derive(Debug, PartialEq, Eq)]
pub struct ParsedInput<'a> {
    pub is_command_position: bool,
    pub command: &'a str,
    pub subcommand: Option<&'a str>,
    pub line_prefix: &'a str,
    pub current_token: &'a str,
    pub line_suffix: &'a str,
}

/// Parse input at current cursor position into command, arguments, and current token.
pub fn parse_input_at_cursor<'a>(input: &'a str, cursor_pos: usize) -> ParsedInput<'a> {
    let cursor = cursor_pos.min(input.len());
    let before_cursor = &input[..cursor];
    let line_suffix = &input[cursor..];

    // Find the boundary of the current token being completed
    if let Some(ws_idx) = before_cursor.rfind(|c: char| c.is_whitespace()) {
        let line_prefix = &before_cursor[..ws_idx + 1];
        let current_token = &before_cursor[ws_idx + 1..];

        let mut words = line_prefix.split_whitespace();
        let command = words.next().unwrap_or("");
        let subcommand = words.next();

        let is_command_position = command.is_empty();

        ParsedInput {
            is_command_position,
            command,
            subcommand,
            line_prefix,
            current_token,
            line_suffix,
        }
    } else {
        // No whitespace before cursor -> typing command name
        ParsedInput {
            is_command_position: true,
            command: "",
            subcommand: None,
            line_prefix: "",
            current_token: before_cursor,
            line_suffix,
        }
    }
}

// Update suggestions based on current input
pub fn update_suggestions(app: &mut App) {
    if app.current_input.is_empty() {
        app.current_suggestions.clear();
        app.show_suggestions = false;
        app.selected_suggestion = 0;
        app.suggestion_scroll_offset = 0;
        return;
    }

    let parsed = parse_input_at_cursor(&app.current_input, app.cursor_position);
    let mut suggestions: Vec<(String, String)> = Vec::new();

    if parsed.is_command_position {
        let token_lower = parsed.current_token.to_lowercase();

        // If user is typing a path in command position (e.g. `./script.sh` or `/bin/ls`)
        if parsed.current_token.contains('/') || parsed.current_token.starts_with('.') {
            let (base_dir, prefix) = parse_dir_path(parsed.current_token);
            let paths = complete_paths(&base_dir, &prefix, false);
            for (completed_path, display) in paths {
                let full = format!("{}{}{}", parsed.line_prefix, completed_path, parsed.line_suffix);
                suggestions.push((full, display));
            }
        } else {
            // Built-in AI + shell commands
            for &cmd in BUILTIN_COMMANDS {
                if cmd.to_lowercase().starts_with(&token_lower) {
                    let full = format!("{}{}{} {}", parsed.line_prefix, cmd, "", parsed.line_suffix);
                    suggestions.push((full, cmd.to_string()));
                }
            }

            // Command completion from PATH
            for cmd in PATH_COMMANDS.iter() {
                if cmd.to_lowercase().starts_with(&token_lower) {
                    let full = format!("{}{}{} ", parsed.line_prefix, cmd, parsed.line_suffix);
                    suggestions.push((full, cmd.clone()));
                }
            }

            // Add matching commands from history
            for entry in app.entries.iter().rev() {
                if entry.entry_type == super::state::EntryType::Command {
                    if let Some(cmd) = entry.content.first() {
                        let cmd_lower = cmd.to_lowercase();
                        if cmd_lower.starts_with(&token_lower)
                            && !suggestions.iter().any(|s| s.1 == *cmd)
                        {
                            let full = format!("{}{}{}", parsed.line_prefix, cmd, parsed.line_suffix);
                            suggestions.push((full, cmd.clone()));
                        }
                    }
                }
                if suggestions.len() >= 30 {
                    break;
                }
            }
        }
    } else {
        // Argument position: file, directory, or flag completion
        let cmd = parsed.command;
        let token = parsed.current_token;
        let is_flag = token.starts_with('-') || (cmd == "chmod" && token.starts_with('+'));
        let is_cd = matches!(cmd, "cd" | "rmdir" | "pushd");

        if is_flag {
            // Suggest matching flags for the command
            let flags = get_command_flags(cmd, parsed.subcommand);
            for &(flag, desc) in flags {
                if flag.starts_with(token) || token == "-" {
                    let full = format!("{}{}{} {}", parsed.line_prefix, flag, "", parsed.line_suffix);
                    suggestions.push((full, desc.to_string()));
                }
            }
        } else {
            // Suggest files and directories
            let (base_dir, prefix) = parse_dir_path(token);
            let paths = complete_paths(&base_dir, &prefix, is_cd);

            for (completed_path, display) in paths {
                let full = format!("{}{}{}", parsed.line_prefix, completed_path, parsed.line_suffix);
                suggestions.push((full, display));
            }

            // Git subcommands and branch completion
            if cmd == "git" {
                if parsed.subcommand.is_none() && base_dir.is_empty() {
                    for &(sc, desc) in GIT_COMMANDS {
                        if !sc.starts_with('-') && (token.is_empty() || sc.starts_with(token)) {
                            let full = format!("{}{}{} ", parsed.line_prefix, sc, parsed.line_suffix);
                            suggestions.push((full, desc.to_string()));
                        }
                    }
                } else if matches!(parsed.subcommand, Some("checkout" | "switch" | "merge" | "rebase" | "branch" | "diff")) {
                    let branches = get_git_branches(token);
                    for (branch_arg, display) in branches {
                        let full = format!("{}{}{}", parsed.line_prefix, branch_arg, parsed.line_suffix);
                        suggestions.push((full, display));
                    }
                }
            }

            // Also show common command flags when the current token is empty and not `cd`
            if token.is_empty() && !is_cd {
                let flags = get_command_flags(cmd, parsed.subcommand);
                for &(flag, desc) in flags.iter().take(6) {
                    let full = format!("{}{}{} ", parsed.line_prefix, flag, parsed.line_suffix);
                    suggestions.push((full, desc.to_string()));
                }
            }
        }
    }

    // Deduplicate suggestions by full completion string (s.0)
    let mut seen = std::collections::HashSet::new();
    suggestions.retain(|item| seen.insert(item.0.clone()));

    app.current_suggestions = suggestions;
    app.show_suggestions = !app.current_suggestions.is_empty();
    app.selected_suggestion = 0;
    app.suggestion_scroll_offset = 0;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_input_at_cursor_command() {
        let p = parse_input_at_cursor("ca", 2);
        assert!(p.is_command_position);
        assert_eq!(p.current_token, "ca");
        assert_eq!(p.line_prefix, "");
    }

    #[test]
    fn test_parse_input_at_cursor_after_cat() {
        let p = parse_input_at_cursor("cat ", 4);
        assert!(!p.is_command_position);
        assert_eq!(p.command, "cat");
        assert_eq!(p.line_prefix, "cat ");
        assert_eq!(p.current_token, "");
    }

    #[test]
    fn test_parse_input_at_cursor_cat_arg() {
        let p = parse_input_at_cursor("cat README.m", 12);
        assert!(!p.is_command_position);
        assert_eq!(p.command, "cat");
        assert_eq!(p.line_prefix, "cat ");
        assert_eq!(p.current_token, "README.m");
    }

    #[test]
    fn test_parse_input_at_cursor_rm_flag() {
        let p = parse_input_at_cursor("rm -", 4);
        assert!(!p.is_command_position);
        assert_eq!(p.command, "rm");
        assert_eq!(p.line_prefix, "rm ");
        assert_eq!(p.current_token, "-");
    }

    #[test]
    fn test_parse_input_at_cursor_rm_rf_arg() {
        let p = parse_input_at_cursor("rm -rf tar", 10);
        assert!(!p.is_command_position);
        assert_eq!(p.command, "rm");
        assert_eq!(p.subcommand, Some("-rf"));
        assert_eq!(p.line_prefix, "rm -rf ");
        assert_eq!(p.current_token, "tar");
    }

    #[test]
    fn test_parse_dir_path() {
        assert_eq!(parse_dir_path(""), (String::new(), String::new()));
        assert_eq!(parse_dir_path("src"), (String::new(), "src".to_string()));
        assert_eq!(parse_dir_path("src/"), ("src/".to_string(), String::new()));
        assert_eq!(parse_dir_path("src/m"), ("src/".to_string(), "m".to_string()));
        assert_eq!(parse_dir_path("~/proj/"), ("~/proj/".to_string(), String::new()));
    }

    #[test]
    fn test_get_command_flags() {
        let rm_flags = get_command_flags("rm", None);
        assert!(rm_flags.iter().any(|(f, _)| *f == "-rf"));
        assert!(rm_flags.iter().any(|(f, _)| *f == "-r"));
        assert!(rm_flags.iter().any(|(f, _)| *f == "-f"));

        let cat_flags = get_command_flags("cat", None);
        assert!(cat_flags.iter().any(|(f, _)| *f == "-n"));
    }

    #[test]
    fn test_complete_paths_current_dir() {
        let _guard = crate::tools::terminal::TEST_CWD_LOCK.lock().unwrap();
        let paths = complete_paths("", "Cargo", false);
        assert!(paths.iter().any(|(p, _)| p.contains("Cargo.toml")));
    }

    #[test]
    fn test_complete_paths_cd_only_dirs() {
        let _guard = crate::tools::terminal::TEST_CWD_LOCK.lock().unwrap();
        let paths = complete_paths("", "src", true);
        assert!(paths.iter().any(|(p, _)| p == "src/"));

        // Files like Cargo.toml should NOT be returned when directories_only is true
        let file_paths = complete_paths("", "Cargo", true);
        assert!(file_paths.is_empty());
    }

    #[test]
    fn test_update_suggestions_cat_files() {
        let _guard = crate::tools::terminal::TEST_CWD_LOCK.lock().unwrap();
        let mut app = App::new();
        app.current_input = "cat ".to_string();
        app.cursor_position = 4;
        update_suggestions(&mut app);

        assert!(app.show_suggestions);
        assert!(!app.current_suggestions.is_empty());
        // Should suggest files like Cargo.toml or README.md
        assert!(
            app.current_suggestions
                .iter()
                .any(|(full, _)| full.starts_with("cat Cargo.toml") || full.starts_with("cat README.md")),
            "cat should suggest files in cwd, got: {:?}",
            app.current_suggestions
        );
    }

    #[test]
    fn test_update_suggestions_rm_flags() {
        let mut app = App::new();
        app.current_input = "rm -".to_string();
        app.cursor_position = 4;
        update_suggestions(&mut app);

        assert!(app.show_suggestions);
        assert!(!app.current_suggestions.is_empty());
        // Should suggest -rf, -r, -f
        assert!(
            app.current_suggestions
                .iter()
                .any(|(full, _)| full.starts_with("rm -rf")),
            "rm - should suggest -rf, got: {:?}",
            app.current_suggestions
        );
        assert!(
            app.current_suggestions
                .iter()
                .any(|(full, _)| full.starts_with("rm -r")),
            "rm - should suggest -r, got: {:?}",
            app.current_suggestions
        );
    }

    #[test]
    fn test_update_suggestions_rm_rf_files() {
        let _guard = crate::tools::terminal::TEST_CWD_LOCK.lock().unwrap();
        let mut app = App::new();
        app.current_input = "rm -rf ".to_string();
        app.cursor_position = 7;
        update_suggestions(&mut app);

        assert!(app.show_suggestions);
        assert!(!app.current_suggestions.is_empty());
        // Should suggest files and directories in cwd for rm -rf
        assert!(
            app.current_suggestions
                .iter()
                .any(|(full, _)| full.starts_with("rm -rf Cargo") || full.starts_with("rm -rf src")),
            "rm -rf should suggest files/directories, got: {:?}",
            app.current_suggestions
        );
    }

    #[test]
    fn test_update_suggestions_rm_rf_target() {
        let _guard = crate::tools::terminal::TEST_CWD_LOCK.lock().unwrap();
        let mut app = App::new();
        app.current_input = "rm -rf tar".to_string();
        app.cursor_position = 10;
        update_suggestions(&mut app);

        assert!(app.show_suggestions);
        assert!(
            app.current_suggestions
                .iter()
                .any(|(full, _)| full.starts_with("rm -rf target/")),
            "rm -rf tar should suggest target/, got: {:?}",
            app.current_suggestions
        );
    }

    #[test]
    fn test_update_suggestions_cd_only_dirs() {
        let _guard = crate::tools::terminal::TEST_CWD_LOCK.lock().unwrap();
        let mut app = App::new();
        app.current_input = "cd ".to_string();
        app.cursor_position = 3;
        update_suggestions(&mut app);

        assert!(app.show_suggestions);
        // Should suggest src/ or target/
        assert!(
            app.current_suggestions
                .iter()
                .any(|(full, _)| full.starts_with("cd src/")),
            "cd should suggest src/, got: {:?}",
            app.current_suggestions
        );
        // Should NOT suggest Cargo.toml
        assert!(
            !app.current_suggestions
                .iter()
                .any(|(full, _)| full.contains("Cargo.toml")),
            "cd should NOT suggest regular files like Cargo.toml"
        );
    }

    #[test]
    fn test_apply_completion_selection() {
        let _guard = crate::tools::terminal::TEST_CWD_LOCK.lock().unwrap();
        let mut app = App::new();
        app.current_input = "rm -r".to_string();
        app.cursor_position = 5;
        update_suggestions(&mut app);

        // Find -rf in suggestions
        let rf_idx = app
            .current_suggestions
            .iter()
            .position(|(full, _)| full.starts_with("rm -rf"))
            .expect("-rf should be suggested");

        let s = &app.current_suggestions[rf_idx];
        app.current_input = s.0.clone();
        app.cursor_position = app.current_input.len();

        assert_eq!(app.current_input, "rm -rf ");
        assert_eq!(app.cursor_position, 7);
    }
}
