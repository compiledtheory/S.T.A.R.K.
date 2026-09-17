#include <Arduino.h>
#include <WiFi.h>
#include <esp_now.h>
#include <esp_wifi.h>
#include <HTTPClient.h>

#define TARGET_URL "http://192.168.0.206:8080/"
#define LED_PIN 8

typedef struct struct_message {
  char type[16];
  int ID;
  char plant[32];
  float measurment;
} struct_message;

struct_message plantData;
volatile bool newDataReceived = false;

String makePayload(char *type, int ID, char *plant, double moisture) {
  return "{\"type\":" + String(type) + \"sensor\":" + String(ID) + ",\"plant\":\"" + String(plant) + "\",\"moisture\":" + String(moisture) + "}";
}

void onDataReceive(const uint8_t *mac, const uint8_t *incomingData, int len) {
  memcpy(&plantData, incomingData, sizeof(plantData));
  newDataReceived = true;
}

void setup() {
  Serial.begin(115200);
  delay(3000); 
  Serial.println("\n--- Hub Booting ---");
  
  WiFi.mode(WIFI_MODE_STA);
  esp_wifi_set_channel(11, WIFI_SECOND_CHAN_NONE);

  Serial.print("Hub is listening on Channel: ");
  Serial.println(WiFi.channel());

  if (esp_now_init() != ESP_OK) return;

  esp_now_register_recv_cb(onDataReceive);
}

void loop() {
  if (!newDataReceived) return;
  newDataReceived = false;
  Serial.println(makePayload(plantData.ID, plantData.plant, plantData.measurment));
}

