use std::{env, error::Error};

use crate::{
    clean_string,
    content::extract_url_content,
    openai::send_text_to_chatgpt,
    user::{set_user, User},
};
use teloxide::{
    prelude::*,
    types::{Message, MessageEntityKind},
    Bot,
};

pub async fn summarize(bot: Bot, message: Message, mut user: User) -> Result<(), Box<dyn Error>> {
    bot.send_message(message.chat.id, "Hmmm.... let me think...")
        .send()
        .await?;

    let url_position = if let Some(message) = message
        .entities()
        .unwrap()
        .iter()
        .find(|entity| entity.kind.eq(&MessageEntityKind::Url))
    {
        (message.offset, message.length)
    } else {
        bot.send_message(message.chat.id, "Please provide a url to summarize")
            .send()
            .await?;
        return Ok(());
    };

    let url = message.text().unwrap()[url_position.0..url_position.0 + url_position.1].to_string();
    let content = extract_url_content(&url).await.unwrap();

    if content.is_none() {
        bot.send_message(message.chat.id, "Could not extract content from url")
            .send()
            .await?;
        return Ok(());
    }

    user.clear_history();
    let content = content.unwrap();
    let content_message = format!("Summarize the following content in a list, ignoring any mentions of subscribing to a newspaper or magazine. ---- \nUrl: \"{url}\". \n\n Content: \n\"{content}\"");

    let response = send_text_to_chatgpt(&content_message, &user).await;

    bot.send_message(message.chat.id, clean_string(response.unwrap()))
        .parse_mode(teloxide::types::ParseMode::MarkdownV2)
        .send()
        .await?;

    user.update_requests();
    user.update_last_message(content_message);

    set_user(user.clone()).await.unwrap();
    Ok(())
}

pub async fn request(bot: Bot, message: Message) -> Result<(), Box<dyn Error>> {
    let request_text = message
        .text()
        .unwrap()
        .replace("/request", "")
        .trim()
        .to_string();

    if request_text.is_empty() {
        bot.send_message(
            message.chat.id,
            "Please provide a description of the feature you would like to see",
        )
        .send()
        .await?;
        return Ok(());
    }
    bot.send_message(
        message.from().unwrap().id,
        "Thank you so much for your request! We will get back to you soon.",
    )
    .send()
    .await?;

    let admin_id = env::var("ADMIN_ID").expect("Missing env variable ADMIN_ID");
    let from = message.from().unwrap();
    bot.send_message(
        admin_id,
        format!(
            "New request from user:\n[{}]({})\n**Request**:\n{}",
            from.clone().username.unwrap_or("Anon".to_string()),
            from.clone().url(),
            request_text
        ),
    )
    .parse_mode(teloxide::types::ParseMode::MarkdownV2)
    .send()
    .await?;
    Ok(())
}
