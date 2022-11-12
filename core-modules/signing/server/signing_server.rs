use curv::{arithmetic::Converter, elliptic::curves::Secp256k1, BigInt};
use dtrust::utils::init_app;
use multi_party_ecdsa::protocols::multi_party_ecdsa::gg_2020::state_machine::{
    keygen::{Keygen, LocalKey, ProtocolMessage},
    sign::{
        CompletedOfflineStage, OfflineProtocolMessage, OfflineStage, PartialSignature, SignManual,
    },
};

use round_based::{Msg, StateMachine};
use std::{
    io::{self, Read, Write},
    net::TcpStream,
};

// Handle all received messages
fn receive(socks: &mut Vec<TcpStream>, party: &mut Keygen, party_index: u16) {
    // Receive from to all other recipients
    for sender in 1..(socks.len() + 1) {
        let recipient = party_index as usize;
        if recipient != sender {
            let mut result_buf = [0; 18000]; // TODO: Figure out buffer size
            socks[sender - 1].read(&mut result_buf);

            // Deserialize message
            let received_msg = serde_json::from_str::<Msg<ProtocolMessage>>(
                &String::from_utf8_lossy(&result_buf).trim_matches(char::from(0)),
            )
            .unwrap();

            // Process received broadcast message
            party.handle_incoming(received_msg);
        }
    }
}

// Handle all received messages
fn receive_sign(
    socks: &mut Vec<TcpStream>,
    party: &mut OfflineStage,
    party_index: u16,
    num_threshold: u16,
) {
    // Receive from to all other recipients
    for sender in 1..(num_threshold + 1) {
        let recipient = party_index as usize;
        if recipient != sender as usize {
            let mut result_buf = [0; 18000]; // TODO: Figure out buffer size
            socks[sender as usize - 1].read(&mut result_buf);

            // Deserialize message
            let received_msg = serde_json::from_str::<Msg<OfflineProtocolMessage>>(
                &String::from_utf8_lossy(&result_buf).trim_matches(char::from(0)),
            )
            .unwrap();
            // Process received broadcast message
            party.handle_incoming(received_msg);
            println!("{:?} hello where we at", party);
        }
    }
}

// Broadcast message to all other parties
fn broadcast(
    msg_queue: &mut Vec<Msg<ProtocolMessage>>,
    socks: &mut Vec<TcpStream>,
    party: &mut Keygen,
    party_index: u16,
) {
    for msg in msg_queue.iter() {
        // Serialize message
        let serialized = serde_json::to_string(&msg).unwrap();

        // Send to all other recipients
        for recipient in 1..(socks.len() + 1) {
            let sender = party_index as usize;
            if recipient != sender {
                // Send message to recipient
                socks[recipient - 1].write(serialized.as_bytes());
            }
        }
    }
    receive(socks, party, party_index);
}

// Broadcast signature message to all other parties
// TODO: Consolidate with other broadcast function
fn broadcast_sign(
    msg_queue: &mut Vec<Msg<OfflineProtocolMessage>>,
    socks: &mut Vec<TcpStream>,
    party: &mut OfflineStage,
    party_index: u16,
    num_threshold: u16,
) {
    for msg in msg_queue.iter() {
        // Serialize message
        let serialized = serde_json::to_string(&msg).unwrap();

        // Send to all other recipients
        for recipient in 1..(num_threshold + 1) {
            let sender = party_index as usize;
            if recipient != sender as u16 {
                // Send message to recipient
                socks[recipient as usize - 1].write(serialized.as_bytes());
            }
        }
    }
    receive_sign(socks, party, party_index, num_threshold);
}

// Send message to one recipient
fn p2p(
    msg_queue: &mut Vec<Msg<ProtocolMessage>>,
    socks: &mut Vec<TcpStream>,
    party: &mut Keygen,
    party_index: u16,
) {
    for msg in msg_queue.iter() {
        // Serialize message
        let serialized = serde_json::to_string(&msg).unwrap();

        // Send to intended recipient
        let recipient = msg.receiver.unwrap() as usize;
        socks[recipient - 1].write(serialized.as_bytes());
    }

    receive(socks, party, party_index);
}

