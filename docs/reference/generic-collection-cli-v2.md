# Collection package commands and native executables

Use `--experimental-collections` together with `--experimental-generics` on a root
package directory. Standalone `.svr` inputs and duplicate collection flags refuse.
Default commands and generic commands without the new flag retain their previous
profiles.

```sh
svr lock --experimental-generics ./my-app
svr check --experimental-generics --experimental-collections --locked ./my-app
svr run --experimental-generics --experimental-collections --locked ./my-app
svr test --experimental-generics --experimental-collections --locked --format json ./my-app
svr build --experimental-generics --experimental-collections --locked --emit exe --output ./my-app-native ./my-app
./my-app-native
```

On Windows, output names must end in `.exe`; actual Windows launch qualification
requires its own host evidence. Linux execution is tested. Native building needs
the existing verified local Rust runtime source/toolchain; running the generated
executable does not need Node, Cargo, `svr`, or the original project.

Check/Run/Build use the explicit collection package compiler, including registered
module/dependency sources, sealed standard types and shared finite compiler/VM
budgets. Test uses `test_generic_collection_package`: every original body and each
selected specialization is preflighted before any VM starts. Instance reservation
covers both passes; each test has fresh policy/state, root-only discovery and the
existing schema-one reporting. Runtime failures continue to later tests.

Locks use the existing immutable source/topology capture. `lock` needs no new
semantic flag. Locked check/run/test/build reject stale or invalid graph sources
before execution/output publication. JSON Check diagnostics and Test reports
retain their existing schemas; package semantic symbol export remains gated.

Native builds embed exact [word-1023 collection package images](generic-collection-native-images-v2.md).
The generated runner selects only that exact profile for collection images; word
511 continues through the ordinary decoder and unknown words refuse. Collection
source is rechecked in memory at startup. Single-entry collection packages also
use this profile rather than the older single-file native environment. Existing
output refusal/staging/runtime-source custody protections remain active.

The command entry must be parameterless Unit `main`. Generated generic package
executables reject arguments, retain finite host-free runtime policy and grant no
filesystem/network/process access. Source-backed VM execution is not an optimized
machine-code backend or process isolation guarantee.

Integration tests execute all collection operations, imported generic record
ownership, source Check/Run/Test and locked native Build. Generated collection
programs run after project deletion in an empty environment with nonexistent PATH.
Separate tests cover single-entry packages, stale-lock output refusal, duplicate
flags, standalone refusal, test failure continuation and all-root preflight.

Broader host/backend/resource qualification and the remaining D1/H1/H2/E2/Q1
requirements remain open; this does not close G13.

The independently runnable [collection consumer](../../examples/intrinsic-collections/README.md)
contains the source used by the dependency-graph native launch test. Its lock is
generated explicitly; Check/Run/Test do not create one.
