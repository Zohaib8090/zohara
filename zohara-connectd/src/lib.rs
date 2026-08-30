//! zohara-connectd library crate.
//!
//! Modules are populated by subsequent tasks; for now this is a stub so
//! `cargo check` passes and CI picks up the new crate.

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_set() {
        assert!(!version().is_empty());
    }
}
