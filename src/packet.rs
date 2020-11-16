use std::collections::HashMap;

use crate::{Actor, Sig};

use std::collections::HashSet;

use crdts::{Dot, VClock};
use serde::Serialize;
use crate::SecureBroadcastAlgorithm;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReplicatedState<A: SecureBroadcastAlgorithm> {
    pub algo_state: A::ReplicatedState,
    pub peers: HashSet<Actor>,
    pub delivered: VClock<Actor>,
}

#[derive(Debug, Clone)]
pub struct Packet<Op> {
    pub source: Actor,
    pub dest: Actor,
    pub payload: Payload<Op>,
    pub sig: Sig,
}

#[derive(Debug, Clone, Serialize)]
pub enum Payload<Op> {
    RequestValidation {
        msg: Msg<Op>,
    },
    SignedValidated {
        msg: Msg<Op>,
        sig: Sig,
    },
    ProofOfAgreement {
        msg: Msg<Op>,
        proof: HashMap<Actor, Sig>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Hash)]
pub struct Msg<Op> {
    pub op: BFTOp<Op>,
    pub dot: Dot<Actor>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Hash)]
pub enum BFTOp<Op> {
    // TODO: support peers leaving
    MembershipNewPeer(Actor),
    AlgoOp(Op),
}

