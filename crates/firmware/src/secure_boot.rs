//! Secure Boot support: variable toggle, shim + MokManager integration helpers.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecureBootState {
    Enabled,
    Disabled,
    Unsupported,
}

#[derive(Debug, Clone)]
pub struct SecureBootManager {
    state: SecureBootState,
}

impl SecureBootManager {
    pub fn new(initial_state: SecureBootState) -> Self {
        Self { state: initial_state }
    }

    pub fn state(&self) -> SecureBootState {
        self.state
    }

    pub fn set_state(&mut self, state: SecureBootState) -> Result<(), String> {
        if self.state == SecureBootState::Unsupported {
            return Err("Secure Boot not supported on this platform".into());
        }
        self.state = state;
        Ok(())
    }

    pub fn is_active(&self) -> bool {
        self.state == SecureBootState::Enabled
    }
}
