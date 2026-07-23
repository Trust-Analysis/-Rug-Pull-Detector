/**
 * Memory Pool Utility for High-Frequency Data Processing
 * 
 * This module provides optimized data structures and utilities for handling
 * frequent token analysis operations with minimal memory allocation overhead.
 */

/**
 * Object pool for reusing token analysis result objects
 * Reduces garbage collection pressure by reusing objects instead of creating new ones
 */
class ObjectPool {
  constructor(createFn, resetFn, initialSize = 10) {
    this.createFn = createFn;
    this.resetFn = resetFn;
    this.pool = [];
    
    // Pre-populate pool
    for (let i = 0; i < initialSize; i++) {
      this.pool.push(this.createFn());
    }
  }

  acquire() {
    if (this.pool.length > 0) {
      return this.pool.pop();
    }
    return this.createFn();
  }

  release(obj) {
    this.resetFn(obj);
    this.pool.push(obj);
  }
}

/**
 * Circular buffer for efficient token history management
 * Provides O(1) insertion and automatic size limiting
 */
class CircularBuffer {
  constructor(maxSize = 100) {
    this.maxSize = maxSize;
    this.buffer = new Array(maxSize);
    this.head = 0;
    this.tail = 0;
    this.size = 0;
  }

  push(item) {
    this.buffer[this.tail] = item;
    this.tail = (this.tail + 1) % this.maxSize;
    
    if (this.size < this.maxSize) {
      this.size++;
    } else {
      this.head = (this.head + 1) % this.maxSize;
    }
  }

toArray() {
    const result = [];
    const start = this.head;
    
    for (let i = 0; i < this.size; i++) {
      result.push(this.buffer[(start + i) % this.maxSize]);
    }
    return result;
  }

  clear() {
    this.head = 0;
    this.tail = 0;
    this.size = 0;
    this.buffer = new Array(this.maxSize);
  }

  get length() {
    return this.size;
  }
}

/**
 * Pre-allocated arrays to avoid repeated array creation
 */
const preallocatedArrays = {
  riskLevels: ['Low', 'Medium', 'High', 'Critical'],
  scoreThresholds: [0.3, 0.6, 0.8],
  
  // Reusable array for stats calculation
  getStatsArray: (() => {
    let arr = new Array(3);
    return () => {
      arr[0] = 0;
      arr[1] = 0;
      arr[2] = 0;
      return arr;
    };
  })()
};

/**
 * Memoized function for risk level color calculation
 * Uses pre-computed values to avoid object creation
 */
const riskLevelColors = {
  Low: 'text-success-400 bg-success-500/20 border-success-500/50',
  Medium: 'text-yellow-400 bg-yellow-500/20 border-yellow-500/50',
  High: 'text-orange-400 bg-orange-500/20 border-orange-500/50',
  Critical: 'text-danger-400 bg-danger-500/20 border-danger-500/50',
  default: 'text-gray-400 bg-gray-500/20 border-gray-500/50'
};

const getRiskLevelColor = (level) => riskLevelColors[level] || riskLevelColors.default;

/**
 * Memoized function for score gradient colors
 */
const scoreGradients = {
  low: 'from-success-500 to-success-400',
  medium: 'from-yellow-500 to-yellow-400',
  high: 'from-orange-500 to-orange-400',
  critical: 'from-danger-500 to-danger-400'
};

const getScoreColor = (score) => {
  if (score < 0.3) return scoreGradients.low;
  if (score < 0.6) return scoreGradients.medium;
  if (score < 0.8) return scoreGradients.high;
  return scoreGradients.critical;
};

/**
 * Efficient stats calculator using single-pass iteration
 * Avoids multiple filter operations on the array
 */
const calculateStats = (tokens) => {
  let total = 0;
  let safe = 0;
  let risky = 0;
  
  for (let i = 0; i < tokens.length; i++) {
    total++;
    const level = tokens[i].riskLevel;
    if (level === 'Low') safe++;
    else if (level === 'High' || level === 'Critical') risky++;
  }
  
  return { total, safe, risky };
};

/**
 * Token data factory for creating normalized token objects
 */
const createTokenResult = (data, tokenAddress) => ({
  ...data,
  tokenAddress,
  timestamp: new Date().toISOString(),
  id: `${tokenAddress}-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`
});

/**
 * Batch processor for handling multiple token analyses
 */
class BatchProcessor {
  constructor(batchSize = 10, processFn) {
    this.batchSize = batchSize;
    this.processFn = processFn;
    this.batch = [];
  }

  add(item) {
    this.batch.push(item);
    if (this.batch.length >= this.batchSize) {
      this.flush();
    }
  }

  flush() {
    if (this.batch.length > 0) {
      this.processFn(this.batch);
      this.batch = [];
    }
  }
}

export {
  ObjectPool,
  CircularBuffer,
  preallocatedArrays,
  getRiskLevelColor,
  getScoreColor,
  calculateStats,
  createTokenResult,
  BatchProcessor
};