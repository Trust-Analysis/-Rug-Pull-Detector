import React, { useMemo, memo } from 'react';
import { Shield, AlertTriangle, CheckCircle, XCircle, Clock } from 'lucide-react';
import { getRiskLevelColor, getScoreColor, calculateStats } from '../utils/memoryPool';

// Memoized icon component to prevent unnecessary re-renders
const RiskIcon = memo(({ level }) => {
  switch (level) {
    case 'Low':
      return <CheckCircle className="w-5 h-5" />;
    case 'Medium':
      return <AlertTriangle className="w-5 h-5" />;
    case 'High':
      return <AlertTriangle className="w-5 h-5" />;
    case 'Critical':
      return <XCircle className="w-5 h-5" />;
    default:
      return <Shield className="w-5 h-5" />;
  }
});

// Memoized token item component
const TokenItem = memo(({ token }) => (
  <div className="bg-white/5 rounded-lg p-4 border border-white/10 hover:border-white/20 transition-colors">
    <div className="flex items-start justify-between mb-3">
      <div className="flex-1">
        <div className="font-mono text-sm text-primary-400 mb-1">
          {token.tokenAddress}
        </div>
        <div className="flex items-center gap-2 text-xs text-gray-400">
          <Clock className="w-3 h-3" />
          {new Date(token.timestamp).toLocaleTimeString()}
        </div>
      </div>
      <div className={`px-3 py-1 rounded-full text-xs font-medium border ${getRiskLevelColor(token.riskLevel)} flex items-center gap-1`}>
        <RiskIcon level={token.riskLevel} />
        {token.riskLevel}
      </div>
    </div>

    {/* Risk Score Bar */}
    <div className="mb-3">
      <div className="flex justify-between text-xs mb-1">
        <span className="text-gray-400">Risk Score</span>
        <span className="text-white font-medium">{(token.score * 100).toFixed(1)}%</span>
      </div>
      <div className="h-2 bg-white/10 rounded-full overflow-hidden">
        <div
          className={`h-full bg-gradient-to-r ${getScoreColor(token.score)} transition-all duration-500`}
          style={{ width: `${token.score * 100}%` }}
        />
      </div>
    </div>

    {/* Component Breakdown */}
    <div className="grid grid-cols-3 gap-2 text-xs">
      <div className="bg-white/5 rounded p-2">
        <div className="text-gray-400 mb-1">Creator</div>
        <div className="text-white font-medium">{(token.components.creatorOwnership * 100).toFixed(0)}%</div>
      </div>
      <div className="bg-white/5 rounded p-2">
        <div className="text-gray-400 mb-1">Liquidity</div>
        <div className="text-white font-medium">{(token.components.liquidityLock * 100).toFixed(0)}%</div>
      </div>
      <div className="bg-white/5 rounded p-2">
        <div className="text-gray-400 mb-1">Honeypot</div>
        <div className="text-white font-medium">{(token.components.honeypot * 100).toFixed(0)}%</div>
      </div>
    </div>
  </div>
));

function RiskDashboard({ analyzedTokens }) {
  // Memoize stats calculation to avoid repeated filtering
  const stats = useMemo(() => calculateStats(analyzedTokens), [analyzedTokens]);

  return (
    <div className="glass-card p-6">
      <h2 className="text-xl font-bold mb-6 flex items-center gap-2">
        <Shield className="w-5 h-5 text-primary-400" />
        Risk Dashboard
      </h2>

      {/* Stats Overview */}
      <div className="grid grid-cols-3 gap-4 mb-6">
        <div className="bg-white/5 rounded-lg p-4 text-center">
          <div className="text-2xl font-bold text-white">{stats.total}</div>
          <div className="text-sm text-gray-400">Total Analyzed</div>
        </div>
        <div className="bg-success-500/20 rounded-lg p-4 text-center border border-success-500/50">
          <div className="text-2xl font-bold text-success-400">{stats.safe}</div>
          <div className="text-sm text-success-300">Safe Tokens</div>
        </div>
        <div className="bg-danger-500/20 rounded-lg p-4 text-center border border-danger-500/50">
          <div className="text-2xl font-bold text-danger-400">{stats.risky}</div>
          <div className="text-sm text-danger-300">Risky Tokens</div>
        </div>
      </div>

      {/* Token List */}
      <div className="space-y-3">
        {analyzedTokens.length === 0 ? (
          <div className="text-center py-12 text-gray-400">
            <Shield className="w-12 h-12 mx-auto mb-4 opacity-50" />
            <p>No tokens analyzed yet</p>
            <p className="text-sm">Use the analyzer to check token risks</p>
          </div>
        ) : (
          analyzedTokens.map((token, index) => (
            <TokenItem key={token.id || index} token={token} />
          ))
        )}
      </div>
    </div>
  );
}

export default RiskDashboard;