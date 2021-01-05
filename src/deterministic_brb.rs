/// An implementation of Byzantine Reliable Broadcast (BRB).
use std::collections::{BTreeMap, BTreeSet, HashMap};

use crate::brb_data_type::BRBDataType;
use crate::packet::{Packet, Payload};

use brb_membership::{self, Actor, Generation, Sig};
use crdts::{CmRDT, Dot, VClock};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("error while processing membership change")]
    Membership(#[from] brb_membership::Error),
    #[error("Failed to serialize all or part of a packet")]
    Encoding(#[from] bincode::Error),
    #[error("Packet failed validation")]
    Validation(#[from] Validation),
}

#[derive(Error, Debug)]
pub enum Validation {
    #[error("The actor `{from}` who sent this packet is different from the actor who incremented the dot: `{dot:?}`")]
    PacketSourceIsNotDot { from: Actor, dot: Dot<Actor> },
    #[error("The dot in this message `{msg_dot:?}` is out of order (expected: {expected_dot:?})")]
    MsgDotNotTheNextDot {
        msg_dot: Dot<Actor>,
        expected_dot: Dot<Actor>,
    },
    #[error("The source of this message already has a pending message, we can not start a new operation until the first one has completed")]
    SourceAlreadyHasPendingMsg {
        msg_dot: Dot<Actor>,
        next_deliver_dot: Dot<Actor>,
    },
    #[error("This message is not from this generation {msg_gen} (expected: {gen})")]
    MessageFromDifferentGeneration {
        msg_gen: Generation,
        gen: Generation,
    },
    #[error("Source is not a voting member ({from:?} not in {members:?})")]
    SourceIsNotVotingMember {
        from: Actor,
        members: BTreeSet<Actor>,
    },
    #[error("the datatype failed to validated the operation")]
    DataTypeValidationFailed,
    #[error("Signature is invalid")]
    InvalidSignature,
    #[error("We received a SignedValidated packet for a message we did not request")]
    SignedValidatedForPacketWeDidNotRequest,
    #[error("Message dot {msg_dot:?} to be applied is not the next message to be delivered (expected: {expected_dot:?}")]
    MsgDotNotNextDotToBeDelivered {
        msg_dot: Dot<Actor>,
        expected_dot: Dot<Actor>,
    },
    #[error("The proof did not contain enough signatures to form quorum")]
    NotEnoughSignaturesToFormQuorum,
    #[error("Proof contains signatures from non-members")]
    ProofContainsSignaturesFromNonMembers,
    #[error("Proof contains invalid signatures")]
    ProofContainsInvalidSignatures,
}

#[derive(Debug)]
pub struct DeterministicBRB<A: BRBDataType> {
    // The identity of a process
    pub membership: brb_membership::State,

    // Msgs this process has initiated and is waiting on BFT agreement for from the network.
    pub pending_proof: HashMap<Msg<A::Op>, BTreeMap<Actor, Sig>>,

    // The clock representing the most recently received messages from each process.
    // These are messages that have been acknowledged but not yet
    // This clock must at all times be greator or equal to the `delivered` clock.
    pub received: VClock<Actor>,

    // The clock representing the most recent msgs we've delivered to the underlying datatype `dt`.
    pub delivered: VClock<Actor>,

    // History is maintained to onboard new members
    #[allow(clippy::type_complexity)]
    pub history_from_source: BTreeMap<Actor, Vec<(Msg<A::Op>, BTreeMap<Actor, Sig>)>>,

    // The state of the datatype that we are running BFT over.
    // This can be the causal bank described in AT2, or it can be a CRDT.
    pub dt: A,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Msg<DataTypeOp> {
    gen: Generation,
    op: DataTypeOp,
    dot: Dot<Actor>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Op<DataTypeOp> {
    RequestValidation {
        msg: Msg<DataTypeOp>,
    },
    SignedValidated {
        msg: Msg<DataTypeOp>,
        sig: Sig,
    },
    ProofOfAgreement {
        msg: Msg<DataTypeOp>,
        proof: BTreeMap<Actor, Sig>,
    },
}

impl<DataTypeOp> Payload<DataTypeOp> {
    pub fn is_proof_of_agreement(&self) -> bool {
        matches!(self, Payload::BRB(Op::ProofOfAgreement { .. }))
    }
}

impl<A: BRBDataType> Default for DeterministicBRB<A> {
    fn default() -> Self {
        Self::new()
    }
}

impl<A: BRBDataType> DeterministicBRB<A> {
    pub fn new() -> Self {
        let membership = brb_membership::State::default();
        let dt = A::new(membership.id.actor());
        Self {
            membership,
            dt,
            pending_proof: Default::default(),
            delivered: Default::default(),
            received: Default::default(),
            history_from_source: Default::default(),
        }
    }

    pub fn actor(&self) -> Actor {
        self.membership.id.actor()
    }

    pub fn peers(&self) -> Result<BTreeSet<Actor>, Error> {
        self.membership
            .members(self.membership.gen)
            .map_err(Error::Membership)
    }

    pub fn force_join(&mut self, peer: Actor) {
        println!("[BRB] {:?} is forcing {:?} to join", self.actor(), peer);
        self.membership.force_join(peer);
    }

    pub fn force_leave(&mut self, peer: Actor) {
        println!("[BRB] {:?} is forcing {:?} to leave", self.actor(), peer);
        self.membership.force_leave(peer);
    }

    pub fn request_membership(&mut self, actor: Actor) -> Result<Vec<Packet<A::Op>>, Error> {
        self.membership
            .propose(brb_membership::Reconfig::Join(actor))?
            .into_iter()
            .map(|vote_msg| self.send(vote_msg.dest, Payload::Membership(Box::new(vote_msg.vote))))
            .collect()
    }

    pub fn kill_peer(&mut self, actor: Actor) -> Result<Vec<Packet<A::Op>>, Error> {
        self.membership
            .propose(brb_membership::Reconfig::Leave(actor))?
            .into_iter()
            .map(|vote_msg| self.send(vote_msg.dest, Payload::Membership(Box::new(vote_msg.vote))))
            .collect()
    }

    /// Sends an AntiEntropy packet to the given peer
    pub fn anti_entropy(&self, peer: Actor) -> Result<Packet<A::Op>, Error> {
        let payload = Payload::AntiEntropy {
            generation: self.membership.gen,
            delivered: self.delivered.clone(),
        };
        self.send(peer, payload)
    }

    pub fn exec_op(&self, op: A::Op) -> Result<Vec<Packet<A::Op>>, Error> {
        let msg = Msg {
            op,
            gen: self.membership.gen,
            // We use the received clock to allow for many operations from this process
            // to be pending agreement at any one point in time.
            dot: self.received.inc(self.actor()),
        };

        println!("[BRB] {} initiating bft for msg {:?}", self.actor(), msg);
        self.broadcast(&Payload::BRB(Op::RequestValidation { msg }), self.peers()?)
    }

    pub fn handle_packet(&mut self, packet: Packet<A::Op>) -> Result<Vec<Packet<A::Op>>, Error> {
        println!(
            "[BRB] handling packet from {}->{}",
            packet.source,
            self.actor()
        );

        self.validate_packet(&packet)?;
        self.process_packet(packet)
    }

    fn process_packet(&mut self, packet: Packet<A::Op>) -> Result<Vec<Packet<A::Op>>, Error> {
        let source = packet.source;
        match packet.payload {
            Payload::AntiEntropy {
                generation,
                delivered,
            } => {
                let mut packets_to_send = self
                    .membership
                    .anti_entropy(generation, source)
                    .into_iter()
                    .map(|vote_msg| {
                        self.send(vote_msg.dest, Payload::Membership(Box::new(vote_msg.vote)))
                    })
                    .collect::<Result<Vec<_>, _>>()?;

                for (actor, msgs) in self.history_from_source.iter() {
                    let seen_counter = delivered.get(actor);
                    packets_to_send.extend(
                        // TODO: This can be optimized using Vec::binary_search. This is linear in the number of messages.
                        msgs.iter()
                            .filter(|(msg, _proof)| msg.dot.counter > seen_counter)
                            .map(|(msg, proof)| {
                                self.send(
                                    source,
                                    Payload::BRB(Op::ProofOfAgreement {
                                        msg: msg.clone(),
                                        proof: proof.clone(),
                                    }),
                                )
                            })
                            .collect::<Result<Vec<_>, _>>()?,
                    );
                }

                Ok(packets_to_send)
            }
            Payload::BRB(op) => self.process_brb_op(packet.source, op),
            Payload::Membership(boxed_vote) => self
                .membership
                .handle_vote(*boxed_vote)
                .map_err(Error::Membership)?
                .into_iter()
                .map(|vote_msg| {
                    self.send(vote_msg.dest, Payload::Membership(Box::new(vote_msg.vote)))
                })
                .collect(),
        }
    }

    fn process_brb_op(
        &mut self,
        source: Actor,
        op: Op<A::Op>,
    ) -> Result<Vec<Packet<A::Op>>, Error> {
        match op {
            Op::RequestValidation { msg } => {
                println!("[BRB] request for validation");
                self.received.apply(msg.dot);

                // NOTE: we do not need to store this message, it will be sent back to us
                // with the proof of agreement. Our signature will prevent tampering.
                let sig = self.membership.id.sign(&msg)?;
                let validation = Op::SignedValidated { msg, sig };
                Ok(vec![self.send(source, Payload::BRB(validation))?])
            }
            Op::SignedValidated { msg, sig } => {
                println!("[BRB] signed validated");
                self.pending_proof
                    .entry(msg.clone())
                    .or_default()
                    .insert(source, sig);

                let num_signatures = self.pending_proof[&msg].len();

                // we don't want to re-broadcast a proof if we've already reached quorum
                // hence we check that (num_sigs - 1) was not quorum
                if self.quorum(num_signatures, msg.gen)?
                    && !self.quorum(num_signatures - 1, msg.gen)?
                {
                    println!("[BRB] we have quorum over msg, sending proof to network");
                    // We have quorum, broadcast proof of agreement to network
                    let proof = self.pending_proof[&msg].clone();

                    // Add ourselves to the broadcast recipients since we may have initiated this request
                    // while we were not yet an accepted member of the network.
                    // e.g. this happens if we request to join the network.
                    let recipients = &self.membership.members(msg.gen).unwrap()
                        | &vec![self.actor()].into_iter().collect();
                    self.broadcast(
                        &Payload::BRB(Op::ProofOfAgreement { msg, proof }),
                        recipients,
                    )
                } else {
                    Ok(vec![])
                }
            }
            Op::ProofOfAgreement { msg, proof } => {
                println!("[BRB] proof of agreement: {:?}", msg);
                // We may not have been in the subset of members to validate this clock
                // so we may not have had the chance to increment received. We must bring
                // received up to this msg's timestamp.
                //
                // Otherwise we won't be able to validate any future messages
                // from this source.
                self.received.apply(msg.dot);
                self.delivered.apply(msg.dot);

                // Log this op in our history with proof
                self.history_from_source
                    .entry(msg.dot.actor)
                    .or_default()
                    .push((msg.clone(), proof));

                // Apply the op
                self.dt.apply(msg.op);

                // TODO: Once we relax our network assumptions, we must put in an ack
                // here so that the source knows that honest procs have applied the transaction
                Ok(vec![])
            }
        }
    }

    fn validate_packet(&self, packet: &Packet<A::Op>) -> Result<(), Error> {
        if !packet.source.verify(&packet.payload, &packet.sig)? {
            println!(
                "[BRB/SIG] Msg failed signature verification {}->{}",
                packet.source,
                self.actor(),
            );
            Err(Error::Validation(Validation::InvalidSignature))
        } else {
            self.validate_payload(packet.source, &packet.payload)
        }
    }

    fn validate_payload(&self, from: Actor, payload: &Payload<A::Op>) -> Result<(), Error> {
        match payload {
            Payload::AntiEntropy { .. } => Ok(()),
            Payload::BRB(op) => self.validate_brb_op(from, op),
            Payload::Membership(_) => Ok(()), // membership votes are validated inside membership.handle_vote(..)
        }
    }

    fn validate_brb_op(&self, from: Actor, op: &Op<A::Op>) -> Result<(), Error> {
        match op {
            Op::RequestValidation { msg } => {
                if from != msg.dot.actor {
                    Err(Validation::PacketSourceIsNotDot { from, dot: msg.dot })
                } else if msg.dot != self.received.inc(from) {
                    Err(Validation::MsgDotNotTheNextDot {
                        msg_dot: msg.dot,
                        expected_dot: self.received.inc(from),
                    })
                } else if msg.dot != self.delivered.inc(from) {
                    Err(Validation::SourceAlreadyHasPendingMsg {
                        msg_dot: msg.dot,
                        next_deliver_dot: self.delivered.inc(from),
                    })
                } else if msg.gen != self.membership.gen {
                    Err(Validation::MessageFromDifferentGeneration {
                        msg_gen: msg.gen,
                        gen: self.membership.gen,
                    })
                } else if !self
                    .membership
                    .members(self.membership.gen)?
                    .contains(&from)
                {
                    Err(Validation::SourceIsNotVotingMember {
                        from,
                        members: self.membership.members(self.membership.gen)?,
                    })
                } else if !self.dt.validate(&from, &msg.op) {
                    Err(Validation::DataTypeValidationFailed)
                } else {
                    Ok(())
                }
            }
            Op::SignedValidated { msg, sig } => {
                if !from.verify(&msg, sig)? {
                    Err(Validation::InvalidSignature)
                } else if self.actor() != msg.dot.actor {
                    Err(Validation::SignedValidatedForPacketWeDidNotRequest)
                } else {
                    Ok(())
                }
            }
            Op::ProofOfAgreement { msg, proof } => {
                let msg_members = self.membership.members(msg.gen)?;
                if self.delivered.inc(msg.dot.actor) != msg.dot {
                    Err(Validation::MsgDotNotNextDotToBeDelivered {
                        msg_dot: msg.dot,
                        expected_dot: self.delivered.inc(msg.dot.actor),
                    })
                } else if !self.quorum(proof.len(), msg.gen)? {
                    Err(Validation::NotEnoughSignaturesToFormQuorum)
                } else if !proof
                    .iter()
                    .all(|(signer, _)| msg_members.contains(&signer))
                {
                    Err(Validation::ProofContainsSignaturesFromNonMembers)
                } else if !proof
                    .iter()
                    .map(|(signer, sig)| signer.verify(&msg, &sig))
                    .collect::<Result<Vec<bool>, _>>()?
                    .into_iter()
                    .all(|v| v)
                {
                    Err(Validation::ProofContainsInvalidSignatures)
                } else {
                    Ok(())
                }
            }
        }
        .map_err(Error::Validation)
    }

    fn quorum(&self, n: usize, gen: Generation) -> Result<bool, Error> {
        Ok(n * 3 > self.membership.members(gen)?.len() * 2)
    }

    fn broadcast(
        &self,
        payload: &Payload<A::Op>,
        targets: BTreeSet<Actor>,
    ) -> Result<Vec<Packet<A::Op>>, Error> {
        println!("[BRB] broadcasting {}->{:?}", self.actor(), targets);

        targets
            .into_iter()
            .map(|dest_p| self.send(dest_p, payload.clone()))
            .collect()
    }

    fn send(&self, dest: Actor, payload: Payload<A::Op>) -> Result<Packet<A::Op>, Error> {
        let sig = self.membership.id.sign(&payload)?;
        Ok(Packet {
            source: self.actor(),
            dest,
            payload,
            sig,
        })
    }
}
