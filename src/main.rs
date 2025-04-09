use futures::{SinkExt, StreamExt};
use std::collections::HashSet;
use std::io::*;
use std::sync::{Arc, Mutex};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast::{self, Sender};
use tokio_util::codec::{FramedRead, FramedWrite, LinesCodec};

#[derive(Clone)]
struct Names(Arc<Mutex<HashSet<String>>>);

impl Names {
    fn new() -> Self {
        Self(Arc::new(Mutex::new(HashSet::new())))
    }

    fn insert(&self, name: String) -> bool {
        self.0.lock().unwrap().insert(name)
    }

    fn get_name(&self) -> String {
        let mut name = String::new();
        let mut m_guard = self.0.lock().unwrap();
        println!("Enter a nickname: ");
        stdin()
            .read_line(&mut name)
            .expect("error reading username.");
        while !m_guard.insert(name.clone()) {
            println!("enter a new nickname");
            stdin()
                .read_line(&mut name)
                .expect("error reading new nickname.");
        }
        name
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let listener = TcpListener::bind("127.0.0.1:8080").await?;
    let names = Names::new();
    let (tx, _rx) = broadcast::channel::<String>(32);
    loop {
        let (socket, _addr) = listener.accept().await?;
        tokio::spawn(handle_client(socket, tx.clone(), names.clone()));
    }
}

async fn handle_client(mut socket: TcpStream, tx: Sender<String>, names: Names) -> Result<()> {
    let (reader, writer) = socket.split();
    let mut stream = FramedRead::new(reader, LinesCodec::new());
    let mut sink = FramedWrite::new(writer, LinesCodec::new());
    let mut rx = tx.subscribe();
    let name = names.get_name();
    let _ = sink.send(format!("You are {:?}", name)).await;
    loop {
        tokio::select! {
            user_msg = stream.next() => {
                let mut user_msg = match user_msg {
                    Some(msg) => msg.unwrap(),
                    None => break,
                };
                if user_msg.starts_with("/quit") {
                    break;
                }
                else {
                    tx.send(format!("{:?}: {:?}", name, user_msg)).expect("failed to send to channel");
                }
            },
            other_msg = rx.recv() => {
                let _ = sink.send(other_msg.unwrap()).await;
            },
        }
    }
    Ok(())
}
