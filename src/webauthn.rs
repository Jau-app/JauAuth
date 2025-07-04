//! WebAuthn implementation for biometric authentication

use webauthn_rs::prelude::*;

#[allow(dead_code)] // Field will be used when WebAuthn flows are implemented
pub struct WebAuthnManager {
    webauthn: Webauthn,
}

impl WebAuthnManager {
    pub fn new(rp_id: String, rp_origin: String, _rp_name: String) -> Self {
        let rp_id = rp_id.as_str();
        let rp_origin = Url::parse(&rp_origin).expect("Invalid RP origin");
        
        let builder = WebauthnBuilder::new(rp_id, &rp_origin)
            .expect("Invalid WebAuthn configuration");
        
        let webauthn = builder.build().expect("Failed to build WebAuthn");
        
        Self { webauthn }
    }
    
    // TODO: Implement registration and authentication flows
}