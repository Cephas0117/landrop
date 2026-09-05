use std::time::Duration;

use futures::{SinkExt, StreamExt};
use landrop_fs::ManifestBuilder;
use landrop_protocol::{codec::FrameCodec, Chunk, FileAck, FileDecision, ManifestAck, WireMessage};
use landrop_testkit::{fixtures::TempFiles, loopback::make_tls_pair};
use landrop_transfer::{receiver::Receiver, sender::Sender};
use tokio::sync::mpsc;
use tokio_util::codec::Framed;
use uuid::Uuid;

#[tokio::test]
async fn tls_connects_without_process_global_provider_setup() {
    tokio::time::timeout(Duration::from_secs(10), make_tls_pair())
        .await
        .unwrap()
        .unwrap();
}

async fn transfer_files(sizes: &[usize], nested: bool) {
    let source = TempFiles::new().unwrap();
    let destination = tempfile::tempdir().unwrap();
    if nested {
        std::fs::create_dir_all(source.dir.path().join("资料/子目录")).unwrap();
    }
    let paths: Vec<_> = sizes
        .iter()
        .enumerate()
        .map(|(i, size)| {
            let name = if nested {
                format!("资料/子目录/{i}.bin")
            } else {
                format!("{i}.bin")
            };
            source.create_file(&name, *size).unwrap()
        })
        .collect();
    let send_paths = if nested {
        vec![source.dir.path().join("资料")]
    } else {
        paths.clone()
    };
    let (server, client) = make_tls_pair().await.unwrap();
    let (send_progress, mut send_rx) = mpsc::channel(256);
    let (recv_progress, _recv_rx) = mpsc::channel(256);
    let (_cancel_tx, cancel_rx) = mpsc::channel(1);
    let (sent, received) = tokio::time::timeout(Duration::from_secs(30), async {
        tokio::join!(
            Sender::run(client, send_paths, Uuid::new_v4(), send_progress, cancel_rx),
            Receiver::run(
                server,
                destination.path().into(),
                Uuid::new_v4(),
                recv_progress
            ),
        )
    })
    .await
    .expect("transfer must not stall");
    sent.unwrap();
    received.unwrap();
    for path in paths {
        let target = destination
            .path()
            .join(path.strip_prefix(source.dir.path()).unwrap());
        assert_eq!(std::fs::read(path).unwrap(), std::fs::read(target).unwrap());
    }
    let mut final_progress = None;
    while let Some(progress) = send_rx.recv().await {
        final_progress = Some(progress);
    }
    let progress = final_progress.expect("even empty files must report completion progress");
    assert_eq!(progress.bytes_sent, sizes.iter().sum::<usize>() as u64);
    assert_eq!(progress.files_done, sizes.len() as u32);
}

#[tokio::test]
async fn small_file() {
    transfer_files(&[321], false).await;
}

#[tokio::test]
async fn empty_file() {
    transfer_files(&[0], false).await;
}

#[tokio::test]
async fn exact_ack_boundary() {
    transfer_files(&[1024 * 1024], false).await;
}

#[tokio::test]
async fn multiple_files_and_nested_directories() {
    transfer_files(&[0, 1024 * 1024, 7, 8 * 1024 * 1024 + 19], true).await;
}

#[tokio::test]
async fn sender_requires_done_ack_after_empty_file_eof_ack() {
    let source = TempFiles::new().unwrap();
    let path = source.create_file("empty.txt", 0).unwrap();
    let (server, client) = make_tls_pair().await.unwrap();
    let (progress_tx, _progress_rx) = mpsc::channel(32);
    let (_cancel_tx, cancel_rx) = mpsc::channel(1);
    let server_task = async move {
        let mut framed = Framed::new(server, FrameCodec);
        let WireMessage::Manifest(manifest) = framed.next().await.unwrap().unwrap() else {
            panic!("manifest");
        };
        framed
            .send(WireMessage::ManifestAck(ManifestAck {
                session_id: manifest.session_id,
                files: manifest
                    .files
                    .iter()
                    .map(|f| FileAck {
                        file_id: f.file_id,
                        decision: FileDecision::Send,
                    })
                    .collect(),
            }))
            .await
            .unwrap();
        let WireMessage::Chunk(eof) = framed.next().await.unwrap().unwrap() else {
            panic!("EOF");
        };
        assert!(eof.eof);
        framed
            .send(WireMessage::ChunkAck(landrop_protocol::ChunkAck {
                session_id: eof.session_id,
                file_id: eof.file_id,
                seq: eof.seq,
                committed_bytes: 0,
            }))
            .await
            .unwrap();
        assert!(matches!(
            framed.next().await,
            Some(Ok(WireMessage::Done(_)))
        ));
        // Simulate the receiver failing before it can confirm completion.
    };
    let (result, ()) = tokio::time::timeout(Duration::from_secs(10), async {
        tokio::join!(
            Sender::run(client, vec![path], Uuid::new_v4(), progress_tx, cancel_rx),
            server_task
        )
    })
    .await
    .unwrap();
    assert!(
        result.is_err(),
        "sender must not report success without DoneAck"
    );
}

