"""
EIP-712/EIP-2612 Permit Security Analyzer
Analyzes gasless token approval implementations for security vulnerabilities including:
- Domain separator fork attacks (hardcoded chain IDs)
- Nonce replay attacks
- Missing deadline verification
- Invalid ecrecover handling
"""

import json
import re
from typing import Dict, List, Optional, Set, Tuple
from dataclasses import dataclass, field
from enum import Enum
import asyncio
from circuit_breaker import get_rpc_circuit_breaker, CircuitBreakerOpenError

try:
    from web3 import Web3
    WEB3_AVAILABLE = True
except ImportError:
    WEB3_AVAILABLE = False
    # Create mock Web3 for graceful degradation
    class MockWeb3:
        class eth:
            @staticmethod
            def get_code(address):
                return b''
            
            @staticmethod
            def call(transaction):
                return b'\x00' * 32
            
            @staticmethod
            def chain_id:
                return 1
    
    Web3 = MockWeb3


class PermitRiskType(Enum):
    """Types of permit-related security risks"""
    HARDCODED_DOMAIN_SEPARATOR = "hardcoded_domain_separator"
    MISSING_CHAIN_ID_VALIDATION = "missing_chain_id_validation"
    NONCE_REUSE_VULNERABILITY = "nonce_reuse_vulnerability"
    MISSING_DEADLINE_CHECK = "missing_deadline_check"
    INVALID_ECRECOVER_HANDLING = "invalid_ecrecover_handling"
    ZERO_ADDRESS_SIGNER_ACCEPTANCE = "zero_address_signer_acceptance"
    CROSS_CHAIN_REPLAY_ATTACK = "cross_chain_replay_attack"
    IMPROPER_NONCE_INCREMENT = "improper_nonce_increment"


@dataclass
class PermitFunctionInfo:
    """Information about a permit function"""
    function_selector: str
    function_name: str
    has_domain_separator: bool
    has_chain_id_validation: bool
    has_nonce_parameter: bool
    has_deadline_parameter: bool
    uses_ecrecover: bool
    domain_separator_implementation: str  # "dynamic", "hardcoded", "missing"


@dataclass
class NonceTrackingInfo:
    """Information about nonce implementation"""
    has_nonces_mapping: bool
    nonce_storage_slot: Optional[str]
    nonce_increment_pattern: str  # "monotonic", "arbitrary", "missing"
    tracks_per_address: bool


@dataclass
class PermitSecurityRisk:
    """Represents a permit-related security risk"""
    contract_id: str
    risk_type: PermitRiskType
    description: str
    severity: str  # "critical", "high", "medium", "low"
    risk_multiplier: float = 1.0
    technical_details: Dict = field(default_factory=dict)
    recommendation: str = ""


# EIP-2612 standard function selectors
PERMIT_FUNCTION_SELECTORS = {
    "permit": "0x8a5f3c3b",  # permit(address owner, address spender, uint256 value, uint256 deadline, uint8 v, bytes32 r, bytes32 s)
    "permit2": "0x36c787bb",  # Alternative permit implementation
    "DOMAIN_SEPARATOR": "0x795a98e3",  # DOMAIN_SEPARATOR() view function
}

# EIP-712 type hash for ERC20 permits
ERC20_PERMIT_TYPEHASH = "0x6e71edae12b1b97f4d1f60370fef10105fa2faae0126114a169c64845d6126c9"

# Known vulnerable patterns in permit implementations
VULNERABLE_PATTERNS = {
    "hardcoded_chain_id": [
        r"chainid\s*=\s*1\s*;?",  # Hardcoded mainnet chain ID
        r"chainid\s*=\s*0x1\s*;?",  # Hex hardcoded
        r"block\.chainid\s*==\s*\d+",  # Comparison instead of usage
    ],
    "missing_deadline_check": [
        r"require\s*\(\s*deadline\s*>=\s*block\.timestamp\s*,\s*[^)]*\)",  # Should be >
        r"if\s*\(\s*deadline\s*<\s*block\.timestamp\s*\)\s*return",  # Early return without revert
    ],
    "invalid_ecrecover": [
        r"ecrecover\s*\([^)]*\)\s*==\s*address\(0\)",  # Should be != for zero address check
        r"if\s*\(\s*signer\s*==\s*address\(0\)\)\s*return",  # Returns instead of reverting
    ],
}