// Send message to one recipient
fn p2p_sign(
    msg_queue: &mut Vec<Msg<OfflineProtocolMessage>>,
    socks: &mut Vec<TcpStream>,
    party: &mut OfflineStage,
    party_index: u16,
    num_threshold: u16,
) {
    for msg in msg_queue.iter() {
        // Serialize message
        let serialized = serde_json::to_string(&msg).unwrap();

        // Send to intended recipient
        let recipient = msg.receiver.unwrap() as usize;
        socks[recipient - 1].write(serialized.as_bytes());
    }

    receive_sign(socks, party, party_index, num_threshold);
}

fn sign_message(
    msg_to_sign: BigInt,
    party_index: u16,
    offline_output: CompletedOfflineStage,
    num_threshold: u16,
    socks: &mut Vec<TcpStream>,
) {
    println!("got into the sign");
    // Obtain party's partial share
    let (manual_sign, partial_share) =
        SignManual::new(msg_to_sign.clone(), offline_output.clone()).unwrap();

    // Send to all other parties
    // Serialize message
    let serialized = serde_json::to_string(&partial_share).unwrap();

    // Send to all other recipients
    // TODO: Change to intended recipients
    for recipient in 1..(num_threshold + 1) {
        let sender = party_index;
        if recipient != sender {
            // Send message to recipient
            socks[recipient as usize - 1].write(serialized.as_bytes());
        }
    }

    let mut other_partial_shares = vec![];
    // Receive everyone else's partial signature shares
    for sender in 1..(num_threshold + 1) {
        let recipient = party_index;
        if recipient != sender {
            let mut result_buf = [0; 18000]; // TODO: Figure out buffer size
            socks[sender as usize - 1].read(&mut result_buf);

            // Deserialize message
            let received_share = serde_json::from_str::<PartialSignature>(
                &String::from_utf8_lossy(&result_buf).trim_matches(char::from(0)),
            )
            .unwrap();

            // Process received broadcast message
            other_partial_shares.push(received_share);
        }
    }

    let signature = manual_sign.complete(&other_partial_shares).unwrap();
    println!("{:#?}", signature);
}

fn keygen(
    num_parties: u16,
    num_threshold: u16,
    socks: &mut Vec<TcpStream>,
    party_index: u16,
) -> Result<Vec<u8>, serde_json::Error> {
    // Set up a party KeyGen state machine the current rank
    let mut party = Keygen::new(party_index, num_threshold, num_parties).unwrap();
    // Unsent messages sit in this queue each round
    let mut msg_queue = vec![];

    // Round 1
    party.proceed();
    msg_queue.push(party.message_queue()[0].clone());
    broadcast(&mut msg_queue, socks, &mut party, party_index);
    msg_queue.clear();

    // Round 2
    msg_queue.push(party.message_queue()[1].clone());
    broadcast(&mut msg_queue, socks, &mut party, party_index);
    party.proceed();

    // Round 3
    msg_queue.clear();
    for i in 0..num_parties - 1 {
        let msg_index = (i + 2) as usize;
        msg_queue.push(party.message_queue()[msg_index].clone());
    }

    p2p(&mut msg_queue, socks, &mut party, party_index);
    party.proceed();

    msg_queue.clear();
    msg_queue.push(party.message_queue()[(num_parties + 1) as usize].clone());

    broadcast(&mut msg_queue, socks, &mut party, party_index);
    party.proceed();

    let local_key = party.pick_output().unwrap().unwrap();

    serde_json::to_vec_pretty(&local_key)
}

