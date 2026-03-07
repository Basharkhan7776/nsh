use rustyline::{
    completion::FilenameCompleter, error::ReadlineError, highlight::MatchingBracketHighlighter,
    hint::HistoryHinter, history::FileHistory, validate::MatchingBracketValidator, Completer,
    CompletionType, Config, Editor, Helper, Hinter, Validator,
};
use std::env;
use std::process::Command;
use std::borrow::Cow;

// 1. Define a Helper to manage completions, hints, and highlights
#[derive(Helper, Completer, Hinter, Validator)]
struct NshHelper {
    #[rustyline(Completer)]
    completer: FilenameCompleter,
    highlighter: MatchingBracketHighlighter,
    #[rustyline(Validator)]
    validator: MatchingBracketValidator,
    #[rustyline(Hinter)]
    hinter: HistoryHinter,
}

impl rustyline::highlight::Highlighter for NshHelper {
    fn highlight<'l>(&self, line: &'l str, _pos: usize) -> Cow<'l, str> {
        let mut colored_line = String::new();
        let mut parts = line.splitn(2, ' ');
        
        if let Some(cmd) = parts.next() {
            let color = match cmd {
                "ask" | "do" | "code" => "\x1b[1;35m", // Bold Magenta for AI
                "exit" | "quit" | "cd" => "\x1b[1;33m", // Bold Yellow for Built-ins
                _ => "\x1b[1;32m",                      // Bold Green for System Binaries
            };
            colored_line.push_str(&format!("{}{}\x1b[0m", color, cmd));
        }
        
        if let Some(rest) = parts.next() {
            colored_line.push(' ');
            colored_line.push_str(rest); 
        }
        
        if colored_line.is_empty() { Cow::Borrowed(line) } else { Cow::Owned(colored_line) }
    }

    fn highlight_hint<'h>(&self, hint: &'h str) -> Cow<'h, str> {
        Cow::Owned(format!("\x1b[90m{}\x1b[0m", hint)) // Bright Black (Gray)
    }

    fn highlight_prompt<'b, 's: 'b, 'p: 'b>(&self, prompt: &'p str, default: bool) -> Cow<'b, str> {
        if default { Cow::Owned(format!("\x1b[1;36m{}\x1b[0m", prompt)) } else { Cow::Borrowed(prompt) }
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
        completer: FilenameCompleter::new(),
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
        let prompt = format!("nsh:{} ~ ", last);
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
                        println!("🧠 Generating response for: {}", args.join(" "));
                        // TODO: Trigger interactive spinner and stream markdown response
                    }
                    "do" => {
                        println!("⚙️ Proposing command for: {}", args.join(" "));
                        // TODO: Render interactive confirmation prompt (Y/n)
                    }
                    "plan" => {
                        println!("📋 Planning for: {}", args.join(" "));
                        // TODO: Render interactive confirmation prompt (Y/n)
                    }
                    "build" => {
                        println!("💻 Generating code for: {}", args.join(" "));
                        // TODO: Render interactive confirmation prompt (Y/n)
                    }
                    "settings" => { //mainly for model selection
                        println!("⚙️ Settings for: {}", args.join(" "));
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
