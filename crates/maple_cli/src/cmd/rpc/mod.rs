mod filer;
mod on_init;
mod on_move;
mod types;

use crossbeam_channel::Sender;
use log::{debug, error};
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::prelude::*;
use std::thread;
use types::GlobalEnv;

static GLOBAL_ENV: OnceCell<GlobalEnv> = OnceCell::new();

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Message {
    pub method: String,
    pub params: serde_json::Map<String, Value>,
    pub id: u64,
}

fn write_response<T: Serialize>(msg: T) {
    if let Ok(s) = serde_json::to_string(&msg) {
        println!("Content-length: {}\n\n{}", s.len(), s);
    }
}

fn loop_read(reader: impl BufRead, sink: &Sender<String>) {
    let mut reader = reader;
    loop {
        let mut message = String::new();
        match reader.read_line(&mut message) {
            Ok(number) => {
                if number > 0 {
                    if let Err(e) = sink.send(message) {
                        println!("Failed to send message, error: {}", e);
                    }
                } else {
                    println!("EOF reached");
                }
            }
            Err(error) => println!("Failed to read_line, error: {}", error),
        }
    }
}

// Session {
// Provider
// Sender<Call>
// }

// Each session is associated with a provider.
//
// Each session can have OnTyped and OnMove event
//
// OnTyped -> query changes, rerun the filter against the source list.
// OnMove -> Preview
pub enum SessionEvent {
    OnTyped(Message),
    OnMove(Message),
}

use std::ops::Deref;

pub fn env() -> impl Deref<Target = GlobalEnv> {
    if let Some(x) = GLOBAL_ENV.get() {
        x
    } else {
        if cfg!(debug_assertions) {
            panic!("uninit static: FOO")
        } else {
            unsafe { std::hint::unreachable_unchecked() }
        }
    }
}

pub fn preview_size_of(provider_id: &str) -> usize {
    env().preview_size_of(provider_id)
}

// stdio channel
//  process SessionEvent
//     ->
//
// on_init => Start a new Session, invoke a new provider. Session(Provider)
// on_typed => send message via channel to Session(Provider)
// on_move
fn loop_handle_message(rx: &crossbeam_channel::Receiver<String>) {
    for msg in rx.iter() {
        // Ignore the invalid message.
        if let Ok(msg) = serde_json::from_str::<Message>(&msg.trim()) {
            let msg_id = msg.id;

            if let Err(err) = thread::Builder::new().name(format!("msg-handler-{}", msg_id)).spawn(move || {
                debug!("Recv: {:?}", msg);
                match &msg.method[..] {
                    "client.initialize_env" => {
                      let enable_icon = msg.params.get("enable_icon") .and_then(|x| x.as_bool()) .unwrap_or(false);
                      let preview_size = msg .params .get("clap_preview_size").expect("Missing clap_preview_size on initialize_env");

                      let global_env = GlobalEnv::new(enable_icon, preview_size.clone());
                      match GLOBAL_ENV.set(global_env) {
                        Ok(_) => debug!("GLOBAL_ENV initialized successfully"),
                        Err(e) => debug!("failed to initialized GLOBAL_ENV, error: {:?}", e)
                      }
                    }
                    "client.on_init" => {
                        if let Err(e) = on_init::handle_message(msg) {
                            write_response(json!({ "error": format!("{}",e), "id": msg_id }));
                        }
                    }
                    "client.on_typed" => filer::handle_message(msg),
                    "client.on_move" => {
                        if let Err(e) = on_move::handle_message(msg) {
                            write_response(json!({ "error": format!("{}",e), "id": msg_id }));
                        }
                    }
                    _ => write_response(
                        json!({ "error": format!("unknown method: {}", &msg.method[..]), "id": msg.id }),
                    ),
                }}) {
            error!("Failed to spawn for message-{}, error:{:?}", msg_id, err);
            }
        } else {
            error!("Invalid message: {:?}", msg);
        };
    }
}

pub fn run_forever<R>(reader: R)
where
    R: BufRead + Send + 'static,
{
    let (tx, rx) = crossbeam_channel::unbounded();
    thread::Builder::new()
        .name("reader".into())
        .spawn(move || {
            loop_read(reader, &tx);
        })
        .expect("Failed to spawn rpc reader thread");
    loop_handle_message(&rx);
}
