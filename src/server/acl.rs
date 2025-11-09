use super::server::Server;
use super::{Args, Result};
use crate::frame::{Frame, TypedNone};
use crate::server::errors::wrong_num_arguments;
use std::collections::HashMap;
use sha2::digest::{FixedOutput, Update};
use sha2::Sha256;

impl Server {
    /// This is a container command for Access Control List commands.
    pub async fn acl(&mut self, mut args: Args) -> Result {
        let command = args.pop_front().ok_or(wrong_num_arguments("acl"))?;
        match command.to_lowercase().as_str() {
            "whoami" => self.acl_who_ami(args).await,
            "getuser" => self.acl_get_user(args).await,
            "setuser" => self.acl_set_user(args).await,
            _ => unimplemented!("{command}"),
        }
    }

    /// Return the username the current connection is authenticated with. New connections
    /// are authenticated with the "default" user. They can change user using AUTH.
    /// ```
    /// ACL WHOAMI
    /// ```
    pub async fn acl_who_ami(&mut self, _: Args) -> Result {
        Ok(self.user.clone().into())
    }

    /// The command returns all the rules defined for an existing ACL user.
    /// ```
    /// ACL GETUSER username
    /// ```
    pub async fn acl_get_user(&mut self, mut args: Args) -> Result {
        let username = args.pop_front().ok_or(wrong_num_arguments("getuser"))?;
        if let Some(user) = self.store.lock().await.users.get(&username) {
            let mut res: Vec<Frame> = vec![];
            let mut keys = user.keys().collect::<Vec<&String>>();
            keys.sort();
            for k in keys  {
                let v = user.get(k).unwrap();
                res.push(k.clone().into());
                res.push(v.clone().into());
            }
            Ok(res.into())
        } else {
            Ok(Frame::None(TypedNone::String))
        }
    }

    /// Create an ACL user with the specified rules or modify the rules of an existing user.
    ///
    /// Manipulate Redis ACL users interactively. If the username does not exist, the command
    /// creates the username without any privilege. It then reads from left to right all the
    /// rules provided as successive arguments, setting the user ACL rules as specified.
    /// If the user already exists, the provided ACL rules are simply applied in addition
    /// to the rules already set.
    /// ```
    /// ACL SETUSER username [rule [rule ...]]
    /// ```
    pub async fn acl_set_user(&mut self, mut args: Args) -> Result {
        let username = args.pop_front().ok_or(wrong_num_arguments("getuser"))?;
        let mut store = self.store.lock().await;
        let user = store.users.entry(username).or_insert(HashMap::from([
            ("flags".into(), vec!["nopass".into()]),
            ("passwords".into(), vec![]),
        ]));
        for arg in args {
            if arg.starts_with(">") {
                let mut hasher = Sha256::default();
                let password = arg[1..].to_string();
                hasher.update(password.as_bytes());
                let x = hasher.finalize_fixed();
                user.get_mut(&"passwords".to_string())
                    .unwrap()
                    .push(format!("{:x}", x));
                let flags = user.get_mut(&"flags".to_string()).unwrap();
                if let Some(idx) = flags.iter().position(|v| v == "nopass") {
                    flags.remove(idx);
                }
            }
        }
        Ok("OK".into())
    }

    /// The AUTH command authenticates the current connection in two cases:
    /// If the Redis server is password protected via the requirepass option.
    /// A Redis 6.0 instance, or greater, is using the Redis ACL system.
    /// ```
    /// AUTH [username] password
    /// ```
    pub async fn auth(&mut self, mut args: Args) -> Result {
        let username = args.pop_front().ok_or(wrong_num_arguments("getuser"))?;
        if let Some(user) = self.store.lock().await.users.get(&username) {
            let mut hasher = Sha256::default();
            let password = args.pop_front().ok_or(wrong_num_arguments("getuser"))?;
            hasher.update(password.as_bytes());
            let x = hasher.finalize_fixed();
            let passwords = user.get("passwords").unwrap();
            if passwords.contains(&format!("{:x}", x)) {
                self.user = username;
                self.authenticated = true;
                Ok("OK".into())
            } else {
                Err("WRONGPASS invalid username-password pair or user is disabled.".into())
            }
        } else {
            Err("WRONGPASS invalid username-password pair or user is disabled.".into())
        }
    }
}
