---
"qubit-macros": patch
---

No longer require handlers to be `async`. The macro will automatically convert all handlers into
async functions upon expansion.
