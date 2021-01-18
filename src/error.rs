use std::collections::BTreeSet;

use brb_membership::signature;
use brb_membership::{Actor, Generation, Sig};
use crdts::Dot;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error<A: Actor<S> + 'static, S: Sig + 'static, V: core::fmt::Debug + 'static> {
    #[error("error while processing membership change")]
    Membership(#[from] brb_membership::Error<A, S>),
    #[error("Failed to serialize all or part of a packet")]
    Encoding(#[from] bincode::Error),
    #[error("Packet failed validation")]
    Validation(#[from] ValidationError<A, S, V>),
    #[error("Failure when working with signature")]
    Signature(#[from] signature::Error),
}

#[derive(Error, Debug)]
pub enum ValidationError<A: Actor<S> + 'static, S: Sig + 'static, V: core::fmt::Debug> {
    #[error("The actor `{from}` who sent this packet is different from the actor who incremented the dot: `{dot:?}`")]
    PacketSourceIsNotDot { from: A, dot: Dot<A> },
    #[error("The dot in this message `{msg_dot:?}` is out of order (expected: {expected_dot:?})")]
    MsgDotNotTheNextDot {
        msg_dot: Dot<A>,
        expected_dot: Dot<A>,
    },
    #[error("The source of this message already has a pending message, we can not start a new operation until the first one has completed")]
    SourceAlreadyHasPendingMsg {
        msg_dot: Dot<A>,
        next_deliver_dot: Dot<A>,
    },
    #[error("This message is not from this generation {msg_gen} (expected: {gen})")]
    MessageFromDifferentGeneration {
        msg_gen: Generation,
        gen: Generation,
    },
    #[error("Source is not a voting member ({from:?} not in {members:?})")]
    SourceIsNotVotingMember { from: A, members: BTreeSet<A> },
    #[error("the datatype failed to validated the operation")]
    DataTypeFailedValidation(V),
    #[error("Signature is invalid")]
    InvalidSignature,
    #[error("We received a SignedValidated packet for a message we did not request")]
    SignedValidatedForPacketWeDidNotRequest,
    #[error("Message dot {msg_dot:?} to be applied is not the next message to be delivered (expected: {expected_dot:?}")]
    MsgDotNotNextDotToBeDelivered {
        msg_dot: Dot<A>,
        expected_dot: Dot<A>,
    },
    #[error("The proof did not contain enough signatures to form quorum")]
    NotEnoughSignaturesToFormQuorum,
    #[error("Proof contains signatures from non-members")]
    ProofContainsSignaturesFromNonMembers,
    #[error("Proof contains invalid signatures")]
    ProofContainsInvalidSignatures,
    #[error("This variant is only here to satisfy the type checker (we need to use S in a field)")]
    PhantomSig(core::marker::PhantomData<S>),
}
