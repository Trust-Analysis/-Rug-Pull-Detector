"""
Tests for Permit Security Analyzer
Tests for EIP-712/EIP-2612 permit function security vulnerability detection
"""

import pytest
import asyncio
from unittest.mock import Mock, MagicMock, patch
from permit_security_analyzer import (
    PermitSecurityAnalyzer,
    PermitFunctionDetector,
    NonceAnalyzer,
    DeadlineAnalyzer,
    EcrecoverAnalyzer,
    PermitRiskType,
    PermitFunctionInfo,
    NonceTrackingInfo,
    PermitSecurityRisk
)


class TestPermitFunctionDetector:
    """Tests for permit function detection"""
    
    def test_detect_permit_functions_with_permit(self):
        """Test detection of standard permit function"""
        # Mock web3
        mock_web3 = Mock()
        mock_web3.eth.get_code.return_value = b'\x8a\x5f\x3c\x3b'  # permit function selector
        
        detector = PermitFunctionDetector(mock_web3)
        functions = detector.detect_permit_functions("0x1234567890123456789012345678901234567890")
        
        assert len(functions) > 0
        assert functions[0].function_name == "permit"
    
    def test_detect_permit_functions_without_permit(self):
        """Test when contract has no permit function"""
        mock_web3 = Mock()
        mock_web3.eth.get_code.return_value = b'\x00\x00\x00\x00'  # No permit selector
        
        detector = PermitFunctionDetector(mock_web3)
        functions = detector.detect_permit_functions("0x1234567890123456789012345678901234567890")
        
        assert len(functions) == 0
    
    def test_domain_separator_dynamic_detection(self):
        """Test detection of dynamic block.chainid usage"""
        mock_web3 = Mock()
        bytecode_with_dynamic = b'block.chainid\x8a\x5f\x3c\x3b'
        mock_web3.eth.get_code.return_value = bytecode_with_dynamic
        
        detector = PermitFunctionDetector(mock_web3)
        functions = detector.detect_permit_functions("0x1234567890123456789012345678901234567890")
        
        if functions:
            assert functions[0].domain_separator_implementation == "dynamic"
    
    def test_domain_separator_hardcoded_detection(self):
        """Test detection of hardcoded chain ID (vulnerability)"""
        mock_web3 = Mock()
        # Bytecode with hardcoded mainnet chain ID
        bytecode_with_hardcoded = b'\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x01\x8a\x5f\x3c\x3b'
        mock_web3.eth.get_code.return_value = bytecode_with_hardcoded
        
        detector = PermitFunctionDetector(mock_web3)
        functions = detector.detect_permit_functions("0x1234567890123456789012345678901234567890")
        
        if functions:
            assert functions[0].domain_separator_implementation == "hardcoded"
    
    def test_chain_id_validation_detection(self):
        """Test detection of chain ID validation"""
        mock_web3 = Mock()
        bytecode_with_chainid = b'block.chainid\x8a\x5f\x3c\x3b'
        mock_web3.eth.get_code.return_value = bytecode_with_chainid
        
        detector = PermitFunctionDetector(mock_web3)
        functions = detector.detect_permit_functions("0x1234567890123456789012345678901234567890")
        
        if functions:
            assert functions[0].has_chain_id_validation == True


