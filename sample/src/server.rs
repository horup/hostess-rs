use std::{collections::{HashMap, VecDeque}, ops::IndexMut, time::Instant};
use glam::Vec2;
use hostess::{Bincoded, log::info, game_server::{Context, GameServer, GameServerMsg, HostMsg}, uuid::Uuid};
use sample_lib::{CustomMsg, Input, Player, State, StateHistory, Thing, apply_input, update_things};
use serde::{Serialize, Deserialize};
use crate::bot::*;

pub struct Server {
    current:State,
    history:StateHistory,
    players:HashMap<Uuid, Player>,
    bots:Vec<Bot>
}

impl Server {
    pub fn new() -> Self {
       
        Self {
            current:State::new(),
            players:HashMap::new(),
            bots:Vec::new(),
            history:StateHistory::new()
        }
    }

    pub fn update(&mut self, context:&mut Context) {
        // clear events 
        self.current.events.clear();
        self.current.timestamp = context.time;
        if self.players.len() < 2 {
            // less than two players and no bots, ensure 10 bots are spawned
            if self.bots.len() == 0 {
                while self.bots.len() < 10 {
                    let mut thing = Thing::random_new_player(&self.current);
                    thing.name = "bot".into();
                    let index = self.current.things.insert(thing);
                    let bot = Bot::new(index);
    
                    self.bots.push(bot);
                }
            }
        } else {
            // more than two players, remove bots and their things
            for bot in self.bots.drain(..) {
                self.current.things.remove(bot.thing_id);
            }
        }

        let tick_rate = self.tick_rate() as u8;

        // do generic update of things, such as moving projectiles
        update_things(&mut self.current, context.delta);
        
        // process inputs from players
        for (_, player) in &mut self.players {
            // if player has no 'thing'
            // ensure one is spawned for the player
            if player.thing == None {
                let mut thing = Thing::random_new_player(&self.current);
                thing.name = player.client_name.clone();
                player.thing = Some(self.current.things.insert(thing));
                // let the player know his thing id and tick_rate
                push_custom_to(context, player.client_id, CustomMsg::ServerPlayerInfo {
                    thing_id:player.thing,
                    tick_rate:tick_rate
                });
            }

            // apply input from players
            let mut trigger = false;
            let mut ability_target = Vec2::new(0.0, 0.0);
            for input in player.inputs.drain(..) {
                player.latest_input_timestamp_sec = input.timestamp_sec;
                ability_target = input.ability_target;
                if input.ability_trigger {
                    trigger = true;
                }

                apply_input(&mut self.current, &input, true);

                let mut spawn = Vec::new();
                if let Some(thing_id) = player.thing {
                    if let Some(thing) = self.current.things.get_mut(thing_id) {
                        if let Some(player) = thing.as_player_mut() {
                            if player.health > 0.0 && trigger && player.ability_cooldown <= 0.0 {
                                player.ability_cooldown = 0.25;
                                let dir = ability_target - thing.pos;
                                if dir.length() > 0.0 {
                                    let dir = dir.normalize();
                                    let mut v = dir * 20.0;
                                    if let Some(old) = self.history.prev().things.get(thing_id) {
                                        v += thing.pos - old.pos;
                                    }
                                    let p = Thing::new_projectile(thing.pos, v, thing_id);
                                    spawn.push(p);
                                }
                            }
                        }
                    }
                }
                
                for thing in spawn.drain(..) {
                    self.current.things.insert(thing);
                }
            }
        }

        // process bots
        for bot in self.bots.iter_mut() {
            bot.tick(&mut self.current, context.delta);
        }

        // for each player, transmit state diff
        for (client_id, player) in &mut self.players {
            let delta = self.current.to_delta_bincode(&player.state);
            push_custom_to(context, *client_id, CustomMsg::ServerSnapshotDelta {
                input_timestamp_sec:player.latest_input_timestamp_sec,
                delta
            });

            player.state = self.current.clone()
        }

        // remember current state
        self.history.remember(self.current.clone());
    }
}

impl GameServer for Server {
    fn tick_rate(&self) -> u64 {
        20
    }

    fn tick(&mut self, mut context:Context) -> Context {
        while let Some(msg) = context.pop_host_msg() {
            match msg {
                HostMsg::ClientJoined { client_id, client_name } => {
                    if !self.players.contains_key(&client_id) {
                        self.players.insert(client_id, Player {
                            client_id:client_id,
                            client_name,
                            thing:None,
                            inputs:VecDeque::default(),
                            latest_input_timestamp_sec: 0.0,
                            state:self.current.clone()
                        });
                    }

                    push_custom_to(&mut context, client_id, CustomMsg::ServerSnapshotFull {
                        input_timestamp_sec:0.0,
                        state:self.current.clone()
                    });

                    push_custom_to(&mut context, client_id, CustomMsg::ServerPlayerInfo {
                        thing_id:None,
                        tick_rate:self.tick_rate() as u8
                    });
                },
                HostMsg::ClientLeft { client_id } => {
                    if let Some(player) = self.players.remove(&client_id) {
                        if let Some(thing_id) = player.thing {
                            self.current.things.remove(thing_id);
                        }
                    }
                },
                HostMsg::CustomMsg { client_id, msg } => {
                    if let Some(msg) = Bincoded::from_bincode(&msg) {
                        self.recv_custom_msg(&mut context, client_id, msg);
                    }
                },
            }
        }

        self.update(&mut context);

       

        return context;
    }
}

fn push_custom_all(context:&mut Context, msg:CustomMsg) {
    let msg = msg.to_bincode();
    context.push_game_msg(GameServerMsg::CustomToAll {
        msg
    });
}
fn push_custom_to(context:&mut Context, client_id:Uuid, msg:CustomMsg) {
    let msg = msg.to_bincode();
    context.push_game_msg(GameServerMsg::CustomTo {
        client_id,
        msg
    });
}

impl Server {
    /// is called on each custom message received from the clients
    pub fn recv_custom_msg(&mut self, context:&mut Context, client_id:Uuid, msg:CustomMsg) {
        match msg {
            CustomMsg::ClientInput { input } => {
                if let Some(player) = self.players.get_mut(&client_id) {
                    // remember input for later processing
                    player.inputs.push_back(input);
                }
            },
            _ => {}
        }
    }
}