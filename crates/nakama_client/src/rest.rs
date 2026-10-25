use reqwest::Client;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RestError {
    #[error("HTTP request error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("API error ({status}): {body}")]
    Api { status: u16, body: String },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthResponse {
    pub token: String,
    #[serde(default)]
    pub refresh_token: Option<String>,
}

/// Whether the account already chose a nickname (`get_profile_status` RPC).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileStatus {
    #[serde(default)]
    pub nickname_set: bool,
    #[serde(default)]
    pub username: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JoinPublicRoomResponse {
    pub room_id: String,
    pub match_id: String,
}

pub struct NakamaRestClient {
    client: Client,
    base_url: String,
    server_key: String,
}

impl NakamaRestClient {
    pub fn new(host: &str, port: u16, server_key: &str) -> Self {
        Self {
            client: Client::builder()
                .connect_timeout(crate::CONNECT_TIMEOUT)
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap(),
            base_url: crate::endpoint("http", host, port),
            server_key: server_key.to_string(),
        }
    }

    /// Authenticates with Nakama using a device ID, automatically creating the account if it does not exist.
    pub async fn authenticate_device(
        &self,
        device_id: &str,
        username: &str,
    ) -> Result<AuthResponse, RestError> {
        let url = format!(
            "{}/v2/account/authenticate/device?create=true&username={}",
            self.base_url, username
        );

        let body = serde_json::json!({
            "id": device_id,
        });

        let resp = self
            .client
            .post(&url)
            .basic_auth(&self.server_key, Some(""))
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let text = resp.text().await.unwrap_or_default();

            // Fallback for 409 (e.g. username taken or already claimed): authenticate device without setting username
            if status == 409 {
                let fallback_url = format!(
                    "{}/v2/account/authenticate/device?create=true",
                    self.base_url
                );
                let fallback_resp = self
                    .client
                    .post(&fallback_url)
                    .basic_auth(&self.server_key, Some(""))
                    .json(&body)
                    .send()
                    .await?;

                if fallback_resp.status().is_success() {
                    let auth_resp = fallback_resp.json::<AuthResponse>().await?;
                    return Ok(auth_resp);
                }
            }

            return Err(RestError::Api { status, body: text });
        }

        let auth_resp = resp.json::<AuthResponse>().await?;
        Ok(auth_resp)
    }

    /// Authenticates with a Google `id_token` obtained by the native sign-in on the device,
    /// creating the account on first login. Nakama picks a random username; the player's
    /// nickname is set afterwards through [`Self::set_nickname`].
    pub async fn authenticate_google(&self, id_token: &str) -> Result<AuthResponse, RestError> {
        let url = format!(
            "{}/v2/account/authenticate/google?create=true",
            self.base_url
        );
        let resp = self
            .client
            .post(&url)
            .basic_auth(&self.server_key, Some(""))
            .json(&serde_json::json!({ "token": id_token }))
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(RestError::Api { status, body });
        }
        Ok(resp.json::<AuthResponse>().await?)
    }

    /// Exchanges a saved refresh token for a fresh session (and a fresh refresh token).
    pub async fn refresh_session(&self, refresh_token: &str) -> Result<AuthResponse, RestError> {
        let url = format!("{}/v2/account/session/refresh", self.base_url);
        let resp = self
            .client
            .post(&url)
            .basic_auth(&self.server_key, Some(""))
            .json(&serde_json::json!({ "token": refresh_token }))
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(RestError::Api { status, body });
        }
        Ok(resp.json::<AuthResponse>().await?)
    }

    /// Revokes the session and its refresh token on the server (`/v2/session/logout`).
    pub async fn logout(&self, token: &str, refresh_token: &str) -> Result<(), RestError> {
        let url = format!("{}/v2/session/logout", self.base_url);
        let resp = self
            .client
            .post(&url)
            .bearer_auth(token)
            .json(&serde_json::json!({ "token": token, "refresh_token": refresh_token }))
            .send()
            .await?;
        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(RestError::Api { status, body });
        }
        Ok(())
    }

    /// Stores the nickname as the player's unique Nakama username (`set_nickname` RPC).
    pub async fn set_nickname(&self, token: &str, nickname: &str) -> Result<(), RestError> {
        let _: serde_json::Value = self
            .rpc(
                token,
                "set_nickname",
                &serde_json::json!({ "nickname": nickname }),
            )
            .await?;
        Ok(())
    }

    /// Tells whether the authenticated account already chose its nickname.
    pub async fn get_profile_status(&self, token: &str) -> Result<ProfileStatus, RestError> {
        self.rpc(token, "get_profile_status", &serde_json::json!({}))
            .await
    }

    /// The appearance the server stores for the authenticated account (`get_avatar_appearance`
    /// RPC). The server answers the default appearance when none was ever saved.
    pub async fn get_avatar_appearance(
        &self,
        token: &str,
    ) -> Result<protocol::AvatarAppearance, RestError> {
        #[derive(Deserialize)]
        struct Response {
            appearance: protocol::AvatarAppearance,
        }
        let response: Response = self
            .rpc(token, "get_avatar_appearance", &serde_json::json!({}))
            .await?;
        Ok(response.appearance)
    }

    /// Calls the join_public_room RPC on Nakama to retrieve the active match_id for the designated room.
    pub async fn join_public_room(
        &self,
        token: &str,
        room_id: &str,
    ) -> Result<JoinPublicRoomResponse, RestError> {
        let url = format!("{}/v2/rpc/join_public_room?unwrap", self.base_url);
        let payload = serde_json::json!({
            "room_id": room_id,
        });

        let resp = self
            .client
            .post(&url)
            .bearer_auth(token)
            .json(&payload)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let text = resp.text().await.unwrap_or_default();
            return Err(RestError::Api { status, body: text });
        }

        let join_resp = resp.json::<JoinPublicRoomResponse>().await?;
        Ok(join_resp)
    }

    /// Fetches all enabled room layouts from the catalog.
    pub async fn list_room_layouts(
        &self,
        token: &str,
    ) -> Result<protocol::ListRoomLayoutsResponse, RestError> {
        let url = format!("{}/v2/rpc/list_room_layouts?unwrap", self.base_url);
        let resp = self
            .client
            .post(&url)
            .bearer_auth(token)
            .json(&serde_json::json!({}))
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let text = resp.text().await.unwrap_or_default();
            return Err(RestError::Api { status, body: text });
        }

        let layouts_resp = resp.json::<protocol::ListRoomLayoutsResponse>().await?;
        Ok(layouts_resp)
    }

    /// Creates a new user space with the specified name and layout template.
    pub async fn create_room(
        &self,
        token: &str,
        name: &str,
        layout_id: &str,
    ) -> Result<protocol::CreateRoomResponse, RestError> {
        let url = format!("{}/v2/rpc/create_room?unwrap", self.base_url);
        let payload = protocol::CreateRoomRequest {
            name: name.to_string(),
            layout_id: layout_id.to_string(),
        };

        let resp = self
            .client
            .post(&url)
            .bearer_auth(token)
            .json(&payload)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let text = resp.text().await.unwrap_or_default();
            return Err(RestError::Api { status, body: text });
        }

        let create_resp = resp.json::<protocol::CreateRoomResponse>().await?;
        Ok(create_resp)
    }

    /// Lists public user spaces ordered by online avatar count.
    pub async fn list_popular_rooms(
        &self,
        token: &str,
    ) -> Result<protocol::ListRoomsResponse, RestError> {
        let url = format!("{}/v2/rpc/list_popular_rooms?unwrap", self.base_url);
        let resp = self
            .client
            .post(&url)
            .bearer_auth(token)
            .json(&serde_json::json!({}))
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let text = resp.text().await.unwrap_or_default();
            return Err(RestError::Api { status, body: text });
        }

        let rooms_resp = resp.json::<protocol::ListRoomsResponse>().await?;
        Ok(rooms_resp)
    }

    /// Lists all rooms owned by the authenticated player.
    pub async fn list_my_rooms(
        &self,
        token: &str,
    ) -> Result<protocol::ListRoomsResponse, RestError> {
        let url = format!("{}/v2/rpc/list_my_rooms?unwrap", self.base_url);
        let resp = self
            .client
            .post(&url)
            .bearer_auth(token)
            .json(&serde_json::json!({}))
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let text = resp.text().await.unwrap_or_default();
            return Err(RestError::Api { status, body: text });
        }

        let rooms_resp = resp.json::<protocol::ListRoomsResponse>().await?;
        Ok(rooms_resp)
    }

    /// Searches public spaces matching the query string.
    pub async fn search_rooms(
        &self,
        token: &str,
        query: &str,
    ) -> Result<protocol::ListRoomsResponse, RestError> {
        let url = format!("{}/v2/rpc/search_rooms?unwrap", self.base_url);
        let payload = serde_json::json!({ "query": query });

        let resp = self
            .client
            .post(&url)
            .bearer_auth(token)
            .json(&payload)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let text = resp.text().await.unwrap_or_default();
            return Err(RestError::Api { status, body: text });
        }

        let rooms_resp = resp.json::<protocol::ListRoomsResponse>().await?;
        Ok(rooms_resp)
    }

    /// Updates room name and visibility (owner must be in-room).
    pub async fn update_room(
        &self,
        token: &str,
        room_id: &str,
        name: &str,
        visibility: &str,
    ) -> Result<(), RestError> {
        let url = format!("{}/v2/rpc/update_room?unwrap", self.base_url);
        let payload = protocol::UpdateRoomRequest {
            room_id: room_id.to_string(),
            name: name.to_string(),
            visibility: visibility.to_string(),
        };

        let resp = self
            .client
            .post(&url)
            .bearer_auth(token)
            .json(&payload)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let text = resp.text().await.unwrap_or_default();
            return Err(RestError::Api { status, body: text });
        }
        Ok(())
    }

    /// Deletes a room (owner must be in-room).
    pub async fn delete_room(&self, token: &str, room_id: &str) -> Result<(), RestError> {
        let url = format!("{}/v2/rpc/delete_room?unwrap", self.base_url);
        let payload = protocol::DeleteRoomRequest {
            room_id: room_id.to_string(),
        };

        let resp = self
            .client
            .post(&url)
            .bearer_auth(token)
            .json(&payload)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let text = resp.text().await.unwrap_or_default();
            return Err(RestError::Api { status, body: text });
        }
        Ok(())
    }

    /// Attempts to join or ring the doorbell for a room.
    pub async fn join_room(
        &self,
        token: &str,
        room_id: &str,
    ) -> Result<protocol::JoinRoomResponse, RestError> {
        let url = format!("{}/v2/rpc/join_room?unwrap", self.base_url);
        let payload = protocol::JoinRoomRequest {
            room_id: room_id.to_string(),
        };

        let resp = self
            .client
            .post(&url)
            .bearer_auth(token)
            .json(&payload)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let text = resp.text().await.unwrap_or_default();
            return Err(RestError::Api { status, body: text });
        }

        let join_resp = resp.json::<protocol::JoinRoomResponse>().await?;
        Ok(join_resp)
    }

    /// Checks if a doorbell request has been allowed or denied by the owner.
    pub async fn check_doorbell(
        &self,
        token: &str,
        match_id: &str,
        request_id: &str,
    ) -> Result<CheckDoorbellResponse, RestError> {
        let url = format!("{}/v2/rpc/check_doorbell?unwrap", self.base_url);
        let payload = serde_json::json!({
            "match_id": match_id,
            "request_id": request_id,
        });

        let resp = self
            .client
            .post(&url)
            .bearer_auth(token)
            .json(&payload)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let text = resp.text().await.unwrap_or_default();
            return Err(RestError::Api { status, body: text });
        }

        let check_resp = resp.json::<CheckDoorbellResponse>().await?;
        Ok(check_resp)
    }

    /// Fetches the user inventory items.
    pub async fn get_user_inventory(
        &self,
        token: &str,
    ) -> Result<protocol::GetUserInventoryResponse, RestError> {
        let url = format!("{}/v2/rpc/get_user_inventory?unwrap", self.base_url);
        let resp = self
            .client
            .post(&url)
            .bearer_auth(token)
            .json(&serde_json::json!({}))
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let text = resp.text().await.unwrap_or_default();
            return Err(RestError::Api { status, body: text });
        }

        let inv_resp = resp.json::<protocol::GetUserInventoryResponse>().await?;
        Ok(inv_resp)
    }

    /// Calls an authenticated RPC and decodes its JSON response.
    async fn rpc<B: Serialize, T: serde::de::DeserializeOwned>(
        &self,
        token: &str,
        name: &str,
        body: &B,
    ) -> Result<T, RestError> {
        let url = format!("{}/v2/rpc/{name}?unwrap", self.base_url);
        let resp = self
            .client
            .post(&url)
            .bearer_auth(token)
            .json(body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let text = resp.text().await.unwrap_or_default();
            return Err(RestError::Api { status, body: text });
        }
        Ok(resp.json::<T>().await?)
    }

    /// Fetches the player's silver and gold balances.
    pub async fn get_wallet(&self, token: &str) -> Result<protocol::WalletDto, RestError> {
        self.rpc(token, "get_wallet", &serde_json::json!({})).await
    }

    /// Reports presence so the server can grant daily-login and play-time silver.
    pub async fn wallet_heartbeat(
        &self,
        token: &str,
    ) -> Result<protocol::HeartbeatResponse, RestError> {
        self.rpc(token, "wallet_heartbeat", &serde_json::json!({}))
            .await
    }

    /// Fetches the store menu and offers.
    pub async fn get_shop_catalog(
        &self,
        token: &str,
    ) -> Result<protocol::GetShopCatalogResponse, RestError> {
        self.rpc(token, "get_shop_catalog", &serde_json::json!({}))
            .await
    }

    /// Fetches the furniture, cosmetics and store catalogs; `if_version` is the cached `catalog_version`
    /// (empty when nothing is cached), so an up-to-date cache gets a minimal `unchanged` reply.
    pub async fn get_catalogs(
        &self,
        token: &str,
        if_version: &str,
    ) -> Result<protocol::GetCatalogsResponse, RestError> {
        let request = protocol::GetCatalogsRequest {
            if_version: if_version.to_string(),
        };
        self.rpc(token, "get_catalogs", &request).await
    }

    /// Buys `quantity` units of an offer; the server decides the price.
    pub async fn purchase_item(
        &self,
        token: &str,
        catalog_id: &str,
        quantity: i32,
    ) -> Result<protocol::PurchaseItemResponse, RestError> {
        let request = protocol::PurchaseItemRequest {
            catalog_id: catalog_id.to_string(),
            quantity,
        };
        self.rpc(token, "purchase_item", &request).await
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckDoorbellResponse {
    pub status: String,
    #[serde(default)]
    pub match_id: Option<String>,
}

/// Extracts the human-readable `message` from a Nakama RPC error body
/// (`{"code":9,"message":"saldo insuficiente"}`), falling back to the raw text.
pub fn api_error_message(error: &RestError) -> String {
    #[derive(Deserialize)]
    struct Body {
        message: String,
    }
    match error {
        RestError::Api { body, .. } => serde_json::from_str::<Body>(body)
            .map(|b| b.message)
            .unwrap_or_else(|_| body.clone()),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_status_parses_server_payload() {
        let set: ProfileStatus =
            serde_json::from_str(r#"{"nickname_set":true,"username":"Alice_1"}"#).unwrap();
        assert!(set.nickname_set);
        assert_eq!(set.username, "Alice_1");
        let unset: ProfileStatus = serde_json::from_str(r#"{"nickname_set":false}"#).unwrap();
        assert_eq!(unset, ProfileStatus::default());
    }

    #[test]
    fn api_error_message_extracts_server_message() {
        let err = RestError::Api {
            status: 400,
            body: r#"{"code":9,"message":"saldo insuficiente"}"#.to_string(),
        };
        assert_eq!(api_error_message(&err), "saldo insuficiente");
    }

    #[test]
    fn api_error_message_falls_back_to_raw_body() {
        let err = RestError::Api {
            status: 502,
            body: "bad gateway".to_string(),
        };
        assert_eq!(api_error_message(&err), "bad gateway");
    }
}
