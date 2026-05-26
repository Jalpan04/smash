# Smash

A cross-platform shell written in Rust with an embedded AI that translates natural language into shell commands.

## Features

- **AI translation** - `smash list all python files` -> runs the right command for your OS
- **Cross-platform** - generates PowerShell on Windows, Bash on Linux automatically
- **Persistent history** - up/down arrows across sessions (`~/.smash_history`)
- **Tab completion** - file path completion with Tab
- **Inline hints** - greyed suggestions as you type, accept with right arrow
- **Aliases** - `alias ll=Get-ChildItem -Force`, persists via `~/.smashrc`
- **Env var expansion** - `$HOME`, `$PATH`, `${MY_VAR}` expanded in all commands
- **Background jobs** - `some_command &` returns prompt immediately
- **Built-in commands** - `cd`, `cd -`, `pwd`, `clear`, `history`, `echo`, `export`, `alias`, `unalias`, `exit`
- **Config file** - `~/.smashrc` loaded on startup for aliases and exports

## Quick Start

### Linux (One-Command Install)

To automatically install Smash on Linux (downloads the latest binary, sets up AI model files under `~/.smash/model`, configures `SMASH_MODEL_DIR`, and updates your shell PATH):

```bash
curl -sSL https://raw.githubusercontent.com/Jalpan04/smash/master/install.sh | bash
```

After installation completes, restart your terminal or reload your shell configuration:
```bash
source ~/.bashrc  # or ~/.zshrc depending on your shell
smash
```

### Windows (One-Command Install)

To automatically install Smash on Windows (downloads the latest binary, sets up AI model files under `~/.smash/model`, configures the `SMASH_MODEL_DIR` environment variable, and appends it to your user PATH):

Open **PowerShell** and run:

```powershell
irm https://raw.githubusercontent.com/Jalpan04/smash/master/install.ps1 | iex
```

After installation completes, **restart your PowerShell terminal** to apply the path changes, then simply type:
```powershell
smash
```

### From source

If you prefer to compile from source:

```bash
git clone https://github.com/Jalpan04/smash.git
cd smash
cargo build --release
./target/release/smash
```

### Pre-built binaries

Alternatively, you can manually download pre-built binaries from [Releases](https://github.com/Jalpan04/smash/releases). To use the AI features when downloading manually, you must clone or download the ONNX model files from this repository and set the `SMASH_MODEL_DIR` environment variable to point to their directory.

## Usage

### Regular shell commands

```
smash:D:\projects> git status
smash:D:\projects> cd src
smash:D:\projects\src> pwd
smash:D:\projects\src> cd -       # go back to D:\projects
```

### AI translation

Prefix with `smash` to translate natural language:

```
smash:~> smash list all python files
[windows] AI suggests: Get-ChildItem -Recurse -Filter *.py

smash:~> smash show free disk space
[linux] AI suggests: df -h

smash:~> smash find files modified today
[linux] AI suggests: find . -mtime -1
```

### Aliases

```
alias ll                        # show alias
alias gs=git status             # create alias
alias deploy=cargo build --release && ./target/release/smash
gs                              # runs: git status
unalias gs                      # remove alias
```

### Background jobs

```
smash:~> ping -t google.com &
[background] 1 process(es) running
smash:~>                         # prompt returns immediately
```

### Environment variables

```
smash:~> export MY_DIR=/home/user/projects
smash:~> cd $MY_DIR
smash:~> echo ${MY_DIR}/src
```

### History

```
smash:~> history                 # print numbered history
smash:~>                         # press up/down to navigate
```

## Configuration: ~/.smashrc

Create `~/.smashrc` to set aliases and environment variables on startup:

```bash
# ~/.smashrc - loaded by smash on every startup

# Aliases
alias ll=ls -la
alias gs=git status
alias gc=git commit -m
alias gp=git push origin master
alias py=python3

# Environment variables
export EDITOR=vim
export MY_PROJECTS=$HOME/projects
```

## AI Model

The shell ships with a fine-tuned T5-Small model (~240 MB) stored via Git LFS.

| Metric | Value |
|--------|-------|
| Architecture | T5-Small (60M params) |
| Inference | ONNX Runtime (CPU) |
| ROUGE-1 | 98.51% |
| Dataset | ~100 cross-platform command pairs |
| Latency | ~200ms per query on CPU |

To retrain:

```bash
pip install -r ml/requirements.txt
python ml/train_smash.py --epochs 10
```

## Running Tests

```bash
cargo test --test parser_tests
```

## License

Apache 2.0 - see [LICENSE](LICENSE).
