"""
Permit Security Analysis API
FastAPI endpoint for EIP-712/EIP-2612 permit function security analysis.
"""

from fastapi import FastAPI, HTTPException
from pydantic import BaseModel
from typing import Optional, List, Dict
from permit_security_analyzer import (
    PermitSecurityAnalyzer,
    PermitRiskType,
    analyze_permit_security
)
from rate_limiter import RateLimiter, RateLimitMiddleware
import asyncio


app = FastAPI(title="Permit Security Analysis API")

# Initialize rate limiter: 10 requests per second, 100 burst capacity
rate_limiter = RateLimiter(rate=10.0, capacity=100)

# Add rate limiting middleware
app.add_middleware(RateLimitMiddleware, rate_limiter=rate_limiter)


class PermitAnalysisRequest(BaseModel):
    contractAddress: str
    rpcUrl: Optional[str] = None


class PermitAnalysisResponse(BaseModel):
    contract_address: str
    has_permit_functionality: bool
    permit_functions: List[Dict]
    nonce_analysis: Dict
    deadline_analysis: Dict
    ecrecover_analysis: Dict
    risks: List[Dict]
    risk_multiplier: float
    recommendations: List[str]


# Initialize analyzer (without RPC URL - will be provided per request)
permit_analyzer = None


@app.post("/api/permit-security-analyze", response_model=PermitAnalysisResponse)
async def analyze_permit_security_endpoint(request: PermitAnalysisRequest):
    """
    Analyze a contract for EIP-712/EIP-2612 permit security vulnerabilities
    
    Args:
        request: PermitAnalysisRequest with contractAddress and optional rpcUrl
        
    Returns:
        PermitAnalysisResponse with permit security analysis results
    """
    try:
        # Create analyzer with provided RPC URL or default
        analyzer = PermitSecurityAnalyzer(rpc_url=request.rpcUrl)
        
        # Analyze contract (async)
        analysis_result = await analyzer.analyze_permit_security(request.contractAddress)
        
        return PermitAnalysisResponse(
            contract_address=analysis_result["contract_address"],
            has_permit_functionality=analysis_result["has_permit_functionality"],
            permit_functions=analysis_result.get("permit_functions", []),
            nonce_analysis=analysis_result.get("nonce_analysis", {}),
            deadline_analysis=analysis_result.get("deadline_analysis", {}),
            ecrecover_analysis=analysis_result.get("ecrecover_analysis", {}),
            risks=analysis_result["risks"],
            risk_multiplier=analysis_result["risk_multiplier"],
            recommendations=analysis_result["recommendations"]
        )
        
    except Exception as e:
        raise HTTPException(status_code=500, detail=str(e))


@app.get("/health")
async def health_check():
    """Health check endpoint"""
    return {"status": "healthy", "service": "permit-security-analyzer"}


if __name__ == "__main__":
    import uvicorn
    uvicorn.run(app, host="0.0.0.0", port=8003)