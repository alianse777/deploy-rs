#![feature(seek_stream_len)]
mod build;
mod chroot;
mod crateinfo;
mod service;
mod ssh;
use service::{
    make_backup_executable, preinstall, push_executable, push_resources, start_systemd_service,
    stop_systemd_service,
};
use std::path::PathBuf;
use structopt::StructOpt;

use crate::{
    chroot::prepare_chroot, crateinfo::CrateInfo, service::execute_post_command, ssh::Ssh,
};

/// Deploy rust binary to server
#[derive(StructOpt, Debug)]
#[structopt(name = "Deploy-rs")]
pub struct Opt {
    /// Features to build with (all features are disabled by default)
    #[structopt(long)]
    features: Option<String>,
    /// NPM package to build (frontend part)
    #[structopt(long, parse(from_os_str), default_value = "./frontend")]
    npm_package: PathBuf,
    /// Command to run after upload
    #[structopt(long)]
    post_command: Option<String>,
    /// Resource files
    #[structopt(long)]
    resource_files: Option<String>,
    /// Perform initial setup on server
    #[structopt(long)]
    preinstall: bool,
    /// Build musl linked binary
    #[structopt(long)]
    musl: bool,
    /// Install in fakechroot
    #[structopt(long)]
    fakechroot: bool,
    /// Do not build projects
    #[structopt(long)]
    no_build: bool,
    /// SSH login
    #[structopt(short, long, default_value = "root")]
    login: String,
    /// SSH private key path (assumed same public key name with .pub)
    #[structopt(short, long, parse(from_os_str))]
    keyfile: Option<PathBuf>,
    /// Server address
    #[structopt(name = "SERVER")]
    address: String,
}

fn main() {
    simple_logger::SimpleLogger::default()
        .with_level(log::LevelFilter::Debug)
        .init()
        .unwrap();
    let opt = Opt::from_args();
    let crateinfo = CrateInfo::load(".").expect("Failed to load package info");
    let keys = opt.keyfile.as_ref().map(|x| (x.with_extension("pub"), x));
    let ssh = Ssh::connect(&opt.address, &opt.login, keys).expect("SSH connection failed");
    log::info!("Connection successful");

    if !opt.no_build {
        build::build_frontend_if_exists("./frontend").expect("Failed to build frontend");
        build::build_backend(
            opt.features
                .as_ref()
                .map(|x| x.split(",").map(|s| s.to_owned()).collect()),
            opt.musl,
        )
        .expect("Failed to build backend");
    }

    if opt.preinstall {
        preinstall(&crateinfo, &ssh, &opt).expect("Failed to preinstall service");
    }

    if opt.fakechroot {
        prepare_chroot(&ssh).expect("Failed to prepare chroot");
    }

    let resources: Vec<PathBuf> = opt
        .resource_files
        .as_ref()
        .map(|r| r.split(",").map(|s| PathBuf::from(s.to_owned())).collect())
        .unwrap_or_default();
    push_resources(&resources, &crateinfo, &ssh, &opt).expect("Failed to upload resource");

    stop_systemd_service(&crateinfo, &ssh).expect("Failed to stop service");
    make_backup_executable(&crateinfo, &ssh, &opt).expect("Failed to backup executable");
    push_executable(&crateinfo, &ssh, &opt).expect("Failed to upload executable file");
    if let Some(post_command) = &opt.post_command {
        execute_post_command(&crateinfo, &post_command, &ssh, &opt)
            .expect("Failed to execute post command");
    }
    start_systemd_service(&crateinfo, &ssh).expect("Failed to start service");
}
