pub mod engine;
pub mod ewma;
pub mod receiver;
pub mod sender;

pub use engine::{
    PairingEvent, TransferDirection, TransferEngine, TransferEvent, TransferEventKind,
};

use anyhow::{Context, Result};
use futures::{SinkExt, StreamExt};
use landrop_protocol::{codec::FrameCodec, WireMessage};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_util::codec::Framed;

const IO_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

async fn read_message<T: AsyncRead + AsyncWrite + Unpin>(
    framed: &mut Framed<T, FrameCodec>,
    expected: &str,
) -> Result<WireMessage> {
    tokio::time::timeout(IO_TIMEOUT, framed.next())
        .await
        .with_context(|| format!("等待 {expected} 超时，请检查对方设备和网络"))?
        .with_context(|| format!("连接在收到 {expected} 前关闭"))?
        .map_err(Into::into)
}

async fn send_message<T: AsyncRead + AsyncWrite + Unpin>(
    framed: &mut Framed<T, FrameCodec>,
    message: WireMessage,
) -> Result<()> {
    tokio::time::timeout(IO_TIMEOUT, framed.send(message))
        .await
        .context("发送超时，请检查对方设备和网络")??;
    Ok(())
}
