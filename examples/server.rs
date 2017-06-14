extern crate websocket;
extern crate serde;
extern crate serde_json;
extern crate rand;

#[macro_use]
extern crate serde_derive;

use serde_json::Error;
use serde::ser::{self};

use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::RwLock;
use std::thread;
use websocket::OwnedMessage;
use websocket::sync::Server;
use std::sync::mpsc::*;
use rand::{thread_rng, Rng};

mod helper;




#[derive(Debug)]
struct User {
    uid: u32,
    name: String,
    msg_tx: Sender<OwnedMessage>,
}

#[derive(Debug)]
struct Group {
	uid: u32,
	name: String,
	user_list: HashSet<u32>,
}

#[derive(Debug)]
struct InternalMsg {
	from: u32,
	content: OwnedMessage,
}

#[derive(Debug)]
enum IDInfo {
	UserID(u32),
	GroupID(u32),
}

fn msg_parse(json_string: &str) -> Result<helper::ClientToServerMsg, ()> {
	// Parse the string of data into serde_json::Value.
    let v: Result<helper::ClientToServerTextMsg, Error> = serde_json::from_str(json_string);
    let u: Result<helper::ClientToServerCtrlMsg, Error> = serde_json::from_str(json_string);
    match v {
    	Ok(json) => {
    		let ret = helper::ClientToServerMsg::ClientToServerTextMsg(json);
    		return Ok(ret)
    	},
    	_ => {},
    };
    match u {
    	Ok(json) => {
    		let ret = helper::ClientToServerMsg::ClientToServerCtrlMsg(json);
    		return Ok(ret)
    	},
    	_ => {},
    };
    Err(())
}

fn msg_construct(structure: helper::ServerToClientMsg) -> Result<String, Error> {
	let json_string = 
	match structure {
		helper::ServerToClientMsg::ServerToClientTextMsg(m) => serde_json::to_string(&m)?,
		helper::ServerToClientMsg::ServerToClientCtrlMsg(m) => serde_json::to_string(&m)?,
	};
	Ok(json_string)
}

fn text_msg_process(m: &helper::ClientToServerTextMsg, from_id: u32, user_table: Arc<Mutex<HashMap<u32, User>>>, group_table: Arc<RwLock<HashMap<u32, Group>>>) -> Result<String, Error> {
	match m.to_type {
		0 => {
			// this message is sent to a user
			let v: helper::ServerToClientTextMsg = helper::ServerToClientTextMsg {
				msg_type: 0,
				from_type: 0,
				from_id: [from_id, 0],
				data: m.data.clone(),
			};
			if user_table.lock().unwrap().contains_key(&m.to_id){
				msg_construct(helper::ServerToClientMsg::ServerToClientTextMsg(v))
			} else {
				Err(ser::Error::custom(
					format!("Invalid to_id (user id) {0} in ClientToServerTextMsg of text_msg_process().", m.to_id)
				))
			}
		},
		1 => {
			// this message is sent to a group
			let v: helper::ServerToClientTextMsg = helper::ServerToClientTextMsg {
				msg_type: 0,
				from_type: 1,
				from_id: [from_id, m.to_id], // to_id is where the sender lives and receiver stays
				data: m.data.clone(),
			};
			if group_table.read().unwrap().contains_key(&m.to_id) {
				msg_construct(helper::ServerToClientMsg::ServerToClientTextMsg(v))
			} else {
				Err(ser::Error::custom(
					format!("Invalid to_id (group id) {0} in ClientToServerTextMsg of text_msg_process().", m.to_id)
				))
			}
		},
		_ => Err(ser::Error::custom(
			format!("Invalid to_type {0} in ClientToServerTextMsg of text_msg_process().", m.to_type)
			)),
	}
}

