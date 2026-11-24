# Installer upgrade qualification (G05)

Status: **partial**. Portable transaction fixtures pass on Linux PowerShell;
actual Windows installation and clean-machine qualification remain pending.
G04 native UI qualification is deferred at the user's request. Neither task
blocks continuing platform-independent contributor work.

## Evidence available without Windows

Run from the repository root with PowerShell 7:

```powershell
pwsh -NoProfile -File scripts/test-install-transaction.ps1 -Report target/installer/transaction-verification.json
```

The optional JSON report records host/PowerShell versions, exact test/helper
SHA-256 hashes, completed assertions and failure information. Fixture compiler
files contain text, not executable compilers. The test covers fresh installation,
no-overwrite, an exclusive prefix lease, modified/foreign files, candidate
tampering, activation failure restoration, retained snapshots, receipt mismatch,
explicit rollback, missing-prefix recovery and linked-directory rejection.
It removes only its own unique temporary fixture directory. Its report explicitly
sets real compiler execution and clean Windows qualification to false, even when
the fixture suite runs on Windows. CI retains this report on success or failure.

## Remaining Windows procedure

Use a disposable clean Windows machine and an isolated test account. Retain
logs before resetting it. Do not use a production installation/profile for these
checks. Follow the [installer commands](../toolchain/README.md) and preserve all
reported backup/receipt paths until recovery has been verified.

1. Record Windows edition/build/architecture, Windows PowerShell version,
   source revision and dirty-state status, old/new package versions and archive
   SHA-256 values. Use two distinct real candidates; retain their compiler hashes
   and `svr --version` output. A checksum alone does not authenticate a publisher.
2. Run the transaction fixture suite under Windows PowerShell 5.1 with `-Report`.
   Retain the JSON and console log, including any failure. This exercises injected
   activation failure without changing a real compiler binary.
3. Run `scripts/test-windows-toolchain.ps1 -Package <candidate.zip>` from a checkout
   matching the candidate, retaining output and its exit code. It covers package
   integrity, real compiler installation, no-overwrite, foreign-file preservation,
   same-candidate replacement/rollback, outside-state preservation, optional
   developer tools and restricted-PATH execution. Same-candidate replacement
   does not establish distinct-version compatibility. This suite uses Node for
   optional tools and is not by itself a clean-machine result.
4. Install the old package into a spaced prefix using `-NoPersistPath`. Save
   hashes of a test project/profile outside that prefix and the exact user PATH
   value (including whether it is absent). Add a foreign file inside the prefix;
   verify candidate `-Upgrade` refuses and preserves every file, then remove only
   that test file. Upgrade with the new package, confirm its version/hash, and
   retain the reported old backup and target-bound receipt.
5. Use the explicit `rollback.ps1 -Prefix <prefix> -Backup <reported-backup>`.
   Verify the old version/hash returns, the displaced new candidate remains,
   project/profile hashes match and user PATH is unchanged. Test missing-prefix
   recovery only within the disposable fixture; never guess a backup location.
6. Separately test default PATH persistence in the isolated account. Retain the
   original raw PATH securely; verify unrelated text survives, a new shell
   resolves the intended compiler, repeated upgrade does not add duplicates,
   and rollback preserves PATH. Restore the account/VM after collecting evidence.
7. On a fresh VM without Rust/Cargo/Node on PATH, verify the installed compiler's
   `--version`, `doctor`, source check and interpreter execution. Test optional
   developer tools separately with supported Node installed. Native executable
   building requires its documented toolchain and is a separate gate.

For each step record passed/failed/not-run, actual exit codes and artifact paths.
If application control blocks execution, retain the block and leave execution
unqualified. Do not turn a skipped check into a pass. Complete G05 only after
distinct-version, PATH/profile and clean-machine evidence has been reviewed.
