#Requires -Version 5.1
<#
.SYNOPSIS
    LiteForge Installer for Windows

.DESCRIPTION
    Installs the LiteForge components including CLI, Python SDK, Node.js SDK,
    configures credentials in Windows Credential Manager, and sets up the environment.

.PARAMETER Version
    Specific version to install (default: latest)

.PARAMETER ForgeHome
    Installation directory (default: $env:USERPROFILE\.forge)

.PARAMETER WithCaBundle
    Install a LiteForge-scoped CA bundle (for internal/proxy CAs). Off by
    default. When set, the bundle is exposed to LiteForge via environment
    variables only; it is never added to the Windows certificate store.

.PARAMETER SkipCerts
    Deprecated. CA bundle install is now opt-in (see -WithCaBundle); this
    switch is accepted as a no-op for backward compatibility.

.PARAMETER NonInteractive
    Run without prompts (requires environment variables to be set)

.EXAMPLE
    git clone https://github.com/seanpoyner/liteforge.git $env:TEMP\liteforge
    & $env:TEMP\liteforge\scripts\install.ps1
    Remove-Item -Recurse -Force $env:TEMP\liteforge

.EXAMPLE
    .\install.ps1 -Version "0.1.0" -ForgeHome "C:\LiteForge"
#>

[CmdletBinding()]
param(
    [string]$Version = "latest",
    [string]$ForgeHome = "$env:USERPROFILE\.forge",
    [switch]$WithCaBundle,
    [switch]$SkipCerts,
    [switch]$NonInteractive
)

$ErrorActionPreference = "Stop"

# ============================================================================
# Configuration
# ============================================================================

$script:GitHubReleaseUrl = "https://github.com/seanpoyner/liteforge/releases"
$script:DefaultBaseUrl = "https://api.example.com/v1"
$script:DefaultModel = "anthropic.claude-haiku-4-5-20251001-v1:0"

# Detect if running from a git clone (for building from source)
$script:ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$script:RepoRoot = Split-Path -Parent $script:ScriptDir
$script:InRepo = Test-Path (Join-Path $script:RepoRoot "Cargo.toml")

# Component selections
$script:InstallCli = $false
$script:InstallPy = $false
$script:InstallJs = $false
$script:InstallRs = $false

# Configuration values
$script:TipApiKey = ""
$script:TipBaseUrl = ""
$script:TipDefaultModel = ""
$script:TipKnowledgeUrl = ""
$script:TipTemporalUrl = ""

# ============================================================================
# Helper Functions
# ============================================================================

function Write-Info {
    param([string]$Message)
    Write-Output "==> $Message"
}

function Write-Success {
    param([string]$Message)
    Write-Output "[OK] $Message"
}

function Write-Warning {
    param([string]$Message)
    Write-Output "[!] $Message"
}

function Write-Error {
    param([string]$Message)
    Write-Output "[X] $Message"
}

function Read-Prompt {
    param(
        [string]$Prompt,
        [string]$Default = "",
        [switch]$Secret
    )

    $displayPrompt = $Prompt
    if ($Default) {
        $displayPrompt = "$Prompt [$Default]"
    }

    if ($Secret) {
        $secureString = Read-Host -Prompt "? $displayPrompt" -AsSecureString
        $BSTR = [System.Runtime.InteropServices.Marshal]::SecureStringToBSTR($secureString)
        $value = [System.Runtime.InteropServices.Marshal]::PtrToStringAuto($BSTR)
        [System.Runtime.InteropServices.Marshal]::ZeroFreeBSTR($BSTR)
    } else {
        $value = Read-Host -Prompt "? $displayPrompt"
    }

    if ([string]::IsNullOrWhiteSpace($value) -and $Default) {
        return $Default
    }

    return $value
}

function Read-Confirm {
    param(
        [string]$Prompt,
        [bool]$Default = $false
    )

    $suffix = if ($Default) { "[Y/n]" } else { "[y/N]" }

    $response = Read-Host -Prompt "? $Prompt $suffix"

    if ([string]::IsNullOrWhiteSpace($response)) {
        return $Default
    }

    return $response -match "^[Yy]"
}

# ============================================================================
# Platform Detection
# ============================================================================

function Get-Platform {
    Write-Info "Detecting platform..."

    $arch = if ([Environment]::Is64BitOperatingSystem) {
        "x86_64"
    } else {
        throw "32-bit Windows is not supported"
    }

    $script:Target = "$arch-pc-windows-msvc"
    Write-Success "Detected platform: Windows ($arch)"
}

