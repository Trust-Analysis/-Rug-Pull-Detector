"""
Test suite for AST Selector Analyzer
Tests 4-byte selector generation, ERC interface shadowing/collisions, and deceptive fallback detection.
"""

import pytest
from ast_selector_analyzer import (
    ASTSelectorAnalyzer,
    FunctionSelectorInfo,
    SelectorClashRisk,
    SelectorRiskType,
    STANDARD_ERC_SELECTORS,
    analyze_ast_selectors,
    _keccak256_4bytes
)


class TestSelectorGeneration:
    """Test 4-byte selector mapping generation"""

    def test_keccak256_4bytes_standard_signatures(self):
        """Test keccak256 calculation for known ERC signatures"""
        assert _keccak256_4bytes("transfer(address,uint256)") == "0xa9059cbb"
        assert _keccak256_4bytes("transferFrom(address,address,uint256)") == "0x23b872dd"
        assert _keccak256_4bytes("approve(address,uint256)") == "0x095ea7b3"
        assert _keccak256_4bytes("balanceOf(address)") == "0x70a08231"
        assert _keccak256_4bytes("ownerOf(uint256)") == "0x6352211e"

    def test_generate_selector_mappings_from_ast(self):
        """Test generating selector mappings for public, external, and fallback functions"""
        analyzer = ASTSelectorAnalyzer()

        sample_ast = {
            "nodeType": "ContractDefinition",
            "nodes": [
                {
                    "nodeType": "FunctionDefinition",
                    "id": 1,
                    "name": "transfer",
                    "kind": "function",
                    "visibility": "public",
                    "stateMutability": "nonpayable",
                    "parameters": {
                        "parameters": [
                            {"typeDescriptions": {"typeString": "address"}},
                            {"typeDescriptions": {"typeString": "uint256"}}
                        ]
                    }
                },
                {
                    "nodeType": "FunctionDefinition",
                    "id": 2,
                    "name": "fallback",
                    "kind": "fallback",
                    "visibility": "external",
                    "stateMutability": "payable",
                    "parameters": {"parameters": []}
                },
                {
                    "nodeType": "FunctionDefinition",
                    "id": 3,
                    "name": "_internalHelper",
                    "kind": "function",
                    "visibility": "internal",
                    "stateMutability": "pure",
                    "parameters": {"parameters": []}
                }
            ]
        }

        mappings = analyzer.generate_selector_mappings(sample_ast)

        assert "0xa9059cbb" in mappings
        assert mappings["0xa9059cbb"].name == "transfer"
        assert mappings["0xa9059cbb"].signature == "transfer(address,uint256)"
        assert mappings["0xa9059cbb"].visibility == "public"

        assert "0x00000000" in mappings
        assert mappings["0x00000000"].is_fallback == True
        assert mappings["0x00000000"].state_mutability == "payable"

        # Internal helper should not be in external selector mappings
        assert len(mappings) == 2


class TestSignatureCollisionDetection:
    """Test identification of deliberate 4-byte signature collisions shadowing standard ERC interfaces"""

    def test_identify_deliberate_erc20_collision(self):
        """Test identifying custom function collision matching ERC-20 transfer selector 0xa9059cbb"""
        analyzer = ASTSelectorAnalyzer()

        # Custom function with overridden or colliding selector matching ERC-20 transfer
        custom_colliding_ast = {
            "nodeType": "ContractDefinition",
            "nodes": [
                {
                    "nodeType": "FunctionDefinition",
                    "id": 10,
                    "name": "drainWalletCustom",
                    "kind": "function",
                    "visibility": "external",
                    "stateMutability": "nonpayable",
                    "functionSelector": "0xa9059cbb",  # Deliberate collision with transfer(address,uint256)
                    "parameters": {
                        "parameters": [
                            {"typeDescriptions": {"typeString": "address"}},
                            {"typeDescriptions": {"typeString": "uint256"}}
                        ]
                    }
                }
            ]
        }

        analysis = analyzer.analyze_contract_ast("0xMaliciousContract", custom_colliding_ast)

        assert len(analysis["risks"]) > 0
        collision_risk = next(
            (r for r in analysis["risks"] if r["risk_type"] == SelectorRiskType.SIGNATURE_COLLISION.value), 
            None
        )
        assert collision_risk is not None
        assert collision_risk["severity"] == "critical"
        assert "0xa9059cbb" in collision_risk["technical_details"]["selector"]
        assert "ERC-20" in collision_risk["technical_details"]["standard"]

    def test_standard_erc20_function_not_flagged_as_collision(self):
        """Test standard transfer function is recognized and not flagged as collision"""
        analyzer = ASTSelectorAnalyzer()

        standard_ast = {
            "nodeType": "ContractDefinition",
            "nodes": [
                {
                    "nodeType": "FunctionDefinition",
                    "id": 1,
                    "name": "transfer",
                    "kind": "function",
                    "visibility": "public",
                    "stateMutability": "nonpayable",
                    "parameters": {
                        "parameters": [
                            {"typeDescriptions": {"typeString": "address"}},
                            {"typeDescriptions": {"typeString": "uint256"}}
                        ]
                    }
                }
            ]
        }

        analysis = analyzer.analyze_contract_ast("0xLegitToken", standard_ast)
        collision_risks = [r for r in analysis["risks"] if r["risk_type"] == SelectorRiskType.SIGNATURE_COLLISION.value]
        assert len(collision_risks) == 0


