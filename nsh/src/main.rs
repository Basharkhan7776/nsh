mod gemini;
mod vector_db;
mod web_search;

use clap::{Parser, Subcommand};
use colored::*; 
use dotenvy::dotenv;
use gemini::GeminiClient;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process::Command as ProcessCommand;
use text_splitter::TextSplitter;
use vector_db::VectorStore;
use web_search::GoogleSearchClient;

#[derive(Parser)]
#[command(name = "nsh")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Ingest documents and initialize vector DB
    Init,
    /// Ask a question using RAG + Google Search
    Ask { query: String },
    /// Execute a natural language instruction (Agentic)
    Do { prompt: String },
    /// Remember a fact or context
    Remember { fact: String },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();
    let cli = Cli::parse();

    match cli.command {
        Some(cmd) => process_command(cmd).await?,
        None => run_repl().await?,
    }

    Ok(())
}

async fn process_command(cmd: Commands) -> anyhow::Result<()> {
    let db_path = "data/vectors";
    
    match cmd {
        Commands::Init => {
            println!("{}", "Initializing nsh knowledge base...".blue());
            let gemini = GeminiClient::new()?;
            let store = VectorStore::new(db_path).await?;
            
            let files = vec!["document.txt", "roadmap.md"];
            let splitter = TextSplitter::default().with_trim_chunks(true);

            for file in files {
                if !Path::new(file).exists() {
                    println!("{} {} not found, skipping.", "Warning:".yellow(), file);
                    continue;
                }
                
                let content = fs::read_to_string(file)?;
                let chunks: Vec<_> = splitter.chunks(&content, 1000).map(|s| s.to_string()).collect();
                
                println!("Processing {} chunks from {}...", chunks.len(), file);

                let mut embeddings = Vec::new();
                for chunk in &chunks {
                    let emb = gemini.embed_text(chunk).await?;
                    embeddings.push(emb);
                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                }

                store.add_texts(chunks, embeddings).await?;
            }
            println!("{}", "Initialization complete.".green());
        }
        Commands::Ask { query } => handle_ask(&query, db_path).await?,
        Commands::Do { prompt } => handle_do(&prompt).await?,
        Commands::Remember { fact } => handle_remember(&fact, db_path).await?,
    }
    Ok(())
}

async fn run_repl() -> anyhow::Result<()> {
    println!("{}", "Welcome to nsh (Neuro-Shell) v0.1.0".bold().cyan());
    println!("Type 'help' for commands, or just type standard shell commands.");
    println!("Special prefixes: '?' (Ask), '!' (Do), '+' (Remember)");

    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let db_path = "data/vectors";

    loop {
        print!("{}", "nsh ➜ ".green().bold());
        stdout.flush()?;

        let mut input = String::new();
        if stdin.read_line(&mut input)? == 0 {
            break;
        }

        let input = input.trim();
        if input.is_empty() {
            continue;
        }

        if input == "exit" || input == "quit" {
            break;
        }

        if input.starts_with('?') {
            let query = input[1..].trim();
            if !query.is_empty() {
                if let Err(e) = handle_ask(query, db_path).await {
                    eprintln!("{}", format!("Error: {}", e).red());
                }
            }
        } else if input.starts_with('!') {
            let prompt = input[1..].trim();
            if !prompt.is_empty() {
                if let Err(e) = handle_do(prompt).await {
                    eprintln!("{}", format!("Error: {}", e).red());
                }
            }
        } else if input.starts_with('+') {
            let fact = input[1..].trim();
            if !fact.is_empty() {
                 if let Err(e) = handle_remember(fact, db_path).await {
                     eprintln!("{}", format!("Error: {}", e).red());
                 }
            }
        } else {
            // Standard Shell Execution
            let parts: Vec<&str> = input.split_whitespace().collect();
            if let Some(cmd) = parts.first() {
                let args = &parts[1..];
                let res = ProcessCommand::new(cmd).args(args).status();
                match res {
                    Ok(status) => {
                        if !status.success() {
                            // Don't panic, just print status
                        }
                    },
                    Err(_) => {
                         // Fallback to sh -c for pipes etc, though simple split won't handle that well.
                         // For a real shell experience we'd need full parsing. 
                         // Let's try executing via sh -c as fallback
                         let _ = ProcessCommand::new("sh").arg("-c").arg(input).status();
                    }
                }
            }
        }
    }
    Ok(())
}

