"""
train_smash.py - Fine-tune T5-small on a cross-platform shell dataset.
Produces the `smash.onnx` model for the Rust smash shell.

The model is trained with an OS-aware prefix so it learns to produce
the correct commands for each platform:
  - Input: "smash translate windows: list all files"  => Output: "Get-ChildItem -Force"
  - Input: "smash translate linux: list all files"    => Output: "ls -la"

Requirements:
    pip install torch transformers datasets sentencepiece "optimum[onnxruntime]" evaluate rouge_score pandas

Usage:
    python train_smash.py            # train + export ONNX
    python train_smash.py --epochs 8 # adjust training epochs
    python train_smash.py --test     # quick smoke-test on existing model
"""

import os
import argparse
import pandas as pd
from datasets import Dataset

MODEL_NAME = "t5-small"
OUTPUT_DIR = "./output/trained"
ONNX_DIR   = "./output/onnx"
MAX_INPUT  = 80
MAX_OUTPUT = 64
BATCH      = 8
LR         = 3e-4


# ---------------------------------------------------------------------------
# Dataset Generation
# ---------------------------------------------------------------------------
def build_dataset():
    """
    Creates a rich cross-platform dataset of natural language to shell command pairs.
    Each pair is tagged with the target OS platform so the model learns
    to produce the correct command depending on where it is running.
    """
    print("[1/4] Generating cross-platform Smash dataset...")

    # (natural_language, linux_command, windows_command)
    pairs = [
        # Navigation & Listing
        ("list all files",                             "ls -la",                                        "Get-ChildItem -Force"),
        ("list files",                                 "ls",                                             "Get-ChildItem"),
        ("list files in current directory",            "ls -la",                                        "Get-ChildItem -Force"),
        ("list only directories",                      "ls -d */",                                       "Get-ChildItem -Directory"),
        ("list files sorted by size",                  "ls -lS",                                        "Get-ChildItem | Sort-Object Length -Descending"),
        ("list files sorted by date",                  "ls -lt",                                        "Get-ChildItem | Sort-Object LastWriteTime -Descending"),
        ("show current directory",                     "pwd",                                            "Get-Location"),
        ("print working directory",                    "pwd",                                            "Get-Location"),
        ("go up one directory",                        "cd ..",                                          "Set-Location .."),
        ("change to home directory",                   "cd ~",                                           "Set-Location $HOME"),
        ("clear the screen",                           "clear",                                          "Clear-Host"),

        # File creation (explicit names)
        ("create a file named test",                   "touch test",                                     "New-Item -ItemType File -Name test"),
        ("create a file named test.txt",               "touch test.txt",                                 "New-Item -ItemType File -Name test.txt"),
        ("create a file called hello.py",              "touch hello.py",                                 "New-Item -ItemType File -Name hello.py"),
        ("create an empty file",                       "touch newfile.txt",                              "New-Item -ItemType File -Name newfile.txt"),
        ("make a new file",                            "touch newfile",                                  "New-Item -ItemType File -Name newfile"),

        # Directory creation
        ("create a directory called mydir",            "mkdir mydir",                                    "New-Item -ItemType Directory -Name mydir"),
        ("make a folder called src",                   "mkdir src",                                      "New-Item -ItemType Directory -Name src"),
        ("create nested directories",                  "mkdir -p a/b/c",                                 "New-Item -ItemType Directory -Force -Path a/b/c"),

        # File operations
        ("copy file from src to dst",                  "cp src.txt dst.txt",                             "Copy-Item src.txt dst.txt"),
        ("copy file to backup",                        "cp file.txt file.txt.bak",                       "Copy-Item file.txt file.txt.bak"),
        ("move file from src to dst",                  "mv src.txt dst.txt",                             "Move-Item src.txt dst.txt"),
        ("rename a file",                              "mv old.txt new.txt",                             "Rename-Item old.txt new.txt"),
        ("delete a file",                              "rm file.txt",                                    "Remove-Item file.txt"),
        ("delete file named test.txt",                 "rm test.txt",                                    "Remove-Item test.txt"),
        ("remove directory and its contents",          "rm -rf mydir",                                   "Remove-Item -Recurse -Force mydir"),
        ("show file contents",                         "cat file.txt",                                   "Get-Content file.txt"),
        ("print contents of a file",                   "cat file.txt",                                   "Get-Content file.txt"),
        ("show first 10 lines of a file",              "head -n 10 file.txt",                            "Get-Content file.txt -TotalCount 10"),
        ("show last 10 lines of a file",               "tail -n 10 file.txt",                            "Get-Content file.txt -Tail 10"),
        ("follow a log file",                          "tail -f logfile.txt",                            "Get-Content logfile.txt -Wait"),
        ("count lines in a file",                      "wc -l file.txt",                                 "(Get-Content file.txt).Count"),
        ("count words in a file",                      "wc -w file.txt",                                 "(Get-Content file.txt | Measure-Object -Word).Words"),
        ("write text to a file",                       "echo hello > output.txt",                        "Set-Content output.txt 'hello'"),
        ("append text to a file",                      "echo hello >> output.txt",                       "Add-Content output.txt 'hello'"),

        # Search
        ("find all python files",                      "find . -name '*.py'",                            "Get-ChildItem -Recurse -Filter *.py"),
        ("find all text files",                        "find . -name '*.txt'",                           "Get-ChildItem -Recurse -Filter *.txt"),
        ("find all log files",                         "find . -name '*.log'",                           "Get-ChildItem -Recurse -Filter *.log"),
        ("find files larger than 100MB",               "find . -size +100M",                             "Get-ChildItem -Recurse | Where-Object {$_.Length -gt 100MB}"),
        ("find files larger than 1GB",                 "find . -size +1G",                               "Get-ChildItem -Recurse | Where-Object {$_.Length -gt 1GB}"),
        ("find empty files",                           "find . -empty",                                  "Get-ChildItem -Recurse | Where-Object {$_.Length -eq 0}"),
        ("search for pattern in a file",               "grep 'pattern' file.txt",                        "Select-String -Pattern 'pattern' file.txt"),
        ("search recursively for a pattern",           "grep -r 'pattern' .",                            "Get-ChildItem -Recurse | Select-String 'pattern'"),
        ("find files modified in last day",            "find . -mtime -1",                               "Get-ChildItem -Recurse | Where-Object {$_.LastWriteTime -gt (Get-Date).AddDays(-1)}"),
        ("find files modified today",                  "find . -mtime -1",                               "Get-ChildItem -Recurse | Where-Object {$_.LastWriteTime -gt (Get-Date).Date}"),

        # System & Info
        ("show free disk space",                       "df -h",                                          "Get-PSDrive -PSProvider FileSystem"),
        ("show disk usage of current directory",       "du -sh .",                                       "Get-ChildItem -Recurse | Measure-Object -Property Length -Sum"),
        ("show memory usage",                          "free -h",                                        "Get-CimInstance Win32_OperatingSystem | Select-Object FreePhysicalMemory,TotalVisibleMemorySize"),
        ("show available memory",                      "free -m",                                        "Get-CimInstance Win32_OperatingSystem | Select-Object FreePhysicalMemory"),
        ("show running processes",                     "ps aux",                                         "Get-Process"),
        ("list all processes",                         "ps -ef",                                         "Get-Process"),
        ("show top processes",                         "top",                                            "Get-Process | Sort-Object CPU -Descending | Select-Object -First 10"),
        ("show cpu info",                              "cat /proc/cpuinfo",                              "Get-CimInstance Win32_Processor"),
        ("show system uptime",                         "uptime",                                         "(Get-Date) - (gcim Win32_OperatingSystem).LastBootUpTime"),
        ("show current date and time",                 "date",                                           "Get-Date"),
        ("show environment variables",                 "env",                                            "Get-ChildItem Env:"),
        ("show network interfaces",                    "ip addr",                                        "Get-NetIPAddress"),
        ("show listening ports",                       "ss -tlnp",                                       "Get-NetTCPConnection -State Listen"),
        ("show all network connections",               "ss -anp",                                        "Get-NetTCPConnection"),
        ("kill a process by name",                     "pkill myprocess",                                "Stop-Process -Name myprocess"),
        ("kill a process by id",                       "kill 1234",                                      "Stop-Process -Id 1234"),
        ("show system hostname",                       "hostname",                                       "hostname"),
        ("show os version",                            "uname -a",                                       "Get-ComputerInfo | Select-Object OsName,OsVersion"),

        # Archives & Network
        ("compress folder to archive",                 "tar -czf archive.tar.gz folder/",                "Compress-Archive -Path folder -DestinationPath archive.zip"),
        ("extract archive",                            "tar -xzf archive.tar.gz",                        "Expand-Archive archive.zip -DestinationPath ."),
        ("extract zip file",                           "unzip archive.zip",                              "Expand-Archive archive.zip -DestinationPath ."),
        ("download a file from the internet",          "wget https://example.com/file",                  "Invoke-WebRequest https://example.com/file -OutFile file"),
        ("download file using curl",                   "curl -O https://example.com/file",               "Invoke-WebRequest https://example.com/file -OutFile file"),
        ("check network connectivity",                 "ping google.com",                                "ping google.com"),

        # Permissions & Ownership
        ("make a file executable",                     "chmod +x file.sh",                              "Set-ExecutionPolicy -Scope Process -ExecutionPolicy Bypass"),
        ("change file permissions to 755",             "chmod 755 file",                                 "icacls file /grant Everyone:F"),
        ("show file permissions",                      "ls -l",                                          "Get-Acl file"),

        # Git
        ("show git status",                            "git status",                                     "git status"),
        ("stage all changes",                          "git add .",                                      "git add ."),
        ("commit with a message",                      "git commit -m 'message'",                        "git commit -m 'message'"),
        ("push to origin",                             "git push origin master",                         "git push origin master"),
        ("pull latest changes",                        "git pull",                                       "git pull"),
        ("show git log",                               "git log --oneline",                              "git log --oneline"),
        ("create a new branch",                        "git checkout -b newbranch",                      "git checkout -b newbranch"),
        ("switch branch",                              "git checkout main",                              "git checkout main"),
        ("show git diff",                              "git diff",                                       "git diff"),

        # Text processing
        ("sort lines in a file",                       "sort file.txt",                                  "Get-Content file.txt | Sort-Object"),
        ("remove duplicate lines",                     "sort -u file.txt",                               "Get-Content file.txt | Sort-Object -Unique"),
        ("count occurrences of word",                  "grep -c 'word' file.txt",                        "Select-String -Pattern 'word' file.txt | Measure-Object | Select-Object Count"),
        ("replace text in a file",                     "sed -i 's/old/new/g' file.txt",                  "(Get-Content file.txt) -replace 'old','new' | Set-Content file.txt"),

        # Shell builtins
        ("exit the shell",                             "exit",                                           "exit"),
        ("quit the shell",                             "exit",                                           "exit"),
        ("set an environment variable",                "export MY_VAR=value",                            "$env:MY_VAR = 'value'"),
        ("print a message",                            "echo hello world",                               "Write-Host hello world"),
        ("run a python script",                        "python script.py",                               "python script.py"),
        ("install a python package",                   "pip install requests",                           "pip install requests"),
        ("check python version",                       "python --version",                               "python --version"),
        ("run a shell script",                         "bash script.sh",                                 "powershell -File script.ps1"),
        ("show command history",                       "history",                                        "Get-History"),
    ]


    linux_records = []
    windows_records = []

    augments = [
        lambda nl: nl,
        lambda nl: f"please {nl}",
        lambda nl: f"can you {nl}",
        lambda nl: nl.capitalize(),
        lambda nl: f"how do i {nl}",
        lambda nl: f"i want to {nl}",
    ]

    for nl, linux_cmd, win_cmd in pairs:
        for aug in augments:
            augmented = aug(nl)
            linux_records.append({"invocation": augmented, "platform": "linux", "cmd": linux_cmd})
            windows_records.append({"invocation": augmented, "platform": "windows", "cmd": win_cmd})

    all_records = linux_records + windows_records
    df = pd.DataFrame(all_records)
    dset = Dataset.from_pandas(df)
    split = dset.train_test_split(test_size=0.1, seed=42)

    print(f"  Generated {len(split['train'])} train / {len(split['test'])} test pairs.")
    return split["train"], split["test"]


