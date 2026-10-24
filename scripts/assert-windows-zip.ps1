# Private release-verifier helper. Inspect every member before any extraction.
function Assert-WindowsZipMembers([string]$Archive) {
    Add-Type -AssemblyName System.IO.Compression
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $zip = [IO.Compression.ZipFile]::OpenRead([IO.Path]::GetFullPath($Archive))
    $members = [Collections.Generic.Dictionary[string,bool]]::new([StringComparer]::OrdinalIgnoreCase)
    $roots = [Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
    try {
        foreach ($entry in $zip.Entries) {
            # Compress-Archive uses backslashes on Windows. Validate the slash
            # form, including Windows case aliases and file/directory conflicts.
            $name = $entry.FullName.Replace('\', '/')
            $directory = $name.EndsWith('/')
            if ($directory) { $name = $name.Substring(0, $name.Length - 1) }
            if (-not $name -or $name.StartsWith('/')) { throw 'ZIP contains an unsafe relative path.' }
            foreach ($part in $name.Split('/')) {
                if (-not $part -or $part -eq '.' -or $part -eq '..' -or
                    $part -match '[<>:"|?*\x00-\x1f]' -or $part -match '[. ]$' -or
                    $part -match '^(?i:CON|PRN|AUX|NUL|COM[1-9]|LPT[1-9])(?:\.|$)') {
                    throw 'ZIP contains an unsafe relative path.'
                }
            }
            if ($members.ContainsKey($name)) { throw 'ZIP contains duplicate member paths.' }
            $members.Add($name, $directory)
            $parts = $name.Split('/')
            if ($parts.Count -lt 2 -and -not $directory) { throw 'ZIP requires one containing root directory.' }
            [void]$roots.Add($parts[0])
            $attributes = $entry.ExternalAttributes
            $unixType = ($attributes -shr 16) -band 0xf000
            if (($attributes -band 0x400) -ne 0 -or
                ($unixType -ne 0 -and $unixType -ne 0x8000 -and $unixType -ne 0x4000)) {
                throw 'ZIP contains a link, reparse point or unsupported member type.'
            }
            if (($unixType -eq 0x4000 -and -not $directory) -or
                ($unixType -eq 0x8000 -and $directory)) { throw 'ZIP member type disagrees with its path.' }
        }
        if ($roots.Count -ne 1) { throw 'ZIP requires exactly one root directory.' }
        foreach ($name in $members.Keys) {
            $parts = $name.Split('/')
            for ($count = 1; $count -lt $parts.Count; $count++) {
                $parent = ($parts[0..($count - 1)] -join '/')
                if ($members.ContainsKey($parent) -and -not $members[$parent]) {
                    throw 'ZIP contains conflicting file/directory member paths.'
                }
            }
        }
        return @($roots)[0]
    } finally { $zip.Dispose() }
}
