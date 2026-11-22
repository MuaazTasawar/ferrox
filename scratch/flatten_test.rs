// Standalone smoke-test file for ferrox-mir-pass, step 7f-i. Not part of
// the cargo workspace -- compiled directly via ferrox-mir-driver.exe,
// bypassing cargo entirely, to isolate "does the Goto-flattening
// transform (step 7b-ii-A) produce a correct binary" from any question
// about cargo's RUSTC_WORKSPACE_WRAPPER protocol (a separate, later step).
//
// classify() has an if/else-if/else chain; sum_to() has a while loop --
// both produce Goto edges in MIR (join points and the loop back-edge)
// that should survive rustc's own optimizations and therefore be
// available for our pass to flatten.

fn classify(n: i32) -> &'static str {
    if n < 0 {
        "negative"
    } else if n == 0 {
        "zero"
    } else {
        "positive"
    }
}

fn sum_to(n: u32) -> u32 {
    let mut total = 0u32;
    let mut i = 0u32;
    while i < n {
        total += i;
        i += 1;
    }
    total
}

fn main() {
    println!("{}", classify(-5));
    println!("{}", classify(0));
    println!("{}", classify(7));
    println!("{}", sum_to(10));
}
