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
    async_tempfile::{
        TempDir,
        TempFile,
    },
    futures::stream::TryStreamExt as _,
    itertools::Itertools as _,
    reqwest::header::{
        HeaderMap,
        HeaderValue,
    },
    serde::Deserialize,
    tokio::{
        io::{
            self,
            AsyncWriteExt as _,
        },
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
    which::which,
    zip::read::ZipArchive,
};
#[cfg(unix)] use std::os::unix::{
    fs::PermissionsExt as _,
    ffi::OsStrExt as _,
};

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
    #[error("failed to find Aurelia")]
    ExactlyOneAureliaAsset,
    #[error("failed to find Mutatum Setup Kit")]
    ExactlyOneMutatumAsset,
    #[error("no Aurelia releases found")]
    MissingAureliaRelease,
    #[error("no OpusMutatum releases found")]
    MissingMutatumRelease,
    #[error("no Quintessential releases found")]
    MissingQuintRelease,
}

struct Exit(bool);

impl wheel::CustomExit for Exit {
    fn exit(self, cmd_name: &'static str) {
        wheel::MainOutput::exit(self.0, cmd_name, true)
    }
}

impl wheel::CustomExit for Error {
    fn exit(self, cmd_name: &'static str) {
        match self {
            Self::Wheel(wheel::Error::CommandExit { name, output }) => {
                eprintln!("{cmd_name}: command `{name}` exited with {}", output.status);
                eprintln!("This is a bug in {cmd_name}. Please report it at https://github.com/fenhl/mod-om and include the following debug info:");
                eprintln!("stdout:");
                eprintln!("{}", String::from_utf8_lossy(&output.stdout));
                eprintln!("stderr:");
                eprintln!("{}", String::from_utf8_lossy(&output.stderr));
            }
            _ => {
                eprintln!("{cmd_name}: {self}");
                eprintln!("This is a bug in {cmd_name}. Please report it at https://github.com/fenhl/mod-om and include the following debug info:");
                eprintln!("{self:?}");
            }
        }
        std::process::exit(1)
    }
}

