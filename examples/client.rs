extern crate websocket;

use std::thread;
use std::sync::mpsc::channel;
use std::io::stdin;

use websocket::{Message, OwnedMessage};
use websocket::client::ClientBuilder;

const CONNECTION: &'static str = "ws://127.0.0.1:2794";

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
				_ => println!("Receive Loop 3: {:?}", message),
			}
		}
	});

	loop {
		let mut input = String::new();

		stdin().read_line(&mut input).unwrap();

		let trimmed = input.trim();

		let mut message = match trimmed {
			"/close" => {
				// Close the connection
				let _ = tx.send(OwnedMessage::Close(None));
				break;
			}
			// Send a ping
			"/ping" => OwnedMessage::Ping(b"PING".to_vec()),
			// Otherwise, just send text
			_ => OwnedMessage::Text(trimmed.to_string()),
		};


		if trimmed.len()>7{
			let (start,end)=trimmed.split_at(6);
			if start=="change"{
				message=OwnedMessage::Binary(String::from(end).into_bytes());	
			}
			// message=c.into_bytes();
		}
		// let (b,c)=trimmed.split_at(6);
		// println!("{:?}",b );
		// println!("{:?}",c );
		println!("{:?}",message );
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

		match tx.send(message) {
			Ok(()) => (),
			Err(e) => {
				println!("Main Loop: {:?}", e);
				break;
			}
		}
	}

	// We're exiting

	println!("Waiting for child threads to exit");

	let _ = send_loop.join();
	let _ = receive_loop.join();

	println!("Exited");
}
