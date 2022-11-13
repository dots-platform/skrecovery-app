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
    active_parties: &Vec<u16>
) {
    // Receive from to all other recipients
    for sender in active_parties {
        let recipient = party_index as usize;
        if recipient != *sender as usize {
            let mut result_buf = [0; 18000]; // TODO: Figure out buffer size
            socks[*sender as usize - 1].read(&mut result_buf);

            // Deserialize message
            let received_msg = serde_json::from_str::<Msg<OfflineProtocolMessage>>(
                &String::from_utf8_lossy(&result_buf).trim_matches(char::from(0)),
            )
            .unwrap();
            // Process received broadcast message
            party.handle_incoming(received_msg);
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
    active_parties: &Vec<u16>
) {
    for msg in msg_queue.iter() {
        // Serialize message
        let serialized = serde_json::to_string(&msg).unwrap();

        // Send to all other recipients
        for recipient in active_parties {
            let sender = party_index as usize;
            if *recipient != sender as u16 {
                // Send message to recipient
                socks[*recipient as usize - 1].write(serialized.as_bytes());
            }
        }
    }
    receive_sign(socks, party, party_index, &active_parties);
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
    active_parties: &Vec<u16>
) {
    for msg in msg_queue.iter() {
        // Serialize message
        let serialized = serde_json::to_string(&msg).unwrap();

        // Send to intended recipient
        let recipient = msg.receiver.unwrap() as usize;
        socks[recipient - 1].write(serialized.as_bytes());
    }

    receive_sign(socks, party, party_index, active_parties);
}

fn sign_message(
    msg_to_sign: BigInt,
    party_index: u16,
    offline_output: CompletedOfflineStage,
    socks: &mut Vec<TcpStream>,
    active_parties: &Vec<u16>
) -> Result<Vec<u8>, serde_json::Error> {
    // Obtain party's partial share
    let (manual_sign, partial_share) =
        SignManual::new(msg_to_sign.clone(), offline_output.clone()).unwrap();

    // Send to all other parties
    // Serialize message
    let serialized = serde_json::to_string(&partial_share).unwrap();

    // Send to all other recipients
    for recipient in active_parties {
        let sender = party_index;
        if *recipient != sender {
            // Send message to recipient
            socks[*recipient as usize - 1].write(serialized.as_bytes());
        }
    }

    let mut other_partial_shares = vec![];
    // Receive everyone else's partial signature shares
    for sender in active_parties {
        let recipient = party_index;
        if recipient != *sender {
            let mut result_buf = [0; 18000]; // TODO: Figure out buffer size
            socks[*sender as usize - 1].read(&mut result_buf);

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
    serde_json::to_vec_pretty(&signature)
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
    active_parties: Vec<u16>,
    key: LocalKey<Secp256k1>,
    socks: &mut Vec<TcpStream>,
    party_index: u16,
    message: Vec<u8>
) -> Result<Vec<u8>, serde_json::Error> {
    if !active_parties.contains(&party_index) {
        println!("I guess I'm not needed.");
        return serde_json::to_vec_pretty("")
    }
    // Initiate offline phase
    // TODO: Comment signing protocol rounds
    let mut offline_stage = OfflineStage::new(party_index, active_parties.clone(), key).unwrap();
    offline_stage.proceed();
    let mut msg_queue = vec![];
    let msg = offline_stage.message_queue()[0].clone();
    msg_queue.push(msg);
    broadcast_sign(
        &mut msg_queue,
        socks,
        &mut offline_stage,
        party_index,
        num_threshold,
        &active_parties
    );
    offline_stage.proceed();
    msg_queue.clear();
    for i in 0..num_threshold {
        let msg_index = (i + 1) as usize;
        msg_queue.push(offline_stage.message_queue()[msg_index].clone());
    }
    p2p_sign(
        &mut msg_queue,
        socks,
        &mut offline_stage,
        party_index,
        &active_parties
    );
    offline_stage.proceed();
    msg_queue.clear();
    let msg = offline_stage.message_queue()[(num_threshold + 1) as usize].clone();
    msg_queue.push(msg);
    broadcast_sign(
        &mut msg_queue,
        socks,
        &mut offline_stage,
        party_index,
        num_threshold,
        &active_parties
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
        &active_parties
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
        &active_parties
    );
    msg_queue.clear();
    offline_stage.proceed();
    let msg = offline_stage.message_queue()[(num_threshold + 4) as usize].clone();
    msg_queue.push(msg);
    broadcast_sign(
        &mut msg_queue,
        socks,
        &mut offline_stage,
        party_index,
        num_threshold,
        &active_parties
    );
    offline_stage.proceed();
    msg_queue.clear();

    let message_int = &BigInt::from_bytes(&message);
    let offline_output = offline_stage.pick_output().unwrap().unwrap();
    sign_message(
        message_int.clone(),
        party_index,
        offline_output,
        socks,
        &active_parties
    )
}

fn main() -> io::Result<()> {
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
        let mut f = &out_files[0];
        f.write(&key)?;

    // Signing
    } else if func_name == "signing" {
        let active_parties_str : Vec<&str> = params[2].split(",").collect();
        let mut active_parties : Vec<u16> = vec![];
        for party in active_parties_str {
            active_parties.push(party.trim_matches(char::from(0)).parse::<u16>().unwrap());
        }

        let mut key_buf: String = "".to_string(); // TODO: Make reading and
        let mut key_file = &in_files[1];
        key_file.read_to_string(&mut key_buf).unwrap();
        let key = serde_json::from_str::<LocalKey<Secp256k1>>(&key_buf).unwrap();
        
        let mut msg_buf: Vec<u8> = [0; 100].to_vec();
        let mut msg_file = &in_files[2];
        msg_file.read(&mut msg_buf)?;
        let msg_buf = String::from_utf8_lossy(&msg_buf).trim_matches(char::from(0)).as_bytes().to_vec();
        
        let signature = sign(num_parties, num_threshold, active_parties, key, &mut socks, party_index, msg_buf)?;
        let mut f = &out_files[0];
        f.write(&signature)?;
    }
    Ok(())
}
