# Format a standalone source file

Preview formatting and check without changing source:

```text
svr fmt examples/hello-world/main.svr
svr fmt --check examples/hello-world/main.svr
```

On Windows or Linux, apply it with `svr fmt --write examples/hello-world/main.svr`.
Check exits 1 when changes are needed. Successful writes exit 0; an already
formatted file retains its timestamp. Formatting preserves comments and literal/
token spelling and never runs your program. `fn main(){print("Hello");}` becomes:

```svr
fn main() {
    print("Hello");
}
```

The formatter handles individual executable files, inline modules and package
imports (`use dependency::module;`) without loading dependencies. Application
declarations and recursive directory traversal are deferred. macOS and other-host
writes await qualification. See the [reference](../reference/formatter.md) for limits.

Check the entry files of a local package and its dependencies without writes:

```text
svr fmt --package --check examples/local-packages/app
```

This follows compiler-owned entries, including dependencies outside the root,
rather than searching all `.svr` files. On Windows/Linux, explicitly apply the same graph with
`svr fmt --package --write examples/local-packages/app`. This also writes dependency
entries outside the root. All files are validated and staged first; replacements
are atomic per file, so a later failure can leave earlier files formatted. The
error reports how many replacements remain. Manifests are never formatted.
