# Enum input-to-file example

The source reads one line through a helper returning a nominal enum. On text it
writes the payload to the first program-argument path and prints its UTF-8 byte
length; at EOF it prints `eof`. Terminators are removed and are not written.
A structured file error prints its category and selects status 1.

```sh
svr check --experimental-v2 main.svr
svr run --experimental-v2 --allow-stdin --allow-files main.svr -- output.txt
```

Both grants are required by whole-program preflight, including on the EOF path.
Without either grant the program refuses before reading input or writing files.
The CLI regression pipes `λ😀` with CRLF, verifies exact file bytes and observes
`6` on stdout. Every test uses private temporary files.

These [explicit process hosts](../../docs/reference/experimental-v2-hosts.md) use
finite runtime limits, but file authority follows process permissions and is not
path-confined. Printed output appears after execution succeeds; blocking host
operations are not interrupted and completed writes are not rolled back on a
later failure. Default commands and package/native integration remain separate.

Inspect complete linked lowering with `svr build --experimental-v2 --emit ir main.svr`.
This prints [bounded compiler IR](../../docs/reference/experimental-v2-ir.md) and
executes no source operations; it does not produce a native/JavaScript image.
