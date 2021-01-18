/// An implementation of Byzantine Reliable Broadcast (BRB).
use std::collections::{BTreeMap, BTreeSet, HashMap};

use crate::brb_data_type::BRBDataType;
use crate::packet::{Packet, Payload};
use crate::{Error, ValidationError};

use log::info;

use brb_membership::{self, Actor, Generation, Sig, SigningActor};
use crdts::{CmRDT, Dot, VClock};
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub struct DeterministicBRB<A: Actor<S>, SA: SigningActor<A, S>, S: Sig, BRBDT: BRBDataType<A>> {
    // The identity of a process
    pub membership: brb_membership::State<A, SA, S>,

    // Msgs this process has initiated and is waiting on BFT agreement for from the network.
    pub pending_proof: HashMap<Msg<A, BRBDT::Op>, BTreeMap<A, S>>,

    // The clock representing the most recently received messages from each process.
    // These are messages that have been acknowledged but not yet
    // This clock must at all times be greator or equal to the `delivered` clock.
    pub received: VClock<A>,

    // The clock representing the most recent msgs we've delivered to the underlying datatype `dt`.
    pub delivered: VClock<A>,

    // History is maintained to onboard new members
    #[allow(clippy::type_complexity)]
    pub history_from_source: BTreeMap<A, Vec<(Msg<A, BRBDT::Op>, BTreeMap<A, S>)>>,

    // The state of the datatype that we are running BFT over.
    // This can be the causal bank described in AT2, or it can be a CRDT.
    pub dt: BRBDT,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Msg<A, DataTypeOp> {
    gen: Generation,
    op: DataTypeOp,
    dot: Dot<A>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Op<A: Ord, S, DataTypeOp> {
    RequestValidation {
        msg: Msg<A, DataTypeOp>,
    },
    SignedValidated {
        msg: Msg<A, DataTypeOp>,
        sig: S,
    },
    ProofOfAgreement {
        msg: Msg<A, DataTypeOp>,
        proof: BTreeMap<A, S>,
    },
}

impl<A: Actor<S>, S: Sig, DataTypeOp> Payload<A, S, DataTypeOp> {
    pub fn is_proof_of_agreement(&self) -> bool {
        matches!(self, Payload::BRB(Op::ProofOfAgreement { .. }))
    }
}

impl<A: Actor<S>, SA: SigningActor<A, S>, S: Sig, BRBDT: BRBDataType<A>> Default
    for DeterministicBRB<A, SA, S, BRBDT>
{
    fn default() -> Self {
        Self::new()
    }
}

impl<A: Actor<S>, SA: SigningActor<A, S>, S: Sig, BRBDT: BRBDataType<A>>
    DeterministicBRB<A, SA, S, BRBDT>
{
    pub fn new() -> Self {
        let membership: brb_membership::State<A, SA, S> = Default::default();
        let dt = BRBDT::new(membership.id.actor());
        Self {
            membership,
            dt,
            pending_proof: Default::default(),
            delivered: Default::default(),
            received: Default::default(),
            history_from_source: Default::default(),
        }
    }

    pub fn actor(&self) -> A {
        self.membership.id.actor()
    }

    pub fn peers(&self) -> Result<BTreeSet<A>, Error<A, S, BRBDT::ValidationError>> {
        self.membership
            .members(self.membership.gen)
            .map_err(Error::Membership)
    }

    pub fn force_join(&mut self, peer: A) {
        info!("[BRB] {:?} is forcing {:?} to join", self.actor(), peer);
        self.membership.force_join(peer);
    }

    pub fn force_leave(&mut self, peer: A) {
        info!("[BRB] {:?} is forcing {:?} to leave", self.actor(), peer);
        self.membership.force_leave(peer);
    }

    pub fn request_membership(
        &mut self,
        actor: A,
    ) -> Result<Vec<Packet<A, S, BRBDT::Op>>, Error<A, S, BRBDT::ValidationError>> {
        self.membership
            .propose(brb_membership::Reconfig::Join(actor))?
            .into_iter()
            .map(|vote_msg| self.send(vote_msg.dest, Payload::Membership(Box::new(vote_msg.vote))))
            .collect()
    }

    pub fn kill_peer(
        &mut self,
        actor: A,
    ) -> Result<Vec<Packet<A, S, BRBDT::Op>>, Error<A, S, BRBDT::ValidationError>> {
        self.membership
            .propose(brb_membership::Reconfig::Leave(actor))?
            .into_iter()
            .map(|vote_msg| self.send(vote_msg.dest, Payload::Membership(Box::new(vote_msg.vote))))
            .collect()
    }

    /// Sends an AntiEntropy packet to the given peer
    pub fn anti_entropy(
        &self,
        peer: A,
    ) -> Result<Packet<A, S, BRBDT::Op>, Error<A, S, BRBDT::ValidationError>> {
        let payload = Payload::AntiEntropy {
            generation: self.membership.gen,
            delivered: self.delivered.clone(),
        };
        self.send(peer, payload)
    }

    pub fn exec_op(
        &self,
        op: BRBDT::Op,
    ) -> Result<Vec<Packet<A, S, BRBDT::Op>>, Error<A, S, BRBDT::ValidationError>> {
        let msg = Msg {
            op,
            gen: self.membership.gen,
            // We use the received clock to allow for many operations from this process
            // to be pending agreement at any one point in time.
            dot: self.received.inc(self.actor()),
        };

        info!("[BRB] {} initiating bft for msg {:?}", self.actor(), msg);
        self.broadcast(&Payload::BRB(Op::RequestValidation { msg }), self.peers()?)
    }

    pub fn handle_packet(
        &mut self,
        packet: Packet<A, S, BRBDT::Op>,
    ) -> Result<Vec<Packet<A, S, BRBDT::Op>>, Error<A, S, BRBDT::ValidationError>> {
        info!(
            "[BRB] handling packet from {}->{}",
            packet.source,
            self.actor()
        );

        self.validate_packet(&packet)?;
        self.process_packet(packet)
    }

    fn process_packet(
        &mut self,
        packet: Packet<A, S, BRBDT::Op>,
    ) -> Result<Vec<Packet<A, S, BRBDT::Op>>, Error<A, S, BRBDT::ValidationError>> {
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
        source: A,
        op: Op<A, S, BRBDT::Op>,
    ) -> Result<Vec<Packet<A, S, BRBDT::Op>>, Error<A, S, BRBDT::ValidationError>> {
        match op {
            Op::RequestValidation { msg } => {
                info!("[BRB] request for validation");
                self.received.apply(msg.dot);

                // NOTE: we do not need to store this message, it will be sent back to us
                // with the proof of agreement. Our signature will prevent tampering.
                let sig = self.sign(&msg)?;
                let validation = Op::SignedValidated { msg, sig };
                Ok(vec![self.send(source, Payload::BRB(validation))?])
            }
            Op::SignedValidated { msg, sig } => {
                info!("[BRB] signed validated");
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
                    info!("[BRB] we have quorum over msg, sending proof to network");
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
                info!("[BRB] proof of agreement: {:?}", msg);
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

                // Remove the message from pending_proof since we now have proof
                self.pending_proof.remove(&msg);

                // Apply the op
                self.dt.apply(msg.op);

                Ok(vec![])
            }
        }
    }

    fn validate_packet(
        &self,
        packet: &Packet<A, S, BRBDT::Op>,
    ) -> Result<(), Error<A, S, BRBDT::ValidationError>> {
        self.verify(&packet.payload, &packet.source, &packet.sig)?;
        self.validate_payload(packet.source, &packet.payload)
    }

    fn validate_payload(
        &self,
        from: A,
        payload: &Payload<A, S, BRBDT::Op>,
    ) -> Result<(), Error<A, S, BRBDT::ValidationError>> {
        match payload {
            Payload::AntiEntropy { .. } => Ok(()),
            Payload::BRB(op) => self.validate_brb_op(from, op),
            Payload::Membership(_) => Ok(()), // membership votes are validated inside membership.handle_vote(..)
        }
    }

    fn validate_brb_op(
        &self,
        from: A,
        op: &Op<A, S, BRBDT::Op>,
    ) -> Result<(), Error<A, S, BRBDT::ValidationError>> {
        match op {
            Op::RequestValidation { msg } => {
                if from != msg.dot.actor {
                    Err(ValidationError::PacketSourceIsNotDot { from, dot: msg.dot })
                } else if msg.dot != self.received.inc(from) {
                    Err(ValidationError::MsgDotNotTheNextDot {
                        msg_dot: msg.dot,
                        expected_dot: self.received.inc(from),
                    })
                } else if msg.dot != self.delivered.inc(from) {
                    Err(ValidationError::SourceAlreadyHasPendingMsg {
                        msg_dot: msg.dot,
                        next_deliver_dot: self.delivered.inc(from),
                    })
                } else if msg.gen != self.membership.gen {
                    Err(ValidationError::MessageFromDifferentGeneration {
                        msg_gen: msg.gen,
                        gen: self.membership.gen,
                    })
                } else if !self
                    .membership
                    .members(self.membership.gen)?
                    .contains(&from)
                {
                    Err(ValidationError::SourceIsNotVotingMember {
                        from,
                        members: self.membership.members(self.membership.gen)?,
                    })
                } else {
                    self.dt
                        .validate(&from, &msg.op)
                        .map_err(ValidationError::DataTypeFailedValidation)
                }
            }
            Op::SignedValidated { msg, sig } => {
                self.verify(&msg, &from, sig)?;

                if self.actor() != msg.dot.actor {
                    Err(ValidationError::SignedValidatedForPacketWeDidNotRequest)
                } else {
                    Ok(())
                }
            }
            Op::ProofOfAgreement { msg, proof } => {
                let msg_members = self.membership.members(msg.gen)?;
                if self.delivered.inc(msg.dot.actor) != msg.dot {
                    Err(ValidationError::MsgDotNotNextDotToBeDelivered {
                        msg_dot: msg.dot,
                        expected_dot: self.delivered.inc(msg.dot.actor),
                    })
                } else if !self.quorum(proof.len(), msg.gen)? {
                    Err(ValidationError::NotEnoughSignaturesToFormQuorum)
                } else if !proof
                    .iter()
                    .all(|(signer, _)| msg_members.contains(&signer))
                {
                    Err(ValidationError::ProofContainsSignaturesFromNonMembers)
                } else if !proof
                    .iter()
                    .map(|(signer, sig)| self.verify(&msg, &signer, &sig))
                    .collect::<Result<Vec<()>, _>>()
                    .is_ok()
                {
                    Err(ValidationError::ProofContainsInvalidSignatures)
                } else {
                    Ok(())
                }
            }
        }
        .map_err(Error::Validation)
    }

    fn quorum(
        &self,
        n: usize,
        gen: Generation,
    ) -> Result<bool, Error<A, S, BRBDT::ValidationError>> {
        Ok(n * 3 > self.membership.members(gen)?.len() * 2)
    }

    fn broadcast(
        &self,
        payload: &Payload<A, S, BRBDT::Op>,
        targets: BTreeSet<A>,
    ) -> Result<Vec<Packet<A, S, BRBDT::Op>>, Error<A, S, BRBDT::ValidationError>> {
        info!("[BRB] broadcasting {}->{:?}", self.actor(), targets);

        targets
            .into_iter()
            .map(|dest_p| self.send(dest_p, payload.clone()))
            .collect()
    }

    fn send(
        &self,
        dest: A,
        payload: Payload<A, S, BRBDT::Op>,
    ) -> Result<Packet<A, S, BRBDT::Op>, Error<A, S, BRBDT::ValidationError>> {
        let sig = self.sign(&payload)?;
        Ok(Packet {
            source: self.actor(),
            dest,
            payload,
            sig,
        })
    }

    fn sign(&self, data: impl Serialize) -> Result<S, Error<A, S, BRBDT::ValidationError>> {
        let bytes = bincode::serialize(&data)?;
        Ok(self.membership.id.sign(&bytes))
    }

    fn verify(
        &self,
        data: impl Serialize,
        signer: &A,
        sig: &S,
    ) -> Result<(), Error<A, S, BRBDT::ValidationError>> {
        let bytes = bincode::serialize(&data)?;
        signer.verify(&bytes, &sig)?;
        Ok(())
    }
}
