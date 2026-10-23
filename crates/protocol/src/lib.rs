pub mod messages;
pub mod opcodes;

pub use messages::*;
pub use opcodes::*;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ProtocolError {
    #[error("Failed to serialize message: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Unknown or unhandled opcode: {0}")]
    UnknownOpcode(i64),
}

/// Decodes raw match data bytes using the opcode into a high-level ServerMessage enum
pub fn decode_server_message(opcode: i64, data: &[u8]) -> Result<ServerMessage, ProtocolError> {
    match opcode {
        OPCODE_ROOM_SNAPSHOT => {
            let msg: RoomSnapshot = serde_json::from_slice(data)?;
            Ok(ServerMessage::RoomSnapshot(msg))
        }
        OPCODE_AVATAR_JOINED => {
            let msg: AvatarJoined = serde_json::from_slice(data)?;
            Ok(ServerMessage::AvatarJoined(msg))
        }
        OPCODE_AVATAR_LEFT => {
            let msg: AvatarLeft = serde_json::from_slice(data)?;
            Ok(ServerMessage::AvatarLeft(msg))
        }
        OPCODE_AVATAR_PATH => {
            let msg: AvatarPath = serde_json::from_slice(data)?;
            Ok(ServerMessage::AvatarPath(msg))
        }
        OPCODE_MOVE_REJECTED => {
            let msg: MoveRejected = serde_json::from_slice(data)?;
            Ok(ServerMessage::MoveRejected(msg))
        }
        OPCODE_DOORBELL_REQUEST => {
            let msg: DoorbellRequest = serde_json::from_slice(data)?;
            Ok(ServerMessage::DoorbellRequest(msg))
        }
        OPCODE_ROOM_CLOSED => {
            let msg: RoomClosed = serde_json::from_slice(data)?;
            Ok(ServerMessage::RoomClosed(msg))
        }
        OPCODE_ITEM_PLACED => {
            let msg: ItemPlaced = serde_json::from_slice(data)?;
            Ok(ServerMessage::ItemPlaced(msg))
        }
        OPCODE_ITEM_MOVED => {
            let msg: ItemMoved = serde_json::from_slice(data)?;
            Ok(ServerMessage::ItemMoved(msg))
        }
        OPCODE_ITEM_ROTATED => {
            let msg: ItemRotated = serde_json::from_slice(data)?;
            Ok(ServerMessage::ItemRotated(msg))
        }
        OPCODE_ITEM_REMOVED => {
            let msg: ItemRemoved = serde_json::from_slice(data)?;
            Ok(ServerMessage::ItemRemoved(msg))
        }
        OPCODE_BUILD_MODE_REJECTED => {
            let msg: BuildModeRejected = serde_json::from_slice(data)?;
            Ok(ServerMessage::BuildModeRejected(msg))
        }
        OPCODE_AVATAR_APPEARANCE_UPDATED => {
            let msg: AvatarAppearanceUpdated = serde_json::from_slice(data)?;
            Ok(ServerMessage::AvatarAppearanceUpdated(msg))
        }
        OPCODE_AVATAR_POSTURE_CHANGED => {
            let msg: AvatarPostureChanged = serde_json::from_slice(data)?;
            Ok(ServerMessage::AvatarPostureChanged(msg))
        }
        _ => Err(ProtocolError::UnknownOpcode(opcode)),
    }
}

