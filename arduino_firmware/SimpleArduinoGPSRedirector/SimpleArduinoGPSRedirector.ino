#include <SoftwareSerial.h>

SoftwareSerial mySerial(13, 12); // RX, TX

void setup() {
  // put your setup code here, to run once:
  mySerial.begin(57600);
  Serial.begin(57600);
}

void loop() {
  // put your main code here, to run repeatedly:
  if (mySerial.available())
    Serial.write(mySerial.read());
  if (Serial.available())
    mySerial.write(Serial.read());
}
