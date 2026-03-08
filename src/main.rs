use rustyline::{
    completion::Completer, error::ReadlineError, highlight::MatchingBracketHighlighter,
    hint::HistoryHinter, history::FileHistory, validate::MatchingBracketValidator,
    CompletionType, Config, Editor, Helper, Hinter, Validator,
};
use std::env;
use std::process::Command;
use std::borrow::Cow;

// 1. Define a Helper to manage completions, hints, and highlights
#[derive(Helper, Hinter, Validator)]
struct NshHelper {
    highlighter: MatchingBracketHighlighter,
    #[rustyline(Validator)]
    validator: MatchingBracketValidator,
    #[rustyline(Hinter)]
    hinter: HistoryHinter,
}

impl Completer for NshHelper {
    type Candidate = rustyline::completion::Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &rustyline::Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Self::Candidate>)> {
        // Find the start of the word being completed
        let (start, word) = rustyline::completion::extract_word(line, pos, None, |c| c == ' ' || c == '\t');
        let word_lower = word.to_lowercase();
        
        let mut candidates = Vec::new();

        let commands = ["ask", "do", "plan", "build", "settings", "exit", "quit", "cd", "code"];
        // Only suggest commands at the beginning of the line
        if start == 0 {
            for cmd in commands {
                if cmd.starts_with(&word_lower) {
                    candidates.push(rustyline::completion::Pair {
                        display: cmd.to_string(),
                        replacement: cmd.to_string(),
                    });
                }
            }
        }
        
        let (dir_str, file_prefix) = match word.rfind('/') {
            Some(idx) => (&word[..=idx], &word[idx + 1..]),
            None => (".", word),
        };

        let expanded_dir = if dir_str.starts_with('~') {
            if let Ok(home) = std::env::var("HOME") {
                dir_str.replacen('~', &home, 1)
            } else {
                dir_str.to_string()
            }
        } else {
            dir_str.to_string()
        };

        let dir_path = std::path::Path::new(if expanded_dir == "." { "." } else { &expanded_dir });
        
        if let Ok(entries) = std::fs::read_dir(dir_path) {
            let prefix_lower = file_prefix.to_lowercase();
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    if name.to_lowercase().starts_with(&prefix_lower) {
                        let is_dir = entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false);
                        let mut display = name.to_string();
                        if is_dir {
                            display.push('/');
                        }
                        
                        let mut replacement = if dir_str == "." {
                            display.clone()
                        } else {
                            format!("{}{}", dir_str, display)
                        };
                        
                        // Basic space escaping
                        replacement = replacement.replace(" ", "\\ ");
                        
                        candidates.push(rustyline::completion::Pair {
                            display,
                            replacement,
                        });
                    }
                }
            }
        }

        Ok((start, candidates))
    }
}

impl rustyline::highlight::Highlighter for NshHelper {
    fn highlight<'l>(&self, line: &'l str, _pos: usize) -> Cow<'l, str> {
        let mut colored_line = String::new();
        let mut parts = line.splitn(2, ' ');
        
        if let Some(cmd) = parts.next() {
            let color = match cmd {
                // AI commands: Bold Red on Black background
                "ask" | "do" | "code" | "plan" | "build" | "settings" => "\x1b[1;31;40m", 
                // Built-ins: Bold White on Black background
                "exit" | "quit" | "cd" => "\x1b[1;37;40m", 
                // System Binaries: Bold Gray on Black background
                _ => "\x1b[1;90;40m",                      
            };
            colored_line.push_str(&format!("{}{}\x1b[0m", color, cmd));
        }
        
        if let Some(rest) = parts.next() {
            colored_line.push(' ');
            // Rest of the arguments: White on Black background
            colored_line.push_str(&format!("\x1b[37;40m{}\x1b[0m", rest)); 
        }
        
        if colored_line.is_empty() { Cow::Borrowed(line) } else { Cow::Owned(colored_line) }
    }

    fn highlight_hint<'h>(&self, hint: &'h str) -> Cow<'h, str> {
        Cow::Owned(format!("\x1b[90;40m{}\x1b[0m", hint)) // Gray with Black bg
    }

    fn highlight_prompt<'b, 's: 'b, 'p: 'b>(&self, prompt: &'p str, default: bool) -> Cow<'b, str> {
        // Prompt in Bold Red with Black bg
        if default { Cow::Owned(format!("\x1b[1;31;40m{}\x1b[0m", prompt)) } else { Cow::Borrowed(prompt) }
    }

    fn highlight_char(&self, line: &str, pos: usize, kind: rustyline::highlight::CmdKind) -> bool {
        self.highlighter.highlight_char(line, pos, kind)
    }
}

