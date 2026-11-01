#include <Arduino.h>
#include <WiFi.h>
#include <esp_now.h>
#include <esp_wifi.h>

#define PIR_PIN 4
#define LED_PIN 8
#define NODE_ID 1
#define PACKET_TYPE 2
#define PEER_CHANNEL 11
#define MOTION_INTERVAL_MS 3000

uint8_t broadcastAddress[] = {0x9C, 0x9E, 0x6E, 0xE2, 0xDE, 0x1C};

esp_now_peer_info_t peerInfo;
volatile bool packetSent = false;

esp_now_send_status_t lastStatus = ESP_NOW_SEND_FAIL;

void onDataSend(const uint8_t *mac_addr, esp_now_send_status_t status) {
  lastStatus = status; packetSent = true;
}

bool sendWithRetry(const uint8_t* data, size_t len, int attempts = 4) {
  for (int i = 0; i < attempts; i++) {
    packetSent = false;
    esp_err_t err = esp_now_send(broadcastAddress, data, len);
    if (err == ESP_OK) {
      uint32_t start = millis();
      while (!packetSent && millis() - start < 50) delay(1);
      if (
        packetSent && lastStatus == ESP_NOW_SEND_SUCCESS) return true;
    }
    delay(10);
  }
  return false;
}

bool sendMotion() {
  uint8_t pkt[2] = {PACKET_TYPE, NODE_ID};
  return sendWithRetry(pkt, sizeof(pkt));
}

void startSleep() {
  while (true) {
    digitalWrite(LED_PIN, !digitalRead(PIR_PIN));
    delay(50);
  }
  esp_deep_sleep_enable_gpio_wakeup(1ULL << PIR_PIN, ESP_GPIO_WAKEUP_GPIO_HIGH);
  esp_deep_sleep_start();
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

  if (esp_now_init() != ESP_OK) startSleep();
  esp_now_register_send_cb(onDataSend);

  memcpy(peerInfo.peer_addr, broadcastAddress, 6);
  peerInfo.channel = PEER_CHANNEL;
  peerInfo.encrypt = false;

  if (esp_now_add_peer(&peerInfo) != ESP_OK) startSleep();

  if (esp_sleep_get_wakeup_cause() == ESP_SLEEP_WAKEUP_GPIO) {
    for (int i = 0; i < 2; i++) {
        digitalWrite(LED_PIN, HIGH);
        delay(300);
        digitalWrite(LED_PIN, LOW);
        delay(300);
    }
    sendMotion();
    uint32_t last = millis();
    while (digitalRead(PIR_PIN) == HIGH) {
      if (millis() - last >= MOTION_INTERVAL_MS) {
        sendMotion();
        last = millis();
      }
      delay(10);
    }
  }
  startSleep();
}

void loop() {}