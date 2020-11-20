use std::collections::HashSet;
use std::fmt::Debug;

use ed25519::Keypair;

use crate::{Actor, Packet, ReplicatedState, SecureBroadcastAlgorithm};

pub trait SecureBroadcastImpl: Debug {
    type Algo: SecureBroadcastAlgorithm;

    fn new(known_peers: HashSet<Actor>) -> Self;

    fn keypair(&self) -> &Keypair;

    fn actor(&self) -> Actor;

    fn state(&self) -> ReplicatedState<Self::Algo>;

    fn peers(&self) -> HashSet<Actor>;

    fn request_membership(&self) -> Vec<Packet<<Self::Algo as SecureBroadcastAlgorithm>::Op>>;

    fn sync_from(&mut self, state: ReplicatedState<Self::Algo>);

    fn exec_algo_op(
        &self,
        f: impl FnOnce(&Self::Algo) -> Option<<Self::Algo as SecureBroadcastAlgorithm>::Op>,
    ) -> Vec<Packet<<Self::Algo as SecureBroadcastAlgorithm>::Op>>;

    fn read_state<V>(&self, f: impl FnOnce(&Self::Algo) -> V) -> V;

    fn handle_packet(
        &mut self,
        packet: Packet<<Self::Algo as SecureBroadcastAlgorithm>::Op>,
    ) -> Vec<Packet<<Self::Algo as SecureBroadcastAlgorithm>::Op>>;
}