# ============================================================================
# Prerequisite Checks
# ============================================================================

function Test-Prerequisites {
    Write-Info "Checking prerequisites..."

    # Check for Python
    $hasPython = Get-Command python -ErrorAction SilentlyContinue
    if (-not $hasPython) {
        Write-Warning "Python not found - Python SDK installation will be skipped"
    }

    # Check for Node.js
    $hasNode = Get-Command node -ErrorAction SilentlyContinue
    $hasNpm = Get-Command npm -ErrorAction SilentlyContinue
    if (-not $hasNode -or -not $hasNpm) {
        Write-Warning "Node.js/npm not found - Node.js SDK installation will be skipped"
    }

    # Check for Cargo
    $hasCargo = Get-Command cargo -ErrorAction SilentlyContinue
    if (-not $hasCargo) {
        Write-Warning "Cargo not found - Rust SDK will provide git instructions only"
    }

    Write-Success "Prerequisite check complete"
}

# ============================================================================
# CA Certificate Installation
# ============================================================================

function Install-CACertificates {
    # Opt-in only. Most users do not need this; it matters behind a
    # TLS-inspecting proxy with an internal CA. When enabled, the bundle is
    # scoped to LiteForge via environment variables and is NEVER added to the
    # Windows certificate store, so this install does not widen the trust base
    # of the whole machine.
    if (-not $WithCaBundle) {
        return
    }

    Write-Info "Installing a LiteForge-scoped CA bundle (no changes to the Windows certificate store)..."

    $certDir = Join-Path $ForgeHome "certs"
    $certDest = Join-Path $certDir "ca-bundle.crt"

    New-Item -ItemType Directory -Path $certDir -Force | Out-Null

    # Prefer a local bundle when running from a git clone
    $certSource = $null
    if ($script:InRepo) {
        $localCert = Join-Path $script:RepoRoot "certs\combined-full-ca-bundle.crt"
        if (Test-Path $localCert) {
            $certSource = $localCert
        }
    }

    if ($certSource) {
        Copy-Item -Path $certSource -Destination $certDest -Force
        Write-Success "CA bundle written to $certDest"
    } else {
        $certUrl = if ($Version -eq "latest") {
            "$script:GitHubReleaseUrl/latest/download/ca-bundle.crt"
        } else {
            "$script:GitHubReleaseUrl/download/v$Version/ca-bundle.crt"
        }

        try {
            Invoke-WebRequest -Uri $certUrl -OutFile $certDest -UseBasicParsing
            Write-Success "CA bundle downloaded to $certDest"
        } catch {
            Write-Warning "Could not download CA bundle - skipping"
            return
        }
    }

    # Expose the bundle to LiteForge and its language bindings via environment
    # variables (process + persisted at User scope). LITEFORGE_EXTRA_CA_FILE is
    # read by the Rust client; the others cover the Python/Node SDKs.
    foreach ($name in @("LITEFORGE_EXTRA_CA_FILE", "SSL_CERT_FILE", "REQUESTS_CA_BUNDLE", "NODE_EXTRA_CA_CERTS")) {
        Set-Item -Path "Env:\$name" -Value $certDest
        [Environment]::SetEnvironmentVariable($name, $certDest, "User")
    }

    Write-Info "These certificates are trusted by LiteForge only, not the Windows certificate store."
}

# ============================================================================
# Configuration Collection
# ============================================================================

function Get-Configuration {
    Write-Info "Configuring LiteForge..."
    Write-Host ""

    # LiteForge API Key (required)
    $script:TipApiKey = Read-Prompt "LiteForge API Key" "" -Secret
    if ([string]::IsNullOrWhiteSpace($script:TipApiKey)) {
        throw "LiteForge API Key is required"
    }

    # Base URL
    $script:TipBaseUrl = Read-Prompt "LiteForge Base URL" $script:DefaultBaseUrl

    # Default model
    $script:TipDefaultModel = Read-Prompt "Default Model" $script:DefaultModel

    # Optional services
    Write-Host ""
    Write-Info "Optional service endpoints (press Enter to skip):"
    $script:TipKnowledgeUrl = Read-Prompt "Knowledge Service URL" ""
    $script:TipTemporalUrl = Read-Prompt "Temporal Endpoint" ""

    Write-Host ""
}

# ============================================================================
# Credential Validation
# ============================================================================

