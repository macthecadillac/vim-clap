mod env;
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
use std::ops::Deref;
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

fn spawn_handle_thread(msg: Message) -> anyhow::Result<()> {
    let msg_id = msg.id;
    thread::Builder::new()
        .name(format!("msg-handle-{}", msg_id))
        .spawn(move || {
            let handle_result = match &msg.method[..] {
                "initialize_global_env" => env::initialize_global(msg),
                "client.on_init" => on_init::handle_message(msg),
                "client.on_typed" => filer::handle_message(msg),
                "client.on_move" => on_move::handle_message(msg),
                _ => Err(anyhow::anyhow!("Unknonw method: {}", msg.method)),
            };

            if let Err(e) = handle_result {
                write_response(json!({ "error": format!("{}",e), "id": msg_id }));
            }
        })?;
    Ok(())
}

fn loop_handle_message(rx: &crossbeam_channel::Receiver<String>) {
    for message in rx.iter() {
        if let Ok(msg) = serde_json::from_str::<Message>(&message.trim()) {
            debug!("Recv: {:?}", msg);
            let msg_id = msg.id;
            if let Err(err) = spawn_handle_thread(msg) {
                error!(
                    "Failed to spawn thread msg-handle-{}, error:{:?}",
                    msg_id, err
                );
            }
        } else {
            error!("Received invalid message: {:?}", message);
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
