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

// Execute shell command and return output
pub fn execute_command(input: &str) -> Vec<String> {
    let input = input.trim();
    if input.is_empty() {
        return vec![];
    }

    let mut parts = input.split_whitespace();
    let program = match parts.next() {
        Some(p) => p.strip_prefix('/').unwrap_or(p),
        None => return vec![],
    };
    let args: Vec<&str> = parts.collect();

    // Built-in commands
    match program {
        "exit" | "quit" => vec![],

        "cd" => {
            let target = if args.is_empty() {
                std::env::var("HOME").unwrap_or_else(|_| String::from("/"))
            } else {
                args[0].to_string()
            };
            if let Err(e) = std::env::set_current_dir(&target) {
                return vec![format!("cd: {}: {}", target, e)];
            }
            vec![]
        }

        "clear" => return vec!["__CLEAR__".to_string()],

        "help" => {
            return vec![
                "Available commands:".to_string(),
                "  ask <question>  - Ask AI (future)".to_string(),
                "  do <task>       - Execute task (future)".to_string(),
                "  plan <goal>     - Plan goal (future)".to_string(),
                "  build <project> - Build project (future)".to_string(),
                "  settings        - Open AI settings".to_string(),
                "  cd <dir>        - Change directory".to_string(),
                "  clear           - Clear screen".to_string(),
                "  exit / quit     - Exit shell".to_string(),
            ];
        }

        "settings" => {
            return vec!["__SETTINGS__".to_string()];
        }

        "ask" | "do" | "plan" | "build" => {
            return vec![format!(
                "{}: This feature will be implemented in a future update.",
                program
            )];
        }

        // External commands
        _ => {
            use std::io::BufRead;
            use std::io::BufReader;
            use std::process::{Command, Stdio};

            let child = Command::new(program)
                .args(&args)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn();

            match child {
                Ok(mut child) => {
                    let mut output = Vec::new();

                    // Read stdout
                    if let Some(stdout) = child.stdout.take() {
                        let reader = BufReader::new(stdout);
                        for line in reader.lines() {
                            if let Ok(line) = line {
                                output.push(line);
                            }
                        }
                    }

                    // Read stderr
                    if let Some(stderr) = child.stderr.take() {
                        let reader = BufReader::new(stderr);
                        for line in reader.lines() {
                            if let Ok(line) = line {
                                output.push(line);
                            }
                        }
                    }

                    child.wait().ok();
                    output
                }
                Err(_e) => {
                    vec![format!("{}: command not found", program)]
                }
            }
        }
    }
}