function Test-Credentials {
    Write-Info "Validating credentials..."

    # Try /models endpoint (standard for OpenAI-compatible APIs like LiteLLM)
    $modelsUrl = "$($script:TipBaseUrl.TrimEnd('/'))/models"

    try {
        $headers = @{
            "Authorization" = "Bearer $script:TipApiKey"
            "Content-Type" = "application/json"
        }
        $response = Invoke-RestMethod -Uri $modelsUrl -Headers $headers -TimeoutSec 15
        if ($response.data -or $response.models) {
            Write-Success "Credentials validated successfully"
            return
        }
        # Got a response but no models - might still be valid
        Write-Success "API responded (no models returned)"
    } catch {
        $errorMsg = $_.Exception.Message
        Write-Warning "Could not validate LiteForge API credentials"
        Write-Output "  Error: $errorMsg"
        Write-Output "  URL: $modelsUrl"
        if (-not (Read-Confirm "Continue anyway?" $false)) {
            throw "Credential validation failed"
        }
    }
}

# ============================================================================
# Component Selection
# ============================================================================

function Select-Components {
    Write-Info "Select components to install:"
    Write-Host ""

    $script:InstallCli = Read-Confirm "  forge-cli (CLI tool)" $true

    # Check if python actually works (not just the Microsoft Store stub on Windows)
    $hasPython = $false
    if (Get-Command python -ErrorAction SilentlyContinue) {
        $pyVersion = & python --version 2>&1
        if ($LASTEXITCODE -eq 0 -and $pyVersion -match "Python \d") {
            $hasPython = $true
        }
    }
    if ($hasPython) {
        $script:InstallPy = Read-Confirm "  liteforge-py (Python SDK)" $false
    }

    $hasNpm = Get-Command npm -ErrorAction SilentlyContinue
    if ($hasNpm) {
        $script:InstallJs = Read-Confirm "  liteforge-js (Node.js SDK)" $false
    }

    $hasCargo = Get-Command cargo -ErrorAction SilentlyContinue
    if ($hasCargo) {
        $script:InstallRs = Read-Confirm "  liteforge (Rust SDK)" $false
    }

    if (-not $script:InstallCli -and -not $script:InstallPy -and -not $script:InstallJs -and -not $script:InstallRs) {
        Write-Warning "No components selected"
        if (-not (Read-Confirm "Continue with configuration only?" $false)) {
            exit 0
        }
    }

    Write-Host ""
}

# ============================================================================
# Component Installation
# ============================================================================

