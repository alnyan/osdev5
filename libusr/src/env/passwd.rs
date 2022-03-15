use crate::file::File;
use crate::io::{self, read_line};
use core::str::FromStr;
use libsys::{
    error::Errno,
    stat::{GroupId, UserId},
    FixedStr,
};

#[derive(Debug, Clone, Copy)]
pub struct UserInfo {
    name: FixedStr<32>,
    uid: UserId,
    gid: GroupId,
    home: FixedStr<64>,
    shell: FixedStr<64>,
}

impl UserInfo {
    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub fn home(&self) -> &str {
        self.home.as_str()
    }

    pub fn shell(&self) -> &str {
        self.shell.as_str()
    }

    pub fn uid(&self) -> UserId {
        self.uid
    }

    pub fn gid(&self) -> GroupId {
        self.gid
    }

    pub fn find<F: Fn(&Self) -> bool>(pred: F) -> Result<Self, io::Error> {
        let mut file = File::open("/etc/passwd")?;
        let mut buf = [0; 128];
        loop {
            let line = read_line(&mut file, &mut buf)?;
            if let Some(line) = line {
                let ent = UserInfo::from_str(line)?;
                if pred(&ent) {
                    return Ok(ent);
                }
            } else {
                break;
            }
        }
        Err(io::Error::from(Errno::InvalidArgument))
    }

    pub fn by_name(name: &str) -> Result<Self, io::Error> {
        Self::find(|ent| ent.name() == name)
    }
}

impl FromStr for UserInfo {
    type Err = io::Error;

    fn from_str(s: &str) -> Result<Self, io::Error> {
        let mut iter = s.split(':');

        let name = iter
            .next()
            .ok_or_else(|| io::Error::from(Errno::InvalidArgument))?;
        let uid = iter
            .next()
            .ok_or_else(|| io::Error::from(Errno::InvalidArgument))
            .and_then(|e| u32::from_str(e).map_err(|_| io::Error::from(Errno::InvalidArgument)))
            .map(UserId::from)?;
        let gid = iter
            .next()
            .ok_or_else(|| io::Error::from(Errno::InvalidArgument))
            .and_then(|e| u32::from_str(e).map_err(|_| io::Error::from(Errno::InvalidArgument)))
            .map(GroupId::from)?;
        let _comment = iter
            .next()
            .ok_or_else(|| io::Error::from(Errno::InvalidArgument))?;
        let home = iter
            .next()
            .ok_or_else(|| io::Error::from(Errno::InvalidArgument))?;
        let shell = iter
            .next()
            .ok_or_else(|| io::Error::from(Errno::InvalidArgument))?;

        if iter.next().is_some() {
            return Err(io::Error::from(Errno::InvalidArgument));
        }

        let mut res = Self {
            uid,
            gid,
            name: FixedStr::empty(),
            home: FixedStr::empty(),
            shell: FixedStr::empty(),
        };

        res.name.copy_from_str(name);
        res.home.copy_from_str(home);
        res.shell.copy_from_str(shell);

        Ok(res)
    }
}
