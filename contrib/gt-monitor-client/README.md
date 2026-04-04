# gt-monitor-client

Lightweight Python SDK for the Gas City memory API. Zero dependencies beyond Python 3.10+.

## Usage

```python
from gt_monitor_client import MemoryClient

client = MemoryClient("http://127.0.0.1:8080", rig="myrig")

# Retrieve relevant memories before starting work
memories = client.recall(q="auth patterns", scope="rig", min_confidence="0.7")
for mem in memories:
    print(f"{mem.id}: {mem.title} (confidence={mem.confidence})")

# Store a new memory after completing work
mem = client.remember(
    "OAuth tokens must be rotated every 30 days",
    kind="pattern",
    confidence=0.9,
    scope="rig",
)
print(f"Stored: {mem.id}")

# Fetch a specific memory
mem = client.get("gc-abc123")
```

## API

### `MemoryClient(base_url, *, rig="", timeout=10.0)`

- `base_url`: Gas City API base URL (e.g., `http://127.0.0.1:8080`)
- `rig`: Default rig for all operations (optional)
- `timeout`: HTTP request timeout in seconds

### `client.recall(**filters) -> list[Memory]`

Query parameters: `q`, `scope`, `kind`, `min_confidence`, `limit`.

### `client.remember(title, **kwargs) -> Memory`

Create a memory. Server defaults: `kind="context"`, `confidence=0.5`, `scope="rig"`.

### `client.get(memory_id) -> Memory`

Fetch a single memory by ID.

## Testing

```bash
python3 -m pytest test_gt_monitor_client.py -v
```
