import {
  type Action,
  type AgentReply,
  type ConnectionConfig,
  type Device,
  type DeviceCommand,
  type DeviceReply,
  type RuntimeInfo,
  validateCommand,
} from "@boardmate/protocol";

export interface BoardmateClient {
  runtime(): Promise<RuntimeInfo>;
  devices(): Promise<Device[]>;
  execute(command: DeviceCommand): Promise<DeviceReply>;
  chat(message: string): Promise<AgentReply>;
  connect(config: ConnectionConfig): Promise<Device>;
  disconnect(deviceId: string): Promise<void>;
  serialPorts(): Promise<string[]>;
  scanBle(): Promise<{ address: string; name: string }[]>;
  setExpanded(expanded: boolean): Promise<void>;
}

export function createSimulatorClient(): BoardmateClient {
  const devices: Device[] = [
    {
      id: "demo-esp32",
      name: "Mesa de trabajo",
      board: "ESP32",
      transport: "simulator",
      simulated: true,
      capabilities: ["move_servo", "set_light"],
      state: { servoAngle: 90, lightOn: false },
    },
    {
      id: "demo-pico",
      name: "Ambiente",
      board: "Raspberry Pi Pico",
      transport: "simulator",
      simulated: true,
      capabilities: ["read_temperature"],
      state: { temperatureC: 24.5 },
    },
  ];
  const recent = new Map<string, { command: string; reply: DeviceReply }>();
  const desktopOnly = async (): Promise<never> => {
    throw new Error("Abre la aplicación desktop para conectar hardware real.");
  };
  const client: BoardmateClient = {
    async runtime() {
      return {
        platform: "browser",
        native: false,
        provider: "demo",
        transports: [
          { kind: "serial", available: false, description: "USB / UART" },
          { kind: "websocket", available: false, description: "Wi-Fi" },
          { kind: "ble", available: false, description: "Bluetooth LE" },
          { kind: "classic", available: false, description: "Bluetooth Classic (Linux)" },
        ],
      };
    },
    async devices() {
      return structuredClone(devices);
    },
    async execute(command) {
      const previous = recent.get(command.id);
      if (previous) {
        if (previous.command !== JSON.stringify(command)) throw new Error("Reused request id");
        return structuredClone(previous.reply);
      }
      const device = devices.find((device) => device.id === command.deviceId);
      if (!device) throw new Error("Unknown device");
      validateCommand(command, device);
      if (command.action === "move_servo")
        device.state.servoAngle = command.arguments.angle as number;
      if (command.action === "set_light")
        device.state.lightOn = command.arguments.enabled as boolean;
      const reply: DeviceReply = {
        requestId: command.id,
        deviceId: device.id,
        ok: true,
        state: structuredClone(device.state),
      };
      if (recent.size >= 64) {
        const first = recent.keys().next().value;
        if (first) recent.delete(first);
      }
      recent.set(command.id, { command: JSON.stringify(command), reply: structuredClone(reply) });
      return reply;
    },
    async chat(message) {
      if (!message.trim() || new TextEncoder().encode(message).length > 4000)
        throw new Error("Escribe un mensaje de hasta 4000 bytes.");
      const text = message.toLowerCase();
      if (text.split(/\s+/).some((word) => word === "si" || word === "cuando")) {
        return {
          text: "El modo demo admite órdenes directas. Configura Groq para interpretar condiciones.",
          executions: [],
        };
      }
      const executions: DeviceReply[] = [];
      const run = async (deviceId: string, action: Action, arguments_: Record<string, unknown>) => {
        executions.push(
          await client.execute({
            id: crypto.randomUUID(),
            deviceId,
            action,
            arguments: arguments_,
          }),
        );
      };
      const phrases: string[] = [];
      if (/servo|gira|mueve/.test(text)) {
        const number = text.match(/-?\d+(?:\.\d+)?/);
        if (!number)
          return {
            text: "Indica un ángulo: por ejemplo, gira el servo a 45 grados.",
            executions: [],
          };
        const angle = Number(number[0]);
        await run("demo-esp32", "move_servo", { angle });
        phrases.push(`Servo simulado ajustado a ${angle}°.`);
      }
      if (/enciende|apaga/.test(text)) {
        const enabled = !text.includes("apaga");
        await run("demo-esp32", "set_light", { enabled });
        phrases.push(`Luz simulada ${enabled ? "encendida" : "apagada"}.`);
      }
      if (text.includes("temperatura")) {
        await run("demo-pico", "read_temperature", {});
        phrases.push("Temperatura simulada: 24.5 °C.");
      }
      return {
        text:
          phrases.join(" ") ||
          "Prueba una de las sugerencias. Esta demostración funciona sin IA; Groq se configura en la aplicación desktop.",
        executions,
      };
    },
    connect: desktopOnly,
    disconnect: desktopOnly,
    serialPorts: desktopOnly,
    scanBle: desktopOnly,
    async setExpanded() {},
  };
  return client;
}

export type NativeInvoke = <T>(command: string, arguments_?: Record<string, unknown>) => Promise<T>;

export function createNativeClient(invoke: NativeInvoke): BoardmateClient {
  return {
    runtime: () => invoke<RuntimeInfo>("runtime_info"),
    devices: () => invoke<Device[]>("list_devices"),
    execute: (command) => invoke<DeviceReply>("execute_command", { command }),
    chat: (message) => invoke<AgentReply>("chat", { message }),
    connect: (config) => invoke<Device>("connect_device", { config }),
    disconnect: (deviceId) => invoke<void>("disconnect_device", { deviceId }),
    serialPorts: () => invoke<string[]>("serial_ports"),
    scanBle: () => invoke<{ address: string; name: string }[]>("scan_ble"),
    setExpanded: (expanded) => invoke<void>("set_expanded", { expanded }),
  };
}

export async function createClient(): Promise<BoardmateClient> {
  const { isTauri, invoke } = await import("@tauri-apps/api/core");
  return isTauri() ? createNativeClient(invoke) : createSimulatorClient();
}
