#include <Arduino.h>
#include <WiFi.h>
#include <esp_now.h>
#include <esp_wifi.h>
#include <HTTPClient.h>

#define LED_PIN 8

typedef struct {
  uint8_t packetType;
  int ID;
  char plant[32];
  float measurment;
} PlantPacket;

typedef struct {
  uint8_t packetType;
  int ID;
  char location[32];
} PirMotionPacket;

typedef struct {
  uint8_t packetType;
  int ID;
  int16_t x;
  int16_t y;
  int16_t speed;
  uint16_t resolution;
  float distance;
} LD240MotionPacket;

typedef String (*PacketHandler)(const uint8_t *data, int len);

PacketHandler handlers[256];

String handlePlant(const uint8_t *data, int len) {
  if (len != sizeof(PlantPacket)) return "";
  PlantPacket plant;
  memcpy(&plant, data, sizeof(PlantPacket));
  return "{\"type\":\"Plant\",\"sensor\":" + String(plant.ID) + 
         ",\"plant\":\"" + String(plant.plant) + 
         "\",\"moisture\":" + String(plant.measurment) + "}";
}

String handlePirMotion(const uint8_t *data, int len) {
  if (len != sizeof(PirMotionPacket)) return "";
  PirMotionPacket motion;
  memcpy(&motion, data, sizeof(PirMotionPacket));
  return "{\"type\":\"PirMotion\",\"sensor\":" + String(motion.ID) + 
         ",\"location\":\"" + String(motion.location) + "}";
}

String handleLD240Motion(const uint8_t *data, int len) {
  if (len != sizeof(LD240MotionPacket)) return "";
  LD240MotionPacket motion;
  memcpy(&motion, data, sizeof(LD240MotionPacket));
  return "{\"type\":\"LD240Motion\",\"sensor\":" + String(motion.ID) + 
         ",\"x\":\"" + String(motion.x) + ",\"y\":\"" + String(motion.y) + 
         ",\"speed\":\"" + String(motion.speed) + "}";
}

volatile bool newDataReceived = false;
unsigned long ledTurnedOnAt = 0;
bool ledIsOn = false;

void onDataReceive(const uint8_t *mac, const uint8_t *incomingData, int len) {
  if (len == 0) return;
  uint8_t type = incomingData[0];

  if (handlers[type] != nullptr) {
    String payload = handlers[type](incomingData, len);
    if (payload != "") {
      Serial.println(payload);
      digitalWrite(LED_PIN, LOW);
      newDataReceived = true;
    }
  } 
}

void setup() {
  Serial.begin(115200);
  delay(2000); 

  handlers[1] = handlePlant;
  handlers[2] = handlePirMotion;
  handlers[3] = handleLD240Motion;
  
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
}