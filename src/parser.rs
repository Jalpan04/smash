#[derive(Debug, PartialEq, Clone)]
pub enum Token {
    Word(String),
    Pipe,
    RedirectOut,
    RedirectOutAppend,
    RedirectIn,
    And,    // &&
    Or,     // ||
    Semi,   // ;
}

/// Tokenize an input string respecting quotes and escaping.
pub fn tokenize(input: &str) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();
    let mut chars = input.chars().peekable();
    let mut current_word = String::new();
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut escaping = false;

    while let Some(c) = chars.next() {
        if escaping {
            current_word.push(c);
            escaping = false;
            continue;
        }

        match c {
            '\\' if !in_single_quote => {
                escaping = true;
            }
            '\'' if !in_double_quote => {
                in_single_quote = !in_single_quote;
            }
            '"' if !in_single_quote => {
                in_double_quote = !in_double_quote;
            }
            ' ' | '\t' | '\n' if !in_single_quote && !in_double_quote => {
                if !current_word.is_empty() {
                    tokens.push(Token::Word(current_word.clone()));
                    current_word.clear();
                }
            }
            '|' if !in_single_quote && !in_double_quote => {
                if !current_word.is_empty() {
                    tokens.push(Token::Word(current_word.clone()));
                    current_word.clear();
                }
                if let Some(&'|') = chars.peek() {
                    chars.next();
                    tokens.push(Token::Or);
                } else {
                    tokens.push(Token::Pipe);
                }
            }
            '>' if !in_single_quote && !in_double_quote => {
                if !current_word.is_empty() {
                    tokens.push(Token::Word(current_word.clone()));
                    current_word.clear();
                }
                if let Some(&'>') = chars.peek() {
                    chars.next();
                    tokens.push(Token::RedirectOutAppend);
                } else {
                    tokens.push(Token::RedirectOut);
                }
            }
            '<' if !in_single_quote && !in_double_quote => {
                if !current_word.is_empty() {
                    tokens.push(Token::Word(current_word.clone()));
                    current_word.clear();
                }
                tokens.push(Token::RedirectIn);
            }
            '&' if !in_single_quote && !in_double_quote => {
                if let Some(&'&') = chars.peek() {
                    chars.next();
                    if !current_word.is_empty() {
                        tokens.push(Token::Word(current_word.clone()));
                        current_word.clear();
                    }
                    tokens.push(Token::And);
                } else {
                    current_word.push('&');
                }
            }
            ';' if !in_single_quote && !in_double_quote => {
                if !current_word.is_empty() {
                    tokens.push(Token::Word(current_word.clone()));
                    current_word.clear();
                }
                tokens.push(Token::Semi);
            }
            _ => {
                current_word.push(c);
            }
        }
    }

    if escaping {
        return Err("Unexpected end of input after escape character".to_string());
    }
    if in_single_quote || in_double_quote {
        return Err("Unclosed quote".to_string());
    }

    if !current_word.is_empty() {
        tokens.push(Token::Word(current_word));
    }

    Ok(tokens)
}

#[derive(Debug, PartialEq, Clone)]
pub struct Command {
    pub args: Vec<String>,
    pub input_redirect: Option<String>,
    pub output_redirect: Option<String>,
    pub output_append: bool,
}

impl Command {
    pub fn new() -> Self {
        Command {
            args: Vec::new(),
            input_redirect: None,
            output_redirect: None,
            output_append: false,
        }
    }
}

pub type Pipeline = Vec<Command>;

/// Parse tokens into a list of pipelines (separated by &&, ||, or ;)
/// For simplicity, we currently just return a single pipeline.
/// In a fuller shell, you'd parse AST nodes for logic operators.
pub fn parse_pipeline(tokens: &[Token]) -> Result<Vec<Command>, String> {
    let mut pipeline = Vec::new();
    let mut current_cmd = Command::new();
    let mut i = 0;

    // A very basic parser focusing on pipes and redirects
    while i < tokens.len() {
        match &tokens[i] {
            Token::Word(w) => {
                current_cmd.args.push(w.clone());
            }
            Token::Pipe => {
                if current_cmd.args.is_empty() {
                    return Err("Syntax error: empty command before pipe".to_string());
                }
                pipeline.push(current_cmd);
                current_cmd = Command::new();
            }
            Token::RedirectOut | Token::RedirectOutAppend => {
                let append = matches!(tokens[i], Token::RedirectOutAppend);
                i += 1;
                if i < tokens.len() {
                    if let Token::Word(file) = &tokens[i] {
                        current_cmd.output_redirect = Some(file.clone());
                        current_cmd.output_append = append;
                    } else {
                        return Err("Syntax error after redirection >".to_string());
                    }
                } else {
                    return Err("Syntax error: expected file after redirection".to_string());
                }
            }
            Token::RedirectIn => {
                i += 1;
                if i < tokens.len() {
                    if let Token::Word(file) = &tokens[i] {
                        current_cmd.input_redirect = Some(file.clone());
                    } else {
                        return Err("Syntax error after redirection <".to_string());
                    }
                } else {
                    return Err("Syntax error: expected file after redirection".to_string());
                }
            }
            // For this basic shell, logic ops break the pipeline, we just ignore for now or return err
            Token::And | Token::Or | Token::Semi => {
                // Return what we have. A full parser would handle sequential/conditional execution.
                break;
            }
        }
        i += 1;
    }

    if !current_cmd.args.is_empty() {
        pipeline.push(current_cmd);
    } else if !pipeline.is_empty() && current_cmd.args.is_empty() {
        return Err("Syntax error: empty command after pipe".to_string());
    }

    Ok(pipeline)
}
