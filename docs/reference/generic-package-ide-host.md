# Generic package IDE host acceptance

The documented experimental Linux G13 profile is qualified with actual VS Code
1.106.0 (headless Electron), Node.js 24 and the built Sovra compiler. The package
fixture tests closed and unsaved dependency diagnostics, invalid-graph evidence
retirement, Unicode/CRLF destination relocation, function/type/alias/local/field
hover and definitions, field outlines, formatting without saving, dirty-manifest
refusal, related-only saves, real package Check/Run targets/cwd and server restart.
An unrelated dirty source remains unsaved by these tasks.

The actual host revealed and now tests the extension's Field symbol mapping.
VS Code may normalize a compiler formatting edit into several edits; acceptance
checks the resulting text and unchanged source buffer rather than their count.

Install the official `@vscode/test-electron@2.5.2` development test runner outside
the extension. Supply its module directory through `NODE_PATH` when needed. A
preinstalled official VS Code executable can avoid a download by the runner:

```sh
NODE_PATH=/path/to/node_modules \
SOVRA_VSCODE_HEADLESS=1 \
SOVRA_VSCODE_EXECUTABLE=/path/to/VSCode/code \
SOVRA_TEST_GENERIC_PACKAGE_WORKSPACE=1 \
node scripts/test-vscode-extension.mjs
```

The pinned default host is 1.106.0. A supplied executable also works for restricted
workspace tests. `SOVRA_TEST_GENERIC_PACKAGE_WORKSPACE`, `SOVRA_TEST_GENERICS`,
`SOVRA_TEST_V2`, `SOVRA_TEST_PACKAGE_WORKSPACE` and `SOVRA_TEST_UNTRUSTED` select
mutually exclusive fixtures. With no mode selected, ordinary standalone acceptance
runs. The qualification harness's `--vscode-host` option now runs all six modes
and records their explicit environment overrides:

```sh
python3 scripts/qualify-g13.py --all-rust --node --vscode-host
```

Retained evidence is in `target/checked-evidence/g13-vscode-host/verification.json`
and adjacent logs/host-generated reports. The preceding full Rust/Node run is
preserved in `target/checked-evidence/g13-package-navigation-consolidated/`.
The host checkpoint changes IDE mapping, descriptions and test infrastructure;
it does not change Rust compiler/runtime source or claim a second full Rust run.
The host report records its own unchanged source fingerprint and compiler hash.

All six Linux host modes and all 16 Node tooling regression scripts pass.
G13's documented experimental Linux scope is complete. Windows/macOS runtime and
IDE acceptance, browser/mobile, default-profile expansion, optimized native code,
HTTP/networking and release publication remain outside this closure.
