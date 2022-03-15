use crate::file::File;
use crate::io::{self, read_line};
use core::str::FromStr;
use libsys::{error::Errno, FixedStr};

#[derive(Debug, Clone, Copy)]
pub struct UserShadow {
    name: FixedStr<32>,
    password: FixedStr<64>,
}

impl UserShadow {
    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub fn password(&self) -> &str {
        self.password.as_str()
    }

    pub fn find<F: Fn(&Self) -> bool>(pred: F) -> Result<Self, io::Error> {
        let mut file = File::open("/etc/shadow")?;
        let mut buf = [0; 128];
        loop {
            let line = read_line(&mut file, &mut buf)?;
            if let Some(line) = line {
                let ent = UserShadow::from_str(line)?;
                if pred(&ent) {
                    return Ok(ent);
                }
            } else {
                break;
            }
        }
        Err(io::Error::from(Errno::DoesNotExist))
    }

    pub fn by_name(name: &str) -> Result<Self, io::Error> {
        Self::find(|ent| ent.name() == name)
    }
}

impl FromStr for UserShadow {
    type Err = io::Error;

    fn from_str(s: &str) -> Result<Self, io::Error> {
        let mut iter = s.split(':');

        let name = iter
            .next()
            .ok_or_else(|| io::Error::from(Errno::InvalidArgument))?;
        let password = iter
            .next()
            .ok_or_else(|| io::Error::from(Errno::InvalidArgument))?;

        if iter.next().is_some() {
            return Err(io::Error::from(Errno::InvalidArgument));
        }

        let mut res = Self {
            name: FixedStr::empty(),
            password: FixedStr::empty(),
        };

        res.name.copy_from_str(name);
        res.password.copy_from_str(password);

        Ok(res)
    }
}
