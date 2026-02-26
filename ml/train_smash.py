"""
train_smash.py - Fine-tune T5-small on a custom cross-platform shell dataset
Produces the `smash.onnx` model for the Rust smash shell.

Requirements:
    pip install torch transformers datasets sentencepiece "optimum[onnxruntime]" evaluate rouge_score pandas
"""

import os
import argparse
import pandas as pd
from datasets import Dataset

MODEL_NAME = "t5-small"
OUTPUT_DIR = "./output/trained"
ONNX_DIR   = "./output/onnx"
MAX_INPUT  = 64
MAX_OUTPUT = 64
BATCH      = 8
LR         = 5e-4

# ---------------------------------------------------------------------------
# Dataset Generation
# ---------------------------------------------------------------------------
def build_custom_dataset():
    """
    Creates a rich, cross-platform dataset of shell commands.
    """
    print("[1/4] Generating custom Smash dataset...")
    pairs = [
        # Basic Navigation & Information
        ("list all files in directory", "ls -la"),
        ("list files", "ls"),
        ("show free disk space", "df -h"),
        ("show disk usage of current directory", "du -sh ."),
        ("show memory usage", "free -h"),
        ("show running processes", "ps aux"),
        ("show current directory", "pwd"),
        
        # File Operations
        ("create a new directory called mydir", "mkdir mydir"),
        ("create nested directories", "mkdir -p a/b/c"),
        ("remove directory and contents", "rm -rf mydir"),
        ("copy file to another location", "cp src.txt dst.txt"),
        ("move file to another location", "mv src.txt dst.txt"),
        ("show file contents", "cat file.txt"),
        ("show first 10 lines", "head -n 10 file.txt"),
        ("show last 10 lines", "tail -n 10 file.txt"),
        ("create an empty file", "touch newfile.txt"),
        ("delete a file", "rm file.txt"),
        ("rename a file", "mv oldname.txt newname.txt"),

        # Search & Filtering
        ("find all python files", "find . -name '*.py'"),
        ("find large files bigger than 100MB", "find . -size +100M"),
        ("count lines in a file", "wc -l file.txt"),
        ("search for pattern in file", "grep 'pattern' file.txt"),
        ("search recursively in all files", "grep -r 'pattern' ."),
        ("find files modified in last 24 hours", "find . -mtime -1"),

        # System & Network
        ("show network interfaces", "ip addr"),
        ("show listening ports", "ss -tlnp"),
        ("kill process by name", "pkill myprocess"),
        ("kill process by pid", "kill 1234"),
        ("show cpu info", "cat /proc/cpuinfo"),
        ("show environment variables", "env"),
        ("show system uptime", "uptime"),
        ("show current date and time", "date"),

        # Archives
        ("compress folder to tar gz", "tar -czf archive.tar.gz folder/"),
        ("extract tar gz file", "tar -xzf archive.tar.gz"),
        ("download a file from the internet", "wget https://example.com/file"),

        # Permissions
        ("show file permissions", "ls -l"),
        ("change file permissions to executable", "chmod +x file.sh"),
        ("change file permissions to 755", "chmod 755 file"),

        # Cross-platform / Windows specific (Smash will handle translation where needed, but good to know)
        ("list folders windows", "dir"),
        ("ipconfig windows", "ipconfig"),
        ("clear screen", "clear"),
        
        # Smash built-ins
        ("exit shell", "exit"),
        ("quit shell", "exit"),
        ("change to home directory", "cd ~"),
        ("go up one directory", "cd .."),
    ]
    
    # Duplicate and augment slightly
    expanded_pairs = []
    for nl, cmd in pairs:
        expanded_pairs.append((nl, cmd))
        expanded_pairs.append((f"please {nl}", cmd))
        expanded_pairs.append((f"can you {nl}", cmd))
        expanded_pairs.append((nl.capitalize(), cmd))

    df    = pd.DataFrame(expanded_pairs, columns=["invocation", "cmd"])
    dset  = Dataset.from_pandas(df)
    split = dset.train_test_split(test_size=0.1, seed=42)
    print(f"  Generated {len(split['train'])} train / {len(split['test'])} test pairs.")
    return split["train"], split["test"]

# ---------------------------------------------------------------------------
# Training Pipeline
# ---------------------------------------------------------------------------
def preprocess(examples, tokenizer):
    inputs = ["smash translate: " + t for t in examples["invocation"]]
    enc    = tokenizer(inputs, max_length=MAX_INPUT, truncation=True, padding="max_length")
    label_enc = tokenizer(
        examples["cmd"], max_length=MAX_OUTPUT, truncation=True, padding="max_length"
    )
    enc["labels"] = [
        [(tok if tok != tokenizer.pad_token_id else -100) for tok in ids]
        for ids in label_enc["input_ids"]
    ]
    return enc

def train(epochs: int = 5):
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

    train_raw, test_raw = build_custom_dataset()

    print("[2/4] Tokenising...")
    tok_fn    = lambda ex: preprocess(ex, tokenizer)
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
        warmup_steps                = 10,
        fp16                        = False,
        no_cuda                     = True, # run on CPU to avoid complex setups, dataset is tiny
        report_to                   = "none",
        logging_steps               = 10,
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
    print(f"Saved to {final_dir}")
    return final_dir

def export_onnx(model_dir: str):
    print(f"[4/4] Exporting to ONNX -> {ONNX_DIR} ...")
    os.makedirs(ONNX_DIR, exist_ok=True)
    ret = os.system(
        f"optimum-cli export onnx --model \"{model_dir}\" --task seq2seq-lm \"{ONNX_DIR}\""
    )
    if ret != 0:
        print("WARNING: ONNX export failed.")

def smoke_test(model_dir: str):
    from transformers import AutoTokenizer, AutoModelForSeq2SeqLM
    tok   = AutoTokenizer.from_pretrained(model_dir, use_fast=False)
    model = AutoModelForSeq2SeqLM.from_pretrained(model_dir)
    tests = [
        "list all files",
        "show free disk space",
        "find python files",
        "delete a file called test.txt"
    ]
    print("\n--- Smoke Test Smash Model ---")
    for t in tests:
        inp = tok("smash translate: " + t, return_tensors="pt")
        out = model.generate(**inp, max_new_tokens=48)
        cmd = tok.decode(out[0], skip_special_tokens=True)
        print(f"  '{t}'  =>  {cmd}")

if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--epochs", type=int,  default=5)
    parser.add_argument("--test",   action="store_true")
    cfg = parser.parse_args()

    final_dir = os.path.join(OUTPUT_DIR, "final")

    if cfg.test:
        smoke_test(final_dir)
    else:
        final_dir = train(cfg.epochs)
        smoke_test(final_dir)
        export_onnx(final_dir)
        print("\nSmash Model Exported! Ready for Rust integration.")
