use crate::config::messenger::Messenger;
use crate::types::error::{Error, Result};
use crate::types::phone::Phone;
use reqwest::Client;
use serde_json::json;

const URL: &str = "https://graph.facebook.com/v21.0/960426547146856/messages";

#[derive(Clone)]
pub struct Whatsapp {
    token: &'static str,
    client: Client,
}

impl Default for Whatsapp {
    fn default() -> Self {
        let token = env!("WHATSAPP_TOKEN");
        let client = Client::new();
        Self { token, client }
    }
}

impl Messenger for Whatsapp {
    type Recipient = Phone;

    async fn send_code(&self, recipient: &Self::Recipient, code: &str) -> Result<()> {
        let json = json!({
            "messaging_product": "whatsapp",
            "to": recipient.as_ref(),
            "type": "template",
            "template": {
                "name": "auth_code",
                "language": {"code": "en"},
                "components": [
                    {
                        "type": "body",
                        "parameters": [{"type": "text", "text": code}]
                    },
                    {
                        "type": "button",
                        "sub_type": "url",
                        "index": 0,
                        "parameters": [{"type": "text", "text": code}]
                    }
                ],
            }
        });
        self.client
            .post(URL)
            .bearer_auth(self.token)
            .json(&json)
            .send()
            .await
            .map_err(Error::internal)?
            .error_for_status()
            .map_err(Error::internal)?;
        Ok(())
    }
}
