{
    "issues": [
      {"id": "issue-001", "title": "Fix login bug", "description": "Users cannot log in"}
    ]
  }'
```

### Example: Stamp Issues

```bash
curl -X POST http://localhost:8080/mcp/v1/agents/wasteland/stamp \
  -H "Content-Type: application/json" \
  -d '{
    "issue_id": "issue-001",
    "stamps": ["priority:high", "component:auth"]
  }'
```

## Agent Reference

### Wasteland Agent

The wasteland agent processes and manages issues within the system.

#### Functions

- **process_issues** — Analyze and route issues through the workflow
- **stamp_issues** — Apply metadata labels to issues
- **map_beads** — Track issue relationships and dependencies

#### Configuration

```python
# agents/wasteland/config.py
WASTELAND_CONFIG = {
    "auto_process": True,
    "stamp_on_process": True,
    "map_dependencies": True,
}