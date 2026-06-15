use std::net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use socket2::{Domain, Protocol, Socket, Type};
use tokio::net::UdpSocket;
use tokio::sync::mpsc;
use uuid::Uuid;

const BROADCAST_PORT: u16 = 7777;
const BROADCAST_ADDR: Ipv4Addr = Ipv4Addr::new(255, 255, 255, 255);
const ANNOUNCE_INTERVAL: Duration = Duration::from_secs(2);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BroadcastMessage {
    Discover {
        device_id: Uuid,
        device_name: String,
        tcp_port: u16,
    },
    Here {
        device_id: Uuid,
        device_name: String,
        tcp_port: u16,
        version: u16,
    },
}

#[derive(Debug, Clone)]
pub struct BroadcastPeer {
    pub device_id: Uuid,
    pub device_name: String,
    pub addr: IpAddr,
    pub tcp_port: u16,
}

#[allow(dead_code)]
pub struct BroadcastDiscovery {
    socket: Arc<UdpSocket>,
    device_id: Uuid,
    device_name: String,
    tcp_port: u16,
}

impl BroadcastDiscovery {
    pub async fn new(
        device_id: Uuid,
        device_name: String,
        tcp_port: u16,
    ) -> Result<(Self, mpsc::UnboundedReceiver<BroadcastPeer>)> {
        let std_sock = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
        std_sock.set_reuse_address(true)?;
        #[cfg(target_os = "macos")]
        std_sock.set_reuse_port(true)?;
        std_sock.set_broadcast(true)?;
        std_sock.set_nonblocking(true)?;
        std_sock.bind(&SocketAddr::from((Ipv4Addr::UNSPECIFIED, BROADCAST_PORT)).into())?;

        let socket = Arc::new(UdpSocket::from_std(std_sock.into())?);
        let (tx, rx) = mpsc::unbounded_channel();

        let recv_socket = socket.clone();
        let my_id = device_id;
        let my_name = device_name.clone();
        let my_port = tcp_port;
        let tx_clone = tx.clone();
        let send_socket = socket.clone();

        // Listener task
        tokio::spawn(async move {
            let mut buf = vec![0u8; 4096];
            loop {
                let Ok((len, src)) = recv_socket.recv_from(&mut buf).await else { break };
                let Ok(msg) = rmp_serde::from_slice::<BroadcastMessage>(&buf[..len]) else { continue };

                match msg {
                    BroadcastMessage::Discover { device_id, device_name, tcp_port }
                        if device_id != my_id =>
                    {
                        // Register the sender directly — don't rely on our reply
                        // making it back (e.g. the sender's firewall may accept
                        // its own outbound broadcasts but drop our unsolicited
                        // unicast reply).
                        let peer = BroadcastPeer {
                            device_id,
                            device_name,
                            addr: src.ip(),
                            tcp_port,
                        };
                        let _ = tx_clone.send(peer);

                        let here = BroadcastMessage::Here {
                            device_id: my_id,
                            device_name: my_name.clone(),
                            tcp_port: my_port,
                            version: 1,
                        };
                        if let Ok(data) = rmp_serde::to_vec(&here) {
                            let _ = send_socket.send_to(&data, src).await;
                        }
                    }
                    BroadcastMessage::Here { device_id, device_name, tcp_port, .. }
                        if device_id != my_id =>
                    {
                        let peer = BroadcastPeer {
                            device_id,
                            device_name,
                            addr: src.ip(),
                            tcp_port,
                        };
                        let _ = tx_clone.send(peer);
                    }
                    _ => {}
                }
            }
        });

        // Periodic DISCOVER announcer
        let probe_socket = socket.clone();
        let announce_id = device_id;
        let announce_name = device_name.clone();
        let announce_port = tcp_port;
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(ANNOUNCE_INTERVAL);
            loop {
                interval.tick().await;
                let msg = BroadcastMessage::Discover {
                    device_id: announce_id,
                    device_name: announce_name.clone(),
                    tcp_port: announce_port,
                };
                if let Ok(data) = rmp_serde::to_vec(&msg) {
                    let target = SocketAddr::V4(SocketAddrV4::new(BROADCAST_ADDR, BROADCAST_PORT));
                    let _ = probe_socket.send_to(&data, target).await;
                }
            }
        });

        Ok((Self { socket, device_id, device_name, tcp_port }, rx))
    }
}