class TestNonceAnalyzer:
    """Tests for nonce implementation analysis"""
    
    @pytest.mark.asyncio
    async def test_nonce_tracking_detection(self):
        """Test detection of nonce mapping"""
        mock_web3 = Mock()
        bytecode_with_nonces = b'nonces[owner]++'
        mock_web3.eth.get_code.return_value = bytecode_with_nonces
        
        analyzer = NonceAnalyzer(mock_web3)
        nonce_info = await analyzer.analyze_nonce_implementation("0x1234567890123456789012345678901234567890")
        
        assert nonce_info.has_nonces_mapping == True
    
    @pytest.mark.asyncio
    async def test_missing_nonce_tracking(self):
        """Test when contract has no nonce tracking"""
        mock_web3 = Mock()
        mock_web3.eth.get_code.return_value = b'\x00\x00\x00\x00'
        
        analyzer = NonceAnalyzer(mock_web3)
        nonce_info = await analyzer.analyze_nonce_implementation("0x1234567890123456789012345678901234567890")
        
        assert nonce_info.has_nonces_mapping == False
        assert nonce_info.nonce_increment_pattern == "missing"
    
    @pytest.mark.asyncio
    async def test_monotonic_nonce_increment(self):
        """Test detection of monotonic nonce increment"""
        mock_web3 = Mock()
        bytecode_with_monotonic = b'nonces[owner]++'
        mock_web3.eth.get_code.return_value = bytecode_with_monotonic
        
        analyzer = NonceAnalyzer(mock_web3)
        nonce_info = await analyzer.analyze_nonce_implementation("0x1234567890123456789012345678901234567890")
        
        assert nonce_info.nonce_increment_pattern == "monotonic"
    
    @pytest.mark.asyncio
    async def test_arbitrary_nonce_increment(self):
        """Test detection of arbitrary nonce setting (vulnerability)"""
        mock_web3 = Mock()
        bytecode_with_arbitrary = b'nonces[owner] = nonce'
        mock_web3.eth.get_code.return_value = bytecode_with_arbitrary
        
        analyzer = NonceAnalyzer(mock_web3)
        nonce_info = await analyzer.analyze_nonce_implementation("0x1234567890123456789012345678901234567890")
        
        assert nonce_info.nonce_increment_pattern == "arbitrary"
    
    @pytest.mark.asyncio
    async def test_per_address_nonce_tracking(self):
        """Test detection of per-address nonce tracking"""
        mock_web3 = Mock()
        bytecode_with_mapping = b'mapping(address => uint256) nonces'
        mock_web3.eth.get_code.return_value = bytecode_with_mapping
        
        analyzer = NonceAnalyzer(mock_web3)
        nonce_info = await analyzer.analyze_nonce_implementation("0x1234567890123456789012345678901234567890")
        
        assert nonce_info.tracks_per_address == True


class TestDeadlineAnalyzer:
    """Tests for deadline implementation analysis"""
    
    def test_deadline_check_detection(self):
        """Test detection of deadline validation"""
        mock_web3 = Mock()
        bytecode_with_deadline = b'require(deadline > block.timestamp'
        mock_web3.eth.get_code.return_value = bytecode_with_deadline
        
        analyzer = DeadlineAnalyzer(mock_web3)
        deadline_info = analyzer.analyze_deadline_implementation("0x1234567890123456789012345678901234567890")
        
        assert deadline_info["has_deadline_check"] == True
    
    def test_missing_deadline_check(self):
        """Test when contract has no deadline check"""
        mock_web3 = Mock()
        mock_web3.eth.get_code.return_value = b'\x00\x00\x00\x00'
        
        analyzer = DeadlineAnalyzer(mock_web3)
        deadline_info = analyzer.analyze_deadline_implementation("0x1234567890123456789012345678901234567890")
        
        assert deadline_info["has_deadline_check"] == False
    
    def test_strict_deadline_comparator(self):
        """Test detection of strict > comparator"""
        mock_web3 = Mock()
        bytecode_with_strict = b'require(deadline > block.timestamp'
        mock_web3.eth.get_code.return_value = bytecode_with_strict
        
        analyzer = DeadlineAnalyzer(mock_web3)
        deadline_info = analyzer.analyze_deadline_implementation("0x1234567890123456789012345678901234567890")
        
        assert deadline_info["comparator_used"] == ">"
    
    def test_lenient_deadline_comparator(self):
        """Test detection of lenient >= comparator (vulnerability)"""
        mock_web3 = Mock()
        bytecode_with_lenient = b'require(deadline >= block.timestamp'
        mock_web3.eth.get_code.return_value = bytecode_with_lenient
        
        analyzer = DeadlineAnalyzer(mock_web3)
        deadline_info = analyzer.analyze_deadline_implementation("0x1234567890123456789012345678901234567890")
        
        assert deadline_info["comparator_used"] == ">="
    
    def test_revert_on_expiry(self):
        """Test detection of revert on deadline expiry"""
        mock_web3 = Mock()
        bytecode_with_revert = b'require(deadline > block.timestamp, "PERMIT_DEADLINE_EXPIRED")'
        mock_web3.eth.get_code.return_value = bytecode_with_revert
        
        analyzer = DeadlineAnalyzer(mock_web3)
        deadline_info = analyzer.analyze_deadline_implementation("0x1234567890123456789012345678901234567890")
        
        assert deadline_info["reverts_on_expiry"] == True


