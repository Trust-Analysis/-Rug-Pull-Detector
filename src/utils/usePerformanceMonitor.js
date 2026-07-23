import { useEffect, useRef } from 'react';

/**
 * Custom hook for monitoring performance and memory usage
 * Useful for tracking high-frequency data processing operations
 */
const usePerformanceMonitor = (name, data) => {
  const renderCountRef = useRef(0);
  const lastRenderTimeRef = useRef(performance.now());

  useEffect(() => {
    renderCountRef.current++;
    const now = performance.now();
    const timeSinceLastRender = now - lastRenderTimeRef.current;
    lastRenderTimeRef.current = now;

    // Log performance metrics in development
    if (process.env.NODE_ENV === 'development') {
      if (timeSinceLastRender > 16) { // More than 1 frame (60fps)
        console.warn(
          `[${name}] Slow render detected: ${timeSinceLastRender.toFixed(2)}ms, ` +
          `Render count: ${renderCountRef.current}, ` +
          `Data size: ${Array.isArray(data) ? data.length : 'N/A'}`
        );
      }
    }

    // Memory usage tracking (if available)
    if (window.performance && window.performance.memory) {
      const memory = window.performance.memory;
      const usedMB = Math.round(memory.usedJSHeapSize / 1048576 * 100) / 100;
      const totalMB = Math.round(memory.totalJSHeapSize / 1048576 * 100) / 100;
      
      if (usedMB > 50) { // Warn if using more than 50MB
        console.warn(`[${name}] High memory usage: ${usedMB}MB / ${totalMB}MB`);
      }
    }
  });

  return {
    renderCount: renderCountRef.current,
    getMemoryUsage: () => {
      if (window.performance && window.performance.memory) {
        return {
          used: window.performance.memory.usedJSHeapSize,
          total: window.performance.memory.totalJSHeapSize,
          limit: window.performance.memory.jsHeapSizeLimit,
        };
      }
      return null;
    }
  };
};

/**
 * Hook for tracking data processing performance
 */
const useDataProcessingMonitor = (processingFn, dependencies = []) => {
  const startTimeRef = useRef(null);
  const callCountRef = useRef(0);

  const start = () => {
    startTimeRef.current = performance.now();
  };

  const end = (label = 'Operation') => {
    if (startTimeRef.current) {
      const duration = performance.now() - startTimeRef.current;
      callCountRef.current++;
      
      if (process.env.NODE_ENV === 'development' && duration > 10) {
        console.log(`[${label}] took ${duration.toFixed(2)}ms (call #${callCountRef.current})`);
      }
      
      startTimeRef.current = null;
      return duration;
    }
    return 0;
  };

  return { start, end, callCount: callCountRef.current };
};

export { usePerformanceMonitor, useDataProcessingMonitor };