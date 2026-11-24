# Checked package local navigation

With `sovra.experimentalGenericPackageDiagnostics` enabled, successful package
checks supply hover and Go to Definition for parameters, immutable and mutable
locals, and match payload bindings. Reads and writes use retained checked binding
identities. Shadowed names and identical names in different functions retain
separate destinations. Hover displays the checked type, including symbolic types.

The compiler emits bounded local and reference tables with a source file and
owning function for every entry. The language server verifies fingerprints,
source names, UTF-8 ranges, function boundaries and reference ownership before
mapping byte offsets to UTF-16 positions. It remaps local indices separately for
each open document. Quota exhaustion discards the complete optional projection
without changing check success. No execution, source writes or lock authority is
introduced. Existing standalone navigation remains available.

[Package field navigation](generic-package-field-navigation.md) covers field reads
and constructor labels. Consolidated Linux and actual VS Code host qualification now pass; see
[host acceptance](generic-package-ide-host.md). G13 is complete for the documented
experimental Linux scope; Windows runtime acceptance remains separate.
