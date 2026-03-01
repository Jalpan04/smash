use std::env;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::{Child, Command as ProcessCommand, Stdio};
use crate::parser::Command;

// ---------------------------------------------------------------------------
// Internal: spawn a single process
// On Windows everything goes through PowerShell so both real executables
// (git, cargo, python) and PowerShell cmdlets work uniformly.
// On Linux the command is spawned directly.
// ---------------------------------------------------------------------------
fn spawn_command(
    args: &[String],
    stdin: Stdio,
    stdout: Stdio,
) -> std::io::Result<Child> {
    if args.is_empty() {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "empty command"));
    }

    #[cfg(target_os = "windows")]
    {
        let ps_cmd = args.join(" ");
        let mut ps = ProcessCommand::new("powershell.exe");
        ps.args(["-NoProfile", "-NonInteractive", "-Command", &ps_cmd]);
        ps.stdin(stdin).stdout(stdout).stderr(Stdio::inherit());
        return ps.spawn();
    }

    #[cfg(not(target_os = "windows"))]
    {
        let mut cmd = ProcessCommand::new(&args[0]);
        cmd.args(&args[1..]);
        cmd.stdin(stdin).stdout(stdout).stderr(Stdio::inherit());
        cmd.spawn()
    }
}

// ---------------------------------------------------------------------------
// Clear the terminal using crossterm
// ---------------------------------------------------------------------------
fn builtin_clear() {
    use crossterm::{execute, terminal::{Clear, ClearType}, cursor::MoveTo};
    let mut out = std::io::stdout();
    let _ = execute!(out, Clear(ClearType::All), MoveTo(0, 0));
}

// ---------------------------------------------------------------------------
// Print command history from the history file
// ---------------------------------------------------------------------------
fn builtin_history(history_path: &Path) {
    match std::fs::read_to_string(history_path) {
        Ok(content) => {
            for (i, line) in content.lines().enumerate() {
                println!("{:5}  {}", i + 1, line);
            }
        }
        Err(e) => eprintln!("history: {}", e),
    }
}

// ---------------------------------------------------------------------------
// Built-in command handler
// Returns Ok(true)  -> handled as builtin, stop processing
//         Ok(false) -> not a builtin, fall through to spawn
//         Err(e)    -> builtin failed
// ---------------------------------------------------------------------------
pub fn execute_builtin(
    cmd: &Command,
    prev_dir: &mut Option<PathBuf>,
    history_path: &Path,
) -> Result<bool, String> {
    if cmd.args.is_empty() {
        return Ok(false);
    }

    let bin = &cmd.args[0];
    match bin.as_str() {
        // ── cd ───────────────────────────────────────────────────────────
        "cd" => {
            let target = cmd.args.get(1).map(|s| s.as_str()).unwrap_or("~");

            let new_path = if target == "-" {
                // cd -  ->  go to previous directory
                if let Some(pd) = prev_dir.as_ref() {
                    let dest = pd.clone();
                    println!("{}", dest.display());
                    dest
                } else {
                    eprintln!("cd: no previous directory");
                    return Ok(true);
                }
            } else {
                // Resolve ~ to home directory
                let resolved = if target.starts_with('~') {
                    let home = env::var("HOME")
                        .or_else(|_| env::var("USERPROFILE"))
                        .unwrap_or_else(|_| ".".to_string());
                    target.replacen('~', &home, 1)
                } else {
                    target.to_string()
                };
                PathBuf::from(resolved)
            };

            // Save current dir before changing
            let old = env::current_dir().ok();
            if let Err(e) = env::set_current_dir(&new_path) {
                eprintln!("cd: {}: {}", new_path.display(), e);
            } else {
                *prev_dir = old;
            }
            Ok(true)
        }

        // ── clear / cls ──────────────────────────────────────────────────
        "clear" | "cls" => {
            builtin_clear();
            Ok(true)
        }

        // ── history ──────────────────────────────────────────────────────
        "history" => {
            builtin_history(history_path);
            Ok(true)
        }

        // ── exit ─────────────────────────────────────────────────────────
        "exit" | "quit" => {
            std::process::exit(0);
        }

        // ── pwd ──────────────────────────────────────────────────────────
        "pwd" => {
            match env::current_dir() {
                Ok(dir) => println!("{}", dir.display()),
                Err(e)  => eprintln!("pwd: {}", e),
            }
            Ok(true)
        }

        // ── export / set ─────────────────────────────────────────────────
        "export" | "set" => {
            for arg in cmd.args.iter().skip(1) {
                if let Some(idx) = arg.find('=') {
                    let key = &arg[..idx];
                    let val = &arg[idx + 1..];
                    unsafe { env::set_var(key, val); }
                } else {
                    eprintln!("export: invalid argument (expected key=value)");
                }
            }
            Ok(true)
        }

        // ── echo ─────────────────────────────────────────────────────────
        // Native Rust echo so it works on both platforms identically.
        "echo" => {
            let out = cmd.args[1..].join(" ");
            println!("{}", out);
            Ok(true)
        }

        _ => Ok(false),
    }
}