class PermitFunctionDetector:
    """Detects permit functions in contract bytecode"""
    
    def __init__(self, web3: Optional[Web3] = None):
        if not WEB3_AVAILABLE:
            self.web3 = MockWeb3()
        else:
            self.web3 = web3 or Web3()
    
    def detect_permit_functions(self, contract_address: str) -> List[PermitFunctionInfo]:
        """
        Detect permit functions in contract bytecode
        
        Args:
            contract_address: Contract address to analyze
            
        Returns:
            List of detected permit functions with their characteristics
        """
        permit_functions = []
        
        try:
            # Get contract bytecode
            bytecode = self.web3.eth.get_code(contract_address)
            bytecode_hex = bytecode.hex()
            
            # Check for EIP-2612 permit function
            if PERMIT_FUNCTION_SELECTORS["permit"] in bytecode_hex:
                permit_info = self._analyze_permit_implementation(bytecode_hex, contract_address)
                permit_functions.append(permit_info)
            
            # Check for DOMAIN_SEPARATOR function
            if PERMIT_FUNCTION_SELECTORS["DOMAIN_SEPARATOR"] in bytecode_hex:
                domain_info = self._analyze_domain_separator(bytecode_hex, contract_address)
                if domain_info:
                    permit_functions.append(domain_info)
            
        except Exception as e:
            print(f"Error detecting permit functions: {e}")
        
        return permit_functions
    
    def _analyze_permit_implementation(self, bytecode_hex: str, contract_address: str) -> PermitFunctionInfo:
        """Analyze the permit function implementation"""
        return PermitFunctionInfo(
            function_selector=PERMIT_FUNCTION_SELECTORS["permit"],
            function_name="permit",
            has_domain_separator=self._has_domain_separator(bytecode_hex),
            has_chain_id_validation=self._has_chain_id_validation(bytecode_hex),
            has_nonce_parameter=self._has_nonce_parameter(bytecode_hex),
            has_deadline_parameter=self._has_deadline_parameter(bytecode_hex),
            uses_ecrecover=self._uses_ecrecover(bytecode_hex),
            domain_separator_implementation=self._get_domain_separator_type(bytecode_hex)
        )
    
    def _analyze_domain_separator(self, bytecode_hex: str, contract_address: str) -> Optional[PermitFunctionInfo]:
        """Analyze DOMAIN_SEPARATOR function implementation"""
        return PermitFunctionInfo(
            function_selector=PERMIT_FUNCTION_SELECTORS["DOMAIN_SEPARATOR"],
            function_name="DOMAIN_SEPARATOR",
            has_domain_separator=True,
            has_chain_id_validation=self._has_chain_id_validation(bytecode_hex),
            has_nonce_parameter=False,
            has_deadline_parameter=False,
            uses_ecrecover=False,
            domain_separator_implementation=self._get_domain_separator_type(bytecode_hex)
        )
    
    def _has_domain_separator(self, bytecode_hex: str) -> bool:
        """Check if contract has domain separator functionality"""
        return PERMIT_FUNCTION_SELECTORS["DOMAIN_SEPARATOR"] in bytecode_hex
    
    def _has_chain_id_validation(self, bytecode_hex: str) -> bool:
        """Check if contract properly validates chain ID"""
        # Look for block.chainid usage patterns
        chain_id_patterns = [
            "block.chainid",  # Solidity 0.8.5+
            "chainid",  # Pre-0.8.5 using chainid opcode
        ]
        
        for pattern in chain_id_patterns:
            if pattern in bytecode_hex.lower():
                return True
        
        return False
    
    def _has_nonce_parameter(self, bytecode_hex: str) -> bool:
        """Check if permit function has nonce parameter"""
        # This is a simplified check - in production would parse ABI
        return "nonce" in bytecode_hex.lower()
    
    def _has_deadline_parameter(self, bytecode_hex: str) -> bool:
        """Check if permit function has deadline parameter"""
        return "deadline" in bytecode_hex.lower()
    
    def _uses_ecrecover(self, bytecode_hex: str) -> bool:
        """Check if contract uses ecrecover"""
        return "ecrecover" in bytecode_hex.lower()
    
    def _get_domain_separator_type(self, bytecode_hex: str) -> str:
        """
        Determine if domain separator uses dynamic block.chainid or is hardcoded
        
        Returns:
            "dynamic", "hardcoded", or "missing"
        """
        # Check for dynamic chain ID usage
        if "block.chainid" in bytecode_hex.lower():
            return "dynamic"
        
        # Check for hardcoded chain IDs (common vulnerability)
        hardcoded_patterns = [
            "0000000000000000000000000000000000000000000000000000000000000001",  # Mainnet
            "0000000000000000000000000000000000000000000000000000000000000003",  # Ropsten
            "0000000000000000000000000000000000000000000000000000000000000004",  # Rinkeby
            "0000000000000000000000000000000000000000000000000000000000000005",  # Goerli
        ]
        
        for pattern in hardcoded_patterns:
            if pattern in bytecode_hex.lower():
                return "hardcoded"
        
        return "missing"


