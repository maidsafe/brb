// Copyright 2021 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under the MIT license <LICENSE-MIT
// http://opensource.org/licenses/MIT> or the Modified BSD license <LICENSE-BSD
// https://opensource.org/licenses/BSD-3-Clause>, at your option. This file may not be copied,
// modified, or distributed except according to those terms. Please review the Licences for the
// specific language governing permissions and limitations relating to use of the SAFE Network
// Software.

//! The purpose of BRB is to provide a BFT transport for CRDT-esque Data Types.
//!
//! The BRBDataType trait defines the contract such Data Types must fulfill.
//! Typically, an existing CRDT algorithm wiil be wrapped by a struct that
//! implements this trait.
//!
//! Examples of types that implement BRBDataType:
//!   brb_dt_orswot, brb_dt_at2, brb_dt_tree

use std::error::Error;
use std::fmt::Debug;
use std::hash::Hash;

use serde::Serialize;

/// The BRBDataType trait
pub trait BRBDataType<A>: Debug {
    /// The set of ops this data type accepts
    type Op: Debug + Clone + Hash + Eq + Serialize;

    /// A validation error specific to this data type.
    type ValidationError: Debug + Error + 'static;

    /// initialize a new replica of this datatype
    fn new(actor: A) -> Self;

    /// Protection against Byzantines
    /// Validate any incoming operations, here you must perform your byzantine fault
    /// tolerance checks specific to your algorithm    
    fn validate(&self, source: &A, op: &Self::Op) -> Result<(), Self::ValidationError>;

    /// Execute an op after it has been validated.
    fn apply(&mut self, op: Self::Op);
}
