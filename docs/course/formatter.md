# Format a standalone source file

Preview formatting and check without changing source:

```text
svr fmt examples/hello-world/main.svr
svr fmt --check examples/hello-world/main.svr
```

On Windows, apply it with `svr fmt --write examples/hello-world/main.svr`.
Check exits 1 when changes are needed. Successful writes exit 0; an already
formatted file retains its timestamp. Formatting preserves comments and literal/
token spelling and never runs your program. `fn main(){print("Hello");}` becomes:

```svr
fn main() {
    print("Hello");
}
```

The first slice handles individual executable files and inline modules. Package
imports, application declarations and directory traversal are deferred. Non-Windows
writes await qualification. See the [reference](../reference/formatter.md) for limits.
