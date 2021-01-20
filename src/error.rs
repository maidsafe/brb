// Copyright 2021 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under the MIT license <LICENSE-MIT
// http://opensource.org/licenses/MIT> or the Modified BSD license <LICENSE-BSD
// https://opensource.org/licenses/BSD-3-Clause>, at your option. This file may not be copied,
// modified, or distributed except according to those terms. Please review the Licences for the
// specific language governing permissions and limitations relating to use of the SAFE Network
// Software.

//! Provides BRB specific errors.

use std::collections::BTreeSet;

use brb_membership::signature;
use brb_membership::{Actor, Generation, Sig};
use crdts::Dot;
use thiserror::Error;

use core::fmt;
use std::error;

/// Enumerates the error conditions that can occur during BRB processing.
#[derive(Error, Debug)]
pub enum Error<A: Actor<S> + 'static, S: Sig + 'static, V: fmt::Debug + error::Error + 'static> {
    /// error while processing membership change
    #[error("error while processing membership change")]
    Membership(#[from] brb_membership::Error<A, S>),

    /// Failed to serialize all or part of a packet
    #[error("Failed to serialize all or part of a packet")]
    Encoding(#[from] bincode::Error),

    /// Packet failed validation
    #[error("Packet failed validation")]
    Validation(#[from] ValidationError<A, S, V>),

    /// Failure when working with signature
    #[error("Failure when working with signature")]
    Signature(#[from] signature::Error),
}

/// Enumerates types of packet validation errors.
///
/// Note that all of these errors are generated within the BRB module
/// itself with the exception of DataTypeFailedValidation, which occurs
/// when a BRBDataType validation fails according to its own internal logic.
#[derive(Error, Debug)]
pub enum ValidationError<
    A: Actor<S> + 'static,
    S: Sig + 'static,
    V: fmt::Debug + error::Error + 'static,
> {
    /// The actor who sent this packet is different from the actor who incremented the dot
    #[error("The actor `{from}` who sent this packet is different from the actor who incremented the dot: `{dot:?}`")]
    PacketSourceIsNotDot {
        /// actor who sent the packet
        from: A,
        /// the associated dot
        dot: Dot<A>,
    },

    /// The dot in this message is out of order
    #[error("The dot in this message `{msg_dot:?}` is out of order (expected: {expected_dot:?})")]
    MsgDotNotTheNextDot {
        /// dot of the message
        msg_dot: Dot<A>,
        /// dot that was expected
        expected_dot: Dot<A>,
    },

    /// The source of this message already has a pending message, we can not start a new operation until the first one has completed
    #[error("The source of this message already has a pending message, we can not start a new operation until the first one has completed")]
    SourceAlreadyHasPendingMsg {
        /// dot of the message
        msg_dot: Dot<A>,
        /// dot of next delivery
        next_deliver_dot: Dot<A>,
    },

    /// This message is not from this generation
    #[error("This message is not from this generation {msg_gen} (expected: {gen})")]
    MessageFromDifferentGeneration {
        /// generation of the message
        msg_gen: Generation,
        /// present generation
        gen: Generation,
    },

    /// Source is not a voting member
    #[error("Source is not a voting member ({from:?} not in {members:?})")]
    SourceIsNotVotingMember {
        /// actor that proposed the action
        from: A,
        /// voting members
        members: BTreeSet<A>,
    },

    /// the datatype failed to validate the operation
    #[error("the datatype failed to validate the operation")]
    DataTypeFailedValidation(V),

    /// Signature is invalid
    #[error("Signature is invalid")]
    InvalidSignature,

    /// We received a SignedValidated packet for a message we did not request
    #[error("We received a SignedValidated packet for a message we did not request")]
    SignedValidatedForPacketWeDidNotRequest,

    /// Message dot to be applied is not the next message to be delivered
    #[error("Message dot {msg_dot:?} to be applied is not the next message to be delivered (expected: {expected_dot:?}")]
    MsgDotNotNextDotToBeDelivered {
        /// the dot in the msg
        msg_dot: Dot<A>,
        /// the dot we are expecting
        expected_dot: Dot<A>,
    },

    /// The proof did not contain enough signatures to form quorum
    #[error("The proof did not contain enough signatures to form quorum")]
    NotEnoughSignaturesToFormQuorum,

    /// Proof contains signatures from non-members
    #[error("Proof contains signatures from non-members")]
    ProofContainsSignaturesFromNonMembers,

    /// Proof contains invalid signatures
    #[error("Proof contains invalid signatures")]
    ProofContainsInvalidSignatures,

    /// Phantom, unused.
    #[error("This variant is only here to satisfy the type checker (we need to use S in a field)")]
    PhantomSig(core::marker::PhantomData<S>),
}