# ---------------------------------------------------------------------------
# Training Pipeline
# ---------------------------------------------------------------------------
def preprocess(examples, tokenizer):
    # Use OS-aware prefix so the model learns per-platform output
    inputs = [
        f"smash translate {platform}: {invocation}"
        for platform, invocation in zip(examples["platform"], examples["invocation"])
    ]
    enc = tokenizer(inputs, max_length=MAX_INPUT, truncation=True, padding="max_length")
    label_enc = tokenizer(
        examples["cmd"], max_length=MAX_OUTPUT, truncation=True, padding="max_length"
    )
    enc["labels"] = [
        [(tok if tok != tokenizer.pad_token_id else -100) for tok in ids]
        for ids in label_enc["input_ids"]
    ]
    return enc


def train(epochs: int = 8):
    import numpy as np
    from transformers import (
        AutoTokenizer, AutoModelForSeq2SeqLM,
        Seq2SeqTrainer, Seq2SeqTrainingArguments, DataCollatorForSeq2Seq,
    )
    import evaluate

    print(f"[2/4] Loading {MODEL_NAME}...")
    tokenizer = AutoTokenizer.from_pretrained(MODEL_NAME, use_fast=False)
    model     = AutoModelForSeq2SeqLM.from_pretrained(MODEL_NAME)
    vocab_sz  = len(tokenizer)

    train_raw, test_raw = build_dataset()

    print("[2/4] Tokenising...")
    tok_fn    = lambda ex: preprocess(ex, tokenizer)
    remove_cols = [c for c in train_raw.column_names if c != "labels"]
    train_tok = train_raw.map(tok_fn, batched=True, remove_columns=train_raw.column_names)
    test_tok  = test_raw.map(tok_fn,  batched=True, remove_columns=test_raw.column_names)

    collator = DataCollatorForSeq2Seq(tokenizer, model=model, label_pad_token_id=-100)
    rouge    = evaluate.load("rouge")

    def compute_metrics(eval_pred):
        preds, labels = eval_pred
        preds  = np.clip(preds, 0, vocab_sz - 1)
        labels = np.where(labels == -100, tokenizer.pad_token_id, labels)
        decoded_preds  = tokenizer.batch_decode(preds,  skip_special_tokens=True)
        decoded_labels = tokenizer.batch_decode(labels, skip_special_tokens=True)
        result = rouge.compute(predictions=decoded_preds, references=decoded_labels)
        return {k: round(v * 100, 2) for k, v in result.items()}

    args = Seq2SeqTrainingArguments(
        output_dir                  = OUTPUT_DIR,
        num_train_epochs            = epochs,
        per_device_train_batch_size = BATCH,
        per_device_eval_batch_size  = BATCH,
        predict_with_generate       = True,
        eval_strategy               = "epoch",
        save_strategy               = "epoch",
        load_best_model_at_end      = True,
        learning_rate               = LR,
        warmup_steps                = 20,
        fp16                        = False,
        no_cuda                     = True,
        report_to                   = "none",
        logging_steps               = 20,
    )

    trainer = Seq2SeqTrainer(
        model=model, args=args,
        train_dataset=train_tok, eval_dataset=test_tok,
        tokenizer=tokenizer, data_collator=collator,
        compute_metrics=compute_metrics,
    )

    print(f"[3/4] Training {epochs} epoch(s) on CPU...")
    trainer.train()

    final_dir = os.path.join(OUTPUT_DIR, "final")
    trainer.save_model(final_dir)
    tokenizer.save_pretrained(final_dir)
    print(f"  Model saved to {final_dir}")
    return final_dir


