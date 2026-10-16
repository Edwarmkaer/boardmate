# Board protocol v0.1

Connect first, then send a description request. The board replies with the same request ID and its identity, capabilities and initial state:

```json
{"id":"describe-1","action":"describe"}
```

```json
{"requestId":"describe-1","device":{"id":"board-001","name":"Mi placa","board":"ESP32","transport":"serial","simulated":false,"capabilities":["move_servo","set_light"],"state":{"lightOn":false}}}
```

IDs must be stable and unique, at most 64 bytes, and must not start with reserved `demo-`. Names must be nonempty and at most 80 bytes. The host owns the selected transport label. Do not advertise capabilities that your firmware cannot execute. Omit unknown initial state fields rather than inventing readings.

Commands and replies:

```json
{"id":"command-1","deviceId":"board-001","action":"move_servo","arguments":{"angle":45}}
```

```json
{"requestId":"command-1","deviceId":"board-001","ok":true,"state":{"servoAngle":45}}
```

Failures use `ok:false` and an `error` string. Reply request/device IDs must match. A partial state update is merged into the last known state; it is not a complete telemetry stream.

| Action | Arguments | Updated state |
| --- | --- | --- |
| `move_servo` | `angle`: integer 0–180 | `servoAngle` |
| `set_light` | `enabled`: boolean | `lightOn` |
| `read_temperature` | empty object | `temperatureC` |

Request IDs must be nonempty and at most 128 bytes. Validate again on the board before actuation. A dropped connection or timeout can leave the physical outcome uncertain; do not repeat actuator commands automatically.

## Framing

Serial and RFCOMM send compact UTF-8 JSON followed by LF. Replies may end in LF or CRLF. The host bounds frames at 64 KiB. WebSocket uses one JSON object per text message; a trailing newline is optional. No MQTT broker is required.

BLE boards must advertise Nordic UART Service `6e400001-b5a3-f393-e0a9-e50e24dcca9e`, expose writable RX `6e400002-b5a3-f393-e0a9-e50e24dcca9e` and notifying TX `6e400003-b5a3-f393-e0a9-e50e24dcca9e`. The host sends 20-byte chunks and assembles notifications until LF. A generic BLE sensor using another GATT profile requires an adapter; matching BLE radio support alone is insufficient.

For Classic, pair the board using your OS and provide its RFCOMM channel. ESP32 variants differ: check that your chip supports Classic before choosing this transport. A Pico without a radio needs external hardware for wireless operation.

## MicroPython serial reference

`firmware/micropython/main.py` and `protocol.py` implement the serial side for ESP32/Pico-class boards running a compatible MicroPython build. This example has not been flashed during repository bootstrap.

1. Install MicroPython for your exact board.
2. Edit the pin constants and board name in `main.py`. `SERVO_PIN=18` and `LED_PIN=2` are examples, not automatic pin detection. Original Pico onboard LED usually uses GPIO25; Pico W uses its board-specific `LED` alias.
3. Copy both files to the board with your normal MicroPython tool, then restart it.
4. Close serial monitors/REPL tools. In boardmate, connect its serial port at 115200 baud.
5. Use direct controls first, then try an assistant request.

The servo starts with PWM duty zero and no reported angle. Match pulse width limits to your servo and supply it from a suitable power source with a shared ground. The LED output is a GPIO signal; switching a mains lamp requires an appropriately isolated external controller.

Temperature is disabled by default. Set `TMP36_PIN` only when an external TMP36 analog sensor is wired. The example converts a 3.3 V ADC reading with the TMP36 transfer function; calibrate the ADC/reference for your board. It does not report MCU die temperature as room temperature. For another sensor, replace `Driver.read_temperature` and advertise the capability only when available.

The reference bounds input lines at 1,024 characters and caches 16 command replies to avoid repeated actuation after duplicate IDs. Successful writes acknowledge the driver call, not mechanical position feedback. Wi-Fi, BLE and Classic firmware are not included in this minimal example.
