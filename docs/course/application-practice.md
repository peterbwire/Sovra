# Practice with real applications

Connect process input, records, persistence and testing through existing Sovra
applications. Their business logic uses general language features.

## Task Manager CLI

With the default installation, copy the bundled source into a new writable directory:

```powershell
New-Item -ItemType Directory .\task-practice
Copy-Item "$env:LOCALAPPDATA\Programs\Sovra\samples\task-manager\main.svr" .\task-practice\main.svr
svr check .\task-practice\main.svr
svr run .\task-practice\main.svr -- add .\task-practice\tasks.txt "Learn Sovra"
svr run .\task-practice\main.svr -- list .\task-practice\tasks.txt
svr run .\task-practice\main.svr -- complete .\task-practice\tasks.txt "Learn Sovra"
svr run .\task-practice\main.svr -- list .\task-practice\tasks.txt
```

Completion selects the exact task title, not its displayed number. The second listing shows the completed task. Data paths resolve from the shell's
working directory. These commands modify real files: use your own practice
directory. Application failures set nonzero status; inspect `$LASTEXITCODE`
immediately after the command in PowerShell.

Read the source to trace argument dispatch, stored-record decoding, task lookup
and replacement writes. The browser GUI uses a Node host; it is not bundled as
a Node-free desktop application. The adjacent sample README gives the full CLI contract.

## Reusable package tests

From the repository root with a current installed or locally built compiler:

```powershell
svr test .\dogfood\sovra-notes\tests
svr test .\dogfood\sovra-budget\tests
```

Each package has three tests consuming its declared local library dependency.
Notes checks keys and record decoding; Budget checks totals, utilization and
invalid inputs. Inspect the manifests and tests, then add an edge-case test with
ordinary branches and `std::set_exit_code(1)`. Pass the directory to enable
package dependency linking.

These packages are repository examples, not included in the minimal installed
sample directory. The broader dogfood runner uses Node to orchestrate tests of
interpreter and JavaScript behavior.

Every new dogfood application must expose a reusable package or host adapter,
declare its contract in `dogfood/suite.json`, and test both app and capability
through real Sovra execution. Business-specific compiler primitives are not needed.
