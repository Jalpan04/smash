/*
  app.js
  Smash Shell Static Website Dynamic Features
  Clipboard utilities and dynamic download scripts for the Clockwork Rust theme
*/

/**
 * Copies the text content of a specified element to the clipboard
 * and provides visual feedback to the user on the button clicked.
 * 
 * @param {string} elementId - The ID of the element containing the text to copy
 * @param {HTMLElement} button - The button element that triggered the copy
 */
function triggerCopy(elementId, button) {
    const codeElement = document.getElementById(elementId);
    if (!codeElement) return;

    const textToCopy = codeElement.textContent || codeElement.innerText;

    navigator.clipboard.writeText(textToCopy).then(() => {
        // Visual feedback - Clockwork Rust orange flavor
        const originalText = button.textContent;
        button.textContent = "COPIED!";
        button.style.color = "#FF8C00";
        button.style.borderColor = "#FF8C00";
        
        setTimeout(() => {
            button.textContent = originalText;
            button.style.color = "";
            button.style.borderColor = "";
        }, 1500);
    }).catch(err => {
        console.error("Failed to copy text: ", err);
    });
}

/**
 * Detects the user's operating system platform and dynamically updates 
 * the direct download binary URL to download the correct platform-specific release.
 */
function initDownloadLink() {
    const downloadBtn = document.getElementById("download-btn");
    if (!downloadBtn) return;

    const userAgent = (window.navigator.userAgent || "").toLowerCase();
    const platform = (window.navigator.platform || "").toLowerCase();
    let assetName = "";

    // Detect Windows or Linux
    if (userAgent.indexOf("win") !== -1 || platform.indexOf("win") !== -1) {
        assetName = "smash-windows-x86_64.exe";
    } else if (userAgent.indexOf("linux") !== -1 || platform.indexOf("linux") !== -1) {
        assetName = "smash-linux-x86_64";
    }

    if (assetName) {
        // Automatically redirects to the latest tag's binary direct download
        downloadBtn.href = `https://github.com/Jalpan04/smash/releases/latest/download/${assetName}`;
    } else {
        // Graceful fallback to the general releases listing page for undetected OS/platforms
        downloadBtn.href = "https://github.com/Jalpan04/smash/releases";
    }
}

// Initialize scripts when DOM content has loaded
document.addEventListener("DOMContentLoaded", () => {
    initDownloadLink();
});
