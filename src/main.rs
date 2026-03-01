mod parser;
mod executor;
mod ai;

use ai::SmashAI;
use reedline::{
    DefaultCompleter, FileBackedHistory, Reedline, Signal,
    DefaultHinter,
};
use std::collections::HashMap;
use std::env;
use std::borrow::Cow;

// Detect the platform at compile time
#[cfg(target_os = "windows")]
const PLATFORM: &str = "windows";
#[cfg(not(target_os = "windows"))]
const PLATFORM: &str = "linux";

// ---------------------------------------------------------------------------
// Prompt
// ---------------------------------------------------------------------------
pub struct SmashPrompt;
impl reedline::Prompt for SmashPrompt {
    fn render_prompt_left(&self) -> Cow<'_, str> {
        if let Ok(cwd) = env::current_dir() {
            Cow::Owned(format!(
                "\x1b[32msmash\x1b[0m:\x1b[34m{}\x1b[0m> ",
                cwd.display()
            ))
        } else {
            Cow::Borrowed("smash> ")
        }
    }
    fn render_prompt_right(&self) -> Cow<'_, str> { Cow::Borrowed("") }
    fn render_prompt_indicator(&self, _: reedline::PromptEditMode) -> Cow<'_, str> { Cow::Borrowed("") }
    fn render_prompt_multiline_indicator(&self) -> Cow<'_, str> { Cow::Borrowed("... ") }
    fn render_prompt_history_search_indicator(&self, _: reedline::PromptHistorySearch) -> Cow<'_, str> { Cow::Borrowed("?") }
}

// ---------------------------------------------------------------------------
// Alias store
// ---------------------------------------------------------------------------
fn parse_alias_command(args: &[&str], aliases: &mut HashMap<String, String>) {
    // alias ll="ls -la"  or  alias ll ls -la
    if args.is_empty() {
        for (k, v) in aliases.iter() {
            println!("alias {}='{}'", k, v);
        }
        return;
    }
    let joined = args.join(" ");
    if let Some(eq) = joined.find('=') {
        let key = joined[..eq].trim().to_string();
        let val = joined[eq + 1..].trim().trim_matches('\'').trim_matches('"').to_string();
        println!("alias {} -> '{}'", key, val);
        aliases.insert(key, val);
    } else {
        // alias ll ls -la  (space-separated)
        let key = args[0].to_string();
        let val = args[1..].join(" ");
        println!("alias {} -> '{}'", key, val);
        aliases.insert(key, val);
    }
}

