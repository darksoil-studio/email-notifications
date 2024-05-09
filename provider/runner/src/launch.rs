use std::{collections::HashMap, path::PathBuf, sync::Arc};

use anyhow::anyhow;
use holochain::{
    conductor::{
        config::{AdminInterfaceConfig, ConductorConfig},
        interface::InterfaceDriver,
        Conductor, ConductorHandle,
    },
    prelude::{
        dependencies::kitsune_p2p_types::{
            config::{
                tuning_params_struct::KitsuneP2pTuningParams, KitsuneP2pConfig, TransportConfig,
            },
            dependencies::lair_keystore_api::dependencies::sodoken::{BufRead, BufWrite},
        },
        AppBundle, AppBundleSource, MembraneProof, NetworkSeed, RoleName,
    },
};
use holochain_client::{
    AdminWebsocket, AppInfo, InstallAppPayload, InstalledAppId, IssueAppAuthenticationTokenPayload,
};
use holochain_types::websocket::AllowedOrigins;
use url2::Url2;

pub fn vec_to_locked(mut pass_tmp: Vec<u8>) -> std::io::Result<BufRead> {
    match BufWrite::new_mem_locked(pass_tmp.len()) {
        Err(e) => {
            pass_tmp.fill(0);
            Err(e.into())
        }
        Ok(p) => {
            {
                let mut lock = p.write_lock();
                lock.copy_from_slice(&pass_tmp);
                pass_tmp.fill(0);
            }
            Ok(p.to_read())
        }
    }
}

#[derive(Clone)]
pub struct AppWebsocketAuth {
    pub app_websocket_port: u16,
    pub token: Vec<u8>,
}

pub async fn authorize_app(
    admin_ws: &mut AdminWebsocket,
    app_id: InstalledAppId,
) -> anyhow::Result<AppWebsocketAuth> {
    let port = admin_ws
        .attach_app_interface(0, AllowedOrigins::Any, Some(app_id.clone()))
        .await
        .map_err(|err| anyhow!("{err:?}"))?;

    let token = admin_ws
        .issue_app_auth_token(IssueAppAuthenticationTokenPayload {
            installed_app_id: app_id,
            expiry_seconds: 999999999,
            single_use: false,
        })
        .await
        .map_err(|err| anyhow!("{err:?}"))?;

    Ok(AppWebsocketAuth {
        app_websocket_port: port,
        token: token.token,
    })
}

pub async fn launch_holochain(
    network_seed: String,
    always_online: bool,
    bootstrap_url: Url2,
    signal_url: Url2,
) -> anyhow::Result<ConductorHandle> {
    let data_dir = dirs::data_local_dir().ok_or(anyhow::anyhow!("Could not get data local dir"))?;

    let conductor_dir = data_dir
        .join("email_notifications_provider")
        .join(network_seed);

    let admin_port = portpicker::pick_unused_port().expect("No ports free");

    let config = conductor_config(
        &conductor_dir,
        admin_port,
        always_online,
        bootstrap_url,
        signal_url,
    );

    let passphrase = vec_to_locked(vec![])?;

    println!("Launching holochain with config: {config:?}");

    let conductor = Conductor::builder()
        .config(config)
        .passphrase(Some(passphrase))
        .build()
        .await?;

    Ok(conductor)
}

pub async fn install_app(
    admin_ws: &mut AdminWebsocket,
    app_id: String,
    bundle: AppBundle,
    membrane_proofs: HashMap<RoleName, MembraneProof>,
    network_seed: Option<NetworkSeed>,
) -> anyhow::Result<AppInfo> {
    println!("Installing app {}", app_id);

    let agent_key = admin_ws
        .generate_agent_pub_key()
        .await
        .map_err(|err| anyhow::anyhow!("Could not generate pub key: {err:?}"))?;

    let app_info = admin_ws
        .install_app(InstallAppPayload {
            agent_key,
            membrane_proofs,
            network_seed,
            source: AppBundleSource::Bundle(bundle),
            installed_app_id: Some(app_id.clone()),
        })
        .await
        .map_err(|err| anyhow::anyhow!("Could not install app: {err:?}"))?;
    println!("Installed app {app_info:?}");

    let response = admin_ws
        .enable_app(app_id.clone())
        .await
        .map_err(|err| anyhow::anyhow!("Could not enable app: {err:?}"))?;

    println!("Enabled app {app_id:?}");

    Ok(response.app)
}

pub fn conductor_config(
    conductor_dir: &PathBuf,
    admin_port: u16,
    always_online: bool,
    bootstrap_url: Url2,
    signal_url: Url2,
) -> ConductorConfig {
    let mut config = ConductorConfig::default();
    config.data_root_path = Some(conductor_dir.clone().into());

    let mut network_config = KitsuneP2pConfig::default();

    network_config.bootstrap_service = Some(bootstrap_url);

    let mut tuning_params = KitsuneP2pTuningParams::default();

    if !always_online {
        tuning_params.gossip_arc_clamping = "empty".into();
    }

    network_config.tuning_params = Arc::new(tuning_params);

    network_config.transport_pool.push(TransportConfig::WebRTC {
        signal_url: signal_url.into_string(),
    });

    config.network = network_config;

    config.admin_interfaces = Some(vec![AdminInterfaceConfig {
        driver: InterfaceDriver::Websocket {
            port: admin_port,
            allowed_origins: AllowedOrigins::Any,
        },
    }]);

    config
}
