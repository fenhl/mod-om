use {
    std::{
        env,
        io::{
            Cursor,
            prelude::*,
        },
        num::NonZero,
        path::{
            Path,
            PathBuf,
        },
        pin::pin,
        time::Duration,
    },
    async_tempfile::TempDir,
    futures::stream::TryStreamExt as _,
    itertools::Itertools as _,
    reqwest::header::{
        HeaderMap,
        HeaderValue,
    },
    serde::Deserialize,
    tokio::{
        io::AsyncWriteExt as _,
        process::Command,
    },
    wheel::{
        fs::{
            self,
            File,
        },
        github::Repo,
        traits::{
            AsyncCommandOutputExt as _,
            IoResultExt as _,
            ReqwestResponseExt as _,
        },
    },
    zip::read::ZipArchive,
};
#[cfg(unix)] use std::os::unix::ffi::OsStrExt as _;

const OM_STEAM_ID: u32 = 558990;

async fn copy_dir(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> wheel::Result {
    let src = src.as_ref();
    let dst = dst.as_ref();
    fs::create_dir(dst).await?;
    let mut entries = pin!(fs::read_dir(src));
    while let Some(entry) = entries.try_next().await? {
        let path = entry.path();
        if entry.file_type().await.at(&path)?.is_dir() {
            Box::pin(copy_dir(path, dst.join(entry.file_name()))).await?;
        } else {
            fs::copy(path, dst.join(entry.file_name())).await?;
        }
    }
    Ok(())
}

#[derive(clap::Parser)]
#[clap(version)]
enum Subcommand {
    /// Create a modded instance of Opus Magnum.
    Init {
        out_path: PathBuf,
    },
}

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error(transparent)] Git(#[from] gix::Error),
    #[cfg(unix)] #[error(transparent)] InvalidHeaderValue(#[from] reqwest::header::InvalidHeaderValue),
    #[error(transparent)] Reqwest(#[from] reqwest::Error),
    #[error(transparent)] Tempfile(#[from] async_tempfile::Error),
    #[error(transparent)] Wheel(#[from] wheel::Error),
    #[error(transparent)] Zip(#[from] zip::result::ZipError),
    #[error("failed to find Mutatum Setup Kit")]
    ExactlyOneReleaseAsset,
    #[error("no OpusMutatum releases found")]
    MissingMutatumRelease,
    #[error("no Quintessential releases found")]
    MissingQuintRelease,
}

