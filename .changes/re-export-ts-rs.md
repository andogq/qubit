---
"qubit-macros": major
"qubit": major
---

Expose `ts-rs::TS` functionality via `ts` attribute macro. To migrate, replace any instances of
`#[derive(ts_rs::TS)]` with `#[qubit::ts]`, and remove `ts-rs` from package dependencies.
