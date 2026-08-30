use crate::UgosError;
use der::{Reader, SliceReader, Tag, TagNumber};
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::{CryptoProvider, ring, verify_tls13_signature_with_raw_key};
use rustls::pki_types::{CertificateDer, ServerName, SubjectPublicKeyInfoDer, UnixTime};
use rustls::{DigitallySignedStruct, Error as RustlsError, SignatureScheme};
use sha2::{Digest, Sha256};
use std::sync::{Arc, Mutex};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct CertFingerprint([u8; 32]);

impl CertFingerprint {
    #[cfg(any(not(debug_assertions), test))]
    pub(crate) fn from_hex(input: &str) -> Result<Self, UgosError> {
        let decoded = hex::decode(input)
            .map_err(|error| UgosError::Encryption(format!("invalid fingerprint: {error}")))?;
        if decoded.len() != 32 {
            return Err(UgosError::Encryption(
                "certificate fingerprint must contain 32 bytes".to_owned(),
            ));
        }
        let mut bytes = [0; 32];
        bytes.copy_from_slice(&decoded);
        Ok(Self(bytes))
    }

    fn of(certificate: &[u8]) -> Self {
        let digest = Sha256::digest(certificate);
        let mut bytes = [0; 32];
        bytes.copy_from_slice(&digest);
        Self(bytes)
    }

    pub(crate) fn to_hex(self) -> String {
        hex::encode(self.0)
    }
}

#[derive(Debug)]
struct PinnedVerifier {
    fingerprint: CertFingerprint,
    provider: Arc<CryptoProvider>,
}

#[derive(Debug)]
struct LearningVerifier {
    fingerprint: Arc<Mutex<Option<CertFingerprint>>>,
    provider: Arc<CryptoProvider>,
}

impl ServerCertVerifier for PinnedVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, RustlsError> {
        if CertFingerprint::of(end_entity.as_ref()) == self.fingerprint {
            return Ok(ServerCertVerified::assertion());
        }
        Err(RustlsError::General(
            "UGOS certificate fingerprint changed".to_owned(),
        ))
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _certificate: &CertificateDer<'_>,
        _signature: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, RustlsError> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        certificate: &CertificateDer<'_>,
        signature: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, RustlsError> {
        if CertFingerprint::of(certificate.as_ref()) != self.fingerprint {
            return Err(RustlsError::General(
                "UGOS certificate changed during the TLS handshake".to_owned(),
            ));
        }
        let subject_public_key = subject_public_key(certificate.as_ref())
            .map_err(|error| RustlsError::General(error.to_string()))?;
        verify_tls13_signature_with_raw_key(
            message,
            &SubjectPublicKeyInfoDer::from(subject_public_key.as_slice()),
            signature,
            &self.provider.signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.provider
            .signature_verification_algorithms
            .supported_schemes()
    }
}

impl ServerCertVerifier for LearningVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, RustlsError> {
        let mut fingerprint = self
            .fingerprint
            .lock()
            .map_err(|error| RustlsError::General(error.to_string()))?;
        *fingerprint = Some(CertFingerprint::of(end_entity.as_ref()));
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _certificate: &CertificateDer<'_>,
        _signature: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, RustlsError> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        certificate: &CertificateDer<'_>,
        signature: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, RustlsError> {
        let subject_public_key = subject_public_key(certificate.as_ref())
            .map_err(|error| RustlsError::General(error.to_string()))?;
        verify_tls13_signature_with_raw_key(
            message,
            &SubjectPublicKeyInfoDer::from(subject_public_key.as_slice()),
            signature,
            &self.provider.signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.provider
            .signature_verification_algorithms
            .supported_schemes()
    }
}

fn client_config(verifier: Arc<dyn ServerCertVerifier>) -> Result<rustls::ClientConfig, UgosError> {
    rustls::ClientConfig::builder_with_provider(Arc::new(ring::default_provider()))
        .with_safe_default_protocol_versions()
        .map_err(|error| UgosError::Encryption(format!("could not enable TLS: {error}")))
        .map(|builder| {
            builder
                .dangerous()
                .with_custom_certificate_verifier(verifier)
                .with_no_client_auth()
        })
}

pub(crate) fn http_client(fingerprint: CertFingerprint) -> Result<reqwest::Client, UgosError> {
    let verifier = Arc::new(PinnedVerifier {
        fingerprint,
        provider: Arc::new(ring::default_provider()),
    });
    reqwest::Client::builder()
        .no_proxy()
        .cookie_store(true)
        .timeout(std::time::Duration::from_secs(30))
        .use_preconfigured_tls(client_config(verifier)?)
        .build()
        .map_err(|error| UgosError::Encryption(format!("could not build HTTP client: {error}")))
}

pub(crate) async fn probe_fingerprint(host: &str, port: u16) -> Result<CertFingerprint, UgosError> {
    let fingerprint = Arc::new(Mutex::new(None));
    let verifier = Arc::new(LearningVerifier {
        fingerprint: Arc::clone(&fingerprint),
        provider: Arc::new(ring::default_provider()),
    });
    let client = reqwest::Client::builder()
        .no_proxy()
        .timeout(std::time::Duration::from_secs(30))
        .use_preconfigured_tls(client_config(verifier)?)
        .build()
        .map_err(|error| UgosError::Encryption(format!("could not build HTTP client: {error}")))?;
    client.get(format!("https://{host}:{port}/")).send().await?;
    let observed = fingerprint
        .lock()
        .map_err(|error| UgosError::Encryption(error.to_string()))?;
    observed.ok_or_else(|| UgosError::Encryption("UGOS certificate was not observed".to_owned()))
}

fn subject_public_key(certificate: &[u8]) -> Result<Vec<u8>, UgosError> {
    fn read(certificate: &[u8]) -> Result<Vec<u8>, der::Error> {
        let mut outer = SliceReader::new(certificate)?;
        outer.sequence(|certificate| {
            let subject_public_key = certificate.sequence(|signed_certificate| {
                let version = Tag::ContextSpecific {
                    constructed: true,
                    number: TagNumber(0),
                };
                if Tag::peek(signed_certificate)? == version {
                    signed_certificate.tlv_bytes()?;
                }
                signed_certificate.tlv_bytes()?;
                signed_certificate.tlv_bytes()?;
                signed_certificate.tlv_bytes()?;
                signed_certificate.tlv_bytes()?;
                signed_certificate.tlv_bytes()?;
                let subject_public_key = signed_certificate.tlv_bytes()?.to_vec();
                while !signed_certificate.is_finished() {
                    signed_certificate.tlv_bytes()?;
                }
                Ok::<Vec<u8>, der::Error>(subject_public_key)
            })?;
            while !certificate.is_finished() {
                certificate.tlv_bytes()?;
            }
            Ok(subject_public_key)
        })
    }

    read(certificate).map_err(|error| {
        UgosError::Encryption(format!(
            "could not read the UGOS certificate public key: {error}"
        ))
    })
}

#[cfg(test)]
#[path = "../tests/unit/tls.rs"]
mod tests;
