use std::{collections::HashMap, path::PathBuf, sync::Arc, time::Duration};

use clap::{Parser, Subcommand};
use email_notifications_types::{EmailCredentials, SendEmailSignal};
use holochain::prelude::{AppBundle, ExternIO, Signal};
use holochain_client::*;
use lettre::{transport::smtp::authentication::Credentials, Message, SmtpTransport, Transport};

mod launch;

use launch::{authorize_app, install_app, launch_holochain};
use url2::Url2;

fn parse_url(arg: &str) -> anyhow::Result<Url2> {
    Ok(Url2::try_parse(arg)?)
}

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to the .happ bundle for the email notifications provider app
    email_notifications_provider_happ: PathBuf,

    /// Network seed for the app to be installed
    #[arg(long)]
    network_seed: String,

    #[arg(long, default_value = "https://bootstrap.holo.host", value_parser = parse_url)]
    bootstrap_url: Url2,

    #[arg(long, default_value = "wss://signal.holo.host", value_parser = parse_url)]
    signal_url: Url2,

    #[command(subcommand)]
    command: Option<Commands>,
}

/// Simple program to greet a person
#[derive(Subcommand, Debug)]
enum Commands {
    RegisterCredentials {
        /// Username for the sender email account
        #[arg(long)]
        sender_email_address: String,

        /// Password for the sender email account
        #[arg(short, long)]
        password: String,

        /// Url of the SMTP relay server
        #[arg(short, long)]
        smtp_relay_url: String,
    },
}

#[tokio::main]
async fn main() {
    if let Err(err) = run().await {
        println!("Error running holochain: {err:?}");
    }
}

const PROVIDER_APP_ID: &'static str = "email_notifications_provider";

fn provider_app_bundle(provider_app_bundle_path: PathBuf) -> anyhow::Result<AppBundle> {
    let bytes = std::fs::read(provider_app_bundle_path)?;
    Ok(AppBundle::decode(bytes.as_slice())?)
}

async fn run() -> anyhow::Result<()> {
    let args = Args::parse();

    let always_online = args.command.is_none();

    let conductor_handle = launch_holochain(
        args.network_seed.clone(),
        always_online,
        args.bootstrap_url,
        args.signal_url,
    )
    .await?;

    let admin_port = conductor_handle
        .get_arbitrary_admin_websocket_port()
        .expect("Could not get admin port");

    let mut admin_ws = AdminWebsocket::connect(format!("ws://localhost:{}", admin_port)).await?;

    let apps = admin_ws
        .list_apps(None)
        .await
        .map_err(|err| anyhow::anyhow!("Could not connect to admin ws: {err:?}"))?;

    let app_auth = authorize_app(&mut admin_ws, PROVIDER_APP_ID.into()).await?;

    if apps.len() == 0 {
        install_app(
            &mut admin_ws,
            String::from(PROVIDER_APP_ID),
            provider_app_bundle(args.email_notifications_provider_happ)?,
            HashMap::new(),
            Some(args.network_seed),
        )
        .await?;
    }

    let mut app_ws = AppWebsocket::connect(
        format!("ws://localhost:{}", app_auth.app_websocket_port),
        app_auth.token,
        Arc::new(LairAgentSigner::new(Arc::new(
            conductor_handle.keystore().lair_client(),
        ))),
    )
    .await?;

    if let Some(Commands::RegisterCredentials {
        sender_email_address,
        password,
        smtp_relay_url,
    }) = args.command
    {
        let email_credentials = EmailCredentials {
            sender_email_address,
            password,
            smtp_relay_url,
        };

        app_ws
            .call_zome(
                ZomeCallTarget::RoleName("email_notifications_provider".into()),
                "email_notifications_provider".into(),
                "publish_new_email_credentials".into(),
                ExternIO::encode(email_credentials.clone())?,
            )
            .await
            .map_err(|err| anyhow::anyhow!("Failed to publish email credentials: {err:?}"))?;

        std::thread::sleep(Duration::from_secs(3));

        let result = app_ws
            .call_zome(
                ZomeCallTarget::RoleName("email_notifications_provider".into()),
                "email_notifications_provider".into(),
                "get_current_email_credentials".into(),
                ExternIO::encode(email_credentials.clone())?,
            )
            .await
            .map_err(|err| anyhow::anyhow!("Failed to get email credentials: {err:?}"))?;

        let maybe_published_email_credentials: Option<EmailCredentials> = result.decode()?;

        let published_email_credentials =
            maybe_published_email_credentials.expect("There are no published email credentials");
        if !published_email_credentials.eq(&email_credentials) {
            panic!("The published email credentials were not successfully gossiped: try again.");
        }

        println!("Successfully registered new email credentials");

        std::process::exit(0);
    }

    // Listen for signal
    app_ws
        .on_signal(|signal| {
            let Signal::App { signal, .. } = signal else {
                return ();
            };

            let Ok(send_email_signal) = signal.into_inner().decode::<SendEmailSignal>() else {
                return ();
            };

            tokio::spawn(async move {
                if let Err(err) = send_email(send_email_signal).await {
                    println!("Error sending email: {err:#?}");
                }
            });
        })
        .await?;

    tokio::signal::ctrl_c()
        .await
        .expect("failed to listen for event");

    Ok(())
}

async fn send_email(send_email_signal: SendEmailSignal) -> anyhow::Result<()> {
    let email = Message::builder()
        .from(
            format!(
                "Sender <{}>",
                send_email_signal.credentials.sender_email_address.clone()
            )
            .parse()
            .unwrap(),
        )
        .to(format!("Receiver <{}>", send_email_signal.email_address)
            .parse()
            .unwrap())
        .subject(send_email_signal.email.subject)
        .body(send_email_signal.email.body)
        .unwrap();

    let creds = Credentials::new(
        send_email_signal.credentials.sender_email_address.clone(),
        send_email_signal.credentials.password.clone(),
    );

    // Open a remote connection to the given smtp relay
    let mailer = SmtpTransport::relay(send_email_signal.credentials.smtp_relay_url.as_str())
        .unwrap()
        .credentials(creds)
        .build();

    // Send the email
    match mailer.send(&email) {
        Ok(_) => Ok(()),
        Err(e) => Err(anyhow::anyhow!("Could not send email: {:?}", e)),
    }
}
