extern crate websocket;
extern crate serde;
extern crate serde_json;


#[macro_use]
extern crate serde_derive;
mod helper;

use serde::ser::{self};


use serde_json::Error;

use std::thread;
use std::sync::mpsc::channel;
use std::io::stdin;
// use std::io::{self, Write};

use websocket::{Message, OwnedMessage};
use websocket::client::ClientBuilder;

const CONNECTION: &'static str = "ws://127.0.0.1:2794";




fn msg_construct(structure: helper::ClientToServerMsg) -> Result<String, Error> {
	let json_string = 
	match structure {
		helper::ClientToServerMsg::ClientToServerTextMsg(m) => serde_json::to_string(&m)?,
		helper::ClientToServerMsg::ClientToServerCtrlMsg(m) => serde_json::to_string(&m)?,
	};
	Ok(json_string)
}



// 3 thread
// Input thread: Read input form stdin, send it to Sender thread
// Sender thread: Send message to server
// Receiver thread: Receive message from server

fn main() {

	println!("Connecting to {}", CONNECTION);

	let client = ClientBuilder::new(CONNECTION)
		.unwrap()
		.add_protocol("rust-websocket")
		.connect_insecure()
		.unwrap();

	println!("Successfully connected");

	let (mut receiver, mut sender) = client.split().unwrap();

	let (tx, rx) = channel();

	let tx_1 = tx.clone();

//Sender thread
	let send_loop = thread::spawn(move || {
		loop {
			// Send loop
			let message = match rx.recv() {
				Ok(m) => m,
				Err(e) => {
					println!("Send Loop 1: {:?}", e);
					return;
				}
			};
			match message {
				OwnedMessage::Close(_) => {
					let _ = sender.send_message(&message);
					// If it's a close message, just send it and then return.
					return;
				}
				_ => (),
			}
			// Send the message
			match sender.send_message(&message) {
				Ok(()) => (),
				Err(e) => {
					println!("Send Loop 2: {:?}", e);
					let _ = sender.send_message(&Message::close());
					return;
				}
			}
		}
	});


//Receiver thread
	let receive_loop = thread::spawn(move || {
		// Receive loop
		for message in receiver.incoming_messages() {
			let message = match message {
				Ok(m) => m,
				Err(e) => {
					println!("Receive Loop 1: {:?}", e);
					let _ = tx_1.send(OwnedMessage::Close(None));
					return;
				}
			};
			match message {
				OwnedMessage::Close(_) => {
					// Got a close message, so send a close message and return
					let _ = tx_1.send(OwnedMessage::Close(None));
					return;
				}
				OwnedMessage::Ping(data) => {
					match tx_1.send(OwnedMessage::Pong(data)) {
						// Send a pong in response
						Ok(()) => (),
						Err(e) => {
							println!("Receive Loop 2: {:?}", e);
							return;
						}
					}
				}
				// Say what we received
				_ => {
					if let OwnedMessage::Text(txt) = message {
						println!("{}", txt);
					}
				},
			}
		}
	});


//Input thread
	loop {
		let mut input = String::new();

		// print!("Self: ");
		// io::stdout().flush().unwrap();
		
		stdin().read_line(&mut input).unwrap();
		let trimmed = input.trim();

		// returning at most n items.
		let v: Vec<&str> = trimmed.splitn(4,' ').collect();
		// for i in &v{
		// 	println!("{:?}",i );
		// }
		// println!("");
		// println!("{:?}",v[0] );


		// let mut message = match trimmed {
		// 	"/close" => {
		// 		// Close the connection
		// 		let _ = tx.send(OwnedMessage::Close(None));
		// 		break;
		// 	}
		// 	// Send a ping
		// 	"/ping" => OwnedMessage::Ping(b"PING".to_vec()),
		// 	// Otherwise, just send text
		// 	_ => OwnedMessage::Text(trimmed.to_string()),
		// };


		// check input validation 
		if v.len()<3 {
			println!("input message is too small");
			continue;
		}
		let v0= match v[0].parse::<u8>() {
		    Ok(expr) => expr,
		    _ => {
		    	println!("msg_type is not a number");
		    	continue;
		    },
		};
		let v1= match v[1].parse::<u8>() {
		    Ok(expr) => expr,
		    _ => {
		    	println!("opcode is not a number");
		    	continue;
		    },
		};
		if v0==0{
			if v.len()!=4{
				println!("input message length is not correct");
				continue;
			}
		}
		else if v0==1{
			if v.len()!=3{
				println!("input message length is not correct");
				continue;
			}
		}
		// parse input
		let msg_send = match v0 {
		    1 => {
	    		let m=helper::ClientToServerCtrlMsg {
				msg_type: v0,
				opcode: v1,
				data: v[2].to_string(),
				};
				msg_construct(helper::ClientToServerMsg::ClientToServerCtrlMsg(m))
			},
			0 => {
			    let m =helper::ClientToServerTextMsg {
					msg_type: v0,
					to_type: v1,
					to_id: v[2].parse::<u32>().unwrap(),
					data: v[3].to_string(),
				};				
				msg_construct(helper::ClientToServerMsg::ClientToServerTextMsg(m))
			},
		    _ => Err(ser::Error::custom(
			format!("Invalid input in ClientToServerTextMsg construction.")
			)),
		};




		// if trimmed.len()>7{
		// 	let (start,end)=trimmed.split_at(6);
		// 	if start=="change"{
		// 		message=OwnedMessage::Binary(String::from(end).trim().to_string().into_bytes());	
		// 	}
		// 	// message=c.into_bytes();
		// }
		// let (b,c)=trimmed.split_at(6);
		// println!("{:?}",b );
		// println!("{:?}",c );
		// println!("{:?}",message );
		// // change makeapp
		// let mut start=1;
		// let mut end=1;
		// if trimmed.len()>7 {
		//     (start, end)=trimmed.split_at_mut(6);
		// };

		// // let mut flag=1;
		// println!("{:?}", start);
		// println!("{:?}", end);
		// if start=="change"{
		// 	message=end.into_bytes();
		// 	// flag=0;
		// }

		//send message
		match msg_send {
		    Ok(expr) => {
		    	let msg_send_own=OwnedMessage::Text(expr);
		    	match tx.send(msg_send_own) {
					Ok(()) => (),
					Err(e) => {
						println!("Main Loop: {:?}", e);
						break;
					},
				}
			},
		    Err(e) => {
		    	println!("User: an invalid input, error is {}",e);
		    	()
		    },
		};
	}

	// We're exiting

	println!("Waiting for child threads to exit");

	let _ = send_loop.join();
	let _ = receive_loop.join();

	println!("Exited");
}
