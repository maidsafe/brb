// Copyright 2021 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under the MIT license <LICENSE-MIT
// http://opensource.org/licenses/MIT> or the Modified BSD license <LICENSE-BSD
// https://opensource.org/licenses/BSD-3-Clause>, at your option. This file may not be copied,
// modified, or distributed except according to those terms. Please review the Licences for the
// specific language governing permissions and limitations relating to use of the SAFE Network
// Software.

//! BRB - Byzantine Reliable Broadcast

#![deny(missing_docs)]

// re-export these
pub use brb_membership as membership;
pub use brb_membership::{Actor, Error as MembershipError, Sig, SigningActor};

pub mod deterministic_brb;
pub use deterministic_brb::DeterministicBRB;

pub mod error;
pub use error::{Error, ValidationError};

pub mod net;

pub mod packet;
pub use packet::{Packet, Payload};

pub mod brb_data_type;
pub use brb_data_type::BRBDataType;
