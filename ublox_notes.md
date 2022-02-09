# Notes on using and setting up U-Blox GPS's with ubxtool

A lot of this was pulled from [here](https://gpsd.io/ubxtool-examples.html).

I had to build from scratch to get PPP reprocessing and gpsfake to run correctly. The current (Feb 2022) Ubuntu package is version 3.20, and has RINEX and gps pipe bugs. The latest 3.23.1 version works.

First, plug in the GPS and start the docker gps_server (or GPSD if running it directly).

You should be able to connect to the GPS with the gpsmon, cgps or xgps utility.

To get a list of supported message, run 

```
ubxtool -h -v 2
```

and compare this to what you need from the interfaces document. Generally, any of the pollable commands have the interface document UBX- dropped (UBX-CFG-USB -> CFG-USB)

The best way to set things with protocol 27 + is to use Configuration Items, rather than trying to poll and set registers directly. You use the ubxtool -g (get) and -x (set) with the configuration item to change the values. A list of configuration items is found on page 227 of the manual.

## Getting the protocol version

```
ubxtool -p MON-VER | grep PROTVER
```

This needs to be passed every time you call the ubx software through the -P flag, so add it to the environment variable UBXOPTS

```
export UBXOPTS="-P 27.30"
```

For my version of the U-Blox chip (ZED-F9P), the protocol version is 27.30.

## Resetting and setting it into binary mode

The following resets all the settings on the  reciever, and then disables NMEA (the default) with the -d flag and enable the UBX binary output (with the -e flag)

```
ubxtool -p RESET
ubxtool -e BINARY
ubxtool -d NMEA
```

## Setting high precision mode (for NMEA strings, when using RTK)

Check the current mode (-g is get):

```
ubxtool -g CFG-NMEA-HIGHPREC
```

To set the NMEA to high precision (-z is set):

```
ubxtool -z CFG-NMEA-HIGHPREC,1
```

## Setting the dynamic platform model

You can check which platform you are using:

```
ubxtool -g CFG-NAVSPG-DYNMODEL
```

The default mode is 0 (portable). You can get better results depending on what you are doing (see page 74 of the interfaces document for more details). We probably want to set it to mode 2 for the base station (stationary) and mode 5 for the boat (sea).

```
ubxtool -z CFG-NAVSPG-DYNMODEL,2
ubxtool -g CFG-NAVSPG-DYNMODEL
```

## Setting rates of messages

The configuration rates of a particular message can be set using it's appropriate configuration item. Every message (NMEA, Binary, RTCM, etc) can be set for every port (USB, UART1, UART2, I2C, etc) using an appropriate configuration item. Setting the rate to 0 turns off that message.

## Setting up the reciever in Time Mode/As an RTK base station

The integration manual covers this on page 19, but the base station has be to put into stationary mode, and the site surveyed in. You can set the position manually using a previous survey, or you can use survey-in mode (CFG-TMODE-MODE can be disabled, surveyed-in or fixed (preset)). There are appropriate registers for inputing the position.

## Capturing data for PPP reprocessing

To reprocess with using PPP, you need to capture RINEX files, which in turn require turning on RAWX messages from the device. The following examples shows how to do this for the USB port.

```
ubxtool -e RAWX
```

To do the above, make sure you have the UBXOPTS set to the correct version of the protocol.

```
ubxtool -g CFG-MSGOUT-UBX_RXM_RAWX_USB
ubxtool -z CFG-MSGOUT-UBX_RXM_RAWX_USB,1
```

The second command turns on raw data at 1 Hz output. You can leave all the consetellations on, but only GPS is used in the raw. 

To capture the RINEX data over 24 hours at 30s intervals, run the following (This will only collect data if the RAWX messages are on):

```
gpsrinex -i 30 -n 2880 -f today.obs
```

The data can be zipped and reprocessed [here](https://webapp.geod.nrcan.gc.ca/geod/tools-outils/ppp.php).

I had some trouble with this, so I re-ran it following the GPSD-PPP How To instructions exactly (although I did add GLONASS):

```
gpsctl -s 115200
ubxtool -d NMEA
ubxtool -e BINARY
ubxtool -e GLONASS
ubxtool -d BEIDOU
ubxtool -d GALILEO
ubxtool -d SBAS
ubxtool -e GPS
ubxtool -p CFG-GNSS
ubxtool -z CFG-MSGOUT-UBX_RXM_RAWX_USB,1
```

## Capturing RAW GPS data and reprocessing:

The following captures 4 hours of raw ublox GPS data. 

```
gpspipe -R -x 14400  > 4h-raw.ubx
```

You can re-run this with:

```
gpsfake -1 -P 3000 4h-raw.ubx
```

And than extract RINEX with:

```
gpsrinex -i 30 -n 1000000 :3000
```

## Plotting GPS data

gpsprof | gnuplot --persist
gpsprof -f polar | gnuplot --persist