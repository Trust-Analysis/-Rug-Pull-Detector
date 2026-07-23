import React, { createContext, useContext, useState, useEffect, useCallback, useRef } from 'react';
import { ethers } from 'ethers';

const Web3Context = createContext({});

// Pre-allocated state object to reduce memory allocation
const createWeb3State = () => ({
  account: null,
  provider: null,
  chainId: null,
  isConnected: false,
});

export const Web3Provider = ({ children }) => {
  const [state, setState] = useState(createWeb3State());
  const providerRef = useRef(null);

  // Memoized connect function to prevent unnecessary re-renders
  const connectWallet = useCallback(async () => {
    if (typeof window.ethereum !== 'undefined') {
      try {
        const provider = new ethers.BrowserProvider(window.ethereum);
        const accounts = await provider.send('eth_requestAccounts', []);
        const network = await provider.getNetwork();
        
        providerRef.current = provider;
        setState({
          account: accounts[0],
          provider,
          chainId: Number(network.chainId),
          isConnected: true,
        });
      } catch (error) {
        console.error('Error connecting wallet:', error);
      }
    } else {
      alert('Please install MetaMask or another Web3 wallet');
    }
  }, []);

  // Memoized disconnect function
  const disconnectWallet = useCallback(() => {
    if (providerRef.current) {
      providerRef.current = null;
    }
    setState(createWeb3State());
  }, []);

  useEffect(() => {
    if (typeof window.ethereum !== 'undefined') {
      const handleAccountsChanged = (accounts) => {
        if (accounts.length === 0) {
          disconnectWallet();
        } else {
          setState(prev => ({ ...prev, account: accounts[0] }));
        }
      };

      const handleChainChanged = (chainId) => {
        setState(prev => ({ ...prev, chainId: Number(chainId) }));
        window.location.reload();
      };

      window.ethereum.on('accountsChanged', handleAccountsChanged);
      window.ethereum.on('chainChanged', handleChainChanged);

      // Proper cleanup with named function references
      return () => {
        window.ethereum.removeListener('accountsChanged', handleAccountsChanged);
        window.ethereum.removeListener('chainChanged', handleChainChanged);
      };
    }
  }, [disconnectWallet]);

  return (
    <Web3Context.Provider value={{ 
      account: state.account, 
      provider: state.provider, 
      chainId: state.chainId, 
      isConnected: state.isConnected, 
      connectWallet, 
      disconnectWallet 
    }}>
      {children}
    </Web3Context.Provider>
  );
};

export const useWeb3 = () => {
  const context = useContext(Web3Context);
  if (!context) {
    throw new Error('useWeb3 must be used within a Web3Provider');
  }
  return context;
};
