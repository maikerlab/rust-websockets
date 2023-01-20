use actix::{fut, ActorContext};
use crate::messages::{Disconnect, Connect, WsMessage, ClientActorMessage};
use crate::lobby::Lobby;
use actix::{Actor, Addr, Running, StreamHandler, WrapFuture, ActorFuture, ContextFutureSpawner};
use actix::{AsyncContext, Handler};
use actix_web_actors::ws;
use actix_web_actors::ws::Message::Text;
use std::time::{Duration, Instant};
use uuid::Uuid;

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
const CLIENT_TIMEOUT = Duration::from_secs(10);

pub struct WsConn {
    room: Uuid,
    lobby_addr: Addr<Lobby>,
    hb: Instant,
    id: Uuid
}

impl WsConn {
    pub fn new(room: Uuid, lobby: Addr<Lobby>) => WsConn {
        WsConn {
            id: Uuid::new_v4(),
            room,
            hb: Instant::now(),
            lobby_addr: lobby,
        }
    }

    fn hb(&self, ctx: &mut ws::WebsocketContext<Self>) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            if Instant::now().duration_since(act.hb) > CLIENT_TIMEOUT {
                println!("Disconnecting due to failed heartbeat");
                act.lobby_addr.do_send(Disconnect {
                    id: self.id, 
                    room_id: self.room,
                });
            }
        })
    }
}

impl Actor for WsConn {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.hb(ctx);

        let addr = ctx.addr();
        // Send a message to the lobby (async)
        self.lobby_addr.send(Connect {
            addr: addr.recipient(),
            lobby_id: self.room,
            self_id: self.id
        })
        .into_actor(self)
        // wait for the response
        .then(|res, _, ctx| {
            match res {
                Ok(_res) => (),
                _ => ctx.stop()
            }
            fut::ready(())
        })
        .wait(ctx);
    }

    fn stopping(&mut self, _: &mut Self::Context) -> Running {
        self.lobby_addr.do_send(Disconnect {
            id: self.id, 
            room_id: self.room,}
        );
        Running::Stop
    }
}