fn main() {
    let config = Config::builder()
        .completion_type(CompletionType::List)
        .build();

    let mut rl: Editor<NshHelper, FileHistory> =
        Editor::with_config(config).expect("Failed to init rust rl");

    let helper = NshHelper {
        highlighter: MatchingBracketHighlighter::new(),
        validator: MatchingBracketValidator::new(),
        hinter: HistoryHinter {},
    };
    rl.set_helper(Some(helper));

    loop {
        let cwd = env::current_dir().unwrap_or_default(); //current working dir
        let last = cwd
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("/"); //showing last / ele
        let prompt = format!("~ {} : ", last);
        let readline = rl.readline(&prompt);

        match readline {
            Ok(line) => {
                let input = line.trim();

                if input.is_empty() {
                    continue;
                }

                let _ = rl.add_history_entry(input);

                let mut parts = input.split_whitespace();
                let program = parts.next().unwrap();
                let args: Vec<&str> = parts.collect();

                match program {
                    "exit" | "quit" => {
                        println!("Quiting nsh!!");
                        break;
                    }
                    "cd" => { // cd won't work with child process
                        let target = if args.is_empty() {
                            env::var("HOME").unwrap_or_else(|_| String::from("/"))
                        } else {
                            args[0].to_string()
                        };
                        if let Err(e) = env::set_current_dir(&target) {
                            eprintln!("nsh: cd: {} - {}", target, e);
                        }
                    }
                    // --- TUI AI HANDLERS GO HERE ---
                    "ask" => {
                        println!("\x1b[1;31;40m🧠 Generating response for:\x1b[0m \x1b[37;40m{}\x1b[0m", args.join(" "));
                        // TODO: Trigger interactive spinner and stream markdown response
                    }
                    "do" => {
                        println!("\x1b[1;31;40m⚙️ Proposing command for:\x1b[0m \x1b[37;40m{}\x1b[0m", args.join(" "));
                        // TODO: Render interactive confirmation prompt (Y/n)
                    }
                    "plan" => {
                        println!("\x1b[1;31;40m📋 Planning for:\x1b[0m \x1b[37;40m{}\x1b[0m", args.join(" "));
                        // TODO: Render interactive confirmation prompt (Y/n)
                    }
                    "build" => {
                        println!("\x1b[1;31;40m💻 Generating code for:\x1b[0m \x1b[37;40m{}\x1b[0m", args.join(" "));
                        // TODO: Render interactive confirmation prompt (Y/n)
                    }
                    "settings" => { //mainly for model selection
                        println!("\x1b[1;31;40m⚙️ Settings for:\x1b[0m \x1b[37;40m{}\x1b[0m", args.join(" "));
                        // TODO: Render interactive confirmation prompt (Y/n)
                    }
                    // --- OS EXECUTION ---
                    _ => {
                        let child = Command::new(program).args(args).spawn();

                        match child {
                            Ok(mut child_process) => {
                                child_process.wait().unwrap();
                            }
                            Err(e) => {
                                eprintln!(
                                    "nsh: command not found or execution error: {} - {}",
                                    program, e
                                );
                            }
                        }
                    }
                }
            }
            Err(ReadlineError::Interrupted) => {
                // Handle Ctrl-C
                println!("^C");
                continue;
            }
            Err(ReadlineError::Eof) => {
                // Handle Ctrl-D
                println!("exit");
                break;
            }
            Err(err) => {
                println!("Error: {:?}", err);
                break;
            }
        }
    }
}
