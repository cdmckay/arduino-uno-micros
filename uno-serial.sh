#!/usr/bin/env sh
set -e

SERIAL_PORT=$(ls /dev/tty.usb*  | head -1)
BAUD_RATE=57600

screen $SERIAL_PORT $BAUD_RATE
