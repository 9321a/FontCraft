# FontCraft for Windows: builds it for x64 and for ARM64 and makes one installer that installs the
# right one on each PC, target\dist\FontCraft-<version>-Windows-Setup.exe, and a portable copy with
# both programs, target\dist\FontCraft-<version>-Windows-Portable.zip.
#
#   powershell -ExecutionPolicy Bypass -File packaging\windows\build-installer.ps1           builds first
#   powershell -ExecutionPolicy Bypass -File packaging\windows\build-installer.ps1 -NoBuild  uses the programs already built
#
# Needs Rust, Visual Studio's C++ build tools with the ARM64 component («MSVC … ARM64/ARM64EC build
# tools») and Inno Setup 6.3 or later, whatever folder its version uses («Inno Setup 6», «Inno
# Setup 7»…); if it's missing, the script installs it with winget or Chocolatey, or else from its
# makers' releases on GitHub (jrsoftware/issrc). This file is saved with a byte-order mark, so
# Windows PowerShell 5 reads its accents right.
param([switch]$NoBuild)
$ErrorActionPreference = 'Stop'
$App = (Resolve-Path "$PSScriptRoot\..\..").Path
Set-Location $App
$Version = ([regex]::Match((Get-Content Cargo.toml -Raw), '(?m)^version = "([^"]+)"')).Groups[1].Value
$Targets = 'x86_64-pc-windows-msvc', 'aarch64-pc-windows-msvc'

if (-not $NoBuild) {
    foreach ($t in $Targets) {
        rustup target add $t | Out-Null
        cargo build --release --target $t
        if ($LASTEXITCODE -ne 0) { throw "No se pudo compilar para $t" }
    }
}
foreach ($t in $Targets) {
    if (-not (Test-Path "target\$t\release\FontCraft.exe")) { throw "Falta target\$t\release\FontCraft.exe: compílalo primero (sin -NoBuild)." }
}

# Both programs carry Microsoft's C runtime inside (.cargo/config.toml: +crt-static). One that asks
# for its DLLs (VCRUNTIME140.dll...) doesn't start on a PC without the Visual C++ Redistributable,
# as many ARM64 PCs are: «no se encontró VCRUNTIME140.dll».
. (Join-Path $PSScriptRoot 'pe-imports.ps1')
foreach ($t in $Targets) {
    $exe = Join-Path $App "target\$t\release\FontCraft.exe"
    $needs = @(Get-VcRuntimeImports $exe)
    if ($needs.Count -gt 0) { throw "$exe pide $($needs -join ', '), que muchos PC (sobre todo ARM64) no tienen: compílalo con el runtime de C dentro (.cargo/config.toml)." }
    Write-Host "${t}: no necesita el Visual C++ Redistributable."
}

# Inno Setup's compiler (ISCC.exe), whatever version installed it and wherever: «Inno Setup 6»,
# «Inno Setup 7» or any later folder, found on the PATH, where its installer recorded it in Windows,
# and in the program folders. The newest one from 6.3 on is used (6.3 is the first that knows ARM64
# PCs: x64compatible, arm64).
#
# Its version comes from where Inno Setup's installer recorded it in Windows (DisplayVersion in its
# «Inno Setup 6_is1» key): ISCC.exe's own file information says 0.0.0.0 (Inno Setup 6.7), so a
# version of 0.0.0.0 means «not said», never «too old».
$MinVersion = [version]'6.3'
# Where this script installs it itself when nothing else can (a folder of the user's).
$OwnInno = Join-Path $env:LOCALAPPDATA 'Programs\Inno Setup (FontCraft)'
$Looked = New-Object System.Collections.Generic.List[string]

# A version written as text («6.7.3», «6.7.3.0»), or $null when it isn't there or is all zeros.
function ConvertTo-Version([string]$Text) {
    $m = [regex]::Match([string]$Text, '\d+(\.\d+){1,3}')
    if (-not $m.Success) { return $null }
    $v = [version]$m.Value
    if ($v -eq [version]'0.0' -or $v -eq [version]'0.0.0' -or $v -eq [version]'0.0.0.0') { return $null }
    return $v
}

function Get-FileVersion([string]$Path) {
    try {
        $info = (Get-Item $Path).VersionInfo
        foreach ($text in @($info.ProductVersion, $info.FileVersion)) {
            $v = ConvertTo-Version $text
            if ($v) { return $v }
        }
    } catch {}
    return $null
}

