param([Parameter(Mandatory=$true, Position=0)][string]$Binary)
$ErrorActionPreference = 'Stop'
$path = [IO.Path]::GetFullPath($Binary)
$bytes = [IO.File]::ReadAllBytes($path)
if ($bytes.Length -gt 256MB) { throw 'PE file exceeds audit size limit.' }

function Read-U16([long]$offset) {
    if ($offset -lt 0 -or $offset + 2 -gt $bytes.Length) { throw 'Truncated PE file.' }
    return [BitConverter]::ToUInt16($bytes, [int]$offset)
}
function Read-U32([long]$offset) {
    if ($offset -lt 0 -or $offset + 4 -gt $bytes.Length) { throw 'Truncated PE file.' }
    return [BitConverter]::ToUInt32($bytes, [int]$offset)
}
function Read-Ascii([long]$offset) {
    if ($offset -lt 0 -or $offset -ge $bytes.Length) { throw 'Invalid PE string offset.' }
    $end = $offset
    while ($end -lt $bytes.Length -and $end - $offset -lt 256 -and $bytes[$end] -ne 0) { $end++ }
    if ($end -ge $bytes.Length -or $end - $offset -ge 256) { throw 'Unterminated PE string.' }
    if ($end -eq $offset) { throw 'Empty PE DLL name.' }
    foreach ($index in $offset..($end - 1)) {
        if ($bytes[$index] -lt 32 -or $bytes[$index] -gt 126) { throw 'Invalid PE DLL name.' }
    }
    return [Text.Encoding]::ASCII.GetString($bytes, [int]$offset, [int]($end - $offset))
}
if ((Read-U16 0) -ne 0x5a4d) { throw 'Not an MZ executable.' }
$pe = Read-U32 0x3c
if ((Read-U32 $pe) -ne 0x00004550) { throw 'Invalid PE signature.' }
$machine = Read-U16 ($pe + 4)
$sectionCount = Read-U16 ($pe + 6)
$optionalSize = Read-U16 ($pe + 20)
$optional = $pe + 24
if ($sectionCount -eq 0 -or $sectionCount -gt 96) { throw 'Invalid PE section count.' }
if ($optional + $optionalSize -gt $bytes.Length) { throw 'Truncated PE optional header.' }
$magic = Read-U16 $optional
if ($magic -eq 0x20b) { $directoryStart = $optional + 112; $directoryCount = Read-U32 ($optional + 108) }
elseif ($magic -eq 0x10b) { $directoryStart = $optional + 96; $directoryCount = Read-U32 ($optional + 92) }
else { throw 'Unsupported PE optional header.' }
if ($directoryCount -lt 2 -or $directoryStart + 16 -gt $optional + $optionalSize) {
    throw 'Missing PE import directory.'
}
$sections = @()
$sectionTable = $optional + $optionalSize
for ($index = 0; $index -lt $sectionCount; $index++) {
    $offset = $sectionTable + 40 * $index
    $sections += [pscustomobject]@{
        VirtualSize = Read-U32 ($offset + 8)
        VirtualAddress = Read-U32 ($offset + 12)
        RawSize = Read-U32 ($offset + 16)
        RawPointer = Read-U32 ($offset + 20)
    }
}
function Resolve-Rva([long]$rva, [long]$length) {
    foreach ($section in $sections) {
        $span = [Math]::Max($section.VirtualSize, $section.RawSize)
        if ($rva -ge $section.VirtualAddress -and $rva + $length -le $section.VirtualAddress + $span) {
            $delta = $rva - $section.VirtualAddress
            if ($delta + $length -gt $section.RawSize) { throw 'PE import points into uninitialized section data.' }
            $offset = $section.RawPointer + $delta
            if ($offset + $length -gt $bytes.Length) { throw 'PE import points outside file.' }
            return $offset
        }
    }
    throw 'PE import RVA does not map to a section.'
}
$importsRva = Read-U32 ($directoryStart + 8)
$importsSize = Read-U32 ($directoryStart + 12)
$dlls = @()
if ($importsRva -ne 0) {
    if ($importsSize -lt 20 -or $importsSize -gt 1MB) { throw 'Invalid PE import directory size.' }
    for ($cursor = 0; $cursor + 20 -le $importsSize; $cursor += 20) {
        $descriptor = Resolve-Rva ($importsRva + $cursor) 20
        $originalThunk = Read-U32 $descriptor
        $timestamp = Read-U32 ($descriptor + 4)
        $forwarder = Read-U32 ($descriptor + 8)
        $nameRva = Read-U32 ($descriptor + 12)
        $firstThunk = Read-U32 ($descriptor + 16)
        if ($originalThunk -eq 0 -and $timestamp -eq 0 -and $forwarder -eq 0 -and
            $nameRva -eq 0 -and $firstThunk -eq 0) { break }
        if ($nameRva -eq 0) { throw 'PE import descriptor has no DLL name.' }
        $nameOffset = Resolve-Rva $nameRva 1
        $dlls += Read-Ascii $nameOffset
    }
    if ($cursor + 20 -gt $importsSize) { throw 'PE import descriptors are not terminated.' }
}
$architecture = switch ($machine) {
    0x8664 { 'x86_64' }
    0x14c { 'x86' }
    0xaa64 { 'arm64' }
    default { "unknown-0x$($machine.ToString('x'))" }
}
[pscustomobject]@{
    binary = [IO.Path]::GetFileName($path)
    architecture = $architecture
    imports = @($dlls | Sort-Object -Unique)
    scope = 'direct PE import descriptors only; excludes transitive and dynamic loads'
} | ConvertTo-Json -Depth 4
