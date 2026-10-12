# File Manager browser prototype

Status: **Partial**. This local browser app lists regular files in an explicitly
chosen workspace. Sovra defines the heading, file labels and Preview/Copy/Create
controls using the private `SVR-UI-1` view protocol shared with the Task
Manager prototype. Preview, Copy and empty-file creation execute `main.svr` through the real Sovra
compiler/interpreter or generated JavaScript. It uses the existing 16 MiB
UTF-8 read and atomic replacement APIs; previews are limited to 64 KiB.

```text
cargo build --locked
node dogfood/file-manager/gui/server.mjs --root PATH_TO_PRIVATE_DIRECTORY
node scripts/test-file-manager.mjs
node scripts/test-dogfood.mjs
```

Open the printed loopback URL. The host provides directory enumeration because
Sovra has no directory-reading API yet; Sovra performs file reads, copies and
empty-file creation. Creation rejects an existing name in the browser host,
then calls the approved `std::write_text` API. That check is not an exclusive
filesystem create: a concurrent writer could create the name between the check
and Sovra's replacement write. Use a trusted workspace without competing writers.
Only entries that are regular files at listing time appear. The UI shows 80
files per page, with Sovra-defined Previous and Next controls. The Node host
still enumerates and sorts the whole directory on each request. Forms hold a
bounded server-side listing snapshot
so adding a new alphabetically earlier file cannot redirect an old button to
a different name. If a listed file disappears, its old Preview/Copy form is
rejected. An empty workspace shows the Create control without file actions.
The app is for a
trusted local workspace, not a security sandbox: the directory can change
between listing and an operation. Do not serve a directory another process can
modify maliciously. Existing destinations are replaced by the approved
`std::write_text` behavior. No delete, move, binary files, recursive navigation,
or registry package is claimed. A proposed general filesystem API is tracked
in ADR 0024 before expanding beyond this slice.

The numbered `02-file-processor` application remains intact. This new app does
not renumber the established dogfood roadmap.
