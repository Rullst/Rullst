use super::*;
use axum::{extract::Request, http::header};
use rullst_core::security::{MachineEndpointError, MachineRequestVerifier};

/// Exact-scope adapter for Core `MachineEndpoint::verified_bearer`. Successful
/// requests receive an `ApiTokenPrincipal` extension; handlers still enforce
/// current tenant membership and domain/resource authorization. No URL token,
/// cookie or duplicate Authorization header can establish this identity.
#[derive(Clone)]
pub struct ApiTokenVerifier {
    service: ApiTokenService,
    required: ApiScopes,
}

impl ApiTokenService {
    pub fn machine_verifier(&self, required: ApiScopes) -> Result<ApiTokenVerifier, RecoveryError> {
        if !self.config.scopes.includes(&required) {
            return Err(RecoveryError::InvalidInput);
        }
        Ok(ApiTokenVerifier {
            service: self.clone(),
            required,
        })
    }
}

#[async_trait::async_trait]
impl MachineRequestVerifier for ApiTokenVerifier {
    async fn verify(&self, mut request: Request) -> Result<Request, MachineEndpointError> {
        if request.headers().contains_key(header::COOKIE)
            || request.headers().contains_key(header::ORIGIN)
            || request.headers().contains_key("sec-fetch-site")
        {
            return Err(MachineEndpointError::Unauthorized);
        }
        let bearer = {
            let mut values = request.headers().get_all(header::AUTHORIZATION).iter();
            let bearer = values
                .next()
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.strip_prefix("Bearer "))
                .filter(|value| value.len() == 92)
                .ok_or(MachineEndpointError::Unauthorized)?;
            if values.next().is_some() {
                return Err(MachineEndpointError::Unauthorized);
            }
            zeroize::Zeroizing::new(bearer.to_owned())
        };
        let principal = self
            .service
            .verify(&bearer, &self.required, &super::super::SystemAuthClock)
            .await
            .map_err(|_| MachineEndpointError::Unauthorized)?;
        request.extensions_mut().insert(principal);
        Ok(request)
    }
}
