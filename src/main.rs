use rustyline::{
    completion::FilenameCompleter, error::ReadlineError, highlight::MatchingBracketHighlighter,
    hint::HistoryHinter, history::FileHistory, validate::MatchingBracketValidator, Completer,
    CompletionType, Config, Editor, Helper, Highlighter, Hinter, Validator,
};
use std::env;
use std::process::Command;

// 1. Define a Helper to manage completions, hints, and highlights
#[derive(Helper, Completer, Hinter, Highlighter, Validator)]
struct NshHelper {
    #[rustyline(Completer)]
    completer: FilenameCompleter,
    #[rustyline(Highlighter)]
    highlighter: MatchingBracketHighlighter,
    #[rustyline(Validator)]
    validator: MatchingBracketValidator,
    #[rustyline(Hinter)]
    hinter: HistoryHinter,
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
                    "cd" => {
                        let target = if args.is_empty() {
                            env::var("HOME").unwrap_or_else(|_| String::from("/"))
                        } else {
                            args[0].to_string()
                        };
                        if let Err(e) = env::set_current_dir(&target) {
                            eprintln!("nsh: cd: {} - {}", target, e);
                        }
                    }
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
