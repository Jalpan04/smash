mod parser;
mod executor;
mod ai;

use ai::SmashAI;
use reedline::{Reedline, Signal};
use std::env;
use std::borrow::Cow;

// Detect the platform at compile time
#[cfg(target_os = "windows")]
const PLATFORM: &str = "windows";
#[cfg(not(target_os = "windows"))]
const PLATFORM: &str = "linux";

pub struct SmashPrompt;
impl reedline::Prompt for SmashPrompt {
    fn render_prompt_left(&self) -> Cow<'_, str> {
        if let Ok(cwd) = env::current_dir() {
            Cow::Owned(format!("\x1b[32msmash\x1b[0m:\x1b[34m{}\x1b[0m> ", cwd.display()))
        } else {
            Cow::Borrowed("smash> ")
        }
    }

    fn render_prompt_right(&self) -> Cow<'_, str> {
        Cow::Borrowed("")
    }

    fn render_prompt_indicator(&self, _prompt_mode: reedline::PromptEditMode) -> Cow<'_, str> {
        Cow::Borrowed("")
    }

    fn render_prompt_multiline_indicator(&self) -> Cow<'_, str> {
        Cow::Borrowed("... ")
    }

    fn render_prompt_history_search_indicator(&self, _history_search: reedline::PromptHistorySearch) -> Cow<'_, str> {
        Cow::Borrowed("?")
    }
}

fn main() {
    println!("Smash (Smart Bash) - running on {}", PLATFORM);
    println!("Loading AI model...");

    let ml_dir = match env::var("SMASH_MODEL_DIR") {
        Ok(val) => val,
        Err(_) => "ml/output/onnx".to_string(),
    };

    let mut ai = match SmashAI::new(&ml_dir) {
        Ok(ai) => {
            println!("AI model loaded.");
            Some(ai)
        }
        Err(e) => {
            eprintln!("Warning: AI model not loaded ({}). Shell will run in passthrough mode.", e);
            None
        }
    };

    let mut line_editor = Reedline::create();
    let prompt = SmashPrompt;

    loop {
        let sig = line_editor.read_line(&prompt);
        match sig {
            Ok(Signal::Success(buffer)) => {
                let input = buffer.trim();
                if input.is_empty() {
                    continue;
                }

                let mut command_to_run = input.to_string();

                if let Some(ref mut smash_ai) = ai {
                    if input.starts_with("smash ") {
                        // Explicit AI invocation: "smash <natural language>"
                        let nl_query = input.trim_start_matches("smash ").trim();
                        match smash_ai.generate(PLATFORM, nl_query) {
                            Ok(translated) => {
                                println!("\x1b[35m[{}] AI suggests:\x1b[0m {}", PLATFORM, translated);
                                command_to_run = translated;
                            }
                            Err(e) => eprintln!("AI error: {}", e),
                        }
                    } else {
                        // Implicit translation heuristic:
                        // more than 2 words, no shell operators, no path separators
                        let word_count = input.split_whitespace().count();
                        let looks_like_nl = word_count > 2
                            && !input.contains('/')
                            && !input.contains('\\')
                            && !input.contains('|')
                            && !input.contains('>')
                            && !input.contains('<')
                            && !input.starts_with('-');

                        if looks_like_nl {
                            match smash_ai.generate(PLATFORM, input) {
                                Ok(translated) if translated != input && !translated.is_empty() => {
                                    println!("\x1b[35m[{}] AI translated:\x1b[0m {}", PLATFORM, translated);
                                    command_to_run = translated;
                                }
                                _ => {}
                            }
                        }
                    }
                }

                // Tokenize and execute
                match parser::tokenize(&command_to_run) {
                    Ok(tokens) => match parser::parse_pipeline(&tokens) {
                        Ok(pipeline) => executor::execute_pipeline(pipeline),
                        Err(e) => eprintln!("smash parse error: {}", e),
                    },
                    Err(e) => eprintln!("smash tokenize error: {}", e),
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
