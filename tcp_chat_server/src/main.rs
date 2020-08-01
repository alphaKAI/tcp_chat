use std::collections::HashMap;
use std::io::{ErrorKind, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::mpsc;
use std::thread;

use tcp_chat_common::{Message, MessageType};

fn sleep() {
    thread::sleep(::std::time::Duration::from_millis(100));
}

fn broad_cast_message(clients: Vec<TcpStream>, message: &Message) -> Vec<TcpStream> {
    clients
        .into_iter()
        .filter_map(|mut client| {
            let buff = message.into_bytes();
            client.write_all(&buff).map(|_| client).ok()
        })
        .collect::<Vec<_>>()
}

fn main() {
    let server_host = "0.0.0.0:3000";
    let server = TcpListener::bind(server_host).expect("Listener failed to bind");
    server
        .set_nonblocking(true)
        .expect("failed to initiate non-blocking");

    let mut clients = vec![];
    let mut client_to_name = HashMap::new();
    let (tx, rx) = mpsc::channel::<(SocketAddr, Message)>();

    println!("Server started!");

    loop {
        if let Ok((mut socket, addr)) = server.accept() {
            println!("Client {} connected", addr);

            let tx = tx.clone();
            clients.push(socket.try_clone().expect("failed to clone client"));
            client_to_name.insert(addr.clone(), String::from("Unkown"));

            thread::spawn(move || loop {
                match Message::parse_from_socket(&mut socket) {
                    Ok(message) => {
                        println!("{}: {:?}", addr, message);
                        tx.send((addr, message)).expect("Failed to send msg to rx");
                    }
                    Err(ref err) if err.kind() == ErrorKind::WouldBlock => (),
                    Err(e) => {
                        println!("closing connection with: {}, reason: {:?}", addr, e);
                        break;
                    }
                }

                sleep();
            });
        }

        if let Ok((addr, message)) = rx.try_recv() {
            let mut messages = vec![];

            let message = match message.mtype {
                MessageType::CHAT_MESSAGE => {
                    let unkown = String::from("Unkown");
                    let name = client_to_name.get(&addr).unwrap_or(&unkown);
                    println!("[{}]: {}", name, message.content_body);

                    let content_body = format!("[{}]: {}", name.clone(), message.content_body);
                    let message = Message::new(message.mtype, content_body);
                    message
                }
                MessageType::REG_NAME => {
                    let name = &message.content_body;
                    client_to_name.insert(addr.clone(), name.clone());

                    messages.push(Message::new(
                        MessageType::CHAT_MESSAGE,
                        format!("[Server]: Hello {}!", name),
                    ));

                    message
                }
            };
            messages.push(message);

            // ブロードキャストと同時に生存しているクライアント一覧を更新する
            while let Some(message) = messages.pop() {
                clients = broad_cast_message(clients, &message);
            }
        }

        sleep();
    }
}
