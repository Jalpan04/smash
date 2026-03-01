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
use std::path::PathBuf;

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
// Environment variable expansion: $VAR  or  ${VAR}
// ---------------------------------------------------------------------------
fn expand_vars(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        if c != '$' {
            result.push(c);
            continue;
        }

        // $
        let braced = chars.peek() == Some(&'{');
        if braced { chars.next(); }

        let mut var_name = String::new();
        loop {
            match chars.peek() {
                Some(&'}') if braced => { chars.next(); break; }
                Some(&ch) if ch.is_alphanumeric() || ch == '_' => {
                    var_name.push(ch);
                    chars.next();
                }
                _ => break,
            }
        }

        if var_name.is_empty() {
            result.push('$');
            if braced { result.push('{'); }
        } else {
            result.push_str(&env::var(&var_name).unwrap_or_default());
        }
    }
    result
}

// ---------------------------------------------------------------------------
// Alias utilities
// ---------------------------------------------------------------------------
fn parse_alias_command(args: &[&str], aliases: &mut HashMap<String, String>) {
    if args.is_empty() {
        if aliases.is_empty() {
            println!("(no aliases defined)");
        } else {
            for (k, v) in aliases.iter() {
                println!("alias {}='{}'", k, v);
            }
        }
        return;
    }
    let joined = args.join(" ");
    if let Some(eq) = joined.find('=') {
        let key = joined[..eq].trim().to_string();
        let val = joined[eq + 1..].trim().trim_matches('\'').trim_matches('"').to_string();
        if key.is_empty() {
            eprintln!("smash: alias: invalid name");
            return;
        }
        aliases.insert(key.clone(), val.clone());
        println!("alias {}='{}'", key, val);
    } else if args.len() >= 2 {
        let key = args[0].to_string();
        let val = args[1..].join(" ");
        aliases.insert(key.clone(), val.clone());
        println!("alias {}='{}'", key, val);
    } else {
        // alias foo  -> show just that alias
        let key = args[0];
        if let Some(val) = aliases.get(key) {
            println!("alias {}='{}'", key, val);
        } else {
            eprintln!("smash: alias: {}: not found", key);
        }
    }
}

