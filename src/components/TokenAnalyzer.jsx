import React, { useState, useCallback, useRef } from 'react';
import { Search, Loader2, AlertCircle } from 'lucide-react';
import axios from 'axios';

const API_URL = 'http://127.0.0.1:8080';

// Pre-allocated form data template to reduce memory allocation
const createFormData = () => ({
  tokenAddress: '',
  totalSupply: '',
  creatorBalance: '',
  lockedLiquidity: '',
  totalLiquidity: '',
  isPotentialHoneypot: false,
});

function TokenAnalyzer({ onAnalysisComplete }) {
  const [formData, setFormData] = useState(createFormData());
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');
  
  // Use ref to track pending requests and prevent memory leaks
  const pendingRequestRef = useRef(null);

  const handleSubmit = useCallback(async (e) => {
    e.preventDefault();
    
    // Cancel any pending request
    if (pendingRequestRef.current) {
      pendingRequestRef.current.cancel();
    }
    
    setLoading(true);
    setError('');

    try {
      // Create cancel token for request cleanup
      const source = axios.CancelToken.source();
      pendingRequestRef.current = source;
      
      const response = await axios.post(`${API_URL}/api/analyze`, {
        token_address: formData.tokenAddress,
        total_supply: formData.totalSupply,
        creator_balance: formData.creatorBalance,
        locked_liquidity: formData.lockedLiquidity,
        total_liquidity: formData.totalLiquidity,
        is_potential_honeypot: formData.isPotentialHoneypot,
      }, {
        cancelToken: source.token
      });

      if (response.data.success) {
        const result = {
          ...response.data.data,
          tokenAddress: formData.tokenAddress,
          timestamp: new Date().toISOString(),
          id: `${formData.tokenAddress}-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`,
        };
        onAnalysisComplete(result);
        // Reset form using pre-allocated template
        setFormData(createFormData());
      } else {
        setError(response.data.error || 'Analysis failed');
      }
    } catch (err) {
      if (!axios.isCancel(err)) {
        setError('Failed to connect to API server. Make sure the Rust backend is running.');
      }
    } finally {
      pendingRequestRef.current = null;
      setLoading(false);
    }
  }, [formData, onAnalysisComplete]);

  const handleChange = useCallback((e) => {
    const { name, value, type, checked } = e.target;
    setFormData(prev => ({
      ...prev,
      [name]: type === 'checkbox' ? checked : value,
    }));
  }, []);

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
        <div>
          <label className="block text-sm font-medium text-gray-300 mb-2">
            Stellar Asset (Code:Issuer)
          </label>
          <input
            type="text"
            name="tokenAddress"
            value={formData.tokenAddress}
            onChange={handleChange}
            placeholder="USDC:GA5ZSEJYB37JRC52Z40060EQ11SVF4XI..."
            className="w-full px-4 py-3 bg-white/5 border border-white/10 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500 text-white placeholder-gray-500"
            required
          />
        </div>

        <div className="grid grid-cols-2 gap-4">
          <div>
            <label className="block text-sm font-medium text-gray-300 mb-2">
              Total Supply
            </label>
            <input
              type="number"
              name="totalSupply"
              value={formData.totalSupply}
              onChange={handleChange}
              placeholder="1000000"
              className="w-full px-4 py-3 bg-white/5 border border-white/10 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500 text-white placeholder-gray-500"
              required
            />
          </div>

          <div>
            <label className="block text-sm font-medium text-gray-300 mb-2">
              Creator Balance
            </label>
            <input
              type="number"
              name="creatorBalance"
              value={formData.creatorBalance}
              onChange={handleChange}
              placeholder="50000"
              className="w-full px-4 py-3 bg-white/5 border border-white/10 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500 text-white placeholder-gray-500"
              required
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
              value={formData.lockedLiquidity}
              onChange={handleChange}
              placeholder="900000"
              className="w-full px-4 py-3 bg-white/5 border border-white/10 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500 text-white placeholder-gray-500"
              required
            />
          </div>

          <div>
            <label className="block text-sm font-medium text-gray-300 mb-2">
              Total Liquidity
            </label>
            <input
              type="number"
              name="totalLiquidity"
              value={formData.totalLiquidity}
              onChange={handleChange}
              placeholder="1000000"
              className="w-full px-4 py-3 bg-white/5 border border-white/10 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500 text-white placeholder-gray-500"
              required
            />
          </div>
        </div>

        <div className="flex items-center gap-3">
          <input
            type="checkbox"
            name="isPotentialHoneypot"
            id="honeypot"
            checked={formData.isPotentialHoneypot}
            onChange={handleChange}
            className="w-5 h-5 rounded bg-white/5 border-white/10 text-primary-500 focus:ring-primary-500"
          />
          <label htmlFor="honeypot" className="text-sm text-gray-300">
            Potential Honeypot Detected
          </label>
        </div>

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
    </div>
  );
}

export default TokenAnalyzer;