extern crate websocket;
extern crate serde;
extern crate serde_json;

#[macro_use]
extern crate serde_derive;

use serde_json::Error;
use serde::ser::{self, Serialize, Serializer};

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use websocket::OwnedMessage;
use websocket::sync::Server;
use std::sync::mpsc::*;

#[derive(Debug)]
enum clientToServerMsg {
	clientToServerCtrlMsg(clientToServerCtrlMsg),
	clientToServerTextMsg(clientToServerTextMsg),
}

#[derive(Debug)]
enum serverToClientMsg {
	serverToClientCtrlMsg(serverToClientCtrlMsg),
	serverToClientTextMsg(serverToClientTextMsg),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct clientToServerCtrlMsg {
	msgType: u8, // 1
	opcode: u8, // even number
	data: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct clientToServerTextMsg {
	msgType: u8, // 0
	toType: u8, // 0=>userID, 1=>groupID
	toID: u32,
	data: String,
}


#[derive(Debug, Serialize, Deserialize)]
pub struct serverToClientCtrlMsg {
	msgType: u8, // 1
	opcode: u8, // odd number
	data: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct serverToClientTextMsg {
	msgType: u8, // 0
	fromType: u8, // 0=>userID, 1=>groupID
	fromID: [u32; 2], // [userID, groupID]
	data: String,
}



#[derive(Debug)]
struct User {
    uid: u32,
    name: String,
    msg_tx: Sender<OwnedMessage>,
}

#[derive(Debug)]
struct InternalMsg {
	from: u32,
	content: OwnedMessage,
}

fn msgParse(json_string: &str) -> Result<clientToServerMsg, ()> {
	// Parse the string of data into serde_json::Value.
    let v: Result<clientToServerTextMsg, Error> = serde_json::from_str(json_string);
    let u: Result<clientToServerCtrlMsg, Error> = serde_json::from_str(json_string);
    match v {
    	Ok(json) => {
    		let ret = clientToServerMsg::clientToServerTextMsg(json);
    		return Ok(ret)
    	},
    	_ => {},
    };
    match u {
    	Ok(json) => {
    		let ret = clientToServerMsg::clientToServerCtrlMsg(json);
    		return Ok(ret)
    	},
    	_ => {},
    };
    Err(())
}

fn msgContruct(structure: serverToClientMsg) -> Result<String, Error> {
	let json_string = 
	match structure {
		serverToClientMsg::serverToClientTextMsg(m) => serde_json::to_string(&m)?,
		serverToClientMsg::serverToClientCtrlMsg(m) => serde_json::to_string(&m)?,
	};
	Ok(json_string)
}

fn textMsgProcess(m: clientToServerTextMsg, fromID: u32) -> Result<String, Error> {
	match m.toType {
		0 => {
			// this message is sent to a user
			let v: serverToClientTextMsg = serverToClientTextMsg {
				msgType: 1,
				fromType: 0,
				fromID: [fromID, 0],
				data: m.data,
			};
			msgContruct(serverToClientMsg::serverToClientTextMsg(v))
		},
		1 => {
			// this message is sent to a group
			let v: serverToClientTextMsg = serverToClientTextMsg {
				msgType: 1,
				fromType: 1,
				fromID: [fromID, m.toID], // toID is where the sender lives and receiver stays
				data: m.data,
			};
			msgContruct(serverToClientMsg::serverToClientTextMsg(v))
		},
		_ => Err(ser::Error::custom(
			format!("Invalid toType {0} in clientToServerTextMsg of textMsgProcess().", m.toType)
			)),
	}
}

fn ctrlMsgProcess(m: clientToServerCtrlMsg) -> Result<(), ()> {
	match m.opcode {
		0 => unimplemented!(),
		2 => unimplemented!(),
		4 => unimplemented!(),
		6 => unimplemented!(),
		8 => unimplemented!(),
		_ => Err(()),
	}

}


fn main() {
	let server = Server::bind("127.0.0.1:2794").unwrap();

	let h: HashMap<u32, User> = HashMap::new();
	let user_table_ = Arc::new(Mutex::new(h));

	let (tx_slaver_, rx_master) = channel(); // send InternalMsg
	let user_table = user_table_.clone();
	thread::spawn(move || {
		loop {
			let internal_msg: InternalMsg = rx_master.recv().unwrap();

			let message = internal_msg.content;
			let from_uid = internal_msg.from;
			match message {
				OwnedMessage::Close(_) => {},
				OwnedMessage::Text(_) => {
					// // Broadcasting
					// let ut = user_table.lock().unwrap();
					// for user in ut.values() {
					// 	if user.uid != from_uid {
					// 		if let OwnedMessage::Text(txt) = message.clone() {
					// 			let new_message =  OwnedMessage::Text(ut[&from_uid].name.to_string() + ": " + &txt.to_string());
					// 			user.msg_tx.send(new_message).unwrap();
					// 		}
					// 	}
					// }
					
				},
				OwnedMessage::Binary(bin) => {
					user_table.lock().unwrap().get_mut(&from_uid).unwrap().name = String::from_utf8(bin).unwrap();
					println!("{:?}", user_table.lock().unwrap()[&from_uid]);
				},
				_ => unimplemented!(),
			}
		};

	});
	for request in server.filter_map(Result::ok) {
		// Spawn a new thread for each connection.
		let user_table = user_table_.clone();
		let tx_slaver = tx_slaver_.clone(); 
		let (tx_master_, rx_slaver) = channel(); // send original message
		let tx_master = tx_master_.clone();
		thread::spawn(move || {
			if !request.protocols().contains(&"rust-websocket".to_string()) {
				request.reject().unwrap();
				return;
			}

			let mut client = request.use_protocol("rust-websocket").accept().unwrap();
			let ip = client.peer_addr().unwrap();

			println!("Connection from {}", ip);

			let message = OwnedMessage::Text("Login successfully.".to_string());
			client.send_message(&message).unwrap();

			let (mut receiver, mut sender) = client.split().unwrap();

			let mut ut = user_table.lock().unwrap();
			//TODO: optimization
			let uid = match ut.keys().max() {
				Some(v) => *v + 1,
				None => 1,
			};

			ut.insert(uid, User{
				uid: uid,
				name: "new user ".to_string() + &uid.to_string(),
				msg_tx: tx_master.clone(),
			});

			drop(ut);


			// Sender Thread
			thread::spawn(move || {
				// if get a msg from master thread, send it to client directly
				loop {
					match rx_slaver.recv() {
						Ok(msg) => {

							sender.send_message(&msg).unwrap();
						},
						_ => break,
					}
				}
			});

			// Receiver Loop
			for message in receiver.incoming_messages() {
				print!("{:?}",message );
				let message = message.unwrap();

				match message {
					OwnedMessage::Close(_) => {
						let message = OwnedMessage::Close(None);
						// sender_.send_message(&message).unwrap();

						// send to my TCP Sender
						tx_master.send(message).unwrap(); //pretend I'm the master..
						println!("Client {} disconnected", ip);
						return;
					}
					OwnedMessage::Ping(ping) => {
						let message = OwnedMessage::Pong(ping);

						// send to my TCP Sender
						tx_master.send(message).unwrap();
						// sender_.send_message(&message).unwrap();
					}
					// _ => sender.send_message(&message).unwrap(),

					// if get a msg from client, send it to master
					_ => {
						tx_slaver.send(InternalMsg{
											from: uid,
											content: message,
										}).unwrap();
					}
				};
			};
		});

	}
}
