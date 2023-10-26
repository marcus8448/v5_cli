use std::io::ErrorKind::WouldBlock;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use clap::{Arg, ArgMatches, Command, value_parser};
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;

use v5_core::connection::{RobotConnectionOptions, SerialConnection};
use v5_core::error::CommandError;

type Result<T> = std::result::Result<T, CommandError>;

pub(crate) const COMMAND: &str = "daemon";
const USER_PORT: &str = "user";
const SYSTEM_PORT: &str = "system";

pub(crate) fn command() -> Command {
    Command::new(COMMAND)
        .about("Simulate a competition")
        .arg(
            Arg::new(SYSTEM_PORT)
                .default_value("5735")
                .value_parser(value_parser!(u16))
                .short('s')
        )
        .arg(
            Arg::new(USER_PORT)
                .default_value("5736")
                .value_parser(value_parser!(u16))
                .short('u')
        )
}

pub(crate) async fn daemon(args: ArgMatches, options: RobotConnectionOptions) -> Result<()> {
    let user_port: u16 = *args.get_one(USER_PORT).unwrap();
    let system_port: u16 = *args.get_one(SYSTEM_PORT).unwrap();
    let user_listener = tokio::net::TcpListener::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), user_port)).await?;
    let system_listener = tokio::net::TcpListener::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), system_port)).await?;
    let (user, brain) = v5_core::connection::connect_to_all(options).await?;
    let active = Arc::new(AtomicBool::new(false));
    let handle = tokio::task::spawn(user_loop(user_listener, user, Arc::clone(&active)));
    let handle2 = tokio::task::spawn(system_loop(system_listener, brain, active));
    handle.await.unwrap();
    handle2.await.unwrap();
    Ok(())
}

async fn user_loop(listener: TcpListener, mut connection: Box<dyn SerialConnection + Send>, active: Arc<AtomicBool>) {
    while let Ok((mut stream, _addr)) = listener.accept().await {
        active.store(true, Ordering::Relaxed);
        let mut buf = [0_u8; 2048];
        let _ = connection.read_to_end(&mut Vec::new()).await;
        loop {
            let len = match connection.try_read(&mut buf).await {
                Ok(len) => len,
                Err(_) => break
            };
            if len > 0 {
                stream.writable().await.unwrap();
                match stream.write_all(&buf[..len]).await {
                    Ok(_) => {},
                    Err(_) => break
                };
            }
            let len = match stream.try_read(&mut buf) {
                Ok(len) => len,
                Err(err) if err.kind() == WouldBlock => 0,
                Err(_) => break
            };
            if len > 0 {
                match connection.write_all(&buf[..len]).await {
                    Ok(_) => {}
                    Err(_) => break
                };
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        active.store(false, Ordering::Relaxed);
    }
}

async fn system_loop(listener: TcpListener, mut connection: Box<dyn SerialConnection + Send>, _active: Arc<AtomicBool>) {
    while let Ok((mut stream, _addr)) = listener.accept().await {
        let mut buf = [0_u8; 2048];
        let _ = connection.read_to_end(&mut Vec::new()).await;
        loop {
            let len = match connection.try_read(&mut buf).await {
                Ok(len) => len,
                Err(_) => break
            };
            if len > 0 {
                stream.writable().await.unwrap();
                match stream.write_all(&buf[..len]).await {
                    Ok(_) => {},
                    Err(_) => break
                };
            }
            let len = match stream.try_read(&mut buf) {
                Ok(len) => len,
                Err(err) if err.kind() == WouldBlock => 0,
                Err(_) => break
            };
            if len > 0 {
                match connection.write_all(&buf[..len]).await {
                    Ok(_) => {}
                    Err(_) => break
                };
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        connection.write_all(&[0_u8; 1024]).await.unwrap();
    }
}
