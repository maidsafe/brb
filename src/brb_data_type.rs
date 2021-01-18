use std::fmt::Debug;
use std::hash::Hash;

use serde::Serialize;

pub trait BRBDataType<A>: Debug {
    type Op: Debug + Clone + Hash + Eq + Serialize;
    type ValidationError: Debug + 'static;

    /// initialize a new replica of this datatype
    fn new(actor: A) -> Self;

    /// Protection against Byzantines
    fn validate(&self, source: &A, op: &Self::Op) -> Result<(), Self::ValidationError>;

    /// Executed once an op has been validated
    fn apply(&mut self, op: Self::Op);
}
