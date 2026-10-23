# ADR 0033: PowerShell-free private Windows desktop host

Status: **Private implementation direction authorized (2026-10-23)** by the
user's request for sustainable application execution without PowerShell.
This supersedes the PowerShell implementation choice for the replacement
adapter in ADR 0029; the existing adapter remains a compatibility baseline.
It approves no public framework ABI, new Sovra syntax or language semantics.

## Decision and boundary

Compile the platform host and generic renderer into a C#/WPF executable rather
than launching PowerShell scripts. Reuse the already selected Windows toolkit
and private profile/view/event contracts from ADRs 0029 and 0030. Keep business
logic, interface descriptions, events and persistence in the real Sovra program.
No task IDs, task storage format or application rules belong in the renderer.

The initial implementation uses .NET Framework/WPF on compatible Windows x64.
It removes a scripting-shell dependency, not the operating-system UI/runtime
dependency. This does not claim a self-contained native host, cross-platform
widgets or a replacement for the Rust-owned compiler/runtime. Development may
continue using PowerShell packaging scripts; deployed compiled packages contain
no PowerShell runtime scripts and do not launch a PowerShell child.

`Bridge.cs` owns Windows argument encoding, strict bounded UTF-8 process I/O,
timeout/exit handling, private v1/v2 decoding and fresh-view event authorization.
`DesktopWindow.cs` owns native controls, serial background execution, draft/focus/
selection/scroll retention and generic accessibility/keyboard behavior.
`ApplicationProfile.cs` reads the existing profile into a bounded snapshot and
validates its existing schema/identity/command fields. `Entry.cs` opens the
native window or displays a startup diagnostic and returns nonzero status.
`DesktopTheme.cs` maps ordinary/high-contrast presentation to platform brushes;
Windows setting notifications update presentation without invoking application
events. Its system-color tests do not certify actual OS theme transitions or DPI.
Background workers carry data only; completion returns explicitly to the WPF
dispatcher. No shell, generated XAML or evaluated script is involved.

Keep the existing 64 KiB per-stream bound, 256 leaves, 256 groups, eight group
levels, 30-second child timeout, one in-flight event and no automatic mutation
retry. A close request waits for success; an error keeps the window open.
Existing application IDs retain their LocalAppData state paths and Sovra formats.
The existing single-writer limitation is unchanged.

## Incremental delivery and validation

1. Implement the reusable compiled bridge and renderer beside the baseline.
2. Test malformed views, output limits, timeout and UTF-8 failures, real Sovra
   Task Manager events and an unrelated component, plus native-control behavior.
3. Offer internal `-CompiledHost` packaging without changing default packages.
   Verify ZIP integrity, absence of PowerShell scripts, direct EXE window
   ownership, native Sovra persistence and restart behavior under restricted PATH.
4. Retain both adapters' existing CI gates. Broaden parity, screen-reader,
   high-contrast/DPI and clean-Windows qualification before changing defaults.
5. Consider future desktop/mobile adapters separately against the renderer
   boundary. Publishing lifecycle, navigation, cancellation, capability APIs or
   compatibility promises requires approval under AGENTS.md and ADR 0023.

The compiler's interpreter, JavaScript backend, native output and source syntax
are unchanged. The compiled adapter is an implemented private candidate; neither
compilation nor local GUI tests certify release readiness.
