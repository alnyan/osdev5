use crate::file::File;
use crate::io::{Read, read_line};
use core::str::FromStr;
use libsys::FixedStr;

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


    pub fn find<F: Fn(&Self) -> bool>(pred: F) -> Result<Self, ()> {
        let mut file = File::open("/etc/shadow").map_err(|_| ())?;
        let mut buf = [0; 128];
        loop {
            let line = read_line(&mut file, &mut buf).map_err(|_| ())?;
            if let Some(line) = line {
                let ent = UserShadow::from_str(line)?;
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

impl FromStr for UserShadow {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, ()> {
        let mut iter = s.split(':');

        let name = iter.next().ok_or(())?;
        let password = iter.next().ok_or(())?;

        if iter.next().is_some() {
            return Err(());
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
