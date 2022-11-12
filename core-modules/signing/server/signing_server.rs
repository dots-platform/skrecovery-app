use dtrust::utils::init_app;
use multi_party_ecdsa::protocols::multi_party_ecdsa::gg_2020::state_machine::keygen::{
    Keygen, ProtocolMessage,
};
use round_based::{Msg, StateMachine};
use std::{
    io::{self, Read, Write},
    net::TcpStream,
};

// Handle all received messages
fn receive(socks: &mut Vec<TcpStream>, party: &mut Keygen, party_index: u16) -> io::Result<()> {
    // Receive from to all other recipients
    for sender in 1..(socks.len() + 1) {
        let recipient = party_index as usize;
        if recipient != sender {
            let mut result_buf = [0; 18000]; // TODO: Figure out buffer size
            socks[sender - 1].read(&mut result_buf)?;

            // Deserialize message
            let received_msg = serde_json::from_str::<Msg<ProtocolMessage>>(
                &String::from_utf8_lossy(&result_buf).trim_matches(char::from(0)),
            )
            .unwrap();

            // Process received broadcast message
            party.handle_incoming(received_msg);
        }
    }
    Ok(())
}

// Broadcast message to all other parties
fn broadcast(
    msg_queue: &mut Vec<Msg<ProtocolMessage>>,
    socks: &mut Vec<TcpStream>,
    party: &mut Keygen,
    party_index: u16,
) -> io::Result<()> {
    for msg in msg_queue.iter() {
        // Serialize message
        let serialized = serde_json::to_string(&msg).unwrap();

        // Send to all other recipients
        for recipient in 1..(socks.len() + 1) {
            let sender = party_index as usize;
            if recipient != sender {
                // Send message to recipient
                socks[recipient - 1].write(serialized.as_bytes())?;
            }
        }
    }
    receive(socks, party, party_index)?;
    Ok(())
}

// Send message to one recipient
fn p2p(
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

    receive(socks, party, party_index)?;
    Ok(())
}

fn keygen(
    num_parties: u16,
    num_threshold: u16,
    socks: &mut Vec<TcpStream>,
    party_index: u16,
) -> io::Result<()> {
    // Set up a party KeyGen state machine the current rank
    let mut party = Keygen::new(party_index, num_threshold, num_parties).unwrap();
    // Unsent messages sit in this queue each round
    let mut msg_queue = vec![];

    // Round 1
    party.proceed();
    msg_queue.push(party.message_queue()[0].clone());
    broadcast(&mut msg_queue, socks, &mut party, party_index)?;
    msg_queue.clear();

    // Round 2
    msg_queue.push(party.message_queue()[1].clone());
    broadcast(&mut msg_queue, socks, &mut party, party_index)?;
    party.proceed();

    // Round 3
    msg_queue.clear();
    for i in 0..num_parties - 1 {
        let msg_index = (i + 2) as usize;
        msg_queue.push(party.message_queue()[msg_index].clone());
    }

    p2p(&mut msg_queue, socks, &mut party, party_index)?;
    party.proceed();

    msg_queue.clear();
    msg_queue.push(party.message_queue()[(num_parties + 1) as usize].clone());

    broadcast(&mut msg_queue, socks, &mut party, party_index)?;
    party.proceed();

    let localkey = party.pick_output().unwrap().unwrap();
    println!("{:?}", localkey);
    Ok(())
}


#[tokio::main]
async fn main() -> io::Result<()> {
    let (rank, func_name, in_files, _out_files, mut socks) = init_app()?;

    let mut param_buf = [0; 10];
    let mut f = in_files.first().unwrap();
    f.read(&mut param_buf)?;
    let param_str = String::from_utf8_lossy(&param_buf);
    let params: Vec<&str> = param_str.split(" ").collect();

    // Keygen
    if func_name == "keygen" {
        let num_parties = params[0].parse::<u16>().unwrap();
        let num_threshold = params[1].trim_matches(char::from(0)).parse::<u16>().unwrap();
        let party_index = (rank + 1) as u16;
        keygen(num_parties, num_threshold, &mut socks, party_index)?;

    // Signing
    } else if func_name == "signing" {
    }
    Ok(())
}
