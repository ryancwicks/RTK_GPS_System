# RTK_GPS_System

"Cheap" RTK GPS system using Raspberry Pi's and uBlox ZED-F9P's

## Raspberry Pi Operating System Setup

Use the Raspberry Pi installer to set up the pi user. I used the 64 bit Lite install of Raspbian. Ensure that SSH access is set up. I also modified the network configuration on the SD card to use a static IP for the ethernet adapter.

## PPS Time Synchronization Setup

### Raspberry Pi Setup

Connect the PPS pin to GPIO4/Pin 7 on the Raspberry Pi 40 pin header, and the ground pin to pin 9. 
