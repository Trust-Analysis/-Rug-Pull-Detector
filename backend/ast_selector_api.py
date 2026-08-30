"""
AST Selector Analysis API
FastAPI endpoint for AST 4-byte selector clash and fallback security analysis.
"""

from fastapi import FastAPI, HTTPException
from pydantic import BaseModel
from typing import Optional, List, Dict, Any
from ast_selector_analyzer import (
    ASTSelectorAnalyzer,
    SelectorRiskType,
    analyze_ast_selectors
)
from rate_limiter import RateLimiter, RateLimitMiddleware


app = FastAPI(title="AST Selector Clash Analysis API")

# Initialize rate limiter: 10 requests per second, 100 burst capacity
rate_limiter = RateLimiter(rate=10.0, capacity=100)

# Add rate limiting middleware
app.add_middleware(RateLimitMiddleware, rate_limiter=rate_limiter)


class ASTSelectorAnalysisRequest(BaseModel):
    contractId: str
    astData: Dict[str, Any]


class ASTSelectorAnalysisResponse(BaseModel):
    contractId: str
    selector_mappings: Dict[str, Dict[str, Any]]
    risks: List[Dict[str, Any]]
    risk_level: str
    recommendations: List[str]
    disclosure: str


# Initialize analyzer
ast_analyzer = ASTSelectorAnalyzer()


@app.post("/api/ast-selector-analyze", response_model=ASTSelectorAnalysisResponse)
async def analyze_ast_selectors_endpoint(request: ASTSelectorAnalysisRequest):
    """
    Analyze contract AST for 4-byte selector collisions and deceptive payable fallbacks
    
    Args:
        request: ASTSelectorAnalysisRequest with contractId and astData
        
    Returns:
        ASTSelectorAnalysisResponse with selector mappings, risks, and disclosure
    """
    try:
        analysis_result = ast_analyzer.analyze_contract_ast(
            request.contractId,
            request.astData
        )
        
        disclosure = ast_analyzer.generate_risk_disclosure(analysis_result)
        
        return ASTSelectorAnalysisResponse(
            contractId=request.contractId,
            selector_mappings=analysis_result["selector_mappings"],
            risks=analysis_result["risks"],
            risk_level=analysis_result["risk_level"],
            recommendations=analysis_result["recommendations"],
            disclosure=disclosure
        )
    except Exception as e:
        raise HTTPException(status_code=500, detail=str(e))


@app.get("/health")
async def health_check():
    """Health check endpoint"""
    return {"status": "healthy", "service": "ast-selector-analyzer"}


if __name__ == "__main__":
    import uvicorn
    uvicorn.run(app, host="0.0.0.0", port=8002)
