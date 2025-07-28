#!/bin/bash
# Create directory
sudo mkdir -p /etc/jauauth/netfilter

# Create localhost-only.net
sudo tee /etc/jauauth/netfilter/localhost-only.net > /dev/null << 'EOF'
*filter
:INPUT DROP [0:0]
:OUTPUT DROP [0:0]
-A OUTPUT -d 127.0.0.1/8 -j ACCEPT
-A OUTPUT -d ::1/128 -j ACCEPT
COMMIT
EOF

# Create telegram-only.net
sudo tee /etc/jauauth/netfilter/telegram-only.net > /dev/null << 'EOF'
*filter
:INPUT DROP [0:0]
:OUTPUT DROP [0:0]
-A OUTPUT -d 127.0.0.1/8 -j ACCEPT
-A OUTPUT -d 149.154.160.0/20 -j ACCEPT
-A OUTPUT -d 91.108.4.0/22 -j ACCEPT
-A OUTPUT -p udp --dport 53 -j ACCEPT
COMMIT
EOF

# Set permissions
sudo chmod 644 /etc/jauauth/netfilter/*.net
echo "Netfilter files created successfully"