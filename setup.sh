#!/usr/bin/env bash
set -e

echo "================================================="
echo "⚙️ Gemini Pool Proxy - Interactive Setup"
echo "================================================="

if [ -f ".env" ]; then
    read -p "⚠️  .env file already exists! Overwrite it? [y/N]: " overwrite_choice
    if [[ "$overwrite_choice" != "y" && "$overwrite_choice" != "Y" ]]; then
        echo "Setup aborted. Your existing .env was not modified."
        exit 0
    fi
fi

if [ ! -f ".env.example" ]; then
    echo "❌ Error: .env.example file is missing. Please ensure you are running this from the project root."
    exit 1
fi

echo ""
echo "Select Setup Mode:"
echo "  [1] Quick Setup (Default) - Configure only essential API keys and tokens."
echo "  [2] Full Setup - Configure advanced settings like Port, Bind Host, and Routing Strategy."
echo ""
read -p "Enter your choice [1 or 2]: " setup_mode

is_full_setup=false
if [[ "$setup_mode" == "2" ]]; then
    is_full_setup=true
    echo "=> 🚀 Proceeding with Full Setup..."
else
    echo "=> ⚡ Proceeding with Quick Setup..."
fi

echo ""
echo "Press [Enter] at any prompt to use default values."
echo ""

# =========== CORE SETTINGS (Quick & Full) ===========

# 1. Auth Token
read -p "1. Enter Admin AUTH_TOKEN (leave blank to auto-generate): " input_auth_token
if [ -z "$input_auth_token" ]; then
    AUTH_TOKEN="sk-admin-$(LC_ALL=C tr -dc A-Za-z0-9 </dev/urandom | head -c 12 2>/dev/null || echo $RANDOM)"
    echo "   -> Generated: $AUTH_TOKEN"
else
    AUTH_TOKEN="$input_auth_token"
fi

# 2. Allowed Tokens
read -p "2. Enter ALLOWED_TOKENS for client access (comma-separated, no quotes): " input_allowed_tokens
if [ -z "$input_allowed_tokens" ]; then
    ALLOWED_TOKENS="[\"sk-user-123456\"]"
    echo "   -> Using default: $ALLOWED_TOKENS"
else
    formatted=""
    IFS=',' read -ra ADDR <<< "$input_allowed_tokens"
    for i in "${ADDR[@]}"; do
        trim_i=$(echo "$i" | xargs) # trim whitespace safely
        if [ -n "$formatted" ]; then
            formatted="$formatted, \"$trim_i\""
        else
            formatted="\"$trim_i\""
        fi
    done
    ALLOWED_TOKENS="[$formatted]"
fi

# 3. API Keys
read -p "3. Enter Google Gemini API_KEYS (comma-separated, no quotes): " input_api_keys
if [ -z "$input_api_keys" ]; then
    API_KEYS="[\"AIzaSy_demo_key_1\"]"
    echo "   -> Warning: Using demo key. Remember to change this later!"
else
    formatted=""
    IFS=',' read -ra ADDR <<< "$input_api_keys"
    for i in "${ADDR[@]}"; do
        trim_i=$(echo "$i" | xargs)
        if [ -n "$formatted" ]; then
            formatted="$formatted, \"$trim_i\""
        else
            formatted="\"$trim_i\""
        fi
    done
    API_KEYS="[$formatted]"
fi

# =========== ADVANCED SETTINGS (Full Setup Only) ===========
POOL_STRATEGY="round_robin"
COMPAT_MODE="true"
RUNTIME_BIND_HOST="127.0.0.1"
RUNTIME_PORT_START="18080"

if [ "$is_full_setup" = true ]; then
    echo ""
    echo "--- Advanced Settings ---"
    
    # Pool Strategy
    read -p "4. POOL_STRATEGY (round_robin / random / least_fail) [default: round_robin]: " input_pool_strategy
    if [ -n "$input_pool_strategy" ]; then
        POOL_STRATEGY="$input_pool_strategy"
    fi

    # Compat Mode
    read -p "5. COMPAT_MODE enable OpenAI bridging? (true / false) [default: true]: " input_compat_mode
    if [ -n "$input_compat_mode" ]; then
        COMPAT_MODE="$input_compat_mode"
    fi
    
    # Bind Host
    read -p "6. RUNTIME_BIND_HOST (127.0.0.1 for local, 0.0.0.0 for public access) [default: 127.0.0.1]: " input_bind
    if [ -n "$input_bind" ]; then
        RUNTIME_BIND_HOST="$input_bind"
    fi

    # Port
    read -p "7. RUNTIME_PORT_START (local listening port) [default: 18080]: " input_port
    if [ -n "$input_port" ]; then
        RUNTIME_PORT_START="$input_port"
    fi
fi

# Generate .env
cp .env.example .env

# Update values safely using sed
sed -i.bak "s|^AUTH_TOKEN=.*|AUTH_TOKEN=${AUTH_TOKEN}|g" .env
sed -i.bak "s|^ALLOWED_TOKENS=.*|ALLOWED_TOKENS=${ALLOWED_TOKENS}|g" .env
sed -i.bak "s|^API_KEYS=.*|API_KEYS=${API_KEYS}|g" .env

if [ "$is_full_setup" = true ]; then
    sed -i.bak "s|^POOL_STRATEGY=.*|POOL_STRATEGY=${POOL_STRATEGY}|g" .env
    sed -i.bak "s|^COMPAT_MODE=.*|COMPAT_MODE=${COMPAT_MODE}|g" .env
    sed -i.bak "s|^RUNTIME_BIND_HOST=.*|RUNTIME_BIND_HOST=${RUNTIME_BIND_HOST}|g" .env
    sed -i.bak "s|^RUNTIME_PORT_START=.*|RUNTIME_PORT_START=${RUNTIME_PORT_START}|g" .env
fi

rm -f .env.bak

echo ""
echo "✅ Setup Complete! Your .env file has been generated successfully."
echo "You can view or edit all advanced settings directly in the '.env' file."
echo ""
echo "To start the server, run:"
echo "👉 ./start-headless.sh (for Server headless mode)"
echo "👉 ./start-desktop.sh  (for GUI dashboard mode)"
echo ""