// ---------------------------------------------------------------------------
// Execute a full pipeline (possibly backgrounded)
// ---------------------------------------------------------------------------
pub fn execute_pipeline(
    pipeline: Vec<Command>,
    prev_dir: &mut Option<PathBuf>,
    background: bool,
    history_path: &Path,
) {
    if pipeline.is_empty() {
        return;
    }

    // Single command – try builtins first (they can't be piped easily)
    if pipeline.len() == 1 {
        match execute_builtin(&pipeline[0], prev_dir, history_path) {
            Ok(true)  => return,
            Ok(false) => {}
            Err(e)    => { eprintln!("{}", e); return; }
        }
    }

    let mut children: Vec<Child> = Vec::new();
    let mut previous_command_stdout: Option<Stdio> = None;
    let pipe_len = pipeline.len();

    for (i, cmd) in pipeline.iter().enumerate() {
        if cmd.args.is_empty() {
            continue;
        }

        let stdin = previous_command_stdout.take().unwrap_or_else(Stdio::inherit);

        // Route stdout: pipe to next command, redirect to file, or inherit terminal
        let stdout = if i < pipe_len - 1 {
            Stdio::piped()
        } else if let Some(ref outfile) = cmd.output_redirect {
            let file_res = if cmd.output_append {
                std::fs::OpenOptions::new().create(true).append(true).open(outfile)
            } else {
                File::create(outfile)
            };
            match file_res {
                Ok(f)  => Stdio::from(f),
                Err(e) => { eprintln!("smash: {}: {}", outfile, e); break; }
            }
        } else {
            Stdio::inherit()
        };

        // Input redirection (first command only)
        let actual_stdin = if i == 0 {
            if let Some(ref infile) = cmd.input_redirect {
                match File::open(infile) {
                    Ok(f)  => Stdio::from(f),
                    Err(e) => { eprintln!("smash: {}: {}", infile, e); break; }
                }
            } else {
                stdin
            }
        } else {
            stdin
        };

        match spawn_command(&cmd.args, actual_stdin, stdout) {
            Ok(mut child) => {
                if i < pipe_len - 1 {
                    // Safe: we passed Stdio::piped() for stdout, so child.stdout is Some
                    previous_command_stdout = child.stdout.take().map(Stdio::from);
                }
                children.push(child);
            }
            Err(e) => {
                eprintln!("smash: {}: {}", cmd.args[0], e);
                break;
            }
        }
    }

    if background {
        // Don't wait – return control to prompt immediately
        if !children.is_empty() {
            println!("[background] {} process(es) running", children.len());
        }
    } else {
        for mut child in children {
            let _ = child.wait();
        }
    }
}
