# RTK_GPS_System

"Cheap" RTK GPS system using Raspberry Pi's and uBlox ZED-F9P's

## Raspberry Pi Operating System Setup

Use the Raspberry Pi installer to set up the pi user. I used the 64 bit Lite install of Raspbian. Ensure that SSH access is set up. I also modified the network configuration on the SD card to use a static IP for the ethernet adapter.

## PPS Time Synchronization Setup

### Raspberry Pi Setup

Most of this is already handled by ansible. 

## Setting up the program to start automatically.

Edit the appropriate .service under the gps_control/startup_scripts and add the appropriate environment variables.

Copy the appropriate .service loop script into the /etc/systemd/system directory.

```
sudo systemctl enable <loop service>
suod systemctl start <loop service>
```


