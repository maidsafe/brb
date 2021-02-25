use core::convert::Infallible;
use std::collections::BTreeSet;

use brb::{
    deterministic_brb::{Msg, Op},
    net::{Actor, Net},
    BRBDataType, Payload,
};
use crdts::Dot;

#[derive(Debug)]
struct TestDT {
    actor: Actor,
    set: BTreeSet<u8>,
}

impl BRBDataType<Actor> for TestDT {
    type Op = u8;
    type ValidationError = Infallible;

    fn new(actor: Actor) -> Self {
        let set = Default::default();
        TestDT { actor, set }
    }

    fn validate(&self, _source: &Actor, _op: &Self::Op) -> Result<(), Self::ValidationError> {
        Ok(())
    }

    fn apply(&mut self, op: Self::Op) {
        self.set.insert(op);
    }
}

type TestNet = Net<TestDT>;

#[test]
fn test_sender_receives_confirmation_after_member_applies_operation() -> Result<(), &'static str> {
    let mut net = TestNet::new();
    let actor_a = net.initialize_proc();
    let actor_b = net.initialize_proc();

    let a_proc = net.proc_mut(&actor_a).ok_or("No proc for actor_a")?;
    a_proc.force_join(actor_a);
    a_proc.force_join(actor_b);

    let b_proc = net.proc_mut(&actor_b).ok_or("No proc for actor_b")?;
    b_proc.force_join(actor_a);
    b_proc.force_join(actor_b);

    let packets = net
        .proc(&actor_a)
        .ok_or("No proc for actor_a")?
        .exec_op(32u8)
        .map_err(|_| "Failed to generate insert op")?;

    let expected_msg = Msg {
        gen: 0,
        op: 32u8,
        dot: Dot::new(actor_a, 1),
    };

    let expected_op = Op::RequestValidation {
        msg: expected_msg.clone(),
    };
    assert_eq!(
        packets
            .iter()
            .cloned()
            .filter_map(|packet| match packet.payload {
                Payload::BRB(msg) => Some(msg),
                _ => None,
            })
            .collect::<Vec<_>>(),
        vec![expected_op.clone(), expected_op]
    );

    let mut sig_packets = Vec::new();
    for packet in packets {
        sig_packets.extend(net.deliver_packet(packet));
    }

    assert_eq!(sig_packets.len(), 2); // Should recieve two signatures back.
    let sig_packet_1 = sig_packets.pop().ok_or("Failed to pop packet")?;
    let sig_packet_2 = sig_packets.pop().ok_or("Failed to pop packet")?;

    assert_eq!(net.deliver_packet(sig_packet_1), vec![]);
    let proof_of_agreement_packets = net.deliver_packet(sig_packet_2);
    assert_eq!(proof_of_agreement_packets.len(), 2);

    let mut delivery_confirmation_packets = vec![];
    for packet in proof_of_agreement_packets.clone() {
        let confirmed_delivered_packets = net.deliver_packet(packet);
        assert_eq!(confirmed_delivered_packets.len(), 1);
        assert_eq!(
            confirmed_delivered_packets
                .iter()
                .cloned()
                .filter_map(|p| match p.payload {
                    Payload::BRB(op) => Some(op),
                    _ => None,
                })
                .collect::<Vec<_>>(),
            vec![Op::Delivered {
                msg: expected_msg.clone()
            }]
        );
        delivery_confirmation_packets.extend(confirmed_delivered_packets);
    }

    assert_eq!(delivery_confirmation_packets.len(), 2);
    let delivery_packet_1 = delivery_confirmation_packets
        .pop()
        .ok_or("Failed to pop delivery packet")?;
    let delivery_packet_2 = delivery_confirmation_packets
        .pop()
        .ok_or("Failed to pop delivery packet")?;

    // If we resend any pending deliveries now, they should match the proof of agreement packets
    // we saw previously.
    assert_eq!(
        net.proc(&actor_a)
            .ok_or("No proc for actor_a")?
            .resend_pending_deliveries()
            .map_err(|_| "Failed to resend pending deliveries")?,
        proof_of_agreement_packets
    );
    assert_eq!(net.deliver_packet(delivery_packet_1.clone()), vec![]);

    // Now, we should only resend the PoA for the one packet we did not receive a delivery packet from.
    assert_eq!(
        net.proc(&actor_a)
            .ok_or("No proc for actor_a")?
            .resend_pending_deliveries()
            .map_err(|_| "Failed to resend pending deliveries")?,
        proof_of_agreement_packets
            .into_iter()
            .filter(|p| p.dest != delivery_packet_1.source)
            .collect::<Vec<_>>()
    );

    assert_eq!(net.deliver_packet(delivery_packet_2), vec![]);

    assert_eq!(
        net.proc(&actor_a)
            .ok_or("No proc for actor_a")?
            .resend_pending_deliveries()
            .map_err(|_| "Failed to resend pending deliveries")?,
        vec![]
    );

    // Make sure we actually arrived at the correct final state.
    assert!(net.members_are_in_agreement());
    assert_eq!(
        net.proc(&actor_a).ok_or("No proc for actor_a")?.dt.set,
        vec![32u8].into_iter().collect()
    );

    Ok(())
}
