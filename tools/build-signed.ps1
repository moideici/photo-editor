$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
$signingDirectory = Join-Path $env:USERPROFILE '.tauri'
$signingKey = Join-Path $signingDirectory 'prisme-updater.key'
$passwordFile = Join-Path $signingDirectory 'prisme-updater.password.dpapi'
if (!(Test-Path -LiteralPath $signingKey) -or !(Test-Path -LiteralPath $passwordFile)) { throw 'Local signing material unavailable. See DISTRIBUTION.md.' }
Add-Type -AssemblyName System.Security
try {
    $env:TAURI_SIGNING_PRIVATE_KEY = $signingKey
    $passwordBytes = [Security.Cryptography.ProtectedData]::Unprotect([IO.File]::ReadAllBytes($passwordFile),$null,[Security.Cryptography.DataProtectionScope]::CurrentUser)
    $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = [Text.Encoding]::UTF8.GetString($passwordBytes)
    [Array]::Clear($passwordBytes,0,$passwordBytes.Length)
    Push-Location $projectRoot
    try {
        python tools/check-release.py
        if ($LASTEXITCODE -ne 0) { throw 'Invalid release metadata' }
        & .\node_modules\.bin\tauri.cmd build --bundles nsis,msi
        if ($LASTEXITCODE -ne 0) { throw 'Signed build failed' }
    } finally { Pop-Location }
} finally {
    Remove-Item Env:TAURI_SIGNING_PRIVATE_KEY -ErrorAction SilentlyContinue
    Remove-Item Env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD -ErrorAction SilentlyContinue
}
