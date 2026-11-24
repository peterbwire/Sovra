# Experimental propagation commands and native images (V2)

The existing `--experimental-generics` selection now admits checked postfix `?`
in source functions through the [complete-source propagation profile](generic-propagation-source-v2.md).
Ordinary commands and ordinary compiler APIs retain their strict syntax.

```sh
svr check --experimental-generics --format json main.svr
svr run --experimental-generics main.svr
svr lock --experimental-generics package
svr check --experimental-generics --locked package
svr run --experimental-generics --locked package
svr build --experimental-generics --locked --emit exe --output application package
```

The same compiler source, propagation, specialization, layout and IR limits apply.
Check validates every symbolic body, including unused functions. Runtime uses the
fixed host-free command policy. Run/build require a parameterless Unit `main`;
handle Option/Result explicitly in `main` and propagate inside compatible helpers.
Compile/runtime diagnostics retain existing E3123/E3125 codes and original-file
locations. Package support retains its existing guarded Linux single-entry,
no-dependency/no-module scope, lock validation and unsupported-host refusals.
This integration does not widen filesystem authority or package capture.

Native images keep magic `SVRG2\0`, version 1, value model 2 and the existing
16-byte header. Feature word **15** selects standard enums, std::Error and
propagation startup checking. Existing feature words **1, 3 and 7** retain their
strict historical profiles. Only these four exact words are accepted; partial or
unknown combinations refuse. Changing a propagation image to an older word does
not make its questions acceptable. Every image rechecks its original UTF-8 source
under finite compiler limits before execution. Header metadata is not authentication.

New experimental CLI builds select feature 15, including zero-question files.
Ordinary `compile_generic_source` encoding still selects 7. No arguments, source
paths or host grants enter the image. Existing source size, exact length, UTF-8,
entry, non-overwrite publication and staging-cleanup checks remain active.
Generated applications are host-native Rust executables containing the Sovra VM
and embedded source; startup repeats compilation. They run without Node.js, the
Sovra compiler executable or original project files. This is not direct Sovra
machine-code lowering.

The [example](../examples/generic-propagation.svr) exercises successful extraction,
Option failure, structured Result failure and different caller success types.
Tests cover source commands, original question diagnostics, invalid-build refusal,
image round trips/downgrades/tampering and a real locked-package native build run
with source/manifest/lock removed and an empty PATH on the current Linux host.
Other-host execution is not qualified by that evidence. Explicit standard-library
adapters and remaining E2/G13 requirements remain open. Task Manager is unchanged.

Adapter environment update: [explicit standard result adapters](generic-result-adapters-v2.md)
now extend experimental commands and select native feature word 31. Historical
words 1/3/7/15 retain their previous environments. Host builtin calls remain gated.
