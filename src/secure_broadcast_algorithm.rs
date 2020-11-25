use std::fmt::Debug;
use std::hash::Hash;

use serde::{Serialize, de::DeserializeOwned};

use crate::Actor;

pub trait SecureBroadcastAlgorithm: Clone + Debug + Eq + Send {
    type Op: Debug + Clone + Hash + Eq + Serialize + DeserializeOwned + Send;
    type ReplicatedState: Clone + Debug + Eq;

    /// initialize a new replica of this algorithm
    fn new(actor: Actor) -> Self;

    fn state(&self) -> Self::ReplicatedState;

    /// Called when onboarding a new replica of this algorithm
    fn sync_from(&mut self, other: Self::ReplicatedState);

    /// Protection against Byzantines
    fn validate(&self, from: &Actor, op: &Self::Op) -> bool;

    /// Executed once an op has been validated
    fn apply(&mut self, op: Self::Op);
}
