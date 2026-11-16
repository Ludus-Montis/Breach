---
paths:
  - "content/definitions/**/*.gd"
  - "native/rust/src/content/**/*.rs"
---

# Content Modifications Reflected in the Designer

- When a change is made under `content/definitions` or `native/rust/src/content`, you must also ensure that the change is reflected in updates to the `tools/designer/designer.js` `buildExport()`, `loadFromExport()` and their translation into the schema `native/rust/src/designer`.
- Why: Decision 153 states that the crate owns the schema and the designer writes it, any changes to the schema would break the designer.
- Changes or additions made as part of this rule must be included in the same PR as the triggering modifications.
- Update or add the matching tests under `native/rust/src/designer/tests.rs` in the same PR.