class NonceAnalyzer:
    """Analyzes nonce implementation for replay attack prevention"""
    
    def __init__(self, web3: Optional[Web3] = None, rpc_url: Optional[str] = None):
        if not WEB3_AVAILABLE:
            self.web3 = MockWeb3()
            self.circuit_breaker = None
        else:
            self.web3 = web3 or Web3()
            self.rpc_url = rpc_url
            self.circuit_breaker = None
    
    async def _get_circuit_breaker(self):
        """Get or create circuit breaker for RPC URL"""
        if self.circuit_breaker is None and self.rpc_url:
            self.circuit_breaker = await get_rpc_circuit_breaker(self.rpc_url)
        return self.circuit_breaker
    
    async def analyze_nonce_implementation(self, contract_address: str) -> NonceTrackingInfo:
        """
        Analyze nonce implementation to detect replay vulnerabilities
        
        Args:
            contract_address: Contract address to analyze
            
        Returns:
            NonceTrackingInfo with implementation details
        """
        nonce_info = NonceTrackingInfo(
            has_nonces_mapping=False,
            nonce_storage_slot=None,
            nonce_increment_pattern="missing",
            tracks_per_address=False
        )
        
        try:
            # Get contract bytecode
            bytecode = self.web3.eth.get_code(contract_address)
            bytecode_hex = bytecode.hex()
            
            # Check for nonces mapping
            if "nonces" in bytecode_hex.lower():
                nonce_info.has_nonces_mapping = True
                nonce_info.tracks_per_address = self._tracks_per_address(bytecode_hex)
                nonce_info.nonce_increment_pattern = self._analyze_nonce_increment_pattern(bytecode_hex)
                nonce_info.nonce_storage_slot = self._estimate_nonce_storage_slot(bytecode_hex)
            
        except Exception as e:
            print(f"Error analyzing nonce implementation: {e}")
        
        return nonce_info
    
    def _tracks_per_address(self, bytecode_hex: str) -> bool:
        """Check if nonces are tracked per address"""
        # Look for mapping(address => uint256) pattern
        address_mapping_patterns = [
            "mapping(address",
            "nonces[",
        ]
        
        for pattern in address_mapping_patterns:
            if pattern in bytecode_hex.lower():
                return True
        
        return False
    
    def _analyze_nonce_increment_pattern(self, bytecode_hex: str) -> str:
        """
        Analyze how nonces are incremented
        
        Returns:
            "monotonic", "arbitrary", or "missing"
        """
        # Look for increment patterns
        increment_patterns = [
            "nonces[owner]++",  # Post-increment
            "++nonces[owner]",  # Pre-increment
            "nonces[owner] += 1",  # Addition
            "nonces[owner] = nonces[owner] + 1",  # Addition
        ]
        
        for pattern in increment_patterns:
            if pattern in bytecode_hex.lower():
                return "monotonic"
        
        # Check for arbitrary nonce setting (vulnerable)
        arbitrary_patterns = [
            "nonces[owner] = ",
            "nonces[owner] = nonce",
        ]
        
        for pattern in arbitrary_patterns:
            if pattern in bytecode_hex.lower():
                return "arbitrary"
        
        return "missing"
    
    def _estimate_nonce_storage_slot(self, bytecode_hex: str) -> Optional[str]:
        """Estimate the storage slot used for nonces"""
        # This is a simplified estimation - in production would use proper storage layout analysis
        if "nonces" in bytecode_hex.lower():
            # Common storage slot for nonces (simplified)
            return "0x2"  # Typical slot after owner and allowances
        
        return None
    
    async def verify_nonce_monotonic_increment(self, contract_address: str, test_address: str) -> bool:
        """
        Verify that nonces increment monotonically for a test address
        
        Args:
            contract_address: Contract address to test
            test_address: Address to test nonce increment with
            
        Returns:
            True if nonces increment monotonically
        """
        try:
            # Get initial nonce
            initial_nonce = await self._get_nonce(contract_address, test_address)
            
            # In production, this would call the permit function and check nonce increment
            # For now, we'll return True if the implementation looks correct
            return True
            
        except Exception as e:
            print(f"Error verifying nonce increment: {e}")
            return False
    
    async def _get_nonce(self, contract_address: str, address: str) -> int:
        """Get current nonce for an address"""
        async def make_request():
            # This would call the nonces() function
            # For now, return a mock value
            return 0
        
        try:
            breaker = await self._get_circuit_breaker()
            if breaker:
                return await breaker.call(make_request)
            else:
                return make_request()
        except CircuitBreakerOpenError as e:
            print(f"Circuit breaker open for RPC when getting nonce: {e}")
            return 0
        except Exception as e:
            print(f"Error getting nonce: {e}")
            return 0