function Install-Cli {
    if (-not $script:InstallCli) {
        return
    }

    Write-Info "Installing forge-cli..."

    $binDir = Join-Path $ForgeHome "bin"
    New-Item -ItemType Directory -Path $binDir -Force | Out-Null

    # First priority: use pre-built binary from repo if running from git clone
    # This avoids issues with VDI environments that can't download or build
    if ($script:InRepo) {
        $prebuiltBinary = Join-Path $script:RepoRoot "bin\forge-x86_64-pc-windows-gnu.exe"
        if (Test-Path $prebuiltBinary) {
            Write-Info "Using pre-built binary from repo..."
            Copy-Item -Path $prebuiltBinary -Destination (Join-Path $binDir "forge.exe") -Force
            Write-Success "forge-cli installed to $binDir\forge.exe"
            return
        }
    }

    # If running from a git clone with cargo available, build from source
    $hasCargo = Get-Command cargo -ErrorAction SilentlyContinue
    if ($script:InRepo -and $hasCargo) {
        Write-Info "Building forge-cli from source..."

        # Check if MSVC link.exe is available
        $canBuild = $true
        if (-not (Get-Command link.exe -ErrorAction SilentlyContinue)) {
            # Try to set up VS environment
            $vsWhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
            if (Test-Path $vsWhere) {
                $vsPath = & $vsWhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
                if ($vsPath) {
                    $vcvars = Join-Path $vsPath "VC\Auxiliary\Build\vcvars64.bat"
                    if (Test-Path $vcvars) {
                        Write-Info "Setting up Visual Studio environment..."
                        cmd /c "`"$vcvars`" >nul 2>&1 && set" | ForEach-Object {
                            if ($_ -match "^([^=]+)=(.*)$") {
                                Set-Item -Path "Env:\$($matches[1])" -Value $matches[2]
                            }
                        }
                        Write-Success "Visual Studio environment configured"
                    }
                }
            }

            # If still no link.exe, try using WSL to cross-compile
            if (-not (Get-Command link.exe -ErrorAction SilentlyContinue)) {
                Write-Info "MSVC not available, checking for WSL..."

                $hasWsl = Get-Command wsl.exe -ErrorAction SilentlyContinue
                if ($hasWsl) {
                    # Use wsl -e (exec) to run commands directly without invoking any shell (avoids fish prompt issues)
                    $wslHasMingw = wsl.exe -e which x86_64-w64-mingw32-gcc 2>$null
                    $wslHasTarget = wsl.exe -e rustup target list --installed 2>$null | Select-String "x86_64-pc-windows-gnu"

                    if (-not $wslHasMingw -or -not $wslHasTarget) {
                        Write-Info "Setting up WSL cross-compilation (one-time setup)..."
                        Write-Output "  Installing mingw-w64 and Rust Windows target..."

                        # Install prerequisites in WSL (--noprofile --norc skips shell init)
                        wsl.exe -e bash --noprofile --norc -c "export PATH=`$HOME/.cargo/bin:`$PATH && sudo apt-get update && sudo apt-get install -y mingw-w64 && rustup target add x86_64-pc-windows-gnu" 2>&1 | Out-Null
                    }

                    Write-Info "Cross-compiling with WSL..."
                    # Convert Windows path to WSL path manually (avoids wslpath shell issues)
                    # C:\Users\foo\bar -> /mnt/c/Users/foo/bar
                    $wslRepoPath = $script:RepoRoot -replace '^([A-Za-z]):', { "/mnt/$($_.Groups[1].Value.ToLower())" }
                    $wslRepoPath = $wslRepoPath.Replace('\', '/')

                    # Build using WSL (--noprofile --norc skips shell init that may invoke fish)
                    $buildResult = wsl.exe -e bash --noprofile --norc -c "cd '$wslRepoPath' && export PATH=`$HOME/.cargo/bin:`$PATH && cargo build --release --target x86_64-pc-windows-gnu --package forge-cli 2>&1"
                    $buildExitCode = $LASTEXITCODE

                    if ($buildExitCode -eq 0) {
                        $wslBinary = Join-Path $script:RepoRoot "target\x86_64-pc-windows-gnu\release\forge.exe"
                        if (Test-Path $wslBinary) {
                            Copy-Item -Path $wslBinary -Destination (Join-Path $binDir "forge.exe") -Force
                            Write-Success "forge-cli cross-compiled via WSL and installed to $binDir\forge.exe"
                            return
                        }
                    } else {
                        Write-Warning "WSL cross-compilation failed"
                        Write-Output $buildResult
                    }
                }

                # No WSL or build failed
                $canBuild = $false
                Write-Host ""
                Write-Output "  Building from source requires either:"
                Write-Output "    - Visual Studio Build Tools (for native MSVC build)"
                Write-Output "    - WSL with mingw-w64 (for cross-compilation)"
                Write-Host ""
                Write-Output "  Attempting to download pre-built binary instead..."
            }
        }

        if ($canBuild) {
            Push-Location $script:RepoRoot
            try {
                Write-Output "  Running: cargo build --release --package forge-cli"
                $buildOutput = & cargo build --release --package forge-cli 2>&1
                if ($LASTEXITCODE -eq 0) {
                    $binary = Join-Path $script:RepoRoot "target\release\forge.exe"
                    if (Test-Path $binary) {
                        Copy-Item -Path $binary -Destination (Join-Path $binDir "forge.exe") -Force
                        Write-Success "forge-cli built and installed to $binDir\forge.exe"
                        return
                    }
                }
                # Build failed - show error and fall through to download
                Write-Warning "Build from source failed"
                $buildOutput | Select-Object -Last 15 | ForEach-Object { Write-Output "  $_" }
            } catch {
                Write-Warning "Build from source failed: $($_.Exception.Message)"
            } finally {
                Pop-Location
            }
        }
    }

    # Try downloading from GitHub releases (requires auth on GHE)
    $archiveName = "forge-cli-$script:Target.zip"
    $downloadUrl = if ($Version -eq "latest") {
        "$script:GitHubReleaseUrl/latest/download/$archiveName"
    } else {
        "$script:GitHubReleaseUrl/download/v$Version/$archiveName"
    }

    $tempDir = Join-Path $env:TEMP "forge-install-$(Get-Random)"
    New-Item -ItemType Directory -Path $tempDir -Force | Out-Null

    try {
        Write-Info "Downloading from $downloadUrl..."
        $archivePath = Join-Path $tempDir "forge-cli.zip"
        try {
            Invoke-WebRequest -Uri $downloadUrl -OutFile $archivePath -UseBasicParsing
        } catch {
            Write-Error "Download failed: $($_.Exception.Message)"
            Write-Host ""
            Write-Host "==================================================================" -ForegroundColor Yellow
            Write-Host "MANUAL INSTALLATION REQUIRED" -ForegroundColor Yellow
            Write-Host "==================================================================" -ForegroundColor Yellow
            Write-Host ""
            Write-Host "Both build from source and binary download have failed." -ForegroundColor White
            Write-Host ""
            Write-Host "To complete installation manually:" -ForegroundColor Cyan
            Write-Host "  1. Request a pre-built forge.exe binary from your team" -ForegroundColor White
            Write-Host "  2. Copy it to: $binDir\forge.exe" -ForegroundColor White
            Write-Host "  3. Restart PowerShell" -ForegroundColor White
            Write-Host "  4. Test with: forge --version" -ForegroundColor White
            Write-Host ""
            Write-Host "Configuration has been saved and Python SDK will still be installed." -ForegroundColor DarkGray
            Write-Host ""
            return
        }

        # Check if download is valid before extracting
        $downloadedSize = (Get-Item $archivePath).Length
        if ($downloadedSize -lt 1000) {
            Write-Warning "Downloaded file is suspiciously small ($downloadedSize bytes)"
            Write-Host ""
            Write-Host "The download may have been corrupted by corporate proxy/DLP." -ForegroundColor Yellow
            Write-Host "Manual installation required (see instructions above)." -ForegroundColor Yellow
            return
        }

        # Verify integrity against the release SHA256SUMS manifest before
        # extracting or executing. Fails closed: a missing manifest, a missing
        # entry, or a hash mismatch aborts the install.
        $sumsUrl = if ($Version -eq "latest") {
            "$script:GitHubReleaseUrl/latest/download/SHA256SUMS"
        } else {
            "$script:GitHubReleaseUrl/download/v$Version/SHA256SUMS"
        }
        try {
            $sumsText = (Invoke-WebRequest -Uri $sumsUrl -UseBasicParsing).Content
        } catch {
            throw "Could not fetch SHA256SUMS from $sumsUrl; refusing to install an unverified binary"
        }
        $expected = $null
        foreach ($line in ($sumsText -split "`n")) {
            $parts = $line.Trim() -split '\s+', 2
            if ($parts.Count -eq 2) {
                $name = $parts[1].TrimStart('*')
                if ($name -eq $archiveName) { $expected = $parts[0].ToLower(); break }
            }
        }
        if (-not $expected) {
            throw "No checksum entry for $archiveName in SHA256SUMS; refusing to install an unverified binary"
        }
        $actual = (Get-FileHash -Path $archivePath -Algorithm SHA256).Hash.ToLower()
        if ($actual -ne $expected) {
            throw "Checksum mismatch for $archiveName (expected $expected, got $actual); aborting"
        }
        Write-Success "Verified $archiveName (sha256)"

        Expand-Archive -Path $archivePath -DestinationPath $tempDir -Force

        $binary = Get-ChildItem -Path $tempDir -Filter "forge.exe" -Recurse | Select-Object -First 1
        if (-not $binary) {
            throw "Could not find forge.exe in archive"
        }

        Copy-Item -Path $binary.FullName -Destination (Join-Path $binDir "forge.exe") -Force
        Write-Success "forge-cli installed to $binDir\forge.exe"
    } catch {
        Write-Error "Installation failed: $($_.Exception.Message)"
        Write-Host ""
        Write-Host "Manual installation instructions provided above." -ForegroundColor Yellow
    } finally {
        Remove-Item -Path $tempDir -Recurse -Force -ErrorAction SilentlyContinue
    }
}

function Install-PythonSdk {
    if (-not $script:InstallPy) {
        return
    }

    Write-Info "Installing liteforge-py..."

    # First priority: use pre-built wheel from repo if running from git clone
    # This avoids issues with VDI environments that can't build or download crates
    if ($script:InRepo) {
        $prebuiltWheel = Get-ChildItem -Path (Join-Path $script:RepoRoot "bin") -Filter "liteforge-*-win_amd64.whl" -ErrorAction SilentlyContinue | Select-Object -First 1
        if ($prebuiltWheel) {
            # Check Python version matches the wheel
            $pyVersion = (& python -c "import sys; print(f'cp{sys.version_info.major}{sys.version_info.minor}')" 2>&1) | Out-String
            $pyVersion = $pyVersion.Trim()
            if ($prebuiltWheel.Name -match $pyVersion) {
                Write-Info "Using pre-built wheel from repo..."
                # Capture output to variable to avoid stderr triggering PowerShell error
                $oldEAP = $ErrorActionPreference
                $ErrorActionPreference = "Continue"
                $pipOutput = & python -m pip install --user $prebuiltWheel.FullName --force-reinstall 2>&1
                $pipExitCode = $LASTEXITCODE
                $ErrorActionPreference = $oldEAP
                if ($pipExitCode -eq 0) {
                    Write-Success "liteforge-py installed from pre-built wheel"
                    return
                } else {
                    Write-Warning "Failed to install pre-built wheel:"
                    $pipOutput | Select-Object -Last 5 | ForEach-Object { Write-Output "  $_" }
                    Write-Warning "Falling back to build..."
                }
            } else {
                Write-Warning "Pre-built wheel is for $($prebuiltWheel.Name -replace '.*-(cp\d+)-.*','$1') but you have $pyVersion - will build from source"
            }
        }
    }

    # If running from a git clone, build from source
    $pyDir = Join-Path $script:RepoRoot "crates\liteforge-py"
    if ($script:InRepo -and (Test-Path $pyDir)) {
        $hasMaturin = Get-Command maturin -ErrorAction SilentlyContinue
        if ($hasMaturin) {
            Write-Info "Building liteforge-py from source with maturin..."
            Push-Location $pyDir
            try {
                $oldEAP = $ErrorActionPreference
                $ErrorActionPreference = "Continue"
                $maturinOutput = & maturin build --release 2>&1
                $maturinExit = $LASTEXITCODE
                $ErrorActionPreference = $oldEAP
                if ($maturinExit -eq 0) {
                    $wheel = $null
                    $wheelDirs = @(
                        (Join-Path $script:RepoRoot "target\wheels"),
                        (Join-Path $pyDir "target\wheels")
                    )
                    foreach ($wheelDir in $wheelDirs) {
                        if (Test-Path $wheelDir) {
                            $wheel = Get-ChildItem -Path "$wheelDir\*.whl" -ErrorAction SilentlyContinue | Select-Object -First 1
                            if ($wheel) { break }
                        }
                    }
                    if ($wheel) {
                        $oldEAP = $ErrorActionPreference
                        $ErrorActionPreference = "Continue"
                        & python -m pip install --user $wheel.FullName --force-reinstall 2>&1 | Out-Null
                        $pipExit = $LASTEXITCODE
                        $ErrorActionPreference = $oldEAP
                        if ($pipExit -eq 0) {
                            Write-Success "liteforge-py built and installed"
                            return
                        }
                    }
                }
                Write-Warning "Maturin build failed"
            } finally {
                Pop-Location
            }
        }

        # Fallback: pip install from local path
        Write-Info "Installing liteforge-py from local source..."
        $oldEAP = $ErrorActionPreference
        $ErrorActionPreference = "Continue"
        & python -m pip install --user $pyDir 2>&1 | Out-Null
        $pipExit = $LASTEXITCODE
        $ErrorActionPreference = $oldEAP
        if ($pipExit -eq 0) {
            Write-Success "liteforge-py installed from source"
            return
        } else {
            Write-Warning "Failed to install from local source"
        }
    }

    # Try downloading from releases (requires auth on GHE)
    $wheelUrl = if ($Version -eq "latest") {
        "$script:GitHubReleaseUrl/latest/download/liteforge-py3-none-any.whl"
    } else {
        "$script:GitHubReleaseUrl/download/v$Version/liteforge-$Version-py3-none-any.whl"
    }

    try {
        & python -m pip install --user $wheelUrl
        Write-Success "liteforge-py installed"
    } catch {
        Write-Warning "Could not install from wheel (GHE requires auth), trying git..."
        try {
            & python -m pip install --user "git+https://github.com/seanpoyner/liteforge.git#subdirectory=crates/liteforge-py"
            Write-Success "liteforge-py installed from git"
        } catch {
            Write-Error "Failed to install Python SDK: $_"
        }
    }
}

function Install-NodeSdk {
    if (-not $script:InstallJs) {
        return
    }

    Write-Info "Installing liteforge-js..."

    # If running from a git clone, build from source
    $jsDir = Join-Path $script:RepoRoot "crates\liteforge-js"
    if ($script:InRepo -and (Test-Path $jsDir)) {
        Write-Info "Building liteforge-js from source..."
        Push-Location $jsDir
        try {
            & npm install
            & npm run build
            if ($LASTEXITCODE -ne 0) {
                throw "npm build failed"
            }
            & npm pack
            $tgz = Get-ChildItem -Path "*.tgz" | Select-Object -First 1
            if ($tgz) {
                & npm install -g $tgz.FullName
                Write-Success "liteforge-js built and installed"
                return
            }
        } catch {
            Write-Warning "Failed to build from source: $_"
        } finally {
            Pop-Location
        }
    }

    # Try downloading from releases (requires auth on GHE)
    $pkgUrl = if ($Version -eq "latest") {
        "$script:GitHubReleaseUrl/latest/download/liteforge.tgz"
    } else {
        "$script:GitHubReleaseUrl/download/v$Version/liteforge-$Version.tgz"
    }

    try {
        & npm install -g $pkgUrl
        Write-Success "liteforge-js installed"
    } catch {
        Write-Warning "Could not install from tgz (GHE requires auth), trying git..."
        try {
            & npm install -g "git+https://github.com/seanpoyner/liteforge.git#subdirectory=crates/liteforge-js"
            Write-Success "liteforge-js installed from git"
        } catch {
            Write-Error "Failed to install Node.js SDK: $_"
        }
    }
}

function Install-RustSdk {
    if (-not $script:InstallRs) {
        return
    }

    Write-Info "Rust SDK installation..."
    Write-Host ""
    Write-Output "  Add to your Cargo.toml:"
    Write-Host ""
    Write-Output '    [dependencies]'
    Write-Output '    liteforge = { git = "https://github.com/seanpoyner/liteforge.git" }'
    Write-Host ""

    Write-Success "Rust SDK instructions provided"
}

# ============================================================================
# Configuration File Setup
# ============================================================================

function Write-Configuration {
    Write-Info "Writing configuration files..."

    New-Item -ItemType Directory -Path $ForgeHome -Force | Out-Null

    # Write config.toml
    $configContent = @"
# LiteForge Configuration
# Generated by install.ps1 on $(Get-Date -Format "o")

api_key = "$script:TipApiKey"

[endpoints]
base_url = "$script:TipBaseUrl"
knowledge_url = "$script:TipKnowledgeUrl"
temporal_url = "$script:TipTemporalUrl"

[defaults]
model = "$script:TipDefaultModel"
timeout = 60

[paths]
# Uncomment to override default paths:
# agents_dir = "$ForgeHome\agents"
# skills_dir = "$ForgeHome\skills"
# mcp_dir = "$ForgeHome\mcp"
# tools_dir = "$ForgeHome\tools"
"@

    $configPath = Join-Path $ForgeHome "config.toml"
    Set-Content -Path $configPath -Value $configContent -Encoding UTF8
    Write-Success "Created $configPath"

    # Set environment variables (always write directly — keyring is unreliable across environments)
    [Environment]::SetEnvironmentVariable("LITEFORGE_API_KEY", $script:TipApiKey, "User")
    [Environment]::SetEnvironmentVariable("LITEFORGE_BASE_URL", $script:TipBaseUrl, "User")
    [Environment]::SetEnvironmentVariable("LITEFORGE_DEFAULT_MODEL", $script:TipDefaultModel, "User")

    if ($script:TipKnowledgeUrl) {
        [Environment]::SetEnvironmentVariable("LITEFORGE_KNOWLEDGE_URL", $script:TipKnowledgeUrl, "User")
    }
    if ($script:TipTemporalUrl) {
        [Environment]::SetEnvironmentVariable("LITEFORGE_TEMPORAL_URL", $script:TipTemporalUrl, "User")
    }

    # LiteForge-scoped CA bundle env vars (only when the opt-in bundle exists).
    # These are read by LiteForge and its bindings; the bundle is never added to
    # the Windows certificate store, and cargo's revocation checks are left on.
    $certPath = Join-Path $ForgeHome "certs\ca-bundle.crt"
    if (Test-Path $certPath) {
        [Environment]::SetEnvironmentVariable("LITEFORGE_EXTRA_CA_FILE", $certPath, "User")
        [Environment]::SetEnvironmentVariable("SSL_CERT_FILE", $certPath, "User")
        [Environment]::SetEnvironmentVariable("REQUESTS_CA_BUNDLE", $certPath, "User")
        [Environment]::SetEnvironmentVariable("NODE_EXTRA_CA_CERTS", $certPath, "User")
    }

    Write-Success "Environment variables configured"

    # Create subdirectories
    @("agents", "skills", "mcp", "tools") | ForEach-Object {
        New-Item -ItemType Directory -Path (Join-Path $ForgeHome $_) -Force | Out-Null
    }
    Write-Success "Created LiteForge directories"
}

# ============================================================================
# PATH Setup
# ============================================================================

function Set-PathEnvironment {
    Write-Info "Setting up PATH..."

    $binDir = Join-Path $ForgeHome "bin"
    $currentPath = [Environment]::GetEnvironmentVariable("PATH", "User")

    if ($currentPath -notlike "*$binDir*") {
        if (Read-Confirm "Add $binDir to PATH?" $true) {
            $newPath = "$binDir;$currentPath"
            [Environment]::SetEnvironmentVariable("PATH", $newPath, "User")
            Write-Success "Added $binDir to PATH"
            Write-Warning "Restart PowerShell to apply PATH changes"
        }
    } else {
        Write-Success "PATH already contains $binDir"
    }
}

# ============================================================================
# Verification
# ============================================================================

function Test-Installation {
    Write-Info "Verifying installation..."

    $successCount = 0
    $totalCount = 0

    if ($script:InstallCli) {
        $totalCount++
        $tipExe = Join-Path $ForgeHome "bin\forge.exe"
        if (Test-Path $tipExe) {
            try {
                $version = & $tipExe --version 2>$null
                Write-Success "forge-cli: $version"
                $successCount++
            } catch {
                Write-Success "forge-cli: installed"
                $successCount++
            }
        } else {
            Write-Error "forge-cli: not found"
        }
    }

    if ($script:InstallPy) {
        $totalCount++
        $oldEAP = $ErrorActionPreference
        $ErrorActionPreference = "Continue"
        & python -c "import liteforge" 2>&1 | Out-Null
        $pyExit = $LASTEXITCODE
        $ErrorActionPreference = $oldEAP
        if ($pyExit -eq 0) {
            Write-Success "liteforge-py: installed"
            $successCount++
        } else {
            Write-Error "liteforge-py: import failed"
        }
    }

    if ($script:InstallJs) {
        $totalCount++
        $npmList = & npm list -g liteforge 2>$null
        if ($LASTEXITCODE -eq 0) {
            Write-Success "liteforge-js: installed"
            $successCount++
        } else {
            Write-Error "liteforge-js: not found"
        }
    }

    Write-Host ""
    if ($successCount -eq $totalCount -and $totalCount -gt 0) {
        Write-Success "All components installed successfully!"
        $script:InstallSuccess = $true
    } else {
        Write-Warning "$successCount/$totalCount components installed"
        $script:InstallSuccess = $false
    }
}

# ============================================================================
# Summary
# ============================================================================

function Write-Summary {
    Write-Output ""
    if ($script:InstallSuccess) {
        Write-Output "Installation Complete!"
    } else {
        Write-Output "Installation Finished with Errors"
        Write-Output "Some components failed to install - see messages above."
    }
    Write-Output ""
    Write-Output "  LiteForge Home:    $ForgeHome"
    Write-Output "  Config:      $ForgeHome\config.toml"

    if ($script:InstallCli) {
        $tipExe = Join-Path $ForgeHome "bin\forge.exe"
        if (Test-Path $tipExe) {
            Write-Output "  CLI:         $tipExe"
        } else {
            Write-Output "  CLI:         NOT INSTALLED - $tipExe missing"
        }
    }

    Write-Output ""
    Write-Output "Next steps:"
    Write-Output "  1. Restart PowerShell to apply environment changes"
    Write-Output "  2. Test the CLI: forge --help"
    Write-Output "  3. Try a chat: forge chat `"Hello, world!`""
    Write-Output ""
    Write-Output "Documentation: https://github.com/seanpoyner/liteforge"
    Write-Output ""
}

# ============================================================================
# Main
# ============================================================================

function Main {
    Write-Host ""
    Write-Output "LiteForge Installer"
    Write-Output "Version: $Version"
    Write-Host ""

    Get-Platform
    Test-Prerequisites
    Install-CACertificates
    Get-Configuration
    Test-Credentials
    Select-Components

    Install-Cli
    Install-PythonSdk
    Install-NodeSdk
    Install-RustSdk

    Write-Configuration
    Set-PathEnvironment
    Test-Installation
    Write-Summary
}

# Run main
Main
