//! F1.3: panic hygiene. In release builds, replaces the default panic
//! hook so panics never write a file path, line number, or message to
//! stderr -- which would otherwise hand a reverse engineer a convenient
//! map straight to interesting code paths (e.g. "thread panicked at
//! src/license.rs:42: invalid license key").
//!
//! Deliberately scoped to *panic hygiene*, not panic *prevention*: a
//! panic can still unwind and terminate the process (or abort, depending
//! on the consuming crate's `panic` profile setting) -- this module only
//! ensures it does so silently. NF6's "no panics in ferrox-runtime"
//! requirement is about this crate's own code never panicking; it says
//! nothing about panics in the *protected* crate's own logic, which this
//! module exists to keep quiet about.

/// Installs a silent panic hook, but only in release builds
/// (`cfg(not(debug_assertions))`). In debug builds, this is a no-op --
/// deliberately, so a developer working on the protected crate itself
/// still sees normal panic output while iterating.
pub fn install() {
    #[cfg(not(debug_assertions))]
    {
        std::panic::set_hook(Box::new(|_info| {
            // Intentionally empty: F1.3 requires nothing be written to
            // stderr, not even a sanitized message. The absence of a
            // crash message is itself the point -- an attacker should
            // not be able to distinguish "panicked" from "process exited"
            // from the console alone.
        }));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn install_does_not_panic_itself() {
        // NF6: this crate's own code must never panic. Calling install()
        // must not panic regardless of build profile.
        install();
    }
}
