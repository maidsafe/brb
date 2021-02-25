// Copyright 2021 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under the MIT license <LICENSE-MIT
// http://opensource.org/licenses/MIT> or the Modified BSD license <LICENSE-BSD
// https://opensource.org/licenses/BSD-3-Clause>, at your option. This file may not be copied,
// modified, or distributed except according to those terms. Please review the Licences for the
// specific language governing permissions and limitations relating to use of the SAFE Network
// Software.

//! This Net module implements a simulated (in-memory) network using Actors based on
//! ed25519 keys.
//!
//! Net is intended only for use by test cases.  It is public so that it may be used
//! by test cases in other crates.
//!
//! Net may be moved outside the brb crate at a later time.  It should not be used
//! or relied upon except in test cases.

use std::collections::{BTreeSet, HashMap};

use log::{info, warn};
use std::fs::File;
use std::io::Write;

use crate::brb_data_type::BRBDataType;
use crate::deterministic_brb::DeterministicBRB;
pub use brb_membership::actor::ed25519::{Actor, Sig, SigningActor};
use brb_membership::SigningActor as SigningActorTrait;

/// A DeterministicBRB specialized to ed25519 types, for use in simulated Network and test cases.
pub type State<BRBDT> = DeterministicBRB<Actor, SigningActor, Sig, BRBDT>;

/// A Packet specialized to ed25519 types, for use in simulated Network and test cases.
pub type Packet<BRBDT> = crate::packet::Packet<Actor, Sig, BRBDT>;

/// A BRBDataType specialized to ed25519::Actor, for use in simulated Network and test cases.
pub trait BRBDT: BRBDataType<Actor> {}
impl<T: BRBDataType<Actor>> BRBDT for T {}

/// Net -- a simulated in-memory network specialized to ed25519 keys.
#[derive(Debug)]
pub struct Net<DT: BRBDT> {
    /// list of processes/nodes comprising the network.
    pub procs: Vec<State<DT>>,
    /// list of packets that have been delivered
    pub delivered_packets: Vec<Packet<DT::Op>>,
    /// total number of packets sent during network's lifetime
    pub n_packets: u64,
    /// count of invalid packets, by actor.
    pub invalid_packets: HashMap<Actor, u64>,
}

impl<DT: BRBDT> Default for Net<DT> {
    /// create a default BRBDT instance
    fn default() -> Self {
        Self::new()
    }
}

impl<DT: BRBDT> Net<DT> {
    /// Create a new BRBDT instance
    pub fn new() -> Self {
        Self {
            procs: Vec::new(),
            n_packets: 0,
            delivered_packets: Default::default(),
            invalid_packets: Default::default(),
        }
    }

    /// The largest set of procs who mutually see each other as peers
    /// are considered to be the network members.
    pub fn members(&self) -> BTreeSet<Actor> {
        self.procs
            .iter()
            .map(|proc| {
                proc.peers()
                    .unwrap()
                    .iter()
                    .flat_map(|peer| self.proc(peer))
                    .filter(|peer_proc| peer_proc.peers().unwrap().contains(&proc.actor()))
                    .map(|peer_proc| peer_proc.actor())
                    .collect::<BTreeSet<_>>()
            })
            .max_by_key(|members| members.len())
            .unwrap_or_default()
    }

    /// Fetch the actors for each process in the network
    pub fn actors(&self) -> BTreeSet<Actor> {
        self.procs.iter().map(|p| p.actor()).collect()
    }

    /// Initialize a new process (NOTE: we do not request membership from the network automatically)
    pub fn initialize_proc(&mut self) -> Actor {
        let proc = DeterministicBRB::new();
        let actor = proc.actor();
        self.procs.push(proc);
        actor
    }

    /// Get a (immutable) reference to a proc with the given actor.
    pub fn proc(&self, actor: &Actor) -> Option<&State<DT>> {
        self.procs
            .iter()
            .find(|secure_p| &secure_p.actor() == actor)
    }

    /// Get a (mutable) reference to a proc with the given actor.
    pub fn proc_mut(&mut self, actor: &Actor) -> Option<&mut State<DT>> {
        self.procs
            .iter_mut()
            .find(|secure_p| &secure_p.actor() == actor)
    }

