# Data toolchain consumer

This independent package checks Byte/Bytes conversions, strict UTF-8/hex/Base64
and recursive JSON round trips through the experimental Rust data profile.

From the repository root:

```sh
svr lock --experimental-generics examples/data-toolchain/app
svr check --experimental-generics --experimental-data --locked examples/data-toolchain/app
svr run --experimental-generics --experimental-data --locked examples/data-toolchain/app
svr test --experimental-generics --experimental-data --locked examples/data-toolchain/app
svr build --experimental-generics --experimental-data --locked --emit exe --output ./data-consumer examples/data-toolchain/app
./data-consumer
```

Native builds need the verified Rust runtime source/toolchain; launching the
generated executable needs neither Node.js nor the original package. Linux
launch is covered by `tests/generic_data_cli_v2.rs`, including execution after
project deletion with an empty environment and unavailable PATH. The same test
target contains an independent external-policy filesystem/process consumer.

See [data/host commands and limits](../../docs/reference/data-host-v3.md) and
[qualification status](../../docs/G13_QUALIFICATION.md). No host grants are needed
for this pure data round trip. Default compiler commands do not select this profile.
