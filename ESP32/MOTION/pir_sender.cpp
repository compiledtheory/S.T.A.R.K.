#include <Arduino.h>
#include <WiFi.h>
#include <esp_now.h>
#include <esp_wifi.h>

#define PIR_PIN 4
#define ROOM "BEDROOM"
#define PEER_CHANNEL 11

uint8_t broadcastAddress[] = {0x9C, 0x9E, 0x6E, 0xE2, 0xDE, 0x1C};

typedef struct pir_sensor_detection {
  char room[32];
} pir_sensor_detection;

pir_sensor_detection sensorData;
esp_now_peer_info_t peerInfo;

volatile bool packetSent = false;
bool motionHandled = false;

void onDataSend(const uint8_t *mac_addr, esp_now_send_status_t status) {
  packetSent = true;
}

void setup() {
  Serial.begin(115200);
  delay(500);

  pinMode(PIR_PIN, INPUT);
  WiFi.mode(WIFI_MODE_STA);
  WiFi.disconnect();

  esp_wifi_set_promiscuous(true);
  esp_wifi_set_channel(PEER_CHANNEL, WIFI_SECOND_CHAN_NONE);
  esp_wifi_set_promiscuous(false);

  if (esp_now_init() != ESP_OK) return;
  if (esp_now_add_peer(&peerInfo) != ESP_OK) {
    Serial.println("Failed to add peer");
    return;
  }
  esp_now_register_send_cb(onDataSend);
  memcpy(peerInfo.peer_addr, broadcastAddress, 6);
  peerInfo.channel = PEER_CHANNEL;
  peerInfo.encrypt = false;

  Serial.println("Calibration started");
  delay(20000); 
  Serial.println("Finished");
}

void loop() {
  int motion = digitalRead(PIR_PIN);
  
  if (motion == HIGH && !motionHandled) {
      
    Serial.println("Motion detected");
      strcpy(sensorData.room, ROOM);
      packetSent = false;

      esp_now_send(broadcastAddress, (uint8_t *)&sensorData, sizeof(sensorData));

      int timeout = 0;
      while (!packetSent && timeout < 500) {
        delay(1);
        timeout++;
      }

      motionHandled = true;
    }

  if (motion == LOW && motionHandled) { motionHandled = false; }

  delay(20);
}