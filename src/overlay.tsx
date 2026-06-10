import React from "react";
import ReactDOM from "react-dom/client";
import { DictationOverlay } from "./components/overlay/DictationOverlay";
import "./styles/globals.css";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <DictationOverlay />
  </React.StrictMode>,
);
