import type { ConnectionConfig, RuntimeInfo } from "@boardmate/protocol";
import type { BoardmateClient } from "@boardmate/sdk";
import { Button } from "@boardmate/ui";
import { useState } from "react";
import { errorText } from "../lib/errors";

export function ConnectionForm({
  client,
  info,
  busy,
  onConnect,
}: {
  client: BoardmateClient;
  info: RuntimeInfo;
  busy: boolean;
  onConnect: (config: ConnectionConfig) => void;
}) {
  const [kind, setKind] = useState<ConnectionConfig["kind"]>("serial");
  const [address, setAddress] = useState("");
  const [baudRate, setBaudRate] = useState(115200);
  const [channel, setChannel] = useState(1);
  const [scanning, setScanning] = useState(false);
  const [found, setFound] = useState<{ address: string; name: string }[]>([]);
  const [note, setNote] = useState("");

  async function discover() {
    setScanning(true);
    setNote("");
    try {
      const found =
        kind === "ble"
          ? await client.scanBle()
          : (await client.serialPorts()).map((address) => ({ address, name: address }));
      setFound(found);
      if (found.length === 0) setNote("No se encontraron dispositivos compatibles.");
    } catch (error) {
      setNote(errorText(error));
    } finally {
      setScanning(false);
    }
  }

  const placeholder =
    kind === "websocket"
      ? "ws://192.168.1.50:8765"
      : kind === "classic"
        ? "AA:BB:CC:DD:EE:FF"
        : kind === "ble"
          ? "Dirección obtenida al buscar"
          : info.platform === "windows"
            ? "COM3"
            : info.platform === "macos"
              ? "/dev/cu.usbserial…"
              : "/dev/ttyUSB0";
  return (
    <details className="connection">
      <summary>Conectar dispositivo</summary>
      {!info.native && (
        <p className="connection-note">
          La vista previa usa simulación. Abre la aplicación desktop para conectar placas reales.
        </p>
      )}
      <form
        onSubmit={(event) => {
          event.preventDefault();
          onConnect({ kind, address: address.trim(), baudRate, channel });
        }}
      >
        <fieldset disabled={!info.native || busy || scanning}>
          <label htmlFor="transport">Conexión</label>
          <select
            id="transport"
            value={kind}
            onChange={(event) => {
              setKind(event.target.value as ConnectionConfig["kind"]);
              setAddress("");
              setFound([]);
              setNote("");
            }}
          >
            {info.transports.map((transport) => (
              <option key={transport.kind} value={transport.kind} disabled={!transport.available}>
                {transport.description}
                {!transport.available && info.native ? " · no disponible" : ""}
              </option>
            ))}
          </select>
          <label htmlFor="address">Dirección o puerto</label>
          <input
            id="address"
            value={address}
            onChange={(event) => setAddress(event.target.value)}
            placeholder={placeholder}
            required
          />
          {kind === "serial" && (
            <>
              <label htmlFor="baud">Velocidad UART</label>
              <input
                id="baud"
                type="number"
                min="1"
                max="4000000"
                value={baudRate}
                onChange={(event) => setBaudRate(Number(event.target.value))}
                required
              />
            </>
          )}
          {kind === "classic" && (
            <>
              <label htmlFor="channel">Canal RFCOMM</label>
              <input
                id="channel"
                type="number"
                min="1"
                max="30"
                value={channel}
                onChange={(event) => setChannel(Number(event.target.value))}
                required
              />
            </>
          )}
          <div className="connection-actions">
            {(kind === "serial" || kind === "ble") && (
              <Button onClick={() => void discover()}>Buscar dispositivos</Button>
            )}
            <Button type="submit" className="button--primary" disabled={!address.trim()}>
              Conectar
            </Button>
          </div>
        </fieldset>
        {scanning && <p role="status">Buscando dispositivos…</p>}
        {found.length > 0 && (
          <label>
            Dispositivos encontrados
            <select
              aria-label="Dispositivos encontrados"
              value=""
              onChange={(event) => setAddress(event.target.value)}
            >
              <option value="" disabled>
                Seleccionar dispositivo
              </option>
              {found.map((device) => (
                <option key={device.address} value={device.address}>
                  {device.name}
                </option>
              ))}
            </select>
          </label>
        )}
        {note && (
          <p role="status" className="connection-note">
            {note}
          </p>
        )}
      </form>
    </details>
  );
}
