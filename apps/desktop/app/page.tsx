"use client";

import type { Action, Device, RuntimeInfo } from "@boardmate/protocol";
import { type BoardmateClient, createClient } from "@boardmate/sdk";
import { BoardIcon, Button, Status } from "@boardmate/ui";
import { type FormEvent, useEffect, useRef, useState } from "react";
import { ConnectionForm } from "../components/connection-form";
import { DeviceControls } from "../components/device-controls";
import { errorText } from "../lib/errors";

interface ChatMessage {
  id: string;
  role: "user" | "assistant";
  text: string;
}

export default function Page() {
  const [client, setClient] = useState<BoardmateClient | null>(null);
  const [info, setInfo] = useState<RuntimeInfo | null>(null);
  const [devices, setDevices] = useState<Device[]>([]);
  const [tab, setTab] = useState<"assistant" | "devices">("assistant");
  const [expanded, setExpanded] = useState(true);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [input, setInput] = useState("");
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const end = useRef<HTMLDivElement>(null);

  useEffect(() => {
    let active = true;
    createClient()
      .then(async (next) => {
        const [runtime, devices] = await Promise.all([next.runtime(), next.devices()]);
        if (active) {
          setClient(next);
          setInfo(runtime);
          setDevices(devices);
        }
      })
      .catch((error: unknown) => {
        if (active) setError(errorText(error));
      });
    return () => {
      active = false;
    };
  }, []);

  useEffect(() => {
    if (messages.length > 0) end.current?.scrollIntoView({ block: "end" });
  }, [messages.length]);

  async function operate(operation: () => Promise<unknown>) {
    if (busy || !client) return;
    setBusy(true);
    setError("");
    try {
      await operation();
    } catch (error) {
      setError(errorText(error));
    } finally {
      try {
        setDevices(await client.devices());
      } catch (error) {
        setError(errorText(error));
      }
      setBusy(false);
    }
  }

  function control(device: Device, action: Action, arguments_: Record<string, unknown>) {
    void operate(async () => {
      const reply = await client?.execute({
        id: crypto.randomUUID(),
        deviceId: device.id,
        action,
        arguments: arguments_,
      });
      if (reply && !reply.ok) throw new Error(reply.error || "La placa rechazó la orden.");
    });
  }

  async function send(text: string) {
    if (!text.trim() || busy || !client) return;
    setMessages((current) =>
      [...current, { id: crypto.randomUUID(), role: "user" as const, text: text.trim() }].slice(
        -30,
      ),
    );
    setInput("");
    await operate(async () => {
      const reply = await client.chat(text.trim());
      setMessages((current) =>
        [
          ...current,
          { id: crypto.randomUUID(), role: "assistant" as const, text: reply.text },
        ].slice(-30),
      );
      const failed = reply.executions.find((execution) => !execution.ok);
      if (reply.error) throw new Error(reply.error);
      if (failed) throw new Error(failed.error || "Una acción no se completó.");
    });
  }

  async function toggle() {
    if (!client) return;
    try {
      await client.setExpanded(!expanded);
      setExpanded(!expanded);
    } catch (error) {
      setError(errorText(error));
    }
  }

  if (!expanded)
    return (
      <main className={`stage ${info?.native ? "stage--native" : ""}`}>
        <button
          type="button"
          className="companion"
          aria-label="Abrir boardmate"
          onClick={() => void toggle()}
        >
          <BoardIcon size={32} />
        </button>
      </main>
    );

  return (
    <main className={`stage ${info?.native ? "stage--native" : ""}`}>
      <section className="panel" aria-label="Compañero de hardware">
        <header className="panel-header" data-tauri-drag-region>
          <span className="brand-mark">
            <BoardIcon size={25} />
          </span>
          <div className="brand-copy" data-tauri-drag-region>
            <h1>boardmate</h1>
            <p>
              {info
                ? `${devices.length} dispositivos · ${info.provider === "demo" ? "Demo local" : info.provider}`
                : "Preparando tu mesa…"}
            </p>
          </div>
          <Button
            className="button--quiet collapse"
            aria-label="Contraer panel"
            disabled={!client}
            onClick={() => void toggle()}
          >
            −
          </Button>
        </header>
        <div className="tabs" role="tablist" aria-label="Panel principal">
          <button
            type="button"
            role="tab"
            id="assistant-tab"
            aria-selected={tab === "assistant"}
            aria-controls="assistant-panel"
            onClick={() => setTab("assistant")}
          >
            Asistente
          </button>
          <button
            type="button"
            role="tab"
            id="devices-tab"
            aria-selected={tab === "devices"}
            aria-controls="devices-panel"
            onClick={() => setTab("devices")}
          >
            Dispositivos <span className="count">{devices.length}</span>
          </button>
        </div>
        {error && (
          <div className="error" role="alert" aria-label="Error de operación">
            <span>{error}</span>
            <button type="button" aria-label="Cerrar error" onClick={() => setError("")}>
              ×
            </button>
          </div>
        )}
        {tab === "assistant" ? (
          <div
            className="assistant"
            id="assistant-panel"
            role="tabpanel"
            aria-labelledby="assistant-tab"
          >
            <div
              className="conversation"
              role="log"
              aria-label="Conversación con boardmate"
              aria-live="polite"
            >
              {messages.length === 0 && (
                <div className="welcome">
                  <span className="welcome-icon">
                    <BoardIcon size={32} />
                  </span>
                  <h2>Tu hardware, a mano.</h2>
                  <p>Consulta un sensor, mueve un servo o controla una luz desde aquí.</p>
                  <div className="suggestions">
                    <Button
                      disabled={!client || busy}
                      onClick={() => void send("¿Qué temperatura hay?")}
                    >
                      Leer temperatura
                    </Button>
                    <Button
                      disabled={!client || busy}
                      onClick={() => void send("Gira el servo a 45 grados")}
                    >
                      Girar servo a 45°
                    </Button>
                    <Button disabled={!client || busy} onClick={() => void send("Enciende la luz")}>
                      Encender luz
                    </Button>
                  </div>
                </div>
              )}
              {messages.map((message) => (
                <div key={message.id} className={`message message--${message.role}`}>
                  <span className="message-author">
                    {message.role === "user" ? "Tú" : "boardmate"}
                  </span>
                  <p>{message.text}</p>
                </div>
              ))}
              {busy && (
                <p className="working" role="status">
                  Procesando solicitud…
                </p>
              )}
              <div ref={end} />
            </div>
            <form
              className="composer"
              onSubmit={(event: FormEvent) => {
                event.preventDefault();
                void send(input);
              }}
            >
              <label className="sr-only" htmlFor="message">
                Mensaje para boardmate
              </label>
              <input
                id="message"
                value={input}
                onChange={(event) => setInput(event.target.value)}
                maxLength={4000}
                placeholder="¿Qué hacemos con tu hardware?"
                autoComplete="off"
                disabled={!client || busy}
              />
              <Button
                type="submit"
                className="button--primary"
                aria-label="Enviar mensaje"
                disabled={!client || busy || !input.trim()}
              >
                Enviar
              </Button>
            </form>
          </div>
        ) : (
          <div
            className="device-panel"
            id="devices-panel"
            role="tabpanel"
            aria-labelledby="devices-tab"
          >
            <div className="section-intro">
              <h2>Tu mesa de trabajo</h2>
              <p>Controles directos, también sin IA.</p>
            </div>
            {devices.map((device) => (
              <DeviceControls
                key={device.id}
                device={device}
                busy={busy}
                onControl={control}
                onDisconnect={() =>
                  void operate(() => client?.disconnect(device.id) ?? Promise.resolve())
                }
              />
            ))}
            {client && info && (
              <ConnectionForm
                client={client}
                info={info}
                busy={busy}
                onConnect={(config) => void operate(() => client.connect(config))}
              />
            )}
          </div>
        )}
        <footer className="panel-footer">
          <Status tone={client ? "success" : "neutral"}>{client ? "Listo" : "Iniciando"}</Status>
          <span>
            {info?.native ? `${info.platform} · control local` : "Vista previa · hardware simulado"}
          </span>
        </footer>
      </section>
    </main>
  );
}