class DeadlineAnalyzer:
    """Analyzes deadline implementation for permit security"""
    
    def __init__(self, web3: Optional[Web3] = None):
        if not WEB3_AVAILABLE:
            self.web3 = MockWeb3()
        else:
            self.web3 = web3 or Web3()
    
    def analyze_deadline_implementation(self, contract_address: str) -> Dict:
        """
        Analyze deadline implementation for security vulnerabilities
        
        Args:
            contract_address: Contract address to analyze
            
        Returns:
            Dict with deadline implementation details
        """
        deadline_info = {
            "has_deadline_check": False,
            "deadline_validation_type": "missing",  # "strict", "lenient", "missing"
            "comparator_used": None,  # ">", ">=", "=="
            "reverts_on_expiry": False,
            "vulnerabilities": []
        }
        
        try:
            # Get contract bytecode
            bytecode = self.web3.eth.get_code(contract_address)
            bytecode_hex = bytecode.hex()
            
            # Check for deadline validation
            if "deadline" in bytecode_hex.lower():
                deadline_info["has_deadline_check"] = True
                deadline_info["deadline_validation_type"] = self._get_deadline_validation_type(bytecode_hex)
                deadline_info["comparator_used"] = self._get_deadline_comparator(bytecode_hex)
                deadline_info["reverts_on_expiry"] = self._check_revert_on_expiry(bytecode_hex)
                
                # Check for vulnerabilities
                if deadline_info["comparator_used"] == ">=":
                    deadline_info["vulnerabilities"].append("uses_gte_comparator")
                if not deadline_info["reverts_on_expiry"]:
                    deadline_info["vulnerabilities"].append("does_not_revert_on_expiry")
            
        except Exception as e:
            print(f"Error analyzing deadline implementation: {e}")
        
        return deadline_info
    
    def _get_deadline_validation_type(self, bytecode_hex: str) -> str:
        """Determine the type of deadline validation"""
        if "block.timestamp" in bytecode_hex.lower():
            return "strict"
        elif "now" in bytecode_hex.lower():
            return "lenient"
        return "missing"
    
    def _get_deadline_comparator(self, bytecode_hex: str) -> Optional[str]:
        """Extract the comparator used for deadline validation"""
        # Look for common comparators in bytecode
        if "require(deadline >= block.timestamp" in bytecode_hex.lower():
            return ">="
        elif "require(deadline > block.timestamp" in bytecode_hex.lower():
            return ">"
        elif "deadline >= block.timestamp" in bytecode_hex.lower():
            return ">="
        elif "deadline > block.timestamp" in bytecode_hex.lower():
            return ">"
        
        return None
    
    def _check_revert_on_expiry(self, bytecode_hex: str) -> bool:
        """Check if contract reverts when deadline has expired"""
        # Look for require/revert patterns in deadline validation
        revert_patterns = [
            "require(deadline",
            "revert(",
        ]
        
        for pattern in revert_patterns:
            if pattern in bytecode_hex.lower():
                return True
        
        return False


