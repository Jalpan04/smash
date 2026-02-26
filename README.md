# Smash (Smart Bash)

Smash is a lightweight, efficient, and cross-platform shell built from the ground up in Rust. It integrates an embedded Natural Language processing engine to bridge the gap between human intent and shell commands.

## Overview

Unlike traditional shell wrappers, Smash is a standalone shell implementation designed for speed and native performance on both Windows and Linux. It features a custom lexical parser and direct OS-level process management, combined with an AI translation layer powered by ONNX Runtime.

## Key Features

- Native Rust implementation: High performance with no garbage collection overhead.
- Cross-Platform support: Unified experience across Windows Powershell/CMD and Linux Bash environments.
- AI Translation: Translate natural language queries (e.g., "list all python files") into valid shell commands instantly.
- Embedded Inference: Runs a fine-tuned T5 model locally using ONNX Runtime, requiring no external API calls or internet connection.
- Modern CLI Experience: Integrated line editing with syntax highlighting and history support via Reedline.
- Native Built-ins: Efficient implementation of essential commands like cd, pwd, and exit.

## Technical Architecture

### Core Shell
The shell is written in Rust, leveraging:
- Reedline: For a robust and interactive command-line interface.
- Crossterm: For cross-platform terminal manipulation.
- Custom Parser: Handles complex shell syntax including pipes, redirection, and quoting.

### Machine Learning Engine
- Model Architecture: T5-Small sequence-to-sequence transformer.
- Inference Backend: ORT (ONNX Runtime) for low-latency CPU inference.
- Tokenization: Native Byte-Pair Encoding (BPE) integration via the Hugging Face Tokenizers library.

## Getting Started

### Prerequisites

- Rust Toolchain (v1.93 or later)
- Python 3.8+ (only required for model training)

### Installation

1. Clone the repository:
   ```bash
   git clone https://github.com/Jalpan04/smash.git
   cd smash
   ```

2. Build the project:
   ```bash
   cargo build --release
   ```

3. Run the shell:
   ```bash
   cargo run --release
   ```

## Usage

Smash functions like a standard shell but introduces the `smash` command for AI assistance.

### Standard Commands
```bash
smash:D:\smash> ls -la
smash:D:\smash> cd src
smash:D:\smash\src> pwd
```

### AI Translation
Prefix your query with `smash` to invoke the AI translator:
```bash
smash:D:\smash> smash list all files in directory
SMASH AI SUGGESTS: ls -la

smash:D:\smash> smash find all python files bigger than 1mb
SMASH AI SUGGESTS: find . -name "*.py" -size +1M
```

## Development

The ML pipeline is located in the `ml/` directory. If you wish to retrain or extend the translation capabilities:

1. Install requirements:
   ```bash
   pip install -r ml/requirements.txt
   ```

2. Run the training script:
   ```bash
   python ml/train_smash.py
   ```

3. The script will automatically export the optimized `smash.onnx` models for use by the Rust core.

## License

This project is licensed under the Apache License, Version 2.0 - see the [LICENSE](LICENSE) file for details.
