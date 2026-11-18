import React from "react";
import ReactDOM from "react-dom/client";

import App from "./App";

import "./styles/globals.css";

const darkScheme = window.matchMedia("(prefers-color-scheme: dark)");
function syncTheme() {
  document.documentElement.classList.toggle("dark", darkScheme.matches);
}
syncTheme();
darkScheme.addEventListener("change", syncTheme);

const root = document.querySelector("#root");
if (root === null) {
  throw new Error("Missing #root element");
}

ReactDOM.createRoot(root).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
