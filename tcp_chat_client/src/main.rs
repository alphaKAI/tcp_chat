use std::io::{self, ErrorKind, Write};
use std::net::TcpStream;
use std::sync::mpsc::{self, TryRecvError};
use std::thread;
use std::time::Duration;

use tcp_chat_common::{Message, MessageType};

fn sleep() {
    thread::sleep(Duration::from_millis(100));
}

fn cli_frontend(mut client: TcpStream) {
    let (tx, rx) = mpsc::channel::<Message>();

    // 受信用スレッド
    let reciever = thread::spawn(move || loop {
        match Message::parse_from_socket(&mut client) {
            Ok(message) => {
                //println!("message recv {:?}", message);
                match message.mtype {
                    MessageType::CHAT_MESSAGE => {
                        println!("{}", message.content_body);
                    }
                    _ => {}
                }
            }
            Err(ref err) if err.kind() == ErrorKind::WouldBlock => (),
            Err(e) => {
                println!("connection with server was serverd, reason: {:?}", e);
                break;
            }
        }

        match rx.try_recv() {
            Ok(msg) => {
                client
                    .write_all(&msg.into_bytes())
                    .expect("writing to socket failed");
                //println!("message send {:?}", msg);
            }
            Err(TryRecvError::Empty) => (),
            Err(TryRecvError::Disconnected) => break,
        }

        sleep();
    });

    // 送信用スレッド
    let sender = thread::spawn(move || {
        print!("Enter your name: ");
        std::io::stdout().flush().expect("failed to flush stdout");
        let mut name = String::new();
        std::io::stdin()
            .read_line(&mut name)
            .expect("failed to read name");
        if tx
            .send(Message::new(
                MessageType::REG_NAME,
                name.trim_end().to_string(),
            ))
            .is_ok()
        {
            println!("Write a Message");
            loop {
                let mut buff = String::new();
                io::stdin()
                    .read_line(&mut buff)
                    .expect("reading from stdin failed");
                let content = buff.trim().to_string();
                let message = Message::new(MessageType::CHAT_MESSAGE, content.clone());

                if content == ":quit" || tx.send(message).is_err() {
                    break;
                }
            }
        }
    });

    sender.join().unwrap();
    reciever.join().unwrap();

    println!("bye bye!");
}

fn main() {
    let mut server_host = String::new();
    print!("Enter TCP Chat Server Host: ");
    std::io::stdout().flush().expect("failed to flush stdout");
    std::io::stdin()
        .read_line(&mut server_host)
        .expect("failed to read server host");
    server_host = server_host.trim().to_string();
    let client = TcpStream::connect(server_host.clone()).expect("Stream failed to connect");
    client
        .set_nonblocking(true)
        .expect("failed to initiate non-blocking");

    println!("Successfully connected to the server({})", &server_host);

    cli_frontend(client);
}
