import React, { useState, useCallback } from 'react';
import { useWeb3 } from '../context/Web3Provider';
import { Wallet, LogOut, Loader2 } from 'lucide-react';

// Memoized address formatter to avoid string operations on every render
const formatAddress = (addr) => {
  if (!addr) return '';
  return `${addr.slice(0, 6)}...${addr.slice(-4)}`;
};

const WalletConnect = () => {
  const { account, isConnected, connectWallet, disconnectWallet } = useWeb3();
  const [isConnecting, setIsConnecting] = useState(false);

  // Memoized connect handler
  const handleConnect = useCallback(async () => {
    setIsConnecting(true);
    await connectWallet();
    setIsConnecting(false);
  }, [connectWallet]);

  if (isConnecting) {
    return (
      <button className="flex items-center gap-2 px-4 py-2 bg-gray-700 rounded-lg text-sm cursor-not-allowed">
        <Loader2 className="w-4 h-4 animate-spin" />
        <span>Connecting...</span>
      </button>
    );
  }

  if (isConnected) {
    return (
      <div className="flex items-center gap-2">
        <div className="px-4 py-2 bg-green-600/20 border border-green-500/30 rounded-lg text-sm">
          <span className="text-green-400">{formatAddress(account)}</span>
        </div>
        <button
          onClick={disconnectWallet}
          className="flex items-center gap-2 px-4 py-2 bg-red-600/20 border border-red-500/30 rounded-lg text-sm hover:bg-red-600/30 transition-colors"
        >
          <LogOut className="w-4 h-4" />
          <span>Disconnect</span>
        </button>
      </div>
    );
  }

  return (
    <button
      onClick={handleConnect}
      className="flex items-center gap-2 px-4 py-2 bg-primary-600 hover:bg-primary-700 rounded-lg text-sm transition-colors"
    >
      <Wallet className="w-4 h-4" />
      <span>Connect Wallet</span>
    </button>
  );
};

export default WalletConnect;
