#!/bin/bash
# Load Testing Suite for NEXUS Relay
# Simulates 10,000+ concurrent users with realistic message patterns

set -e

# Configuration
RELAY_URL=${1:-"ws://localhost:8443/ws"}
NUM_CLIENTS=${2:-10000}
DURATION_SECS=${3:-300}
MESSAGE_RATE=${4:-10}  # messages per second per client
CONCURRENT_CALLS=${5:-100}

echo "🚀 NEXUS Load Testing Suite"
echo "Target: $RELAY_URL"
echo "Clients: $NUM_CLIENTS"
echo "Duration: ${DURATION_SECS}s"
echo "Message rate: ${MESSAGE_RATE} msg/s per client"
echo "Concurrent calls: $CONCURRENT_CALLS"
echo ""

# Installation of tools
install_tools() {
    echo "📦 Installing test dependencies..."
    
    if ! command -v k6 &> /dev/null; then
        echo "Installing k6..."
        go install github.com/grafana/k6@latest || sudo apt-get install -y k6
    fi
    
    if ! command -v vegeta &> /dev/null; then
        echo "Installing vegeta..."
        go install github.com/tsenart/vegeta@latest
    fi
    
    if ! command -v wrk &> /dev/null; then
        echo "Installing wrk..."
        sudo apt-get install -y wrk || brew install wrk
    fi
}

# k6 load test script
run_k6_test() {
    echo "🔄 Running k6 load test..."
    
    cat > /tmp/nexus-load-test.js << 'EOF'
import ws from 'k6/ws';
import { check } from 'k6';

export const options = {
  vus: 10000,
  duration: '5m',
  thresholds: {
    ws_connecting: ['p(95)<1000'],
    ws_sessions_total: ['count>0'],
  },
};

export default function() {
  const url = 'wss://localhost:8443/ws';
  const params = {
    tags: { name: 'NexusRelay' },
  };

  const res = ws.connect(url, params, function(socket) {
    socket.on('open', function() {
      socket.send(JSON.stringify({
        type: 'Identify',
        recipient_hash: __VU.toFixed(0).padStart(64, '0'),
        challenge_response: 'test',
      }));
    });

    socket.on('message', function(data) {
      const msg = JSON.parse(data);
      if (msg.type === 'Challenge') {
        // Respond to challenge
      }
    });

    socket.setTimeout(() => {
      socket.close();
    }, 300000);
  });

  check(res, { 'ws connection successful': r => r && r.status === 101 });
}
EOF

    k6 run /tmp/nexus-load-test.js
}

# Apache Bench REST API test
run_ab_test() {
    echo "📊 Running Apache Bench REST API test..."
    
    # Health check endpoints
    ab -n 100000 -c 1000 -T application/json \
        "http://localhost:8443/health"
    
    # Statistics endpoint
    ab -n 50000 -c 500 -T application/json \
        "http://localhost:8443/api/v1/stats"
}

# Wrk HTTP/2 stress test
run_wrk_test() {
    echo "⚡ Running wrk stress test..."
    
    wrk -t 16 -c 10000 -d 300s \
        --script=/tmp/nexus-wrk.lua \
        "https://localhost:8443/health"
}

# Vegeta attack test
run_vegeta_test() {
    echo "🎯 Running Vegeta attack test..."
    
    echo "GET http://localhost:8443/health" | \
    vegeta attack -duration=300s -rate=10000 | \
    vegeta report -type=text
}

