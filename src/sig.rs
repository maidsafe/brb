use std::fmt;
use std::hash::{Hash, Hasher};

use ed25519::Signature;

use serde::{Serialize, Deserialize};

#[derive(Eq, Clone, Copy, Serialize, Deserialize)]
pub struct Sig(pub Signature);

impl PartialEq for Sig {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Hash for Sig {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.to_bytes().hash(state);
    }
}

impl fmt::Display for Sig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let bytes = self.0.to_bytes();
        write!(f, "sig:{}..", hex::encode(&bytes[..2]))
    }
}

impl fmt::Debug for Sig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self, f)
    }
}
