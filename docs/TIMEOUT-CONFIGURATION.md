# JauAuth Timeout Configuration

## Overview

JauAuth supports custom timeouts for long-running operations through a special `__timeout` parameter that can be added to any tool's arguments. This feature is essential for tools that require extended time to complete, such as approval workflows, searches, or external API calls.

## How It Works

1. **Client adds `__timeout` to tool arguments**
2. **TypeScript MCP server extracts the timeout value**
3. **Creates a request-specific HTTP client with the custom timeout**
4. **Removes `__timeout` before forwarding to the backend**
5. **Backend processes the request normally, unaware of the timeout**

## Using the __timeout Parameter

### Basic Usage

Add `__timeout` to any tool's arguments:

```javascript
// Wait up to 5 minutes for approval
wait_for_approval({
  content: "Please approve this change",
  contentType: "approval",
  __timeout: 300000  // 5 minutes in milliseconds
})

// No timeout - wait indefinitely
wait_for_approval({
  content: "Take your time to review",
  contentType: "question",
  __timeout: "*"  // Special value for no timeout
})

// Works with any tool
jaumemory_consolidate({
  __timeout: 120000  // 2 minutes for memory consolidation
})
```

### Timeout Values

| Value | Description | Example |
|-------|-------------|---------|
| Number | Timeout in milliseconds | `__timeout: 60000` (1 minute) |
| String number | Same as number | `__timeout: "60000"` |
| `"*"` | No timeout - wait indefinitely | `__timeout: "*"` |
| Not specified | Default timeout (30 seconds) | - |

## Implementation Details

### TypeScript MCP Server

The timeout handling is implemented in `mcp-server/src/index.ts`:

```typescript
// Extract timeout parameter if provided
let timeout = appConfig.backend.timeout; // Default timeout
let cleanArgs = args;

if (args && typeof args === 'object' && '__timeout' in args) {
  const timeoutParam = args.__timeout;
  
  // Handle special case: '*' means no timeout
  if (timeoutParam === '*') {
    timeout = 0; // 0 means no timeout in axios
  } else if (typeof timeoutParam === 'number' && timeoutParam > 0) {
    timeout = timeoutParam;
  } else if (typeof timeoutParam === 'string' && !isNaN(parseInt(timeoutParam))) {
    timeout = parseInt(timeoutParam);
  }
  
  // Remove __timeout from arguments before forwarding
  const { __timeout, ...restArgs } = args;
  cleanArgs = restArgs;
}
```

### Tool Schema Enhancement

All tools routed through JauAuth automatically get:
- Enhanced description mentioning the `__timeout` parameter
- Updated input schema with `__timeout` property documentation

## Common Use Cases

### 1. User Approval Workflows
```javascript
wait_for_approval({
  content: "Deploy to production?",
  contentType: "approval",
  initialTimeout: 30,
  maxFollowUps: 3,
  __timeout: 600000  // 10 minutes total
})
```

### 2. Long-Running Searches
```javascript
search_large_dataset({
  query: "complex pattern",
  scope: "all",
  __timeout: 300000  // 5 minutes
})
```

### 3. External API Calls
```javascript
fetch_external_data({
  endpoint: "slow-api.example.com",
  retries: 3,
  __timeout: 120000  // 2 minutes
})
```

### 4. Batch Operations
```javascript
process_batch({
  items: largeArray,
  operation: "transform",
  __timeout: "*"  // No timeout for large batches
})
```

## Testing Timeouts

### Using the test-tool.js Script

```bash
# Test with command-line timeout
node scripts/test-tool.js jau-tg_wait_for_approval \
  '{"content":"Test approval"}' --timeout=300000

# Test with in-argument timeout
node scripts/test-tool.js jau-tg_wait_for_approval \
  '{"content":"Test approval","__timeout":300000}'

# Test with no timeout
node scripts/test-tool.js long_running_tool \
  '{"data":"process","__timeout":"*"}'
```

### Using the test-timeout.js Script

```bash
# Run comprehensive timeout tests
node scripts/test-timeout.js
```

This script tests:
- Quick operations with default timeout
- Operations that timeout as expected
- Long operations with adequate timeout
- No timeout operations
- String timeout values

## Error Handling

When a timeout occurs, users receive a helpful error message:

```
Error: Request timeout: The operation took longer than expected. 
Consider using __timeout parameter for long-running operations.
```

## Best Practices

1. **Set appropriate timeouts**: Don't use `"*"` unless necessary
2. **Consider network latency**: Add buffer time for network operations
3. **Document expected duration**: In your tool descriptions, mention typical execution time
4. **Handle timeouts gracefully**: Implement proper cleanup in your tools
5. **Monitor timeout patterns**: If a tool frequently times out, consider optimizing it

## Troubleshooting

### Tool times out despite setting __timeout

1. Check if the timeout is being passed correctly:
   ```bash
   tail -f /tmp/jauauth-mcp.log | grep "custom timeout"
   ```

2. Verify the backend process is still running:
   ```bash
   ps aux | grep [backend-name]
   ```

3. Check if the issue is with the backend itself:
   - Some backends may have their own internal timeouts
   - The operation might be genuinely taking longer than expected

### __timeout parameter not recognized

Ensure you're using the latest version of JauAuth with TypeScript MCP server:
```bash
cd mcp-server && npm run build
```

## Future Enhancements

1. **Timeout Profiles**: Predefined timeout sets for different operation types
2. **Dynamic Timeout Adjustment**: Automatically adjust based on historical performance
3. **Timeout Warnings**: Warn when approaching timeout limit
4. **Partial Results**: Return partial results when timeout occurs
5. **Retry with Backoff**: Automatic retry with increasing timeouts