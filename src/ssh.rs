use std::{
    collections::HashSet,
    fs::{self, read_dir},
    io::{copy, Cursor, Read, Seek},
    net::TcpStream,
    path::{Path, PathBuf},
};

use anyhow::Result;
use ssh2::Session;

fn normalize_path<P: AsRef<Path>>(path: P) -> PathBuf {
    path.as_ref()
        .components()
        .filter(|c| c.as_os_str() != ".")
        .collect()
}

pub struct Ssh {
    session: Session,
}

impl Ssh {
    pub fn connect<A: AsRef<Path>, B: AsRef<Path>>(
        address: &str,
        login: &str,
        keys: Option<(A, B)>,
    ) -> Result<Self> {
        let tcp = TcpStream::connect(address)?;
        let mut session = Session::new()?;
        session.set_tcp_stream(tcp);
        session.handshake()?;
        if let Some(keys) = keys {
            let passphrase = rpassword::read_password_from_tty(Some("Key password:"))?;
            session.userauth_pubkey_file(
                login,
                Some(keys.0.as_ref()),
                keys.1.as_ref(),
                Some(&passphrase[..]).filter(|x| x.is_empty()),
            )?;
        } else {
            let password = rpassword::read_password_from_tty(Some("Password:"))?;
            session.userauth_password(login, &password.trim())?;
        }
        if !session.authenticated() {
            return Err(anyhow::anyhow!("login failed for user {}", login));
        }
        Ok(Self { session })
    }

    pub fn execute(&self, cmd: &str) -> Result<String> {
        log::info!(">{}", cmd);
        let mut channel = self.session.channel_session()?;
        channel.exec(cmd)?;
        let mut s = String::new();
        channel.read_to_string(&mut s)?;
        channel.wait_close().unwrap();
        Ok(s)
    }

    pub fn send<R: AsRef<Path>, B: Read + Seek>(
        &self,
        remote: R,
        mode: i32,
        reader: &mut B,
    ) -> Result<()> {
        let size = reader.stream_len()?;
        let mut remote_file = self.session.scp_send(remote.as_ref(), mode, size, None)?;
        copy(reader, &mut remote_file)?;
        Ok(())
    }

    pub fn send_file<L: AsRef<Path>, R: AsRef<Path>>(
        &self,
        local: L,
        remote: R,
        mode: i32,
    ) -> Result<()> {
        log::debug!("Sending {:?} -> {:?}", local.as_ref(), remote.as_ref());
        let buf = fs::read(local)?;
        let mut cursor = Cursor::new(buf); // only works with cursor
        self.send(remote, mode, &mut cursor)?;
        Ok(())
    }

    pub fn send_files<L: AsRef<Path>, R: AsRef<Path>>(&self, local: L, remote: R) -> Result<()> {
        let mut paths = Vec::new();
        let mut processed_paths = HashSet::new();
        paths.push(local.as_ref().to_owned());
        while let Some(path) = paths.pop().map(|x| x.to_owned()) {
            if path.is_file() {
                self.send_file(&path, normalize_path(remote.as_ref().join(&path)), 0o755)?;
            } else if path.is_dir() {
                self.execute(&format!(
                    "mkdir -p {:?}",
                    normalize_path(remote.as_ref().join(&path))
                ))?;
                for dir_ent in read_dir(&path)? {
                    let dir_ent = dir_ent?;
                    if !processed_paths.contains(&dir_ent.path()) {
                        paths.push(dir_ent.path());
                    }
                }
            }
            processed_paths.insert(path);
        }
        Ok(())
    }
}
