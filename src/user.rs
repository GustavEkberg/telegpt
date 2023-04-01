use std::error::Error;

use crate::db::db_sql;
use chrono::Utc;
use reqwest::Url;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct User {
    pub telegram_id: u64,
    pub first_name: String,
    pub username: Option<String>,
    pub url: Option<Url>,
    pub requests_left: u32,
    pub previous_messages: Vec<String>,
    pub last_message: i64,
    pub total_request: u32,
    pub created_at: i64,
}

impl User {
    pub fn new(
        telegram_id: u64,
        first_name: String,
        username: Option<String>,
        url: Option<Url>,
    ) -> Self {
        Self {
            telegram_id,
            first_name,
            username,
            url,
            requests_left: 25,
            previous_messages: Vec::new(),
            last_message: Utc::now().timestamp(),
            total_request: 0,
            created_at: Utc::now().timestamp(),
        }
    }

    pub fn has_requests_left(&self) -> bool {
        self.requests_left.gt(&0)
    }

    pub fn update_requests(&mut self) {
        self.requests_left -= 1;
        self.total_request += 1;
    }

    pub fn update_last_message(&mut self, message: String) {
        if self.previous_messages.len() > 3 {
            self.previous_messages.remove(0);
        }
        self.last_message = Utc::now().timestamp();
        self.previous_messages.push(message);
    }

    pub fn clear_history(&mut self) {
        self.previous_messages.clear();
    }
}

pub async fn init_user(id: &u64, user: teloxide::types::User) -> Result<User, Box<dyn Error>> {
    let user = get_user(id).await.unwrap().unwrap_or(User::new(
        *id,
        user.clone().first_name,
        user.clone().username,
        user.tme_url(),
    ));

    Ok(user)
}

pub async fn get_user(id: &u64) -> Result<Option<User>, Box<dyn Error>> {
    let user = db_sql::<User>(format!("SELECT * FROM user:{id};").as_str()).await?;
    Ok(user)
}

pub async fn set_user(user: User) -> Result<(), Box<dyn Error>> {
    let user_string = serde_json::to_string(&user).unwrap();

    let query = format!("UPDATE user:{} CONTENT {user_string}", user.telegram_id);

    db_sql::<User>(query.as_str()).await?;
    Ok(())
}
