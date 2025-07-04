#!/bin/bash

echo "JauAuth Sandbox Tools Installer"
echo "==============================="
echo ""
echo "This script will install sandboxing tools for JauAuth."
echo "You'll need sudo privileges to install system packages."
echo ""
echo "Available sandboxing tools:"
echo "1. Firejail (Recommended for development)"
echo "2. Bubblewrap (Alternative lightweight option)"
echo "3. Docker (Best for production)"
echo "4. Podman (Rootless Docker alternative)"
echo ""

# Check if running with sudo
if [ "$EUID" -ne 0 ]; then 
    echo "Please run this script with sudo:"
    echo "  sudo ./install-sandbox-tools.sh"
    exit 1
fi

echo "Which tools would you like to install?"
echo "1) Firejail only (lightweight, recommended)"
echo "2) Firejail + Bubblewrap"
echo "3) Docker"
echo "4) All tools"
read -p "Enter your choice (1-4): " choice

# Update package list
echo "Updating package list..."
apt update

case $choice in
    1)
        echo "Installing Firejail..."
        apt install -y firejail
        ;;
    2)
        echo "Installing Firejail and Bubblewrap..."
        apt install -y firejail bubblewrap
        ;;
    3)
        echo "Installing Docker..."
        apt install -y docker.io
        systemctl enable docker
        systemctl start docker
        echo "Note: Add your user to the docker group with:"
        echo "  sudo usermod -aG docker $USER"
        ;;
    4)
        echo "Installing all sandboxing tools..."
        apt install -y firejail bubblewrap docker.io
        # Try to install podman if available
        apt install -y podman || echo "Podman not available in repositories"
        systemctl enable docker
        systemctl start docker
        ;;
    *)
        echo "Invalid choice"
        exit 1
        ;;
esac

echo ""
echo "Installation complete! Running sandbox check..."
echo ""

# Run sandbox check as the original user
sudo -u $SUDO_USER ./target/release/sandbox-check

echo ""
echo "You can now use the installed sandbox strategies in your router-config.json"