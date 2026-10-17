#include <Arduino.h>
#include <WiFi.h>
#include <esp_now.h>
#include <esp_wifi.h>
#include <HTTPClient.h>

#define LED_PIN 8

typedef struct struct_message {
  int ID;
  char plant[32];
  float measurment;
  char type[16];
} struct_message;

struct_message plantData;
volatile bool newDataReceived = false;
unsigned long ledTurnedOnAt = 0;
bool ledIsOn = false;

String makePayload(char *type, int ID, char *plant, double moisture) {
  return "{\"type\":\"" + String(type) + "\",\"sensor\":" + String(ID) + ",\"plant\":\"" + String(plant) + "\",\"moisture\":" + String(moisture) + "}";
}

void onDataReceive(const uint8_t *mac, const uint8_t *incomingData, int len) {
  digitalWrite(LED_PIN, LOW); 
  memcpy(&plantData, incomingData, sizeof(plantData));
  newDataReceived = true;
}

void setup() {
  Serial.begin(115200);
  delay(3000); 
  
  pinMode(LED_PIN, OUTPUT);
  for(int i=0; i<3; i++) {
    digitalWrite(LED_PIN, LOW);
    delay(100);
    digitalWrite(LED_PIN, HIGH);
    delay(100);
  }
  digitalWrite(LED_PIN, HIGH); 

  Serial.println("\n--- Hub Booting ---");
  
  WiFi.mode(WIFI_MODE_STA);
  WiFi.disconnect();
  WiFi.setSleep(false); 
  
  esp_wifi_set_promiscuous(true);
  esp_wifi_set_channel(11, WIFI_SECOND_CHAN_NONE);
  esp_wifi_set_promiscuous(false);

  Serial.print("Hub is listening on Channel: ");
  Serial.println(WiFi.channel());

  if (esp_now_init() != ESP_OK) return;

  esp_now_register_recv_cb(onDataReceive);
}

void loop() {
  if (ledIsOn && (millis() - ledTurnedOnAt >= 50)) {
    digitalWrite(LED_PIN, HIGH);
    ledIsOn = false;
  }

  if (!newDataReceived) return;
  newDataReceived = false;
  
  ledTurnedOnAt = millis();
  ledIsOn = true;
  
  Serial.println(makePayload(plantData.type, plantData.ID, plantData.plant, plantData.measurment));
}