# Rug Pull Detector - Frontend

React frontend for the DeFi Rug Pull Detection System.

## Prerequisites

- Node.js 16+ and npm/yarn
- Rust backend running on http://127.0.0.1:8080

## Installation

```bash
cd frontend
npm install
```

## Development

```bash
npm start
```

The app will open at http://localhost:3000

## Build for Production

```bash
npm run build
```

## Features

- **Token Analyzer**: Input token metrics and get instant risk analysis
- **Risk Dashboard**: View analysis history with visual risk indicators
- **Real-time Updates**: Connects to Rust backend API
- **Modern UI**: Built with TailwindCSS and Lucide icons

## API Endpoints

- `POST /api/analyze` - Analyze a single token
- `POST /api/batch-analyze` - Analyze multiple tokens
- `GET /health` - Health check

## Tech Stack

- React 18
- TailwindCSS
- Lucide React (icons)
- Axios (HTTP client)
- Recharts (charts)

## Memory Optimization

This application implements several memory optimization strategies for high-frequency data processing:

### Key Optimizations

1. **Circular Buffer** (`src/utils/memoryPool.js`)
   - O(1) insertion time for token history
   - Pre-allocated memory prevents dynamic allocation overhead
   - Automatic size limiting prevents memory leaks

2. **Efficient Stats Calculation**
   - Single-pass iteration instead of multiple `filter()` operations
   - No intermediate array creation during calculations

3. **React Component Memoization**
   - `React.memo` for TokenItem and RiskIcon components
   - `useCallback` for event handlers to prevent unnecessary re-renders
   - `useRef` for persistent data without triggering re-renders

4. **Request Cancellation**
   - Axios CancelToken prevents memory leaks from abandoned requests
   - Clean async handling for high-frequency API calls

5. **Consolidated State Management**
   - Single state object for Web3 provider
   - Atomic state updates reduce re-render frequency

### Performance Monitoring

Use the performance monitoring hooks in `src/utils/usePerformanceMonitor.js`:

```javascript
import { usePerformanceMonitor } from './utils/usePerformanceMonitor';

// In your component
const { renderCount, getMemoryUsage } = usePerformanceMonitor('ComponentName', data);
```

See `src/utils/MEMORY_OPTIMIZATION.md` for detailed documentation.
