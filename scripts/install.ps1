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

function Get-InstalledVersion {
    param([string]$InstallPath)

    $binaryPath = Join-Path $InstallPath $BinaryName
    if (Test-Path $binaryPath) {
        try {
            $versionOutput = & $binaryPath --version 2>&1
            if ($versionOutput -match '(\d+\.\d+\.\d+)') {
                return $Matches[1]
            }
        }
        catch {
            return $null
        }
    }
    return $null
}

function Get-NormalizedVersion {
    param([string]$Version)
    return $Version -replace '^v', ''
}

function Install-Binary {
    param(
        [string]$SourcePath,
        [string]$DestinationPath
    )

    # Clean up any stale .old file left over from a previous update where the
    # binary was still loaded and couldn't be deleted immediately.
    $oldPath = "$DestinationPath.old"
    if (Test-Path $oldPath) {
        Remove-Item -Path $oldPath -Force -ErrorAction SilentlyContinue
    }

    # Fresh install: nothing to replace, just move.
    if (-not (Test-Path $DestinationPath)) {
        Move-Item -Path $SourcePath -Destination $DestinationPath -Force
        return
    }

    # Try a direct overwrite first. This works when the existing binary isn't
    # currently loaded by any process.
    try {
        Move-Item -Path $SourcePath -Destination $DestinationPath -Force -ErrorAction Stop
        return
    }
    catch {
        # The existing binary is in use (daemon running, open in another shell,
        # AV scanning, etc.). On Windows a running .exe can be renamed but not
        # overwritten, so move it aside and install the new one in its place.
        Write-Info "Existing binary is in use; swapping via rename..."
    }

    try {
        Move-Item -Path $DestinationPath -Destination $oldPath -Force -ErrorAction Stop
    }
    catch {
        Remove-Item -Path $SourcePath -ErrorAction SilentlyContinue
        Write-Error "Failed to replace existing binary at ${DestinationPath}: $_`nIf smart-scribe is running (e.g. as a daemon), stop it and re-run the installer."
    }

    try {
        Move-Item -Path $SourcePath -Destination $DestinationPath -Force -ErrorAction Stop
    }
    catch {
        # Best effort: restore the previous binary so the user isn't left empty-handed.
        Move-Item -Path $oldPath -Destination $DestinationPath -Force -ErrorAction SilentlyContinue
        Remove-Item -Path $SourcePath -ErrorAction SilentlyContinue
        Write-Error "Failed to install new binary: $_"
    }

    # Try to remove the old binary now. If it's still loaded the delete will
    # fail; in that case it'll be cleaned up on the next install run.
    Remove-Item -Path $oldPath -Force -ErrorAction SilentlyContinue
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

    # Construct download URL
    $artifact = "smart-scribe-windows-x86_64.exe"
    $downloadUrl = "https://github.com/$Repo/releases/download/$version/$artifact"

    # Determine install directory
    $installPath = Get-InstallPath

    # Check for existing installation
    $currentVersion = Get-InstalledVersion -InstallPath $installPath
    $targetVersion = Get-NormalizedVersion -Version $version

    # Determine install type and show appropriate message
    if (-not $currentVersion) {
        $installType = "fresh"
        Write-Info "Installing smart-scribe v$targetVersion..."
    }
    elseif ($currentVersion -eq $targetVersion) {
        $installType = "reinstall"
        Write-Info "Reinstalling smart-scribe v$targetVersion..."
    }
    else {
        $installType = "update"
        Write-Info "Updating smart-scribe from v$currentVersion to v$targetVersion..."
    }

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

    try {
        Install-Binary -SourcePath $tempFile -DestinationPath $binaryPath
    }
    catch {
        Remove-Item -Path $tempFile -ErrorAction SilentlyContinue
        throw
    }

    # Verify installation
    if (Test-Path $binaryPath) {
        try {
            $installedVersion = & $binaryPath --version 2>&1
            if ($installedVersion -match '(\d+\.\d+\.\d+)') {
                $versionNum = $Matches[1]
            } else {
                $versionNum = "unknown"
            }

            switch ($installType) {
                "fresh"     { Write-Success "Successfully installed: smart-scribe $versionNum" }
                "update"    { Write-Success "Successfully updated: smart-scribe $versionNum" }
                "reinstall" { Write-Success "Successfully reinstalled: smart-scribe $versionNum" }
            }
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
    Write-Host "  1. Sign in with your ChatGPT account (recommended):"
    Write-Host "     smart-scribe login"
    Write-Host ""
    Write-Host "     Or, to use a metered OpenAI API key instead:"
    Write-Host "     smart-scribe config set auth api_key"
    Write-Host "     smart-scribe config set openai_api_key sk-..."
    Write-Host ""
    Write-Host "  2. Test it:"
    Write-Host "     smart-scribe --help"
    Write-Host ""
    Write-Host "Get an OpenAI API key at: https://platform.openai.com/api-keys"
    Write-Host ""
}

Main
