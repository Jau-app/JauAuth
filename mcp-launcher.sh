#!/bin/bash
export RUST_BACKEND_URL="http://localhost:7447"
exec node <PATH>/dist/index.js
