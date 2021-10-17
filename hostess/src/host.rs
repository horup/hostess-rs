use std::{collections::{HashMap, VecDeque}, time::Duration};

use tokio::{sync::{mpsc::channel, mpsc::Sender}, time::{interval}};
use uuid::Uuid;
use log::{info};

use crate::{ClientMsg, ClientSink, ConnectedClient, Context, Game, GameMsg, HostInfo, HostMsg, ServerMsg};

enum Msg {
    HostMsg(HostMsg),
    ClientTransfer {
        client_id:Uuid,
        sink:ClientSink,
        return_sink:tokio::sync::oneshot::Sender<ClientSink>
    }
}
#[derive(Clone)]
pub struct Host {
    pub info:HostInfo,
    sender:Sender<Msg>,
}

impl Host {
    pub fn new<T:Game>(info:HostInfo) -> Self {
        let buffer_len = 1024;
        let (sender, mut receiver) = channel::<Msg>(buffer_len);
        let host = Self {
            info:info.clone(),
            sender,
        };

        tokio::spawn(async move {
            let mut g = T::new();
            let period = Duration::from_millis(1000 / g.tick_rate());
            let mut timer = interval(period);

            let mut context = Context {
                game_messages:VecDeque::new(),
                host_messages:Vec::with_capacity(buffer_len)
            };

            let mut clients:HashMap<Uuid, (ClientSink, tokio::sync::oneshot::Sender<ClientSink>)> = HashMap::new();

            loop {
                loop {
                    match receiver.try_recv() {
                        Ok(msg) => {
                            match msg {
                                Msg::HostMsg(msg) => {
                                    info!("{:?}", msg);
                                    match &msg {
                                        HostMsg::ClientLeft { client_id } => {
                                            if let Some((tx, transfer)) = clients.remove(client_id) {
                                                let _ = transfer.send(tx);
                                            }
                                        },
                                        _=>{}
                                    }

                                    context.host_messages.push(msg);
                                },
                                Msg::ClientTransfer { 
                                    client_id, 
                                    sink: mut tx, 
                                    return_sink: return_tx 
                                } => {
                                    context.host_messages.push(HostMsg::ClientJoined {
                                        client_id:client_id
                                    });
                                    let _ = tx.send(ServerMsg::HostJoined {
                                        host:info.clone()
                                    }).await;

                                    clients.insert(client_id, (tx, return_tx));
                                },
                            }
                        },
                        Err(_) => {
                            break;
                        },
                    }
                }

                //messages_to_game.read().await;
                g.update(&mut context);

                // TODO: can be fuffered maybe?
                for msg in context.game_messages.drain(..) {
                    match msg {
                        GameMsg::CustomToAll { msg } => {
                            for (sink, _) in &mut clients.values_mut() {
                                let _ = sink.send(ServerMsg::Custom{
                                    msg:msg.clone()
                                }).await;
                            }
                        },
                        GameMsg::CustomTo { client_id, msg } => {
                            if let Some((sink, _)) = clients.get_mut(&client_id) {
                                let _ = sink.send(ServerMsg::Custom{
                                    msg:msg.clone()
                                }).await;
                            }
                        },
                    }
                }
                
                //while let Some(msg) in context.game_messages.remove(0)
                timer.tick().await;
            }
        });

        host
    }

    pub async fn join(&self, client:ConnectedClient) -> Option<ConnectedClient> {
        info!("Client {} joined Host {}", client.client_id, self.info.id);
        let tx = client.sink;
        let mut rx = client.stream;

        let (return_tx, return_rx) = tokio::sync::oneshot::channel::<ClientSink>();
        let host_sender = self.sender.clone();
        let _ = host_sender.send(Msg::ClientTransfer {
            client_id: client.client_id,
            sink: tx,
            return_sink: return_tx,
        }).await;

        while let Some(msg) = rx.next::<ClientMsg>().await {
            match msg {
                Ok(msg) => {
                    match msg {
                        ClientMsg::LeaveHost {} => {
                            // exit while and leave host
                            break;
                        },
                        ClientMsg::CustomMsg {
                            msg
                        } => {
                            let _ = host_sender.send(Msg::HostMsg(HostMsg::CustomMsg {
                                client_id:client.client_id,
                                msg
                            })).await;
                        }
                        _ => {}
                    }
                },
                Err(_) => {
                    break;
                },
            }
        }

        let _ = host_sender.send(Msg::HostMsg(HostMsg::ClientLeft {
            client_id:client.client_id
        })).await;
        
        info!("Client {} left Host {}", client.client_id, self.info.id);
        if let Ok(tx) = return_rx.await {
            return Some(ConnectedClient {
                sink: tx,
                stream: rx,
                client_id:client.client_id
            });
        };

        None
    }
}