#[tokio::test]
async fn receiver_rejects_disconnect_before_done() {
    let source = TempFiles::new().unwrap();
    let path = source.create_file("partial.bin", 1024).unwrap();
    let manifest = ManifestBuilder::build(&[path]).unwrap();
    let destination = tempfile::tempdir().unwrap();
    let (server, client) = make_tls_pair().await.unwrap();
    let (progress_tx, _progress_rx) = mpsc::channel(32);
    let sender = async move {
        let mut framed = Framed::new(client, FrameCodec);
        framed
            .send(WireMessage::Manifest(manifest.clone()))
            .await
            .unwrap();
        assert!(matches!(
            framed.next().await,
            Some(Ok(WireMessage::ManifestAck(_)))
        ));
        framed
            .send(WireMessage::Chunk(Chunk {
                session_id: manifest.session_id,
                file_id: manifest.files[0].file_id,
                seq: 0,
                offset: 0,
                data: vec![1; 10],
                eof: false,
            }))
            .await
            .unwrap();
        framed.close().await.unwrap();
    };
    let (result, ()) = tokio::time::timeout(Duration::from_secs(10), async {
        tokio::join!(
            Receiver::run(
                server,
                destination.path().into(),
                Uuid::new_v4(),
                progress_tx
            ),
            sender
        )
    })
    .await
    .unwrap();
    assert!(
        result.is_err(),
        "receiver must not report an interrupted transfer as completed"
    );
}

#[tokio::test]
async fn pairing_then_restart_and_engine_transfer_delivers_files_and_ordered_events() {
    use landrop_security::{DeviceIdentity, TrustStore};
    use landrop_transfer::{PairingEvent, TransferEngine, TransferEventKind};
    use parking_lot::RwLock;
    use std::sync::Arc;

    let source = TempFiles::new().unwrap();
    let destination = tempfile::tempdir().unwrap();
    let config = tempfile::tempdir().unwrap();
    let path = source.create_file("payload.bin", 1024 * 1024).unwrap();
    let alice = Arc::new(
        DeviceIdentity::load_or_create_at(&config.path().join("alice-identity.json")).unwrap(),
    );
    let bob = Arc::new(
        DeviceIdentity::load_or_create_at(&config.path().join("bob-identity.json")).unwrap(),
    );
    let alice_trust = Arc::new(RwLock::new(
        TrustStore::load_from(config.path().join("alice.json")).unwrap(),
    ));
    let bob_trust = Arc::new(RwLock::new(
        TrustStore::load_from(config.path().join("bob.json")).unwrap(),
    ));
    let (sender, _send_events, mut send_pairing) =
        TransferEngine::new(alice.clone(), alice_trust.clone(), source.dir.path().into());
    let (receiver, _recv_events, mut recv_pairing) =
        TransferEngine::new(bob.clone(), bob_trust.clone(), destination.path().into());
    let port = receiver
        .listen_on(([127, 0, 0, 1], 0).into())
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(15), async {
        let session_id = sender
            .initiate_pairing(
                ([127, 0, 0, 1], port).into(),
                alice.device_id,
                "123456".into(),
                alice.fingerprint.0.clone(),
            )
            .await
            .unwrap();
        match recv_pairing.recv().await.unwrap() {
            PairingEvent::IncomingRequest {
                session_id: incoming,
                peer_id,
                pin,
                ..
            } => {
                assert_eq!(incoming, session_id);
                assert_eq!(peer_id, alice.device_id);
                assert_eq!(pin, "123456");
                receiver.resolve_pairing(incoming, true);
            }
            _ => panic!("expected pairing request"),
        }
        assert!(matches!(
            send_pairing.recv().await,
            Some(PairingEvent::OutgoingResolved { accepted: true, .. })
        ));
        assert!(alice_trust
            .read()
            .is_trusted(bob.device_id, &bob.fingerprint.0));
        assert!(bob_trust
            .read()
            .is_trusted(alice.device_id, &alice.fingerprint.0));
        // Reconstruct both services from saved identity and trust, as on restart.
        let (sender, mut send_events, _) = TransferEngine::new(
            Arc::new(
                DeviceIdentity::load_or_create_at(&config.path().join("alice-identity.json"))
                    .unwrap(),
            ),
            Arc::new(RwLock::new(
                TrustStore::load_from(config.path().join("alice.json")).unwrap(),
            )),
            source.dir.path().into(),
        );
        let (receiver, mut recv_events, _) = TransferEngine::new(
            Arc::new(
                DeviceIdentity::load_or_create_at(&config.path().join("bob-identity.json"))
                    .unwrap(),
            ),
            Arc::new(RwLock::new(
                TrustStore::load_from(config.path().join("bob.json")).unwrap(),
            )),
            destination.path().into(),
        );
        let port = receiver
            .listen_on(([127, 0, 0, 1], 0).into())
            .await
            .unwrap();
        let id = sender
            .send(
                ([127, 0, 0, 1], port).into(),
                bob.device_id,
                "Bob".into(),
                vec![path.clone()],
            )
            .await
            .unwrap();
        let first = send_events.recv().await.unwrap();
        assert_eq!(first.transfer_id, id);
        assert!(matches!(
            first.event,
            TransferEventKind::Queued {
                total_bytes: 1048576,
                files_total: 1,
                ..
            }
        ));
        for events in [&mut send_events, &mut recv_events] {
            let mut last_progress = None;
            loop {
                match events.recv().await.unwrap().event {
                    TransferEventKind::Progress(progress) => last_progress = Some(progress),
                    TransferEventKind::Completed => break,
                    TransferEventKind::Failed(e) => panic!("engine failed: {e}"),
                    TransferEventKind::Queued { .. } => {}
                }
            }
            let progress = last_progress.expect("final progress precedes Completed");
            assert_eq!(progress.files_done, 1);
            assert_eq!(progress.bytes_sent, 1048576);
            assert!(
                events.try_recv().is_err(),
                "no delayed progress after Completed"
            );
        }
    })
    .await
    .unwrap();
    assert_eq!(
        std::fs::read(path).unwrap(),
        std::fs::read(destination.path().join("payload.bin")).unwrap()
    );
}

