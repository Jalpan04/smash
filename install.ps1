# install.ps1 - One-command Smash installer for Windows
$Repo = "Jalpan04/smash"
$InstallDir = "$env:USERPROFILE\.local\bin"
$ModelDir = "$env:USERPROFILE\.smash\model"
$ExeName = "smash.exe"

Write-Host "Installing Smash shell for Windows..." -ForegroundColor Cyan

# 1. Create directories
New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
New-Item -ItemType Directory -Force -Path $ModelDir | Out-Null

# 2. Download latest binary from GitHub Releases
Write-Host "Downloading latest release binary..."
$ReleaseUrl = "https://api.github.com/repos/$Repo/releases/latest"
$Release = try { Invoke-RestMethod -Uri $ReleaseUrl } catch { $null }

if ($Release) {
    $Asset = $Release.assets | Where-Object { $_.name -match "smash-windows-x86_64.exe" }
    if ($Asset) {
        Invoke-WebRequest -Uri $Asset.browser_download_url -OutFile "$InstallDir\$ExeName"
        Write-Host "Downloaded smash.exe to $InstallDir" -ForegroundColor Green
    } else {
        Write-Host "Could not find Windows binary in the latest release." -ForegroundColor Red
        Exit
    }
} else {
    Write-Host "Could not fetch release data. Check your internet connection." -ForegroundColor Red
    Exit
}

# 3. Download AI Model Files directly from GitHub Releases
if (-Not (Test-Path "$ModelDir\encoder_model.onnx") -Or -Not (Test-Path "$ModelDir\decoder_model.onnx") -Or -Not (Test-Path "$ModelDir\tokenizer.json")) {
    Write-Host "Downloading AI model files from Releases..."
    foreach ($File in @("encoder_model.onnx", "decoder_model.onnx", "tokenizer.json")) {
        $Asset = $Release.assets | Where-Object { $_.name -eq $File }
        if ($Asset) {
            Write-Host "Downloading $File..."
            Invoke-WebRequest -Uri $Asset.browser_download_url -OutFile "$ModelDir\$File"
        } else {
            Write-Host "Could not find $File in the latest release." -ForegroundColor Red
            Exit
        }
    }
    Write-Host "AI Models installed to $ModelDir" -ForegroundColor Green
}

# 4. Set Environment Variables
Write-Host "Configuring Environment Variables..."

# Set SMASH_MODEL_DIR permanently for the user
[Environment]::SetEnvironmentVariable("SMASH_MODEL_DIR", $ModelDir, "User")
Write-Host "Set SMASH_MODEL_DIR to $ModelDir" -ForegroundColor Green

# Add to PATH if not exists
$UserPath = [Environment]::GetEnvironmentVariable("PATH", "User")
if ($UserPath -notmatch [regex]::Escape($InstallDir)) {
    $NewPath = "$UserPath;$InstallDir"
    [Environment]::SetEnvironmentVariable("PATH", $NewPath, "User")
    Write-Host "Added $InstallDir to PATH." -ForegroundColor Green
}

Write-Host "`nInstallation Complete!" -ForegroundColor Cyan
Write-Host "IMPORTANT: Please restart your PowerShell terminal to apply PATH changes, then type 'smash' to start." -ForegroundColor Yellow
