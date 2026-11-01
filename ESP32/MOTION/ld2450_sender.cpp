#include <Arduino.h>
#include <WiFi.h>
#include <esp_now.h>
#include <esp_wifi.h>
#include <vector>

#define PACKET_TYPE 3
#define NODE_ID 4
#define LED_PIN 8
#define RADAR_RX_PIN 4
#define RADAR_TX_PIN 3
#define PEER_CHANNEL 11

HardwareSerial Radar(1);
uint8_t headerBytes[] = {0xAA, 0xFF, 0x03, 0x00};
uint8_t broadcastAddress[] = {0x9C, 0x9E, 0x6E, 0xE2, 0xDE, 0x1C};
int headerIndex = 0;

uint32_t lastPush = 0;
bool hadTargets = false;

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

// low + high * 256 - 2^15
int16_t decodeValue(uint8_t lowByte, uint8_t highByte) {
  uint16_t raw = lowByte | (highByte << 8);
  return (raw & 0x8000) ? (raw & 0x7FFF) : -(uint16_t)raw;
}

void onDataSend(const uint8_t* mac_addr, esp_now_send_status_t status) {
  return;
}

static void putI16(uint8_t* payload, int16_t value) {
  payload[0] = value & 0xFF;
  payload[1] = (value >> 8) & 0xFF;
}

esp_now_peer_info_t peerInfo;

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

  memcpy(peerInfo.peer_addr, broadcastAddress, 6);
  peerInfo.channel = PEER_CHANNEL;
  peerInfo.encrypt = false;

  if (esp_now_add_peer(&peerInfo) != ESP_OK) return;

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

    uint8_t LD2450Payload[26];
    LD2450Payload[0] = byteIn;
    size_t readCount = Radar.readBytes(&LD2450Payload[1], 25);

    if (!(readCount == 25 && LD2450Payload[24] == 0x55 && LD2450Payload[25] == 0xCC)) { 
      headerIndex = 0;
      continue; 
    }

    uint8_t payloadOut[3 + 3 * 6]; // 3 bytes header, max 3 targets: 2 bytes x, 2 bytes y, 2 bytes speed
    payloadOut[0] = PACKET_TYPE;
    payloadOut[1] = NODE_ID;

    uint8_t count = 0;

    for (int i = 0; i <3; i++) {
      int offset = i * 8;

      uint8_t* target = &LD2450Payload[offset]; // starting at offset memory address
      int16_t x = decodeValue(target[X], target[X+1]);
      int16_t y = decodeValue(target[Y], target[Y+1]);
      
      if (x == 0 && y == 0) { continue; }

      float distance = sqrtf((float)x * x + (float)y*y);
      if (distance < 500 || distance > 6000) { continue; }

      uint8_t *out = &payloadOut[3 + count * 6];
      putI16(out, x);
      putI16(out + Y, y);
      putI16(out + SPEED, decodeValue(target[SPEED], target[SPEED+1]));
      count++;

    }

    payloadOut[2] = count;
    size_t len = 3 + count * 6;
    
    // if count > 0 and 500 millis since last push -> send. If not send empty heartbeat every 5s;
    bool shouldSend = count ? (millis() - lastPush >= 500) : (hadTargets || millis() - lastPush >= 5000);

    if (shouldSend) {
      esp_now_send(broadcastAddress, payloadOut, len);
      lastPush = millis();
    }

    hadTargets = count > 0;
    headerIndex = 0;
  } 
}

