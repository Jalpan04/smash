/*
  app.js
  Smash Shell Static Website Dynamic Features
  Terminal simulation and clipboard utilities
*/

// ---------------------------------------------------------------------------
// 1. Text Copy Utility
// ---------------------------------------------------------------------------
function triggerCopy(elementId, button) {
    const codeElement = document.getElementById(elementId);
    if (!codeElement) return;

    const textToCopy = codeElement.textContent || codeElement.innerText;

    navigator.clipboard.writeText(textToCopy).then(() => {
        // Visual feedback
        const originalText = button.textContent;
        button.textContent = "COPIED!";
        button.style.color = "#e05b38";
        button.style.borderColor = "#e05b38";
        
        setTimeout(() => {
            button.textContent = originalText;
            button.style.color = "";
            button.style.borderColor = "";
        }, 1500);
    }).catch(err => {
        console.error("Failed to copy text: ", err);
    });
}

// ---------------------------------------------------------------------------
// 2. Interactive Terminal Emulator
// ---------------------------------------------------------------------------
const terminalScreen = document.getElementById("emulator-screen");

// Scenario actions:
// 'type' = typing a command
// 'output' = rendering output
// 'suggest' = showing AI translation suggestion
// 'pause' = wait time
const demoScript = [
    { type: "output", text: "Smash (Smart Bash) - running on windows\nLoading AI model...\nAI model loaded. Type 'smash <query>' for AI translation.\n\n" },
    { type: "pause", duration: 1000 },
    
    // Command 1: AI translation on Windows
    { type: "prompt", text: "smash:D:\\projects\\smash> " },
    { type: "type", text: "smash list all python files" },
    { type: "pause", duration: 600 },
    { type: "suggest", text: "[windows] AI suggests: Get-ChildItem -Recurse -Filter *.py\n" },
    { type: "pause", duration: 800 },
    { type: "output", text: "    Directory: D:\\projects\\smash\\src\n\nMode                 LastWriteTime         Length Name\n----                 -------------         ------ ----\n-a---          26-05-2026    16:22           4742 ai.rs\n-a---          26-05-2026    16:05           8212 executor.rs\n-a---          26-05-2026    16:32          17841 main.rs\n-a---          26-05-2026    12:20           5218 parser.rs\n\n" },
    { type: "pause", duration: 1500 },

    // Command 2: Set alias
    { type: "prompt", text: "smash:D:\\projects\\smash> " },
    { type: "type", text: "alias gs=git status" },
    { type: "pause", duration: 400 },
    { type: "output", text: "alias gs='git status'\n" },
    { type: "pause", duration: 1000 },

    // Command 3: Run alias
    { type: "prompt", text: "smash:D:\\projects\\smash> " },
    { type: "type", text: "gs" },
    { type: "pause", duration: 500 },
    { type: "output", text: "On branch master\nYour branch is up to date with 'origin/master'.\n\nnothing to commit, working tree clean\n\n" },
    { type: "pause", duration: 1800 },

    // Command 4: Linux simulation
    { type: "output", text: "smash: exit\n\nSmash (Smart Bash) - running on linux\nLoading AI model...\nAI model loaded. Type 'smash <query>' for AI translation.\n\n" },
    { type: "pause", duration: 1000 },
    
    // Command 5: AI translation on Linux
    { type: "prompt", text: "smash:~> " },
    { type: "type", text: "smash show free disk space" },
    { type: "pause", duration: 600 },
    { type: "suggest", text: "[linux] AI suggests: df -h\n" },
    { type: "pause", duration: 800 },
    { type: "output", text: "Filesystem      Size  Used Avail Use% Mounted on\n/dev/sda1        50G   14G   34G  30% /\ntmpfs            64M     0   64M   0% /dev\nshm              64M     0   64M   0% /dev/shm\n\n" },
    { type: "pause", duration: 2500 },
    
    // Reset terminal
    { type: "clear" }
];

async function runTerminalDemo() {
    if (!terminalScreen) return;

    while (true) { // Infinite demo loop
        terminalScreen.innerHTML = "";
        
        for (const step of demoScript) {
            if (step.type === "clear") {
                terminalScreen.innerHTML = "";
                continue;
            }

            if (step.type === "pause") {
                await sleep(step.duration);
                continue;
            }

            if (step.type === "prompt") {
                const promptSpan = document.createElement("span");
                promptSpan.className = "terminal-prompt";
                promptSpan.innerHTML = step.text;
                terminalScreen.appendChild(promptSpan);
                terminalScreen.scrollTop = terminalScreen.scrollHeight;
                continue;
            }

            if (step.type === "output") {
                const outputSpan = document.createElement("span");
                outputSpan.className = "terminal-output";
                outputSpan.innerHTML = step.text;
                terminalScreen.appendChild(outputSpan);
                terminalScreen.scrollTop = terminalScreen.scrollHeight;
                continue;
            }

            if (step.type === "suggest") {
                const suggestSpan = document.createElement("span");
                suggestSpan.className = "terminal-suggest";
                suggestSpan.innerHTML = step.text;
                terminalScreen.appendChild(suggestSpan);
                terminalScreen.scrollTop = terminalScreen.scrollHeight;
                continue;
            }

            if (step.type === "type") {
                const inputSpan = document.createElement("span");
                inputSpan.className = "terminal-input";
                terminalScreen.appendChild(inputSpan);
                
                const cursorSpan = document.createElement("span");
                cursorSpan.className = "cursor";
                terminalScreen.appendChild(cursorSpan);

                // Simulate typewriter typing:
                for (let i = 0; i < step.text.length; i++) {
                    inputSpan.textContent += step.text[i];
                    terminalScreen.scrollTop = terminalScreen.scrollHeight;
                    await sleep(Math.random() * 80 + 40); // randomized keystroke speed
                }
                
                // Remove cursor from this input line
                cursorSpan.remove();
                
                // Add new line break at the end of command typing
                const lineBreak = document.createTextNode("\n");
                terminalScreen.appendChild(lineBreak);
                terminalScreen.scrollTop = terminalScreen.scrollHeight;
            }
        }
    }
}

function sleep(ms) {
    return new Promise(resolve => setTimeout(resolve, ms));
}

// Start terminal demo on window load
window.addEventListener("DOMContentLoaded", () => {
    runTerminalDemo();
});
