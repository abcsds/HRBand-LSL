import questionary
from bleak import BleakScanner, BleakClient
import asyncio
from pylsl import StreamInfo, StreamOutlet

class BLEDeviceManager:
    HR_UUID = "00002a37-0000-1000-8000-00805f9b34fb"  # 0x180D

    def __init__(self):
        self.client = None
        self.device = None
        self.outlet_hr = None
        self.outlet_rr = None

    async def search_for_devices(self):
        scanner = BleakScanner()
        devices = await scanner.discover(timeout=10.0)
        return devices

    def filter_devices(self, devices):
        devices = [d for d in devices if d.name]
        return [d for d in devices if "-" not in d.name]

    def select_device(self, devices):
        if len(devices) == 0:
            return None
        devices = self.filter_devices(devices)
        action = questionary.select(
            "Select a Device",
            choices=[f"{d.name} ({d.address})" for d in devices]
        ).ask()

        device_address = action.split(" ")[-1][1:-1]
        device_idx = [d.address for d in devices].index(device_address)
        self.device = devices[device_idx]

    def search_device(self):
        devices = asyncio.run(self.search_for_devices())
        if len(devices) == 0:
            action = questionary.confirm("No devices found. Try again?").ask()
            while action:
                devices = asyncio.run(self.search_for_devices())
                if len(devices) > 0:
                    break
                action = questionary.confirm("No devices found. Try again?").ask()
        self.select_device(devices)
        print(f"Setting up streams for device: {self.device.name} ({self.device.address})")
        self.setup_lsl()
        return self.device

    def setup_lsl(self):
        info_hr = StreamInfo(name='HR ' + self.device.name,
                             type='Markers',
                             channel_count=1,
                             channel_format='int32',
                             source_id='HR_markers')
        info_rr = StreamInfo(name='RR ' + self.device.name,
                             type='Markers',
                             channel_count=1,
                             channel_format='int32',
                             source_id='RR_markers')
        self.outlet_hr = StreamOutlet(info_hr)
        self.outlet_rr = StreamOutlet(info_rr)

    def interpret(self, data):
        byte0 = data[0]
        res = {}
        res["hrv_uint8"] = (byte0 & 1) == 0
        sensor_contact = (byte0 >> 1) & 3
        if sensor_contact == 2:
            res["sensor_contact"] = "No contact detected"
        elif sensor_contact == 3:
            res["sensor_contact"] = "Contact detected"
        else:
            res["sensor_contact"] = "Sensor contact not supported"

        res["ee_status"] = ((byte0 >> 3) & 1) == 1
        res["rr_interval"] = ((byte0 >> 4) & 1) == 1
        if res["hrv_uint8"]:
            res["hr"] = data[1]
            i = 2
        else:
            res["hr"] = (data[2] << 8) | data[1]
            i = 3
        if res["ee_status"]:
            res["ee"] = (data[i + 1] << 8) | data[i]
            i += 2
        if res["rr_interval"]:
            res["rr"] = []
            while i < len(data):
                res["rr"].append((data[i + 1] << 8) | data[i])
                i += 2
        return res

    def callback(self, sender: int, data: bytearray):
        if data:
            data = self.interpret(data)
            print(f"HR: {data['hr']}")
            self.outlet_hr.push_sample([data["hr"]])
            if "rr" in data.keys():
                for data_rr in data["rr"]:
                    print(f"    RR: {data_rr}")
                    self.outlet_rr.push_sample([data_rr])
            return data
        else:
            return None

    async def main(self, address):
        self.client = BleakClient(address)
        try:
            await self.client.connect()
            hr_data = await self.client.start_notify(self.HR_UUID, self.callback)
            while True:
                await asyncio.sleep(0.1)
        except Exception as e:
            print(f"Exception: {e}")
        except KeyboardInterrupt:
            print(f"Stopped by user")
            await self.client.stop_notify(self.HR_UUID)
        finally:
            await self.client.disconnect()

if __name__ == "__main__":
    manager = BLEDeviceManager()
    print("Starting BLE device discovery...")
    device = manager.search_device()
    print(f"Connecting to device: {device.name} ({device.address}) ...")
    asyncio.run(manager.main(device.address))