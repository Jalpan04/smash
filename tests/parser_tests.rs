use smash_shell::parser::{tokenize, parse_pipeline, Token};

// ── Tokenizer ────────────────────────────────────────────────────────────────

#[test]
fn tokenize_simple() {
    let toks = tokenize("echo hello").unwrap();
    assert_eq!(toks, vec![Token::Word("echo".into()), Token::Word("hello".into())]);
}

#[test]
fn tokenize_pipe() {
    let toks = tokenize("a | b").unwrap();
    assert!(toks.contains(&Token::Pipe));
}

#[test]
fn tokenize_double_pipe_or() {
    let toks = tokenize("a || b").unwrap();
    assert!(toks.contains(&Token::Or));
}

#[test]
fn tokenize_redirect_out() {
    let toks = tokenize("echo x > file.txt").unwrap();
    assert!(toks.contains(&Token::RedirectOut));
}

#[test]
fn tokenize_redirect_out_append() {
    let toks = tokenize("echo x >> file.txt").unwrap();
    assert!(toks.contains(&Token::RedirectOutAppend));
}

#[test]
fn tokenize_redirect_in() {
    let toks = tokenize("cat < file.txt").unwrap();
    assert!(toks.contains(&Token::RedirectIn));
}

#[test]
fn tokenize_quoted_single() {
    let toks = tokenize("echo 'hello world'").unwrap();
    assert_eq!(toks, vec![Token::Word("echo".into()), Token::Word("hello world".into())]);
}

#[test]
fn tokenize_quoted_double() {
    let toks = tokenize(r#"echo "hello world""#).unwrap();
    assert_eq!(toks, vec![Token::Word("echo".into()), Token::Word("hello world".into())]);
}

#[test]
fn tokenize_unclosed_single_quote_is_err() {
    assert!(tokenize("echo 'hello").is_err());
}

#[test]
fn tokenize_unclosed_double_quote_is_err() {
    assert!(tokenize(r#"echo "hello"#).is_err());
}

#[test]
fn tokenize_trailing_backslash_is_err() {
    assert!(tokenize("echo test\\").is_err());
}

#[test]
fn tokenize_empty_string() {
    let toks = tokenize("").unwrap();
    assert!(toks.is_empty());
}

#[test]
fn tokenize_only_whitespace() {
    let toks = tokenize("   ").unwrap();
    assert!(toks.is_empty());
}

#[test]
fn tokenize_semicolon() {
    let toks = tokenize("echo a ; echo b").unwrap();
    assert!(toks.contains(&Token::Semi));
}

// ── Parser ───────────────────────────────────────────────────────────────────

#[test]
fn parse_simple_command() {
    let toks = tokenize("echo hello").unwrap();
    let pipeline = parse_pipeline(&toks).unwrap();
    assert_eq!(pipeline.len(), 1);
    assert_eq!(pipeline[0].args, vec!["echo", "hello"]);
}

#[test]
fn parse_pipe_two_commands() {
    let toks = tokenize("cat file | grep foo").unwrap();
    let pipeline = parse_pipeline(&toks).unwrap();
    assert_eq!(pipeline.len(), 2);
}

#[test]
fn parse_bare_pipe_is_err() {
    let toks = tokenize("|").unwrap();
    assert!(parse_pipeline(&toks).is_err());
}

#[test]
fn parse_trailing_pipe_is_err() {
    let toks = tokenize("echo a |").unwrap();
    assert!(parse_pipeline(&toks).is_err());
}

#[test]
fn parse_double_pipe_is_err() {
    let toks = tokenize("echo a | | echo b").unwrap();
    assert!(parse_pipeline(&toks).is_err());
}

#[test]
fn parse_redirect_out_no_file_is_err() {
    let toks = tokenize("echo a >").unwrap();
    assert!(parse_pipeline(&toks).is_err());
}

#[test]
fn parse_redirect_in_no_file_is_err() {
    let toks = tokenize("<").unwrap();
    assert!(parse_pipeline(&toks).is_err());
}

#[test]
fn parse_redirect_out_with_file() {
    let toks = tokenize("echo hello > out.txt").unwrap();
    let pipeline = parse_pipeline(&toks).unwrap();
    assert_eq!(pipeline[0].output_redirect, Some("out.txt".to_string()));
    assert!(!pipeline[0].output_append);
}

#[test]
fn parse_redirect_out_append_with_file() {
    let toks = tokenize("echo hello >> out.txt").unwrap();
    let pipeline = parse_pipeline(&toks).unwrap();
    assert_eq!(pipeline[0].output_redirect, Some("out.txt".to_string()));
    assert!(pipeline[0].output_append);
}

#[test]
fn parse_redirect_in_with_file() {
    let toks = tokenize("cat < in.txt").unwrap();
    let pipeline = parse_pipeline(&toks).unwrap();
    assert_eq!(pipeline[0].input_redirect, Some("in.txt".to_string()));
}

#[test]
fn parse_empty_tokens_returns_empty_pipeline() {
    let toks = tokenize("").unwrap();
    let pipeline = parse_pipeline(&toks).unwrap();
    assert!(pipeline.is_empty());
}
