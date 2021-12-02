use crate::io::{Read, read_line};
use core::str::FromStr;
use core::fmt;
use crate::trace_debug;
use crate::file::File;
use libsys::{FixedStr, stat::{UserId, GroupId}};

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

    pub fn find<F: Fn(&Self) -> bool>(pred: F) -> Result<Self, ()> {
        let mut file = File::open("/etc/passwd").map_err(|_| ())?;
        let mut buf = [0; 128];
        loop {
            let line = read_line(&mut file, &mut buf).map_err(|_| ())?;
            if let Some(line) = line {
                let ent = UserInfo::from_str(line)?;
                if pred(&ent) {
                    return Ok(ent);
                }
            } else {
                break;
            }
        }
        Err(())
    }

    pub fn by_name(name: &str) -> Result<Self, ()> {
        Self::find(|ent| ent.name() == name)
    }
}

impl FromStr for UserInfo {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, ()> {
        let mut iter = s.split(":");

        let name = iter.next().ok_or(())?;
        let uid = iter
            .next()
            .ok_or(())
            .and_then(|e| u32::from_str(e).map_err(|_| ()))
            .map(UserId::from)?;
        let gid = iter
            .next()
            .ok_or(())
            .and_then(|e| u32::from_str(e).map_err(|_| ()))
            .map(GroupId::from)?;
        let comment = iter.next().ok_or(())?;
        let home = iter.next().ok_or(())?;
        let shell = iter.next().ok_or(())?;

        if iter.next().is_some() {
            return Err(());
        }

        let mut res = Self {
            uid,
            gid,
            name: FixedStr::empty(),
            home: FixedStr::empty(),
            shell: FixedStr::empty(),
        };

        res.name.copy_from_str(&name);
        res.home.copy_from_str(&home);
        res.shell.copy_from_str(&shell);

        Ok(res)
    }
}
