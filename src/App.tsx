import { useState } from "react";
import reactLogo from "./assets/react.svg";
import { invoke } from "@tauri-apps/api/core";
import { AnalyseDuMot } from "./types";
import "./App.css";

function App() {
  const [mot, setMot] = useState("");
  // Le typage pour TS sur les useState se fait juste après le useState
  const [motAnalysé, setMotAnalysé] = useState<AnalyseDuMot | null>(null);

  // Pour utiliser ma fonction, je dois la faire en asynchrone car elle appelle l'API de Rust et donc il faut que j'attende la réponse de Rust pour pouvoir set la valeur.
  async function analyser_mot() {
    setMotAnalysé(await invoke<AnalyseDuMot>("analyser_mot", { mot }));
  }

  return (
    <main className="container">
      <h1>Welcome to Tauri + React</h1>

      <div className="row">
        <a href="https://vite.dev" target="_blank">
          <img src="/vite.svg" className="logo vite" alt="Vite logo" />
        </a>
        <a href="https://tauri.app" target="_blank">
          <img src="/tauri.svg" className="logo tauri" alt="Tauri logo" />
        </a>
        <a href="https://react.dev" target="_blank">
          <img src={reactLogo} className="logo react" alt="React logo" />
        </a>
      </div>
      <p>Click on the Tauri, Vite, and React logos to learn more.</p>

      <form
        className="row"
        onSubmit={(e) => {
          e.preventDefault();
          analyser_mot();
        }}
      >
        <input
          id="analyser_mot-input"
          onChange={(e) => setMot(e.currentTarget.value)}
          placeholder="Entre un mot..."
        />
        <button type="submit">Analyser mot</button>
      </form>
      {/* Vu que motAnalysé peut être null, je dois vérifier s'il est présent */}
      <p>{motAnalysé && motAnalysé.octets}</p>
      <p>{motAnalysé && motAnalysé.caracteres}</p>
    </main>
  );
}

export default App;
