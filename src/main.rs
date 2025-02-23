use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::net::{TcpListener};
use tokio_tungstenite::accept_async;
use futures_util::{SinkExt, StreamExt};
use std::collections::HashMap;
use std::net::SocketAddr;
use serde::Deserialize;

type Clients = Arc<Mutex<HashMap<SocketAddr, tokio::sync::mpsc::UnboundedSender<String>>>>;

#[derive(Debug, Deserialize)]
struct ServerConfig {
    host: String,
    port: u16,
}

#[derive(Debug, Deserialize)]
struct Config {
    server: ServerConfig,
}

#[tokio::main]
async fn main() {
    // 读取配置文件
    let config = config::Config::builder()
        .add_source(config::File::new("config", config::FileFormat::Toml))
        .build()
        .unwrap();
    let config: Config = config.try_deserialize().unwrap();
    
    let clients: Clients = Arc::new(Mutex::new(HashMap::new()));
    let addr = format!("{}:{}", config.server.host, config.server.port);
    let listener = TcpListener::bind(&addr).await.expect("Failed to bind");
    println!("WebSocket server started on {}", addr);

    while let Ok((stream, addr)) = listener.accept().await {
        let clients = clients.clone();
        
        tokio::spawn(async move {
            let ws_stream = accept_async(stream)
                .await
                .expect("Failed to accept WebSocket connection");
            
            let (mut ws_sender, mut ws_receiver) = ws_stream.split();
            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();

            // 检查是否已达到最大连接数
            if clients.lock().await.len() >= 2 {
                let _ = ws_sender
                    .send(tokio_tungstenite::tungstenite::Message::Text(
                        "服务器已达到最大连接数（2个客户端）".to_string(),
                    ))
                    .await;
                return;
            }

            // 添加新客户端
            clients.lock().await.insert(addr, tx);
            let client_number = clients.lock().await.len();
            println!("新客户端连接。当前连接数: {}", client_number);

            // 发送欢迎消息
            let _ = ws_sender
                .send(tokio_tungstenite::tungstenite::Message::Text(
                    format!("你是客户端 #{}", client_number),
                ))
                .await;

            // 处理接收消息
            tokio::spawn(async move {
                while let Some(msg) = rx.recv().await {
                    let _ = ws_sender
                        .send(tokio_tungstenite::tungstenite::Message::Text(msg))
                        .await;
                }
            });

            // 处理发送消息
            while let Some(result) = ws_receiver.next().await {
                match result {
                    Ok(msg) => {
                        if let tokio_tungstenite::tungstenite::Message::Text(text) = msg {
                            println!("收到消息: {}", text);
                            
                            // 转发消息给其他客户端
                            let clients_lock = clients.lock().await;
                            for (&other_addr, tx) in clients_lock.iter() {
                                if other_addr != addr {
                                    let _ = tx.send(text.clone());
                                }
                            }
                        }
                    }
                    Err(_) => break,
                }
            }

            // 处理客户端断开连接
            clients.lock().await.remove(&addr);
            println!("客户端断开连接。当前连接数: {}", clients.lock().await.len());
        });
    }
}
