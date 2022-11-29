use curv::{arithmetic::Converter, elliptic::curves::Secp256k1, BigInt};
use dtrust::utils::init_app;
use multi_party_ecdsa::protocols::multi_party_ecdsa::gg_2020::state_machine::{
    keygen::{Keygen, LocalKey, ProtocolMessage},
    sign::{
        CompletedOfflineStage, OfflineProtocolMessage, OfflineStage, PartialSignature, SignManual,
    },
};

use round_based::{Msg, StateMachine};
use serde_json::Value;
use std::{
    io::{self, Error, ErrorKind, Read, Write},
    net::TcpStream,
};

const PROTOCOL_MSG_SIZE: usize = 18000;
const PARAM_SIZE: usize = 100;

/// This party receives incoming messages in present round of the keygen protocol
///
/// # Arguments
///
/// * `socks` - Peer-to-peer socket TCP connections between parties
/// * `party` - KeyGen protocol state machine of current party
/// * `party_index` - Index of current party
fn receive_keygen(
    socks: &mut Vec<TcpStream>,
    party: &mut Keygen,
    party_index: u16,
) -> io::Result<()> {
    // Receive from to all other recipients
    for sender in 1..(socks.len() + 1) {
        let recipient = party_index as usize;
        if recipient != sender {
            let mut result_buf = [0; PROTOCOL_MSG_SIZE];
            socks[sender - 1].read(&mut result_buf)?;

            // Deserialize message
            let received_msg = serde_json::from_str::<Msg<ProtocolMessage>>(
                &String::from_utf8_lossy(&result_buf).trim_matches(char::from(0)),
            )
            .unwrap();

            // Process received broadcast message
            party
                .handle_incoming(received_msg)
                .map_err(|e| Error::new(ErrorKind::Other, e))?;
        }
    }
    Ok(())
}

/// Current party receives incoming messages in present round of the signing protocol
///
/// # Arguments
///
/// * `socks` - Peer-to-peer socket TCP connections between parties
/// * `party` - OfflineStage protocol state machine of current party
/// * `party_index` - Index of current party
/// * `active_parties` - Parties participating in producing the signature
fn receive_sign(
    socks: &mut Vec<TcpStream>,
    party: &mut OfflineStage,
    party_index: u16,
    active_parties: &Vec<u16>,
) -> io::Result<()> {
    // Receive from to all other recipients
    for sender in active_parties {
        let recipient = party_index as usize;
        if recipient != *sender as usize {
            let mut result_buf = [0; PROTOCOL_MSG_SIZE];
            socks[*sender as usize - 1].read(&mut result_buf)?;

            // Deserialize message
            let received_msg = serde_json::from_str::<Msg<OfflineProtocolMessage>>(
                &String::from_utf8_lossy(&result_buf).trim_matches(char::from(0)),
            )
            .unwrap();
            // Process received broadcast message
            party
                .handle_incoming(received_msg)
                .map_err(|e| Error::new(ErrorKind::Other, e))?;
        }
    }
    Ok(())
}

/// Current party broadcasts a message to all other parties in present round of the keygen protocol
///
/// # Arguments
///
/// * `msg_index` - Index of message which this party is broadcasting to all other parties
/// * `socks` - Peer-to-peer socket TCP connections between parties
/// * `party` - KeyGen protocol state machine of current party
/// * `party_index` - Index of current party
fn broadcast_keygen(
    msg_index: usize,
    socks: &mut Vec<TcpStream>,
    party: &mut Keygen,
    party_index: u16,
) -> io::Result<()> {
    let msg = &party.message_queue()[msg_index];

    // Serialize message
    let serialized = serde_json::to_string(&msg).unwrap();

    // Send to all other recipients
    for recipient in 1..(socks.len() + 1) {
        let sender = party_index as usize;
        if recipient != sender {
            // Send message to recipient
            println!("MY RANK: {:?}\nI WRITE TO: {:?}", party_index - 1, recipient - 1);
            socks[recipient - 1].write(serialized.as_bytes())?;
        }
    }
    receive_keygen(socks, party, party_index)?;
    Ok(())
}

