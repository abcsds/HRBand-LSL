# HRBand-LSL

Connect BLE Heart Rate bands to Lab Streaming Layer (LSL) using Python. It works with any BLE device that supports the Heart Rate service.

## Requirements

A bluetooth dongle is required to connect to the BLE device. Requires Python 3 and three libraries: `questionary`, `pylsl` and `bleak`. Install the required libraries using the following command:

```bash
pip install -r requirements.txt
```

## Usage

```bash
python main.py
```
Select your device from the list and press enter. Two LSL string marker streams will be created, one for the heart rate and another for the RR intervals.

## Future Work

Polar Sense device requires modifications. LSL streams should be updated accordingly.