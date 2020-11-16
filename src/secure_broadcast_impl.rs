use std::collections::HashSet;
use std::fmt::Debug;

use ed25519::Keypair;

use crate::{Actor, Packet, ReplicatedState, SecureBroadcastAlgorithm};

pub trait SecureBroadcastImpl<A: SecureBroadcastAlgorithm>: Debug {
    type Algo: SecureBroadcastAlgorithm;

    fn new(known_peers: HashSet<Actor>) -> Self; 

    fn keypair(&self) -> &Keypair;

    fn actor(&self) -> Actor;

    fn state(&self) -> ReplicatedState<A>;

    fn peers(&self) -> HashSet<Actor>;

    fn request_membership(&self) -> Vec<Packet<A::Op>>;

    fn sync_from(&mut self, state: ReplicatedState<A>);

    fn exec_algo_op(&self, f: impl FnOnce(&A) -> Option<A::Op>) -> Vec<Packet<A::Op>>;

    fn read_state<V>(&self, f: impl FnOnce(&A) -> V) -> V;

    fn handle_packet(&mut self, packet: Packet<A::Op>) -> Vec<Packet<A::Op>>;
}
