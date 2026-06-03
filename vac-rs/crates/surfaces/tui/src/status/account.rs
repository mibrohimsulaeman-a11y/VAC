#[derive(Debug, Clone)]
pub(crate) enum StatusAccountDisplay {
    ProviderCredential {
        email: Option<String>,
        plan: Option<String>,
    },
    ApiKey,
}
