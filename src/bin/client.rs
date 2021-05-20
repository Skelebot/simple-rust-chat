use std::net;
use std::{io, sync::mpsc, thread};
use std::{io::prelude::*, sync::mpsc::TryRecvError};

const ERR: u8 = 0xFF;
const _OK: u8 = 0x01;

fn main() -> anyhow::Result<()> {
    let (ip, nickname) = parse_args()?;

    let mut stream = net::TcpStream::connect(ip)?;
    let mut reader = io::BufReader::new(stream.try_clone()?);

    let mut srv_name = String::new();
    reader.read_line(&mut srv_name)?;
    srv_name.pop(); // Pop the newline
    println!("Connected to {}!", srv_name);
    print!("> ");
    std::io::stdout().flush()?;

    // Write our nickname
    stream.write_all(nickname.as_bytes())?;
    stream.write_all(b"\n")?;

    // Check for errors
    let mut buf = [0; 1];
    if stream.read(&mut buf)? == 1 && buf[0] == ERR {
        let mut msg = String::new();
        reader.read_line(&mut msg)?;
        println!(" Server Error: {}", msg);
        return Err(anyhow::anyhow!("Server returned an error"));
    }

    let stdin_channel = spawn_stdin_channel();
    let stream_channel = spawn_stream_channel(stream.try_clone()?);

    loop {
        if let Ok(msg) = stdin_channel.try_recv() {
            match msg.as_str() {
                "/exit\n" | "/quit\n" => {
                    println!("\nDisconnected!");
                    break;
                }
                s => {
                    stream.write_all(s.as_bytes())?;
                    print!("> ");
                    std::io::stdout().flush()?;
                }
            }
        }
        match stream_channel.try_recv() {
            Ok(msg) => match msg.as_str() {
                "exit\n" => break,
                s => {
                    print!("\r< {}> ", s);
                    std::io::stdout().flush()?;
                }
            },
            Err(TryRecvError::Disconnected) => {
                println!("\nServer disconnected.");
                break;
            }
            _ => (),
        }
    }

    Ok(())
}

fn spawn_stdin_channel() -> mpsc::Receiver<String> {
    let (tx, rx) = mpsc::channel::<String>();
    thread::spawn(move || loop {
        let mut buffer = String::new();
        io::stdin().read_line(&mut buffer).unwrap();
        tx.send(buffer).unwrap();
    });
    rx
}

fn spawn_stream_channel(stream: net::TcpStream) -> mpsc::Receiver<String> {
    let (tx, rx) = mpsc::channel::<String>();
    let mut bufread = io::BufReader::new(stream);
    thread::spawn(move || loop {
        let mut buffer = String::new();
        if bufread.read_line(&mut buffer).unwrap() == 0 {
            return;
        }
        tx.send(buffer).unwrap();
    });
    rx
}

fn parse_args() -> anyhow::Result<(String, String)> {
    match std::env::args().count() {
        3 => Ok((
            std::env::args().nth(1).unwrap(),
            std::env::args().nth(2).unwrap(),
        )),
        _ => Err(anyhow::anyhow!("Expected 2 arguments: IP, client name")),
    }
}
