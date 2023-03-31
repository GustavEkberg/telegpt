use std::{env, fmt::Debug};

use reqwest::{header, Client, Error};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Response<T> {
    pub result: Vec<T>,
    pub information: Option<String>,
}

pub async fn db_sql<T>(data: &str) -> Result<Option<T>, Error>
where
    T: for<'a> Deserialize<'a> + Clone + Debug,
{
    let username = env::var("DB_USER").unwrap();
    let password = env::var("DB_PASSWORD").unwrap();
    let url = format!("{}{}", env::var("DB_ENDPOINT").unwrap(), "/sql");
    let client = Client::new();
    let response: Vec<Response<T>> = client
        .post(&url)
        .header(header::ACCEPT, "application/json")
        .basic_auth(username, Some(password))
        .header("NS", env::var("DB_NS").unwrap())
        .header("DB", env::var("DB_DB").unwrap())
        .body(data.to_string())
        .send()
        .await?
        .json()
        .await?;

    if response.first().unwrap().result.is_empty() {
        Ok(None)
    } else {
        let response = response.first().unwrap().result.clone();
        Ok(Some(response.first().unwrap().clone()))
    }
}
