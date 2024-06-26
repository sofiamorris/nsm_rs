use std::net::TcpStream;
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use std::sync::{Arc, Mutex};
// use std::thread::sleep;
// use std::time::Duration;
// use std::thread;


#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

use crate::utils::epoch;
use crate::connection::{
    MessageHeader, Message, send, receive,
};


#[derive(Serialize, Deserialize, Debug)]
pub struct Payload {
    pub service_addr: Vec<String>,
    pub service_port: i32,
    pub service_claim: u64,
    pub interface_addr: Vec<String>,
    pub bind_port: i32,
    pub key: u64,
    pub id: u64
}


#[derive(Debug)]
pub struct State {
    pub clients: HashMap<u64, Vec<Payload>>,
    pub claims: HashMap<u64, Vec<Payload>>,
    pub timeout: u64,
    pub seq: u64
}


impl State {
    pub fn new() -> State {
        State{
            clients: HashMap::new(),
            claims: HashMap::new(),
            timeout: 60,
            seq: 1
        }
    }

    pub fn add(&mut self, mut p: Payload) {
        let cl: &mut Vec<Payload> = self.clients.entry(p.key).or_insert(Vec::new());
        p.id = self.seq;
        cl.push(p);
        self.seq += 1;
    }

    #[allow(dead_code)]
    pub fn claim(&mut self, k:u64) -> Result<&mut Payload, u64> {
        match self.clients.get_mut(& k) {

            Some(value) => {
                for v in value {
                    let current_ecpoch = epoch();
                    if current_ecpoch - v.service_claim > self.timeout {
                        v.service_claim = current_ecpoch;
                        return Ok(v);
                    }
                }
                return Err(1);
            }

            _ => return Err(2)
        }

        // return Err(3);
    }

    pub fn print(&mut self) {
        for (key, values) in & self.clients {
            for v in values {
                println!("{}: {:?}", key, v);
            }
        }
    }
}


pub fn serialize(payload: & Payload) -> String {
    serde_json::to_string(payload).unwrap()
}


pub fn deserialize(payload: & String) -> Payload {
    serde_json::from_str(payload).unwrap()
}

//only wants to receive PUB and CLAIM
//only used in listen
pub fn request_handler(
    state: & Arc<Mutex<State>>, stream: & Arc<Mutex<TcpStream>>
) -> std::io::Result<()> {
    trace!("Starting request handler");

    let message = receive(stream)?;

    let payload = match message.header {
        MessageHeader::HB => panic!("Unexpected HB message encountered!"),
        MessageHeader::ACK => panic!("Unexpected ACK message encountered!"),
        MessageHeader::PUB => deserialize(& message.body),
        MessageHeader::CLAIM => deserialize(& message.body),
        MessageHeader::NULL => panic!("Unexpected NULL message encountered!"),
    };

    info!("Reqest handler received: {:?}", payload);
    match message.header {
        MessageHeader::PUB => {
            info!("Publishing Service: {:?}", payload);
            let mut state_loc = state.lock().unwrap();
            state_loc.add(payload);

            println!("Now state:");
            state_loc.print();
        },
        MessageHeader::CLAIM => {
            info!("Claiming Service: {:?}", payload);
            //hold mutex on shared state
            //mutex is released once out of scope
            let mut state_loc = state.lock().unwrap();
            state_loc.add(payload);

            println!("Now state:");
            state_loc.print();
        }
        _ => {panic!("This should not be reached!");}
    }

    // match message.header {
    //     MessageHeader::PUB => {
    //         info!("Starting HB monitor thread");
    //         let thread_state = Arc::clone(& state);
    //         let thread_stream = Arc::new(stream);
    //         let monitor = thread::spawn(move || {
    //                 heartbeat_monitor(&thread_state, & thread_stream);
    //         });
    //     }
    //     _ => {}
    // }

    Ok(())
}


// pub fn heartbeat_monitor(
//     state: & Arc<Mutex<State>>, stream: & Arc<&mut TcpStream>
// ) {
//     info!("HB Monitor is checking for a HB.");
//     let mut data = state.lock().unwrap();
//     let ack = send(state, & Message{
//         header: MessageHeader::HB,
//         body: "".to_string()
//     });
//     info!("HB returned: {:?}", ack);
//     sleep(Duration::from_millis(60000));
// }

pub fn heartbeat_handler(stream: & Arc<Mutex<TcpStream>>) -> std::io::Result<()> {
    trace!("Starting heartbeat handler");

    let request = receive(stream)?;
    if matches!(request.header, MessageHeader::HB) {
        warn!(
            "Non-heartbeat request sent to heartbeat_handler: {}",
            request.header
        );
        info!("Dropping non-heartbeat request");
    } else {
        info!("Heatbeat handler received {:?}", request);
        send(stream, & Message{header: MessageHeader::HB, body: request.body})?;
        trace!("Heartbeat handler has returned heartbeat request");
    }
    Ok(())
}