class TestDeceptiveFallbackDetection:
    """Test flagging payable fallback functions redirecting state modifications without explicit events"""

    def test_flag_payable_fallback_modifying_state_without_events(self):
        """Test payable fallback with state modification and missing event emission is flagged as critical"""
        analyzer = ASTSelectorAnalyzer()

        deceptive_fallback_ast = {
            "nodeType": "ContractDefinition",
            "nodes": [
                {
                    "nodeType": "FunctionDefinition",
                    "id": 20,
                    "name": "fallback",
                    "kind": "fallback",
                    "visibility": "external",
                    "stateMutability": "payable",
                    "parameters": {"parameters": []},
                    "body": {
                        "nodeType": "Block",
                        "statements": [
                            {
                                "nodeType": "Assignment",
                                "expression": "owner = msg.sender"
                            },
                            {
                                "nodeType": "ExpressionStatement",
                                "expression": "payable(owner).transfer(address(this).balance)"
                            }
                        ]
                    }
                }
            ]
        }

        analysis = analyzer.analyze_contract_ast("0xDeceptiveVault", deceptive_fallback_ast)

        assert len(analysis["risks"]) > 0
        fallback_risk = next(
            (r for r in analysis["risks"] if r["risk_type"] == SelectorRiskType.UNEMITTED_FALLBACK_STATE_CHANGE.value), 
            None
        )
        assert fallback_risk is not None
        assert fallback_risk["severity"] == "critical"
        assert fallback_risk["technical_details"]["events_emitted"] == False

    def test_payable_fallback_with_event_emissions_not_flagged(self):
        """Test payable fallback that emits explicit event is not flagged as deceptive"""
        analyzer = ASTSelectorAnalyzer()

        legit_fallback_ast = {
            "nodeType": "ContractDefinition",
            "nodes": [
                {
                    "nodeType": "FunctionDefinition",
                    "id": 21,
                    "name": "fallback",
                    "kind": "fallback",
                    "visibility": "external",
                    "stateMutability": "payable",
                    "parameters": {"parameters": []},
                    "body": {
                        "nodeType": "Block",
                        "statements": [
                            {
                                "nodeType": "Assignment",
                                "expression": "totalReceived += msg.value"
                            },
                            {
                                "nodeType": "EmitStatement",
                                "expression": "emit DepositReceived(msg.sender, msg.value)"
                            }
                        ]
                    }
                }
            ]
        }

        analysis = analyzer.analyze_contract_ast("0xLegitVault", legit_fallback_ast)
        fallback_risks = [r for r in analysis["risks"] if r["risk_type"] == SelectorRiskType.UNEMITTED_FALLBACK_STATE_CHANGE.value]
        assert len(fallback_risks) == 0


class TestConvenienceAndDisclosure:
    """Test convenience functions and risk disclosures"""

    def test_analyze_ast_selectors_convenience(self):
        """Test analyze_ast_selectors helper"""
        sample_ast = {"nodes": []}
        result = analyze_ast_selectors("0xTestContract", sample_ast)
        assert result["contract_id"] == "0xTestContract"
        assert "selector_mappings" in result
        assert "risks" in result
        assert "risk_level" in result

    def test_generate_risk_disclosure_format(self):
        """Test formatting of human-readable risk disclosure string"""
        analyzer = ASTSelectorAnalyzer()
        mock_result = {
            "contract_id": "0x12345",
            "selector_mappings": {"0xa9059cbb": {}},
            "risks": [
                {
                    "severity": "critical",
                    "risk_type": "signature_collision",
                    "description": "Test collision",
                    "affected_functions": ["drain()"]
                }
            ],
            "risk_level": "CRITICAL",
            "recommendations": ["Rename custom function"]
        }

        disclosure = analyzer.generate_risk_disclosure(mock_result)
        assert "AST SELECTOR CLASH & FALLBACK SECURITY AUDIT DISCLOSURE" in disclosure
        assert "Contract ID: 0x12345" in disclosure
        assert "CRITICAL" in disclosure
        assert "RECOMMENDED REMEDIATIONS" in disclosure


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
