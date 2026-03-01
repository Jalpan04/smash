"""
Automated test harness for Smash shell.
Sends input via stdin to smash.exe, captures output, checks for crashes.
"""
import subprocess
import sys
import time
import os

SCRIPT_DIR  = os.path.dirname(os.path.abspath(__file__))
PROJECT_ROOT = os.path.dirname(SCRIPT_DIR)   # smash root (parent of ml/)
SMASH = os.path.join(PROJECT_ROOT, "target", "debug", "smash.exe")
RUN_DIR = PROJECT_ROOT

PASS = "\033[32mPASS\033[0m"
FAIL = "\033[31mFAIL\033[0m"
WARN = "\033[33mWARN\033[0m"

results = []

def run_test(name, inputs, expect_no_crash=True, expect_in_output=None, expect_not_in_output=None):
    """Run smash with the given inputs, return (passed, output)."""
    inp = "\n".join(inputs) + "\nexit\n"
    try:
        proc = subprocess.run(
            [SMASH],
            input=inp,
            capture_output=True,
            text=True,
            timeout=10,
            cwd=RUN_DIR
        )
        out = proc.stdout + proc.stderr
        crashed = proc.returncode not in (0, None)

        ok = True
        reason = ""

        if expect_no_crash and crashed:
            ok = False
            reason = f"crashed with exit code {proc.returncode}"

        if expect_in_output:
            for s in expect_in_output:
                if s.lower() not in out.lower():
                    ok = False
                    reason += f" missing '{s}' in output"

        if expect_not_in_output:
            for s in expect_not_in_output:
                if s.lower() in out.lower():
                    ok = False
                    reason += f" unexpected '{s}' in output"

        tag = PASS if ok else FAIL
        print(f"  [{tag}] {name}")
        if not ok:
            print(f"         Reason: {reason}")
            print(f"         Output: {out[:300].strip()}")
        results.append((name, ok))
        return ok, out

    except subprocess.TimeoutExpired:
        print(f"  [{FAIL}] {name} -- TIMED OUT (hung)")
        results.append((name, False))
        return False, ""
    except FileNotFoundError:
        print(f"  [{FAIL}] {name} -- smash.exe not found at {SMASH}")
        results.append((name, False))
        return False, ""


def main():
    print(f"\nSmash Robustness Test Suite")
    print(f"Binary: {SMASH}")
    print("=" * 60)

    # ---------------------------------------------------------------
    print("\n-- Parser Edge Cases --")
    run_test("bare pipe",            ["|"],             expect_not_in_output=["panic", "unwrap"])
    run_test("bare redirect out",    [">"],             expect_not_in_output=["panic", "unwrap"])
    run_test("bare redirect in",     ["<"],             expect_not_in_output=["panic", "unwrap"])
    run_test("double pipe",          ["echo a | | echo b"], expect_not_in_output=["panic", "unwrap"])
    run_test("trailing pipe",        ["echo a |"],      expect_not_in_output=["panic", "unwrap"])
    run_test("trailing redirect",    ["echo a >"],      expect_not_in_output=["panic", "unwrap"])
    run_test("redirect no file",     ["< nonexistent_xyz.txt"], expect_not_in_output=["panic", "unwrap"])
    run_test("unclosed single quote", ["echo 'hello"],  expect_not_in_output=["panic", "unwrap"])
    run_test("unclosed double quote", ['echo "hello'],  expect_not_in_output=["panic", "unwrap"])
    run_test("empty semicolon",      [";"],             expect_not_in_output=["panic", "unwrap"])
    run_test("only whitespace",      ["   "],           expect_not_in_output=["panic", "unwrap"])
    run_test("backslash at end",     ["echo test\\"],   expect_not_in_output=["panic", "unwrap"])

    # ---------------------------------------------------------------
    print("\n-- AI Edge Cases --")
    run_test("smash with no query",          ["smash"], expect_not_in_output=["panic", "unwrap"])
    run_test("smash with only spaces",       ["smash   "], expect_not_in_output=["panic", "unwrap"])
    run_test("smash with garbage",           ["smash asdfjkl;qwer12345"], expect_not_in_output=["panic", "unwrap"])
    run_test("smash with unicode",           ["smash 列出所有文件"],   expect_not_in_output=["panic", "unwrap"])
    run_test("smash with very long input",   ["smash " + "word " * 200], expect_not_in_output=["panic", "unwrap"])
    run_test("smash delete everything",      ["smash delete everything"], expect_not_in_output=["panic", "unwrap"])

    # ---------------------------------------------------------------
    print("\n-- Alias Edge Cases --")
    run_test("alias list empty",             ["alias"], expect_not_in_output=["panic", "unwrap"])
    run_test("alias with equals",            ["alias mytest=echo hello", "mytest"], expect_in_output=["hello"])
    run_test("alias malformed = only",       ["alias =bad"],   expect_not_in_output=["panic", "unwrap"])
    run_test("alias no rhs",                 ["alias foo"],    expect_not_in_output=["panic", "unwrap"])
    run_test("alias self-reference",         ["alias a=a", "a"], expect_not_in_output=["panic", "unwrap"])
    run_test("alias chain depth 1",          ["alias a=echo done", "a"], expect_in_output=["done"])

    # ---------------------------------------------------------------
    print("\n-- cd Edge Cases --")
    run_test("cd to nonexistent",      ["cd /totally/nonexistent/path/xyz"], expect_not_in_output=["panic", "unwrap"])
    run_test("cd with no args",        ["cd"],                               expect_not_in_output=["panic", "unwrap"])
    run_test("cd past root",           ["cd ../../../../../../../../"],       expect_not_in_output=["panic", "unwrap"])
    run_test("cd to file (not dir)",   ["cd Cargo.toml"],                    expect_not_in_output=["panic", "unwrap"])

    # ---------------------------------------------------------------
    print("\n-- Pipe and Redirect --")
    run_test("simple echo pipe",       ["echo hello | findstr h"],        expect_not_in_output=["panic", "unwrap"])
    run_test("redirect output",        ["echo test123 > _smash_test.txt", "type _smash_test.txt", "del _smash_test.txt"],
             expect_not_in_output=["panic", "unwrap"])
    run_test("redirect nonexistent in", ["cat < _nonexistent_file_xyz.txt"], expect_not_in_output=["panic", "unwrap"])

    # ---------------------------------------------------------------
    print("\n-- Command Edge Cases --")
    run_test("nonexistent command",    ["totally_fake_command_xyz_123"], expect_not_in_output=["panic", "unwrap"])
    run_test("empty string command",   [""],                              expect_not_in_output=["panic", "unwrap"])
    run_test("command with unicode",   ["echo \u00e9\u00e0\u00fc"],      expect_not_in_output=["panic", "unwrap"])
    run_test("export valid",           ["export FOO=bar"],               expect_not_in_output=["panic", "unwrap"])
    run_test("export invalid",         ["export NOEQUALS"],              expect_not_in_output=["panic", "unwrap"])
    run_test("pwd",                    ["pwd"],                          expect_not_in_output=["panic", "unwrap"])

    # ---------------------------------------------------------------
    print("\n" + "=" * 60)
    passed = sum(1 for _, ok in results if ok)
    total  = len(results)
    print(f"Results: {passed}/{total} passed")
    if passed < total:
        print("\nFailed tests:")
        for name, ok in results:
            if not ok:
                print(f"  - {name}")

    return 0 if passed == total else 1


if __name__ == "__main__":
    sys.exit(main())