def export_onnx(model_dir: str):
    print(f"[4/4] Exporting to ONNX -> {ONNX_DIR} ...")
    os.makedirs(ONNX_DIR, exist_ok=True)
    ret = os.system(
        f'optimum-cli export onnx --model "{model_dir}" --task seq2seq-lm "{ONNX_DIR}"'
    )
    if ret != 0:
        print("WARNING: ONNX export failed. Check that optimum is installed.")


def smoke_test(model_dir: str):
    from transformers import AutoTokenizer, AutoModelForSeq2SeqLM
    tok   = AutoTokenizer.from_pretrained(model_dir, use_fast=False)
    model = AutoModelForSeq2SeqLM.from_pretrained(model_dir)

    tests = [
        ("windows", "list all files"),
        ("linux",   "list all files"),
        ("windows", "show free disk space"),
        ("linux",   "show free disk space"),
        ("windows", "find all python files"),
        ("linux",   "find all python files"),
        ("windows", "delete a file"),
        ("linux",   "delete a file"),
    ]

    print("\n--- Smoke Test Smash Model (Cross-Platform) ---")
    for platform, nl in tests:
        prompt = f"smash translate {platform}: {nl}"
        inp = tok(prompt, return_tensors="pt")
        out = model.generate(**inp, max_new_tokens=48)
        cmd = tok.decode(out[0], skip_special_tokens=True)
        print(f"  [{platform:7}] '{nl}'  =>  {cmd}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--epochs", type=int,  default=8)
    parser.add_argument("--test",   action="store_true")
    cfg = parser.parse_args()

    final_dir = os.path.join(OUTPUT_DIR, "final")

    if cfg.test:
        smoke_test(final_dir)
    else:
        final_dir = train(cfg.epochs)
        smoke_test(final_dir)
        export_onnx(final_dir)
        print("\nSmash Cross-Platform Model exported. Ready for Rust integration.")
