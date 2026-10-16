import { describe, expect, it } from "vitest";
import { type Device, type DeviceCommand, validateCommand } from "./index";

const servo: Device = {
  id: "bench",
  name: "Bench",
  board: "ESP32",
  transport: "simulator",
  simulated: true,
  capabilities: ["move_servo", "set_light"],
  state: { servoAngle: 90, lightOn: false },
};
const command: DeviceCommand = {
  id: "request-1",
  deviceId: "bench",
  action: "move_servo",
  arguments: { angle: 45 },
};

describe("device command boundary", () => {
  it("accepts a supported command with a valid servo angle", () => {
    expect(() => validateCommand(command, servo)).not.toThrow();
  });
  it.each([-1, 181, 45.5, Number.NaN, Number.POSITIVE_INFINITY, "45"])(
    "rejects invalid servo angle %s before hardware execution",
    (angle) => {
      expect(() => validateCommand({ ...command, arguments: { angle } }, servo)).toThrow(/angle/i);
    },
  );
  it("rejects actions the connected board does not advertise", () => {
    expect(() =>
      validateCommand({ ...command, action: "read_temperature", arguments: {} }, servo),
    ).toThrow(/capability/i);
  });
  it("rejects a command addressed to a different device", () => {
    expect(() => validateCommand({ ...command, deviceId: "other" }, servo)).toThrow(/device/i);
  });
  it("rejects string booleans for physical light controls", () => {
    expect(() =>
      validateCommand({ ...command, action: "set_light", arguments: { enabled: "false" } }, servo),
    ).toThrow(/enabled/i);
  });
});
