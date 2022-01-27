# Notes on using and setting up U-Blox GPS's with ubxtool

A lot of this was pulled from [here](https://gpsd.io/ubxtool-examples.html).

First, plug in the GPS and start the gps_server (or GPSD if running it directly).

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
export UBXOPTS="-P 27"
```

For my version of the U-Blox chip (ZED-F9P), the protocol version is 27.

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

You can check which platform you are on with (-p is poll):

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

