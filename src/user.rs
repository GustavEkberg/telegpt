use chrono::Utc;
use serde::{Deserialize, Serialize};
use surrealdb;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct User {
    pub telegram_id: u64,
    pub username: Option<String>,
    pub requests_left: u32,
    pub pretend: Option<String>,
    pub previous_messages: Vec<String>,
    pub last_message: Option<u64>,
    pub total_request: u32,
    pub created_at: i64,
}

impl User {
    pub fn new(telegram_id: u64, username: Option<String>) -> Self {
        Self {
            telegram_id,
            username,
            requests_left: 666,
            pretend: None,
            previous_messages: Vec::new(),
            last_message: None,
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
        if self.previous_messages.len() > 5 {
            self.previous_messages.remove(0);
        }
        self.previous_messages.push(message);
    }
}

pub async fn init_user(
    id: &u64,
    username: Option<String>,
) -> Result<User, Box<dyn std::error::Error>> {
    let user = get_user(&id)
        .await
        .unwrap()
        .or(Some(User::new(id.clone(), username)))
        .unwrap();

    println!("USER: {:#?}", user);

    Ok(user)
}

pub async fn get_user(id: &u64) -> Result<Option<User>, Box<dyn std::error::Error>> {
    let db = surrealdb::Datastore::new(dotenvy::var("DB_ENDPOINT").unwrap().as_str()).await?;

    let mut transaction = db.transaction(false, false).await?;
    let value = transaction.get(format!("user:{id}")).await?;
    if value.is_none() {
        Ok(None)
    } else {
        let value = serde_json::from_slice(&value.unwrap()).unwrap();
        Ok(value)
    }
}

pub async fn set_user(user: User) -> Result<(), Box<dyn std::error::Error>> {
    let db = surrealdb::Datastore::new(dotenvy::var("DB_ENDPOINT").unwrap().as_str()).await?;

    let mut transaction = db.transaction(true, false).await?;
    transaction
        .set(
            format!("user:{}", user.telegram_id),
            serde_json::to_vec(&user).unwrap(),
        )
        .await?;
    transaction.commit().await?;

    Ok(())
}