#[wheel::main]
async fn main(subcommand: Subcommand) -> Result<bool, Error> {
    let _ = rustls::crypto::ring::default_provider().install_default();
    match subcommand {
        Subcommand::Init { out_path } => {
            #[derive(Deserialize)]
            struct Game {
                active_branch: String,
                app_id: u32,
                install_path: Option<PathBuf>,
            }

            //TODO eliminate `aurelia` subprocess call (copy required functionality from aurelia's source?)
            let Some(om) = serde_json::from_slice::<Vec<Game>>(&Command::new("aurelia").arg("list").arg("--json").check("aurelia list --json").await?.stdout).at_command("aurelia list --json")?
                .into_iter()
                .find(|game| game.app_id == OM_STEAM_ID)
            else {
                eprintln!("mod-om: could not find Opus Magnum in your Steam library");
                eprintln!("You can buy the game at https://store.steampowered.com/app/558990/Opus_Magnum/");
                return Ok(false)
            };
            if om.active_branch != "community-edition" {
                eprintln!("mod-om: this command requires the Community Edition of Opus Magnum");
                //TODO interactively switch to community-edition?
                eprintln!("1. In your Steam library, select Opus Magnum, click on Manage (gear icon)");
                eprintln!("2. Select “Properties”");
                eprintln!("3. Select “Game Versions & Betas”, then select “community-edition”");
                return Ok(false)
            }
            let Some(install_path) = om.install_path else {
                eprintln!("mod-om: Opus Magnum is not installed");
                eprintln!("In your Steam library, select Opus Magnum, click Install");
                return Ok(false)
            };
            copy_dir(install_path, &out_path).await?;
            {
                let opus_mutatum = TempDir::new().await?;
                gix::prepare_clone("https://github.com/QuintessentialOM/OpusMutatum.git", &opus_mutatum)?
                    .with_shallow(gix::remote::fetch::Shallow::DepthAtRemote(NonZero::<u32>::MIN))
                    .fetch_then_checkout(gix::progress::Discard /*TODO show progress on command line? */, &gix::interrupt::IS_INTERRUPTED)?.0
                    .main_worktree(gix::progress::Discard /*TODO show progress on command line? */, &gix::interrupt::IS_INTERRUPTED)?;
                Command::new("dotnet").arg("publish").arg("-c:Debug").current_dir(&opus_mutatum).spawn().at_command("dotnet")?.check("dotnet").await?;
                let mut entries = pin!(fs::read_dir(opus_mutatum.join("bin").join("Debug").join("net10.0").join("publish")));
                while let Some(entry) = entries.try_next().await? {
                    fs::copy(entry.path(), out_path.join(entry.file_name())).await?;
                }
            }
            {
                let om_mappings = TempDir::new().await?;
                gix::prepare_clone("https://github.com/QuintessentialOM/om_mappings.git", &om_mappings)?
                    .with_shallow(gix::remote::fetch::Shallow::DepthAtRemote(NonZero::<u32>::MIN))
                    .fetch_then_checkout(gix::progress::Discard /*TODO show progress on command line? */, &gix::interrupt::IS_INTERRUPTED)?.0
                    .main_worktree(gix::progress::Discard /*TODO show progress on command line? */, &gix::interrupt::IS_INTERRUPTED)?;
                fs::create_dir_all(out_path.join("modded").join("Mappings")).await?;
                copy_dir(om_mappings.join("intermediary"), out_path.join("modded").join("Mappings").join("intermediary")).await?;
                copy_dir(om_mappings.join("named"), out_path.join("modded").join("Mappings").join("named")).await?;
            }
            #[cfg_attr(not(unix), allow(unused_mut))] let mut headers = HeaderMap::default();
            #[cfg(unix)] {
                if let Some(github_token) = env::var_os("GITHUB_TOKEN") {
                    headers.insert(reqwest::header::AUTHORIZATION, HeaderValue::from_bytes(github_token.as_bytes())?);
                }
            }
            let github_http_client = reqwest::Client::builder()
                .user_agent(concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"), " (https://github.com/fenhl/mod-om)"))
                .timeout(Duration::from_secs(30))
                .default_headers(headers)
                .use_rustls_tls()
                .hickory_dns(!cfg!(windows)) // Windows Firewall breaks hickory-dns
                .https_only(true)
                .http2_prior_knowledge()
                .build()?;
            let mutatum_setup_kit = Repo::new("QuintessentialOM", "OpusMutatum").latest_release(&github_http_client).await?
                .ok_or(Error::MissingMutatumRelease)?
                .assets
                .into_iter()
                .filter(|asset| asset.name.starts_with("MutatumSetupKit"))
                .exactly_one().map_err(|_| Error::ExactlyOneReleaseAsset)?
                .browser_download_url;
            let mutatum_setup_kit = github_http_client.get(mutatum_setup_kit)
                .send().await?
                .detailed_error_for_status().await?
                .bytes().await?; // async-zip misbehaves if the source is not buffered
            let mut mutatum_setup_kit = ZipArchive::new(Cursor::new(mutatum_setup_kit))?;
            let mut buf = [0; 4096];
            for idx in 0..mutatum_setup_kit.len() {
                let mut entry = mutatum_setup_kit.by_index(idx)?;
                if let Some(path) = entry.enclosed_name() && let Ok(path) = path.strip_prefix("modded") && path.extension().is_some_and(|ext| ext == "dll") {
                    let path = out_path.join("modded").join(path);
                    let mut file = File::create(&path).await?;
                    loop {
                        let len = entry.read(&mut buf).at_unknown()?;
                        if len == 0 { break }
                        file.write_all(&buf[..len]).await.at(&path)?;
                    }
                }
            }
            let mods_path = out_path.join("modded").join("Mods");
            fs::create_dir_all(&mods_path).await?;
            for asset in Repo::new("QuintessentialOM", "Quintessential").latest_release(&github_http_client).await?
                .ok_or(Error::MissingQuintRelease)?
                .assets
            {
                github_http_client.get(asset.browser_download_url)
                    .send().await?
                    .detailed_error_for_status().await?
                    .download(mods_path.join(asset.name)).await?;
            }
            let mut cmd = cfg_select! {
                target_os = "linux" => {{
                    let mut cmd = Command::new("steam-run");
                    cmd.arg("./OpusMutatum");
                    cmd
                }}
                _ => Command::new("./OpusMutatum"),
            };
            cmd.current_dir(out_path).spawn().at_command("OpusMutatum")?.check("OpusMutatum").await?;
        }
    }
    Ok(true)
}
