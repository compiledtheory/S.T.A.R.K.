#include <Arduino.h>

#define PACKET_TYPE 3
#define RADAR_RX_PIN 4
#define RADAR_TX_PIN 3

HardwareSerial Radar(1);
uint8_t headerBytes[] = {0xAA, 0xFF, 0x03, 0x00};
int headerIndex = 0;

// LD2450 sends: 
// HEADER: AA FF 03 00 
// TARGET: 0E 03 -X B1 86 -Y 10 00 -speed 40 01 -resolution
// TARGET: 00 00 00 00 00 00 00 00
// TARGET: 00 00 00 00 00 00 00 00 
// TAIL: 55 CC

#define X 0
#define Y 2
#define SPEED 4
#define RESOLUTION 6

#define MINIMUM 300
#define ROOM_DEPTH 4000
#define ROOM_WIDTH 2250

struct Target {
  int16_t x;
  int16_t y;
  int16_t speed;
  uint16_t resolution;
  float distance;
  float angle;
};

// low + high * 256 - 2^15
int16_t decodeValue(uint8_t lowByte, uint8_t highByte) {
  uint16_t raw = lowByte | (highByte << 8);
  return (raw & 0x8000) ? -(raw & 0x7FFF) : raw;
}

void setup() {
  Serial.begin(115200);
  delay(1000);

  Serial.println("Initializing...");
  Radar.begin(256000, SERIAL_8N1, RADAR_RX_PIN, RADAR_TX_PIN);
}

void loop() {
  while (Radar.available()) {
    uint8_t byteIn = Radar.read();

    if (headerIndex < 4) {
      if (byteIn == headerBytes[headerIndex]) {
        headerIndex++;
      } else {
        headerIndex = (byteIn == headerBytes[0]) ? 1 : 0; // maybe start of new header ????
      }
      continue;
    }

    uint8_t payload[26];
    payload[0] = byteIn;
    size_t readCount = Radar.readBytes(&payload[1], 25);

    if (!(readCount == 25 && payload[24] == 0x55 && payload[25] == 0xCC)) { 
      headerIndex = 0;
      continue; 
    }

    Target targets[3];

    for (int i = 0; i <3; i++) {
      int offset = i * 8;

      targets[i].x = decodeValue(payload[offset + X], payload[offset + X + 1]);
      targets[i].y = decodeValue(payload[offset + Y], payload[offset + Y + 1]);

      if (targets[i].y == 0 && targets[i].x == 0) {
        continue;
      }

      targets[i].speed = decodeValue(payload[offset + SPEED], payload[offset + SPEED + 1]);

      targets[i].resolution = payload[offset + RESOLUTION] | (payload[offset + RESOLUTION + 1] << 8); // unsigned
      targets[i].distance = sqrt((float)targets[i].x * targets[i].x + (float)targets[i].y * targets[i].y); // pitagorasik

      if (targets[i].distance > 6000 || targets[i].distance < 500) {
        continue;
      }

      targets[i].angle = atan2((float)targets[i].x, (float)targets[i].y) * 180.0 / PI;

      Serial.printf("[Target %d] X: %5d mm | Y: %5d mm | Dist: %4.0f mm | Ang: %3.0f° | Spd: %3d cm/s\n",
                      i + 1,
                      targets[i].x,
                      targets[i].y,
                      targets[i].distance,
                      targets[i].angle,
                      targets[i].speed);

    }

    headerIndex = 0;
  } 
}