fn sign(
    num_parties: u16,
    num_threshold: u16,
    key: LocalKey<Secp256k1>,
    socks: &mut Vec<TcpStream>,
    party_index: u16,
) {
    // If we have already reached our threshold, no need for another additional signer
    // TODO: Fix to online party indices
    if party_index > num_threshold {
        println!("I guess I'm not needed.");
        return;
    }
    let mut s_l = vec![];
    for i in 1..=num_threshold {
        s_l.push(i);
    }
    // println!("{:?}", party_indices);
    // Initiate offline phase
    // TODO: Comment signing protocol rounds
    let mut offline_stage = OfflineStage::new(party_index, s_l, key).unwrap();
    println!("offline_stage: {:?}", offline_stage);
    println!("{:?}", offline_stage.proceed());
    println!("Offline stage {:?}", offline_stage);
    let mut msg_queue = vec![];
    let msg = offline_stage.message_queue()[0].clone();
    msg_queue.push(msg);
    broadcast_sign(
        &mut msg_queue,
        socks,
        &mut offline_stage,
        party_index,
        num_threshold,
    );
    println!("{:?}", offline_stage.proceed());
    println!("Offline stage round 2 {:?}", offline_stage);
    msg_queue.clear();
    println!("{:?}", num_threshold);
    for i in 0..num_threshold - 1 {
        println!("{}", i);
        let msg_index = (i + 1) as usize;
        msg_queue.push(offline_stage.message_queue()[msg_index].clone());
    }
    p2p_sign(
        &mut msg_queue,
        socks,
        &mut offline_stage,
        party_index,
        num_threshold,
    );
    offline_stage.proceed();
    msg_queue.clear();
    let msg = offline_stage.message_queue()[(num_threshold) as usize].clone();
    msg_queue.push(msg);
    broadcast_sign(
        &mut msg_queue,
        socks,
        &mut offline_stage,
        party_index,
        num_threshold,
    );
    msg_queue.clear(); // TODO: Confirm this is correct
    offline_stage.proceed();
    let msg = offline_stage.message_queue()[(num_threshold + 1) as usize].clone();
    msg_queue.push(msg);
    broadcast_sign(
        &mut msg_queue,
        socks,
        &mut offline_stage,
        party_index,
        num_threshold,
    );
    msg_queue.clear();
    offline_stage.proceed();
    let msg = offline_stage.message_queue()[(num_threshold + 2) as usize].clone();
    msg_queue.push(msg);
    broadcast_sign(
        &mut msg_queue,
        socks,
        &mut offline_stage,
        party_index,
        num_threshold,
    );
    msg_queue.clear();
    offline_stage.proceed();
    let msg = offline_stage.message_queue()[(num_threshold + 3) as usize].clone();
    msg_queue.push(msg);
    broadcast_sign(
        &mut msg_queue,
        socks,
        &mut offline_stage,
        party_index,
        num_threshold,
    );
    offline_stage.proceed();
    msg_queue.clear();
    println!("Offline Stage: {:?}", offline_stage);

    // TODO: Add functionality for an actual message to sign
    let msg_bytes = b"hello";
    let msg_to_sign = &BigInt::from_bytes(msg_bytes);
    let offline_output = offline_stage.pick_output().unwrap().unwrap();
    sign_message(
        msg_to_sign.clone(),
        party_index,
        offline_output,
        num_threshold,
        socks,
    );
}

// TODO: Remove from tokio
#[tokio::main]
async fn main() -> io::Result<()> {
    let (rank, func_name, in_files, out_files, mut socks) = init_app()?;
    let mut param_buf = [0; 10];
    let mut f = &in_files[0];
    f.read(&mut param_buf)?;
    let param_str = String::from_utf8_lossy(&param_buf);
    let params: Vec<&str> = param_str.split(" ").collect();
    let num_parties = params[0].parse::<u16>().unwrap();
    let num_threshold = params[1]
        .trim_matches(char::from(0))
        .parse::<u16>()
        .unwrap();
    let party_index = (rank + 1) as u16;

    // Keygen
    if func_name == "keygen" {
        let key = keygen(num_parties, num_threshold, &mut socks, party_index)?;
        let mut f = out_files.first().unwrap();
        f.write(&key)?;

    // Signing
    } else if func_name == "signing" {
        let mut key_buf: String = "".to_string(); // TODO: Make reading and
        let mut f = &in_files[1];
        f.read_to_string(&mut key_buf).unwrap();
        let key = serde_json::from_str::<LocalKey<Secp256k1>>(&key_buf).unwrap();
        sign(num_parties, num_threshold, key, &mut socks, party_index);
    }
    Ok(())
}
