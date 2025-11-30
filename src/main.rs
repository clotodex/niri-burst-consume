use anyhow::{Context, Result, anyhow};
use niri_ipc::{Action, Event, Reply, Request};
use std::env;
use std::os::unix::net::UnixStream as StdUnixStream;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

const THRESHOLD_MS: u128 = 500;

fn get_time_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()
}

async fn connect() -> Result<UnixStream> {
    let socket_path = env::var_os("NIRI_SOCKET")
        .or_else(|| env::var_os("NIRI_SOCKET_PATH"))
        .ok_or_else(|| anyhow!("NIRI_SOCKET or NIRI_SOCKET_PATH environment variable not set"))?;
    let std_stream = StdUnixStream::connect(socket_path)?;
    std_stream.set_nonblocking(true)?;
    UnixStream::from_std(std_stream).context("Failed to convert stream")
}

async fn send_action(id: u64) -> Result<()> {
    let mut stream = connect().await?;

    let action = Action::ConsumeOrExpelWindowLeft { id: Some(id) };
    let request = Request::Action(action);
    let request_json = serde_json::to_string(&request)? + "\n";

    stream.write_all(request_json.as_bytes()).await?;
    stream.flush().await?;
    println!("Sent action for window {}", id);

    let mut reader = BufReader::new(stream);
    let mut response_line = String::new();
    reader.read_line(&mut response_line).await?;

    let reply: Reply = serde_json::from_str(&response_line)?;
    reply.map_err(|e| anyhow!("Niri error: {}", e)).map(|_| ())
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut stream = connect().await?;

    // Send the EventStream request
    let request_json = serde_json::to_string(&Request::EventStream)? + "\n";
    stream.write_all(request_json.as_bytes()).await?;
    stream.flush().await?;

    // Create BufReader for the lifetime of this connection
    let mut reader = BufReader::new(stream);

    // Read the Handled response
    let mut line = String::new();
    reader.read_line(&mut line).await?;
    let reply: Reply = serde_json::from_str(&line).context("Failed to parse handshake")?;
    if let Err(e) = reply {
        return Err(anyhow!("Niri refused EventStream: {}", e));
    }

    // State for iterator-like processing
    let mut last_id: Option<u64> = None;
    let mut last_timestamp: Option<u128> = None;
    let mut cluster: Vec<u64> = Vec::new();
    let mut cluster_ids: std::collections::HashSet<u64> = std::collections::HashSet::new();

    loop {
        line.clear();
        let bytes_read = reader.read_line(&mut line).await?;
        if bytes_read == 0 {
            break; // EOF
        }

        let event: Event = serde_json::from_str(&line).context("Failed to parse Niri event")?;

        // Filter and map to window events
        if let Event::WindowOpenedOrChanged { window } = event {
            let id = window.id;
            let timestamp = get_time_ms();

            // Deduplicate consecutive duplicates
            if last_id == Some(id) {
                continue;
            }

            last_id = Some(id);

            // Calculate time difference
            let time_diff = last_timestamp.map(|last| timestamp - last);
            last_timestamp = Some(timestamp);

            // Cluster logic
            match time_diff {
                Some(diff) if diff <= THRESHOLD_MS => {
                    // Within threshold, add to cluster if not already present
                    if cluster_ids.insert(id) {
                        cluster.push(id);
                        println!("Window {} opened within {}ms of previous window", id, diff);
                        println!("Current cluster: {:?}", cluster);

                        // Consume this window into the column with the previous window
                        if let Err(e) = send_action(id).await {
                            eprintln!("Failed to send consume action for window {}: {}", id, e);
                        } else {
                            println!("Sent consume-left action for window {}", id);
                        }
                    }
                }
                _ => {
                    // Outside threshold or first window
                    if cluster.len() > 1 {
                        println!(
                            "=== Cluster completed with {} windows: {:?} ===",
                            cluster.len(),
                            cluster
                        );
                    }

                    cluster = vec![id];
                    cluster_ids.clear();
                    cluster_ids.insert(id);

                    if let Some(diff) = time_diff {
                        println!("New window: {} ({}ms gap)", id, diff);
                    } else {
                        println!("New window: {}", id);
                    }
                }
            }
        }
    }

    Ok(())
}
