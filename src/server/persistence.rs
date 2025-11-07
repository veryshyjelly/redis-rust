use crate::server::errors::wrong_num_arguments;
use super::{Args, Result};
use super::server::Server;

impl Server {
    pub async fn config(&mut self, mut args: Args) -> Result {
        let store = self.store.lock().await;
        let get = args.pop_front().ok_or(wrong_num_arguments("config"))?;
        assert_eq!(get.to_lowercase(), "get");
        let mut res = vec![];
        for key in args {
            res.push(key.clone()); 
            let val = match key.to_lowercase().as_str() {
               "dir" => store.info.dir.clone(),
                "dbfilename" => store.info.db_filename.clone(),
                _ => unimplemented!()
            };
            res.push(val);
        }
        Ok(res.into())
    }
}