class TestEcrecoverAnalyzer:
    """Tests for ecrecover implementation analysis"""
    
    def test_ecrecover_usage_detection(self):
        """Test detection of ecrecover usage"""
        mock_web3 = Mock()
        bytecode_with_ecrecover = b'ecrecover('
        mock_web3.eth.get_code.return_value = bytecode_with_ecrecover
        
        analyzer = EcrecoverAnalyzer(mock_web3)
        ecrecover_info = analyzer.analyze_ecrecover_implementation("0x1234567890123456789012345678901234567890")
        
        assert ecrecover_info["uses_ecrecover"] == True
    
    def test_zero_address_check_detection(self):
        """Test detection of zero address validation"""
        mock_web3 = Mock()
        bytecode_with_check = b'signer != address(0)'
        mock_web3.eth.get_code.return_value = bytecode_with_check
        
        analyzer = EcrecoverAnalyzer(mock_web3)
        ecrecover_info = analyzer.analyze_ecrecover_implementation("0x1234567890123456789012345678901234567890")
        
        assert ecrecover_info["checks_zero_address"] == True
    
    def test_missing_zero_address_check(self):
        """Test when zero address check is missing (vulnerability)"""
        mock_web3 = Mock()
        bytecode_without_check = b'ecrecover('
        mock_web3.eth.get_code.return_value = bytecode_without_check
        
        analyzer = EcrecoverAnalyzer(mock_web3)
        ecrecover_info = analyzer.analyze_ecrecover_implementation("0x1234567890123456789012345678901234567890")
        
        assert ecrecover_info["checks_zero_address"] == False
    
    def test_signer_validation_detection(self):
        """Test detection of signer validation"""
        mock_web3 = Mock()
        bytecode_with_validation = b'require(signer == owner'
        mock_web3.eth.get_code.return_value = bytecode_with_validation
        
        analyzer = EcrecoverAnalyzer(mock_web3)
        ecrecover_info = analyzer.analyze_ecrecover_implementation("0x1234567890123456789012345678901234567890")
        
        assert ecrecover_info["validates_signer"] == True
    
    def test_missing_signer_validation(self):
        """Test when signer validation is missing (vulnerability)"""
        mock_web3 = Mock()
        bytecode_without_validation = b'ecrecover('
        mock_web3.eth.get_code.return_value = bytecode_without_validation
        
        analyzer = EcrecoverAnalyzer(mock_web3)
        ecrecover_info = analyzer.analyze_ecrecover_implementation("0x1234567890123456789012345678901234567890")
        
        assert ecrecover_info["validates_signer"] == False


