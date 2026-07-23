import React, { useState, useCallback } from 'react';
import { Shield, Activity } from 'lucide-react';
import TokenAnalyzer from './components/TokenAnalyzer';
import RiskDashboard from './components/RiskDashboard';
import WalletConnect from './components/WalletConnect';
import { Web3Provider } from './context/Web3Provider';
import { CircularBuffer } from './utils/memoryPool';
import './index.css';

function App() {
  // Use useRef to maintain circular buffer across renders without causing re-renders
  const tokenBufferRef = React.useRef(new CircularBuffer(50));
  const [analyzedTokens, setAnalyzedTokens] = useState([]);

  // Memoized callback to prevent unnecessary re-renders
  const handleAnalysisComplete = useCallback((result) => {
    // Add to circular buffer for efficient memory management
    tokenBufferRef.current.push(result);
    
    // Only update state when buffer changes (throttled)
    setAnalyzedTokens(tokenBufferRef.current.toArray());
  }, []);

  return (
    <div className="min-h-screen text-white">
      {/* Header */}
      <header className="glass-card m-4 p-6">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            <div className="p-3 bg-primary-500 rounded-xl">
              <Shield className="w-8 h-8" />
            </div>
            <div>
              <h1 className="text-2xl font-bold">Rug Pull Detector</h1>
              <p className="text-gray-400 text-sm">DeFi Token Scam Predictive Analytics</p>
            </div>
          </div>
          <div className="flex items-center gap-4">
            <div className="flex items-center gap-2 text-sm text-gray-400">
              <Activity className="w-4 h-4" />
              <span>API Status: Online</span>
            </div>
            <WalletConnect />
          </div>
        </div>
      </header>

      {/* Main Content */}
      <main className="container mx-auto px-4 py-8">
        <div className="grid lg:grid-cols-2 gap-8">
          {/* Token Analyzer */}
          <div>
            <TokenAnalyzer onAnalysisComplete={handleAnalysisComplete} />
          </div>

          {/* Risk Dashboard */}
          <div>
            <RiskDashboard analyzedTokens={analyzedTokens} />
          </div>
        </div>
      </main>

      {/* Footer */}
      <footer className="glass-card m-4 p-4 text-center text-gray-400 text-sm">
        <p>Built with Rust + React | Predictive Modeling for DeFi Security</p>
      </footer>
    </div>
  );
}

export default function AppWrapper() {
  return (
    <Web3Provider>
      <App />
    </Web3Provider>
  );
}
