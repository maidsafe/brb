use brb_membership;
use serde::{Deserialize, Serialize};

use crate::deterministic_brb;
use crate::{Actor, Sig};

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
    Membership(brb_membership::Vote),
}
