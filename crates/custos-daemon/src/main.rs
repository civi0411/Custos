use custos_daemon::{ApiRequest, ApiResponse, CustosRuntime};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let database_path = std::env::var("CUSTOS_DB_PATH").unwrap_or_else(|_| "custos.db".to_string());
    let runtime = CustosRuntime::bootstrap(&database_path)?;
    eprintln!("Custos daemon ready; database={database_path}; transport=stdio-jsonl");

    let stdin = BufReader::new(tokio::io::stdin());
    let mut lines = stdin.lines();
    let mut stdout = tokio::io::stdout();

    while let Some(line) = lines.next_line().await? {
        if line.trim().is_empty() {
            continue;
        }

        let response = match serde_json::from_str::<ApiRequest>(&line) {
            Ok(request) => runtime.local_api.handle_request(request).await,
            Err(error) => ApiResponse::error("", format!("Invalid request JSON: {error}")),
        };

        let mut encoded = serde_json::to_vec(&response)?;
        encoded.push(b'\n');
        stdout.write_all(&encoded).await?;
        stdout.flush().await?;
    }

    Ok(())
}
