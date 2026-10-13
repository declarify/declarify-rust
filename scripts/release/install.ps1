# Installs the declarify/notes/notes-server binaries from this directory
# (bundled alongside this script in the release .zip) and adds the install
# directory to the current user's PATH.
#
# Override the install location with $env:DECLARIFY_INSTALL_DIR; otherwise
# this uses %LOCALAPPDATA%\Declarify\bin.

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$Dest = $env:DECLARIFY_INSTALL_DIR
if (-not $Dest) {
    $Dest = Join-Path $env:LOCALAPPDATA "Declarify\bin"
}

New-Item -ItemType Directory -Force -Path $Dest | Out-Null

foreach ($bin in "declarify.exe", "notes.exe", "notes-server.exe") {
    Copy-Item (Join-Path $ScriptDir $bin) (Join-Path $Dest $bin) -Force
    Write-Host "installed $(Join-Path $Dest $bin)"
}

$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
$pathEntries = @()
if ($userPath) {
    $pathEntries = $userPath -split ";" | Where-Object { $_ -ne "" }
}

if ($pathEntries -notcontains $Dest) {
    $newPath = if ($userPath) { "$userPath;$Dest" } else { $Dest }
    [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
    Write-Host ""
    Write-Host "Added $Dest to your user PATH. Restart your terminal for it to take effect."
} else {
    Write-Host ""
    Write-Host "$Dest is already on your PATH."
}

Write-Host ""
Write-Host "Done. Try: declarify --help (after restarting your terminal)"
