use std::io::ErrorKind::WouldBlock;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::Duration;

use clap::{Arg, ArgMatches, Command, value_parser};
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;
use tokio::sync::mpsc::Sender;

use v5_core::connection::{RobotConnectionOptions, RobotConnection};
use v5_core::error::CommandError;

pub(crate) const COMMAND: &str = "daemon";
const USER_PORT: &str = "user";
const SYSTEM_PORT: &str = "system";

pub(crate) fn command() -> Command {
    Command::new(COMMAND)
        .about("Share connection to robot")
        .arg(
            Arg::new(SYSTEM_PORT)
                .default_value("5735")
                .value_parser(value_parser!(u16))
                .short('s'),
        )
        .arg(
            Arg::new(USER_PORT)
                .default_value("5736")
                .value_parser(value_parser!(u16))
                .short('u'),
        )
}

pub(crate) async fn daemon(
    _cmd: &mut Command,
    args: ArgMatches,
    options: RobotConnectionOptions,
) -> Result<(), CommandError> {
    let user_port: u16 = *args.get_one(USER_PORT).expect("user port should exist");
    let system_port: u16 = *args.get_one(SYSTEM_PORT).expect("system port should exist");
    let user_listener =
        TcpListener::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), user_port)).await?;
    let system_listener = TcpListener::bind(SocketAddr::new(
        IpAddr::V4(Ipv4Addr::LOCALHOST),
        system_port,
    ))
    .await?;
    let (brain, user) = v5_core::connection::connect_to_all(options).await?;

    let (tx, mut rx) = tokio::sync::mpsc::channel(16);

    tokio::task::spawn(user_loop(user_listener, user, tx.clone()));
    system_loop(system_listener, brain, tx.clone()).await;

    Ok(())
}

async fn user_loop(
    listener: TcpListener,
    mut connection: Box<dyn RobotConnection + Send>,
    active: Sender<u8>,
) {
    while let Ok((mut stream, _addr)) = listener.accept().await {
        let mut buf = [0_u8; 4096];
        let _ = connection.try_read(&mut buf).await;
        loop {
            let len = match connection.try_read(&mut buf).await {
                Ok(len) => len,
                Err(_) => break,
            };
            if len > 0 {
                match stream.writable().await {
                    Ok(_) => {}
                    Err(_) => break,
                }
                match stream.write_all(&buf[..len]).await {
                    Ok(_) => {}
                    Err(_) => break,
                };
            }
            match stream.readable().await {
                Ok(_) => {}
                Err(_) => break,
            }
            let len = match stream.try_read(&mut buf) {
                Ok(len) => len,
                Err(err) if err.kind() == WouldBlock => 0,
                Err(_) => break,
            };
            if len > 0 {
                match connection.write_all(&buf[..len]).await {
                    Ok(_) => {}
                    Err(_) => break,
                };
                match connection.flush().await {
                    Ok(_) => {}
                    Err(_) => break,
                };
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    }
}

async fn system_loop(
    listener: TcpListener,
    mut connection: Box<dyn RobotConnection + Send>,
    active: Sender<u8>,
) {
    while let Ok((mut stream, _addr)) = listener.accept().await {
        println!("new connection");
        let mut buf = [0_u8; 4096];
        let _ = connection.try_read(&mut buf).await;
        loop {
            let len = match connection.try_read(&mut buf).await {
                Ok(len) => len,
                Err(err) if err.kind() == WouldBlock => 0,
                Err(_) => break,
            };
            if len > 0 {
                match stream.writable().await {
                    Ok(_) => {}
                    Err(err) if err.kind() == WouldBlock => {},
                    Err(_) => break,
                }
                match stream.write_all(&buf[..len]).await {
                    Ok(_) => {}
                    Err(_) => break,
                };
            }
            let len = match stream.try_read(&mut buf) {
                Ok(0) => break,
                Ok(len) => len,
                Err(err) if err.kind() == WouldBlock => 0,
                Err(_) => break,
            };
            if len > 0 {
                match connection.write_all(&buf[..len]).await {
                    Ok(_) => {}
                    Err(_) => break,
                };
                match connection.flush().await {
                    Ok(_) => {}
                    Err(_) => break,
                };
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        println!("end connection");
    }
}
