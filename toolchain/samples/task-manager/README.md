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