fn expand_aliases<'a>(input: &'a str, aliases: &HashMap<String, String>) -> Cow<'a, str> {
    let first_word = input.split_whitespace().next().unwrap_or("");
    if let Some(expanded) = aliases.get(first_word) {
        let rest = input[first_word.len()..].trim_start();
        if rest.is_empty() {
            Cow::Owned(expanded.clone())
        } else {
            Cow::Owned(format!("{} {}", expanded, rest))
        }
    } else {
        Cow::Borrowed(input)
    }
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------
fn main() {
    println!("Smash (Smart Bash) - running on {}", PLATFORM);
    println!("Loading AI model...");

    let model_dir = match env::var("SMASH_MODEL_DIR") {
        Ok(val) => val,
        Err(_) => "output/onnx".to_string(),
    };

    let mut ai = match SmashAI::new(&model_dir) {
        Ok(ai) => {
            println!("AI model loaded. Type 'smash <query>' for AI translation.");
            Some(ai)
        }
        Err(e) => {
            eprintln!("Warning: AI model not loaded ({}). Passthrough mode.", e);
            None
        }
    };

    // Persistent history stored at ~/.smash_history
    let history_path = dirs::home_dir()
        .map(|h| h.join(".smash_history"))
        .unwrap_or_else(|| std::path::PathBuf::from(".smash_history"));

    let history = Box::new(
        FileBackedHistory::with_file(1000, history_path)
            .expect("Could not create history file"),
    );

    // File path tab completion
    let completer = Box::new(DefaultCompleter::default());

    let mut line_editor = Reedline::create()
        .with_history(history)
        .with_completer(completer)
        .with_hinter(Box::new(DefaultHinter::default()));

    let prompt = SmashPrompt;
    let mut aliases: HashMap<String, String> = HashMap::new();

    // Seed some common cross-platform aliases
    #[cfg(target_os = "windows")]
    {
        aliases.insert("ll".to_string(),    "Get-ChildItem -Force".to_string());
        aliases.insert("la".to_string(),    "Get-ChildItem -Force".to_string());
        aliases.insert("cls".to_string(),   "Clear-Host".to_string());
    }
    #[cfg(not(target_os = "windows"))]
    {
        aliases.insert("ll".to_string(),  "ls -la".to_string());
        aliases.insert("la".to_string(),  "ls -la".to_string());
        aliases.insert("grep".to_string(), "grep --color=auto".to_string());
    }

    loop {
        let sig = line_editor.read_line(&prompt);
        match sig {
            Ok(Signal::Success(buffer)) => {
                let raw = buffer.trim();
                if raw.is_empty() {
                    continue;
                }

                // --- built-in: alias ---
                let mut parts = raw.splitn(2, ' ');
                if parts.next() == Some("alias") {
                    let alias_args: Vec<&str> = parts
                        .next()
                        .unwrap_or("")
                        .split_whitespace()
                        .collect();
                    parse_alias_command(&alias_args, &mut aliases);
                    continue;
                }

                // --- alias expansion ---
                let expanded = expand_aliases(raw, &aliases);
                let input = expanded.as_ref();

                // --- AI translation ---
                let mut command_to_run = input.to_string();

                if let Some(ref mut smash_ai) = ai {
                    if input.starts_with("smash ") {
                        let nl_query = input.trim_start_matches("smash ").trim();
                        if nl_query.is_empty() {
                            eprintln!("Usage: smash <natural language query>");
                            eprintln!("  e.g. smash list all files");
                        } else {
                            match smash_ai.generate(PLATFORM, nl_query) {
                                Ok(translated) => {
                                    println!("\x1b[35m[{}] AI suggests:\x1b[0m {}", PLATFORM, translated);
                                    command_to_run = translated;
                                }
                                Err(e) => eprintln!("AI error: {}", e),
                            }
                        }
                    } else {
                        // Implicit translation heuristic
                        let word_count = input.split_whitespace().count();
                        let looks_like_nl = word_count > 2
                            && !input.contains('/')
                            && !input.contains('\\')
                            && !input.contains('|')
                            && !input.contains('>')
                            && !input.contains('<')
                            && !input.starts_with('-');

                        if looks_like_nl {
                            if let Ok(translated) = smash_ai.generate(PLATFORM, input) {
                                if translated != input && !translated.is_empty() {
                                    println!("\x1b[35m[{}] AI translated:\x1b[0m {}", PLATFORM, translated);
                                    command_to_run = translated;
                                }
                            }
                        }
                    }
                }

                // --- Safety guard: never execute an empty command ---
                let command_to_run = command_to_run.trim().to_string();
                if command_to_run.is_empty() {
                    eprintln!("smash: nothing to execute");
                    continue;
                }

                // --- Execute ---
                match parser::tokenize(&command_to_run) {
                    Ok(tokens) => match parser::parse_pipeline(&tokens) {
                        Ok(pipeline) => executor::execute_pipeline(pipeline),
                        Err(e) => eprintln!("smash: parse error: {}", e),
                    },
                    Err(e) => eprintln!("smash: tokenize error: {}", e),
                }
            }
            Ok(Signal::CtrlD) | Ok(Signal::CtrlC) => {
                println!();
                break;
            }
            _ => {}
        }
    }
}
