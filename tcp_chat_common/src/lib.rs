use std::io::{ErrorKind, Read};
use std::mem::size_of;
use std::net::TcpStream;

/*
Protocol:
    <Message Format>
        Content-Length: usize <7bytes>
        Message-Type: REG_NAME | CHAT_MESSAGE
        BodyContent

*/
#[allow(non_camel_case_types)]
#[derive(Debug, Clone)]
pub enum MessageType {
    REG_NAME,
    CHAT_MESSAGE,
}

impl MessageType {
    fn as_str(&self) -> &str {
        match self {
            Self::REG_NAME => "REG_NAME",
            Self::CHAT_MESSAGE => "CHAT_MESSAGE",
        }
    }
}

impl MessageType {
    fn from_str(s: &str) -> Option<Self> {
        match s {
            "REG_NAME" => Some(Self::REG_NAME),
            "CHAT_MESSAGE" => Some(Self::CHAT_MESSAGE),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Message {
    pub content_size: usize,
    pub mtype: MessageType,
    pub content_body: String,
}

const MESSAGE_TYPE_HEADER: &str = "Message-Type:";

impl Message {
    fn gen_message_type_str(mtype: &MessageType) -> String {
        format!("{} {}\n", MESSAGE_TYPE_HEADER, mtype.as_str())
    }

    fn calc_size(mtype: &MessageType, content_body: &String) -> usize {
        let mtype_str = Self::gen_message_type_str(mtype);
        mtype_str.len() + content_body.len()
    }

    pub fn new(mtype: MessageType, content_body: String) -> Self {
        let content_size = Self::calc_size(&mtype, &content_body);
        Self {
            content_size: content_size,
            mtype: mtype,
            content_body: content_body,
        }
    }

    pub fn parse_from_socket(socket: &mut TcpStream) -> std::result::Result<Self, std::io::Error> {
        let mut buf = [0; size_of::<usize>()];
        socket
            .read_exact(&mut buf)
            .and_then(|_| Ok(usize::from_be_bytes(buf)))
            .and_then(|content_size| {
                let mut buf = vec![0; content_size];
                match socket.read_exact(&mut buf) {
                    Ok(_) => {
                        let content_size = content_size;
                        let raw_message = String::from_utf8(buf).expect("Invalid UTF-8 sequence");
                        let raw_content_body = raw_message.lines().collect::<Vec<_>>();

                        //println!("raw_message: {:?}", raw_message);

                        if raw_content_body.len() >= 2 {
                            let message_type = raw_content_body[0];
                            if message_type.starts_with(MESSAGE_TYPE_HEADER) {
                                message_type
                                    .split(":")
                                    .nth(1)
                                    .and_then(|x| MessageType::from_str(x.trim()))
                                    .and_then(|message_type| {
                                        let mut content_body = raw_content_body
                                            .into_iter()
                                            .map(String::from)
                                            .collect::<Vec<_>>();
                                        content_body.remove(0);

                                        Some(Self {
                                            content_size: content_size,
                                            mtype: message_type,
                                            content_body: content_body.join(""),
                                        })
                                    })
                                    .ok_or(std::io::Error::from(ErrorKind::InvalidData))
                            } else {
                                std::result::Result::Err(std::io::Error::from(
                                    ErrorKind::InvalidData,
                                ))
                            }
                        } else {
                            std::result::Result::Err(std::io::Error::from(ErrorKind::InvalidData))
                        }
                    }
                    Err(err) => std::result::Result::Err(err),
                }
            })
    }

    pub fn update_size(&mut self) {
        self.content_size = Self::calc_size(&self.mtype, &self.content_body);
    }

    pub fn into_bytes(&self) -> Vec<u8> {
        let mut r = usize::to_be_bytes(self.content_size).to_vec();
        r.append(
            &mut Self::gen_message_type_str(&self.mtype)
                .to_string()
                .into_bytes(),
        );
        r.append(&mut self.content_body.clone().into_bytes());
        r
    }
}
