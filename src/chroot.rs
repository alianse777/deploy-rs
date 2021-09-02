use anyhow::Result;

use crate::ssh::Ssh;

pub fn prepare_chroot(ssh: &Ssh) -> Result<()> {
    log::info!("Bootstrapping fakechroot");
    ssh.execute("debootstrap --variant=buildd hirsute /opt/chroot")?;
    Ok(())
}
