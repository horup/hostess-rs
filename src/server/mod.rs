use std::{collections::VecDeque, sync::Arc};
use uuid::Uuid;

#[derive(Clone, Debug)]
pub enum InstanceMsg {
    ClientJoined {
        client_id:Uuid,
        client_name:String
    },
    ClientLeft {
        client_id:Uuid
    },
    CustomMsg {
        client_id:Uuid,
        msg:Vec<u8>
    }
}

#[derive(Clone, Debug)]
pub enum GameServerMsg {
    CustomToAll {
        msg:Vec<u8>
    },
    CustomTo {
        client_id:Uuid,
        msg:Vec<u8>
    }
}

#[derive(Debug)]
pub struct Config {
    pub tick_rate:u64,
    pub max_players:u32
}

pub struct Ctx {
    pub instance_messages:VecDeque<InstanceMsg>,
    pub game_messages:VecDeque<GameServerMsg>,

    /// delta time between ticks in seconds between ticks
    /// this value can go from close zero to many thousands 
    /// and needs to be truncated or similar by the consumer to avoid
    /// unintended behavior, e.g. players jumping through walls due to high tick
    pub delta:f64,
    pub time:f64
}

impl Ctx {
    pub fn pop_host_msg(&mut self) -> Option<InstanceMsg> {
        let msg = self.instance_messages.pop_front();
        return msg;
    }

    pub fn push_game_msg(&mut self, msg:GameServerMsg) {
        let msg = msg.into();
        self.game_messages.push_back(msg);
    }
}

pub trait Server : Send + Sync + 'static {
    fn init(&mut self) -> Config;
    fn tick(&mut self, ctx:&mut Ctx);
}

pub type GameServerConstructorFn = Box<dyn Fn() -> Box<dyn Server> + Send + Sync>;

#[derive(Clone)]
pub struct Constructor {
    arc:Arc<GameServerConstructorFn>
}


impl Constructor {
    pub fn new_constructor(f:GameServerConstructorFn) -> Self {
        Self {
            arc:Arc::new(Box::new(f))
        }
    }

    pub fn new<T:Server + Default>() -> Self {

        let f:fn()->Box<dyn Server> = || {
            return Box::new(T::default());
        };

        let boxed = Box::new(f);
        Self {
            arc:Arc::new(boxed)
        }
    }

    pub fn construct(&self) -> Box<dyn Server> {
        let f = self.arc.as_ref();
        f()
    }
}
