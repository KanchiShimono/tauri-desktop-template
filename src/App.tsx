import { invoke } from "@tauri-apps/api/core";
import { ThemeProvider } from "next-themes";
import { useState } from "react";

import { Button } from "@/components/ui/button";
import { Field, FieldGroup, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";

import reactLogo from "./assets/react.svg";

function App() {
  const [greetMsg, setGreetMsg] = useState("");
  const [name, setName] = useState("");

  async function greet() {
    try {
      setGreetMsg(await invoke<string>("greet", { name }));
    } catch {
      setGreetMsg("Could not get a greeting. Please try again.");
    }
  }

  return (
    <ThemeProvider attribute="class" defaultTheme="system" enableSystem disableTransitionOnChange>
      <main className="flex flex-col items-center gap-4 px-4 pt-[10vh] text-center">
        <h1 className="text-2xl font-semibold">Welcome to Tauri + React</h1>

        <div className="flex justify-center">
          <a href="https://vite.dev" target="_blank" rel="noreferrer">
            <img
              src="/vite.svg"
              className="h-24 p-6 transition-[filter] duration-750 hover:drop-shadow-[0_0_2em_#747bff]"
              alt="Vite logo"
            />
          </a>
          <a href="https://tauri.app" target="_blank" rel="noreferrer">
            <img
              src="/tauri.svg"
              className="h-24 p-6 transition-[filter] duration-750 hover:drop-shadow-[0_0_2em_#24c8db]"
              alt="Tauri logo"
            />
          </a>
          <a href="https://react.dev" target="_blank" rel="noreferrer">
            <img
              src={reactLogo}
              className="h-24 p-6 transition-[filter] duration-750 hover:drop-shadow-[0_0_2em_#61dafb]"
              alt="React logo"
            />
          </a>
        </div>
        <p>Click on the Tauri, Vite, and React logos to learn more.</p>

        <form
          className="w-full max-w-xs"
          onSubmit={(event) => {
            event.preventDefault();
            void greet();
          }}
        >
          <FieldGroup>
            <Field orientation="horizontal">
              <FieldLabel className="sr-only" htmlFor="greet-input">
                Name
              </FieldLabel>
              <Input
                id="greet-input"
                name="name"
                value={name}
                onChange={(event) => {
                  setName(event.currentTarget.value);
                }}
                placeholder="Enter a name..."
              />
              <Button type="submit">Greet</Button>
            </Field>
          </FieldGroup>
        </form>
        <output className="max-w-full wrap-anywhere" htmlFor="greet-input">
          {greetMsg}
        </output>
      </main>
    </ThemeProvider>
  );
}

export default App;
