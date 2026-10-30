$ErrorActionPreference = 'Stop'
$installer = Join-Path $PSScriptRoot '..\dist\EmOrbit-Setup.exe'
$builtExe = Join-Path $PSScriptRoot '..\target\release\emorbit.exe'
$installDir = Join-Path $env:RUNNER_TEMP 'EmOrbitInstallTest'
$programsDir = [Environment]::GetFolderPath([Environment+SpecialFolder]::Programs)
$shortcutDir = Join-Path $programsDir 'EmOrbit'
$installLog = Join-Path $env:RUNNER_TEMP 'EmOrbit-Install.log'
$registryKey = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\Noctistt.EmOrbit_is1'

function Install-EmOrbit {
    $arguments = @('/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART', '/SP-',
        "/DIR=`"$installDir`"", "/LOG=`"$installLog`"", '/TASKS=""')
    $process = Start-Process -FilePath $installer -ArgumentList $arguments -Wait -PassThru
    if ($process.ExitCode -ne 0) { throw "Installer failed: $($process.ExitCode)" }
    $installedExe = Join-Path $installDir 'emorbit.exe'
    if (-not (Test-Path $installedExe)) { throw 'Installed executable missing' }
    if ((Get-FileHash $installedExe).Hash -ne (Get-FileHash $builtExe).Hash) {
        throw 'Installed executable differs from the release build'
    }
    if (-not (Test-Path (Join-Path $shortcutDir 'EmOrbit.lnk'))) {
        Get-Content $installLog -Tail 100 | Write-Output
        throw "Start menu shortcut missing at $shortcutDir"
    }
    if (-not (Test-Path $registryKey)) { throw 'Uninstall registration missing' }
}

$cleanupNeeded = $true
try {
    Install-EmOrbit
    # Reinstallation must update the same installation and preserve its shortcut.
    Install-EmOrbit
    $uninstaller = Join-Path $installDir 'unins000.exe'
    $process = Start-Process -FilePath $uninstaller -ArgumentList '/VERYSILENT /SUPPRESSMSGBOXES /NORESTART' -Wait -PassThru
    if ($process.ExitCode -ne 0) { throw "Uninstaller failed: $($process.ExitCode)" }
    # Inno's temporary uninstaller can finish removing files after its parent exits.
    for ($attempt = 0; $attempt -lt 50 -and (Test-Path (Join-Path $installDir 'emorbit.exe')); $attempt++) {
        Start-Sleep -Milliseconds 100
    }
    if (Test-Path (Join-Path $installDir 'emorbit.exe')) { throw 'Uninstall left the executable behind' }
    if (Test-Path (Join-Path $shortcutDir 'EmOrbit.lnk')) { throw 'Uninstall left the shortcut behind' }
    if (Test-Path $registryKey) { throw 'Uninstall registration was not removed' }
    $cleanupNeeded = $false
    Write-Output 'Installer, reinstallation, shortcuts, file integrity, and uninstall checks passed.'
} finally {
    if ($cleanupNeeded -and (Test-Path (Join-Path $installDir 'unins000.exe'))) {
        Start-Process -FilePath (Join-Path $installDir 'unins000.exe') -ArgumentList '/VERYSILENT /SUPPRESSMSGBOXES /NORESTART' -Wait
    }
}