/// Encodes a ClientMessage into opcode and JSON payload bytes
pub fn encode_client_message(msg: &ClientMessage) -> Result<(i64, Vec<u8>), ProtocolError> {
    match msg {
        ClientMessage::Move(req) => {
            let bytes = serde_json::to_vec(req)?;
            Ok((OPCODE_MOVE_REQUEST, bytes))
        }
        ClientMessage::DoorbellResponse(resp) => {
            let bytes = serde_json::to_vec(resp)?;
            Ok((OPCODE_DOORBELL_RESPONSE, bytes))
        }
        ClientMessage::PlaceItem(req) => {
            let bytes = serde_json::to_vec(req)?;
            Ok((OPCODE_PLACE_ITEM_REQUEST, bytes))
        }
        ClientMessage::MoveItem(req) => {
            let bytes = serde_json::to_vec(req)?;
            Ok((OPCODE_MOVE_ITEM_REQUEST, bytes))
        }
        ClientMessage::RotateItem(req) => {
            let bytes = serde_json::to_vec(req)?;
            Ok((OPCODE_ROTATE_ITEM_REQUEST, bytes))
        }
        ClientMessage::PickupItem(req) => {
            let bytes = serde_json::to_vec(req)?;
            Ok((OPCODE_PICKUP_ITEM_REQUEST, bytes))
        }
        ClientMessage::UpdateAppearance(req) => {
            let bytes = serde_json::to_vec(req)?;
            Ok((OPCODE_UPDATE_APPEARANCE_REQUEST, bytes))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn appearance_from_the_previous_schema_still_parses() {
        // Saved before the gender was removed and the face became a slot: the unknown
        // `gender` is ignored and the missing `face_id` falls back to the default.
        let legacy = r##"{"gender":"girl","skin_color":"#D6A374","hair_id":"hair_girl_old","accessories":[]}"##;
        let parsed: AvatarAppearance = serde_json::from_str(legacy).unwrap();
        assert_eq!(parsed.skin_color, "#D6A374");
        assert_eq!(parsed.hair_id, "hair_girl_old");
        assert_eq!(parsed.face_id, AvatarAppearance::default().face_id);
    }

    #[test]
    fn appearance_serializes_the_face_and_no_gender() {
        let json = serde_json::to_value(AvatarAppearance::default()).unwrap();
        assert!(json.get("face_id").is_some());
        assert!(json.get("gender").is_none());
    }

    #[test]
    fn test_encode_move_request() {
        let msg = ClientMessage::Move(MoveRequest { x: 3, z: 7 });
        let (opcode, bytes) = encode_client_message(&msg).expect("encode should succeed");
        assert_eq!(opcode, OPCODE_MOVE_REQUEST);

        let parsed: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(parsed["x"], 3);
        assert_eq!(parsed["z"], 7);
    }

    #[test]
    fn test_decode_room_snapshot() {
        let raw = r#"{
            "room_id": "lobby",
            "room_name": "Central Plaza",
            "layout": {
                "id": "lobby_12x10",
                "width": 12,
                "height": 10,
                "door": [1, 5],
                "heightmap": ["00", "00"]
            },
            "avatars": [
                {
                    "user_id": "u1",
                    "display_name": "Alice",
                    "x": 1,
                    "z": 5
                }
            ]
        }"#;

        let decoded = decode_server_message(OPCODE_ROOM_SNAPSHOT, raw.as_bytes())
            .expect("should decode snapshot");

        match decoded {
            ServerMessage::RoomSnapshot(snap) => {
                assert_eq!(snap.room_id, "lobby");
                assert_eq!(snap.avatars.len(), 1);
                assert_eq!(snap.avatars[0].display_name, "Alice");
            }
            _ => panic!("wrong message variant"),
        }
    }

    #[test]
    fn test_doorbell_roundtrip() {
        let req_raw = r#"{
            "request_id": "req-1",
            "requester_id": "u-bob",
            "requester_name": "Bob"
        }"#;

        let decoded = decode_server_message(OPCODE_DOORBELL_REQUEST, req_raw.as_bytes())
            .expect("should decode doorbell request");
        match decoded {
            ServerMessage::DoorbellRequest(req) => {
                assert_eq!(req.request_id, "req-1");
                assert_eq!(req.requester_name, "Bob");
            }
            _ => panic!("wrong message variant"),
        }

        let resp_msg = ClientMessage::DoorbellResponse(DoorbellResponse {
            request_id: "req-1".into(),
            allowed: true,
        });
        let (opcode, bytes) =
            encode_client_message(&resp_msg).expect("should encode doorbell response");
        assert_eq!(opcode, OPCODE_DOORBELL_RESPONSE);

        let parsed: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(parsed["request_id"], "req-1");
        assert_eq!(parsed["allowed"], true);
    }

    #[test]
    fn test_room_closed() {
        let raw = r#"{"reason":"room_deleted"}"#;
        let decoded = decode_server_message(OPCODE_ROOM_CLOSED, raw.as_bytes())
            .expect("should decode room closed");
        match decoded {
            ServerMessage::RoomClosed(closed) => {
                assert_eq!(closed.reason, "room_deleted");
            }
            _ => panic!("wrong message variant"),
        }
    }

    #[test]
    fn test_furniture_client_encode_decode() {
        let place = ClientMessage::PlaceItem(PlaceItemRequest {
            catalog_id: "solid_red_block".into(),
            x: 5,
            z: 7,
            rotation: 1,
        });
        let (opcode, bytes) = encode_client_message(&place).expect("should encode place item");
        assert_eq!(opcode, OPCODE_PLACE_ITEM_REQUEST);

        let parsed: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(parsed["catalog_id"], "solid_red_block");
        assert_eq!(parsed["rotation"], 1);

        let placed_raw = r##"{
            "id": "placed_123",
            "catalog_id": "solid_red_block",
            "owner_id": "u_alice",
            "x": 5,
            "z": 7,
            "y": 0.1,
            "rotation": 1,
            "width": 1,
            "depth": 1,
            "walkable": false,
            "color": "#eb4d4b",
            "asset_id": ""
        }"##;

        let decoded = decode_server_message(OPCODE_ITEM_PLACED, placed_raw.as_bytes())
            .expect("should decode placed item");
        match decoded {
            ServerMessage::ItemPlaced(item) => {
                assert_eq!(item.id, "placed_123");
                assert!(!item.walkable);
                assert_eq!(item.color, "#eb4d4b");
            }
            _ => panic!("wrong message variant"),
        }
    }

    #[test]
    fn test_decode_avatar_posture_changed_documented_json() {
        // Shared fixture owned by the server (contract/fixtures, opcode 113).
        let raw = include_str!("../../../contract/fixtures/avatar_posture_changed.json");
        let decoded = decode_server_message(OPCODE_AVATAR_POSTURE_CHANGED, raw.as_bytes())
            .expect("should decode posture change");
        assert_eq!(
            decoded,
            ServerMessage::AvatarPostureChanged(AvatarPostureChanged {
                user_id: "99a9a0e6-8152-4414-b657-3fbb1c77174e".to_string(),
                posture: Posture::Sit,
                rotation: 4,
                y: 0.8,
                rest_x: 4.5,
                rest_z: 3.0,
            })
        );

        let lay = r#"{"user_id": "u", "posture": "lay", "rotation": 0, "y": 0.0, "rest_x": 1.0, "rest_z": 2.0}"#;
        match decode_server_message(OPCODE_AVATAR_POSTURE_CHANGED, lay.as_bytes()).unwrap() {
            ServerMessage::AvatarPostureChanged(msg) => assert_eq!(msg.posture, Posture::Lay),
            _ => panic!("wrong message variant"),
        }
    }

    #[test]
    fn test_avatar_posture_fields_in_snapshot_and_joined() {
        let joined_raw = r#"{"user_id": "b3", "display_name": "Bob", "x": 1, "z": 5, "posture": "stand", "rotation": 2, "y": 0.0}"#;
        match decode_server_message(OPCODE_AVATAR_JOINED, joined_raw.as_bytes()).unwrap() {
            ServerMessage::AvatarJoined(j) => {
                assert_eq!(j.posture, Posture::Stand);
                assert_eq!(j.rotation, 2);
            }
            _ => panic!("wrong message variant"),
        }

        // Older servers omit the fields: default to standing.
        let legacy = r#"{"user_id": "b3", "display_name": "Bob", "x": 1, "z": 5}"#;
        let joined: AvatarJoined = serde_json::from_str(legacy).unwrap();
        assert_eq!(
            (joined.posture, joined.rotation, joined.y),
            (Posture::Stand, 0, 0.0)
        );

        let snap: AvatarSnapshot = serde_json::from_str(
            r#"{"user_id": "u1", "display_name": "Alice", "x": 1, "z": 5, "posture": "sit", "rotation": 4, "y": 0.8}"#,
        )
        .unwrap();
        assert_eq!(
            (snap.posture, snap.rotation, snap.y),
            (Posture::Sit, 4, 0.8)
        );
    }

    // The JSON samples below are the shared fixtures owned by the server (contract/fixtures).
    #[test]
    fn test_decode_get_catalogs_samples() {
        let full: GetCatalogsResponse =
            serde_json::from_str(include_str!("../../../contract/fixtures/get_catalogs.json"))
                .unwrap();
        assert!(!full.unchanged);
        assert_eq!(full.furniture[0].catalog_id, "seat_stool_1x1");
        assert_eq!(full.furniture[0].status, None);
        assert!(full.cosmetics.is_some());
        assert_eq!(full.shop.unwrap().offers[0].price_silver, 15);
        assert!(full.assets_manifest_url.ends_with("manifest.json"));

        let same: GetCatalogsResponse = serde_json::from_str(include_str!(
            "../../../contract/fixtures/get_catalogs_unchanged.json"
        ))
        .unwrap();
        assert!(same.unchanged);
        assert!(same.furniture.is_empty() && same.cosmetics.is_none() && same.shop.is_none());
        assert_eq!(same.catalog_version, full.catalog_version);
    }

    #[test]
    fn test_decode_shop_catalog_sample() {
        let raw = include_str!("../../../contract/fixtures/shop_catalog.json");
        let cat: GetShopCatalogResponse = serde_json::from_str(raw).unwrap();
        assert_eq!(cat.categories[0].parent, "furniture");
        assert_eq!(cat.offers[0].price_silver, 15);
        assert_eq!(cat.offers[0].price_gold, 0);
    }

    #[test]
    fn test_decode_wallet_samples() {
        let w: WalletDto =
            serde_json::from_str(include_str!("../../../contract/fixtures/wallet.json")).unwrap();
        assert_eq!(
            w,
            WalletDto {
                silver: 12,
                gold: 3
            }
        );

        let hb: HeartbeatResponse = serde_json::from_str(include_str!(
            "../../../contract/fixtures/heartbeat_response.json"
        ))
        .unwrap();
        assert!(hb.daily_bonus);
        assert_eq!(hb.granted_silver, 3);

        let n: WalletChangedNotification = serde_json::from_str(include_str!(
            "../../../contract/fixtures/wallet_changed_notification.json"
        ))
        .unwrap();
        assert_eq!(n.delta_gold, 50);
    }

    #[test]
    fn test_purchase_roundtrip_and_covers() {
        let req = PurchaseItemRequest {
            catalog_id: "bed_single_1x2".into(),
            quantity: 1,
        };
        let json = serde_json::to_string(&req).unwrap();
        assert_eq!(
            serde_json::from_str::<PurchaseItemRequest>(&json).unwrap(),
            req
        );

        let resp: PurchaseItemResponse = serde_json::from_str(
            r#"{"catalog_id":"bed_single_1x2","quantity":1,"silver":40,"gold":1}"#,
        )
        .unwrap();
        assert_eq!(resp.silver, 40);

        let wallet = WalletDto {
            silver: 10,
            gold: 2,
        };
        assert!(wallet.covers(10, 2));
        assert!(!wallet.covers(11, 0));
        assert!(!wallet.covers(0, 3));
    }
}
