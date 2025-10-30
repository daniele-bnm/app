import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import "./App.css";

function App() {
  const [messages, setMessages] = useState<string[]>([]);
  const [input, setInput] = useState("");
  const [targetId, setTargetId] = useState("");
  const [chatMode, setChatMode] = useState<"private" | "group">("private");
  const [connectionStatus, setConnectionStatus] = useState("disconnected");

  useEffect(() => {
    // Ascolta gli eventi dal backend Rust
    const unlistenMessage = listen<string>("new-message", (event) => {
      try {
        const msg = JSON.parse(event.payload);
        const prettyMessage = JSON.stringify(msg, null, 2);
        setMessages((prev) => [...prev, `Server: ${prettyMessage}`]);
      } catch (e) {
        setMessages((prev) => [...prev, `Server (raw): ${event.payload}`]);
      }
    });

    const unlistenStatus = listen<string>("connection-status", (event) => {
      setConnectionStatus(event.payload);
    });

    return () => {
      // Pulisci i listener quando il componente viene smontato
      unlistenMessage.then(f => f());
      unlistenStatus.then(f => f());
    };
  }, []);

  const sendMessage = async () => {
    if (!input || !targetId) {
      alert("Assicurati di aver inserito ID e messaggio.");
      return;
    }

    const messageType = chatMode === "private" ? "private_message" : "group_message";
    const idKey = chatMode === "private" ? "recipient_id" : "group_id";

    const message = {
      type: messageType,
      payload: {
        [idKey]: targetId,
        content: input,
      },
    };

    const messageString = JSON.stringify(message);

    try {
      // Chiama il comando Rust per inviare il messaggio
      await invoke("send_message", { message: messageString });
      setMessages((prev) => [...prev, `Tu: ${input} (a ${targetId})`]);
      setInput("");
    } catch (error) {
      console.error("Errore durante l'invio del messaggio:", error);
      alert(`Errore invio: ${error}`);
    }
  };

  return (
    <div className="container">
      <h1>Tauri Chat Client (Rust-Managed)</h1>
      <p>Stato Connessione: <span className={connectionStatus}>{connectionStatus}</span></p>
      
      <div className="chat-box">
        {messages.map((msg, index) => (
          <p key={index}>{msg}</p>
        ))}
      </div>

      <div className="controls">
        <select value={chatMode} onChange={(e) => setChatMode(e.target.value as "private" | "group")}>
          <option value="private">Messaggio Privato</option>
          <option value="group">Messaggio di Gruppo</option>
        </select>
        <input
          value={targetId}
          onChange={(e) => setTargetId(e.target.value)}
          placeholder={chatMode === 'private' ? "ID Utente Destinatario" : "ID Gruppo"}
        />
      </div>

      <div className="row">
        <input
          value={input}
          onChange={(e) => setInput(e.target.value)}
          onKeyPress={(e) => e.key === 'Enter' && sendMessage()}
          placeholder="Scrivi un messaggio..."
        />
        <button onClick={sendMessage}>Invia</button>
      </div>
    </div>
  );
}

export default App;