fn ctrl_msg_process(m: &helper::ClientToServerCtrlMsg, from_id: u32, user_table: Arc<Mutex<HashMap<u32, User>>>, group_table: Arc<RwLock<HashMap<u32, Group>>>) -> Result<String, Error> {
	let mut v: helper::ServerToClientCtrlMsg = helper::ServerToClientCtrlMsg {
		msg_type: 1,
		opcode: m.opcode + 1,
		data: "".to_string(),
	};
	match m.opcode {
		0 => {
			// change user nickname
			user_table.lock().unwrap().get_mut(&from_id).unwrap().name = m.data.clone();
			v.data = format!("Nickname has been changed to {}", m.data.clone());
		},
		2 => {
			// create new group
			let mut rng = thread_rng();
			let mut x: u32 = rng.gen();
			while group_table.read().unwrap().contains_key(&x) {
				// TODO: avoid dead loop
				x = rng.gen();
			}

			let mut hs = HashSet::new();
			hs.insert(from_id);
			let group: Group = Group {
				uid: 0,
				name: m.data.clone(),
				user_list: hs,
			};
			group_table.write().unwrap().insert(x, group);
			v.data = format!("New group is created, group ID = {}", x.to_string());
		},
		4 => {
			// join a group
			let x = m.data.clone().parse::<u32>();
			if let Ok(num) = x {
				match group_table.write().unwrap().get_mut(&num) {
					Some(expr) => {
						if expr.user_list.insert(from_id) {
							v.data = format!("You have joined a new group, group ID = {}", m.data.clone());
						} else {
							v.data = format!("You have already joined this group, group ID = {}", m.data.clone());
						}
					},
					None => v.data = format!("Invalid group ID = {}", m.data.clone()),
				};
			} else {
				v.data = format!("Invalid group ID = {}", m.data.clone());
			}
		},
		6 => {
			// leave a group
			let x = m.data.clone().parse::<u32>();
			if let Ok(num) = x {
				match group_table.write().unwrap().get_mut(&num) {
					Some(expr) => {
						if expr.user_list.remove(&from_id) {
							v.data = format!("You have left this group, group ID = {}", m.data.clone());
						} else {
							v.data = format!("You are not in this group, group ID = {}", m.data.clone());
						}
					},
					None => v.data = format!("Invalid group ID = {}", m.data.clone()),
				};
			} else {
				v.data = format!("Invalid group ID = {}", m.data.clone());
			}
		},
		8 => {
			// request nickname of a user
			let x = m.data.clone().parse::<u32>();
			if let Ok(num) = x {
				match user_table.lock().unwrap().get_mut(&num) {
					Some(expr) => v.data = expr.name.clone(),
					None => v.data = format!("Invalid User ID = {}", m.data.clone()),
				}
			} else {
				v.data = format!("Invalid User ID = {}", m.data.clone());
			}
		},
		10 => {
			// request nickname of a group
			let x = m.data.clone().parse::<u32>();
			if let Ok(num) = x {
				match group_table.read().unwrap().get(&num) {
					Some(expr) => {
						let mut result: Vec<String> = Vec::new();
						let ut = user_table.lock().unwrap();
						for user in &expr.user_list { // TODO: use map
							result.push(ut[user].name.clone());
						};
						v.data = serde_json::to_string(&result)?;
					},
					None => v.data = format!("Invalid group ID = {}", m.data.clone()),
				};
			} else {
				v.data = format!("Invalid group ID = {}", m.data.clone());
			}
		},
		_ => return Err(ser::Error::custom(
			format!("Invalid opcode {0} in ClientToServerCtrlMsg of ctrl_msg_process().", m.opcode)
			)),
	};
	msg_construct(helper::ServerToClientMsg::ServerToClientCtrlMsg(v))
}


fn main() {
	let server = Server::bind("127.0.0.1:2794").unwrap();

	let h1: HashMap<u32, User> = HashMap::new();
	let user_table_ = Arc::new(Mutex::new(h1));

	let h2: HashMap<u32, Group> = HashMap::new();
	let group_table = Arc::new(RwLock::new(h2));

	let (tx_slaver_, rx_master) = channel(); // send InternalMsg
	let user_table = user_table_.clone();

	// Master Thread
	thread::spawn(move || {
		loop {
			let internal_msg: InternalMsg = rx_master.recv().unwrap();

			let message = internal_msg.content;
			let from_uid = internal_msg.from;
			match message {
				OwnedMessage::Close(_) => {},
				OwnedMessage::Text(s) => {
					if let Ok(msg_struct) = msg_parse(s.as_ref()) {
						let id_info: IDInfo;
						let msg_str_to_send = 
							match msg_struct {
								helper::ClientToServerMsg::ClientToServerTextMsg(m) => {
									match text_msg_process(&m, from_uid, user_table.clone(), group_table.clone()) {
										Ok(json_string) => {
											id_info = 
												match m.to_type {
													0 => IDInfo::UserID(m.to_id),
													1 => IDInfo::GroupID(m.to_id),
													_ => unimplemented!(),// should never happen
												};
											json_string
										},
										Err(e) => {
											println!("{:?}", e);
											continue;
										},
									}
								},
								helper::ClientToServerMsg::ClientToServerCtrlMsg(m) => {
									match ctrl_msg_process(&m, from_uid, user_table.clone(), group_table.clone()) {
										Ok(json_string) => {
											id_info = IDInfo::UserID(from_uid);
											json_string
										},
										Err(e) => {
											println!("{:?}", e);
											continue;
										},
									}
								},
							};
						let new_message = OwnedMessage::Text(msg_str_to_send);

						let ut = user_table.lock().unwrap();
						match id_info {
							IDInfo::UserID(id) => {
								ut[&id].msg_tx.send(new_message).unwrap();
							},
							IDInfo::GroupID(id) => {
								if id == 0 {
									for user in ut.values() {
										if user.uid != from_uid {
											user.msg_tx.send(new_message.clone()).unwrap();
										}
									}
								} else {
									let gt = group_table.read().unwrap();
									for i in &gt[&id].user_list {
										if ut[i].uid != from_uid {
											ut[i].msg_tx.send(new_message.clone()).unwrap();
										}
									}
								};
							},
						}
					} else {
						println!("Invalid JSON!");
					}
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
						println!("[{}] {:?}", uid, message.clone());
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
