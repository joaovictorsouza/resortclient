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
        OPCODE_AVATAR_EXPRESSION_CHANGED => {
            let msg: AvatarExpressionChanged = serde_json::from_slice(data)?;
            Ok(ServerMessage::AvatarExpressionChanged(msg))
        }
        OPCODE_AVATAR_HOLDING_CHANGED => {
            let msg: AvatarHoldingChanged = serde_json::from_slice(data)?;
            Ok(ServerMessage::AvatarHoldingChanged(msg))
        }
        OPCODE_AVATAR_ACTION => {
            let msg: AvatarAction = serde_json::from_slice(data)?;
            Ok(ServerMessage::AvatarAction(msg))
        }
        OPCODE_PET_PLACED => Ok(ServerMessage::PetPlaced(serde_json::from_slice(data)?)),
        OPCODE_PET_PATH => Ok(ServerMessage::PetPath(serde_json::from_slice(data)?)),
        OPCODE_PET_ACTION => Ok(ServerMessage::PetAction(serde_json::from_slice(data)?)),
        OPCODE_PET_REMOVED => Ok(ServerMessage::PetRemoved(serde_json::from_slice(data)?)),
        OPCODE_PET_INTERACT_RESULT => Ok(ServerMessage::PetInteractResult(serde_json::from_slice(
            data,
        )?)),
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
        ClientMessage::SetExpression(req) => {
            let bytes = serde_json::to_vec(req)?;
            Ok((OPCODE_SET_EXPRESSION_REQUEST, bytes))
        }
        ClientMessage::SetHolding(req) => {
            let bytes = serde_json::to_vec(req)?;
            Ok((OPCODE_SET_HOLDING_REQUEST, bytes))
        }
        ClientMessage::AvatarAction(req) => {
            let bytes = serde_json::to_vec(req)?;
            Ok((OPCODE_AVATAR_ACTION_REQUEST, bytes))
        }
        ClientMessage::PetInteract(req) => {
            let bytes = serde_json::to_vec(req)?;
            Ok((OPCODE_PET_INTERACT_REQUEST, bytes))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn appearance_from_the_previous_schema_still_parses_as_the_default_look() {
        // Saved with the version 1 schema (one colour per slot, accessories): the unknown fields
        // are ignored and every v2 field falls back to its default; the server sanitizes it on load.
        let legacy = r##"{"gender":"girl","skin_color":"#D6A374","hair_id":"hair_girl_old","accessories":[]}"##;
        let parsed: AvatarAppearance = serde_json::from_str(legacy).unwrap();
        assert_eq!(parsed.body_type, BODY_FEMALE);
        assert_eq!(parsed.face_id, AvatarAppearance::default().face_id);
        assert_eq!(parsed.hair_id, "hair_girl_old");
        assert!(parsed.colors.is_empty());
    }

    #[test]
    fn appearance_v2_matches_the_shared_fixture() {
        let raw = include_str!("../../../contract/fixtures/avatar_appearance.json");
        let parsed: AvatarAppearance = serde_json::from_str(raw).unwrap();
        assert_eq!(parsed.body_type, BODY_FEMALE);
        assert_eq!(parsed.glasses_id, "Glasses.001");
        assert_eq!(parsed.colors["top"], vec!["#2196F3", "", "#FFFFFF"]);
        assert_eq!(parsed.face_morphs["eye_size"], 128);
        // It serializes back to the same JSON value (the fixture is the contract).
        let again = serde_json::to_value(&parsed).unwrap();
        assert_eq!(
            again,
            serde_json::from_str::<serde_json::Value>(raw).unwrap()
        );
    }

    #[test]
    fn expression_holding_and_action_match_the_shared_fixtures() {
        const USER: &str = "99a9a0e6-8152-4414-b657-3fbb1c77174e";
        let expression: AvatarExpressionChanged = serde_json::from_str(include_str!(
            "../../../contract/fixtures/avatar_expression_changed.json"
        ))
        .unwrap();
        assert_eq!(
            expression,
            AvatarExpressionChanged {
                user_id: USER.into(),
                expression: "happy".into()
            }
        );
        let holding: AvatarHoldingChanged = serde_json::from_str(include_str!(
            "../../../contract/fixtures/avatar_holding_changed.json"
        ))
        .unwrap();
        assert_eq!(holding.prop_id, "cup");
        let action: AvatarAction = serde_json::from_str(include_str!(
            "../../../contract/fixtures/avatar_action.json"
        ))
        .unwrap();
        assert_eq!(action.action, ACTION_DRINK);

        let (op, _) = encode_client_message(&ClientMessage::SetExpression(SetExpressionRequest {
            expression: "sad".into(),
        }))
        .unwrap();
        assert_eq!(op, 8);
        let (op, _) = encode_client_message(&ClientMessage::SetHolding(SetHoldingRequest {
            prop_id: "cup".into(),
        }))
        .unwrap();
        assert_eq!(op, 9);
        let (op, _) = encode_client_message(&ClientMessage::AvatarAction(AvatarActionRequest {
            action: ACTION_DRINK.into(),
        }))
        .unwrap();
        assert_eq!(op, 10);
        for (op, name) in [(114, "expression"), (115, "holding"), (116, "action")] {
            let data = match op {
                114 => include_bytes!("../../../contract/fixtures/avatar_expression_changed.json")
                    .as_slice(),
                115 => include_bytes!("../../../contract/fixtures/avatar_holding_changed.json")
                    .as_slice(),
                _ => include_bytes!("../../../contract/fixtures/avatar_action.json").as_slice(),
            };
            assert!(
                decode_server_message(op, data).is_ok(),
                "{name} must decode"
            );
        }
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
            "owner_id": "",
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
    fn test_decode_room_snapshot_sample_with_owner_id() {
        let raw = include_str!("../../../contract/fixtures/room_snapshot.json");
        let snap: RoomSnapshot = serde_json::from_str(raw).unwrap();
        assert_eq!(snap.owner_id, "99a9a0e6-8152-4414-b657-3fbb1c77174e");
        assert_eq!(snap.avatars.len(), 1);
    }

    #[test]
    fn a_room_snapshot_without_owner_id_is_rejected_since_protocol_2() {
        let raw = r#"{"room_id":"r","room_name":"r","layout":{"id":"l","width":1,"height":1,"door":[0,0],"heightmap":[]},"avatars":[]}"#;
        assert!(serde_json::from_str::<RoomSnapshot>(raw).is_err());
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

    #[test]
    fn pet_messages_match_the_shared_fixtures() {
        let fixture = |name: &str| {
            std::fs::read(format!(
                "{}/../../contract/fixtures/{name}",
                env!("CARGO_MANIFEST_DIR")
            ))
            .unwrap()
        };
        let ServerMessage::PetPlaced(placed) =
            decode_server_message(OPCODE_PET_PLACED, &fixture("pet_placed.json")).unwrap()
        else {
            panic!("not a pet placed");
        };
        assert_eq!(
            (placed.species.as_str(), placed.breed.as_str()),
            ("dog", "shihtzu")
        );
        assert_eq!(
            (placed.coat.as_str(), placed.action.as_str()),
            ("default", "idle")
        );
        let ServerMessage::PetPath(path) =
            decode_server_message(OPCODE_PET_PATH, &fixture("pet_path.json")).unwrap()
        else {
            panic!("not a pet path");
        };
        assert_eq!(path.path, vec![[5, 3], [6, 3], [6, 4]]);
        assert_eq!((path.tiles_per_sec, path.action.as_str()), (0.35, "walk"));
        let ServerMessage::PetAction(action) =
            decode_server_message(OPCODE_PET_ACTION, &fixture("pet_action.json")).unwrap()
        else {
            panic!("not a pet action");
        };
        assert_eq!((action.action.as_str(), action.rotation), ("bark", 4));
        assert!(matches!(
            decode_server_message(OPCODE_PET_REMOVED, &fixture("pet_removed.json")),
            Ok(ServerMessage::PetRemoved(_))
        ));
        let ServerMessage::PetInteractResult(result) = decode_server_message(
            OPCODE_PET_INTERACT_RESULT,
            &fixture("pet_interact_result.json"),
        )
        .unwrap() else {
            panic!("not a result");
        };
        assert!(!result.ok);
        assert_eq!(result.reason, "unsupported_interaction");
        let request: PetInteractRequest =
            serde_json::from_slice(&fixture("pet_interact_request.json")).unwrap();
        let (op, bytes) =
            encode_client_message(&ClientMessage::PetInteract(request.clone())).unwrap();
        assert_eq!(op, OPCODE_PET_INTERACT_REQUEST);
        assert_eq!(
            serde_json::from_slice::<PetInteractRequest>(&bytes).unwrap(),
            request
        );
    }

    #[test]
    fn the_room_snapshot_fixture_carries_its_pets_and_one_without_pets_still_parses() {
        let raw = include_str!("../../../contract/fixtures/room_snapshot.json");
        let snapshot: RoomSnapshot = serde_json::from_str(raw).unwrap();
        assert_eq!(snapshot.pets.len(), 1);
        assert_eq!(snapshot.pets[0].action, "sit");
        let mut value: serde_json::Value = serde_json::from_str(raw).unwrap();
        value.as_object_mut().unwrap().remove("pets");
        let old: RoomSnapshot = serde_json::from_value(value).unwrap();
        assert!(old.pets.is_empty());
    }
}
