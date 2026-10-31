#include <Arduino.h>
#include <WiFi.h>
#include <esp_now.h>
#include <esp_wifi.h>
#include <vector>

#define PACKET_TYPE 3
#define LED_PIN 8
#define RADAR_RX_PIN 4
#define RADAR_TX_PIN 3
#define PEER_CHANNEL 11

HardwareSerial Radar(1);
uint8_t headerBytes[] = {0xAA, 0xFF, 0x03, 0x00};
uint8_t broadcastAddress[] = {0x9C, 0x9E, 0x6E, 0xE2, 0xDE, 0x1C};
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

struct Target {
  uint8_t PacketType;
  int ID;
  int16_t x;
  int16_t y;
  int16_t speed;
  float distance;
};

// low + high * 256 - 2^15
int16_t decodeValue(uint8_t lowByte, uint8_t highByte) {
  uint16_t raw = lowByte | (highByte << 8);
  return (raw & 0x8000) ? (raw & 0x7FFF) : -(int16_t)raw;
}

void onDataSend(const uint8_t* mac_addr, esp_now_send_status_t status) {
  return;
}

Target makeTarget(uint8_t* payload) {
  Target target;
  target.PacketType = PACKET_TYPE;
  target.ID = 1;
  target.x = decodeValue(payload[X], payload[X + 1]);
  target.y = decodeValue(payload[Y], payload[Y + 1]);
  target.distance = sqrt((float) target.x*target.x + (float)target.y*target.y); // pitagorasik;
  target.speed = decodeValue(payload[SPEED], payload[SPEED + 1]);
  return target;
}


void setup() {
  Serial.begin(115200);
  delay(1000);
  pinMode(LED_PIN, OUTPUT);
  digitalWrite(LED_PIN, LOW);

  WiFi.mode(WIFI_MODE_STA);
  WiFi.setTxPower(WIFI_POWER_7dBm); // this seems to fix issue with packets not sending --__(-_-)__--
  WiFi.disconnect();

  esp_wifi_set_promiscuous(true);
  esp_wifi_set_channel(PEER_CHANNEL, WIFI_SECOND_CHAN_NONE);
  esp_wifi_set_promiscuous(false);

  if (esp_now_init()!= ESP_OK) return;
  esp_now_register_send_cb(onDataSend);

  Serial.println("Initializing...");
  Radar.begin(256000, SERIAL_8N1, RADAR_RX_PIN, RADAR_TX_PIN);
  digitalWrite(LED_PIN, HIGH);
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

    std::vector<Target> targets;

    for (int i = 0; i <3; i++) {
      int offset = i * 8;

      Target target = makeTarget(&payload[offset]); // send data starting at offset memory address

      if (target.y == 0 && target.x == 0) continue;
      if (target.distance > 6000 || target.distance < 500) continue;

      targets.push_back(target);

      // not really needed currently
      // target.angle = atan2((float)target.x, (float)targets.y) * 180.0 / PI;
      // target.resolution = payload[RESOLUTION] | (payload[RESOLUTION + 1] << 8); // unsigned

    }

    headerIndex = 0;
    if (targets.empty()) continue;

    uint8_t *data = (uint8_t*)targets.data();
    size_t dataSize = targets.size() * sizeof(Target);
    esp_err_t result = esp_now_send(broadcastAddress, data, dataSize);
    Serial.println(result == ESP_OK ? "success" : "fail");
  } 
}

