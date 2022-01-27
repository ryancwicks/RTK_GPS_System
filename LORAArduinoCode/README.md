# LORA communication

These arduiono programs are built around the [Sparkfun LoRa Things Plus Explorable](https://www.sparkfun.com/products/17506). This board contains an SX1262 chip.

The example code was taken from [here](https://learn.sparkfun.com/tutorials/sparkfun-explorable-hookup-guide/peer-to-peer-example) and modified.

To program the boards, you must install the Sparkfun Apollo3 board definitions. You have to add:

https://raw.githubusercontent.com/sparkfun/Arduino_Apollo3/master/package_sparkfun_apollo3_index.json

To the board definitions under the arduino preferences to get these boards.

You also need to install the RadioLib and ArduinoBLE libraries.

Switch to the appropriate port and the "LoRa Things Plus Explorable" to program the transmitter and reciever.

## Initial Testing

I was able to send data at 8.9kbps a distance of 1m with the following settings: 

```
int state = radio.begin(915.0, 500.0, 7, 5, 0x34, 20, 10, 0, false);
```

No idea about range yet.

