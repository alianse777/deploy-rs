use std::path::{Path, PathBuf};

use crate::{crateinfo::CrateInfo, ssh::Ssh, Opt};
use anyhow::Result;

pub fn get_base_path(crateinfo: &CrateInfo, opt: &Opt) -> PathBuf {
    if opt.fakechroot {
        PathBuf::from(format!("/opt/chroot/apps/{}", crateinfo.crate_name()))
    } else {
        PathBuf::from(format!("/opt/{}", crateinfo.crate_name()))
    }
}

pub fn get_executable_path(crateinfo: &CrateInfo, opt: &Opt) -> PathBuf {
    get_base_path(crateinfo, opt).join(crateinfo.crate_name())
}

pub fn get_service_dirs(crateinfo: &CrateInfo, opt: &Opt) -> Vec<PathBuf> {
    vec![get_base_path(crateinfo, opt)]
}

pub fn make_service_file(crateinfo: &CrateInfo, env: &[(String, String)], opt: &Opt) -> String {
    let env: String = env.iter().map(|(k, v)| format!("{}={}", k, v)).collect();
    let exec_cmd = if opt.fakechroot {
        format!(
            "chroot /opt/chroot /apps/{name}/{name}",
            name = crateinfo.crate_name()
        )
    } else {
        get_executable_path(crateinfo, opt)
            .to_str()
            .unwrap()
            .to_owned()
    };
    format!(
        r#"[Unit]
Description={name}
After=syslog.target network.target

[Service]
ExecStart={exec_cmd}
WorkingDirectory={pwd}
Environment="{env}"

[Install]
WantedBy=multi-user.target"#,
        name = crateinfo.crate_name(),
        exec_cmd = exec_cmd,
        pwd = get_base_path(crateinfo, opt).to_str().unwrap(),
        env = env
    )
}

pub fn install_systemd_service(crateinfo: &CrateInfo, ssh: &Ssh, opt: &Opt) -> Result<()> {
    let systemd_file = make_service_file(crateinfo, &[], opt);
    let mut file = std::io::Cursor::new(systemd_file.as_bytes());
    ssh.send(
        &format!("/lib/systemd/system/{}.service", crateinfo.crate_name()),
        0o644,
        &mut file,
    )?;
    ssh.execute(&format!(
        "systemctl enable {}.service",
        crateinfo.crate_name()
    ))?;
    Ok(())
}

pub fn stop_systemd_service(crateinfo: &CrateInfo, ssh: &Ssh) -> Result<()> {
    ssh.execute(&format!(
        "systemctl stop {}.service",
        crateinfo.crate_name()
    ))?;
    Ok(())
}

pub fn start_systemd_service(crateinfo: &CrateInfo, ssh: &Ssh) -> Result<()> {
    ssh.execute(&format!(
        "systemctl start {}.service",
        crateinfo.crate_name()
    ))?;
    Ok(())
}

pub fn create_service_dirs(crateinfo: &CrateInfo, ssh: &Ssh, opt: &Opt) -> Result<()> {
    let dirs = get_service_dirs(crateinfo, opt);
    for d in dirs {
        ssh.execute(&format!("mkdir -p {:?}", d))?;
    }
    Ok(())
}

pub fn make_backup_executable(crateinfo: &CrateInfo, ssh: &Ssh, opt: &Opt) -> Result<()> {
    let exec_path = get_executable_path(crateinfo, opt);
    let bak_path = exec_path.with_extension("bak");
    ssh.execute(&format!(
        "mv {} {}",
        exec_path.to_str().unwrap(),
        bak_path.to_str().unwrap()
    ))?;
    Ok(())
}

pub fn push_executable(crateinfo: &CrateInfo, ssh: &Ssh, opt: &Opt) -> Result<()> {
    let remote = get_executable_path(crateinfo, opt);
    let local = if opt.musl {
        format!(
            "/opt/cargo/target/x86_64-unknown-linux-musl/release/{}",
            crateinfo.crate_name()
        )
    } else {
        format!(
            "/opt/cargo/target/x86_64-unknown-linux-gnu/release/{}",
            crateinfo.crate_name()
        )
    };
    ssh.send_file(local, remote, 0o755)?;
    Ok(())
}

pub fn push_resources<P: AsRef<Path>>(
    resources: &[P],
    crateinfo: &CrateInfo,
    ssh: &Ssh,
    opt: &Opt,
) -> Result<()> {
    let resources_path = get_base_path(crateinfo, opt);
    for resource in resources {
        ssh.send_files(resource, &resources_path)?;
    }
    Ok(())
}

pub fn preinstall(crateinfo: &CrateInfo, ssh: &Ssh, opt: &Opt) -> Result<()> {
    install_systemd_service(crateinfo, ssh, opt)?;
    create_service_dirs(crateinfo, ssh, opt)?;
    Ok(())
}

pub fn execute_post_command(crateinfo: &CrateInfo, cmd: &str, ssh: &Ssh, opt: &Opt) -> Result<()> {
    ssh.execute(&format!(
        "cd {pwd} && {cmd}",
        pwd = get_base_path(crateinfo, opt).to_str().unwrap(),
        cmd = cmd
    ))?;
    Ok(())
}
