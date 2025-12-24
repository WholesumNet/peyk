use std::{
    io,
    time::Instant
};
use futures::{
    AsyncRead,
    AsyncReadExt,
    AsyncWrite,
    AsyncWriteExt
};
use async_trait::async_trait;
use libp2p::{
    StreamProtocol,
    request_response::Codec
};
use log::info;

#[derive(Debug, Clone)]
pub struct BlobCodec;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request(
    // param: blob hash
    pub String
); 

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response(
    // param: blob data
    pub Vec<u8>
); 

#[async_trait]
impl Codec for BlobCodec {
    type Protocol = StreamProtocol;
    type Request = Request;
    type Response = Response;

    async fn read_request<T>(
        &mut self, _: &Self::Protocol,
        io: &mut T
    ) -> io::Result<Self::Request>
    where
        T: AsyncRead + Unpin + Send,
    {
        // Keep it simple: Read request size (u64) then string bytes
        // For this demo, we assume a small fixed buffer for simplicity
        let mut buf = vec![0u8; 1024];
        let n = io.read(&mut buf).await?;

        Ok(
            Request(
                String::from_utf8_lossy(&buf[..n]).to_string()
            )
        )
    }

    async fn read_response<T>(
        &mut self,
        _: &Self::Protocol,
        io: &mut T
    ) -> io::Result<Self::Response>
    where
        T: AsyncRead + Unpin + Send,
    {
        info!("--> Starting download stream...");
        const ONE_MB: f64 = 1_048_576.0;
        
        let mut buffer = Vec::with_capacity(30 * (1<<20)); 
        
        // A temporary buffer for "chunks" off the wire
        let mut fragment = [0u8; 64 * 1024]; // 64KB reads
        let mut total_bytes = 0;
        let start_time = Instant::now();
        let mut last_log = Instant::now();

        loop {
            // This reads directly from the Yamux stream
            let n = io.read(&mut fragment).await?;
            
            if n == 0 {
                break; // EOF - Stream finished
            }
            
            buffer.extend_from_slice(&fragment[..n]);
            total_bytes += n;

            // Log progress every ~500ms
            if last_log.elapsed().as_millis() > 500 {
                let elapsed_total = start_time.elapsed().as_secs_f64();
                let mb_total = total_bytes as f64 / ONE_MB;
                let speed_mbps = (total_bytes as f64 * 8.0) / (elapsed_total * 1_000_000.0);
                
                info!(
                    "    Downloading blob: {:.2} MB received | Avg Speed: {:.2} Mbps",
                    mb_total, speed_mbps
                );
                last_log = Instant::now();
            }
        }

        let total_time = start_time.elapsed();
        info!(
            "--> Download is finished! {:.2}MB in {:.2}s",
            total_bytes as f64 / ONE_MB, 
            total_time.as_secs_f64(),
        );
        
        Ok(Response(buffer))
    }

    async fn write_response<T>(
        &mut self,
        _: &Self::Protocol,
        io: &mut T,
        res: Self::Response
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        // Write the entire buffer. 
        // Yamux handles breaking this into frames internally.
        io.write_all(&res.0).await?;
        io.close().await?; // Close is essential to trigger EOF on receiver
        Ok(())
    }

    // write_request implementation omitted for brevity (similar to write_response)
    async fn write_request<T>(
        &mut self,
        _: &Self::Protocol,
        io: &mut T,
        req: Self::Request
    ) -> io::Result<()>
    where T: AsyncWrite + Unpin + Send {
        io.write_all(req.0.as_bytes()).await?;
        io.close().await?;
        Ok(())
    }
}