// Copyright 2021 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under the MIT license <LICENSE-MIT
// http://opensource.org/licenses/MIT> or the Modified BSD license <LICENSE-BSD
// https://opensource.org/licenses/BSD-3-Clause>, at your option. This file may not be copied,
// modified, or distributed except according to those terms. Please review the Licences for the
// specific language governing permissions and limitations relating to use of the SAFE Network
// Software.

//! BRB uses Packet and Payload types that are not specific to any network transport
//! layer such as tcp/ip. As such, BRB may easily be adapted to work over various
//! transports.

use serde::{Deserialize, Serialize};

use crate::deterministic_brb;
use crate::{Actor, Sig};

/// Represents a logical message packet with a BRB specific payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Packet<A: Actor<S>, S: Sig, Op> {
    /// source actor
    pub source: A,
    /// destination actor
    pub dest: A,
    /// payload data
    pub payload: Payload<A, S, Op>,
    /// signature of payload data by source actor
    pub sig: S,
}

/// Enumerates types of BRB data that may be included in a Packet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Payload<A: Actor<S>, S: Sig, DataTypeOp> {
    /// Represents an AntiEntropy request
    AntiEntropy {
        /// last-seen generation
        generation: brb_membership::Generation,
        /// delivered clock
        delivered: crdts::VClock<A>,
    },
    /// Represents a BRB operation
    BRB(deterministic_brb::Op<A, S, DataTypeOp>),
    // Box to avoid https://rust-lang.github.io/rust-clippy/master/index.html#large_enum_variant
    /// Represents a brb_membership Vote
    Membership(Box<brb_membership::Vote<A, S>>),
}
