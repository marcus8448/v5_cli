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
    let active = Arc::new(AtomicBool::new(false));
    tokio::select! {
        v = tokio::task::spawn(user_loop(user_listener, user, Arc::clone(&active))) => v.unwrap(),
        v = tokio::task::spawn(system_loop(system_listener, brain, active)) => v.unwrap(),
    }
    Ok(())
}

async fn user_loop(
    listener: TcpListener,
    mut connection: Box<dyn SerialConnection + Send>,
    active: Arc<AtomicBool>,
) {
    while let Ok((mut stream, _addr)) = listener.accept().await {
        active.store(true, Ordering::Relaxed);
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
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        active.store(false, Ordering::Relaxed);
    }
}

async fn system_loop(
    listener: TcpListener,
    mut connection: Box<dyn SerialConnection + Send>,
    active: Arc<AtomicBool>,
) {
    while let Ok((mut stream, _addr)) = listener.accept().await {
        println!("new conneciton");
        active.store(true, Ordering::Relaxed);
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
        print!("end connection");
        active.store(false, Ordering::Relaxed);
    }
}