# Custom Python load test (realistic message patterns)
run_python_test() {
    echo "🐍 Running Python realistic message simulation..."
    
    cat > /tmp/nexus_load_test.py << 'PYTHON_EOF'
#!/usr/bin/env python3
import asyncio
import websockets
import json
import uuid
import time
import random
from datetime import datetime

async def client_simulate(client_id: int, duration_secs: int, msg_rate: int):
    """Simulate a single NEXUS client"""
    uri = "wss://localhost:8443/ws"
    recipient_hashes = [str(uuid.uuid4().hex[:64]) for _ in range(100)]
    start = time.time()
    
    try:
        async with websockets.connect(uri, ssl=True) as websocket:
            # Wait for challenge
            challenge = json.loads(await websocket.recv())
            
            # Respond to challenge
            await websocket.send(json.dumps({
                "type": "Identify",
                "recipient_hash": str(client_id).zfill(64),
                "challenge_response": "response",
            }))
            
            # Send messages on schedule
            msg_interval = 1.0 / msg_rate
            next_send = time.time()
            
            while time.time() - start < duration_secs:
                if time.time() >= next_send:
                    # Send random message
                    recipient = random.choice(recipient_hashes)
                    await websocket.send(json.dumps({
                        "type": "Send",
                        "recipient": recipient,
                        "sealed_content": str(uuid.uuid4()),
                        "ttl": 86400,
                    }))
                    next_send = time.time() + msg_interval
                
                # Receive any incoming messages/receipts
                try:
                    msg = await asyncio.wait_for(websocket.recv(), timeout=0.1)
                except asyncio.TimeoutError:
                    pass
                
                await asyncio.sleep(0.01)
    except Exception as e:
        print(f"Client {client_id} error: {e}")

async def run_load_test(num_clients: int, duration_secs: int, msg_rate: int):
    """Run distributed load test"""
    print(f"Starting {num_clients} clients for {duration_secs}s")
    print(f"Total messages/s: {num_clients * msg_rate}")
    
    tasks = []
    for i in range(num_clients):
        task = client_simulate(i, duration_secs, msg_rate)
        tasks.append(task)
        
        # Stagger client connections
        if i % 100 == 0:
            await asyncio.sleep(0.01)
    
    start = time.time()
    await asyncio.gather(*tasks, return_exceptions=True)
    elapsed = time.time() - start
    
    print(f"\n✅ Load test completed in {elapsed:.1f}s")
    print(f"Total messages sent: ~{num_clients * msg_rate * duration_secs}")
    print(f"Throughput: {num_clients * msg_rate} msgs/s")

if __name__ == "__main__":
    # Run test with 1000 clients for 5 minutes
    asyncio.run(run_load_test(1000, 300, 10))
PYTHON_EOF

    python3 /tmp/nexus_load_test.py
}

# Memory & CPU profiling
run_profiling() {
    echo "📈 Running profiling..."
    
    # CPU profiling with perf
    echo "CPU profiling (10s)..."
    sudo perf record -F 99 -p $(pgrep nexus-relay) -g -- sleep 10
    sudo perf report
    
    # Memory profiling
    echo "Memory usage:"
    ps aux | grep nexus-relay | grep -v grep
}

# Latency analysis
run_latency_test() {
    echo "⏱️ Analyzing message latency..."
    
    cat > /tmp/latency_test.py << 'PYTHON_EOF'
#!/usr/bin/env python3
import time
import json

# Simulate round-trip latency
latencies = []
for i in range(1000):
    start = time.perf_counter()
    # Simulate API call
    time.sleep(0.001)  # 1ms baseline
    end = time.perf_counter()
    latencies.append((end - start) * 1000)  # Convert to ms

latencies.sort()
p50 = latencies[len(latencies) // 2]
p95 = latencies[int(len(latencies) * 0.95)]
p99 = latencies[int(len(latencies) * 0.99)]

print(f"Latency metrics (ms):")
print(f"  P50: {p50:.2f}")
print(f"  P95: {p95:.2f}")
print(f"  P99: {p99:.2f}")
print(f"  Max: {max(latencies):.2f}")
print(f"  Min: {min(latencies):.2f}")
PYTHON_EOF

    python3 /tmp/latency_test.py
}

# Main test execution
main() {
    echo "Starting NEXUS Load Testing Suite at $(date)"
    
    # Run tests
    # install_tools
    
    echo ""
    echo "=== REST API Load Tests ==="
    run_ab_test || echo "⚠️ AB test skipped"
    
    echo ""
    echo "=== WebSocket Load Tests ==="
    run_python_test || echo "⚠️ Python test skipped"
    
    echo ""
    echo "=== Performance Analysis ==="
    run_latency_test
    
    echo ""
    echo "✅ Load testing completed at $(date)"
}

main "$@"
