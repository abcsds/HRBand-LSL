import questionary
from bleak import BleakScanner, BleakClient
import asyncio
from pylsl import StreamInfo, StreamOutlet


async def search_for_devices():
    scanner = BleakScanner()
    devices = await scanner.discover(timeout=10.0)
    return devices

def search_device():
    devices = asyncio.run(search_for_devices())
    if len(devices) == 0:
        action = questionary.confirm("No devices found. Try again?").ask()
        while action:
            devices = asyncio.run(search_for_devices())
            if len(devices) > 0:
                break
            action = questionary.confirm("No devices found. Try again?").ask()
    device = select_device(devices)
    return device

def select_device(devices):
    if len(devices) == 0:
        return None
    devices = [d for d in devices if "-" not in d.name]
    action = questionary.select(
        "Select a Device",
        choices=[f"{d.name} ({d.address})" for d in devices]
    ).ask()

    device_address = action.split(" ")[-1][1:-1]
    device_idx = [d.address for d in devices].index(device_address)
    return devices[device_idx]

def callback(sender: int, data: bytearray):
    if data:
        data = interpret(data)
        print(f"HR: {data['hr']}")
        if "rr" in data.keys():
            for data_rr in data["rr"]:
                print(f"    RR: {data_rr}")
        return data
    else:
        return None

def interpret(data):
    """
    from fg1/BLEHeartRateLogger
    data is a list of integers corresponding to readings from the BLE HR monitor
    """
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
            # Note: Need to divide the value by 1024 to get in seconds
            res["rr"].append((data[i + 1] << 8) | data[i])
            i += 2
    return res

def setup_lsl(device_name):
    # TODO: HR? RR? NN? Can we select?
    info_hr = StreamInfo(name='HR '+device_name,
        type='Markers',
        channel_count=1,
        channel_format='int32',
        source_id='HR_markers'
    )
    info_rr = StreamInfo(name='RR '+device_name,
        type='Markers',
        channel_count=1,
        channel_format='int32',
        source_id='RR_markers'
    )
    outlet_hr = StreamOutlet(info_hr)
    outlet_rr = StreamOutlet(info_rr)
    return outlet_hr, outlet_rr

async def main(address):
    client = BleakClient(address)
    try:
        await client.connect()
        hr_data = await client.start_notify(HR_UUID, callback)
        while True:
            await asyncio.sleep(0.1)
    except Exception as e:
        print(f"Exeption: {e}")
    except KeyboardInterrupt:
        print(f"Stopped by user")
        await client.stop_notify(HR_UUID)
    finally:
        await client.disconnect()

if __name__ == "__main__":
    print("Starting BLE device discovery...")
    device = search_device()
    print(f"Setting up streams for device: {device.name} ({device.address})")
    outlet_hr, outlet_rr = setup_lsl(device.name)
    print(f"Connecting to device: {device.name} ({device.address}) ...")
    HR_UUID = "00002a37-0000-1000-8000-00805f9b34fb"
    asyncio.run(main(device.address))
    

