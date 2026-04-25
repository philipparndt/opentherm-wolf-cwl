## ADDED Requirements

### Requirement: WiFi AP mode for initial setup
The firmware SHALL start a WiFi access point when no WiFi credentials are configured.

#### Scenario: First boot
- **WHEN** the ESP32 boots with no stored WiFi credentials
- **THEN** a WiFi AP named "WolfCWL-XXXX" (last 4 of MAC) is started with a captive portal for configuration

#### Scenario: Configuration saved
- **WHEN** the user submits WiFi and MQTT credentials via the AP web UI
- **THEN** the credentials are saved to NVS and the ESP32 restarts in station mode

### Requirement: NVS configuration persistence
The firmware SHALL persist all configuration in ESP32 NVS (Non-Volatile Storage).

#### Scenario: Configuration survives reboot
- **WHEN** the ESP32 restarts
- **THEN** all previously saved settings (WiFi, MQTT, pins, ventilation level) are restored

#### Scenario: Factory reset
- **WHEN** the encoder button is held for 10 seconds during boot
- **THEN** NVS is cleared and the ESP32 restarts in AP mode

### Requirement: Web server with JWT authentication
The firmware SHALL serve a web UI on port 80 with JWT-protected API endpoints.

#### Scenario: Login required
- **WHEN** an unauthenticated request hits `/api/*`
- **THEN** a 401 response is returned

#### Scenario: Authenticated access
- **WHEN** a request includes a valid JWT auth_token cookie
- **THEN** the API request is processed normally

### Requirement: OTA firmware update
The firmware SHALL accept firmware uploads via HTTP POST to `/api/ota/upload`.

#### Scenario: Successful OTA
- **WHEN** a valid firmware binary is uploaded
- **THEN** the firmware is flashed and the ESP32 restarts with the new firmware

### Requirement: mDNS discovery
The firmware SHALL register an mDNS hostname for easy network discovery.

#### Scenario: mDNS available
- **WHEN** the ESP32 is connected to WiFi
- **THEN** it is reachable at `wolf-cwl.local`

### Requirement: WiFi reconnection
The firmware SHALL automatically reconnect to WiFi if the connection is lost.

#### Scenario: WiFi dropout
- **WHEN** the WiFi connection drops
- **THEN** the firmware attempts reconnection every 10 seconds until successful
