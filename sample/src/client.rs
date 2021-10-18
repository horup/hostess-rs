
use hostess::{ClientMsg, ServerMsg, log::info, uuid::Uuid, Bincoded};
use crate::{GameClientMsg, GameServerMsg, GameState, performance_now};
use super::Canvas;

pub struct Client {
    id:Uuid,
    canvas:Canvas,
    state:GameState,
    status:String,
    ping:f64,
    updates:u64,
    pub server_messages:Vec<ServerMsg>,
    pub client_messages:Vec<ClientMsg>
}

pub type KeyCode = u32;

impl Client {
    pub fn new() -> Self {
        Self {
            canvas:Canvas::new(),
            state:GameState::new(),
            server_messages:Vec::new(),
            status:"Not connected!".into(),
            client_messages:Vec::new(),
            id:Uuid::new_v4(),
            ping:0.0,
            updates:0
        }
    }

    pub fn init(&mut self) {
        self.canvas.set_image_src(0, "dummy.png");
    }

    pub fn draw(&self) {
        self.canvas.clear();
        let grid_size = 16.0;
        self.canvas.set_scale(grid_size);

        // draw debug circle of things
        for (_, thing) in &self.state.things {
            let x = thing.pos.x as f64;
            let y = thing.pos.y as f64;
            self.canvas.draw_circle(x, y, thing.radius as f64);
        }

        // draw things
        for (_, thing) in &self.state.things {
            let x = thing.pos.x as f64;
            let y = thing.pos.y as f64;
            self.canvas.draw_normalized_image(0, x, y);
        }

        // draw names of things
        for (_, thing) in &self.state.things {
            let x = thing.pos.x as f64;
            let y = thing.pos.y as f64;
            self.canvas.fill_text(&thing.name, x, y - 1.0);
        }


        self.canvas.set_text_style("center", "middle");
        self.canvas.fill_text(&self.status, (self.canvas.width() / 2 / grid_size as u32) as f64, 0.5);
        self.canvas.set_text_style("right", "middle");
        self.canvas.fill_text(format!("ping:{:0.00}ms", self.ping).as_str(), self.canvas.width() as f64 / grid_size - 0.1, 0.5);
        
    }

    pub fn send(&mut self, msg:ClientMsg) {
        self.client_messages.push(msg)
    }

    pub fn recv(&mut self, msg:&ServerMsg) {
        match msg {
            ServerMsg::LobbyJoined {  } => {
                self.status = "Connected to Server".into();
               
            },
            ServerMsg::Hosts {hosts} => {
                if let Some(host) = hosts.first() {
                    self.status = format!("Joining host {}..", host.id);
                    let id = host.id;
                    self.send(ClientMsg::JoinHost {
                        host_id:id
                    });
                }
            },
            ServerMsg::HostJoined {host} => {
                self.status = format!("✓ Joined host {} ✓ ", host.id);
            },
            ServerMsg::Pong {
                tick
            } => {
                let ping:f64 = performance_now() - tick;
                self.ping = ping;
            },
            ServerMsg::Custom { msg } => {
                let msg = GameServerMsg::from_bincode(msg).unwrap();
                match msg {
                    GameServerMsg::SnapshotFull { state } => {
                        self.state = state;
                    },
                }
            }
            _ => {}
        }
    }

    pub fn update(&mut self) {
        for msg in &self.server_messages.clone() {
            self.recv(msg);
        }

        self.updates += 1; 

        if self.updates % 10 == 0 {
            self.send(ClientMsg::Ping {
                tick:performance_now()
            });
        }

        self.draw();
    }

    pub fn keyup(&mut self, _code:KeyCode) {
    }

    pub fn keydown(&mut self, code:KeyCode) {
        // w = 87
        // s = 83
        // a = 65
        // d = 68
        // space = 32
        // up = 38
        // down = 40
        // left = 37
        // right = 39
        // esc = 27

        // space
        if code == 32 {
            self.client_messages.push(ClientMsg::CustomMsg {
                msg:GameClientMsg::ClientInput {
                    position:None,
                    shoot:true
                }.to_bincode()
            });
        }
        info!("{}", code);
    }

    pub fn connected(&mut self) {
        self.status = format!("Sending Hello");
        self.client_messages.push(ClientMsg::Hello {
            client_id:self.id.clone()
        });
    }

    pub fn disconnected(&mut self) {
        self.status = "Trying to reconnect...".into();
    }
}

unsafe impl Send for Client {
}
unsafe impl Sync for Client {
}