use std::collections::HashMap;

use anyhow::Result;
use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use tokio::sync::mpsc;
use uuid::Uuid;

use landrop_protocol::Capabilities;

const SERVICE_TYPE: &str = "_lanfile._tcp.local.";
const PORT: u16 = 7878;

#[derive(Debug, Clone)]
pub struct MdnsPeer {
    pub device_id: Uuid,
    pub device_name: String,
    pub addr: std::net::IpAddr,
    pub port: u16,
}

pub enum MdnsEvent {
    Found(MdnsPeer),
    Removed(String),
}

pub struct MdnsDiscovery {
    daemon: ServiceDaemon,
    service_name: String,
}

impl MdnsDiscovery {
    pub fn new(device_id: Uuid, device_name: &str) -> Result<(Self, mpsc::UnboundedReceiver<MdnsEvent>)> {
        let daemon = ServiceDaemon::new()?;
        let instance_name = format!("landrop-{}", device_id);
        let service_name = format!("{}.{}", instance_name, SERVICE_TYPE);

        let mut properties = HashMap::new();
        properties.insert("device_id".to_string(), device_id.to_string());
        properties.insert("name".to_string(), device_name.to_string());
        properties.insert("version".to_string(), Capabilities::default().protocol_version.to_string());

        let my_service = ServiceInfo::new(
            SERVICE_TYPE,
            &instance_name,
            &format!("{}.local.", hostname::get()?.to_string_lossy()),
            (),
            PORT,
            Some(properties),
        )?;

        daemon.register(my_service)?;

        let receiver = daemon.browse(SERVICE_TYPE)?;
        let (tx, rx) = mpsc::unbounded_channel();
        let my_id = device_id;

        tokio::spawn(async move {
            loop {
                match receiver.recv_async().await {
                    Ok(event) => {
                        let evt = match event {
                            ServiceEvent::ServiceResolved(info) => {
                                let props = info.get_properties();
                                let id_str = props.get("device_id").map(|p| p.val_str()).unwrap_or("");
                                let id = match Uuid::parse_str(id_str) {
                                    Ok(id) if id != my_id => id,
                                    _ => continue,
                                };
                                let name = props.get("name").map(|p| p.val_str().to_string())
                                    .unwrap_or_else(|| info.get_hostname().to_string());
                                let addr = match info.get_addresses().iter().next() {
                                    Some(a) => *a,
                                    None => continue,
                                };
                                MdnsEvent::Found(MdnsPeer {
                                    device_id: id,
                                    device_name: name,
                                    addr,
                                    port: info.get_port(),
                                })
                            }
                            ServiceEvent::ServiceRemoved(_, fullname) => {
                                MdnsEvent::Removed(fullname)
                            }
                            _ => continue,
                        };
                        if tx.send(evt).is_err() { break; }
                    }
                    Err(_) => break,
                }
            }
        });

        Ok((Self { daemon, service_name }, rx))
    }

    pub fn shutdown(self) -> Result<()> {
        self.daemon.unregister(&self.service_name)?;
        self.daemon.shutdown()?;
        Ok(())
    }
}
