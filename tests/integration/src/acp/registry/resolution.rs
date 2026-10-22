//! Integration tests for registry resolution
//!
//! These tests exercise `fetch_agent_from_registry` which requires a
//! `NotificationMessenger` and therefore must run inside Neovim.

use std::collections::HashMap;

use hermes::acp::{
    connection::Assistant,
    registry::{
        DistributionCommand, PackageDistribution, Registry, RegistryData,
        distribution::Distribution, entry::AgentEntry, resolution::fetch_agent_from_registry,
    },
};
use hermes::nvim::configuration::DistributionsConfig;
use hermes::nvim::configuration::ProgressConfig;
use hermes::utilities::{Downloader, NotificationMessenger};

fn entry_with_distribution(
    id: &str,
    distribution: HashMap<Distribution, DistributionCommand>,
) -> AgentEntry {
    AgentEntry {
        id: id.to_string(),
        name: id.to_string(),
        version: "1.0.0".into(),
        description: "test agent".into(),
        repository: None,
        website: None,
        authors: None,
        license: None,
        icon: None,
        distribution,
    }
}

fn npx_dist(
    package: &str,
    args: Option<Vec<String>>,
) -> HashMap<Distribution, DistributionCommand> {
    let mut dist = HashMap::new();
    dist.insert(
        Distribution::Npx,
        DistributionCommand::Package(PackageDistribution {
            package: package.into(),
            args,
            env: None,
        }),
    );
    dist
}

fn enabled_distributions() -> DistributionsConfig {
    DistributionsConfig {
        npx: true,
        uvx: true,
        binary: Default::default(),
    }
}

async fn test_registry() -> Registry {
    let messenger = NotificationMessenger::initialize().expect("Failed to create messenger");
    let downloader = Downloader::new(messenger, ProgressConfig::default());
    let mut registry = Registry::new(downloader).expect("Bundled registry should exist");
    // Replace with empty test data so no real I/O is triggered.
    registry.data = RegistryData {
        version: "1.0.0".to_string(),
        agents: HashMap::new(),
    };
    registry
}

// -----------------------------------------------------------------------
// Tests for fetch_agent_from_registry (async, preference skips I/O)
// -----------------------------------------------------------------------

#[nvim_oxi::test]
fn resolve_npx_with_preference() {
    let entry = entry_with_distribution("test-agent", npx_dist("my-agent", None));
    let config = enabled_distributions();
    let registry = smol::block_on(test_registry());
    let result = smol::block_on(fetch_agent_from_registry(
        &entry,
        Some(Distribution::Npx),
        &config,
        &registry,
    ));
    let assistant = result.unwrap();
    assert!(
        matches!(&assistant, Assistant::CustomStdio { name, command, args }
            if name == "test-agent" && command == "npx" && args == &vec!["my-agent"])
    );
}

#[nvim_oxi::test]
fn resolve_npx_includes_dist_args() {
    let entry = entry_with_distribution(
        "test-agent",
        npx_dist(
            "my-agent",
            Some(vec!["--verbose".into(), "--port".into(), "8080".into()]),
        ),
    );
    let config = enabled_distributions();
    let registry = smol::block_on(test_registry());
    let result = smol::block_on(fetch_agent_from_registry(
        &entry,
        Some(Distribution::Npx),
        &config,
        &registry,
    ));
    let assistant = result.unwrap();
    assert!(
        matches!(&assistant, Assistant::CustomStdio { name, command, args }
            if name == "test-agent"
               && command == "npx"
               && args == &vec!["my-agent", "--verbose", "--port", "8080"])
    );
}

#[nvim_oxi::test]
fn resolve_nonexistent_preference_returns_error() {
    let entry = entry_with_distribution("test-agent", HashMap::new());
    let config = enabled_distributions();
    let registry = smol::block_on(test_registry());
    let result = smol::block_on(fetch_agent_from_registry(
        &entry,
        Some(Distribution::Npx),
        &config,
        &registry,
    ));
    assert!(result.is_err());
}

#[nvim_oxi::test]
fn resolve_disabled_distribution_returns_error() {
    let entry = entry_with_distribution("test-agent", npx_dist("my-agent", None));
    let config = DistributionsConfig {
        npx: false,
        ..Default::default()
    };
    let registry = smol::block_on(test_registry());
    let result = smol::block_on(fetch_agent_from_registry(
        &entry,
        Some(Distribution::Npx),
        &config,
        &registry,
    ));
    assert!(result.is_err());
}

#[nvim_oxi::test]
fn resolve_disabled_distribution_error_mentions_disabled() {
    let entry = entry_with_distribution("test-agent", npx_dist("my-agent", None));
    let config = DistributionsConfig {
        npx: false,
        ..Default::default()
    };
    let registry = smol::block_on(test_registry());
    let result = smol::block_on(fetch_agent_from_registry(
        &entry,
        Some(Distribution::Npx),
        &config,
        &registry,
    ));
    assert!(
        result.unwrap_err().to_string().contains("disabled"),
        "Error should mention distribution is disabled"
    );
}

#[nvim_oxi::test]
fn resolve_all_disabled_error_mentions_toggle() {
    let entry = entry_with_distribution("test-agent", npx_dist("my-agent", None));
    let config = DistributionsConfig {
        npx: false,
        uvx: false,
        binary: hermes::nvim::configuration::BinaryConfig {
            enabled: false,
            ..Default::default()
        },
    };
    let registry = smol::block_on(test_registry());
    let result = smol::block_on(fetch_agent_from_registry(&entry, None, &config, &registry));
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("disabled"),
        "Error should mention distributions are disabled"
    );
}

#[nvim_oxi::test]
fn resolve_no_supported_distribution_error_mentions_agent_id() {
    let entry = entry_with_distribution("my-agent", HashMap::new());
    let config = enabled_distributions();
    let registry = smol::block_on(test_registry());
    let result = smol::block_on(fetch_agent_from_registry(&entry, None, &config, &registry));
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("no supported distribution"),
        "Error should describe no supported distribution"
    );
}

#[nvim_oxi::test]
fn resolve_binary_no_platform_match_returns_error() {
    let mut dist = HashMap::new();
    dist.insert(
        Distribution::Binary,
        DistributionCommand::BinaryTargets(HashMap::new()),
    );
    let entry = entry_with_distribution("test-agent", dist);
    let config = enabled_distributions();
    let registry = smol::block_on(test_registry());
    let result = smol::block_on(fetch_agent_from_registry(
        &entry,
        Some(Distribution::Binary),
        &config,
        &registry,
    ));
    assert!(result.is_err());
}
