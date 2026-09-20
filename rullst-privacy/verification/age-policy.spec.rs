// Verus-only clauses for the unchanged AgePolicy::permits executable body.
// All AgePolicy values and all three-by-three enum combinations are in scope.
// This file deliberately contains no executable implementation or precondition.
ensures
    self.risk_spec() == RiskLevel::Restricted ==> (allowed <==> method == AgeMethod::VerifiedAttribute),
    self.risk_spec() == RiskLevel::Elevated ==> (allowed <==> method != AgeMethod::SelfDeclaration),
    self.risk_spec() == RiskLevel::Low ==> allowed,
