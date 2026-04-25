## ADDED Requirements

### Requirement: Multi-page OLED display
The firmware SHALL drive a 128x64 I2C OLED display (SSD1306/SH1110) showing system state across multiple pages.

#### Scenario: Home page displayed
- **WHEN** the display shows the home page
- **THEN** the current ventilation level name (Off/Reduced/Normal/Party), relative ventilation %, and bypass state are visible

#### Scenario: Temperature page displayed
- **WHEN** the display shows the temperature page
- **THEN** supply inlet and exhaust inlet temperatures are shown in °C

#### Scenario: Status page displayed
- **WHEN** the display shows the status page
- **THEN** fault indication, filter status, current airflow (from TSP), and bypass status are visible

#### Scenario: System page displayed
- **WHEN** the display shows the system page
- **THEN** WiFi RSSI, MQTT connection state, IP address, and uptime are visible

### Requirement: Display auto-refresh
The firmware SHALL refresh the display content after each OpenTherm polling cycle.

#### Scenario: Data update reflected
- **WHEN** new sensor data is received from a polling cycle
- **THEN** the currently displayed page updates to show the new values

### Requirement: Display configuration
The firmware SHALL support configurable I2C pins for the OLED display via NVS.

#### Scenario: Custom I2C pins
- **WHEN** the user configures SDA and SCL pins via the web UI
- **THEN** the display initializes on the configured pins

### Requirement: Boot screen
The firmware SHALL show a boot screen during initialization.

#### Scenario: Boot sequence
- **WHEN** the ESP32 starts up
- **THEN** the display shows "Wolf CWL" and connection status until the first polling cycle completes
