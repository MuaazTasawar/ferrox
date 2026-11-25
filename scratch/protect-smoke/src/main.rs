// Phase 8, step 8b discrimination smoke test. `classify_protected` and
// `classify_unprotected` are IDENTICAL three-way if/else-if/else logic --
// the only difference is the #[protect] attribute. If ferrox-mir-pass's
// detection (flatten.rs::is_protected) works correctly, FERROX_MIR_DEBUG=1
// output should show flattening applied ONLY to classify_protected, and
// classify_unprotected should be reported as skipped (not marked
// #[protect]). Both must still produce correct output regardless.

use ferrox_macros::protect;

#[protect]
fn classify_protected(n: i32) -> &'static str {
    if n < 0 {
        "negative"
    } else if n == 0 {
        "zero"
    } else {
        "positive"
    }
}

fn classify_unprotected(n: i32) -> &'static str {
    if n < 0 {
        "negative"
    } else if n == 0 {
        "zero"
    } else {
        "positive"
    }
}

fn main() {
    println!("protected: {}", classify_protected(-5));
    println!("protected: {}", classify_protected(0));
    println!("protected: {}", classify_protected(7));
    println!("unprotected: {}", classify_unprotected(-5));
    println!("unprotected: {}", classify_unprotected(0));
    println!("unprotected: {}", classify_unprotected(7));
}
