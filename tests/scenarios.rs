// Comprehensive scenario test suite for nsh
// Tests terminal command parsing, streaming execution, ANSI filtering,
// state management, autocomplete, filesystem tools, DeepSeek DSML tool parsing,
// interactive plan/build workflows, and binary smoke tests.
// Note: All test names, docstrings, and outputs adhere strictly to monochrome styling with zero emojis.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use crossterm::event::{KeyCode, KeyModifiers};

use nsh::ai::agent::{
    extract_balanced_json, parse_tool_calls,
};
use nsh::modules::commands::{
    clean_interactive_input, execute_command, has_unquoted_shell_metachars, is_fullscreen_tui,
    is_interactive_command, parse_command_line, strip_ansi_escapes,
};
use nsh::modules::completions::{
    complete_paths, get_command_flags, parse_dir_path, parse_input_at_cursor,
};
use nsh::modules::keybindings::{execute_action, get_action, Action};
use nsh::modules::state::{App, Entry, EntryType, PlanSession};
use nsh::tools::{
    cat, copy_path, delete_path, edit_file, execute_tool, get_tool_definitions, grep, ls,
    mkdir, move_path, touch, write_file,
};
use nsh::{
    classify_prompt, compute_auth_modal_area, render_auth_modal, secure_wipe_string,
    AskPassServer, AuthPromptType,
};
use ratatui::backend::TestBackend;
use ratatui::Terminal;

/// RAII helper for creating and safely removing isolated temporary directories.
struct TempDirGuard {
    path: PathBuf,
}

