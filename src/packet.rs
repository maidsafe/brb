use serde::{Deserialize, Serialize};

use crate::actor::{Actor, Sig};
use crate::bft_membership;
use crate::deterministic_brb;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Packet<Op> {
    pub source: Actor,
    pub dest: Actor,
    pub payload: Payload<Op>,
    pub sig: Sig,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Payload<AlgoOp> {
    BRB(deterministic_brb::Op<AlgoOp>),
    Membership(bft_membership::Vote),
}
