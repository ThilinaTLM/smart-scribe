# SmartScribe installer for Windows
# Usage: irm https://raw.githubusercontent.com/ThilinaTLM/smart-scribe/main/scripts/install.ps1 | iex
#
# Environment variables:
#   INSTALL_DIR - Override installation directory
#   VERSION     - Install specific version (default: latest)

$ErrorActionPreference = "Stop"

$Repo = "ThilinaTLM/smart-scribe"
$BinaryName = "smart-scribe.exe"

function Write-Info {
    param([string]$Message)
    Write-Host "==> " -ForegroundColor Cyan -NoNewline
    Write-Host $Message
}

function Write-Success {
    param([string]$Message)
    Write-Host "==> " -ForegroundColor Green -NoNewline
    Write-Host $Message
}

function Write-Warning {
    param([string]$Message)
    Write-Host "Warning: " -ForegroundColor Yellow -NoNewline
    Write-Host $Message
}

function Write-Error {
    param([string]$Message)
    Write-Host "Error: " -ForegroundColor Red -NoNewline
    Write-Host $Message
    exit 1
}

function Get-LatestVersion {
    $url = "https://api.github.com/repos/$Repo/releases/latest"
    try {
        $response = Invoke-RestMethod -Uri $url -UseBasicParsing
        return $response.tag_name
    }
    catch {
        Write-Error "Failed to fetch latest release: $_"
    }
}

function Get-InstallPath {
    if ($env:INSTALL_DIR) {
        return $env:INSTALL_DIR
    }
    return "$env:LOCALAPPDATA\Programs\smart-scribe"
}

function Add-ToPath {
    param([string]$Directory)

    $currentPath = [Environment]::GetEnvironmentVariable("Path", "User")

    if ($currentPath -split ";" -contains $Directory) {
        return $false
    }

    $newPath = "$currentPath;$Directory"
    [Environment]::SetEnvironmentVariable("Path", $newPath, "User")

    # Update current session
    $env:Path = "$env:Path;$Directory"

    return $true
}

function Main {
    Write-Host ""
    Write-Host "SmartScribe Installer" -ForegroundColor White
    Write-Host "==============================" -ForegroundColor White
    Write-Host ""

    # Platform check
    Write-Info "Detected platform: windows-x86_64"

    # Get version
    if ($env:VERSION) {
        $version = $env:VERSION
    }
    else {
        Write-Info "Fetching latest release..."
        $version = Get-LatestVersion
    }

    if (-not $version) {
        Write-Error "Could not determine version to install"
    }

    Write-Info "Installing version: $version"

    # Construct download URL
    $artifact = "smart-scribe-windows-x86_64.exe"
    $downloadUrl = "https://github.com/$Repo/releases/download/$version/$artifact"

    # Determine install directory
    $installPath = Get-InstallPath

    # Create install directory if needed
    if (-not (Test-Path $installPath)) {
        Write-Info "Creating directory: $installPath"
        New-Item -ItemType Directory -Path $installPath -Force | Out-Null
    }

    # Download binary
    $tempFile = [System.IO.Path]::GetTempFileName()
    Write-Info "Downloading $artifact..."

    try {
        Invoke-WebRequest -Uri $downloadUrl -OutFile $tempFile -UseBasicParsing
    }
    catch {
        Remove-Item -Path $tempFile -ErrorAction SilentlyContinue
        Write-Error "Failed to download binary: $_"
    }

    # Install binary
    $binaryPath = Join-Path $installPath $BinaryName
    Write-Info "Installing to $binaryPath"

    Move-Item -Path $tempFile -Destination $binaryPath -Force

    # Verify installation
    if (Test-Path $binaryPath) {
        try {
            $installedVersion = & $binaryPath --version 2>&1
            Write-Success "Installed: $installedVersion"
        }
        catch {
            Write-Success "Binary installed (version check skipped)"
        }
    }
    else {
        Write-Error "Installation failed - binary not found"
    }

    # Add to PATH if needed
    $pathAdded = Add-ToPath -Directory $installPath

    if ($pathAdded) {
        Write-Success "Added $installPath to user PATH"
        Write-Host ""
        Write-Warning "Restart your terminal to use 'smart-scribe' command"
    }

    # Print next steps
    Write-Host ""
    Write-Success "Installation complete!"
    Write-Host ""
    Write-Host "Next steps:"
    Write-Host "  1. Set your Gemini API key:"
    Write-Host "     smart-scribe config set api_key YOUR_API_KEY"
    Write-Host ""
    Write-Host "  2. Test it:"
    Write-Host "     smart-scribe --help"
    Write-Host ""
    Write-Host "Get your API key at: https://aistudio.google.com/apikey"
    Write-Host ""
}

Main
