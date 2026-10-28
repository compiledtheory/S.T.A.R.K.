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
bool motionHandled = false;

void onDataSend(const uint8_t *mac_addr, esp_now_send_status_t status) {
  packetSent = true;
}

void setup() {
  Serial.begin(115200);
  
  pinMode(PIR_PIN, INPUT);
  pinMode(LED_PIN, OUTPUT);
  digitalWrite(LED_PIN, LOW);
  delay(100);

  WiFi.mode(WIFI_MODE_STA);
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
  esp_err_t result = esp_now_send(broadcastAddress, (uint8_t*) &motionData, sizeof(motionData));
}

void loop() {
  int motion = digitalRead(PIR_PIN);

  if (motion == HIGH && !motionHandled) {
    Serial.println("Motion!");
    
    packetSent = false;
    esp_err_t result = esp_now_send(broadcastAddress, (uint8_t*) &motionData, sizeof(motionData));
    
    int timeout = 0;
    while (!packetSent && timeout < 500) {
      delay(1);
      timeout++;
    }
    motionHandled = true;
  }

  if (motion == LOW && motionHandled) { motionHandled = false; }
}