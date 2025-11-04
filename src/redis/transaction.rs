use crate::redis::errors::wrong_num_arguments;
use crate::resp::RESP;
use super::Redis;
use super::redis::Command;

impl Redis {
    pub fn multi(&mut self, _: Command) -> std::io::Result<RESP> {
        self.is_transaction = true;
        Ok("OK".into())
    }

    pub fn transaction(&mut self, cmd: Command) -> std::io::Result<RESP> {
        if cmd.get(0).ok_or(wrong_num_arguments("exec"))?.to_lowercase() != "exec" {
            self.transaction.push(cmd);
            Ok("QUEUED".into())
        } else {
            self.is_transaction = false;
            let commands: Vec<_> = self.transaction.drain( ..).collect();

            #[cfg(debug_assertions)]
            println!("running queued commands: \n{:?}", commands);

            let mut res = vec![];
            for v in commands {
                res.push(self.execute(v)?);
            }
            
            Ok(res.into())
        }
    }
}