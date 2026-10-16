import type { Action, Device } from "@boardmate/protocol";
import { BoardIcon, Button, Status } from "@boardmate/ui";
import { useEffect, useState } from "react";

export function DeviceControls({
  device,
  busy,
  onControl,
  onDisconnect,
}: {
  device: Device;
  busy: boolean;
  onControl: (device: Device, action: Action, arguments_: Record<string, unknown>) => void;
  onDisconnect: () => void;
}) {
  const [angle, setAngle] = useState(device.state.servoAngle ?? 90);
  useEffect(() => {
    setAngle(device.state.servoAngle ?? 90);
  }, [device.state.servoAngle]);
  return (
    <section className="device" aria-label={device.name}>
      <div className="device-header">
        <span className="device-icon">
          <BoardIcon size={22} />
        </span>
        <div>
          <h3>{device.name}</h3>
          <p>{device.board}</p>
        </div>
        <Status tone={device.simulated ? "warning" : "success"}>
          {device.simulated ? "Simulado" : "Conectado"}
        </Status>
      </div>
      {device.capabilities.includes("move_servo") && (
        <div className="control">
          <label htmlFor={`servo-${device.id}`}>
            Ángulo del servo <strong>{angle}°</strong>
          </label>
          <input
            id={`servo-${device.id}`}
            type="range"
            min="0"
            max="180"
            step="1"
            value={angle}
            disabled={busy}
            onChange={(event) => setAngle(Number(event.target.value))}
          />
          <Button
            disabled={busy || angle === device.state.servoAngle}
            onClick={() => onControl(device, "move_servo", { angle })}
          >
            Aplicar ángulo
          </Button>
        </div>
      )}
      {device.capabilities.includes("set_light") && (
        <div className="control control--inline">
          <span>Luz {device.state.lightOn ? "encendida" : "apagada"}</span>
          <Button
            disabled={busy}
            onClick={() => onControl(device, "set_light", { enabled: !device.state.lightOn })}
          >
            {device.state.lightOn ? "Apagar luz" : "Encender luz"}
          </Button>
        </div>
      )}
      {device.capabilities.includes("read_temperature") && (
        <div className="control control--inline">
          <div>
            <span className="reading">
              {device.state.temperatureC === undefined || device.state.temperatureC === null
                ? "—"
                : `${device.state.temperatureC} °C`}
            </span>
            <span className="reading-label">
              Temperatura {device.simulated ? "simulada" : "medida"}
            </span>
          </div>
          <Button disabled={busy} onClick={() => onControl(device, "read_temperature", {})}>
            Leer temperatura
          </Button>
        </div>
      )}
      {!device.simulated && (
        <Button className="button--quiet disconnect" disabled={busy} onClick={onDisconnect}>
          Desconectar
        </Button>
      )}
    </section>
  );
}
