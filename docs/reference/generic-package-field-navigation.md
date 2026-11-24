# Checked package field navigation

With `sovra.experimentalGenericPackageDiagnostics` enabled, successful checks
supply Go to Definition and hover for user-record field reads and constructor
labels. Field declarations also appear as Field symbols in checked outlines.
Hover shows the original declaration annotation, such as `value:T`, preserving
the record's symbolic parameter names rather than claiming a substituted type.

The destination comes from the checked nominal record identity and field index.
Identically named fields in different records/packages remain distinct. Grouped
and chained reads retain their own receiver identities. Constructor labels use
the checked source-order initializer indices, so reordered initializers still
navigate to the correct declarations. Original tokens locate and verify the
selector/label; spelling does not select a destination. Checked initializer
expression ranges delimit values, including grouped and nested expressions.

Fields share the existing declaration/reference, string, output and work quotas.
Source fingerprints and UTF-8/UTF-16 checks apply to definitions and references.
Exhaustion discards the complete optional metadata batch without invalidating
a successful check. Compiler-injected fields have no invented user destination.
The projection introduces no source writes, execution or lock authority.

Linux compiler/LSP tests cover dependency destinations, same-named fields,
Unicode/CRLF, grouped/chained reads, constructor labels, reordered values and
whole-batch quota refusal. Consolidated compiler/runtime qualification and actual Linux VS Code acceptance
now pass; see [host qualification](generic-package-ide-host.md). G13 is complete
for the documented experimental Linux scope. Additional host/runtime acceptance
remains separate.
