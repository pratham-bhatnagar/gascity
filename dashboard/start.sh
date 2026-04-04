#!/usr/bin/env bash
# Deepwork Intelligence Dashboard — Start Script
#
# Usage:
#   ./start.sh              # Default port 8090
#   ./start.sh 8091         # Custom port
#   DI_DASHBOARD_PORT=9000 ./start.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PORT="${1:-${DI_DASHBOARD_PORT:-8090}}"

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  DEEPWORK INTELLIGENCE — Dashboard"
echo "  Port: ${PORT}"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

exec python3 "${SCRIPT_DIR}/api.py" --port "${PORT}"
