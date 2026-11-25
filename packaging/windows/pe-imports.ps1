# The DLLs a Windows program asks for: its import table and its delay-loaded imports, read from
# the file itself (no Visual Studio tools needed). Used by build-installer.ps1 and
# check-installer.ps1 to make sure FontCraft doesn't need the Visual C++ Redistributable.
#
#   . packaging\windows\pe-imports.ps1
#   Get-PeImports target\aarch64-pc-windows-msvc\release\FontCraft.exe

function Get-PeImports([string]$Path) {
    $b = [System.IO.File]::ReadAllBytes($Path)
    $pe = [BitConverter]::ToInt32($b, 0x3C)
    if ([BitConverter]::ToUInt32($b, $pe) -ne 0x00004550) { throw "$Path no es un programa de Windows." }
    $sections = [BitConverter]::ToUInt16($b, $pe + 6)
    $optSize = [BitConverter]::ToUInt16($b, $pe + 20)
    $opt = $pe + 24
    # The data directories follow the optional header's fields: 96 bytes in PE32, 112 in PE32+.
    $dirs = if ([BitConverter]::ToUInt16($b, $opt) -eq 0x20B) { $opt + 112 } else { $opt + 96 }
    $secTable = $opt + $optSize
    # Where an address of the loaded program is in the file.
    $toOffset = {
        param([uint32]$rva)
        for ($i = 0; $i -lt $sections; $i++) {
            $s = $secTable + 40 * $i
            $va = [BitConverter]::ToUInt32($b, $s + 12)
            $size = [Math]::Max([BitConverter]::ToUInt32($b, $s + 8), [BitConverter]::ToUInt32($b, $s + 16))
            if ($rva -ge $va -and $rva -lt $va + $size) { return [int]($rva - $va + [BitConverter]::ToUInt32($b, $s + 20)) }
        }
        return -1
    }
    $out = New-Object System.Collections.Generic.List[string]
    # Imports (directory 1: 20-byte entries, the DLL's name at +12) and delay-loaded imports
    # (directory 13: 32-byte entries, the name at +4); each list ends with an empty entry.
    foreach ($d in @(@(1, 20, 12), @(13, 32, 4))) {
        $rva = [BitConverter]::ToUInt32($b, $dirs + 8 * $d[0])
        if ($rva -eq 0) { continue }
        $start = & $toOffset $rva
        if ($start -lt 0) { continue }
        for ($e = $start; $e + $d[1] -le $b.Length; $e += $d[1]) {
            $nameRva = [BitConverter]::ToUInt32($b, $e + $d[2])
            if ($nameRva -eq 0) { break }
            $at = & $toOffset $nameRva
            if ($at -lt 0) { continue }
            $end = $at
            while ($end -lt $b.Length -and $b[$end] -ne 0) { $end++ }
            $out.Add([System.Text.Encoding]::ASCII.GetString($b, $at, $end - $at).ToLowerInvariant())
        }
    }
    return $out.ToArray()
}

# The Visual C++ Redistributable's DLLs among a program's imports (the Universal C Runtime is part
# of Windows 10 and 11, so it doesn't count).
function Get-VcRuntimeImports([string]$Path) {
    return @(Get-PeImports $Path | Where-Object { $_ -like 'vcruntime*' -or $_ -like 'msvcp*' -or $_ -like 'concrt*' -or $_ -like 'vccorlib*' })
}
