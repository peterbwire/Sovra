# Decode text safely

Sovra's text-file API returns a string, leaving the application responsible for
its format. `std::split_once` and `std::parse_int` expose failures explicitly:

```svr
fn main() {
    let pair: std::SplitOnce = std::split_once("42|open", "|");
    if (pair.found) {
        let number: std::ParsedInt = std::parse_int(pair.before);
        if (number.ok) { print(number.value); }
    }
}
```

The first split returns the text before and after the first exact delimiter.
An empty delimiter matches before the first character. Integer parsing accepts
ASCII decimal text only, including a leading minus sign, and rejects whitespace,
leading zeros, plus signs, and values outside signed 64-bit range. Always check
`ok` before using `value` and validate the rest of your file's schema; a
successful number parse does not make a whole file valid.
Even very long decimal text returns a failure result without constructing an
unbounded integer value in either execution engine.

For line-oriented formats, `std::lines_unique(text)` checks exact LF-separated
lines in one pass. It returns true for `""`, `"a\nb\n"` and `"a\na\r\n"`,
but false for `"a\na\n"`. A final LF does not add an empty line. CR remains
part of the line, and empty lines can be duplicates. Validate each line's
syntax separately; uniqueness only checks equality.

Run `cargo run -- run tests/fixtures/text-decoding.svr` for edge cases. The
Task Manager in `dogfood/01-task-manager` shows a versioned file format with
strict record validation and restart tests.
