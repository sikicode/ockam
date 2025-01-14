use clap::Args;
use colorful::Colorful;
use miette::IntoDiagnostic;
use std::fmt::Write;

use ockam::Context;
use ockam_api::cloud::lease_manager::models::influxdb::Token;
use ockam_api::InfluxDbTokenLease;
use time::format_description::well_known::Iso8601;
use time::PrimitiveDateTime;
use tokio::sync::Mutex;
use tokio::try_join;

use crate::lease::create_project_client;
use crate::output::Output;
use crate::terminal::OckamColor;
use crate::util::api::{CloudOpts, TrustOpts};
use crate::util::async_cmd;
use crate::{docs, CommandGlobalOpts};

const HELP_DETAIL: &str = "";

/// List tokens within the lease token manager
#[derive(Clone, Debug, Args)]
#[command(help_template = docs::after_help(HELP_DETAIL))]
pub struct ListCommand;

impl ListCommand {
    pub fn run(
        self,
        opts: CommandGlobalOpts,
        cloud_opts: CloudOpts,
        trust_opts: TrustOpts,
    ) -> miette::Result<()> {
        async_cmd(&self.name(), opts.clone(), |ctx| async move {
            self.async_run(&ctx, opts, cloud_opts, trust_opts).await
        })
    }

    pub fn name(&self) -> String {
        "list tokens".into()
    }

    async fn async_run(
        &self,
        ctx: &Context,
        opts: CommandGlobalOpts,
        cloud_opts: CloudOpts,
        trust_opts: TrustOpts,
    ) -> miette::Result<()> {
        let is_finished: Mutex<bool> = Mutex::new(false);
        let project_node = create_project_client(ctx, &opts, &cloud_opts, &trust_opts).await?;

        let send_req = async {
            let tokens: Vec<Token> = project_node.list_tokens(ctx).await?;
            *is_finished.lock().await = true;
            Ok(tokens)
        };

        let output_messages = vec![format!("Listing Tokens...\n")];

        let progress_output = opts
            .terminal
            .progress_output(&output_messages, &is_finished);

        let (tokens, _) = try_join!(send_req, progress_output)?;

        let plain = opts.terminal.build_list(
            &tokens,
            "Tokens",
            "No active tokens found within service.",
        )?;
        let json = serde_json::to_string_pretty(&tokens).into_diagnostic()?;
        opts.terminal
            .stdout()
            .plain(plain)
            .json(json)
            .write_line()?;
        Ok(())
    }
}

impl Output for Token {
    fn output(&self) -> crate::error::Result<String> {
        let mut output = String::new();
        let status = match self.status.as_str() {
            "active" => self
                .status
                .to_uppercase()
                .color(OckamColor::Success.color()),
            _ => self
                .status
                .to_uppercase()
                .color(OckamColor::Failure.color()),
        };
        let expires_at = {
            PrimitiveDateTime::parse(&self.expires, &Iso8601::DEFAULT)?
                .to_string()
                .color(OckamColor::PrimaryResource.color())
        };
        let id = self
            .id
            .to_string()
            .color(OckamColor::PrimaryResource.color());

        writeln!(output, "Token {id}")?;
        writeln!(output, "Expires {expires_at} {status}")?;
        write!(output, "{}", self.token)?;

        Ok(output)
    }
}