async fn handle_ask(query: &str, db_path: &str) -> anyhow::Result<()> {
    println!("{}", "Thinking...".dimmed());
    let gemini = GeminiClient::new()?;
    let store = VectorStore::new(db_path).await?;
    let search_client = GoogleSearchClient::new().ok();

    let query_emb = gemini.embed_text(query).await?;
    let results = store.search(query_emb, 3).await?;
    let context = results.iter().map(|(txt, _)| txt.clone()).collect::<Vec<_>>().join("\n---\n");

    let mut web_context = String::new();
    if let Some(web) = search_client {
        print!("{}", "Searching web... ".dimmed());
        io::stdout().flush()?;
        match web.search(query).await {
            Ok(results) => {
                println!("{}", "Done.".dimmed());
                for res in results.iter().take(3) {
                    web_context.push_str(&format!("Title: {}\nSnippet: {}\nLink: {}\n\n", res.title, res.snippet, res.link));
                }
            }
            Err(e) => println!("{}", format!("Web search failed: {}", e).yellow()),
        }
    }

    let prompt = format!(
        "You are an intelligent assistant for the 'nsh' project (Agentic AI Linux).\nUse the following context to answer the user's question.\nIf the context is insufficient, rely on the web search results.\n\nContext from Documents:\n{}\n\nContext from Web Search:\n{}\n\nUser Question: {}\nAnswer:",
        context, web_context, query
    );

    let answer = gemini.generate(&prompt).await?;
    println!("\n{}\n{}", "Answer:".bold().underline(), answer);
    Ok(())
}

async fn handle_do(prompt: &str) -> anyhow::Result<()> {
    println!("{}", "Analyzing intent...".dimmed());
    let gemini = GeminiClient::new()?;
    
    // 1. Generate Shell Command
    let prompt_template = format!(
        "You are an agentic Linux shell assistant. Convert the following natural language request into a specific, executable Linux command (Bash).\n        Do not explain. Return ONLY the command string.\n        Request: {}", prompt
    );
    let command_str = gemini.generate(&prompt_template).await?;
    let command_str = command_str.trim().trim_matches('`').trim(); // Cleanup

    println!("{} {}", "Generated Command:".blue(), command_str);

    // 2. Smart-Sudo Risk Assessment
    let risk_score = assess_risk(command_str);
    
    if risk_score >= 0.7 {
        println!("{}", "SECURITY WARNING: High-risk command detected (Smart-Sudo)".on_red().white().bold());
        println!("The command '{}' has a calculated risk score of {:.2}", command_str, risk_score);
        println!("This exceeds the safety threshold. Entering SANDBOX mode.");
        
        println!("\n{} (Sandbox Dry-Run)", "Executing...".yellow());
        // In a real implementation, this would use unshare/chroot.
        // For prototype, we verify logic but don't actually run dangerous commands.
        println!("> [SANDBOX] Command '{}' would be executed in isolated environment.", command_str);
        println!("> [SANDBOX] Simulation: Operation completed successfully (Virtual State).");
        println!("> [SANDBOX] No changes applied to host system.");
    } else {
        // Low risk, execute
        print!("Execute? [Y/n] ");
        io::stdout().flush()?;
        let mut confirm = String::new();
        io::stdin().read_line(&mut confirm)?;
        if confirm.trim().eq_ignore_ascii_case("y") || confirm.trim().is_empty() {
             let _ = ProcessCommand::new("sh").arg("-c").arg(command_str).status();
        } else {
            println!("Aborted.");
        }
    }

    Ok(())
}

async fn handle_remember(fact: &str, db_path: &str) -> anyhow::Result<()> {
    println!("{}", "Committing to memory...".dimmed());
    let gemini = GeminiClient::new()?;
    let store = VectorStore::new(db_path).await?;
    
    let emb = gemini.embed_text(fact).await?;
    store.add_texts(vec![fact.to_string()], vec![emb]).await?;
    
    println!("{}", "Fact saved to long-term memory.".green());
    Ok(())
}

fn assess_risk(cmd: &str) -> f32 {
    let risky_keywords = ["rm", "dd", "mkfs", "mv /", "chmod 777", "sudo", ":(){ :|:& };:"];
    let mut risk = 0.1; // Base risk

    for kw in risky_keywords {
        if cmd.contains(kw) {
            risk += 0.8;
        }
    }
    
    // Cap at 1.0
    if risk > 1.0 { 1.0 } else { risk }
}
