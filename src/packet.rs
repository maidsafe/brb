use serde::{Deserialize, Serialize};

use crate::deterministic_brb;
use crate::{Actor, Sig};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Packet<A: Actor<S>, S: Sig, Op> {
    pub source: A,
    pub dest: A,
    pub payload: Payload<A, S, Op>,
    pub sig: S,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Payload<A: Actor<S>, S: Sig, DataTypeOp> {
    AntiEntropy {
        generation: brb_membership::Generation,
        delivered: crdts::VClock<A>,
    },
    BRB(deterministic_brb::Op<A, S, DataTypeOp>),
    // Box to avoid https://rust-lang.github.io/rust-clippy/master/index.html#large_enum_variant
    Membership(Box<brb_membership::Vote<A, S>>),
}
