mod filer;

use crossbeam_channel::Sender;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::prelude::*;
use std::thread;
use structopt::StructOpt;

const REQUEST_FILER: &str = "filer";

/// Fuzzy filter the current vim buffer given the query.
#[derive(StructOpt, Debug, Clone)]
pub struct Rpc {
    /// Check if the newer release binary is avaliable.
    #[structopt(short, long)]
    check_new_release: bool,
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
        loop_handle_message(&rx);
    }

    fn notify<T: Serialize>(&self, msg: T) {
        if let Ok(s) = serde_json::to_string(&msg) {
            println!("Content-length: {}\n\n{}", s.len(), s);
        }
    }
}

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

fn handle_message_on_move(msg: Message) {
    let cwd = String::from(
        msg.params
            .get("cwd")
            .and_then(|x| x.as_str())
            .unwrap_or("Missing cwd when deserializing into FilerParams"),
    );

    let fname = String::from(
        msg.params
            .get("fname")
            .and_then(|x| x.as_str())
            .unwrap_or("Missing fname when deserializing into FilerParams"),
    );

    let enable_icon = msg
        .params
        .get("enable_icon")
        .and_then(|x| x.as_bool())
        .unwrap_or(false);

    let provider_id = msg
        .params
        .get("id")
        .and_then(|x| x.as_str())
        .unwrap_or("Unknown provider id");

    if provider_id == "grep" {
        lazy_static::lazy_static! {
            static ref GREP_RE: regex::Regex = regex::Regex::new(r"^(.*):\d+:\d+:").unwrap();
        }
    }

    if let Ok(line_iter) = crate::utils::read_first_lines(fname.clone(), 10) {
        let mut lines = line_iter.collect::<Vec<_>>();
        let abs_path = std::fs::canonicalize(&std::path::PathBuf::from(fname))
            .unwrap()
            .into_os_string()
            .into_string()
            .unwrap();
        lines.insert(0, abs_path);
        write_response(json!({ "lines": lines, "id": msg.id }));
    } else {
        write_response(json!({ "data": serde_json::to_string(&msg).unwrap(), "id": msg.id }));
    }
}

fn loop_handle_message(rx: &crossbeam_channel::Receiver<String>) {
    for msg in rx.iter() {
        thread::spawn(move || {
            // Ignore the invalid message.
            if let Ok(msg) = serde_json::from_str::<Message>(&msg.trim()) {
                match &msg.method[..] {
                    REQUEST_FILER => filer::handle_message(msg),
                    "client.on_move" => handle_message_on_move(msg),
                    _ => write_response(json!({ "error": "unknown method", "id": msg.id })),
                }
            }
        });
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

#[test]
fn test_grep_regex() {
    use regex::Regex;
    lazy_static::lazy_static! {
        static ref GREP_RE: Regex = regex::Regex::new(r"^(.*):(\d+):(\d+):").unwrap();
    }
    let line = "mock/mock.go:35:1:func NewMockSectorMgr(ssize abi.SectorSize) *SectorMgr {";
    let fname = GREP_RE
        .captures(line)
        .and_then(|cap| cap.get(1))
        .map(|m| m.as_str())
        .unwrap();

    let lnum = GREP_RE
        .captures(line)
        .and_then(|cap| cap.get(2))
        .map(|m| m.as_str())
        .unwrap();

    println!("fname: {:?}, lnum: {}", fname, lnum);
    // .and_then(|cap| cap.get(1))
    // .map(|m| m.as_str())
    // .unwrap_or(DEFAULT_ICON)
}