fn unalias_command(args: &[&str], aliases: &mut HashMap<String, String>) {
    if args.is_empty() {
        eprintln!("smash: unalias: usage: unalias <name>");
        return;
    }
    for name in args {
        if aliases.remove(*name).is_none() {
            eprintln!("smash: unalias: {}: not found", name);
        }
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
// Load ~/.smashrc  (alias and export directives)
// ---------------------------------------------------------------------------
fn load_smashrc(aliases: &mut HashMap<String, String>) {
    let rc_path = dirs::home_dir()
        .map(|h| h.join(".smashrc"))
        .unwrap_or_else(|| PathBuf::from(".smashrc"));

    let content = match std::fs::read_to_string(&rc_path) {
        Ok(c) => c,
        Err(_) => return, // file doesn't exist, that's fine
    };

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.splitn(2, ' ');
        match parts.next() {
            Some("alias") => {
                let rest = parts.next().unwrap_or("").trim();
                let alias_args: Vec<&str> = rest.split_whitespace().collect();
                // quiet – no println during rc load
                let joined = alias_args.join(" ");
                if let Some(eq) = joined.find('=') {
                    let key = joined[..eq].trim().to_string();
                    let val = joined[eq + 1..].trim().trim_matches('\'').trim_matches('"').to_string();
                    if !key.is_empty() { aliases.insert(key, val); }
                } else if alias_args.len() >= 2 {
                    aliases.insert(alias_args[0].to_string(), alias_args[1..].join(" "));
                }
            }
            Some("export") | Some("set") => {
                let rest = parts.next().unwrap_or("").trim();
                if let Some(idx) = rest.find('=') {
                    let key = rest[..idx].trim();
                    let val = rest[idx + 1..].trim().trim_matches('"').trim_matches('\'');
                    unsafe { env::set_var(key, val); }
                }
            }
            _ => {}
        }
    }
    println!("Loaded ~/.smashrc");
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------
fn main() {
    println!("Smash (Smart Bash) - running on {}", PLATFORM);
    println!("Loading AI model...");

    let model_dir = env::var("SMASH_MODEL_DIR").unwrap_or_else(|_| "output/onnx".to_string());

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
        .unwrap_or_else(|| PathBuf::from(".smash_history"));

    let history = Box::new(
        FileBackedHistory::with_file(5000, history_path.clone())
            .expect("Could not create history file"),
    );

    let completer = Box::new(DefaultCompleter::default());

    let mut line_editor = Reedline::create()
        .with_history(history)
        .with_completer(completer)
        .with_hinter(Box::new(DefaultHinter::default()));

    let prompt = SmashPrompt;
    let mut aliases: HashMap<String, String> = HashMap::new();

    // Built-in aliases (platform-specific)
    #[cfg(target_os = "windows")]
    {
        aliases.insert("ll".to_string(),  "Get-ChildItem -Force".to_string());
        aliases.insert("la".to_string(),  "Get-ChildItem -Force".to_string());
    }
    #[cfg(not(target_os = "windows"))]
    {
        aliases.insert("ll".to_string(),   "ls -la".to_string());
        aliases.insert("la".to_string(),   "ls -la".to_string());
        aliases.insert("grep".to_string(), "grep --color=auto".to_string());
    }

    // Load ~/.smashrc (user config) - sets aliases and env vars
    load_smashrc(&mut aliases);

    // Track previous directory for `cd -`
    let mut prev_dir: Option<PathBuf> = None;

    loop {
        let sig = line_editor.read_line(&prompt);
        match sig {
            Ok(Signal::Success(buffer)) => {
                let raw = buffer.trim();
                if raw.is_empty() {
                    continue;
                }

                // --- Detect background execution (&) ---
                let background = raw.ends_with(" &") || raw == "&";
                let raw = if background {
                    raw.trim_end_matches('&').trim()
                } else {
                    raw
                };

                if raw.is_empty() {
                    continue;
                }

                // --- Built-ins handled before alias expansion ---
                let first = raw.split_whitespace().next().unwrap_or("");

                if first == "alias" {
                    let rest: Vec<&str> = raw.splitn(2, ' ')
                        .nth(1).unwrap_or("").split_whitespace().collect();
                    parse_alias_command(&rest, &mut aliases);
                    continue;
                }

                if first == "unalias" {
                    let rest: Vec<&str> = raw.splitn(2, ' ')
                        .nth(1).unwrap_or("").split_whitespace().collect();
                    unalias_command(&rest, &mut aliases);
                    continue;
                }

                // --- Alias expansion ---
                let expanded = expand_aliases(raw, &aliases);
                let after_alias = expanded.as_ref();

                // --- Environment variable expansion ($VAR) ---
                let input_owned = expand_vars(after_alias);
                let input = input_owned.as_str();

                // --- AI translation ---
                let mut command_to_run = input.to_string();

                if let Some(ref mut smash_ai) = ai {
                    if input.starts_with("smash ") {
                        let nl_query = input.trim_start_matches("smash ").trim();
                        if nl_query.is_empty() {
                            eprintln!("Usage: smash <natural language query>");
                            eprintln!("  e.g. smash list all files");
                            continue;
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
                        // Implicit translation heuristic (multi-word, no special chars)
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

                // --- Safety guard ---
                let command_to_run = command_to_run.trim().to_string();
                if command_to_run.is_empty() {
                    eprintln!("smash: nothing to execute");
                    continue;
                }

                // --- Execute ---
                match parser::tokenize(&command_to_run) {
                    Ok(tokens) => match parser::parse_pipeline(&tokens) {
                        Ok(pipeline) => {
                            executor::execute_pipeline(pipeline, &mut prev_dir, background, &history_path);
                        }
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
