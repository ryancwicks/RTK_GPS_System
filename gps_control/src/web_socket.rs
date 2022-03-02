use std::time::{Duration, Instant};

use actix::prelude::*;
use actix::Message;
use actix_web::{ web, Error, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use uuid::Uuid;
use std::collections::HashMap;

use crate::gps_interface::GPSData;

/// How often heartbeat pings are sent
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
/// How long before lack of client response causes a timeout
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);


/// This message is used to register new GPS web sockets with the GPSWebSocketMonitor
#[derive(Message)]
#[rtype(result = "()")]
struct RegisterGPSWebSocketClient {
    uuid: Uuid,
    addr: Addr<GPSWebSocket>,
}

///Remove this web socket from the monitor list.
#[derive(Message)]
#[rtype(result = "()")]
struct DeRegisterGPSWebSocketClient {
    uuid: Uuid,
} 

/// This message is sent via the GPSWebSocketMonitor to all the GPS web sockets.
#[derive(Message, Clone)]
#[rtype(result = "()")]
pub struct GPSEvent {
    pub data: GPSData,
}



/// do websocket handshake and start `MyWebSocket` actor
pub async fn ws_index(r: HttpRequest, stream: web::Payload, data: web::Data<Addr<GPSWebSocketMonitor>>,) -> Result<HttpResponse, Error> {
    log::info!("{:?}", r);
    let uuid = Uuid::new_v4();
    let (addr, res) = ws::WsResponseBuilder::new(GPSWebSocket::new(&uuid, data.get_ref()), &r, stream).start_with_addr()?;

    data.get_ref().do_send(RegisterGPSWebSocketClient { uuid: uuid, addr: addr });

    log::info!("{:?}", res);
    Ok(res)
}

/// websocket connection is long running connection, it easier
/// to handle with an actor
struct GPSWebSocket {
    /// unique identifier of the socket
    uuid: Uuid,
    ///Used to communicate with containing actor to tell it when this socket closes.
    monitor_address: Addr<GPSWebSocketMonitor>,
    /// heartbeat
    hb: Instant,
}

impl Actor for GPSWebSocket {
    type Context = ws::WebsocketContext<Self>;

    /// Method is called on actor start. We start the heartbeat process here.
    fn started(&mut self, ctx: &mut Self::Context) {
        self.hb(ctx);
    }
}

/// Handler for `ws::Message`
impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for GPSWebSocket {
    fn handle(
        &mut self,
        msg: Result<ws::Message, ws::ProtocolError>,
        ctx: &mut Self::Context,
    ) {
        // process websocket messages
        match msg {
            Ok(ws::Message::Ping(msg)) => {
                self.hb = Instant::now();
                ctx.pong(&msg);
            }
            Ok(ws::Message::Pong(_)) => {
                self.hb = Instant::now();
            }
            Ok(ws::Message::Text(_)) => {},//No reason to respond
            Ok(ws::Message::Binary(_)) => {},//ditto
            Ok(ws::Message::Close(reason)) => {
                ctx.close(reason);
                ctx.stop();
                self.monitor_address.do_send( DeRegisterGPSWebSocketClient{uuid: self.uuid} );
            }
            _ => {
                ctx.stop();
                self.monitor_address.do_send( DeRegisterGPSWebSocketClient{uuid: self.uuid} );                
            },
        }
    }
}

impl GPSWebSocket {
    fn new(uuid: &Uuid, monitor_address: &Addr<GPSWebSocketMonitor>) -> Self {
        Self { uuid: uuid.clone(), monitor_address: monitor_address.clone(), hb: Instant::now()}
    }

    /// helper method that sends ping to client every second.
    ///
    /// also this method checks heartbeats from client
    fn hb(&self, ctx: &mut <Self as Actor>::Context) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            // check client heartbeats
            if Instant::now().duration_since(act.hb) > CLIENT_TIMEOUT {
                // heartbeat timed out
                log::error!("Websocket Client heartbeat failed, disconnecting!");

                // stop actor
                ctx.stop();
                act.monitor_address.do_send( DeRegisterGPSWebSocketClient{uuid: act.uuid} );

                // don't try to send a ping
                return;
            }

            ctx.ping(b"");
        });
    }
}


impl Handler<GPSEvent> for GPSWebSocket {
    type Result = ();

    fn handle(&mut self, msg: GPSEvent, ctx: &mut Self::Context) {
        match serde_json::to_string(&msg.data) {
            Ok(gps_data) => ctx.text(gps_data),
            Err(e) => log::error!("Failed to parse GPS data to json: {}", e)
        };
    }
}


///This structure keeps track of new web sockets and allows the GPS process to send data to running websockets.
pub struct GPSWebSocketMonitor {
    listeners: HashMap<Uuid, Addr<GPSWebSocket>>,
}

impl GPSWebSocketMonitor {
    pub fn new() -> Self {
        GPSWebSocketMonitor {
            listeners: HashMap::new()
        }
    }
}

impl Actor for GPSWebSocketMonitor {
    type Context = Context<Self>;

    fn started(&mut self, _: &mut Self::Context) {
        log::info!("Starting web socket monitoring actor.");
    }
}

impl Handler<RegisterGPSWebSocketClient> for GPSWebSocketMonitor {
    type Result = ();

    fn handle(&mut self, msg: RegisterGPSWebSocketClient, _: &mut Self::Context) {
        self.listeners.insert(msg.uuid, msg.addr);
    }
}

impl Handler<DeRegisterGPSWebSocketClient> for GPSWebSocketMonitor {
    type Result = ();

    fn handle(&mut self, msg: DeRegisterGPSWebSocketClient, _: &mut Self::Context) {
        self.listeners.remove(&msg.uuid);
    }
}

impl Handler<GPSEvent> for GPSWebSocketMonitor {
    type Result = ();

    fn handle(&mut self, msg: GPSEvent, _: &mut Context<Self>) {
        for (_, addr) in &self.listeners {
            addr.do_send(msg.clone());
        }
    }
}
