"""Edit pins for your board, then copy this file and protocol.py with mpremote."""
import json
import sys
from machine import ADC, PWM, Pin, unique_id
from ubinascii import hexlify
from protocol import Board

# Example GPIO numbers, not a board autodetection mechanism.
SERVO_PIN = 18
LED_PIN = 2  # Original Pico onboard LED uses 25; Pico W uses Pin("LED").
TMP36_PIN = None  # An external TMP36 analog sensor; None disables temperature.
BOARD_NAME = "ESP32"  # Change to "Raspberry Pi Pico" when appropriate.


class Driver:
    def __init__(self):
        self.servo = PWM(Pin(SERVO_PIN), freq=50, duty_u16=0)
        self.light = Pin(LED_PIN, Pin.OUT, value=0)
        self.sensor = ADC(Pin(TMP36_PIN)) if TMP36_PIN is not None else None
        # ESP32 ADC attenuation must suit your voltage range and board.
        if self.sensor is not None and hasattr(self.sensor, "atten"):
            self.sensor.atten(ADC.ATTN_11DB)

    def move_servo(self, angle):
        self.servo.duty_ns(500_000 + angle * 2_000_000 // 180)

    def set_light(self, enabled):
        self.light.value(enabled)

    def read_temperature(self):
        volts = self.sensor.read_u16() / 65535 * 3.3
        return round((volts - 0.5) * 100, 2)


board = Board("board-" + hexlify(unique_id()).decode(), BOARD_NAME, Driver(), TMP36_PIN is not None)
# Only protocol JSON goes to stdout. Keep REPL tools closed while boardmate uses serial.
while True:
    line = ""
    oversized = False
    while True:
        char = sys.stdin.read(1)
        if not char:
            continue
        if char == "\n":
            break
        if len(line) < 1024:
            line += char
        else:
            oversized = True
    try:
        if oversized:
            raise ValueError("Request frame too large")
        response = board.handle(json.loads(line))
    except Exception as error:
        response = {"requestId": "", "deviceId": board.device["id"], "ok": False, "error": str(error)}
    print(json.dumps(response))
