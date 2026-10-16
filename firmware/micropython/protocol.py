"""Dependency-free protocol shared by the MicroPython example and host tests."""


class Board:
    def __init__(self, device_id, board, driver, temperature=False):
        self.driver = driver
        self.device = {
            "id": device_id, "name": "Mi placa", "board": board,
            "transport": "serial", "simulated": False,
            "capabilities": ["move_servo", "set_light"], "state": {},
        }
        if temperature:
            self.device["capabilities"].append("read_temperature")
        self.recent = []

    def handle(self, request):
        request_id = request.get("id", "") if isinstance(request, dict) else ""
        reply = {"requestId": request_id, "deviceId": self.device["id"], "ok": False}
        try:
            if not isinstance(request, dict):
                raise ValueError("Request must be an object")
            if not isinstance(request_id, str) or not request_id.strip() or len(request_id) > 128:
                raise ValueError("Invalid request id")
            if request.get("action") == "describe":
                return {"requestId": request_id, "device": self.device}
            for previous, previous_reply in self.recent:
                if previous["id"] == request_id:
                    if previous != request:
                        raise ValueError("Request id already used with different command")
                    return previous_reply
            if request.get("deviceId") != self.device["id"]:
                raise ValueError("Command addresses another device")
            action = request.get("action")
            if action not in self.device["capabilities"]:
                raise ValueError("Unsupported device capability")
            arguments = request.get("arguments")
            if not isinstance(arguments, dict):
                raise ValueError("Invalid arguments")
            state = self.device["state"]
            if action == "move_servo":
                angle = arguments.get("angle")
                if type(angle) is not int or not 0 <= angle <= 180:
                    raise ValueError("Servo angle must be an integer between 0 and 180")
                self.driver.move_servo(angle)
                state["servoAngle"] = angle
            elif action == "set_light":
                enabled = arguments.get("enabled")
                if type(enabled) is not bool:
                    raise ValueError("Light enabled must be a boolean")
                self.driver.set_light(enabled)
                state["lightOn"] = enabled
            else:
                state["temperatureC"] = self.driver.read_temperature()
            reply["ok"] = True
            reply["state"] = dict(state)
        except Exception as error:
            reply["error"] = str(error)
        if isinstance(request, dict) and request_id:
            # Cache failures too: an uncertain driver failure must not actuate twice.
            self.recent.append((request, reply))
            if len(self.recent) > 16:
                self.recent.pop(0)
        return reply
