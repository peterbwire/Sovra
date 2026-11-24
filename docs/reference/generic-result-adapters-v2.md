# Explicit standard result adapters (V2)

`compile_generic_adapter_source` extends the propagation source profile with five
immutable compiler-owned legacy record schemas. Their qualified names and fields
match the existing core standard library; user lookalikes remain distinct.
The existing builtin return types are unchanged.

| Record | Existing fields | Explicit adapter result |
| --- | --- | --- |
| `std::InputLine` | `eof:Bool`, `text:String` | `std::Option<String>`; EOF is None |
| `std::ParsedInt` | `ok:Bool`, `value:Int` | `std::Option<Int>`; false is None |
| `std::SplitOnce` | `found:Bool`, `before:String`, `after:String` | `std::Option<[String]>`; Some contains both parts |
| `std::TextRead` | `ok:Bool`, `text:String`, `error:String` | `std::Result<String,std::Error>` |
| `std::TextWrite` | `ok:Bool`, `error:String` | `std::Result<Unit,std::Error>` |

The reusable [Sovra adapter source](../../src/compiler/v2/standard_result_adapters.svr)
is exported as `STANDARD_RESULT_ADAPTER_SOURCE`. Include these complete source
bytes before your own functions and compile the combined file with the explicit
adapter profile. The functions are `adapt_input_line`, `adapt_parsed_int`,
`adapt_split_once`, `adapt_text_read`, `adapt_text_write` and the Unit helper
`adapter_unit`. They are ordinary checked Sovra functions, not implicit coercions
or automatically injected declarations. Normal duplicate-name rules apply.
The [consumer example](../examples/generic-result-adapters.svr) composes them with `?`.

Read/write failure preserves the legacy error message exactly, sets code to
`text_read_failed` or `text_write_failed` respectively, and sets offset to None.
No location is invented. Success ignores the old error string. Missing/false flags
never unwrap payloads. Empty successful strings remain Some/Ok; successful writes
carry a real Unit payload. The split adapter preserves both sides, including empty
strings. These choices are explicit in reusable source that developers can inspect
or replace with their own conversion policy.

All adapter source bytes, names, bodies, instances, snapshots and instructions use
existing whole-source and shared stage/runtime quotas. Qualified standard record
construction validates original source tokens, exact nominal identity, field
coverage/types and zero type parameters. Unit tests compare schemas to the core
standard record contract. No host I/O or builtin calls are enabled: this slice
adapts already provided record values. Connecting existing builtin calls to the
generic execution pipeline remains open.

`--experimental-generics` now uses this adapter-capable source environment.
Native feature word **31** preserves that environment at bounded startup rechecking.
Words 1/3/7/15 retain their previous environments. Only those five exact words are
accepted; downgrading an adapter consumer refuses rather than substituting user
types. Ordinary source APIs stay strict. Adapter source is packaged under `src`
so the existing runtime-source payload copy includes it.

Tests cover all flag branches, empty payloads, structured failures, lookalikes,
field/type-argument refusal, resource exhaustion, source/native profile isolation,
and an actual locked Linux native consumer run without its inputs or tool PATH.
Other-host execution, builtin-call integration and remaining E2/G13 requirements
remain open. Task Manager is unchanged.

Host-free builtin update: [checked parsing and splitting](generic-builtin-calls-v2.md)
now use the explicit builtin source profile and experimental CLI/native feature 63.
Schema-only profile 31 stays strict; other host/builtin operations remain gated.
