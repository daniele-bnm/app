use std::{net::SocketAddr, sync::Arc};
use futures_util::{SinkExt, StreamExt};
use log::{info, warn};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use tokio_tungstenite::accept_async;

// Un alias per una lista di client condivisa e thread-safe
type ClientList = Arc<Mutex<Vec<futures_util::stream::SplitSink<tokio_tungstenite::WebSocketStream<TcpStream>, tokio_tungstenite::tungstenite::Message>>>>;

async fn handle_connection(peer: SocketAddr, stream: TcpStream, clients: ClientList) {
    let ws_stream = match accept_async(stream).await {
        Ok(ws) => ws,
        Err(e) => {
            warn!("Errore durante l'handshake websocket con {}: {}", peer, e);
            return;
        }
    };

    info!("Nuova connessione WebSocket: {}", peer);
    let (write, mut read) = ws_stream.split();

    // Aggiungi il nuovo client alla lista
    clients.lock().await.push(write);

    // Loop per ascoltare i messaggi da questo specifico client
    while let Some(msg) = read.next().await {
        match msg {
            Ok(msg) => {
                if msg.is_text() || msg.is_binary() {
                    info!("Ricevuto messaggio da {}: {}", peer, msg.to_text().unwrap_or(""));
                    
                    // Inoltra il messaggio a tutti i client connessi
                    let mut clients_guard = clients.lock().await;
                    for client_write in clients_guard.iter_mut() {
                        if client_write.send(msg.clone()).await.is_err() {
                            // Potrebbe essere necessario rimuovere i client che danno errore
                        }
                    }
                }
            }
            Err(e) => {
                warn!("Errore ricezione messaggio da {}: {}", peer, e);
                break;
            }
        }
    }

    info!("Connessione con {} chiusa", peer);
    // Qui dovresti implementare la logica per rimuovere il client dalla lista `clients`
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let addr = "127.0.0.1:8080";
    let listener = TcpListener::bind(&addr).await.expect("Impossibile mettersi in ascolto");
    info!("Server WebSocket in ascolto su: ws://{}", addr);

    let clients = ClientList::new(Mutex::new(Vec::new()));

    while let Ok((stream, _)) = listener.accept().await {
        let peer = stream.peer_addr().expect("I flussi connessi dovrebbero avere un indirizzo peer");
        info!("Indirizzo peer: {}", peer);
        tokio::spawn(handle_connection(peer, stream, clients.clone()));
    }
}

