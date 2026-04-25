#!/bin/bash

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# Use PlatformIO from venv if available, otherwise global
if [ -x "$SCRIPT_DIR/.venv/bin/pio" ]; then
  PIO="$SCRIPT_DIR/.venv/bin/pio"
elif command -v pio &> /dev/null; then
  PIO="pio"
else
  PIO=""
fi

# Upload firmware and/or filesystem to Wolf CWL controller via network (OTA)
# Usage: ./upload-ota.sh [options] [environment]

set -e

# Load .env file if present
if [ -f "$SCRIPT_DIR/.env" ]; then
    set -a
    source "$SCRIPT_DIR/.env"
    set +a
fi

# Defaults (env vars from .env take precedence)
ENV="${OTA_ENV:-esp32}"
HOST="${OTA_HOST:-wolf-cwl.local}"
USERNAME="${OTA_USER:-admin}"
PASSWORD="${OTA_PASS:-admin}"
UPLOAD_FIRMWARE=true
UPLOAD_FILESYSTEM=true
DO_BUILD=false

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --firmware)
            UPLOAD_FIRMWARE=true
            UPLOAD_FILESYSTEM=false
            shift
            ;;
        --filesystem|--fs)
            UPLOAD_FIRMWARE=false
            UPLOAD_FILESYSTEM=true
            shift
            ;;
        --all)
            UPLOAD_FIRMWARE=true
            UPLOAD_FILESYSTEM=true
            shift
            ;;
        --host)
            HOST="$2"
            shift 2
            ;;
        --user)
            USERNAME="$2"
            shift 2
            ;;
        --pass)
            PASSWORD="$2"
            shift 2
            ;;
        --build)
            DO_BUILD=true
            shift
            ;;
        -h|--help)
            echo "Usage: $0 [options] [environment]"
            echo ""
            echo "Options:"
            echo "  --firmware    Upload firmware only"
            echo "  --filesystem  Upload filesystem only"
            echo "  --all         Upload both firmware and filesystem (default)"
            echo "  --host HOST   Specify device host (default: wolf-cwl.local)"
            echo "  --user USER   Specify username (default: admin)"
            echo "  --pass PASS   Specify password (default: admin)"
            echo "  --build       Build before uploading"
            echo "  -h, --help    Show this help"
            exit 0
            ;;
        -*)
            echo "Unknown option: $1"
            exit 1
            ;;
        *)
            ENV="$1"
            shift
            ;;
    esac
done

FIRMWARE_FILE=".pio/build/$ENV/firmware.bin"
FILESYSTEM_FILE=".pio/build/$ENV/littlefs.bin"

echo -e "${YELLOW}==================================${NC}"
echo -e "${YELLOW}Wolf CWL - OTA Upload${NC}"
echo -e "${YELLOW}==================================${NC}"
echo "Host: $HOST"
echo "Environment: $ENV"
echo -n "Upload: "
if [ "$UPLOAD_FIRMWARE" = true ] && [ "$UPLOAD_FILESYSTEM" = true ]; then
    echo "Firmware + Filesystem"
elif [ "$UPLOAD_FIRMWARE" = true ]; then
    echo "Firmware only"
else
    echo "Filesystem only"
fi
echo ""

# Build if requested
if [ "$DO_BUILD" = true ]; then
    if [ -z "$PIO" ]; then
        echo -e "${RED}Error: PlatformIO not found. Run 'make setup-venv' first.${NC}"
        exit 1
    fi
    if [ "$UPLOAD_FIRMWARE" = true ]; then
        echo -e "${CYAN}Building firmware...${NC}"
        "$PIO" run -e "$ENV"
        echo ""
    fi
    if [ "$UPLOAD_FILESYSTEM" = true ]; then
        echo -e "${CYAN}Building filesystem...${NC}"
        "$PIO" run -t buildfs -e "$ENV"
        echo ""
    fi
fi