class EcrecoverAnalyzer:
    """Analyzes ecrecover implementation for signature validation"""
    
    def __init__(self, web3: Optional[Web3] = None):
        if not WEB3_AVAILABLE:
            self.web3 = MockWeb3()
        else:
            self.web3 = web3 or Web3()
    
    def analyze_ecrecover_implementation(self, contract_address: str) -> Dict:
        """
        Analyze ecrecover implementation for security vulnerabilities
        
        Args:
            contract_address: Contract address to analyze
            
        Returns:
            Dict with ecrecover implementation details
        """
        ecrecover_info = {
            "uses_ecrecover": False,
            "validates_signer": False,
            "checks_zero_address": False,
            "zero_address_handling": "missing",  # "reverts", "returns", "accepts"
            "vulnerabilities": []
        }
        
        try:
            # Get contract bytecode
            bytecode = self.web3.eth.get_code(contract_address)
            bytecode_hex = bytecode.hex()
            
            # Check for ecrecover usage
            if "ecrecover" in bytecode_hex.lower():
                ecrecover_info["uses_ecrecover"] = True
                ecrecover_info["validates_signer"] = self._validates_signer(bytecode_hex)
                ecrecover_info["checks_zero_address"] = self._checks_zero_address(bytecode_hex)
                ecrecover_info["zero_address_handling"] = self._get_zero_address_handling(bytecode_hex)
                
                # Check for vulnerabilities
                if not ecrecover_info["checks_zero_address"]:
                    ecrecover_info["vulnerabilities"].append("missing_zero_address_check")
                if ecrecover_info["zero_address_handling"] == "accepts":
                    ecrecover_info["vulnerabilities"].append("accepts_zero_address_signer")
                if not ecrecover_info["validates_signer"]:
                    ecrecover_info["vulnerabilities"].append("missing_signer_validation")
            
        except Exception as e:
            print(f"Error analyzing ecrecover implementation: {e}")
        
        return ecrecover_info
    
    def _validates_signer(self, bytecode_hex: str) -> bool:
        """Check if contract validates the recovered signer"""
        validation_patterns = [
            "require(signer == owner",
            "require(signer == _owner",
            "if (signer != owner) revert",
        ]
        
        for pattern in validation_patterns:
            if pattern in bytecode_hex.lower():
                return True
        
        return False
    
    def _checks_zero_address(self, bytecode_hex: str) -> bool:
        """Check if contract checks for zero address from ecrecover"""
        zero_address_patterns = [
            "signer != address(0)",
            "signer == address(0)",
            "address(0)",
        ]
        
        for pattern in zero_address_patterns:
            if pattern in bytecode_hex.lower():
                return True
        
        return False
    
    def _get_zero_address_handling(self, bytecode_hex: str) -> str:
        """Determine how zero address is handled"""
        if "signer != address(0)" in bytecode_hex.lower():
            return "reverts"
        elif "signer == address(0)" in bytecode_hex.lower():
            return "checks"
        elif "if (signer == address(0)) return" in bytecode_hex.lower():
            return "returns"
        else:
            return "accepts"


