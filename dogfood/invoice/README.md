# Invoice CLI

Status: **Implemented bounded slice**. This command accepts 1–50
`LABEL=AMOUNT` arguments, validates nonnegative whole-unit amounts up to
1,000,000, prints each line and a total, and returns status 1 for bad input.
Labels are bounded to 80 UTF-8 bytes and escaped for terminal output. The
maximum total is 50,000,000, within Sovra's Int range. This is a calculation
example, not a payment or tax system.

```text
cargo run --locked -- run dogfood/invoice/main.svr -- Hosting=20 Storage=35
node scripts/test-dogfood.mjs
```

The dogfood suite checks source, IR, interpreter and generated JavaScript
behavior against reviewed success and failure transcripts.