impl TempDirGuard {
    fn new(prefix: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let name = format!("nsh_test_{}_{}_{}", prefix, std::process::id(), nanos);
        let path = std::env::temp_dir().join(name);
        fs::create_dir_all(&path).expect("failed to create temp directory");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn subpath(&self, relative: &str) -> PathBuf {
        self.path.join(relative)
    }
}

impl Drop for TempDirGuard {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

// -----------------------------------------------------------------------------
// Scenario 1: Shell Parsing, Quotes, Escapes, Metacharacters, and Prompt Cleaning
// -----------------------------------------------------------------------------

#[test]
fn scenario_1_shell_parsing_quotes_and_escapes() {
    // Empty and whitespace input
    assert!(parse_command_line("").is_empty());
    assert!(parse_command_line("   \t  \n  ").is_empty());

    // Basic tokenization
    assert_eq!(
        parse_command_line("ls -la /tmp"),
        vec!["ls", "-la", "/tmp"]
    );

    // Double quote handling with spaces
    assert_eq!(
        parse_command_line(r#"echo "hello world" "another string""#),
        vec!["echo", "hello world", "another string"]
    );

    // Single quote handling with spaces
    assert_eq!(
        parse_command_line("echo 'hello single' 'world quote'"),
        vec!["echo", "hello single", "world quote"]
    );

    // Mixed quotes and unquoted arguments
    assert_eq!(
        parse_command_line(r#"cmd --flag "double quoted" 'single quoted' unquoted"#),
        vec!["cmd", "--flag", "double quoted", "single quoted", "unquoted"]
    );

    // Escaped spaces without quotes
    assert_eq!(
        parse_command_line(r#"cat my\ file.txt second\ file.txt"#),
        vec!["cat", "my file.txt", "second file.txt"]
    );

    // Escaped quotes inside double quotes
    assert_eq!(
        parse_command_line(r#"echo "quoted \"inner\" words""#),
        vec!["echo", r#"quoted "inner" words"#]
    );

    // Graceful handling of unclosed quotes
    assert_eq!(
        parse_command_line(r#"echo "unclosed string"#),
        vec!["echo", "unclosed string"]
    );
}

#[test]
fn scenario_1_shell_metacharacter_detection() {
    // Unquoted shell metacharacters that require /bin/sh invocation
    assert!(has_unquoted_shell_metachars("cat file.txt | grep error"));
    assert!(has_unquoted_shell_metachars("make && make install"));
    assert!(has_unquoted_shell_metachars("cmd1 || cmd2"));
    assert!(has_unquoted_shell_metachars("cd /tmp; ls"));
    assert!(has_unquoted_shell_metachars("echo output > out.log"));
    assert!(has_unquoted_shell_metachars("echo append >> out.log"));
    assert!(has_unquoted_shell_metachars("sort < in.txt"));
    assert!(has_unquoted_shell_metachars("long_running_job &"));

    // Properly quoted metacharacters should not trigger shell fallback
    assert!(!has_unquoted_shell_metachars("echo 'foo | bar'"));
    assert!(!has_unquoted_shell_metachars(r#"echo "hello && world""#));
    assert!(!has_unquoted_shell_metachars("git commit -m 'feat(auth): add login; fix bug'"));
    assert!(!has_unquoted_shell_metachars("ls -la /home/user"));
    assert!(!has_unquoted_shell_metachars("cargo test --package nsh"));
}

#[test]
fn scenario_1_clean_interactive_input() {
    // Strips explicit interactive prefixes
    assert_eq!(clean_interactive_input("live git push origin main"), "git push origin main");
    assert_eq!(clean_interactive_input("dev npm run dev"), "npm run dev");
    assert_eq!(clean_interactive_input("tui htop"), "htop");
    assert_eq!(clean_interactive_input("term bash"), "bash");
    assert_eq!(clean_interactive_input("interactive python"), "python");

    // Strips prefix when preceded by sudo / doas
    assert_eq!(clean_interactive_input("sudo live apt update"), "sudo apt update");
    assert_eq!(clean_interactive_input("sudo dev vite"), "sudo vite");
    assert_eq!(clean_interactive_input("doas tui htop"), "doas htop");

    // Leaves standard inputs unaffected
    assert_eq!(clean_interactive_input("cargo build --release"), "cargo build --release");
    assert_eq!(clean_interactive_input("git push origin main"), "git push origin main");
    assert_eq!(clean_interactive_input(""), "");
}

// -----------------------------------------------------------------------------
// Scenario 2: Fullscreen vs Streaming Command Classification
// -----------------------------------------------------------------------------

#[test]
fn scenario_2_fullscreen_tui_classification() {
    // Fullscreen interactive editors and tools (must suspend TUI)
    let fullscreen_commands = [
        "nvim",
        "nvim src/main.rs",
        "/usr/bin/nvim -u NONE",
        "vim",
        "vim +10 file.txt",
        "vi",
        "nano",
        "/usr/local/bin/nano config.toml",
        "emacs",
        "helix",
        "hx src/lib.rs",
        "kak",
        "htop",
        "top",
        "btop",
        "glances",
        "nvtop",
        "tmux",
        "tmux attach -t dev",
        "screen",
        "less README.md",
        "more output.log",
        "lazygit",
        "ranger",
        "yazi",
        "tig",
        "zellij",
        "nnn",
    ];

    for cmd in fullscreen_commands {
        assert!(
            is_fullscreen_tui(cmd),
            "Command should be classified as fullscreen TUI: {}",
            cmd
        );
    }

    // Streaming / CLI commands (must NOT suspend TUI, remain live in output window)
    let streaming_commands = [
        "git push",
        "git push origin main",
        "git pull",
        "git status",
        "git clone https://github.com/example/repo",
        "cargo build",
        "cargo build --release",
        "cargo run",
        "cargo test",
        "npm install",
        "npm run build",
        "yarn add react",
        "pnpm dev",
        "vite",
        "vite dev",
        "npx next dev",
        "sudo apt update",
        "sudo apt-get install -y build-essential",
        "sudo dnf update",
        "docker build -t test .",
        "docker run --rm alpine",
        "python train.py",
        "python3 -m unittest discover",
        "ping -c 4 8.8.8.8",
        "curl -fsSL https://example.com",
    ];

    for cmd in streaming_commands {
        assert!(
            !is_fullscreen_tui(cmd),
            "Command should NOT be classified as fullscreen TUI: {}",
            cmd
        );
    }
}

#[test]
fn scenario_2_interactive_command_classification() {
    // is_interactive_command is an alias for is_fullscreen_tui
    assert!(is_interactive_command("ssh user@remote.host"));
    assert!(is_interactive_command("nvim file.txt"));
    assert!(is_interactive_command("htop"));
    assert!(is_interactive_command("tmux"));
    assert!(is_interactive_command("bash"));

    assert!(!is_interactive_command("sudo apt install curl"));
    assert!(!is_interactive_command("ls -la"));
    assert!(!is_interactive_command("cat file.txt"));
    assert!(!is_interactive_command("cargo check"));
}

// -----------------------------------------------------------------------------
// Scenario 3: ANSI Control Code and Escape Sequence Stripping
// -----------------------------------------------------------------------------

#[test]
fn scenario_3_ansi_control_stripping() {
    // Colors and styling (SGR)
    assert_eq!(
        strip_ansi_escapes("\x1b[31;1mError:\x1b[0m Failed to compile"),
        "Error: Failed to compile"
    );
    assert_eq!(
        strip_ansi_escapes("\x1b[38;5;196m256-color text\x1b[0m"),
        "256-color text"
    );
    assert_eq!(
        strip_ansi_escapes("\x1b[38;2;255;100;50mRGB truecolor\x1b[0m"),
        "RGB truecolor"
    );

    // Cursor movement and line clear codes
    assert_eq!(
        strip_ansi_escapes("\x1b[2K\x1b[1GLine cleared and cursor at column 1"),
        "Line cleared and cursor at column 1"
    );
    assert_eq!(
        strip_ansi_escapes("\x1b[2J\x1b[HScreen cleared"),
        "Screen cleared"
    );
    assert_eq!(
        strip_ansi_escapes("\x1b[1A\x1b[2B\x1b[3C\x1b[4DMovement"),
        "Movement"
    );

    // OSC window title sequences
    assert_eq!(
        strip_ansi_escapes("\x1b]0;my-window-title\x07Visible content"),
        "Visible content"
    );
    assert_eq!(
        strip_ansi_escapes("\x1b]2;terminal-title\x1b\\Output text"),
        "Output text"
    );

    // Preserves standard UTF-8 characters, punctuation, tabs, and newlines
    let utf8_sample = "Testing: alpha, beta, gamma; accents: cafe, naive; symbols: ->, <=, !=";
    assert_eq!(strip_ansi_escapes(utf8_sample), utf8_sample);

    let multiline = "Line 1\nLine 2\tTabbed\nLine 3";
    assert_eq!(strip_ansi_escapes(multiline), multiline);
}

// -----------------------------------------------------------------------------
// Scenario 4: Live Progress Bar, In-Place Carriage Return (\r), and Output Finalization
// -----------------------------------------------------------------------------

#[test]
fn scenario_4_live_progress_bar_carriage_return_replacement() {
    let mut app = App::new();

    // Setup initial command entry
    app.add_entry(Entry {
        entry_type: EntryType::Command,
        content: vec!["git push origin main".to_string()],
        cwd: "~/projects/nsh".to_string(),
    });

    // Simulate progress bar updates with carriage returns (\r)
    app.append_live_output("Writing objects:  10% (1/10)");
    assert_eq!(app.entries.len(), 2);
    assert!(app.entries[1].entry_type == EntryType::Output);
    assert_eq!(app.entries[1].content.last().unwrap(), "Writing objects:  10% (1/10)");

    // Next progress increment with \r overwrites in-place instead of creating new line
    app.append_live_output("\rWriting objects:  50% (5/10)");
    assert_eq!(app.entries[1].content.len(), 1);
    assert_eq!(app.entries[1].content.last().unwrap(), "Writing objects:  50% (5/10)");

    // Completion with \r followed by newline and next line
    app.append_live_output("\rWriting objects: 100% (10/10), done.\nTotal 10 (delta 2)\n");
    assert_eq!(
        app.entries[1].content,
        vec![
            "Writing objects: 100% (10/10), done.",
            "Total 10 (delta 2)",
            "" // pending next chunk
        ]
    );

    // Finalize output cleans up trailing empty line
    let exit_status = Command::new("true").status().expect("failed to run true");
    app.finalize_live_output(exit_status);

    assert_eq!(
        app.entries[1].content,
        vec![
            "Writing objects: 100% (10/10), done.",
            "Total 10 (delta 2)",
        ]
    );
}

#[test]
fn scenario_4_live_output_empty_command_cleanup() {
    let mut app = App::new();

    app.add_entry(Entry {
        entry_type: EntryType::Command,
        content: vec!["true".to_string()],
        cwd: "~".to_string(),
    });

    // Initial empty output entry
    app.append_live_output("");
    // Finalize successful command with no output: entry is removed
    let exit_status = Command::new("true").status().expect("failed to run true");
    app.finalize_live_output(exit_status);

    assert_eq!(app.entries.len(), 1);
    assert!(app.entries[0].entry_type == EntryType::Command);
}

// -----------------------------------------------------------------------------
// Scenario 5: Application State, History, Clear, and Sudo Password Security
// -----------------------------------------------------------------------------

#[test]
fn scenario_5_command_history_deduplication_and_filtering() {
    let mut app = App::new();

    // Consecutive duplicate commands should not be duplicated
    app.add_command_history("ls -la");
    app.add_command_history("ls -la");
    assert_eq!(app.command_history.len(), 1);
    assert_eq!(app.command_history[0], "ls -la");

    // New unique command is added
    app.add_command_history("git status");
    assert_eq!(app.command_history.len(), 2);
    assert_eq!(app.command_history[1], "git status");

    // Empty or whitespace-only inputs are ignored
    app.add_command_history("");
    app.add_command_history("   ");
    app.add_command_history("\t\n");
    assert_eq!(app.command_history.len(), 2);
}

#[test]
fn scenario_5_screen_clear_wipes_entries_and_scroll() {
    let mut app = App::new();

    app.add_entry(Entry {
        entry_type: EntryType::System,
        content: vec!["Welcome to nsh".to_string()],
        cwd: "~".to_string(),
    });
    app.add_entry(Entry {
        entry_type: EntryType::Command,
        content: vec!["cargo build".to_string()],
        cwd: "~".to_string(),
    });
    app.add_entry(Entry {
        entry_type: EntryType::Output,
        content: vec!["Finished dev profile in 1.2s".to_string()],
        cwd: "~".to_string(),
    });

    app.scroll_offset = 12;
    assert_eq!(app.entries.len(), 3);

    app.clear();

    assert!(app.entries.is_empty());
    assert_eq!(app.scroll_offset, 0);
    assert_eq!(app.total_lines, 0);
}

#[test]
fn scenario_5_sudo_password_memory_security() {
    let mut app = App::new();

    app.sudo_password = "sensitive_root_password".to_string();
    app.pending_sudo_command = Some("sudo apt update".to_string());
    app.show_sudo_prompt = true;
    app.sudo_error = Some("Authentication failed".to_string());
    app.sudo_prompt_mode = nsh::SudoPromptMode::TuiModal;

    // Clear sudo state must wipe credentials and pending command
    app.clear_sudo_state();

    assert!(app.sudo_password.is_empty());
    assert!(app.pending_sudo_command.is_none());
    assert!(!app.show_sudo_prompt);
    assert!(app.sudo_error.is_none());
}

// -----------------------------------------------------------------------------
// Scenario 6: Autocomplete and @ File Reference Completions
// -----------------------------------------------------------------------------

#[test]
fn scenario_6_dir_path_parsing() {
    assert_eq!(parse_dir_path(""), (String::new(), String::new()));
    assert_eq!(
        parse_dir_path("simple"),
        (String::new(), "simple".to_string())
    );
    assert_eq!(
        parse_dir_path("src/"),
        ("src/".to_string(), String::new())
    );
    assert_eq!(
        parse_dir_path("src/ai/agent"),
        ("src/ai/".to_string(), "agent".to_string())
    );
    assert_eq!(
        parse_dir_path("/etc/nginx/"),
        ("/etc/nginx/".to_string(), String::new())
    );
}

#[test]
fn scenario_6_command_flags_lookup() {
    let git_cmds = get_command_flags("git", None);
    assert!(git_cmds.iter().any(|(cmd, _)| *cmd == "push"));
    assert!(git_cmds.iter().any(|(cmd, _)| *cmd == "pull"));
    assert!(git_cmds.iter().any(|(cmd, _)| *cmd == "commit"));

    let git_commit_flags = get_command_flags("git", Some("commit"));
    assert!(git_commit_flags.iter().any(|(flag, _)| *flag == "-m"));
    assert!(git_commit_flags.iter().any(|(flag, _)| *flag == "--amend"));

    let cargo_cmds = get_command_flags("cargo", None);
    assert!(cargo_cmds.iter().any(|(cmd, _)| *cmd == "build"));
    assert!(cargo_cmds.iter().any(|(cmd, _)| *cmd == "check"));
    assert!(cargo_cmds.iter().any(|(cmd, _)| *cmd == "test"));

    let ls_flags = get_command_flags("ls", None);
    assert!(ls_flags.iter().any(|(flag, _)| *flag == "-l"));
    assert!(ls_flags.iter().any(|(flag, _)| *flag == "-a"));
}

#[test]
fn scenario_6_path_completion_directory_filtering() {
    let guard = TempDirGuard::new("completion");
    let base = guard.path().to_str().unwrap();

    // Create subdirectories and regular files
    fs::create_dir_all(guard.subpath("docs")).unwrap();
    fs::create_dir_all(guard.subpath("src")).unwrap();
    fs::write(guard.subpath("README.md"), "# Test").unwrap();
    fs::write(guard.subpath("config.json"), "{}").unwrap();

    // When directories_only is true (as for cd)
    let dir_results = complete_paths(&format!("{}/", base), "", true);
    let dir_names: Vec<String> = dir_results.into_iter().map(|(_, disp)| disp).collect();

    assert!(dir_names.contains(&"docs/".to_string()));
    assert!(dir_names.contains(&"src/".to_string()));
    assert!(!dir_names.contains(&"README.md".to_string()));
    assert!(!dir_names.contains(&"config.json".to_string()));

    // When directories_only is false
    let all_results = complete_paths(&format!("{}/", base), "", false);
    let all_names: Vec<String> = all_results.into_iter().map(|(_, disp)| disp).collect();

    assert!(all_names.contains(&"docs/".to_string()));
    assert!(all_names.contains(&"src/".to_string()));
    assert!(all_names.contains(&"README.md".to_string()));
    assert!(all_names.contains(&"config.json".to_string()));
}

#[test]
fn scenario_6_cursor_input_parsing() {
    let input = "cargo test --package ";
    let parsed = parse_input_at_cursor(input, input.len());

    assert!(!parsed.is_command_position);
    assert_eq!(parsed.command, "cargo");
    assert_eq!(parsed.subcommand, Some("test"));
    assert_eq!(parsed.line_prefix, "cargo test --package ");
    assert_eq!(parsed.current_token, "");

    let typing_cmd = "g";
    let parsed_cmd = parse_input_at_cursor(typing_cmd, 1);
    assert!(parsed_cmd.is_command_position);
    assert_eq!(parsed_cmd.current_token, "g");
}

// -----------------------------------------------------------------------------
// Scenario 7: Filesystem Tools and Surgical File Modification (edit_file)
// -----------------------------------------------------------------------------

#[test]
fn scenario_7_filesystem_tools_crud() {
    let guard = TempDirGuard::new("fs_crud");
    let test_dir = guard.subpath("workspace");
    let test_dir_str = test_dir.to_str().unwrap();

    // mkdir
    let dir_res = mkdir(test_dir_str);
    assert!(dir_res.is_ok());
    assert!(test_dir.is_dir());

    // touch
    let file_path = test_dir.join("sample.txt");
    let file_str = file_path.to_str().unwrap();
    let touch_res = touch(file_str);
    assert!(touch_res.is_ok());
    assert!(file_path.is_file());

    // write_file
    let write_res = write_file(file_str, "alpha\nbeta\ngamma\n");
    assert!(write_res.is_ok());

    // cat
    let cat_content = cat(file_str).expect("cat failed");
    assert_eq!(cat_content, "alpha\nbeta\ngamma\n");

    // ls
    let entries = ls(Some(test_dir_str)).expect("ls failed");
    assert!(entries.iter().any(|e| e.name == "sample.txt"));

    // grep
    let grep_res = grep("beta", Some(test_dir_str)).expect("grep failed");
    assert!(grep_res.contains("sample.txt"));
    assert!(grep_res.contains("beta"));

    // copy_path
    let copied_path = test_dir.join("copied.txt");
    let copied_str = copied_path.to_str().unwrap();
    let copy_res = copy_path(file_str, copied_str);
    assert!(copy_res.is_ok());
    assert!(copied_path.is_file());

    // move_path
    let moved_path = test_dir.join("moved.txt");
    let moved_str = moved_path.to_str().unwrap();
    let move_res = move_path(copied_str, moved_str);
    assert!(move_res.is_ok());
    assert!(!copied_path.exists());
    assert!(moved_path.is_file());

    // delete_path
    let del_res = delete_path(moved_str);
    assert!(del_res.is_ok());
    assert!(!moved_path.exists());
}

#[test]
fn scenario_7_edit_file_line_range_replacement() {
    let guard = TempDirGuard::new("edit_range");
    let target = guard.subpath("code.rs");
    let target_str = target.to_str().unwrap();

    let initial = "line 1\nline 2\nline 3\nline 4\nline 5\n";
    fs::write(&target, initial).unwrap();

    // Replace lines 2 through 4 (inclusive)
    let res = edit_file(
        target_str,
        Some(2),
        Some(4),
        "replaced line 2 to 4",
        None,
        None,
    );
    assert!(res.is_ok());

    let updated = fs::read_to_string(&target).unwrap();
    assert_eq!(updated, "line 1\nreplaced line 2 to 4\nline 5\n");
}

#[test]
fn scenario_7_edit_file_target_string_replacement() {
    let guard = TempDirGuard::new("edit_target");
    let target = guard.subpath("config.env");
    let target_str = target.to_str().unwrap();

    let initial = "PORT=8080\nDEBUG=false\nLOG_LEVEL=info\n";
    fs::write(&target, initial).unwrap();

    // Target search and replace
    let res = edit_file(
        target_str,
        None,
        None,
        "DEBUG=true",
        Some("DEBUG=false"),
        None,
    );
    assert!(res.is_ok());

    let updated = fs::read_to_string(&target).unwrap();
    assert_eq!(updated, "PORT=8080\nDEBUG=true\nLOG_LEVEL=info\n");
}

#[test]
fn scenario_7_edit_file_unified_diff_patch() {
    let guard = TempDirGuard::new("edit_diff");
    let target = guard.subpath("patch_target.txt");
    let target_str = target.to_str().unwrap();

    let initial = "first\nsecond\nthird\n";
    fs::write(&target, initial).unwrap();

    let diff = "@@ -1,3 +1,3 @@\n first\n-second\n+second_patched\n third\n";
    let res = edit_file(target_str, None, None, diff, None, None);
    assert!(res.is_ok());

    let updated = fs::read_to_string(&target).unwrap();
    assert_eq!(updated, "first\nsecond_patched\nthird\n");
}

#[test]
fn scenario_7_edit_file_line_deletion() {
    let guard = TempDirGuard::new("edit_del");
    let target = guard.subpath("items.txt");
    let target_str = target.to_str().unwrap();

    let initial = "item 1\nitem 2\nitem 3\n";
    fs::write(&target, initial).unwrap();

    // Deletion: empty replacement
    let res = edit_file(target_str, Some(2), Some(2), "", None, None);
    assert!(res.is_ok());

    let updated = fs::read_to_string(&target).unwrap();
    assert_eq!(updated, "item 1\nitem 3\n");
}

#[test]
fn scenario_7_edit_file_error_conditions() {
    let guard = TempDirGuard::new("edit_err");
    let target = guard.subpath("valid.txt");
    let target_str = target.to_str().unwrap();
    fs::write(&target, "content\n").unwrap();

    // Non-existent file
    let err1 = edit_file("non_existent_file_xyz.txt", Some(1), Some(1), "new", None, None);
    assert!(err1.is_err());

    // Target string not present
    let err2 = edit_file(target_str, None, None, "new", Some("missing_target_phrase"), None);
    assert!(err2.is_err());

    // Out of range line indices
    let err3 = edit_file(target_str, Some(100), Some(200), "new", None, None);
    assert!(err3.is_err());
}

// -----------------------------------------------------------------------------
// Scenario 8: Agent Tool Call Parsing & DeepSeek DSML Robustness
// -----------------------------------------------------------------------------

#[test]
fn scenario_8_tool_calling_standard_json_format() {
    let response = r#"TOOL: write_file
ARGS: {"path": "src/app.rs", "content": "pub fn run() {}"}"#;

    let calls = parse_tool_calls(response);
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, "write_file");
    assert_eq!(calls[0].1["path"], "src/app.rs");
    assert_eq!(calls[0].1["content"], "pub fn run() {}");
}

#[test]
fn scenario_8_tool_calling_block_format_for_write_and_edit() {
    // Block write_file format
    let write_text = "TOOL: write_file\nPATH: index.html\nCONTENT:\n<!DOCTYPE html>\n<html></html>";
    let calls1 = parse_tool_calls(write_text);
    assert_eq!(calls1.len(), 1);
    assert_eq!(calls1[0].0, "write_file");
    assert_eq!(calls1[0].1["path"], "index.html");
    assert_eq!(calls1[0].1["content"], "<!DOCTYPE html>\n<html></html>");

    // Block edit_file format
    let edit_text = "TOOL: edit_file\nPATH: styles.css\nSTART_LINE: 10\nEND_LINE: 12\nREPLACEMENT:\ncolor: #ffffff;";
    let calls2 = parse_tool_calls(edit_text);
    assert_eq!(calls2.len(), 1);
    assert_eq!(calls2[0].0, "edit_file");
    assert_eq!(calls2[0].1["path"], "styles.css");
    assert_eq!(calls2[0].1["start_line"], 10);
    assert_eq!(calls2[0].1["end_line"], 12);
    assert_eq!(calls2[0].1["replacement"], "color: #ffffff;");
}

#[test]
fn scenario_8_tool_calling_xml_and_special_tags() {
    let xml_response = "<tool_call>\n{\"name\": \"cat\", \"arguments\": {\"path\": \"Cargo.toml\"}}\n</tool_call>";
    let calls = parse_tool_calls(xml_response);
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, "cat");
    assert_eq!(calls[0].1["path"], "Cargo.toml");
}

#[test]
fn scenario_8_deepseek_dsml_tool_calls_ascii_and_fullwidth_pipe() {
    // DeepSeek DSML with ASCII pipe (|)
    let dsml_ascii = r#"<|DSML|tool_calls>
<|DSML|invoke name="ls">
<|DSML|parameter name="path" string="true">/home/user/project</|DSML|parameter>
</|DSML|invoke>
</|DSML|tool_calls>"#;

    let calls_ascii = parse_tool_calls(dsml_ascii);
    assert_eq!(calls_ascii.len(), 1);
    assert_eq!(calls_ascii[0].0, "ls");
    assert_eq!(calls_ascii[0].1["path"], "/home/user/project");

    // DeepSeek DSML with fullwidth Unicode pipe (｜, \u{FF5C})
    let dsml_fullwidth = "<｜DSML｜tool_calls>\n<｜DSML｜invoke name=\"mkdir\">\n<｜DSML｜parameter name=\"path\" string=\"true\">/tmp/test_dir</｜DSML｜parameter>\n</｜DSML｜invoke>\n</｜DSML｜tool_calls>";

    let calls_fullwidth = parse_tool_calls(dsml_fullwidth);
    assert_eq!(calls_fullwidth.len(), 1);
    assert_eq!(calls_fullwidth[0].0, "mkdir");
    assert_eq!(calls_fullwidth[0].1["path"], "/tmp/test_dir");
}

#[test]
fn scenario_8_deepseek_dsml_multi_param_edit_file() {
    let text = r#"I will now update the button in index.html.
<｜DSML｜tool_calls>
<｜DSML｜invoke name="edit_file">
<｜DSML｜parameter name="path" string="true">index.html</｜DSML｜parameter>
<｜DSML｜parameter name="start_line">15</｜DSML｜parameter>
<｜DSML｜parameter name="end_line">18</｜DSML｜parameter>
<｜DSML｜parameter name="replacement" string="true"><button class="btn">Click</button></｜DSML｜parameter>
</｜DSML｜invoke>
</｜DSML｜tool_calls>
Proceeding with file modification."#;

    let calls = parse_tool_calls(text);
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, "edit_file");
    assert_eq!(calls[0].1["path"], "index.html");
    assert_eq!(calls[0].1["start_line"], 15);
    assert_eq!(calls[0].1["end_line"], 18);
    assert_eq!(calls[0].1["replacement"], "<button class=\"btn\">Click</button>");
}

#[test]
fn scenario_8_extract_balanced_json() {
    let messy = "Here is some introductory text {\"key\": \"value\", \"nested\": {\"flag\": true}} and trailing info.";
    let parsed = extract_balanced_json(messy).expect("json extraction failed");
    assert_eq!(parsed["key"], "value");
    assert_eq!(parsed["nested"]["flag"], true);
}

// -----------------------------------------------------------------------------
// Scenario 9: Interactive Plan & Build Agent Lifecycle
// -----------------------------------------------------------------------------

#[test]
fn scenario_9_interactive_plan_and_build_workflow() {
    let mut app = App::new();
    assert!(app.active_plan_session.is_none());

    let goal = "Build a lightweight web server with metrics endpoint".to_string();
    let initial_plan = "# Plan (Iteration 1)\n- Setup axum router\n- Add /metrics handler".to_string();

    // Step 1: Initialize session (Iteration 1)
    app.active_plan_session = Some(PlanSession {
        goal: goal.clone(),
        current_plan: initial_plan.clone(),
        iteration: 1,
    });
    assert_eq!(app.active_plan_session.as_ref().unwrap().iteration, 1);

    // Step 2: User suggests change -> refine to Iteration 2
    let suggestion_1 = "include prometheus client format";
    let mut session = app.active_plan_session.take().unwrap();
    session.iteration += 1;

    let refine_prompt_1 = format!(
        "User's original goal: {}\n\nCurrent Plan:\n{}\n\nUser Feedback / Suggestion:\n{}\n\nUpdate and refine the implementation plan incorporating the user's feedback. Output the complete updated Markdown plan.",
        session.goal, session.current_plan, suggestion_1
    );
    assert!(refine_prompt_1.contains(&goal));
    assert!(refine_prompt_1.contains(suggestion_1));

    session.current_plan = format!("{}\n- Format output with prometheus encoder", session.current_plan);
    app.active_plan_session = Some(session);
    assert_eq!(app.active_plan_session.as_ref().unwrap().iteration, 2);

    // Step 3: Second refinement -> Iteration 3
    let suggestion_2 = "add healthcheck endpoint at /healthz";
    let mut session = app.active_plan_session.take().unwrap();
    session.iteration += 1;
    session.current_plan = format!("{}\n- Add /healthz route", session.current_plan);
    let refine_prompt_2 = format!(
        "User's original goal: {}\n\nCurrent Plan:\n{}\n\nUser Feedback / Suggestion:\n{}\n\nUpdate and refine the implementation plan incorporating the user's feedback. Output the complete updated Markdown plan.",
        session.goal, session.current_plan, suggestion_2
    );
    assert!(refine_prompt_2.contains(suggestion_2));
    app.active_plan_session = Some(session);
    assert_eq!(app.active_plan_session.as_ref().unwrap().iteration, 3);

    // Step 4: User approves -> triggers build transition and clears plan session
    let session = app.active_plan_session.clone().unwrap();
    let approval_input = "approve";
    let is_approved = approval_input.eq_ignore_ascii_case("approve")
        || approval_input.eq_ignore_ascii_case("yes")
        || approval_input.eq_ignore_ascii_case("y")
        || approval_input.eq_ignore_ascii_case("/approve");
    assert!(is_approved);

    app.clear_plan_session();
    assert!(app.active_plan_session.is_none());

    let build_prompt = format!(
        "Execute the following approved plan step-by-step to achieve the goal.\n\nApproved Plan:\n{}\n\nGoal: {}",
        session.current_plan, session.goal
    );
    assert!(build_prompt.contains(&goal));
    assert!(build_prompt.contains("/healthz"));
    assert!(build_prompt.contains("prometheus"));

    // Step 5: User denies -> clears plan session cleanly
    app.active_plan_session = Some(PlanSession {
        goal: "Temporary discarded plan".to_string(),
        current_plan: "# Discarded".to_string(),
        iteration: 1,
    });
    let deny_input = "deny";
    let is_denied = deny_input.eq_ignore_ascii_case("deny")
        || deny_input.eq_ignore_ascii_case("no")
        || deny_input.eq_ignore_ascii_case("n")
        || deny_input.eq_ignore_ascii_case("cancel")
        || deny_input.eq_ignore_ascii_case("/deny");
    assert!(is_denied);

    app.clear_plan_session();
    assert!(app.active_plan_session.is_none());
}

// -----------------------------------------------------------------------------
// Scenario 10: Installed Binary Smoke Test
// -----------------------------------------------------------------------------

#[test]
fn scenario_10_installed_binary_smoke_test() {
    let standard_bin = PathBuf::from("/home/bashar-khan/.cargo/bin/nsh");
    let bin_path = if standard_bin.exists() {
        standard_bin
    } else {
        PathBuf::from(env!("CARGO_BIN_EXE_nsh"))
    };

    assert!(
        bin_path.exists(),
        "nsh binary should exist at {:?}",
        bin_path
    );
    let metadata = fs::metadata(&bin_path).expect("failed to inspect binary metadata");
    assert!(metadata.is_file(), "nsh target should be a regular file");

    let mode = metadata.permissions().mode();
    assert!(
        mode & 0o111 != 0,
        "nsh binary must have executable permissions (mode: {:o})",
        mode
    );

    // Spawn process to verify it links and starts cleanly without missing libraries
    let mut child = Command::new(&bin_path)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("failed to spawn nsh binary");

    assert!(child.id() > 0, "child process must have a valid pid");

    // Give process a brief moment to initialize then terminate cleanly
    std::thread::sleep(std::time::Duration::from_millis(100));
    let _ = child.kill();
    let wait_res = child.wait();
    assert!(wait_res.is_ok(), "process termination wait succeeded");
}

// -----------------------------------------------------------------------------
// Scenario 11: Keybindings and Action Dispatch
// -----------------------------------------------------------------------------

#[test]
fn scenario_11_keybinding_action_resolution() {
    assert!(get_action(KeyCode::Enter, KeyModifiers::NONE) == Action::Execute);
    assert!(get_action(KeyCode::Tab, KeyModifiers::NONE) == Action::Complete);
    assert!(get_action(KeyCode::Up, KeyModifiers::NONE) == Action::HistoryUp);
    assert!(get_action(KeyCode::Down, KeyModifiers::NONE) == Action::HistoryDown);
    assert!(get_action(KeyCode::Left, KeyModifiers::CONTROL) == Action::MoveWordLeft);
    assert!(get_action(KeyCode::Right, KeyModifiers::CONTROL) == Action::MoveWordRight);
    assert!(get_action(KeyCode::Char('w'), KeyModifiers::CONTROL) == Action::DeleteWord);
    assert!(get_action(KeyCode::Char('u'), KeyModifiers::CONTROL) == Action::DeleteToLineStart);
    assert!(get_action(KeyCode::Char('k'), KeyModifiers::CONTROL) == Action::DeleteToLineEnd);
    assert!(get_action(KeyCode::Char('y'), KeyModifiers::CONTROL) == Action::Yank);
    assert!(get_action(KeyCode::Char('l'), KeyModifiers::CONTROL) == Action::ClearScreen);
    assert!(get_action(KeyCode::Char('c'), KeyModifiers::CONTROL) == Action::Interrupt);
    assert!(get_action(KeyCode::Char('d'), KeyModifiers::CONTROL) == Action::Eof);
}

#[test]
fn scenario_11_text_editing_and_kill_ring() {
    let mut app = App::new();

    // Type "hello world test"
    for c in "hello world test".chars() {
        execute_action(&mut app, Action::InsertChar(c));
    }
    assert_eq!(app.current_input, "hello world test");
    assert_eq!(app.cursor_position, 16);

    // Move word left -> cursor should be at "test" (index 12)
    execute_action(&mut app, Action::MoveWordLeft);
    assert_eq!(app.cursor_position, 12);

    // Move to end
    execute_action(&mut app, Action::MoveLineEnd);
    assert_eq!(app.cursor_position, 16);

    // Delete word -> removes "test" and stores in kill ring
    execute_action(&mut app, Action::DeleteWord);
    assert_eq!(app.current_input, "hello world ");
    assert_eq!(app.cursor_position, 12);
    assert_eq!(app.kill_ring.last().map(|s| s.as_str()), Some("test"));

    // Yank -> pastes "test" back
    execute_action(&mut app, Action::Yank);
    assert_eq!(app.current_input, "hello world test");
    assert_eq!(app.cursor_position, 16);
}

// -----------------------------------------------------------------------------
// Scenario 12: Built-in Command and Async Tool Execution Integration
// -----------------------------------------------------------------------------

#[tokio::test]
async fn scenario_12_execute_command_and_tool_integration() {
    // Builtin help command
    let help_output = execute_command("help");
    assert!(!help_output.is_empty());
    assert!(help_output[0].contains("Available commands:"));

    // Builtin clear command marker
    let clear_output = execute_command("clear");
    assert_eq!(clear_output, vec!["__CLEAR__".to_string()]);

    // External echo command via execute_command
    let echo_output = execute_command("echo test_nsh_scenario_12");
    assert!(echo_output.iter().any(|l| l == "test_nsh_scenario_12"));

    // execute_tool integration
    let tool_defs = get_tool_definitions();
    assert!(tool_defs.iter().any(|d| d.name == "cat"));
    assert!(tool_defs.iter().any(|d| d.name == "edit_file"));
    assert!(tool_defs.iter().any(|d| d.name == "exec_cmd"));

    // execute_tool exec_cmd
    let exec_res = execute_tool("exec_cmd", serde_json::json!({ "cmd": "echo tool_pipeline_ok" })).await;
    assert!(exec_res.is_ok());
    let res_val = exec_res.unwrap();
    assert!(res_val["output"].as_str().unwrap().contains("tool_pipeline_ok"));
}

// -----------------------------------------------------------------------------
// Scenario 13: History Modal Workflow (Commands, Navigation, Input Entry)
// -----------------------------------------------------------------------------

#[test]
fn scenario_13_history_modal_workflow() {
    // 1. Built-in command triggers
    let res_bare = execute_command("history");
    assert_eq!(res_bare, vec!["__HISTORY__".to_string()]);

    let res_slash = execute_command("/history");
    assert_eq!(res_slash, vec!["__HISTORY__".to_string()]);

    // 2. State & modal lifecycle
    let mut app = App::new();
    assert!(!app.show_history_modal);

    // Populate chronological command history
    app.command_history = vec![
        "ls -la".to_string(),
        "cargo check".to_string(),
        "git status".to_string(),
    ];

    // Open history modal
    app.open_history_modal();
    assert!(app.show_history_modal);
    // Selected should default to the most recent command ("git status")
    assert_eq!(app.history_modal_selected, 2);
    // Crucial requirement: Selecting enters into current_input
    assert_eq!(app.current_input, "git status");

    // Navigate Up with arrow key
    app.history_modal_select_up();
    assert_eq!(app.history_modal_selected, 1);
    assert_eq!(app.current_input, "cargo check");

    // Navigate Up again
    app.history_modal_select_up();
    assert_eq!(app.history_modal_selected, 0);
    assert_eq!(app.current_input, "ls -la");

    // Navigate Down with arrow key
    app.history_modal_select_down();
    assert_eq!(app.history_modal_selected, 1);
    assert_eq!(app.current_input, "cargo check");

    // Test filter capability
    app.history_modal_filter = "git".to_string();
    let filtered = app.filtered_history_commands();
    assert_eq!(filtered, vec!["git status".to_string()]);

    // Confirm selection (e.g. Enter key)
    app.history_modal_selected = 0;
    app.history_modal_confirm();
    assert!(!app.show_history_modal);
    assert_eq!(app.current_input, "git status");
}

// -----------------------------------------------------------------------------
// Scenario 14: Universal Authentication and SSH Password/Passphrase Workflow
// -----------------------------------------------------------------------------

#[test]
fn scenario_14_prompt_classification_and_masking() {
    // SSH VPS password prompt
    let (pt, title, desc, label, masked) = classify_prompt("root@192.168.1.100's password: ");
    assert_eq!(pt, AuthPromptType::SshPassword);
    assert_eq!(title, "SSH Authentication");
    assert_eq!(desc, "Host: root@192.168.1.100");
    assert_eq!(label, "Password:");
    assert!(masked);

    // SSH key passphrase prompt
    let (pt, title, desc, label, masked) =
        classify_prompt("Enter passphrase for key '/home/user/.ssh/id_ed25519': ");
    assert_eq!(pt, AuthPromptType::SshKeyPassphrase);
    assert_eq!(title, "SSH Key Passphrase");
    assert_eq!(desc, "Key: /home/user/.ssh/id_ed25519");
    assert_eq!(label, "Passphrase:");
    assert!(masked);

    // Device / authenticator PIN prompt
    let (pt, title, desc, label, masked) = classify_prompt("Enter PIN for authenticator: ");
    assert_eq!(pt, AuthPromptType::DevicePin);
    assert_eq!(title, "Device Security PIN");
    assert_eq!(desc, "Enter PIN for authenticator:");
    assert_eq!(label, "PIN:");
    assert!(masked);

    // Host key verification (unmasked text!)
    let (pt, title, desc, label, masked) = classify_prompt(
        "The authenticity of host 'github.com (140.82.121.4)' can't be established. Are you sure you want to continue connecting (yes/no/[fingerprint])? ",
    );
    assert_eq!(pt, AuthPromptType::HostVerification);
    assert_eq!(title, "Host Key Verification");
    assert!(desc.contains("github.com"));
    assert_eq!(label, "Continue (yes/no):");
    assert!(!masked);

    // Git username (unmasked) and password (masked)
    let (pt_u, title_u, _desc_u, label_u, masked_u) =
        classify_prompt("Username for 'https://github.com': ");
    assert_eq!(pt_u, AuthPromptType::GitCredentials);
    assert_eq!(title_u, "Git Authentication");
    assert_eq!(label_u, "Username:");
    assert!(!masked_u);

    let (pt_p, title_p, _desc_p, label_p, masked_p) =
        classify_prompt("Password for 'https://alice@github.com': ");
    assert_eq!(pt_p, AuthPromptType::GitCredentials);
    assert_eq!(title_p, "Git Authentication");
    assert_eq!(label_p, "Password:");
    assert!(masked_p);

    // Sudo password prompt
    let (pt, title, desc, label, masked) = classify_prompt("[sudo] password for devops: ");
    assert_eq!(pt, AuthPromptType::SudoPassword);
    assert_eq!(title, "Authentication Required");
    assert_eq!(desc, "User: devops");
    assert_eq!(label, "Password:");
    assert!(masked);
}

#[test]
fn scenario_14_auth_modal_state_and_line_editing() {
    let mut app = App::new();
    assert!(!app.auth_modal.is_active);

    // Open modal
    app.open_auth_modal(
        AuthPromptType::SshPassword,
        "SSH Authentication",
        "user@vps.internal",
        "Password:",
        true,
        None,
    );
    assert!(app.auth_modal.is_active);
    assert_eq!(app.auth_modal.title, "SSH Authentication");
    assert_eq!(app.auth_modal.input_value, "");
    assert_eq!(app.auth_modal.cursor_pos, 0);

    // Typing characters
    for ch in "secret123".chars() {
        app.auth_modal_input_char(ch);
    }
    assert_eq!(app.auth_modal.input_value, "secret123");
    assert_eq!(app.auth_modal.cursor_pos, 9);

    // Backspace
    app.auth_modal_backspace();
    assert_eq!(app.auth_modal.input_value, "secret12");
    assert_eq!(app.auth_modal.cursor_pos, 8);

    // Cursor navigation
    app.auth_modal_cursor_left();
    app.auth_modal_cursor_left();
    assert_eq!(app.auth_modal.cursor_pos, 6);

    // Mid-line insert
    app.auth_modal_input_char('X');
    assert_eq!(app.auth_modal.input_value, "secretX12");
    assert_eq!(app.auth_modal.cursor_pos, 7);

    // Delete forward
    app.auth_modal_delete();
    assert_eq!(app.auth_modal.input_value, "secretX2");
    assert_eq!(app.auth_modal.cursor_pos, 7);

    // Home and End
    app.auth_modal_cursor_home();
    assert_eq!(app.auth_modal.cursor_pos, 0);
    app.auth_modal_cursor_end();
    assert_eq!(app.auth_modal.cursor_pos, app.auth_modal.input_value.len());

    // Word delete (Ctrl+W)
    app.auth_modal_delete_word();
    assert_eq!(app.auth_modal.input_value, "");
    assert_eq!(app.auth_modal.cursor_pos, 0);

    // Bracketed paste
    app.auth_modal_paste("p@ssw0rd!_phrase\r\n");
    assert_eq!(app.auth_modal.input_value, "p@ssw0rd!_phrase");

    // Clear line (Ctrl+U)
    app.auth_modal_clear_input();
    assert_eq!(app.auth_modal.input_value, "");

    // Close modal
    app.close_auth_modal();
    assert!(!app.auth_modal.is_active);
}

#[test]
fn scenario_14_auth_modal_rendering_and_layout() {
    // Clamping checks
    let area_small = ratatui::layout::Rect::new(0, 0, 40, 20);
    let modal_small = compute_auth_modal_area(area_small);
    assert_eq!(modal_small.width, 36);

    let area_standard = ratatui::layout::Rect::new(0, 0, 120, 40);
    let modal_standard = compute_auth_modal_area(area_standard);
    assert!(modal_standard.width >= 52 && modal_standard.width <= 86);

    // Ratatui test backend rendering
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = App::new();
    app.open_auth_modal(
        AuthPromptType::SshKeyPassphrase,
        "SSH Key Passphrase",
        "Key: /home/bashar-khan/.ssh/id_ed25519",
        "Passphrase:",
        true,
        None,
    );
    app.auth_modal_paste("mypassword");
    app.auth_modal.error_message = Some("Incorrect passphrase, try again".to_string());

    terminal
        .draw(|f| {
            render_auth_modal(f, &app);
        })
        .unwrap();

    let buffer = terminal.backend().buffer();
    let rendered_text = format!("{:?}", buffer);

    assert!(rendered_text.contains("SSH Key Passphrase"));
    assert!(rendered_text.contains("[Esc Cancel]"));
    assert!(rendered_text.contains("••••••••••"));
    assert!(rendered_text.contains("Incorrect passphrase, try again"));
    assert!(rendered_text.contains("[Enter] Submit"));
}

#[test]
fn scenario_14_ssh_command_classification() {
    // Remote interactive shell is fullscreen TUI (suspends TUI)
    assert!(is_fullscreen_tui("ssh user@vps"));
    assert!(is_fullscreen_tui("ssh -i ~/.ssh/id_rsa root@10.0.0.1"));

    // Remote batch/streaming command execution stays inside nsh output screen (NOT fullscreen)
    assert!(!is_fullscreen_tui("ssh user@vps uptime"));
    assert!(!is_fullscreen_tui("ssh user@vps ls -la /var/log"));
    assert!(!is_fullscreen_tui("ssh -p 2222 root@myserver docker ps"));

    // scp, sftp, ssh-copy-id stay inside nsh output screen (NOT fullscreen)
    assert!(!is_fullscreen_tui("scp index.html user@vps:/var/www"));
    assert!(!is_fullscreen_tui("sftp user@vps"));
    assert!(!is_fullscreen_tui("ssh-copy-id user@vps"));
}

#[test]
fn scenario_14_askpass_ipc_roundtrip_and_zeroization() {
    use std::io::{BufRead, BufReader, Write};
    use std::os::unix::net::UnixStream;

    let (server, rx) = AskPassServer::start().expect("failed to start askpass server");
    let socket_path = server.socket_path.clone();
    assert!(socket_path.exists());

    let client_handle = std::thread::spawn(move || {
        let mut stream = UnixStream::connect(&socket_path).expect("client connect failed");
        stream
            .write_all(b"Enter passphrase for key '/home/user/.ssh/id_rsa': \n")
            .unwrap();
        stream.flush().unwrap();

        let mut reader = BufReader::new(stream);
        let mut response = String::new();
        reader.read_line(&mut response).expect("read response");
        response
    });

    let event = rx
        .recv_timeout(std::time::Duration::from_secs(3))
        .expect("server event received");
    assert_eq!(event.prompt_type, AuthPromptType::SshKeyPassphrase);
    assert_eq!(event.label, "Passphrase:");
    assert!(event.is_masked);

    // Send secret from auth modal response channel
    event
        .response_tx
        .send(Some("super_secret_ssh_pass".to_string()))
        .unwrap();

    let client_received = client_handle.join().expect("client thread join");
    assert_eq!(client_received.trim(), "super_secret_ssh_pass");

    // Test zeroization
    let mut sensitive = "highly_confidential_key".to_string();
    secure_wipe_string(&mut sensitive);
    assert!(sensitive.is_empty());
}
