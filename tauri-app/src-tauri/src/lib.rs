use tauri::{AppHandle, Emitter, State};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use url::Url;
use futures_util::{StreamExt, SinkExt};
use async_channel::{unbounded, Sender, Receiver};

// Lo stato che conterrà il canale per comunicare con il task WebSocket
pub struct AppState {
    ws_sender: Sender<String>,
}

// Il comando che il frontend chiamerà per inviare un messaggio.
#[tauri::command]
async fn send_message(message: String, state: State<'_, AppState>) -> Result<(), String> {
    state.ws_sender.send(message).await.map_err(|e| e.to_string())
}

// Task asincrono che gestisce la connessione WebSocket
async fn websocket_client_task(app_handle: AppHandle, ws_sender_rx: Receiver<String>) {
    // NOTA: Questo deve essere l'indirizzo del tuo SERVER CENTRALE.
    let url_str = "ws://127.0.0.1:8080/ws";
    let url = Url::parse(url_str).unwrap();

    // Loop di riconnessione in caso di disconnessione
    loop {
        println!("Tentativo di connessione a {}", url);
        match connect_async(url_str).await {
            Ok((ws_stream, _)) => {
                println!("Connesso al server WebSocket");
                let _ = app_handle.emit("connection-status", "connected");

                let (mut write, mut read) = ws_stream.split();

                loop {
                    tokio::select! {
                        // Ascolta i messaggi in arrivo dal server centrale
                        Some(msg) = read.next() => {
                            match msg {
                                Ok(Message::Text(text)) => {
                                    println!("Ricevuto messaggio dal server: {}", text);
                                    // Emetti un evento al frontend con il nuovo messaggio
                                    let _ = app_handle.emit("new-message", text);
                                }
                                Ok(_) => { /* Ignora altri tipi di messaggi */ }
                                Err(e) => {
                                    println!("Errore WebSocket: {}", e);
                                    break; // Esce dal loop interno per tentare la riconnessione
                                }
                            }
                        },
                        // Ascolta i messaggi in arrivo dal frontend (tramite il comando `send_message`)
                        Ok(msg_to_send) = ws_sender_rx.recv() => {
                            if write.send(Message::Text(msg_to_send)).await.is_err() {
                                println!("Impossibile inviare messaggio, connessione persa.");
                                break; // Esce dal loop interno per tentare la riconnessione
                            }
                        }
                    }
                }
            }
            Err(e) => {
                println!("Impossibile connettersi al server WebSocket: {}", e);
                let _ = app_handle.emit("connection-status", "disconnected");
            }
        }
        // Attendi un po' prima di tentare la riconnessione
        println!("Riconnessione tra 5 secondi...");
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let (ws_sender_tx, ws_sender_rx) = unbounded::<String>();

    tauri::Builder::default()
        .manage(AppState { ws_sender: ws_sender_tx })
        .setup(|app| {
            let app_handle = app.handle().clone();
            // Avvia il nostro gestore WebSocket in un task separato
            tauri::async_runtime::spawn(async move {
                websocket_client_task(app_handle, ws_sender_rx).await;
            });
            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![send_message])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
