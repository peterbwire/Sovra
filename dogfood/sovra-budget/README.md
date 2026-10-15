# Sovra Budget

Status: **Implemented bounded numerical CLI**. The app evaluates three
nonnegative categories against a positive limit, reports total spending,
remaining amount, and whole-percent utilization. All inputs are whole units
and at most 1,000,000,000; the library validates them before addition,
multiplication or division, keeping intermediates within Sovra Int bounds.
Utilization truncates fractional percentages. Currency, tax and rounding
rules are deliberately outside this whole-unit slice.

The reusable `library/` package exports nominal `Plan` and `Evaluated` records,
`evaluate`, and `status`. Other local Sovra apps can depend on it by manifest
path. Invalid inputs return `ok: false`, including a zero limit.

```text
cargo run --locked -- run dogfood/sovra-budget/app -- 100 40 20 10
node scripts/test-dogfood.mjs
cargo run --locked -- test dogfood/sovra-budget/tests
```

The separate test package consumes the same library and checks totals, remaining
amounts, truncated percentages, over-budget status and rejected input bounds.
