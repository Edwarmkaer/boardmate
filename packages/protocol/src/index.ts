export type Action = "read_temperature" | "move_servo" | "set_light";
export type TransportKind = "simulator" | "serial" | "websocket" | "ble" | "classic";

export interface DeviceState {
  temperatureC?: number | null;
  servoAngle?: number | null;
  lightOn?: boolean | null;
}

export interface Device {
  id: string;
  name: string;
  board: string;
  transport: TransportKind;
  simulated: boolean;
  capabilities: Action[];
  state: DeviceState;
}

export interface DeviceCommand {
  id: string;
  deviceId: string;
  action: Action;
  arguments: Record<string, unknown>;
}

export interface DeviceReply {
  requestId: string;
  deviceId: string;
  ok: boolean;
  state?: DeviceState | null;
  error?: string | null;
}

export interface AgentReply {
  text: string;
  executions: DeviceReply[];
  error?: string;
}

export interface RuntimeInfo {
  platform: string;
  provider: string;
  native: boolean;
  transports: { kind: TransportKind; available: boolean; description: string }[];
}

export interface ConnectionConfig {
  kind: Exclude<TransportKind, "simulator">;
  address: string;
  baudRate?: number;
  channel?: number;
}

export function validateCommand(command: DeviceCommand, device: Device): void {
  if (!command.id.trim() || command.id.length > 128) throw new Error("Invalid request id");
  if (command.deviceId !== device.id) throw new Error("Command addresses another device");
  if (!device.capabilities.includes(command.action))
    throw new Error("Unsupported device capability");
  const args = command.arguments;
  if (!args || typeof args !== "object" || Array.isArray(args))
    throw new Error("Invalid arguments");
  if (command.action === "move_servo") {
    const angle = args.angle;
    if (typeof angle !== "number" || !Number.isInteger(angle) || angle < 0 || angle > 180) {
      throw new Error("Servo angle must be an integer between 0 and 180");
    }
  } else if (command.action === "set_light" && typeof args.enabled !== "boolean") {
    throw new Error("Light enabled must be a boolean");
  }
}
