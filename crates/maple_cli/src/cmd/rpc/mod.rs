mod filer;

use crossbeam_channel::Sender;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::prelude::*;
use std::thread;
use structopt::StructOpt;

#[derive(StructOpt, Debug, Clone)]
pub struct Rpc {
    /// Check if there is a newer maple release.
    #[structopt(short, long)]
    check_release: bool,
}

impl Rpc {
    pub fn run_forever<R>(&self, reader: R)
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

        // spawn a check release thread

        loop_handle_message(&rx);
    }
}

const REQUEST_FILER: &str = "filer";

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

pub(super) fn handle_rpc_message(msg: Message) -> anyhow::Result<()> {
    let latest_remote_release = crate::cmd::check_release::latest_remote_release()?;
    let version_number =
        crate::cmd::check_release::extract_remote_version_number(&latest_remote_release.tag_name);
    write_response(json!({ "version_number": version_number, "id": msg.id }));
    Ok(())
}

fn loop_handle_message(rx: &crossbeam_channel::Receiver<String>) {
    for msg in rx.iter() {
        thread::spawn(move || {
            // Ignore the invalid message.
            if let Ok(msg) = serde_json::from_str::<Message>(&msg.trim()) {
                match &msg.method[..] {
                    REQUEST_FILER => filer::handle_message(msg),
                    "clap.rpc" => {
                        let _ = handle_rpc_message(msg);
                    }
                    _ => write_response(json!({ "error": "unknown method", "id": msg.id })),
                }
            }
        });
    }
}
