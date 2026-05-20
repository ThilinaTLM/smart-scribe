//! OAuth token value object and JWT helpers

use std::time::{SystemTime, UNIX_EPOCH};

use base64::Engine;
use serde::{Deserialize, Serialize};

use super::error::OAuthError;

/// Persistent OAuth credentials for the ChatGPT backend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthToken {
    pub access_token: String,
    pub refresh_token: String,
    /// Absolute expiry of `access_token`, as Unix seconds.
    pub expires_at_unix: i64,
    /// ChatGPT account id extracted from the access-token JWT.
    pub account_id: String,
}

impl OAuthToken {
    /// True if the access token has expired or is within `lead_secs` seconds
    /// of expiring.
    pub fn is_expired_within(&self, lead_secs: i64) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        self.expires_at_unix.saturating_sub(now) <= lead_secs
    }
}

/// Decode the JWT access token to extract the `exp` claim (absolute Unix seconds)
/// and the `https://api.openai.com/auth.chatgpt_account_id` claim.
///
/// We do NOT verify the JWT signature — we trust the token because we obtained
/// it directly from `auth.openai.com` over TLS. The decoded claims are only
/// used as cache metadata, never for authorization decisions.
pub fn decode_jwt_exp_and_account_id(jwt: &str) -> Result<(i64, String), OAuthError> {
    let parts: Vec<&str> = jwt.split('.').collect();
    if parts.len() != 3 {
        return Err(OAuthError::JwtParse(
            "expected 3 dot-separated segments".into(),
        ));
    }

    let payload_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(parts[1].as_bytes())
        .or_else(|_| {
            // Some libraries pad — try the padding-tolerant alphabet too.
            base64::engine::general_purpose::URL_SAFE.decode(parts[1].as_bytes())
        })
        .map_err(|e| OAuthError::JwtParse(format!("base64 decode failed: {e}")))?;

    let claims: JwtClaims = serde_json::from_slice(&payload_bytes)
        .map_err(|e| OAuthError::JwtParse(format!("json decode failed: {e}")))?;

    let exp = claims
        .exp
        .ok_or_else(|| OAuthError::JwtParse("missing exp claim".into()))?;

    let account_id = claims
        .openai_auth
        .as_ref()
        .and_then(|a| a.chatgpt_account_id.clone())
        .or_else(|| {
            claims
                .organizations
                .as_ref()
                .and_then(|orgs| orgs.first().map(|o| o.id.clone()))
        })
        .ok_or_else(|| OAuthError::JwtParse("missing chatgpt_account_id claim".into()))?;

    Ok((exp, account_id))
}

#[derive(Debug, Deserialize)]
struct JwtClaims {
    exp: Option<i64>,
    #[serde(rename = "https://api.openai.com/auth")]
    openai_auth: Option<OpenAiAuth>,
    organizations: Option<Vec<Organization>>,
}

#[derive(Debug, Deserialize)]
struct OpenAiAuth {
    chatgpt_account_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Organization {
    id: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn b64url(bytes: &[u8]) -> String {
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
    }

    fn make_jwt(payload_json: &str) -> String {
        format!(
            "{}.{}.{}",
            b64url(b"{\"alg\":\"none\"}"),
            b64url(payload_json.as_bytes()),
            b64url(b"sig"),
        )
    }

    #[test]
    fn is_expired_within_returns_true_when_past() {
        let tok = OAuthToken {
            access_token: String::new(),
            refresh_token: String::new(),
            expires_at_unix: 0,
            account_id: String::new(),
        };
        assert!(tok.is_expired_within(0));
    }

    #[test]
    fn is_expired_within_returns_false_when_fresh() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let tok = OAuthToken {
            access_token: String::new(),
            refresh_token: String::new(),
            expires_at_unix: now + 3600,
            account_id: String::new(),
        };
        assert!(!tok.is_expired_within(60));
    }

    #[test]
    fn decode_jwt_picks_openai_auth_account_id() {
        let payload =
            r#"{"exp":1700000000,"https://api.openai.com/auth":{"chatgpt_account_id":"acc-abc"}}"#;
        let jwt = make_jwt(payload);
        let (exp, account) = decode_jwt_exp_and_account_id(&jwt).unwrap();
        assert_eq!(exp, 1700000000);
        assert_eq!(account, "acc-abc");
    }

    #[test]
    fn decode_jwt_falls_back_to_organizations() {
        let payload = r#"{"exp":42,"organizations":[{"id":"org-xyz"}]}"#;
        let jwt = make_jwt(payload);
        let (_, account) = decode_jwt_exp_and_account_id(&jwt).unwrap();
        assert_eq!(account, "org-xyz");
    }

    #[test]
    fn decode_jwt_errors_on_missing_exp() {
        let jwt = make_jwt(r#"{"https://api.openai.com/auth":{"chatgpt_account_id":"a"}}"#);
        assert!(decode_jwt_exp_and_account_id(&jwt).is_err());
    }

    #[test]
    fn decode_jwt_errors_on_malformed_input() {
        assert!(decode_jwt_exp_and_account_id("not-a-jwt").is_err());
    }
}
