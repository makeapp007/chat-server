// store common data 

#[derive(Debug)]
pub enum ClientToServerMsg {
	ClientToServerCtrlMsg(ClientToServerCtrlMsg),
	ClientToServerTextMsg(ClientToServerTextMsg),
}


#[derive(Debug, Serialize, Deserialize)]
pub struct ClientToServerTextMsg {
	pub msg_type: u8, // 0
	pub to_type: u8, // 0=>userID, 1=>groupID
	pub to_id: u32,
	pub data: String,
}


#[derive(Debug, Serialize, Deserialize)]
pub struct ClientToServerCtrlMsg {
	pub msg_type: u8, // 1
	pub opcode: u8, // even number
	pub data: String,
}

#[derive(Debug)]
pub enum ServerToClientMsg {
	ServerToClientCtrlMsg(ServerToClientCtrlMsg),
	ServerToClientTextMsg(ServerToClientTextMsg),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServerToClientCtrlMsg {
	pub msg_type: u8, // 1
	pub opcode: u8, // odd number
	pub data: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServerToClientTextMsg {
	pub msg_type: u8, // 0
	pub from_type: u8, // 0=>userID, 1=>groupID
	pub from_id: [u32; 2], // [userID, groupID]
	pub data: String,
}


 