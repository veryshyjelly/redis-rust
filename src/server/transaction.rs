use super::Args;
use super::Result;
use super::errors::wrong_num_arguments;
use super::server::Server;
use crate::frame::Frame;

impl Server {
    pub fn multi(&mut self, _: Args) -> Result {
        self.in_transaction = true;
        Ok("OK".into())
    }

    pub async fn transaction(&mut self, cmd: Args) -> Result {
        match cmd
            .get(0)
            .ok_or(wrong_num_arguments("exec"))?
            .to_lowercase()
            .as_str()
        {
            "exec" => {
                self.in_transaction = false;
                let commands: Vec<_> = self.transaction.drain(..).collect();

                #[cfg(debug_assertions)]
                println!("running queued commands: \n{:?}", commands);

                let mut res = vec![];
                for v in commands {
                    match self.execute(v).await {
                        Ok(r) => res.push(r),
                        Err(e) => res.push(Frame::SimpleError(format!("{e}"))),
                    }
                }

                Ok(res.into())
            }
            "discard" => {
                self.transaction.drain(..);
                self.in_transaction = false;
                Ok("OK".into())
            }
            _ => {
                self.transaction.push_back(cmd);
                Ok("QUEUED".into())
            }
        }
    }
}