/// Current party broadcasts a message to all other parties in present round of the signing protocol
///
/// # Arguments
///
/// * `msg` - Index of message which this party is broadcasting to all other parties
/// * `socks` - Peer-to-peer socket TCP connections between parties
/// * `party` - OfflineStage protocol state machine of current party
/// * `party_index` - Index of current party
/// * `active_parties` - Parties participating in producing the signature
fn broadcast_sign(
    msg_index: usize,
    socks: &mut Vec<TcpStream>,
    party: &mut OfflineStage,
    party_index: u16,
    active_parties: &Vec<u16>,
) -> io::Result<()> {
    let msg = &party.message_queue()[msg_index];

    // Serialize message
    let serialized = serde_json::to_string(&msg).unwrap();

    // Send to all other recipients
    for recipient in active_parties {
        let sender = party_index as usize;
        if *recipient != sender as u16 {
            // Send message to recipient
            socks[*recipient as usize - 1].write(serialized.as_bytes())?;
        }
    }
    receive_sign(socks, party, party_index, &active_parties)?;
    Ok(())
}

/// Current party sends p2p messages to specific recipients in present round of the keygen protocol
///
/// # Arguments
///
/// * `msg_queue` - Messages this party is sending p2p to specific recipients
/// * `socks` - Peer-to-peer socket TCP connections between parties
/// * `party` - KeyGen protocol state machine of current party
/// * `party_index` - Index of current party
fn p2p_keygen(
    msg_queue: &mut Vec<Msg<ProtocolMessage>>,
    socks: &mut Vec<TcpStream>,
    party: &mut Keygen,
    party_index: u16,
) -> io::Result<()> {
    for msg in msg_queue.iter() {
        // Serialize message
        let serialized = serde_json::to_string(&msg).unwrap();

        // Send to intended recipient
        let recipient = msg.receiver.unwrap() as usize;
        socks[recipient - 1].write(serialized.as_bytes())?;
    }

    receive_keygen(socks, party, party_index)?;
    Ok(())
}

/// Current party sends p2p messages to specific recipients in present round of the signing protocol
///
/// # Arguments
///
/// * `msg_queue` - Messages this party is sending p2p to specific recipients
/// * `socks` - Peer-to-peer socket TCP connections between parties
/// * `party` - OfflineStage protocol state machine of current party
/// * `party_index` - Index of current party
/// * `active_parties` - Parties participating in producing the signature
fn p2p_sign(
    msg_queue: &mut Vec<Msg<OfflineProtocolMessage>>,
    socks: &mut Vec<TcpStream>,
    party: &mut OfflineStage,
    party_index: u16,
    active_parties: &Vec<u16>,
) -> io::Result<()> {
    for msg in msg_queue.iter() {
        // Serialize message
        let serialized = serde_json::to_string(&msg).unwrap();

        // Send to intended recipient
        let recipient = msg.receiver.unwrap() as usize;
        socks[recipient - 1].write(serialized.as_bytes())?;
    }

    receive_sign(socks, party, party_index, active_parties)?;
    Ok(())
}

