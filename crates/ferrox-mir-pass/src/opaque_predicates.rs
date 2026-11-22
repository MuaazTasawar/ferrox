//! Opaque predicates: conditional branches whose outcome is always the
//! same at runtime but not staticially determinable by a decompiler's
//! simple analysis (e.g. `(x * x) % 2 == (x % 2)` type invariants),
//! inserted into the flattened dispatch to further obscure which paths
//! are actually reachable.
//!
//! **Step 7c implements this.** Stub for step 7a.
