/// The token for email verification.
///
/// In addition to general email confirmation, we use this verifacion token
/// also for an user activation by the user's primary email address.
use std::ops::Deref;

use redis::{Commands, RedisError};
use rocket::{Request, State};
use rocket::http::RawStr;
use rocket::request::{FromRequest, Outcome};
use rocket_slog::SyncLogger;

use config::Config;
use model::token::VerificationClaims;
use request::token::{AUTHORIZATION_HEADER_PREFIX, verify_token};
use ss::SsConn;

use {bad_request_by, not_found_by, unprocessable_entity_by};

pub struct VerificationToken(pub String);

impl Deref for VerificationToken {
    type Target = str;

    fn deref(&self) -> &str {
        &self.0
    }
}

#[derive(Debug)]
pub enum VerificationTokenError {
    Expired,
    Invalid,
    Missing,
    Unknown,
}

// Extract and verify verification token given through HTTP Authorization
// header and a private cookie.
impl<'a, 'r> FromRequest<'a, 'r> for VerificationToken {
    type Error = VerificationTokenError;

    fn from_request(req: &'a Request<'r>) -> Outcome<Self, Self::Error> {
        let logger = req.guard::<State<SyncLogger>>().unwrap();

        if req.headers().get_one("X-Requested-With") != Some("XMLHttpRequest") {
            error!(logger, "request: {}", req);
            return bad_request_by!(VerificationTokenError::Invalid);
        }

        let headers: Vec<_> = req.headers().get("Authorization").collect();
        match headers.len() {
            1 => {
                let h = &headers[0];
                if !h.starts_with(AUTHORIZATION_HEADER_PREFIX) {
                    return bad_request_by!(VerificationTokenError::Invalid);
                }

                // TODO:
                // * check Origin and Referer header
                // * validate token format

                let token = h[AUTHORIZATION_HEADER_PREFIX.len()..].to_string();
                if !token.contains('.') {
                    return bad_request_by!(VerificationTokenError::Invalid);
                }
                // NOTE:
                // append signature taken by session id to the parts extracted
                // from authorization header.
                let mut ss_conn = req.guard::<SsConn>().unwrap();
                // /_api/password/reset/<...> and /_api/user/activate/<...>
                let session_id: &'a RawStr = req
                    .get_param(2)
                    .and_then(|r| r.ok())
                    .unwrap_or_else(|| "".into());

                if session_id.is_empty() {
                    return bad_request_by!(VerificationTokenError::Invalid);
                }

                let result: Result<String, RedisError> =
                    ss_conn.get(session_id.as_str()).map_err(|e| {
                        error!(logger, "error: {}", e);
                        e
                    });
                if result.is_err() {
                    return not_found_by!(VerificationTokenError::Unknown);
                }

                let verification_token = token + "." + &result.unwrap();
                let config = req.guard::<State<Config>>().unwrap();
                match verify_token::<VerificationClaims>(
                    &verification_token,
                    &config.verification_token_issuer,
                    &config.verification_token_secret,
                ) {
                    Ok(t) => Outcome::Success(VerificationToken(t)),
                    Err(e) => {
                        error!(logger, "error: {}", e);
                        unprocessable_entity_by!(
                            VerificationTokenError::Expired
                        )
                    },
                }
            },
            0 => {
                error!(logger, "error: Authorization header is missing");
                bad_request_by!(VerificationTokenError::Missing)
            },
            _ => bad_request_by!(VerificationTokenError::Invalid),
        }
    }
}
