//! Integration test for `#[protect]`. Like `enc!`, attribute-macro
//! expansion needs a real separate crate to exercise properly rather than
//! calling expansion logic directly from a unit test.
//!
//! This only proves the *stable* half of Phase 8: the attribute injects a
//! call and the function still compiles and runs correctly under plain
//! `cargo test`. It says nothing about `ferrox-mir-pass` actually
//! detecting and flattening these functions -- that's step 8b, verified
//! separately via the driver smoke-test process, not through `cargo test`.

use ferrox_macros::protect;

#[protect]
fn classify(n: i32) -> &'static str {
    if n < 0 {
        "negative"
    } else if n == 0 {
        "zero"
    } else {
        "positive"
    }
}

#[protect]
fn add(a: i32, b: i32) -> i32 {
    a + b
}

#[test]
fn protected_function_still_behaves_correctly() {
    assert_eq!(classify(-5), "negative");
    assert_eq!(classify(0), "zero");
    assert_eq!(classify(7), "positive");
}

#[test]
fn protected_function_with_no_branches_still_works() {
    assert_eq!(add(2, 3), 5);
}
