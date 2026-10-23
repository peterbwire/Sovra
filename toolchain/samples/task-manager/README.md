# Task Manager CLI sample

`main.svr` is copied unchanged from Sovra's Task Manager dogfood application
when the toolchain ZIP is built. Its logic runs through the installed `svr.exe`.
The sample stores tasks in a named UTF-8 file; use a private directory and one
writer per file. From PowerShell, for example:

```powershell
svr check .\samples\task-manager\main.svr
svr run .\samples\task-manager\main.svr -- add .\tasks.txt "Write tests"
svr run .\samples\task-manager\main.svr -- list .\tasks.txt
```

The browser GUI is a separate Node-hosted adapter and is not bundled here.

This sample requires a compiler with approved ADR 0034 `std::write_text_if`.
Current programs reject stale/busy saves without automatically retrying them.
Old applications and external editors must not write the same file concurrently.
A crashed writer can leave `<file>.svr-write-lock`: stop all writers, inspect
the data, then deliberately remove the retained marker before retrying. Never
remove it based only on age. See `docs/adr/0034-conditional-text-file-replacement.md`
in the toolchain for the cooperative scope and recovery policy.
