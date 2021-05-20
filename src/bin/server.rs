use std::{
    collections::HashMap,
    io,
    io::prelude::*,
    net,
    sync::{mpsc, Arc, Mutex},
};

const ERR: u8 = 0xff;
const OK: u8 = 0x01;

type UsersList = Arc<Mutex<HashMap<Arc<String>, net::TcpStream>>>;

fn main() -> anyhow::Result<()> {
    let (socket, srv_name) = parse_args()?;
    let listener = net::TcpListener::bind(socket)?;

    // Client threads send messages through this channel to the broadcast thread
    let (msg_tx, msg_rx) = mpsc::channel::<Message>();

    let users: UsersList = Arc::new(Mutex::new(HashMap::new()));

    // Spawn the thread that broadcasts messages to connected clients
    {
        let handle = users.clone();
        std::thread::spawn(move || serve_messages(msg_rx, handle));
    }

    loop {
        let (mut stream, addr) = listener.accept()?;

        // Send server name
        stream.write_all(srv_name.as_bytes())?;
        stream.write_all(b"\n")?;

        // Receive the client's nickname
        let nickname = {
            let mut reader = io::BufReader::new(stream.try_clone().unwrap());
            let mut client_name = String::new();
            reader.read_line(&mut client_name)?;
            client_name.pop();
            Arc::new(client_name)
        };

        let tx = msg_tx.clone();
        // Check if the nickname is available
        {
            let mut users = users.lock().unwrap();
            if users.contains_key(&*nickname) {
                stream.write_all(&[ERR])?;
                stream.write_all(b"Nickname already taken!\n")?;
            } else {
                stream.write_all(&[OK])?;
                users.insert(nickname.clone(), stream.try_clone().unwrap());
                tx.send(Message::Connect(nickname.clone(), addr))?;
                std::thread::spawn(move || handle_client(stream, tx, nickname));
            }
        }
    }
}

#[derive(Debug)]
enum Message {
    Connect(Arc<String>, net::SocketAddr),
    Disconnect(Arc<String>),
    Text(Arc<String>, String),
}

fn handle_client(
    stream: net::TcpStream,
    tx: mpsc::Sender<Message>,
    nick: Arc<String>,
) -> anyhow::Result<()> {
    let mut reader = io::BufReader::new(stream.try_clone().unwrap());

    loop {
        let mut msg = String::new();
        if reader.read_line(&mut msg)? == 0 {
            tx.send(Message::Disconnect(nick))?;
            return Ok(());
        }
        match msg.as_str() {
            "disconnect\n" => break,
            text if msg.len() > 1 => {
                tx.send(Message::Text(nick.clone(), text.to_string()))?;
            }
            _ => {}
        }
    }

    tx.send(Message::Disconnect(nick))?;
    std::io::stdout().flush().unwrap();
    Ok(())
}

fn serve_messages(rx: mpsc::Receiver<Message>, users: UsersList) -> anyhow::Result<()> {
    loop {
        let msg = rx.recv()?;
        let mut users = users.lock().unwrap();
        match msg {
            Message::Connect(nick, addr) => {
                println!("User {} connected from ip: {}!", nick, addr);
                for (uid, user) in users.iter_mut() {
                    if *uid != nick {
                        user.write_fmt(format_args!(
                            "User {} connected from ip: {}!\n",
                            nick, addr
                        ))?;
                    }
                }
            }
            Message::Disconnect(nick) => {
                println!("User {} disconnected.", nick);
                // Properly disconnect the user
                let u = users.get_mut(&nick).unwrap();
                u.flush()?;
                u.shutdown(net::Shutdown::Both)?;
                users.remove(&*nick);
                for (uid, user) in users.iter_mut() {
                    if *uid != nick {
                        user.write_fmt(format_args!("User {} disconnected.\n", nick))?;
                    }
                }
            }
            Message::Text(nick, text) => {
                print!("{}: {}", nick, text);
                for (uid, user) in users.iter_mut() {
                    if *uid != nick {
                        user.write_fmt(format_args!("{}: {}", nick, text))?;
                    }
                }
            }
        }
        std::io::stdout().flush().unwrap();
    }
}

fn parse_args() -> anyhow::Result<(String, String)> {
    use std::env::args;
    match args().count() {
        3 => Ok((args().nth(1).unwrap(), args().nth(2).unwrap())),
        _ => Err(anyhow::anyhow!(
            "Expected 2 arguments: socket address, server name"
        )),
    }
}
