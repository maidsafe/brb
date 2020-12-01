use std::collections::HashMap;

use crate::{Actor, Sig};

use std::collections::HashSet;

use crate::SecureBroadcastAlgorithm;
use crdts::{Dot, VClock};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplicatedState<A: SecureBroadcastAlgorithm> {
    pub algo_state: A::ReplicatedState,
    pub peers: HashSet<Actor>,
    pub delivered: VClock<Actor>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Packet<Op> {
    pub source: Actor,
    pub dest: Actor,
    pub payload: Payload<Op>,
    pub sig: Sig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct Msg<Op> {
    pub op: BFTOp<Op>,
    pub dot: Dot<Actor>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum BFTOp<Op> {
    // TODO: support peers leaving
    MembershipNewPeer(Actor),
    AlgoOp(Op),
}
