#include <Arduino.h>
#include <WiFi.h>
#include <esp_now.h>
#include <esp_wifi.h>
#include <esp_sleep.h>
#include <driver/gpio.h>

#define PIR_PIN 4
#define PIR_PIN_GPIO GPIO_NUM_4
#define LED_PIN 8
#define NODE_ID 1
#define PACKET_TYPE 2
#define PEER_CHANNEL 11
#define MOTION_INTERVAL_MS 3000

uint8_t broadcastAddress[] = {0x80, 0x53, 0xE0, 0xB4, 0xE1, 0x28};

esp_now_peer_info_t peerInfo;
volatile bool packetSent = false;
esp_now_send_status_t lastStatus = ESP_NOW_SEND_FAIL;

void onDataSend(const uint8_t *mac_addr, esp_now_send_status_t status) {
  lastStatus = status;
  packetSent = true;
}

bool sendWithRetry(const uint8_t* data, size_t len, int attempts = 4) {
  for (int i = 0; i < attempts; i++) {
    packetSent = false;
    esp_err_t err = esp_now_send(broadcastAddress, data, len);
    if (err == ESP_OK) {
      uint32_t start = millis();
      while (!packetSent && millis() - start < 50) delay(1);
      if (packetSent && lastStatus == ESP_NOW_SEND_SUCCESS) return true;
    }
    delay(10);
  }
  return false;
}

bool sendMotion() {
  uint8_t pkt[2] = {PACKET_TYPE, NODE_ID};
  bool success = sendWithRetry(pkt, sizeof(pkt));

  digitalWrite(LED_PIN, LOW);
  delay(100);
  digitalWrite(LED_PIN, HIGH);

  return success;
}

void startSleep() {
  while (digitalRead(PIR_PIN) == HIGH) delay(20);
  esp_wifi_stop();
  gpio_pullup_dis(PIR_PIN_GPIO);
  gpio_pulldown_dis(PIR_PIN_GPIO); // this was the fix thx gemini i spend all day debugging WHY its not waking up
  esp_deep_sleep_enable_gpio_wakeup(1ULL << PIR_PIN_GPIO, ESP_GPIO_WAKEUP_GPIO_HIGH);
  gpio_hold_en(PIR_PIN_GPIO);
  gpio_deep_sleep_hold_en();
  esp_deep_sleep_start();
}

void setup() {
  gpio_hold_dis(PIR_PIN_GPIO);
  pinMode(PIR_PIN, INPUT);
  pinMode(LED_PIN, OUTPUT);
  digitalWrite(LED_PIN, HIGH);

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

  uint32_t lastSend = millis();
  sendMotion();

  while (digitalRead(PIR_PIN) == HIGH) {
    if (millis() - lastSend >= MOTION_INTERVAL_MS) {
      lastSend = millis();
      sendMotion();
    }
    delay(20);
  }

  startSleep();
}

void loop() {}