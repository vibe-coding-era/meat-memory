pub mod domain;
pub mod ports;

use anyhow::Result;
use async_trait::async_trait;
use memory_domain::{Artifact, ContextBundle, Memory, ScopeId};
use tracing::info;

#[derive(Debug, Clone)]
pub struct ServiceInfo {
    pub name: &'static str,
    pub version: &'static str,
    pub default_scope: ScopeId,
}

impl Default for ServiceInfo {
    fn default() -> Self {
        Self {
            name: "meat-memory",
            version: env!("CARGO_PKG_VERSION"),
            default_scope: ScopeId::from_string("scp_default_local"),
        }
    }
}

#[async_trait]
pub trait MemoryService: Send + Sync {
    async fn remember(&self, artifact: Artifact) -> Result<Memory>;
    async fn fetch_context(&self, query: &str, scope_id: ScopeId) -> Result<ContextBundle>;
}

pub fn startup_banner(service_info: &ServiceInfo) -> String {
    format!(
        "{} v{} bootstrapped for scope {}",
        service_info.name,
        service_info.version,
        service_info.default_scope.as_str()
    )
}

pub fn log_startup(service_info: &ServiceInfo) {
    info!(
        service = service_info.name,
        version = service_info.version,
        "memory kernel initialized"
    );
}

#[cfg(test)]
mod tests {
    use super::{ServiceInfo, log_startup, startup_banner};

    #[test]
    fn default_service_info_exposes_local_defaults() {
        let service_info = ServiceInfo::default();

        assert_eq!(service_info.name, "meat-memory");
        assert_eq!(service_info.version, env!("CARGO_PKG_VERSION"));
        assert_eq!(service_info.default_scope.as_str(), "scp_default_local");
    }

    #[test]
    fn startup_banner_includes_name_version_and_scope() {
        let service_info = ServiceInfo::default();
        let banner = startup_banner(&service_info);

        assert!(banner.contains("meat-memory"));
        assert!(banner.contains(env!("CARGO_PKG_VERSION")));
        assert!(banner.contains("scp_default_local"));
    }

    #[test]
    fn log_startup_accepts_service_info() {
        log_startup(&ServiceInfo::default());
    }
}
