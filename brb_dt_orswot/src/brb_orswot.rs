use crdts::{orswot, CmRDT};
use std::cmp::Ordering;
use std::{fmt::Debug, hash::Hash};

use brb::{Actor, BRBDataType};

use serde::Serialize;

#[derive(Debug, Serialize, PartialEq, Eq, Clone)]
pub struct BRBOrswot<M: Clone + Eq + Debug + Hash + Serialize> {
    actor: Actor,
    orswot: orswot::Orswot<M, Actor>,
}

impl<M: Clone + Eq + Debug + Hash + Serialize> BRBOrswot<M> {
    pub fn add(&self, member: M) -> orswot::Op<M, Actor> {
        let add_ctx = self.orswot.read_ctx().derive_add_ctx(self.actor);
        self.orswot.add(member, add_ctx)
    }

    pub fn rm(&self, member: M) -> orswot::Op<M, Actor> {
        let rm_ctx = self.orswot.read_ctx().derive_rm_ctx();
        self.orswot.rm(member, rm_ctx)
    }

    pub fn contains(&self, member: &M) -> bool {
        self.orswot.contains(member).val
    }

    pub fn actor(&self) -> &Actor {
        &self.actor
    }

    pub fn orswot(&self) -> &orswot::Orswot<M, Actor> {
        &self.orswot
    }
}

impl<M: Clone + Eq + Debug + Hash + Serialize> BRBDataType for BRBOrswot<M> {
    type Op = orswot::Op<M, Actor>;

    fn new(actor: Actor) -> Self {
        BRBOrswot {
            actor,
            orswot: orswot::Orswot::new(),
        }
    }

    fn validate(&self, from: &Actor, op: &Self::Op) -> bool {
        match op {
            orswot::Op::Add { dot, members: _ } => {
                if &dot.actor != from {
                    println!(
                        "[ORSWOT/INVALID] Attempting to add with a dot different from the source proc"
                    );
                    false
                } else {
                    true
                }
            }
            orswot::Op::Rm { clock, members } => {
                if members.len() != 1 {
                    println!("[ORSWOT/INVALID] We only support removes of a single element");
                    false
                } else if matches!(
                    clock.partial_cmp(&self.orswot.clock()),
                    None | Some(Ordering::Greater)
                ) {
                    // NOTE: this check renders all the "deferred_remove" logic in the ORSWOT obsolete.
                    //       The deferred removes would buffer these out-of-order removes.
                    println!("[ORSWOT/INVALID] This rm op is removing data we have not yet seen");
                    false
                } else {
                    true
                }
            }
        }
    }

    fn apply(&mut self, op: Self::Op) {
        self.orswot.apply(op);
    }
}
