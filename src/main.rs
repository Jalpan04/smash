mod parser;
mod executor;
mod ai;

use ai::SmashAI;
use reedline::{Reedline, Signal};
use std::env;
use std::borrow::Cow;

pub struct SmashPrompt;
impl reedline::Prompt for SmashPrompt {
    fn render_prompt_left(&self) -> Cow<str> {
        if let Ok(cwd) = env::current_dir() {
            Cow::Owned(format!("\x1b[32msmash\x1b[0m:\x1b[34m{}\x1b[0m> ", cwd.display()))
        } else {
            Cow::Borrowed("smash> ")
        }
    }
    
    fn render_prompt_right(&self) -> Cow<str> {
        Cow::Borrowed("")
    }
    
    fn render_prompt_indicator(&self, _prompt_mode: reedline::PromptEditMode) -> Cow<str> {
        Cow::Borrowed("")
    }
    
    fn render_prompt_multiline_indicator(&self) -> Cow<str> {
        Cow::Borrowed("... ")
    }
    
    fn render_prompt_history_search_indicator(&self, _history_search: reedline::PromptHistorySearch) -> Cow<str> {
        Cow::Borrowed("?")
    }
}

fn main() {
    println!("Welcome to Smash (Smart Bash)!");
    println!("Loading AI model... (this may take a moment)");

    let ml_dir = match env::var("SMASH_MODEL_DIR") {
        Ok(val) => val,
        Err(_) => "ml/output/onnx".to_string(), // Default assuming we run from smash root
    };

    let mut ai = match SmashAI::new(&ml_dir) {
        Ok(ai) => Some(ai),
        Err(e) => {
            eprintln!("Failed to load Smash AI model: {}", e);
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
                    // Try to translate if it looks like natural language
                    let is_natural_language = input.starts_with("smash ") || input.starts_with("please ") || input.starts_with("can you ") || (!input.contains(" -") && !input.contains("/") && !input.contains("\\"));
                    
                    if input.starts_with("smash ") {
                        let nl_query = input.trim_start_matches("smash ").trim();
                        match smash_ai.generate(nl_query) {
                            Ok(translated) => {
                                println!("\x1b[35mSMASH AI SUGGESTS:\x1b[0m {}", translated);
                                command_to_run = translated.clone();
                            }
                            Err(e) => eprintln!("AI Error: {}", e),
                        }
                    } else if is_natural_language && input.split_whitespace().count() > 2 {
                        // implicit translation
                        match smash_ai.generate(input) {
                            Ok(translated) => {
                                // If translation generated something different and looks like a real command
                                if translated != input && !translated.is_empty() {
                                    println!("\x1b[35mSMASH AI TRANSLATED:\x1b[0m {}", translated);
                                    command_to_run = translated.clone();
                                }
                            }
                            Err(_) => {}
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
