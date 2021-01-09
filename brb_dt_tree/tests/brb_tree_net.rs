#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, HashMap, HashSet};

    use crdt_tree::{Clock, OpMove, State, Tree, TreeNode};
    use quickcheck::{self, TestResult};

    use brb::{Actor, Error, MembershipError, Net, Packet};
    use brb_dt_tree::BRBTree;

    type TestTree = BRBTree<u8, String>;

    fn bootstrap_net(net: &mut Net<TestTree>, n_procs: u8) {
        let genesis_actor = net.initialize_proc();
        net.on_proc_mut(&genesis_actor, |p| p.force_join(genesis_actor))
            .unwrap();

        // 1 proc was taken by the genesis, so subtract 1
        for _ in 0..(n_procs - 1) {
            let actor = net.initialize_proc();
            net.on_proc_mut(&actor, |p| p.force_join(genesis_actor));
            let packets = net
                .on_proc_mut(&genesis_actor, |p| p.request_membership(actor).unwrap())
                .unwrap();
            net.run_packets_to_completion(packets);
            net.anti_entropy();
        }

        assert_eq!(net.members(), net.actors());
        assert!(net.members_are_in_agreement());
    }

    #[test]
    fn test_sequential_ops_run_cuncurrently() {
        let mut net = Net::new();
        bootstrap_net(&mut net, 1);
        let actor = net.members().into_iter().next().unwrap();
        let mut clock: Clock<Actor> = Clock::<Actor>::new(actor, None);

        let root_id = 255;

        // Initiate the signing round DSB but don't deliver signatures
        let pending_packets = net
            .on_proc(&actor, |proc| {
                proc.exec_op(
                    proc.dt
                        .opmove(clock.tick(), root_id, "Hello".to_string(), 1),
                )
                .unwrap()
            })
            .unwrap()
            .into_iter()
            .flat_map(|p| net.deliver_packet(p))
            .collect::<Vec<_>>();

        // Initiate the signing round again but for a different op (adding World instead of Hello)
        let invalid_pending_packets_cnt = net
            .on_proc(&actor, |proc| {
                proc.exec_op(
                    proc.dt
                        .opmove(clock.tick(), root_id, "World".to_string(), 2),
                )
                .unwrap()
            })
            .unwrap()
            .into_iter()
            .flat_map(|p| net.deliver_packet(p))
            .count();

        assert_eq!(net.count_invalid_packets(), 1);
        assert_eq!(invalid_pending_packets_cnt, 0);

        net.run_packets_to_completion(pending_packets);

        assert!(net.members_are_in_agreement());

        assert_eq!(
            net.on_proc(&actor, |p| p.dt.treestate().tree().num_nodes()),
            Some(1)
        );
    }

    #[test]
    fn test_concurrent_op_and_member_change() {
        let mut net = Net::new();
        bootstrap_net(&mut net, 3);
        let mut members = net.members().into_iter();
        let (a, b, c) = (
            members.next().unwrap(),
            members.next().unwrap(),
            members.next().unwrap(),
        );
        let mut clock_a: Clock<Actor> = Clock::<Actor>::new(a, None);

        // let value_to_add = 32;
        let root_id = 255;
        let child_id = 1;

        // initiating process 'a' broadcasts requests for validation
        let req_for_valid_packets = net
            .on_proc(&a, |p| {
                p.exec_op(p.dt.opmove(clock_a.tick(), root_id, "32".to_string(), child_id))
                    .unwrap()
            })
            .unwrap();

        // we deliver these packets to destinations
        // and collect responses with signatures
        let signed_validated_packets: Vec<_> = req_for_valid_packets
            .into_iter()
            .flat_map(|p| net.deliver_packet(p))
            .collect();

        // signatures are delivered back to 'a' who then procedes to
        // broadcast the proof of agreement back to the network.
        let proofs_packets = signed_validated_packets
            .into_iter()
            .flat_map(|p| net.deliver_packet(p))
            .collect();

        // hold onto the proofs, don't deliver them till we've removed a few members
        let packets_b = net.on_proc_mut(&b, |p| p.kill_peer(b).unwrap()).unwrap();
        net.run_packets_to_completion(packets_b);
        let packets_c = net.on_proc_mut(&c, |p| p.kill_peer(c).unwrap()).unwrap();
        net.run_packets_to_completion(packets_c);
        net.run_packets_to_completion(proofs_packets);

        assert!(net.members_are_in_agreement());
        assert!(net
            .on_proc(&a, |p| p.dt.treestate().tree().find(&child_id).is_some())
            .unwrap());
    }

    quickcheck::quickcheck! {
        fn prop_ops_show_up_on_read(n_procs: u8, members: Vec<u8>) -> TestResult {
            if n_procs == 0 || n_procs > 7 || members.len() > 10 {
                return TestResult::discard();
            }

            let root_id = 255;
            let mut clocks: HashMap<Actor, Clock<Actor>> = Default::default();

            let mut net: Net<TestTree> = Net::new();
            bootstrap_net(&mut net, n_procs);

            let actors_loop = net.actors().into_iter().collect::<Vec<_>>().into_iter().cycle();
            for (actor, member) in actors_loop.zip(members.clone().into_iter()) {
                let clock = clocks
                    .entry(actor)
                    .or_insert_with(|| Clock::<Actor>::new(actor, None));

                net.run_packets_to_completion(
                    net.on_proc(&actor, |p| p.exec_op(p.dt.opmove(clock.tick(), root_id, member.to_string(), member)).unwrap()).unwrap()
                )
            }

            assert!(net.members_are_in_agreement());

            // Next we will verify that all the members are actually in the tree.

            // Obtain a HashSet of members from the tree, where each node's child_id is a member.
            let tree_set: HashSet<_> = net.on_proc(
                &net.actors().into_iter().next().unwrap(),
                |p| p.dt.treestate().tree()
                        .clone()
                        .into_iter()
                        .filter(|(child_id, _)| *child_id != root_id )  // exclude root.  members are its children.
                        .map(|(child_id, _)|child_id)  // we only care about node id for this test
                        .into_iter()
                        .collect::<HashSet<_>>()

                // Alternative to above line.  avoids clone() of tree, but is less idiomatic.
                // |p| { let mut ts: HashSet<u8> = Default::default();
                //       p.dt.treestate().tree().walk(&root_id, |_, id, _| if *id != root_id {ts.insert(*id);});
                //       ts }
            ).unwrap();

            assert_eq!(members.into_iter().collect::<HashSet<_>>(),
                      tree_set);

            TestResult::passed()
        }


        fn prop_ops_behave_as_tree(n_procs: u8, members: Vec<u8>) -> TestResult {
            if n_procs == 0 || n_procs > 7 || members.len() > 10 {
                return TestResult::discard();
            }

            let root_id = 255;

            let mut net: Net<TestTree> = Net::new();
            bootstrap_net(&mut net, n_procs);

            // Model testing against the HashSet
            let mut model: Tree<u8, String> = Tree::new();
            let mut clocks: HashMap<Actor, Clock<Actor>> = Default::default();

            let actors_loop = net.actors().into_iter().collect::<Vec<_>>().into_iter().cycle();
            for (actor, member) in actors_loop.zip(members.into_iter()) {
                let clock = clocks
                    .entry(actor)
                    .or_insert_with(|| Clock::<Actor>::new(actor, None));

                model.add_node(member, TreeNode::new(root_id, member.to_string()));
                net.run_packets_to_completion(
                    net.on_proc(&actor, |p| p.exec_op(p.dt.opmove((*clock).tick(), root_id, member.to_string(), member)).unwrap()).unwrap()
                )
            }

            assert!(net.members_are_in_agreement());

            let treestate: State<_, _, _> = net.on_proc(
                &net.actors().into_iter().next().unwrap(),
                |p| p.dt.treestate().clone()
            ).unwrap();

            assert_eq!(model, *treestate.tree());

            TestResult::passed()
        }

        fn prop_interpreter(instructions: Vec<(u8, u8, u8)>) -> TestResult {
            if instructions.len() > 12 {
                return TestResult::discard();
            }

            println!("------");
            println!("instr: {:?}", instructions);
            let mut net: Net<TestTree> = Net::new();
            let genesis_actor = net.initialize_proc();
            net.on_proc_mut(&genesis_actor, |p| p.force_join(genesis_actor)).unwrap();

            let mut packet_queues: BTreeMap<(Actor, Actor), Vec<Packet<_>>> = Default::default();
            let mut model: State<u8, String, Actor> = Default::default();
            let mut clocks_model: HashMap<Actor, Clock<Actor>> = Default::default();
            let mut clocks_net: HashMap<Actor, Clock<Actor>> = Default::default();

            let root_id = 0;

            let mut blocked: HashSet<Actor> = Default::default();

            for mut instr in instructions {
                let members: Vec<_> = net.members().into_iter().collect();
                instr.0 %= 5;
                match instr {
                    (0, queue_idx, _)  if !packet_queues.is_empty() => {
                        // deliver packet
                        let queue = packet_queues.keys().nth(queue_idx as usize % packet_queues.len()).cloned().unwrap();
                        let packets = packet_queues.entry(queue).or_default();
                        if !packets.is_empty() {
                            let packet = packets.remove(0);

                            if packet.payload.is_proof_of_agreement() {
                                // we are completing the transaction, the source is no longer blocked
                                assert!(blocked.remove(&packet.source));
                            }

                            for resp_packet in net.deliver_packet(packet) {
                                let queue = (resp_packet.source, resp_packet.dest);
                                packet_queues
                                    .entry(queue)
                                    .or_default()
                                    .push(resp_packet)
                            }
                        }
                    }
                    (1, _, _) if net.actors().len() < 7 => {
                        // add peer
                        let actor = net.initialize_proc();
                        net.on_proc_mut(&actor, |p| p.force_join(genesis_actor));
                        net.run_packets_to_completion(vec![net.on_proc(&actor, |p| p.anti_entropy(genesis_actor).unwrap()).unwrap()]);
                    }
                    (2, actor_idx, _) if !members.is_empty() => {
                        // request membership
                        let actor = members[actor_idx as usize % members.len()];
                        if blocked.contains(&actor) {continue};
                        blocked.insert(actor);

            let join_request_resp = net.on_proc_mut(&genesis_actor, |p| p.request_membership(actor)).unwrap();
            match join_request_resp {
                Ok(packets) => {
                        for packet in packets {
                            for resp_packet in net.deliver_packet(packet) {
                                let queue = (resp_packet.source, resp_packet.dest);
                                packet_queues
                                    .entry(queue)
                                    .or_default()
                                    .push(resp_packet)
                            }
                        }
                }
                Err(Error::Membership(MembershipError::JoinRequestForExistingMember {..})) => {
                assert!(net.on_proc(&genesis_actor, |p| p.peers().unwrap()).unwrap().contains(&actor));
                },
                e => panic!("Unexpected error {:?}", e)
            }
                    }
                    (3, actor_idx, v) if !members.is_empty() => {
                        // move v
                        let actor = members[actor_idx as usize % members.len()];
                        if blocked.contains(&actor) {continue};
                        blocked.insert(actor);

                        let clock_model = clocks_model
                            .entry(actor)
                            .or_insert_with(|| Clock::<Actor>::new(actor, None));
                        let clock_net = clocks_net
                            .entry(actor)
                            .or_insert_with(|| Clock::<Actor>::new(actor, None));

                        let op = OpMove::new((*clock_model).tick(), 0, v.to_string(), v);
                        model.apply_op(op);

                        for packet in net.on_proc(&actor, |p| p.exec_op(p.dt.opmove((*clock_net).tick(), root_id, v.to_string(), v)).unwrap()).unwrap() {
                            for resp_packet in net.deliver_packet(packet) {
                                let queue = (resp_packet.source, resp_packet.dest);
                                packet_queues
                                    .entry(queue)
                                    .or_default()
                                    .push(resp_packet)
                            }
                        }
                    }
                    (4, actor_idx, target_actor_idx) if !members.is_empty() => {
                        // kill peer
                        let actor = members[actor_idx as usize % members.len()];
                        if blocked.contains(&actor) {continue};
                        blocked.insert(actor);
                        let target_actor = members[target_actor_idx as usize % members.len()];
                        for packet in net.on_proc_mut(&actor, |p| p.kill_peer(target_actor).unwrap()).unwrap() {
                            for resp_packet in net.deliver_packet(packet) {
                                let queue = (resp_packet.source, resp_packet.dest);
                                packet_queues
                                    .entry(queue)
                                    .or_default()
                                    .push(resp_packet)
                            }
                        }
                    }
                    _ => (),
                }
            }

            println!("--- draining packet queues ---");
            for (_queue, packets) in packet_queues {
                net.run_packets_to_completion(packets);
            }

            assert!(net.members_are_in_agreement());

            assert_eq!(
                net.on_proc(&genesis_actor, |p| {
                    p.dt.treestate().clone()
                }),
                Some(model)
            );

            TestResult::passed()
        }
    }
}
