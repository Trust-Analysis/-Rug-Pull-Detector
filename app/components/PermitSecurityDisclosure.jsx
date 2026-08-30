'use client';

import { AlertCircle, Shield, AlertTriangle, CheckCircle, RefreshCw } from 'lucide-react';

function PermitSecurityDisclosure({ contractAddress, permitAnalysisResult, onRefresh }) {
  const getSeverityColor = (severity) => {
    switch (severity) {
      case 'critical':
        return 'text-danger-400 bg-danger-500/20 border-danger-500/50';
      case 'high':
        return 'text-orange-400 bg-orange-500/20 border-orange-500/50';
      case 'medium':
        return 'text-yellow-400 bg-yellow-500/20 border-yellow-500/50';
      case 'low':
        return 'text-blue-400 bg-blue-500/20 border-blue-500/50';
      default:
        return 'text-gray-400 bg-gray-500/20 border-gray-500/50';
    }
  };

  const getSeverityIcon = (severity) => {
    switch (severity) {
      case 'critical':
        return <AlertCircle className="w-5 h-5" />;
      case 'high':
        return <AlertTriangle className="w-5 h-5" />;
      case 'medium':
        return <AlertTriangle className="w-5 h-5" />;
      case 'low':
        return <AlertCircle className="w-5 h-5" />;
      default:
        return <AlertCircle className="w-5 h-5" />;
    }
  };

  const hasPermitFunctionality = permitAnalysisResult?.has_permit_functionality;
  const risks = permitAnalysisResult?.risks || [];
  const criticalRisks = risks.filter(r => r.severity === 'critical');
  const highRisks = risks.filter(r => r.severity === 'high');
  const riskMultiplier = permitAnalysisResult?.risk_multiplier || 1.0;

  return (
    <div className="space-y-4 p-4 bg-white/5 border border-white/10 rounded-lg">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <Shield className="w-5 h-5 text-primary-400" />
          <div>
            <h3 className="text-sm font-medium text-gray-300">
              Permit Security Analysis
            </h3>
            <p className="text-xs text-gray-400">
              EIP-712/EIP-2612 Gasless Approval Security
            </p>
          </div>
        </div>
        <button
          onClick={onRefresh}
          className="p-2 hover:bg-white/10 rounded-lg transition-colors"
          title="Refresh analysis"
        >
          <RefreshCw className="w-4 h-4 text-gray-400" />
        </button>
      </div>

      {!hasPermitFunctionality ? (
        <div className="p-4 bg-success-500/20 border border-success-500/50 rounded-lg">
          <div className="flex items-center gap-2 text-success-300">
            <CheckCircle className="w-5 h-5" />
            <span className="text-sm">
              No permit functionality detected in this contract
            </span>
          </div>
        </div>
      ) : (
        <>
          {/* Security Summary */}
          <div className="grid grid-cols-2 gap-4">
            <div className="p-3 bg-white/5 border border-white/10 rounded-lg">
              <div className="text-xs text-gray-400 mb-1">Security Score</div>
              <div className="text-2xl font-bold text-gray-300">
                {Math.max(0, 10 - riskMultiplier).toFixed(1)}/10
              </div>
            </div>
            <div className="p-3 bg-white/5 border border-white/10 rounded-lg">
              <div className="text-xs text-gray-400 mb-1">Risk Level</div>
              <div className={`text-2xl font-bold ${
                riskMultiplier >= 4.0 ? 'text-danger-400' :
                riskMultiplier >= 2.0 ? 'text-orange-400' :
                riskMultiplier >= 1.0 ? 'text-yellow-400' :
                'text-success-400'
              }`}>
                {riskMultiplier >= 4.0 ? 'CRITICAL' :
                 riskMultiplier >= 2.0 ? 'HIGH' :
                 riskMultiplier >= 1.0 ? 'MEDIUM' : 'LOW'}
              </div>
            </div>
          </div>

          {/* Permit Functions */}
          {permitAnalysisResult?.permit_functions && permitAnalysisResult.permit_functions.length > 0 && (
            <div className="space-y-2">
              <h4 className="text-xs font-medium text-gray-400 uppercase tracking-wider">
                Detected Permit Functions
              </h4>
              {permitAnalysisResult.permit_functions.map((func, index) => (
                <div key={index} className="p-3 bg-white/5 border border-white/10 rounded-lg">
                  <div className="flex items-center justify-between mb-2">
                    <span className="text-sm font-medium text-gray-300">
                      {func.function_name}
                    </span>
                    <span className="text-xs text-gray-400 font-mono">
                      {func.function_selector}
                    </span>
                  </div>
                  <div className="space-y-1 text-xs">
                    <div className="flex items-center gap-2">
                      <span className="text-gray-400">Domain Separator:</span>
                      <span className={`${
                        func.domain_separator_implementation === 'dynamic' ? 'text-success-400' :
                        func.domain_separator_implementation === 'hardcoded' ? 'text-danger-400' :
                        'text-yellow-400'
                      }`}>
                        {func.domain_separator_implementation}
                      </span>
                    </div>
                    <div className="flex items-center gap-2">
                      <span className="text-gray-400">Chain ID Validation:</span>
                      <span className={`${
                        func.has_chain_id_validation ? 'text-success-400' : 'text-danger-400'
                      }`}>
                        {func.has_chain_id_validation ? '✓ Validated' : '✗ Missing'}
                      </span>
                    </div>
                  </div>
                </div>
              ))}
            </div>
          )}

          {/* Nonce Analysis */}
          {permitAnalysisResult?.nonce_analysis && (
            <div className="space-y-2">
              <h4 className="text-xs font-medium text-gray-400 uppercase tracking-wider">
                Nonce Implementation
              </h4>
              <div className="p-3 bg-white/5 border border-white/10 rounded-lg">
                <div className="grid grid-cols-2 gap-4 text-xs">
                  <div className="flex items-center gap-2">
                    <span className="text-gray-400">Nonce Tracking:</span>
                    <span className={`${
                      permitAnalysisResult.nonce_analysis.has_nonces_mapping ? 'text-success-400' : 'text-danger-400'
                    }`}>
                      {permitAnalysisResult.nonce_analysis.has_nonces_mapping ? '✓ Implemented' : '✗ Missing'}
                    </span>
                  </div>
                  <div className="flex items-center gap-2">
                    <span className="text-gray-400">Per-Address:</span>
                    <span className={`${
                      permitAnalysisResult.nonce_analysis.tracks_per_address ? 'text-success-400' : 'text-danger-400'
                    }`}>
                      {permitAnalysisResult.nonce_analysis.tracks_per_address ? '✓ Yes' : '✗ No'}
                    </span>
                  </div>
                  <div className="flex items-center gap-2">
                    <span className="text-gray-400">Increment Pattern:</span>
                    <span className={`${
                      permitAnalysisResult.nonce_analysis.nonce_increment_pattern === 'monotonic' ? 'text-success-400' :
                      permitAnalysisResult.nonce_analysis.nonce_increment_pattern === 'arbitrary' ? 'text-danger-400' :
                      'text-yellow-400'
                    }`}>
                      {permitAnalysisResult.nonce_analysis.nonce_increment_pattern}
                    </span>
                  </div>
                </div>
              </div>
            </div>
          )}

          {/* Deadline Analysis */}
          {permitAnalysisResult?.deadline_analysis && (
            <div className="space-y-2">
              <h4 className="text-xs font-medium text-gray-400 uppercase tracking-wider">
                Deadline Validation
              </h4>
              <div className="p-3 bg-white/5 border border-white/10 rounded-lg">
                <div className="grid grid-cols-2 gap-4 text-xs">
                  <div className="flex items-center gap-2">
                    <span className="text-gray-400">Deadline Check:</span>
                    <span className={`${
                      permitAnalysisResult.deadline_analysis.has_deadline_check ? 'text-success-400' : 'text-danger-400'
                    }`}>
                      {permitAnalysisResult.deadline_analysis.has_deadline_check ? '✓ Implemented' : '✗ Missing'}
                    </span>
                  </div>
                  <div className="flex items-center gap-2">
                    <span className="text-gray-400">Comparator:</span>
                    <span className={`${
                      permitAnalysisResult.deadline_analysis.comparator_used === '>' ? 'text-success-400' :
                      permitAnalysisResult.deadline_analysis.comparator_used === '>=' ? 'text-yellow-400' :
                      'text-gray-400'
                    }`}>
                      {permitAnalysisResult.deadline_analysis.comparator_used || 'N/A'}
                    </span>
                  </div>
                  <div className="flex items-center gap-2">
                    <span className="text-gray-400">Reverts on Expiry:</span>
                    <span className={`${
                      permitAnalysisResult.deadline_analysis.reverts_on_expiry ? 'text-success-400' : 'text-danger-400'
                    }`}>
                      {permitAnalysisResult.deadline_analysis.reverts_on_expiry ? '✓ Yes' : '✗ No'}
                    </span>
                  </div>
                </div>
              </div>
            </div>
          )}

          {/* Ecrecover Analysis */}
          {permitAnalysisResult?.ecrecover_analysis && (
            <div className="space-y-2">
              <h4 className="text-xs font-medium text-gray-400 uppercase tracking-wider">
                Signature Validation
              </h4>
              <div className="p-3 bg-white/5 border border-white/10 rounded-lg">
                <div className="grid grid-cols-2 gap-4 text-xs">
                  <div className="flex items-center gap-2">
                    <span className="text-gray-400">Uses Ecrecover:</span>
                    <span className={`${
                      permitAnalysisResult.ecrecover_analysis.uses_ecrecover ? 'text-success-400' : 'text-gray-400'
                    }`}>
                      {permitAnalysisResult.ecrecover_analysis.uses_ecrecover ? '✓ Yes' : '✗ No'}
                    </span>
                  </div>
                  <div className="flex items-center gap-2">
                    <span className="text-gray-400">Validates Signer:</span>
                    <span className={`${
                      permitAnalysisResult.ecrecover_analysis.validates_signer ? 'text-success-400' : 'text-danger-400'
                    }`}>
                      {permitAnalysisResult.ecrecover_analysis.validates_signer ? '✓ Yes' : '✗ No'}
                    </span>
                  </div>
                  <div className="flex items-center gap-2">
                    <span className="text-gray-400">Zero Address Check:</span>
                    <span className={`${
                      permitAnalysisResult.ecrecover_analysis.checks_zero_address ? 'text-success-400' : 'text-danger-400'
                    }`}>
                      {permitAnalysisResult.ecrecover_analysis.checks_zero_address ? '✓ Yes' : '✗ No'}
                    </span>
                  </div>
                  <div className="flex items-center gap-2">
                    <span className="text-gray-400">Zero Address Handling:</span>
                    <span className={`${
                      permitAnalysisResult.ecrecover_analysis.zero_address_handling === 'reverts' ? 'text-success-400' :
                      permitAnalysisResult.ecrecover_analysis.zero_address_handling === 'accepts' ? 'text-danger-400' :
                      'text-yellow-400'
                    }`}>
                      {permitAnalysisResult.ecrecover_analysis.zero_address_handling}
                    </span>
                  </div>
                </div>
              </div>
            </div>
          )}

          {/* Critical Risks */}
          {criticalRisks.length > 0 && (
            <div className="space-y-2">
              <h4 className="text-xs font-medium text-danger-400 uppercase tracking-wider flex items-center gap-2">
                <AlertCircle className="w-4 h-4" />
                Critical Vulnerabilities
              </h4>
              {criticalRisks.map((risk, index) => (
                <div key={index} className={`p-3 border rounded-lg ${getSeverityColor(risk.severity)}`}>
                  <div className="flex items-start gap-2">
                    {getSeverityIcon(risk.severity)}
                    <div className="flex-1">
                      <div className="text-sm font-medium mb-1">
                        {risk.risk_type.replace(/_/g, ' ').toUpperCase()}
                      </div>
                      <div className="text-xs opacity-80 mb-2">
                        {risk.description}
                      </div>
                      {risk.recommendation && (
                        <div className="text-xs font-medium mt-2 pt-2 border-t border-white/10">
                          <span className="opacity-70">Recommendation: </span>
                          {risk.recommendation}
                        </div>
                      )}
                    </div>
                  </div>
                </div>
              ))}
            </div>
          )}

          {/* High Risks */}
          {highRisks.length > 0 && (
            <div className="space-y-2">
              <h4 className="text-xs font-medium text-orange-400 uppercase tracking-wider flex items-center gap-2">
                <AlertTriangle className="w-4 h-4" />
                High Priority Issues
              </h4>
              {highRisks.map((risk, index) => (
                <div key={index} className={`p-3 border rounded-lg ${getSeverityColor(risk.severity)}`}>
                  <div className="flex items-start gap-2">
                    {getSeverityIcon(risk.severity)}
                    <div className="flex-1">
                      <div className="text-sm font-medium mb-1">
                        {risk.risk_type.replace(/_/g, ' ').toUpperCase()}
                      </div>
                      <div className="text-xs opacity-80 mb-2">
                        {risk.description}
                      </div>
                      {risk.recommendation && (
                        <div className="text-xs font-medium mt-2 pt-2 border-t border-white/10">
                          <span className="opacity-70">Recommendation: </span>
                          {risk.recommendation}
                        </div>
                      )}
                    </div>
                  </div>
                </div>
              ))}
            </div>
          )}

          {/* Recommendations */}
          {permitAnalysisResult?.recommendations && permitAnalysisResult.recommendations.length > 0 && (
            <div className="space-y-2">
              <h4 className="text-xs font-medium text-gray-400 uppercase tracking-wider">
                Security Recommendations
              </h4>
              <div className="p-3 bg-primary-500/10 border border-primary-500/30 rounded-lg">
                <ul className="space-y-2">
                  {permitAnalysisResult.recommendations.map((rec, index) => (
                    <li key={index} className="text-xs text-gray-300 flex items-start gap-2">
                      <span className="text-primary-400 mt-0.5">•</span>
                      <span>{rec}</span>
                    </li>
                  ))}
                </ul>
              </div>
            </div>
          )}
        </>
      )}
    </div>
  );
}

export default PermitSecurityDisclosure;