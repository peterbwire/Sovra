# Displaying untrusted text in a CLI

Use `std::escape_terminal` before printing text that came from an argument,
input line or file. It turns terminal controls into printable notation while
leaving ordinary Unicode and backslashes unchanged.

```svr
fn main() {
    let input = std::read_line();
    if input.eof { return; }
    print(std::escape_terminal(input.text));
}
```

For example, an ESC character is shown as `\u{001B}` instead of being sent to
the terminal. The function escapes C0, DEL and C1 controls; it is a display
helper, not a reversible format or a guarantee against all visual spoofing.

The [Task Manager](../../dogfood/01-task-manager/README.md) rejects new titles
containing these controls. It still reads older files with such titles and
escapes them when listing tasks or reporting errors. Exact title lookup uses
the original stored text.
