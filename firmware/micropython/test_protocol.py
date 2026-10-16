import unittest
from protocol import Board


class FakeDriver:
    def __init__(self):
        self.moves = []
        self.lights = []

    def move_servo(self, angle):
        self.moves.append(angle)

    def set_light(self, enabled):
        self.lights.append(enabled)

    def read_temperature(self):
        return 23.5


class ProtocolTest(unittest.TestCase):
    def setUp(self):
        self.driver = FakeDriver()
        self.board = Board("pico-test", "Pico", self.driver)

    def command(self, action, arguments):
        return {"id": "test-1", "deviceId": "pico-test", "action": action,
                "arguments": arguments}

    def test_describe_only_advertises_available_hardware(self):
        reply = self.board.handle({"id": "describe-1", "action": "describe"})
        self.assertEqual(reply["requestId"], "describe-1")
        self.assertNotIn("read_temperature", reply["device"]["capabilities"])
        self.assertNotIn("servoAngle", reply["device"]["state"])

    def test_invalid_angles_never_reach_servo(self):
        for angle in (-1, 181, 1.5, True, "45"):
            self.assertFalse(self.board.handle(self.command("move_servo", {"angle": angle}))["ok"])
        self.assertEqual(self.driver.moves, [])

    def test_repeated_request_is_not_executed_twice(self):
        command = self.command("move_servo", {"angle": 45})
        self.assertTrue(self.board.handle(command)["ok"])
        self.assertTrue(self.board.handle(command)["ok"])
        self.assertEqual(self.driver.moves, [45])
        changed = self.command("move_servo", {"angle": 90})
        self.assertFalse(self.board.handle(changed)["ok"])
        self.assertEqual(self.driver.moves, [45])

    def test_wrong_device_and_string_boolean_are_rejected(self):
        command = self.command("set_light", {"enabled": True})
        command["deviceId"] = "another"
        self.assertFalse(self.board.handle(command)["ok"])
        self.assertFalse(self.board.handle(self.command("set_light", {"enabled": "true"}))["ok"])
        self.assertEqual(self.driver.lights, [])

    def test_enabled_sensor_returns_actual_driver_reading(self):
        board = Board("pico-test", "Pico", self.driver, temperature=True)
        reply = board.handle(self.command("read_temperature", {}))
        self.assertEqual(reply["state"]["temperatureC"], 23.5)


if __name__ == "__main__":
    unittest.main()
