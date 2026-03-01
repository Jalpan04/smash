use std::env;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use std::process::{Child, Command as ProcessCommand, Stdio};
use crate::parser::Command;

/// Attempt to spawn a command. On Windows, if the binary is not found as a
/// standalone executable, transparently retry it through PowerShell so that
/// cmdlets like `Get-ChildItem`, `Get-PSDrive`, etc. work out of the box.
fn spawn_command(
    args: &[String],
    stdin: Stdio,
    stdout: Stdio,
) -> std::io::Result<Child> {
    let mut cmd = ProcessCommand::new(&args[0]);
    cmd.args(&args[1..]);
    cmd.stdin(stdin).stdout(stdout).stderr(Stdio::inherit());

    #[cfg(target_os = "windows")]
    match cmd.spawn() {
        Ok(child) => return Ok(child),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // Re-run the whole thing through PowerShell
            let ps_cmd = args.join(" ");
            let mut ps = ProcessCommand::new("powershell.exe");
            ps.args(["-NoProfile", "-NonInteractive", "-Command", &ps_cmd]);
            ps.stdin(Stdio::inherit()).stdout(Stdio::inherit()).stderr(Stdio::inherit());
            return ps.spawn();
        }
        Err(e) => return Err(e),
    }

    #[cfg(not(target_os = "windows"))]
    cmd.spawn()
}

pub fn execute_builtin(cmd: &Command) -> Result<bool, String> {
    if cmd.args.is_empty() {
        return Ok(false);
    }

    let bin = &cmd.args[0];
    match bin.as_str() {
        "cd" => {
            let new_dir = if cmd.args.len() > 1 {
                &cmd.args[1]
            } else {
                "~"
            };

            // Handle ~
            let path = if new_dir.starts_with('~') {
                let home = env::var("HOME").or_else(|_| env::var("USERPROFILE")).unwrap_or_else(|_| "/".to_string());
                new_dir.replacen('~', &home, 1)
            } else {
                new_dir.to_string()
            };

            let root = Path::new(&path);
            if let Err(e) = env::set_current_dir(&root) {
                eprintln!("cd: {}: {}", root.display(), e);
            }
            Ok(true)
        }
        "exit" => {
            std::process::exit(0);
        }
        "pwd" => {
            match env::current_dir() {
                Ok(dir) => println!("{}", dir.display()),
                Err(e) => eprintln!("pwd: {}", e),
            }
            Ok(true)
        }
        "export" | "set" => {
            for arg in cmd.args.iter().skip(1) {
                if let Some(idx) = arg.find('=') {
                    let key = &arg[..idx];
                    let val = &arg[idx + 1..];
                    unsafe { env::set_var(key, val); }
                } else {
                    eprintln!("export: invalid argument. Use key=value");
                }
            }
            Ok(true)
        }
        _ => Ok(false),
    }
}

pub fn execute_pipeline(pipeline: Vec<Command>) {
    if pipeline.is_empty() {
        return;
    }

    // Check if it's a single builtin
    if pipeline.len() == 1 {
        match execute_builtin(&pipeline[0]) {
            Ok(true) => return,
            Ok(false) => {}
            Err(e) => {
                eprintln!("{}", e);
                return;
            }
        }
    }

    let mut children: Vec<Child> = Vec::new();
    let mut previous_command_stdout: Option<Stdio> = None;

    let pipe_len = pipeline.len();

    for (i, cmd) in pipeline.iter().enumerate() {
        // Builtins inside a pipe are not natively supported in this basic version
        // We'll just skip builtins if they are piped for now, or you could spawn them in threads.
        if cmd.args.is_empty() {
            continue;
        }

        let stdin = previous_command_stdout.take().unwrap_or_else(Stdio::inherit);
        
        // Handle output routing
        let stdout = if i == pipe_len - 1 {
            // Last command routes to stdout or file
            if let Some(ref outfile) = cmd.output_redirect {
                let file_res = if cmd.output_append {
                    std::fs::OpenOptions::new().create(true).append(true).open(outfile)
                } else {
                    File::create(outfile)
                };
                match file_res {
                    Ok(f) => Stdio::from(f),
                    Err(e) => {
                        eprintln!("smash: {}: {}", outfile, e);
                        break;
                    }
                }
            } else {
                Stdio::inherit()
            }
        } else {
            Stdio::piped()
        };

        // Handle possible input redirection on the FIRST command
        let actual_stdin = if i == 0 && cmd.input_redirect.is_some() {
            if let Some(ref infile) = cmd.input_redirect {
                match File::open(infile) {
                    Ok(f) => Stdio::from(f),
                    Err(e) => {
                        eprintln!("smash: {}: {}", infile, e);
                        break;
                    }
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
                    previous_command_stdout = Some(Stdio::from(child.stdout.take().unwrap()));
                }
                children.push(child);
            }
            Err(e) => {
                eprintln!("smash: {}: {}", cmd.args[0], e);
                break;
            }
        }
    }

    // Wait for all children
    for mut child in children {
        let _ = child.wait();
    }
}
