#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CompromisedPasswordDecision {
    compromised: bool,
}

impl CompromisedPasswordDecision {
    pub fn new(compromised: bool) -> Self {
        Self { compromised }
    }

    pub fn is_compromised(&self) -> bool {
        self.compromised
    }
}
