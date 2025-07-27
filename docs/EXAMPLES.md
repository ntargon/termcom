# TermCom Examples

This document provides practical examples of using TermCom for various embedded device communication scenarios.

## Table of Contents

- [Basic Usage](#basic-usage)
- [Serial Communication](#serial-communication)
- [TCP Communication](#tcp-communication)
- [Configuration Management](#configuration-management)
- [Session Management](#session-management)
- [Advanced Scenarios](#advanced-scenarios)
- [Automation Scripts](#automation-scripts)
- [Troubleshooting Examples](#troubleshooting-examples)

## Basic Usage

### First Time Setup

```bash
# Initialize global configuration
termcom config init --global

# Initialize project-specific configuration
cd my-embedded-project
termcom config init

# Check configuration
termcom config show
```

### Quick Serial Connection

```bash
# Connect to Arduino on default USB port
termcom serial connect --port /dev/ttyACM0 --baud 115200

# List available serial ports
termcom serial list

# Send AT command to modem
termcom serial connect --port /dev/ttyUSB0 --baud 9600
termcom serial send "AT" --session <session-id>
```

## Serial Communication

### Arduino Development

```bash
# Connect to Arduino Uno
termcom serial connect \
  --port /dev/ttyACM0 \
  --baud 115200 \
  --name "Arduino Uno"

# Send sensor reading command
termcom serial send "READ_TEMP" --session arduino-session

# Monitor continuous data
termcom serial monitor --session arduino-session --output temp_log.txt
```

### ESP32 Serial Communication

```bash
# Connect to ESP32 with high baud rate
termcom serial connect \
  --port /dev/ttyUSB0 \
  --baud 921600 \
  --data-bits 8 \
  --stop-bits 1 \
  --parity none \
  --flow-control none

# Send AT commands for WiFi setup
termcom serial send "AT+CWMODE=1" --session esp32-session
termcom serial send "AT+CWJAP=\"MyWiFi\",\"password\"" --session esp32-session
```

### Industrial Modbus Communication

```bash
# Connect to Modbus device
termcom serial connect \
  --port /dev/ttyRS485-1 \
  --baud 19200 \
  --data-bits 8 \
  --stop-bits 1 \
  --parity even

# Send Modbus RTU frame (hex format)
termcom serial send "01 03 00 00 00 0A C5 CD" \
  --session modbus-session \
  --format hex
```

### GPS Module Communication

```bash
# Connect to GPS module
termcom serial connect \
  --port /dev/ttyAMA0 \
  --baud 9600 \
  --name "GPS Module"

# Monitor NMEA sentences
termcom serial monitor --session gps-session --output gps_log.txt

# Send GPS configuration commands
termcom serial send "\$PMTK314,0,1,0,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0*28" \
  --session gps-session
```

## TCP Communication

### ESP32 Web Server

```bash
# Connect to ESP32 web server
termcom tcp connect 192.168.1.100 80 --name "ESP32 Server"

# Send HTTP GET request
termcom tcp send "GET /api/sensors HTTP/1.1\r\nHost: 192.168.1.100\r\n\r\n" \
  --session esp32-server
```

### IoT Device Communication

```bash
# Start TCP server for IoT devices
termcom tcp server --port 8080 --name "IoT Gateway"

# Monitor incoming connections
termcom session list --type monitoring

# Send command to specific device
termcom tcp send "{\"command\":\"status\"}" \
  --session iot-device-001 \
  --format text
```

### Embedded Linux Device

```bash
# Connect to embedded Linux device via SSH tunnel
# (Assuming SSH tunnel is set up: ssh -L 2222:localhost:22 device)
termcom tcp connect localhost 2222 --name "Embedded Linux"

# Send shell commands (if using telnet-like interface)
termcom tcp send "cat /proc/cpuinfo" --session linux-device
```

### Network Testing

```bash
# Test connection to remote server
termcom tcp connect example.com 80 --timeout 10

# Send simple HTTP request
termcom tcp send "GET / HTTP/1.1\r\nHost: example.com\r\n\r\n" \
  --session web-test

# Monitor response
termcom tcp monitor --session web-test
```

## Configuration Management

### Creating Device Profiles

Create `.termcom/config.toml`:

```toml
[global]
log_level = "info"
max_sessions = 5
timeout_ms = 5000
auto_save = true
history_limit = 1000

# Arduino Uno Profile
[[devices]]
name = "arduino_uno"
description = "Arduino Uno Development Board"

[devices.connection]
type = "serial"
port = "/dev/ttyACM0"
baud_rate = 115200
data_bits = 8
stop_bits = 1
parity = "none"
flow_control = "none"

[[devices.commands]]
name = "led_on"
description = "Turn on built-in LED"
template = "LED_ON\r\n"
response_pattern = "OK"
timeout_ms = 1000

[[devices.commands]]
name = "read_sensors"
description = "Read all sensor values"
template = "READ_ALL\r\n"
response_pattern = "SENSORS:.*"
timeout_ms = 2000

# ESP32 WiFi Module Profile
[[devices]]
name = "esp32_wifi"
description = "ESP32 WiFi Module"

[devices.connection]
type = "tcp"
host = "192.168.1.50"
port = 80
timeout_ms = 3000
keep_alive = true

[[devices.commands]]
name = "get_status"
description = "Get device status"
template = "GET /status HTTP/1.1\r\nHost: {host}\r\n\r\n"
response_pattern = "HTTP/1.1 200 OK"
timeout_ms = 5000

# Industrial PLC Profile
[[devices]]
name = "industrial_plc"
description = "Industrial PLC via Modbus"

[devices.connection]
type = "serial"
port = "/dev/ttyRS485-1"
baud_rate = 19200
data_bits = 8
stop_bits = 1
parity = "even"
flow_control = "none"

[[devices.commands]]
name = "read_coils"
description = "Read coil status"
template = "01 01 00 00 00 10 3D CC"  # Modbus RTU frame in hex
response_pattern = "01 01 02 .*"
timeout_ms = 1000
```

### Environment-Specific Configurations

Development environment (`.termcom/config.toml`):
```toml
[global]
log_level = "debug"
max_sessions = 10

[[devices]]
name = "dev_board"
description = "Development board on USB"

[devices.connection]
type = "serial"
port = "/dev/ttyUSB0"
baud_rate = 115200
```

Production environment (`~/.config/termcom/config.toml`):
```toml
[global]
log_level = "warn"
max_sessions = 20
timeout_ms = 10000

[[devices]]
name = "production_gateway"
description = "Production IoT Gateway"

[devices.connection]
type = "tcp"
host = "10.0.1.100"
port = 502
timeout_ms = 5000
keep_alive = true
```

## Session Management

### Working with Multiple Sessions

```bash
# Create multiple sessions
termcom serial connect --port /dev/ttyUSB0 --baud 9600 --name "Device1"
termcom serial connect --port /dev/ttyUSB1 --baud 115200 --name "Device2"
termcom tcp connect 192.168.1.100 80 --name "WebServer"

# List all sessions
termcom session list

# Filter sessions by type
termcom session list --type interactive

# Show detailed session information
termcom session show <session-id> --messages --activities

# Export session data
termcom session export <session-id> --output session_data.json --format json
```

### Session Statistics

```bash
# Get overall statistics
termcom session stats

# Monitor specific session
termcom session show session-123 --messages

# List sessions by device type
termcom session list --device "Arduino"
```

### Session Lifecycle Management

```bash
# Start a stopped session
termcom session start <session-id>

# Stop an active session
termcom session stop <session-id>

# Remove a session completely
termcom session remove <session-id>

# Create session from config
termcom session create arduino_uno --name "My Arduino" --type interactive
```

## Advanced Scenarios

### Multi-Device Testing Setup

```bash
#!/bin/bash
# Script: setup_test_environment.sh

# Initialize project
termcom config init

# Connect to multiple test devices
echo "Connecting to test devices..."

# Main controller
CONTROLLER=$(termcom serial connect --port /dev/ttyUSB0 --baud 115200 --name "Controller")
echo "Controller session: $CONTROLLER"

# Sensor modules
SENSOR1=$(termcom serial connect --port /dev/ttyUSB1 --baud 9600 --name "Sensor1")
SENSOR2=$(termcom serial connect --port /dev/ttyUSB2 --baud 9600 --name "Sensor2")

# Network gateway
GATEWAY=$(termcom tcp connect 192.168.1.50 8080 --name "Gateway")

echo "Test environment ready!"
echo "Controller: $CONTROLLER"
echo "Sensor1: $SENSOR1"
echo "Sensor2: $SENSOR2"
echo "Gateway: $GATEWAY"

# Start monitoring all devices
termcom serial monitor --session $CONTROLLER --output controller.log &
termcom serial monitor --session $SENSOR1 --output sensor1.log &
termcom serial monitor --session $SENSOR2 --output sensor2.log &
termcom tcp monitor --session $GATEWAY --output gateway.log &

echo "Monitoring started. Logs will be saved to respective files."
```

### Automated Device Configuration

```bash
#!/bin/bash
# Script: configure_esp32.sh

SESSION_ID=$(termcom serial connect --port /dev/ttyUSB0 --baud 115200 --name "ESP32")

echo "Configuring ESP32..."

# Reset to factory defaults
termcom serial send "AT+RESTORE" --session $SESSION_ID
sleep 2

# Set WiFi mode
termcom serial send "AT+CWMODE=1" --session $SESSION_ID
sleep 1

# Connect to WiFi
termcom serial send "AT+CWJAP=\"MyNetwork\",\"MyPassword\"" --session $SESSION_ID
sleep 5

# Set up TCP server
termcom serial send "AT+CIPMUX=1" --session $SESSION_ID
sleep 1
termcom serial send "AT+CIPSERVER=1,80" --session $SESSION_ID

echo "ESP32 configuration complete!"

# Show final status
termcom serial send "AT+CIFSR" --session $SESSION_ID
```

### Data Collection and Analysis

```bash
#!/bin/bash
# Script: collect_sensor_data.sh

# Create session for data collection
SESSION_ID=$(termcom serial connect --port /dev/ttyUSB0 --baud 9600 --name "DataLogger")

# Collect data for 1 hour
echo "Starting data collection..."
timeout 3600 termcom serial monitor --session $SESSION_ID --output sensor_data.log

# Process collected data
echo "Processing collected data..."
grep "TEMP:" sensor_data.log | cut -d: -f2 > temperature.csv
grep "HUMID:" sensor_data.log | cut -d: -f2 > humidity.csv

echo "Data collection complete!"
echo "Temperature data: temperature.csv"
echo "Humidity data: humidity.csv"
```

### Load Testing

```bash
#!/bin/bash
# Script: load_test.sh

# Start TCP server
termcom tcp server --port 8080 --name "LoadTestServer" &
SERVER_PID=$!

sleep 2

# Create multiple client connections
for i in {1..10}; do
    SESSION_ID=$(termcom tcp connect localhost 8080 --name "Client$i")
    
    # Send test data
    for j in {1..100}; do
        termcom tcp send "TEST_DATA_$i_$j" --session $SESSION_ID
        sleep 0.1
    done &
done

# Wait for all background jobs
wait

# Stop server
kill $SERVER_PID

echo "Load test complete!"
termcom session stats
```

## Automation Scripts

### Continuous Integration Testing

```bash
#!/bin/bash
# Script: ci_hardware_test.sh

set -e

echo "Starting hardware CI tests..."

# Initialize test environment
termcom config init --output ci_config.toml

# Test serial communication
echo "Testing serial communication..."
SESSION_ID=$(termcom serial connect --port /dev/ttyUSB0 --baud 115200 --name "CI_Test")

# Send test commands
termcom serial send "TEST_START" --session $SESSION_ID
sleep 1

# Verify response
RESPONSE=$(termcom session show $SESSION_ID --messages | grep "TEST_OK")
if [ -z "$RESPONSE" ]; then
    echo "Serial test failed!"
    exit 1
fi

echo "Serial test passed!"

# Test TCP communication
echo "Testing TCP communication..."
TCP_SESSION=$(termcom tcp connect localhost 8080 --name "TCP_Test")

termcom tcp send "PING" --session $TCP_SESSION
sleep 1

# Verify TCP response
TCP_RESPONSE=$(termcom session show $TCP_SESSION --messages | grep "PONG")
if [ -z "$TCP_RESPONSE" ]; then
    echo "TCP test failed!"
    exit 1
fi

echo "TCP test passed!"

# Clean up
termcom session remove $SESSION_ID
termcom session remove $TCP_SESSION

echo "All hardware tests passed!"
```

### Device Firmware Update

```bash
#!/bin/bash
# Script: firmware_update.sh

DEVICE_PORT="/dev/ttyUSB0"
FIRMWARE_FILE="firmware.hex"

echo "Starting firmware update process..."

# Connect to device
SESSION_ID=$(termcom serial connect --port $DEVICE_PORT --baud 115200 --name "FirmwareUpdate")

# Enter bootloader mode
echo "Entering bootloader mode..."
termcom serial send "ENTER_BOOTLOADER" --session $SESSION_ID
sleep 2

# Verify bootloader mode
BOOTLOADER_RESPONSE=$(termcom session show $SESSION_ID --messages | grep "BOOTLOADER_READY")
if [ -z "$BOOTLOADER_RESPONSE" ]; then
    echo "Failed to enter bootloader mode!"
    exit 1
fi

# Send firmware data
echo "Uploading firmware..."
while IFS= read -r line; do
    termcom serial send "$line" --session $SESSION_ID --format hex
    sleep 0.1
done < "$FIRMWARE_FILE"

# Exit bootloader and restart
termcom serial send "EXIT_BOOTLOADER" --session $SESSION_ID
sleep 3

# Verify new firmware
termcom serial send "GET_VERSION" --session $SESSION_ID
sleep 1

echo "Firmware update complete!"
termcom session show $SESSION_ID --messages | tail -10
```

### Production Testing

```bash
#!/bin/bash
# Script: production_test.sh

DEVICE_ID=$1
if [ -z "$DEVICE_ID" ]; then
    echo "Usage: $0 <device_id>"
    exit 1
fi

LOG_FILE="production_test_${DEVICE_ID}.log"

echo "Starting production test for device $DEVICE_ID" | tee $LOG_FILE

# Test setup
SESSION_ID=$(termcom serial connect --port /dev/ttyUSB0 --baud 115200 --name "Production_$DEVICE_ID")

# Test sequence
TESTS=(
    "SELF_TEST"
    "COMM_TEST"
    "SENSOR_TEST"
    "ACTUATOR_TEST"
    "STRESS_TEST"
)

PASSED=0
TOTAL=${#TESTS[@]}

for test in "${TESTS[@]}"; do
    echo "Running $test..." | tee -a $LOG_FILE
    
    termcom serial send "$test" --session $SESSION_ID
    sleep 5
    
    # Check test result
    RESULT=$(termcom session show $SESSION_ID --messages | tail -1 | grep "PASS")
    if [ -n "$RESULT" ]; then
        echo "$test: PASSED" | tee -a $LOG_FILE
        ((PASSED++))
    else
        echo "$test: FAILED" | tee -a $LOG_FILE
    fi
done

# Generate test report
echo "=================" | tee -a $LOG_FILE
echo "PRODUCTION TEST SUMMARY" | tee -a $LOG_FILE
echo "Device ID: $DEVICE_ID" | tee -a $LOG_FILE
echo "Tests Passed: $PASSED/$TOTAL" | tee -a $LOG_FILE
echo "Date: $(date)" | tee -a $LOG_FILE

if [ $PASSED -eq $TOTAL ]; then
    echo "STATUS: PASS" | tee -a $LOG_FILE
    exit 0
else
    echo "STATUS: FAIL" | tee -a $LOG_FILE
    exit 1
fi
```

## Troubleshooting Examples

### Debug Serial Connection Issues

```bash
# Check available ports
termcom serial list

# Test with different baud rates
for baud in 9600 19200 38400 57600 115200; do
    echo "Testing baud rate: $baud"
    SESSION_ID=$(termcom serial connect --port /dev/ttyUSB0 --baud $baud --name "Test_$baud")
    termcom serial send "AT" --session $SESSION_ID
    sleep 1
    
    # Check for response
    RESPONSE=$(termcom session show $SESSION_ID --messages | grep -E "(OK|ERROR)")
    if [ -n "$RESPONSE" ]; then
        echo "Success with baud rate $baud: $RESPONSE"
        break
    fi
    
    termcom session remove $SESSION_ID
done
```

### Network Connectivity Testing

```bash
#!/bin/bash
# Script: network_debug.sh

HOST=$1
PORT=$2

if [ -z "$HOST" ] || [ -z "$PORT" ]; then
    echo "Usage: $0 <host> <port>"
    exit 1
fi

echo "Testing connectivity to $HOST:$PORT"

# Test with different timeouts
for timeout in 1 5 10 30; do
    echo "Trying with ${timeout}s timeout..."
    
    SESSION_ID=$(termcom tcp connect $HOST $PORT --timeout $timeout --name "Debug_$timeout" 2>/dev/null)
    
    if [ $? -eq 0 ]; then
        echo "Connected successfully with ${timeout}s timeout"
        termcom tcp send "TEST" --session $SESSION_ID
        sleep 1
        termcom session show $SESSION_ID --messages
        termcom session remove $SESSION_ID
        break
    else
        echo "Failed with ${timeout}s timeout"
    fi
done
```

### Configuration Validation

```bash
#!/bin/bash
# Script: validate_config.sh

CONFIG_FILE=$1

if [ -z "$CONFIG_FILE" ]; then
    CONFIG_FILE=".termcom/config.toml"
fi

echo "Validating configuration: $CONFIG_FILE"

# Validate configuration syntax
if termcom config validate $CONFIG_FILE; then
    echo "✓ Configuration syntax is valid"
else
    echo "✗ Configuration syntax error"
    exit 1
fi

# Test each device configuration
echo "Testing device configurations..."

# Extract device names from config
DEVICES=$(grep "^name = " $CONFIG_FILE | cut -d'"' -f2)

for device in $DEVICES; do
    echo "Testing device: $device"
    
    # Create session for device
    SESSION_ID=$(termcom session create $device --name "Test_$device" 2>/dev/null)
    
    if [ $? -eq 0 ]; then
        echo "✓ Device $device: Configuration valid"
        termcom session remove $SESSION_ID
    else
        echo "✗ Device $device: Configuration error"
    fi
done

echo "Configuration validation complete"
```

These examples demonstrate the versatility and power of TermCom for embedded device communication. Adapt them to your specific use cases and device requirements.