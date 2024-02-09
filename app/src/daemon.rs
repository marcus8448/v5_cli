use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;

use clap::{Arg, ArgMatches, Command, value_parser};
use log::{info, warn};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Mutex, MutexGuard, Notify};
use tokio::sync::mpsc::Receiver;
use tokio::sync::mpsc::Sender;

use v5_serial::connection::{RobotConnection, RobotConnectionOptions};
use v5_serial::connection::daemon::DaemonCommand;
use v5_serial::error::{CommandError, ConnectionError};

pub(crate) const COMMAND: &str = "daemon";
const DAEMON_PORT: &str = "daemon-port";

pub(crate) fn command() -> Command {
    Command::new(COMMAND)
        .about("Share connection to robot")
        .arg(
            Arg::new(DAEMON_PORT)
                .default_value("5735")
                .value_parser(value_parser!(u16))
                .index(1),
        )
}

pub(crate) async fn daemon(
    _cmd: &mut Command,
    args: ArgMatches,
    options: RobotConnectionOptions,
) -> Result<(), CommandError> {
    let system_listener = TcpListener::bind(SocketAddr::new(
        IpAddr::V4(Ipv4Addr::LOCALHOST),
        *args.get_one(DAEMON_PORT).expect("port should exist"),
    ))
    .await?;

    let mut brain = v5_serial::connection::connect_to_brain(options).await?;
    let (tx, mut system_rx) = tokio::sync::mpsc::channel(1024);
    let (system_tx, rx) = tokio::sync::mpsc::channel(1024);

    let system_handle = Arc::new(Mutex::new((tx, rx)));

    let (tx, mut user_rx) = tokio::sync::mpsc::channel(1024);
    let (user_tx, _) = tokio::sync::broadcast::channel(1024);
    let sender_ = user_tx.clone();

    let user_handle = Arc::new(Mutex::new(tx));
    let packet_size = brain.get_max_packet_size();

    let error_handle = Arc::new(Notify::new());
    let eh3 = Arc::clone(&error_handle);

    tokio::task::spawn(async move {
        loop {
            let (mut stream, address) = system_listener.accept().await.unwrap();
            info!("new connection from {:?}", address);
            stream.write_u16(packet_size).await.unwrap();
            tokio::task::spawn(connection_handler(
                stream,
                Arc::clone(&system_handle),
                Arc::clone(&user_handle),
                sender_.subscribe(),
                Arc::clone(&eh3),
            ));
        }
    });

    let mut buf = [0_u8; 1024];
    loop {
        tokio::select! {
            t = system_rx.recv() => {
                system_tx.send(brain.connection.send_packet(&t.unwrap()).await.unwrap().consume()).await.unwrap()
            }
            t = user_rx.recv() => {
                brain.connection.write_serial(&t.unwrap()).await.unwrap();
            }
            t = brain.connection.read_serial(&mut buf) => {
                if t.is_ok() {
                    let l = t.unwrap();
                    if l > 0 {
                        user_tx.send(buf[..l].to_vec().into_boxed_slice()).unwrap();
                    }
                }
            }
            _ = error_handle.notified() => {
                warn!("Attempting to reset connection");
                brain.connection.reset().await.unwrap();
            }
        }
    }
    Ok(())
}

async fn connection_handler(
    mut stream: TcpStream,
    system_handle: Arc<Mutex<(Sender<Box<[u8]>>, Receiver<Box<[u8]>>)>>,
    user_handle: Arc<Mutex<Sender<Box<[u8]>>>>,
    mut user_rx: tokio::sync::broadcast::Receiver<Box<[u8]>>,
    arc: Arc<Notify>,
) -> Result<(), ConnectionError> {
    let mut exclusive: Option<MutexGuard<(Sender<Box<[u8]>>, Receiver<Box<[u8]>>)>> = None;
    loop {
        tokio::select! {
            t = stream.read_u8() => {
                if t.is_ok() {
                    let t = t.unwrap();
                    match DaemonCommand::try_from(t).unwrap() {
                        DaemonCommand::SendSystem => {
                            let len = stream.read_u16().await?;
                            let mut buf = vec![0_u8; len as usize].into_boxed_slice();
                            stream.read_exact(&mut buf).await?;

                            let mut guard;
                            let keep = exclusive.is_some();
                            if let Some(t)  = exclusive.take() {
                                guard = t;
                            } else {
                                guard = system_handle.lock().await;
                            }

                            guard.0.send(buf).await.unwrap();

                            let response = guard.1.recv().await.unwrap();

                            stream.write_u16(response.len() as u16).await?;
                            stream.write_all(&response).await?;
                            stream.flush().await?;
                            if keep {
                                exclusive = Some(guard);
                            }
                        }
                        DaemonCommand::SendUser => {
                            let len = stream.read_u16().await?;
                            let mut buf = vec![0_u8; len as usize].into_boxed_slice();
                            stream.read_exact(&mut buf).await?;
                            user_handle.lock().await.send(buf).await.unwrap();
                        }
                        DaemonCommand::ClaimExclusive => {
                            assert!(exclusive.is_none());
                            exclusive = Some(system_handle.lock().await);
                        }
                        DaemonCommand::UnclaimExclusive => {
                            assert!(exclusive.is_some());
                            exclusive = None;
                        },
                        DaemonCommand::Reset => {
                            arc.notify_one();
                        }
                    }
                } else {
                    if exclusive.is_some() {
                        let _guard = exclusive.unwrap();
                        arc.notify_one();
                    }
                    return Ok(());
                }
            }
            b = user_rx.recv() => {
                let buf = b.unwrap();
                stream.write_u8(DaemonCommand::SendUser as u8).await?;
                stream.write_u16(buf.len() as u16).await?;
                stream.write_all(&buf).await?;
            }
        }
    }
}
