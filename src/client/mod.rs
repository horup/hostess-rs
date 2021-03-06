#[cfg(not(target_arch = "wasm32"))]
pub mod tungstenite_client;

pub use crate::shared::{InstanceInfo};
pub use uuid::Uuid;
pub use serde::{Deserialize, Serialize};
pub use crate::bincoded::Bincoded;

#[derive(Clone, Debug, Serialize, Deserialize)]
/// message sent from Client to Server
pub enum ClientMsg {
    Hello {
        client_id:Uuid,
        client_name:String
    },
    JoinInstance {
        instance_id:Uuid
    },
    LeaveInstance {
    },
    CustomMsg {
        msg:Vec<u8>
    },
    Ping {
        tick:f64
    },
    RefreshInstances,
}


#[derive(Clone, Debug, Serialize, Deserialize)]
/// message sent from Server to Client
pub enum ServerMsg {
    JoinedLobby {

    },
    Instances {
        instances:Vec<InstanceInfo>
    },
    JoinedInstance {
        instance:InstanceInfo
    },
    Pong {
        tick:f64,

        /// number of bytes send from the server to the client per second
        /// on the application level only, i.e. does not account for websocket and tcp overhead
        server_bytes_sec:f32,

        /// number of bytes send from the client to the server per second
        /// on the application level only, i.e. does not account for websocket and tcp overhead
        client_bytes_sec:f32
    },
    Custom {
        msg:Vec<u8>
    },
    JoinRejected {
        instance:InstanceInfo
    }
}


impl Bincoded for ClientMsg {
}

impl Bincoded for ServerMsg {
}