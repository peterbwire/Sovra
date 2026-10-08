# Process input and interactive output

Implemented Stage 1 of [ADR 0014](../adr/0014-cli-input-and-io-foundation.md)
adds general input to executable Sovra source programs.

Run a program with arguments after `--`:

```text
svr run program.svr -- interactive "Review café release"
```

Inside the program, `std::arg_count()` returns the number of arguments after the
delimiter. `std::arg(0)` returns the first. Guard the index before calling:

```svr
fn main() {
    if std::arg_count() > 0 {
        print(std::arg(0));
    }
}
```

`std::read_line()` returns the public nominal `std::InputLine` record. It has
`eof: Bool` and `text: String`. An empty entered line has `eof == false`; when
input ends, `eof == true`. These states are distinct:

```svr
fn main() {
    print("Enter a title:");
    let line: std::InputLine = std::read_line();
    if (line.eof) {
        print("No more input");
        return;
    }
    print(line.text);
}
```

`print` output is written and flushed when the call executes, so the prompt
appears before the program blocks waiting for input. Printed lines remain visible
if a later operation fails. `std::read_line` accepts UTF-8, strips one LF and its
preceding CR, and limits each line to 1 MiB. Invalid UTF-8, oversized lines,
read failures and invalid argument indices are runtime errors. The interpreter
and generated JavaScript have matching supported behavior; generated JS uses
Node.js process streams and receives arguments after its script path.

The [task manager](../../dogfood/01-task-manager) runs a batch by default and an
interactive, in-memory command loop with `-- interactive`. It still has three
active task slots and no persistence. General file I/O is Stage 2 of ADR 0014
and has not been approved or implemented.
