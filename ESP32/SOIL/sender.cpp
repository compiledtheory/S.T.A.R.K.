#include <Arduino.h>
#include <WiFi.h>
#include <esp_now.h>

#define AIR 3800
#define WATER 1400
#define SOIL_PIN 4
#define SLEEP 3

uint8_t broadcastAddress[] = {0x9C, 0x9E, 0x6E, 0xE2, 0xDE, 0x1C};

typedef struct struct_message {
  int ID;
  char plant[32];
  float measurment;
} struct_message;

struct_message plantData;
esp_now_peer_info_t peerInfo;

void onDataSend(const uint8_t *mac_addr, esp_now_send_status_t status) {
  return;
}

double calculate_percentage(int measurment) {
  double percentage = (double)(AIR-measurment) / (AIR-WATER) * 100;
  return constrain(percentage, 0.0, 100.0);
}

int getAverageMeasurment(int samples) {
  int total = 0;
  for (int i = 0; i < samples; i++) {
    total += analogRead(SOIL_PIN);
    delay(10);
  }
  return total / samples;
}

void setup(){
  Serial.begin(115200);
  WiFi.mode(WIFI_STA);
  if (esp_now_init() != ESP_OK) {
    return;
  }

  esp_now_register_send_cb(onDataSend);

  memcpy(peerInfo.peer_addr, broadcastAddress, 6);
  peerInfo.channel = 0;
  peerInfo.encrypt = false;

  if (esp_now_add_peer(&peerInfo) != ESP_OK){
    return;
  }

  int average = getAverageMeasurment(10);
  double percentage = calculate_percentage(average);

  plantData.ID = 1;
  strcpy(plantData.plant, "MONSTERA");
  plantData.measurment = percentage;

  esp_err_t result = esp_now_send(broadcastAddress, (uint8_t *) &plantData, sizeof(plantData));

  Serial.flush();
  Serial.end();
  delay(100);

  esp_sleep_enable_timer_wakeup(SLEEP * 1000000ULL);
  esp_deep_sleep_start();
}

void loop() {}

