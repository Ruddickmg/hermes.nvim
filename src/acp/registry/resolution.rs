use crate::{
    acp::{
        Result,
        connection::Assistant,
        error::Error,
        registry::{DistributionCommand, PackageDistribution, binary, distribution::Distribution},
        utilities::command::command_available,
    },
    nvim::configuration::DistributionsConfig,
};

fn make_package_assistant(agent_id: &str, command: &str, pkg: &PackageDistribution) -> Assistant {
    let mut args = vec![pkg.package.clone()];
    if let Some(dist_args) = &pkg.args {
        args.extend(dist_args.clone());
    }
    Assistant::CustomStdio {
        name: agent_id.to_string(),
        command: command.to_string(),
        args,
    }
}

pub async fn fetch_agent_from_registry(
    entry: &crate::acp::registry::AgentEntry,
    preference: Option<Distribution>,
    distributions_config: &DistributionsConfig,
    registry: &crate::acp::registry::Registry,
) -> Result<Assistant> {
    let agent_id = &entry.id;

    if let Some(pref) = preference
        && !pref.is_enabled(distributions_config)
    {
        return Err(Error::InvalidInput(format!(
            "Cannot install distribution: '{}' is disabled",
            pref
        )));
    }

    let mut considered_distributions: Vec<Distribution> = preference
        .map(|p| vec![p])
        .unwrap_or_else(|| entry.distributions());
    considered_distributions.retain(|distribution| entry.has_distribution(distribution));

    // Ensure deterministic auto-selection order when no explicit preference is provided.
    if preference.is_none() {
        considered_distributions.sort_by_key(|d| match d {
            Distribution::Npx => 0,
            Distribution::Uvx => 1,
            Distribution::Binary => 2,
            Distribution::Invalid => 3,
        });
    }

    let candidates: Vec<Distribution> = considered_distributions
        .iter()
        .copied()
        .filter(|distribution| distribution.is_enabled(distributions_config))
        .collect();

    for dist_type in &candidates {
        match entry.get_distribution(dist_type) {
            Some(DistributionCommand::Package(pkg)) => {
                if command_available(&dist_type.to_string()).await {
                    return Ok(make_package_assistant(
                        agent_id,
                        &dist_type.to_string(),
                        pkg,
                    ));
                }
            }
            Some(DistributionCommand::BinaryTargets(targets)) => {
                if let Some(target) = binary::platform_target(targets) {
                    let cache_dir_override = if distributions_config.binary.path.is_empty() {
                        None
                    } else {
                        Some(distributions_config.binary.path.as_str())
                    };
                    let path = registry
                        .download_binary(
                            agent_id,
                            &entry.version,
                            target,
                            cache_dir_override.map(std::path::Path::new),
                        )
                        .await?;
                    return Ok(Assistant::CustomStdio {
                        name: agent_id.to_string(),
                        command: path.to_string_lossy().to_string(),
                        args: target.args.clone().unwrap_or_default(),
                    });
                }
            }
            _ => {}
        }
    }

    if candidates.is_empty() && !considered_distributions.is_empty() {
        Err(Error::InvalidInput(format!(
            "Agent '{}' has all distribution methods disabled. Set at least one of \
             'distributions.uvx', 'distributions.npx', or 'distributions.binary.enabled' to true.",
            entry.id
        )))
    } else {
        Err(Error::InvalidInput(format!(
            "Agent '{}' has no supported distribution for this platform",
            entry.id
        )))
    }
}
