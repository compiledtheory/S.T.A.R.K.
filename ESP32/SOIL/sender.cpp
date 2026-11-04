#include <Arduino.h>
#include <WiFi.h>
#include <esp_now.h>
#include <esp_wifi.h>

#define AIR 3800
#define SOIL 2400
#define SOIL_PIN 4
#define ENERGY_PIN 3
#define SLEEP 3600
#define LED_PIN 8
#define PEER_CHANNEL 11
#define PACKET_TYPE 1
#define NODE_ID 4

uint8_t broadcastAddress[] = {0x80, 0x53, 0xE0, 0xB4, 0xE1, 0x28};


esp_now_peer_info_t peerInfo;
volatile bool packetSent = false;
esp_now_send_status_t lastStatus = ESP_NOW_SEND_FAIL;

void onDataSend(const uint8_t *mac_addr, esp_now_send_status_t status) {
  lastStatus = status;
  packetSent = true;
}

int calculate_percentage(int measurement) {
  int percentage = (AIR - measurement) * 100 / (AIR - SOIL);
  return constrain(percentage, 0, 100);
}

int getAverageMeasurment(int samples) {
  int total = 0;
  for (int i = 0; i < samples; i++) {
    Serial.println(analogRead(SOIL_PIN));
    total += analogRead(SOIL_PIN);
    delay(10);
  }
  return total / samples;
}

void sendWithRetry(const uint8_t* data, size_t len, int attempts = 4) {
  for (int i = 0; i < attempts; i++) {
    packetSent = false;
    esp_err_t err = esp_now_send(broadcastAddress, data, len);
    if (err == ESP_OK) {
      uint32_t start = millis();
      while (!packetSent && millis() - start < 50) delay(1);
      if (packetSent && lastStatus == ESP_NOW_SEND_SUCCESS) return;
    }
    delay(10);
  }
}

void sendMoisture(int moisture) {
  uint8_t pkt[3] = {PACKET_TYPE, NODE_ID, moisture};
  sendWithRetry(pkt, sizeof(pkt));
  digitalWrite(LED_PIN, LOW);
  delay(100);
  digitalWrite(LED_PIN, HIGH);
}

void setup(){
  Serial.begin(115200);
  pinMode(ENERGY_PIN, OUTPUT);
  digitalWrite(ENERGY_PIN, HIGH);
  delay(100);
  
  pinMode(LED_PIN, OUTPUT);
  digitalWrite(LED_PIN, LOW);

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

  if (esp_now_add_peer(&peerInfo) != ESP_OK){
    delay(100);
    esp_sleep_enable_timer_wakeup(SLEEP * 1000000ULL);
    esp_deep_sleep_start();
  }

  digitalWrite(LED_PIN, HIGH);
  int moisture = calculate_percentage(getAverageMeasurment(10));

  digitalWrite(ENERGY_PIN, LOW);

  delay(100); 
  sendMoisture(moisture);

  digitalWrite(LED_PIN, LOW);
  Serial.flush();
  
  esp_sleep_enable_timer_wakeup(SLEEP * 1000000ULL);
  esp_deep_sleep_start();
}

void loop() {}