# Check if files exist
if [ "$UPLOAD_FIRMWARE" = true ] && [ ! -f "$FIRMWARE_FILE" ]; then
    echo -e "${RED}Error: Firmware file not found: $FIRMWARE_FILE${NC}"
    echo "Build first with: make build-firmware"
    exit 1
fi

if [ "$UPLOAD_FILESYSTEM" = true ] && [ ! -f "$FILESYSTEM_FILE" ]; then
    echo -e "${RED}Error: Filesystem file not found: $FILESYSTEM_FILE${NC}"
    echo "Build first with: make build-fs"
    exit 1
fi

# Check connectivity
echo -n "Checking connectivity to $HOST... "
if ! ping -c 1 -W 2 "$HOST" > /dev/null 2>&1; then
    echo -e "${RED}Failed${NC}"
    echo ""
    echo "Cannot reach $HOST. Try:"
    echo "  1. Check if the device is powered on"
    echo "  2. Use IP address: ./upload-ota.sh --host 192.168.x.x"
    exit 1
fi

RESOLVED_IP=$(ping -c 1 "$HOST" 2>/dev/null | head -1 | grep -oE '[0-9]+\.[0-9]+\.[0-9]+\.[0-9]+' | head -1)
if [ -n "$RESOLVED_IP" ]; then
    echo -e "${GREEN}OK${NC} ($RESOLVED_IP)"
    HOST="$RESOLVED_IP"
else
    echo -e "${GREEN}OK${NC}"
fi

# Login
echo -n "Logging in... "
COOKIE_FILE=$(mktemp)
trap "rm -f $COOKIE_FILE" EXIT

LOGIN_OUTPUT=$(mktemp)
HTTP_CODE=$(curl -s -o "$LOGIN_OUTPUT" -w "%{http_code}" -c "$COOKIE_FILE" -X POST "http://$HOST/api/login" \
    -H "Content-Type: application/json" \
    -d "{\"username\":\"$USERNAME\",\"password\":\"$PASSWORD\"}" \
    --connect-timeout 10 2>&1) || {
    echo -e "${RED}Failed${NC}"
    rm -f "$LOGIN_OUTPUT"
    exit 1
}
rm -f "$LOGIN_OUTPUT"

if [ "$HTTP_CODE" != "200" ]; then
    echo -e "${RED}Failed (HTTP $HTTP_CODE)${NC}"
    if [ "$HTTP_CODE" = "401" ]; then
        echo "Invalid credentials. Check --user and --pass options."
    fi
    exit 1
fi
echo -e "${GREEN}OK${NC}"
echo ""

# Upload filesystem first (if selected)
if [ "$UPLOAD_FILESYSTEM" = true ]; then
    FILESIZE=$(ls -lh "$FILESYSTEM_FILE" | awk '{print $5}')
    echo -e "${CYAN}Uploading filesystem ($FILESIZE)...${NC}"

    RESPONSE_FILE=$(mktemp)
    PROGRESS_FILE=$(mktemp)

    curl -X POST "http://$HOST/api/ota/filesystem" \
        -b "$COOKIE_FILE" \
        -F "file=@$FILESYSTEM_FILE" \
        -o "$RESPONSE_FILE" \
        -w "\n%{http_code}" \
        --connect-timeout 10 \
        -# \
        2>"$PROGRESS_FILE" &
    CURL_PID=$!

    set +e
    UPLOAD_COMPLETE=false
    while kill -0 $CURL_PID 2>/dev/null; do
        PROGRESS=$(tail -c 100 "$PROGRESS_FILE" 2>/dev/null || echo "")
        printf "\r%s" "$PROGRESS"
        if echo "$PROGRESS" | grep -q "100"; then
            if [ "$UPLOAD_COMPLETE" = false ]; then
                UPLOAD_COMPLETE=true
                echo ""
                echo -n "Upload complete, waiting for response..."
                sleep 3
                kill $CURL_PID 2>/dev/null
                wait $CURL_PID 2>/dev/null
                break
            fi
        fi
        sleep 0.2
    done
    wait $CURL_PID 2>/dev/null
    set -e

    echo ""
    RESPONSE=$(cat "$RESPONSE_FILE" 2>/dev/null || echo "")
    rm -f "$RESPONSE_FILE" "$PROGRESS_FILE"

    if echo "$RESPONSE" | grep -q '"success":true'; then
        echo -e "${GREEN}Filesystem uploaded successfully!${NC}"
    elif [ "$UPLOAD_COMPLETE" = true ]; then
        echo -e "${GREEN}Filesystem uploaded (device rebooted)${NC}"
    else
        echo -e "${RED}Filesystem upload may have failed${NC}"
        echo "Response: $RESPONSE"
    fi

    if [ "$UPLOAD_FIRMWARE" = true ]; then
        echo -n "Waiting for device to reboot"
        for i in {1..5}; do sleep 1; echo -n "."; done
        echo ""

        echo -n "Re-authenticating "
        HTTP_CODE=""
        for i in {1..20}; do
            HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" -c "$COOKIE_FILE" -X POST "http://$HOST/api/login" \
                -H "Content-Type: application/json" \
                -d "{\"username\":\"$USERNAME\",\"password\":\"$PASSWORD\"}" \
                --connect-timeout 2 --max-time 3 2>/dev/null) || true
            if [ "$HTTP_CODE" = "200" ]; then break; fi
            echo -n "."
            sleep 1
        done

        if [ "$HTTP_CODE" != "200" ]; then
            echo -e " ${RED}Failed${NC}"
            echo "Try firmware upload manually: ./upload-ota.sh --firmware"
            exit 1
        fi
        echo -e " ${GREEN}OK${NC}"
        echo ""
    fi
