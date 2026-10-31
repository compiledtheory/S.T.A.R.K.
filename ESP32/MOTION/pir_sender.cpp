#include <Arduino.h>
#include <WiFi.h>
#include <esp_now.h>
#include <esp_wifi.h>

#define PIR_PIN 4
#define LED_PIN 8
#define ROOM "BEDROOM"
#define PEER_CHANNEL 11
#define PACKET_TYPE 2

uint8_t broadcastAddress[] = {0x9C, 0x9E, 0x6E, 0xE2, 0xDE, 0x1C};

typedef struct {
  uint8_t packetType;
  int ID;
  char location[32];
} MotionPacket;

MotionPacket motionData;
esp_now_peer_info_t peerInfo;
volatile bool packetSent = false;

esp_now_send_status_t lastStatus = ESP_NOW_SEND_FAIL;

void onDataSend(const uint8_t *mac_addr, esp_now_send_status_t status) {
  lastStatus = status; packetSent = true;
}

bool sendWithRetry(const MotionPacket &p, int attempts = 4) {
  for (int i = 0; i < attempts; i++) {
    packetSent = false;
    esp_err_t err = esp_now_send(broadcastAddress, (uint8_t *) &p, sizeof(p));
    if (err == ESP_OK) {
      uint32_t start = millis();
      while (!packetSent && millis() - start < 50) delay(1);
      if (
        packetSent && lastStatus == ESP_NOW_SEND_SUCCESS) return true;
    }
    Serial.printf("send attempt %d failed err=%d, status=%d, done=%d\n", i+1, err, lastStatus, packetSent);
    delay(10);
  }
  return false;
}

void setup() {
  Serial.begin(115200);
  
  pinMode(PIR_PIN, INPUT);
  pinMode(LED_PIN, OUTPUT);
  digitalWrite(LED_PIN, LOW);
  delay(100);

  WiFi.mode(WIFI_MODE_STA);
  WiFi.setTxPower(WIFI_POWER_7dBm);
  WiFi.disconnect();

  esp_wifi_set_promiscuous(true);
  esp_wifi_set_channel(PEER_CHANNEL, WIFI_SECOND_CHAN_NONE);
  esp_wifi_set_promiscuous(false);

  if (esp_now_init() != ESP_OK) return;
  esp_now_register_send_cb(onDataSend);

  memcpy(peerInfo.peer_addr, broadcastAddress, 6);
  peerInfo.channel = PEER_CHANNEL;
  peerInfo.encrypt = false;

  if (esp_now_add_peer(&peerInfo) != ESP_OK) {return;}

  digitalWrite(LED_PIN, HIGH);

  motionData.packetType = PACKET_TYPE;
  motionData.ID = 1;
  strcpy(motionData.location, ROOM);
}

void loop() {
  if (digitalRead(PIR_PIN) == HIGH) {
    Serial.println("Motion!");
    
    packetSent = false;
    digitalWrite(LED_PIN, LOW);
    delay(200);
    bool ok = sendWithRetry(motionData);
    digitalWrite(LED_PIN, HIGH);
    
    int timeout = 0;
    while (!packetSent && timeout < 500) {
      delay(1);
      timeout++;
    }
  }
}