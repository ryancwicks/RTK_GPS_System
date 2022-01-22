#!/bin/sh

stty -F /dev/ttyACM0 speed 57600 
gpsd -Nn /dev/ttyACM0 -F /var/run/gpsd.sock