#[tokio::test]
async fn cancel_interrupts_wait_for_manifest_ack() {
    let source = TempFiles::new().unwrap();
    let path = source.create_file("cancel.bin", 100).unwrap();
    let (server, client) = make_tls_pair().await.unwrap();
    let (progress, _progress_rx) = mpsc::channel(32);
    let (cancel, cancel_rx) = mpsc::channel(1);
    let send = tokio::spawn(Sender::run(
        client,
        vec![path],
        Uuid::new_v4(),
        progress,
        cancel_rx,
    ));
    let mut framed = Framed::new(server, FrameCodec);
    assert!(matches!(
        framed.next().await,
        Some(Ok(WireMessage::Manifest(_)))
    ));
    cancel.send(()).await.unwrap();
    let result = tokio::time::timeout(Duration::from_secs(2), send)
        .await
        .unwrap()
        .unwrap();
    assert!(result.unwrap_err().to_string().contains("canceled"));
}

#[tokio::test]
async fn receiver_rejects_truncated_file_even_with_eof() {
    let source = TempFiles::new().unwrap();
    let path = source.create_file("truncated.bin", 100).unwrap();
    let manifest = ManifestBuilder::build(&[path]).unwrap();
    let destination = tempfile::tempdir().unwrap();
    let (server, client) = make_tls_pair().await.unwrap();
    let (progress, _progress_rx) = mpsc::channel(32);
    let recv_dir = destination.path().to_path_buf();
    let recv = tokio::spawn(Receiver::run(server, recv_dir, Uuid::new_v4(), progress));
    let mut framed = Framed::new(client, FrameCodec);
    framed
        .send(WireMessage::Manifest(manifest.clone()))
        .await
        .unwrap();
    assert!(matches!(
        framed.next().await,
        Some(Ok(WireMessage::ManifestAck(_)))
    ));
    framed
        .send(WireMessage::Chunk(Chunk {
            session_id: manifest.session_id,
            file_id: manifest.files[0].file_id,
            seq: 0,
            offset: 0,
            data: vec![],
            eof: true,
        }))
        .await
        .unwrap();
    let result = tokio::time::timeout(Duration::from_secs(2), recv)
        .await
        .unwrap()
        .unwrap();
    assert!(result.unwrap_err().to_string().contains("incomplete file"));
}

#[tokio::test]
async fn same_name_files_are_received_without_skipping_or_corrupting_existing_data() {
    for existing_size in [10, 100, 200] {
        let source = TempFiles::new().unwrap();
        let path = source.create_file("duplicate.bin", 100).unwrap();
        let destination = tempfile::tempdir().unwrap();
        let existing = vec![42; existing_size];
        std::fs::write(destination.path().join("duplicate.bin"), &existing).unwrap();
        let (server, client) = make_tls_pair().await.unwrap();
        let (send_progress, _send_rx) = mpsc::channel(32);
        let (recv_progress, _recv_rx) = mpsc::channel(32);
        let (_cancel, cancel_rx) = mpsc::channel(1);
        let (sent, received) = tokio::time::timeout(Duration::from_secs(10), async {
            tokio::join!(
                Sender::run(
                    client,
                    vec![path.clone()],
                    Uuid::new_v4(),
                    send_progress,
                    cancel_rx
                ),
                Receiver::run(
                    server,
                    destination.path().into(),
                    Uuid::new_v4(),
                    recv_progress
                ),
            )
        })
        .await
        .unwrap();
        sent.unwrap();
        received.unwrap();
        assert_eq!(
            std::fs::read(destination.path().join("duplicate.bin")).unwrap(),
            existing
        );
        assert_eq!(
            std::fs::read(destination.path().join("duplicate (1).bin")).unwrap(),
            std::fs::read(path).unwrap()
        );
    }
}