#[wheel::main(custom_exit)]
async fn main(subcommand: Subcommand) -> Result<Exit, Error> {
    let _ = rustls::crypto::ring::default_provider().install_default();
    match subcommand {
        Subcommand::Init { out_path } => {
            #[derive(Deserialize)]
            struct Game {
                active_branch: String,
                app_id: u32,
                install_path: Option<PathBuf>,
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
            //TODO eliminate `aurelia` subprocess call (copy required functionality from aurelia's source?)
            let mut aurelia_tempfile = None;
            let aurelia = if let Ok(aurelia) = which("aurelia") {
                aurelia
            } else {
                let aurelia_url = Repo::new("Drackrath", "Aurelia").latest_release(&github_http_client).await?
                    .ok_or(Error::MissingAureliaRelease)?
                    .assets
                    .into_iter()
                    .filter(|asset| asset.name == cfg_select! {
                        all(target_os = "windows", target_arch = "aarch64") => "aurelia_windows_arm64.exe",
                        all(target_os = "windows", target_arch = "x86_64") => "aurelia_windows_x86_64.exe",
                        all(target_os = "macos", target_arch = "aarch64") => "aurelia_macOS_arm64",
                        all(target_os = "macos", target_arch = "x86_64") => "aurelia_macOS_x86_64",
                        all(target_os = "linux", target_arch = "aarch64") => "aurelia_linux_arm64",
                        all(target_os = "linux", target_arch = "x86_64") => "aurelia_linux_x86_64",
                    })
                    .exactly_one().map_err(|_| Error::ExactlyOneAureliaAsset)?
                    .browser_download_url;
                let mut aurelia_file = TempFile::builder().prefix("aurelia_").suffix(cfg_select! {
                    target_os = "windows" => ".exe",
                    _ => "",
                }).create().await?;
                github_http_client.get(aurelia_url)
                    .send().await?
                    .detailed_error_for_status().await?
                    .write_to(&mut aurelia_file).await.at(aurelia_file.file_path())?;
                #[cfg(unix)] { aurelia_file.set_permissions(fs::Permissions::from_mode(0x755)).await.at(aurelia_file.file_path())? }
                let path = aurelia_file.file_path().clone();
                aurelia_tempfile = Some(aurelia_file);
                path
            };
            let Some(om) = serde_json::from_slice::<Vec<Game>>(&Command::new(aurelia).arg("list").arg("--json").check("aurelia list --json").await?.stdout).at_command("aurelia list --json")?
                .into_iter()
                .find(|game| game.app_id == OM_STEAM_ID)
            else {
                eprintln!("mod-om: could not find Opus Magnum in your Steam library");
                eprintln!("You can buy the game at https://store.steampowered.com/app/558990/Opus_Magnum/");
                return Ok(Exit(false))
            };
            if let Some(aurelia_tempfile) = aurelia_tempfile {
                aurelia_tempfile.drop_async().await;
            }
            if om.active_branch != "community-edition" {
                eprintln!("mod-om: this command requires the Community Edition of Opus Magnum");
                //TODO interactively switch to community-edition?
                eprintln!("1. In your Steam library, select Opus Magnum, click on Manage (gear icon)");
                eprintln!("2. Select “Properties”");
                eprintln!("3. Select “Game Versions & Betas”, then select “community-edition”");
                return Ok(Exit(false))
            }
            let Some(install_path) = om.install_path else {
                eprintln!("mod-om: Opus Magnum is not installed");
                eprintln!("In your Steam library, select Opus Magnum, click Install");
                return Ok(Exit(false))
            };
            match copy_dir(install_path, &out_path).await {
                Ok(()) => {}
                Err(wheel::Error::Io { inner, context }) if inner.kind() == io::ErrorKind::AlreadyExists && let wheel::IoErrorContext::Path(error_path) = &context && *error_path == out_path => {
                    eprintln!("mod-om: the path `{}` already exists", out_path.display());
                    return Ok(Exit(false))
                }
                Err(e) => return Err(e.into()),
            }
            if which("dotnet").is_err() {
                cfg_select! {
                    target_os = "macos" => if let Ok(brew) = which("brew") {
                        Command::new(brew).arg("install").arg("dotnet").spawn().at_command("brew")?.check("brew").await?;
                    },
                    //TODO on Linux, check for common package managers and offer to install using the first detected one
                    _ => {}
                }
                if which("dotnet").is_err() {
                    eprintln!("mod-om: dotnet is not installed");
                    eprintln!("The recommended version is 10.0, you will need both the runtime and the SDK");
                    cfg_select! {
                        target_os = "macos" => eprintln!("See https://learn.microsoft.com/en-us/dotnet/core/install/macos for install instructions"),
                        target_os = "linux" => eprintln!("See https://learn.microsoft.com/en-us/dotnet/core/install/linux for install instructions"),
                        _ => {}
                    }
                    return Ok(Exit(false))
                }
            }
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
            let mutatum_setup_kit = Repo::new("QuintessentialOM", "OpusMutatum").latest_release(&github_http_client).await?
                .ok_or(Error::MissingMutatumRelease)?
                .assets
                .into_iter()
                .filter(|asset| asset.name.starts_with("MutatumSetupKit"))
                .exactly_one().map_err(|_| Error::ExactlyOneMutatumAsset)?
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
            if which("mono").is_err() {
                cfg_select! {
                    target_os = "macos" => if let Ok(brew) = which("brew") {
                        Command::new(brew).arg("install").arg("mono").spawn().at_command("brew")?.check("brew").await?;
                    },
                    //TODO on Linux, check for common package managers and offer to install using the first detected one
                    _ => {}
                }
                if which("mono").is_err() {
                    eprintln!("mod-om: mono is not installed");
                    cfg_select! {
                        target_os = "macos" => eprintln!("See https://www.mono-project.com/docs/getting-started/install/mac/ for install instructions"),
                        target_os = "linux" => eprintln!("See https://www.mono-project.com/docs/getting-started/install/linux/ for install instructions"),
                        _ => {}
                    }
                    return Ok(Exit(false))
                }
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
    Ok(Exit(true))
}
