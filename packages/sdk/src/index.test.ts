import { describe, expect, it } from "vitest";
import { createNativeClient, createSimulatorClient, type NativeInvoke } from "./index";

describe("explicit browser simulator", () => {
  it("updates the selected board and returns the resulting state", async () => {
    const client = createSimulatorClient();
    const result = await client.execute({
      id: "1",
      deviceId: "demo-esp32",
      action: "move_servo",
      arguments: { angle: 45 },
    });
    expect(result.state?.servoAngle).toBe(45);
    expect(
      (await client.devices()).find((device) => device.id === "demo-esp32")?.state.servoAngle,
    ).toBe(45);
  });
  it("does not mutate state after a rejected physical command", async () => {
    const client = createSimulatorClient();
    await expect(
      client.execute({
        id: "bad",
        deviceId: "demo-esp32",
        action: "move_servo",
        arguments: { angle: 400 },
      }),
    ).rejects.toThrow(/angle/i);
    expect((await client.devices())[0]?.state.servoAngle).toBe(90);
  });
  it("does not expose mutable internal device references", async () => {
    const client = createSimulatorClient();
    const devices = await client.devices();
    if (devices[0]) devices[0].state.servoAngle = 0;
    expect((await client.devices())[0]?.state.servoAngle).toBe(90);
  });
  it("labels every simulated board and refuses real browser connections", async () => {
    const client = createSimulatorClient();
    expect((await client.devices()).every((device) => device.simulated)).toBe(true);
    await expect(client.connect({ kind: "serial", address: "/dev/ttyUSB0" })).rejects.toThrow(
      /desktop/i,
    );
  });
  it("executes an offline demonstration command without API credentials", async () => {
    const result = await createSimulatorClient().chat("Gira el servo a 45 grados");
    expect(result.executions).toHaveLength(1);
    expect(result.executions[0]?.state?.servoAngle).toBe(45);
  });
  it("rejects an unknown board rather than claiming success", async () => {
    await expect(
      createSimulatorClient().execute({
        id: "1",
        deviceId: "missing",
        action: "set_light",
        arguments: { enabled: true },
      }),
    ).rejects.toThrow(/device/i);
  });
});

it("preserves native connection failures without falling back to simulation", async () => {
  const invoke: NativeInvoke = async () => {
    throw new Error("Serial device disconnected");
  };
  const client = createNativeClient(invoke);
  await expect(client.connect({ kind: "serial", address: "/dev/ttyUSB0" })).rejects.toThrow(
    "Serial device disconnected",
  );
});
