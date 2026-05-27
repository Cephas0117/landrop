// Loopback test harness for integration tests
// Provides a pair of connected TLS streams for testing sender/receiver

use anyhow::Result;
use landrop_security::DeviceIdentity;
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::{TlsAcceptor, TlsConnector};

pub async fn make_tls_pair() -> Result<(
    tokio_rustls::server::TlsStream<TcpStream>,
    tokio_rustls::client::TlsStream<TcpStream>,
)> {
    let server_id = DeviceIdentity::generate()?;
    let client_id = DeviceIdentity::generate()?;

    let server_config = server_id.server_config()?;
    let acceptor = TlsAcceptor::from(server_config);

    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;

    let accept_handle = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        acceptor.accept(stream).await.unwrap()
    });

    let trust_store = Arc::new(parking_lot::RwLock::new(
        landrop_security::TrustStore::default(),
    ));
    let verifier = landrop_security::verifier::TofuVerifier::new(trust_store);
    let client_config = client_id.client_config(verifier)?;
    let connector = TlsConnector::from(client_config);

    let client_stream = TcpStream::connect(addr).await?;
    let server_name = rustls::pki_types::ServerName::try_from("localhost")
        .unwrap()
        .to_owned();
    let client_tls = connector.connect(server_name, client_stream).await?;
    let server_tls = accept_handle.await?;

    Ok((server_tls, client_tls))
}

use std::sync::Arc;