fi

# Upload firmware (if selected)
if [ "$UPLOAD_FIRMWARE" = true ]; then
    FILESIZE=$(ls -lh "$FIRMWARE_FILE" | awk '{print $5}')
    echo -e "${CYAN}Uploading firmware ($FILESIZE)...${NC}"

    RESPONSE_FILE=$(mktemp)
    PROGRESS_FILE=$(mktemp)

    curl -X POST "http://$HOST/api/ota/upload" \
        -b "$COOKIE_FILE" \
        -F "file=@$FIRMWARE_FILE" \
        -o "$RESPONSE_FILE" \
        -w "\n%{http_code}" \
        --connect-timeout 10 \
        -# \
        2>"$PROGRESS_FILE" &
    CURL_PID=$!

    set +e
    UPLOAD_COMPLETE=false
    while kill -0 $CURL_PID 2>/dev/null; do
        PROGRESS=$(tail -c 100 "$PROGRESS_FILE" 2>/dev/null || echo "")
        printf "\r%s" "$PROGRESS"
        if echo "$PROGRESS" | grep -q "100"; then
            if [ "$UPLOAD_COMPLETE" = false ]; then
                UPLOAD_COMPLETE=true
                echo ""
                echo -n "Upload complete, waiting for response..."
                sleep 3
                kill $CURL_PID 2>/dev/null
                wait $CURL_PID 2>/dev/null
                break
            fi
        fi
        sleep 0.2
    done
    wait $CURL_PID 2>/dev/null
    set -e

    echo ""
    RESPONSE=$(cat "$RESPONSE_FILE" 2>/dev/null || echo "")
    rm -f "$RESPONSE_FILE" "$PROGRESS_FILE"

    if echo "$RESPONSE" | grep -q '"success":true'; then
        echo -e "${GREEN}Firmware uploaded successfully!${NC}"
    elif [ "$UPLOAD_COMPLETE" = true ]; then
        echo -e "${GREEN}Firmware uploaded (device rebooted)${NC}"
    else
        echo -e "${RED}Firmware upload may have failed${NC}"
        echo "Response: $RESPONSE"
    fi
fi

echo ""
echo -e "${GREEN}==================================${NC}"
echo -e "${GREEN}OTA Upload Complete!${NC}"
echo -e "${GREEN}Device is rebooting...${NC}"
echo -e "${GREEN}==================================${NC}"
echo ""
echo "Wait ~10 seconds, then access: http://$HOST"
