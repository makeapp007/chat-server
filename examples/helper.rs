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



