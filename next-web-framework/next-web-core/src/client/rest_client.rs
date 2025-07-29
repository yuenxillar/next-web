use std::{ops::Deref, sync::Arc};

pub struct RestClient {
    client: reqwest::Client,
}


impl RestClient {
    
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

impl Clone for RestClient {
    fn clone(&self) -> Self {
        Self { 
            client: self.client.clone()
         }
    }
}

impl Deref for RestClient {
    type Target = reqwest::Client;

    fn deref(&self) -> &Self::Target {
        &self.client
    }
}