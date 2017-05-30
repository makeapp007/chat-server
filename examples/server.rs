extern crate websocket;

use std::thread;
use websocket::OwnedMessage;
use websocket::sync::Server;
use std::sync::mpsc::*;


// #[derive(Debug)]
// struct User {
//     uid: u32,
//     name: String,
//     msg_tx: Sender,
//     msg_rx: Receiver,

// }


fn main() {
	let server = Server::bind("127.0.0.1:2794").unwrap();

	// let (tx, rx) = channel();
	// thread::spawn(move || {


	// };
	for request in server.filter_map(Result::ok) {
		print!("------" );
		// Spawn a new thread for each connection.
		thread::spawn(move || {
			if !request.protocols().contains(&"rust-websocket".to_string()) {
				request.reject().unwrap();
				return;
			}

			let mut client = request.use_protocol("rust-websocket").accept().unwrap();
			let ip = client.peer_addr().unwrap();

			println!("Connection from {}", ip);

			let message = OwnedMessage::Text("Hello".to_string());
			client.send_message(&message).unwrap();

			let (mut receiver, mut sender) = client.split().unwrap();


			// loops here 
			for message in receiver.incoming_messages() {
				println!("server receives {:?}",message );
				// match message {
				//     OwnedMessage::Binary(a) => String::from_utf8(a),
				//     None => (),
				// }


				let message = message.unwrap();
			// println!("shit1");

				match message {
					OwnedMessage::Close(_) => {
						let message = OwnedMessage::Close(None);
						sender.send_message(&message).unwrap();
						println!("Client {} disconnected", ip);
						return;
					}
					OwnedMessage::Ping(ping) => {
						let message = OwnedMessage::Pong(ping);
						sender.send_message(&message).unwrap();
					}
					_ => sender.send_message(&message).unwrap(),
				}
			}
		});
	}
}