# The same file, written alike (folder case and trailing backslashes aside).
function Get-PathKey([string]$Path) {
    try { return [System.IO.Path]::GetFullPath($Path).TrimEnd('\').ToLowerInvariant() } catch { return $Path.ToLowerInvariant() }
}

function Find-Iscc {
    $found = New-Object System.Collections.Generic.List[string]
    # The versions Inno Setup's installer recorded, by the ISCC.exe of each installation.
    $recorded = @{}
    $cmd = Get-Command ISCC.exe -ErrorAction SilentlyContinue
    if ($cmd) { $found.Add($cmd.Source) }
    # Where Inno Setup's own installer recorded it (its key is «Inno Setup 6_is1», «Inno Setup 7_is1»...).
    $keys = 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall',
            'HKLM:\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall',
            'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall'
    foreach ($k in $keys) {
        if (-not $Looked.Contains("$k\Inno Setup*")) { $Looked.Add("$k\Inno Setup*") }
        Get-ChildItem $k -ErrorAction SilentlyContinue | Where-Object { $_.PSChildName -like 'Inno Setup*' } | ForEach-Object {
            $item = Get-ItemProperty $_.PSPath -ErrorAction SilentlyContinue
            if ($item -and $item.InstallLocation) {
                $exe = Join-Path $item.InstallLocation 'ISCC.exe'
                $found.Add($exe)
                $v = ConvertTo-Version $item.DisplayVersion
                if ($v) { $recorded[(Get-PathKey $exe)] = $v }
            }
        }
    }
    # Every «Inno Setup…» folder of the program folders (ISCC.exe in it or a level below).
    $roots = @(${env:ProgramFiles(x86)}, $env:ProgramFiles, $env:ProgramW6432, (Join-Path $env:LOCALAPPDATA 'Programs')) | Where-Object { $_ } | Select-Object -Unique
    foreach ($r in $roots) {
        if (-not $Looked.Contains("$r\Inno Setup*")) { $Looked.Add("$r\Inno Setup*") }
        Get-ChildItem -Path $r -Directory -Filter 'Inno Setup*' -ErrorAction SilentlyContinue | ForEach-Object {
            Get-ChildItem -Path $_.FullName -Filter 'ISCC.exe' -Recurse -Depth 1 -ErrorAction SilentlyContinue | ForEach-Object { $found.Add($_.FullName) }
        }
    }
    # The newest of those whose version is known; one whose version isn't said (a shortcut to it
    # such as Chocolatey's, or a build that doesn't write it) only when there's nothing else.
    $best = $null
    $bestVersion = $null
    $unknown = $null
    $seen = @{}
    foreach ($c in $found) {
        $key = Get-PathKey $c
        if ($seen.ContainsKey($key) -or -not (Test-Path $c)) { continue }
        $seen[$key] = $true
        $v = $recorded[$key]
        if (-not $v) { $v = Get-FileVersion $c }
        if (-not $v) {
            Write-Host "Inno Setup (versión sin indicar) en $c"
            if (-not $unknown -or $c -notlike '*\chocolatey\bin\*') { $unknown = $c }
            continue
        }
        if ($v -lt $MinVersion) {
            Write-Host "Inno Setup $v en $c es anterior a la 6.3: no sirve."
            continue
        }
        Write-Host "Inno Setup $v en $c"
        if (-not $best -or $v -gt $bestVersion) {
            $best = $c
            $bestVersion = $v
        }
    }
    if (-not $best -and $unknown) { $best = $unknown }
    if ($best) { Write-Host "Se usa: $best" }
    return $best
}

# Installs Inno Setup: with winget or Chocolatey when there are, or else straight from its makers'
# releases on GitHub (jrsoftware/issrc) into a folder of its own (jrsoftware.org's own download
# links have answered 404).
function Install-InnoSetup {
    if (Get-Command winget -ErrorAction SilentlyContinue) {
        Write-Host 'Instalando Inno Setup con winget...'
        winget install --id JRSoftware.InnoSetup --exact --silent --accept-package-agreements --accept-source-agreements | Out-Host
        if (Find-Iscc) { return }
    }
    if (Get-Command choco -ErrorAction SilentlyContinue) {
        Write-Host 'Instalando Inno Setup con Chocolatey...'
        choco upgrade innosetup -y --no-progress | Out-Host
        if (Find-Iscc) { return }
    }
    $setup = Join-Path $env:TEMP 'innosetup-instalador.exe'
    try {
        [Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12
        # The latest version its makers published; if GitHub doesn't answer, the one known to work.
        $url = $null
        try {
            $release = Invoke-RestMethod -Uri 'https://api.github.com/repos/jrsoftware/issrc/releases/latest' -Headers @{ 'User-Agent' = 'FontCraft-build' }
            $url = ($release.assets | Where-Object { $_.name -match '^innosetup-[\d.]+\.exe$' } | Select-Object -First 1).browser_download_url
        } catch {
            Write-Warning "No se pudo preguntar a GitHub por la última versión de Inno Setup: $($_.Exception.Message)"
        }
        if (-not $url) { $url = 'https://github.com/jrsoftware/issrc/releases/download/is-6_7_3/innosetup-6.7.3.exe' }
        Write-Host "Descargando Inno Setup: $url"
        Invoke-WebRequest -Uri $url -OutFile $setup -UseBasicParsing
        # A signed installer whose signature doesn't match was changed after it was signed.
        $sig = Get-AuthenticodeSignature $setup
        Write-Host "Firma del instalador de Inno Setup: $($sig.Status) $($sig.SignerCertificate.Subject)"
        if ($sig.Status -ne 'Valid' -and $sig.Status -ne 'NotSigned') { throw "la firma del instalador descargado no es válida ($($sig.Status))" }
        $p = Start-Process -FilePath $setup -ArgumentList '/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART', '/SP-', '/CURRENTUSER', "/DIR=`"$OwnInno`"" -Wait -PassThru
        if ($p.ExitCode -ne 0) { Write-Warning "El instalador de Inno Setup terminó con el código $($p.ExitCode)." }
    } catch {
        Write-Warning "No se pudo descargar o instalar Inno Setup: $($_.Exception.Message)"
    }
}

$Iscc = Find-Iscc
if (-not $Iscc) {
    Install-InnoSetup
    $Iscc = Find-Iscc
    if (-not $Iscc) {
        throw "Falta Inno Setup 6.3 o posterior (se buscó en el PATH y en: $($Looked -join '; ')). Descárgalo de https://jrsoftware.org/isdl.php"
    }
}

New-Item -ItemType Directory -Force (Join-Path $App 'target\dist') | Out-Null
& $Iscc "/DAppVersion=$Version" (Join-Path $App 'packaging\windows\fontcraft.iss')
if ($LASTEXITCODE -ne 0) { throw 'Inno Setup no pudo crear el instalador.' }
$Setup = Join-Path $App "target\dist\FontCraft-$Version-Windows-Setup.exe"
Write-Host "Listo: $Setup ($([math]::Round((Get-Item $Setup).Length / 1MB, 1)) MB)"

# The portable copy: both programs in one folder with portable.txt, so FontCraft keeps its settings,
# libraries and downloads in FontCraftData beside them (a USB stick); FontCraft.exe for Intel and
# AMD PCs, FontCraft-ARM64.exe for ARM ones.
$Portable = Join-Path $App 'target\dist\windows\FontCraft'
if (Test-Path $Portable) { Remove-Item -Recurse -Force $Portable }
New-Item -ItemType Directory -Force $Portable | Out-Null
Copy-Item (Join-Path $App 'target\x86_64-pc-windows-msvc\release\FontCraft.exe') (Join-Path $Portable 'FontCraft.exe')
Copy-Item (Join-Path $App 'target\aarch64-pc-windows-msvc\release\FontCraft.exe') (Join-Path $Portable 'FontCraft-ARM64.exe')
Copy-Item (Join-Path $App 'assets\icons\fontcraft.ico'), (Join-Path $App 'assets\icons\fontcraft-library.ico'), (Join-Path $App 'packaging\windows\LEEME-PORTABLE.txt') $Portable
Set-Content -Path (Join-Path $Portable 'portable.txt') -Value 'FontCraft portable'
$Zip = Join-Path $App "target\dist\FontCraft-$Version-Windows-Portable.zip"
if (Test-Path $Zip) { Remove-Item -Force $Zip }
Compress-Archive -Path (Join-Path $Portable '*') -DestinationPath $Zip
Write-Host "Listo: $Zip ($([math]::Round((Get-Item $Zip).Length / 1MB, 1)) MB)"
