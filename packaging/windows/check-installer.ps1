# Installs FontCraft with its installer without questions (for this user, in a test folder), checks
# that the program installed is the one made for this PC's processor, that it needs no Visual C++
# Redistributable and that it starts, then uninstalls it.
#
#   powershell -ExecutionPolicy Bypass -File packaging\windows\check-installer.ps1 [instalador.exe]
param([string]$Setup)
$ErrorActionPreference = 'Stop'
$App = (Resolve-Path "$PSScriptRoot\..\..").Path
if (-not $Setup) {
    $Setup = (Get-ChildItem "$App\target\dist\FontCraft-*-Windows-Setup.exe" | Sort-Object LastWriteTime -Descending | Select-Object -First 1).FullName
}
$Dir = Join-Path $env:TEMP 'FontCraft-prueba'
$p = Start-Process -FilePath $Setup -ArgumentList '/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART', '/CURRENTUSER', "/DIR=`"$Dir`"" -Wait -PassThru
if ($p.ExitCode -ne 0) { throw "El instalador terminó con el código $($p.ExitCode)." }
$Exe = Join-Path $Dir 'FontCraft.exe'
if (-not (Test-Path $Exe)) { throw "No se instaló $Exe." }

# Which processor the installed program is for: the Machine field of its PE header.
$bytes = [System.IO.File]::ReadAllBytes($Exe)
$pe = [BitConverter]::ToInt32($bytes, 0x3C)
$machine = [BitConverter]::ToUInt16($bytes, $pe + 4)
$built = switch ($machine) { 0x8664 { 'x64' } 0xAA64 { 'ARM64' } default { '0x{0:X}' -f $machine } }
$native = if ([System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture -eq 'Arm64') { 'ARM64' } else { 'x64' }
Write-Host "Este PC: $native. Programa instalado: $built."
if ($built -ne $native) { throw 'El instalador eligió el programa de otro procesador.' }

# It starts on PCs without the Visual C++ Redistributable (many ARM64 ones): it doesn't ask for
# VCRUNTIME140.dll or the other DLLs of that package (this machine has them, so starting it here
# wouldn't tell).
. (Join-Path $PSScriptRoot 'pe-imports.ps1')
$needs = @(Get-VcRuntimeImports $Exe)
if ($needs.Count -gt 0) { throw "FontCraft pide $($needs -join ', '): no arrancaría en un PC sin el Visual C++ Redistributable." }
Write-Host 'No necesita el Visual C++ Redistributable.'

# FontCraft is a window program: Start-Process waits for it and gives it a file for what it prints.
$Out = Join-Path $env:TEMP 'fontcraft-version.txt'
$v = Start-Process -FilePath $Exe -ArgumentList '--version' -Wait -PassThru -NoNewWindow -RedirectStandardOutput $Out
$Text = (Get-Content $Out -Raw).Trim()
if ($v.ExitCode -ne 0 -or $Text -notlike 'FontCraft *') { throw "FontCraft no arrancó (código $($v.ExitCode), salida «$Text»)." }
Write-Host "Arranca: $Text"

# The uninstaller copies itself elsewhere and carries on there, so wait for the program to go.
Start-Process -FilePath (Join-Path $Dir 'unins000.exe') -ArgumentList '/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART' -Wait
for ($i = 0; $i -lt 30 -and (Test-Path $Exe); $i++) { Start-Sleep -Seconds 1 }
if (Test-Path $Exe) { throw 'El desinstalador no quitó FontCraft.' }
Write-Host 'Se desinstala bien. Todo correcto.'
