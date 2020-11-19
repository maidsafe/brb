use std::collections::HashSet;
use std::fmt::Debug;

use crate::{Actor, Packet, SecureBroadcastAlgorithm, SecureBroadcastImpl};

pub trait SecureBroadcastNetwork<I: SecureBroadcastImpl>: Debug {

    fn new() -> Self;

    /// Delivers a given packet to it's target recipiant.
    /// The recipiant, upon processing this packet, may produce it's own packets.
    /// This next set of packets are returned to the caller.
    fn deliver_packet(&mut self, packet: Packet<<I::Algo as SecureBroadcastAlgorithm>::Op>) -> Vec<Packet<<I::Algo as SecureBroadcastAlgorithm>::Op>>;
}

pub trait SecureBroadcastNetworkSimulator<I: SecureBroadcastImpl>: Debug {

    /// The largest set of procs who mutually see each other as peers
    /// are considered to be the network members.
    fn members(&self) -> HashSet<Actor>;

    fn num_packets(&self) -> u64;

    /// Fetch the actors for each process in the network
    fn actors(&self) -> HashSet<Actor>;

    /// Initialize a new process (NOTE: we do not request membership from the network automatically)
    fn initialize_proc(&mut self) -> Actor;

    /// Execute arbitrary code on a proc (immutable)
    fn on_proc<V>(
        &self,
        actor: &Actor,
        f: impl FnOnce(&I) -> V,
    ) -> Option<V>;

    /// Execute arbitrary code on a proc (mutating)
    fn on_proc_mut<V>(
        &mut self,
        actor: &Actor,
        f: impl FnOnce(&mut I) -> V,
    ) -> Option<V>;

    /// Get a (immutable) reference to a proc with the given actor.
    fn proc_from_actor(&self, actor: &Actor) -> Option<&I>;

    /// Get a (mutable) reference to a proc with the given actor.
    fn proc_from_actor_mut(&mut self, actor: &Actor) -> Option<&mut I>;

    /// Perform anti-entropy corrections on the network.
    /// Currently this is God mode implementations in that we don't
    /// use message passing and we share process state directly.
    fn anti_entropy(&mut self);

    /// Checks if all members of the network have converged to the same state.
    fn members_are_in_agreement(&self) -> bool;

    /// Convenience function to iteratively deliver all packets along with any packets
    /// that may result from delivering a packet.
    /// TODO: refactor to remove this allow(patterns_in_fns_without_body)
    #[allow(patterns_in_fns_without_body)]
    fn run_packets_to_completion(&mut self, mut packets: Vec<Packet<<I::Algo as SecureBroadcastAlgorithm>::Op>>);
}