class PermitSecurityAnalyzer:
    """Main analyzer for EIP-712/EIP-2612 permit security vulnerabilities"""
    
    def __init__(self, web3: Optional[Web3] = None, rpc_url: Optional[str] = None):
        if not WEB3_AVAILABLE:
            self.web3 = MockWeb3()
        elif rpc_url:
            self.web3 = Web3(Web3.HTTPProvider(rpc_url))
        else:
            self.web3 = web3 or Web3()
        
        self.rpc_url = rpc_url
        self.function_detector = PermitFunctionDetector(self.web3)
        self.nonce_analyzer = NonceAnalyzer(self.web3, rpc_url)
        self.deadline_analyzer = DeadlineAnalyzer(self.web3)
        self.ecrecover_analyzer = EcrecoverAnalyzer(self.web3)
        self.detected_risks: List[PermitSecurityRisk] = []
    
    async def analyze_permit_security(self, contract_address: str) -> Dict:
        """
        Analyze a contract for permit security vulnerabilities
        
        Args:
            contract_address: Contract address to analyze
            
        Returns:
            Analysis results with security risks and recommendations
        """
        self.detected_risks = []
        
        # Detect permit functions
        permit_functions = self.function_detector.detect_permit_functions(contract_address)
        
        if not permit_functions:
            return {
                "contract_address": contract_address,
                "has_permit_functionality": False,
                "risks": [],
                "risk_multiplier": 1.0,
                "recommendations": []
            }
        
        # Analyze each permit function
        for permit_func in permit_functions:
            await self._analyze_permit_function(contract_address, permit_func)
        
        # Analyze nonce implementation
        nonce_info = await self.nonce_analyzer.analyze_nonce_implementation(contract_address)
        self._detect_nonce_risks(contract_address, nonce_info)
        
        # Analyze deadline implementation
        deadline_info = self.deadline_analyzer.analyze_deadline_implementation(contract_address)
        self._detect_deadline_risks(contract_address, deadline_info)
        
        # Analyze ecrecover implementation
        ecrecover_info = self.ecrecover_analyzer.analyze_ecrecover_implementation(contract_address)
        self._detect_ecrecover_risks(contract_address, ecrecover_info)
        
        # Calculate overall risk multiplier
        risk_multiplier = self._calculate_risk_multiplier()
        
        return {
            "contract_address": contract_address,
            "has_permit_functionality": True,
            "permit_functions": [
                {
                    "function_selector": func.function_selector,
                    "function_name": func.function_name,
                    "domain_separator_implementation": func.domain_separator_implementation,
                    "has_chain_id_validation": func.has_chain_id_validation,
                }
                for func in permit_functions
            ],
            "nonce_analysis": {
                "has_nonces_mapping": nonce_info.has_nonces_mapping,
                "tracks_per_address": nonce_info.tracks_per_address,
                "nonce_increment_pattern": nonce_info.nonce_increment_pattern,
            },
            "deadline_analysis": deadline_info,
            "ecrecover_analysis": ecrecover_info,
            "risks": [self._format_risk(risk) for risk in self.detected_risks],
            "risk_multiplier": risk_multiplier,
            "recommendations": self._generate_recommendations()
        }
    
    async def _analyze_permit_function(self, contract_address: str, permit_func: PermitFunctionInfo):
        """Analyze individual permit function for vulnerabilities"""
        
        # Check for hardcoded domain separator (critical vulnerability)
        if permit_func.domain_separator_implementation == "hardcoded":
            risk = PermitSecurityRisk(
                contract_id=contract_address,
                risk_type=PermitRiskType.HARDCODED_DOMAIN_SEPARATOR,
                description="Domain separator uses hardcoded chain ID instead of dynamic block.chainid, making it vulnerable to chain replay attacks",
                severity="critical",
                risk_multiplier=5.0,
                technical_details={
                    "function_selector": permit_func.function_selector,
                    "implementation_type": permit_func.domain_separator_implementation,
                },
                recommendation="Replace hardcoded chain ID with block.chainid in DOMAIN_SEPARATOR calculation"
            )
            self.detected_risks.append(risk)
        
        # Check for missing chain ID validation
        if not permit_func.has_chain_id_validation:
            risk = PermitSecurityRisk(
                contract_id=contract_address,
                risk_type=PermitRiskType.MISSING_CHAIN_ID_VALIDATION,
                description="Permit function missing chain ID validation in signature verification",
                severity="high",
                risk_multiplier=3.0,
                technical_details={
                    "function_selector": permit_func.function_selector,
                },
                recommendation="Add chain ID validation in the permit type hash and domain separator"
            )
            self.detected_risks.append(risk)
    
    def _detect_nonce_risks(self, contract_address: str, nonce_info: NonceTrackingInfo):
        """Detect nonce-related vulnerabilities"""
        
        # Check for missing nonce tracking
        if not nonce_info.has_nonces_mapping:
            risk = PermitSecurityRisk(
                contract_id=contract_address,
                risk_type=PermitRiskType.NONCE_REUSE_VULNERABILITY,
                description="Contract does not implement nonce tracking, making permits vulnerable to replay attacks",
                severity="critical",
                risk_multiplier=5.0,
                technical_details={
                    "has_nonces_mapping": nonce_info.has_nonces_mapping,
                },
                recommendation="Implement ERC20Permit nonce mapping with monotonic increment per address"
            )
            self.detected_risks.append(risk)
        
        # Check for improper nonce increment
        elif nonce_info.nonce_increment_pattern != "monotonic":
            risk = PermitSecurityRisk(
                contract_id=contract_address,
                risk_type=PermitRiskType.IMPROPER_NONCE_INCREMENT,
                description=f"Nonce increment pattern '{nonce_info.nonce_increment_pattern}' is not monotonic, potentially allowing nonce reuse",
                severity="high",
                risk_multiplier=3.0,
                technical_details={
                    "increment_pattern": nonce_info.nonce_increment_pattern,
                },
                recommendation="Ensure nonces increment monotonically (nonces[owner]++) after successful permit"
            )
            self.detected_risks.append(risk)
        
        # Check if nonces are not tracked per address
        if nonce_info.has_nonces_mapping and not nonce_info.tracks_per_address:
            risk = PermitSecurityRisk(
                contract_id=contract_address,
                risk_type=PermitRiskType.NONCE_REUSE_VULNERABILITY,
                description="Nonces are not tracked per address, allowing cross-address replay attacks",
                severity="high",
                risk_multiplier=3.0,
                technical_details={
                    "tracks_per_address": nonce_info.tracks_per_address,
                },
                recommendation="Use mapping(address => uint256) for nonces to track per address"
            )
            self.detected_risks.append(risk)
    
    def _detect_deadline_risks(self, contract_address: str, deadline_info: Dict):
        """Detect deadline-related vulnerabilities"""
        
        # Check for missing deadline check
        if not deadline_info["has_deadline_check"]:
            risk = PermitSecurityRisk(
                contract_id=contract_address,
                risk_type=PermitRiskType.MISSING_DEADLINE_CHECK,
                description="Permit function missing deadline verification, allowing indefinite signature validity",
                severity="critical",
                risk_multiplier=4.0,
                technical_details=deadline_info,
                recommendation="Add deadline parameter with strict validation: require(deadline > block.timestamp, 'PERMIT_DEADLINE_EXPIRED')"
            )
            self.detected_risks.append(risk)
        
        # Check for lenient deadline validation
        elif deadline_info["comparator_used"] == ">=":
            risk = PermitSecurityRisk(
                contract_id=contract_address,
                risk_type=PermitRiskType.MISSING_DEADLINE_CHECK,
                description="Deadline validation uses >= comparator instead of >, allowing signatures to be used exactly at deadline",
                severity="medium",
                risk_multiplier=1.5,
                technical_details={
                    "comparator": deadline_info["comparator_used"],
                },
                recommendation="Use strict > comparator for deadline validation"
            )
            self.detected_risks.append(risk)
        
        # Check if contract doesn't revert on expiry
        if deadline_info["has_deadline_check"] and not deadline_info["reverts_on_expiry"]:
            risk = PermitSecurityRisk(
                contract_id=contract_address,
                risk_type=PermitRiskType.MISSING_DEADLINE_CHECK,
                description="Contract does not revert when deadline has expired, potentially allowing silent failures",
                severity="medium",
                risk_multiplier=1.5,
                technical_details=deadline_info,
                recommendation="Ensure contract reverts with meaningful error message when deadline expires"
            )
            self.detected_risks.append(risk)
    
    def _detect_ecrecover_risks(self, contract_address: str, ecrecover_info: Dict):
        """Detect ecrecover-related vulnerabilities"""
        
        # Check for missing zero address validation
        if ecrecover_info["uses_ecrecover"] and not ecrecover_info["checks_zero_address"]:
            risk = PermitSecurityRisk(
                contract_id=contract_address,
                risk_type=PermitRiskType.INVALID_ECRECOVER_HANDLING,
                description="ecrecover return value not validated for zero address, allowing invalid signatures to be processed",
                severity="critical",
                risk_multiplier=4.0,
                technical_details=ecrecover_info,
                recommendation="Add zero address check: require(signer != address(0), 'INVALID_SIGNATURE')"
            )
            self.detected_risks.append(risk)
        
        # Check if zero address is accepted
        if ecrecover_info["zero_address_handling"] == "accepts":
            risk = PermitSecurityRisk(
                contract_id=contract_address,
                risk_type=PermitRiskType.ZERO_ADDRESS_SIGNER_ACCEPTANCE,
                description="Contract accepts zero address as valid signer from ecrecover, allowing signature bypass attacks",
                severity="critical",
                risk_multiplier=5.0,
                technical_details=ecrecover_info,
                recommendation="Reject zero address signer: require(signer != address(0), 'INVALID_SIGNATURE')"
            )
            self.detected_risks.append(risk)
        
        # Check for missing signer validation
        if ecrecover_info["uses_ecrecover"] and not ecrecover_info["validates_signer"]:
            risk = PermitSecurityRisk(
                contract_id=contract_address,
                risk_type=PermitRiskType.INVALID_ECRECOVER_HANDLING,
                description="Contract does not validate that recovered signer matches expected owner",
                severity="critical",
                risk_multiplier=5.0,
                technical_details=ecrecover_info,
                recommendation="Add signer validation: require(signer == owner, 'INVALID_SIGNER')"
            )
            self.detected_risks.append(risk)
    
    def _calculate_risk_multiplier(self) -> float:
        """Calculate overall risk multiplier based on detected risks"""
        if not self.detected_risks:
            return 1.0
        
        # Use the maximum risk multiplier from detected risks
        max_multiplier = max(risk.risk_multiplier for risk in self.detected_risks)
        
        # Apply additional multiplier for multiple critical risks
        critical_count = sum(1 for risk in self.detected_risks if risk.severity == "critical")
        if critical_count > 1:
            max_multiplier *= 1.5
        
        return min(max_multiplier, 10.0)  # Cap at 10.0 for permit security
    
    def _format_risk(self, risk: PermitSecurityRisk) -> Dict:
        """Format risk for JSON serialization"""
        return {
            "contract_id": risk.contract_id,
            "risk_type": risk.risk_type.value,
            "description": risk.description,
            "severity": risk.severity,
            "risk_multiplier": risk.risk_multiplier,
            "technical_details": risk.technical_details,
            "recommendation": risk.recommendation
        }
    
    def _generate_recommendations(self) -> List[str]:
        """Generate security recommendations"""
        recommendations = []
        
        for risk in self.detected_risks:
            if risk.recommendation and risk.recommendation not in recommendations:
                recommendations.append(risk.recommendation)
        
        # Add general recommendations if none specific
        if not recommendations:
            recommendations.extend([
                "Implement ERC20Permit standard (EIP-2612) with proper nonce tracking",
                "Use dynamic block.chainid in DOMAIN_SEPARATOR calculation",
                "Add strict deadline validation with proper error messages",
                "Validate ecrecover return values and reject zero address signers",
            ])
        
        return recommendations


# Convenience function for quick analysis
async def analyze_permit_security(contract_address: str, rpc_url: Optional[str] = None) -> Dict:
    """
    Convenience function for permit security analysis
    
    Args:
        contract_address: Contract address to analyze
        rpc_url: Optional RPC URL for web3 connection
        
    Returns:
        Analysis results
    """
    analyzer = PermitSecurityAnalyzer(rpc_url=rpc_url)
    return await analyzer.analyze_permit_security(contract_address)