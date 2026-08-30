'use client';

import { useState, useCallback } from 'react';
import { Search, Loader2, AlertCircle, RefreshCw, Shield, Settings, FileKey } from 'lucide-react';
import { useChain } from '../context/ChainProvider';
import { buildReportHref } from '../lib/report';
import ZKPrivacyDisclosure from './ZKPrivacyDisclosure';
import ProxyPatternDisclosure from './ProxyPatternDisclosure';
import PermitSecurityDisclosure from './PermitSecurityDisclosure';

function TokenAnalyzer({ onAnalysisComplete }) {
  const { activeAdapter, activeChainId } = useChain();
  
  const [tokenAddress, setTokenAddress] = useState('');
  const [autoFetched, setAutoFetched] = useState(null);
  const [formOverrides, setFormOverrides] = useState({
    totalSupply: '',
    creatorBalance: '',
    lockedLiquidity: '',
    totalLiquidity: '',
    isPotentialHoneypot: null,
  });
  const [poolType, setPoolType] = useState('standard');
  const [isLendingPool, setIsLendingPool] = useState(false);
  const [loading, setLoading] = useState(false);
  const [fetchingData, setFetchingData] = useState(false);
  const [error, setError] = useState('');
  const [zkAnalysisResult, setZkAnalysisResult] = useState(null);
  const [analyzingZK, setAnalyzingZK] = useState(false);
  const [proxyAnalysisResult, setProxyAnalysisResult] = useState(null);
  const [analyzingProxy, setAnalyzingProxy] = useState(false);
  const [permitAnalysisResult, setPermitAnalysisResult] = useState(null);
  const [analyzingPermit, setAnalyzingPermit] = useState(false);

  // Helper: Get the final inputs, combining auto-fetched data with overrides
  const getFinalInputs = useCallback(() => {
    if (!autoFetched) {
      return {
        tokenAddress,
        ...formOverrides,
      };
    }
    
    return {
      tokenAddress: autoFetched.tokenAddress,
      tokenSymbol: autoFetched.tokenSymbol,
      totalSupply: formOverrides.totalSupply || String(autoFetched.totalSupply),
      creatorBalance: formOverrides.creatorBalance || String(autoFetched.creatorBalance),
      lockedLiquidity: formOverrides.lockedLiquidity || String(autoFetched.lockedLiquidity),
      totalLiquidity: formOverrides.totalLiquidity || String(autoFetched.totalLiquidity),
      isPotentialHoneypot:
        formOverrides.isPotentialHoneypot ?? autoFetched.isPotentialHoneypot,
    };
  }, [autoFetched, formOverrides, tokenAddress]);

  const handleFetchData = useCallback(async () => {
    if (!tokenAddress) return;
    setFetchingData(true);
    setError('');
    try {
      const riskInput = await activeAdapter.analyzeRiskForToken(tokenAddress);
      setAutoFetched(riskInput);
      setFormOverrides({
        totalSupply: '',
        creatorBalance: '',
        lockedLiquidity: '',
        totalLiquidity: '',
        isPotentialHoneypot: null,
      });
    } catch (err) {
      console.error(err);
      setError(`Failed to fetch data from ${activeChainId}: ${err.message}`);
    } finally {
      setFetchingData(false);
    }
  }, [tokenAddress, activeAdapter, activeChainId]);

  const handleSubmit = async (e) => {
    e.preventDefault();
    setLoading(true);
    setError('');

    try {
      const inputs = getFinalInputs();
      
      const response = await fetch('/api/analyze', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          tokenAddress: inputs.tokenAddress,
          totalSupply: inputs.totalSupply,
          creatorBalance: inputs.creatorBalance,
          lockedLiquidity: inputs.lockedLiquidity,
          totalLiquidity: inputs.totalLiquidity,
          isPotentialHoneypot: inputs.isPotentialHoneypot,
          chainId: activeChainId,
          normalizedChainData: autoFetched?.rawChainData || null,
          isLendingPool,
          poolType,
        }),
      });
      const payload = await response.json();

      if (payload.success) {
        onAnalysisComplete({
          ...payload.data,
          timestamp: new Date().toISOString(),
          reportHref: buildReportHref({
            tokenAddress: inputs.tokenAddress,
            totalSupply: inputs.totalSupply,
            creatorBalance: inputs.creatorBalance,
            lockedLiquidity: inputs.lockedLiquidity,
            totalLiquidity: inputs.totalLiquidity,
            isPotentialHoneypot: inputs.isPotentialHoneypot,
            chainId: activeChainId,
          }),
        });
        
        setTokenAddress('');
        setAutoFetched(null);
        setFormOverrides({
          totalSupply: '',
          creatorBalance: '',
          lockedLiquidity: '',
          totalLiquidity: '',
          isPotentialHoneypot: null,
        });
        setZkAnalysisResult(null);
        setProxyAnalysisResult(null);
        setPermitAnalysisResult(null);
      } else {
        setError(payload.error || 'Analysis failed');
      }
    } catch (_error) {
      setError('Failed to connect to API server. Make sure the Rust backend is running.');
    } finally {
      setLoading(false);
    }
  };

  const handleOverrideChange = (e) => {
    const { name, value, type, checked } = e.target;
    setFormOverrides((prev) => ({
      ...prev,
      [name]: type === 'checkbox' ? checked : value,
    }));
  };

  const handleZKAnalysis = async () => {
    if (!tokenAddress) return;
    setAnalyzingZK(true);
    setError('');
    
    try {
      const response = await fetch('/api/zk-verification-analyze', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          contractId: tokenAddress,
          bytecode: null, // Would be fetched from blockchain in production
          shieldedPoolInfo: {
            contract_id: tokenAddress,
            total_shielded: 0,
            commitment_tree_depth: 0,
            recent_proof_count: 0,
            verification_enabled: true
          }
        }),
      });
      
      const payload = await response.json();
      
      if (response.ok) {
        setZkAnalysisResult(payload);
      } else {
        setError(payload.error || 'ZK analysis failed');
      }
    } catch (err) {
      setError('Failed to analyze ZK verification: ' + err.message);
    } finally {
      setAnalyzingZK(false);
    }
  };

  const handleProxyAnalysis = async () => {
    if (!tokenAddress) return;
    setAnalyzingProxy(true);
    setError('');
    
    try {
      const response = await fetch('/api/proxy-pattern-analyze', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          contractAddress: tokenAddress,
          rpcUrl: null // Would use default RPC in production
        }),
      });
      
      const payload = await response.json();
      
      if (response.ok) {
        setProxyAnalysisResult(payload);
      } else {
        setError(payload.error || 'Proxy pattern analysis failed');
      }
    } catch (err) {
      setError('Failed to analyze proxy pattern: ' + err.message);
    } finally {
      setAnalyzingProxy(false);
    }
  };

  const handlePermitAnalysis = async () => {
    if (!tokenAddress) return;
    setAnalyzingPermit(true);
    setError('');
    
    try {
      const response = await fetch('/api/permit-security-analyze', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          contractAddress: tokenAddress,
          rpcUrl: null // Would use default RPC in production
        }),
      });
      
      const payload = await response.json();
      
      if (response.ok) {
        setPermitAnalysisResult(payload);
      } else {
        setError(payload.error || 'Permit security analysis failed');
      }
    } catch (err) {
      setError('Failed to analyze permit security: ' + err.message);
    } finally {
      setAnalyzingPermit(false);
    }
  };

  const getLabelForAddressField = () => {
    switch (activeChainId) {
      case 'stellar':
        return 'Stellar Asset (Code:Issuer)';
      case 'ethereum':
        return 'Token Contract Address';
      default:
        return 'Asset / Token Address';
    }
  };

  const getPlaceholderForAddressField = () => {
    switch (activeChainId) {
      case 'stellar':
        return 'USDC:GA5ZSEJYB37JRC52Z40060EQ11SVF4XI...';
      case 'ethereum':
        return '0x...';
      default:
        return 'Enter address...';
    }
  };

  return (
    <div className="glass-card p-6">
      <h2 className="text-xl font-bold mb-6 flex items-center gap-2">
        <Search className="w-5 h-5 text-primary-400" />
        Token Analyzer
      </h2>

      {error && (
        <div className="mb-4 p-4 bg-danger-500/20 border border-danger-500/50 rounded-lg flex items-center gap-2">
          <AlertCircle className="w-5 h-5 text-danger-400" />
          <span className="text-danger-300">{error}</span>
        </div>
      )}

      <form onSubmit={handleSubmit} className="space-y-4">
        {/* Asset Address Field + Fetch Data Button */}
        <div className="space-y-2">
          <label className="block text-sm font-medium text-gray-300">
            {getLabelForAddressField()}
          </label>
          <div className="flex gap-2">
            <input
              type="text"
              value={tokenAddress}
              onChange={(e) => setTokenAddress(e.target.value)}
              placeholder={getPlaceholderForAddressField()}
              className="flex-1 px-4 py-3 bg-white/5 border border-white/10 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500 text-white placeholder-gray-500"
              required
            />
            <button
              type="button"
              onClick={handleFetchData}
              disabled={!tokenAddress || fetchingData}
              className="px-4 py-3 bg-primary-600 hover:bg-primary-700 disabled:bg-primary-800 disabled:cursor-not-allowed rounded-lg font-medium transition-colors flex items-center gap-2"
            >
              {fetchingData ? (
                <Loader2 className="w-5 h-5 animate-spin" />
              ) : (
                <RefreshCw className="w-5 h-5" />
              )}
              <span>Fetch</span>
            </button>
          </div>
          {autoFetched && (
            <div className="p-3 bg-success-500/20 border border-success-500/50 rounded-lg">
              <div className="text-sm text-success-300">
                Data auto-fetched from chain! Adjust values below if needed.
              </div>
            </div>
          )}
        </div>

        {/* Editable Risk Input Fields */}
        <div className="grid grid-cols-2 gap-4">
          <div>
            <label className="block text-sm font-medium text-gray-300 mb-2">
              Total Supply
            </label>
            <input
              type="number"
              name="totalSupply"
              value={formOverrides.totalSupply}
              placeholder={autoFetched ? String(autoFetched.totalSupply) : '1000000'}
              onChange={handleOverrideChange}
              className="w-full px-4 py-3 bg-white/5 border border-white/10 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500 text-white placeholder-gray-500"
            />
          </div>
          <div>
            <label className="block text-sm font-medium text-gray-300 mb-2">
              Creator Balance
            </label>
            <input
              type="number"
              name="creatorBalance"
              value={formOverrides.creatorBalance}
              placeholder={autoFetched ? String(autoFetched.creatorBalance) : '50000'}
              onChange={handleOverrideChange}
              className="w-full px-4 py-3 bg-white/5 border border-white/10 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500 text-white placeholder-gray-500"
            />
          </div>
        </div>

        <div className="grid grid-cols-2 gap-4">
          <div>
            <label className="block text-sm font-medium text-gray-300 mb-2">
              Locked Liquidity
            </label>
            <input
              type="number"
              name="lockedLiquidity"
              value={formOverrides.lockedLiquidity}
              placeholder={autoFetched ? String(autoFetched.lockedLiquidity) : '900000'}
              onChange={handleOverrideChange}
              className="w-full px-4 py-3 bg-white/5 border border-white/10 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500 text-white placeholder-gray-500"
            />
          </div>
          <div>
            <label className="block text-sm font-medium text-gray-300 mb-2">
              Total Liquidity
            </label>
            <input
              type="number"
              name="totalLiquidity"
              value={formOverrides.totalLiquidity}
              placeholder={autoFetched ? String(autoFetched.totalLiquidity) : '1000000'}
              onChange={handleOverrideChange}
              className="w-full px-4 py-3 bg-white/5 border border-white/10 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500 text-white placeholder-gray-500"
            />
          </div>
        </div>

        <div className="flex items-center gap-3">
          <input
            type="checkbox"
            name="isPotentialHoneypot"
            id="honeypot"
            checked={
              formOverrides.isPotentialHoneypot ?? Boolean(autoFetched?.isPotentialHoneypot)
            }
            onChange={handleOverrideChange}
            className="w-5 h-5 rounded bg-white/5 border-white/10 text-primary-500 focus:ring-primary-500"
          />
          <label htmlFor="honeypot" className="text-sm text-gray-300">
            Potential Honeypot Detected
          </label>
        </div>

        {/* Lending Pool Detection */}
        <div className="space-y-3 p-4 bg-white/5 border border-white/10 rounded-lg">
          <div className="flex items-center gap-3">
            <input
              type="checkbox"
              id="isLendingPool"
              checked={isLendingPool}
              onChange={(e) => setIsLendingPool(e.target.checked)}
              className="w-5 h-5 rounded bg-white/5 border-white/10 text-primary-500 focus:ring-primary-500"
            />
            <label htmlFor="isLendingPool" className="text-sm font-medium text-gray-300">
              This is a Lending Pool
            </label>
          </div>

          {isLendingPool && (
            <div>
              <label className="block text-sm font-medium text-gray-300 mb-2">
                Pool Type
              </label>
              <select
                value={poolType}
                onChange={(e) => setPoolType(e.target.value)}
                className="w-full px-4 py-2 bg-white/5 border border-white/10 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500 text-white"
              >
                <option value="standard">Standard Lending</option>
                <option value="liquidity">Liquidity Pool</option>
                <option value="stable">Stable Pool</option>
                <option value="yield">Yield Pool</option>
                <option value="rwa">RWA Tokenized</option>
              </select>
              <p className="mt-2 text-xs text-gray-400">
                Enables specialized risk scoring for lending pools including TVL tracking, collateral withdrawal detection, and oracle manipulation monitoring.
              </p>
            </div>
          )}
        </div>

        {/* ZK Privacy Analysis */}
        <div className="space-y-3 p-4 bg-white/5 border border-white/10 rounded-lg">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-3">
              <Shield className="w-5 h-5 text-primary-400" />
              <span className="text-sm font-medium text-gray-300">
                Zero-Knowledge Privacy Analysis
              </span>
            </div>
            <button
              type="button"
              onClick={handleZKAnalysis}
              disabled={!tokenAddress || analyzingZK}
              className="px-4 py-2 bg-primary-600 hover:bg-primary-700 disabled:bg-primary-800 disabled:cursor-not-allowed rounded-lg text-sm font-medium transition-colors flex items-center gap-2"
            >
              {analyzingZK ? (
                <>
                  <Loader2 className="w-4 h-4 animate-spin" />
                  Analyzing...
                </>
              ) : (
                <>
                  <Shield className="w-4 h-4" />
                  Analyze Privacy
                </>
              )}
            </button>
          </div>
          <p className="text-xs text-gray-400">
            Analyze ZK verification contracts for cryptographic pairings, commitment tree integrity, and privacy risks.
          </p>
        </div>

        {/* ZK Privacy Disclosure Component */}
        {zkAnalysisResult && (
          <ZKPrivacyDisclosure
            contractId={tokenAddress}
            zkAnalysisResult={zkAnalysisResult}
            onRefresh={handleZKAnalysis}
          />
        )}

        {/* Proxy Pattern Analysis */}
        <div className="space-y-3 p-4 bg-white/5 border border-white/10 rounded-lg">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-3">
              <Settings className="w-5 h-5 text-primary-400" />
              <span className="text-sm font-medium text-gray-300">
                Proxy Pattern Analysis
              </span>
            </div>
            <button
              type="button"
              onClick={handleProxyAnalysis}
              disabled={!tokenAddress || analyzingProxy}
              className="px-4 py-2 bg-primary-600 hover:bg-primary-700 disabled:bg-primary-800 disabled:cursor-not-allowed rounded-lg text-sm font-medium transition-colors flex items-center gap-2"
            >
              {analyzingProxy ? (
                <>
                  <Loader2 className="w-4 h-4 animate-spin" />
                  Analyzing...
                </>
              ) : (
                <>
                  <Settings className="w-4 h-4" />
                  Analyze Proxy
                </>
              )}
            </button>
          </div>
          <p className="text-xs text-gray-400">
            Analyze proxy contracts (EIP-1967, EIP-897, Beacon) for implementation changes and timelock governance.
          </p>
        </div>

        {/* Proxy Pattern Disclosure Component */}
        {proxyAnalysisResult && (
          <ProxyPatternDisclosure
            contractAddress={tokenAddress}
            proxyAnalysisResult={proxyAnalysisResult}
            onRefresh={handleProxyAnalysis}
          />
        )}

        {/* Permit Security Analysis */}
        <div className="space-y-3 p-4 bg-white/5 border border-white/10 rounded-lg">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-3">
              <FileKey className="w-5 h-5 text-primary-400" />
              <span className="text-sm font-medium text-gray-300">
                Permit Security Analysis
              </span>
            </div>
            <button
              type="button"
              onClick={handlePermitAnalysis}
              disabled={!tokenAddress || analyzingPermit}
              className="px-4 py-2 bg-primary-600 hover:bg-primary-700 disabled:bg-primary-800 disabled:cursor-not-allowed rounded-lg text-sm font-medium transition-colors flex items-center gap-2"
            >
              {analyzingPermit ? (
                <>
                  <Loader2 className="w-4 h-4 animate-spin" />
                  Analyzing...
                </>
              ) : (
                <>
                  <FileKey className="w-4 h-4" />
                  Analyze Permit
                </>
              )}
            </button>
          </div>
          <p className="text-xs text-gray-400">
            Analyze EIP-712/EIP-2612 permit functions for domain separator attacks, nonce replay vulnerabilities, and signature validation issues.
          </p>
        </div>

        {/* Permit Security Disclosure Component */}
        {permitAnalysisResult && (
          <PermitSecurityDisclosure
            contractAddress={tokenAddress}
            permitAnalysisResult={permitAnalysisResult}
            onRefresh={handlePermitAnalysis}
          />
        )}

        <button
          type="submit"
          disabled={loading}
          className="w-full py-3 px-4 bg-primary-600 hover:bg-primary-700 disabled:bg-primary-800 disabled:cursor-not-allowed rounded-lg font-medium transition-colors flex items-center justify-center gap-2"
        >
          {loading ? (
            <>
              <Loader2 className="w-5 h-5 animate-spin" />
              Analyzing...
            </>
          ) : (
            <>
              <Search className="w-5 h-5" />
              Analyze Token
            </>
          )}
        </button>
      </form>

      <p className="mt-4 text-xs text-gray-400">
        Successful analyses generate shareable public report routes with server-rendered metadata.
      </p>
    </div>
  );
}

export default TokenAnalyzer;
