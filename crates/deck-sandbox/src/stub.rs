//! Non-Linux stub. Reports unavailable; never enforces.

use deck_core::Sandbox;

#[derive(Debug, Default)]
pub struct StubSandbox {
    _placeholder: (),
}

impl Sandbox for StubSandbox {
    fn availability(&self) -> &'static str {
        "unsupported (non-linux)"
    }

    fn enforces(&self) -> bool {
        false
    }
}