    /// Perform anti-entropy corrections on the network.
    /// Currently this is God mode implementations in that we don't
    /// use message passing and we share process state directly.
    pub fn anti_entropy(&mut self) {
        // TODO: this should be done through a message passing interface.
        info!("[NET] anti-entropy");

        let packets: Vec<_> = self
            .procs
            .iter()
            .flat_map(|proc| {
                proc.peers()
                    .unwrap()
                    .into_iter()
                    .map(move |peer| proc.anti_entropy(peer).unwrap())
            })
            .collect();

        self.run_packets_to_completion(packets);
    }

    /// Delivers a given packet to it's target recipiant.
    /// The recipiant, upon processing this packet, may produce it's own packets.
    /// This next set of packets are returned to the caller.
    pub fn deliver_packet(&mut self, packet: Packet<DT::Op>) -> Vec<Packet<DT::Op>> {
        info!("[NET] packet {}->{}", packet.source, packet.dest);
        self.n_packets += 1;
        let dest = packet.dest;
        self.delivered_packets.push(packet.clone());
        self.proc_mut(&dest)
            .map(|p| p.handle_packet(packet))
            .unwrap_or_else(|| Ok(vec![])) // no proc to deliver too
            .unwrap_or_else(|err| {
                warn!("[BRB] Rejected packet: {:?}", err);
                let count = self.invalid_packets.entry(dest).or_default();
                *count += 1;
                vec![]
            })
    }

    /// Checks if all members of the network have converged to the same state.
    pub fn members_are_in_agreement(&self) -> bool {
        // Procs are in agreement if the their op histories are identical
        let mut member_states_iter = self
            .members()
            .into_iter()
            .flat_map(|actor| self.proc(&actor))
            .map(|p| &p.history_from_source);

        if let Some(reference_state) = member_states_iter.next() {
            member_states_iter.all(|s| s == reference_state)
        } else {
            true // vacuously, there are no members
        }
    }

    /// counts number of invalid packets received by any proc
    pub fn count_invalid_packets(&self) -> u64 {
        self.invalid_packets.values().sum()
    }

    /// Convenience function to iteratively deliver all packets along with any packets
    /// that may result from delivering a packet.
    pub fn run_packets_to_completion(&mut self, mut packets: Vec<Packet<DT::Op>>) {
        while !packets.is_empty() {
            let packet = packets.remove(0);
            packets.extend(self.deliver_packet(packet));
        }
    }

    /// Generates an MSC file representing a packet sequence diagram.
    /// See http://www.mcternan.me.uk/mscgen/
    /// See https://github.com/maidsafe/brb_membership#tests
    pub fn generate_msc(&self, chart_name: &str) {
        // See: http://www.mcternan.me.uk/mscgen/
        let mut msc = String::from(
            "
msc {\n
  hscale = \"2\";\n
",
        );
        let procs = self
            .procs
            .iter()
            .map(|p| p.membership.id.actor())
            .collect::<BTreeSet<_>>() // sort by actor id
            .into_iter()
            .map(|id| format!("{:?}", id))
            .collect::<Vec<_>>()
            .join(",");
        msc.push_str(&procs);
        msc.push_str(";\n");
        for packet in self.delivered_packets.iter() {
            msc.push_str(&format!(
                "{}->{} [ label=\"{:?}\"];\n",
                packet.source, packet.dest, packet.payload
            ));
        }

        msc.push_str("}\n");

        // Replace process identifiers with friendlier numbers
        // 1, 2, 3 ... instead of i:3b2, i:7def, ...
        for (idx, proc_id) in self
            .procs
            .iter()
            .map(|p| p.membership.id.actor())
            .enumerate()
        {
            let proc_id_as_str = format!("{}", proc_id);
            msc = msc.replace(&proc_id_as_str, &format!("{}", idx + 1));
        }
        let mut msc_file = File::create(format!("{}.msc", chart_name)).unwrap();
        msc_file.write_all(msc.as_bytes()).unwrap();
    }
}