/// Generates a signature on the message after offline stage is complete
///
/// # Arguments
///
/// * `msg_to_sign` - Message that parties must sign
/// * `party_index` - Index of current party
/// * `offline_output` - CompletedOfflineStage protocol state machine of current party
/// * `socks` - Peer-to-peer socket TCP connections between parties
/// * `active_parties` - Parties participating in producing the signature
fn sign_message(
    msg_to_sign: BigInt,
    party_index: u16,
    offline_output: CompletedOfflineStage,
    socks: &mut Vec<TcpStream>,
    active_parties: &Vec<u16>,
) -> Result<Vec<u8>, Error> {
    // Obtain party's partial share
    let (manual_sign, partial_share) = SignManual::new(msg_to_sign, offline_output).unwrap();

    // Send to all other parties
    // Serialize message
    let serialized = serde_json::to_string(&partial_share).unwrap();

    // Send to all other recipients
    for recipient in active_parties {
        let sender = party_index;
        if *recipient != sender {
            // Send message to recipient
            socks[*recipient as usize - 1].write(serialized.as_bytes())?;
        }
    }

    // Receive everyone else's partial signature shares
    let mut other_partial_shares = vec![];
    for sender in active_parties {
        let recipient = party_index;
        if recipient != *sender {
            let mut result_buf = [0; PROTOCOL_MSG_SIZE];
            socks[*sender as usize - 1].read(&mut result_buf)?;

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
    println!("Signature: {:?}", serde_json::to_string(&signature).unwrap());
    return serde_json::to_vec_pretty(&signature).map_err(|e| Error::new(ErrorKind::Other, e));
}

/// Generates local key share of the multi-party ECDSA threshold signing scheme for this party
///
/// # Arguments
///
/// * `num_parties` - Total number of parties
/// * `num_threshold` - The threshold t such that the number of honest and online parties must be at least t + 1 to produce a valid signature
/// * `socks` - Peer-to-peer socket TCP connections between parties
/// * `party_index` - Index of current party
fn keygen(
    num_parties: u16,
    num_threshold: u16,
    socks: &mut Vec<TcpStream>,
    party_index: u16,
) -> Result<Vec<u8>, Error> {
    // Set up current rank's party KeyGen state machine
    let mut party = Keygen::new(party_index, num_threshold, num_parties).unwrap();

    // Round 1
    party
        .proceed()
        .map_err(|e| Error::new(ErrorKind::Other, e))?;
    broadcast_keygen(0, socks, &mut party, party_index)?;

    // Round 2
    broadcast_keygen(1, socks, &mut party, party_index)?;
    party
        .proceed()
        .map_err(|e| Error::new(ErrorKind::Other, e))?;

    // Round 3
    let mut msg_queue = vec![];
    for i in 0..num_parties - 1 {
        let msg_index = (i + 2) as usize;
        msg_queue.push(party.message_queue()[msg_index].clone());
    }

    p2p_keygen(&mut msg_queue, socks, &mut party, party_index)?;
    party
        .proceed()
        .map_err(|e| Error::new(ErrorKind::Other, e))?;

    broadcast_keygen((num_parties + 1) as usize, socks, &mut party, party_index)?;
    party
        .proceed()
        .map_err(|e| Error::new(ErrorKind::Other, e))?;

    let local_key = party.pick_output().unwrap().unwrap();

    serde_json::to_vec_pretty(&local_key).map_err(|e| Error::new(ErrorKind::Other, e))
}

/// Generates signature of the multi-party ECDSA threshold signing scheme for this party
///
/// # Arguments
///
/// * `num_threshold` - The threshold t such that the number of honest and online parties must be at least t + 1 to produce a valid signature
/// * `active_parties` - Parties participating in producing the signature
/// * `key` - Local key share of current party generated in the keygen phase of the protocol
/// * `socks` - Peer-to-peer socket TCP connections between parties
/// * `party_index` - Index of current party
/// * `message` - Message that must be signed
fn sign(
    num_threshold: u16,
    active_parties: &Vec<u16>,
    key: LocalKey<Secp256k1>,
    socks: &mut Vec<TcpStream>,
    party_index: u16,
    message: String,
) -> Result<Vec<u8>, Error> {
    if !active_parties.contains(&party_index) {
        println!("Party {:?} is not needed in this signature generation.", party_index);
        return Ok(Vec::new());
    }
    // Initiate offline phase
    let mut offline_stage = OfflineStage::new(party_index, active_parties.clone(), key).unwrap();
    offline_stage
        .proceed()
        .map_err(|e| Error::new(ErrorKind::Other, e))?;

    // Round 1
    broadcast_sign(0, socks, &mut offline_stage, party_index, &active_parties)?;
    offline_stage
        .proceed()
        .map_err(|e| Error::new(ErrorKind::Other, e))?;

    // Round 2
    let mut msg_queue = vec![];
    for i in 0..num_threshold {
        let msg_index = (i + 1) as usize;
        msg_queue.push(offline_stage.message_queue()[msg_index].clone());
    }
    p2p_sign(
        &mut msg_queue,
        socks,
        &mut offline_stage,
        party_index,
        &active_parties,
    )?;
    offline_stage
        .proceed()
        .map_err(|e| Error::new(ErrorKind::Other, e))?;

    // Round 3
    broadcast_sign(
        (num_threshold + 1) as usize,
        socks,
        &mut offline_stage,
        party_index,
        &active_parties,
    )?;
    offline_stage
        .proceed()
        .map_err(|e| Error::new(ErrorKind::Other, e))?;

    // Round 4
    broadcast_sign(
        (num_threshold + 2) as usize,
        socks,
        &mut offline_stage,
        party_index,
        &active_parties,
    )?;
    offline_stage
        .proceed()
        .map_err(|e| Error::new(ErrorKind::Other, e))?;

    // Round 5
    broadcast_sign(
        (num_threshold + 3) as usize,
        socks,
        &mut offline_stage,
        party_index,
        &active_parties,
    )?;
    offline_stage
        .proceed()
        .map_err(|e| Error::new(ErrorKind::Other, e))?;

    // Round 6
    broadcast_sign(
        (num_threshold + 4) as usize,
        socks,
        &mut offline_stage,
        party_index,
        &active_parties,
    )?;
    offline_stage
        .proceed()
        .map_err(|e| Error::new(ErrorKind::Other, e))?;

    // Sign message
    let message_int = BigInt::from_bytes(&message.as_bytes());
    let offline_output = offline_stage.pick_output().unwrap().unwrap();
    sign_message(
        message_int,
        party_index,
        offline_output,
        socks,
        &active_parties,
    )
}

fn main() -> io::Result<()> {
    let (rank, func_name, in_files, out_files, mut socks) = init_app()?;
    let party_index = (rank + 1) as u16;
    let mut params_buf: Vec<u8> = [0; PARAM_SIZE].to_vec();
    let mut param_file = &in_files[0];
    param_file.read(&mut params_buf)?;
    let params_str = String::from_utf8_lossy(&params_buf);
    let params: Value = serde_json::from_str(params_str.trim_matches(char::from(0)))?;

    println!("SOCKLEN {}", socks.len());

    // Keygen
    if func_name == "keygen" {
        println!("Generating local key share for party {:?}...", party_index);
        let mut key_file = &out_files[0];
        let key = keygen(
            params["num_parties"].as_u64().unwrap() as u16,
            params["num_threshold"].as_u64().unwrap() as u16,
            &mut socks,
            party_index,
        )?;
        key_file.write(&key)?;
        println!("Key generation complete!");

    // Signing
    } else if func_name == "signing" {
        println!("Initiating signature generation for party {:?}...", party_index);
        let mut key_buf: String = "".to_string();
        let mut key_file = &in_files[1];
        key_file.read_to_string(&mut key_buf).unwrap();
        let key = serde_json::from_str::<LocalKey<Secp256k1>>(&key_buf).unwrap();

        let active_party_iter = params["active_parties"].as_array().unwrap().iter();
        let active_parties : Vec<u16> = active_party_iter.map( |x| x.as_u64().unwrap() as u16).collect();

        let signature = sign(
            params["num_threshold"].as_u64().unwrap() as u16,
            &active_parties,
            key,
            &mut socks,
            party_index,
            params["message"].to_string(),
        )?;

        let mut sig_file = &out_files[0];
        sig_file.write(&signature)?;
        println!("Signature generation complete.");
    }
    Ok(())
}