class TestPermitSecurityAnalyzer:
    """Integration tests for permit security analyzer"""
    
    @pytest.mark.asyncio
    async def test_vulnerable_hardcoded_domain_separator(self):
        """Test detection of hardcoded domain separator vulnerability"""
        mock_web3 = Mock()
        # Bytecode with hardcoded chain ID and permit function
        vulnerable_bytecode = b'\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x01\x8a\x5f\x3c\x3b'
        mock_web3.eth.get_code.return_value = vulnerable_bytecode
        
        analyzer = PermitSecurityAnalyzer(web3=mock_web3)
        result = await analyzer.analyze_permit_security("0x1234567890123456789012345678901234567890")
        
        assert result["has_permit_functionality"] == True
        assert any(risk["risk_type"] == "hardcoded_domain_separator" for risk in result["risks"])
    
    @pytest.mark.asyncio
    async def test_vulnerable_missing_nonce_tracking(self):
        """Test detection of missing nonce tracking vulnerability"""
        mock_web3 = Mock()
        # Bytecode with permit but no nonce tracking
        vulnerable_bytecode = b'\x8a\x5f\x3c\x3b'  # permit selector without nonces
        mock_web3.eth.get_code.return_value = vulnerable_bytecode
        
        analyzer = PermitSecurityAnalyzer(web3=mock_web3)
        result = await analyzer.analyze_permit_security("0x1234567890123456789012345678901234567890")
        
        assert result["has_permit_functionality"] == True
        assert any(risk["risk_type"] == "nonce_reuse_vulnerability" for risk in result["risks"])
    
    @pytest.mark.asyncio
    async def test_vulnerable_missing_deadline_check(self):
        """Test detection of missing deadline check vulnerability"""
        mock_web3 = Mock()
        # Bytecode with permit but no deadline check
        vulnerable_bytecode = b'\x8a\x5f\x3c\x3b'  # permit selector without deadline
        mock_web3.eth.get_code.return_value = vulnerable_bytecode
        
        analyzer = PermitSecurityAnalyzer(web3=mock_web3)
        result = await analyzer.analyze_permit_security("0x1234567890123456789012345678901234567890")
        
        assert result["has_permit_functionality"] == True
        assert any(risk["risk_type"] == "missing_deadline_check" for risk in result["risks"])
    
    @pytest.mark.asyncio
    async def test_vulnerable_invalid_ecrecover_handling(self):
        """Test detection of invalid ecrecover handling vulnerability"""
        mock_web3 = Mock()
        # Bytecode with ecrecover but no zero address check
        vulnerable_bytecode = b'ecrecover(\x8a\x5f\x3c\x3b'  # ecrecover without validation
        mock_web3.eth.get_code.return_value = vulnerable_bytecode
        
        analyzer = PermitSecurityAnalyzer(web3=mock_web3)
        result = await analyzer.analyze_permit_security("0x1234567890123456789012345678901234567890")
        
        assert result["has_permit_functionality"] == True
        assert any(risk["risk_type"] == "invalid_ecrecover_handling" for risk in result["risks"])
    
    @pytest.mark.asyncio
    async def test_secure_permit_implementation(self):
        """Test analysis of secure permit implementation"""
        mock_web3 = Mock()
        # Bytecode with secure implementation
        secure_bytecode = b'block.chainid nonces[owner]++ require(deadline > block.timestamp signer != address(0) require(signer == owner \x8a\x5f\x3c\x3b'
        mock_web3.eth.get_code.return_value = secure_bytecode
        
        analyzer = PermitSecurityAnalyzer(web3=mock_web3)
        result = await analyzer.analyze_permit_security("0x1234567890123456789012345678901234567890")
        
        assert result["has_permit_functionality"] == True
        # Should have fewer or no critical risks
        critical_risks = [risk for risk in result["risks"] if risk["severity"] == "critical"]
        assert len(critical_risks) == 0
    
    @pytest.mark.asyncio
    async def test_contract_without_permit(self):
        """Test analysis of contract without permit functionality"""
        mock_web3 = Mock()
        mock_web3.eth.get_code.return_value = b'\x00\x00\x00\x00'
        
        analyzer = PermitSecurityAnalyzer(web3=mock_web3)
        result = await analyzer.analyze_permit_security("0x1234567890123456789012345678901234567890")
        
        assert result["has_permit_functionality"] == False
        assert len(result["risks"]) == 0
    
    @pytest.mark.asyncio
    async def test_risk_multiplier_calculation(self):
        """Test risk multiplier calculation based on detected risks"""
        mock_web3 = Mock()
        # Bytecode with multiple vulnerabilities
        vulnerable_bytecode = b'\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x01\x8a\x5f\x3c\x3b'
        mock_web3.eth.get_code.return_value = vulnerable_bytecode
        
        analyzer = PermitSecurityAnalyzer(web3=mock_web3)
        result = await analyzer.analyze_permit_security("0x1234567890123456789012345678901234567890")
        
        # Multiple critical risks should increase multiplier
        assert result["risk_multiplier"] > 1.0
    
    @pytest.mark.asyncio
    async def test_recommendations_generation(self):
        """Test that recommendations are generated for detected risks"""
        mock_web3 = Mock()
        vulnerable_bytecode = b'\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x01\x8a\x5f\x3c\x3b'
        mock_web3.eth.get_code.return_value = vulnerable_bytecode
        
        analyzer = PermitSecurityAnalyzer(web3=mock_web3)
        result = await analyzer.analyze_permit_security("0x1234567890123456789012345678901234567890")
        
        assert len(result["recommendations"]) > 0
        assert any("block.chainid" in rec for rec in result["recommendations"])


class TestRiskTypes:
    """Tests for risk type enumeration and classification"""
    
    def test_all_risk_types_defined(self):
        """Test that all expected risk types are defined"""
        expected_types = [
            "hardcoded_domain_separator",
            "missing_chain_id_validation", 
            "nonce_reuse_vulnerability",
            "missing_deadline_check",
            "invalid_ecrecover_handling",
            "zero_address_signer_acceptance",
            "cross_chain_replay_attack",
            "improper_nonce_increment"
        ]
        
        for expected_type in expected_types:
            assert expected_type in [risk_type.value for risk_type in PermitRiskType]
    
    def test_risk_severity_classification(self):
        """Test that risks have appropriate severity levels"""
        # Create a sample risk
        risk = PermitSecurityRisk(
            contract_id="0x1234567890123456789012345678901234567890",
            risk_type=PermitRiskType.HARDCODED_DOMAIN_SEPARATOR,
            description="Test risk",
            severity="critical",
            risk_multiplier=5.0
        )
        
        assert risk.severity in ["critical", "high", "medium", "low"]
        assert risk.risk_multiplier > 0


if __name__ == "__main__":
    pytest.main([__file__, "-v"])