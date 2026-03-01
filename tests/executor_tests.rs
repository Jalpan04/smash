use smash_shell::executor::execute_builtin;
use smash_shell::parser::{Command};
use std::path::PathBuf;
use std::env;

fn make_cmd(args: &[&str]) -> Command {
    Command {
        args: args.iter().map(|s| s.to_string()).collect(),
        input_redirect: None,
        output_redirect: None,
        output_append: false,
    }
}

fn dummy_history() -> PathBuf {
    std::env::temp_dir().join("smash_test_history.txt")
}

// ── cd ────────────────────────────────────────────────────────────────────

#[test]
fn cd_no_args_goes_to_home() {
    let mut prev: Option<PathBuf> = None;
    let cmd = make_cmd(&["cd"]);
    let result = execute_builtin(&cmd, &mut prev, &dummy_history());
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), true);
    // Should have moved to home (HOME or USERPROFILE)
    let cwd = env::current_dir().unwrap();
    let home = env::var("HOME")
        .or_else(|_| env::var("USERPROFILE"))
        .unwrap_or_default();
    assert_eq!(cwd.to_string_lossy().to_lowercase(), home.to_lowercase());
}

#[test]
fn cd_to_nonexistent_returns_ok_not_panic() {
    let mut prev: Option<PathBuf> = None;
    let cmd = make_cmd(&["cd", "/totally_nonexistent_dir_xyz_smash_test"]);
    // Should return Ok(true) even on error (prints error, doesn't panic)
    let result = execute_builtin(&cmd, &mut prev, &dummy_history());
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), true);
}

#[test]
fn cd_dash_no_previous_returns_error_message_not_panic() {
    let mut prev: Option<PathBuf> = None;
    let cmd = make_cmd(&["cd", "-"]);
    let result = execute_builtin(&cmd, &mut prev, &dummy_history());
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), true); // handled, didn't crash
}

#[test]
fn cd_sets_prev_dir() {
    let original = env::current_dir().unwrap();
    let mut prev: Option<PathBuf> = None;
    let temp = env::temp_dir();

    let cmd = make_cmd(&["cd", &temp.to_string_lossy()]);
    let _ = execute_builtin(&cmd, &mut prev, &dummy_history());

    assert!(prev.is_some());
    assert_eq!(prev.unwrap().canonicalize().ok(), original.canonicalize().ok());

    // Restore
    let _ = env::set_current_dir(&original);
}

#[test]
fn cd_dash_returns_to_previous() {
    let original = env::current_dir().unwrap();
    let mut prev: Option<PathBuf> = None;
    let temp = env::temp_dir();

    // cd to temp
    let cmd1 = make_cmd(&["cd", &temp.to_string_lossy()]);
    let _ = execute_builtin(&cmd1, &mut prev, &dummy_history());

    assert!(env::current_dir().unwrap().canonicalize().unwrap()
        == temp.canonicalize().unwrap());

    // cd -  -> should go back to original
    let cmd2 = make_cmd(&["cd", "-"]);
    let _ = execute_builtin(&cmd2, &mut prev, &dummy_history());

    assert_eq!(
        env::current_dir().unwrap().canonicalize().ok(),
        original.canonicalize().ok()
    );

    // Restore
    let _ = env::set_current_dir(&original);
}

// ── clear ─────────────────────────────────────────────────────────────────

#[test]
fn clear_returns_true_no_panic() {
    let mut prev: Option<PathBuf> = None;
    let cmd = make_cmd(&["clear"]);
    let result = execute_builtin(&cmd, &mut prev, &dummy_history());
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), true);
}

#[test]
fn cls_returns_true_no_panic() {
    let mut prev: Option<PathBuf> = None;
    let cmd = make_cmd(&["cls"]);
    let result = execute_builtin(&cmd, &mut prev, &dummy_history());
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), true);
}

// ── history ───────────────────────────────────────────────────────────────

#[test]
fn history_with_nonexistent_file_no_panic() {
    let mut prev: Option<PathBuf> = None;
    let cmd = make_cmd(&["history"]);
    let fake = PathBuf::from("/tmp/no_such_smash_history_xyz.txt");
    let result = execute_builtin(&cmd, &mut prev, &fake);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), true);
}

#[test]
fn history_with_real_file_no_panic() {
    let hist = dummy_history();
    std::fs::write(&hist, "echo hello\nls -la\n").unwrap();

    let mut prev: Option<PathBuf> = None;
    let cmd = make_cmd(&["history"]);
    let result = execute_builtin(&cmd, &mut prev, &hist);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), true);

    let _ = std::fs::remove_file(&hist);
}

// ── echo ──────────────────────────────────────────────────────────────────

#[test]
fn echo_no_args_no_panic() {
    let mut prev: Option<PathBuf> = None;
    let cmd = make_cmd(&["echo"]);
    let result = execute_builtin(&cmd, &mut prev, &dummy_history());
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), true);
}

// ── pwd ───────────────────────────────────────────────────────────────────

#[test]
fn pwd_returns_true_no_panic() {
    let mut prev: Option<PathBuf> = None;
    let cmd = make_cmd(&["pwd"]);
    let result = execute_builtin(&cmd, &mut prev, &dummy_history());
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), true);
}

// ── export ────────────────────────────────────────────────────────────────

#[test]
fn export_valid_key_value() {
    let mut prev: Option<PathBuf> = None;
    let cmd = make_cmd(&["export", "SMASH_TEST_VAR=hello123"]);
    let result = execute_builtin(&cmd, &mut prev, &dummy_history());
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), true);
    assert_eq!(std::env::var("SMASH_TEST_VAR").unwrap_or_default(), "hello123");
}

#[test]
fn export_no_equals_no_panic() {
    let mut prev: Option<PathBuf> = None;
    let cmd = make_cmd(&["export", "NOEQUALSIGN"]);
    let result = execute_builtin(&cmd, &mut prev, &dummy_history());
    assert!(result.is_ok()); // prints error but doesn't panic
}

// ── unknown command ───────────────────────────────────────────────────────

#[test]
fn unknown_command_returns_false() {
    let mut prev: Option<PathBuf> = None;
    let cmd = make_cmd(&["totally_not_a_builtin_xyz"]);
    let result = execute_builtin(&cmd, &mut prev, &dummy_history());
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), false); // not a builtin, fall through to spawn
}

#[test]
fn empty_command_returns_false() {
    let mut prev: Option<PathBuf> = None;
    let cmd = make_cmd(&[]);
    let result = execute_builtin(&cmd, &mut prev, &dummy_history());
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), false);
}
